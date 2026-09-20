//! Output coverage analysis and closed-form determination.

use std::collections::HashSet;

use crate::ast::{Ambient, Attr, BlockKind, Kind, Output};
use crate::typed::{TBlock, TExpr, TExprKind, TStmt, Target};

// ---------------------------------------------------------------------------
// Closed form versus accumulating.
// ---------------------------------------------------------------------------

/// Whether this procedure is a pure function of `seed`, `t`, and its params —
/// so any `t` can be evaluated directly. See
/// [`Checked::closed_form`](crate::typed::Checked::closed_form) for what that
/// buys (no priming, and — the larger half — the material can be scrubbed
/// forwards, held, or run backwards) and `docs/ir-spec.md`, "Closed form versus
/// accumulating", for the whole of it.
///
/// Three things make a procedure accumulating, and only the first is the one
/// the name suggests:
///
/// 1. It reads an attribute it emits. `age = age + dt` reads one; `position =
///    f(seed, t)` does not. Every read counts, wherever it is — inside an `if`,
///    inside a `for`, or bound to a `let` and used later — so this walks every
///    expression in every block rather than trying to decide which reads "reach" a
///    write. A read that reaches an emitted attribute's value through a local is
///    still a read of that attribute, and it is the read this looks at, not the
///    local.
///
///    The set tested against is what the procedure *carries*, not what it emits,
///    and the difference arrived with attribute derivation: an L1 may consume
///    `age` or `velocity` without emitting either, and both are per-element state
///    carried across frames. Reading one is reading where the element has been,
///    which is exactly what this property is about. This paragraph used to say
///    `consumes ⊆ emit` held inside an L1 and that the membership test was
///    therefore belt-and-braces; it no longer holds, and a permissive answer here
///    is the one the block below calls far worse — a scrub that produces garbage,
///    and material put on air unwarmed.
///
/// 2. It has a `spawn` block. Spawning and closed form cannot coexist, and the
///    reason is not about attributes at all: an element that does not exist yet
///    cannot be stepped, and *whether it exists* is engine state — the spawn
///    accumulator, the seed counter, the live range — accumulated from every frame
///    since the Set started. Jumping to `t = 30` on a Set that spawns 8000
///    elements a second does not produce 240,000 elements; it produces the handful
///    of them one frame's accumulator emits, at their spawn state. The population
///    is the state that had to be warmed, and it is not reachable from `seed` and
///    `t`. So a `spawn` block disqualifies, however pure the `element` block is.
///
/// 3. It can `kill()`. A killed element stays killed, so the live set at `t` is
///    a function of every step taken to get there and not of `t`. Even a kill
///    condition written purely in `seed` and `t` is history-dependent in the
///    direction that matters: `if t > 5.0 && t < 5.1 { kill() }` removes nothing
///    at all if `t = 6.0` is arrived at in one step.
///
/// The conservative direction is the safe one, and this errs into it
/// deliberately. There is no diagnostic attached to this decision — nothing is
/// rejected either way — so the only way it can be wrong is silently. Wrong in
/// the strict direction costs a warm-up that was not needed: a slot primes for
/// a few seconds it could have skipped, and cannot be scrubbed when it could
/// have been. Wrong in the permissive direction is far worse, and gets worse
/// the more the property is used for. It puts a slot on air showing an unwarmed
/// image — particles being born, an integrator at its initial condition — while
/// telling the governor it needed no warming, so nothing anywhere is looking
/// for the problem. And once transport is built on this, a wrongly-claimed
/// closed form is a scrub that produces garbage rather than merely a bad first
/// second: seeking an accumulating procedure to an arbitrary `t` evaluates it
/// once from wherever it happened to be. That is the failure this whole pass
/// exists to refuse — checking clean and then coming up short at runtime.
/// Under-claim.
pub(crate) fn is_closed_form(
    kind: Kind,
    retains: bool,
    carried: &HashSet<Attr>,
    blocks: &[TBlock],
) -> bool {
    // **An L5 is the stateless layers' shape with one exception, and the
    // exception is the whole of what `retains` declares.** A frame effect with
    // no `retains` is a function of the picture it is handed, the clock and its
    // params — nothing to warm, on an L4's terms. One that reads `held` is
    // reading its own output from the previous frame, which is accumulation
    // whatever it is spelled with: the trail at `t` is every frame that led to
    // it, and there is no un-integrating one. Under-claim.
    if kind == Kind::L5 {
        return !retains;
    }
    // Vacuously true for L4, and said here rather than left to fall out of an
    // empty `emit`. An L4 procedure holds no per-element state: it reads what
    // L1 wrote and throws the result at a target, so there is nothing about it
    // to warm at any `t`. Nothing rejects a stray `emit` on an L4 — it has no
    // meaning there and no buffer behind it — and without this line such a
    // procedure reads its own `emit` list in `vertex`, is called accumulating,
    // and drags a Set that needs no priming into needing it.
    // **Vacuously true for the stateless layers.** An L4 draws what it is given
    // and an L2 is stateless by rule — `docs/ir-spec.md`, "L2 and L3" — so
    // neither can be the reason a Set has to be run forward to reach an instant.
    // That rule is what keeps `closed_form` an L1 question however long a chain
    // gets, and it is enforced below rather than assumed: a `deform` that
    // accumulated would be refused by `check_header`.
    if kind == Kind::L4 || kind == Kind::L2 {
        return true;
    }
    if blocks.iter().any(|b| b.kind == BlockKind::Spawn) {
        return false;
    }
    blocks.iter().all(|b| !accumulates(&b.stmts, carried))
}

