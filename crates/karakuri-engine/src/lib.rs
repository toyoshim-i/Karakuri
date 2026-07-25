//! The render graph, Set lifecycle, and pipeline management.
//!
//! The invariants here are the ones that break the project if they are broken
//! later, so they are worth restating where the code lives:
//!
//! - **Never allocate on the render thread. Never compile shaders on it.**
//! - Pipelines are double-buffered; swaps happen only on frame boundaries.
//! - If a new pipeline exceeds the frame budget, roll back automatically.
//! - Every structural change forks a Set. A live Set is never mutated in place;
//!   parameter values are the one exception, and they are uniform writes.
//! - Element order is preserved. Compaction is order-preserving, which is what
//!   makes reproduction bit-exact — floating-point addition is not associative,
//!   so even additive blending depends on a stable order.
//! - Simulation time advances by `steps * dt` from a `tick` record. Nothing in
//!   this crate reads a clock.

pub mod camera;
pub mod gpu;
pub mod points;
pub mod present;
pub mod video_source;

pub use camera::Orbit;
pub use gpu::{Gpu, GpuError};
pub use points::{Params, Points};
pub use present::Present;
pub use video_source::VideoSource;
