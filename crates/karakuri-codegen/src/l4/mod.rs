//! Compiles L4 `vertex` and `fragment` blocks into camera-facing quad render pipelines.

use std::collections::HashSet;

use karakuri_ir::typed::{Checked, TBlock, TExpr, TExprKind, TStmt, Target};
use karakuri_ir::{Ambient, Attr, Blend, BlockKind, Kind, Output, Topology};

use karakuri_ir::layout::ElementLayout;

use crate::layout::{self, group, UniformLayout, UniformLayoutBuilder};
use crate::lower::{lower_expr, mangle_local, Resolver};
use crate::prelude::{self, Requirements};
use crate::ty::wgsl_ty;

mod fullscreen;
use fullscreen::generate_fullscreen;

/// Generated L4 WGSL plus the layout the engine needs to drive it.
#[derive(Debug, Clone)]
pub struct L4Shader {
    pub source: String,
    pub uniform_layout: UniformLayout,
    pub uniform_pad_f32: u32,
    /// The `Element` struct this shader declares — always exactly the
    /// `elements` argument [`generate_l4`] was called with, echoed back so
    /// callers that only kept the `L4Shader` still know what buffer it
    /// expects.
    pub element_layout: ElementLayout,
    /// The bind group index this shader reads the camera at, or `None` if it
    /// never reads one.
    pub camera_group: Option<u32>,
}

pub(super) enum L4Block {
    Vertex,
    Fragment,
}

pub(super) struct L4Resolver {
    block: L4Block,
    /// Tracks whether camera bindings are accessed in shader code.
    camera_used: std::cell::Cell<bool>,
}

impl L4Resolver {
    pub(super) fn new(block: L4Block) -> L4Resolver {
        L4Resolver {
            block,
            camera_used: std::cell::Cell::new(false),
        }
    }
}

impl Resolver for L4Resolver {
    fn read_attr(&self, attr: Attr) -> String {
        match self.block {
            // Storage buffers are read as storage, indexed by
            // `@builtin(instance_index)`, not as vertex buffers — the local
            // was bound from `elements[elem].<name>` in the vertex prologue.
            L4Block::Vertex => attr.name().to_string(),
            // Already resolved to a flat varying by the time fragment runs.
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
            // Copy index read in vertex stage and forwarded to fragment as flat varying.
            Ambient::Copy => match self.block {
                L4Block::Vertex => "copy".to_string(),
                L4Block::Fragment => "in.copy".to_string(),
            },
            Ambient::Point => {
                unreachable!("`point` is a field's only input and appears in no other block")
            }
            Ambient::T => "u.t".to_string(),
            // Geometry source identity is represented by seed_salt.
            Ambient::Source => "u.seed_salt".to_string(),
            Ambient::Beats => "u.beats".to_string(),
            // The camera is its own bind group, written on the GPU by
            // `karakuri_engine`'s derivation pass — not a field of `u`, because
            // an L3 that follows an element cannot be evaluated on the host.
            Ambient::Camera => {
                self.camera_used.set(true);
                "cam.view_proj".to_string()
            }
            Ambient::PointCoord => match self.block {
                L4Block::Fragment => "in.point_coord".to_string(),
                L4Block::Vertex => unreachable!("point_coord is fragment-only"),
            },
            // Bound in the fragment prologue: `eye` straight from the camera,
            // `ray` built there from the basis and this fragment's screen
            // position. Fullscreen only, and the check pass refuses them
            // anywhere else.
            Ambient::Eye => {
                self.camera_used.set(true);
                "cam.eye".to_string()
            }
            Ambient::Ray => "ray".to_string(),
            Ambient::Seed => unreachable!("read_seed handles this"),
            Ambient::Capacity | Ambient::Dt => {
                unreachable!("{amb:?} is L1-only and cannot appear in a Checked L4 block")
            }
        }
    }
}

fn output_local(o: Output) -> &'static str {
    match o {
        Output::Clip => "_clip",
        Output::ClipB => "_clip_b",
        Output::PointRate => "_point_rate",
        Output::Color => "_color",
        // The camera's six. Unreachable here: `Output::block()` puts them in a
        // `camera` block and the checker refuses one in an L4, so a `Checked`
        // L4 cannot carry an assignment to any of them.
        other => unreachable!("{other:?} belongs to a camera block, not to an L4 stage"),
    }
}

