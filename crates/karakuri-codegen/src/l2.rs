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
//! it, and the pass-through above is the whole of the mechanism.** Because every
//! output slot is overwritten from the input before the block runs, there is no
//! previous value left to accumulate onto: `position = position + v` moves an
//! element by `v` from wherever the *input* put it this frame, never from where
//! this node left it last frame. Zeroing a slot the input did not have is the
//! same property for a newly emitted attribute — leaving it stale would be
//! exactly the previous frame's value coming back.
//!
//! Reads addressing `dst` rather than `src` is **not** what makes this true, and
//! it would be easy to mistake for it. After the copy the two hold the same
//! bytes, so for an attribute that came from upstream either spelling reads the
//! same value. `dst` is used because it is the only one that works for an
//! attribute this node *added*, which has no field in `ElementIn` at all.
//!
//! # A mask is where the deformation applies, and `weight` is how much
//!
//! Both are optional and both end at the same number. An L2 that declares
//! neither is what every L2 was before they existed: applied everywhere, in
//! full. What the two add is a blend at the very end of the entry point —
//! `dst[i] <- mix(input, deformed, weight * strength)` — so a modulator becomes
//! *partial* rather than becoming a different modulator.
//!
//! **They are separate because their audiences are.** `weight` is a declared
//! `param`, so it goes on a fader, takes a signal binding, moves under a
//! transition and is saved in a Set file; `strength` is an expression over the
//! attributes reaching this node, and decides *where*. Folding the first into
//! the second would put the operator's control inside a block and take every one
//! of those surfaces away from it.
//!
//! **The mask runs before the body and reads the input**, which costs nothing to
//! arrange: after the pass-through, `dst` holds exactly what `src` does, so a
//! mask reading `dst[i].position` is reading what reached this node. It has to
//! be that way round — a mask evaluated on the *deformed* element would be
//! deciding where to apply a deformation from a position that deformation had
//! already moved.
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
    self, binding, group, ElementLayout, Synthetic, UniformLayout, UniformLayoutBuilder,
    WORKGROUP_SIZE,
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
    /// The engine-written slots this node's **output** carries: what reached it,
    /// plus `copy` if this node is the one that amplified. Travels with
    /// [`L2Shader::emits`] for the same reason — the next node in the chain
    /// compiles against this node's buffer and has to name the same fields.
    pub synthetic: Synthetic,
    /// The declared `amplify` factor, echoed back so the engine sizes the
    /// output buffer from the same number the shader loops to. `None` is the
    /// endomorphism, which shares its input's liveness and counts and allocates
    /// nothing of either.
    pub amplify: Option<u32>,
}

