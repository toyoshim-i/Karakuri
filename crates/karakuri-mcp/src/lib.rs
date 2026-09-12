//! A control surface for a model rather than for a pair of hands.
//!
//! `--mcp PORT` serves the Model Context Protocol over HTTP on the loopback
//! interface, so a chat client can read a slot's procedure, rewrite it, and be
//! told what the compiler and the frame budget made of the result. The name for
//! what that enables is **vibe live coding**: "the one that is showing now, a
//! bit more vivid" is a small edit to a declarative file, and the file is the
//! thing this system was already built to hot-swap.
//!
//! ## It is the third surface, and it obeys the same rule as the other two
//!
//! Keys, then MIDI, now this. The invariant
//! (`docs/principles/0090-a-surface-offers-it-never-decides.md`)
//! is that everything an
//! operator moves goes through a record, is read back, and only then applied —
//! so that an agent is structurally incapable of doing anything a human could
//! not do through the same interface.
//!
//! An agent from outside the process is still an agent — and this surface is
//! the reason the invariant is now true of *material* as well as of the mix.
//! It was not: a procedure change was a file and not a record, so a session in
//! which a model rewrote slot 0 at minute ten replayed with the procedure it
//! started with, silently. The hole predated this module by as long as
//! `--watch` has existed, and nobody was going to be misled by a human typing
//! in vim; somebody would certainly have been misled by this. `Record::Procedure`
//! closes it, so **a session driven by a model does replay with no model
//! attached**, and what an agent did during a set can be watched back.
//!
//! ## It names its operations, and it performs them itself
//!
//! Every one of the seven tools is one of the vocabulary's operations
//! `docs/manual/operations.html` specifies — `read_procedure`,
//! `write_procedure`, `wire_input`, `swap_outcome`, `save_set`, `read_set` and
//! `list_sets` are `ReadProcedure`, `WriteProcedure`, `WireInput`,
//! `SwapOutcome`, `SaveSet`, `ReadSet` and `ListSets` — and the call becomes
//! that operation in [`asked`] before anything is done with it. [`perform`]
//! then dispatches on the operation rather than on the tool's name, so the row
//! on the page a tool claims is the row its operation's title names.
//!
//! **What it does not do is hand the operation to `Live::operate`, and that is
//! `Silent`'s shape rather than an omission.** `karakuri_operation_record`'s
//! `written` answers `Silent` for all seven: `Question` for the four that ask —
//! a record is what a replay reconstructs a performance from, and a question
//! changes no performance — `OnLanding` for the two whose record is
//! written where the work lands, `Record::Procedure` at the swap and
//! `Record::Save` at the frame the save landed, and `NoRecord` for
//! `wire_input`. An operation routed through
//! `operate` that writes no record prints *no record* and does nothing, which
//! is `docs/adr/0198-…`'s finding about twelve of the keyboard's keys and holds
//! here for all seven tools. There is no `Live` on these threads to route into
//! either: this server reaches the render loop for two things, and both go on
//! the channels below.
//!
//! ## The seventh tool is a hole in the first paragraph of this file
//!
//! `Record::Procedure` closed the material half of P-0090 for a *procedure*.
//! `wire_input` reopens a strip of it: `written` answers
//! `Silent(Silent::NoRecord)` for `WireInput`, because `Record::Edge` is a
//! **Set file's** record and has no `slot` to carry the deck a live rewiring
//! names. So a model that binds `morph.far` at minute ten replays with the
//! Set's launch wiring — which, where the `uses` was written in the same
//! session, is a replay that does not build at all. That is stated here rather
//! than left to be discovered, it is asserted in
//! `no_tool_writes_a_record_where_it_is_asked` below, and what closes it is
//! a `slot` on `Record::Edge` and a `written` arm for it, neither of which is
//! this file's to write.
//!
//! ## Most of this never touches the frame
//!
//! Reading a procedure is reading a file. Writing one is checking it and
//! writing a file — the compile, the frame-boundary swap, the measurement and
//! the stopped slot if it costs too much are `--watch`'s, built for
//! editing by hand and now doing the most dangerous part of this: **a model
//! that writes something too expensive is caught by the machinery that already
//! catches a human who does.**
//!
//! ## Two channels, and neither of them is a lock
//!
//! The outcome of a swap comes from the render loop over a channel rather than
//! a lock, so the frame path neither blocks nor waits. `save_set` is the first
//! tool that needs the *other* direction: what a slot is playing lives on the
//! render thread and is reachable from nowhere else, so a request goes back the
//! same way, is taken where the MIDI surface is taken, and ends in the method
//! the `k` key ends in. **There is one save path in this program**, and it is
//! `Live::save_set`; this is a way to ask for it and not a second copy of it.
//! What the two calls differ in is one argument — `karakuri_environment::Asked`
//! — and what it decides is the directory: a save asked for here lands in
//! `<store>/sandbox/` and the operator's own key writes the library
//! (`docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md`).
//!
//! **The wait for its answer happens with [`State`] unlocked.** There is a
//! thread per connection and one mutex over the state, so a tool that waited
//! for a disk while holding it would stop every other connection — including
//! one that only wanted to read a procedure — for as long as the loop took.
//! [`Pending`] exists for no other reason.
//!
//! ## Loopback only
//!
//! There is no bind option and there should not be one. A venue network is
//! shared, and a port that can rewrite what is on the projector is not
//! something to expose by a flag anyone might pass without meaning it. Reaching
//! a render machine from a laptop is `ssh -L`, which is a thing an operator
//! does deliberately and can see.

use std::io::{BufRead, Read, Write};
use std::sync::mpsc;

use karakuri_ir::Kind;
#[cfg(test)]
pub(crate) use karakuri_operation::gate;
pub(crate) use karakuri_operation::{InputPort, NodeAddress, Operation};
use serde_json::{json, Value};
mod spelled;
pub(crate) use spelled::*;

pub mod tools;
pub(crate) use tools::*;
pub use tools::{check_procedure, check_set_configuration, checked_id, OperateRequest, LISTED};

pub use karakuri_ir::{Diagnostic, DiagnosticReport};

/// What the render loop tells the server about, over a channel.
///
/// **A channel and not a shared lock**: the frame path may wait for nothing,
/// and a swap is rare enough that the send costs less than the `eprintln!`
/// beside it already does.
pub enum Event {
    /// A build landed, was overloaded, or failed, in the words the operator
    /// saw on the terminal.
    Swap { slot: usize, said: String },
}

/// **A save one client is asking the render loop for.**
///
/// The channel this travels on is the one this module did not have, and the
/// reason a fourth tool cost more than the first three: reading and writing a
/// procedure are files, and this is the *live Set* — which only the render
/// thread holds. So the request goes to the loop and the answer comes back,
/// rather than this module growing a second idea of what a slot is playing.
pub struct SaveRequest {
    /// Which deck slot. Checked against [`Slots`] before it is sent, so a slot
    /// this deck does not hold meets the refusal `read_procedure` gives it; the
    /// loop checks the range again, because it is the only thing that knows how
    /// many slots the *deck* has.
    pub slot: usize,
    /// What to file it under, or `None` to let the loop name it after the
    /// moment — which is what a key press gets, for the reason
    /// [`crate::history::stamped_id`] states: a key cannot type a name.
    pub id: Option<String>,
    /// Where the answer goes.
    pub reply: Reply,
}

