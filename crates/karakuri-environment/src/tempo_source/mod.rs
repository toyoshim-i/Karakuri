//! What a tempo source says, and how it is heard.
//!
//! A tempo source is a separate program that knows where the beat is and
//! tells this one. Its first implementation is an Ableton Link peer, but the
//! interface is deliberately not Link's: a MIDI clock reader, a timecode
//! reader, or a script that emits a fixed grid are all tempo sources, and the
//! beat tracker in `crate::audio` is a fourth that happens to live in-process.
//!
//! ## Why a separate program at all
//!
//! Two reasons that arrive at the same answer, which is what makes it the
//! answer rather than a workaround.
//!
//! Licensing. Ableton Link is GPL-2.0-or-later and this workspace is MIT.
//! The combination is permitted, and it means a binary that links Link is
//! distributed under the GPL. Two programs exchanging data over a pipe are
//! separate works; linking them into one binary is one work. So the helper is
//! its own program, in its own repository, under its own licence.
//!
//! Stability. A show does not stop because a peer discovery library
//! misbehaved. A helper that crashes takes its own process with it and this one
//! carries on at the tempo it already had — see [`Source::poll`].
//!
//! ## What crosses, and why it is an anchor
//!
//! Never a sample of "the beat right now". Link's beat is a function of
//! time, so a message saying "the beat is 42.7" is stale the instant it is
//! written and staler still when it arrives. What crosses is an anchor —
//! a beat value, the tempo, and *the source's clock reading at which both were
//! true*:
//!
//! ```ndjson
//! {"t":"hello","v":1,"source":"ableton-link"}
//! {"t":"grid","bpm":128.0,"beat":1024.25,"clock_us":881234567}
//! {"t":"status","peers":2,"playing":true}
//! ```
//!
//! That is what makes the transport's latency irrelevant to accuracy. A
//! message delayed ten milliseconds is not ten milliseconds of phase error; it
//! is a correctly timestamped statement that arrived late. Jitter likewise. So
//! the transport is chosen for what it costs to *implement and inspect* rather
//! than for what it costs to run — a pipe carrying ndjson, at a few messages a
//! second, read on a thread that is not the frame path.
//!
//! ## Forward compatibility is the normal case, not the exception
//!
//! The helper lives in another repository and is released on its own schedule,
//! so a version older or newer than this build is what usually happens rather
//! than what occasionally does. Hence: a `v` in the greeting that is checked
//! once, unknown message types ignored rather than fatal, and unknown fields
//! ignored — the same three rules `docs/ir-spec.md` gives for records, for the
//! same reason.

use serde::{Deserialize, Serialize};

/// How long a source has to introduce itself before it is given up on.
///
/// Generous, because a helper that is discovering peers can legitimately take a
/// moment, and short enough that an operator is told rather than left looking
/// at nothing.
const GREETING_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

/// How many recent clock differences the offset estimate is the minimum of.
///
/// A window and not a running minimum over all time. A plain minimum can only
/// fall, so one message whose clock reads ahead of ours — a helper on
/// `CLOCK_BOOTTIME` across a lid-close, a helper reporting milliseconds — pins
/// the estimate there forever and every later anchor is placed that far in the
/// past. A window forgets, so a single wrong reading costs a few seconds rather
/// than the run.
const OFFSET_WINDOW: usize = 64;

/// The most one *trim* may move the grid, in beats.
///
/// The bound that keeps a wrong anchor from becoming a wrong show. The
/// in-process tracker has had gates, evidence counters and a trim limit since
/// it was written; a source is another program from another repository and was
/// for a while the one input trusted absolutely. 0.05 beats is 23 ms at 128 bpm
/// — enough to absorb the drift a dropped frame leaves, small enough that a
/// single bad reading is a wobble rather than a jump.
const TRIM_LIMIT_BEATS: f64 = 0.05;

/// Past this, a trim would take too long to be honest and something real has
/// happened — a tempo change, a peer resyncing, a long stall.
const RELOCK_BEATS: f64 = 1.0;

