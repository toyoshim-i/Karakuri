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

/// The spelling [`Output::PointRate`] had before its unit stopped being pixels.
///
/// Kept as a name this pass still recognises so that a `.kir` written against
/// the old language is refused with the new spelling rather than with "never
/// declared". It is not a reserved word: nothing stops an author declaring a
/// local or a param called `point_size`, and if one does the declaration wins —
/// this only catches the name when nothing else claims it.
pub(super) const OLD_POINT_SIZE: &str = "point_size";

/// What to do about it, in one sentence.
///
/// The division is deliberately not given a number. `point_rate` is a fraction
/// of the render target's height, and which height a file's old pixel values
/// were authored against is a fact about that file rather than about the
/// language. Naming one here would make it an anchor.
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