/// **The half of one [`SaveRequest`] the render loop answers on.**
///
/// Two messages rather than one, because the loop already says two things: that
/// it has started a save, at the frame it was asked, and what became of it, at
/// the frame the disk answered. A client still waiting when the deadline passes
/// has the first of them — which is what makes a truthful timeout possible at
/// all, since "accepted, under this id, outcome not yet known" is a different
/// fact from either success or failure.
pub struct Reply(pub(crate) mpsc::Sender<News>);

impl Reply {
    /// The loop has taken it and named the set, in the words it printed.
    ///
    /// **Never blocks**: the channel is unbounded and one save puts at most two
    /// messages in it. This is called from a frame.
    pub fn accepted(&self, said: &str) {
        let _ = self.0.send(News::Accepted(said.to_string()));
    }

    /// What the save came to, in the words the operator was given for it.
    ///
    /// Sent from the frame the outcome landed on, which is the frame the `save`
    /// record is written on — so what a model is told and what the stream says
    /// come from one place. `Err` is a refusal or a disk that said no, and both
    /// reach the client as a failed tool call.
    pub fn settled(self, said: Result<String, String>) {
        let _ = self.0.send(News::Settled(said));
    }
}

/// **The other half of [`karakuri_environment::accepted_save`]'s call**, so
/// that crate can hand a save's acceptance back to whichever client is
/// waiting on it without naming `Reply` — the dependency runs one way, from
/// this crate into `karakuri-environment`, and this `impl` is what lets it
/// stay that way.
impl karakuri_environment::SaveReply for Reply {
    fn accepted(&self, said: &str) {
        Reply::accepted(self, said)
    }
}

/// **An edge one client is asking the render loop to write.**
///
/// **The second thing this server reaches the loop for, and it is the loop for
/// the same reason a save is**: the wiring a slot rebuilds with is not on disk
/// anywhere. A procedure is a file, so `write_procedure` writes one and lets
/// `--watch` find it; an edge is a *statement about a Set* that the run holds —
/// `Args::edges` at launch, `Watch::edges` on every rebuild, `Live::edges` when
/// a save asks what the slot is wired with — and nothing in this process but
/// the render loop can see any of the three. A copy kept here would be a second
/// answer to what the run is wired with, which is the shape
/// `docs/principles/0086-a-procedure-knows-only-what-it-declares.md` warns about from
/// the other end: *every surface that rebuilds one has to carry the names it
/// was spelled with rather than regenerate them.*
///
/// **What the loop owes a request it takes**, so that the contract is written
/// where the sender is rather than in whoever drains it:
///
/// 1. **Replace, keyed on `(node, slot)`, in the run's own wiring for that deck
///    — the list a rebuild restates (`Watch::edges`) and a save records
///    (`Live::edges`), which are one list and not two — and touch no other
///    edge.** That is
///    what `--edge` beside `--load-set` already does — the file's edges, minus
///    the slots the flags name, plus the flags' — and it is forced rather than
///    chosen: `SetError::SlotBoundTwice` refuses two edges on one slot, so an
///    append would make the *second* call on a slot a refusal and leave a model
///    unable to change its mind.
/// 2. **Rebuild the slot the deck names**, on the path an edit takes: compiled
///    on a worker, swapped at a frame boundary, judged against the budget and
///    left in the slot with the slot stopped if it costs too much. An edge is priced by the same
///    validation as everything else between nodes, which is the whole of why
///    this is inside MCP's scope — see the description of [`tools`]'s
///    `wire_input`.
/// 3. **Answer once, at the frame it was applied on** — [`Reply::settled`],
///    with what the loop would have printed. Not at the swap: what the *build*
///    made of it is `swap_outcome`'s answer, as it is for every other rebuild,
///    and a tool that waited for a verdict would be a tool that holds a
///    connection open across a transition.
pub struct WireRequest {
    /// Which deck slot the edge is about. Checked against [`Slots`] before it
    /// is sent, in the sentence every other surface refuses an absent slot in.
    ///
    /// **The deck is on the operation** — `Operation::WireInput` carries one —
    /// so an edge asked for here is a statement about *this* deck's Set and not
    /// about the run. `--edge` is run-wide because a launch flag is one command
    /// line for every Set it starts; a request made during a show names the
    /// deck it is about.
    pub slot: usize,
    /// The edge itself, in the engine's own type: both ends by name, which is
    /// `Record::Edge`'s decision and the reason `Operation::WireInput` is the
    /// one operation addressed by name at both ends.
    pub edge: karakuri_engine::set::Edge,
    /// Where the answer goes. One message: see the contract above.
    pub reply: Reply,
}

/// What a [`Reply`] carries, in the order it carries it.
pub(crate) enum News {
    Accepted(String),
    Settled(Result<String, String>),
}

/// The half of the server the render loop holds.
pub struct Reporter {
    sender: mpsc::SyncSender<Event>,
    /// What clients have asked the loop to do. The receiving half, because this
    /// is the direction [`Event`] does not go in.
    requests: mpsc::Receiver<SaveRequest>,
    /// And the edges they have asked it to write. **A channel of its own rather
    /// than one queue of a request enum**: the two are drained by one loop but
    /// they are not one queue's worth of pressure — a deck being saved to a slow
    /// disk must not be able to fill the queue an edit is rewired through, and
    /// [`ASKED`] is a bound on each kind rather than on both together.
    wires: mpsc::Receiver<WireRequest>,
    /// And the operations they have asked it to perform, on a third channel for
    /// the second one's reason: a save to a slow disk must not be able to fill
    /// the queue a fader moves through, and [`ASKED`] is a bound on each kind
    /// rather than on all three together.
    operations: mpsc::Receiver<OperateRequest>,
    /// Reports the queue had no room for. **Counted rather than lost quietly**:
    /// a client that is told what happened must be told when it is not the
    /// whole story.
    dropped: std::sync::Arc<std::sync::atomic::AtomicU64>,
    port: u16,
}

impl Reporter {
    /// Tell the server what a slot's swap machinery just did.
    ///
    /// **Never blocks and never waits for room.** A bounded queue and
    /// `try_send`: a client that has stopped asking must not be able to make
    /// the render thread wait, and the queue in front of the bounded history
    /// used to be unbounded, so it was not a report on the present at all.
    pub fn swap(&self, slot: usize, said: &str) {
        let event = Event::Swap {
            slot,
            said: said.to_string(),
        };
        if self.sender.try_send(event).is_err() {
            self.dropped
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// **Every save a client has asked for since this was last called.**
    ///
    /// Drained and never waited on, from the frame path, exactly as the MIDI
    /// surface is drained beside it: a frame owes a client nothing, and each
    /// request ends in the same method the `k` key ends in. Empty on almost
    /// every frame, and an empty `collect` allocates nothing.
    pub fn saves(&self) -> impl Iterator<Item = SaveRequest> + '_ {
        self.requests.try_iter()
    }

    /// **Every edge a client has asked to be written since this was last
    /// called.**
    ///
    /// Drained beside [`Reporter::saves`] and on the same terms, and what the
    /// loop owes each one is written at [`WireRequest`].
    ///
    /// **A loop that never calls this is not silently obeyed.** Every request
    /// carries a [`Reply`] the client waits [`WIRE_REPLY`] for, so a run whose
    /// loop does not drain this answers *the render loop had not taken this
    /// edge* — which is true, is loud, and is the third of
    /// `docs/principles/0079-…`'s answers rather than a tool that reports work
    /// nobody did.
    pub fn wires(&self) -> impl Iterator<Item = WireRequest> + '_ {
        self.wires.try_iter()
    }

