//! Writing a session stream, and replaying one.
//!
//! A Set file says what one Set's material *is*. A session stream is the
//! timeline: a **head** saying what the whole deck held, then `tick` records and
//! the edits between them, so every edit lands at an exact frame position
//! because it sits between two known ticks.
//!
//! ## The head
//!
//! Everything before the first `tick` that is not a frame's own — see [`split`]
//! and [`head`], and `docs/ir-spec.md` for the specification. One Set file's
//! records for slot 0, one `procedure` record per node of every other slot, and
//! then the deck: the canvas, every slot's gain, opacity, blend, residency, mask
//! and transport, the look, the level at the master chain's entry and the chain
//! itself. A replay builds a deck as wide as the head names and puts all of it
//! back before the first frame renders.
//!
//! With this, `tick` finally has a writer and the record stream is the whole
//! path — every control ending at the same record
//! (`docs/principles/0090-a-surface-offers-it-never-decides.md`) stops being a
//! target. What a session reproduces is the *performance*: the same material,
//! the same frames, the same fader moves at the same instants, and the same
//! audio, without a microphone.
//!
//! ## The render thread writes nothing
//!
//! `Line::new` serialises eagerly and a `String` is an allocation, so a frame
//! that produced a line would be allocating on the render thread, which nothing
//! does — and a rule this instrument has already had to repair once for a
//! record it built per frame.
//!
//! So the frame path only ever moves a `Record` into a `Vec` that already has
//! room, and a writer thread does the serialising and the I/O. An `audio`
//! record carries a `Vec` of its own, so it is *swapped* for an empty shell
//! rather than copied — see [`Recorder::push_audio`] — and the writer returns
//! each band buffer after serialising it so the shells circulate too. The batch
//! is handed over whole and an empty one comes back on a return channel, so
//! there is one allocation per batch buffer for the life of the run and none
//! after the buffers exist.
//!
//! What happens when the writer falls behind is the interesting part. The
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
//! of opening a device — so a binding to `energy` replays at what a microphone
//! heard rather than at what the bus invents — and applies the mix and
//! transport records where they sit. That is the whole of it — the arrangement
//! `audio.rs` and `mix.rs`, both beside this file now, were built for, with a
//! file on the other end instead of a device and a keyboard.

use std::io::Write;
use std::sync::mpsc::{Receiver, SyncSender, TrySendError};

use karakuri_engine::deck::{Blend, Mask, Residency};
use karakuri_engine::master::SlotSpec;
use karakuri_engine::transport::Transport;
use karakuri_engine::Look;
use karakuri_signal::measured::MAX_BANDS;
use karakuri_store::hash::Hash;
use karakuri_store::ndjson::Line;
use karakuri_store::record::{DeckSlot, Layer, NodeAddress, Record};
use karakuri_store::store::Store;

/// Records per batch. A frame emits at most a handful — a tick, an audio frame,
/// sometimes a tempo correction — so this is a second or two of a session at 60
/// Hz, which is the granularity a crash loses.
const BATCH: usize = 256;

/// Batches the writer may be behind by. Two is enough to cover a disk hiccup
/// and small enough that a writer which has genuinely stopped is noticed in
/// seconds rather than after the buffer has eaten the session.
const QUEUE: usize = 2;

/// What a finished recording amounts to. Three numbers because the two ways a
/// stream can be short of what happened are different holes: a lost batch is a
/// second of everything, a lost audio frame is one frame's measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Written {
    pub records: u64,
    pub dropped_batches: u64,
    pub dropped_audio: u64,
}

/// The frame path's half of the writer.
pub struct Recorder {
    /// This batch, with [`BATCH`] of capacity reserved. A push never grows it,
    /// because it is handed away the moment it is full.
    batch: Vec<Record>,
    /// `None` once the writer has been told to stop. An `Option` so that both
    /// [`Recorder::finish`] and `Drop` can close it, which a plain field could not:
    /// moving out of a type that implements `Drop` does not compile.
    to_writer: Option<SyncSender<Vec<Record>>>,
    /// Emptied batches coming back. A run steady-state cycles the same [`QUEUE`] +
    /// 1 buffers forever.
    spares: Receiver<Vec<Record>>,
    /// Emptied `Record::Audio` shells coming back from the writer, with their band
    /// buffers intact. The whole reason audio can be recorded at all: its record
    /// carries a `Vec`, so cloning one per frame is exactly the allocation this
    /// module exists to avoid, and swapping needs something to swap with.
    audio_shells: Receiver<Record>,
    /// Frames whose audio was not recorded because no shell was free. Counted apart
    /// from `dropped_batches`: a stream missing a measurement is a different hole
    /// from a stream missing a second of everything.
    dropped_audio: u64,
    /// Batches the writer could not take. Frames, not bytes: the number that
    /// matters is how much of the performance is missing.
    dropped_batches: u64,
    writer: Option<std::thread::JoinHandle<Result<u64, String>>>,
}

