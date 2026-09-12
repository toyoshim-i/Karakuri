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

use karakuri_ir::typed::Checked;
use karakuri_ir::Kind;
// **The vocabulary this surface names its operations in.** Not a dependency
// this module performs anything through — see [`asked`] — but the one list of
// what an operation *is*, so that a tool and the manual's row for it cannot
// drift apart.
//
// `karakuri_operation::Layer` is spelled in full everywhere below, because
// `Layer` in this file is already `karakuri_store::record::Layer` and a name
// means one thing across the system
// (`docs/contributing.md` §4). That
// there are three spellings of one list — the compiler's `Kind`, the record's
// `Layer` and the vocabulary's — is the cost `karakuri-operation` states it
// pays on purpose, and this package is where two of them are checked against
// each other.
use karakuri_operation::gate::{self, Allowed};
use karakuri_operation::{InputPort, NodeAddress, Operation};
use karakuri_store::hash::Hash;
use karakuri_store::ndjson::Line;
use karakuri_store::record::{Layer, Record};
use karakuri_store::store::{Store, StoreError};
use serde_json::{json, Value};
mod spelled;
pub(crate) use spelled::*;

pub use karakuri_ir::{Diagnostic, DiagnosticReport};

/// Check a procedure source against the full IR pipeline (parse, type, contract, cost)
/// and return a machine-readable `DiagnosticReport`.
pub fn check_procedure(source: &str) -> DiagnosticReport {
    let mut diagnostics = Vec::new();
    match karakuri_ir::parse(source) {
        Err(errs) => {
            diagnostics.extend(errs.iter().map(|e| Diagnostic::from_ir_error(e, source)));
        }
        Ok(proc) => match karakuri_ir::check::check(&proc) {
            Err(errs) => {
                diagnostics.extend(errs.iter().map(|e| Diagnostic::from_ir_error(e, source)));
            }
            Ok(checked) => {
                if let Err(errs) = karakuri_ir::cost::estimate(&checked) {
                    diagnostics.extend(errs.iter().map(|e| Diagnostic::from_ir_error(e, source)));
                }
            }
        },
    }
    let success = diagnostics.is_empty();
    DiagnosticReport {
        diagnostics,
        success,
    }
}

/// Verify a set configuration in the store and return a `DiagnosticReport`.
pub fn check_set_configuration(store_path: &std::path::Path, id: &str) -> DiagnosticReport {
    let store = match Store::open(store_path) {
        Ok(s) => s,
        Err(e) => {
            return DiagnosticReport {
                diagnostics: vec![Diagnostic {
                    code: "KIR-E501-STORE-OPEN-FAILED".to_string(),
                    message: format!("cannot open store at `{}`: {e}", store_path.display()),
                    line: None,
                    column: None,
                    remedy: Some(
                        "Verify the store directory path exists and has correct permissions."
                            .to_string(),
                    ),
                }],
                success: false,
            };
        }
    };
    match store.read_set(id) {
        Ok(_) => DiagnosticReport::ok(),
        Err(e) => DiagnosticReport {
            diagnostics: vec![Diagnostic {
                code: "KIR-E502-SET-READ-FAILED".to_string(),
                message: format!("failed to read set `{id}`: {e}"),
                line: None,
                column: None,
                remedy: Some(
                    "Check that the set ID is correctly spelled and saved in the library."
                        .to_string(),
                ),
            }],
            success: false,
        },
    }
}

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
pub struct Reply(mpsc::Sender<News>);

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
enum News {
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
const LAYERS: [Kind; 5] = [Kind::L1, Kind::L2, Kind::L3, Kind::L4, Kind::Field];

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
fn layer_named(name: &str) -> Option<Kind> {
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
fn layer_name(layer: Kind) -> &'static str {
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
fn layer_of(layer: Kind) -> karakuri_operation::Layer {
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
fn kind_of(layer: karakuri_operation::Layer) -> Kind {
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
fn layer_list() -> String {
    LAYERS
        .iter()
        .map(|layer| layer_name(*layer))
        .collect::<Vec<_>>()
        .join(", ")
}

/// What it means for a slot to hold none of a layer, which is a different thing
/// for each of them: three are optional and two cannot be missing.
fn absent(layer: Kind) -> &'static str {
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
const ASKED: usize = 16;

/// The longest an `id` a client names may be.
///
/// It becomes part of a file name in the store — `<store>/sandbox/` for a save
/// a model asked for, `<store>/sets/` for the id a read names — and a stamp is
/// twenty characters.
const MAX_ID: usize = 64;

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
const ID_PATTERN: &str = "^[A-Za-z0-9_-]+$";

/// **How long a `save_set` call waits for the render loop before it answers
/// without an outcome.**
///
/// The run's own bound on a save is [`crate::SAVE_WAIT`] — how long quitting
/// will wait for a disk that is not answering — and this is that, plus room for
/// the frame that takes the request and the frame that reports it back. Under
/// the run's own bound it would give up on saves the run itself would still
/// have finished, which is the one number this must not be below.
const SAVE_REPLY: std::time::Duration =
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
const WIRE_REPLY: std::time::Duration = std::time::Duration::from_secs(5);

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

struct State {
    /// **Where each slot's procedures are *now***, read through on every call
    /// and never copied out — see [`Slots`], whose head is the whole of why
    /// this is a handle. A layout this server held would be the launch one
    /// forever, and a library load re-points a slot at new scratch files.
    slots: Slots,
    /// **Where the library lives** — `--store DIR`, the same root every other
    /// half of this run reads and writes. A root rather than an open [`Store`];
    /// see [`serve`].
    store: std::path::PathBuf,
    /// Whether `--watch` is on. Without it a written procedure sits on disk and
    /// changes nothing, which a model has no way to discover and every reason
    /// to be told.
    watching: bool,
    /// **Which classes the operator has opened**, read on every call — see
    /// [`crate::Opening`] and [`audited`]. A parameter of [`serve`] and not a
    /// value this module chooses: a server that decided its own opening would
    /// be the surface holding the authority, which is the shape
    /// `docs/principles/0090-a-surface-offers-it-never-decides.md`
    /// rules out. **Nor can a caller forget it**: it is positional, so a run
    /// that does not mention an opening does not compile.
    opening: karakuri_environment::Opening,
    events: mpsc::Receiver<Event>,
    /// Where a save a client asks for goes. **Bounded and never blocked on** —
    /// see [`ASKED`]: this is sent into from a connection thread, and a render
    /// loop that has stopped taking requests must produce an answer rather than
    /// a thread that never returns.
    asked: mpsc::SyncSender<SaveRequest>,
    /// And where an edge a client asks for goes, on the same terms and for the
    /// same reasons — see [`WireRequest`] and [`Reporter::wires`].
    wiring: mpsc::SyncSender<WireRequest>,
    /// And where an operation a client asks for goes, on the same terms again —
    /// see [`OperateRequest`] and [`Reporter::operations`]. **The audit has
    /// already run** when something is put here: [`audited`] is between
    /// [`asked`] and [`perform`], and nothing else constructs an
    /// [`OperateRequest`].
    operating: mpsc::SyncSender<OperateRequest>,
    /// What the swap machinery has said, newest last, bounded.
    recent: Vec<String>,
    dropped: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

/// How much swap history is kept. Enough for a model to see what its own last
/// write did and no more: this is a report on the present, not a log.
const RECENT: usize = 32;

impl State {
    fn drain(&mut self) {
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

// -- tools -----------------------------------------------------------------

fn tools() -> Value {
    // The layers, from [`LAYERS`] rather than written out beside it. A client
    // is offered exactly what [`layer_named`] accepts and what [`Slots::path`]
    // resolves, because it is the same list — the drift this closes is the one
    // that left three of a slot's five layers unaddressable while the files
    // were sitting right there.
    let layers: Vec<&str> = LAYERS.iter().map(|layer| layer_name(*layer)).collect();
    json!([
        {
            "name": "read_procedure",
            "description":
                "The source of one node of one deck slot. `layer` says which: L1 is what \
                 the elements are and how they move, L2 a deformation applied to them, L3 \
                 the camera, L4 how they are drawn, and Field a distance function the \
                 others evaluate. Read before writing: the edit is usually small, and what \
                 is already there is the best guide to the language.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "slot": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "deck slot, from 0",
                    },
                    "layer": { "type": "string", "enum": layers },
                    "index": {
                        "type": "integer",
                        "minimum": 0,
                        "description":
                            "which node of that layer, from 0, in the order the slot's files \
                             were named. A slot draws with as many L4s as it likes — the same \
                             cloud as sprites and as strokes is one slot with two — and may \
                             simulate with more than one L1, look from more than one L3 and \
                             hold more than one Field. Omit for the first.",
                    },
                },
                "required": ["slot", "layer"],
            },
        },
        {
            "name": "write_procedure",
            "description":
                "Check a procedure and, if it compiles, write it. It is then compiled on a \
                 worker thread, swapped in at a frame boundary, and measured for thirty \
                 frames — if it costs more than the frame budget it is dropped and the \
                 previous one comes back at the time it was parked at. So an expensive \
                 mistake is survivable. **The diagnostics are the point of the return \
                 value**: if it does not compile, what comes back is what the checker \
                 said, against the source. **The check is of this procedure alone, and \
                 the slot is rebuilt whole**: everything between nodes — what a renderer \
                 consumes against what a geometry emits, what each `uses` slot is bound \
                 to, the capacities, the cost of a field inlined into its caller — is \
                 decided when the slot is assembled, so a clean write can still be \
                 followed by a build failure `swap_outcome` reports. **A `uses` \
                 declaration needs an `edge`, and `wire_input` writes one**: adding \
                 `uses <name> : <Geometry|Field|Camera|Source>` to a procedure leaves \
                 the slot unable to build until an edge says which node fills it, so \
                 write the procedure and then call `wire_input`. The builds between \
                 the two are refusals `swap_outcome` reports, and what was on air \
                 stays on air through them. **Taking one back is the half that is \
                 missing**: nothing here unbinds an edge, so a procedure rewritten \
                 without a `uses` it had been wired for leaves an edge naming a slot \
                 nothing declares, and the slot refuses to build for that instead.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "slot": { "type": "integer", "minimum": 0 },
                    "layer": { "type": "string", "enum": layers },
                    "index": {
                        "type": "integer",
                        "minimum": 0,
                        "description":
                            "which node of that layer, from 0. Omit for the first. The \
                             source's own `kind` line must name the same layer as this \
                             address, which is what stops a deformation being written over \
                             a renderer.",
                    },
                    "source": { "type": "string", "description": "the whole procedure" },
                },
                "required": ["slot", "layer", "source"],
            },
        },
        {
            "name": "wire_input",
            "description":
                "Bind one node's declared input to another node of the same deck slot \
                 — the `edge` a `uses` declaration needs before the slot can build. A \
                 procedure declares each input under a name of its own (`uses far : \
                 Geometry`, `uses shape : Field`, `uses view : Camera`, `uses only : \
                 Source`) and never names the node that fills it; the Set says that, \
                 and this is how it is said. **Both ends are node names, not \
                 addresses**: `read_procedure`'s `layer` and `index` are a position, \
                 and a position moves when a slot's files are reordered — which would \
                 silently change which geometry a morph blends towards. A node is \
                 called what the Set named it, or what its own procedure calls itself \
                 where nothing named it; `read_set` and `list_sets` answer in those \
                 names. **An edge already binding this input is replaced**, and every \
                 other edge is left alone — so changing your mind is one call and \
                 never a refusal about a slot being bound twice. **Nothing here \
                 unbinds one**: an edge outlives the `uses` that needed it, so a \
                 procedure rewritten without a `uses` it was wired for leaves an edge \
                 naming a slot nothing declares, and the slot refuses to build for \
                 that instead. **The names are the Set's and this server cannot check \
                 them**: a node the Set does not hold, a slot the node does not \
                 declare, or a far end of the wrong kind is refused where the slot is \
                 built — in the same sentence `--edge` meets — and comes back through \
                 `swap_outcome` with everything else a rebuild decided. The rebuild \
                 itself is a procedure write's: compiled on a worker thread, swapped \
                 at a frame boundary, judged on that Set's own measured frame, and \
                 left in the slot with the slot stopped if it costs too much.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "slot": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "deck slot, from 0: whose Set this edge is about",
                    },
                    "node": {
                        "type": "string",
                        "description":
                            "the node that declares the input, by name — `morph` in \
                             `--edge morph.far=sphere_shell`",
                    },
                    "input": {
                        "type": "string",
                        "description":
                            "what that node's procedure calls the input — `far`, from \
                             `uses far : Geometry`. It is `input` and not `slot` \
                             because `slot` means the deck slot in every tool here, \
                             and one word means one thing across this surface.",
                    },
                    "to": {
                        "type": "string",
                        "description":
                            "the node bound to it, by name — `sphere_shell`. A slot \
                             declared `: Geometry` or `: Source` takes an L1, `: \
                             Field` a Field, `: Camera` an L3 or the built-in camera; \
                             a far end of the wrong kind is refused where the slot is \
                             built and names what the Set holds.",
                    },
                },
                "required": ["slot", "node", "input", "to"],
            },
        },
        {
            "name": "swap_outcome",
            "description":
                "What the swap machinery has said recently: whether a written procedure \
                 landed, was overloaded, or failed to build. Call it after a \
                 write to find out what happened — a write returning cleanly means it \
                 compiled, not that it is on screen. **Overloaded is a state rather \
                 than an outcome**: one frame of that Set costs more than a frame may, \
                 so it is still in the slot and the slot has stopped updating — it \
                 takes no step and draws no frame, and the picture holds the last frame \
                 it drew. Nothing puts anything back and nothing ends it on its own: \
                 the ways out are a write that fits, an earlier version landed on the \
                 node, or the operator's hand on the fader. So a procedure this names \
                 is one worth writing again, cheaper.",
            "inputSchema": { "type": "object", "properties": {} },
        },
        {
            "name": "save_set",
            "description":
                "Keep what a slot is playing, as a Set file. It writes the material \
                 **on screen** — the versions the \
                 slot is running, by content hash, with the whole of the wiring and the \
                 state around them: the parameters, the capacities, the bindings and the \
                 seeds the live Set holds now, the edges binding each `uses` slot, \
                 whether it composites and which renderer is live, and the camera — and \
                 not what any file on disk says, which is exactly what the operator's \
                 `k` key writes. That distinction is the \
                 point: a procedure that was written and never picked up is on disk and \
                 not on screen, and this saves the screen. **What you save goes \
                 into the store's sandbox, `<store>/sandbox/`, and not into the \
                 operator's library**: the library is written by the operator's own act \
                 and nothing else, and what lands in the sandbox is the edit history of \
                 this session — the files a person goes looking for afterwards. So \
                 `read_set`, `list_sets` and `--load-set` do not reach what you write \
                 here; if the operator wants one of these in their library they save it \
                 themselves, or move the file. **It waits for the \
                 disk and tells you what happened**, so what comes back names the id it \
                 was saved under; do not report a set as kept until it does, and use the \
                 id it gives you rather than the one you asked for.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "slot": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "deck slot, from 0",
                    },
                    "id": {
                        "type": "string",
                        "pattern": ID_PATTERN,
                        "maxLength": MAX_ID,
                        "description":
                            "what to call it: letters, digits, `-` and `_`, and it \
                             becomes part of a file name. **Nothing in the sandbox is \
                             overwritten**, so what you save is filed under the moment it \
                             was saved with your name behind it — `20260905-143052-271_my_take` \
                             — and two saves under one name are two files rather than one. \
                             That is the point of the directory: it is a session's edit \
                             history, and a snapshot a later snapshot can replace is not \
                             one. Omit it and the set is named after the moment alone, \
                             which is what the operator's key press gets. **The answer \
                             names the id the file was actually written under**; use that \
                             one.",
                    },
                },
                "required": ["slot"],
            },
        },
        {
            "name": "read_set",
            "description":
                "What a saved Set holds, and what each procedure in it declares — read \
                 out of the library without loading anything and without compiling \
                 anything. A Set is a slot's material kept under a name: the operator's \
                 `k` key writes one and `--load-set ID` plays one back. **This reads the \
                 operator's library and not the sandbox `save_set` writes into**, so a \
                 set you kept yourself is not here. For every node this says which layer it is on — L1 is what \
                 the elements are and how they move, L2 a deformation, L3 the camera, \
                 L4 how they are drawn, Field a distance function the others evaluate — \
                 what the procedure calls itself, and what it *declares*: each \
                 parameter with the two numbers a value must lie between and the value \
                 it takes when nothing turns it; the element count an L1 may run at, \
                 lowest, highest and the count it runs at unless a Set says otherwise; \
                 and the attributes it emits, which are what a renderer drawn over it \
                 can consume. **Ranges are declarations, not settings**: a range says \
                 what a value will be refused outside of, not where this Set has it. \
                 Call it to choose between things you have kept, and to find out what \
                 there is to turn on one, without fetching its source and compiling it.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "pattern": ID_PATTERN,
                        "maxLength": MAX_ID,
                        "description":
                            "the id the set was filed under: letters, digits, `-` and \
                             `_`. It is what `save_set` came back naming, what \
                             `--save-set ID` was given, or the stamp a save that named \
                             nothing was called after.",
                    },
                },
                "required": ["id"],
            },
        },
        {
            "name": "list_sets",
            "description":
                "What this store holds: every Set saved into it, most recently written \
                 first, with an address and a name per node. A Set is a slot's material \
                 kept under a name — the operator's `k` key writes one and `--load-set \
                 ID` plays one back; this is **the operator's library and not the \
                 sandbox `save_set` writes into** — and until this there was no \
                 way to find out what had been kept: `read_set` answers about an id you \
                 already have, and the ids of everything saved before this conversation \
                 are not something a model can guess. **Call this first, then `read_set` \
                 on the one you want.** What comes back is a line per set and not what \
                 any of it declares: the parameters, the element counts and what a node \
                 emits are `read_set`'s answer, because they need a card per artifact and \
                 a listing that read them all would be reading a library to print an \
                 index. **What a node is called here is what `read_set` calls it** — the \
                 name the set gave it, the name its procedure gives itself where the set \
                 gave none, and the short hash of its source where there is neither, \
                 which is an ordinary state and not a damaged store. **The list is \
                 capped**: what comes back says how many matched and how many are shown, \
                 and if those differ you are looking at part of a library — narrow it \
                 with the filters rather than assuming the rest is not there.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "holds": {
                        "type": "string",
                        "description":
                            "list only sets holding a node whose name contains this, \
                             matched without regard to case — `drift_shell` finds every \
                             set built on that geometry. Omit to list everything.",
                    },
                    "layer": {
                        "type": "string",
                        "enum": layers,
                        "description":
                            "list only sets holding a node on this layer — `L2` for the \
                             ones that deform something, `Field` for the ones with a \
                             distance function. Given with `holds`, both must be true of \
                             the set, though not of the same node. Omit to list \
                             everything.",
                    },
                },
            },
        },
        {
            // **The eighth, and it is one of the seven's kind rather than
            // `operate`'s**: it reads the store — `history::list` walks
            // `<store>/history/` newest first and opens no file — which is
            // what `list_sets` and `read_set` do and what nothing on the
            // render loop's frame can do without paying for a directory walk
            // on the path that must not wait
            // ([ADR-0199](../../../docs/adr/0199-mcp-names-its-operations-and-performs-them-itself.md),
            // `docs/adr/0342-…`). The reply is the listing, which is where a
            // read's answer goes: back to the surface that asked
            // ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
            "name": "walk_history",
            "description":
                "Every version of one Set's material that compiled, most recent first. \
                 Every build this instrument accepts is filed under `<store>/history/` \
                 whether or not it stayed on screen — the gate is **compiling** and not \
                 landing, so the version that cost too much to run is in here too — and \
                 a row is the name the store filed it under: when it was written, which \
                 slot, which node of that slot, and what the procedure called itself. \
                 **This is the listing and not the landing**: put one of these versions \
                 back with `operate` naming *Put a node's previous version back* and \
                 `{\"deck\": <the row's slot>, \"revision\": {\"picked\": \"<row>\"}}`, \
                 which is what a row is a name for. **Narrowed to one Set and never to a deck**: two decks playing one \
                 Set have one history between them, and a version written while a slot \
                 was running no Set is filed under none and is matched by no id. **The \
                 walk is capped and says so**: it stops entering day directories once it \
                 has enough, so an answer that says the walk stopped short is part of \
                 what is there rather than all of it.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "set": {
                        "type": "string",
                        "pattern": ID_PATTERN,
                        "maxLength": MAX_ID,
                        "description":
                            "the Set whose versions to walk, by the id it is filed under \
                             — `list_sets` names them. Required: a walk that names no Set \
                             lists nothing, because those versions are filed under no Set \
                             rather than under all of them.",
                    },
                },
                "required": ["set"],
            },
        },
        // **The ninth, and it is generated** — see [`operate_tool`] and
        // [`SPELLED`]. The eight above are written out because each of them
        // performs something only this server can; this one is the vocabulary,
        // and a hand-written copy of it beside the vocabulary is the drift
        // `karakuri-operation` exists to end.
        operate_tool(),
    ])
}

