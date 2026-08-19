//! What a tempo source says, and how it is heard.
//!
//! A **tempo source** is a separate program that knows where the beat is and
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
//! **Licensing.** Ableton Link is GPL-2.0-or-later and this workspace is MIT.
//! The combination is permitted, and it means a binary that links Link is
//! distributed under the GPL. Two programs exchanging data over a pipe are
//! separate works; linking them into one binary is one work. So the helper is
//! its own program, in its own repository, under its own licence.
//!
//! **Stability.** A show does not stop because a peer discovery library
//! misbehaved. A helper that crashes takes its own process with it and this one
//! carries on at the tempo it already had — see [`Source::poll`].
//!
//! ## What crosses, and why it is an anchor
//!
//! **Never a sample of "the beat right now".** Link's beat is a function of
//! time, so a message saying "the beat is 42.7" is stale the instant it is
//! written and staler still when it arrives. What crosses is an **anchor** —
//! a beat value, the tempo, and *the source's clock reading at which both were
//! true*:
//!
//! ```ndjson
//! {"t":"hello","v":1,"source":"ableton-link"}
//! {"t":"grid","bpm":128.0,"beat":1024.25,"clock_us":881234567}
//! {"t":"status","peers":2,"playing":true}
//! ```
//!
//! **That is what makes the transport's latency irrelevant to accuracy.** A
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
/// **A window and not a running minimum over all time.** A plain minimum can
/// only fall, so one message whose clock reads ahead of ours — a helper on
/// `CLOCK_BOOTTIME` across a lid-close, a helper reporting milliseconds —
/// pins the estimate there forever and every later anchor is placed that far
/// in the past. A window forgets, so a single wrong reading costs a few
/// seconds rather than the run.
const OFFSET_WINDOW: usize = 64;

/// The most one *trim* may move the grid, in beats.
///
/// **The bound that keeps a wrong anchor from becoming a wrong show.** The
/// in-process tracker has had gates, evidence counters and a trim limit since
/// it was written; a source is another program from another repository and was
/// for a while the one input trusted absolutely. 0.05 beats is 23 ms at
/// 128 bpm — enough to absorb the drift a dropped frame leaves, small enough
/// that a single bad reading is a wobble rather than a jump.
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
/// A multiple of four, so **bar phase is preserved exactly** — the whole point
/// of following a shared grid. What it buys is magnitude: `beats()` is read by
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
    /// **First, once.** What the source is and what version it speaks.
    ///
    /// A source that never sends one is refused rather than guessed at: the
    /// whole point of the field is that the reader knows before it has to
    /// interpret anything.
    Hello {
        v: u32,
        /// For the operator, not for the protocol — this is what the status
        /// line says the tempo is coming from.
        source: String,
    },
    /// **The anchor.** At the source's clock reading `clock_us`, the shared
    /// beat was `beat` and the tempo was `bpm`.
    ///
    /// `beat` is *absolute and shared*, which is the whole reason this
    /// interface exists: a beat tracker can find how fast beats go and where
    /// they are, and cannot find which one is beat one. A number every peer
    /// agrees on can.
    Grid {
        bpm: f64,
        beat: f64,
        /// The source's own monotonic clock, in microseconds, at the instant
        /// the other two were true.
        ///
        /// Its epoch is the source's and means nothing here on its own. What it
        /// is for is the *difference* from this program's clock: kept as a
        /// running minimum over many messages, that difference is the clock
        /// offset with the transport's delay filtered out of it, which is how
        /// a message the operating system sat on for fifty milliseconds still
        /// lands on the right beat.
        clock_us: i64,
    },
    /// What the source can see. Sent when it changes, and periodically, so that
    /// a silent source and a dead one are distinguishable.
    Status {
        /// Other peers on the session. Zero means the source is running and
        /// alone, which is a different thing from no source at all.
        peers: u32,
        /// Whether the session is playing, where the source has a notion of it.
        #[serde(default)]
        playing: bool,
    },
    /// An unrecognised `t`. **Ignored, never fatal** — a newer helper saying
    /// something this build has no use for must not stop the show.
    #[serde(other)]
    Unknown,
}

