//! Shared expression lowering: converts `TExpr` into a WGSL expression string.
//!
//! Handles expression forms common to all procedure stages (literals, operators, builtins,
//! constructors, swizzles) using [`Resolver`] to handle stage-specific name resolutions.

use karakuri_ir::builtin::Builtin;
use karakuri_ir::typed::{TExpr, TExprKind, TexRef};
use karakuri_ir::{Ambient, Attr, BinOp, Lit, Ty, UnOp};

use crate::layout::{mangle_param, mangle_source_slot};
use crate::prelude::{mod_helper_name, Requirements};
use crate::ty::wgsl_ty;

/// Mangles user identifiers with a prefix (`usr_`) to prevent shadowing internal shader variables.
pub fn mangle_local(name: &str) -> String {
    format!("usr_{name}")
}

/// How names resolve in the block currently being lowered. Implemented once
/// per (kind, block) combination — see `l1::Resolver` and `l4::Resolver`.
pub trait Resolver {
    /// Emits the expression to read `attr` from the appropriate stage-specific source.
    fn read_attr(&self, attr: Attr) -> String;

    /// Reads the far element from the geometry bound to this node's declared slot.
    fn read_far(&self, attr: Attr) -> String {
        unreachable!(
            "a read of `{}` from a used geometry is refused where none is declared",
            attr.name()
        )
    }
    fn read_seed(&self) -> String;
    fn read_ambient(&self, amb: Ambient) -> String;

    /// Reads a declared `param`, allowing custom naming or prefixes.
    ///
    /// Overridden by field resolvers whose spliced bodies access parameters from
    /// the caller's uniform struct using prefixed identifiers.
    fn read_param(&self, name: &str) -> Option<String> {
        let _ = name;
        None
    }

    /// WGSL binding name for an input texture in an L5 pass.
    ///
    /// Texture fetches are validated by the IR checker to occur only in L5 compositors.
    fn read_texture(&self, tex: &TexRef) -> String {
        unreachable!("a fetch from {tex:?} is refused outside a `frame` block")
    }

    /// Integer coordinate of the current fragment's texel, used by unfiltered `textureLoad`.
    fn texel_at(&self) -> String {
        unreachable!("`texel` is refused outside a `frame` block")
    }

    /// Identifier of the sampler used for filtered `tap` reads.
    fn sampler(&self) -> String {
        unreachable!("`tap` is refused outside a `frame` block")
    }
}

/// Lowers one expression, recording any helper functions or `mod`
/// instantiations it needs along the way.
pub fn lower_expr(expr: &TExpr, resolver: &dyn Resolver, req: &mut Requirements) -> String {
    match &expr.kind {
        TExprKind::Lit(lit) => lower_lit(*lit),
        TExprKind::Local(name) => mangle_local(name),
        TExprKind::Param(name) => resolver
            .read_param(name)
            .unwrap_or_else(|| format!("u.{}", mangle_param(name))),
        TExprKind::Attr(attr) => resolver.read_attr(*attr),
        TExprKind::Far(attr) => resolver.read_far(*attr),
        TExprKind::Ambient(Ambient::Seed) => resolver.read_seed(),
        TExprKind::Ambient(amb) => resolver.read_ambient(*amb),
        // Source slots read directly from module-level uniforms u.
        TExprKind::Source { slot } => format!("u.{}", mangle_source_slot(slot)),
        TExprKind::Unary { op, value } => {
            let v = lower_expr(value, resolver, req);
            match op {
                UnOp::Neg => format!("(-{v})"),
                UnOp::Not => format!("(!{v})"),
            }
        }
        TExprKind::Binary { op, lhs, rhs } => lower_binary(*op, lhs, rhs, resolver, req),
        TExprKind::Builtin { func, args } => lower_builtin(*func, args, expr.ty, resolver, req),
        // Spliced field procedure evaluation.
        TExprKind::Field { slot, point } => format!(
            "{}({}, {}, {})",
            crate::field::fn_name(slot),
            lower_expr(point, resolver, req),
            resolver.read_ambient(Ambient::T),
            resolver.read_ambient(Ambient::Beats),
        ),
        // Texture sampling operations: unfiltered integer load or filtered level-0 sample.
        TExprKind::Sample { func, texture, at } => {
            let tex = resolver.read_texture(texture);
            match func {
                Builtin::Texel => format!("textureLoad({tex}, {}, 0)", resolver.texel_at()),
                Builtin::Tap => {
                    let uv = at
                        .as_ref()
                        .map(|a| lower_expr(a, resolver, req))
                        .expect("a checked `tap` carries its coordinate");
                    format!(
                        "textureSampleLevel({tex}, {}, {uv}, 0.0)",
                        resolver.sampler()
                    )
                }
                other => unreachable!("{other:?} does not take a texture"),
            }
        }
        TExprKind::Construct { args } => {
            let inner: Vec<String> = args.iter().map(|a| lower_expr(a, resolver, req)).collect();
            format!("{}({})", wgsl_ty(expr.ty), inner.join(", "))
        }
        TExprKind::Swizzle { value, components } => {
            let v = lower_expr(value, resolver, req);
            let letters: String = components
                .iter()
                .map(|&c| b"xyzw"[c as usize] as char)
                .collect();
            format!("{v}.{letters}")
        }
    }
}