/// What one tool call came to: an answer, or a wait that belongs outside the
/// state lock. See [`Pending`].
enum Called {
    Answered(Result<String, String>),
    Saving(mpsc::Receiver<News>),
    /// An edge the loop has been asked for, and what this server has to add to
    /// whatever it answers — see [`Pending::Wiring`].
    Wiring {
        news: mpsc::Receiver<News>,
        note: String,
    },
    /// An operation the loop has been asked to perform — see
    /// [`OperateRequest`]. No note beside it: what this server knows about the
    /// run that the loop will not say is the class the audit refused on, and a
    /// refusal never reaches here.
    Operating(mpsc::Receiver<News>),
}

/// **What one tool call names, in the vocabulary** — or the refusal its
/// arguments earned.
///
/// A tool call is a request from outside the process naming a thing to do,
/// which is a MIDI message's shape rather than a key press's, and
/// [ADR-0196](../../../docs/adr/0196-a-map-line-names-a-state-and-an-old-line-is-refused.md)
/// is what that surface did with it: message becomes `Operation`, and something
/// else performs it. **The second half of that does not exist here and cannot.**
/// `karakuri_operation_record::written` answers `Silent` for all six of these —
/// `Question` for the four that ask and `OnLanding` for the two whose record is
/// written where the work lands — so `Live::operate` would print *no record* and
/// do nothing, which is
/// [ADR-0198](../../../docs/adr/0198-a-gesture-converts-in-the-parts-that-are-decided.md)'s
/// finding about twelve keys, holding here for all six tools. There is also no
/// `Live` on this thread to route into: this server reaches the render loop for
/// exactly one thing, over the channel [`SaveRequest`] travels on.
///
/// So what routes is the **naming**. The wire's own words — a tool name and a
/// JSON object — become the operation the manual specifies, once, here; and
/// [`perform`] dispatches on that operation rather than on the string. A tool
/// whose payload the vocabulary cannot say does not compile, and the row on
/// `docs/manual/operations.html` that a tool claims is the row its operation's
/// title names rather than one a second list asserts.
///
/// **The order arguments are refused in is the order they were refused in
/// before this routed**, deliberately: a change of route may not change what a
/// tool answers, and the refusals here are the surface's product — a model that
/// is told which mistake it made fixes its own call. So the slot is checked
/// where each tool checked it, `checked_id` runs where each tool ran it, and
/// nothing new is decided in front of anything old.
fn asked(name: &str, args: &Value, slots: &Slots) -> Result<Asked, String> {
    Ok(match name {
        "read_procedure" => match address(args, slots) {
            Ok((deck, node)) => Asked::Named(Operation::ReadProcedure { deck, node }),
            Err(refusal) => Asked::Refused(refusal),
        },
        "write_procedure" => match written_procedure(args, slots) {
            Ok(operation) => Asked::Named(operation),
            Err(refusal) => Asked::Refused(refusal),
        },
        "wire_input" => match wired_input(args, slots) {
            Ok(operation) => Asked::Named(operation),
            Err(refusal) => Asked::Refused(refusal),
        },
        "swap_outcome" => Asked::Named(Operation::SwapOutcome),
        "read_set" => match named_set(args) {
            Ok(operation) => Asked::Named(operation),
            Err(refusal) => Asked::Refused(refusal),
        },
        "list_sets" => match listing(args) {
            Ok(operation) => Asked::Named(operation),
            Err(refusal) => Asked::Refused(refusal),
        },
        // **The store's other listing**, beside `list_sets` for its reason: the
        // walk is a directory read this thread can do and the render loop
        // cannot afford, and its answer is rows rather than a report that
        // something was performed (`docs/adr/0342-…`).
        "walk_history" => match walked_history(args) {
            Ok(operation) => Asked::Named(operation),
            Err(refusal) => Asked::Refused(refusal),
        },
        "save_set" => match kept(args, slots) {
            Ok(operation) => Asked::Named(operation),
            Err(refusal) => Asked::Refused(refusal),
        },
        // **The ninth tool, and the one that names rather than does.** The
        // eight above each turn a tool's own arguments into the operation the
        // manual specifies; this one is handed the operation's own name and
        // looks it up — see [`operated`] and [`SPELLED`]. It is here at the end
        // rather than first so that a tool with a name of its own is still
        // matched by that name, which is what keeps `operate` from becoming a
        // second spelling of any of them.
        "operate" => match operated(args, slots) {
            Ok(operation) => Asked::Named(operation),
            Err(refusal) => Asked::Refused(refusal),
        },
        other => return Err(format!("no tool `{other}`")),
    })
}

/// What [`asked`] made of one call: the operation, or what the caller is told
/// instead.
///
/// **The refusal is a tool result and not a protocol error**, which is why it
/// is carried in the `Ok` half rather than returned — see [`tool_result`]. The
/// `Err` of [`asked`] is the one thing that really is a protocol mistake: a
/// tool this server does not publish.
enum Asked {
    Named(Operation),
    Refused(String),
}

/// **The deck one slot number names.**
///
/// `Operation` carries `deck: u8`, and every `slot` in
/// `karakuri_store::record::Record` is a `u8` too, so a slot past 255 is not a
/// deck anything in this program can address. Refused in the words
/// [`Slots::nodes`] and [`Slots::holds`] refuse an absent slot in, because it
/// is the same mistake and an operator is told one story about it
/// ([`crate::no_such_slot`]).
///
/// **Called after every argument the tool used to parse before it reached the
/// slot**, so that a call with two mistakes in it is still told about the same
/// one it was told about before.
fn deck_named(slot: usize, slots: &Slots) -> Result<u8, String> {
    slots.holds(slot)?;
    u8::try_from(slot).map_err(|_| karakuri_environment::no_such_slot(slot, slots.count()))
}

/// `read_procedure`'s arguments as the deck and node they name.
fn address(args: &Value, slots: &Slots) -> Result<(u8, NodeAddress), String> {
    let (slot, layer, index) = slot_layer_index(args)?;
    let deck = deck_named(slot, slots)?;
    Ok((
        deck,
        NodeAddress {
            layer: layer_of(layer),
            index: index as u32,
        },
    ))
}

/// `write_procedure`'s arguments as the operation they name.
///
/// **`source` is parsed before the slot is checked**, which is the order this
/// tool has always refused in: a call with no `source` at all is told that
/// first, whatever slot it named.
fn written_procedure(args: &Value, slots: &Slots) -> Result<Operation, String> {
    let (slot, layer, index) = slot_layer_index(args)?;
    let source = args
        .get("source")
        .and_then(Value::as_str)
        .ok_or("`source` is required")?;
    let deck = deck_named(slot, slots)?;
    Ok(Operation::WriteProcedure {
        deck,
        node: NodeAddress {
            layer: layer_of(layer),
            index: index as u32,
        },
        source: source.to_string(),
    })
}

