//! The session recorder's start/stop/poll state machine, and the save,
//! keep-procedure and rewire request queues an operator's key presses and an
//! MCP client's requests both land in.
//!
//! Both halves end on a disk or a watcher rather than on the frame that asked,
//! so both are the same shape: gather what is known in memory, hand the slow
//! part to a thread of its own, and read the outcome back at whichever frame
//! it arrives on. [`Sessions`] is the first of those and [`Keeping`] is the
//! second, and they are one module because `main.rs`'s own [`KeyCtx`] reaches
//! both from the same key press.
//!
//! [`KeyCtx`]: crate::KeyCtx

use crate::{
    built_nodes, copied, deck_letter, ir_layer, node_addr, playing_values, refused, slot_in_range,
    Aiming, Engine, Kept, Playing, Save, Saved, Sent, Sources,
};
use karakuri_console::view;
use karakuri_engine::set::Layering;
use karakuri_engine::{Gpu, HotSwap, Set, DEFAULT_BUDGET_MS};
use karakuri_environment::{history, mcp, session, setfile, watch, Asked};
use karakuri_store::store::Store;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Recording the session
// ---------------------------------------------------------------------------

/// **Which deck slot's material a session's head describes.**
///
/// `karakuri-cli`'s `session_head` says why there is a number here at all: *"a
/// session stream cannot say what a deck held"*, so a head describes one Set
/// and the slots beside it will not replay. That program takes slot 0 and says
/// so out loud, and this takes the same one for the same reason — a replay
/// drives slot 0, and a head written from whichever deck happened to be
/// selected would make *which slot replays* depend on where a hand was.
const HEAD_SLOT: usize = 0;

/// **One open recording**: the writer, and the id it is filing under.
///
/// The id is kept because it is what every sentence about this recording names
/// and what `--replay` will be typed with, and because [`Sessions`] hands the
/// recorder away to a thread when it stops — after which the id is the only
/// thing left to say the sentence with.
struct Stream {
    id: String,
    recorder: session::Recorder,
}

/// **What a thread that opened or closed a recording came back with.**
///
/// One channel for both because they are the same kind of answer: a press
/// asked for something slow, the frame did not wait, and this is what happened.
/// It is [`Saved`]'s shape one control along.
enum Ended {
    /// A recorder that opened, with the id it is filing under and how many
    /// records of material are at its head.
    Began {
        id: String,
        head: usize,
        recorder: session::Recorder,
    },
    /// A start that never opened, in the words it failed with. **Nothing is
    /// half-started**: the pill goes back to reading `rec` because nothing is
    /// being recorded, which is the truth.
    Failed(String),
    /// A recording flushed and closed, and what the writer made of it.
    Finished {
        id: String,
        written: Result<session::Written, String>,
    },
}

/// **The session recorder this window holds, and the two presses that move
/// it.**
///
/// # A press starts one and a press stops one
///
/// `docs/manual/console.html` draws one capsule at the end of the transport
/// row and the operations page gives it one row, so it is one control with two
/// ends — [`karakuri_console::view::TransportRow::record`], which reads the
/// pill's own state to say which end a press is. Nothing here decides that a
/// second time.
///
/// # Neither end happens on the frame, and that is the whole of this type
///
/// **A start creates two files and spawns a thread.** The head a replay
/// reconstructs a session from is a Set file, so beginning one writes that
/// file, reads it back, opens `sessions/<id>.ndjson` and starts a writer —
/// which is `karakuri-cli`'s own sentence about the same call, *"opened before
/// the first frame and never on one"*.
///
/// **A stop blocks on that writer.** [`session::Recorder::finish`] hands the
/// last batch over and joins the thread, and so does `Drop` — so a recorder
/// let go of on the frame path stalls the frame just as surely as one that was
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
/// **Nothing is being recorded until the recorder exists**, and the pill says
/// so: [`Sessions::rec`] is `Running` only while [`Sessions::open`] holds a
/// writer. A start that is still opening reads `rec`, and a stop that is still
/// flushing reads `rec` too — the stream stopped taking records the instant
/// the recorder left, and the tail is being written by a thread nobody is
/// waiting for. A third state on the pill would be this panel drawing a
/// promise instead of a fact.
///
/// **A second press while a thread is out is refused and says so**, rather
/// than opening a second recorder or joining a queue: two recorders would be
/// two writers over one deck, and a queued press is a gesture whose effect
/// arrives after the operator has stopped looking at it.
pub(crate) struct Sessions {
    /// The recorder, and `None` whenever nothing is being recorded — which
    /// includes both sides of a start that is still opening.
    open: Option<Stream>,
    /// **Whether a thread is out**, which is what makes a second press a
    /// refusal. One flag for both ends because there is at most one thread and
    /// what it is doing does not change the answer.
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

    /// **What the `rec` pill reads this frame.**
    ///
    /// Always a value and never `None` on this side: this program holds a
    /// store, so *whether a recording is running* is a question it can always
    /// answer. `None` is the console's word for *nobody said*, and it is what
    /// a console with no program behind it draws — see
    /// `karakuri_console::view::Transport::rec`.
    pub(crate) fn rec(&self) -> view::Rec {
        match self.open {
            Some(_) => view::Rec::Running,
            None => view::Rec::Idle,
        }
    }

    /// **The open recorder, for the frame path to push into.**
    ///
    /// `None` for the whole of a run nobody pressed the pill on, which is most
    /// runs and costs one branch.
    pub(crate) fn recorder(&mut self) -> Option<&mut session::Recorder> {
        self.open.as_mut().map(|stream| &mut stream.recorder)
    }

    /// **A press on the `rec` pill, performed.**
    ///
    /// The reading of the deck happens here and every byte of I/O happens on a
    /// thread, which is [`Keeping::save_set`]'s division and its reason: what
    /// is above the spawn is values already in memory.
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