impl Recorder {
    /// Start recording into `sessions/<id>.ndjson`, beginning with `head` — the Set
    /// file's records, which are what makes the stream self-contained.
    pub fn open(store: &Store, id: &str, head: &[Line]) -> Result<Recorder, String> {
        let mut file = store
            .append_session(id)
            .map_err(|e| format!("session `{id}`: {e}"))?;
        for line in head {
            writeln!(file, "{}", line.as_str()).map_err(|e| format!("session `{id}`: {e}"))?;
        }

        let (to_writer, from_frames) = std::sync::mpsc::sync_channel::<Vec<Record>>(QUEUE);
        // Shells go round the same way batches do, and there have to be enough
        // to cover **everything in flight**, not everything in a frame. A shell
        // does not come back until the batch holding it has been written, and a
        // batch is not handed over until it is full — so in the worst case,
        // every record being a measurement, the pool has to carry the batch
        // being filled plus the ones queued behind it. Three was not enough by
        // two orders of magnitude and the frame path spent the run without one:
        // the batch never filled, because almost nothing was going into it.
        const SHELLS: usize = BATCH * (QUEUE + 1);
        let (return_shells, audio_shells) = std::sync::mpsc::sync_channel::<Record>(SHELLS);
        for _ in 0..SHELLS {
            let _ = return_shells.send(Record::Audio {
                energy: 0.0,
                onset: 0.0,
                bands: Vec::with_capacity(MAX_BANDS),
                confidence: 0.0,
            });
        }
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
                    // An audio record's band buffer goes back to the frame
                    // path rather than being freed with the record, so the
                    // shells circulate the way the batches do and nothing
                    // allocates one after start-up.
                    let shell = match &record {
                        Record::Audio { .. } => Some(()),
                        _ => None,
                    };
                    // Serialised here, on this thread, which is the whole
                    // reason this thread exists.
                    let line = Line::new(record);
                    text.push_str(line.as_str());
                    text.push('\n');
                    written += 1;
                    if shell.is_some() {
                        if let Record::Audio { mut bands, .. } = line.into_record() {
                            bands.clear();
                            let _ = return_shells.try_send(Record::Audio {
                                energy: 0.0,
                                onset: 0.0,
                                bands,
                                confidence: 0.0,
                            });
                        }
                    }
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
            audio_shells,
            dropped_audio: 0,
            dropped_batches: 0,
            writer: Some(writer),
        })
    }

    /// Put one record in the stream. Allocates nothing and never blocks.
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

    /// Put an audio record in the stream by swapping, never by cloning.
    ///
    /// The caller keeps a record it reuses every frame; this takes that one and
    /// leaves an empty shell in its place, so the band buffer moves rather than
    /// being copied. Allocates nothing, and is the only way `Record::Audio` can
    /// reach a session stream from a frame at all.
    ///
    /// With no shell free the frame's audio is not recorded and is counted. A
    /// stream missing a measurement replays with that frame's bindings at the
    /// confidence the bus invents, which is wrong quietly — so the count is
    /// reported at the end rather than left to be inferred.
    pub fn push_audio(&mut self, record: &mut Record) {
        match self.audio_shells.try_recv() {
            Ok(shell) => {
                let filled = std::mem::replace(record, shell);
                self.push(filled);
            }
            Err(_) => self.dropped_audio += 1,
        }
    }

    /// Hand this batch to the writer and take an empty one back.
    ///
    /// Both halves are non-blocking. A writer that cannot take the batch loses it,
    /// counted; a run with no spare to take reuses this one after clearing it,
    /// which is the same loss seen from the other side. Neither stalls a frame, and
    /// both are reported at the end.
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

    /// Flush what is left and stop the writer. Blocks, and is for the end of a run
    /// — a stall is free there and losing the last second of a session to tidiness
    /// would not be.
    pub fn finish(mut self) -> Result<Written, String> {
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
        Ok(Written {
            records: written,
            dropped_batches: self.dropped_batches,
            dropped_audio: self.dropped_audio,
        })
    }
}

