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
//! What this crate does generate is the half of compaction that *is*
//! per procedure: the index substitution at the point `element` writes. An
//! earlier revision of this doc predicted that substitution would be a
//! one-line change, and it was — `let out = dest[i];` in place of `let out =
//! i;`, plus the skip for an element that is already dead. The scan's shape
//! did not reach this file at all.
//!
//! # Compacted and static procedures
//!
//! A procedure with no `spawn` block and no `kill()` never changes its live
//! set: `capacity` elements are live at frame zero and the same `capacity`
//! elements are live forever. Compaction for it is pure cost — a scan whose
//! answer is always the identity permutation — so [`L1Shader::compacted`]
//! says which shape a procedure is, and the engine skips the scan, the
//! spawn dispatch and the `advance` pass entirely for a static one.
//!
//! The two shapes differ in `element` by three lines. A compacted procedure
//! returns early when `prev_alive[i]` is zero (its slot is being reclaimed
//! this step) and writes to `dest[i]`; a static one writes to `i`, does not
//! read `prev_alive`, and — the point of the distinction — does not bind or
//! read `dest` at all. Everything else, including how any block lowers, is
//! identical between them.
//!
//! # Where the live range lives
//!
//! `element` dispatches indirectly over `counts.range` and bounds-checks
//! against it, and `spawn` writes at `counts.survivors + i`. Both of those
//! numbers are produced on the GPU by the scan and by `advance`; the host
//! never learns them and never needs to. See
//! [`crate::layout::counts`] for the buffer they live in.
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
//! # What is per frame and what is per substep
//!
//! The spec's WGSL lowering section says `param` values "pack into a single
//! uniform buffer along with `t`, `dt`, `capacity`, and the layer's seed
//! salt". Three of those four do. **`t` does not**, and the split is the
//! useful thing to understand about this file.
//!
//! A `param`, a signal binding, and the camera are *input*: external control,
//! genuinely sampled once per frame, held constant across that frame's
//! substeps. `t` is not input, it is the simulation's own clock, and a frame
//! of two steps has to leave the simulation exactly where two frames of one
//! step would — that invariance is the entire reason substepping exists. Hold
//! `t` constant across substeps and both passes run at the same instant, which
//! any procedure reading `t` can see. So `t` is per substep, alongside the
//! spawn count and the seed base, in [`crate::layout::step_args`].
//!
//! Where the live range ends is neither: it is GPU state after compaction,
//! in [`crate::layout::counts`]. Putting it in the uniform buffer (which an
//! earlier revision did) meant a `live_count` field nothing could write
//! truthfully.

use karakuri_ir::typed::{Checked, TStmt, Target};
use karakuri_ir::{Ambient, Attr, BlockKind, Kind};

use crate::layout::{self, binding, group, ElementLayout, UniformLayout, UniformLayoutBuilder, WORKGROUP_SIZE};
use crate::lower::{lower_expr, mangle_local, Resolver};
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
    /// The `Element` struct this procedure's `prev`/`next` buffers are laid
    /// out with. `Set::build` passes this straight to [`crate::generate_l4`]
    /// for the paired L4 procedure — see that function's doc for why L4
    /// cannot derive its own.
    pub element_layout: ElementLayout,
    /// Whether a `spawn` entry point was emitted. `false` for a procedure
    /// with no `spawn` block, in which case the engine must initialize the
    /// seed buffer to each slot's own index before the first frame — see
    /// [Element identity](../../../docs/ir-spec.md#element-identity).
    pub has_spawn: bool,
    /// Whether this procedure's live set can change, and therefore whether
    /// the engine has to run the compaction scan, the spawn dispatch and the
    /// `advance` pass for it. True when it has a `spawn` block or calls
    /// `kill()` anywhere in `element`; see the module doc.
    pub compacted: bool,
}

struct L1Resolver {
    /// The slot `prev` is read from. Never the same expression as
    /// [`L1Resolver::write_idx`] in a compacted `element`: that is the whole
    /// of what compaction changes about a lowered block.
    read_idx: &'static str,
    write_idx: &'static str,
    block: BlockKind,
}

impl Resolver for L1Resolver {
    fn read_attr(&self, attr: Attr) -> String {
        format!("prev[{}].{}.{}", self.read_idx, attr.name(), crate::ty::attr_swizzle(attr.ty()))
    }

    fn read_seed(&self) -> String {
        "seed".to_string()
    }

