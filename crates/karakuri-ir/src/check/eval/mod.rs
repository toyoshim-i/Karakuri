//! Expression and statement type-checking and semantic evaluation.

mod call;
mod expr;
mod stmt;

pub(crate) use super::*;
pub(crate) use crate::ast::{
    Ambient, Attr, BinOp, Expr, Kind, Lit, Output, Stmt, Ty, UnOp, TEXTURE_HELD, TEXTURE_SRC,
};
pub(crate) use crate::builtin::{Builtin, Domain, Shape};
pub(crate) use crate::error::Stage;
pub(crate) use crate::span::Span;
pub(crate) use crate::typed::{TExpr, TExprKind, TStmt, Target, TexRef};

/// Legacy identifier for `point_rate` recognized for user migration diagnostics.
pub(super) const OLD_POINT_SIZE: &str = "point_size";

/// Diagnostic migration hint explaining the transition from pixel `point_size` to normalized `point_rate`.
pub(super) const POINT_RATE_HINT: &str = concat!(
    "the output is now `point_rate`, a fraction of the render target's height ",
    "rather than a count of pixels — rename it and divide the old pixel value ",
    "by the height it was authored against"
);

pub(crate) fn domain_allows(d: Domain, ty: Ty) -> bool {
    match d {
        Domain::None => false,
        Domain::FloatOrVector => matches!(ty, Ty::Float | Ty::Vec2 | Ty::Vec3 | Ty::Vec4),
        Domain::VectorOnly => matches!(ty, Ty::Vec2 | Ty::Vec3 | Ty::Vec4),
    }
}

pub(crate) fn op_symbol(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Rem => "%",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::And => "&&",
        BinOp::Or => "||",
    }
}

/// A well-typed placeholder used only for continued error recovery. The tree it
/// lands in is discarded whenever any error was recorded — `check` never
/// returns `Ok` otherwise — so this only has to keep `check_stmts` from
/// panicking, not represent anything meaningful.
pub(crate) fn poison(ty: Ty, span: Span) -> TExpr {
    match ty {
        Ty::Float => TExpr::new(ty, span, TExprKind::Lit(Lit::Float(0.0))),
        Ty::Int => TExpr::new(ty, span, TExprKind::Lit(Lit::Int(0))),
        Ty::Uint => TExpr::new(ty, span, TExprKind::Lit(Lit::Uint(0))),
        Ty::Bool => TExpr::new(ty, span, TExprKind::Lit(Lit::Bool(false))),
        Ty::Vec2 | Ty::Vec3 | Ty::Vec4 => {
            let zero = TExpr::new(Ty::Float, span, TExprKind::Lit(Lit::Float(0.0)));
            TExpr::new(ty, span, TExprKind::Construct { args: vec![zero] })
        }
        Ty::Mat3 | Ty::Mat4 => TExpr::new(ty, span, TExprKind::Lit(Lit::Float(0.0))),
    }
}