impl Drop for Recorder {
    /// A recorder dropped without [`Recorder::finish`] still ends the writer, so
    /// the file is closed and flushed. What it cannot do is report, which is why
    /// `finish` exists and is what a surface calls.
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
    /// The head's material: one Set file's records, in order, so the Set the head
    /// slot held can be built the same way `--load-set` builds it.
    ///
    /// One Set file and not one per slot. The other slots' material is named in
    /// [`Session::opening`], by the `procedure` records that already name it
    /// mid-stream — see [`head`].
    pub head: Vec<Line>,
    /// The deck's state at frame 0: every record before the first `tick` that is
    /// not a Set file's and not a frame's own measurement.
    ///
    /// `procedure` records naming what every slot beyond the head's held, and then
    /// the mix — `canvas`, `look`, `master_out`, `master_chain`, and per slot
    /// `gain`, `opacity`, `blend`, `residency`, `mask` and `transport`. Applied
    /// before the first frame renders, which is where they were true.
    pub opening: Vec<Record>,
    /// One entry per `tick`: what to apply *before* that frame, and how many steps
    /// the frame advances.
    pub frames: Vec<Frame>,
    /// Records after the last tick. A session that ended between frames has them,
    /// and dropping them silently would lose the last thing an operator did.
    pub trailing: Vec<Record>,
}

impl Session {
    /// What the performance rendered at, and how many *later* `canvas` records the
    /// stream also holds.
    ///
    /// Read before the deck is built rather than applied as the replay reaches it,
    /// because it decides the size of everything a replay allocates: the deck's
    /// slot targets, the HDR target, the PNG target and the readback buffer are all
    /// made once, and honouring this after they exist would mean remaking all four
    /// mid-run — the allocation the frame path forbids, and the reason
    /// [`Record::Canvas`] is fixed for a run in the first place.
    ///
    /// The count is returned rather than swallowed. A stream with a second one was
    /// not written by this program, and a replay that quietly obeyed the first
    /// would look exactly like one that had obeyed all of them.
    pub fn canvas(&self) -> (Option<(u32, u32)>, usize) {
        let mut found = None;
        let mut extra = 0;
        // **`opening` and `trailing` as well.** The canvas is written into the
        // head by both writers, so `opening` is where a session written by this
        // program keeps it; `trailing` is where a session that never drew a
        // frame puts everything, because no tick means no `Frame` to hold it,
        // and a run closed before the first frame — or one whose every frame
        // was abandoned — still recorded the canvas it was going to use.
        // Scanning the frames alone reported "no `canvas` record" about a
        // stream that plainly has one.
        for record in self
            .opening
            .iter()
            .chain(self.frames.iter().flat_map(|f| f.before.iter()))
            .chain(self.trailing.iter())
        {
            if let Record::Canvas { width, height } = record {
                match found {
                    None => found = Some((*width, *height)),
                    Some(_) => extra += 1,
                }
            }
        }
        (found, extra)
    }
}

/// One frame of a replay.
pub struct Frame {
    /// The edits that sit between the previous tick and this one. Applied before
    /// the frame renders, which is where they were applied live: a key press takes
    /// effect on the next frame, not the one already drawn.
    pub before: Vec<Record>,
    pub steps: u8,
}

/// Split a session stream into its head and its frames.
///
/// **The head is everything before the first `tick` that is not a frame's own.**
/// A record after the first tick is an edit made during the performance, and
/// folding it into the head would apply it before the run started; a
/// [`Record::is_measurement`] record before the first tick is the first frame's
/// `audio` or `tempo` and belongs to that frame, because a frame writes its
/// edits, then what it heard, then the tick that closes it.
///
/// The head reaches the caller as two lists rather than one, because it has two
/// readers and they read two vocabularies. [`Record::is_set_state`] says which:
/// a Set file's records go to [`Session::head`], where `setfile::from_lines`
/// builds a Set out of them, and the deck's own records go to
/// [`Session::opening`], where a replay applies them to the deck it has just
/// built. The file has one rule and the struct has two fields.
pub fn split(lines: Vec<Line>) -> Session {
    let mut head = Vec::new();
    let mut opening = Vec::new();
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
            record if !started && !record.is_measurement() => opening.push(record.clone()),
            record => pending.push(record.clone()),
        }
    }

    Session {
        head,
        opening,
        frames,
        trailing: pending,
    }
}