    /// **Every operation a client has asked to be performed since this was last
    /// called.**
    ///
    /// Drained beside [`Reporter::saves`] and [`Reporter::wires`] and on the
    /// same terms, and what the loop owes each one is written at
    /// [`OperateRequest`]. **Already audited**: `karakuri_operation::gate` ran
    /// on the connection thread, so a loop draining this performs what it is
    /// handed and does not judge it again — the audit is the one call the
    /// server makes and not a check every drain repeats.
    ///
    /// **A loop that never calls this is not silently obeyed**, which is
    /// [`Reporter::wires`]'s clause and its reason: every request carries a
    /// [`Reply`] the client waits [`OPERATE_REPLY`] for, so a run whose loop
    /// does not drain this answers *the render loop had not taken this
    /// operation*.
    pub fn operations(&self) -> impl Iterator<Item = OperateRequest> + '_ {
        self.operations.try_iter()
    }

    /// The port actually bound, which is not the one asked for when that was 0.
    pub fn port(&self) -> u16 {
        self.port
    }
}

/// Where a slot's procedures live, so a tool can name a slot rather than a
/// path.
///
/// **Paths never cross the protocol.** A client may be on another machine
/// through an `ssh -L`, where a path means nothing — and a tool that took one
/// would be inviting a model to write anywhere on the render machine's disk.
///
/// **It is a handle and not a list, for [`crate::Opening`]'s reason.** This
/// used to be the launch working copies, copied into [`serve`] before the
/// window opened and never written again — and a surface that loads a Set into
/// a running slot re-points that slot at *new* scratch files. So after one
/// library load every address this resolved was the pre-load layout: a
/// `read_procedure` handed back the source of a procedure the deck had stopped
/// running, a `write_procedure` wrote a file no watcher was looking at and
/// reported that it was being built, and a node the loaded Set does hold was
/// refused for not existing. Each of those is a **plausible wrong answer** on
/// the one surface whose reader is a program in a loop
/// ([P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)),
/// and none of them fails loudly. `Opening` had the same shape of problem — an
/// operator opens a class *while* the run is going — and its answer is this
/// one: a shared handle the host writes and the server reads on every call.
///
/// **What a slot is running is [`crate::watch::Aim`], and this publishes it.**
/// [`Slots::re_point`] takes the aim itself rather than two paths, so the walk
/// from *where a watcher is pointed* to *the files behind a slot* is written
/// once, in the crate that owns both types
/// ([P-0087](../../../docs/principles/0087-name-the-property-never-the-shape.md)).
/// A host publishes where it sends an aim and nowhere else, which is one write
/// per re-point: a load writes several files and then sends one aim, so a call
/// arriving mid-load sees the layout before it or the layout after it and
/// never half of either.
#[derive(Clone, Default)]
pub struct Slots(std::sync::Arc<std::sync::RwLock<Vec<Pointed>>>);

/// **Where one slot is pointed**: the head of its chain, and the rest in file
/// order.
///
/// It is [`crate::watch::Aim`]'s own `head` and `rest` less the names, which is
/// what [`Slots::re_point`] derives a published one from, and it is `--set`'s
/// shape on the command line.
pub type Pointed = (std::path::PathBuf, Vec<std::path::PathBuf>);

impl Slots {
    /// **The deck as it stands**, one pair per slot in slot order.
    ///
    /// What a run seeds this with is where its watchers are pointed when it is
    /// made, and a run whose slots never move — `karakuri-cli`, which loads
    /// nothing mid-run — keeps that value for the whole run and calls nothing
    /// else here.
    pub fn of(pairs: Vec<Pointed>) -> Slots {
        Slots(std::sync::Arc::new(std::sync::RwLock::new(pairs)))
    }

    /// **A deck nothing is published to**, for a harness that builds an engine
    /// and serves nothing.
    ///
    /// It holds no row, so [`Slots::re_point`] writes nothing into it and every
    /// address it could be asked about is refused with what it holds — which is
    /// zero slots. **It cannot reach a client**: [`serve`] refuses a handle with
    /// no slots in it before it binds, which is the same refusal a run given no
    /// procedure files gets.
    pub fn unpointed() -> Slots {
        Slots::of(Vec::new())
    }

    /// **One slot is pointed somewhere else**, said by whoever moved it.
    ///
    /// Called where an aim is *sent* — a library load, and a rewiring that
    /// restates one — so that the published layout cannot be a step behind the
    /// watcher's. The pair is derived from the aim here rather than by the
    /// caller, because a caller deriving it would be the second derivation of
    /// *what is this slot running* that [`Slots`]' own head is about.
    ///
    /// **A slot this handle does not hold is not written**, and nothing is
    /// grown to make room: the row count is the deck's, settled when the run
    /// made the handle, and a handle with no row for this slot is one no server
    /// and no landing is reading — a harness that built an engine and served
    /// nothing.
    ///
    /// **A poisoned lock is recovered rather than dropped**, which is where
    /// this parts company with [`crate::Opening::set`]. What is behind that
    /// lock is a whole pair per slot written in one assignment, so a panic
    /// elsewhere cannot have left half of one; and the safe answer there — a
    /// closed class — has no counterpart here, because *the layout before the
    /// load* is exactly the wrong answer this type exists to stop giving.
    pub fn re_point(&self, slot: usize, at: &karakuri_environment::watch::Aim) {
        let mut held = self.0.write().unwrap_or_else(|held| held.into_inner());
        if let Some(pair) = held.get_mut(slot) {
            *pair = (
                at.head.path.clone(),
                at.rest.iter().map(|node| node.path.clone()).collect(),
            );
        }
    }

    /// How many slots this deck holds, which is what every refusal about a slot
    /// number is measured against ([`crate::no_such_slot`]).
    pub fn count(&self) -> usize {
        self.held().len()
    }