    fn read_ambient(&self, amb: Ambient) -> String {
        match amb {
            Ambient::Capacity => "u.capacity".to_string(),
            Ambient::T => "step_args.t".to_string(),
            // Per substep alongside `t`, and for the same reason: a frame of
            // two steps has to land on the same two musical instants two
            // frames of one step do, or substepping would move the beat.
            Ambient::Beats => "step_args.beats".to_string(),
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
                    Target::Attr(attr) => {
                        let wrapped = pad_to_vec4(attr.ty(), "f32", &v);
                        out.push_str(&format!("{pad}next[{}].{} = {wrapped};\n", resolver.write_idx, attr.name()));
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
                let v = mangle_local(var);
                out.push_str(&format!("{pad}for (var {v}: i32 = {start}; {v} < {end}; {v} = {v} + 1) {{\n"));
                emit_stmts(body, resolver, req, indent + 1, out);
                out.push_str(&format!("{pad}}}\n"));
            }
            TStmt::Kill { .. } => {
                out.push_str(&format!("{pad}_killed = true;\n"));
            }
        }
    }
}

/// The engine-state bindings that share [`group::UNIFORMS`] with the uniform
/// buffer: the counts buffer both entry points read, and — for a compacted
/// procedure only — the scan's destination indices. A static procedure must
/// not declare `dest`, or naga would require the engine to bind a buffer
/// that has no reason to exist.
fn write_engine_bindings(out: &mut String, compacted: bool) {
    out.push_str(layout::counts::WGSL);
    out.push_str(&format!(
        "\n@group({}) @binding({}) var<storage, read> counts: Counts;\n",
        group::UNIFORMS,
        binding::COUNTS,
    ));
    if compacted {
        out.push_str(&format!(
            "@group({}) @binding({}) var<storage, read> dest: array<u32>;\n",
            group::UNIFORMS,
            binding::DEST,
        ));
    }
}

/// The per-substep binding. Declared unconditionally, unlike `dest`: even a
/// procedure that never spawns reads `t` out of it, because `t` advances per
/// substep rather than per frame — see [`layout::step_args`].
fn write_step_args_binding(out: &mut String) {
    out.push_str(layout::step_args::WGSL);
    out.push_str(&format!(
        "\n@group({}) @binding({}) var<uniform> step_args: StepArgs;\n",
        group::STEP,
        binding::UNIFORM,
    ));
}

fn write_element_bindings(out: &mut String, layout: &ElementLayout) {
    layout::write_element_struct(out, layout);
    out.push('\n');
    out.push_str(&format!(
        "@group({}) @binding({}) var<storage, read> prev: array<Element>;\n",
        group::PREV,
        layout::binding::ELEMENT,
    ));
    out.push_str(&format!(
        "@group({}) @binding({}) var<storage, read> prev_alive: array<u32>;\n",
        group::PREV,
        layout::binding::ALIVE,
    ));
    out.push_str(&format!(
        "@group({}) @binding({}) var<storage, read_write> next: array<Element>;\n",
        group::NEXT,
        layout::binding::ELEMENT,
    ));
    out.push_str(&format!(
        "@group({}) @binding({}) var<storage, read_write> next_alive: array<u32>;\n",
        group::NEXT,
        layout::binding::ALIVE,
    ));
}

/// `spawn` writes into the head of the free range, which begins where the
/// survivors end. `counts.survivors` is the scan's answer for *this* step,
/// written by `finalize` and not yet rolled into `counts.range` — `advance`
/// does that afterwards, because `element` still needs the pre-scan range.
fn spawn_entry(body: &str) -> String {
    format!(
        "@compute @workgroup_size({WORKGROUP_SIZE})
fn spawn(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let i = gid.x;
    if i >= step_args.spawn_count {{ return; }}
    let slot = counts.survivors + i;
    if slot >= u.capacity {{ return; }}
    let seed = step_args.seed_base + i;
    let birth_frac = (f32(i) + 0.5) / max(f32(step_args.spawn_count), 1.0);
{body}    next[slot].seed = vec4<u32>(seed, 0u, 0u, 0u);
    next_alive[slot] = 1u;
    next[slot].birth_frac = vec4<f32>(birth_frac, 0.0, 0.0, 0.0);
}}

"
    )
}