fn lower_lit(lit: Lit) -> String {
    match lit {
        // `{:?}` on f32 always prints a decimal point (`1.0`, not `1`),
        // which is what makes the result an unambiguous WGSL float literal.
        Lit::Float(f) => format!("{f:?}"),
        Lit::Int(i) => format!("{i}"),
        Lit::Uint(u) => format!("{u}u"),
        Lit::Bool(b) => b.to_string(),
    }
}

fn lower_binary(
    op: BinOp,
    lhs: &TExpr,
    rhs: &TExpr,
    resolver: &dyn Resolver,
    req: &mut Requirements,
) -> String {
    let l = lower_expr(lhs, resolver, req);
    let r = lower_expr(rhs, resolver, req);

    if op == BinOp::Rem {
        // IR `%` follows `mod` semantics on float-family types (always the
        // sign of the divisor), which WGSL's `%` does not — see
        // `docs/ir-spec.md`, "Operators follow GLSL." `int`/`uint` keep the
        // native operator, which already matches ordinary remainder.
        return match lhs.ty {
            Ty::Float | Ty::Vec2 | Ty::Vec3 | Ty::Vec4 => {
                req.note_mod(lhs.ty);
                format!("{}({l}, {r})", mod_helper_name(lhs.ty))
            }
            _ => format!("({l} % {r})"),
        };
    }

    let sym = match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Rem => unreachable!("handled above"),
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::And => "&&",
        BinOp::Or => "||",
    };
    format!("({l} {sym} {r})")
}

fn lower_builtin(
    func: Builtin,
    args: &[TExpr],
    ret_ty: Ty,
    resolver: &dyn Resolver,
    req: &mut Requirements,
) -> String {
    if func == Builtin::Fbm {
        return lower_fbm(args, resolver, req);
    }

    let inner: Vec<String> = args.iter().map(|a| lower_expr(a, resolver, req)).collect();

    if func == Builtin::Mod {
        // WGSL has no function named `mod`; this is the one builtin whose
        // call-site name is not `Builtin::name()` verbatim. See
        // `prelude::mod_helper_name`.
        req.note_mod(ret_ty);
        return format!("{}({})", mod_helper_name(ret_ty), inner.join(", "));
    }

    req.note_builtin(func);
    format!("{}({})", func.name(), inner.join(", "))
}

/// Unrolls `fbm(p, octaves)` into a compile-time sum of scaled `perlin` octaves.
fn lower_fbm(args: &[TExpr], resolver: &dyn Resolver, req: &mut Requirements) -> String {
    let p = lower_expr(&args[0], resolver, req);
    let octaves = match &args[1].kind {
        TExprKind::Lit(Lit::Int(n)) => *n,
        other => unreachable!("fbm's octave count is a check-pass const_arg, not {other:?}"),
    };
    // Fbm itself has no helper function — it is inlined — but it pulls in
    // perlin's dependency chain, so register it for the prelude to see.
    req.note_builtin(Builtin::Fbm);

    let mut terms = Vec::new();
    let mut amp = 0.5_f32;
    let mut freq = 1.0_f32;
    for _ in 0..octaves.max(0) {
        terms.push(format!("perlin({p} * {freq:?}) * {amp:?}"));
        amp *= 0.5;
        freq *= 2.0;
    }
    if terms.is_empty() {
        "0.0".to_string()
    } else {
        format!("({})", terms.join(" + "))
    }
}