/// Generate the compute shader for one L2, against the attributes available
/// where it sits.
///
/// `upstream` is what reaches this node — the L1's `emit`, widened by every L2
/// between. It is passed in rather than derived for the same reason
/// `generate_l4` takes an `ElementLayout`: the input buffer is somebody else's
/// and this shader has to address it with the identical struct.
pub fn generate_l2(
    checked: &Checked,
    upstream: &[Attr],
    synthetic: Synthetic,
    derived: &[Attr],
) -> L2Shader {
    assert_eq!(checked.kind, Kind::L2, "generate_l2 called on a non-L2 procedure");

    let in_layout = layout::generate_element_layout(upstream, synthetic, derived);
    // **An amplifier is where `copy` starts existing**, and once it exists it
    // is carried by every node below — so this is an `||`, not an assignment.
    let out_synthetic = Synthetic { copy: synthetic.copy || checked.amplify.is_some() };
    // Upstream order first, then whatever this node adds, so a chain's layouts
    // share a prefix and a reader that only wants `position` finds it at the
    // same offset however many modulators ran.
    let mut emits: Vec<Attr> = upstream.to_vec();
    for &attr in &checked.emit {
        if !emits.contains(&attr) {
            emits.push(attr);
        }
    }
    let out_layout = layout::generate_element_layout(&emits, out_synthetic, derived);

    let mut b = UniformLayoutBuilder::new();
    // **`t` and `beats` are here rather than in `StepArgs`.** An L1 is
    // substepped and each substep lands on its own instant, so its clock has to
    // be per substep. An L2 runs once, after the simulation has reached the
    // frame's last instant — there is one `t` for it, and reading a per-substep
    // buffer would mean choosing which substep a node that ran after all of
    // them belongs to.
    b.field("t", "f32");
    b.field("beats", "f32");
    b.field("dt", "f32");
    b.field("capacity", "u32");
    b.field("seed_salt", "u32");
    for p in &checked.params {
        b.param_field(p.name.clone(), wgsl_ty(p.ty));
    }
    let (uniform_layout, uniform_pad_f32) = b.finish();

    let resolver = L2Resolver { has_copy: out_synthetic.copy, derived: out_layout.derived.clone() };
    let mut req = Requirements::default();
    let deform = checked
        .block(BlockKind::Deform)
        .expect("an L2 procedure must have a deform block");
    let body = {
        let mut out = String::new();
        emit_stmts(&deform.stmts, &resolver, &mut req, 1, &mut out);
        out
    };
    // **`weight` is a declared param the lowering gives a meaning to**, on the
    // same terms `spawn_rate` is one the engine reads: the checker refuses it as
    // anything but a `float`, and multiplying it in here is what keeps it an
    // ordinary param everywhere else — publishable, bindable, saved in a Set
    // file — rather than a second mechanism beside the mask.
    let weight = checked
        .params
        .iter()
        .any(|p| p.name == "weight")
        .then(|| format!("    strength = strength * u.{};\n", layout::mangle_param("weight")));
    let mask = checked.block(BlockKind::Mask).map(|block| {
        let mut out = String::new();
        emit_stmts(&block.stmts, &resolver, &mut req, 1, &mut out);
        out
    });
    // A gate exists if either half does. Neither is what every L2 was before
    // they arrived, and it lowers to the identical shader.
    let gate = match (&mask, &weight) {
        (None, None) => None,
        _ => Some(format!(
            "{}{}",
            mask.as_deref().unwrap_or_default(),
            weight.as_deref().unwrap_or_default()
        )),
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
        "@group({}) @binding({}) var<storage, read_write> dst: array<ElementOut>;\n",
        group::NEXT,
        binding::ELEMENT,
    ));
    // **Only an amplifier writes liveness, and it writes rather than decides
    // it.** A node that keeps the element count shares the very alive buffer
    // its input came with, so there is nothing for it to bind. An amplifier
    // cannot share one — its buffer has `factor` times the entries — so it
    // needs its own, and what it puts there is each parent's flag repeated.
    // That is not an L2 deciding liveness, which is refused: it is the same
    // decision, re-indexed onto a longer buffer.
    if checked.amplify.is_some() {
        src.push_str(&format!(
            "@group({}) @binding({}) var<storage, read_write> dst_alive: array<u32>;\n",
            group::NEXT,
            binding::ALIVE,
        ));
    }
    src.push('\n');
    src.push_str(&prelude::render(&req));
    src.push('\n');
    src.push_str(&deform_entry(
        &in_layout,
        &out_layout,
        &body,
        gate.as_deref(),
        checked.amplify,
    ));

    L2Shader {
        source: src,
        uniform_layout,
        uniform_pad_f32,
        element_layout: out_layout,
        emits,
        synthetic: out_synthetic,
        amplify: checked.amplify,
    }
}

/// Reads and writes both address `dst`, which is what makes an L2 stateless by
/// construction — see the module doc.
struct L2Resolver {
    /// Whether the **output** layout has a `copy` slot: this node amplifies, or
    /// something above it did. Where it does not, `copy` is `0u` — the answer a
    /// chain that never amplified gives at every position in it.
    has_copy: bool,
    /// Attributes readable here that have no slot, synthesised at the read
    /// site. See `karakuri_ir::Derivation::is_stored`.
    derived: Vec<Attr>,
}

impl Resolver for L2Resolver {
    fn read_attr(&self, attr: Attr) -> String {
        if self.derived.contains(&attr) {
            return match attr.derivation() {
                Some(karakuri_ir::Derivation::SinceBirth) => "(u.t - dst[i].birth_t.x)".to_string(),
                other => unreachable!("{other:?} is not synthesised at the read site"),
            };
        }
        format!("dst[i].{}.{}", attr.name(), crate::ty::attr_swizzle(attr.ty()))
    }

