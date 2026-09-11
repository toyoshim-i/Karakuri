//! Pass Fusion: inlining L2 deformation directly into L4 rendering stages.
//!
//! Eliminates intermediate VRAM ping-pong buffers by embedding the L2 deformation
//! logic as a pure in-shader function evaluated within the L4 vertex stage.

use std::collections::HashSet;

use karakuri_ir::layout::ElementLayout;
use karakuri_ir::typed::{Checked, TBlock, TExpr, TExprKind, TStmt, Target};
use karakuri_ir::{Ambient, Attr, Blend, BlockKind, Kind, Output, Topology};

use crate::l4::L4Shader;
use crate::layout::{self, group, mangle_param, UniformLayoutBuilder};
use crate::lower::{lower_expr, mangle_local, Resolver};
use crate::prelude::{self, Requirements};
use crate::ty::wgsl_ty;

/// A fused shader is an L4 render shader containing inlined L2 deformation.
pub type FusedShader = L4Shader;

const CAMERA_GROUP: u32 = 2;

const CORNER_OF: &str = "\
// The six corners of a unit quad, two triangles, in [0, 1].
fn corner_of(i: u32) -> vec2<f32> {
    switch i {
        case 0u: { return vec2<f32>(0.0, 0.0); }
        case 1u: { return vec2<f32>(1.0, 0.0); }
        case 2u: { return vec2<f32>(0.0, 1.0); }
        case 3u: { return vec2<f32>(0.0, 1.0); }
        case 4u: { return vec2<f32>(1.0, 0.0); }
        default: { return vec2<f32>(1.0, 1.0); }
    }
}
";

const SEGMENT_EXPANSION: &str = "\
    let corner = corner_of(corner_idx);
    let half_vp = u.viewport * 0.5;
    let a_px = _clip.xy / _clip.w * half_vp;
    let b_px = _clip_b.xy / _clip_b.w * half_vp;
    let seg = b_px - a_px;
    let dir = seg / max(length(seg), 1e-6);
    let _width_px = _point_rate * u.viewport.y;
    let _drawn_px = max(_width_px, 1.0);
    let across = vec2<f32>(-dir.y, dir.x) * (_drawn_px * 0.5) * (corner.y * 2.0 - 1.0);
    let p_px = mix(a_px, b_px, corner.x) + across;
    let w = mix(_clip.w, _clip_b.w, corner.x);
    let z = mix(_clip.z / _clip.w, _clip_b.z / _clip_b.w, corner.x);
    out.clip = vec4<f32>(p_px / half_vp * w, z * w, w);
    out.point_coord = corner;
    out.coverage = clamp(_width_px, 0.0, 1.0);
";

const WEIGHTED_FS_OUT: &str = "struct FsOut {
    @location(0) accum: vec4<f32>,
    @location(1) reveal: f32,
};

";

const WEIGHTED_FS_EPILOGUE: &str = "    let _a = clamp(_color.a, 0.0, 1.0);
    let _w = _a * max(1e-2, pow(1.0 - _depth01, 3.0));
    var _out: FsOut;
    _out.accum = vec4<f32>(_color.rgb * _a * _w, _a * _w);
    _out.reveal = _a;
    return _out;
";

const WEIGHTED_DEPTH: &str = "\
    let _near = cam.planes.x;
    let _far = cam.planes.y;
    let _depth01 = clamp((in.view_depth - _near) / (_far - _near), 0.0, 1.0);
";

/// Resolves expressions inside the inlined L2 deformation function.
struct L2FusedResolver {
    has_copy: bool,
    derived: Vec<Attr>,
}

impl Resolver for L2FusedResolver {
    fn read_attr(&self, attr: Attr) -> String {
        if self.derived.contains(&attr) {
            return match attr.derivation() {
                Some(karakuri_ir::Derivation::SinceBirth) => "(u.t - elem.birth_t)".to_string(),
                other => unreachable!("{other:?} is not synthesised at the read site"),
            };
        }
        format!("elem.{}", attr.name())
    }

    fn read_seed(&self) -> String {
        "seed".to_string()
    }

    fn read_ambient(&self, amb: Ambient) -> String {
        match amb {
            Ambient::Copy if self.has_copy => "elem.copy".to_string(),
            Ambient::Copy => "0u".to_string(),
            Ambient::Point => unreachable!("`point` is a field's only input"),
            Ambient::Capacity => "u.capacity".to_string(),
            Ambient::Source => "u.seed_salt".to_string(),
            Ambient::T => "u.t".to_string(),
            Ambient::Beats => "u.beats".to_string(),
            Ambient::Dt => "u.dt".to_string(),
            Ambient::Seed => unreachable!("read_seed handles this"),
            Ambient::Camera | Ambient::PointCoord | Ambient::Eye | Ambient::Ray => {
                unreachable!("{amb:?} is L4-only")
            }
        }
    }

