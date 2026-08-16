//! Nodes, and the edges between them.
//!
//! **`Ln` is a node and a Set is a grouping around some** — `docs/roadmap.md`,
//! "a Set stops owning everything". The unit that owns GPU state is the node,
//! not the Set, which is what lets two L4 nodes read one L1 node's geometry for
//! one simulation, and what leaves a place for an L2 node to be inserted rather
//! than for a fixed pipeline to grow a third position.
//!
//! Two nodes live here today:
//!
//! - [`Simulation`], the **L1 node**: `() -> Geometry`. It owns the element and
//!   alive buffers, the counts, the compaction scan, the spawn accumulator, its
//!   pipelines and every bind group naming them.
//! - [`Renderer`], the **L4 node**: `(Geometry, Camera) -> Texture`. It owns its
//!   pipeline, its uniform, its accumulation targets under `blend weighted`, and
//!   the bind groups naming the geometry it reads.
//!
//! What is left in [`crate::set`] is the grouping: the camera, the parameter
//! values and their bindings, the viewport, the clock, and the order the nodes
//! run in.
//!
//! # The edges are typed, and one of them always was
//!
//! [`Geometry`] is the whole of what crosses from an L1 node to whatever is
//! downstream: an element layout, and the element and alive buffers indexed by
//! parity. A `Renderer` is built *against* one and is not portable to another —
//! its bind groups name those buffers and its generated `Element` struct is
//! compiled against that layout. That is not a limitation to lift later; it is
//! the slot interface contract, which `Set::build`'s `consumes ⊆ emit` check has
//! been enforcing informally since M1.
//!
//! [`View`] and [`Tick`] are the other direction: not edges between nodes but
//! what the *grouping* hands each node about the frame. They exist because one
//! clock and one camera serve every node in a Set, so a node holding its own
//! copy would be a second place for them to be — and two nodes in one Set could
//! then disagree about what "this frame" was.

mod renderer;
mod simulation;

pub(crate) use renderer::Renderer;
pub(crate) use simulation::Simulation;

use karakuri_codegen::layout::ElementLayout;

use crate::set::MAX_STEPS;

/// What an L1 node offers whatever reads it — the edge, as the resources that
/// cross it.
///
/// Borrowed rather than owned, and only for the duration of a build: a
/// `Renderer` keeps bind groups, and a bind group holds its buffers alive.
pub(crate) struct Geometry<'a> {
    /// The `Element` struct the L1 declared. An L4 declares the same one, byte
    /// for byte, because it reads the same physical buffer.
    pub layout: &'a ElementLayout,
    /// Indexed by parity: what L1 last wrote.
    pub elements: [&'a wgpu::Buffer; 2],
    pub alive: [&'a wgpu::Buffer; 2],
}

/// Everything the frame's uniform block needs that is not the node's own.
///
/// **The camera is here on borrowed time.** `L4 : (Geometry, Camera) -> Texture`
/// makes it an input *edge*, not a property of the grouping, and two renderers
/// reading different cameras is what a Set that composites two scenes is made
/// of. It rides in this struct because `Set` still owns one `Orbit`; when L3
/// becomes a node it becomes an edge like [`Geometry`] above, and a GPU buffer
/// rather than six numbers on the host — see `docs/ir-spec.md`, "L2 and L3".
pub(crate) struct View<'a> {
    pub t: f32,
    pub beats: f32,
    pub seed_salt: u32,
    pub viewport: [f32; 2],
    pub camera: &'a crate::camera::Orbit,
    /// One value per declared param, by name. The Set resolves bindings against
    /// its own state and hands the answer down; a node does not know what a
    /// binding is.
    ///
    /// **`None` rather than a panic** for a declared name with no scalar value —
    /// a vector param, which nothing drives yet. This is called on the render
    /// thread, and a panic there is not the way to find out that a `.kir`
    /// declared something the uniform path cannot write; see
    /// `Set::resolve_bindings`, which already says so about the same map.
    pub param: &'a dyn Fn(&str) -> Option<f32>,
}

/// Everything one frame of simulation needs that is the grouping's rather than
/// the node's.
pub(crate) struct Tick<'a> {
    /// How many substeps to run, already clamped to [`MAX_STEPS`].
    pub steps: u8,
    /// The fixed simulation step — [`crate::set::DT`].
    pub dt: f32,
    /// Substep `k`'s instant and its position on the tempo grid: `(t, beats)`.
    /// Only the first `steps` entries are read, and the rest are whatever the
    /// caller left there.
    ///
    /// **The pair is derived together, above.** `beats` is defined as the grid
    /// at the instant `t` names, so a node counting back from a step number of
    /// its own would arrive at the same number by a different route — which is
    /// the same number and a different claim. See `Oscillator::at_time`.
    pub instants: [(f32, f32); MAX_STEPS as usize],
    /// This frame's parameter values, on the same terms as [`View::param`].
    pub param: &'a dyn Fn(&str) -> Option<f32>,
}
