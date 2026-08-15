//! L4 lowering: `vertex` / `fragment` as a render pipeline.
//!
//! # Quad expansion, not `PointList`
//!
//! WebGPU has no point size — `PrimitiveTopology::PointList` always
//! rasterizes a single pixel — so every element becomes a quad: six
//! vertices per instance (`@builtin(vertex_index)` 0..6, the corner) times
//! one instance per element (`@builtin(instance_index)`). `point_size`
//! scales the quad in clip space so a sprite keeps its pixel size at any
//! depth, and `point_coord` falls out of the corner directly. This is not a
//! decision this crate made; it is `crates/karakuri-engine/src/shaders/points.wgsl`,
//! which this module follows structurally (`corner_of`, the six-corner
//! winding, the `viewport`-based clip-space offset).
//!
//! **Both topologies are that same quad**, which is why `lines` cost the
//! engine nothing: `VERTICES_PER_ELEMENT` is six either way, the pipeline
//! stays a `TriangleList`, and the indirect draw arguments are untouched.
//! Only where the six corners land changes — around a point, or along the
//! segment from `clip` to `clip_b`. See [`SEGMENT_EXPANSION`].
//!
//! One thing worth being suspicious of, per the brief: the spec's L4
//! lowering section motivates quad expansion by saying "the corner in
//! `@builtin(vertex_index)` **and the element in `@builtin(instance_index)`**",
//! which is correct — but elsewhere the spec's ambient table and prose
//! sometimes talk as though `seed`/element identity could be read off
//! `vertex_index` directly. It cannot: with quad expansion, `vertex_index`
//! is the corner (0..6, repeating every instance) and only
//! `instance_index` identifies the element. Every attribute read in this
//! module is indexed by `instance_index`, never `vertex_index`.
//!
//! # What decides which expansion
//!
//! `Checked::topology`, which for an L4 is **inferred by the check pass**
//! from whether the `vertex` block assigns `clip_b`. It used to be `None`
//! here — `topology` was an L1 header field and this generator had nothing to
//! branch on even in principle, which was fine while there was one rendering
//! strategy and a gap the moment there were two.
//!
//! What closed it is *not* a `topology` declaration on the L4 header. A
//! procedure that writes a second endpoint is drawing a segment and there is
//! nothing else it could be doing, so a header field would only be a second
//! place for that fact to be stated and a first place for it to disagree with
//! itself.
//!
//! **The L1's declaration is not consulted here, and nothing consults it.**
//! An L1 declaring `lines` may be paired with a points L4 and the reverse, on
//! purpose: a segment gets both of its ends from attributes the L4 consumes,
//! so a renderer needs nothing from the geometry that Set composition does not
//! already check. `examples/drift_shell.kir` says `topology points` and is
//! paired with `drift_streaks.kir`, which draws segments. See the comment in
//! `Set::build` for why that is allowed rather than overlooked.
//!
//! # Dead elements inside the draw range
//!
//! The draw's instance count is `counts.range`, which is how many slots the
//! element buffer holds — not how many of them are alive. An element killed
//! during the step that just ran keeps its slot until the *next* step's scan
//! reclaims it, so it is inside the draw range for exactly one frame, and
//! drawing it would show a particle that has already died.
//!
//! The vertex stage therefore reads the alive flag and collapses a dead
//! element's quad to a single point, which rasterizes to nothing. Not a
//! `discard` in the fragment stage: that would run the whole vertex stage,
//! rasterize six vertices' worth of fragments, and pay the fragment block's
//! cost per covered pixel only to throw the result away.
//!
//! # Varyings are minimal, not exhaustive
//!
//! `consumes` and `seed` are readable in both blocks, but only the ones
//! `fragment` actually references need to survive interpolation — carrying
//! every consumed attribute through regardless would waste varying slots on
//! values `vertex` only used to compute `clip`. This module scans the
//! fragment block once (see `used_in_fragment`) and gives exactly that set
//! `@interpolate(flat)` varyings, in `consumes` order.