    /// What is published now. Read on every call and never held across one, for
    /// [`crate::Opening::read`]'s reason said about a layout: a slot the
    /// operator loaded a Set onto between two calls has moved for the second.
    fn held(&self) -> std::sync::RwLockReadGuard<'_, Vec<Pointed>> {
        self.0.read().unwrap_or_else(|held| held.into_inner())
    }

    /// A slot's files, each under the layer and the index the rest of this
    /// program addresses it by.
    ///
    /// **The layer is read off the file, not off its position — the head
    /// included.** Every path, the first one as much as the rest, is on the
    /// layer its own `kind` line names, at its position *within that layer*,
    /// keeping file order. It is [`crate::history::declared_kind`] here rather
    /// than a second scanner: two readers of a `kind` line would be two answers
    /// to what layer a file is on, and the layer a version is filed under has
    /// to be the layer an agent addresses it by.
    ///
    /// **The head used to be filed as `L1` whatever it declared**, and nothing
    /// downstream agreed with that: [`crate::compile::sort_compiled`] matches
    /// on the head's own `kind` like every other entry, and `history::seed`
    /// reads the head's `kind` line too. `--set a,b,c` takes whatever the
    /// operator typed first, so a slot headed by a `kind L3` was addressed as
    /// `L1:0` — a `write_procedure` carrying a valid L1 passed the kind guard,
    /// overwrote the camera, and left the slot with a second geometry, no
    /// camera and no way to reach the real L3 at all.
    ///
    /// A text scan and not a parse, for that function's reason: this surface
    /// works without compiling anything, and **a file that does not compile
    /// must still be addressable** — it is the one a model most needs to read.
    /// One that cannot be read, or that declares no `kind`, is counted as a
    /// renderer, which is what `history::seed` makes of it and what the compile
    /// is about to refuse it as.
    ///
    /// Read on every call rather than worked out once at startup. The files are
    /// a handful and nothing here is on a frame path, and a layout cached
    /// beside a directory `--watch` is editing is a layout that can be wrong.
    ///
    /// **The paths come back owned**, because what they are read out of is a
    /// lock this must not hold past the call — see [`Slots::held`].
    fn nodes(&self, slot: usize) -> Result<Vec<(Kind, usize, std::path::PathBuf)>, String> {
        let held = self.held();
        let pair = held
            .get(slot)
            // **The one sentence, from [`crate::no_such_slot`].** This used to
            // be its own spelling — `this deck holds 0-3` against the keys'
            // `this deck holds slots 0-3` — so a model calling `save_set` and
            // an operator pressing a digit were told the same mistake in
            // different words about the same control. It also handled the empty
            // deck for its own reason, which that function has too: `len() - 1`
            // underflowed on an empty deck and took the whole surface with it,
            // the panic unwinding out of the listener thread so that a process
            // which had announced a port was silently no longer on it. `serve`
            // refuses an empty deck now; this stays correct anyway.
            .ok_or_else(|| karakuri_environment::no_such_slot(slot, held.len()))?;
        // **The head's own `kind` line, and `L1` only where it has none.**
        // That fallback is `history::seed`'s, exactly: the first path of a
        // chain with no `kind` in it falls back to L1 and every later one to
        // L4, so a file this scan cannot read is addressed here under the layer
        // its snapshots are filed under there.
        let head = std::fs::read(&pair.0)
            .ok()
            .and_then(|source| karakuri_environment::history::declared_kind(&source))
            .and_then(layer_named)
            .unwrap_or(Kind::L1);
        let mut nodes = vec![(head, 0, pair.0.clone())];
        // The next free index per layer, which the head has already taken one
        // of: a `--set` chain naming a second `kind L1` is a second source, and
        // it is L1 number 1 rather than the beginning of a fresh count.
        let mut next: Vec<(Kind, usize)> = vec![(head, 1)];
        for path in &pair.1 {
            let layer = std::fs::read(path)
                .ok()
                .and_then(|source| karakuri_environment::history::declared_kind(&source))
                .and_then(layer_named)
                .unwrap_or(Kind::L4);
            let index = match next.iter_mut().find(|(held, _)| *held == layer) {
                Some((_, free)) => {
                    let index = *free;
                    *free += 1;
                    index
                }
                None => {
                    next.push((layer, 1));
                    0
                }
            };
            nodes.push((layer, index, path.clone()));
        }
        Ok(nodes)
    }

    /// **Whether this deck holds `slot` at all, without reading a byte off
    /// disk.**
    ///
    /// [`Slots::nodes`] answers this too, and costs a `read` per file of the
    /// slot to do it, because it works each node's layer out of the file's own
    /// `kind` line. `save_set` wants nothing but the range and was calling
    /// `nodes` for it — **under the state mutex**, which is the one lock in this
    /// server a slow disk can be held across, and it is held across every other
    /// connection's request as well. The number was [`Slots::count`] all along.
    fn holds(&self, slot: usize) -> Result<(), String> {
        let count = self.count();
        if slot < count {
            Ok(())
        } else {
            Err(karakuri_environment::no_such_slot(slot, count))
        }
    }

    /// **The file one `(slot, layer, index)` address names, where the layer is
    /// spelled the way a snapshot's name spells it** — one of
    /// [`crate::history::LAYERS`], which is the same list
    /// [`crate::history::declared_kind`] answers with and the same list
    /// [`Slots::nodes`] files a node under.
    ///
    /// **It exists for the panel's landing on a row of the edit history**, and
    /// it delegates rather than repeating: turning an address into a file is
    /// this type's own walk over a slot's `kind` lines, and a second one beside
    /// it would be two answers to *which file is `L4:0` of this slot* — the
    /// mistake [`Slots::nodes`]' own head records under a different name. The
    /// answer follows a library load because this handle does — see
    /// [`Slots::re_point`], which is the same reason the server's own address
    /// resolution follows one.
    ///
    /// A layer word this crate does not write is an `Err` naming it, on the
    /// same terms an address the slot does not hold is.
    pub fn file(
        &self,
        slot: usize,
        layer: &str,
        index: usize,
    ) -> Result<std::path::PathBuf, String> {
        let Some(kind) = layer_named(layer) else {
            return Err(format!(
                "`{layer}` is not a layer: {}",
                LAYERS
                    .iter()
                    .map(|kind| layer_name(*kind))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        };
        self.path(slot, kind, index)
    }

    /// The file one `(slot, layer, index)` address names.
    pub fn path(
        &self,
        slot: usize,
        layer: Kind,
        index: usize,
    ) -> Result<std::path::PathBuf, String> {
        let nodes = self.nodes(slot)?;
        if let Some((_, _, path)) = nodes.iter().find(|(l, i, _)| *l == layer && *i == index) {
            return Ok(path.clone());
        }
        // **What the slot holds, rather than "no such node".** An index past
        // the end and a layer this slot does not use are different mistakes,
        // and a model told which one it made can fix its own call — the same
        // reason the checker's diagnostics come back through here instead of
        // going to a terminal nobody is watching.
        let name = layer_name(layer);
        Err(match nodes.iter().filter(|(l, _, _)| *l == layer).count() {
            0 => format!("slot {slot} holds no {name}: {}", absent(layer)),
            1 => format!("slot {slot} holds one {name} and `index` is {index}"),
            n => format!(
                "slot {slot} holds {n} {name} nodes, so `index` is 0-{}",
                n - 1
            ),
        })
    }
}

/// Every layer a slot's files can be on, in the order they compose.
///
/// **One list, so the enum a client is handed, the address this resolves and
/// the `kind` a written source must declare cannot disagree.** They did: the
/// schema offered `L1` and `L4` alone for as long as a slot could hold five
/// kinds of node, so the most interesting material in the language — the
/// deformations, the camera, the field — was in the files and unreachable from
/// the one surface built for editing them.
///
/// **Five, and [`Kind`] is six.** `kind L5` is a language and a lowering as of
/// M5.16's IR/codegen pass, and it is *not* a layer a slot holds: a frame
/// effect runs in the master chain, the chain is still three fixed passes, and
/// a Set has no L5 node for an `edge` to bind — `compile::sort_compiled`
/// refuses one where it is loaded. Advertising it here would offer a client an
/// address that cannot resolve, which is the same failure this list's own
/// history records from the other direction. **What the sixth kind reaches a
/// model through today is the curriculum**, which renders
/// [`karakuri_ir::builtin::Builtin::ALL`] and the `kind` table from the
/// checker's own tables rather than from a copy — so `texel`, `tap` and
/// `frame_step` are published the day they exist. The pass that gives the chain
/// its slots is what adds the sixth entry here.
pub(crate) const LAYERS: [Kind; 5] = [Kind::L1, Kind::L2, Kind::L3, Kind::L4, Kind::Field];

/// A layer as a client writes it, in the compiler's own `Kind`.
///
/// Case-folded because `l1` is what a model tends to type and refusing it
/// teaches nobody anything. `Field` is spelled as `--param` and `--bind` spell
/// it, which is as the `kind` line does.
///
/// **This name exists twice in this library**, here and as
/// [`crate::setfile::layer_named`], which is exact rather than case-folded.
/// Both moved in from separate binaries under ADR-0215 and the collision was
/// left visible rather than fixed: merging them is a redesign — deciding which
/// spellings the one function accepts — and not a boundary move.
/// `docs/contributing.md` §4 (a name
/// means one thing across the system) is the question to answer.
pub(crate) fn layer_named(name: &str) -> Option<Kind> {
    Some(match name.to_ascii_uppercase().as_str() {
        "L1" => Kind::L1,
        "L2" => Kind::L2,
        "L3" => Kind::L3,
        "L4" => Kind::L4,
        "FIELD" => Kind::Field,
        _ => return None,
    })
}

/// The name back again, for a schema and for a sentence.
///
/// Exhaustive on purpose: a sixth `Kind` should not compile until somebody has
/// decided what this surface calls it and whether [`LAYERS`] offers it.
pub(crate) fn layer_name(layer: Kind) -> &'static str {
    match layer {
        Kind::L1 => "L1",
        Kind::L2 => "L2",
        Kind::L3 => "L3",
        Kind::L4 => "L4",
        Kind::Field => "Field",
        Kind::L5 => "L5",
    }
}