/// How many anchors in a row have to agree that the grid is [`RELOCK_BEATS`]
/// out before it is moved there in one step.
///
/// The same shape `karakuri-audio`'s lock uses, for the same reason: one
/// disagreeing reading is noise and three in a row is news.
const RELOCK_EVIDENCE: u8 = 3;

/// What an alignment reduces the shared beat modulo.
///
/// A multiple of four, so bar phase is preserved exactly — the whole point of
/// following a shared grid. What it buys is magnitude: `beats()` is read by
/// procedures as `f32`, and a peer that has been running for days carries a
/// beat number large enough for one frame's motion to fall below an ulp. Taking
/// the alignment modulo this keeps the session's own count small without moving
/// where the downbeat is.
const ALIGN_MODULUS: f64 = 1024.0;

/// The wire version this build speaks.
///
/// Bumped only when an *older* reader could misunderstand a newer message
/// rather than merely miss it. Adding a field or a message type is not a bump,
/// because both are already ignorable.
pub const PROTOCOL_VERSION: u32 = 1;

/// One line from a tempo source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Message {
    /// First, once. What the source is and what version it speaks.
    ///
    /// A source that never sends one is refused rather than guessed at: the whole
    /// point of the field is that the reader knows before it has to interpret
    /// anything.
    Hello {
        v: u32,
        /// For the operator, not for the protocol — this is what the status line says
        /// the tempo is coming from.
        source: String,
    },
    /// The anchor. At the source's clock reading `clock_us`, the shared beat was
    /// `beat` and the tempo was `bpm`.
    ///
    /// `beat` is *absolute and shared*, which is the whole reason this interface
    /// exists: a beat tracker can find how fast beats go and where they are, and
    /// cannot find which one is beat one. A number every peer agrees on can.
    Grid {
        bpm: f64,
        beat: f64,
        /// The source's own monotonic clock, in microseconds, at the instant the other
        /// two were true.
        ///
        /// Its epoch is the source's and means nothing here on its own. What it is for
        /// is the *difference* from this program's clock: kept as a running minimum
        /// over many messages, that difference is the clock offset with the transport's
        /// delay filtered out of it, which is how a message the operating system sat on
        /// for fifty milliseconds still lands on the right beat.
        clock_us: i64,
    },
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

