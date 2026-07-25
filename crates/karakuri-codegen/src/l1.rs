//! L1 lowering: `spawn` and `element` as separate compute entry points.
//!
//! # What this crate generates, and what it does not
//!
//! The ir-spec's L1 lowering section also describes an order-preserving
//! compaction scan — a prefix sum over the previous frame's alive flags that
//! produces each survivor's destination index, run once per frame ahead of
//! `element`. That scan is **not generated here**. It operates purely on the
//! alive-flag buffer's geometry (a length and a set of 0/1 values) and never
//! touches a procedure's own attributes or statements, so it is generic
//! compute infrastructure — the same shader regardless of which `.kir`
//! produced the buffers it is compacting — not a per-procedure lowering
//! target. Folding it into this crate would mean every `Checked` tree
//! carries an identical multi-pass reduction shader as dead weight.
//!
//! What this crate commits to instead, so the engine can add that scan
//! without `element`'s generated WGSL changing shape: `element` dispatches
//! over `[0, live_count)` and writes each slot `i` it reads to `next_*[i]` —
//! **in place**, not yet to a scanned destination index. Wiring in real
//! compaction later is an index substitution (`i` to a `dest_index[i]` read
//! from a buffer the scan pass fills) at one call site in this file, not a
//! change to how any block lowers. Until then, `capacity`-sized dispatches
//! over a partially-live buffer are exactly as correct as they were before
//! compaction existed, just not compacted — dead slots keep their `alive`
//! flag false and `spawn` still only overwrites `[live_count, capacity)`.
//!
//! # The birth-fraction substitution
//!
//! [Spawn timing](../../../docs/ir-spec.md#spawn-timing) asks for an
//! element's *first* `element` pass to scale `dt` by its birth fraction, and
//! for nothing else to change. Rather than carrying an extra "is this my
//! first update" flag, `birth_frac` does double duty: `spawn` writes the
//! real fraction in `(0, 1]`, `element` reads it, computes `_dt = u.dt *
//! birth_frac`, uses `_dt` everywhere the block reads the `dt` ambient, and
//! then always writes `1.0` back. The next frame reads `1.0`, and `_dt`
//! becomes `u.dt` unscaled — the correction expires itself by construction,
//! with no branch and no second piece of state.
//!
//! # Uniform fields beyond the spec's literal list
//!
//! The spec's WGSL lowering section says `param` values "pack into a single
//! uniform buffer along with `t`, `dt`, `capacity`, and the layer's seed
//! salt." Taken as exhaustive, `spawn` has no way to know where the
//! previous live range ends (`live_count`), how many elements to create this
//! frame (`spawn_count`), or what seed value the first of them gets
//! (`seed_base`) — none of those are expressible as a compile-time constant,
//! an ambient the spec defines, or a param. They are per-frame engine state,
//! exactly like `t` and `dt`, so this generator adds them to the same
//! uniform buffer rather than inventing a second binding for three scalars.
//! This is a lowering decision the spec's L1 section does not settle.

use karakuri_ir::typed::{Checked, TStmt, Target};
use karakuri_ir::{Ambient, Attr, BlockKind, Kind};

use crate::layout::{self, group, AttrSlot, UniformLayout, UniformLayoutBuilder, WORKGROUP_SIZE};
use crate::lower::{lower_expr, Resolver};
use crate::prelude::{self, Requirements};
use crate::ty::{pad_to_vec4, wgsl_ty};

/// Generated L1 WGSL plus the layout the engine needs to drive it.
#[derive(Debug, Clone)]
pub struct L1Shader {
    pub source: String,
    pub uniform_layout: UniformLayout,
    /// Trailing `f32` pad slots after `uniform_layout`'s named fields —
    /// wire padding only, never a value the engine sets. `uniform_layout`'s
    /// own `total_size` already accounts for these bytes.
    pub uniform_pad_f32: u32,
    pub attr_slots: Vec<AttrSlot>,
    /// Whether a `spawn` entry point was emitted. `false` for a procedure
    /// with no `spawn` block, in which case the engine must hold
    /// `live_count == capacity` permanently and initialize the seed buffer
    /// to each slot's own index before the first frame — see
    /// [Element identity](../../../docs/ir-spec.md#element-identity).
    pub has_spawn: bool,
}

struct L1Resolver {
    idx: &'static str,
    block: BlockKind,
}

impl Resolver for L1Resolver {
    fn read_attr(&self, attr: Attr) -> String {
        format!("prev_{}[{}].{}", attr.name(), self.idx, crate::ty::attr_swizzle(attr.ty()))
    }

    fn read_seed(&self) -> String {
        "seed".to_string()
    }

    fn read_ambient(&self, amb: Ambient) -> String {
        match amb {
            Ambient::Capacity => "u.capacity".to_string(),
            Ambient::T => "u.t".to_string(),
            // Element's first pass substitutes `birth_frac * dt` for `dt`;
            // see the module doc. `spawn` has no notion of "first update"
            // to correct for and reads the uniform directly.
            Ambient::Dt => match self.block {
                BlockKind::Element => "_dt".to_string(),
                BlockKind::Spawn => "u.dt".to_string(),
                _ => unreachable!("L1 resolver used on a non-L1 block"),
            },
            Ambient::Seed => unreachable!("read_seed handles this"),
            Ambient::Camera | Ambient::PointCoord => {
                unreachable!("{amb:?} is L4-only and cannot appear in a Checked L1 block")
            }
        }
    }
}

