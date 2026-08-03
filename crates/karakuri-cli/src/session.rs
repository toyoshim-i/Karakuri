//! Writing a session stream, and replaying one.
//!
//! A Set file says what the material *is*. A session stream is the timeline:
//! that file's records, then `tick` records and the edits between them, so
//! every edit lands at an exact frame position because it sits between two
//! known ticks.
//!
//! With this, `tick` finally has a writer and the record stream is the whole
//! path — `README.md`'s invariant stops being a target. What a session
//! reproduces is the *performance*: the same material, the same frames, the
//! same fader moves at the same instants, and the same audio, without a
//! microphone.
//!
//! ## The render thread writes nothing
//!
//! `Line::new` serialises eagerly and a `String` is an allocation, so a frame
//! that produced a line would be allocating on the render thread — the first
//! invariant in `README.md`, and one this crate has already had to repair once
//! for a record it built per frame.
//!
//! So the frame path only ever **moves a `Record` into a `Vec` that already has
//! room**, and a writer thread does the serialising and the I/O. The batch is
//! handed over whole and an empty one comes back on a return channel, so there
//! is one allocation per batch buffer for the life of the run and none after
//! the buffers exist.
//!
//! **What happens when the writer falls behind is the interesting part.** The
//! channel is bounded. A frame that finds it full does not block, does not
//! grow, and does not silently skip: it counts the batch as dropped and says so
//! at the end. A session with a hole in it is not a session, and the honest
//! response to a disk that cannot keep up is to say which frames are missing
//! rather than to stall the show or to hand back a file that looks complete.
//!
//! ## Replay drives the same engine
//!
//! Nothing about the engine changes. A replay reads `tick` for the step count
//! that a live run measures from the clock, reads `audio` and `tempo` instead
//! of opening a device, and applies the mix and transport records where they
//! sit. That is the whole of it — the arrangement `audio.rs` and `mix.rs` were
//! built for, with a file on the other end instead of a device and a keyboard.

use std::io::Write;
use std::sync::mpsc::{Receiver, SyncSender, TrySendError};

use karakuri_store::ndjson::Line;
use karakuri_store::record::Record;
use karakuri_store::store::Store;

/// Records per batch. A frame emits at most a handful — a tick, an audio
/// frame, sometimes a tempo correction — so this is a second or two of a
/// session at 60 Hz, which is the granularity a crash loses.
const BATCH: usize = 256;

/// Batches the writer may be behind by. Two is enough to cover a disk hiccup
/// and small enough that a writer which has genuinely stopped is noticed in
/// seconds rather than after the buffer has eaten the session.
const QUEUE: usize = 2;

/// The frame path's half of the writer.
pub struct Recorder {
    /// This batch, with [`BATCH`] of capacity reserved. A push never grows it,
    /// because it is handed away the moment it is full.
    batch: Vec<Record>,
    /// `None` once the writer has been told to stop. An `Option` so that both
    /// [`Recorder::finish`] and `Drop` can close it, which a plain field could
    /// not: moving out of a type that implements `Drop` does not compile.
    to_writer: Option<SyncSender<Vec<Record>>>,
    /// Emptied batches coming back. A run steady-state cycles the same
    /// [`QUEUE`] + 1 buffers forever.
    spares: Receiver<Vec<Record>>,
    /// Batches the writer could not take. **Frames, not bytes**: the number
    /// that matters is how much of the performance is missing.
    dropped_batches: u64,
    writer: Option<std::thread::JoinHandle<Result<u64, String>>>,
}

impl Recorder {
    /// Start recording into `sessions/<id>.ndjson`, beginning with `head` —
    /// the Set file's records, which are what makes the stream self-contained.
    pub fn open(store: &Store, id: &str, head: &[Line]) -> Result<Recorder, String> {
        let mut file = store
            .append_session(id)
            .map_err(|e| format!("session `{id}`: {e}"))?;
        for line in head {
            writeln!(file, "{}", line.as_str())
                .map_err(|e| format!("session `{id}`: {e}"))?;
        }

        let (to_writer, from_frames) = std::sync::mpsc::sync_channel::<Vec<Record>>(QUEUE);
        let (return_spares, spares) = std::sync::mpsc::sync_channel::<Vec<Record>>(QUEUE + 1);
        // The buffers, all of them, allocated here. Nothing after this point
        // allocates one.
        for _ in 0..QUEUE {
            let _ = return_spares.send(Vec::with_capacity(BATCH));
        }

        let writer = std::thread::spawn(move || -> Result<u64, String> {
            let mut written = 0u64;
            let mut text = String::with_capacity(BATCH * 64);
            while let Ok(mut batch) = from_frames.recv() {
                text.clear();
                for record in batch.drain(..) {
                    // Serialised here, on this thread, which is the whole
                    // reason this thread exists.
                    text.push_str(Line::new(record).as_str());
                    text.push('\n');
                    written += 1;
                }
                file.write_all(text.as_bytes())
                    .map_err(|e| format!("writing session: {e}"))?;
                // Back to the frame path. A full return channel means the
                // frame path is not consuming spares, which cannot happen —
                // and if it did, dropping the buffer costs one allocation
                // later rather than a deadlock now.
                let _ = return_spares.try_send(batch);
            }
            file.flush().map_err(|e| format!("flushing session: {e}"))?;
            Ok(written)
        });

        Ok(Recorder {
            batch: Vec::with_capacity(BATCH),
            to_writer: Some(to_writer),
            spares,
            dropped_batches: 0,
            writer: Some(writer),
        })
    }