/// **The compiler's layer, in the vocabulary's spelling.**
///
/// One function per list, in the one package that depends on both — which is
/// what `karakuri-operation`'s module documentation prescribes for every list
/// it copies, and what `mix::blend_mode` and its three neighbours already are
/// for the mix. Exhaustive both ways round, so a sixth `Kind` or a sixth
/// `karakuri_operation::Layer` stops the build here until somebody has said
/// what the other one calls it.
///
/// **[`NodeAddress`]'s first caller in this workspace is this surface**, and that is
/// not an accident: the manual's own gap section says MIDI *"cannot express a
/// node address, a parameter name or an id"*, a key press has nothing to say
/// one with, and the panel does not reach inside a Set. A node address is the
/// thing MCP can say and the other three cannot.
pub(crate) fn layer_of(layer: Kind) -> karakuri_operation::Layer {
    match layer {
        Kind::L1 => karakuri_operation::Layer::L1,
        Kind::L2 => karakuri_operation::Layer::L2,
        Kind::L3 => karakuri_operation::Layer::L3,
        Kind::L4 => karakuri_operation::Layer::L4,
        Kind::Field => karakuri_operation::Layer::Field,
        Kind::L5 => karakuri_operation::Layer::L5,
    }
}

/// And back, for the two things that want the compiler's own: resolving an
/// address to a file, and comparing a written source's `kind` line against the
/// address it arrived at.
pub(crate) fn kind_of(layer: karakuri_operation::Layer) -> Kind {
    match layer {
        karakuri_operation::Layer::L1 => Kind::L1,
        karakuri_operation::Layer::L2 => Kind::L2,
        karakuri_operation::Layer::L3 => Kind::L3,
        karakuri_operation::Layer::L4 => Kind::L4,
        karakuri_operation::Layer::Field => Kind::Field,
        karakuri_operation::Layer::L5 => Kind::L5,
    }
}

/// The layers, as a client is told them in a refusal.
pub(crate) fn layer_list() -> String {
    LAYERS
        .iter()
        .map(|layer| layer_name(*layer))
        .collect::<Vec<_>>()
        .join(", ")
}

/// What it means for a slot to hold none of a layer, which is a different thing
/// for each of them: three are optional and two cannot be missing.
pub(crate) fn absent(layer: Kind) -> &'static str {
    match layer {
        // Reachable now that the head is filed under the `kind` it declares:
        // a chain of nothing but a deformation and a renderer holds no
        // geometry, and this is what such a slot is told. It does not get as
        // far as a frame — the build refuses a Set with no L1 — but this
        // surface answers before anything is built.
        Kind::L1 => {
            "a Set needs a geometry, and every file this slot names declares some other layer"
        }
        Kind::L2 => {
            "a deformation is optional, and one is added by naming its file in the same \
             `--set` chain"
        }
        Kind::L3 => "a camera is optional, and a slot without one looks from the built-in orbit",
        Kind::L4 => "a Set needs at least one renderer",
        Kind::Field => {
            "a `kind Field` is optional, and is code the other procedures evaluate rather \
             than a node of its own"
        }
        // **Every slot holds none, and will while the chain is fixed.** A frame
        // effect runs in the master chain rather than in a Set, and the chain is
        // still three hand-written passes — so this is what a slot is told
        // about a kind that compiles and has nowhere to be placed yet.
        Kind::L5 => {
            "a frame effect runs in the master chain rather than in a Set, and the chain is \
             still three fixed passes"
        }
    }
}

/// The largest request body this will read.
///
/// **Checked before anything is allocated**, which is not tidiness: `length`
/// arrives from whoever opened the socket, and `vec![0u8; length]` on a number
/// it cannot serve calls `handle_alloc_error`, which **aborts the process**. It
/// is not catchable and it is not confined to this thread — a fifty-six byte
/// request line took the projector out mid-set. A procedure is a few kilobytes.
const MAX_BODY: usize = 1 << 20;

/// How long one connection may say nothing before it is dropped.
///
/// A crashed client, a browser keep-alive, or a socket that connects and waits
/// used to hold the *only* server thread for the rest of the run. There is a
/// thread per connection now as well, and this is the second half of that fix:
/// threads are cheap, but not if they accumulate forever.
const IDLE: std::time::Duration = std::time::Duration::from_secs(30);

/// How many swap reports may queue for a client that is not asking.
///
/// The queue used to be unbounded, which made "a report on the present" false
/// of everything in front of the bounded part: two hundred thousand events, each
/// holding a `String` allocated on the render thread, materialised in one drain.
/// Dropped rather than queued past this, and **counted**, because a report with
/// a hole in it must say so.
const QUEUED: usize = 256;

/// How many requests of one kind may be waiting for the render loop at once.
///
/// **Per channel, not per loop**: the saves and the edges are counted apart —
/// see [`Reporter::wires`] — so a client waiting on a slow disk cannot use up
/// the room an edit needs to be rewired through.
///
/// [`QUEUED`]'s trade in the other direction, and the same one: a bound, and a
/// refusal rather than a wait when it is reached. The loop takes every request
/// it has on the next frame, so this is only ever full when the loop has
/// stopped running frames — which is a thing to *answer*, because a client
/// queued behind a loop that will never take its request would wait forever.
pub(crate) const ASKED: usize = 16;