pub(super) fn emit_stmts(
    stmts: &[TStmt],
    resolver: &L4Resolver,
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
                        unreachable!("L4 never assigns attribute {a:?}, only reads it")
                    }
                }
            }
            TStmt::If {
                cond, then, els, ..
            } => {
                let c = lower_expr(cond, resolver, req);
                out.push_str(&format!("{pad}if {c} {{\n"));
                emit_stmts(then, resolver, req, indent + 1, out);
                if els.is_empty() {
                    out.push_str(&format!("{pad}}}\n"));
                } else {
                    out.push_str(&format!("{pad}}} else {{\n"));
                    emit_stmts(els, resolver, req, indent + 1, out);
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
                emit_stmts(body, resolver, req, indent + 1, out);
                out.push_str(&format!("{pad}}}\n"));
            }
            TStmt::Kill { .. } => unreachable!("kill() is L1 element-block only"),
        }
    }
}

fn scan_expr(e: &TExpr, seed: &mut bool, copy: &mut bool, attrs: &mut HashSet<Attr>) {
    match &e.kind {
        TExprKind::Far(_) => {
            unreachable!("a far read belongs to an L2 with a `uses` slot, and an L4 has none")
        }
        TExprKind::Attr(a) => {
            attrs.insert(*a);
        }
        TExprKind::Ambient(Ambient::Seed) => *seed = true,
        TExprKind::Ambient(Ambient::Copy) => *copy = true,
        // Source slot reads are uniform reads and do not require varying channels.
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
        // The point handed over, and nothing behind it: a field reads no
        // element of its caller's.
        TExprKind::Field { point, .. } => scan_expr(point, seed, copy, attrs),
        // Recurses on optional sampling coordinate if present.
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

/// Which of `seed` and `consumes` the fragment block actually reads —
/// exactly the set that needs to survive as a flat varying.
fn used_in_fragment(block: &TBlock) -> (bool, bool, HashSet<Attr>) {
    let mut seed = false;
    let mut copy = false;
    let mut attrs = HashSet::new();
    scan_stmts(&block.stmts, &mut seed, &mut copy, &mut attrs);
    (seed, copy, attrs)
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

/// Per-element identity metadata passed from vertex to fragment stages.
#[derive(Clone, Copy)]
struct Identity {
    /// The fragment block reads `seed`, so it needs a varying.
    seed: bool,
    /// The fragment block reads `copy`, so it needs one too.
    copy: bool,
    /// The geometry carries a `copy` slot, because something upstream
    /// amplified. Where it does not, the vertex prologue binds `0u`.
    has_copy_slot: bool,
}

/// A derived attribute's value, bound to a local of its own name.
///
/// Only the rules `Derivation::is_stored` says *false* for reach here — the
/// others are ordinary slots by the time anything reads them.
fn derived_binding(attr: Attr) -> String {
    match attr.derivation() {
        Some(karakuri_ir::Derivation::SinceBirth) => {
            format!("    let {} = u.t - elements[elem].birth_t;\n", attr.name())
        }
        other => unreachable!("{:?} is not synthesised at the read site", other),
    }
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
    // View depth varying used by weighted blended rendering.
    if depth {
        out.push_str(&format!("    @location({loc}) view_depth: f32,\n"));
        loc += 1;
    }
    // Coverage factor compensating for sub-pixel size clamping to one pixel.
    out.push_str(&format!(
        "    @location({loc}) @interpolate(flat) coverage: f32,\n"
    ));
    out.push_str("};\n");
}

fn vertex_entry(
    consumes: &[Attr],
    derived: &[Attr],
    id: Identity,
    attrs_used: &[Attr],
    body: &str,
    topology: Topology,
    weighted: bool,
) -> String {
    let mut out = String::new();
    out.push_str("@vertex\n");
    out.push_str("fn vs(@builtin(vertex_index) corner_idx: u32, @builtin(instance_index) elem: u32) -> VsOut {\n");
    out.push_str("    let seed = elements[elem].seed;\n");
    // Default copy index to 0u when geometry has no copy slot.
    out.push_str(if id.has_copy_slot {
        "    let copy = elements[elem].copy;\n"
    } else {
        "    let copy = 0u;\n"
    });
    // Bind derived attributes or read directly from element storage buffer.
    for &a in consumes {
        if derived.contains(&a) {
            out.push_str(&derived_binding(a));
            continue;
        }
        out.push_str(&format!(
            "    let {} = elements[elem].{};\n",
            a.name(),
            a.name()
        ));
    }
    out.push_str("    var _clip: vec4<f32>;\n");
    if topology == Topology::Lines {
        out.push_str("    var _clip_b: vec4<f32>;\n");
    }
    out.push_str("    var _point_rate: f32;\n");
    out.push_str(body);
    out.push_str("    var out: VsOut;\n");
    match topology {
        Topology::Points => {
            out.push_str("    let corner = corner_of(corner_idx) * 2.0 - 1.0;\n");
            // Quad extent in pixels clamped to minimum 1.0 px to prevent disappearing sub-pixel sprites.
            out.push_str("    let _side_px = _point_rate * u.viewport.y;\n");
            out.push_str("    let _drawn_px = max(_side_px, 1.0);\n");
            out.push_str("    let _rate_ndc = _drawn_px / u.viewport;\n");
            out.push_str("    let ndc_offset = corner * _rate_ndc * _clip.w;\n");
            out.push_str("    out.clip = vec4<f32>(_clip.xy + ndc_offset, _clip.zw);\n");
            out.push_str("    out.point_coord = corner_of(corner_idx);\n");
            // Compensate sub-pixel coverage via squared area factor.
            out.push_str("    let _cov = clamp(_side_px, 0.0, 1.0);\n");
            out.push_str("    out.coverage = _cov * _cov;\n");
            // The whole sprite is at one depth, because a billboard is: all six
            // corners take the element's own `w` and the interpolation across
            // them is constant.
            if weighted {
                out.push_str("    out.view_depth = _clip.w;\n");
            }
        }
        Topology::Lines => {
            out.push_str(SEGMENT_EXPANSION);
            // `w` is the endpoint this corner belongs to — `SEGMENT_EXPANSION`
            // selects it rather than blending it — so a stroke running away from
            // the eye is weighted along its length rather than at one depth.
            if weighted {
                out.push_str("    out.view_depth = w;\n");
            }
        }
        // Never reached: a fullscreen procedure has no vertex block to lower,
        // so `generate_l4` takes the other path entirely.
        Topology::Fullscreen => unreachable!("fullscreen has no per-element vertex stage"),
    }
    if id.seed {
        out.push_str("    out.seed = seed;\n");
    }
    if id.copy {
        out.push_str("    out.copy = copy;\n");
    }
    for &a in attrs_used {
        out.push_str(&format!("    out.{} = {};\n", a.name(), a.name()));
    }
    // Dead elements collapse to zero-area quads to skip rasterization.
    let dropped = match topology {
        Topology::Points => "alive[elem] == 0u",
        Topology::Lines => "alive[elem] == 0u || _clip.w <= 0.0 || _clip_b.w <= 0.0",
        Topology::Fullscreen => unreachable!("fullscreen has no per-element vertex stage"),
    };
    out.push_str(&format!("    if {dropped} {{\n"));
    out.push_str("        out.clip = vec4<f32>(0.0, 0.0, 0.0, 1.0);\n");
    out.push_str("    }\n");
    out.push_str("    return out;\n");
    out.push_str("}\n\n");
    out
}

/// Quad expansion for [`Topology::Lines`]: expands line segments into screen-space thick quads.
///
/// Computes screen-space perpendiculars in pixels based on viewport height and `point_rate`.
/// Segments with endpoints behind the eye are dropped.
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

/// Fragment output structure for weighted blended order-independent transparency.
const WEIGHTED_FS_OUT: &str = "struct FsOut {
    @location(0) accum: vec4<f32>,
    @location(1) reveal: f32,
};

";

/// Epilogue for weighted blended transparency fragment shaders.
///
/// Accumulates color and transmittance weights scaled for `f16` HDR precision.
/// Clamps alpha to `[0.0, 1.0]` to guarantee monotonic occlusion.
const WEIGHTED_FS_EPILOGUE: &str = "    let _a = clamp(_color.a, 0.0, 1.0);
    let _w = _a * max(1e-2, pow(1.0 - _depth01, 3.0));
    var _out: FsOut;
    _out.accum = vec4<f32>(_color.rgb * _a * _w, _a * _w);
    _out.reveal = _a;
    return _out;
";

/// Evaluates linear view depth between camera near and far planes in `[0, 1]`.
const WEIGHTED_DEPTH: &str =
    "    let _depth01 = clamp((in.view_depth - cam.depth_range.x) * cam.depth_range.y, 0.0, 1.0);\n";

fn fragment_entry(id: Identity, attrs_used: &[Attr], body: &str, weighted: bool) -> String {
    let mut out = String::new();
    out.push_str("@fragment\n");
    if weighted {
        out.push_str("fn fs(in: VsOut) -> FsOut {\n");
    } else {
        out.push_str("fn fs(in: VsOut) -> @location(0) vec4<f32> {\n");
    }
    out.push_str("    let point_coord = in.point_coord;\n");
    if id.copy {
        out.push_str("    let copy = in.copy;\n");
    }
    if id.seed {
        out.push_str("    let seed = in.seed;\n");
    }
    for &a in attrs_used {
        out.push_str(&format!("    let {} = in.{};\n", a.name(), a.name()));
    }
    out.push_str("    var _color: vec4<f32>;\n");
    out.push_str(body);
    // Scale RGB by sub-pixel coverage factor while leaving alpha intact.
    out.push_str("    _color = vec4<f32>(_color.rgb * in.coverage, _color.a);\n");
    if weighted {
        out.push_str(WEIGHTED_DEPTH);
        out.push_str(WEIGHTED_FS_EPILOGUE);
    } else {
        out.push_str("    return _color;\n");
    }
    out.push_str("}\n");
    out
}

/// Lowers a `Checked` L4 procedure to WGSL against upstream `ElementLayout`.
pub fn generate_l4(
    checked: &Checked,
    elements: &ElementLayout,
    fields: crate::Bound<'_>,
) -> L4Shader {
    assert_eq!(
        checked.kind,
        Kind::L4,
        "generate_l4 called on a non-L4 procedure"
    );
    // Topology is inferred by the check pass from whether vertex assigns clip_b.
    let topology = checked
        .topology
        .expect("a checked L4 procedure always carries an inferred topology");
    // Declared blend mode (e.g. weighted order-independent transparency).
    let weighted = checked.blend == Some(Blend::Weighted);

    let fragment_blk = checked
        .block(BlockKind::Fragment)
        .expect("an L4 procedure must have a fragment block");

    // Fullscreen procedures bypass vertex processing and use screen-quad shaders.
    if topology == Topology::Fullscreen {
        return generate_fullscreen(checked, fragment_blk, elements, weighted, fields);
    }

    let mut b = UniformLayoutBuilder::new();
    b.field("t", "f32");
    b.field("beats", "f32");
    b.field("seed_salt", "u32");
    // Emits one u32 per declared source slot.
    for slot in checked.source_slots() {
        b.source_slot_field(slot);
    }
    // Viewport dimensions for point_rate to clip-space conversion.
    b.field("viewport", "vec2<f32>");
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

    let vertex_blk = checked
        .block(BlockKind::Vertex)
        .expect("a per-element L4 procedure has a vertex block");

    let (seed_used, copy_used, attrs_used_set) = used_in_fragment(fragment_blk);
    let id = Identity {
        seed: seed_used,
        copy: copy_used,
        has_copy_slot: elements.has_slot("copy"),
    };
    let attrs_used: Vec<Attr> = checked
        .consumes
        .iter()
        .copied()
        .filter(|a| attrs_used_set.contains(a))
        .collect();

    let mut req = Requirements::default();

    let vertex = L4Resolver::new(L4Block::Vertex);
    let vertex_body = {
        let mut out = String::new();
        emit_stmts(&vertex_blk.stmts, &vertex, &mut req, 1, &mut out);
        out
    };
    let fragment = L4Resolver::new(L4Block::Fragment);
    let fragment_body = {
        let mut out = String::new();
        emit_stmts(&fragment_blk.stmts, &fragment, &mut req, 1, &mut out);
        out
    };
    // Weighted blend modes implicitly depend on camera depth planes.
    let camera_group = (vertex.camera_used.get() || fragment.camera_used.get() || weighted)
        .then_some(CAMERA_GROUP);

    let mut src = String::new();
    layout::write_uniform_struct(&mut src, &uniform_layout, uniform_pad_f32);
    src.push_str("\n@group(0) @binding(0) var<uniform> u: Uniforms;\n\n");
    write_element_bindings(&mut src, elements);
    src.push('\n');
    if let Some(g) = camera_group {
        write_camera_binding(&mut src, g);
    }
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
    src.push_str(CORNER_OF);
    src.push('\n');
    write_vsout_struct(&mut src, id, &attrs_used, weighted);
    src.push('\n');
    if weighted {
        src.push_str(WEIGHTED_FS_OUT);
    }
    src.push_str(&vertex_entry(
        &checked.consumes,
        &elements.derived,
        id,
        &attrs_used,
        &vertex_body,
        topology,
        weighted,
    ));
    src.push_str(&fragment_entry(id, &attrs_used, &fragment_body, weighted));

    L4Shader {
        source: src,
        uniform_layout,
        uniform_pad_f32,
        element_layout: elements.clone(),
        camera_group,
    }
}

/// Where a per-element shader reads the camera: after [`group::ATTRS`], which it
/// always binds.
const CAMERA_GROUP: u32 = 2;

pub(super) fn write_camera_binding(out: &mut String, at: u32) {
    out.push_str(layout::camera::WGSL);
    out.push_str(&format!(
        "\n@group({at}) @binding(0) var<uniform> cam: Camera;\n\n"
    ));
}
