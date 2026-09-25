//! External process tempo synchronization protocol.
//!
//! Spawns and manages external tempo source helper processes (such as Ableton Link peers),
//! receiving timestamped beat anchors and status over NDJSON pipes.

use serde::{Deserialize, Serialize};

/// Timeout waiting for the initial greeting message.
const GREETING_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

/// Window size of clock differences used to estimate network/pipe latency minimum.
const OFFSET_WINDOW: usize = 64;

/// Maximum beat adjustment allowed per incremental trim step.
const TRIM_LIMIT_BEATS: f64 = 0.05;

/// Threshold in beats past which a resync requires realignment rather than trim.
const RELOCK_BEATS: f64 = 1.0;

/// Number of consecutive disagreeing anchors required before triggering realignment.
const RELOCK_EVIDENCE: u8 = 3;

/// Modulus applied to shared beats during alignment to keep local values bounded while preserving bar phase.
const ALIGN_MODULUS: f64 = 1024.0;

/// Wire protocol version spoken by this build.
pub const PROTOCOL_VERSION: u32 = 1;

/// One line from a tempo source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Message {
    /// Initial handshake greeting identifying the source and protocol version.
    Hello { v: u32, source: String },
    /// Timestamped beat anchor specifying tempo, beat count, and source monotonic timestamp.
    Grid { bpm: f64, beat: f64, clock_us: i64 },
    /// What the source can see. Sent when it changes, and periodically, so that a
    /// silent source and a dead one are distinguishable.
    Status {
        /// Other peers on the session. Zero means the source is running and alone,
        /// which is a different thing from no source at all.
        peers: u32,
        /// Whether the session is playing, where the source has a notion of it.
        #[serde(default)]
        playing: bool,
    },
    /// An unrecognised `t`. Ignored, never fatal — a newer helper saying something
    /// this build has no use for must not stop the show.
    #[serde(other)]
    Unknown,
}

/// Why a source was refused before it ever produced a beat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// A version this build cannot read. Carries both so the message can name them
    /// — "refuse and say which" is the whole of the handshake.
    Version { theirs: u32, ours: u32 },
    /// Something arrived before the greeting did.
    NoGreeting,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::Version { theirs, ours } => write!(
                f,
                "speaks protocol v{theirs} and this build speaks v{ours} — \
                 update whichever is older"
            ),
            Refusal::NoGreeting => {
                write!(f, "sent a message before saying what it was")
            }
        }
    }
}

/// Whether a greeting is one this build can go on with.
pub fn accept(message: &Message) -> Result<String, Refusal> {
    match message {
        Message::Hello { v, source } if *v == PROTOCOL_VERSION => Ok(source.clone()),
        Message::Hello { v, .. } => Err(Refusal::Version {
            theirs: *v,
            ours: PROTOCOL_VERSION,
        }),
        _ => Err(Refusal::NoGreeting),
    }
}

/// External tempo source running as an isolated child process.
pub struct Source {
    child: std::process::Child,
    name: String,
    events: std::sync::mpsc::Receiver<Event>,
    /// Local clock origin in microseconds for anchor reference.
    base: std::time::Instant,
    /// Estimated clock offset between host and source with transport latency filtered out.
    offset_us: Option<i64>,
    /// The last [`OFFSET_WINDOW`] clock differences, oldest overwritten first. The
    /// estimate is the minimum of these rather than of everything ever seen — see
    /// the constant.
    recent: [i64; OFFSET_WINDOW],
    seen: usize,
    /// Whether the grid has been put onto the shared one yet. The first anchor is
    /// an alignment and moves the grid however far it has to; every one after it is
    /// a trim and is bounded. See [`Correction`].
    aligned: bool,
    /// Consecutive anchors saying the grid is [`RELOCK_BEATS`] out.
    relock_evidence: u8,
    peers: u32,
    /// Anchors thrown away because their numbers were not usable. Counted and
    /// reported rather than swallowed: a source producing them is misbehaving and
    /// an operator has no other way to find out.
    rejected: u64,
    /// `None` until a `status` has been heard. Distinct from `Some(false)`, which
    /// is a source that has told us it is stopped — an operator would act
    /// differently on those and they used to look identical.
    playing: Option<bool>,
    /// Why the source stopped, once it has. Latched, because a source that has gone
    /// stays gone until an operator does something about it, and the reason is
    /// worth saying once rather than sixty times a second.
    ended: Option<String>,
    /// Whether [`Source::ended`] has been said out loud yet.
    reported: bool,
}