/// `wire_input`'s arguments as the operation they name.
///
/// **Both ends are names and neither is a [`NodeAddress`]**, which is the one place
/// this surface departs from the address the rest of it uses — and it is a
/// decision made twice before this tool existed. `Record::Edge` states it: *a
/// position moves when the list is reordered, and reordering silently changing
/// which geometry a morph blends towards is the exact failure this record exists
/// to end.* [`NodeAddress`]'s own documentation states the other half: *so
/// `Operation::WireInput` takes names and everything else takes this, and the
/// two are not interchangeable.* A tool here that took `{layer, index}` because
/// its five neighbours do would be spelling an edge in the one address an edge
/// may not be spelled in, and
/// `docs/principles/0086-a-procedure-knows-only-what-it-declares.md` is what it would
/// be breaking: the names are the *Set's* answer, and a procedure never knows
/// them.
///
/// **Nothing here checks that the names resolve, and that is not laziness.**
/// The nodes of a Set are named by the Set — `--set morph=warp.kir` names one,
/// and a bare file is named after the procedure inside it — and [`Slots`] holds
/// paths, not names, because a path never crosses this protocol. So a check
/// built from the files alone would accept an edge that is wrong wherever a
/// name was given on the command line and refuse one that is right, which is
/// `docs/principles/0084-…`'s failure exactly: a warning that fires on healthy
/// material. The names are refused where the Set is built, in the sentence
/// `--edge` meets there too
/// (`docs/principles/0090-a-surface-offers-it-never-decides.md`).
///
/// **The deck is checked last**, after all four arguments have been read, on
/// [`written_procedure`]'s terms: an argument this tool cannot do without is
/// named before a slot number that may also be wrong.
fn wired_input(args: &Value, slots: &Slots) -> Result<Operation, String> {
    let slot = args
        .get("slot")
        .and_then(Value::as_u64)
        .ok_or("`slot` is required and is a number")? as usize;
    let node = edge_name(args, "node", "the node that declares the input")?;
    let input = edge_name(args, "input", "what that node's procedure calls the input")?;
    let to = edge_name(args, "to", "the node bound to it")?;
    let deck = deck_named(slot, slots)?;
    Ok(Operation::WireInput {
        deck,
        node,
        // **`Operation::WireInput`'s `slot` is the *input*, and this surface's
        // `slot` is the deck's.** One word for two things is what the schema's
        // `input` exists to avoid
        // (`docs/contributing.md` §4),
        // and this line is where the two spellings meet.
        slot: input.into(),
        to,
    })
}

/// One end of an edge, or the refusal an empty one earns.
///
/// **Empty is refused rather than resolved to nothing**, which is `parse_edge`'s
/// rule on the command line and its sentence: *every part names something.* An
/// edge with no slot in it is a statement about a node, and there is no such
/// statement — and a name is not otherwise constrained here, because a node is
/// called whatever `--set` or a `proc` line called it and this server is not the
/// thing that decides what that may be.
fn edge_name(args: &Value, key: &str, what: &str) -> Result<String, String> {
    let named = args
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("`{key}` is required and is a string: {what}"))?;
    if named.is_empty() {
        return Err(format!(
            "`{key}` is empty, and every part of an edge names something: a node, the \
             input it declares, and the node bound to it"
        ));
    }
    Ok(named.to_string())
}

/// `save_set`'s arguments as the operation they name.
///
/// **`slot` is required and `id` is not**, which is [`slot_layer_index`]'s
/// convention and its reason: an argument a client says nothing about should
/// mean the obvious thing, and the obvious thing here is the name a key press
/// gets. Unlike `index` there is no default written down — the loop stamps it,
/// and stamping it here would be a second answer to what a nameless save is
/// called, which is why [`Operation::SaveSet`]'s `id` is an `Option` as well.
///
/// The slot is checked before the id is read, because that is the order this
/// tool refused in before it routed. See [`Slots::holds`] on why this is not
/// [`Slots::nodes`].
fn kept(args: &Value, slots: &Slots) -> Result<Operation, String> {
    let slot = args
        .get("slot")
        .and_then(Value::as_u64)
        .ok_or("`slot` is required and is a number")? as usize;
    let deck = deck_named(slot, slots)?;
    let id = match args.get("id") {
        // **`null` is absent, not a bad string.** A client that builds its
        // arguments from a record with an empty field sends `"id": null`, and
        // that is a caller saying nothing about the id rather than one getting
        // its type wrong — "`id` is a string" is a refusal about a mistake it
        // did not make. Every other optional argument here reads an absent one
        // as its default; `null` is the second spelling of absent and gets the
        // same answer. It is *not* the same as `""`, which is a caller naming a
        // file with no name and is still refused — see [`checked_id`].
        None | Some(Value::Null) => None,
        Some(id) => Some(checked_id(
            id.as_str()
                .ok_or("`id` is a string: what to file the set under")?,
        )?),
    };
    Ok(Operation::SaveSet { deck, id })
}

/// `read_set`'s argument as the operation it names.
///
/// **The same check `save_set` puts a name through, and the reason is the same
/// one.** This id becomes `<store>/sets/<id>.kbset`, so
/// `../../../somewhere/else` is a path, and paths never cross this protocol —
/// see [`checked_id`] and [`Slots`]. A read is not the harmless half of that
/// rule: it is the half that hands a file's contents back to the caller.
///
/// It runs here rather than in the tool because an id that is a path is not an
/// id, and an operation carries what it acts on.
fn named_set(args: &Value) -> Result<Operation, String> {
    let id = args
        .get("id")
        .and_then(Value::as_str)
        .ok_or("`id` is required and is a string: which set to read")?;
    Ok(Operation::ReadSet {
        id: checked_id(id)?,
    })
}

/// `list_sets`'s arguments as the operation they name.
///
/// **The caller's spelling of `holds` is carried, not a folded one.** The match
/// is case-insensitive and that is [`list_sets`]'s decision about matching; an
/// operation carries what it was asked for.
fn listing(args: &Value) -> Result<Operation, String> {
    let holds = match args.get("holds") {
        // `null` is absent, for the reason [`kept`]'s `id` says: a client
        // building arguments from a record with an empty field sends one, and
        // that is a caller saying nothing rather than a caller getting a type
        // wrong.
        None | Some(Value::Null) => None,
        Some(value) => Some(
            value
                .as_str()
                .ok_or("`holds` is a string: part of a node's name")?
                .to_string(),
        ),
    };
    let layer = match args.get("layer") {
        None | Some(Value::Null) => None,
        Some(value) => {
            let spelled = value
                .as_str()
                .ok_or("`layer` is a string: which layer a set must hold a node on")?;
            // **The same spellings the other tools take**, from the same table:
            // a model that addressed `Field` in `read_procedure` must not be
            // told there is no such layer here.
            let kind = layer_named(spelled)
                .ok_or_else(|| format!("no layer `{spelled}` — {}", layer_list()))?;
            Some(layer_of(kind))
        }
    };
    Ok(Operation::ListSets { holds, layer })
}

/// `walk_history`'s arguments as the operation they name.
///
/// **`set` is required where the payload's own field is an `Option`**, and the
/// two do not disagree: `None` there is *a walk that names no Set*, which is
/// the panel's answer for a deck running the pair the run launched with — those
/// versions are filed under no Set and a narrowing matches none of them
/// (ADR-0276). A model asking for that would be asking for a listing that is
/// empty by construction, so this surface does not offer it and says so.
///
/// **[`checked_id`] for [`named_set`]'s reason**: a Set id is one path
/// component, and the walk matches it against what the store filed a version
/// under.
fn walked_history(args: &Value) -> Result<Operation, String> {
    let set = args.get("set").and_then(Value::as_str).ok_or(
        "`set` is required and is a string: which Set's versions to walk. A walk is \
         narrowed to one Set — `list_sets` names the ids — because two decks playing one \
         Set have one history between them and a version written under no Set is matched \
         by no id",
    )?;
    Ok(Operation::WalkHistory {
        set: Some(checked_id(set)?),
    })
}

// -- `operate`: one operation of the vocabulary, named on the wire ----------

/// **How long an `operate` call waits for the render loop to perform it.**
///
/// [`WIRE_REPLY`] and not [`SAVE_REPLY`], for that constant's reason and the
/// same one: this waits for the loop to reach the top of a frame and act, which
/// is one frame at any frame rate anybody plays at, and no disk is involved.
/// What a *rebuild* made of an operation that starts one lands thirty judged
/// frames later and is `swap_outcome`'s answer.
const OPERATE_REPLY: std::time::Duration = WIRE_REPLY;

