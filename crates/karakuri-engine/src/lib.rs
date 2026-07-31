//! The render graph, Set lifecycle, and pipeline management.
//!
//! The invariants here are the ones that break the project if they are broken
//! later, so they are worth restating where the code lives:
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
//! - Simulation time advances by `steps * dt` from a `tick` record. Nothing in
//!   this crate reads a clock.
//! - A frame is recorded through a guard that owns the command encoder, so a
//!   frame cannot be built from two generations of Sets. That is [`deck`];
//!   a [`swap::HotSwap`] driven on its own still relies on its caller, and
//!   says so.

pub mod camera;
pub mod compaction;
pub mod deck;
pub mod gpu;
pub mod points;
pub mod present;
pub mod probe;
pub mod set;
pub mod swap;
pub mod uniforms;
pub mod video_source;

pub use camera::Orbit;
pub use compaction::Compaction;
pub use deck::{Deck, Frame, Residency};
pub use gpu::{Gpu, GpuError};
pub use points::{Params, Points};
pub use present::{Present, TonemapOp};
pub use probe::{Measurement, Probe};
pub use set::{Set, SetError};
pub use swap::{Event, HotSwap, Request, Source, DEFAULT_BUDGET_MS};
pub use video_source::VideoSource;
