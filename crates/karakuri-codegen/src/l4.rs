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
//! # What decides quad expansion
//!
//! `topology` is declared on the L1 procedure's header, not L4's —
//! `Checked::topology` is `None` for an L4 tree, so this generator has no
//! field to branch on even in principle. v0.2 has exactly one L4 rendering
//! strategy, so quad expansion is unconditional here rather than gated on a
//! topology check the L4 side has no way to perform. Should a second L4
//! output style arrive later, `Checked` would need a way to say which one a
//! given L4 procedure wants — that is a gap in today's tree, not a decision
//! this crate is positioned to paper over.
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
use karakuri_ir::{Ambient, Attr, BlockKind, Kind, Output};

use crate::layout::{self, group, AttrSlot, UniformLayout, UniformLayoutBuilder};
use crate::lower::{lower_expr, Resolver};
use crate::prelude::{self, Requirements};
use crate::ty::wgsl_ty;

/// Generated L4 WGSL plus the layout the engine needs to drive it.
#[derive(Debug, Clone)]
pub struct L4Shader {
    pub source: String,
    pub uniform_layout: UniformLayout,
    pub uniform_pad_f32: u32,
    /// `seed` plus `consumes`, in the order `group::ATTRS` binds them.
    pub attr_slots: Vec<AttrSlot>,
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
            // was bound from `attr_<name>[elem]` in the vertex prologue.
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
            Ambient::Camera => "u.camera".to_string(),
            Ambient::PointCoord => match self.block {
                L4Block::Fragment => "in.point_coord".to_string(),
                L4Block::Vertex => unreachable!("point_coord is fragment-only"),
            },
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
                out.push_str(&format!("{pad}let {name} = {v};\n"));
            }
            TStmt::Var { name, value, .. } => {
                let v = lower_expr(value, resolver, req);
                out.push_str(&format!("{pad}var {name} = {v};\n"));
            }
            TStmt::Assign { target, value, .. } => {
                let v = lower_expr(value, resolver, req);
                match target {
                    Target::Local(name) => out.push_str(&format!("{pad}{name} = {v};\n")),
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
                out.push_str(&format!(
                    "{pad}for (var {var}: i32 = {start}; {var} < {end}; {var} = {var} + 1) {{\n"
                ));
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

fn write_uniform_struct(out: &mut String, layout: &UniformLayout, pad_f32: u32) {
    out.push_str("struct Uniforms {\n");
    for f in &layout.fields {
        out.push_str(&format!("    {}: {},\n", f.name, f.wgsl_ty));
    }
    match pad_f32 {
        0 => {}
        1 => out.push_str("    _pad: f32,\n"),
        n => out.push_str(&format!("    _pad: array<f32, {n}>,\n")),
    }
    out.push_str("};\n");
}

fn write_attr_bindings(out: &mut String, slots: &[AttrSlot]) {
    for s in slots {
        out.push_str(&format!(
            "@group({}) @binding({}) var<storage, read> attr_{}: array<{}>;\n",
            group::ATTRS,
            s.binding,
            s.name,
            s.elem_ty.wgsl_name(),
        ));
    }
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

fn vertex_entry(consumes: &[Attr], seed_used: bool, attrs_used: &[Attr], body: &str) -> String {
    let mut out = String::new();
    out.push_str("@vertex\n");
    out.push_str("fn vs(@builtin(vertex_index) corner_idx: u32, @builtin(instance_index) elem: u32) -> VsOut {\n");
    out.push_str("    let seed = attr_seed[elem].x;\n");
    for &a in consumes {
        out.push_str(&format!(
            "    let {} = attr_{}[elem].{};\n",
            a.name(),
            a.name(),
            crate::ty::attr_swizzle(a.ty())
        ));
    }
    out.push_str("    var _clip: vec4<f32>;\n");
    out.push_str("    var _point_size: f32;\n");
    out.push_str(body);
    out.push_str("    var out: VsOut;\n");
    out.push_str("    let corner = corner_of(corner_idx) * 2.0 - 1.0;\n");
    out.push_str("    let ndc_offset = corner * _point_size / u.viewport * _clip.w;\n");
    out.push_str("    out.clip = vec4<f32>(_clip.xy + ndc_offset, _clip.zw);\n");
    out.push_str("    out.point_coord = corner_of(corner_idx);\n");
    if seed_used {
        out.push_str("    out.seed = seed;\n");
    }
    for &a in attrs_used {
        out.push_str(&format!("    out.{} = {};\n", a.name(), a.name()));
    }
    out.push_str("    return out;\n");
    out.push_str("}\n\n");
    out
}

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

/// Lowers a `Checked` L4 procedure to WGSL. Panics if `checked.kind` is not
/// `Kind::L4` or either block is missing — preconditions a real check pass
/// already guarantees.
pub fn generate_l4(checked: &Checked) -> L4Shader {
    assert_eq!(checked.kind, Kind::L4, "generate_l4 called on a non-L4 procedure");

    let attr_slots = layout::l4_attr_slots(&checked.consumes);

    let mut b = UniformLayoutBuilder::new();
    b.field("t", "f32");
    b.field("seed_salt", "u32");
    // Not an IR ambient: converting `point_size` (pixels) into a clip-space
    // offset needs the render target's dimensions, which is engine state,
    // not a value any procedure computes. Present in `points.wgsl` today
    // for the same reason.
    b.field("viewport", "vec2<f32>");
    b.field("camera", "mat4x4<f32>");
    for p in &checked.params {
        b.field(p.name.clone(), wgsl_ty(p.ty));
    }
    let (uniform_layout, uniform_pad_f32) = b.finish();

    let vertex_blk = checked.block(BlockKind::Vertex).expect("an L4 procedure must have a vertex block");
    let fragment_blk = checked.block(BlockKind::Fragment).expect("an L4 procedure must have a fragment block");

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
    write_uniform_struct(&mut src, &uniform_layout, uniform_pad_f32);
    src.push_str("\n@group(0) @binding(0) var<uniform> u: Uniforms;\n\n");
    write_attr_bindings(&mut src, &attr_slots);
    src.push('\n');
    src.push_str(&prelude::render(&req));
    src.push('\n');
    src.push_str(CORNER_OF);
    src.push('\n');
    write_vsout_struct(&mut src, seed_used, &attrs_used);
    src.push('\n');
    src.push_str(&vertex_entry(&checked.consumes, seed_used, &attrs_used, &vertex_body));
    src.push_str(&fragment_entry(seed_used, &attrs_used, &fragment_body));

    L4Shader { source: src, uniform_layout, uniform_pad_f32, attr_slots }
}