/// **What this surface can say of one operation, and why it cannot where it
/// cannot.**
///
/// The `operate` tool takes an operation of `karakuri-operation` by its own
/// name — the heading `docs/manual/operations.html` specifies it under — and
/// hands it to the frame the panel performs every other surface's presses on.
/// It does not take all sixty-four, and the four reasons it does not are here
/// rather than in four scattered refusals
/// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).
///
/// **No wildcard arm.** [`sayable`] is a `match` over every variant, which is
/// `karakuri_operation::gate::standing`'s discipline and its reason: a
/// sixty-fifth operation does not compile until somebody has said whether this
/// surface can name it, and the page's MCP column cannot quietly go stale
/// beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sayable {
    /// `operate` takes it. The audit still answers.
    Operable,
    /// **This server publishes a tool of its own for it**, which does something
    /// only the server can — a file, a store, a listing. A second spelling of a
    /// tool is a second spelling
    /// ([P-0087](../../../docs/principles/0087-name-the-property-never-the-shape.md)),
    /// so `operate` refuses it and names the tool.
    Tool(&'static str),
    /// **A model has no window.** The row's MCP badge is `gap` and the sentence
    /// is
    /// [ADR-0315](../../../docs/adr/0315-a-model-has-no-window-so-the-twelve-surface-rows-mcp-badges-are-gap.md)'s,
    /// worded once here as it is worded once on the page.
    Window,
    /// **This surface will not reach it, and the clause says why and where the
    /// route that does is.** The row's MCP badge is `gap`, and what makes it
    /// `gap` rather than `plan` is that **nothing is owed**: no performer
    /// moving onto the drain's frame would change it, because what stops it is
    /// the shape of this protocol rather than a gap in this program
    /// ([ADR-0341](../../../docs/adr/0341-a-route-that-answers-is-built-and-a-send-that-ends-in-a-dialog-is-gap.md)).
    ///
    /// **There was a sixth answer beside this one until 2026-09-10** —
    /// `Unperformed`, *the vocabulary names it and nothing on this frame
    /// performs it yet*, which is what a `plan` badge in the MCP column meant.
    /// It went when its last row did (ADR-0341): every operation this
    /// vocabulary names either has a performer, has a tool, is a window's, is
    /// unsettled, or is this. **A `plan` badge in that column is now only an
    /// `Undecided` payload**, and the day a row is added that a surface can
    /// say and the frame cannot perform, this `match` has no wildcard and
    /// stops the build until somebody puts the answer back.
    Never(&'static str),
}

/// **Whether `operate` names this operation, and what it says where it does
/// not.** Exhaustive, with no wildcard arm — see [`Sayable`].
fn sayable(operation: &Operation) -> Sayable {
    match operation {
        // ----- the thirty `operate` takes ----------------------------------
        //
        // Twenty-eight of them are refused by the audit until an operator opens
        // their class, which is a built route and not a missing one
        // (ADR-0235: *"a closed class is reached and answered with a refusal"*).
        // `Operation::RestoreProcedure` is the one the audit lets through and
        // the panel then performs, and `Operation::LoadSet` is the one whose
        // class is a predicate over its target.
        Operation::TapBeat
        | Operation::ScaleGrid { .. }
        | Operation::SetLatencyOffset { .. }
        | Operation::SetSync { .. }
        | Operation::ScrubDeck { .. }
        | Operation::SetFreeRunTempo { .. }
        | Operation::AttachBeatSource { .. }
        | Operation::SetResidency { .. }
        | Operation::LoadSet { .. }
        | Operation::SetCompositing { .. }
        | Operation::SetGain { .. }
        | Operation::SetOpacity { .. }
        | Operation::SetBlendMode { .. }
        | Operation::FadeDeck { .. }
        | Operation::Crossfade { .. }
        | Operation::Wipe { .. }
        | Operation::SetMaskShape { .. }
        | Operation::SelectRenderer { .. }
        | Operation::SetMasterOut { .. }
        | Operation::SetFeedback { .. }
        | Operation::SetBloom { .. }
        | Operation::SetRgbShift { .. }
        | Operation::SetTonemap { .. }
        | Operation::SetExposure { .. }
        | Operation::WriteParam { .. }
        | Operation::AttachSignal { .. }
        | Operation::TakeParamBack { .. }
        | Operation::SetProperty { .. }
        | Operation::SetAuthority { .. }
        | Operation::RestoreProcedure { .. }
        // **The five ADR-0334 left `plan` for want of a performer on this
        // frame, and each of them has one now** (ADR-0341). Four were already
        // written and sat in the window's own press arm — the star's refusal,
        // the projector's window, the `rec` pill's two ends and the publish
        // mark's re-aim — and `App::operated` calls them where the pointer's
        // button-up arm calls them; the mask position was a missing reading and
        // is one arm of `crates/karakuri`'s `reading`. They leave this list by
        // having a performer rather than by this rule bending, which is the
        // shape ADR-0334 said each of them would leave in.
        | Operation::SetMaskPosition { .. }
        | Operation::Publish { .. }
        | Operation::SetFavourite { .. }
        | Operation::RouteFrame { .. }
        | Operation::RecordSession { .. }
        // **And ADR-0338's load, which left the `plan` column on 2026-09-10 for
        // the reason *Narrow the published interface* did**: its performer was
        // never missing. `overlaid` sits in `App::performed`, in the arm the
        // fold and the library load are in, and this drain lands there — so
        // the sentence this row used to carry, *nothing re-aims a slot with
        // one layer replaced yet*, had stopped being true before it was read.
        | Operation::LoadProcedure { .. }
        // **And ADR-0338's keep, which left the `plan` column on 2026-09-10 by
        // gaining a performer.** `App::operated` hands it to
        // `Keeping::keep_procedure` where the pointer's button-up arm hands
        // the capsule's press to the same method, and the two differ in one
        // argument: a model's is `Asked::Model` and lands in
        // `<store>/sandbox/`, stamped and overwriting nothing, where an
        // operator's own act writes `<store>/procedures/`
        // (`docs/principles/0096-…`, `docs/adr/0261-…`).
        //
        // **A model is not refused here where its star is**, and ADR-0301 is
        // why: a favourite has no sandbox form to land in and a kept procedure
        // is a file, so it has one.
        //
        // **The answer arrives when the file does.** A keep is *"on a
        // worker"*, so the drain hands the reply to the write thread rather
        // than saying *performed* on the frame it arrived on — which is
        // `save_set`'s own arrangement one file kind along.
        | Operation::KeepProcedure { .. }
        | Operation::Quit => Sayable::Operable,

        // ----- the seven that have a tool of their own ---------------------
        Operation::ReadProcedure { .. } => Sayable::Tool("read_procedure"),
        Operation::WriteProcedure { .. } => Sayable::Tool("write_procedure"),
        Operation::WireInput { .. } => Sayable::Tool("wire_input"),
        Operation::SwapOutcome => Sayable::Tool("swap_outcome"),
        Operation::SaveSet { .. } => Sayable::Tool("save_set"),
        Operation::ReadSet { .. } => Sayable::Tool("read_set"),
        Operation::ListSets { .. } => Sayable::Tool("list_sets"),
        // **The eighth, and it joined the seven on 2026-09-10 by having its
        // payload settled** — it names the Set it is a walk of now, so this
        // surface can say it. It is a tool rather than an `operate` name for
        // the property that puts the other seven here and not for its shape: it
        // reads the store and answers with rows, which is what only this server
        // can do, and a walk performed on the drain's frame would be a
        // directory walk on the path that must not wait. See
        // `docs/adr/0342-…`.
        Operation::WalkHistory { .. } => Sayable::Tool("walk_history"),

        // ----- the seventeen a model has no window for ---------------------
        //
        // ADR-0315's twelve and the sequencer's five, which carry that record's
        // sentence on the page for the same reason: a route into a surface's
        // own state is a route into a window the model is not looking at.
        Operation::SelectDeck { .. }
        | Operation::SelectScope { .. }
        | Operation::SetTransition { .. }
        | Operation::KeepCandidate { .. }
        | Operation::FoldBay { .. }
        | Operation::FoldPane { .. }
        | Operation::Unfold { .. }
        | Operation::Solo { .. }
        | Operation::ResetArrangement
        | Operation::SaveArrangement { .. }
        | Operation::RestoreArrangement { .. }
        | Operation::SizeWindow { .. }
        | Operation::SetStep { .. }
        | Operation::SetLaneMute { .. }
        | Operation::PointLane { .. }
        | Operation::SetPatternGrid { .. }
        | Operation::SelectPattern { .. }
        // **A bay's own narrowing of what it is drawing**, which is
        // `Operation::SelectScope`'s answer arrived at from the other side:
        // which kinds of row the Library bay shows is a fact about a window,
        // and a model is not looking at one. What a model actually wants here
        // is a listing that holds procedures at all, and that is
        // `Operation::ListSets`' to grow rather than this row's
        // (`docs/adr/0338-a-procedure-is-a-row-of-the-library-and-one-loaded-over-a-layer-makes-a-set-with-no-name.md`).
        | Operation::FilterLibrary { .. }
        // **A pane's target is a pointer inside one console**, like the deck
        // selection two groups up and for its sentence.
        | Operation::PointPane { .. } => Sayable::Window,

        // ----- the group that emptied on 2026-09-10 ------------------------
        //
        // **There were three here — the payloads the vocabulary had not
        // settled — and all three left on one day** (`docs/adr/0342-…`).
        // `Operation::WalkHistory` left by having its payload settled and is a
        // tool above. The other two left by being asked the question this
        // classification is actually about: **not** *is the payload settled*
        // but *what can this surface reach*. `Operation::MoveBoundary` is a
        // surface's own state and is [`Sayable::Window`] with ADR-0315's
        // sentence — it was that record's own thirteenth row, held out of it
        // only because its payload is open. `Operation::WatchFiles` names an
        // event a model does not perform and is [`Sayable::Never`] below.
        //
        // **So `Sayable::Undecided` is gone**, the way `Unperformed` went in
        // ADR-0341 and for its reason: a variant nothing constructs is dead
        // code that says something false about the program, and this `match`
        // has no wildcard, so the day an operation arrives that a surface
        // cannot say the build stops until somebody puts an answer back.
        //
        // `Operation::SelectScope` is here for the window's reason too, which
        // is where it has always been.
        Operation::MoveBoundary { .. } => Sayable::Window,

        // ----- the two no route here will ever take -------------------------
        //
        // **This was the last group and it used to have a neighbour**: rows
        // the vocabulary named that nothing on the drain's frame performed
        // yet, which is what a `plan` badge in the MCP column meant.
        // `SetProperty` left it when the Inspector's deck head grew the two
        // chips that perform it (ADR-0328), the five ADR-0334 named followed
        // on 2026-09-10, and ADR-0338's two went the same day (ADR-0341) — so
        // that group and its answer are gone, and this row is what is left:
        // the one that is not waiting for anything.
        Operation::TransferSet { .. } => Sayable::Never(
            "both halves of it are outside what this protocol carries. A send names no \
             destination and never will — it is a read, and a read's answer goes where the \
             surface that asked puts answers, which on the panel is the system's own save \
             dialog and a model cannot answer one. A take names a file, and paths never cross \
             this protocol. The route is the command line: `--package ID > FILE.kbset` sends \
             one and `--take-in FILE` takes one in",
        ),
        // **A model does not edit a file in another program**, which is the
        // whole of what that row names: somebody saving a source a watcher is
        // looking at, and the rebuild that follows. What a model has instead is
        // the act itself — `write_procedure` **is** its edit — so there is
        // nothing here it would say that the write does not already say, and
        // nothing is owed. That is `docs/adr/0205-…`'s kind of answer: a route
        // whose only content is a second spelling of one that exists is not a
        // route somebody has yet to build (`docs/adr/0342-…`).
        Operation::WatchFiles { .. } => Sayable::Never(
            "a model does not edit a file in another program: it calls `write_procedure`, \
             which **is** its edit — checked, written, built on a worker and swapped at a \
             frame boundary — and there is nothing this row would add to it. The row names \
             an event rather than an act, somebody saving a source a watcher is looking at, \
             and the watching itself is the command line's: `--watch` turns it on for a run",
        ),
    }
}

/// **[`Operation`], asked of the render loop.**
///
/// The third thing this server reaches the loop for, and it is the loop for the
/// reason a save and an edge are: this is where every other surface's presses
/// are performed. A model's `SetGain` and an operator's hand on the fader end
/// in one function, on one thread, at one frame — which is what
/// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
/// asks of a fourth route and what performing it here instead would give up.
///
/// **What the loop owes a request it takes**, written where the sender is:
///
/// 1. **Perform it where a press of the same operation is performed**, and
///    nowhere else. Not a second route into the deck.
/// 2. **Answer once, at the frame it was performed on** — [`Reply::settled`].
///    Not at the swap: what a *rebuild* made of an operation that starts one is
///    `swap_outcome`'s answer, as it is for a written procedure.
pub struct OperateRequest {
    /// The operation, already through the audit — see [`audited`]. The loop
    /// performs it and does not judge it again.
    pub operation: Operation,
    /// Where the answer goes. One message: see the contract above.
    pub reply: Reply,
}

/// **[`Operation`], done**: hand it to the render loop and give the caller back
/// the half it waits on.
///
/// Nothing is performed here and nothing could be: this thread holds no deck,
/// no look and no chain, and a surface that performed the mix on a connection
/// thread would be the second route
/// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
/// exists to prevent. It is [`save_set`]'s shape and [`wire_input`]'s promise.
fn operate(operation: &Operation, state: &State) -> Result<mpsc::Receiver<News>, String> {
    let (tx, rx) = mpsc::channel();
    state
        .operating
        .try_send(OperateRequest {
            operation: operation.clone(),
            reply: Reply(tx),
        })
        .map_err(|e| match e {
            mpsc::TrySendError::Full(_) => format!(
                "the render loop has {ASKED} operations queued and no room for another: it \
                 is taking them slower than they are arriving, or it is not running frames \
                 at all. Nothing was performed, and asking again is safe"
            ),
            mpsc::TrySendError::Disconnected(_) => {
                "the render loop has ended: this run is shutting down and nothing was \
                 performed"
                    .to_string()
            }
        })?;
    Ok(rx)
}

/// **One named operation, done.**
///
/// **It takes an [`Allowed`] and not an [`Operation`], which is the audit made
/// structural.** `karakuri_operation::gate::audit` is the only thing that
/// builds one and its field is private to that crate, so there is no way to
/// reach this function with an operation nobody checked — a path that skipped
/// the gate does not compile rather than passing review.
/// [ADR-0235](../../../docs/adr/0235-mcp-reaches-every-operation-and-what-could-stop-the-show-is-refused-until-the-operator-opens-it.md):
/// *"an audit skipped on one path is the whole mechanism gone."*
///
/// The dispatch is over the vocabulary rather than over the tool's name, which
/// is the whole of what routing buys this surface: the arm that reads a
/// procedure is chosen by [`Operation::ReadProcedure`], so a tool renamed on the
/// wire goes on doing what its operation says, and a tool that named a different
/// operation would visibly do something else.
///
/// **The last arm cannot happen** — [`asked`] builds seven operations and this
/// matches those seven. It is written out rather than left to a wildcard for
/// [`absent`]'s reason: the arm that cannot happen is the one that stops saying
/// so quietly when the shape around it changes, and if an eighth tool ever
/// arrives without an arm here the client is told which operation nothing
/// performs rather than being answered by the wrong one.
fn perform(allowed: &Allowed<'_>, state: &mut State) -> Called {
    match allowed.operation() {
        Operation::ReadProcedure { deck, node } => {
            Called::Answered(read_procedure(*deck, *node, state))
        }
        Operation::WriteProcedure { deck, node, source } => {
            Called::Answered(write_procedure(*deck, *node, source, state))
        }
        Operation::SwapOutcome => Called::Answered(swap_outcome(state)),
        // **Refused here for what this server can decide and waited for
        // elsewhere**, which is `save_set`'s shape and for the same reason: the
        // wiring a slot rebuilds with is the render loop's, and the names in it
        // are the Set's.
        Operation::WireInput {
            deck,
            node,
            slot,
            to,
        } => match wire_input(*deck, node, slot.as_str(), to, state) {
            Ok((news, note)) => Called::Wiring { news, note },
            Err(refusal) => Called::Answered(Err(refusal)),
        },
        // Answered here like a read and unlike `save_set`: a card is a file, the
        // render loop does not hold one, and there is nothing to wait for.
        Operation::ReadSet { id } => Called::Answered(read_set(id, state)),
        // A directory read and a file read per set, and nothing else — see
        // [`list_sets`]. Answered here for the same reason `read_set` is.
        Operation::ListSets { holds, layer } => {
            Called::Answered(list_sets(holds.as_deref(), *layer, state))
        }
        // **The store's other listing, and no file is opened at all** — see
        // [`walk_history`]. Answered here for `list_sets`' reason, and it is
        // the reason this row is a tool rather than a name `operate` takes:
        // handing a directory walk to the render loop would put it on the path
        // that must not wait.
        Operation::WalkHistory { set } => Called::Answered(walk_history(set.as_deref(), state)),
        // **Refused before it is sent and waited for elsewhere.** Everything
        // this module can decide by itself — a slot that does not exist, an `id`
        // that is not a name — was decided in [`asked`] under the lock like any
        // other tool's arguments, and only the wait for somebody else's thread
        // is deferred.
        Operation::SaveSet { deck, id } => match save_set(*deck, id.as_deref(), state) {
            Ok(news) => Called::Saving(news),
            Err(refusal) => Called::Answered(Err(refusal)),
        },
        // **Everything `operate` names, and it is one arm because it is one
        // route.** The seven above are matched by their own operations, so
        // nothing reaches here that has a tool of its own; what does reach here
        // has been through [`operated`], which took it only if [`sayable`] says
        // this surface can name it, and then through [`audited`]. So the answer
        // is *hand it to the frame every other surface's presses are performed
        // on* — and the last arm is the one that cannot happen, kept for
        // [`absent`]'s reason.
        other => match sayable(other) {
            Sayable::Operable => match operate(other, state) {
                Ok(news) => Called::Operating(news),
                Err(refusal) => Called::Answered(Err(refusal)),
            },
            _ => Called::Answered(Err(format!(
                "`{}` is an operation this server publishes no tool for",
                other.title()
            ))),
        },
    }
}

fn call_tool(request: &Value, state: &mut State) -> Result<Called, String> {
    let params = request.get("params").ok_or("no params")?;
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or("no tool name")?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    if name == "check_procedure" {
        let source = args
            .get("source")
            .and_then(Value::as_str)
            .ok_or("`source` is required")?;
        let report = check_procedure(source);
        let report_json = serde_json::to_string_pretty(&report).unwrap_or_default();
        return Ok(Called::Answered(if report.success {
            Ok(report_json)
        } else {
            Err(report_json)
        }));
    }
    if name == "check_set" {
        let id = args
            .get("id")
            .and_then(Value::as_str)
            .ok_or("`id` is required")?;
        let report = check_set_configuration(&state.store, id);
        let report_json = serde_json::to_string_pretty(&report).unwrap_or_default();
        return Ok(Called::Answered(if report.success {
            Ok(report_json)
        } else {
            Err(report_json)
        }));
    }

    Ok(match asked(name, &args, &state.slots)? {
        // **The gate, and there is one of it.** Named, then audited, then done
        // — every tool crosses this seam because [`perform`] takes what
        // [`audited`] returns and nothing else can make one.
        Asked::Named(operation) => match audited(&operation, state) {
            Ok(allowed) => perform(&allowed, state),
            Err(refused) => Called::Answered(Err(refused)),
        },
        Asked::Refused(refusal) => Called::Answered(Err(refusal)),
    })
}

/// **This surface's one call into the audit.**
///
/// The classification, the four classes and the refusal sentence are
/// `karakuri_operation::gate`'s and not this module's, which is
/// [ADR-0236](../../../docs/adr/0236-a-map-is-the-layer-between-a-surface-and-the-vocabulary-and-the-audit-is-one-of-the-things-it-does.md)
/// refusing to let one surface hold the rule: *"a rule held by one surface
/// binds one surface"*, and a sequencer lane is already decided as a fifth
/// route that would otherwise arrive with a second copy of the table. What is
/// this module's is the two things only it can supply — **the opening the run
/// was handed** and **what it has read of what is running**.
///
/// **And it has read nothing, which is said rather than defaulted.**
/// `Running::unread()` is honest: this server holds `Slots`, a store root and a
/// watch flag, and no residency at all. Exactly one row turns on that reading —
/// `Operation::LoadSet`, whose class is *a deck in live mode* — and this
/// surface publishes no tool that names it, so nothing is refused today that
/// was not refused yesterday. The day a `load_set` tool lands it is refused
/// with *which decks are live was not read* until somebody wires the reading,
/// which is
/// `docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md`'s
/// answer rather than a guess that the deck is idle.
fn audited<'a>(operation: &'a Operation, state: &State) -> Result<Allowed<'a>, String> {
    gate::audit(operation, state.opening.read(), gate::Running::unread())
}

