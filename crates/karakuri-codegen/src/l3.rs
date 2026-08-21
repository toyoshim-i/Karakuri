//! L3 lowering: `camera` as one compute entry point that writes the camera
//! state.
//!
//! # One invocation, and that is the whole shape of the layer
//!
//! Every other block in this language runs over a quantity — elements, spawns,
//! fragments. A `camera` block runs *once*, and produces six numbers. That is
//! why the dispatch below is `@workgroup_size(1)` with no index, no bounds
//! check and no alive flag: there is nothing to be at index `i` of.
//!
//! It is a compute pass rather than host arithmetic for one reason, and it is
//! not this one. A camera on the clock alone could be evaluated on the host and
//! written with a `queue.write_buffer` — that is exactly what the built-in
//! `Orbit` does. But `docs/ir-spec.md` settles that an L3 may **read geometry**,
//! and an element's position lives in a buffer this architecture never reads
//! back. Lowering every L3 to a pass means the camera that follows an element
//! joins as a second thing this shader can do, rather than as a second place a
//! camera can be computed.
//!
//! # Four of the six are defaulted, not required
//!
//! The entry point opens by writing `up`, `fov_y`, `near` and `far`, then runs
//! the block over them. So an author overrides what they mean to change and
//! restates nothing — the same shape as an L2's pass-through, and the reason
//! `required_keys` asks only for `eye` and `target`.
//!
//! **The defaults are `Orbit::default`'s**, deliberately: replacing the built-in
//! camera with the simplest L3 anyone would write should not change the field of
//! view underneath the picture.
//!
//! # What it cannot do yet
//!
//! **Hold state.** `docs/ir-spec.md` allows an L3 to, and gives the reason —
//! *a camera's craft is mostly smoothing, and smoothing is lag, and lag is
//! state*. Nothing here has a previous frame's value to read: the state buffer
//! is written and never read by the shader that writes it. What that costs is
//! damping, which is the next thing this layer will want.

use karakuri_ir::typed::{Checked, TStmt, Target};
use karakuri_ir::{Ambient, BlockKind, Kind, Output};

use crate::layout::{self, binding, group, UniformLayout, UniformLayoutBuilder};
use crate::lower::{lower_expr, mangle_local, Resolver};
use crate::prelude::{self, Requirements};
use crate::ty::wgsl_ty;

#[derive(Debug, Clone)]
pub struct L3Shader {
    pub source: String,
    pub uniform_layout: UniformLayout,
    /// Trailing `f32` pad slots after [`L3Shader::uniform_layout`]'s named
    /// fields — wire padding only, never a value the engine sets.
    pub uniform_pad_f32: u32,
}

/// The entry point an engine dispatches. Not `camera`, which the generated
/// module also uses as a binding name.
pub const ENTRY: &str = "produce";

/// Generate the compute shader for one L3.
pub fn generate_l3(checked: &Checked, fields: crate::Bound<'_>) -> L3Shader {
    assert_eq!(
        checked.kind,
        Kind::L3,
        "generate_l3 called on a non-L3 procedure"
    );

    let mut b = UniformLayoutBuilder::new();
    b.field("t", "f32");
    b.field("beats", "f32");
    // **Present although nothing can use it yet.** An L3 is allowed to hold
    // state and a damped follow is written against a step; the field costs four
    // bytes in a buffer written once a frame, and leaving it out would make
    // adding state a change to the wire format rather than to the body.
    b.field("dt", "f32");
    // **Because the prelude's hashes read it**, and a camera that cuts on the
    // beat is the first thing anyone writes here — `hash1(floor(beats))` is how
    // a cut is chosen without state. Salting it means two Sets running one
    // camera procedure cut to different places, which is the same argument the
    // salt makes for geometry: an instance of a procedure is not the procedure.
    b.field("seed_salt", "u32");
    for p in &checked.params {
        b.param_field(p.name.clone(), wgsl_ty(p.ty));
    }
    // **A spliced field's params live here**, under a prefix of their own so
    // that this procedure and the field it evaluates may both declare
    // `exposure` — see `layout::mangle_field_param`.
    //
    // **One set per slot, and only for the slots the procedure evaluates.** The
    // field used to be spliced into every module in the Set, so a renderer that
    // never mentions one still carried its params and still failed to compile
    // if the field's body did — a `.kir` taking down shaders that have nothing
    // to do with it. The slot is in the name because two fields in one caller
    // are two independent sets of values.
    let splices = crate::splices(checked, fields);
    for f in &splices {
        for (name, ty) in &f.params {
            b.field_param_field(&f.slot, name, ty);
        }
    }
    let (uniform_layout, uniform_pad_f32) = b.finish();

    let mut req = Requirements::default();
    let block = checked
        .block(BlockKind::Camera)
        .expect("an L3 procedure must have a camera block");
    let body = {
        let mut out = String::new();
        emit_stmts(&block.stmts, &mut req, 1, &mut out);
        out
    };

    let mut src = String::new();
    layout::write_uniform_struct(&mut src, &uniform_layout, uniform_pad_f32);
    src.push_str(&format!(
        "\n@group({}) @binding({}) var<uniform> u: Uniforms;\n\n",
        group::UNIFORMS,
        binding::UNIFORM,
    ));
    src.push_str(layout::camera::STATE_WGSL);
    src.push_str(&format!(
        "\n@group({}) @binding({}) var<storage, read_write> cam: CameraState;\n\n",
        group::STATE,
        binding::UNIFORM,
    ));
    // **The field's helpers before its body, and its body before every entry
    // point.** A spliced field lives in this module, so this module's prelude
    // has to carry what it calls — the prelude is demand-driven, and a field
    // calling `sd_torus` in a caller that does not would otherwise produce a
    // call to a function nothing emitted, in a shader that checked clean.
    for f in &splices {
        req.absorb(&f.requirements);
    }
    src.push_str(&prelude::render(&req));
    for f in &splices {
        src.push('\n');
        src.push_str(&f.source);
    }
    src.push('\n');
    src.push_str(&entry(&body));

    L3Shader {
        source: src,
        uniform_layout,
        uniform_pad_f32,
    }
}