/// What one deck slot held when a recording began.
///
/// The nodes it was playing, by the addresses a [`Record::Procedure`] names, and
/// the mix controls the deck holds for it. Plain data with nothing borrowed, so
/// a surface can read it on a frame and write it on a thread.
#[derive(Debug, Clone)]
pub struct SlotHeld {
    /// Every node of the Set in the slot: which layer, which index on that layer,
    /// and the store address its source is at.
    ///
    /// The address is the promise. A `procedure` record names bytes the store must
    /// already hold, so a writer puts every one of these before it writes a head —
    /// `setfile::Sources::into_nodes` on one side, `Placed::put` on the other.
    pub nodes: Vec<(Layer, u32, Hash)>,
    pub gain: f32,
    pub opacity: f32,
    pub blend: Blend,
    /// What the slot was *asked* to do, never the level the governor granted:
    /// [`Record::Residency`] records the request, so a session recorded on a fast
    /// machine and replayed on a slow one re-derives the rest.
    pub residency: Residency,
    pub mask: Mask,
    pub transport: Transport,
}

/// What the deck held when a recording began: one entry per slot, and the four
/// values that belong to the fold rather than to anything folded.
#[derive(Debug, Clone)]
pub struct Held {
    pub canvas: (u32, u32),
    pub look: Look,
    pub master_out: f32,
    /// The master chain as it stands, whole. A Set file carries nothing for the
    /// chain (ADR-0340), so this is the only thing in a head that can put it back.
    pub master_chain: Vec<SlotSpec>,
    /// Every slot of the deck, in deck order. A slot cannot hold nothing — a deck
    /// is built with one `HotSwap` per slot — so this is as long as the deck is,
    /// and `slots[0]` is the slot whose Set is the head's material.
    pub slots: Vec<SlotHeld>,
}