    /// Put one record in the stream. **Allocates nothing and never blocks.**
    ///
    /// Safe to call from the frame path, which is the only reason any of the
    /// machinery above exists.
    pub fn push(&mut self, record: Record) {
        if self.batch.len() == self.batch.capacity() {
            self.hand_off();
        }
        // Only ever after a hand-off has made room, so this cannot grow the
        // buffer — unless the hand-off failed, and then the batch was cleared.
        self.batch.push(record);
    }

    /// Hand this batch to the writer and take an empty one back.
    ///
    /// Both halves are non-blocking. A writer that cannot take the batch loses
    /// it, counted; a run with no spare to take reuses this one after clearing
    /// it, which is the same loss seen from the other side. Neither stalls a
    /// frame, and both are reported at the end.
    fn hand_off(&mut self) {
        let spare = self.spares.try_recv().ok();
        let batch = match spare {
            Some(spare) => std::mem::replace(&mut self.batch, spare),
            None => {
                self.dropped_batches += 1;
                self.batch.clear();
                return;
            }
        };
        let Some(to_writer) = &self.to_writer else {
            self.batch = batch;
            self.batch.clear();
            return;
        };
        if let Err(TrySendError::Full(mut lost) | TrySendError::Disconnected(mut lost)) =
            to_writer.try_send(batch)
        {
            self.dropped_batches += 1;
            lost.clear();
            // Put it back in circulation rather than dropping it, so the run
            // does not slowly lose its buffers to a writer that stalled once.
            let _ = self.spares.try_recv();
            self.batch = lost;
        }
    }

    /// Flush what is left and stop the writer. Blocks, and is for the end of a
    /// run — a stall is free there and losing the last second of a session to
    /// tidiness would not be.
    pub fn finish(mut self) -> Result<(u64, u64), String> {
        if !self.batch.is_empty() {
            let batch = std::mem::take(&mut self.batch);
            // Blocking, unlike every send above: this one is not on a frame.
            if let Some(to_writer) = &self.to_writer {
                let _ = to_writer.send(batch);
            }
        }
        // Closing the channel is what ends the writer's `recv` loop.
        self.to_writer = None;
        let written = match self.writer.take() {
            Some(handle) => handle
                .join()
                .map_err(|_| "the session writer panicked".to_string())??,
            None => 0,
        };
        Ok((written, self.dropped_batches))
    }
}

impl Drop for Recorder {
    /// A recorder dropped without [`Recorder::finish`] still ends the writer,
    /// so the file is closed and flushed. What it cannot do is report, which is
    /// why `finish` exists and is what the CLI calls.
    fn drop(&mut self) {
        self.to_writer = None;
        if let Some(handle) = self.writer.take() {
            let _ = handle.join();
        }
    }
}

/// A session stream, split into what it says about the material and what it
/// says about the performance.
pub struct Session {
    /// The head: the Set file's records, in order, so the material can be
    /// built the same way `--load-set` builds it.
    pub head: Vec<Line>,
    /// One entry per `tick`: what to apply *before* that frame, and how many
    /// steps the frame advances.
    pub frames: Vec<Frame>,
    /// Records after the last tick. A session that ended between frames has
    /// them, and dropping them silently would lose the last thing an operator
    /// did.
    pub trailing: Vec<Record>,
}

/// One frame of a replay.
pub struct Frame {
    /// The edits that sit between the previous tick and this one. **Applied
    /// before the frame renders**, which is where they were applied live: a
    /// key press takes effect on the next frame, not the one already drawn.
    pub before: Vec<Record>,
    pub steps: u8,
}

