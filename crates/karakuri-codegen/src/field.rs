//! Lowers `kind Field` procedures into pure WGSL distance functions and parameter lists.
//! Functions are named per slot ([`fn_name`]) to avoid collisions across multiple slots.

use karakuri_ir::typed::{Checked, TStmt, Target};
use karakuri_ir::{Ambient, Attr, BlockKind, Kind, Output};

use crate::layout;
use crate::lower::{lower_expr, mangle_local, Resolver};
use crate::prelude::Requirements;

/// Returns the WGSL function name generated for a field reached via `slot`.
pub fn fn_name(slot: &str) -> String {
    format!("_field_{slot}_at")
}

/// The WGSL parameter name representing the input point to the field.
const POINT: &str = "_field_p";

/// The clock, passed as a parameter rather than read globally.
///
/// Caller passes its own time representation at the call site (e.g. L1 reads substepped time
/// from `step_args`, whereas L2/L4 reads `u.t`).
const T: &str = "_field_t";
const BEATS: &str = "_field_beats";

pub struct FieldShader {
    /// The caller's name for this field slot, under which functions and parameters are scoped.
    pub slot: String,
    /// The function, ready to splice ahead of a caller's entry points.
    pub source: String,
    /// Prelude requirements for the field body, absorbed into the caller module.
    pub requirements: Requirements,
    /// Declared `param` names, in order, for the caller's uniform.
    pub params: Vec<(String, &'static str)>,
}

/// Generate the WGSL function for one field, as reached through `slot`.
pub fn generate_field(checked: &Checked, slot: &str) -> FieldShader {
    assert_eq!(
        checked.kind,
        Kind::Field,
        "generate_field called on a non-Field procedure"
    );

    let block = checked
        .block(BlockKind::Field)
        .expect("a Field procedure must have a field block");

    let mut req = Requirements::default();
    let resolver = FieldResolver { slot };
    let body = {
        let mut out = String::new();
        emit_stmts(&block.stmts, &resolver, &mut req, 2, &mut out);
        out
    };

    let params = checked
        .params
        .iter()
        .map(|p| (p.name.clone(), crate::ty::wgsl_ty(p.ty)))
        .collect();

    // `var` rather than `let`, because the block may assign `distance` on
    // several paths — the coverage check requires every path, not one.
    let name = fn_name(slot);
    let source = format!(
        "fn {name}({POINT}: vec3<f32>, {T}: f32, {BEATS}: f32) -> f32 {{\n\
         \x20   var _distance: f32;\n\
         \x20   {{\n\
         {body}\
         \x20   }}\n\
         \x20   return _distance;\n\
         }}\n"
    );

    FieldShader {
        slot: slot.to_string(),
        source,
        requirements: req,
        params,
    }
}

/// Resolves reads within a field body to the function parameters or the caller's uniform.
struct FieldResolver<'a> {
    slot: &'a str,
}

impl Resolver for FieldResolver<'_> {
    fn read_attr(&self, attr: Attr) -> String {
        unreachable!(
            "`{}` is refused in a field block: a field has no element",
            attr.name()
        )
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
        Some(format!("u.{}", layout::mangle_field_param(self.slot, name)))
    }
}

fn emit_stmts(
    stmts: &[TStmt],
    resolver: &FieldResolver<'_>,
    req: &mut Requirements,
    indent: usize,
    out: &mut String,
) {
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
                    Target::Local(name) => {
                        out.push_str(&format!("{pad}{} = {v};\n", mangle_local(name)))
                    }
                    Target::Output(Output::Distance) => {
                        out.push_str(&format!("{pad}_distance = {v};\n"));
                    }
                    other => unreachable!("a field never assigns {other:?}"),
                }
            }
            TStmt::If {
                cond, then, els, ..
            } => {
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
                emit_stmts(body, resolver, req, indent + 1, out);
                out.push_str(&format!("{pad}}}\n"));
            }
            TStmt::Kill { .. } => unreachable!("`kill()` in a field is refused by the checker"),
        }
    }
}