    fn read_seed(&self) -> String {
        "dst[i].seed.x".to_string()
    }

    fn read_ambient(&self, amb: Ambient) -> String {
        match amb {
            // **Read from the element, not from a loop variable.** In an
            // amplifying node the two agree only for the copy this invocation
            // is on and only after the write above; below one they do not agree
            // at all, since the value there is a composed index and there is no
            // loop. One spelling for both is the element's own slot.
            Ambient::Copy if self.has_copy => "dst[i].copy.x".to_string(),
            Ambient::Copy => "0u".to_string(),
            Ambient::Capacity => "u.capacity".to_string(),
            Ambient::T => "u.t".to_string(),
            Ambient::Beats => "u.beats".to_string(),
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
fn emit_stmts(
    stmts: &[TStmt],
    r: &L2Resolver,
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
                    Target::Attr(attr) => {
                        let wrapped = pad_to_vec4(attr.ty(), "f32", &v);
                        out.push_str(&format!("{pad}dst[i].{} = {wrapped};\n", attr.name()));
                    }
                    // `strength` is the only one an L2 has, and it belongs to
                    // the `mask` block — the checker refuses it in a `deform`.
                    Target::Output(karakuri_ir::Output::Strength) => {
                        out.push_str(&format!("{pad}strength = {v};\n"));
                    }
                    Target::Output(o) => {
                        unreachable!("L2 never assigns stage output {o:?}")
                    }
                }
            }
            TStmt::If { cond, then, els, .. } => {
                let c = lower_expr(cond, r, req);
                out.push_str(&format!("{pad}if {c} {{\n"));
                emit_stmts(then, r, req, indent + 1, out);
                if els.is_empty() {
                    out.push_str(&format!("{pad}}}\n"));
                } else {
                    out.push_str(&format!("{pad}}} else {{\n"));
                    emit_stmts(els, r, req, indent + 1, out);
                    out.push_str(&format!("{pad}}}\n"));
                }
            }
            TStmt::For { var, start, end, body, .. } => {
                let v = mangle_local(var);
                out.push_str(&format!(
                    "{pad}for (var {v}: i32 = {start}; {v} < {end}; {v} = {v} + 1) {{\n"
                ));
                emit_stmts(body, r, req, indent + 1, out);
                out.push_str(&format!("{pad}}}\n"));
            }
            TStmt::Kill { .. } => {
                unreachable!("`kill()` in a deform is refused by the checker")
            }
        }
    }
}

