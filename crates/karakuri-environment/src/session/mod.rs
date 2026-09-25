//! Session stream recording and replay.
//!
//! Captures timeline events (ticks, audio, mix controls, Set heads) with non-blocking
//! batching on the render thread and asynchronous disk serialization.

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
        // Shell pool covers worst-case active batch plus queue capacity.
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

    /// Records an audio measurement by swapping buffers to avoid allocation.
    ///
    /// Increments `dropped_audio` if no preallocated shell is currently available.
    pub fn push_audio(&mut self, record: &mut Record) {
        match self.audio_shells.try_recv() {
            Ok(shell) => {
                let filled = std::mem::replace(record, shell);
                self.push(filled);
            }
            Err(_) => self.dropped_audio += 1,
        }
    }

    /// Dispatches the current batch to the writer thread and acquires a spare buffer.
    ///
    /// Increments `dropped_batches` if the writer channel is full or disconnected.
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
    /// Initial Set state records for the primary slot.
    pub head: Vec<Line>,
    /// Deck configuration and state records applied before frame 0.
    pub opening: Vec<Record>,
    /// One entry per `tick`: what to apply *before* that frame, and how many steps
    /// the frame advances.
    pub frames: Vec<Frame>,
    /// Records after the last tick. A session that ended between frames has them,
    /// and dropping them silently would lose the last thing an operator did.
    pub trailing: Vec<Record>,
}

impl Session {
    /// Returns the initial canvas dimensions and the count of any subsequent duplicate declarations.
    pub fn canvas(&self) -> (Option<(u32, u32)>, usize) {
        let mut found = None;
        let mut extra = 0;
        // Check opening, per-frame, and trailing records to discover canvas dimensions.
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

/// Partitions a session NDJSON stream into head Set records, opening mix records, and frames.
///
/// Records prior to the first `Tick` are split into Set definitions and opening deck states.
/// Subsequent records are grouped into per-frame slices demarcated by `Tick`.
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

/// Initial state and mix parameters of a single deck slot at recording start.
#[derive(Debug, Clone)]
pub struct SlotHeld {
    /// Set procedures running in this slot as `(layer, index, source_hash)`.
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

/// Constructs the session head stream combining initial Set records with deck state records.
pub fn head(material: Vec<Line>, held: &Held) -> Vec<Line> {
    let mut lines = material;
    // Canvas dimensions are written first among the deck records.
    lines.push(Line::new(Record::Canvas {
        width: held.canvas.0,
        height: held.canvas.1,
    }));
    for (slot, state) in held.slots.iter().enumerate() {
        // Slot 0 is defined by the preceding Set material; other slots emit procedure records.
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