fn emit_stmts(stmts: &[TStmt], resolver: &L1Resolver, req: &mut Requirements, indent: usize, out: &mut String) {
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
                    Target::Attr(attr) => {
                        let wrapped = pad_to_vec4(attr.ty(), "f32", &v);
                        out.push_str(&format!("{pad}next_{}[{}] = {wrapped};\n", attr.name(), resolver.idx));
                    }
                    Target::Output(o) => unreachable!("L1 never assigns stage output {o:?}"),
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
            TStmt::Kill { .. } => {
                out.push_str(&format!("{pad}_killed = true;\n"));
            }
        }
    }
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
            "@group({}) @binding({}) var<storage, read> prev_{}: array<{}>;\n",
            group::PREV,
            s.binding,
            s.name,
            s.elem_ty.wgsl_name(),
        ));
    }
    for s in slots {
        out.push_str(&format!(
            "@group({}) @binding({}) var<storage, read_write> next_{}: array<{}>;\n",
            group::NEXT,
            s.binding,
            s.name,
            s.elem_ty.wgsl_name(),
        ));
    }
}

fn spawn_entry(body: &str) -> String {
    format!(
        "@compute @workgroup_size({WORKGROUP_SIZE})
fn spawn(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let i = gid.x;
    if i >= u.spawn_count {{ return; }}
    let slot = u.live_count + i;
    if slot >= u.capacity {{ return; }}
    let seed = u.seed_base + i;
    let birth_frac = (f32(i) + 0.5) / max(f32(u.spawn_count), 1.0);
{body}    next_seed[slot] = vec4<u32>(seed, 0u, 0u, 0u);
    next_alive[slot] = vec4<u32>(1u, 0u, 0u, 0u);
    next_birth_frac[slot] = vec4<f32>(birth_frac, 0.0, 0.0, 0.0);
}}

"
    )
}

fn element_entry(body: &str) -> String {
    format!(
        "@compute @workgroup_size({WORKGROUP_SIZE})
fn element(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let i = gid.x;
    if i >= u.live_count {{ return; }}
    let seed = prev_seed[i].x;
    let _dt = u.dt * prev_birth_frac[i].x;
    var _killed = false;
{body}    next_seed[i] = vec4<u32>(seed, 0u, 0u, 0u);
    next_alive[i] = vec4<u32>(select(1u, 0u, _killed), 0u, 0u, 0u);
    next_birth_frac[i] = vec4<f32>(1.0, 0.0, 0.0, 0.0);
}}
"
    )
}

/// Lowers a `Checked` L1 procedure to WGSL. Panics if `checked.kind` is not
/// `Kind::L1` or it has no `element` block — both are preconditions a
/// `Checked` value from a real check pass already guarantees.
pub fn generate_l1(checked: &Checked) -> L1Shader {
    assert_eq!(checked.kind, Kind::L1, "generate_l1 called on a non-L1 procedure");

    let attr_slots = layout::l1_attr_slots(&checked.emit);

    let mut b = UniformLayoutBuilder::new();
    b.field("t", "f32");
    b.field("dt", "f32");
    b.field("capacity", "u32");
    b.field("seed_salt", "u32");
    b.field("live_count", "u32");
    b.field("spawn_count", "u32");
    b.field("seed_base", "u32");
    for p in &checked.params {
        b.field(p.name.clone(), wgsl_ty(p.ty));
    }
    let (uniform_layout, uniform_pad_f32) = b.finish();

    let mut req = Requirements::default();

    let element_body = {
        let blk = checked
            .block(BlockKind::Element)
            .expect("an L1 procedure must have an element block");
        let resolver = L1Resolver { idx: "i", block: BlockKind::Element };
        let mut out = String::new();
        emit_stmts(&blk.stmts, &resolver, &mut req, 1, &mut out);
        out
    };
    let spawn_body = checked.block(BlockKind::Spawn).map(|blk| {
        let resolver = L1Resolver { idx: "slot", block: BlockKind::Spawn };
        let mut out = String::new();
        emit_stmts(&blk.stmts, &resolver, &mut req, 1, &mut out);
        out
    });
    let has_spawn = spawn_body.is_some();

    let mut src = String::new();
    write_uniform_struct(&mut src, &uniform_layout, uniform_pad_f32);
    src.push_str("\n@group(0) @binding(0) var<uniform> u: Uniforms;\n\n");
    write_attr_bindings(&mut src, &attr_slots);
    src.push('\n');
    src.push_str(&prelude::render(&req));
    src.push('\n');
    if let Some(body) = &spawn_body {
        src.push_str(&spawn_entry(body));
    }
    src.push_str(&element_entry(&element_body));

    L1Shader { source: src, uniform_layout, uniform_pad_f32, attr_slots, has_spawn }
}