    /// **Begin one**, under a stamp.
    ///
    /// # The id is a stamp and each start takes a fresh one
    ///
    /// `karakuri_environment::history::stamped_id` is this repository's
    /// convention for something an operator looks for by *when they made it*,
    /// and it is what a keep with no typed name already files under. A capsule
    /// types no name, so a press passes `None` and this is what `None` means.
    ///
    /// **It is also what keeps a second recording from destroying the first.**
    /// `Store::append_session` appends and `session::split` sets `started` at
    /// the first tick and never clears it, so a second head written under an id
    /// that already has a stream lands in the middle of it and is read back as
    /// edits. A fresh id per start is what makes that unreachable rather than
    /// merely unlikely — ADR-0289.
    ///
    /// # The head is written from the live deck, and a replay starts from the
    /// top
    ///
    /// A Set file names the material and the values it is holding, which is
    /// exactly what [`playing_values`] reads off the running Set — so a head
    /// at an arbitrary frame is producible, and this produces one.
    ///
    /// **What it is not is a resume.** A Set file carries no running state: an
    /// accumulating renderer's picture is what it has accumulated, and
    /// `docs/manual/console.html` says of one shipped deck that its material
    /// *accumulates*. So a replay of a recording begun mid-performance
    /// restarts that material from the top rather than continuing the picture
    /// that was on screen when the press happened. **It is said out loud at
    /// the start** rather than left for whoever plays the file back to
    /// discover, which is the whole of P-0094 applied to a sentence instead of
    /// to a pixel.
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
        let material = match keeping.head_material(engine, root, &id) {
            Ok(material) => material,
            Err(why) => return println!("  rec: {why}"),
        };
        println!(
            "  rec: opening `{id}` — its head is deck {}'s material as it stands, so \
             `--replay {id}` will need nothing else. A replay starts that material from the \
             top: a Set file says what is playing and at what values and carries no running \
             state, so material that accumulates begins again rather than continuing the picture \
             on screen now. Its ticks carry the step count measured between one frame and the \
             last, capped at four, so the replay runs at the speed this run ran and not at the \
             speed the machine playing it draws",
            deck_letter(HEAD_SLOT as u8)
        );
        let root = root.to_path_buf();
        let tx = self.tx.clone();
        self.working = true;
        // **A thread, and detached**: no frame waits for it. Everything below
        // this line is a store being opened, two files being written and one
        // being read back.
        std::thread::spawn(move || {
            let _ = tx.send(began(root, id, material));
        });
    }

    /// **End the one running**, and hand the flush to a thread.
    ///
    /// The recorder is **moved** rather than borrowed, which is the point: it
    /// blocks on its writer in `Drop` as well as in
    /// [`session::Recorder::finish`], so a recorder still owned by this frame
    /// is a frame that can still be stalled by a disk.
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

    /// **Every start and stop that has landed since the last frame, said.**
    ///
    /// Drained and never waited on, which is [`Keeping::finished_saves`]'
    /// rule: a frame owes the display a picture and owes a disk nothing.
    pub(crate) fn finished(&mut self) {
        while let Ok(ended) = self.done.try_recv() {
            self.took(ended);
        }
    }

    /// One thread's outcome, said. The frame's drain and the quit's wait are
    /// two ways of *getting* one and this is the one place either acts on it.
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

    /// **The recording still open when the window closes, flushed here.**
    ///
    /// [`Keeping::awaited_saves`]' moment and its argument: a frame owes a
    /// disk nothing, and the end of the run is the one place where that is the
    /// wrong trade — a session left to `Drop` would still be flushed, because
    /// the recorder ends its writer either way, but nothing would say what was
    /// written or what was lost. **This is where a stall is free**, which is
    /// `karakuri-cli`'s `exiting` said on this side.
    ///
    /// A stop already in flight is waited for by the same call, because the
    /// thread it is on is what holds the recorder.
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

/// **A recording, opened**: the material written, read back, and a writer
/// started over it.
///
/// A free function because every line of it is on the thread
/// [`Sessions::begin`] spawned, and none of it may be reachable from a frame.
fn began(root: std::path::PathBuf, id: String, material: Save) -> Ended {
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
    // **Read back rather than kept**, which is `karakuri-cli`'s own route to a
    // head: the writer is what decides the lines a Set file is, so a head
    // assembled here would be a second spelling of that format.
    let head = match store.read_set(&name) {
        Ok(head) => head,
        Err(e) => {
            return Ended::Failed(format!(
                "`{id}` was not opened — reading back its material `{name}`: {e}"
            ))
        }
    };
    match session::Recorder::open(&store, &id, &head) {
        Ok(recorder) => Ended::Began {
            id,
            head: head.len(),
            recorder,
        },
        Err(why) => Ended::Failed(format!("`{id}` was not opened — {why}")),
    }
}

/// **Every edge a client asked for on one frame, applied to the run's wiring and
/// answered.**
///
/// This is `karakuri_environment::mcp::WireRequest`'s three points, and it is a
/// free function so that all three are checkable without a window, a GPU or a
/// `Deck` — the wiring, the re-aim and the sentence are the whole of what this
/// decides, and none of them needs one. `karakuri-cli`'s `rewired` is the same
/// three decisions for the same reasons; it is restated rather than called for
/// [`number_for`]'s reason.
///
/// # Replace, keyed on the input
///
/// An edge is dropped and the new one appended, keyed on `(node, slot)` — the
/// node that declares the input and what its procedure calls it. It is forced
/// rather than chosen: `SetError::SlotBoundTwice` refuses two edges on one input
/// where the Set is built, so an append would make the *second* call on an input
/// a refusal and leave a model unable to change its mind.
///
/// **The key does not include the deck slot, because the run's wiring does
/// not.** [`App::edges`] is one list for the whole run and an edge naming a node
/// a Set has not got is passed over where the Set is built. So a request names a
/// deck slot to say *which slot rebuilds*, and two slots holding a node of the
/// same name share one entry in this list.
///
/// # A slot this deck does not hold
///
/// **Refused, in [`karakuri_environment::no_such_slot`]'s words, and nothing is
/// rewired** — the decision [`App::save_set`] already makes and for its reason:
/// the server checks the number against its own `Slots` before it sends, and
/// this is the guard that does not depend on it having.
///
/// # The same input wired twice on one frame
///
/// **Every request is applied, in the order it arrived, and the last one is what
/// the run is wired with.** One aim per slot goes out after all of them are in
/// the list, so the rebuild carries the settled wiring rather than an
/// intermediate one. **A request the same frame overwrote is told so**: its edge
/// *was* written and then replaced, and a reply saying only "wired" would be a
/// true sentence about a state the run no longer holds.
pub(crate) fn rewired(
    asked: &[(usize, karakuri_engine::set::Edge)],
    edges: &mut Vec<karakuri_engine::set::Edge>,
    aims: &mut [Aiming],
    slot_count: usize,
) -> Vec<Result<String, String>> {
    use std::fmt::Write as _;
    // **Every edge into the list before any watcher is re-aimed**, so that a
    // frame carrying two of them rebuilds once, at the wiring the frame ended
    // with.
    let mut said: Vec<Option<Result<String, String>>> = asked.iter().map(|_| None).collect();
    let mut named: Vec<usize> = Vec::new();
    for (at, (slot, edge)) in asked.iter().enumerate() {
        if !slot_in_range(*slot, slot_count) {
            said[at] = Some(Err(format!(
                "{}, and nothing was rewired",
                karakuri_environment::no_such_slot(*slot, slot_count)
            )));
            continue;
        }
        edges.retain(|held| !(held.node == edge.node && held.slot == edge.slot));
        edges.push(edge.clone());
        if !named.contains(slot) {
            named.push(*slot);
        }
    }
    // `None` for a slot with no watcher at all, `Some(false)` for one whose
    // build worker has ended: two different things to say, and neither of them
    // is "the slot is recompiling".
    let rebuilding: Vec<(usize, Option<bool>)> = named
        .into_iter()
        .map(|slot| {
            let state = aims
                .get_mut(slot)
                .map(|aiming| aiming.re_aim(edges.clone()).is_ok());
            (slot, state)
        })
        .collect();
    for (at, (slot, edge)) in asked.iter().enumerate() {
        if said[at].is_some() {
            continue;
        }
        let mut line = format!(
            "slot {slot}: wired `{}.{}={}`",
            edge.node, edge.slot, edge.to
        );
        // Keyed on the input alone, like the replacement above, and **only ones
        // that were applied**: a later request refused for its slot number wrote
        // nothing, and telling this one it had been replaced by an edge that
        // never landed would be the same lie in the other direction.
        let over = asked[at + 1..].iter().find(|(later_slot, later)| {
            slot_in_range(*later_slot, slot_count)
                && later.node == edge.node
                && later.slot == edge.slot
        });
        if let Some((_, later)) = over {
            let _ = write!(
                line,
                ", and a later request on this frame replaced it with `{}` — the run is \
                 wired with that one and it is what the rebuild carries",
                later.to
            );
        }
        let state = rebuilding
            .iter()
            .find(|(named, _)| named == slot)
            .and_then(|(_, state)| *state);
        let tail = match state {
            Some(true) => {
                " — the slot is recompiling with it, and `swap_outcome` says what the build \
                 made of it"
            }
            Some(false) => {
                " — this slot's build worker has ended, so nothing will rebuild: the edge is \
                 the run's from here on and a `save_set` of this slot records it"
            }
            None => {
                " — this slot has no watcher, so nothing rebuilds: what is on air was built \
                 with the wiring the run started with, and a `save_set` of this slot records \
                 the edge"
            }
        };
        line.push_str(tail);
        said[at] = Some(Ok(line));
    }
    // Every entry was filled by one of the two loops above: the first answers
    // the refusals and the second answers everything it skipped.
    said.into_iter().map(Option::unwrap).collect()
}