    fn read_param(&self, name: &str) -> Option<String> {
        Some(format!("u.{}", mangle_param(&format!("l2_{name}"))))
    }
}

fn emit_l2_stmts(
    stmts: &[TStmt],
    r: &L2FusedResolver,
    req: &mut Requirements,
    indent: usize,
    out: &mut String,
) {
    let pad = "    ".repeat(indent);
    for stmt in stmts {
        match stmt {
            TStmt::Let { name, value, .. } => {
                let v = lower_expr(value, r, req);
                out.push_str(&format!("{pad}let {} = {v};\n", mangle_local(name)));
            }
            TStmt::Var { name, value, .. } => {
                let v = lower_expr(value, r, req);
                out.push_str(&format!("{pad}var {} = {v};\n", mangle_local(name)));
            }
            TStmt::Assign { target, value, .. } => {
                let v = lower_expr(value, r, req);
                match target {
                    Target::Local(name) => {
                        out.push_str(&format!("{pad}{} = {v};\n", mangle_local(name)))
                    }
                    Target::Attr(a) => out.push_str(&format!("{pad}elem.{} = {v};\n", a.name())),
                    Target::Output(Output::Strength) => {
                        out.push_str(&format!("{pad}strength = {v};\n"));
                    }
                    Target::Output(o) => unreachable!("L2 never assigns stage output {o:?}"),
                }
            }
            TStmt::If {
                cond, then, els, ..
            } => {
                let c = lower_expr(cond, r, req);
                out.push_str(&format!("{pad}if {c} {{\n"));
                emit_l2_stmts(then, r, req, indent + 1, out);
                if els.is_empty() {
                    out.push_str(&format!("{pad}}}\n"));
                } else {
                    out.push_str(&format!("{pad}}} else {{\n"));
                    emit_l2_stmts(els, r, req, indent + 1, out);
                    out.push_str(&format!("{pad}}}\n"));
                }
            }
            TStmt::For {
                var,
                start,
                end,
                body,
                ..
            } => {
                let v = mangle_local(var);
                out.push_str(&format!(
                    "{pad}for (var {v}: i32 = {start}; {v} < {end}; {v} = {v} + 1) {{\n"
                ));
                emit_l2_stmts(body, r, req, indent + 1, out);
                out.push_str(&format!("{pad}}}\n"));
            }
            TStmt::Kill { .. } => unreachable!("kill() is L1 only"),
        }
    }
}

enum L4Block {
    Vertex,
    Fragment,
}

struct L4FusedResolver {
    block: L4Block,
    camera_used: std::cell::Cell<bool>,
}

impl Resolver for L4FusedResolver {
    fn read_attr(&self, attr: Attr) -> String {
        match self.block {
            L4Block::Vertex => attr.name().to_string(),
            L4Block::Fragment => format!("in.{}", attr.name()),
        }
    }

    fn read_seed(&self) -> String {
        match self.block {
            L4Block::Vertex => "seed".to_string(),
            L4Block::Fragment => "in.seed".to_string(),
        }
    }

    fn read_ambient(&self, amb: Ambient) -> String {
        match amb {
            Ambient::Copy => match self.block {
                L4Block::Vertex => "copy".to_string(),
                L4Block::Fragment => "in.copy".to_string(),
            },
            Ambient::Point => unreachable!("point is field only"),
            Ambient::T => "u.t".to_string(),
            Ambient::Source => "u.seed_salt".to_string(),
            Ambient::Beats => "u.beats".to_string(),
            Ambient::Camera => {
                self.camera_used.set(true);
                "cam.view_proj".to_string()
            }
            Ambient::PointCoord => match self.block {
                L4Block::Fragment => "in.point_coord".to_string(),
                L4Block::Vertex => unreachable!("point_coord is fragment-only"),
            },
            Ambient::Eye => {
                self.camera_used.set(true);
                "cam.eye".to_string()
            }
            Ambient::Ray => "ray".to_string(),
            Ambient::Seed => unreachable!("read_seed handles this"),
            Ambient::Capacity => "u.capacity".to_string(),
            Ambient::Dt => "u.dt".to_string(),
        }
    }