/// The whole of compaction, as it reaches a lowered block: read `prev[i]`,
/// write `next[dest[i]]`. `dest[i] <= i` always, but that is not why this is
/// safe to do in one pass — `prev` and `next` are different buffers, so an
/// invocation's write can never land on any invocation's read.
///
/// A static procedure gets `out = i` and no `prev_alive` read, because its
/// alive flags are all 1 and its scan would be the identity.
fn element_entry(body: &str, compacted: bool) -> String {
    let skip_dead = if compacted {
        // Its slot is being reclaimed by this step's scan; `dest[i]` for a
        // dead element is some survivor's destination, so writing through it
        // would corrupt that survivor.
        "    if prev_alive[i] == 0u { return; }\n"
    } else {
        ""
    };
    let out = if compacted { "dest[i]" } else { "i" };
    format!(
        "@compute @workgroup_size({WORKGROUP_SIZE})
fn element(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let i = gid.x;
    if i >= counts.range {{ return; }}
{skip_dead}    let out = {out};
    let seed = prev[i].seed.x;
    let _dt = u.dt * prev[i].birth_frac.x;
    var _killed = false;
{body}    next[out].seed = vec4<u32>(seed, 0u, 0u, 0u);
    next_alive[out] = select(1u, 0u, _killed);
    next[out].birth_frac = vec4<f32>(1.0, 0.0, 0.0, 0.0);
}}
"
    )
}

/// Whether `stmts` reaches a `kill()` anywhere, including inside an `if` or
/// a `for`. A procedure that can never call it can never lose an element,
/// which together with having no `spawn` block is what makes it static.
fn contains_kill(stmts: &[TStmt]) -> bool {
    stmts.iter().any(|s| match s {
        TStmt::Kill { .. } => true,
        TStmt::If { then, els, .. } => contains_kill(then) || contains_kill(els),
        TStmt::For { body, .. } => contains_kill(body),
        TStmt::Let { .. } | TStmt::Var { .. } | TStmt::Assign { .. } => false,
    })
}

/// Lowers a `Checked` L1 procedure to WGSL. Panics if `checked.kind` is not
/// `Kind::L1` or it has no `element` block — both are preconditions a
/// `Checked` value from a real check pass already guarantees.
pub fn generate_l1(checked: &Checked) -> L1Shader {
    assert_eq!(checked.kind, Kind::L1, "generate_l1 called on a non-L1 procedure");

    let element_layout = layout::generate_element_layout(&checked.emit);

    // No `t`: it is per substep, not per frame, and lives in `StepArgs`.
    let mut b = UniformLayoutBuilder::new();
    b.field("dt", "f32");
    b.field("capacity", "u32");
    b.field("seed_salt", "u32");
    for p in &checked.params {
        b.param_field(p.name.clone(), wgsl_ty(p.ty));
    }
    let (uniform_layout, uniform_pad_f32) = b.finish();

    let mut req = Requirements::default();

    let element_blk = checked
        .block(BlockKind::Element)
        .expect("an L1 procedure must have an element block");
    let has_spawn = checked.block(BlockKind::Spawn).is_some();
    let compacted = has_spawn || contains_kill(&element_blk.stmts);

    let element_body = {
        let resolver = L1Resolver {
            read_idx: "i",
            write_idx: "out",
            block: BlockKind::Element,
        };
        let mut out = String::new();
        emit_stmts(&element_blk.stmts, &resolver, &mut req, 1, &mut out);
        out
    };
    let spawn_body = checked.block(BlockKind::Spawn).map(|blk| {
        let resolver = L1Resolver {
            read_idx: "slot",
            write_idx: "slot",
            block: BlockKind::Spawn,
        };
        let mut out = String::new();
        emit_stmts(&blk.stmts, &resolver, &mut req, 1, &mut out);
        out
    });

    let mut src = String::new();
    layout::write_uniform_struct(&mut src, &uniform_layout, uniform_pad_f32);
    src.push_str(&format!(
        "\n@group({}) @binding({}) var<uniform> u: Uniforms;\n\n",
        group::UNIFORMS,
        binding::UNIFORM,
    ));
    write_engine_bindings(&mut src, compacted);
    src.push('\n');
    write_step_args_binding(&mut src);
    src.push('\n');
    write_element_bindings(&mut src, &element_layout);
    src.push('\n');
    src.push_str(&prelude::render(&req));
    src.push('\n');
    if let Some(body) = &spawn_body {
        src.push_str(&spawn_entry(body));
    }
    src.push_str(&element_entry(&element_body, compacted));

    L1Shader { source: src, uniform_layout, uniform_pad_f32, element_layout, has_spawn, compacted }
}