/// **One slot, with a worker watching its own two files behind it** — which is
/// what puts a candidate in the Staging lane and is the whole of what that
/// took.
///
/// # It is `karakuri-cli`'s wiring and deliberately not a second one
///
/// That program builds every `--watch` slot as `HotSwap::new` over a
/// `watch::Watch`, and every argument below is the reading `Set::build` made
/// at startup restated, because that is what a rebuild is: the *material*
/// changed and nothing else about the slot did. A request that derived any of
/// them again would be a slot that comes back as a different Set on the first
/// save — which is the failure `Watch`'s own fields are each documented
/// against.
///
/// - **`Layering::Overdraw` and no `live`** — `Set::build`'s own, which is
///   what every slot was built with: one target, however many renderers.
/// - **No `capacity`** — so each geometry is rebuilt at the capacity it
///   declares, which is [`capacity_of`]'s line asked again on the worker.
///   This program has no `--capacity` to override it (see [`USAGE`]), and
///   passing the startup reading would pin the slot to a declaration the file
///   may have just changed.
/// - **The slot's own salt, and no per-source salts** — `Set::build` passes
///   `&[]` and says why: *"A pair assigns nothing, so the one source is salted
///   from the Set's seed and its ordinal — which for source 0 is that seed
///   unchanged."* So a rebuild is the same simulation of new material rather
///   than a new one, and every slot stays at the salt [`slot_salt`] counted
///   off for it.
/// - **The default camera** — `Request::camera` is an `Orbit` rather than an
///   `Option` because *"a Set holds a built-in camera whatever its files
///   declare"*, and this program loads none, so the default is what it is
///   running.
/// - **No overrides, no published controls, no bindings, no edges and no
///   authorities** — this program has no flag for any of the five and grants
///   nothing (ADR-0216), so each is the empty list the startup build used.
///
/// # The store, and the two things a watcher is given
///
/// `Watch::storing_to` puts every build's sources in the store and reports them
/// as [`watch::Built`], which is where a rebuilt node's address comes from —
/// and nothing could derive one until something needed one, which is *Keep what
/// a deck is playing*: a Set file references its sources by hash, so a slot
/// whose builds were never stored is a slot that cannot be written down. See
/// [`Playing`], which is the other end of that channel.
///
/// `Watch::snapshotting_to` keeps every version that compiled under
/// `<store>/history/`, so an edit can be walked back — a hand at an editor and
/// a model writing over MCP both reach a file through the same path, and this
/// is where the version they replaced is kept (P-0096, ADR-0089). **The whole
/// run shares one `history::Snapshots`** with the launch-time seed, or the
/// first rebuild files the untouched procedure a second time.
///
/// **Every slot launches under no Set**, which is the truth rather than a
/// placeholder: this program opens on a pair, and a pair somebody typed is not
/// a Set (ADR-0276). What turns that into an id is a library load — [`loading`]
/// sends the id on the aim, and the watcher moves it — so the versions written
/// after a load are filed under the Set that was loaded.
///
/// # What it costs the frame path, which is nothing
///
/// A worker thread per slot, polling the two files every hundred milliseconds
/// and compiling on that thread. The render thread's side is unchanged:
/// `install_if_ready` polls the same channel with `try_recv` whether the
/// `Sender` is live or was dropped at construction, and a swap has always
/// landed at a frame boundary (ADR-0005). What is new on a *frame* is a
/// build's install, which is the mechanism this deck was already built on.
// **Eight, and each is a distinct thing this slot's watcher needs**: a device, a
// pair, a live Set, which slot it is, its salt, where builds go, where its
// layout is published and where its versions are kept. A struct bundling them
// would be one type with one construction site and one reader.
#[allow(clippy::too_many_arguments)]
pub(crate) fn watched(
    gpu: &Gpu,
    sources: &Sources,
    live: Set,
    slot: usize,
    salt: u32,
    // Where this watcher puts what it builds, and where it says so — or `None`
    // for a harness with no store to write into. See [`Engine::new`].
    stored: Option<(std::sync::Arc<Store>, std::sync::mpsc::Sender<watch::Built>)>,
    // **The run's one published layout**, made in [`main`] beside the opening
    // and for the same reason — see [`Aiming::pointing`]. This slot's row of it
    // is written here, at construction, and again on every re-point.
    pointing: karakuri_environment::mcp::Slots,
    // **The run's one history**, seeded in [`main`] from the same files this
    // slot watches, and `None` for a harness with no store — the same
    // condition `stored` above is `None` under, and a separate argument
    // because the two keep different things: that one is what reached the
    // *screen* and this is what reached the *compiler*. A version that cost
    // too much to run is in both — it is in the slot, stopped — and a version
    // that compiled and was superseded before it landed is in this alone.
    snapshots: Option<history::Shared>,
) -> (HotSwap, Aiming) {
    // **The other end of `Watch::aimed_by`**, kept by [`Engine`] so that a
    // load can say *look at these files instead*. It is made here rather than
    // by the caller because the watcher it belongs to is made here, and a
    // sender paired with the wrong slot's watcher would load a deck the
    // operator did not name.
    let (aim, aimed) = std::sync::mpsc::channel();
    // **The aim this watcher is constructed with, said once in a value rather
    // than only in the argument list below.** It is `Watch::new`'s arguments
    // less the slot, which is what [`watch::Aim`] is, and it is kept so that a
    // rewiring can restate the twelve fields it does not change. The two lists
    // are read side by side here on purpose: a field that disagreed would be a
    // rewiring that quietly moved something else.
    let at = watch::Aim {
        head: karakuri_environment::compile::Named::bare(&sources.l1),
        rest: vec![karakuri_environment::compile::Named::bare(&sources.l4)],
        layering: Layering::Overdraw,
        live: None,
        capacity: None,
        seed_salt: salt,
        salts: Vec::new(),
        camera: karakuri_engine::camera::Orbit::default(),
        overrides: Vec::new(),
        published: Vec::new(),
        bindings: Vec::new(),
        edges: Vec::new(),
        authorities: Vec::new(),
        // **No Set, because a slot launches on the pair this program was
        // started with and a pair somebody typed is not a Set.** The nearest
        // thing to a name is [`Sources::material`], which is a readout for the
        // mixer strip — filing versions under it would put rows in the history
        // under a Set no listing can ever match (ADR-0276). [`loading`] is what
        // turns this into an id.
        set: None,
    };
    let watching = watch::Watch::new(
        slot,
        // **Bare, so every node is called what its procedure declares**,
        // which is `Set::build`'s own: *"A pair names nothing, so both
        // nodes are called what their procedures are."* A name here
        // belongs to the *use* and this program has no syntax for one.
        karakuri_environment::compile::Named::bare(&sources.l1),
        vec![karakuri_environment::compile::Named::bare(&sources.l4)],
        Layering::Overdraw,
        None,
        None,
        salt,
        Vec::new(),
        karakuri_engine::camera::Orbit::default(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .aimed_by(aimed);
    // **Where a rebuild's sources go**, so that what a slot is running has an
    // address a Set file can name. Nothing is put until a build happens, and
    // the put is on the worker thread that compiled it.
    let watching = match stored {
        Some((store, tx)) => watching.storing_to(store, tx),
        None => watching,
    };
    // **Where every version that compiles is kept**, under the Set this slot is
    // running — which at launch is none, and is `at.set` for the same reason
    // the CLI reads its own aim there: one answer, and the aim is what moves it.
    let watching = match snapshots {
        Some(shared) => watching.snapshotting_to(shared, at.set.clone()),
        None => watching,
    };
    let swap = HotSwap::new(
        &gpu.device,
        &gpu.queue,
        live,
        // **The engine's own default rather than a number written here**: a
        // budget transcribed into this file would be a second answer to *how
        // long may a frame take* the day the engine's moves (ADR-0179 on a
        // number that is not even the mock's). It is 20 ms, which is 60 Hz
        // with room, and it is what makes the budget's verdict reachable in
        // this program at all — `HotSwap::fixed` judged against infinity, so no
        // slot could ever be stopped for cost.
        //
        // **And it is the opening value rather than the final one.** A
        // `HotSwap` is built here, from the launch pair, before there is a
        // window to ask what the display's interval is; `App::resumed` reads
        // `budget_ms(&window)` the moment there is one and narrows every slot
        // through `Deck::set_frame_budget_ms`. The constant is what stands where
        // the platform names no refresh rate (ADR-0313).
        DEFAULT_BUDGET_MS,
        Box::new(watching),
    );
    (swap, Aiming::new(aim, at, pointing, slot))
}

// ---------------------------------------------------------------------------
// Keeping what a deck is playing, and rewiring it
// ---------------------------------------------------------------------------

/// **What this run holds so that a deck can be kept, and rewired.**
///
/// One value rather than seven fields on [`App`], because the seven move
/// together and every one of them is read by the same three moments: a request
/// arriving, a build landing, and a save coming back off the disk. It is also
/// what makes those three reachable at all — the window loop binds `gfx` out of
/// `self.gfx` and holds it for the length of the handler, so a method on `App`
/// could not be called there. This is the piece that is passed instead.
///
/// **Every field is `pub(crate)`** rather than reached only through methods:
/// `App::new` builds one whole (its fields come from the same opening that
/// builds the rest of `App`) and `mod gpu`'s tests build one from nothing, and
/// both are outside this module — see `crate::App::new` and
/// `crate::gpu::keeping`.
pub(crate) struct Keeping {
    /// **The server's half of the channel, when `--mcp` asked for one**, and the
    /// whole of what a model reaches this program through.
    ///
    /// Told what the swap machinery said, handed what a client asked the render
    /// loop for, and nothing else — see [`karakuri_environment::mcp`]. It is
    /// bound in [`main`], before the window, for the reason the working copies
    /// are made there: `serve` binds a socket and can fail, and a failure has to
    /// be a sentence on a terminal rather than a panic inside a `winit`
    /// callback, where it aborts with no message at all.
    pub(crate) mcp: Option<mcp::Reporter>,
    /// **What each deck is playing**, seeded before the first frame and moved by
    /// every build that lands — see [`Playing`].
    pub(crate) playing: Playing,
    /// Where the watchers report what they built and stored — the other end of
    /// [`watch::Watch::storing_to`], drained where a build lands.
    pub(crate) built: std::sync::mpsc::Receiver<watch::Built>,
    /// **Builds reported but not yet landed**, kept by their build id.
    ///
    /// The two arrive on two channels and in either order: a watcher stores a
    /// build on its worker thread and the swap lands at a frame boundary some
    /// frames later, so a report that came in before its `Swapped` has to wait
    /// somewhere. **Removed when it lands**, so a build that was refused or that
    /// the deck never took leaves nothing behind — there is at most one
    /// outstanding build per slot, which is what `HotSwap` allows.
    pub(crate) pending: Vec<watch::Built>,
    /// Where a save that has reached the disk comes back, and the sending half
    /// each save thread is given a clone of.
    pub(crate) saves: std::sync::mpsc::Receiver<Saved>,
    pub(crate) save_tx: std::sync::mpsc::Sender<Saved>,
    /// **Where a send that has answered the dialog comes back**, and the
    /// sending half each send thread is given a clone of. [`saves`]' shape one
    /// act along, and it is a second channel rather than a second arm of the
    /// first because a send is not a save: it writes outside the store, under
    /// a name the operator typed into a window this program does not own, and
    /// nothing is waiting on it over MCP.
    ///
    /// **The run does not wait for these**, where it waits for the saves once
    /// at the end ([`Keeping::awaited_saves`]). A save is bounded by a disk; a
    /// send is bounded by a hand that has not answered a dialog yet, and a
    /// quit that blocked on one would be a program refusing to close because
    /// it had opened a window over itself. So there is no count kept here: a
    /// send still waiting on its dialog when the run ends wrote nothing, which
    /// is the same answer a dismissal gives.
    ///
    /// [`saves`]: Self::saves
    pub(crate) sends: std::sync::mpsc::Receiver<Sent>,
    pub(crate) send_tx: std::sync::mpsc::Sender<Sent>,
    /// **Where a kept procedure's outcome comes back**, and it is a third
    /// channel beside [`saves`](Self::saves) and [`sends`](Self::sends) for
    /// their reason: three acts that end on a disk, each answered at the frame
    /// its answer arrives on, and a queue apiece so that a slow write of one
    /// cannot delay another's answer.
    ///
    /// **It is not the save channel with a flag on it.** A keep writes one
    /// `.kir` under a name and a save writes a Set file naming every node; the
    /// two outcomes say different things, land in different directories and
    /// are refused for different reasons — one of them refuses a name that is
    /// taken, which a Set save does not — so folding them would be one
    /// sentence meaning two things.
    pub(crate) keeps: std::sync::mpsc::Receiver<Kept>,
    pub(crate) keep_tx: std::sync::mpsc::Sender<Kept>,
    /// How many saves are being written right now. The run waits for these once,
    /// at the end and under a bound — see [`Keeping::awaited_saves`].
    pub(crate) in_flight: usize,
}

impl Keeping {
    /// **What a model has asked for since the last frame.**
    ///
    /// `karakuri-cli`'s `Live::run_requests` is this function, and it is at the
    /// top of the frame for its reason: a surface is polled once and every
    /// request it produces ends in the method a key press ends in. That is what
    /// makes `--mcp` a second pair of hands rather than a second way to do
    /// anything.
    ///
    /// **Collected out of the borrow before any of it is acted on**, because
    /// both arms below take `&mut self`. Nothing is allocated on a frame that
    /// was asked for nothing: collecting an empty iterator makes no allocation.
    ///
    /// **Both channels drained before either is acted on**, for that reason and
    /// for a second one: they are two queues by design — see
    /// `mcp::Reporter::wires` — so a deck being saved to a slow disk cannot
    /// delay a rewiring, and taking them in one pass is what keeps that true on
    /// this side too.
    pub(crate) fn requests(&mut self, engine: &mut Engine, root: &std::path::Path) {
        let Some(mcp) = &self.mcp else {
            return;
        };
        let asked: Vec<mcp::SaveRequest> = mcp.saves().collect();
        let wires: Vec<mcp::WireRequest> = mcp.wires().collect();
        for request in asked {
            self.save_set(
                engine,
                root,
                Asked::Model,
                request.slot,
                request.id,
                Some(request.reply),
            );
        }
        self.rewire(engine, wires);
    }

    /// **Every edge asked for since the last frame, written and answered here,
    /// on this frame.**
    ///
    /// The decisions are [`rewired`]'s and are written there, because none of
    /// them needs a device. What is here is the two things that do: the deck's
    /// own slot count, which is the only thing that knows how many slots there
    /// are, and the answer going back to whoever asked.
    ///
    /// **Answered once, at the frame it was applied on**, which is
    /// `mcp::WireRequest`'s third point. Not at the swap: what the *build* made
    /// of the edge is `swap_outcome`'s answer, as it is for every other rebuild,
    /// and a tool that waited for thirty judged frames would hold a connection
    /// open across a transition.
    ///
    /// **One sentence for both audiences**, which is [`refused`]'s rule: what
    /// the terminal is told and what the client is handed are the same words, so
    /// the second cannot be right on the day it is written and wrong at the next
    /// correction.
    fn rewire(&mut self, engine: &mut Engine, asked: Vec<mcp::WireRequest>) {
        if asked.is_empty() {
            return;
        }
        let mut wires = Vec::with_capacity(asked.len());
        let mut replies = Vec::with_capacity(asked.len());
        for mcp::WireRequest { slot, edge, reply } in asked {
            wires.push((slot, edge));
            replies.push(reply);
        }
        let said = rewired(
            &wires,
            &mut engine.edges,
            &mut engine.aimed,
            engine.deck.slot_count(),
        );
        for (reply, said) in replies.into_iter().zip(said) {
            match &said {
                Ok(line) | Err(line) => println!("{line}"),
            }
            reply.settled(said);
        }
    }

    /// **Write what a deck is playing as a Set file.**
    ///
    /// `karakuri-cli`'s `Live::save_set` is the same method and says of itself
    /// that it is *the only save path* in that program; this is that path in
    /// this one, and every surface here ends in it — the `k` key, the MCP tool,
    /// and whatever control the Library bay grows. That is what makes a refusal
    /// and an outcome one sentence each rather than one sentence per surface.
    ///
    /// **The slot is an argument and the key passes the selected deck in.** A
    /// Set file describes one Set and this deck holds four; the one an
    /// *operator* means is the deck they have already selected, and a model has
    /// no selection and names the slot as it names one to read a procedure.
    ///
    /// **`id` is what the caller wanted it called, or a stamp.** A key press
    /// cannot type a name, so it passes `None`; see
    /// [`karakuri_environment::accepted_save`], whose convention that is.
    ///
    /// **Read off the live Set, not off the command line.** Every number a Set
    /// file carries can have moved since this run started — a param through a
    /// record, a capacity or a salt through a rebuild — so the only reading that
    /// cannot be stale is the Set's own. See [`playing_values`].
    ///
    /// **Gathered here, written elsewhere.** Everything up to the spawn is a
    /// read off values already in memory; the store I/O goes to a thread of its
    /// own — one per save, since saves are rare and a pool would be machinery
    /// for a rate of a few an hour. The outcome comes back over `saves` and is
    /// said at the frame it arrives, not at this press.
    pub(crate) fn save_set(
        &mut self,
        engine: &Engine,
        root: &std::path::Path,
        asked: Asked,
        slot: usize,
        id: Option<String>,
        reply: Option<mcp::Reply>,
    ) {
        // **Checked here rather than only where the request came from.** A key
        // press cannot name a slot this deck does not hold and a tool call can,
        // and below this line `playing_values` reads `deck.slot(slot)`, which
        // indexes. The server refuses it too, in its own words, so a model never
        // reaches this — and this is the guard that does not depend on it
        // having.
        let count = engine.deck.slot_count();
        if !slot_in_range(slot, count) {
            return refused(reply, karakuri_environment::no_such_slot(slot, count));
        }
        let Some(nodes) = self.playing.at(slot) else {
            // **The only way to reach this in this program**: a build landed
            // whose sources the store would not take, which the watcher said at
            // the time. Every slot is seeded at launch, so a slot that has never
            // rebuilt always has an address.
            return refused(
                reply,
                karakuri_environment::nothing_to_save(slot, None, false),
            );
        };
        let sources = setfile::Sources(nodes.iter().map(copied).collect());
        if sources.is_empty() {
            return refused(
                reply,
                karakuri_environment::nothing_to_save(slot, None, true),
            );
        }
        // **Named and answered above the line that needs a deck**, which is
        // where the whole of `accepted_save`'s doc lives: `playing_values` below
        // is the one read here that needs one, and the accept has to be on the
        // side of it a test can reach.
        // **An id an operator typed is checked here**, which is
        // `checked_name`'s wall for an arrangement's name one bay along: the
        // panel owns the affordance and never the authority (P-0090).
        // `filed_as` takes an `Asked::Operator` id verbatim, and until the
        // pane head's name (ADR-0292) nothing on an operator's side of this
        // call could carry one — `k` and the `keep` capsule both pass `None`.
        // The first thing that can is the first thing that could put a `/` in
        // a file name.
        if let Some(said) = id
            .as_deref()
            .and_then(|id| karakuri_environment::mcp::checked_id(id).err())
        {
            println!("keep: {said}");
            return refused(reply, said);
        }
        let id =
            karakuri_environment::accepted_save(slot, asked, id, &sources, root, reply.as_ref());
        let values = playing_values(engine.deck.slot(slot).set(), &engine.edges);
        let save = Save {
            slot,
            asked,
            id,
            root: root.to_path_buf(),
            sources,
            values,
        };
        let tx = self.save_tx.clone();
        // **A thread per save**, and detached: no frame waits for it. The *run*
        // waits, once, at the end and under a bound — see
        // [`Keeping::awaited_saves`], which is what this count is for.
        self.in_flight += 1;
        std::thread::spawn(move || {
            let (slot, asked, id) = (save.slot, save.asked, save.id.clone());
            // **Carried back rather than answered from here.** This thread knows
            // the outcome and could say it, and that would be a second place a
            // save is reported from.
            let outcome = save.run();
            let _ = tx.send(Saved {
                slot,
                asked,
                id,
                outcome,
                reply,
            });
        });
    }

    /// **Write one node's source into a library**, which is the act that makes
    /// the operator's tier of procedures exist at all
    /// ([P-0096](../../../docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md),
    /// ADR-0338 decision 4).
    ///
    /// **[`Keeping::save_set`]'s shape one node down**, and every division it
    /// makes is made here for its reason: the slot is an argument because a
    /// Set file describes one deck and this deck holds four; the id is what
    /// the caller wanted it called or a stamp, because the capsule on a node
    /// head types nothing; an operator-typed name is checked here, because the
    /// panel owns the affordance and never the authority (P-0090); and
    /// everything up to the spawn is a read off values already in memory,
    /// because a disk write is not a thing to do on a frame (P-0091).
    ///
    /// **Where it lands is decided by who asked and never by which control
    /// carried it**: `Asked::Operator` writes `<store>/procedures/` and
    /// `Asked::Model` writes `<store>/sandbox/`, which is a Set save's own
    /// division one file kind along (ADR-0261). A model is not refused here
    /// where its *star* is, because what it keeps is a file and so has a
    /// sandbox form to land in (ADR-0301).
    ///
    /// **The bytes are this run's rather than the disk's**, which is the whole
    /// of what [`Playing`] is for: what is kept is the version the node is
    /// *running*, and a `.kir` rewritten since the compile cannot reach a
    /// kept file.
    ///
    /// **Eight arguments, which is [`Keeping::save_set`]'s seven and the
    /// node.** Grouping them into a request type would be a shape only this
    /// call site can fill and would hide the one thing worth reading at a
    /// glance: which of `Asked`'s two this keep is, since that decides the
    /// directory.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn keep_procedure(
        &mut self,
        engine: &Engine,
        root: &std::path::Path,
        asked: Asked,
        slot: usize,
        node: karakuri_operation::NodeAddress,
        id: Option<String>,
        reply: Option<mcp::Reply>,
    ) {
        // **Checked here rather than only where the request came from**, which
        // is `save_set`'s own guard: a press cannot name a slot this deck does
        // not hold and a tool call can.
        let count = engine.deck.slot_count();
        if !slot_in_range(slot, count) {
            return refused(reply, karakuri_environment::no_such_slot(slot, count));
        }
        // **The address as the pane draws it** — `node_addr`'s own spelling,
        // which is the run of text under the operator's eye when they pressed.
        let addr = node_addr(ir_layer(node.layer), node.index);
        let Some(nodes) = self.playing.at(slot) else {
            return refused(
                reply,
                karakuri_environment::nothing_to_save(slot, None, false),
            );
        };
        // **The one node of that slot, by layer and index.** A node the list
        // has no entry for is the built-in camera or an address nobody drew,
        // and either way there is no source: the panel draws no capsule on the
        // camera's head, so a hand cannot reach this, and a model naming it is
        // told what it named rather than handed an empty file (P-0083).
        let Some(found) = nodes.iter().find(|kept| {
            kept.layer == setfile::kind_name(ir_layer(node.layer)) && kept.index == node.index
        }) else {
            let said = format!(
                "`{addr}` on deck {} has no source to keep — the built-in camera is a node with \
                 no procedure behind it, and no other address on this deck is missing one",
                deck_letter(slot as u8)
            );
            println!("keep: {said}");
            return refused(reply, said);
        };
        // **An id an operator typed is checked here**, which is `save_set`'s
        // own wall and its reason: `<name>` becomes one path component, and
        // the console emits what was typed including the empty string.
        if let Some(said) = id
            .as_deref()
            .and_then(|id| karakuri_environment::mcp::checked_id(id).err())
        {
            println!("keep: {said}");
            return refused(reply, said);
        }
        // **A stamp where nobody typed**, which is `accepted_save`'s own
        // convention read one file kind along: the capsule is the press that
        // types nothing (ADR-0128, ADR-0287).
        let name = id.unwrap_or_else(karakuri_environment::history::stamped_id);
        let kept = Kept {
            asked,
            name,
            root: root.to_path_buf(),
            source: found.source.clone(),
            hash: found.hash,
            addr,
            // **Filled by the thread**, and this value is never read: the
            // request and the outcome are one type here because the two carry
            // the same fields, and the `Ok` below is the unwritten state
            // rather than a claim.
            outcome: Ok(std::path::PathBuf::new()),
            reply,
        };
        let tx = self.keep_tx.clone();
        // **A thread per keep**, and detached: no frame waits for it — the
        // save path's own arrangement, and a keep is rarer than a save.
        self.in_flight += 1;
        std::thread::spawn(move || {
            let _ = tx.send(kept.run());
        });
    }

    /// **Every kept procedure that has landed since the last frame, said and
    /// answered.**
    ///
    /// [`Keeping::finished_saves`]' drain one act along and on the same terms:
    /// drained and never waited on, because a frame owes the display a picture
    /// and owes a disk nothing.
    ///
    /// **It returns whether any of them landed**, which is what the Library
    /// bay's listing is re-read on: a keep is the only thing in this program
    /// that adds a procedure to the operator's tier, and a bay that did not
    /// list it would be a readout that is wrong and silent. **A model's does
    /// not count**, for the reason a sandbox save does not: `all` lists the
    /// operator's library and the sandbox is not in it.
    pub(crate) fn finished_keeps(&mut self) -> bool {
        let mut landed = false;
        while let Ok(kept) = self.keeps.try_recv() {
            self.in_flight = self.in_flight.saturating_sub(1);
            let said = kept.said();
            match &said {
                Ok(line) | Err(line) => println!("{line}"),
            }
            landed |= said.is_ok() && kept.asked == Asked::Operator;
            if let Some(reply) = kept.reply {
                reply.settled(said);
            }
        }
        landed
    }

    /// **What a session's head is written from**, gathered off the live deck
    /// and written by whoever is handed it.
    ///
    /// It is [`Keeping::save_set`]'s first half with the answering taken out:
    /// the same two readings — what slot [`HEAD_SLOT`] is playing, and what
    /// that Set is holding — put into the same [`Save`], which is what keeps a
    /// head and a keep one description of a deck rather than two. **The whole
    /// of the difference is the id**: a keep files under what the caller
    /// wanted it called, and this files under the session's id with
    /// `-material` after it, which is `karakuri-cli`'s own spelling for the
    /// same file.
    ///
    /// **`Asked::Operator`, so it lands in the library**: this is the material
    /// of a run somebody started, kept where they will look for it — and it is
    /// what `--replay` resolves the stream's nodes through.
    ///
    /// **Read off the live Set and not off anything this program was told**,
    /// which is [`playing_values`]' whole argument: every number a Set file
    /// carries can have moved since this run started, so a head written from
    /// the launch arguments would describe a deck nobody is looking at. That
    /// is also what makes a recording begun mid-performance possible at all.
    fn head_material(
        &self,
        engine: &Engine,
        root: &std::path::Path,
        session: &str,
    ) -> Result<Save, String> {
        let count = engine.deck.slot_count();
        if !slot_in_range(HEAD_SLOT, count) {
            return Err(karakuri_environment::no_such_slot(HEAD_SLOT, count));
        }
        let Some(nodes) = self.playing.at(HEAD_SLOT) else {
            return Err(karakuri_environment::nothing_to_save(
                HEAD_SLOT, None, false,
            ));
        };
        let sources = setfile::Sources(nodes.iter().map(copied).collect());
        if sources.is_empty() {
            return Err(karakuri_environment::nothing_to_save(HEAD_SLOT, None, true));
        }
        Ok(Save {
            slot: HEAD_SLOT,
            asked: Asked::Operator,
            id: format!("{session}-material"),
            root: root.to_path_buf(),
            sources,
            values: playing_values(engine.deck.slot(HEAD_SLOT).set(), &engine.edges),
        })
    }

    /// **Every save that has landed since the last frame, said and answered.**
    ///
    /// Drained and never waited on: a frame owes the display a picture and owes
    /// a disk nothing. Called at the top of the frame beside
    /// [`Keeping::requests`], because a window that has faulted still has saves
    /// finishing behind it and a run that told nobody about them until it quit
    /// would be withholding the one answer a waiting client cannot get anywhere
    /// else.
    ///
    /// Returns whether any of them landed, which is what the Library bay's
    /// listing is re-read on: a save is the only thing in this program that adds
    /// a Set, and a bay that did not list it would be a readout that is wrong
    /// and silent.
    pub(crate) fn finished_saves(&mut self) -> bool {
        let mut landed: Vec<Saved> = Vec::new();
        while let Ok(saved) = self.saves.try_recv() {
            landed.push(saved);
        }
        let mut written = false;
        for saved in landed {
            written |= self.took_save(saved);
        }
        // **Every send that has answered since the last frame, said here.**
        // The same drain in the same place and for the same reason — a frame
        // owes the display a picture and owes a disk nothing — and it is on
        // this call rather than on one of its own so that the two outcomes a
        // press can be waiting for are reported at one moment.
        //
        // **It returns nothing to the caller.** A save adds a row to the
        // Library bay's listing and a send does not: the file lands wherever
        // the operator sent it, which is outside this store by construction,
        // so there is no listing here that could have gone stale.
        while let Ok(sent) = self.sends.try_recv() {
            Keeping::took_send(sent);
        }
        written
    }

    /// One send's outcome, said.
    ///
    /// [`Keeping::took_save`]'s shape one act along, with the answering taken
    /// out: nothing over MCP is waiting on a send — the tool does not exist
    /// and would be handed the bytes rather than a path
    /// ([ADR-0260](../../../docs/adr/0260-sending-a-set-is-a-read-and-a-reads-answer-goes-where-the-surface-that-asked-puts-answers.md))
    /// — so the terminal is the whole audience and there is no second copy of
    /// the words to keep in step.
    ///
    /// **A dialog that was dismissed is one of the three outcomes and is said
    /// out loud**, rather than being silence: rule 04 of the manual is that
    /// nothing is hidden quietly, and a press that opened a window and then
    /// wrote nothing is exactly the case a reader would otherwise read as a
    /// fault.
    ///
    /// **A failure is printed and nothing claims otherwise** — [`Saved`]'s own
    /// rule: a program saying a file was written when the disk refused is the
    /// shape of lie this codebase is arranged against.
    fn took_send(sent: Sent) {
        println!("{}", sent.said());
    }

    /// One save's outcome, said and answered.
    ///
    /// **One sentence for both audiences.** What the terminal is told and what a
    /// waiting client is handed are the same fact, so the words are formed once
    /// here and the client gets the ones the operator got. The `Ok`/`Err` split
    /// is what a tool call's `isError` is built from — see `mcp::Reply::settled`.
    fn took_save(&mut self, saved: Saved) -> bool {
        let Saved {
            slot,
            asked,
            id,
            outcome,
            reply,
        } = saved;
        self.in_flight = self.in_flight.saturating_sub(1);
        let (written, said) = match outcome {
            // **Two sentences, and the `written` beside them is two answers
            // too.** `all` lists the operator's library, so a sandbox save
            // adds no row and re-reading the listing would be a repaint that
            // changes nothing — and telling a model that a keyboard load takes its file
            // back would send it after a row the bay does not draw
            // (P-0096, ADR-0261).
            Ok(()) => match asked {
                Asked::Operator => {
                    let said = format!(
                        "  keep: deck {}: saved as set `{id}` — the Library bay's `all` \
                         lists it, and a load off that row puts it back",
                        deck_letter(slot as u8)
                    );
                    println!("{said}");
                    (true, Ok(said))
                }
                Asked::Model => {
                    let said = format!(
                        "  keep: deck {}: saved as set `{id}` in the sandbox — \
                         `<store>/{}/{id}{}`. A save asked for over MCP is kept there \
                         rather than in the operator's library, so `all` does not list \
                         it and no load off that row reaches it; the operator's own `k` writes \
                         library",
                        deck_letter(slot as u8),
                        karakuri_store::store::Store::SANDBOX,
                        karakuri_store::store::Store::SET_FILE_SUFFIX,
                    );
                    println!("{said}");
                    (false, Ok(said))
                }
            },
            // **Printed, and nothing claiming otherwise.** See `Saved::outcome`.
            Err(e) => {
                let said = format!(
                    "  keep: deck {}: set `{id}` was not saved: {e}",
                    deck_letter(slot as u8)
                );
                println!("{said}");
                (false, Err(said))
            }
        };
        if let Some(reply) = reply {
            reply.settled(said);
        }
        written
    }

    /// **Take up what a slot is now playing.**
    ///
    /// `landed` is the id of the build that went in, and there is no other
    /// case: a swap is the only event that changes what a slot holds
    /// (ADR-0316). It used to take an `Option`, whose `None` was a rollback,
    /// and the version that came back had to have been remembered because
    /// nothing would name it again.
    ///
    /// The `built` channel is drained here rather than per frame: it only has
    /// anything in it when a build has just been requested, and this runs when
    /// one has just landed.
    ///
    /// # It answers which nodes changed, because this is the only place both
    /// lists exist
    ///
    /// A build reports a hash per node and this list replaces the one the slot
    /// was on, so the two are in one hand for exactly the length of this
    /// function. **The diff is taken here or it is not taken at all** — a
    /// caller coming back for it afterwards would find one list — and it is
    /// what the Staging lane's rows are, one per changed node
    /// (`docs/adr/0326-a-staging-row-is-a-changed-node-and-the-row-is-the-keep.md`).
    ///
    /// **Empty is three states and one answer**, which is deliberate: a build
    /// whose sources the store would not take has no new list, a slot with no
    /// baseline has no old one, and a rebuild that restated the stack
    /// unchanged has a diff with nothing in it. In every one of them there is
    /// no node this verdict can honestly be pinned to, and the lane draws one
    /// row on the slot — see [`settle`]. The first two say so at the time, on
    /// the watcher's own line.
    ///
    /// **A node the new list has and the old one does not counts as
    /// changed**, and one the old list had and the new one does not is not
    /// reported at all: a lane row is a version somebody has still to rule on,
    /// and a node a rebuild removed has none.
    pub(crate) fn took_up(
        &mut self,
        aims: &[Aiming],
        slot: usize,
        landed: u64,
    ) -> Vec<(&'static str, u32)> {
        let mut ready: Vec<watch::Built> = Vec::new();
        while let Ok(built) = self.built.try_recv() {
            ready.push(built);
        }
        for built in ready {
            self.pending.push(built);
        }
        // **A build with nothing in `pending` is a build whose sources the
        // watcher could not store**, which it said at the time. It is handed to
        // `landed` as `None` rather than returned on, because the swap happened
        // either way: a slot that took a version nobody can name is a slot with
        // no address, not a slot still on its old one.
        let at = self.pending.iter().position(|built| built.id == landed);
        let built = at.map(|at| self.pending.remove(at));
        let nodes = built
            .as_ref()
            .zip(aims.get(slot))
            .map(|(built, aiming)| built_nodes(built, &aiming.at));
        // **Against what the slot was on, before that is overwritten.** The
        // hash is the whole of the comparison: `karakuri_store`'s address is
        // the content, so two builds of one file differ here exactly when the
        // bytes differ, which is what an operator means by *changed*.
        let changed = match (self.playing.at(slot), nodes.as_ref()) {
            (Some(was), Some(now)) => now
                .iter()
                .filter(|node| {
                    !was.iter().any(|before| {
                        before.layer == node.layer
                            && before.index == node.index
                            && before.hash == node.hash
                    })
                })
                .map(|node| (node.layer, node.index))
                .collect(),
            _ => Vec::new(),
        };
        self.playing.landed(slot, nodes);
        changed
    }

    /// **Every save still being written, waited for — up to
    /// [`karakuri_environment::SAVE_WAIT`].**
    ///
    /// A frame owes the disk nothing, which is why [`Keeping::finished_saves`]
    /// drains and never blocks. The end of the run is the one moment where that
    /// is the wrong trade: a save asked for in the last second reached the disk
    /// under an id nobody was ever told, so the client that asked for it waits
    /// out its deadline and the operator is told nothing at all.
    ///
    /// **Bounded, because a disk can hang and quitting must not depend on one.**
    /// Past the bound the run says how many saves it left behind and exits.
    ///
    /// The wait is only ever paid by a run that saved and quit within a few
    /// frames; the count is zero for every other run and this returns without
    /// blocking.
    pub(crate) fn awaited_saves(&mut self) {
        self.finished_saves();
        // **And the keeps, because they raise the same count.** A run that
        // kept a procedure and quit a frame later has a thread still writing
        // it, and the bound below is what it is waited for under.
        self.finished_keeps();
        if self.in_flight == 0 {
            return;
        }
        println!(
            "waiting up to {:.0}s for {} save{} still being written",
            karakuri_environment::SAVE_WAIT.as_secs_f32(),
            self.in_flight,
            match self.in_flight {
                1 => "",
                _ => "s",
            }
        );
        let deadline = Instant::now() + karakuri_environment::SAVE_WAIT;
        while self.in_flight > 0 {
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            // Timed out, or every sender is gone and nothing more can arrive.
            // Either way there is nothing left to wait for.
            // **Both queues under one bound**, and it is a poll rather than a
            // block because `recv_timeout` waits on *one* channel and the two
            // are two by design (`Keeping::keeps`). Waiting the whole deadline
            // on the saves would make a run that kept a procedure and quit sit
            // out the bound with the file already written. The slice is short
            // enough that a quit is not noticeably slower and long enough that
            // this is not a spin.
            let slice = left.min(std::time::Duration::from_millis(20));
            if let Ok(saved) = self.saves.recv_timeout(slice) {
                self.took_save(saved);
            }
            self.finished_keeps();
        }
        if self.in_flight > 0 {
            println!(
                "  {} save{} still unfinished after {:.0}s — each is written or it is not, and \
                 nothing here claims either way",
                self.in_flight,
                match self.in_flight {
                    1 => "",
                    _ => "s",
                },
                karakuri_environment::SAVE_WAIT.as_secs_f32(),
            );
        }
    }
}