/// The longest an `id` a client names may be.
///
/// It becomes part of a file name in the store — `<store>/sandbox/` for a save
/// a model asked for, `<store>/sets/` for the id a read names — and a stamp is
/// twenty characters.
pub(crate) const MAX_ID: usize = 64;

/// **What [`checked_id`] accepts, spelled for a schema.**
///
/// The same rule and not a second one: `checked_id` is still what refuses a
/// bad id, because a client may send anything whatever a schema says, and this
/// is that rule written where a model reads it *before* calling. A charset a
/// tool enforces and does not publish is a refusal a caller had no way to
/// avoid — which is [`kept`]'s argument about `"id": null`, applied to the
/// argument it is about rather than to a null.
///
/// **The one constraint that cannot be published this way is the deck's**: how
/// many slots this run holds is not knowable when `tools/list` is answered, and
/// `minimum` is all a schema can say about `slot`. The upper bound stays a
/// runtime refusal, and it names what the deck holds, which is more than a
/// schema could have said.
pub(crate) const ID_PATTERN: &str = "^[A-Za-z0-9_-]+$";

/// **How long a `save_set` call waits for the render loop before it answers
/// without an outcome.**
///
/// The run's own bound on a save is [`crate::SAVE_WAIT`] — how long quitting
/// will wait for a disk that is not answering — and this is that, plus room for
/// the frame that takes the request and the frame that reports it back. Under
/// the run's own bound it would give up on saves the run itself would still
/// have finished, which is the one number this must not be below.
pub(crate) const SAVE_REPLY: std::time::Duration =
    std::time::Duration::from_secs(karakuri_environment::SAVE_WAIT.as_secs() + 5);

/// **How long a `wire_input` call waits for the render loop to take its edge.**
///
/// Not [`SAVE_REPLY`], and the difference is what is being waited for. A save
/// waits for a *disk*, whose worst case is somebody else's mount; this waits for
/// the loop to reach the top of a frame, apply the edge and say so — which is
/// one frame at any frame rate anybody plays at, and this many seconds is room
/// for a loop that is busy rather than an estimate of the work.
///
/// **It is not a wait for the build.** What the rebuild made of the edge lands
/// thirty judged frames later and is `swap_outcome`'s answer, exactly as it is
/// for a written procedure. A tool that waited for that would hold a connection
/// open across a transition to tell a model something a second call can ask
/// for.
pub(crate) const WIRE_REPLY: std::time::Duration = std::time::Duration::from_secs(5);

/// Serve MCP on `port`, loopback only, until the process ends.
///
/// Returns the [`Reporter`] the render loop keeps. The listener and everything
/// behind it live on threads of their own; nothing here is ever called from a
/// frame.
///
/// **`store` is a root and not an open [`Store`]**, which is the same decision
/// [`Slots`] makes about a path and for a milder version of the same reason.
/// Opening here would fail a whole run for a library nothing has asked for yet,
/// and would hold one answer to "where is the store" against a directory the
/// operator is free to move; opening per call is four `create_dir_all`s off a
/// frame path, on a surface where the expensive thing is already a compile.
///
/// **`slots` is a live handle and is *shared* rather than moved**, exactly as
/// `opening` beside it is: the host goes on writing it every time it re-points
/// a slot, and this server resolves through it on every call. A run that hands
/// this a layout taken at launch and then loads a Set onto a deck answers a
/// model about the material it stopped running — see [`Slots`].
pub fn serve(
    port: u16,
    slots: Slots,
    store: std::path::PathBuf,
    watching: bool,
    opening: karakuri_environment::Opening,
) -> Result<Reporter, String> {
    if slots.count() == 0 {
        return Err("this run has no procedure files to serve — see `--load-set`".into());
    }
    let listener = std::net::TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("port {port}: {e}"))?;
    // Asked back rather than echoed: `--mcp 0` binds an ephemeral port, and
    // printing the 0 tells the operator a port that is not the port.
    let bound = listener
        .local_addr()
        .map_err(|e| format!("port {port}: {e}"))?;
    let (tx, rx) = mpsc::sync_channel(QUEUED);
    // The other direction, made here for the same reason: the render loop is
    // handed one half of everything it shares with this server, once, before a
    // frame has run.
    let (asked, requests) = mpsc::sync_channel(ASKED);
    // The edges, on a channel of their own — see [`Reporter::wires`].
    let (wiring, wires) = mpsc::sync_channel(ASKED);
    // And the operations, on a third — see [`Reporter::operations`].
    let (operating, operations) = mpsc::sync_channel(ASKED);

    let state = std::sync::Arc::new(std::sync::Mutex::new(State {
        slots,
        store,
        watching,
        opening,
        events: rx,
        asked,
        wiring,
        operating,
        recent: Vec::new(),
        dropped: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
    }));
    let dropped = state.lock().expect("fresh mutex").dropped.clone();

    std::thread::Builder::new()
        .name("mcp".into())
        .spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let state = state.clone();
                // **A thread per connection.** One thread for the listener and
                // every conversation meant a socket that said nothing wedged
                // the surface for the rest of the run — and a panic inside it
                // dropped the listener, leaving a process that had announced a
                // port and was no longer on it.
                let spawned =
                    std::thread::Builder::new()
                        .name("mcp-conn".into())
                        .spawn(move || {
                            if let Err(e) = handle(stream, &state) {
                                eprintln!("mcp: {e}");
                            }
                        });
                if spawned.is_err() {
                    eprintln!("mcp: could not start a thread for a connection");
                }
            }
        })
        .map_err(|e| format!("starting the mcp thread: {e}"))?;

    Ok(Reporter {
        sender: tx,
        requests,
        wires,
        operations,
        dropped,
        port: bound.port(),
    })
}

