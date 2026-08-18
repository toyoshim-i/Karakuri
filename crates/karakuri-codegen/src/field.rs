//! Field lowering: a `kind Field` procedure as one WGSL function.
//!
//! # The only kind that lowers to no pass
//!
//! Every other generator in this crate produces a module: bindings, a uniform
//! struct, an entry point. A field produces **a function and a list of
//! params**, and nothing else — no bindings, because it reads none; no uniform
//! struct, because its params live in the uniform of whoever calls it; no
//! entry point, because it is not dispatched.
//!
//! `docs/roadmap.md` settled that a `kind` says what a procedure *lowers to*,
//! and that an L5 has no `kind` because it has no code to lower. This is the
//! mirror — only code, so a file and no node.
//!
//! # Its params are the caller's uniform, under a prefix of their own
//!
//! A spliced body reads `u.field_ball`, in the caller's `Uniforms` struct. The
//! prefix is not decoration: a renderer declaring `exposure` beside a field
//! declaring `exposure` would otherwise be one uniform field with two meanings,
//! and prefixing them apart removes a refusal that would otherwise have to
//! exist. See [`crate::layout::mangle_field_param`].
//!
//! # What it may not read
//!
//! No attribute, and no ambient but `point` — refused in the check pass, with
//! the reason rather than with advice to declare something. A field is handed a
//! position and the material that happens to be at it is not something it can
//! see.

use karakuri_ir::typed::{Checked, TStmt, Target};
use karakuri_ir::{Ambient, Attr, BlockKind, Kind, Output};

use crate::layout;
use crate::lower::{lower_expr, mangle_local, Resolver};
use crate::prelude::Requirements;

/// The WGSL name of the function a field lowers to.
///
/// **One name, because there is one field per Set** — the same restriction the
/// camera has, and for the same reason: several would need naming, and naming
/// is fan-in.
pub const FN: &str = "_field_at";

/// The parameter the body reads as `point`.
///
/// Not spelled `point`: a WGSL identifier chosen by this crate should never be
/// one an author could also have chosen, and the same argument that mangles
/// every `param` applies to a function parameter that shares a scope with the
/// body's locals.
const POINT: &str = "_field_p";

/// The clock, passed in rather than read.
///
/// **A field cannot know how its caller spells `t`.** An L1 reads it from
/// `step_args`, because it is substepped and each substep lands on its own
/// instant; everything else reads `u.t`. One spliced body cannot say both, and
/// generating the body once per caller would be the same text compiled several
/// ways for no reason. So the caller passes its own answer at the call site —
/// see `lower::lower_call`, which asks its resolver for exactly the spelling it
/// would have used itself.
const T: &str = "_field_t";
const BEATS: &str = "_field_beats";

pub struct FieldShader {
    /// The function, ready to splice ahead of a caller's entry points.
    pub source: String,
    /// What the body requires of the prelude. **The caller's requirements have
    /// to absorb these**, since the helpers live in the caller's module.
    pub requirements: Requirements,
    /// Declared `param` names, in order, for the caller's uniform.
    pub params: Vec<(String, &'static str)>,
}

/// Generate the WGSL function for one field.
pub fn generate_field(checked: &Checked) -> FieldShader {
    assert_eq!(checked.kind, Kind::Field, "generate_field called on a non-Field procedure");

    let block = checked
        .block(BlockKind::Field)
        .expect("a Field procedure must have a field block");

    let mut req = Requirements::default();
    let body = {
        let mut out = String::new();
        emit_stmts(&block.stmts, &mut req, 2, &mut out);
        out
    };

    let params = checked
        .params
        .iter()
        .map(|p| (p.name.clone(), crate::ty::wgsl_ty(p.ty)))
        .collect();

    // `var` rather than `let`, because the block may assign `distance` on
    // several paths — the coverage check requires every path, not one.
    let source = format!(
        "fn {FN}({POINT}: vec3<f32>, {T}: f32, {BEATS}: f32) -> f32 {{\n\
         \x20   var _distance: f32;\n\
         \x20   {{\n\
         {body}\
         \x20   }}\n\
         \x20   return _distance;\n\
         }}\n"
    );

    FieldShader { source, requirements: req, params }
}

/// Reads resolve to the function parameter and to the caller's uniform; there
/// is nothing else in scope.
struct FieldResolver;

impl Resolver for FieldResolver {
    fn read_attr(&self, attr: Attr) -> String {
        unreachable!("`{}` is refused in a field block: a field has no element", attr.name())
    }

    fn read_seed(&self) -> String {
        unreachable!("`seed` is per element and a field has none")
    }

    fn read_ambient(&self, amb: Ambient) -> String {
        match amb {
            Ambient::Point => POINT.to_string(),
            // **`t` and `beats` are readable**, and a field that moves with the
            // clock is a legitimate thing to write. They arrive as arguments
            // rather than out of a uniform, for the reason above.
            Ambient::T => T.to_string(),
            Ambient::Beats => BEATS.to_string(),
            other => unreachable!("{other:?} is not available in a checked field block"),
        }
    }

    fn read_param(&self, name: &str) -> Option<String> {
        Some(format!("u.{}", layout::mangle_field_param(name)))
    }
}

fn emit_stmts(stmts: &[TStmt], req: &mut Requirements, indent: usize, out: &mut String) {
    let pad = "    ".repeat(indent);
    for stmt in stmts {
        match stmt {
            TStmt::Let { name, value, .. } => {
                let v = lower_expr(value, &FieldResolver, req);
                out.push_str(&format!("{pad}let {} = {v};\n", mangle_local(name)));
            }
            TStmt::Var { name, value, .. } => {
                let v = lower_expr(value, &FieldResolver, req);
                out.push_str(&format!("{pad}var {} = {v};\n", mangle_local(name)));
            }
            TStmt::Assign { target, value, .. } => {
                let v = lower_expr(value, &FieldResolver, req);
                match target {
                    Target::Local(name) => {
                        out.push_str(&format!("{pad}{} = {v};\n", mangle_local(name)))
                    }
                    Target::Output(Output::Distance) => {
                        out.push_str(&format!("{pad}_distance = {v};\n"));
                    }
                    other => unreachable!("a field never assigns {other:?}"),
                }
            }
            TStmt::If { cond, then, els, .. } => {
                let c = lower_expr(cond, &FieldResolver, req);
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
            TStmt::Kill { .. } => unreachable!("`kill()` in a field is refused by the checker"),
        }
    }
}
