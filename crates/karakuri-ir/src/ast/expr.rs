use crate::span::Span;

use super::Ty;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lit {
    Float(f32),
    Int(i32),
    Uint(u32),
    Bool(bool),
}

impl Lit {
    pub fn ty(self) -> Ty {
        match self {
            Lit::Float(_) => Ty::Float,
            Lit::Int(_) => Ty::Int,
            Lit::Uint(_) => Ty::Uint,
            Lit::Bool(_) => Ty::Bool,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    /// `%`. Follows GLSL `mod` semantics on the float family — the result takes the
    /// sign of the divisor, which is what `a - b * floor(a / b)` gives and is what
    /// `karakuri-codegen`'s `prelude::mod_helper_name` wrapper emits; ordinary
    /// remainder on integers, which is WGSL's own `%` and takes the sign of the
    /// dividend.
    ///
    /// This said *always non-negative* until 2026-09-08, which was wrong rather
    /// than imprecise: `(-1.0) % 3.0` is 2.0 under these semantics and `1.0 %
    /// (-3.0)` is -2.0. `karakuri-codegen`'s `lower.rs` states the rule correctly
    /// at the one place it is applied — *"always the sign of the divisor"* — and
    /// the emitted helper carries the same sentence into the shader.
    Rem,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    And,
    Or,
}

impl BinOp {
    pub fn is_comparison(self) -> bool {
        matches!(
            self,
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq | BinOp::Ne
        )
    }

    pub fn is_logical(self) -> bool {
        matches!(self, BinOp::And | BinOp::Or)
    }

    /// Binding power, higher binds tighter. Comparison below arithmetic, logical
    /// below comparison — GLSL order, which is what LLMs will assume.
    pub fn precedence(self) -> u8 {
        match self {
            BinOp::Or => 1,
            BinOp::And => 2,
            BinOp::Eq | BinOp::Ne => 3,
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => 4,
            BinOp::Add | BinOp::Sub => 5,
            BinOp::Mul | BinOp::Div | BinOp::Rem => 6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Lit {
        value: Lit,
        span: Span,
    },
    Ident {
        name: String,
        span: Span,
    },
    Unary {
        op: UnOp,
        value: Box<Expr>,
        span: Span,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },
    /// Builtin calls (`hash1`, `curl`) and type constructors (`vec3`, `float`) are
    /// both this. The check pass tells them apart by name.
    Call {
        name: String,
        args: Vec<Expr>,
        span: Span,
    },
    /// `v.xy`, `v.zyx`. `components` is the raw suffix, unvalidated.
    Swizzle {
        value: Box<Expr>,
        components: String,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Lit { span, .. }
            | Expr::Ident { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Call { span, .. }
            | Expr::Swizzle { span, .. } => *span,
        }
    }
}
