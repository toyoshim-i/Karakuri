//! Fullscreen L4 rendering pipeline lowering.

use karakuri_ir::layout::ElementLayout;
use karakuri_ir::typed::{Checked, TBlock};

use super::{
    emit_stmts, write_camera_binding, L4Block, L4Resolver, L4Shader, WEIGHTED_FS_EPILOGUE,
    WEIGHTED_FS_OUT,
};
use crate::layout::{self, group, UniformLayoutBuilder};
use crate::prelude::{self, Requirements};
use crate::ty::wgsl_ty;

/// Vertex stage for [`Topology::Fullscreen`], emitting a single covering triangle (-1,-1), (3,-1), (-1,3) in NDC.
const FULLSCREEN_VS: &str = "struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) point_coord: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) i: u32) -> VsOut {
    // (0,0), (2,0), (0,2) in `point_coord`, so the frame is the 0..1 corner.
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var out: VsOut;
    out.clip = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    // Flipped in y: NDC runs up and a framebuffer runs down, and the whole
    // point of this value is that a fragment can say where on screen it is.
    out.point_coord = vec2<f32>(uv.x, 1.0 - uv.y);
    return out;
}

";

/// The ray, built once at the top of a fullscreen fragment stage.
///
/// `cam.right` and `cam.up` arrive pre-scaled by the field of view and the
/// aspect ratio — see `camera::State::basis` — so this is an interpolation and a
/// normalize rather than a projection. Everything about *which* projection is
/// on the engine's side of the seam, where the camera is.
const FULLSCREEN_RAY: &str =
    "    let _ndc = vec2<f32>(in.point_coord.x * 2.0 - 1.0, 1.0 - in.point_coord.y * 2.0);
    let ray = normalize(cam.fwd + cam.right * _ndc.x + cam.up * _ndc.y);
";

/// Where a fullscreen shader reads it: at [`group::ATTRS`]'s number, which that
/// path leaves free by consuming no attribute.
const FULLSCREEN_CAMERA_GROUP: u32 = group::ATTRS;

/// The whole of a [`Topology::Fullscreen`] shader.
///
/// Split out rather than branched into `generate_l4` because almost nothing is
/// shared: no element buffer is bound, no attribute is read, no varying is
/// chosen, and the vertex stage is [`FULLSCREEN_VS`] rather than anything the
/// procedure wrote. What *is* shared is the uniform — the same `t`, `beats` and
/// params every L4 gets — plus the four fields the ray needs.
pub(super) fn generate_fullscreen(
    checked: &Checked,
    fragment_blk: &TBlock,
    elements: &ElementLayout,
    weighted: bool,
    fields: crate::Bound<'_>,
) -> L4Shader {
    let mut b = UniformLayoutBuilder::new();
    b.field("t", "f32");
    b.field("beats", "f32");
    b.field("seed_salt", "u32");
    // **One `u32` per declared Source slot**, holding the identity of the
    // geometry an edge bound to it. A comparison against `source` is then two
    // uniform loads — the same value in every lane, which is the branch a GPU
    // costs least.
    for slot in checked.source_slots() {
        b.source_slot_field(slot);
    }
    // No `viewport` and no camera field of any kind: the ray basis is in the
    // camera's own bind group, and the projection is already in it.
    for p in &checked.params {
        b.param_field(p.name.clone(), wgsl_ty(p.ty));
    }
    // **A spliced field's params live here**, under a prefix of their own so
    // that this procedure and the field it evaluates may both declare
    // `exposure` — see `layout::mangle_field_param`.
    //
    // **One set per slot, and only for the slots the procedure evaluates.** The
    // field used to be spliced into every module in the Set, so a renderer that
    // never mentions one still carried its params and still failed to compile
    // if the field's body did — a `.kir` taking down shaders that have nothing
    // to do with it. The slot is in the name because two fields in one caller
    // are two independent sets of values.
    let splices = crate::splices(checked, fields);
    for f in &splices {
        for (name, ty) in &f.params {
            b.field_param_field(&f.slot, name, ty);
        }
    }
    let (uniform_layout, uniform_pad_f32) = b.finish();

    let mut req = Requirements::default();
    let body = {
        let resolver = L4Resolver::new(L4Block::Fragment);
        let mut out = String::new();
        emit_stmts(&fragment_blk.stmts, &resolver, &mut req, 1, &mut out);
        out
    };

    let mut src = String::new();
    layout::write_uniform_struct(&mut src, &uniform_layout, uniform_pad_f32);
    src.push_str("\n@group(0) @binding(0) var<uniform> u: Uniforms;\n\n");
    // **Unconditionally, unlike the per-element path.** [`FULLSCREEN_RAY`] is
    // emitted whether or not the procedure names `ray`, so this binding is
    // always read — a marcher with no ray in it would be a fullscreen quad, and
    // the one that draws a flat colour still pays for a basis it computes and
    // discards.
    write_camera_binding(&mut src, FULLSCREEN_CAMERA_GROUP);
    // **The field's helpers before its body, and its body before every entry
    // point.** A spliced field lives in this module, so this module's prelude
    // has to carry what it calls — the prelude is demand-driven, and a field
    // calling `sd_torus` in a caller that does not would otherwise produce a
    // call to a function nothing emitted, in a shader that checked clean.
    for f in &splices {
        req.absorb(&f.requirements);
    }
    src.push_str(&prelude::render(&req));
    for f in &splices {
        src.push('\n');
        src.push_str(&f.source);
    }
    src.push('\n');
    src.push_str(FULLSCREEN_VS);
    if weighted {
        src.push_str(WEIGHTED_FS_OUT);
        src.push_str("@fragment\nfn fs(in: VsOut) -> FsOut {\n");
    } else {
        src.push_str("@fragment\nfn fs(in: VsOut) -> @location(0) vec4<f32> {\n");
    }
    src.push_str("    let point_coord = in.point_coord;\n");
    src.push_str(FULLSCREEN_RAY);
    src.push_str("    var _color: vec4<f32>;\n");
    src.push_str(&body);
    if weighted {
        // **A frame is not at a depth**, so there is nothing to normalise
        // against the camera's planes and no `depth_range` in the uniform above.
        // Every fragment weighs the same, which for one layer per texel makes
        // the resolve the identity — and that is exactly why `Set::build`
        // refuses this pairing rather than paying two targets and a pass for it.
        // The generator stays total anyway: a rule about what a *Set* is worth
        // building is not a hole in what this function can lower.
        src.push_str("    let _depth01 = 0.0;\n");
        src.push_str(WEIGHTED_FS_EPILOGUE);
        src.push_str("}\n");
    } else {
        src.push_str("    return _color;\n}\n");
    }

    // Echoed back unchanged, as the per-element path does. Nothing here reads
    // an element — the check pass refuses a fullscreen `consumes` — but the
    // caller's contract is that an `L4Shader` says what buffer it expects, and
    // "the one its L1 wrote, and it reads none of it" is the honest answer.
    L4Shader {
        source: src,
        uniform_layout,
        uniform_pad_f32,
        element_layout: elements.clone(),
        camera_group: Some(FULLSCREEN_CAMERA_GROUP),
    }
}
