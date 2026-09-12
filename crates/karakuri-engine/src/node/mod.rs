//! Pipeline nodes and intermediate edge resources.
//!
//! Defines the core node types ([`Simulation`], [`Deform`], [`Camera`], [`Renderer`],
//! [`Merge`]) and the intermediate data structures passed between them ([`Geometry`])
//! or provided by the frame environment ([`View`], [`Tick`]).

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

/// GPU resources provided by an upstream node (L1 or L2) at pipeline build time.
pub(crate) struct Geometry<'a> {
    /// Element memory layout declared upstream.
    pub layout: &'a ElementLayout,
    /// Ping-pong element buffers indexed by parity.
    pub elements: [&'a wgpu::Buffer; 2],
    /// Alive flag buffers indexed by parity.
    pub alive: [&'a wgpu::Buffer; 2],
    /// Indirect dispatch and draw counts buffer.
    pub counts: &'a wgpu::Buffer,
}

pub(crate) type ParamValueLookup<'a> = &'a dyn Fn(&str) -> Option<karakuri_store::record::Value>;

/// Frame-level timing, canvas, and parameter context provided to per-frame nodes (L2, L3, L4).
pub(crate) struct View<'a> {
    pub t: f32,
    pub beats: f32,
    pub seed_salt: u32,
    pub viewport: [f32; 2],
    /// Lookup for scalar parameter values by addressable key.
    pub param: &'a dyn Fn(&str) -> Option<f32>,
    /// Optional typed vector parameter value lookup.
    pub param_value: Option<ParamValueLookup<'a>>,
    /// Spliced field parameter names present in shader layout.
    pub field_params: &'a [String],
    /// Lookup for spliced field parameter values.
    pub field_value: &'a dyn Fn(&str) -> Option<f32>,
    /// Lookup for source slot geometry salt bindings.
    pub source_value: &'a dyn Fn(&str) -> Option<u32>,
}

/// Substep simulation timing and parameter context provided to L1 nodes.
pub(crate) struct Tick<'a> {
    /// Number of substeps to execute (capped at [`MAX_STEPS`]).
    pub steps: u8,
    /// Fixed simulation delta time (`DT`).
    pub dt: f32,
    /// Instant and tempo grid positions `(t, beats)` for each substep.
    pub instants: [(f32, f32); MAX_STEPS as usize],
    /// Scalar parameter lookup.
    pub param: &'a dyn Fn(&str) -> Option<f32>,
    /// Optional vector parameter lookup.
    pub param_value: Option<ParamValueLookup<'a>>,
    /// Spliced field parameter names.
    pub field_params: &'a [String],
    /// Spliced field parameter values.
    pub field_value: &'a dyn Fn(&str) -> Option<f32>,
    /// Source slot geometry salt lookup.
    pub source_value: &'a dyn Fn(&str) -> Option<u32>,
}

/// Writes spliced field parameters declared in `layout` into the uniform packer.
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

/// Writes source geometry salt values for declared source slots into the uniform packer.
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

/// Packs declared parameter values into the uniform buffer.
///
/// If vector values are present in `vector_value`, packs them directly;
/// otherwise resolves individual scalar components (`.x`, `.y`, `.z`, etc.).
pub(crate) fn write_params(
    p: &mut crate::uniforms::UniformPacker<'_>,
    layout: &karakuri_codegen::layout::UniformLayout,
    names: &[String],
    value: &dyn Fn(&str) -> Option<f32>,
    vector_value: Option<ParamValueLookup<'_>>,
) {
    // Reuse a single key buffer to avoid allocations on the frame path.
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
