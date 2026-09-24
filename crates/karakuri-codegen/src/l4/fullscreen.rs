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

/// Reconstructs view ray in world space for fullscreen fragment stage.
const FULLSCREEN_RAY: &str =
    "    let _ndc = vec2<f32>(in.point_coord.x * 2.0 - 1.0, 1.0 - in.point_coord.y * 2.0);
    let ray = normalize(cam.fwd + cam.right * _ndc.x + cam.up * _ndc.y);
";

/// Where a fullscreen shader reads it: at [`group::ATTRS`]'s number, which that
/// path leaves free by consuming no attribute.
const FULLSCREEN_CAMERA_GROUP: u32 = group::ATTRS;

/// Lowers an L4 procedure with [`Topology::Fullscreen`] to a fullscreen vertex/fragment pipeline.
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
    // Emits one u32 per declared source slot.
    for slot in checked.source_slots() {
        b.source_slot_field(slot);
    }
    // No `viewport` and no camera field of any kind: the ray basis is in the
    // camera's own bind group, and the projection is already in it.
    for p in &checked.params {
        b.param_field(p.name.clone(), wgsl_ty(p.ty));
    }
    // Spliced field parameters, prefixed with the slot name to avoid naming collisions.
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
    // Camera basis buffer is bound unconditionally for ray calculation.
    write_camera_binding(&mut src, FULLSCREEN_CAMERA_GROUP);
    // Absorb prelude requirements and functions needed by spliced fields.
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
        // Fullscreen passes have constant zero depth.
        src.push_str("    let _depth01 = 0.0;\n");
        src.push_str(WEIGHTED_FS_EPILOGUE);
        src.push_str("}\n");
    } else {
        src.push_str("    return _color;\n}\n");
    }

    // Retain element layout contract for compatibility with pipeline binding expectations.
    L4Shader {
        source: src,
        uniform_layout,
        uniform_pad_f32,
        element_layout: elements.clone(),
        camera_group: Some(FULLSCREEN_CAMERA_GROUP),
    }
}
