//! L2 lowering: `deform` as one compute entry point over the elements that
//! reach it.
//!
//! # Two element buffers, and the copy between them is the layer
//!
//! An L1 reads `prev` and writes `next` — one buffer shape, two instances,
//! swapped. An L2 reads the buffer **its upstream node wrote** and writes one
//! of its own, which is a different relationship: the two layouts are not the
//! same struct. The output carries every attribute the input had, plus whatever
//! this procedure adds by declaring `emit`, so an L2 can widen the element for
//! everything downstream of it — `docs/ir-spec.md`, "L2 and L3".
//!
//! Every generated `deform` therefore begins with a **pass-through**: each
//! output slot is filled from the input's slot of the same name, and any slot
//! the input did not have is zeroed. Then the block runs, over the output. That
//! is what makes a modulator a modulator rather than a second generator — it
//! rewrites some of what reaches it and leaves the rest alone, without every
//! procedure having to restate the whole element.
//!
//! # Statelessness is structural here rather than checked
//!
//! `docs/ir-spec.md` makes "an L2 is stateless" the load-bearing decision of the
//! layer: it is what keeps a modulator freely stackable, keeps `closed_form` and
//! priming questions the L1 alone answers, and makes fusion legal for a graph
//! compiler rather than merely plausible.
//!
//! **The lowering makes it true rather than the checker refusing what breaks
//! it.** Reads and writes in a `deform` both address the output buffer, and the
//! output buffer is rebuilt from the input every frame before the block runs —
//! so `position = position + v` moves an element by `v` from wherever the *input*
//! put it this frame, never from where this node left it last frame. There is no
//! previous value to accumulate onto, so accumulation cannot be written. A
//! zeroed slot for a newly emitted attribute is part of that: leaving it stale
//! would be exactly the previous frame's value coming back.
//!
//! # What it does not do
//!
//! **It cannot `kill()`** — refused in the checker, with its own reason. Liveness
//! is settled by the compaction that runs once after L1, and nothing downstream
//! of a deformation reconsiders it, so an L2 removing an element would be
//! removing it from a range that had already been decided.
//!
//! Dead slots are skipped rather than deformed. Whatever the output holds there
//! is never read: a reader takes its alive flags from the same buffer the input
//! did, and the flag is what a renderer's vertex stage tests per instance.

use karakuri_ir::typed::{Checked, TStmt, Target};
use karakuri_ir::{Ambient, Attr, BlockKind, Kind};

use crate::layout::{
    self, binding, group, ElementLayout, UniformLayout, UniformLayoutBuilder, WORKGROUP_SIZE,
};
use crate::lower::{lower_expr, mangle_local, Resolver};
use crate::prelude::{self, Requirements};
use crate::ty::{pad_to_vec4, wgsl_ty};

pub struct L2Shader {
    pub source: String,
    pub uniform_layout: UniformLayout,
    /// Trailing `f32` pad slots after `uniform_layout`'s named fields — wire
    /// padding only, never a value the engine sets.
    pub uniform_pad_f32: u32,
    /// What this node **writes**: everything that reached it, plus its own
    /// `emit`. The layout every node downstream of it compiles against.
    pub element_layout: ElementLayout,
    /// The attribute list behind [`L2Shader::element_layout`], for the next
    /// node in a chain to widen in turn.
    pub emits: Vec<Attr>,
}

