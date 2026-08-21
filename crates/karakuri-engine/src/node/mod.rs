//! Nodes, and the edges between them.
//!
//! **`Ln` is a node and a Set is a grouping around some** — `docs/roadmap.md`,
//! "a Set stops owning everything". The unit that owns GPU state is the node,
//! not the Set, which is what lets two L4 nodes read one L1 node's geometry for
//! one simulation, and what leaves a place for an L2 node to be inserted rather
//! than for a fixed pipeline to grow a third position.
//!
//! The nodes that live here:
//!
//! - [`Simulation`], the **L1 node**: `() -> Geometry`. It owns the element and
//!   alive buffers, the counts, the compaction scan, the spawn accumulator, its
//!   pipelines and every bind group naming them.
//! - [`Deform`], the **L2 node**: `Geometry -> Geometry`.
//! - [`Camera`], the **camera edge**: `() -> Camera`, and `Geometry -> Camera`
//!   once an L3 can be a procedure. It owns the state buffer, the pass that
//!   derives what a renderer reads from it, and the bind group every L4 names.
//! - [`Renderer`], the **L4 node**: `(Geometry, Camera) -> Texture`. It owns its
//!   pipeline, its uniform, its accumulation targets under `blend weighted`, and
//!   the bind groups naming the geometry and the camera it reads.
//! - [`Merge`], the **L5 node**: `[Texture] -> Texture`. Optional, and placing
//!   it is what turns several renderers from overdraw into compositing. It owns
//!   a target per input and the mix that folds them — the same mix the deck
//!   rides, since an L5 is one node kind with two roles.
//!
//! What is left in [`crate::set`] is the grouping: the parameter values and
//! their bindings, the viewport, the clock, and the order the nodes run in.
//!
//! # The edge from L1 has two halves, and only one of them has a type
//!
//! [`Geometry`] is the **build-time half**: an element layout, and the element
//! and alive buffers indexed by parity. A `Renderer` is built *against* one and
//! is not portable to another — its bind groups name those buffers and its
//! generated `Element` struct is compiled against that layout. That is not a
//! limitation to lift later; it is the slot interface contract, which
//! `Set::build`'s `consumes ⊆ emit` check has been enforcing informally since M1.
//!
//! **One value still crosses outside it: the parity.** The counts buffer used
//! to as well, and moved in when [`Deform`] arrived and needed it at build time
//! — it is a fixed buffer, so it belonged in the build-time half all along and
//! was outside only because the first reader happened to want it per frame.
//! Parity is genuinely per frame: which of two buffers holds what was last
//! written is a fact about *this* frame, so it is an argument to `record` and
//! `draw` rather than a field here.
//!
//! That leaves the edge honest: everything a reader must *name* is in this
//! struct, and the one thing that changes between frames is passed when it
//! changes.
//!
//! [`View`] and [`Tick`] are the other direction: not edges between nodes but
//! what the *grouping* hands each node about the frame. They exist because one
//! clock serves every node in a Set, so a node holding its own copy would be a
//! second place for it to be — and two nodes in one Set could then disagree
//! about what "this frame" was.
//!
//! **The camera used to ride in [`View`] and no longer does**, which is the
//! shape of what a node split is for: it was in the grouping's hand-down because
//! the host owned an `Orbit` and packed a matrix from it, and once it became a
//! node with an edge of its own there was nothing left for the host to pack.

mod camera;
mod deform;
mod merge;
mod renderer;
mod simulation;

pub(crate) use camera::Camera;
pub(crate) use deform::Deform;
pub(crate) use merge::Merge;
pub(crate) use renderer::Renderer;
pub(crate) use simulation::Simulation;

use karakuri_ir::layout::ElementLayout;

use crate::set::MAX_STEPS;

