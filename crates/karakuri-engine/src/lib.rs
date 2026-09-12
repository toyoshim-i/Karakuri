//! The render graph, Set lifecycle, and GPU pipeline management.
//!
//! # Architectural Invariants
//! - Zero allocation on the render thread; shaders are never compiled on the render thread.
//! - Pipelines are double-buffered; hot-swaps occur strictly on frame boundaries.
//! - Over-budget pipeline loads disable the offending slot rather than rolling back global state.
//! - Structural modifications fork a new Set instance; active Sets are never mutated in place.
//! - Element compaction preserves ordering to guarantee deterministic floating-point accumulation.
//! - Simulation time advances deterministically by `steps * dt` from tick records.
//! - Frame recording is serialized through guards owning command encoders to prevent generational tearing.

pub mod binding;
pub mod camera;
pub mod compaction;
pub mod deck;
pub mod estimate;
pub mod frame;
pub mod governor;
pub mod gpu;
pub mod graph;
/// Master compositing and post-processing chain.
pub mod master;
pub mod meter;
pub mod mix;
/// Internal node graph implementations.
mod node;
/// Order-independent transparency pipeline for weighted blending.
mod oit;
pub mod pass;
pub mod points;
pub mod present;
pub mod probe;
pub mod set;
/// Memory sizing helpers for simulation and deformation storage buffers.
mod storage;
pub mod swap;
pub mod transition;
pub mod transport;
pub mod uniforms;
pub mod video_source;

pub use binding::{Binding, Curve, ParamWrite, Signals};
pub use camera::Orbit;
pub use compaction::Compaction;
pub use deck::{Blend, Deck, DeckSlot, Frame, Mask, MaskKind, Residency};
pub use estimate::{Estimate, Fit, Unfit, PREPARATION_RESOLUTION};
pub use frame::{compose, Committed, Look, Outcome, Sink, Skip, WindowSink};
pub use governor::{Basis, Decision, Estimated, FloorRead, Governor, Reason, Report, SlotState};
pub use gpu::{Gpu, GpuError};
pub use graph::{
    BufferDesc, CompiledGraph, GpuPassFn, GraphError, GraphMetrics, GraphResource, MockPassFn,
    PassBuilder, PassExecution, PassId, PassNode, RenderGraph, ResourceId, ResourceResolver,
    TextureDesc, TransientMemoryPool,
};
pub use karakuri_store::record::{Layer, NodeAddress, Value};
pub use master::{Chain, Clock, Cut, Slot, SlotError, SlotSpec};
pub use meter::{Level, Meters};
pub use pass::{BoundImagePass, ImagePass, RenderPassNode, RetentionManager};
pub use points::{Params, Points};
pub use present::{letterbox, Present, TonemapOp};
pub use probe::{check_degeneracy, Degeneracy, Measurement, Probe};
pub use set::{Authority, Set, SetError};
pub use swap::{measure, Event, HotSwap, Request, Source, DEFAULT_BUDGET_MS};
pub use transition::{Control, Transition};
pub use video_source::VideoSource;