/// One tool call's answer, in the shape the protocol gives a tool.
///
/// **A tool failure is a result, not a protocol error.** A model that is told
/// "the call was malformed" learns nothing; one handed the checker's
/// diagnostics can fix its own source, which is the whole loop.
///
/// A function rather than a `json!` at each call site, because `save_set`'s
/// answer is built after the lock is gone — see [`Pending`] — and two spellings
/// of this shape would be two chances to disagree about `isError`.
fn tool_result(outcome: Result<String, String>) -> Value {
    match outcome {
        Ok(text) => {
            if let Ok(report) = serde_json::from_str::<DiagnosticReport>(&text) {
                json!({ "content": [{ "type": "text", "text": text }], "isError": false, "report": report })
            } else {
                json!({ "content": [{ "type": "text", "text": text }], "isError": false })
            }
        }
        Err(text) => {
            if let Ok(report) = serde_json::from_str::<DiagnosticReport>(&text) {
                json!({ "content": [{ "type": "text", "text": text }], "isError": true, "report": report })
            } else {
                json!({ "content": [{ "type": "text", "text": text }], "isError": true })
            }
        }
    }
}

/// **`index` is optional and defaults to 0**, unlike the wildcard an absent
/// `index` means on a `param` record. The difference is the same one that runs
/// through the whole address: this names *a procedure to read or rewrite*, and
/// there is no such thing as rewriting every renderer at once with one source
/// — where a `param` addresses a *value*, and one value reaching every
/// declaration is both meaningful and the useful default.
///
/// It defaults because the first node of a layer is what a client that says
/// nothing means, on every layer: a slot holding one camera and one shape has
/// nothing else `index` could name, and one holding two has an order its files
/// were given in.
///
/// **The layer is parsed here into the compiler's own `Kind`** and travels as
/// one from here on, so the layer this resolves a file for and the layer a
/// written source is checked against are the same value rather than two
/// readings of one string.
fn slot_layer_index(args: &Value) -> Result<(usize, Kind, usize), String> {
    let slot = args
        .get("slot")
        .and_then(Value::as_u64)
        .ok_or("`slot` is required and is a number")? as usize;
    let named = args
        .get("layer")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("`layer` is required and is one of {}", layer_list()))?;
    let layer = layer_named(named)
        .ok_or_else(|| format!("no layer `{named}`: a slot's nodes are {}", layer_list()))?;
    let index = match args.get("index") {
        None => 0,
        Some(v) => v
            .as_u64()
            .ok_or("`index` is a number: which node of that layer, from 0")?
            as usize,
    };
    Ok((slot, layer, index))
}

/// **[`Operation::ReadProcedure`], done**: the source of one node of one deck.
///
/// The address arrives as the vocabulary's [`NodeAddress`] and is turned back into
/// the compiler's own [`Kind`] here, at the one place that resolves a file —
/// see [`kind_of`].
fn read_procedure(deck: u8, node: NodeAddress, state: &State) -> Result<String, String> {
    let path = state
        .slots
        .path(usize::from(deck), kind_of(node.layer), node.index as usize)?;
    std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))
}

/// **[`Operation::WriteProcedure`], done**: check a procedure and, if it
/// compiles, write it.
///
/// The record this owes is `Record::Procedure` and it is not written here —
/// `karakuri_operation_record::written` answers `Silent(OnLanding)`, because a
/// record written at the ask would claim a swap the budget went on to roll
/// back. It is written where the swap lands, which is the render loop, and this
/// tool's answer says as much.
fn write_procedure(
    deck: u8,
    node: NodeAddress,
    source: &str,
    state: &State,
) -> Result<String, String> {
    let slot = usize::from(deck);
    let layer = kind_of(node.layer);
    let index = node.index as usize;
    let path = state.slots.path(slot, layer, index)?;
    let name = layer_name(layer);

    // **Checked before it is written, and the diagnostics are handed back.**
    // Writing first and letting the watcher report would put the compiler's
    // answer on a terminal the model cannot see.
    let report = check_procedure(source);
    if !report.success {
        return Err(serde_json::to_string_pretty(&report).unwrap_or_default());
    }
    let checked = karakuri_environment::compile::check(source).map_err(|e| {
        let fallback = DiagnosticReport {
            diagnostics: vec![Diagnostic {
                code: "KIR-E100-COMPILE-FAILED".to_string(),
                message: format!("compile: {e}"),
                line: None,
                column: None,
                remedy: None,
            }],
            success: false,
        };
        serde_json::to_string_pretty(&fallback).unwrap_or(e)
    })?;
    // **The address and the source have to agree**, and the comparison is now
    // between two `Kind`s rather than between a string and a guess. The guess
    // was `L1`, or `L4` for everything else, which made this refusal answer
    // about a layer nobody had named: a `kind L2` sent to a slot's L2 was
    // turned away for not being a renderer, which is a refusal about a mistake
    // the caller had not made.
    if checked.kind != layer {
        let report = DiagnosticReport {
            diagnostics: vec![Diagnostic {
                code: "KIR-E300-LAYER-MISMATCH".to_string(),
                message: format!(
                    "this is a {:?} procedure and it was addressed to slot {slot}'s {name} — \
                     the two layers are not interchangeable, and what a file is is the `kind` \
                     line inside it",
                    checked.kind
                ),
                line: None,
                column: None,
                remedy: Some(format!(
                    "Change `kind {:?}` to match `{name}` or address the appropriate slot layer.",
                    checked.kind
                )),
            }],
            success: false,
        };
        return Err(serde_json::to_string_pretty(&report).unwrap_or_default());
    }

    // **What else this write reaches**, which is normally nothing now and is
    // still asked. `scratch::materialise` gives every slot its own copy, so two
    // slots of a run cannot hold one path however the operator spelled the
    // command line, and this scan comes back empty *because of that rule*
    // rather than because nobody happens to share. It is kept, and kept as a
    // scan rather than replaced by the constant it usually equals: `Slots` is a
    // list of paths this module is handed, the sentence is true of whatever it
    // is handed, and a `Vec::new()` written here would be this module asserting
    // something about its caller.
    //
    // It was load-bearing under the rule that went: `watch.rs` documents two
    // slots sharing a pair as supported, and the manual's own example gave one
    // `soft_points.kir` to three slots — so naming one slot was reporting a
    // third of what happened. Anything skipped or widened is said with a count.
    //
    // **Every node of every other slot**, whatever layer it is on: the scan
    // walked an L1 and a list of renderers, which is the shape a slot had
    // before it could hold a deformation chain — so one `swirl_warp.kir` given
    // to two slots was a write that silently changed both and named one.
    let also: Vec<String> = (0..state.slots.count())
        .filter(|other| *other != slot)
        .filter(|other| {
            state
                .slots
                .nodes(*other)
                .is_ok_and(|nodes| nodes.iter().any(|(_, _, held)| *held == path))
        })
        .map(|other| other.to_string())
        .collect();

    std::fs::write(&path, source).map_err(|e| format!("{}: {e}", path.display()))?;
    let shared = if also.is_empty() {
        String::new()
    } else {
        format!(
            " This file is also slot{} {}, which now show the same procedure.",
            if also.len() == 1 { "" } else { "s" },
            also.join(", ")
        )
    };
    // Named as an address rather than as a layer, because two renderers or two
    // sources are only told apart by the index — the same `layer:index:`
    // `--param` writes.
    // **The edit history is true of both branches**, and it used to be said in
    // only one. `--mcp` on its own makes a run editable exactly as `--watch`
    // does — `main.rs`'s `editable` is `watch || mcp.is_some()` — so the same
    // two things have already happened either way: the deck runs from copies in
    // the scratch, so the paths the operator named on the command line are not
    // what this wrote to, and `history::seed` has filed the version the run
    // started with. "It replaced the file on disk and there is no backup" was
    // wrong about both halves, on the one surface whose reader has no other way
    // to find out.
    let kept = "This replaced the run's copy of the file, under the scratch the deck runs \
                from — the paths named on the command line are not written to. The version \
                it replaced is in the run's edit history under `<store>/history/`, so it can \
                be got back, but not from here.";
    Ok(if state.watching {
        format!(
            "compiled and written to slot {slot} {name}:{index}.{shared} It is being built on a \
             worker thread and will swap in at a frame boundary; call `swap_outcome` to \
             find out whether it landed or was overloaded.\n\n{kept} Every later \
             version that compiles is kept there too."
        )
    } else {
        format!(
            "compiled and written to slot {slot} {name}:{index}.{shared} **This run was started \
             without `--watch`, so nothing will pick it up** — the file has changed and the \
             screen has not.\n\n{kept} Only the version the run started with is there: \
             nothing is snapshotting without `--watch`, so writing this node twice \
             replaces the first write with no record of it."
        )
    })
}

fn swap_outcome(state: &mut State) -> Result<String, String> {
    state.drain();
    let dropped = state.dropped.load(std::sync::atomic::Ordering::Relaxed);
    let missing = if dropped == 0 {
        String::new()
    } else {
        format!(
            "\n\n({dropped} earlier report{} were dropped for want of room — this is \
                 not the whole history)",
            if dropped == 1 { "" } else { "s" }
        )
    };
    Ok(if state.recent.is_empty() {
        format!(
            "nothing has swapped, been overloaded or failed to build since this run \
             started.{missing}"
        )
    } else {
        format!("{}{missing}", state.recent.join("\n"))
    })
}

/// **[`Operation::SaveSet`], done**: ask the render loop to keep what a slot is
/// playing.
///
/// Nothing about the Set is read here and nothing could be: this thread does not
/// hold it. What this does is hand the request over and give the caller back the
/// half it waits on — everything decidable from the arguments alone was decided
/// in [`kept`].
///
/// **This is the one tool that reaches the render loop, and it is not a second
/// save path.** The request is taken where the MIDI surface is taken and ends in
/// `Live::save_set`, the method the `k` key ends in. The record is
/// `Record::Save` and it is written at the frame the save landed, which is why
/// `karakuri_operation_record::written` answers `Silent(OnLanding)` for this
/// operation rather than handing anybody a record to write here.
fn save_set(deck: u8, id: Option<&str>, state: &State) -> Result<mpsc::Receiver<News>, String> {
    let slot = usize::from(deck);
    let id = id.map(str::to_string);
    let (tx, rx) = mpsc::channel();
    state
        .asked
        .try_send(SaveRequest {
            slot,
            id,
            reply: Reply(tx),
        })
        // **Answered rather than waited for**, both ways. A model must never be
        // left holding a call on a loop that will not answer it, and these are
        // the two shapes of "it will not": one that has stopped taking requests,
        // and one that is gone.
        .map_err(|e| match e {
            // **What `Full` proves and no more.** It used to say the loop "has
            // taken none of them", which the error does not support: the queue
            // holds [`ASKED`] requests nobody has taken *yet*, and a loop
            // running slowly reaches that as surely as one that has stopped.
            // Naming the second as though it were the fact would send a model
            // looking for a dead render thread when the answer is to ask again.
            mpsc::TrySendError::Full(_) => format!(
                "the render loop has {ASKED} save requests queued and no room for another: \
                 it is taking them slower than they are arriving, or it is not running \
                 frames at all. Nothing was saved, and asking again is safe"
            ),
            mpsc::TrySendError::Disconnected(_) => {
                "the render loop has ended: this run is shutting down and nothing was saved"
                    .to_string()
            }
        })?;
    Ok(rx)
}

