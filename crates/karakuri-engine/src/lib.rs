//! The render graph, Set lifecycle, and GPU pipeline management.
//! Coordinates background compilation, frame budgeting, deterministic simulation, and presentation.

pub mod binding;
pub mod camera;
/// Master chain compilation on a worker thread.
pub mod chain_swap;
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
pub use chain_swap::{ChainEvent, ChainRefusal, ChainSlot, ChainSwap};
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
pub use master::{
    Chain, ChainTargets, ChainWorkshop, Clock, Cut, RetiredChain, Slot, SlotError, SlotParam,
    SlotReading, SlotSpec,
};
pub use meter::{Level, Meters};
pub use pass::{BoundImagePass, ImagePass, RenderPassNode, Retained, RetentionManager};
pub use points::{Params, Points};
pub use present::{letterbox, Present, TonemapOp};
pub use probe::{check_degeneracy, Degeneracy, Measurement, Probe};
pub use set::{Authority, Set, SetError};
pub use swap::{measure, Event, HotSwap, Request, Source, DEFAULT_BUDGET_MS};
pub use transition::{Control, Transition};
pub use video_source::VideoSource;