    fn read_param(&self, name: &str) -> Option<String> {
        Some(format!("u.{}", mangle_param(&format!("l4_{name}"))))
    }
}

fn output_local(o: Output) -> &'static str {
    match o {
        Output::Clip => "_clip",
        Output::ClipB => "_clip_b",
        Output::PointRate => "_point_rate",
        Output::Color => "_color",
        other => unreachable!("{other:?} belongs to camera block"),
    }
}

fn emit_l4_stmts(
    stmts: &[TStmt],
    resolver: &L4FusedResolver,
    req: &mut Requirements,
    indent: usize,
    out: &mut String,
) {
    let pad = "    ".repeat(indent);
    for stmt in stmts {
        match stmt {
            TStmt::Let { name, value, .. } => {
                let v = lower_expr(value, resolver, req);
                out.push_str(&format!("{pad}let {} = {v};\n", mangle_local(name)));
            }
            TStmt::Var { name, value, .. } => {
                let v = lower_expr(value, resolver, req);
                out.push_str(&format!("{pad}var {} = {v};\n", mangle_local(name)));
            }
            TStmt::Assign { target, value, .. } => {
                let v = lower_expr(value, resolver, req);
                match target {
                    Target::Local(name) => {
                        out.push_str(&format!("{pad}{} = {v};\n", mangle_local(name)))
                    }
                    Target::Output(o) => {
                        out.push_str(&format!("{pad}{} = {v};\n", output_local(*o)))
                    }
                    Target::Attr(a) => {
                        unreachable!("L4 never assigns attribute {a:?}")
                    }
                }
            }
            TStmt::If {
                cond, then, els, ..
            } => {
                let c = lower_expr(cond, resolver, req);
                out.push_str(&format!("{pad}if {c} {{\n"));
                emit_l4_stmts(then, resolver, req, indent + 1, out);
                if els.is_empty() {
                    out.push_str(&format!("{pad}}}\n"));
                } else {
                    out.push_str(&format!("{pad}}} else {{\n"));
                    emit_l4_stmts(els, resolver, req, indent + 1, out);
                    out.push_str(&format!("{pad}}}\n"));
                }
            }
            TStmt::For {
                var,
                start,
                end,
                body,
                ..
            } => {
                let v = mangle_local(var);
                out.push_str(&format!(
                    "{pad}for (var {v}: i32 = {start}; {v} < {end}; {v} = {v} + 1) {{\n"
                ));
                emit_l4_stmts(body, resolver, req, indent + 1, out);
                out.push_str(&format!("{pad}}}\n"));
            }
            TStmt::Kill { .. } => unreachable!("kill() is L1 only"),
        }
    }
}

fn scan_expr(e: &TExpr, seed: &mut bool, copy: &mut bool, attrs: &mut HashSet<Attr>) {
    match &e.kind {
        TExprKind::Far(_) => unreachable!("far read in L4"),
        TExprKind::Attr(a) => {
            attrs.insert(*a);
        }
        TExprKind::Ambient(Ambient::Seed) => *seed = true,
        TExprKind::Ambient(Ambient::Copy) => *copy = true,
        TExprKind::Ambient(_)
        | TExprKind::Lit(_)
        | TExprKind::Local(_)
        | TExprKind::Param(_)
        | TExprKind::Source { .. } => {}
        TExprKind::Unary { value, .. } => scan_expr(value, seed, copy, attrs),
        TExprKind::Binary { lhs, rhs, .. } => {
            scan_expr(lhs, seed, copy, attrs);
            scan_expr(rhs, seed, copy, attrs);
        }
        TExprKind::Field { point, .. } => scan_expr(point, seed, copy, attrs),
        TExprKind::Sample { at, .. } => {
            if let Some(a) = at {
                scan_expr(a, seed, copy, attrs);
            }
        }
        TExprKind::Builtin { args, .. } | TExprKind::Construct { args } => {
            for a in args {
                scan_expr(a, seed, copy, attrs);
            }
        }
        TExprKind::Swizzle { value, .. } => scan_expr(value, seed, copy, attrs),
    }
}