/// **[`Operation::WireInput`], done**: ask the render loop to bind one node's
/// declared input to another node.
///
/// Nothing about the Set is read here and nothing could be — see
/// [`wired_input`] on why the names are not checked on this side — so what this
/// does is hand the request over and give the caller back the half it waits on.
///
/// **It is `save_set`'s shape and `write_procedure`'s promise.** The request
/// goes to the loop like a save, because only the loop holds what a slot is
/// wired with; but what comes back is a write's answer rather than a save's — an
/// edge lands the way an edited procedure lands, at a frame boundary and under
/// the same budget, so the tool answers when the edge is *written* and points at
/// `swap_outcome` for what the build made of it. See [`WIRE_REPLY`].
///
/// **No record is written anywhere, and that is a hole rather than a design.**
/// `karakuri_operation_record::written` answers `Silent(Silent::NoRecord)` for
/// `WireInput`: `Record::Edge` exists and is a *Set file's*, with no `slot` to
/// carry the deck this operation names. So a rewiring during a set is the one
/// thing a model can do here that a replay does not reconstruct — see the module
/// documentation, which says what would close it.
fn wire_input(
    deck: u8,
    node: &str,
    input: &str,
    to: &str,
    state: &State,
) -> Result<(mpsc::Receiver<News>, String), String> {
    let slot = usize::from(deck);
    let (tx, rx) = mpsc::channel();
    state
        .wiring
        .try_send(WireRequest {
            slot,
            edge: karakuri_engine::set::Edge {
                node: node.to_string(),
                slot: input.into(),
                to: to.to_string(),
            },
            reply: Reply(tx),
        })
        // **Answered rather than waited for**, in [`save_set`]'s two shapes and
        // its words: a queue nobody is emptying and a loop that has ended are
        // different facts, and neither of them may leave a model holding a call.
        .map_err(|e| match e {
            mpsc::TrySendError::Full(_) => format!(
                "the render loop has {ASKED} edges queued and no room for another: it is \
                 taking them slower than they are arriving, or it is not running frames at \
                 all. Nothing was rewired, and asking again is safe"
            ),
            mpsc::TrySendError::Disconnected(_) => {
                "the render loop has ended: this run is shutting down and nothing was \
                 rewired"
                    .to_string()
            }
        })?;
    // **What the loop's own sentence will not say.** The loop knows what it did
    // with the edge; only this side knows how the run was started, and a run
    // without `--watch` has no watcher to rebuild the slot with the new wiring —
    // which is the same thing `write_procedure` says about a file nothing will
    // pick up, about the other half of one edit.
    let note = if state.watching {
        String::new()
    } else {
        "**This run was started without `--watch`, so no watcher will rebuild the slot** \
         — what is on screen was built with the wiring this run started with and will go \
         on being it. The edge is the run's from here on, so a `save_set` of this slot \
         records it; the picture does not change."
            .to_string()
    };
    Ok((rx, note))
}

/// **What one saved Set holds, and what each of its procedures declares** —
/// off the store, with nothing loaded, nothing compiled and no GPU.
///
/// **This is the reader `<hash>.meta.ndjson` did not have.** Every path that
/// stores an artifact from a compile writes a card beside it — see
/// [`crate::meta::card`] — and until this, `Store::read_meta` had no caller
/// outside its own tests. A figure with a producer and no consumer is how the
/// last wrong number in this program got published, so the card gets its reader
/// in the same milestone that gave it a writer.
///
/// **A tool and not a resource.** The resource list is a curriculum, not an
/// index — see `docs/adr/0092-a-resource-listing-is-a-curriculum-not-an-index.md`.
/// A resource is a curated few a client reads in
/// full, and a user's Sets are neither curated nor few nor knowable at startup.
/// The two resources here are the spec and the vocabulary, which every client
/// should read once; a library is searched, and searching is a call.
///
/// **The caller names a Set, because a Set id is the only handle a model can
/// hold.** The three candidates were the address the other tools take
/// (`slot`, `layer`, `index`), a Set id, and a content hash:
///
/// - **A hash is what the card is filed under and it is the one to reject**,
///   easiest though it is. Nothing in this protocol has ever handed a model a
///   hash, so the first call could not be made — a tool whose argument only
///   this tool's own output can supply is a tool nobody can start using. It is
///   also the *only* one of the three that needs no validation, being hex and
///   64 characters, and choosing an argument for the convenience of its
///   validation is choosing the wrong argument.
/// - **`(slot, layer, index)` names what is on screen**, whose source a model
///   can already fetch with `read_procedure` and read the declarations off
///   directly. It would answer a question that is already answerable.
/// - **A Set id names material this surface is otherwise blind to.** A saved
///   Set that has not been loaded has no file behind it that `read_procedure`
///   can reach — its sources are bytes in the store under hashes nothing shows
///   — so *"which of these saved things should I use"* is unanswerable without
///   this. `save_set` comes back naming the id it wrote, and `--load-set ID`
///   is spelled with one, so a model that has kept anything has one in hand.
///
/// **What this does not say is what the Set has those knobs turned to.** The
/// file read here carries `param` and `capacity` records beside the `slot`s and
/// they are deliberately passed over: a `param_decl` says a knob exists and
/// what it may be turned between, a `param` says where this Set left it, and
/// the record vocabulary keeps them apart under two names for exactly that
/// reason. Rendering both in one block would be the place they get confused,
/// and *"what is it set to"* is a second question that deserves being asked as
/// one. The lines are in hand the moment anybody wants it.
///
/// **One number here is not a declaration**, and it is the last block: what the
/// Set will allocate to hold elements. See [`element_storage_block`] for why
/// that is computed rather than measured, and why it is the one figure in this
/// answer that is about the Set as a whole rather than about a procedure.
fn read_set(id: &str, state: &State) -> Result<String, String> {
    // Already one path component, because that is part of naming a set rather
    // than part of reading one — see [`asked`]'s `read_set` arm and
    // [`checked_id`].
    let store = Store::open(&state.store)
        .map_err(|e| format!("the store at `{}`: {e}", state.store.display()))?;
    let lines = store.read_set(id).map_err(|e| {
        format!(
            "reading set `{id}`: {e} — this reads the operator's library, which is \
             filed under the id a set was saved under by the `k` key or by \
             `--save-set ID`. A set kept with `save_set` is in `<store>/{}/` and not \
             here, because the library is written by the operator's own act",
            Store::SANDBOX
        )
    })?;
    // **The file's own order**, which is the order [`crate::setfile::save`]
    // wrote the nodes in, and the order a hand-written file chose. Sorting by
    // layer would impose a reading nobody wrote, for the reason
    // [`crate::meta::card`] keeps a procedure's parameters in declaration order.
    let nodes: Vec<(Layer, u32, Option<String>, Hash)> = lines
        .iter()
        .filter_map(|line| match line.record() {
            Record::Slot {
                at,
                name,
                proc_hash,
            } => Some((at.layer, at.index, name.clone(), *proc_hash)),
            _ => None,
        })
        .collect();
    if nodes.is_empty() {
        return Ok(format!(
            "set `{id}` is in this store and names no material: it holds {} record{} \
             and none of them is a `slot`, so there is nothing in it to describe.",
            lines.len(),
            if lines.len() == 1 { "" } else { "s" },
        ));
    }
    let mut out = format!(
        "set `{id}` holds {} node{}, and `--load-set {id}` plays it. Everything below \
         is what a procedure *declares* — the range a value is refused outside of — \
         and not what this set has anything turned to.\n",
        nodes.len(),
        if nodes.len() == 1 { "" } else { "s" },
    );
    for (layer, index, name, hash) in nodes {
        out.push('\n');
        out.push_str(&node_block(&store, layer, index, name.as_deref(), &hash));
    }
    out.push('\n');
    out.push_str(&element_storage_block(&store, id));
    Ok(out)
}

/// **How many Sets one answer renders, however many matched.**
///
/// A library is not bounded by anything: a run that presses `k` between takes
/// keeps a Set a minute, and a store two thousand deep is an ordinary end state
/// rather than a broken one. A protocol answer is read into a context window,
/// so the choice is between a fixed ceiling and an answer whose size is the
/// user's own filing habits — and twenty is about what a reader can weigh in
/// one go. What must never happen is the ceiling being reached silently, which
/// is why [`list_sets`] says the total and the shown count in the same
/// sentence.
pub const LISTED: usize = 20;

/// **What this store holds** — every Set saved into it, most recent first, with
/// what each one is made of.
///
/// **The listing `read_set` needed and did not have.** `read_set` takes an id
/// and its own description ends by telling a model to use it to choose between
/// things it has kept — which was unreachable, because nothing said what was
/// kept. A model could read a Set it had just saved, in the same conversation,
/// and nothing else; an operator had `ls` on a directory of `.kbset`. This
/// is the other half, and it is the half the milestone is named for.
///
/// **Most recent first, and the tie-break is why this sorts at all.**
/// `Store::list_sets` orders by id, which is total and repeatable and is the
/// right order for the store to promise; *what did I just save* is the question
/// this surface is mostly asked, so it sorts on the write time and breaks ties
/// by id. The tie-break is not decoration: two Sets written within one tick of
/// a coarse filesystem clock carry the same mtime, and a sort whose keys tie
/// falls back to whatever order the entries arrived in — which is not an order,
/// and would differ between two calls on an unchanged store. A model asking
/// twice must not be told two different things about a library nobody touched.
///
/// **What it does not say is what any of it declares.** That needs a card per
/// artifact and, for the element storage, a compile pass over the whole Set —
/// which is what `read_set` is for, on one Set a caller has chosen. A listing
/// that did it for a library would compile a thousand procedures to print a
/// thousand lines. The per-node cards this *does* read are only the ones a
/// name needs: a node the file named costs nothing to name here.
///
/// **The summary comes from [`crate::setfile::summarise`]**, which `--list-sets`
/// renders too. One derivation, two renderings — an operator's line and this —
/// so the two surfaces cannot come to disagree about what a store holds or
/// about what a node in it is called.
fn list_sets(
    holds: Option<&str>,
    layer: Option<karakuri_operation::Layer>,
    state: &State,
) -> Result<String, String> {
    // **Lowercased once here rather than per node.** Case-insensitive because a
    // model that read `drift_shell` in one answer and types `Drift_Shell` into
    // the next is not asking a different question. Folded here rather than in
    // [`listing`], because it is a decision about *matching* and an operation
    // carries what it was asked for.
    let holds = holds.map(str::to_ascii_lowercase);
    // The vocabulary's layer into the record's, which is the third spelling of
    // this list and the one a Set file is written in — see [`layer_of`].
    let layer = layer.map(|layer| karakuri_environment::setfile::layer_of(kind_of(layer)));
    let opened = |e: StoreError| format!("the store at `{}`: {e}", state.store.display());
    let store = Store::open(&state.store).map_err(opened)?;
    let mut sets = karakuri_environment::setfile::summarise(&store).map_err(opened)?;
    let held = sets.len();
    // **An empty store is an answer and not a failure**, and it is a different
    // answer from a filter that matched nothing: one sends a reader to
    // `save_set`, the other to a different filter. Answered before the filters
    // are applied, because a filter over nothing has nothing to say.
    if held == 0 {
        return Ok(format!(
            "this store holds no sets at all — nothing has been kept here yet. This lists \
             the operator's library, which is written by their own act: the `k` key, or \
             `--save-set ID` on the command line. What `save_set` keeps goes to the \
             sandbox and is not listed here. This store is `{}`.",
            state.store.display()
        ));
    }
    sets.sort_by(|a, b| b.written.cmp(&a.written).then_with(|| a.id.cmp(&b.id)));
    // **A set matches, not a node.** With both filters given the question is
    // "which of the sets that use this also deform something", so each is
    // answered against the whole set rather than against one node — a set whose
    // `drift_shell` is a geometry and whose deformation is called something
    // else is exactly what that question is looking for.
    sets.retain(|set| {
        holds.as_ref().is_none_or(|holds| {
            set.nodes
                .iter()
                .any(|node| node.name.to_ascii_lowercase().contains(holds))
        }) && layer.is_none_or(|layer| set.nodes.iter().any(|node| node.layer == layer))
    });
    let narrowed = describe_filters(holds.as_deref(), layer);
    let matched = sets.len();
    if matched == 0 {
        return Ok(format!(
            "none of the {held} set{} {narrowed}. The store is not empty — \
             call this with no arguments to see everything in it. `holds` is matched \
             against what each node is called, which is the name the set gave it or the \
             name its procedure gives itself.",
            plural(held),
        ));
    }
    let shown = matched.min(LISTED);
    let mut out = if matched > shown {
        // **Never a truncated list that reads as a whole one.** A model told
        // "here are your sets" over twenty of two hundred will tell its user
        // they have twenty, and act on a library it has not seen.
        format!(
            "{matched} set{} {narrowed}, and the {shown} most recently written are below — \
             **this is not all of them**: {} more matched and are not listed. Narrow it \
             with `holds`, or with `layer`, or ask for a set by id with `read_set`.\n",
            plural(matched),
            matched - shown,
        )
    } else {
        format!(
            "{matched} set{} {narrowed}, most recently written first — all of them are \
             below.\n",
            plural(matched),
        )
    };
    for set in sets.iter().take(shown) {
        out.push_str(&set_line(set));
    }
    out.push_str(
        "\nEach line is a set's id, when it was written, and what it holds: an address \
         per node and what that node is called in this set. `read_set` with one of these \
         ids says what each of its procedures declares — the parameters, the element \
         counts and what it emits — and `--load-set ID` is what plays one.\n",
    );
    Ok(out)
}

/// One Set as a line of a listing.
fn set_line(set: &karakuri_environment::setfile::SetSummary) -> String {
    let written = karakuri_environment::setfile::written_at(set.written);
    // **A file in `sets/` that will not read is listed and named.** Dropping it
    // would answer "what have I kept" with something missing, and rendering it
    // as a set of no nodes would say it holds nothing.
    if let Some(why) = &set.unreadable {
        return format!(
            "`{}` — written {written}, and could not be read: {why}\n",
            set.id
        );
    }
    if set.nodes.is_empty() {
        return format!(
            "`{}` — written {written}, and names no material: it holds no `slot` record\n",
            set.id
        );
    }
    let nodes: Vec<String> = set
        .nodes
        .iter()
        .map(|node| {
            format!(
                "{}:{} `{}`",
                layer_spelled(node.layer),
                node.index,
                node.name
            )
        })
        .collect();
    format!(
        "`{}` — written {written}, {} node{}: {}\n",
        set.id,
        set.nodes.len(),
        plural(set.nodes.len()),
        nodes.join(", "),
    )
}