/// Split a session stream into its head and its frames.
///
/// A record is the head's if [`Record::is_set_state`] says so **and no tick has
/// happened yet**. The second half matters: a `param` record after the first
/// tick is an edit made during the performance, and folding it into the head
/// would apply it before the run started.
pub fn split(lines: Vec<Line>) -> Session {
    let mut head = Vec::new();
    let mut frames = Vec::new();
    let mut pending = Vec::new();
    let mut started = false;

    for line in lines {
        match line.record() {
            Record::Tick { steps } => {
                started = true;
                frames.push(Frame {
                    before: std::mem::take(&mut pending),
                    steps: *steps,
                });
            }
            record if !started && record.is_set_state() => head.push(line),
            record => pending.push(record.clone()),
        }
    }

    Session {
        head,
        frames,
        trailing: pending,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use karakuri_store::record::{Layer, Value};

    fn set_line() -> Line {
        Line::new(Record::Set {
            id: "s".into(),
            v: 1,
        })
    }

    /// **The head stops at the first tick.** A `param` before it is what the
    /// Set was built with; the same record after it is an edit an operator
    /// made, and applying that one at the start would be a different
    /// performance.
    #[test]
    fn state_after_the_first_tick_is_an_edit_and_not_the_head() {
        let param = |v: f32| {
            Line::new(Record::Param {
                layer: Layer::L1,
                key: "radius".into(),
                value: Value::Scalar(v),
            })
        };
        let session = split(vec![
            set_line(),
            param(1.0),
            Line::new(Record::Tick { steps: 1 }),
            param(2.0),
            Line::new(Record::Tick { steps: 2 }),
        ]);

        assert_eq!(session.head.len(), 2, "the set and the param before the tick");
        assert_eq!(session.frames.len(), 2);
        assert!(session.frames[0].before.is_empty());
        assert_eq!(session.frames[0].steps, 1);
        assert_eq!(
            session.frames[1].before.len(),
            1,
            "the second param is an edit on the second frame"
        );
        assert_eq!(session.frames[1].steps, 2);
    }

    /// Records after the last tick are kept. A session that ended between
    /// frames still recorded what the operator last did.
    #[test]
    fn records_after_the_last_tick_are_not_lost() {
        let session = split(vec![
            set_line(),
            Line::new(Record::Tick { steps: 1 }),
            Line::new(Record::Gain {
                slot: 0,
                value: 0.5,
            }),
        ]);
        assert_eq!(session.frames.len(), 1);
        assert_eq!(session.trailing.len(), 1);
    }

    /// **The frame path allocates nothing.** A batch is handed away the moment
    /// it is full and an empty one comes back, so the buffer's capacity — and
    /// therefore its pointer — never changes.
    ///
    /// A counting allocator would be the direct assertion and cannot be used
    /// here: `#[global_allocator]` is per binary and this is one. The capacity
    /// is the observable consequence, and it is not a proxy — a `Vec` that grew
    /// would report a larger one.
    #[test]
    fn pushing_records_never_grows_the_batch() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::open(dir.path()).expect("store");
        let mut recorder = Recorder::open(&store, "s", &[]).expect("open");

        let capacity = recorder.batch.capacity();
        // Several batches' worth, so the hand-off and the return trip both
        // happen many times over.
        for _ in 0..BATCH * 8 {
            recorder.push(Record::Tick { steps: 1 });
            assert_eq!(
                recorder.batch.capacity(),
                capacity,
                "the batch grew, so a frame allocated"
            );
        }
        // **Drops are expected here and are the second half of the claim.**
        // Nothing paces this loop, so it offers batches far faster than a disk
        // takes them — which is precisely the case the design is about. What
        // must hold is that the frame path did not grow, did not block, and
        // *counted* what was lost. A recorder that silently skipped would pass
        // the capacity assertion above and hand back a file that looked whole.
        let (written, dropped) = recorder.finish().expect("finish");
        assert!(written > 0, "nothing reached the file");
        assert_eq!(
            written + dropped * BATCH as u64,
            (BATCH * 8) as u64,
            "records went missing without being counted as dropped"
        );
    }

    /// At a frame's pace nothing is dropped, which is the case that actually
    /// happens: a batch is a second or two of a session and the writer has that
    /// long to put 256 short lines on a disk.
    #[test]
    fn a_session_at_a_frames_pace_loses_nothing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::open(dir.path()).expect("store");
        let mut recorder = Recorder::open(&store, "s", &[]).expect("open");
        // Three batches' worth, handed over with the writer given a moment
        // between them — far less of a moment than a frame is.
        for i in 0..BATCH * 3 {
            recorder.push(Record::Tick { steps: 1 });
            if i % 64 == 0 {
                std::thread::yield_now();
            }
        }
        let (written, dropped) = recorder.finish().expect("finish");
        assert_eq!(dropped, 0, "a paced session lost {dropped} batches");
        assert_eq!(written, (BATCH * 3) as u64);
    }

    /// What was pushed is what the file holds, in order.
    #[test]
    fn a_recorded_session_reads_back_as_what_was_pushed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::open(dir.path()).expect("store");
        let mut recorder = Recorder::open(&store, "s", &[set_line()]).expect("open");
        for steps in [1u8, 2, 1, 4] {
            recorder.push(Record::Tick { steps });
        }
        let (written, dropped) = recorder.finish().expect("finish");
        assert_eq!((written, dropped), (4, 0));

        let session = split(store.read_session("s").expect("read"));
        assert_eq!(session.head.len(), 1);
        assert_eq!(
            session.frames.iter().map(|f| f.steps).collect::<Vec<_>>(),
            vec![1, 2, 1, 4]
        );
    }
}