/// Generate the compute shader for one L2, against the attributes available
/// where it sits.
///
/// `upstream` is what reaches this node — the L1's `emit`, widened by every L2
/// between. It is passed in rather than derived for the same reason
/// `generate_l4` takes an `ElementLayout`: the input buffer is somebody else's
/// and this shader has to address it with the identical struct.
pub fn generate_l2(checked: &Checked, upstream: &[Attr]) -> L2Shader {
    assert_eq!(checked.kind, Kind::L2, "generate_l2 called on a non-L2 procedure");

    let in_layout = layout::generate_element_layout(upstream);
    // Upstream order first, then whatever this node adds, so a chain's layouts
    // share a prefix and a reader that only wants `position` finds it at the
    // same offset however many modulators ran.
    let mut emits: Vec<Attr> = upstream.to_vec();
    for &attr in &checked.emit {
        if !emits.contains(&attr) {
            emits.push(attr);
        }
    }
    let out_layout = layout::generate_element_layout(&emits);

    let mut b = UniformLayoutBuilder::new();
    b.field("dt", "f32");
    b.field("capacity", "u32");
    b.field("seed_salt", "u32");
    for p in &checked.params {
        b.param_field(p.name.clone(), wgsl_ty(p.ty));
    }
    let (uniform_layout, uniform_pad_f32) = b.finish();

    let mut req = Requirements::default();
    let deform = checked
        .block(BlockKind::Deform)
        .expect("an L2 procedure must have a deform block");
    let body = {
        let mut out = String::new();
        emit_stmts(&deform.stmts, &mut req, 2, &mut out);
        out
    };

    let mut src = String::new();
    layout::write_uniform_struct(&mut src, &uniform_layout, uniform_pad_f32);
    src.push_str(&format!(
        "\n@group({}) @binding({}) var<uniform> u: Uniforms;\n\n",
        group::UNIFORMS,
        binding::UNIFORM,
    ));
    src.push_str(layout::counts::WGSL);
    src.push_str(&format!(
        "\n@group({}) @binding({}) var<storage, read> counts: Counts;\n\n",
        group::UNIFORMS,
        binding::COUNTS,
    ));
    src.push_str(layout::step_args::WGSL);
    src.push_str(&format!(
        "\n@group({}) @binding({}) var<uniform> step_args: StepArgs;\n\n",
        group::STEP,
        binding::UNIFORM,
    ));

    // Two structs, because the two buffers are two shapes. `ElementIn` is
    // byte-identical to what the upstream node wrote — this is the same
    // contract `generate_l4` has with its L1, one node further along.
    layout::write_element_struct_named(&mut src, "ElementIn", &in_layout);
    src.push('\n');
    layout::write_element_struct_named(&mut src, "ElementOut", &out_layout);
    src.push_str(&format!(
        "\n@group({}) @binding({}) var<storage, read> src: array<ElementIn>;\n",
        group::PREV,
        binding::ELEMENT,
    ));
    src.push_str(&format!(
        "@group({}) @binding({}) var<storage, read> src_alive: array<u32>;\n",
        group::PREV,
        binding::ALIVE,
    ));
    src.push_str(&format!(
        "@group({}) @binding({}) var<storage, read_write> dst: array<ElementOut>;\n\n",
        group::NEXT,
        binding::ELEMENT,
    ));
    src.push_str(&prelude::render(&req));
    src.push('\n');
    src.push_str(&deform_entry(&in_layout, &out_layout, &body));

    L2Shader { source: src, uniform_layout, uniform_pad_f32, element_layout: out_layout, emits }
}

/// Reads and writes both address `dst`, which is what makes an L2 stateless by
/// construction — see the module doc.
struct L2Resolver;

impl Resolver for L2Resolver {
    fn read_attr(&self, attr: Attr) -> String {
        format!("dst[i].{}.{}", attr.name(), crate::ty::attr_swizzle(attr.ty()))
    }

    fn read_seed(&self) -> String {
        "dst[i].seed.x".to_string()
    }

    fn read_ambient(&self, amb: Ambient) -> String {
        match amb {
            Ambient::Capacity => "u.capacity".to_string(),
            Ambient::T => "step_args.t".to_string(),
            Ambient::Beats => "step_args.beats".to_string(),
            // **The uniform, unscaled.** An L1 substitutes a birth-fraction
            // corrected `dt` on an element's first update, which exists so that
            // a frame's worth of new elements do not all start at one phase.
            // Nothing here integrates — a `deform` cannot accumulate at all —
            // so there is no first update to correct and nothing for the
            // correction to apply to.
            Ambient::Dt => "u.dt".to_string(),
            Ambient::Seed => unreachable!("read_seed handles this"),
            Ambient::Camera | Ambient::PointCoord | Ambient::Eye | Ambient::Ray => {
                unreachable!("{amb:?} is L4-only and cannot appear in a Checked L2 block")
            }
        }
    }
}

