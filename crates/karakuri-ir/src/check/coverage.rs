//! Output coverage analysis and closed-form determination.

use std::collections::HashSet;

use crate::ast::{Ambient, Attr, BlockKind, Kind, Output};
use crate::typed::{TBlock, TExpr, TExprKind, TStmt, Target};

// ---------------------------------------------------------------------------
// Closed form versus accumulating.
// ---------------------------------------------------------------------------

/// Determines whether a procedure is closed-form (evaluable at arbitrary `t` without simulation priming).
pub(crate) fn is_closed_form(
    kind: Kind,
    retains: bool,
    carried: &HashSet<Attr>,
    blocks: &[TBlock],
) -> bool {
    if kind == Kind::L5 {
        return !retains;
    }
    if kind == Kind::L4 || kind == Kind::L2 {
        return true;
    }
    if blocks.iter().any(|b| b.kind == BlockKind::Spawn) {
        return false;
    }
    blocks.iter().all(|b| !accumulates(&b.stmts, carried))
}

/// Returns true if any statement invokes `kill()` or reads an emitted attribute.
pub(crate) fn accumulates(stmts: &[TStmt], carried: &HashSet<Attr>) -> bool {
    stmts.iter().any(|s| match s {
        TStmt::Kill { .. } => true,
        TStmt::Let { value, .. } | TStmt::Var { value, .. } => reads_carried(value, carried),
        TStmt::Assign { value, .. } => reads_carried(value, carried),
        TStmt::If {
            cond, then, els, ..
        } => {
            reads_carried(cond, carried) || accumulates(then, carried) || accumulates(els, carried)
        }
        TStmt::For { body, .. } => accumulates(body, carried),
    })
}

/// Returns true if an expression reads any carried or derived attributes.
pub(crate) fn reads_carried(e: &TExpr, carried: &HashSet<Attr>) -> bool {
    match &e.kind {
        TExprKind::Attr(a) | TExprKind::Far(a) => carried.contains(a),
        TExprKind::Lit(_)
        | TExprKind::Local(_)
        | TExprKind::Param(_)
        | TExprKind::Ambient(_)
        | TExprKind::Source { .. } => false,
        TExprKind::Unary { value, .. } => reads_carried(value, carried),
        TExprKind::Binary { lhs, rhs, .. } => {
            reads_carried(lhs, carried) || reads_carried(rhs, carried)
        }
        TExprKind::Builtin { args, .. } | TExprKind::Construct { args } => {
            args.iter().any(|a| reads_carried(a, carried))
        }
        TExprKind::Sample { at, .. } => at.as_ref().is_some_and(|a| reads_carried(a, carried)),
        TExprKind::Field { point, .. } => reads_carried(point, carried),
        TExprKind::Swizzle { value, .. } => reads_carried(value, carried),
    }
}

/// Returns true if the procedure reads [`Ambient::Beats`] anywhere.
pub(crate) fn reads_beats(blocks: &[TBlock]) -> bool {
    fn in_stmts(stmts: &[TStmt]) -> bool {
        stmts.iter().any(|s| match s {
            TStmt::Kill { .. } => false,
            TStmt::Let { value, .. } | TStmt::Var { value, .. } => in_expr(value),
            TStmt::Assign { value, .. } => in_expr(value),
            TStmt::If {
                cond, then, els, ..
            } => in_expr(cond) || in_stmts(then) || in_stmts(els),
            TStmt::For { body, .. } => in_stmts(body),
        })
    }
    fn in_expr(e: &TExpr) -> bool {
        match &e.kind {
            TExprKind::Ambient(Ambient::Beats) => true,
            TExprKind::Lit(_)
            | TExprKind::Local(_)
            | TExprKind::Param(_)
            | TExprKind::Attr(_)
            | TExprKind::Far(_)
            | TExprKind::Source { .. }
            | TExprKind::Ambient(_) => false,
            TExprKind::Unary { value, .. } => in_expr(value),
            TExprKind::Binary { lhs, rhs, .. } => in_expr(lhs) || in_expr(rhs),
            TExprKind::Builtin { args, .. } | TExprKind::Construct { args } => {
                args.iter().any(in_expr)
            }
            // The field's own body is another file's answer to this question,
            // and the Set asks it there — see `Checked::reads_beats`. What is
            // this procedure's is the point it hands over.
            TExprKind::Field { point, .. } => in_expr(point),
            // The same shape: what is this procedure's is the coordinate it
            // computes, and a texture holds no clock.
            TExprKind::Sample { at, .. } => at.as_ref().is_some_and(|a| in_expr(a)),
            TExprKind::Swizzle { value, .. } => in_expr(value),
        }
    }
    blocks.iter().any(|b| in_stmts(&b.stmts))
}

