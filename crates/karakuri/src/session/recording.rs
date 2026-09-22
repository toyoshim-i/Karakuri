use crate::{deck_letter, Engine, Save};
use karakuri_console::view;
use karakuri_environment::{session, setfile};
use karakuri_store::store::Store;

use super::keeping::Keeping;
use super::HEAD_SLOT;

/// One open recording: the writer, and the id it is filing under.
///
/// The id is kept because it is what every sentence about this recording names
/// and what `--replay` will be typed with, and because [`Sessions`] hands the
/// recorder away to a thread when it stops — after which the id is the only
/// thing left to say the sentence with.
struct Stream {
    id: String,
    recorder: session::Recorder,
}

/// What a thread that opened or closed a recording came back with.
///
/// One channel for both because they are the same kind of answer: a press asked
/// for something slow, the frame did not wait, and this is what happened. It is
/// [`Saved`]'s shape one control along.
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

/// The session recorder this window holds, and the two presses that move it.
///
/// # A press starts one and a press stops one
///
/// `docs/manual/console.html` draws one capsule at the end of the transport row
/// and the operations page gives it one row, so it is one control with two ends
/// — [`karakuri_console::view::TransportRow::record`], which reads the pill's
/// own state to say which end a press is. Nothing here decides that a second
/// time.
///
/// # Neither end happens on the frame, and that is the whole of this type
///
/// A start creates two files and spawns a thread. The head a replay
/// reconstructs a session from is a Set file, so beginning one writes that
/// file, reads it back, opens `sessions/<id>.ndjson` and starts a writer —
/// which is `karakuri-cli`'s own sentence about the same call, *"opened before
/// the first frame and never on one"*.
///
/// A stop blocks on that writer. [`session::Recorder::finish`] hands the last
/// batch over and joins the thread, and so does `Drop` — so a recorder let go
/// of on the frame path stalls the frame just as surely as one that was
/// finished there. `karakuri-cli` finishes in `exiting`, where a stall is free;
/// a press is not that place
/// ([P-0094](../../../../docs/principles/0094-a-panel-that-lies-is-worse-than-a-panel-that-is-plain.md)).
///
/// So both ends go to a thread of their own and the outcome comes back over
/// [`Sessions::done`], said at the frame it arrives — which is
/// [`Keeping::save_set`]'s arrangement exactly, and for the same reason: this
/// is the second thing in this program a press asks for that a disk answers.
///
/// # What the pill reads while a thread is out
///
/// Nothing is being recorded until the recorder exists, and the pill says so:
/// [`Sessions::rec`] is `Running` only while [`Sessions::open`] holds a writer.
/// A start that is still opening reads `rec`, and a stop that is still flushing
/// reads `rec` too — the stream stopped taking records the instant the recorder
/// left, and the tail is being written by a thread nobody is waiting for. A
/// third state on the pill would be this panel drawing a promise instead of a
/// fact.
///
/// A second press while a thread is out is refused and says so, rather than
/// opening a second recorder or joining a queue: two recorders would be two
/// writers over one deck, and a queued press is a gesture whose effect arrives
/// after the operator has stopped looking at it.
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

    /// What the `rec` pill reads this frame.
    ///
    /// Always a value and never `None` on this side: this program holds a store, so
    /// *whether a recording is running* is a question it can always answer. `None`
    /// is the console's word for *nobody said*, and it is what a console with no
    /// program behind it draws — see `karakuri_console::view::Transport::rec`.
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

    /// A press on the `rec` pill, performed.
    ///
    /// The reading of the deck happens here and every byte of I/O happens on a
    /// thread, which is [`Keeping::save_set`]'s division and its reason: what is
    /// above the spawn is values already in memory.
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

    /// End the one running, and hand the flush to a thread.
    ///
    /// The recorder is moved rather than borrowed, which is the point: it blocks on
    /// its writer in `Drop` as well as in [`session::Recorder::finish`], so a
    /// recorder still owned by this frame is a frame that can still be stalled by a
    /// disk.
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
                        // **Named apart, which is `karakuri-cli`'s own
                        // reading**: a lost batch is a second of everything
                        // and a lost audio frame is one frame's measurement,
                        // and an operator deciding what to do about a stream
                        // needs to know which they have.
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

    /// The recording still open when the window closes, flushed here.
    ///
    /// [`Keeping::awaited_saves`]' moment and its argument: a frame owes a disk
    /// nothing, and the end of the run is the one place where that is the wrong
    /// trade — a session left to `Drop` would still be flushed, because the
    /// recorder ends its writer either way, but nothing would say what was written
    /// or what was lost. This is where a stall is free, which is `karakuri-cli`'s
    /// `exiting` said on this side.
    ///
    /// A stop already in flight is waited for by the same call, because the thread
    /// it is on is what holds the recorder.
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

/// One node of a running Set as a `procedure` record addresses it: which layer,
/// which index on it, and the store address of its source.
///
/// `None` for a node on a layer the record vocabulary has no word for, which is
/// [`karakuri_environment::meta::layer_named`]'s answer and not a second one —
/// a head that invented a layer would be a head a replay refuses.
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

/// Everything a start gathers on the frame, for the thread to write.
///
/// Named for the press rather than for the head's second half, because
/// `karakuri_environment::Opening` is a different thing this crate uses a lot.
/// Three parts because a head has three: the head slot's Set file, the bytes
/// every *other* slot's address has to resolve at, and what the deck held.
/// Nothing here borrows and nothing here is I/O — [`Sessions::begin`]'s whole
/// division.
pub(crate) struct Starting {
    /// The head slot's Set file, written and read back by [`began`].
    pub(crate) material: Save,
    /// Every other slot's sources, so that the `procedure` records naming them
    /// resolve. Slot 0's ride along inside `material`.
    ///
    /// A slot the run has no addresses for contributes nothing, which is
    /// [`Playing::at`]'s `None`: a build whose sources never reached the store has
    /// no address until the next one lands.
    pub(crate) sources: Vec<setfile::Sources>,
    /// What the deck held at the press, read off the deck.
    pub(crate) held: session::Held,
}

/// A recording, opened: the material written, every slot's sources stored, the
/// head assembled and a writer started over it.
///
/// A free function because every line of it is on the thread
/// [`Sessions::begin`] spawned, and none of it may be reachable from a frame.
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
    // **Every other slot's bytes before the head that names them.** A
    // `procedure` record is an address, and a replay refuses a slot whose
    // address the store cannot resolve — so a head written over sources that
    // did not land would be a recording that cannot be played back, which is
    // the one failure this start must not report as a success.
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