fn scan_stmts(stmts: &[TStmt], seed: &mut bool, copy: &mut bool, attrs: &mut HashSet<Attr>) {
    for s in stmts {
        match s {
            TStmt::Let { value, .. } | TStmt::Var { value, .. } | TStmt::Assign { value, .. } => {
                scan_expr(value, seed, copy, attrs)
            }
            TStmt::If {
                cond, then, els, ..
            } => {
                scan_expr(cond, seed, copy, attrs);
                scan_stmts(then, seed, copy, attrs);
                scan_stmts(els, seed, copy, attrs);
            }
            TStmt::For { body, .. } => scan_stmts(body, seed, copy, attrs),
            TStmt::Kill { .. } => {}
        }
    }
}

fn used_in_fragment(block: &TBlock) -> (bool, bool, HashSet<Attr>) {
    let mut seed = false;
    let mut copy = false;
    let mut attrs = HashSet::new();
    scan_stmts(&block.stmts, &mut seed, &mut copy, &mut attrs);
    (seed, copy, attrs)
}

#[derive(Clone, Copy)]
struct Identity {
    seed: bool,
    copy: bool,
    has_copy_slot: bool,
}

fn write_vsout_struct(out: &mut String, id: Identity, attrs_used: &[Attr], depth: bool) {
    out.push_str("struct VsOut {\n");
    out.push_str("    @builtin(position) clip: vec4<f32>,\n");
    out.push_str("    @location(0) point_coord: vec2<f32>,\n");
    let mut loc = 1;
    if id.seed {
        out.push_str(&format!(
            "    @location({loc}) @interpolate(flat) seed: u32,\n"
        ));
        loc += 1;
    }
    if id.copy {
        out.push_str(&format!(
            "    @location({loc}) @interpolate(flat) copy: u32,\n"
        ));
        loc += 1;
    }
    for &a in attrs_used {
        out.push_str(&format!(
            "    @location({loc}) @interpolate(flat) {}: {},\n",
            a.name(),
            wgsl_ty(a.ty())
        ));
        loc += 1;
    }
    if depth {
        out.push_str(&format!("    @location({loc}) view_depth: f32,\n"));
        loc += 1;
    }
    out.push_str(&format!(
        "    @location({loc}) @interpolate(flat) coverage: f32,\n"
    ));
    out.push_str("};\n");
}

fn write_element_bindings(out: &mut String, layout: &ElementLayout) {
    layout::write_element_struct(out, layout);
    out.push('\n');
    out.push_str(&format!(
        "@group({}) @binding({}) var<storage, read> elements: array<Element>;\n",
        group::ATTRS,
        layout::binding::ELEMENT,
    ));
    out.push_str(&format!(
        "@group({}) @binding({}) var<storage, read> alive: array<u32>;\n",
        group::ATTRS,
        layout::binding::ALIVE,
    ));
}

fn write_camera_binding(out: &mut String, at: u32) {
    out.push_str(layout::camera::WGSL);
    out.push_str(&format!(
        "\n@group({at}) @binding(0) var<uniform> cam: Camera;\n\n"
    ));
}