/// The local each camera output accumulates into before the entry point writes
/// the six of them out together.
///
/// Locals rather than direct stores into `cam`, for the same reason the L4 path
/// uses them: a block may assign an output more than once, or on one arm of an
/// `if`, and a store per assignment would make the buffer's contents depend on
/// the order the passes happened to write it.
fn output_local(o: Output) -> &'static str {
    match o {
        Output::Eye => "_eye",
        Output::Target => "_target",
        Output::Up => "_up",
        Output::FovY => "_fov_y",
        Output::Near => "_near",
        Output::Far => "_far",
        other => unreachable!(
            "{other:?} is not a camera output and cannot appear in a checked camera block"
        ),
    }
}

struct L3Resolver;

impl Resolver for L3Resolver {
    fn read_attr(&self, attr: karakuri_ir::Attr) -> String {
        unreachable!(
            "`{}` cannot appear in a checked camera block: an L3 has no element",
            attr.name()
        )
    }

    fn read_seed(&self) -> String {
        unreachable!("`seed` is per element and an L3 has none")
    }

    fn read_ambient(&self, amb: Ambient) -> String {
        match amb {
            Ambient::T => "u.t".to_string(),
            Ambient::Beats => "u.beats".to_string(),
            // The fixed step, unscaled. There is no birth fraction to correct
            // for here — that is an L1's first-update adjustment, and a camera
            // is never new.
            Ambient::Dt => "u.dt".to_string(),
            Ambient::Seed => unreachable!("read_seed handles this"),
            Ambient::Point
            | Ambient::Copy
            | Ambient::Capacity
            | Ambient::Camera
            | Ambient::PointCoord
            | Ambient::Eye
            | Ambient::Ray => {
                unreachable!("{amb:?} is not available in a checked camera block")
            }
        }
    }
}

fn emit_stmts(stmts: &[TStmt], req: &mut Requirements, indent: usize, out: &mut String) {
    let pad = "    ".repeat(indent);
    for stmt in stmts {
        match stmt {
            TStmt::Let { name, value, .. } => {
                let v = lower_expr(value, &L3Resolver, req);
                out.push_str(&format!("{pad}let {} = {v};\n", mangle_local(name)));
            }
            TStmt::Var { name, value, .. } => {
                let v = lower_expr(value, &L3Resolver, req);
                out.push_str(&format!("{pad}var {} = {v};\n", mangle_local(name)));
            }
            TStmt::Assign { target, value, .. } => {
                let v = lower_expr(value, &L3Resolver, req);
                match target {
                    Target::Local(name) => {
                        out.push_str(&format!("{pad}{} = {v};\n", mangle_local(name)))
                    }
                    Target::Output(o) => {
                        out.push_str(&format!("{pad}{} = {v};\n", output_local(*o)));
                    }
                    Target::Attr(a) => {
                        unreachable!("a camera block cannot assign `{}`", a.name())
                    }
                }
            }
            TStmt::If {
                cond, then, els, ..
            } => {
                let c = lower_expr(cond, &L3Resolver, req);
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
                emit_stmts(body, req, indent + 1, out);
                out.push_str(&format!("{pad}}}\n"));
            }
            TStmt::Kill { .. } => {
                unreachable!("`kill()` in a camera block is refused by the checker")
            }
        }
    }
}

/// `Orbit::default`'s field of view and planes, so that replacing the built-in
/// camera with the simplest L3 anyone would write does not change the picture
/// underneath it. A third of pi, and 0.1 to 100.
const DEFAULTS: &str = "\
    var _eye    = vec3<f32>(0.0, 0.0, 0.0);
    var _target = vec3<f32>(0.0, 0.0, 0.0);
    var _up     = vec3<f32>(0.0, 1.0, 0.0);
    var _fov_y  = 1.0471976;
    var _near   = 0.1;
    var _far    = 100.0;
";

fn entry(body: &str) -> String {
    format!(
        "@compute @workgroup_size(1)\n\
         fn {ENTRY}() {{\n\
         {DEFAULTS}\n\
         {body}\n\
         \x20   cam.eye     = _eye;\n\
         \x20   // `target` is a reserved word in WGSL and nowhere else — see\n\
         \x20   // `layout::camera::STATE_WGSL`.\n\
         \x20   cam.look_at = _target;\n\
         \x20   cam.up      = _up;\n\
         \x20   cam.fov_y   = _fov_y;\n\
         \x20   cam.near    = _near;\n\
         \x20   cam.far     = _far;\n\
         }}\n"
    )
}