pub(crate) struct State {
    /// **Where each slot's procedures are *now***, read through on every call
    /// and never copied out — see [`Slots`], whose head is the whole of why
    /// this is a handle. A layout this server held would be the launch one
    /// forever, and a library load re-points a slot at new scratch files.
    pub(crate) slots: Slots,
    /// **Where the library lives** — `--store DIR`, the same root every other
    /// half of this run reads and writes. A root rather than an open [`Store`];
    /// see [`serve`].
    pub(crate) store: std::path::PathBuf,
    /// Whether `--watch` is on. Without it a written procedure sits on disk and
    /// changes nothing, which a model has no way to discover and every reason
    /// to be told.
    pub(crate) watching: bool,
    /// **Which classes the operator has opened**, read on every call — see
    /// [`crate::Opening`] and [`audited`]. A parameter of [`serve`] and not a
    /// value this module chooses: a server that decided its own opening would
    /// be the surface holding the authority, which is the shape
    /// `docs/principles/0090-a-surface-offers-it-never-decides.md`
    /// rules out. **Nor can a caller forget it**: it is positional, so a run
    /// that does not mention an opening does not compile.
    pub(crate) opening: karakuri_environment::Opening,
    pub(crate) events: mpsc::Receiver<Event>,
    /// Where a save a client asks for goes. **Bounded and never blocked on** —
    /// see [`ASKED`]: this is sent into from a connection thread, and a render
    /// loop that has stopped taking requests must produce an answer rather than
    /// a thread that never returns.
    pub(crate) asked: mpsc::SyncSender<SaveRequest>,
    /// And where an edge a client asks for goes, on the same terms and for the
    /// same reasons — see [`WireRequest`] and [`Reporter::wires`].
    pub(crate) wiring: mpsc::SyncSender<WireRequest>,
    /// And where an operation a client asks for goes, on the same terms again —
    /// see [`OperateRequest`] and [`Reporter::operations`]. **The audit has
    /// already run** when something is put here: [`audited`] is between
    /// [`asked`] and [`perform`], and nothing else constructs an
    /// [`OperateRequest`].
    pub(crate) operating: mpsc::SyncSender<OperateRequest>,
    /// What the swap machinery has said, newest last, bounded.
    pub(crate) recent: Vec<String>,
    pub(crate) dropped: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

/// How much swap history is kept. Enough for a model to see what its own last
/// write did and no more: this is a report on the present, not a log.
const RECENT: usize = 32;

impl State {
    pub(crate) fn drain(&mut self) {
        while let Ok(Event::Swap { slot, said }) = self.events.try_recv() {
            self.recent.push(format!("slot {slot}: {said}"));
        }
        if self.recent.len() > RECENT {
            self.recent.drain(..self.recent.len() - RECENT);
        }
    }
}

// -- the transport ---------------------------------------------------------

/// One connection: read requests, answer them, keep it open.
///
/// **Hand-rolled, and what that costs is worth stating rather than assuming.**
/// The first version argued it was fine because this is "loopback, from one
/// client". Loopback is not a boundary — any process on the machine reaches it,
/// and so does a `fetch()` from any web page the operator happens to have open,
/// because a POST with a plain content type needs no preflight. "One client" was
/// an assumption about the *good* client and nothing enforced it. What enforces
/// anything now: an `Origin` check, a body cap before any allocation, a read
/// timeout, and a thread per connection.
fn handle(stream: std::net::TcpStream, state: &std::sync::Mutex<State>) -> Result<(), String> {
    stream.set_nodelay(true).ok();
    stream.set_read_timeout(Some(IDLE)).ok();
    let mut reader = std::io::BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut writer = stream;

    loop {
        let mut line = String::new();
        if read_capped(&mut reader, &mut line)? == 0 {
            return Ok(()); // the client hung up
        }
        let mut request_line = line.split_whitespace();
        let method = request_line.next().unwrap_or("").to_string();
        // **The path, which the first version threw away.** It answered 405 to
        // every request whatever it asked for, and a client's auth discovery
        // asks for `/.well-known/oauth-protected-resource` before it does
        // anything else. 405 says "that exists, but not by this verb", so the
        // client concluded there was protected-resource metadata to fetch and
        // went looking for it — then failed parsing `this server only answers
        // POST` as JSON. The whole handshake died on a path this server has
        // never had.
        let target = request_line.next().unwrap_or("").to_string();

        let mut length: Option<usize> = None;
        let mut origin: Option<String> = None;
        loop {
            let mut header = String::new();
            if read_capped(&mut reader, &mut header)? == 0 {
                return Ok(());
            }
            let header = header.trim_end();
            if header.is_empty() {
                break;
            }
            let Some((name, value)) = header.split_once(':') else {
                // A line with no colon is not a header. Refusing the whole
                // request beats guessing what was meant.
                return respond(&mut writer, 400, "text/plain", b"malformed header");
            };
            // **Case-insensitively**, because field names are, and because
            // `CONTENT-LENGTH:` is lawful and used to be read as a body of zero
            // — after which the client was told its JSON was malformed and its
            // bytes were reparsed as headers.
            match name.trim().to_ascii_lowercase().as_str() {
                "content-length" => match value.trim().parse::<usize>() {
                    Ok(n) => length = Some(n),
                    Err(_) => {
                        return respond(
                            &mut writer,
                            400,
                            "text/plain",
                            b"content-length is not a number",
                        )
                    }
                },
                "origin" => origin = Some(value.trim().to_string()),
                _ => {}
            }
        }

        // **The check the first version did not have.** A page on any site can
        // POST here; it cannot read the reply, and `write_procedure` does not
        // need to be read to have happened. A client that is genuinely local
        // sends no `Origin` at all.
        if let Some(origin) = &origin {
            if !is_local_origin(origin) {
                return respond(
                    &mut writer,
                    403,
                    "text/plain",
                    b"this server does not answer cross-origin requests",
                );
            }
        }

        // **Path before method**, because "no such thing here" and "not by that
        // verb" are different answers and only one of them is true of a path
        // this server does not serve. A 404 is what tells a client there is no
        // authorization metadata to find, which is how a server with no auth
        // says so.
        let path = target.split(['?', '#']).next().unwrap_or("");
        if path != ENDPOINT {
            return respond(
                &mut writer,
                404,
                "application/json",
                br#"{"error":"no such path: this server serves MCP at / and nothing else"}"#,
            );
        }

        if method != "POST" {
            // 405 rather than 404 *here*: the path is real, and the Streamable
            // HTTP transport says a server offering no SSE stream at its
            // endpoint answers GET with exactly this. Answered and closed
            // rather than answered and continued: the body of a non-POST was
            // left in the reader, so it became the next request line and ran.
            // Closing cannot be smuggled through.
            //
            // JSON rather than plain text because a client that reached here
            // is a client parsing JSON — the same reason the 404 above carries
            // a body it can read.
            return respond(
                &mut writer,
                405,
                "application/json",
                br#"{"error":"this endpoint answers POST only: there is no SSE stream here"}"#,
            );
        }

        let Some(length) = length else {
            return respond(
                &mut writer,
                411,
                "text/plain",
                b"a POST needs a content-length",
            );
        };
        if length > MAX_BODY {
            return respond(&mut writer, 413, "text/plain", b"that body is too large");
        }

        let mut body = vec![0u8; length];
        reader.read_exact(&mut body).map_err(|e| e.to_string())?;

        let reply = match serde_json::from_slice::<Value>(&body) {
            Ok(request) => {
                let mut state = state.lock().map_err(|_| "the mcp state is poisoned")?;
                let pending = dispatch(&request, &mut state);
                // **The lock, let go before anything is waited for.** There is
                // a thread per connection and one mutex over the state, so a
                // `save_set` that waited for the render loop and the disk here
                // would hold up every other connection for as long as it took —
                // including a client that only wanted to read a procedure, and
                // including the client that would have asked what the swap did.
                // Dropped by name rather than by a scope, because a scope is a
                // thing somebody widens later without noticing what it was for.
                drop(state);
                pending.settled()
            }
            Err(e) => Some(error(&Value::Null, -32700, &format!("parse error: {e}"))),
        };
        match reply {
            Some(reply) => {
                let bytes = serde_json::to_vec(&reply).map_err(|e| e.to_string())?;
                respond(&mut writer, 200, "application/json", &bytes)?;
            }
            // A notification: answered with 202 and no body, which is what the
            // protocol asks for.
            None => respond(&mut writer, 202, "text/plain", b"")?,
        }
    }
}

/// A header or request line, refused past [`MAX_BODY`] rather than grown.
///
/// `read_line` has no cap, so a line with no newline in it is a second way to
/// exhaust memory — quieter than a huge `Content-Length` and the same ending.
fn read_capped(reader: &mut impl BufRead, into: &mut String) -> Result<usize, String> {
    let mut taken = std::io::Read::take(reader.by_ref(), MAX_BODY as u64);
    let read = taken
        .read_line(into)
        .map_err(|e| format!("reading a header: {e}"))?;
    if read >= MAX_BODY {
        return Err("a request line was longer than this server will read".into());
    }
    Ok(read)
}

/// Whether an `Origin` is one this server will answer.
///
/// Only the loopback names, and only because a browser sends `Origin` on every
/// cross-site request while a real client sends none at all.
fn is_local_origin(origin: &str) -> bool {
    let rest = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
        .unwrap_or(origin);
    let host = rest.split(':').next().unwrap_or("");
    matches!(host, "localhost" | "127.0.0.1" | "[::1]" | "::1")
}

fn respond(
    writer: &mut std::net::TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> Result<(), String> {
    // The reason, not "OK" after every code. `HTTP/1.1 405 OK` was on the wire.
    let reason = match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        411 => "Length Required",
        413 => "Payload Too Large",
        _ => "Error",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n",
        body.len()
    );
    writer
        .write_all(head.as_bytes())
        .map_err(|e| e.to_string())?;
    writer.write_all(body).map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())
}

