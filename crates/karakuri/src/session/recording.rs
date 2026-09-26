use crate::{deck_letter, Engine, Save};
use karakuri_console::view;
use karakuri_environment::{session, setfile};
use karakuri_store::store::Store;

use super::keeping::Keeping;
use super::HEAD_SLOT;

/// Active session recording stream with its session identifier and writer.
struct Stream {
    id: String,
    recorder: session::Recorder,
}

/// Completion outcome from background recording open or close tasks.
enum Ended {
    /// A recorder that opened, with the id it is filing under and how many records
    /// of material are at its head.
    Began {
        id: String,
        head: usize,
        recorder: session::Recorder,
    },
    /// A start that never opened, in the words it failed with. Nothing is
    /// half-started: the pill goes back to reading `rec` because nothing is being
    /// recorded, which is the truth.
    Failed(String),
    /// A recording flushed and closed, and what the writer made of it.
    Finished {
        id: String,
        written: Result<session::Written, String>,
    },
}

/// Session recording coordinator managing asynchronous start and stop operations (ADR-0289, P-0094).
///
/// Dispatches start and stop I/O to background threads so render frames never stall.
pub(crate) struct Sessions {
    /// The recorder, and `None` whenever nothing is being recorded — which includes
    /// both sides of a start that is still opening.
    open: Option<Stream>,
    /// Whether a thread is out, which is what makes a second press a refusal. One
    /// flag for both ends because there is at most one thread and what it is doing
    /// does not change the answer.
    working: bool,
    /// Where a thread's outcome comes back, and the sending half it is given a
    /// clone of.
    done: std::sync::mpsc::Receiver<Ended>,
    tx: std::sync::mpsc::Sender<Ended>,
}

impl Sessions {
    pub(crate) fn new() -> Sessions {
        let (tx, done) = std::sync::mpsc::channel();
        Sessions {
            open: None,
            working: false,
            done,
            tx,
        }
    }

    /// Returns the current recording state for the transport row pill (`Running` or `Idle`).
    pub(crate) fn rec(&self) -> view::Rec {
        match self.open {
            Some(_) => view::Rec::Running,
            None => view::Rec::Idle,
        }
    }

    /// The open recorder, for the frame path to push into.
    ///
    /// `None` for the whole of a run nobody pressed the pill on, which is most runs
    /// and costs one branch.
    pub(crate) fn recorder(&mut self) -> Option<&mut session::Recorder> {
        self.open.as_mut().map(|stream| &mut stream.recorder)
    }

    /// Handles a transport record button interaction, delegating I/O to worker threads.
    pub(crate) fn asked(
        &mut self,
        keeping: &Keeping,
        engine: &Engine,
        root: &std::path::Path,
        recording: &karakuri_operation::Recording,
    ) {
        if self.working {
            return println!(
                "  rec: a recording is still being opened or closed — the press is refused \
                 rather than queued, because a gesture whose effect lands after you have \
                 stopped looking at it is worse than one that says no"
            );
        }
        match recording {
            karakuri_operation::Recording::Start { id } => self.begin(keeping, engine, root, id),
            karakuri_operation::Recording::Stop => self.end(),
        }
    }

    /// Starts a session recording under a unique timestamped identifier (ADR-0289, Principle 0094).
    fn begin(
        &mut self,
        keeping: &Keeping,
        engine: &Engine,
        root: &std::path::Path,
        id: &Option<String>,
    ) {
        // **A capsule types no name and this is the only route there is**, so
        // a payload naming one would be a press this program cannot make. It
        // is matched rather than ignored: the day a route that can name one
        // arrives, this is the line that has to say what it means.
        if let Some(named) = id {
            return println!(
                "  rec: `{named}` — nothing here can name a recording, and the id is a stamp \
                 so that a second start cannot land in a stream that already exists"
            );
        }
        let id = karakuri_environment::history::stamped_id();
        let starting = match keeping.head_opening(engine, root, &id) {
            Ok(starting) => starting,
            Err(why) => return println!("  rec: {why}"),
        };
        println!(
            "  rec: opening `{id}` — its head says what all {} decks held, deck {} in full as a \
             Set file and the rest by the addresses they are playing, so `--replay {id}` will \
             need nothing else. A replay starts that material from the top: a Set file says what \
             is playing and at what values and carries no running state, so material that \
             accumulates begins again rather than continuing the picture on screen now. Its \
             ticks carry the step count measured between one frame and the last, capped at four, \
             so the replay runs at the speed this run ran and not at the speed the machine \
             playing it draws",
            starting.held.slots.len(),
            deck_letter(HEAD_SLOT as u8)
        );
        let root = root.to_path_buf();
        let tx = self.tx.clone();
        self.working = true;
        // **A thread, and detached**: no frame waits for it. Everything below
        // this line is a store being opened, two files being written and one
        // being read back.
        std::thread::spawn(move || {
            let _ = tx.send(began(root, id, starting));
        });
    }

    /// Stops the active recording, offloading writer flush and join to a background thread.
    fn end(&mut self) {
        let Some(Stream { id, recorder }) = self.open.take() else {
            return println!("  rec: nothing is being recorded");
        };
        println!("  rec: `{id}` stopped — the last records are being flushed");
        let tx = self.tx.clone();
        self.working = true;
        std::thread::spawn(move || {
            let written = recorder.finish();
            let _ = tx.send(Ended::Finished { id, written });
        });
    }

