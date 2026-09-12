//! A control surface for a model rather than for a pair of hands.
//!
//! `--mcp PORT` serves the Model Context Protocol over HTTP on the loopback
//! interface, so a chat client can read a slot's procedure, rewrite it, and be
//! told what the compiler and the frame budget made of the result. The name for
//! what that enables is vibe live coding: "the one that is showing now, a bit
//! more vivid" is a small edit to a declarative file, and the file is the thing
//! this system was already built to hot-swap.
//!
//! ## It is the third surface, and it obeys the same rule as the other two
//!
//! Keys, then MIDI, now this. The invariant
//! (`docs/principles/0090-a-surface-offers-it-never-decides.md`) is that
//! everything an operator moves goes through a record, is read back, and only
//! then applied — so that an agent is structurally incapable of doing anything
//! a human could not do through the same interface.
//!
//! An agent from outside the process is still an agent — and this surface is
//! the reason the invariant is now true of *material* as well as of the mix. It
//! was not: a procedure change was a file and not a record, so a session in
//! which a model rewrote slot 0 at minute ten replayed with the procedure it
//! started with, silently. The hole predated this module by as long as
//! `--watch` has existed, and nobody was going to be misled by a human typing
//! in vim; somebody would certainly have been misled by this.
//! `Record::Procedure` closes it, so a session driven by a model does replay
//! with no model attached, and what an agent did during a set can be watched
//! back.
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
//! What it does not do is hand the operation to `Live::operate`, and that is
//! `Silent`'s shape rather than an omission. `karakuri_operation_record`'s
//! `written` answers `Silent` for all seven: `Question` for the four that ask —
//! a record is what a replay reconstructs a performance from, and a question
//! changes no performance — `OnLanding` for the two whose record is written
//! where the work lands, `Record::Procedure` at the swap and `Record::Save` at
//! the frame the save landed, and `NoRecord` for `wire_input`. An operation
//! routed through `operate` that writes no record prints *no record* and does
//! nothing, which is `docs/adr/0198-…`'s finding about twelve of the keyboard's
//! keys and holds here for all seven tools. There is no `Live` on these threads
//! to route into either: this server reaches the render loop for two things,
//! and both go on the channels below.
//!
//! ## The seventh tool is a hole in the first paragraph of this file
//!
//! `Record::Procedure` closed the material half of P-0090 for a *procedure*.
//! `wire_input` reopens a strip of it: `written` answers
//! `Silent(Silent::NoRecord)` for `WireInput`, because `Record::Edge` is a Set
//! file's record and has no `slot` to carry the deck a live rewiring names. So
//! a model that binds `morph.far` at minute ten replays with the Set's launch
//! wiring — which, where the `uses` was written in the same session, is a
//! replay that does not build at all. That is stated here rather than left to
//! be discovered, it is asserted in `no_tool_writes_a_record_where_it_is_asked`
//! below, and what closes it is a `slot` on `Record::Edge` and a `written` arm
//! for it, neither of which is this file's to write.
//!
//! ## Most of this never touches the frame
//!
//! Reading a procedure is reading a file. Writing one is checking it and
//! writing a file — the compile, the frame-boundary swap, the measurement and
//! the stopped slot if it costs too much are `--watch`'s, built for editing by
//! hand and now doing the most dangerous part of this: a model that writes
//! something too expensive is caught by the machinery that already catches a
//! human who does.
//!
//! ## Two channels, and neither of them is a lock
//!
//! The outcome of a swap comes from the render loop over a channel rather than
//! a lock, so the frame path neither blocks nor waits. `save_set` is the first
//! tool that needs the *other* direction: what a slot is playing lives on the
//! render thread and is reachable from nowhere else, so a request goes back the
//! same way, is taken where the MIDI surface is taken, and ends in the method
//! the `k` key ends in. There is one save path in this program, and it is
//! `Live::save_set`; this is a way to ask for it and not a second copy of it.
//! What the two calls differ in is one argument — `karakuri_environment::Asked`
//! — and what it decides is the directory: a save asked for here lands in
//! `<store>/sandbox/` and the operator's own key writes the library
//! (`docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md`).
//!
//! The wait for its answer happens with [`State`] unlocked. There is a thread
//! per connection and one mutex over the state, so a tool that waited for a
//! disk while holding it would stop every other connection — including one that
//! only wanted to read a procedure — for as long as the loop took. [`Pending`]
//! exists for no other reason.
//!
//! ## Loopback only
//!
//! There is no bind option and there should not be one. A venue network is
//! shared, and a port that can rewrite what is on the projector is not
//! something to expose by a flag anyone might pass without meaning it. Reaching
//! a render machine from a laptop is `ssh -L`, which is a thing an operator
//! does deliberately and can see.

pub mod protocol;
pub mod resources;
pub mod server;
pub mod spelled;
pub mod state;
pub mod tools;

#[cfg(test)]
mod tests;

pub use karakuri_ir::{Diagnostic, DiagnosticReport};
pub use protocol::PROTOCOL;
pub use server::serve;
pub use state::{Event, Pointed, Reply, Reporter, SaveRequest, Slots, WireRequest};
pub use tools::{check_procedure, check_set_configuration, checked_id, OperateRequest, LISTED};

pub(crate) use protocol::*;
pub(crate) use resources::*;
pub(crate) use spelled::*;
pub(crate) use state::*;
pub(crate) use tools::*;