/// Why a source was refused before it ever produced a beat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// A version this build cannot read. Carries both so the message can name
    /// them — "refuse and say which" is the whole of the handshake.
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
/// **Opening blocks and running does not.** The greeting is read on this thread
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
    /// The **smallest** difference seen between this program's clock and the
    /// source's, which is the offset with the transport's delay filtered out:
    /// every message is late by some unknown amount, and the least late one is
    /// the closest to the truth. A single sample would carry whatever delay
    /// that one message happened to suffer.
    offset_us: Option<i64>,
    /// The last [`OFFSET_WINDOW`] clock differences, oldest overwritten first.
    /// The estimate is the minimum of these rather than of everything ever
    /// seen — see the constant.
    recent: [i64; OFFSET_WINDOW],
    seen: usize,
    /// Whether the grid has been put onto the shared one yet. The first anchor
    /// is an **alignment** and moves the grid however far it has to; every one
    /// after it is a **trim** and is bounded. See [`Correction`].
    aligned: bool,
    /// Consecutive anchors saying the grid is [`RELOCK_BEATS`] out.
    relock_evidence: u8,
    peers: u32,
    /// Anchors thrown away because their numbers were not usable. **Counted
    /// and reported rather than swallowed**: a source producing them is
    /// misbehaving and an operator has no other way to find out.
    rejected: u64,
    /// `None` until a `status` has been heard. Distinct from `Some(false)`,
    /// which is a source that has told us it is stopped — an operator would act
    /// differently on those and they used to look identical.
    playing: Option<bool>,
    /// Why the source stopped, once it has. **Latched**, because a source that
    /// has gone stays gone until an operator does something about it, and the
    /// reason is worth saying once rather than sixty times a second.
    ended: Option<String>,
    /// Whether [`Source::ended`] has been said out loud yet.
    reported: bool,
}

impl Source {
    /// Spawn `command` and read its greeting.
    ///
    /// **`command` is a program and space-separated arguments, and there is no
    /// quoting.** Not an oversight: quoting rules are a shell, and half a shell
    /// is worse than none — a value that looks quoted and is not would put the
    /// wrong arguments in front of a program that then fails for a reason
    /// nobody can see. A path with a space in it needs a wrapper script, which
    /// is a thing an operator can see and edit.
    ///
    /// `stderr` is inherited rather than captured: whatever the helper has to
    /// say about a network interface or a permission belongs on the same
    /// terminal as everything else this program says, at the moment it says it.
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
    /// **Bounded, because the likeliest bug in a separate helper is a slow
    /// start.** A source that spawns and never writes — still discovering
    /// peers, waiting on a permission — used to hang the entire run before the
    /// window opened, with nothing on the terminal saying why. Reading the
    /// greeting through a channel rather than straight off the pipe is what
    /// makes a timeout expressible at all.
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
    /// **On the frame path**: drains without blocking and allocates nothing.
    /// Only the newest anchor is kept, because an older one describes the same
    /// grid less recently — there is no history here to lose.
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
    /// **The first anchor aligns and every one after it trims.** Aligning is
    /// the whole point — putting `beats()` onto a number every peer agrees on
    /// is what makes `bar` the room's bar — and it is exactly the operation
    /// that must not happen sixty times a second on the word of a program this
    /// one did not build.
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

    /// Why the source is gone, if it is. **The grid is not touched when this
    /// becomes `Some`**: the last tempo it gave is still the best information
    /// anyone has, and a show does not want its beat reset because a helper
    /// died.
    pub fn ended(&self) -> Option<&str> {
        self.ended.as_deref()
    }

    /// Why the source stopped, **the first time it is asked and never again**.
    ///
    /// The latch is here rather than on the caller so that the frame path holds
    /// no state about a source at all — it asks, and the source remembers. The
    /// same shape the window sink's fault latch has, for the same reason: only
    /// the thing that knows what happened can know whether this is the same
    /// happening as last frame.
    pub fn unreported_end(&mut self) -> Option<&str> {
        if self.reported || self.ended.is_none() {
            return None;
        }
        self.reported = true;
        self.ended.as_deref()
    }