use std::collections::HashSet;

use karakuri_ir::typed::{Checked, TBlock, TExpr, TExprKind, TStmt, Target};
use karakuri_ir::{Ambient, Attr, BlockKind, Kind, Output, Topology};

use crate::layout::{self, group, ElementLayout, UniformLayout, UniformLayoutBuilder};
use crate::lower::{lower_expr, mangle_local, Resolver};
use crate::prelude::{self, Requirements};
use crate::ty::wgsl_ty;

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
}

enum L4Block {
    Vertex,
    Fragment,
}

struct L4Resolver {
    block: L4Block,
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
            Ambient::T => "u.t".to_string(),
            Ambient::Beats => "u.beats".to_string(),
            Ambient::Camera => "u.camera".to_string(),
            Ambient::PointCoord => match self.block {
                L4Block::Fragment => "in.point_coord".to_string(),
                L4Block::Vertex => unreachable!("point_coord is fragment-only"),
            },
            // Bound in the fragment prologue: `eye` straight from the
            // uniform, `ray` built there from the basis and this fragment's
            // screen position. Fullscreen only, and the check pass refuses
            // them anywhere else.
            Ambient::Eye => "u.eye".to_string(),
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
        Output::PointSize => "_point_size",
        Output::Color => "_color",
    }
}

fn emit_stmts(stmts: &[TStmt], resolver: &L4Resolver, req: &mut Requirements, indent: usize, out: &mut String) {
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
                    Target::Local(name) => out.push_str(&format!("{pad}{} = {v};\n", mangle_local(name))),
                    Target::Output(o) => out.push_str(&format!("{pad}{} = {v};\n", output_local(*o))),
                    Target::Attr(a) => unreachable!("L4 never assigns attribute {a:?}, only reads it"),
                }
            }
            TStmt::If { cond, then, els, .. } => {
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
            TStmt::For { var, start, end, body, .. } => {
                let v = mangle_local(var);
                out.push_str(&format!("{pad}for (var {v}: i32 = {start}; {v} < {end}; {v} = {v} + 1) {{\n"));
                emit_stmts(body, resolver, req, indent + 1, out);
                out.push_str(&format!("{pad}}}\n"));
            }
            TStmt::Kill { .. } => unreachable!("kill() is L1 element-block only"),
        }
    }
}

fn scan_expr(e: &TExpr, seed: &mut bool, attrs: &mut HashSet<Attr>) {
    match &e.kind {
        TExprKind::Attr(a) => {
            attrs.insert(*a);
        }
        TExprKind::Ambient(Ambient::Seed) => *seed = true,
        TExprKind::Ambient(_) | TExprKind::Lit(_) | TExprKind::Local(_) | TExprKind::Param(_) => {}
        TExprKind::Unary { value, .. } => scan_expr(value, seed, attrs),
        TExprKind::Binary { lhs, rhs, .. } => {
            scan_expr(lhs, seed, attrs);
            scan_expr(rhs, seed, attrs);
        }
        TExprKind::Builtin { args, .. } | TExprKind::Construct { args } => {
            for a in args {
                scan_expr(a, seed, attrs);
            }
        }
        TExprKind::Swizzle { value, .. } => scan_expr(value, seed, attrs),
    }
}

fn scan_stmts(stmts: &[TStmt], seed: &mut bool, attrs: &mut HashSet<Attr>) {
    for s in stmts {
        match s {
            TStmt::Let { value, .. } | TStmt::Var { value, .. } | TStmt::Assign { value, .. } => {
                scan_expr(value, seed, attrs)
            }
            TStmt::If { cond, then, els, .. } => {
                scan_expr(cond, seed, attrs);
                scan_stmts(then, seed, attrs);
                scan_stmts(els, seed, attrs);
            }
            TStmt::For { body, .. } => scan_stmts(body, seed, attrs),
            TStmt::Kill { .. } => {}
        }
    }
}