impl Source {
    /// Spawns the tempo source child process and performs the initial version handshake.
    pub fn open(command: &str) -> Result<Source, String> {
        let mut parts = command.split_whitespace();
        let program = parts.next().ok_or("empty command")?;
        let mut child = std::process::Command::new(program)
            .args(parts)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .map_err(|e| format!("`{program}`: {e}"))?;

        match Source::greet(&mut child, command) {
            Ok((name, base, events)) => Ok(Source {
                child,
                name,
                events,
                base,
                offset_us: None,
                recent: [0; OFFSET_WINDOW],
                seen: 0,
                aligned: false,
                relock_evidence: 0,
                peers: 0,
                rejected: 0,
                playing: None,
                ended: None,
                reported: false,
            }),
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                Err(e)
            }
        }
    }

    /// Spawns the background reader thread and waits for the Hello handshake message.
    #[allow(clippy::type_complexity)]
    fn greet(
        child: &mut std::process::Child,
        command: &str,
    ) -> Result<(String, std::time::Instant, std::sync::mpsc::Receiver<Event>), String> {
        let stdout = child.stdout.take().ok_or("no stdout")?;
        let base = std::time::Instant::now();
        let (tx, events) = std::sync::mpsc::channel();
        let (hello_tx, hello) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("tempo-source".into())
            .spawn(move || {
                let mut greeted = false;
                for line in std::io::BufRead::lines(std::io::BufReader::new(stdout)) {
                    let Ok(line) = line else {
                        let _ = tx.send(Event::Ended("the pipe broke".into()));
                        return;
                    };
                    if !greeted {
                        greeted = true;
                        let outcome = match serde_json::from_str::<Message>(&line) {
                            Ok(message) => accept(&message).map_err(|r| r.to_string()),
                            Err(e) => Err(format!(
                                "its first line is not a message this build reads: {e}"
                            )),
                        };
                        let refused = outcome.is_err();
                        let _ = hello_tx.send(outcome);
                        if refused {
                            return;
                        }
                        continue;
                    }
                    let Ok(message) = serde_json::from_str::<Message>(&line) else {
                        continue;
                    };
                    let event = match message {
                        Message::Grid {
                            bpm,
                            beat,
                            clock_us,
                        } => Event::Grid {
                            bpm,
                            beat,
                            clock_us,
                            arrived_us: base.elapsed().as_micros() as i64,
                        },
                        Message::Status { peers, playing } => Event::Status { peers, playing },
                        Message::Hello { .. } | Message::Unknown => continue,
                    };
                    if tx.send(event).is_err() {
                        return;
                    }
                }
                if !greeted {
                    let _ = hello_tx.send(Err("it closed before saying anything".into()));
                }
                let _ = tx.send(Event::Ended("it closed its output".into()));
            })
            .map_err(|e| format!("starting the reader thread: {e}"))?;

        match hello.recv_timeout(GREETING_TIMEOUT) {
            Ok(Ok(name)) => Ok((name, base, events)),
            Ok(Err(e)) => Err(e),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Err(format!(
                "`{command}` has said nothing in {} seconds — it is running and has not \
                 introduced itself",
                GREETING_TIMEOUT.as_secs()
            )),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                Err("it closed before saying anything".into())
            }
        }
    }

    /// Drains incoming messages from the background reader without blocking or allocating.
    pub fn poll(&mut self) -> Heard {
        let mut heard = Heard::default();
        while let Ok(event) = self.events.try_recv() {
            match event {
                Event::Grid {
                    bpm,
                    beat,
                    clock_us,
                    arrived_us,
                } if !(bpm.is_finite() && bpm > 0.0) || !beat.is_finite() => {
                    let _ = (bpm, beat, clock_us, arrived_us);
                    self.rejected += 1;
                }
                Event::Grid {
                    bpm,
                    beat,
                    clock_us,
                    arrived_us,
                } => {
                    let delta = arrived_us.saturating_sub(clock_us);
                    self.recent[self.seen % OFFSET_WINDOW] = delta;
                    self.seen += 1;
                    let filled = self.seen.min(OFFSET_WINDOW);
                    let offset = self.recent[..filled].iter().copied().min().unwrap_or(delta);
                    self.offset_us = Some(offset);
                    heard.anchor = Some(Anchor {
                        bpm,
                        beat,
                        at_us: clock_us.saturating_add(offset),
                    });
                }
                Event::Status { peers, playing } => {
                    self.peers = peers;
                    self.playing = Some(playing);
                    heard.status = Some((peers, playing));
                }
                Event::Ended(why) => {
                    self.ended.get_or_insert(why);
                }
            };
        }
        heard
    }

    /// Evaluates `anchor` relative to local grid time and returns the required alignment or trim correction.
    pub fn correction(&mut self, anchor: Anchor, ours: f64, now_us: i64) -> Correction {
        let theirs = anchor.beat_at(now_us);
        let want = theirs - ours;
        if !want.is_finite() {
            self.rejected += 1;
            return Correction::Hold;
        }

        if !self.aligned {
            self.aligned = true;
            self.relock_evidence = 0;
            let target = theirs.rem_euclid(ALIGN_MODULUS);
            return Correction::Align {
                bpm: anchor.bpm,
                shift: target - ours,
            };
        }

        if want.abs() > RELOCK_BEATS {
            self.relock_evidence = self.relock_evidence.saturating_add(1);
            if self.relock_evidence >= RELOCK_EVIDENCE {
                self.relock_evidence = 0;
                let target = theirs.rem_euclid(ALIGN_MODULUS);
                return Correction::Align {
                    bpm: anchor.bpm,
                    shift: target - ours,
                };
            }
        } else {
            self.relock_evidence = 0;
        }

        Correction::Trim {
            bpm: anchor.bpm,
            shift: want.clamp(-TRIM_LIMIT_BEATS, TRIM_LIMIT_BEATS),
        }
    }

    /// Anchors thrown away because their numbers were not usable.
    pub fn rejected(&self) -> u64 {
        self.rejected
    }

    /// This program's clock, in the units [`Anchor::at_us`] is in.
    pub fn now_us(&self) -> i64 {
        self.base.elapsed().as_micros() as i64
    }

    /// What the source called itself, for the status line.
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn peers(&self) -> u32 {
        self.peers
    }

    /// Whether the source says the session is playing, or `None` if it has not
    /// said. The third state is the point — see the field.
    pub fn playing(&self) -> Option<bool> {
        self.playing
    }

    /// Why the source is gone, if it is. The grid is not touched when this becomes
    /// `Some`: the last tempo it gave is still the best information anyone has, and
    /// a show does not want its beat reset because a helper died.
    pub fn ended(&self) -> Option<&str> {
        self.ended.as_deref()
    }

    /// Returns the reason the source stopped, reporting it exactly once.
    pub fn unreported_end(&mut self) -> Option<&str> {
        if self.reported || self.ended.is_none() {
            return None;
        }
        self.reported = true;
        self.ended.as_deref()
    }

    /// Terminates the child process and awaits exit.
    pub fn close(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Beat anchor tying tempo and beat position to a local microsecond timestamp.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Anchor {
    pub bpm: f64,
    pub beat: f64,
    pub at_us: i64,
}

impl Anchor {
    /// Extrapolates beat position at local time `now_us` based on anchor tempo and timestamp.
    pub fn beat_at(&self, now_us: i64) -> f64 {
        self.beat + (now_us - self.at_us) as f64 * self.bpm / 60_000_000.0
    }
}

/// What to do about an anchor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Correction {
    /// Put the grid onto the shared one, however far that is. The first anchor, and
    /// any sustained disagreement past [`RELOCK_BEATS`].
    Align { bpm: f64, shift: f64 },
    /// Move toward it by at most [`TRIM_LIMIT_BEATS`].
    Trim { bpm: f64, shift: f64 },
    /// Nothing usable.
    Hold,
}

/// What a source has told this program since it was last asked.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Heard {
    /// A fresh anchor, when one arrived. `None` means nothing new — not that the
    /// source is gone.
    pub anchor: Option<Anchor>,
    /// A fresh status, when one arrived.
    pub status: Option<(u32, bool)>,
}

/// One line as the reader thread hands it over.
enum Event {
    /// A `grid`, with this program's clock at the moment it was read.
    Grid {
        bpm: f64,
        beat: f64,
        clock_us: i64,
        arrived_us: i64,
    },
    Status {
        peers: u32,
        playing: bool,
    },
    /// The source stopped, for whatever reason, including cleanly.
    Ended(String),
}

#[cfg(test)]
mod tests;