    /// Stop the child and wait for it.
    ///
    /// Killed rather than asked: the protocol is one-way, so there is nothing
    /// to ask *with*, and a helper whose parent has gone has nothing left to
    /// do. `kill` on an already-dead child is not an error here.
    ///
    /// **It kills the child and not its children.** A helper that spawns
    /// processes of its own has to reap them; nothing here can, short of
    /// putting the child in its own process group, which is a dependency and a
    /// platform branch for a case a real helper does not have. A shell wrapper
    /// is where this shows: kill the shell and its `sleep` lives on, holding
    /// the inherited stderr open. Point `--tempo-source` at the program.
    pub fn close(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(line: &str) -> Message {
        let message: Message = serde_json::from_str(line).expect("parse");
        let back = serde_json::to_string(&message).expect("serialise");
        let again: Message = serde_json::from_str(&back).expect("reparse");
        assert_eq!(message, again);
        message
    }

    /// **The three lines the module doc prints**, parsed and written back.
    ///
    /// The doc is the specification — the helper is another repository and may
    /// be another language, so it implements what is written here rather than
    /// importing a type. That only works if what is written here is what this
    /// build actually reads, which is what this test is.
    #[test]
    fn the_lines_the_doc_prints_are_the_lines_this_reads() {
        assert_eq!(
            round_trip(r#"{"t":"hello","v":1,"source":"ableton-link"}"#),
            Message::Hello {
                v: 1,
                source: "ableton-link".to_string()
            }
        );
        assert_eq!(
            round_trip(r#"{"t":"grid","bpm":128.0,"beat":1024.25,"clock_us":881234567}"#),
            Message::Grid {
                bpm: 128.0,
                beat: 1024.25,
                clock_us: 881_234_567
            }
        );
        assert_eq!(
            round_trip(r#"{"t":"status","peers":2,"playing":true}"#),
            Message::Status {
                peers: 2,
                playing: true
            }
        );
    }

    /// A newer helper saying something this build has no use for is ignored
    /// rather than fatal, and so is a field it has never heard of.
    ///
    /// **This is the normal case rather than the exceptional one**, which is
    /// the whole argument for a self-describing format here: the helper is
    /// released on its own schedule from its own repository, so the versions
    /// being out of step is what usually happens.
    #[test]
    fn a_newer_helper_is_understood_as_far_as_it_goes() {
        assert_eq!(
            round_trip(r#"{"t":"quantum","beats":4.0}"#),
            Message::Unknown,
            "an unknown message type must be skippable, not fatal"
        );
        assert_eq!(
            round_trip(r#"{"t":"grid","bpm":128.0,"beat":1.0,"clock_us":1,"confidence":0.9}"#),
            Message::Grid {
                bpm: 128.0,
                beat: 1.0,
                clock_us: 1
            },
            "an unknown field must be ignored, not fatal"
        );
        // And a field with a default may be absent entirely.
        assert_eq!(
            round_trip(r#"{"t":"status","peers":0}"#),
            Message::Status {
                peers: 0,
                playing: false
            }
        );
    }

    /// The greeting is checked once and names both versions when it fails.
    ///
    /// A shared library rots loudly at compile time; a separately released
    /// program rots quietly at run time, so the one thing that must not happen
    /// is a mismatch that looks like a working connection. See
    /// `docs/plugins.md`.
    #[test]
    fn a_version_this_build_cannot_read_is_refused_by_name() {
        let ours = accept(&Message::Hello {
            v: PROTOCOL_VERSION,
            source: "ableton-link".into(),
        });
        assert_eq!(ours, Ok("ableton-link".to_string()));

        let theirs = accept(&Message::Hello {
            v: PROTOCOL_VERSION + 1,
            source: "ableton-link".into(),
        });
        let Err(refusal) = theirs else {
            panic!("a future version was accepted");
        };
        let said = refusal.to_string();
        assert!(
            said.contains(&format!("v{}", PROTOCOL_VERSION + 1)),
            "{said}"
        );
        assert!(said.contains(&format!("v{PROTOCOL_VERSION}")), "{said}");

        assert_eq!(
            accept(&Message::Status {
                peers: 1,
                playing: false
            }),
            Err(Refusal::NoGreeting),
            "a source that starts talking without introducing itself is refused"
        );
    }
}

/// An anchor as this program holds it: the source's numbers, plus when it
/// arrived on a clock this program can read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Anchor {
    pub bpm: f64,
    /// The shared beat at [`Anchor::at_us`].
    pub beat: f64,
    /// When that was true, in **this program's** microseconds since the source
    /// was opened — the source's own reading translated through the offset
    /// estimate. See [`Source::offset_us`].
    pub at_us: i64,
}

impl Anchor {
    /// Where the shared beat is at `now_us`, extrapolated at the anchor's own
    /// tempo.
    ///
    /// This is why an anchor is sent rather than a sample: extrapolating from a
    /// timestamp is exact for as long as the tempo holds, so a message that
    /// took ten milliseconds to arrive is not ten milliseconds of error.
    pub fn beat_at(&self, now_us: i64) -> f64 {
        self.beat + (now_us - self.at_us) as f64 * self.bpm / 60_000_000.0
    }
}

/// What to do about an anchor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Correction {
    /// Put the grid onto the shared one, however far that is. The first anchor,
    /// and any sustained disagreement past [`RELOCK_BEATS`].
    Align { bpm: f64, shift: f64 },
    /// Move toward it by at most [`TRIM_LIMIT_BEATS`].
    Trim { bpm: f64, shift: f64 },
    /// Nothing usable.
    Hold,
}

/// What a source has told this program since it was last asked.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Heard {
    /// A fresh anchor, when one arrived. `None` means nothing new — **not**
    /// that the source is gone.
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
mod runner_tests {
    use super::*;

    /// A fake source as a script on disk.
    ///
    /// A script file rather than `sh -c '...'` because [`Source::open`] takes a
    /// program and space-separated arguments and deliberately does not parse
    /// quotes — so a test that needed quoting would be testing a shell this
    /// program does not have.
    ///
    /// **What these tests prove and do not prove is worth stating.** They
    /// exercise the spawn, the handshake, the reader thread, the clock offset
    /// and the poll. They say nothing whatever about Ableton Link, which is a
    /// network protocol nothing here speaks. Only a second peer can say that.
    fn fake(lines: &str) -> (Source, tempfile::TempDir) {
        let (source, dir) = try_fake(lines);
        (source.expect("open"), dir)
    }

    fn try_fake(lines: &str) -> (Result<Source, String>, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let script = dir.path().join("source.sh");
        std::fs::write(&script, lines).expect("write script");
        let command = format!("sh {}", script.display());
        (Source::open(&command), dir)
    }

    /// Poll until an anchor arrives or the patience runs out. Polling rather
    /// than sleeping a guessed amount: the reader is a thread and a fixed sleep
    /// is either flaky or slow.
    fn wait_for_anchor(source: &mut Source) -> Option<Anchor> {
        for _ in 0..200 {
            if let Some(anchor) = source.poll().anchor {
                return Some(anchor);
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        None
    }

    /// The happy path: greet, then anchor.
    #[test]
    fn an_anchor_arrives_and_extrapolates_from_its_own_timestamp() {
        let (mut source, _dir) = fake(concat!(
            "echo '{\"t\":\"hello\",\"v\":1,\"source\":\"fake\"}'\n",
            "echo '{\"t\":\"grid\",\"bpm\":120.0,\"beat\":100.0,\"clock_us\":0}'\n",
            "echo '{\"t\":\"status\",\"peers\":3,\"playing\":true}'\n",
            "sleep 5\n",
        ));
        assert_eq!(source.name(), "fake");

        let anchor = wait_for_anchor(&mut source).expect("no anchor arrived");
        for _ in 0..50 {
            source.poll();
            if source.peers() == 3 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(source.peers(), 3);
        assert_eq!(source.playing(), Some(true));

        // 120 bpm is two beats a second, so a second past the anchor is beat
        // 102. **This is the arithmetic that makes the transport's delay
        // irrelevant**: the anchor is read from its own timestamp rather than
        // from when it happened to arrive.
        let a_second_later = anchor.at_us + 1_000_000;
        assert!(
            (anchor.beat_at(a_second_later) - 102.0).abs() < 1e-6,
            "beat {} at one second past the anchor",
            anchor.beat_at(a_second_later)
        );
        source.close();
    }

    /// A source that dies says so **once**.
    ///
    /// The caller prints that once and does nothing else — the grid holds where
    /// it was, because the last tempo a source gave is still the best anyone
    /// has and a beat that jumped because a helper crashed would be worse than
    /// one that merely stopped being corrected. What is asserted here is the
    /// part this module owns: that a caller polling every frame cannot be
    /// handed the news more than once.
    #[test]
    fn a_source_that_dies_is_reported_once() {
        let (mut source, _dir) = fake("echo '{\"t\":\"hello\",\"v\":1,\"source\":\"fake\"}'\n");
        let mut said = Vec::new();
        for _ in 0..250 {
            source.poll();
            if let Some(why) = source.unreported_end() {
                said.push(why.to_string());
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(source.ended().is_some(), "the source outlived its script");
        assert_eq!(said.len(), 1, "reported {said:?}");
        source.close();
    }

    /// **The offset estimate is what the module is about, and nothing pinned
    /// it.** Deleting it entirely, and inverting the minimum to a maximum, both
    /// left the whole suite green — the existing anchor test asserts
    /// `beat_at(at_us + 1s)`, which is relative to whatever `at_us` came out
    /// as, so it constrains the extrapolation and says nothing about the
    /// derivation.
    ///
    /// Here the source's clock has an epoch far from ours, which is the
    /// realistic case and the one a `clock_us: 0` script never exercises.
    #[test]
    fn the_offset_is_estimated_from_the_least_delayed_message() {
        let epoch = 1_000_000_000_i64;
        let (mut source, _dir) = fake(&format!(
            concat!(
                "echo '{{\"t\":\"hello\",\"v\":1,\"source\":\"fake\"}}'\n",
                "echo '{{\"t\":\"grid\",\"bpm\":120.0,\"beat\":0.0,\"clock_us\":{}}}'\n",
                "sleep 5\n",
            ),
            epoch
        ));
        let anchor = wait_for_anchor(&mut source).expect("no anchor");
        // Our clock starts at zero, theirs at a billion, so the offset is about
        // minus a billion and the anchor lands near *our* now rather than a
        // billion microseconds in the future.
        assert!(
            anchor.at_us.abs() < 2_000_000,
            "the anchor landed at {} — the source's epoch was not removed",
            anchor.at_us
        );
        source.close();
    }

    /// **One forward clock step must not pin the estimate forever.**
    ///
    /// A running minimum can only fall, so a single message reading ahead of us
    /// — a helper on a clock that counts suspend, one reporting milliseconds —
    /// used to latch the offset there permanently and place every later anchor
    /// that far in the past. The window forgets.
    #[test]
    fn a_single_clock_step_does_not_latch_the_offset() {
        let far_ahead = 3_600_000_000_i64;
        let mut lines = String::from("echo '{\"t\":\"hello\",\"v\":1,\"source\":\"fake\"}'\n");
        lines.push_str("echo '{\"t\":\"grid\",\"bpm\":120.0,\"beat\":0.0,\"clock_us\":0}'\n");
        // The liar, once.
        lines.push_str(&format!(
            "echo '{{\"t\":\"grid\",\"bpm\":120.0,\"beat\":1.0,\"clock_us\":{far_ahead}}}'\n"
        ));
        // Then enough honest ones to fill the window past the liar.
        for i in 0..(OFFSET_WINDOW + 4) {
            lines.push_str(&format!(
                "echo '{{\"t\":\"grid\",\"bpm\":120.0,\"beat\":{}.0,\"clock_us\":0}}'\n",
                i + 2
            ));
        }
        lines.push_str("sleep 5\n");

        let (mut source, _dir) = fake(&lines);
        let mut last = None;
        for _ in 0..300 {
            if let Some(anchor) = source.poll().anchor {
                last = Some(anchor);
            }
            if source.offset_us.is_some() && last.is_some_and(|a| a.beat >= 4.0) {
                std::thread::sleep(std::time::Duration::from_millis(20));
                while let Some(anchor) = source.poll().anchor {
                    last = Some(anchor);
                }
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let anchor = last.expect("no anchor");
        assert!(
            anchor.at_us.abs() < 5_000_000,
            "the anchor landed at {} — one forward step pinned the offset",
            anchor.at_us
        );
        source.close();
    }

    /// **A message that was written long before it arrived must not drag the
    /// estimate**, which is the whole reason the estimate is a minimum.
    ///
    /// The window test above catches a clock reading *ahead*; this catches the
    /// case the minimum exists for — a message delayed on its way here. Without
    /// both, inverting `min` to `max` leaves the suite green, which is exactly
    /// what a review found it doing.
    #[test]
    fn a_message_that_arrived_late_does_not_drag_the_offset() {
        let stale = -30_000_000_i64;
        let mut lines = String::from("echo '{\"t\":\"hello\",\"v\":1,\"source\":\"fake\"}'\n");
        // One anchor stamped thirty seconds ago — a message that sat somewhere
        // on its way here. Its delta is thirty seconds larger than everyone
        // else's, and a maximum would take it.
        lines.push_str(&format!(
            "echo '{{\"t\":\"grid\",\"bpm\":120.0,\"beat\":0.0,\"clock_us\":{stale}}}'\n"
        ));
        for i in 1..8 {
            lines.push_str(&format!(
                "echo '{{\"t\":\"grid\",\"bpm\":120.0,\"beat\":{i}.0,\"clock_us\":0}}'\n"
            ));
        }
        lines.push_str("sleep 5\n");

        let (mut source, _dir) = fake(&lines);
        let mut last = None;
        for _ in 0..300 {
            if let Some(anchor) = source.poll().anchor {
                last = Some(anchor);
            }
            if last.is_some_and(|a: Anchor| a.beat >= 7.0) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let anchor = last.expect("no anchor");
        assert!(
            anchor.at_us.abs() < 5_000_000,
            "the anchor landed at {} — a thirty-second-old message set the offset",
            anchor.at_us
        );
        source.close();
    }

    /// A number that cannot be used is counted, not applied and not fatal.
    ///
    /// `clock_us` near `i64::MIN` used to panic on the frame path in a debug
    /// build; a non-finite tempo used to be smuggled as far as the oscillator.
    #[test]
    fn an_unusable_anchor_is_counted_and_dropped() {
        let (mut source, _dir) = fake(concat!(
            "echo '{\"t\":\"hello\",\"v\":1,\"source\":\"fake\"}'\n",
            "echo '{\"t\":\"grid\",\"bpm\":0.0,\"beat\":1.0,\"clock_us\":0}'\n",
            "echo '{\"t\":\"grid\",\"bpm\":120.0,\"beat\":1.0,\"clock_us\":-9223372036854775808}'\n",
            "echo '{\"t\":\"grid\",\"bpm\":120.0,\"beat\":5.0,\"clock_us\":0}'\n",
            "sleep 5\n",
        ));
        let mut last = None;
        for _ in 0..300 {
            if let Some(anchor) = source.poll().anchor {
                last = Some(anchor);
            }
            if last.is_some_and(|a: Anchor| a.beat == 5.0) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(last.expect("no anchor").beat, 5.0);
        assert_eq!(source.rejected(), 1, "the zero tempo was not counted");
        source.close();
    }

    /// **The first anchor aligns; the ones after it are bounded.**
    ///
    /// The bound is the difference between a wrong reading and a wrong show.
    /// Without it a source with a broken clock moves the grid by whatever it
    /// says, instantly, at full confidence — which is exactly what the
    /// in-process tracker has never been allowed to do.
    #[test]
    fn the_first_anchor_aligns_and_the_rest_are_trimmed() {
        let (mut source, _dir) = fake(concat!(
            "echo '{\"t\":\"hello\",\"v\":1,\"source\":\"fake\"}'\n",
            "sleep 5\n",
        ));
        let anchor = Anchor {
            bpm: 120.0,
            beat: 1024.25,
            at_us: 0,
        };

        // Our grid is at 7.0 and the room is at 1024.25: the first anchor moves
        // the whole way, and modulo `ALIGN_MODULUS` so the count stays small.
        let Correction::Align { shift, bpm } = source.correction(anchor, 7.0, 0) else {
            panic!("the first anchor did not align");
        };
        assert_eq!(bpm, 120.0);
        assert!(
            (7.0 + shift - 0.25).abs() < 1e-9,
            "landed at {}",
            7.0 + shift
        );
        // 1024.25 mod 1024 is 0.25, and 1024 is a multiple of four — so the
        // bar phase is the room's even though the number is not.
        assert!(
            ((1024.25_f64).rem_euclid(4.0) - (0.25_f64).rem_euclid(4.0)).abs() < 1e-9,
            "the alignment moved the downbeat"
        );

        // A second anchor claiming we are a hundred beats out is *not* obeyed:
        // it is trimmed, and only sustained disagreement re-aligns.
        for _ in 0..(RELOCK_EVIDENCE - 1) {
            let Correction::Trim { shift, .. } = source.correction(anchor, 924.25, 0) else {
                panic!("a single large disagreement re-aligned");
            };
            assert!(
                shift.abs() <= TRIM_LIMIT_BEATS + 1e-9,
                "trim of {shift} beats is past the limit"
            );
        }
        // The third in a row is news rather than noise.
        assert!(
            matches!(
                source.correction(anchor, 924.25, 0),
                Correction::Align { .. }
            ),
            "sustained disagreement never re-aligned"
        );
        source.close();
    }

    /// An ordinary small disagreement is passed through untouched — the clamp
    /// must not be a permanent brake on following the room.
    #[test]
    fn a_small_disagreement_is_followed_exactly() {
        let (mut source, _dir) = fake(concat!(
            "echo '{\"t\":\"hello\",\"v\":1,\"source\":\"fake\"}'\n",
            "sleep 5\n",
        ));
        let anchor = Anchor {
            bpm: 128.0,
            beat: 100.0,
            at_us: 0,
        };
        let _ = source.correction(anchor, 0.0, 0); // the alignment
        let Correction::Trim { shift, .. } = source.correction(anchor, 100.0 - 0.01, 0) else {
            panic!("a small disagreement did not trim");
        };
        assert!((shift - 0.01).abs() < 1e-9, "0.01 beats became {shift}");
        source.close();
    }

    /// A version this build cannot read is refused at open, before a frame.
    #[test]
    fn a_source_speaking_another_version_never_opens() {
        let (result, _dir) = try_fake(concat!(
            "echo '{\"t\":\"hello\",\"v\":99,\"source\":\"fake\"}'\n",
            "sleep 5\n",
        ));
        // `let Err(..) else` rather than `expect_err`, which would want a
        // `Debug` on `Source` that exists only for this line.
        let Err(error) = result else {
            panic!("a future version was accepted");
        };
        assert!(error.contains("v99"), "{error}");
        assert!(error.contains(&format!("v{PROTOCOL_VERSION}")), "{error}");
    }

    /// Noise on the stream is stepped over rather than fatal — a helper that
    /// logged to the wrong stream must not end the session.
    #[test]
    fn a_line_that_is_not_a_message_is_stepped_over() {
        let (mut source, _dir) = fake(concat!(
            "echo '{\"t\":\"hello\",\"v\":1,\"source\":\"fake\"}'\n",
            "echo 'listening on en0'\n",
            "echo '{\"t\":\"grid\",\"bpm\":90.0,\"beat\":7.0,\"clock_us\":0}'\n",
            "sleep 5\n",
        ));
        assert_eq!(wait_for_anchor(&mut source).expect("no anchor").beat, 7.0);
        source.close();
    }
}