/// Which of `seed` and `consumes` the fragment block actually reads —
/// exactly the set that needs to survive as a flat varying.
fn used_in_fragment(block: &TBlock) -> (bool, HashSet<Attr>) {
    let mut seed = false;
    let mut attrs = HashSet::new();
    scan_stmts(&block.stmts, &mut seed, &mut attrs);
    (seed, attrs)
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

fn write_vsout_struct(out: &mut String, seed_used: bool, attrs_used: &[Attr]) {
    out.push_str("struct VsOut {\n");
    out.push_str("    @builtin(position) clip: vec4<f32>,\n");
    out.push_str("    @location(0) point_coord: vec2<f32>,\n");
    let mut loc = 1;
    if seed_used {
        out.push_str(&format!("    @location({loc}) @interpolate(flat) seed: u32,\n"));
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
    out.push_str("};\n");
}

fn vertex_entry(
    consumes: &[Attr],
    seed_used: bool,
    attrs_used: &[Attr],
    body: &str,
    topology: Topology,
) -> String {
    let mut out = String::new();
    out.push_str("@vertex\n");
    out.push_str("fn vs(@builtin(vertex_index) corner_idx: u32, @builtin(instance_index) elem: u32) -> VsOut {\n");
    out.push_str("    let seed = elements[elem].seed.x;\n");
    for &a in consumes {
        out.push_str(&format!(
            "    let {} = elements[elem].{}.{};\n",
            a.name(),
            a.name(),
            crate::ty::attr_swizzle(a.ty())
        ));
    }
    out.push_str("    var _clip: vec4<f32>;\n");
    if topology == Topology::Lines {
        out.push_str("    var _clip_b: vec4<f32>;\n");
    }
    out.push_str("    var _point_size: f32;\n");
    out.push_str(body);
    out.push_str("    var out: VsOut;\n");
    match topology {
        Topology::Points => {
            out.push_str("    let corner = corner_of(corner_idx) * 2.0 - 1.0;\n");
            out.push_str("    let ndc_offset = corner * _point_size / u.viewport * _clip.w;\n");
            out.push_str("    out.clip = vec4<f32>(_clip.xy + ndc_offset, _clip.zw);\n");
            out.push_str("    out.point_coord = corner_of(corner_idx);\n");
        }
        Topology::Lines => out.push_str(SEGMENT_EXPANSION),
        // Never reached: a fullscreen procedure has no vertex block to lower,
        // so `generate_l4` takes the other path entirely.
        Topology::Fullscreen => unreachable!("fullscreen has no per-element vertex stage"),
    }
    if seed_used {
        out.push_str("    out.seed = seed;\n");
    }
    for &a in attrs_used {
        out.push_str(&format!("    out.{} = {};\n", a.name(), a.name()));
    }
    // A dead element still occupies its slot until the next step's scan
    // reclaims it — see the module doc. Every corner collapsing to the same
    // clip-space point makes both triangles zero-area, so the rasterizer
    // drops it without the fragment stage running at all. Written as an
    // override of `out.clip` rather than folded into the expression above so
    // that the live path's arithmetic is textually unchanged.
    let dropped = match topology {
        Topology::Points => "alive[elem] == 0u",
        // Plus both endpoints being in front of the eye — see
        // [`SEGMENT_EXPANSION`] for why a segment that straddles the eye is
        // dropped rather than clipped.
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

/// The whole vertex stage for [`Topology::Fullscreen`], generated rather than
/// lowered — the procedure has no `vertex` block to lower.
///
/// **One triangle, not two.** Three vertices covering the frame beat a quad's
/// six: no diagonal seam where two triangles meet, and the rasterizer walks one
/// primitive. The corners are (-1,-1), (3,-1) and (-1,3) in NDC, which is the
/// standard trick — the triangle is twice the frame and the half outside it is
/// clipped for free.
///
/// `point_coord` comes out 0..1 across the *frame*, which is the same sentence
/// it already means for a sprite and for a stroke: 0..1 across the primitive.
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
/// `u.cam_right` and `u.cam_up` arrive pre-scaled by the field of view and the
/// aspect ratio — see `Orbit::basis` — so this is an interpolation and a
/// normalize rather than a projection. Everything about *which* projection is
/// on the engine's side of the seam, where the camera is.
const FULLSCREEN_RAY: &str = "    let _ndc = vec2<f32>(in.point_coord.x * 2.0 - 1.0, 1.0 - in.point_coord.y * 2.0);
    let ray = normalize(u.cam_fwd + u.cam_right * _ndc.x + u.cam_up * _ndc.y);
";

/// The quad expansion for [`Topology::Lines`]: the same six corners, laid over
/// the segment from `_clip` to `_clip_b` instead of around a point.
///
/// **The work happens in pixels**, because that is the space `point_size` is
/// given in — the direction of the segment and the perpendicular the width is
/// laid along are both properties of the projected picture, not of the world,
/// so both ends are divided through by `w` first. What goes back out is
/// multiplied by `w` again, which is what makes the rasterizer's own divide
/// land on the pixel position computed here.
///
/// **Nothing here interpolates**, and an earlier version of this comment said
/// it did. `corner_of` returns 0.0 or 1.0 in each component, so every `mix`
/// below is *selection*: each of the six vertices belongs to one end of the
/// segment and takes that end's `w` and that end's divided `z`. The values in
/// between are the rasterizer's, produced from the six it is given.
///
/// What keeps the stroke straight on screen is the last line rather than the
/// mixes — `p_px / half_vp * w` cancels the divide the rasterizer is about to
/// perform, so the vertex lands on the pixel computed here whatever `w` is.
/// Deleting that `* w` is what a depth-varying segment fails on.
///
/// **A segment with an endpoint behind the eye is dropped, not clipped.**
/// Doing it properly means intersecting the segment with the near plane and
/// moving the endpoint there, which is real work; the rasterizer would have
/// done it for free had the divide not already happened here, and the divide
/// is what makes a width in pixels expressible at all. The honest failure is a
/// missing stroke rather than one drawn through the camera.
///
/// **A zero-length segment needs no guard, and draws nothing.** Both ends land
/// on the same pixel, every corner offsets from it by the perpendicular of a
/// zero direction, and the quad is zero-area. That is the arithmetic behaving;
/// what it is *not* is the same behaviour a sprite has, and the difference
/// reaches the operator. A sprite at zero velocity is still a sprite; a stroke
/// whose two ends coincide is gone. Any parameter that scales the distance
/// between the ends therefore has a value that blanks the material, and its
/// declared range should not include it — see `examples/drift_streaks.kir`.
///
/// A second way for a stroke to vanish silently, and the only one with no
/// operator in front of it: `length(seg)` overflows `f32` above roughly 1.8e19
/// pixels, making `dir` zero and the quad zero-width. Reaching it takes a
/// vertex essentially at the eye, since `w` is only guarded against being
/// non-positive rather than against being tiny.
const SEGMENT_EXPANSION: &str = "\
    let corner = corner_of(corner_idx);
    let half_vp = u.viewport * 0.5;
    let a_px = _clip.xy / _clip.w * half_vp;
    let b_px = _clip_b.xy / _clip_b.w * half_vp;
    let seg = b_px - a_px;
    let dir = seg / max(length(seg), 1e-6);
    let across = vec2<f32>(-dir.y, dir.x) * (_point_size * 0.5) * (corner.y * 2.0 - 1.0);
    let p_px = mix(a_px, b_px, corner.x) + across;
    let w = mix(_clip.w, _clip_b.w, corner.x);
    let z = mix(_clip.z / _clip.w, _clip_b.z / _clip_b.w, corner.x);
    out.clip = vec4<f32>(p_px / half_vp * w, z * w, w);
    out.point_coord = corner;
";

fn fragment_entry(seed_used: bool, attrs_used: &[Attr], body: &str) -> String {
    let mut out = String::new();
    out.push_str("@fragment\n");
    out.push_str("fn fs(in: VsOut) -> @location(0) vec4<f32> {\n");
    out.push_str("    let point_coord = in.point_coord;\n");
    if seed_used {
        out.push_str("    let seed = in.seed;\n");
    }
    for &a in attrs_used {
        out.push_str(&format!("    let {} = in.{};\n", a.name(), a.name()));
    }
    out.push_str("    var _color: vec4<f32>;\n");
    out.push_str(body);
    out.push_str("    return _color;\n");
    out.push_str("}\n");
    out
}

/// Lowers a `Checked` L4 procedure to WGSL against `elements`, the paired L1
/// procedure's [`ElementLayout`]. Panics if `checked.kind` is not `Kind::L4`
/// or either block is missing — preconditions a real check pass already
/// guarantees.
///
/// `elements` is a parameter rather than something this function derives
/// from `checked.consumes`, because L4 reads the *same physical buffer* L1
/// wrote: its `Element` struct has to be byte-identical to L1's, not merely
/// wide enough to hold what this procedure happens to consume. Deriving a
/// separate slot list from `consumes` (the old behaviour) could silently
/// disagree with L1's `emit` — different attribute order, or a struct sized
/// for fewer fields — and nothing here would catch it; the mismatch would
/// only show up as a shader reading another attribute's bytes. `Set::build`
/// has both checked procedures, so it is what passes the L1 side's layout
/// through. L4 still only *reads* the slots it `consumes`, plus `seed` — it
/// just declares the full struct so its layout matches.
pub fn generate_l4(checked: &Checked, elements: &ElementLayout) -> L4Shader {
    assert_eq!(checked.kind, Kind::L4, "generate_l4 called on a non-L4 procedure");
    // Inferred by the check pass from whether `vertex` assigns `clip_b`. It is
    // **the L4's own answer and the only one that reaches lowering** — the
    // paired L1's declaration is never read, here or anywhere, and the two are
    // allowed to differ. See the module doc.
    let topology = checked
        .topology
        .expect("a checked L4 procedure always carries an inferred topology");

    let fragment_blk = checked
        .block(BlockKind::Fragment)
        .expect("an L4 procedure must have a fragment block");

    // **A fullscreen procedure has no vertex block to lower**, so it takes an
    // entirely separate path: the vertex stage is generated, there are no
    // element bindings to declare, and no varyings to choose because the only
    // one is the screen position. Returning early keeps the per-element path
    // below textually unchanged rather than threading a condition through it,
    // and keeps `viewport` and `camera` out of a uniform that never reads them.
    if topology == Topology::Fullscreen {
        return generate_fullscreen(checked, fragment_blk, elements);
    }

    let mut b = UniformLayoutBuilder::new();
    b.field("t", "f32");
    b.field("beats", "f32");
    b.field("seed_salt", "u32");
    // Not an IR ambient: converting `point_size` (pixels) into a clip-space
    // offset needs the render target's dimensions, which is engine state,
    // not a value any procedure computes. Present in `points.wgsl` today
    // for the same reason.
    b.field("viewport", "vec2<f32>");
    b.field("camera", "mat4x4<f32>");
    for p in &checked.params {
        b.param_field(p.name.clone(), wgsl_ty(p.ty));
    }
    let (uniform_layout, uniform_pad_f32) = b.finish();

    let vertex_blk = checked
        .block(BlockKind::Vertex)
        .expect("a per-element L4 procedure has a vertex block");

    let (seed_used, attrs_used_set) = used_in_fragment(fragment_blk);
    let attrs_used: Vec<Attr> = checked.consumes.iter().copied().filter(|a| attrs_used_set.contains(a)).collect();

    let mut req = Requirements::default();

    let vertex_body = {
        let resolver = L4Resolver { block: L4Block::Vertex };
        let mut out = String::new();
        emit_stmts(&vertex_blk.stmts, &resolver, &mut req, 1, &mut out);
        out
    };
    let fragment_body = {
        let resolver = L4Resolver { block: L4Block::Fragment };
        let mut out = String::new();
        emit_stmts(&fragment_blk.stmts, &resolver, &mut req, 1, &mut out);
        out
    };

    let mut src = String::new();
    layout::write_uniform_struct(&mut src, &uniform_layout, uniform_pad_f32);
    src.push_str("\n@group(0) @binding(0) var<uniform> u: Uniforms;\n\n");
    write_element_bindings(&mut src, elements);
    src.push('\n');
    src.push_str(&prelude::render(&req));
    src.push('\n');
    src.push_str(CORNER_OF);
    src.push('\n');
    write_vsout_struct(&mut src, seed_used, &attrs_used);
    src.push('\n');
    src.push_str(&vertex_entry(
        &checked.consumes,
        seed_used,
        &attrs_used,
        &vertex_body,
        topology,
    ));
    src.push_str(&fragment_entry(seed_used, &attrs_used, &fragment_body));

    L4Shader { source: src, uniform_layout, uniform_pad_f32, element_layout: elements.clone() }
}


/// The whole of a [`Topology::Fullscreen`] shader.
///
/// Split out rather than branched into `generate_l4` because almost nothing is
/// shared: no element buffer is bound, no attribute is read, no varying is
/// chosen, and the vertex stage is [`FULLSCREEN_VS`] rather than anything the
/// procedure wrote. What *is* shared is the uniform — the same `t`, `beats` and
/// params every L4 gets — plus the four fields the ray needs.
fn generate_fullscreen(
    checked: &Checked,
    fragment_blk: &TBlock,
    elements: &ElementLayout,
) -> L4Shader {
    let mut b = UniformLayoutBuilder::new();
    b.field("t", "f32");
    b.field("beats", "f32");
    b.field("seed_salt", "u32");
    // The ray basis. Only here, so a per-element shader's uniform does not grow
    // four vectors it would never read — and no `viewport` or `camera`, which
    // this path has no use for: the projection is already in the basis.
    b.field("eye", "vec3<f32>");
    b.field("cam_fwd", "vec3<f32>");
    b.field("cam_right", "vec3<f32>");
    b.field("cam_up", "vec3<f32>");
    for p in &checked.params {
        b.param_field(p.name.clone(), wgsl_ty(p.ty));
    }
    let (uniform_layout, uniform_pad_f32) = b.finish();

    let mut req = Requirements::default();
    let body = {
        let resolver = L4Resolver { block: L4Block::Fragment };
        let mut out = String::new();
        emit_stmts(&fragment_blk.stmts, &resolver, &mut req, 1, &mut out);
        out
    };

    let mut src = String::new();
    layout::write_uniform_struct(&mut src, &uniform_layout, uniform_pad_f32);
    src.push_str("\n@group(0) @binding(0) var<uniform> u: Uniforms;\n\n");
    src.push_str(&prelude::render(&req));
    src.push('\n');
    src.push_str(FULLSCREEN_VS);
    src.push_str("@fragment\nfn fs(in: VsOut) -> @location(0) vec4<f32> {\n");
    src.push_str("    let point_coord = in.point_coord;\n");
    src.push_str(FULLSCREEN_RAY);
    src.push_str("    var _color: vec4<f32>;\n");
    src.push_str(&body);
    src.push_str("    return _color;\n}\n");

    // Echoed back unchanged, as the per-element path does. Nothing here reads
    // an element — the check pass refuses a fullscreen `consumes` — but the
    // caller's contract is that an `L4Shader` says what buffer it expects, and
    // "the one its L1 wrote, and it reads none of it" is the honest answer.
    L4Shader {
        source: src,
        uniform_layout,
        uniform_pad_f32,
        element_layout: elements.clone(),
    }
}