/// A tempo source, running as a child process.
///
/// Opening blocks and running does not. The greeting is read on this thread
/// before the first frame, where a stall costs nothing and a refusal has to be
/// fatal; everything after it is read on a thread of its own and collected with
/// [`Source::poll`], which never blocks and never allocates. That is the same
/// arrangement the audio device has, for the same reason: the frame path may
/// wait for nothing.
pub struct Source {
    child: std::process::Child,
    name: String,
    events: std::sync::mpsc::Receiver<Event>,
    /// This program's clock origin. Microseconds since here is what
    /// [`Anchor::at_us`] is in.
    base: std::time::Instant,
    /// The smallest difference seen between this program's clock and the source's,
    /// which is the offset with the transport's delay filtered out: every message
    /// is late by some unknown amount, and the least late one is the closest to the
    /// truth. A single sample would carry whatever delay that one message happened
    /// to suffer.
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
    /// Spawn `command` and read its greeting.
    ///
    /// `command` is a program and space-separated arguments, and there is no
    /// quoting. Not an oversight: quoting rules are a shell, and half a shell is
    /// worse than none — a value that looks quoted and is not would put the wrong
    /// arguments in front of a program that then fails for a reason nobody can see.
    /// A path with a space in it needs a wrapper script, which is a thing an
    /// operator can see and edit.
    ///
    /// `stderr` is inherited rather than captured: whatever the helper has to say
    /// about a network interface or a permission belongs on the same terminal as
    /// everything else this program says, at the moment it says it.
    pub fn open(command: &str) -> Result<Source, String> {
        let mut parts = command.split_whitespace();
        let program = parts.next().ok_or("empty command")?;
        let mut child = std::process::Command::new(program)
            .args(parts)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .map_err(|e| format!("`{program}`: {e}"))?;

        // **From here on every failure must kill the child.** `Child` does not
        // kill on drop, so an early `?` would leave the helper running — and a
        // leaked Link peer holds the discovery socket, so the *next* run fails
        // for a reason belonging to the previous one.
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

    /// Start the reader and wait for the greeting, or give up on it.
    ///
    /// Bounded, because the likeliest bug in a separate helper is a slow start. A
    /// source that spawns and never writes — still discovering peers, waiting on a
    /// permission — used to hang the entire run before the window opened, with
    /// nothing on the terminal saying why. Reading the greeting through a channel
    /// rather than straight off the pipe is what makes a timeout expressible at
    /// all.
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
                    // A line this build cannot parse at all is skipped rather
                    // than fatal, on the same terms as an unknown `t`: a helper
                    // that logged to the wrong stream must not end the session.
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
                        // A second greeting, or something from the future.
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

    /// Everything the source has said since the last call.
    ///
    /// On the frame path: drains without blocking and allocates nothing. Only the
    /// newest anchor is kept, because an older one describes the same grid less
    /// recently — there is no history here to lose.
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
                    // **Validated because it crossed a process boundary.** The
                    // oscillator refuses a non-finite shift and clamps a wild
                    // tempo, but a source that has gone wrong should be
                    // ignored here rather than smuggled that far in.
                    let _ = (bpm, beat, clock_us, arrived_us);
                    self.rejected += 1;
                }
                Event::Grid {
                    bpm,
                    beat,
                    clock_us,
                    arrived_us,
                } => {
                    // Saturating, because both sides are another program's
                    // numbers: a `clock_us` near `i64::MIN` used to panic here,
                    // on the frame path, in a debug build.
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

    /// What to do about `anchor`, given where this session's grid is now.
    ///
    /// The first anchor aligns and every one after it trims. Aligning is the whole
    /// point — putting `beats()` onto a number every peer agrees on is what makes
    /// `bar` the room's bar — and it is exactly the operation that must not happen
    /// sixty times a second on the word of a program this one did not build.
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
            // Modulo, so the session's beat count stays small. `ALIGN_MODULUS`
            // is a multiple of four, so this moves the grid by whole bars and
            // leaves the downbeat exactly where the room has it.
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

    /// Why the source stopped, the first time it is asked and never again.
    ///
    /// The latch is here rather than on the caller so that the frame path holds no
    /// state about a source at all — it asks, and the source remembers. The same
    /// shape the window sink's fault latch has, for the same reason: only the thing
    /// that knows what happened can know whether this is the same happening as last
    /// frame.
    pub fn unreported_end(&mut self) -> Option<&str> {
        if self.reported || self.ended.is_none() {
            return None;
        }
        self.reported = true;
        self.ended.as_deref()
    }

    /// Stop the child and wait for it.
    ///
    /// Killed rather than asked: the protocol is one-way, so there is nothing to
    /// ask *with*, and a helper whose parent has gone has nothing left to do.
    /// `kill` on an already-dead child is not an error here.
    ///
    /// It kills the child and not its children. A helper that spawns processes of
    /// its own has to reap them; nothing here can, short of putting the child in
    /// its own process group, which is a dependency and a platform branch for a
    /// case a real helper does not have. A shell wrapper is where this shows: kill
    /// the shell and its `sleep` lives on, holding the inherited stderr open. Point
    /// `--tempo-source` at the program.
    pub fn close(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// An anchor as this program holds it: the source's numbers, plus when it
/// arrived on a clock this program can read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Anchor {
    pub bpm: f64,
    /// The shared beat at [`Anchor::at_us`].
    pub beat: f64,
    /// When that was true, in this program's microseconds since the source was
    /// opened — the source's own reading translated through the offset estimate.
    /// See [`Source::offset_us`].
    pub at_us: i64,
}

impl Anchor {
    /// Where the shared beat is at `now_us`, extrapolated at the anchor's own
    /// tempo.
    ///
    /// This is why an anchor is sent rather than a sample: extrapolating from a
    /// timestamp is exact for as long as the tempo holds, so a message that took
    /// ten milliseconds to arrive is not ten milliseconds of error.
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