/// What the filters did to a listing, as the middle of a sentence — so that
/// every count this tool prints is said to be a count *of* something, and a
/// filtered answer can never be read as the whole store.
fn describe_filters(holds: Option<&str>, layer: Option<Layer>) -> String {
    match (holds, layer) {
        (None, None) => "in this store".to_string(),
        (Some(holds), None) => format!("in this store hold a node whose name contains `{holds}`"),
        (None, Some(layer)) => format!("in this store hold a {} node", layer_spelled(layer)),
        (Some(holds), Some(layer)) => format!(
            "in this store hold both a node whose name contains `{holds}` and a {} node",
            layer_spelled(layer)
        ),
    }
}

/// **How many rows the walk asks for before the narrowing**, and it is a
/// number this surface chooses rather than one `history::list` has.
///
/// `list`'s cap is on **days opened** and the narrowing to one Set happens
/// after it, so a store whose day directories hold several Sets' versions
/// yields fewer of each — asking for two hundred is asking for the last two
/// hundred versions *this store* wrote, of which some are the Set that was
/// asked about. Larger than the twenty a reply renders, so an ordinary Set's
/// recent history survives the narrowing whole; small enough that the walk
/// stops after a handful of day directories, which is the whole of what it
/// costs
/// ([P-0091](../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)).
/// What lies past it is not counted — counting it is the cost the cap exists
/// not to pay — and `Listing::stopped_short` is what says the walk stopped.
const WALKED: usize = 200;

/// **Every version of one Set that compiled**, most recent first, as rows a
/// landing can name back.
///
/// **A read, answered on this thread**, which is why the row is a tool beside
/// `read_set` and `list_sets` rather than a name `operate` takes: it walks
/// `<store>/history/` and opens no file at all, and a directory walk handed to
/// the render loop is a directory walk on the path that must not wait
/// (`docs/adr/0342-…`, ADR-0199).
///
/// **Narrowed to one Set here rather than in `history::list`**, which is that
/// module's own rule — *"which rows an operator is looking at is a question the
/// surface asks"* — and the same division `list_sets`' two filters are applied
/// under. **A row filed under no Set matches no id** and is never folded in:
/// those versions were written where the slot was running the pair a run
/// launched with, and a filter that let them through would be inventing a
/// history for whichever Set was asked about (ADR-0276, ADR-0308).
///
/// **The three things a walk has to say beside its rows** are said: the walk
/// stopping short, the entries under `history/` the layout does not claim, and
/// a rendering shorter than what matched. Each of the three is a way for a
/// listing to read as the whole of something it is not.
fn walk_history(set: Option<&str>, state: &State) -> Result<String, String> {
    // Cannot happen from this surface — [`walked_history`] requires `set` — and
    // written out rather than unwrapped, because the payload's `None` is a real
    // value on another surface and this is what it would mean here.
    let Some(id) = set else {
        return Err(
            "this walk names no Set, and a walk of no Set lists nothing: versions written \
             while a slot was running material nobody had saved are filed under no Set, and \
             no id matches them. Name a Set — `list_sets` says which ones this store holds"
                .to_string(),
        );
    };
    let found = karakuri_environment::history::list(&state.store, WALKED)?;
    let rows: Vec<&karakuri_environment::history::Version> = found
        .versions
        .iter()
        .filter(|version| version.set.as_deref() == Some(id))
        .collect();
    let mut out = String::new();
    if rows.is_empty() {
        out.push_str(&format!(
            "no version of set `{id}` is in this store's edit history. Every build that \
             compiles is filed there, so this is a Set nothing has been edited on in what \
             the walk covers — or one this store has never played. `read_set` says what \
             `{id}` holds and `list_sets` says what else is here.\n"
        ));
    } else {
        let shown = rows.len().min(LISTED);
        if rows.len() > shown {
            // **Never a truncated list that reads as a whole one**, which is
            // [`list_sets`]' rule on its own truncation.
            out.push_str(&format!(
                "{} version{} of set `{id}` are in what this walk covered, and the {shown} \
                 most recent are below — **this is not all of them**: {} more matched and \
                 are not listed.\n",
                rows.len(),
                plural(rows.len()),
                rows.len() - shown,
            ));
        } else {
            out.push_str(&format!(
                "{} version{} of set `{id}`, most recent first — all of the ones this walk \
                 covered are below.\n",
                rows.len(),
                plural(rows.len()),
            ));
        }
        for version in rows.iter().take(shown) {
            out.push_str(&format!("`{}`\n", version.filed_as()));
        }
    }
    if found.stopped_short {
        out.push_str(
            "\nThe walk stopped with day directories unread, so this is part of what is \
             there rather than all of it: it asks for the last few hundred versions this \
             store wrote and narrows them to the Set afterwards, so a store several Sets \
             are being edited in shows fewer of each.\n",
        );
    }
    if found.unclaimed > 0 {
        out.push_str(&format!(
            "\n{} entr{} under `history/` that this layout does not claim {} passed over. A \
             day directory is a place an operator works in by hand — `rm -rf \
             history/2026/07` is this store's whole retention policy — so whatever else is \
             in there is theirs.\n",
            found.unclaimed,
            match found.unclaimed {
                1 => "y",
                _ => "ies",
            },
            match found.unclaimed {
                1 => "was",
                _ => "were",
            }
        ));
    }
    // **The closing paragraph is the rows' own and is not printed under an
    // empty answer**, where every sentence in it would be about something that
    // is not there.
    if !rows.is_empty() {
        out.push_str(
            "\nEach row is the name the store filed a version under: when it was written, \
             which slot, which node of that slot, and what the procedure called itself. To \
             put one back, call `operate` with `Put a node's previous version back` and \
             `{\"deck\": <the row's slot>, \"revision\": {\"picked\": \"<row>\"}}` — the \
             version's bytes are written over that node's working copy and built like any \
             other edit. The gate on this history is **compiling** and not landing, so a \
             version that cost too much to run is in here too.\n",
        );
    }
    Ok(out)
}

/// The `s` on a count, in the one place, because every sentence here has one.
fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

/// **What this Set will allocate to hold its elements**, node by node and in
/// total — computed from the Set file and a compile pass, with nothing built,
/// no adapter opened and no GPU touched.
///
/// **A figure with a producer and no reader is how the last wrong number in
/// this program got published.** `Set::element_storage` has reported this per
/// node since the buffers existed and no binary in this tree printed it; the
/// figure that *was* printed, at stage 4, was a second arithmetic over one
/// procedure's `emit` list — it claimed 96 bytes per element where 312 were
/// allocated, and it was withdrawn rather than corrected — see
/// `docs/adr/0116-stage-four-stops-claiming-the-byte-figure.md`.
/// So this one is not a second arithmetic: `Plan::element_storage` calls
/// the same sizing the allocation calls, over the same walk a build makes, and
/// a test in `karakuri-engine` asserts the two answers about one Set are equal.
///
/// **Every number needs the whole Set and not one card**, which is why this is
/// a block of its own rather than a line inside [`node_block`]. What a node
/// allocates depends on what *reaches* it: an L2 writes everything upstream
/// emitted as well as its own `emit`, an `amplify` above a node multiplies the
/// element count for everything below it, and an L1 that can `kill()` pays for
/// a buffer its text never mentions. A per-node figure read off a card would be
/// wrong in exactly the three ways the withdrawn one was.
///
/// **Not computed at all, rather than computed from a guess**, wherever the
/// Set does not check out: a Set naming an artifact this store has not got, or
/// one whose nodes do not compose, has no figure — and being told which is more
/// use than a number that assumed its way past the problem.
fn element_storage_block(store: &Store, id: &str) -> String {
    // **The same shape as the "no card" branch of [`node_block`]**: a Set this
    // cannot cost is an ordinary thing to meet in a working store rather than a
    // failed call, and what a model is owed is the sentence saying which half
    // is missing.
    let unavailable = |why: &str| {
        format!(
            "element storage: not computed — {why}. This is the one figure here that \
             needs the set to hold together as a whole, because what a node allocates \
             depends on what reaches it; everything above is read off each artifact's \
             own card and stands on its own.\n"
        )
    };
    let loaded = match karakuri_environment::setfile::load(store, id) {
        Ok(loaded) => loaded,
        Err(why) => return unavailable(&why),
    };
    // **What each geometry runs at: the file's number, or the procedure's own
    // default where the file names none** — `capacity [min, max] = default`,
    // and the default is what the spec says applies. The other branch of
    // `capacity_for` is `--capacity`, an operator's flag overriding both, and
    // there are no flags in an MCP call: this answer is about a file.
    let mut sources: Vec<(&Checked, u32)> = Vec::with_capacity(loaded.l1s.len());
    for (at, l1) in loaded.l1s.iter().enumerate() {
        match loaded
            .capacities
            .get(at)
            .copied()
            .flatten()
            .or_else(|| l1.capacity.map(|declared| declared.default))
        {
            Some(capacity) => sources.push((l1, capacity)),
            None => {
                return unavailable(&format!(
                    "`{}` declares no `capacity` and this set records none for it, so \
                     there is no element count to size anything against",
                    l1.name
                ))
            }
        }
    }
    let l2s: Vec<&Checked> = loaded.l2s.iter().collect();
    let l3s: Vec<&Checked> = loaded.l3s.iter().collect();
    let fields: Vec<&Checked> = loaded.fields.iter().collect();
    let l4s: Vec<&Checked> = loaded.l4s.iter().collect();
    let plan = match karakuri_engine::Set::validate(
        &sources,
        &l2s,
        &l3s,
        &fields,
        &l4s,
        // **What `--load-set` builds this file as**, which the file itself now
        // records — a `merge` record, or its absence for a Set that overdraws.
        // Read from the file rather than defaulted, so this refuses a
        // composited Set with more renderers than a fold can hold exactly where
        // loading it would. The choice changes no *number* here either way:
        // compositing costs a render target per renderer, and a render target
        // is not element storage.
        loaded.layering,
        // A salt decides what the elements *are* and never how many bytes they
        // take, so the set's own is enough here and a source deriving one from
        // it changes nothing this block prints.
        loaded.salts.first().copied().flatten().unwrap_or_default(),
        &loaded.salts,
        karakuri_engine::set::Wiring {
            l1s: &loaded.names.l1s,
            l2s: &loaded.names.l2s,
            l3s: &loaded.names.l3s,
            l4s: &loaded.names.l4s,
            fields: &loaded.names.fields,
            edges: &loaded.edges,
        },
    ) {
        Ok(plan) => plan,
        Err(e) => return unavailable(&format!("this set does not build: {e}")),
    };

    let planned = plan.element_storage();
    let names = plan.node_names();
    let total: u64 = planned.iter().map(|p| p.storage.bytes).sum();
    let mut out = format!(
        "element storage: {total} bytes in total, across the {} node{} of this set that \
         hold elements, at the capacities the file records. Nothing was built to find \
         that out: it is the arithmetic the allocation itself is sized by, run over the \
         file.\n",
        planned.len(),
        if planned.len() == 1 { "" } else { "s" },
    );
    for entry in &planned {
        out.push_str(&format!(
            "  `{}` — {} bytes for {} element{}, {} bytes each\n",
            names[entry.node],
            entry.storage.bytes,
            entry.storage.capacity,
            if entry.storage.capacity == 1 { "" } else { "s" },
            entry.storage.per_element(),
        ));
    }
    // **A repeated name is not a mistake and has to say so.** A set over two
    // geometries instantiates its whole chain of deformations once per
    // geometry, so one deform procedure is two nodes with buffers of their own
    // — and they are different sizes whenever the geometries are.
    if planned
        .iter()
        .enumerate()
        .any(|(at, entry)| planned[..at].iter().any(|seen| seen.node == entry.node))
    {
        out.push_str(
            "A name appears twice above because this set has more than one geometry: the \
             chain is instantiated once per geometry, and each instance holds buffers of \
             its own.\n",
        );
    }
    // **What the number means, for a reader who has never seen this system.**
    // The withdrawn figure was as wrong in what it was taken to mean as in its
    // arithmetic, and a number relayed as "what this costs a GPU" would be that
    // mistake in a new costume. A renderer having no row at all is part of the
    // same sentence: it draws from the buffer the node above it allocated, so a
    // row for it would be that memory counted twice.
    out.push_str(
        "What that covers: one element struct per element, the four-byte liveness flag \
         beside it, the second copy a geometry keeps so it can read what it wrote last \
         step, and the destination index a geometry that spawns or kills pays for. A \
         renderer and a camera hold no elements and so have no row. It is NOT what this \
         set costs a GPU: render targets, uniform blocks and every other buffer not \
         indexed by an element are outside it, so it is a floor on device memory and \
         never the figure to allocate against. What it is exactly is the cost of one \
         more element — the per-element numbers above are exact divisions rather than \
         averages.\n",
    );
    out
}