/// What an L1 node offers whatever reads it, as of build time: the resources a
/// reader has to name in a bind group.
///
/// **Not the whole edge** — the parity and the counts buffer cross every frame
/// and are not here; see the module doc.
///
/// Borrowed rather than owned, and only for the duration of a build: a
/// `Renderer` keeps bind groups, and a bind group holds its buffers alive.
pub(crate) struct Geometry<'a> {
    /// The `Element` struct the L1 declared. An L4 declares the same one, byte
    /// for byte, because it reads the same physical buffer.
    pub layout: &'a ElementLayout,
    /// Indexed by parity: what the upstream node last wrote.
    pub elements: [&'a wgpu::Buffer; 2],
    /// **The L1's, however far down a chain this edge is.** An L2 cannot
    /// `kill()`, so liveness is settled once by the compaction that runs after
    /// L1 and passes through every deformation untouched.
    pub alive: [&'a wgpu::Buffer; 2],
    /// The engine's per-frame counts, which is where the live range lives —
    /// what a reader dispatches or draws indirectly over. Not indexed by
    /// parity: there is one of it, and the simulation rewrites it in place.
    pub counts: &'a wgpu::Buffer,
}

/// Everything the frame's uniform block needs that is not the node's own.
///
/// **No camera.** `L4 : (Geometry, Camera) -> Texture` makes it an input *edge*
/// rather than a property of the grouping, and it left this struct when it
/// became one: a renderer names [`Camera`]'s bind group at build and reads it on
/// the GPU, so there is nothing about it for the host to pack. What is here is
/// what genuinely is the grouping's — one clock, one canvas, one salt.
pub(crate) struct View<'a> {
    pub t: f32,
    pub beats: f32,
    pub seed_salt: u32,
    pub viewport: [f32; 2],
    /// One value per declared param, by name. The Set resolves bindings against
    /// its own state and hands the answer down; a node does not know what a
    /// binding is.
    ///
    /// **`None` rather than a panic** for a declared name the Set has no scalar
    /// value under. This is called on the render thread, and a panic there is
    /// not the way to find out that a `.kir` declared something the uniform path
    /// cannot write; see `Set::resolve_bindings`, which already says so about the
    /// same map.
    ///
    /// It is not only vector params that land here. `Set::default_scalar` reads
    /// a *literal* float out of the declaration, so any `float` param whose
    /// default is an expression — `-0.35`, which parses as a negation of a
    /// literal — misses too. That is a defect of `default_scalar` rather than of
    /// this signature, and `0.0` is the wrong answer for it either way; it is
    /// merely a quieter wrong answer than the panic it replaced.
    pub param: &'a dyn Fn(&str) -> Option<f32>,
    /// **The spliced field's params, already under their WGSL names.**
    ///
    /// A field has no node, so nothing writes its uniform — every procedure
    /// that evaluates it carries its params in its own, and each of them writes
    /// these beside its own. The names are prefixed already because that is what
    /// the layout has: a caller and the field it evaluates may both declare
    /// `exposure`, and they are kept apart there rather than here.
    pub field_params: &'a [String],
    /// The value behind each of those names. Separate from [`View::param`]
    /// because they come from a different map: a field's params are the
    /// field's, addressed as `Field:0:…`, and every caller writes the same
    /// answer into its own uniform.
    pub field_value: &'a dyn Fn(&str) -> Option<f32>,
    /// **The identity of the geometry behind each declared Source slot**, by
    /// the key its uniform field carries.
    ///
    /// Separate from [`View::param`] and [`View::field_value`] because it comes
    /// from a third place and is a `u32`: what fills a Source slot is an L1's
    /// assigned salt, resolved once where the Set was built and looked up
    /// through the declaring node's own index — two nodes may each call a slot
    /// `only` and name different geometries.
    pub source_value: &'a dyn Fn(&str) -> Option<u32>,
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
    /// **The spliced field's params, already under their WGSL names.**
    ///
    /// A field has no node, so nothing writes its uniform — every procedure
    /// that evaluates it carries its params in its own, and each of them writes
    /// these beside its own. The names are prefixed already because that is what
    /// the layout has: a caller and the field it evaluates may both declare
    /// `exposure`, and they are kept apart there rather than here.
    pub field_params: &'a [String],
    /// The value behind each of those names. Separate from [`View::param`]
    /// because they come from a different map: a field's params are the
    /// field's, addressed as `Field:0:…`, and every caller writes the same
    /// answer into its own uniform.
    pub field_value: &'a dyn Fn(&str) -> Option<f32>,
    /// The same as [`View::source_value`], for the layer that has a `Tick`
    /// where every other has a `View`.
    pub source_value: &'a dyn Fn(&str) -> Option<u32>,
}

