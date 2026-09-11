//! Nodes, and the edges between them.
//!
//! **`Ln` is a node and a Set is a grouping around some**, so a Set stops
//! owning everything. The unit that owns GPU state is the node,
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
//! `Set::build`'s `consumes ⊆ emit` check has been enforcing informally all along.
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
pub(crate) type ParamValueLookup<'a> = &'a dyn Fn(&str) -> Option<karakuri_store::record::Value>;

/// Became one: a renderer names [`Camera`]'s bind group at build and reads it on
/// the GPU, so there is nothing about it for the host to pack. What is here is
/// what genuinely is the grouping's — one clock, one canvas, one salt.
pub(crate) struct View<'a> {
    pub t: f32,
    pub beats: f32,
    pub seed_salt: u32,
    pub viewport: [f32; 2],
    /// One value per **addressable key**, which is the declared name for a
    /// `float` and one component key per component for a `vec2` or a `vec3` —
    /// `glow.x`, `glow.y`, `glow.z`. The Set resolves bindings against its own
    /// state and hands the answer down; a node does not know what a binding is.
    ///
    /// **One `f32`, and no wider**, which is
    /// [ADR-0268](../../../../docs/adr/0268-a-vector-parameter-is-driven-one-component-at-a-time.md):
    /// everything downstream of this — a binding, a fader, a published control,
    /// a control change — is one number, so the component is part of the key
    /// rather than the value being three floats wide.
    ///
    /// **`None` rather than a panic** for a key the Set has no value under.
    /// This is called on the render thread, and a panic there is not the way to
    /// find out that a `.kir` declared something the fold cannot state; see
    /// `Set::resolve_bindings`, which already says so about the same map.
    ///
    /// What lands there is a **default the fold cannot state**, at any width: a
    /// literal with an optional sign is what `Param::default_components` reads,
    /// so a `float` whose default is an expression misses, and so does every
    /// component of a `vec3` whose constructor holds one. `0.0` is the wrong
    /// answer for it either way; it is merely a quieter wrong answer than the
    /// panic it replaced.
    pub param: &'a dyn Fn(&str) -> Option<f32>,
    /// Optional typed vector parameter value lookup. When present, enables
    /// direct packing of contiguous vector parameters without string formatting.
    pub param_value: Option<ParamValueLookup<'a>>,
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
    /// Optional typed vector parameter value lookup.
    pub param_value: Option<ParamValueLookup<'a>>,
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

/// **Every spliced field param this node carries**, written into the uniform
/// under its prefixed name.
///
/// **Layout-driven**, because a field's params are the union over every binding
/// and no one node carries all of them: a marcher's uniform may carry
/// `field\u{1}0\u{1}ball` and not `field\u{1}1\u{1}wave`, where a deformed mesh carries
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
    write_params(p, layout, &mine, value, None);
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

/// **Every declared param, packed as the layout declares it.**
///
/// A `float` gets the value the Set resolved for it — a binding's, an override's
/// or the declaration's default — and `0.0` when it has none, on the terms
/// [`View::param`] states.
///
/// **When a vector value is available**, it packs directly into the uniform buffer
/// without string formatting or individual component lookups. If component
/// modulation or binding overrides exist, it falls back to component lookup
/// under `glow.x`, `glow.y`, `glow.z`.
pub(crate) fn write_params(
    p: &mut crate::uniforms::UniformPacker<'_>,
    layout: &karakuri_codegen::layout::UniformLayout,
    names: &[String],
    value: &dyn Fn(&str) -> Option<f32>,
    vector_value: Option<ParamValueLookup<'_>>,
) {
    // The component of `name` the Set holds a value under, or `0.0`.
    //
    // **One buffer, reused, because this is the frame path.** A `String::new`
    // allocates nothing until it is written to, so a node with no vector param
    // — which is every node in `examples/` — pays exactly what it paid before;
    // one with a vector param allocates once on the first component and reuses
    // the capacity for the rest of the call. `format!` here would be an
    // allocation per component per node per frame.
    //
    // The key is composed by `karakuri_ir::push_component_key` and is not
    // spelled a second time here: a separator agreed by two crates is a
    // separator two crates can stop agreeing about.
    let mut key = String::new();
    let mut component = |name: &str, i: usize| {
        key.clear();
        karakuri_ir::push_component_key(&mut key, name, i);
        value(&key).unwrap_or(0.0)
    };
    for name in names {
        let ty = layout
            .fields
            .iter()
            .find(|f| &f.name == name)
            .map(|f| f.wgsl_ty)
            .unwrap_or("f32");
        match ty {
            "vec2<f32>" => {
                if let Some(karakuri_store::record::Value::Vec2(arr)) =
                    vector_value.and_then(|lookup| lookup(name))
                {
                    p.vec2(name, arr);
                    continue;
                }
                p.vec2(name, [component(name, 0), component(name, 1)]);
            }
            "vec3<f32>" => {
                if let Some(karakuri_store::record::Value::Vec3(arr)) =
                    vector_value.and_then(|lookup| lookup(name))
                {
                    p.vec3(name, arr);
                    continue;
                }
                p.vec3(
                    name,
                    [component(name, 0), component(name, 1), component(name, 2)],
                );
            }
            "vec4<f32>" => {
                if let Some(
                    karakuri_store::record::Value::Vec4(arr)
                    | karakuri_store::record::Value::Color(arr),
                ) = vector_value.and_then(|lookup| lookup(name))
                {
                    p.vec4(name, arr);
                    continue;
                }
                p.vec4(
                    name,
                    [
                        component(name, 0),
                        component(name, 1),
                        component(name, 2),
                        component(name, 3),
                    ],
                );
            }
            _ => {
                if let Some(karakuri_store::record::Value::Scalar(s)) =
                    vector_value.and_then(|lookup| lookup(name))
                {
                    p.f32(name, s);
                    continue;
                }
                p.f32(name, value(name).unwrap_or(0.0));
            }
        }
    }
}