/// One node of a Set: its address in the Set, its artifact, and its card.
///
/// **What it is called is [`crate::setfile::node_called`]'s answer**, and this
/// is the function that used to decide it. `list_sets` names the same node in a
/// listing and a model has to find, when it reads the Set, the node the listing
/// told it about — so the three candidates are weighed in one place and read
/// here rather than weighed a second time.
fn node_block(store: &Store, layer: Layer, index: u32, name: Option<&str>, hash: &Hash) -> String {
    // **Twelve hex characters and not sixty-four.** A hash is not an argument
    // anything here takes — see [`read_set`] — so what this is for is telling
    // two nodes apart and recognising the same artifact in two Sets, which
    // twelve does at a length a reader can hold. `Hash::short` is the same
    // shortening every log line in this program uses.
    let short = hash.short(12);
    let address = format!("{}:{index}", layer_spelled(layer));
    // The head of one block: the address, what the node is called, and the
    // address its source is stored under — except where the name *is* that
    // address, which is what a node with no name of its own and no card to
    // declare one gets, and saying it twice adds nothing to saying it once.
    let head = |called: &str| {
        if called == short {
            format!("{address} `{short}`")
        } else {
            format!("{address} `{called}` — stored as {short}")
        }
    };
    match store.read_meta(hash) {
        Ok(card) => {
            let (declared, body) = rendered_card(&card);
            let called =
                karakuri_environment::setfile::node_called(name, declared.as_deref(), hash);
            // **What did not win, where something had to lose.** A Set's own
            // name for a node hides the name the procedure gives itself, and a
            // model choosing between saved material wants both: the one this
            // set addresses the node by, and the one that identifies the
            // artifact wherever else it appears.
            let also = match &declared {
                Some(declared) if *declared != called => {
                    format!(", and the artifact calls itself `{declared}`")
                }
                _ => String::new(),
            };
            format!("{}{also}\n{body}", head(&called))
        }
        // **Not an error, and it must not read as one.** `Store::read_meta`
        // answers `NotFound` for a card that was never written, which is an
        // ordinary state of a working store rather than damage: a card is
        // derived, `Store::put_artifact` writes none of its own — it takes bytes
        // and does not compile — and an artifact stored before cards existed has
        // none either. A model told "not found" would report a broken library;
        // what it is owed is the sentence that says the source is there and the
        // description is not.
        //
        // **Two absences, and the pair is worth the extra read.** A hash with no
        // card and a hash this store has never seen are the same `NotFound` from
        // here and completely different facts: the second means the Set was
        // written against another store and will not load here at all, which is
        // the more useful thing anyone could be told and is invisible if both
        // say "no card". The artifact is only fetched on this branch, so the
        // ordinary path pays nothing for it.
        Err(StoreError::NotFound(_)) => {
            let standing = if store.get_artifact(hash).is_err() {
                "this store does not hold that artifact at all, so nothing here can \
                 say what it declares and `--load-set` could not build this set \
                 either — the set was saved somewhere else, or beside a store that \
                 has since been moved"
            } else {
                "its source is here and it has no metadata card. That is an ordinary \
                 state and not a damaged store: a card is derived rather than kept, so \
                 an artifact stored as bytes, or stored by a build older than cards, \
                 has none until something compiles it and stores it again. What it \
                 declares is in its source, at the top of the procedure"
            };
            // No card, so there is no declared name to weigh: the set's own
            // name if it has one, and the short hash otherwise.
            let called = karakuri_environment::setfile::node_called(name, None, hash);
            format!("{}\n  {standing}.\n", head(&called))
        }
        // A card that is there and will not read is the one case that *is* a
        // damaged store, and it says so in different words for that reason.
        Err(e) => {
            let called = karakuri_environment::setfile::node_called(name, None, hash);
            format!("{}\n  its card could not be read: {e}\n", head(&called))
        }
    }
}

/// A card's four records as prose: what the procedure calls itself, and the
/// lines describing what it declares.
///
/// **Only the four a card can carry today.** `origin`, `parent`, `perf`, `tag`
/// and `thumbnail` are specified and nothing writes one — see
/// [`crate::meta::card`], which says why each is absent rather than empty — so
/// they fall through the catch-all, which is also what makes this reader survive
/// meeting a card written by a build that has more of them.
fn rendered_card(card: &[Line]) -> (Option<String>, String) {
    let mut declared = None;
    let mut body = String::new();
    let mut params = 0usize;
    for line in card {
        match line.record() {
            Record::Meta { name, .. } => declared = Some(name.clone()),
            Record::ParamDecl {
                key,
                ty,
                min,
                max,
                default,
            } => {
                params += 1;
                body.push_str(&format!(
                    "  param {key} : {ty}, anywhere from {min} to {max}{}\n",
                    match default {
                        Some(default) => format!(", and {default} until something turns it"),
                        // The record's own reading, in words: an absent
                        // `default` says the default is not a number this build
                        // can state, never that there is none — every declared
                        // param has one, because the `.kir` grammar makes the
                        // expression mandatory.
                        None => ". Its default is an expression rather than a literal, so the \
                             card cannot state it as a number"
                            .to_string(),
                    }
                ));
            }
            Record::CapacityDecl { min, max, default } => body.push_str(&format!(
                "  capacity: between {min} and {max} elements, and {default} of them \
                 until a set says otherwise\n"
            )),
            Record::Emit { attrs } => body.push_str(&format!(
                "  emits {} — what a renderer drawn over it can consume\n",
                attrs.join(", ")
            )),
            _ => {}
        }
    }
    // Said rather than left to silence: a block with no `param` line reads as a
    // rendering that dropped them. Every other absence here is a whole record
    // the card deliberately does not write — see [`crate::meta::card`] — and
    // reads correctly as nothing, but "there is nothing to turn on this one" is
    // an answer to the question that was asked.
    if params == 0 {
        body.push_str("  no parameters: there is nothing to turn on this one\n");
    }
    (declared, body)
}

/// A record [`Layer`] under the name this protocol already spells it with.
///
/// **Found through [`crate::setfile::layer_of`] rather than matched again.**
/// The mapping between a record's `Layer` and the compiler's `Kind` exists once,
/// is total, and is the one `--load-set` reads a Set through; a second match
/// here would be a second answer to which layer a stored node is on, and the
/// name a model is given for a node has to be the name it addresses one by. The
/// fallback cannot be reached while that mapping stays total — and it renders as
/// a word rather than panicking, because a layer added on one side only is a
/// thing to see in an answer, not a thread to take down.
fn layer_spelled(layer: Layer) -> &'static str {
    LAYERS
        .iter()
        .copied()
        .find(|kind| karakuri_environment::setfile::layer_of(*kind) == layer)
        .map_or("unknown", layer_name)
}

/// A Set id a client may name, or why not.
///
/// **A Set id is one path component.** [`crate::history::stamped_id`] says so
/// where it explains why the date is spelled `20260816` rather than
/// `2026/08/16`, and the store spells the file `<dir>/<id>.kbset`
/// without checking that what it was handed is one. That is the operator's own
/// business on `--save-set`, where the id came out of their own shell. It is not
/// a model's: this is the same rule [`Slots`] exists for — **paths never cross
/// the protocol** — and `../../../somewhere/else` is a path.
///
/// Letters, digits, `-` and `_`, which is what a stamp is made of and what a
/// name anybody would type is made of. **Refused rather than sanitised**: a set
/// filed under a name its caller did not ask for is a worse answer than one that
/// is told to pick another.
///
/// **A name a client picks twice no longer overwrites, and the reason it once
/// did no longer holds.** This paragraph said the opposite until 2026-09-05, and
/// the argument it made was sound on its own premise: a name a caller typed is
/// an instruction, `--save-set ID` has always obeyed it by overwriting, and
/// `save_set` did what `--save-set` did. What changed is where a model's save
/// lands. It writes `<store>/sandbox/` rather than the operator's library
/// ([P-0096](../../../docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md)),
/// and nothing in that directory is an id an operator typed — it is a session's
/// edit history, and a snapshot a later snapshot can replace is not one. So
/// `crate::filed_as` puts the stamp in front of whatever name a client chose and
/// the accept names what was written; the renaming this paragraph refused is
/// still refused **here**, because refusing a bad id and naming a good file are
/// two different jobs and this one is still the first. What remains true
/// unchanged is that the behaviour is documented — in the tool description a
/// model reads and in `docs/manual.md`. Undocumented was the thing that was not
/// allowed.
///
/// **No `con`, `nul`, `aux`, `com1` check.** They are reserved device names on
/// Windows and would be a file that is not a file. There is no Windows target
/// today and no `cfg` for one here; this sentence is the record that the case is
/// known, so that whoever ports this finds it written down rather than finds it
/// on a projector.
///
/// **Public since 2026-09-08**, because it stopped being the model's wall
/// alone: the Inspector pane head's name is an operator typing an id, which
/// `crate::filed_as` passes straight through for `Asked::Operator` (ADR-0292).
/// One wall, so a `/` cannot become a path on either route.
pub fn checked_id(id: &str) -> Result<String, String> {
    if id.is_empty() {
        return Err(
            "`id` is empty: a set is filed under a name, or under none at all if \
                    `id` is left out"
                .into(),
        );
    }
    // **Bytes, and the message says bytes.** `str::len` is bytes and this said
    // "characters", which is the same number for everything that gets past the
    // charset check below and a different one for what does not — so the one
    // caller the message existed for, the one sending something this refuses,
    // was told a number it could not count to.
    if id.len() > MAX_ID {
        return Err(format!(
            "`id` is {} bytes and the most is {MAX_ID}: it becomes a file name",
            id.len()
        ));
    }
    if let Some(bad) = id
        .chars()
        .find(|c| !c.is_ascii_alphanumeric() && *c != '-' && *c != '_')
    {
        return Err(format!(
            "`id` holds `{bad}`, and a set id is letters, digits, `-` and `_`: it is one \
             path component and it names a file in the store"
        ));
    }
    Ok(id.to_string())
}

/// **Wait for one save's outcome, and say something true when it does not
/// come.**
///
/// A free function over the channel rather than a loop inside [`Pending`], for
/// the reason `main.rs`'s `drained_saves` is one: the bound is the whole of what
/// makes waiting here safe, and it has to be checkable without a render loop, a
/// window or a disk.
///
/// **It waits, rather than returning on acceptance, and that was the decision
/// worth arguing.** Answering the moment the loop has the request would make
/// this tool cheap and its answer worthless: a model told "saved" before the
/// disk has spoken will tell its user the set is kept, and the cases where that
/// is a lie — a full store, a network mount that stopped answering, a slot whose
/// sources are not savable — are precisely the ones anybody would want to hear
/// about. The other three tools already work this way: `write_procedure` hands
/// back the checker's verdict and not "it is being checked".
///
/// **A timeout is neither success nor failure, and the text says so.** The
/// protocol has one boolean and it cannot carry a third state, so `isError` is
/// set — a model reading `isError: false` reports the set as kept, which is the
/// one thing that must not happen here, while a model reading `true` looks
/// again. What the flag cannot carry, the sentence does.
fn awaited(news: &mpsc::Receiver<News>, wait: std::time::Duration) -> Result<String, String> {
    let deadline = std::time::Instant::now() + wait;
    let mut accepted: Option<String> = None;
    while let Some(left) = deadline.checked_duration_since(std::time::Instant::now()) {
        match news.recv_timeout(left) {
            Ok(News::Accepted(said)) => accepted = Some(said),
            Ok(News::Settled(outcome)) => return outcome,
            Err(mpsc::RecvTimeoutError::Timeout) => break,
            // **The loop dropped the request without answering it**, which is
            // what the end of a run looks like from here. Which of the two
            // sentences depends on whether it was ever taken: one that was
            // never taken saved nothing, and one that was may well have reached
            // the disk on the way out.
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(match accepted {
                    Some(said) => format!(
                        "{said}\n\nThe render loop then ended without saying what became of \
                         it. Whether that file was written is not something this server can \
                         still find out."
                    ),
                    None => "the render loop ended before it took this save: nothing was saved"
                        .to_string(),
                })
            }
        }
    }
    Err(match accepted {
        Some(said) => format!(
            "{said}\n\n**This is neither a success nor a failure.** The save was accepted \
             and had not reported back after {wait:?}. It is being written or it is not; \
             nothing here knows which, and no `save` record claims either way until it \
             lands. Do not report the set as kept — look for it under that id."
        ),
        None => format!(
            "the render loop had not taken this save after {wait:?} — it is running slowly \
             or not at all. Nothing was saved, and asking again is safe."
        ),
    })
}

/// **Wait for one edge to be applied, and say something true when it is not.**
///
/// **Not [`awaited`], because a save and an edge are not waiting for the same
/// kind of thing.** A save's third state is real and unavoidable — the loop took
/// it, the disk has not answered, and *neither a success nor a failure* is the
/// only honest report. An edge has no such state by construction: the loop
/// applies it at the frame it takes it and answers there, and everything slow
/// about it — the compile, the swap, the thirty judged frames — happens after
/// the answer and is `swap_outcome`'s to report. So the two sentences a timeout
/// can produce here are *it was not taken* and *it was taken and then the loop
/// went quiet*, and the second one is a loop at odds with what
/// [`WireRequest`] says it owes rather than an ordinary outcome.
///
/// **A run whose loop does not drain [`Reporter::wires`] at all ends up in the
/// first of those**, which is the point: a tool that reported success into a
/// channel nobody empties would be this surface claiming work that never
/// happened.
fn applied(news: &mpsc::Receiver<News>, wait: std::time::Duration) -> Result<String, String> {
    let deadline = std::time::Instant::now() + wait;
    let mut accepted: Option<String> = None;
    while let Some(left) = deadline.checked_duration_since(std::time::Instant::now()) {
        match news.recv_timeout(left) {
            Ok(News::Accepted(said)) => accepted = Some(said),
            Ok(News::Settled(outcome)) => return outcome,
            Err(mpsc::RecvTimeoutError::Timeout) => break,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(match accepted {
                    Some(said) => format!(
                        "{said}\n\nThe render loop then ended without saying what it did \
                         with the edge. Whether the slot was rewired is not something this \
                         server can still find out — `read_set` on a set saved since would \
                         say, and nothing else here will."
                    ),
                    None => "the render loop ended before it took this edge: nothing was \
                             rewired"
                        .to_string(),
                })
            }
        }
    }
    Err(match accepted {
        Some(said) => format!(
            "{said}\n\n**This is neither a success nor a failure.** The edge was taken and \
             the loop had not said what it did with it after {wait:?}, which is a frame it \
             should have answered on. Do not write the same edge again on the assumption \
             that it was lost — ask `swap_outcome` what the slot has been doing."
        ),
        None => format!(
            "the render loop had not taken this edge after {wait:?} — it is running slowly, \
             it is not running frames at all, or this run's loop does not take edges. \
             Nothing was rewired, and asking again is safe."
        ),
    })
}

mod resources;
pub(crate) use resources::*;

#[cfg(test)]
mod tests;
