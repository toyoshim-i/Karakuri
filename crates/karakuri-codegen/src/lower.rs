//! Shared expression lowering: `TExpr` to a WGSL expression string.
//!
//! Both L1 and L4 blocks share every expression form — literals, operators,
//! builtins, constructors, swizzles — and differ only in how a *name*
//! resolves: an attribute read is a prev-buffer index on L1 and an
//! interpolated varying (or an instance-indexed storage read, in `vertex`)
//! on L4. That one axis of difference is factored out as [`Resolver`]; this
//! module is the axis that does not vary.
//!
//! Statement lowering (`let`/`var`/`if`/`for`/assignment) is *not* shared —
//! L1 assigns to attributes and calls `kill()`, L4 assigns to stage outputs
//! and does neither, and trying to unify the two targets was worse than two
//! short, direct implementations in `l1.rs` and `l4.rs`.

use karakuri_ir::builtin::Builtin;
use karakuri_ir::typed::{TExpr, TExprKind};
use karakuri_ir::{Ambient, Attr, BinOp, Lit, Ty, UnOp};

use crate::layout::mangle_param;
use crate::prelude::{mod_helper_name, Requirements};
use crate::ty::wgsl_ty;

/// Mangles a user-chosen identifier — a `let`/`var` binding or a `for` loop
/// variable — into the WGSL identifier this crate actually emits for it.
///
/// Every other identifier this crate writes (`u`, `prev_<attr>`,
/// `next_<attr>`, `attr_<attr>`, entry-point locals like `seed`/`slot`/`i`,
/// every helper function name) is a fixed string chosen by the generator,
/// never by IR text. Locals are the one category that *is* IR text passed
/// through — the language spec calls `let`/`var` lowering "a rename rather
/// than a transformation" — and a rename that reuses the source spelling
/// verbatim makes every fixed name above capturable: a procedure that opens
/// its `spawn` block with `let u = hash1(seed);`, which is not a contrived
/// example but the ir-spec's own `drift_shell`, shadows the generated
/// `var<uniform> u` and every later `u.<param>` silently resolves to the
/// local instead. Naga catches the resulting nonsense, but only because the
/// mistake happens to produce a type error a few lines later — nothing
/// stops the same shadowing from landing on a name whose reuse compiles
/// clean and just computes the wrong thing.
///
/// The fix is not to rename the generator's own identifiers away from
/// whatever a user local might plausibly be called — enumerating "plausible"
/// is exactly the reasoning that missed `u` — it is to make the two
/// namespaces disjoint by construction. Every local this crate ever writes
/// carries this prefix; no fixed identifier this crate emits does or ever
/// will start with it. That turns "no realistic procedure names a local
/// this" into "no procedure's local can spell this," which does not depend
/// on which names turn out to be realistic.
///
/// The prefix is kept short and the source name is kept intact after it
/// specifically so a human reading generated WGSL can still tell which IR
/// name a given local came from — `usr_radius` for `radius`, not a hash or
/// a counter.
pub fn mangle_local(name: &str) -> String {
    format!("usr_{name}")
}

/// How names resolve in the block currently being lowered. Implemented once
/// per (kind, block) combination — see `l1::Resolver` and `l4::Resolver`.
pub trait Resolver {
    /// A read of `attr`. Per the state-semantics rule, this is the *only*
    /// place an attribute read is ever produced, and it never depends on
    /// whether that attribute was assigned earlier in the same block — the
    /// generator has no "current value" register for attributes at all, only
    /// a fixed expression pointing at the previous frame's buffer (L1) or the
    /// value threaded in from the vertex stage (L4). That is what makes the
    /// prev/next rule impossible to get wrong by construction rather than by
    /// discipline.
    fn read_attr(&self, attr: Attr) -> String;

    /// A read of the **far** element, from the geometry bound to this node's
    /// declared slot. Only an L2 that declares one has such a resolver; the
    /// checker refuses the read anywhere else, so every other implementation
    /// says so rather than inventing an answer.
    fn read_far(&self, attr: Attr) -> String {
        unreachable!(
            "a read of `{}` from a used geometry is refused where none is declared",
            attr.name()
        )
    }
    fn read_seed(&self) -> String;
    fn read_ambient(&self, amb: Ambient) -> String;

    /// A read of a declared `param`, where this resolver spells them
    /// differently from every other one.
    ///
    /// **Only a field overrides this**, and it has to: its body is spliced into
    /// a caller's shader and reads its params out of the *caller's* uniform, so
    /// the two sets of names share one struct and are kept apart by a prefix.
    /// Defaulted rather than required, because there is one such resolver and
    /// four that want the ordinary spelling.
    fn read_param(&self, name: &str) -> Option<String> {
        let _ = name;
        None
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
        TExprKind::Unary { op, value } => {
            let v = lower_expr(value, resolver, req);
            match op {
                UnOp::Neg => format!("(-{v})"),
                UnOp::Not => format!("(!{v})"),
            }
        }
        TExprKind::Binary { op, lhs, rhs } => lower_binary(*op, lhs, rhs, resolver, req),
        TExprKind::Builtin { func, args } => lower_builtin(*func, args, expr.ty, resolver, req),
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

    // **Not `field(...)`.** The call site's name is the language's; the
    // function's name is this crate's, and it is spliced in from another
    // procedure entirely. Noting it as a builtin would also ask the prelude for
    // a body it does not have.
    if func == Builtin::Field {
        // **The caller's own spelling of the clock, at the call site.** A field
        // is one body spliced into several kinds of module and an L1 reads `t`
        // from `step_args` where everything else reads `u.t`, so the answer
        // comes from the resolver that is lowering this call — which is the
        // resolver that would have written it inline.
        return format!(
            "{}({}, {}, {})",
            crate::field::FN,
            inner.join(", "),
            resolver.read_ambient(Ambient::T),
            resolver.read_ambient(Ambient::Beats),
        );
    }

    req.note_builtin(func);
    format!("{}({})", func.name(), inner.join(", "))
}

/// Unrolls `fbm(p, octaves)` into a sum of `octaves` scaled `perlin` calls at
/// generation time. WGSL has no preprocessor and no loop whose trip count is
/// visible to a constant folder strong enough to unroll it for us, so this
/// crate does the unrolling itself, in Rust, before any WGSL text exists —
/// `octaves` is one of the check pass's `const_args`, guaranteed to be a
/// literal `int` by the time a `Checked` tree reaches this crate.
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