/// `kill()`, or a read of an emitted attribute, anywhere under `stmts`.
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

/// A read of a carried attribute anywhere in one expression — one this
/// procedure emits, or one the engine derives for it.
///
/// The second half is not a detail. A derived attribute is per-element state
/// carried across frames exactly as an emitted one is: `age` is the clock minus
/// a stored spawn instant, `velocity` is a stored difference. A procedure
/// reading one is reading where it has been, which is what `closed_form` is
/// asking about — and the block above this one used to say that could not
/// happen, because `consumes ⊆ emit` held inside an L1. It does not any more.
pub(crate) fn reads_carried(e: &TExpr, carried: &HashSet<Attr>) -> bool {
    match &e.kind {
        // **Both sides.** A paired read is a read of the other source's carried
        // state, which is state all the same.
        TExprKind::Attr(a) | TExprKind::Far(a) => carried.contains(a),
        // **A Source slot is a constant for the whole run**, which is the
        // strongest thing that can be said about a value here: it is the
        // assigned identity of a geometry, written once into a uniform and
        // never moved. Not carried state, so it disqualifies nothing.
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
        // **A fetch reads a picture, and a picture is not carried state.** The
        // frame an L5 is handed was drawn this frame by everything upstream of
        // it; `held` is the exception in appearance only, since an L5 has no
        // per-element state for `closed_form` to be a question about — see
        // `is_closed_form`, which never asks an L5.
        TExprKind::Sample { at, .. } => at.as_ref().is_some_and(|a| reads_carried(a, carried)),
        // **The argument, and nothing behind it.** A field is a function of the
        // position it is handed and holds no state of its own, so what decides
        // this is whatever the caller computed the point from.
        TExprKind::Field { point, .. } => reads_carried(point, carried),
        TExprKind::Swizzle { value, .. } => reads_carried(value, carried),
    }
}

/// Whether the procedure reads [`Ambient::Beats`] anywhere.
///
/// Recorded because it decides what a *transport* may do to the slot, and it is
/// the one property there that the engine cannot work out for itself: a
/// procedure written against the grid already follows the room's tempo, and
/// putting it in a tempo-synced slot would make it follow twice — the slot's
/// clock scaled by the tempo, and the grid read on that scaled clock. The
/// result runs at roughly the square of the tempo ratio, which reads as a
/// broken artifact rather than as two controls doing the same job.
///
/// So this is a fact, not a judgement, and nothing is rejected for it. Unlike
/// [`is_closed_form`] there is no conservative direction to err into:
/// over-claiming greys out a control that would have worked, under-claiming
/// offers one that compounds. Both are wrong, and neither is safe, which is why
/// this walks the tree rather than approximating.
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

/// Whether the block assigns [`Output::ClipB`] anywhere, including on a path
/// coverage does not count — one arm of an `if`, or a `for` body.
///
/// Deliberately not the same question coverage asks. Mentioning `clip_b` is
/// what makes it *required*, and then coverage decides whether it was assigned
/// on every path: a procedure that writes a second endpoint under some
/// condition and not others is drawing a segment sometimes and an uninitialised
/// one the rest of the time, which is a diagnostic rather than a picture.
/// Asking only the coverage question would silently accept it as a points
/// procedure with a dead store.
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
        // The whole of what a field produces, and there is nothing optional
        // beside it — a field that assigned nothing would be a function with no
        // return value.
        BlockKind::Field => vec![CovKey::Output(Output::Distance)],
        // `point_rate` is required unconditionally here — see the module
        // docs on why the literal "when the topology is points" condition
        // cannot be evaluated from an L4 file alone. It stays required now
        // that `lines` exists, because a segment has a width for the same
        // reason a sprite has a size.
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
        // **The same output and the same requirement**, because it is the same
        // value at the next node down: an L5 writes what an L4 writes, `vec4`,
        // linear and unclamped, and a `frame` block that assigned nothing would
        // be a pass with no picture in it.
        BlockKind::Frame => vec![CovKey::Output(Output::Color)],
        // **Nothing is required of a `deform`.** An L2 rewrites some of what
        // reaches it and passes the rest through untouched — that is what makes
        // a modulator a modulator rather than a second generator, and requiring
        // it to assign everything it emits would make every one of them restate
        // the whole element.
        BlockKind::Deform => Vec::new(),
        // **`strength` is the whole of a mask**, so it is required — a block
        // that leaves it unassigned is one whose author meant to say something
        // about where the deformation applies and did not.
        BlockKind::Mask => vec![CovKey::Output(Output::Strength)],
        // **Two of the six, and the other four have defaults.** Where the
        // camera is and what it looks at are the whole of what makes one
        // camera different from another; `up`, the field of view and the two
        // planes have answers that are right far more often than not, and
        // requiring them would make the simplest camera anyone writes four
        // lines longer for nothing. The lowering writes the defaults before
        // the block runs, so an author overrides rather than restates — the
        // same shape as an L2's pass-through.
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
