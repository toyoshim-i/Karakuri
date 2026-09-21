use crate::span::Span;

use super::{BinOp, Expr};

#[derive(Debug, Clone)]
pub enum Stmt {
    /// `let <name> = <expr>;` — immutable.
    Let {
        name: String,
        value: Expr,
        span: Span,
    },
    /// `var <name> = <expr>;` — mutable, block scoped, does not survive the frame.
    Var {
        name: String,
        value: Expr,
        span: Span,
    },
    /// `<target> = <expr>;` or `<target> op= <expr>;`
    ///
    /// The target is a bare name: whether it is a local, an attribute, or a stage
    /// output is decided during resolution. Compound assignment (`op` is `Some`) is
    /// legal on locals only — on an attribute it would look like accumulation while
    /// re-reading the previous frame every time.
    Assign {
        target: String,
        op: Option<BinOp>,
        value: Expr,
        span: Span,
    },
    If {
        cond: Expr,
        then: Vec<Stmt>,
        els: Vec<Stmt>,
        span: Span,
    },
    /// `for <var> in <start>..<end> { .. }` — bounds are compile-time constants,
    /// which is what makes cost estimation possible.
    For {
        var: String,
        start: i32,
        end: i32,
        body: Vec<Stmt>,
        span: Span,
    },
    /// `kill();` — L1 `element` only.
    Kill { span: Span },
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. }
            | Stmt::Var { span, .. }
            | Stmt::Assign { span, .. }
            | Stmt::If { span, .. }
            | Stmt::For { span, .. }
            | Stmt::Kill { span } => *span,
        }
    }
}