/// Statement lowering, on the same terms `l1` and `l4` have their own: the
/// assignment target is what differs between layers, and unifying three targets
/// was worse than three short direct versions. An L2 assigns to the output
/// element and to locals, and to nothing else — `kill()` is refused in the
/// checker and a stage output has no meaning here.
fn emit_stmts(stmts: &[TStmt], req: &mut Requirements, indent: usize, out: &mut String) {
    let pad = "    ".repeat(indent);
    for stmt in stmts {
        match stmt {
            TStmt::Let { name, value, .. } => {
                let v = lower_expr(value, &L2Resolver, req);
                out.push_str(&format!("{pad}let {} = {v};\n", mangle_local(name)));
            }
            TStmt::Var { name, value, .. } => {
                let v = lower_expr(value, &L2Resolver, req);
                out.push_str(&format!("{pad}var {} = {v};\n", mangle_local(name)));
            }
            TStmt::Assign { target, value, .. } => {
                let v = lower_expr(value, &L2Resolver, req);
                match target {
                    Target::Local(name) => {
                        out.push_str(&format!("{pad}{} = {v};\n", mangle_local(name)))
                    }
                    Target::Attr(attr) => {
                        let wrapped = pad_to_vec4(attr.ty(), "f32", &v);
                        out.push_str(&format!("{pad}dst[i].{} = {wrapped};\n", attr.name()));
                    }
                    Target::Output(o) => unreachable!("L2 never assigns stage output {o:?}"),
                }
            }
            TStmt::If { cond, then, els, .. } => {
                let c = lower_expr(cond, &L2Resolver, req);
                out.push_str(&format!("{pad}if {c} {{\n"));
                emit_stmts(then, req, indent + 1, out);
                if els.is_empty() {
                    out.push_str(&format!("{pad}}}\n"));
                } else {
                    out.push_str(&format!("{pad}}} else {{\n"));
                    emit_stmts(els, req, indent + 1, out);
                    out.push_str(&format!("{pad}}}\n"));
                }
            }
            TStmt::For { var, start, end, body, .. } => {
                let v = mangle_local(var);
                out.push_str(&format!(
                    "{pad}for (var {v}: i32 = {start}; {v} < {end}; {v} = {v} + 1) {{\n"
                ));
                emit_stmts(body, req, indent + 1, out);
                out.push_str(&format!("{pad}}}\n"));
            }
            TStmt::Kill { .. } => {
                unreachable!("`kill()` in a deform is refused by the checker")
            }
        }
    }
}

fn deform_entry(in_layout: &ElementLayout, out_layout: &ElementLayout, body: &str) -> String {
    let mut copy = String::new();
    for slot in &out_layout.slots {
        if in_layout.slots.iter().any(|s| s.name == slot.name) {
            copy.push_str(&format!("    dst[i].{0} = src[i].{0};\n", slot.name));
        } else {
            // **Zeroed, not left as it was.** A slot this node adds has no
            // input to come from, and whatever the buffer holds there is this
            // node's own output from the previous frame — which is exactly the
            // accumulation an L2 is not allowed to have.
            copy.push_str(&format!(
                "    dst[i].{} = {}(0);\n",
                slot.name,
                slot.elem_ty.wgsl_name()
            ));
        }
    }
    format!(
        "@compute @workgroup_size({WORKGROUP_SIZE})\n\
         fn deform(@builtin(global_invocation_id) gid: vec3<u32>) {{\n\
         \x20   let i = gid.x;\n\
         \x20   if (i >= counts.range) {{\n\
         \x20       return;\n\
         \x20   }}\n\
         \x20   // A dead slot is skipped rather than deformed. Its output is\n\
         \x20   // never read: a reader takes its alive flags from the buffer\n\
         \x20   // this node's input came with, and the flag is what a vertex\n\
         \x20   // stage tests per instance.\n\
         \x20   if (src_alive[i] == 0u) {{\n\
         \x20       return;\n\
         \x20   }}\n\
         {copy}\n\
         {body}}}\n"
    )
}
