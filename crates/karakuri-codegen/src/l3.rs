//! Compiles L3 `camera` procedures into a single compute entry point writing `CameraState`.

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
    b.field("dt", "f32");
    b.field("seed_salt", "u32");
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
    src.push_str(&entry(&body));

    L3Shader {
        source: src,
        uniform_layout,
        uniform_pad_f32,
    }
}

/// Returns the local variable name used to accumulate each camera output before writing to `CameraState`.
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
            // An L3 runs once a frame over no geometry, so there is no chain
            // instance for `source` to name — refused in the checker, beside
            // `seed` and for the same reason.
            Ambient::Source
            | Ambient::Point
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