/// The head of a session stream: what the deck held at frame 0, said in the
/// records that already say it.
///
/// `material` is one Set file's records, read back out of the store — the Set in
/// the head slot, with its params, its bindings, its seeds and its edges. Every
/// other slot is named by `procedure` records, one per node, which is the record
/// a swap already writes and a replay already obeys; those slots are built
/// against this file's parameter table rather than against a second copy of it.
///
/// Then the deck, as ordinary session records: the canvas, every slot's gain,
/// opacity, blend, residency, mask and transport, the look, the level at the
/// chain's entry and the chain itself.
///
/// **Written always and never only where it differs from a fresh deck.** A
/// replay that had to know what a deck starts at would be a second derivation of
/// the deck's defaults, kept in step with the engine's by nothing; a head that
/// says all of it is a head a reader can obey without knowing anything.
///
/// The one derivation of the sentence *what did this deck hold*. Both writers —
/// `karakuri-cli`'s `--record-session` and the console's `rec` pill — call this
/// one function, so the two cannot spell a head two ways.
pub fn head(material: Vec<Line>, held: &Held) -> Vec<Line> {
    let mut lines = material;
    // **The canvas first among the deck's records**, because a replay reads it
    // before it allocates anything — see [`Session::canvas`].
    lines.push(Line::new(Record::Canvas {
        width: held.canvas.0,
        height: held.canvas.1,
    }));
    for (slot, state) in held.slots.iter().enumerate() {
        // **Material before mix, and the head slot's material is the file
        // above.** A `procedure` record for slot 0 would restate what the Set
        // file already says, and a replay obeying it would rebuild that slot
        // from addresses alone — without the params, bindings and seeds only
        // the file carries.
        if slot > 0 {
            for (layer, index, hash) in &state.nodes {
                lines.push(Line::new(Record::Procedure {
                    slot: DeckSlot(slot as u8),
                    at: NodeAddress {
                        layer: *layer,
                        index: *index,
                    },
                    proc_hash: *hash,
                }));
            }
        }
        lines.push(Line::new(Record::Gain {
            slot: DeckSlot(slot as u8),
            value: state.gain,
        }));
        lines.push(Line::new(Record::Opacity {
            slot: DeckSlot(slot as u8),
            value: state.opacity,
        }));
        lines.push(Line::new(Record::Blend {
            slot: DeckSlot(slot as u8),
            mode: state.blend.name().to_string(),
        }));
        lines.push(Line::new(Record::Residency {
            slot: DeckSlot(slot as u8),
            level: crate::mix::residency_wire_name(state.residency).to_string(),
        }));
        lines.push(Line::new(Record::Mask {
            slot: DeckSlot(slot as u8),
            kind: state.mask.kind().name().to_string(),
            angle: state.mask.angle(),
            position: state.mask.position(),
            softness: state.mask.softness(),
        }));
        lines.push(Line::new(crate::mix::transport_record(
            slot,
            &state.transport,
        )));
    }
    lines.push(Line::new(crate::mix::look_record(&held.look)));
    lines.push(Line::new(Record::MasterOut {
        value: held.master_out,
    }));
    lines.push(Line::new(Record::MasterChain(
        karakuri_store::record::Chain {
            slots: crate::mix::current_chain(&held.master_chain).slots,
        },
    )));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use karakuri_store::record::{DeckSlot, Value};

    fn set_line() -> Line {
        Line::new(Record::Set {
            id: "s".into(),
            v: 1,
        })
    }

    /// The head stops at the first tick. A `param` before it is what the Set was
    /// built with; the same record after it is an edit an operator made, and
    /// applying that one at the start would be a different performance.
    #[test]
    fn state_after_the_first_tick_is_an_edit_and_not_the_head() {
        let param = |v: f32| {
            Line::new(Record::Param {
                at: None,
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

        assert_eq!(
            session.head.len(),
            2,
            "the set and the param before the tick"
        );
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

    /// A `tick` is a terminator, not a header, and that is what decides where a
    /// frame's own measurements land.
    ///
    /// Written in the order `Live::frame` emits them — the edits an operator made,
    /// then the audio that frame heard, then the tick that closes it — every record
    /// reaches the frame it was produced during. The audio used to go out *after*
    /// the tick, which put frame N's reading in front of frame N+1: live, frame N
    /// rendered with what frame N heard; replayed, with what N−1 heard. One frame
    /// late, every frame, in the two signals every binding is driven by.
    ///
    /// What this test cannot see is the writer. `Live::frame` needs a window, so
    /// the order it pushes in is checked by reading it and this checks only that
    /// `split` honours that order once written. Swap the two pushes back and
    /// nothing here goes red — said out loud rather than left for someone to assume
    /// otherwise.
    #[test]
    fn a_frames_own_records_land_in_that_frame_and_not_the_next() {
        let audio = |energy: f32| {
            Line::new(Record::Audio {
                energy,
                onset: 0.0,
                bands: vec![energy; 4],
                confidence: 1.0,
            })
        };
        let session = split(vec![
            set_line(),
            audio(0.1),
            Line::new(Record::Tick { steps: 1 }),
            audio(0.9),
            Line::new(Record::Tick { steps: 1 }),
        ]);

        assert_eq!(session.frames.len(), 2);
        assert!(
            session.trailing.is_empty(),
            "a stream that ends on its tick leaves nothing over"
        );
        for (i, expected) in [0.1f32, 0.9].into_iter().enumerate() {
            match session.frames[i].before.as_slice() {
                [Record::Audio { energy, .. }] => assert_eq!(
                    *energy, expected,
                    "frame {i} got the audio of another frame"
                ),
                other => panic!("frame {i} carries {other:?}"),
            }
        }
    }

    /// The canvas is read out of the stream, not out of the head.
    ///
    /// It is session state, so `split` puts it in the *first frame's* edits rather
    /// than in the head — and a replay needs it strictly earlier than that, before
    /// it allocates anything. The two facts together are why [`Session::canvas`]
    /// exists instead of a field on the head.
    #[test]
    fn the_canvas_is_found_before_the_first_frame_renders() {
        let session = split(vec![
            set_line(),
            Line::new(Record::Canvas {
                width: 1920,
                height: 1080,
            }),
            Line::new(Record::Tick { steps: 1 }),
        ]);

        assert_eq!(
            session.head.len(),
            1,
            "the canvas is the session's, so the head is the set alone"
        );
        assert_eq!(session.canvas(), (Some((1920, 1080)), 0));
    }

    /// A session that never drew a frame still carries its canvas.
    ///
    /// The record is written before the first tick, so with no tick at all there is
    /// no `Frame` to hold it and it lands in `trailing` — the one place a scan over
    /// frames alone cannot see. A run closed during startup produces exactly this
    /// stream.
    #[test]
    fn a_session_with_no_tick_still_carries_its_canvas() {
        let session = split(vec![
            set_line(),
            Line::new(Record::Canvas {
                width: 1920,
                height: 1080,
            }),
        ]);
        assert!(session.frames.is_empty());
        assert_eq!(session.canvas(), (Some((1920, 1080)), 0));
    }

    /// A stream with no canvas says so rather than answering with a size.
    ///
    /// `None` and "the default" have to stay distinguishable here: the caller
    /// prints a line saying the size is a guess, and a `Session::canvas` that
    /// helpfully returned 1920x1080 would make that line unwritable.
    #[test]
    fn a_stream_without_a_canvas_has_no_opinion_about_its_size() {
        let session = split(vec![set_line(), Line::new(Record::Tick { steps: 1 })]);
        assert_eq!(session.canvas(), (None, 0));
    }

    /// Later ones are counted, not obeyed and not swallowed.
    ///
    /// This program writes exactly one, at the head — a canvas change would be a
    /// GPU reallocation mid-run. So a second one means a stream something else
    /// wrote, and the count is what lets a replay say it did not honour it.
    /// Silently taking the first would be indistinguishable from a replay that had
    /// followed every one.
    #[test]
    fn later_canvases_are_counted_rather_than_obeyed() {
        let session = split(vec![
            set_line(),
            Line::new(Record::Canvas {
                width: 1920,
                height: 1080,
            }),
            Line::new(Record::Tick { steps: 1 }),
            Line::new(Record::Canvas {
                width: 640,
                height: 480,
            }),
            Line::new(Record::Tick { steps: 1 }),
            Line::new(Record::Canvas {
                width: 800,
                height: 600,
            }),
            Line::new(Record::Tick { steps: 1 }),
        ]);

        assert_eq!(
            session.canvas(),
            (Some((1920, 1080)), 2),
            "the first is the performance's, and the other two are reported"
        );
    }

    /// Records after the last tick are kept. A session that ended between frames
    /// still recorded what the operator last did.
    #[test]
    fn records_after_the_last_tick_are_not_lost() {
        let session = split(vec![
            set_line(),
            Line::new(Record::Tick { steps: 1 }),
            Line::new(Record::Gain {
                slot: DeckSlot(0),
                value: 0.5,
            }),
        ]);
        assert_eq!(session.frames.len(), 1);
        assert_eq!(session.trailing.len(), 1);
    }

    /// The frame path allocates nothing. A batch is handed away the moment it is
    /// full and an empty one comes back, so the buffer's capacity — and therefore
    /// its pointer — never changes.
    ///
    /// A counting allocator would be the direct assertion and cannot be used here:
    /// `#[global_allocator]` is per binary and this is one. The capacity is the
    /// observable consequence, and it is not a proxy — a `Vec` that grew would
    /// report a larger one.
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
        let w = recorder.finish().expect("finish");
        assert!(w.records > 0, "nothing reached the file");
        assert_eq!(
            w.records + w.dropped_batches * BATCH as u64,
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
        let w = recorder.finish().expect("finish");
        assert_eq!(w.dropped_batches, 0, "a paced session lost batches");
        assert_eq!(w.records, (BATCH * 3) as u64);
    }

    /// The audio record is swapped, not copied, which is the whole reason it can be
    /// recorded from a frame at all.
    ///
    /// The caller's record comes back with a *different* band buffer — the shell's
    /// — and the one it had went into the stream. Pointer identity is the
    /// observable form of that: a clone would leave the caller's own buffer where
    /// it was.
    #[test]
    fn pushing_audio_takes_the_buffer_rather_than_copying_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::open(dir.path()).expect("store");
        let mut recorder = Recorder::open(&store, "s", &[]).expect("open");

        let mut mine = Record::Audio {
            energy: 0.5,
            onset: 0.0,
            bands: Vec::with_capacity(MAX_BANDS),
            confidence: 1.0,
        };
        let Record::Audio { bands, .. } = &mut mine else {
            unreachable!()
        };
        bands.extend_from_slice(&[0.1, 0.2, 0.3]);
        let was = bands.as_ptr();

        recorder.push_audio(&mut mine);

        let Record::Audio { bands, energy, .. } = &mine else {
            unreachable!()
        };
        assert_ne!(
            bands.as_ptr(),
            was,
            "the caller kept its buffer, so the record was copied rather than taken"
        );
        assert!(bands.is_empty(), "the shell left behind is not empty");
        assert_eq!(*energy, 0.0, "the shell left behind carries a measurement");

        let w = recorder.finish().expect("finish");
        assert_eq!((w.records, w.dropped_audio), (1, 0));
        // And what reached the file is what was measured, not the shell.
        let session = split(store.read_session("s").expect("read"));
        assert_eq!(session.trailing.len(), 1);
        let Record::Audio { energy, bands, .. } = &session.trailing[0] else {
            panic!("not an audio record")
        };
        assert_eq!(*energy, 0.5);
        assert_eq!(bands, &[0.1, 0.2, 0.3]);
    }

    /// Shells circulate. A frame path that ran out would allocate one per frame,
    /// which is the thing this whole arrangement exists to prevent — so the writer
    /// returns each band buffer after serialising it, and a long run never asks for
    /// a new one.
    #[test]
    fn audio_shells_come_back_from_the_writer_and_are_reused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::open(dir.path()).expect("store");
        let mut recorder = Recorder::open(&store, "s", &[]).expect("open");

        let mut mine = Record::Audio {
            energy: 0.25,
            onset: 0.0,
            bands: Vec::with_capacity(MAX_BANDS),
            confidence: 1.0,
        };
        // Far more frames than there are shells, paced so the writer keeps up.
        for i in 0..BATCH * 2 {
            if let Record::Audio { bands, .. } = &mut mine {
                bands.clear();
                bands.extend_from_slice(&[0.1, 0.2]);
            }
            recorder.push_audio(&mut mine);
            if i % 32 == 0 {
                std::thread::yield_now();
            }
        }
        let w = recorder.finish().expect("finish");
        assert_eq!(
            w.dropped_audio, 0,
            "the frame path ran out of shells, so it would have had to allocate"
        );
        assert_eq!(w.records, (BATCH * 2) as u64);
    }

    /// A `Held` for a deck of `slots`, at values nothing else in this file uses,
    /// so an assertion naming one is naming the value the head carried.
    fn held(slots: usize) -> Held {
        Held {
            canvas: (640, 360),
            look: karakuri_engine::Look {
                op: karakuri_engine::present::TonemapOp::Aces,
                exposure: 1.25,
                white_point: 4.0,
            },
            master_out: 0.75,
            master_chain: Vec::new(),
            slots: (0..slots)
                .map(|slot| SlotHeld {
                    nodes: vec![
                        (Layer::L1, 0, Hash::of(format!("l1 {slot}").as_bytes())),
                        (Layer::L4, 0, Hash::of(format!("l4a {slot}").as_bytes())),
                        (Layer::L4, 1, Hash::of(format!("l4b {slot}").as_bytes())),
                    ],
                    gain: 0.1 * slot as f32,
                    opacity: 0.5,
                    blend: Blend::Over,
                    residency: Residency::Priming,
                    mask: Mask::default(),
                    transport: Transport::default(),
                })
                .collect(),
        }
    }

    /// **The head names every slot of the deck and the chain**, which is the whole
    /// of what a session stream could not say.
    ///
    /// One Set file for slot 0 and a `procedure` record per node of every other
    /// slot — never one for slot 0, whose Set file already carries its nodes *and*
    /// the params, bindings and seeds a `procedure` record cannot. Then the mix,
    /// for every slot and not only for the ones that differ from a fresh deck: a
    /// replay that had to know the deck's defaults would be a second derivation of
    /// them.
    #[test]
    fn the_head_names_every_slot_and_the_chain() {
        let head = head(vec![set_line()], &held(3));
        let records: Vec<Record> = head.iter().map(|line| line.record().clone()).collect();

        let procedures: Vec<usize> = records
            .iter()
            .filter_map(|r| match r {
                Record::Procedure { slot, .. } => Some(slot.index()),
                _ => None,
            })
            .collect();
        assert_eq!(
            procedures,
            vec![1, 1, 1, 2, 2, 2],
            "every node of every slot but the head's, and none for the head's"
        );

        for slot in 0..3 {
            let about: Vec<&Record> = records
                .iter()
                .filter(|r| match r {
                    Record::Gain { slot: s, .. }
                    | Record::Opacity { slot: s, .. }
                    | Record::Blend { slot: s, .. }
                    | Record::Residency { slot: s, .. }
                    | Record::Mask { slot: s, .. }
                    | Record::Transport { slot: s, .. } => s.index() == slot,
                    _ => false,
                })
                .collect();
            assert_eq!(
                about.len(),
                6,
                "slot {slot}: the six the deck holds per slot, always and not only \
                 where they differ from a fresh deck"
            );
        }

        assert!(
            records.contains(&Record::Canvas {
                width: 640,
                height: 360
            }),
            "no canvas: a replay reads it before it allocates anything"
        );
        assert!(
            records.contains(&Record::MasterOut { value: 0.75 }),
            "no level at the chain's entry"
        );
        assert!(
            records.iter().any(|r| matches!(r, Record::MasterChain(_))),
            "no `master_chain`: a Set file carries nothing for the chain, so the head \
             is the only thing that can put it back"
        );
        assert!(
            records.iter().any(|r| matches!(r, Record::Look { .. })),
            "no look"
        );
    }

    /// A head goes through [`split`] as a head: the Set file's records on one side,
    /// the deck's on the other, and no frame in between.
    #[test]
    fn a_head_round_trips_through_split() {
        let written = head(vec![set_line()], &held(2));
        let deck_records = written.len() - 1;
        let session = split(written);

        assert_eq!(
            session.head.len(),
            1,
            "the material is the Set file's records and nothing else"
        );
        assert_eq!(
            session.opening.len(),
            deck_records,
            "every record the head wrote about the deck is in `opening`"
        );
        assert!(session.frames.is_empty(), "a head is not a frame");
        assert!(session.trailing.is_empty());
        assert_eq!(
            session.canvas(),
            (Some((640, 360)), 0),
            "the canvas is read out of the head"
        );
    }

    /// **A measurement before the first tick belongs to the first frame**, not to
    /// the head.
    ///
    /// A frame writes its edits, then what it heard, then the tick that closes it —
    /// so the `audio` line in front of the very first tick is frame 0's
    /// measurement. Sorting the head by position alone would move it into the head
    /// and replay frame 0 at what the bus invents rather than at what the room
    /// heard.
    #[test]
    fn the_first_frames_measurement_is_not_the_heads() {
        let session = split(vec![
            set_line(),
            Line::new(Record::Gain {
                slot: DeckSlot(0),
                value: 0.5,
            }),
            Line::new(Record::Audio {
                energy: 0.4,
                onset: 0.0,
                bands: vec![0.1; 4],
                confidence: 1.0,
            }),
            Line::new(Record::Tick { steps: 1 }),
        ]);
        assert_eq!(session.head.len(), 1, "the Set file's record");
        assert_eq!(session.opening.len(), 1, "the deck's gain");
        assert_eq!(
            session.frames[0].before.len(),
            1,
            "the measurement stayed with the frame it was measured in"
        );
        assert!(matches!(session.frames[0].before[0], Record::Audio { .. }));
    }

    /// **Every program that opens a recorder writes its head through [`head`].**
    ///
    /// A recorder takes a head and appends nothing to it afterwards, so whatever
    /// is handed to [`Recorder::open`] *is* the head of that stream. Two surfaces
    /// record sessions — `karakuri-cli`'s `--record-session` and the console's
    /// `rec` pill — and what a head has to say is the same sentence for both: what
    /// every slot of the deck held. A second assembly of that sentence is a second
    /// answer to it, which is what this file's two writers used to be.
    ///
    /// **A source scan, because the round trip it would rather be cannot be run.**
    /// Both writers need a deck and a deck needs a device; `karakuri-cli`'s live
    /// path needs a *window* on top of that, which is why `--record-session` is
    /// refused alongside `--render`, `--seq` and `--replay`. So the far end of the
    /// claim is checked where it can be — `karakuri-cli/tests/replay.rs` drives a
    /// hand-written two-slot head through the binary — and this is the near end.
    ///
    /// What it cannot see is a writer that calls [`head`] and then appends records
    /// of its own. It is coarse in the safe direction: the failure it refuses is
    /// the one that already happened.
    #[test]
    fn every_recorder_is_opened_over_a_head_this_module_wrote() {
        fn walk(dir: &std::path::Path, into: &mut Vec<std::path::PathBuf>) {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, into);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    into.push(path);
                }
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("workspace root");
        let mut files = Vec::new();
        walk(&root.join("crates"), &mut files);
        // This module's own tests open recorders over heads written by hand, to
        // check the recorder rather than the head.
        let defines = root.join("crates/karakuri-environment/src/session.rs");
        let mut opened = 0;
        for path in files {
            if path == defines {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("read source");
            if !text.contains("Recorder::open(") {
                continue;
            }
            opened += 1;
            assert!(
                text.contains("session::head("),
                "{}: opens a session recorder over a head it assembled itself — a head \
                 says what the deck held, and `session::head` is the one place that \
                 sentence is spelled",
                path.display()
            );
        }
        assert_eq!(
            opened, 2,
            "expected the two surfaces that record a session — `karakuri-cli`'s \
             `--record-session` and the console's `rec` pill. A third is welcome and owes \
             this count a line; fewer means the scan has stopped finding either"
        );
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
        let w = recorder.finish().expect("finish");
        assert_eq!((w.records, w.dropped_batches), (4, 0));

        let session = split(store.read_session("s").expect("read"));
        assert_eq!(session.head.len(), 1);
        assert_eq!(
            session.frames.iter().map(|f| f.steps).collect::<Vec<_>>(),
            vec![1, 2, 1, 4]
        );
    }
}