fn deform_entry(
    in_layout: &ElementLayout,
    out_layout: &ElementLayout,
    body: &str,
    gate: Option<&str>,
    amplify: Option<u32>,
) -> String {
    // **`src` is indexed by the input element and `dst` by the output one, and
    // an amplifier is where those stop being the same number.** Without one they
    // are both `i` and the generated text is exactly what it was before
    // amplification existed; with one, `_e` walks the input and `i` is
    // `_e * factor + _c`, so every read site the lowering produces — all of
    // which address `dst[i]` — needs no knowledge of any of this.
    let src_i = if amplify.is_some() { "_e" } else { "i" };
    let mut copy = String::new();
    for slot in &out_layout.slots {
        if slot.name == "copy" {
            // Written below rather than copied: this node either *is* the
            // amplifier, in which case the value is being made here, or it is
            // downstream of one, in which case the pass-through belongs to the
            // `in_layout.slots` branch and this arm is not reached.
            if amplify.is_some() {
                continue;
            }
        }
        if in_layout.slots.iter().any(|s| s.name == slot.name) {
            copy.push_str(&format!("    dst[i].{0} = src[{src_i}].{0};\n", slot.name));
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
    // **Stacked amplifiers compose the index rather than overwrite it.** A node
    // of factor `n` under a parent that already carried a `copy` turns it into
    // `copy * n + c`, which is the mixed-radix numbering of the whole chain: it
    // stays unique, and the parent's own index is still recoverable by dividing.
    // Overwriting would make two elements of one parent indistinguishable the
    // moment a second amplifier ran, which is the whole of what `copy` is for.
    if let Some(factor) = amplify {
        let parent = if in_layout.slots.iter().any(|s| s.name == "copy") {
            format!("src[{src_i}].copy.x")
        } else {
            "0u".to_string()
        };
        copy.push_str(&format!(
            "    dst[i].copy = vec4<u32>({parent} * {factor}u + _c, 0u, 0u, 0u);\n"
        ));
    }
    // **The gate is computed before the body and applied after it**, over a copy
    // of what reached this node. Two consequences, and both are the point: the
    // mask reads the *input* — `dst` holds it, the pass-through having just run
    // — and every slot the body wrote is blended back toward that input rather
    // than being written or not written.
    //
    // Only the slots carrying an attribute are blended. `seed` is a `u32` and
    // has no meaningful midpoint; the birth fraction is the engine's and no
    // `deform` can write it; `copy` is an index and mixing two of them names a
    // third element. All were copied and none can have moved, so blending them
    // would be a no-op spelled as arithmetic.
    let (gate_decl, gate_apply) = match gate {
        None => (String::new(), String::new()),
        Some(mask) => {
            let mut blend = String::new();
            for slot in out_layout.slots.iter().filter(|s| s.attr.is_some()) {
                blend.push_str(&format!(
                    "    dst[i].{0} = mix(_input.{0}, dst[i].{0}, _gate);\n",
                    slot.name
                ));
            }
            (
                format!(
                    "    let _input = dst[i];\n\
                     \x20   var strength = 1.0;\n\
                     \x20   // **The mask's locals get a scope of their own.** Two\n\
                     \x20   // blocks are spliced into one WGSL function here, and\n\
                     \x20   // a local is mangled by its name alone — so a `let d`\n\
                     \x20   // in both would be a redefinition, from a `.kir` the\n\
                     \x20   // checker accepted. It checks each block in its own\n\
                     \x20   // scope, and this is that scope made real. `strength`\n\
                     \x20   // is declared outside it and assigned from within,\n\
                     \x20   // which is what a scope is for.\n\
                     \x20   {{\n\
                     {mask}\
                     \x20   }}\n\
                     \x20   // Clamped, because `mix` extrapolates: a strength\n\
                     \x20   // of 2 would apply the deformation twice over, and\n\
                     \x20   // one of -1 would apply its inverse.\n\
                     \x20   let _gate = clamp(strength, 0.0, 1.0);\n"
                ),
                blend,
            )
        }
    };
    let Some(factor) = amplify else {
        return format!(
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
             {gate_decl}\n\
             {body}\n\
             {gate_apply}}}\n"
        );
    };
    // **The invocation still walks the *input*, and the copies are a loop.**
    // Dispatching `factor` times as many invocations would need a second
    // indirect argument computed before this pass could run; looping needs
    // nothing, and `factor` is a compile-time constant so the loop bound is one
    // too. What it costs is that a node of a large factor is `factor` times the
    // work in one invocation rather than spread over more of them — which
    // matters only when the element count is already too small to fill the
    // device, and a Set with too few elements to fill a GPU is not the case this
    // engine is sized for.
    //
    // **Liveness is written before the early return**, and that ordering is the
    // whole of why dead parents do not leave stale copies behind: the flags for
    // every copy of a dead element have to say dead, and a `return` above the
    // write would leave whatever the buffer held from the frame before.
    format!(
        "@compute @workgroup_size({WORKGROUP_SIZE})\n\
         fn deform(@builtin(global_invocation_id) gid: vec3<u32>) {{\n\
         \x20   let _e = gid.x;\n\
         \x20   if (_e >= counts.range) {{\n\
         \x20       return;\n\
         \x20   }}\n\
         \x20   let _live = src_alive[_e];\n\
         \x20   for (var _c: u32 = 0u; _c < {factor}u; _c = _c + 1u) {{\n\
         \x20       dst_alive[_e * {factor}u + _c] = _live;\n\
         \x20   }}\n\
         \x20   if (_live == 0u) {{\n\
         \x20       return;\n\
         \x20   }}\n\
         \x20   for (var _c: u32 = 0u; _c < {factor}u; _c = _c + 1u) {{\n\
         \x20   let i = _e * {factor}u + _c;\n\
         {copy}\n\
         {gate_decl}\n\
         {body}\n\
         {gate_apply}\
         \x20   }}\n\
         }}\n"
    )
}
