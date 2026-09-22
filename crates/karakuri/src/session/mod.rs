//! The session recorder's start/stop/poll state machine, and the save,
//! keep-procedure and rewire request queues an operator's key presses and an
//! MCP client's requests both land in.
//!
//! Both halves end on a disk or a watcher rather than on the frame that asked,
//! so both are the same shape: gather what is known in memory, hand the slow
//! part to a thread of its own, and read the outcome back at whichever frame it
//! arrives on. [`Sessions`] is the first of those and [`Keeping`] is the
//! second, and they are one module because `main.rs`'s own [`KeyCtx`] reaches
//! both from the same key press.
//!
//! [`KeyCtx`]: crate::KeyCtx

pub mod keeping;
pub mod recording;
pub mod watch;

pub(crate) use keeping::Keeping;
pub(crate) use recording::Sessions;
pub(crate) use watch::{rewired, watched};

/// Which deck slot's Set file a session's head carries.
///
/// A head says what every slot held — the others by the `procedure` records
/// that name their sources — and exactly one of them by a whole Set file, which
/// is what carries the params, the bindings and the seeds the rest are built
/// against. Slot 0, and not whichever deck happened to be selected: a replay
/// numbers its slots the way the deck did, so *which slot is described in full*
/// must not depend on where a hand was.
pub(crate) const HEAD_SLOT: usize = 0;
