//! Generates L2 compute shader passes for stateless element deformation and spatial masking.

use karakuri_ir::typed::{Checked, TStmt, Target};
use karakuri_ir::{Ambient, Attr, BlockKind, Kind};

use karakuri_ir::layout::{self as ir_layout, ElementLayout, Synthetic};

use crate::layout::{self, binding, group, UniformLayout, UniformLayoutBuilder, WORKGROUP_SIZE};
use crate::lower::{lower_expr, mangle_local, Resolver};
use crate::prelude::{self, Requirements};
use crate::ty::wgsl_ty;

pub struct L2Shader {
    pub source: String,
    pub uniform_layout: UniformLayout,
    /// Trailing `f32` pad slots after `uniform_layout`'s named fields — wire
    /// padding only, never a value the engine sets.
    pub uniform_pad_f32: u32,
    /// Attributes written by this node (upstream plus own emits).
    pub element_layout: ElementLayout,
    /// The attribute list behind [`L2Shader::element_layout`], for the next
    /// node in a chain to widen in turn.
    pub emits: Vec<Attr>,
    /// Engine-managed synthetic slots carried in output layout.
    pub synthetic: Synthetic,
    /// Whether this node binds a second geometry (`uses <name> : Geometry`).
    ///
    /// Directs the engine to bind an additional geometry storage buffer.
    pub uses: bool,
    /// The declared `amplify` factor, echoed back so the engine sizes the
    /// output buffer from the same number the shader loops to. `None` is the
    /// endomorphism, which shares its input's liveness and counts and allocates
    /// nothing of either.
    pub amplify: Option<u32>,
}

/// Generates the compute shader for one L2 procedure against available upstream attributes.
///
/// `upstream` represents attributes produced by L1 and preceding L2 passes.
/// `far` provides attributes emitted by an external geometry when `uses` is declared.
pub fn generate_l2(
    checked: &Checked,
    upstream: &[Attr],
    synthetic: Synthetic,
    derived: &[Attr],
    far: Option<&[Attr]>,
    fields: crate::Bound<'_>,
) -> L2Shader {
    assert_eq!(
        checked.kind,
        Kind::L2,
        "generate_l2 called on a non-L2 procedure"
    );

    let in_layout = ir_layout::generate_element_layout(upstream, synthetic, derived);
    // Copy slot exists if upstream already had it or if this node amplifies.
    let out_synthetic = Synthetic {
        copy: synthetic.copy || checked.amplify.is_some(),
    };
    // Far geometry element layout for cross-geometry deformation.
    debug_assert_eq!(
        checked.geometry_slot().is_some(),
        far.is_some(),
        "`{}` declares a geometry slot and was handed no geometry for it, or the reverse",
        checked.name
    );
    let far_layout = far.map(|emits| {
        // The far side carries no `copy`: it is a *source*, and only an
        // amplifier below one puts that slot on an element.
        ir_layout::generate_element_layout(emits, Synthetic::NONE, derived)
    });
    // Upstream order first, then whatever this node adds, so a chain's layouts
    // share a prefix and a reader that only wants `position` finds it at the
    // same offset however many modulators ran.
    let mut emits: Vec<Attr> = upstream.to_vec();
    for &attr in &checked.emit {
        if !emits.contains(&attr) {
            emits.push(attr);
        }
    }
    let out_layout = ir_layout::generate_element_layout(&emits, out_synthetic, derived);

    let mut b = UniformLayoutBuilder::new();
    // Clocks and step interval are stored in uniforms since L2 runs once per frame.
    b.field("t", "f32");
    b.field("beats", "f32");
    b.field("dt", "f32");
    b.field("capacity", "u32");
    b.field("seed_salt", "u32");
    // One u32 per declared source slot to identify connected geometries.
    for slot in checked.source_slots() {
        b.source_slot_field(slot);
    }
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

    let resolver = L2Resolver {
        uses: checked.geometry_slot().is_some(),
        has_copy: out_synthetic.copy,
        derived: out_layout.derived.clone(),
    };
    let mut req = Requirements::default();
    let deform = checked
        .block(BlockKind::Deform)
        .expect("an L2 procedure must have a deform block");
    let body = {
        let mut out = String::new();
        emit_stmts(&deform.stmts, &resolver, &mut req, 1, &mut out);
        out
    };
    // Multiplies evaluated mask strength by the declared `weight` parameter if present.
    let weight = checked.params.iter().any(|p| p.name == "weight").then(|| {
        format!(
            "    strength = strength * u.{};\n",
            layout::mangle_param("weight")
        )
    });
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
    if let Some(layout) = &far_layout {
        src.push('\n');
        layout::write_element_struct_named(&mut src, "ElementFar", layout);
        // The far buffer uses a fixed identifier to avoid collisions with user symbols.
        src.push_str(&format!(
            "@group({}) @binding({}) var<storage, read> far: array<ElementFar>;\n",
            group::PREV,
            binding::FAR,
        ));
    }
    src.push_str(&format!(
        "@group({}) @binding({}) var<storage, read_write> dst: array<ElementOut>;\n",
        group::NEXT,
        binding::ELEMENT,
    ));
    // Amplifiers replicate input liveness flags across output copies in a new alive buffer.
    if checked.amplify.is_some() {
        src.push_str(&format!(
            "@group({}) @binding({}) var<storage, read_write> dst_alive: array<u32>;\n",
            group::NEXT,
            binding::ALIVE,
        ));
    }
    src.push('\n');
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
        uses: checked.geometry_slot().is_some(),
        amplify: checked.amplify,
    }
}