/// Fuses an L2 deformation procedure into an L4 rendering procedure.
pub fn fuse_l2_into_l4(
    l2: &Checked,
    l4: &Checked,
    elements: &ElementLayout,
    fields: crate::Bound<'_>,
) -> L4Shader {
    assert_eq!(l2.kind, Kind::L2, "l2 argument must be Kind::L2");
    assert_eq!(l4.kind, Kind::L4, "l4 argument must be Kind::L4");
    assert_ne!(
        l4.topology,
        Some(Topology::Fullscreen),
        "cannot fuse L2 deformation into fullscreen L4 renderer"
    );

    let topology = l4.topology.unwrap_or(Topology::Points);
    let weighted = l4.blend == Some(Blend::Weighted);

    // Build fused uniform layout.
    let mut b = UniformLayoutBuilder::new();
    b.field("t", "f32");
    b.field("beats", "f32");
    b.field("dt", "f32");
    b.field("capacity", "u32");
    b.field("seed_salt", "u32");
    b.field("viewport", "vec2<f32>");

    // Merge source slots from both L2 and L4.
    let mut source_slots: Vec<String> = l2
        .source_slots()
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    for slot in l4.source_slots() {
        let slot_str = slot.to_string();
        if !source_slots.contains(&slot_str) {
            source_slots.push(slot_str);
        }
    }
    for slot in &source_slots {
        b.source_slot_field(slot);
    }

    // Namespace L2 and L4 parameters to eliminate collisions.
    for p in &l2.params {
        b.param_field(format!("l2_{}", p.name), wgsl_ty(p.ty));
    }
    for p in &l4.params {
        b.param_field(format!("l4_{}", p.name), wgsl_ty(p.ty));
    }

    // Spliced fields for both L2 and L4.
    let l2_splices = crate::splices(l2, fields);
    for f in &l2_splices {
        for (name, ty) in &f.params {
            b.field_param_field(&format!("l2_{}", f.slot), name, ty);
        }
    }
    let l4_splices = crate::splices(l4, fields);
    for f in &l4_splices {
        for (name, ty) in &f.params {
            b.field_param_field(&f.slot, name, ty);
        }
    }

    let (uniform_layout, uniform_pad_f32) = b.finish();

    let mut req = Requirements::default();

    // 1. Lower L2 deformation function: `deform_element(in_elem: Element, seed: u32) -> Element`
    let l2_resolver = L2FusedResolver {
        has_copy: elements.has_slot("copy"),
        derived: elements.derived.clone(),
    };
    let deform_blk = l2
        .block(BlockKind::Deform)
        .expect("an L2 procedure must have a deform block");
    let mut l2_body = String::new();
    emit_l2_stmts(&deform_blk.stmts, &l2_resolver, &mut req, 1, &mut l2_body);

    let weight_opt = l2.params.iter().any(|p| p.name == "weight").then(|| {
        format!(
            "    strength = strength * u.{};\n",
            mangle_param("l2_weight")
        )
    });
    let mask_opt = l2.block(BlockKind::Mask).map(|block| {
        let mut out = String::new();
        emit_l2_stmts(&block.stmts, &l2_resolver, &mut req, 2, &mut out);
        out
    });

    let (gate_decl, gate_apply) = match (&mask_opt, &weight_opt) {
        (None, None) => (String::new(), String::new()),
        _ => {
            let mut blend = String::new();
            for slot in elements.slots.iter().filter(|s| s.attr.is_some()) {
                blend.push_str(&format!(
                    "    elem.{0} = mix(_input.{0}, elem.{0}, _gate);\n",
                    slot.name
                ));
            }
            let mask_str = mask_opt.as_deref().unwrap_or_default();
            let weight_str = weight_opt.as_deref().unwrap_or_default();
            (
                format!(
                    "    var strength = 1.0;\n    {{\n{mask_str}    }}\n{weight_str}    let _gate = clamp(strength, 0.0, 1.0);\n"
                ),
                blend,
            )
        }
    };

    let mut deform_fn = String::new();
    deform_fn.push_str("fn deform_element(in_elem: Element, seed: u32) -> Element {\n");
    deform_fn.push_str("    var elem = in_elem;\n");
    if !gate_decl.is_empty() {
        deform_fn.push_str("    let _input = in_elem;\n");
        deform_fn.push_str(&gate_decl);
    }
    deform_fn.push_str(&l2_body);
    if !gate_apply.is_empty() {
        deform_fn.push_str(&gate_apply);
    }
    deform_fn.push_str("    return elem;\n}\n");

    // 2. Lower L4 vertex and fragment stages.
    let vertex_blk = l4
        .block(BlockKind::Vertex)
        .expect("an L4 procedure has a vertex block");
    let fragment_blk = l4
        .block(BlockKind::Fragment)
        .expect("an L4 procedure has a fragment block");

    let (seed_used, copy_used, attrs_used_set) = used_in_fragment(fragment_blk);
    let id = Identity {
        seed: seed_used,
        copy: copy_used,
        has_copy_slot: elements.has_slot("copy"),
    };
    let attrs_used: Vec<Attr> = l4
        .consumes
        .iter()
        .copied()
        .filter(|a| attrs_used_set.contains(a))
        .collect();

    let vertex_res = L4FusedResolver {
        block: L4Block::Vertex,
        camera_used: std::cell::Cell::new(false),
    };
    let mut vertex_body = String::new();
    emit_l4_stmts(
        &vertex_blk.stmts,
        &vertex_res,
        &mut req,
        1,
        &mut vertex_body,
    );

    let fragment_res = L4FusedResolver {
        block: L4Block::Fragment,
        camera_used: std::cell::Cell::new(false),
    };
    let mut fragment_body = String::new();
    emit_l4_stmts(
        &fragment_blk.stmts,
        &fragment_res,
        &mut req,
        1,
        &mut fragment_body,
    );

    let camera_group = (vertex_res.camera_used.get() || fragment_res.camera_used.get() || weighted)
        .then_some(CAMERA_GROUP);

    // Assemble shader module WGSL source.
    let mut src = String::new();
    layout::write_uniform_struct(&mut src, &uniform_layout, uniform_pad_f32);
    src.push_str("\n@group(0) @binding(0) var<uniform> u: Uniforms;\n\n");
    write_element_bindings(&mut src, elements);
    src.push('\n');
    if let Some(g) = camera_group {
        write_camera_binding(&mut src, g);
    }

    for f in l2_splices.iter().chain(l4_splices.iter()) {
        req.absorb(&f.requirements);
    }
    src.push_str(&prelude::render(&req));
    for f in &l2_splices {
        src.push('\n');
        src.push_str(&f.source);
    }
    for f in &l4_splices {
        src.push('\n');
        src.push_str(&f.source);
    }
    src.push('\n');
    src.push_str(CORNER_OF);
    src.push('\n');
    write_vsout_struct(&mut src, id, &attrs_used, weighted);
    src.push('\n');
    if weighted {
        src.push_str(WEIGHTED_FS_OUT);
    }

    // Embed the L2 deformation function.
    src.push_str(&deform_fn);
    src.push('\n');

    // Emit the fused vertex entry point `vs`.
    src.push_str("@vertex\n");
    src.push_str("fn vs(@builtin(vertex_index) corner_idx: u32, @builtin(instance_index) elem_idx: u32) -> VsOut {\n");
    src.push_str("    let seed = elements[elem_idx].seed;\n");
    src.push_str("    var elem = elements[elem_idx];\n");
    src.push_str("    elem = deform_element(elem, seed);\n");
    src.push_str(if id.has_copy_slot {
        "    let copy = elem.copy;\n"
    } else {
        "    let copy = 0u;\n"
    });
    for &a in &l4.consumes {
        if elements.derived.contains(&a) {
            src.push_str(&format!("    let {} = u.t - elem.birth_t;\n", a.name()));
            continue;
        }
        src.push_str(&format!("    let {} = elem.{};\n", a.name(), a.name()));
    }
    src.push_str("    var _clip: vec4<f32>;\n");
    if topology == Topology::Lines {
        src.push_str("    var _clip_b: vec4<f32>;\n");
    }
    src.push_str("    var _point_rate: f32;\n");
    src.push_str(&vertex_body);
    src.push_str("    var out: VsOut;\n");

    match topology {
        Topology::Points => {
            src.push_str("    let corner = corner_of(corner_idx) * 2.0 - 1.0;\n");
            src.push_str("    let _side_px = _point_rate * u.viewport.y;\n");
            src.push_str("    let _drawn_px = max(_side_px, 1.0);\n");
            src.push_str("    let _rate_ndc = _drawn_px / u.viewport;\n");
            src.push_str("    let ndc_offset = corner * _rate_ndc * _clip.w;\n");
            src.push_str("    out.clip = vec4<f32>(_clip.xy + ndc_offset, _clip.zw);\n");
            src.push_str("    out.point_coord = corner_of(corner_idx);\n");
            src.push_str("    let _cov = clamp(_side_px, 0.0, 1.0);\n");
            src.push_str("    out.coverage = _cov * _cov;\n");
            if weighted {
                src.push_str("    out.view_depth = _clip.w;\n");
            }
        }
        Topology::Lines => {
            src.push_str(SEGMENT_EXPANSION);
            if weighted {
                src.push_str("    out.view_depth = w;\n");
            }
        }
        Topology::Fullscreen => unreachable!(),
    }

    if id.seed {
        src.push_str("    out.seed = seed;\n");
    }
    if id.copy {
        src.push_str("    out.copy = copy;\n");
    }
    for &a in &attrs_used {
        src.push_str(&format!("    out.{0} = {0};\n", a.name()));
    }
    src.push_str("    return out;\n}\n\n");

    // Emit fragment entry point `fs`.
    let ret_sig = if weighted {
        "-> FsOut"
    } else {
        "-> @location(0) vec4<f32>"
    };
    src.push_str(&format!("@fragment\nfn fs(in: VsOut) {ret_sig} {{\n"));
    src.push_str("    var _color: vec4<f32>;\n");
    src.push_str(&fragment_body);
    src.push_str("    _color = vec4<f32>(_color.rgb, _color.a * in.coverage);\n");
    if weighted {
        src.push_str(WEIGHTED_DEPTH);
        src.push_str(WEIGHTED_FS_EPILOGUE);
    } else {
        src.push_str("    return _color;\n");
    }
    src.push_str("}\n");

    L4Shader {
        source: src,
        uniform_layout,
        uniform_pad_f32,
        element_layout: elements.clone(),
        camera_group,
    }
}