// -- the protocol ----------------------------------------------------------

/// The version of MCP this speaks.
pub const PROTOCOL: &str = "2024-11-05";

/// **A reply, or the one step of a reply that must happen with [`State`]
/// unlocked.**
///
/// This type is the whole of the concurrency design, so it is worth stating
/// plainly what it buys. `handle` locks the state around [`dispatch`], and there
/// is a thread per connection: anything waited for under that lock is waited for
/// by every other client too. Five of the six tools are a file read or a file
/// write and finish under it — `list_sets` is the widest of them, a directory
/// read plus a set file each and a card for each node those files left unnamed,
/// which is a bounded count of reads off the store rather than a wait on
/// anybody else's thread, and is why it is capped and why it compiles nothing;
/// `read_set` is the same shape over one set. `save_set` waits for a
/// render loop and then for a disk, which is unbounded in the only sense that
/// matters — it depends on somebody else's frame rate.
///
/// So the send happens under the lock, where the channel is, and the *wait*
/// comes back out here. Returning a value that still has work in it is the
/// smallest thing that makes the boundary visible: a comment saying "do not
/// wait here" would be a comment.
///
/// **`wire_input` is the second thing that waits**, and it waits for a frame
/// rather than for a disk — which is shorter and is still somebody else's
/// thread, so it belongs out here for exactly the same reason.
enum Pending {
    /// Nothing left to do. `None` is a notification, which is answered with no
    /// body at all.
    Done(Option<Value>),
    /// A save the render loop has been asked for, and the JSON-RPC id it is
    /// answered under.
    Saving {
        id: Value,
        news: mpsc::Receiver<News>,
    },
    /// An edge the render loop has been asked to write.
    ///
    /// **A variant of its own rather than a second `Saving`**, because the two
    /// wait different lengths for differently shaped news — see [`WIRE_REPLY`]
    /// against [`SAVE_REPLY`], and [`applied`] against [`awaited`].
    /// An operation the render loop has been asked to perform.
    ///
    /// **[`Pending::Wiring`]'s shape with no note**, and it waits with
    /// [`applied`] for that variant's reason: the loop performs it at the frame
    /// it takes it and answers there, and everything slow that an operation
    /// starts — a rebuild, a transition, a save — happens after the answer and
    /// is reported where it lands.
    Operating {
        id: Value,
        news: mpsc::Receiver<News>,
    },
    Wiring {
        id: Value,
        news: mpsc::Receiver<News>,
        /// What this server knows about the run that the loop's own sentence
        /// will not say — today, that a run without `--watch` has no watcher to
        /// rebuild the slot. Built under the lock, where [`State`] is; appended
        /// to an answer the loop wrote, because it is a fact about the run
        /// rather than about the edge.
        note: String,
    },
}

impl Pending {
    /// The reply, waiting for the render loop if that is what is left.
    ///
    /// **Called with the state unlocked**, which is the entire reason this type
    /// exists — see above.
    fn settled(self) -> Option<Value> {
        match self {
            Pending::Done(reply) => reply,
            Pending::Saving { id, news } => Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": tool_result(awaited(&news, SAVE_REPLY)),
            })),
            Pending::Operating { id, news } => Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": tool_result(applied(&news, OPERATE_REPLY)),
            })),
            // **The note is appended to what the loop said and only where the
            // loop said it worked.** A refusal is the loop's whole sentence;
            // adding "and by the way this run does not rebuild" to it would put
            // two answers in front of a model that has one mistake to fix.
            Pending::Wiring { id, news, note } => Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": tool_result(applied(&news, WIRE_REPLY).map(|said| {
                    if note.is_empty() {
                        said
                    } else {
                        format!("{said}\n\n{note}")
                    }
                })),
            })),
        }
    }
}

fn dispatch(request: &Value, state: &mut State) -> Pending {
    let id = request.get("id").cloned();
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    // **No `id` is a notification: never answered, and this server acts on none
    // of them.** The comment here used to say "acted on", which was false —
    // nothing below this line runs — and the test asserting `is_none()` could
    // not tell the difference. There is nothing a notification asks of this
    // server today; when there is, it goes above this line.
    let Some(id) = id else {
        return Pending::Done(None);
    };

    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL,
            "capabilities": { "tools": {}, "resources": {} },
            "serverInfo": { "name": "karakuri", "version": env!("CARGO_PKG_VERSION") },
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools() })),
        "resources/list" => Ok(json!({ "resources": resources() })),
        "resources/read" => read_resource(request).map_err(Refused::BadParams),
        "tools/call" => match call_tool(request, state) {
            // **Out from under the lock before it is waited for.** See
            // [`Pending`]; this `return` is the only thing carrying that
            // decision, so it is the one line here worth reading twice.
            Ok(Called::Saving(news)) => return Pending::Saving { id, news },
            Ok(Called::Wiring { news, note }) => return Pending::Wiring { id, news, note },
            Ok(Called::Operating(news)) => return Pending::Operating { id, news },
            Ok(Called::Answered(outcome)) => Ok(tool_result(outcome)),
            Err(e) => Err(Refused::BadParams(e)),
        },
        other => Err(Refused::NoMethod(format!("no method `{other}`"))),
    };

    Pending::Done(Some(match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        // **The code says which kind of wrong.** Everything used to come back
        // as "method not found", so a client could not tell a method it had
        // invented from arguments it had got wrong.
        Err(Refused::NoMethod(m)) => error(&id, -32601, &m),
        Err(Refused::BadParams(m)) => error(&id, -32602, &m),
    }))
}

/// Why a request could not be answered, in the two shapes JSON-RPC has codes
/// for.
enum Refused {
    NoMethod(String),
    BadParams(String),
}

fn error(id: &Value, code: i32, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

mod resources;
pub(crate) use resources::*;

#[cfg(test)]
mod tests;