// ---------------------------------------------------------------------------
// Coverage: every emitted attribute and every required stage output must be
// assigned on every path through its block.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CovKey {
    Attr(Attr),
    Output(Output),
}

impl CovKey {
    pub(crate) fn name(self) -> &'static str {
        match self {
            CovKey::Attr(a) => a.name(),
            CovKey::Output(o) => o.name(),
        }
    }
}

/// Returns true if `stmts` assigns `Output::ClipB` on any branch or loop body.
pub(crate) fn assigns_clip_b(stmts: &[TStmt]) -> bool {
    stmts.iter().any(|s| match s {
        TStmt::Assign { target, .. } => matches!(target, Target::Output(Output::ClipB)),
        TStmt::If { then, els, .. } => assigns_clip_b(then) || assigns_clip_b(els),
        TStmt::For { body, .. } => assigns_clip_b(body),
        TStmt::Let { .. } | TStmt::Var { .. } | TStmt::Kill { .. } => false,
    })
}

pub(crate) fn required_keys(
    block: BlockKind,
    emit: &HashSet<Attr>,
    draws_lines: bool,
) -> Vec<CovKey> {
    match block {
        BlockKind::Spawn | BlockKind::Element => emit.iter().map(|a| CovKey::Attr(*a)).collect(),
        BlockKind::Field => vec![CovKey::Output(Output::Distance)],
        BlockKind::Vertex => {
            let mut keys = vec![
                CovKey::Output(Output::Clip),
                CovKey::Output(Output::PointRate),
            ];
            if draws_lines {
                keys.push(CovKey::Output(Output::ClipB));
            }
            keys
        }
        BlockKind::Fragment => vec![CovKey::Output(Output::Color)],
        BlockKind::Frame => vec![CovKey::Output(Output::Color)],
        BlockKind::Deform => Vec::new(),
        BlockKind::Mask => vec![CovKey::Output(Output::Strength)],
        BlockKind::Camera => {
            vec![CovKey::Output(Output::Eye), CovKey::Output(Output::Target)]
        }
    }
}

pub(crate) fn coverage(stmts: &[TStmt]) -> HashSet<CovKey> {
    covered_from(stmts, &HashSet::new())
}

pub(crate) fn covered_from(stmts: &[TStmt], base: &HashSet<CovKey>) -> HashSet<CovKey> {
    let mut set = base.clone();
    for s in stmts {
        match s {
            TStmt::Assign {
                target: Target::Attr(a),
                ..
            } => {
                set.insert(CovKey::Attr(*a));
            }
            TStmt::Assign {
                target: Target::Output(o),
                ..
            } => {
                set.insert(CovKey::Output(*o));
            }
            TStmt::Assign {
                target: Target::Local(_),
                ..
            } => {}
            TStmt::If { then, els, .. } => {
                let then_set = covered_from(then, &set);
                let els_set = covered_from(els, &set);
                set = then_set.intersection(&els_set).copied().collect();
            }
            // Conservative: a `for` body might run zero times even though
            // bounds are literal, so nothing it assigns is treated as
            // definite afterward. Still recursed into so its own statements
            // get their own diagnostics.
            TStmt::For { body, .. } => {
                let _ = covered_from(body, &set);
            }
            TStmt::Let { .. } | TStmt::Var { .. } | TStmt::Kill { .. } => {}
        }
    }
    set
}