    /// Every start and stop that has landed since the last frame, said.
    ///
    /// Drained and never waited on, which is [`Keeping::finished_saves`]' rule: a
    /// frame owes the display a picture and owes a disk nothing.
    pub(crate) fn finished(&mut self) {
        while let Ok(ended) = self.done.try_recv() {
            self.took(ended);
        }
    }

    /// One thread's outcome, said. The frame's drain and the quit's wait are two
    /// ways of *getting* one and this is the one place either acts on it.
    fn took(&mut self, ended: Ended) {
        self.working = false;
        {
            match ended {
                Ended::Began { id, head, recorder } => {
                    println!(
                        "  rec: `{id}` open — {head} record{} of material at its head",
                        match head {
                            1 => "",
                            _ => "s",
                        }
                    );
                    self.open = Some(Stream { id, recorder });
                }
                // **Said and nothing claims otherwise**, which is the shape a
                // save's failure already takes here: a program saying a
                // recording started when the disk refused is the lie this
                // codebase is arranged against.
                Ended::Failed(why) => println!("  rec: {why}"),
                Ended::Finished { id, written } => match written {
                    Ok(w) => {
                        println!("  rec: `{id}` written — {} records", w.records);
                        // Report dropped stream batches and audio frames separately.
                        if w.dropped_batches > 0 {
                            println!(
                                "    {} batch{} lost because the disk could not keep up — the \
                                 stream has gaps",
                                w.dropped_batches,
                                match w.dropped_batches {
                                    1 => "",
                                    _ => "es",
                                }
                            );
                        }
                        if w.dropped_audio > 0 {
                            println!(
                                "    {} frame{} of audio not recorded — those frames replay at \
                                 what the bus invents rather than at what the room heard",
                                w.dropped_audio,
                                match w.dropped_audio {
                                    1 => "",
                                    _ => "s",
                                }
                            );
                        }
                    }
                    Err(why) => println!("  rec: `{id}` — {why}"),
                },
            }
        }
    }

    /// Synchronously awaits completion and flush of any active or finalizing recording on shutdown.
    pub(crate) fn awaited(&mut self) {
        self.finished();
        if self.open.is_some() {
            self.end();
        }
        // Blocking, unlike [`Sessions::finished`]: this is the end of the run.
        // Every sender being gone is the other way out, and it means the
        // thread died without answering — which is nothing left to wait for.
        while self.working {
            let Ok(ended) = self.done.recv() else {
                return;
            };
            self.took(ended);
        }
    }
}

/// Resolves a running set node's layer, index, and content hash for header recording.
pub(crate) fn addressed(
    node: &setfile::SavedNode,
) -> Option<(
    karakuri_store::record::Layer,
    u32,
    karakuri_store::hash::Hash,
)> {
    let kind = karakuri_environment::meta::layer_named(node.layer)?;
    Some((
        karakuri_environment::meta::layer_of(kind),
        node.index,
        node.hash,
    ))
}

/// Pre-gathered state from the current frame required to initialize a recording session.
pub(crate) struct Starting {
    /// The head slot's Set file, written and read back by [`began`].
    pub(crate) material: Save,
    /// Source definitions for non-head slots so procedure records resolve.
    pub(crate) sources: Vec<setfile::Sources>,
    /// What the deck held at the press, read off the deck.
    pub(crate) held: session::Held,
}

/// Worker function that writes session material, stores node sources, and opens the recording stream.
fn began(root: std::path::PathBuf, id: String, starting: Starting) -> Ended {
    let Starting {
        material,
        sources,
        held,
    } = starting;
    let name = material.id.clone();
    if let Err(why) = material.run() {
        return Ended::Failed(format!(
            "`{id}` was not opened — writing its material: {why}"
        ));
    }
    let store = match Store::open(&root) {
        Ok(store) => store,
        Err(e) => {
            return Ended::Failed(format!(
                "`{id}` was not opened — store `{}`: {e}",
                root.display()
            ))
        }
    };
    // Ensure all referenced node sources are written to the store before committing the head.
    for (slot, sources) in sources.into_iter().enumerate() {
        if let Err(why) = sources.into_nodes(&store) {
            return Ended::Failed(format!(
                "`{id}` was not opened — storing what deck {} is playing: {why}",
                deck_letter(slot as u8)
            ));
        }
    }
    // **Read back rather than kept**, which is `karakuri-cli`'s own route to a
    // head: the writer is what decides the lines a Set file is, so a head
    // assembled here would be a second spelling of that format.
    let material = match store.read_set(&name) {
        Ok(head) => head,
        Err(e) => {
            return Ended::Failed(format!(
                "`{id}` was not opened — reading back its material `{name}`: {e}"
            ))
        }
    };
    // **The one function either writer spells a head with.** `karakuri-cli`
    // calls this same one over its own deck reading, so the two programs cannot
    // write two shapes of head.
    let head = session::head(material, &held);
    match session::Recorder::open(&store, &id, &head) {
        Ok(recorder) => Ended::Began {
            id,
            head: head.len(),
            recorder,
        },
        Err(why) => Ended::Failed(format!("`{id}` was not opened — {why}")),
    }
}
