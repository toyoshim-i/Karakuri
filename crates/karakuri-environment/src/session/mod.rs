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
    /// What the slot's MCP policy was set to (`auto`, `on`, `off`).
    pub policy: karakuri_operation::SlotPolicy,
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
        lines.push(Line::new(Record::Policy {
            slot: DeckSlot(slot as u8),
            policy: state.policy.name().to_string(),
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
mod tests;
