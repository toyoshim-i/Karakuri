//! The render graph, Set lifecycle, and pipeline management.
//!
//! The invariants here are the ones that break the project if they are broken
//! later, so they are worth restating where the code lives. **These are
//! one-line restatements; the canonical text is one file each in
//! `docs/principles/`, and where these disagree with it, this comment is the
//! one that is wrong.** They were kept here rather than replaced by a pointer
//! because whoever is reading this crate is exactly who needs them, and a link
//! out of the workspace does not resolve in rendered docs.
//!
//! - **Never allocate on the render thread. Never compile shaders on it.**
//! - Pipelines are double-buffered; swaps happen only on frame boundaries.
//! - If a new pipeline exceeds the frame budget, roll back automatically.
//!   (The last two live in [`swap`], which is where all four of these are
//!   under load at once and where the reasoning behind them is written down.)
//! - Every structural change forks a Set. A live Set is never mutated in place;
//!   parameter values are the one exception, and they are uniform writes.
//! - Element order is preserved. Compaction is order-preserving, which is what
//!   makes reproduction bit-exact — floating-point addition is not associative,
//!   so even additive blending depends on a stable order. The same reasoning
//!   fixes the order slots are composited in; see [`deck`].
//! - Simulation time advances by `steps * dt` from a `tick` record. A clock is
//!   read only to judge cost — the frame-interval watchdog in [`swap`] and the
//!   measurements in [`probe`] — and no value derived from one reaches
//!   simulation state.
//! - A frame is recorded through a guard that owns the command encoder, so a
//!   frame cannot be built from two generations of Sets. That is [`deck`];
//!   a [`swap::HotSwap`] driven on its own still relies on its caller, and
//!   says so.

pub mod binding;
pub mod camera;
pub mod compaction;
pub mod deck;
pub mod estimate;
pub mod frame;
pub mod governor;
pub mod gpu;
pub mod meter;
pub mod mix;
/// Private: the node types a Set is a grouping of. Nothing outside chooses
/// them — a `.kir`'s `kind` does — and the graph they form is [`set`]'s.
mod node;
/// Private: `blend weighted` is declared in a `.kir` and everything about how
/// it is run belongs to [`node`]. Nothing outside chooses these targets.
mod oit;
pub mod points;
pub mod present;
pub mod probe;
pub mod set;
/// Private: how large a node's per-element buffers are, which is a question
/// between a node and [`set::Plan`] and never a caller's. What the figure
/// *means* is public, as [`set::ElementStorage`].
mod storage;
pub mod swap;
pub mod transition;
pub mod transport;
pub mod uniforms;
pub mod video_source;

pub use binding::{Binding, Curve, ParamWrite, Signals};
pub use camera::Orbit;
pub use compaction::Compaction;
pub use deck::{Blend, Deck, Frame, Mask, MaskKind, Residency};
pub use estimate::{Estimate, Fit, Unfit, PREPARATION_RESOLUTION};
pub use frame::{compose, Committed, Look, Outcome, Sink, Skip, WindowSink};
pub use governor::{Decision, Governor, Reason, Report, SlotState};
pub use gpu::{Gpu, GpuError};
pub use meter::{Level, Meters};
pub use points::{Params, Points};
pub use present::{letterbox, Present, TonemapOp};
pub use probe::{Measurement, Probe};
pub use set::{Authority, Set, SetError};
pub use swap::{measure, Event, HotSwap, Request, Source, DEFAULT_BUDGET_MS};
pub use transition::{Control, Transition};
pub use video_source::VideoSource;