/// **Every declared param, packed as the layout declares it.**
///
/// A `float` gets the value the Set resolved for it — a binding's, an override's
/// or the declaration's default — and `0.0` when it has none, on the terms
/// [`View::param`] states. **A vector param gets zeroes**, because nothing in
/// this engine drives one: `Set::default_scalar` reads a scalar out of a
/// declaration and skips anything else, so a `vec3` param never enters a node's
/// value map at all.
///
/// The zeroes are not a choice so much as the honest form of what was already
/// true, and writing them is the part that was missing. Every node used to pack
/// *every* declared name as an `f32`, and the packer panics on a field its
/// layout says is a `vec3<f32>` — so a `.kir` declaring one parsed, checked,
/// costed, and then took the render thread down on the first `prepare`. That is
/// not the swap worker, so it was not caught as a `SetError::Panicked` either.
/// Skipping the field instead trips the packer's other assertion, which is the
/// one that keeps a half-written uniform from reaching a shader: the layout
/// declares the field, so something has to fill it.
///
/// One copy, called by all four nodes, so that a layer added later cannot
/// reintroduce the panic by writing its own loop.
/// The spliced field's params, for a node that has them.
///
/// **Filtered by the layout rather than by the caller.** A Set hands every node
/// the same list, and only the nodes that actually evaluate the field carry
/// those fields — so a node that does not must skip them, and the thing that
/// knows is the layout in its hand. Writing them blind panics the packer with
/// "uniform layout has no field", on the frame path.
pub(crate) fn write_field_params(
    p: &mut crate::uniforms::UniformPacker<'_>,
    layout: &karakuri_codegen::layout::UniformLayout,
    names: &[String],
    value: &dyn Fn(&str) -> Option<f32>,
) {
    let mine: Vec<String> = names
        .iter()
        .filter(|n| layout.fields.iter().any(|f| &f.name == *n))
        .cloned()
        .collect();
    write_params(p, layout, &mine, value);
}

/// **Every Source slot this node declared**, found in the layout rather than
/// listed by the caller.
///
/// The same trick [`write_field_params`] uses and for a stronger reason: a Set
/// hands every node one answer function, and which slots a node has is a fact
/// its own uniform layout already carries. Nothing else in the engine has to
/// keep a per-node list in step with the generator.
///
/// **`0` for a key the Set has no answer under is not a silent default**, it is
/// unreachable: a declared slot is bound or the Set was refused at build. It is
/// written rather than skipped because the packer refuses to produce bytes for
/// a layout field nothing set, which is the assertion that keeps a half-written
/// uniform away from a shader.
pub(crate) fn write_source_slots(
    p: &mut crate::uniforms::UniformPacker<'_>,
    layout: &karakuri_codegen::layout::UniformLayout,
    value: &dyn Fn(&str) -> Option<u32>,
) {
    let keys: Vec<String> = layout
        .fields
        .iter()
        .filter(|f| f.name.starts_with("source\u{1}"))
        .map(|f| f.name.clone())
        .collect();
    for key in keys {
        p.u32(&key, value(&key).unwrap_or(0));
    }
}

pub(crate) fn write_params(
    p: &mut crate::uniforms::UniformPacker<'_>,
    layout: &karakuri_codegen::layout::UniformLayout,
    names: &[String],
    value: &dyn Fn(&str) -> Option<f32>,
) {
    for name in names {
        let ty = layout
            .fields
            .iter()
            .find(|f| &f.name == name)
            .map(|f| f.wgsl_ty)
            .unwrap_or("f32");
        match ty {
            "vec2<f32>" => {
                p.vec2(name, [0.0; 2]);
            }
            "vec3<f32>" => {
                p.vec3(name, [0.0; 3]);
            }
            _ => {
                p.f32(name, value(name).unwrap_or(0.0));
            }
        }
    }
}