/// Reads and writes both address `dst`, which is what makes an L2 stateless by
/// construction — see the module doc.
struct L2Resolver {
    /// Whether this node declares a geometry slot, so that a far read has a
    /// buffer to address.
    uses: bool,
    /// Whether the output layout includes a synthetic `copy` slot.
    has_copy: bool,
    /// Attributes readable here that have no slot, synthesised at the read
    /// site. See `karakuri_ir::Derivation::is_stored`.
    derived: Vec<Attr>,
}

impl Resolver for L2Resolver {
    fn read_attr(&self, attr: Attr) -> String {
        if self.derived.contains(&attr) {
            return match attr.derivation() {
                Some(karakuri_ir::Derivation::SinceBirth) => "(u.t - dst[i].birth_t)".to_string(),
                other => unreachable!("{other:?} is not synthesised at the read site"),
            };
        }
        format!("dst[i].{}", attr.name())
    }

    /// Reads an attribute from the far element at matching slot index `i`.
    fn read_far(&self, attr: Attr) -> String {
        debug_assert!(
            self.uses,
            "a far read reached a resolver for a node that declares no geometry slot"
        );
        format!("far[i].{}", attr.name())
    }

    fn read_seed(&self) -> String {
        "dst[i].seed".to_string()
    }

    fn read_ambient(&self, amb: Ambient) -> String {
        match amb {
            // Copy index is read directly from dst[i].copy.
            Ambient::Copy if self.has_copy => "dst[i].copy".to_string(),
            Ambient::Copy => "0u".to_string(),
            Ambient::Point => {
                unreachable!("`point` is a field's only input and appears in no other block")
            }
            Ambient::Capacity => "u.capacity".to_string(),
            // Geometry source identity is represented by seed_salt.
            Ambient::Source => "u.seed_salt".to_string(),
            Ambient::T => "u.t".to_string(),
            Ambient::Beats => "u.beats".to_string(),
            // Unscaled step delta dt.
            Ambient::Dt => "u.dt".to_string(),
            Ambient::Seed => unreachable!("read_seed handles this"),
            Ambient::Camera | Ambient::PointCoord | Ambient::Eye | Ambient::Ray => {
                unreachable!("{amb:?} is L4-only and cannot appear in a Checked L2 block")
            }
        }
    }
}

/// Lowers IR statements into WGSL, writing to element fields in `dst` or local variables.
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
                        out.push_str(&format!("{pad}dst[i].{} = {v};\n", attr.name()));
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
            TStmt::If {
                cond, then, els, ..
            } => {
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
    // When amplifying, `_e` walks inputs while `i` indexes into output copies.
    let src_i = if amplify.is_some() { "_e" } else { "i" };
    let mut copy = String::new();
    for slot in &out_layout.slots {
        if slot.name == "copy" {
            // Handled separately below when amplifying.
            if amplify.is_some() {
                continue;
            }
        }
        if in_layout.slots.iter().any(|s| s.name == slot.name) {
            copy.push_str(&format!("    dst[i].{0} = src[{src_i}].{0};\n", slot.name));
        } else {
            // Newly added attributes are zero-initialized to prevent cross-frame leakage.
            copy.push_str(&format!(
                "    dst[i].{} = {}(0);\n",
                slot.name,
                slot.elem_ty.wgsl_name()
            ));
        }
    }
    // Composes nested copy index using mixed-radix: copy * factor + c.
    if let Some(factor) = amplify {
        let parent = if in_layout.slots.iter().any(|s| s.name == "copy") {
            format!("src[{src_i}].copy")
        } else {
            "0u".to_string()
        };
        copy.push_str(&format!("    dst[i].copy = {parent} * {factor}u + _c;\n"));
    }
    // Blends written attribute slots with the original input using the calculated gate strength.
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
                     \x20   // Isolates mask declarations in an inner scope.\n\
                     \x20   {{\n\
                     {mask}\
                     \x20   }}\n\
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
             \x20   if (src_alive[i] == 0u) {{\n\
             \x20       return;\n\
             \x20   }}\n\
             {copy}\n\
             {gate_decl}\n\
             {body}\n\
             {gate_apply}}}\n"
        );
    };
    // Loops over output copies per input element, propagating liveness flags before processing.
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
