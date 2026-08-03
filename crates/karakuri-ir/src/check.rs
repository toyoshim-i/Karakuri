//! Stages 2 and 3: type checking and contract checking.
//!
//! Takes the syntax tree the parser produced and returns the resolved one, or
//! every reason it could not. There is one severity: a diagnostic is an error
//! and failure means no artifact, because the response to a rejection is to
//! regenerate rather than to proceed with a caveat. Every diagnostic below is
//! collected rather than raised immediately, so a repair prompt gets every
//! problem in one pass — see `IrResult`.
//!
//! ## Design decisions the spec left to this pass
//!
//! `docs/ir-spec.md` is the authority this crate enforces, but a few rules it
//! states only apply cleanly across two procedures, or don't quite settle a
//! case a real `.kir` file hits. Each is called out at its point of use; the
//! summary:
//!
//! - **`consumes` ⊆ `emit`** is, read literally, a cross-proc rule: `emit` is
//!   declared by the L1 procedure and `consumes` by the L4 procedure that
//!   will be paired with it in a Set, and `check` only ever sees one
//!   procedure. Applying it within a single proc would reject the spec's own
//!   `soft_points` example (it consumes `position`, which `soft_points`
//!   never declares in `emit`). This pass therefore runs the subset check
//!   only when a procedure declares *both* lists itself (which only an L1
//!   procedure can meaningfully do, since only L1 has persistent per-element
//!   state to emit); an L4 procedure's `consumes` is recorded as-is and left
//!   for Set-composition time, outside this crate's scope.
//! - **No attribute derivation.** `docs/ir-spec.md` once specified deriving
//!   `velocity` from `position` and `age` from spawn time when `consumes`
//!   was not covered by `emit`; that section now lives below the "specified,
//!   not implemented" line. Neither rule has ever been backed by a WGSL
//!   emitter or by the per-element state either would need (two frames of
//!   `position` history for `velocity`, a spawn timestamp for `age`), so a
//!   `consumes` entry relying on one used to check clean and then be missing
//!   at runtime — the one failure mode "one severity" cannot tolerate,
//!   because a regenerating model gets no diagnostic to react to. This pass
//!   rejects every `consumes` entry not covered by `emit`, unconditionally.
//! - **Signal-bus names are not enumerable here.** `karakuri-signal`'s bus
//!   accepts *any* name (falling back to a zero-confidence synthesized
//!   sample), so there is no closed vocabulary to match against. Consequently
//!   every bare identifier that fails to resolve to a local, a param, an
//!   attribute, or an ambient is reported as an attempted signal-bus read
//!   with the `bind` hint — that is the only diagnosis available once the
//!   closed categories are exhausted, and it is also what the spec asks for.
//! - **`point_size` is required unconditionally in an L4 `vertex` block.**
//!   The rule as stated ("required when the source topology is points") is a
//!   property of the *paired* L1 procedure's `topology`, which an L4 file
//!   never declares and this pass never sees. Since v0.2's `Topology` enum
//!   has exactly one inhabitant (`points`), the condition is always true in
//!   practice, so this pass requires `point_size` unconditionally rather than
//!   leaving it unchecked.
//! - **Attribute names are only readable/writable when declared.** The spec
//!   states this explicitly for L4 ("Consumed attributes and seed are
//!   readable in both blocks"); it does not restate it for L1, but the
//!   parallel is structural, not stylistic — only attributes in `emit` get a
//!   buffer pair at all ("Each emitted attribute gets a pair of storage
//!   buffers"), so referencing one that is not emitted has nothing behind
//!   it. This pass applies the same rule symmetrically to L1.
//! - **Shadowing also covers stage outputs.** The spec's shadowing rule lists
//!   params, attributes, and ambients; it does not mention `clip`/
//!   `point_size`/`color`. Leaving them unprotected would let a param or
//!   local named `color` silently steal precedence over the output in an
//!   assignment target, which is exactly the class of bug the documented
//!   shadowing rule exists to prevent, so this pass extends it to outputs
//!   too.
//! - **`capacity` and `topology` are required on every L1 procedure**, and
//!   `blend` on every L4 procedure. The spec doesn't say so in as many words,
//!   but `capacity`'s range-plus-default is the only source of a value the
//!   Set format can fall back to ("`capacity` is optional [in the Set file];
//!   without it the `.kir` default applies"), so the `.kir` has to declare
//!   one; `topology` and `blend` are load-bearing for lowering in the same
//!   way.
//! - **`let`/`var` may shadow neither a param nor another local**, per the
//!   literal sentence in "Statements and expressions" ("`let`, `var`, and the
//!   loop variable may not shadow a param, an attribute, or an ambient
//!   value") and per `typed.rs`'s doc on `TStmt::Let` ("Locals do not shadow
//!   anything — not a param, not an attribute, not an ambient, and not
//!   another local"). This is stricter than the task checklist's paraphrase,
//!   which drops params from the protected set; the spec text and the seam
//!   type's own doc comment agree with each other and this pass follows
//!   them, not the paraphrase.
//!
//! ## One thing this pass decides that is not a diagnostic
//!
//! **Closed form versus accumulating.** A procedure that is a pure function of
//! `seed`, `t`, and its params can be evaluated at any `t` directly, so the
//! engine may take it Cold to Live with no priming and — the larger half — may
//! scrub it forwards, hold it, or run it backwards. That is a property of the
//! procedure, so it is decided here — where `emit`, `consumes` and cost
//! already live — and recorded on
//! [`Checked::closed_form`](crate::typed::Checked::closed_form) rather than
//! rediscovered by the engine. Nothing is rejected either way, which is exactly
//! why it has to be conservative: see [`is_closed_form`] for what it refuses to
//! claim and why under-claiming is the safe direction.

use std::collections::{HashMap, HashSet};

use crate::ast::{
    Ambient, Attr, BinOp, BlockKind, Expr, Kind, Lit, Output, Proc, Stmt, Ty, UnOp,
};
use crate::builtin::{Builtin, Domain, Shape};
use crate::error::{IrError, IrResult, Stage};
use crate::span::Span;
use crate::typed::{Checked, TBlock, TExpr, TExprKind, TStmt, Target};

/// Resolve names, type every expression, and enforce the contracts.
pub fn check(proc: &Proc) -> IrResult<Checked> {
    let mut errors = Vec::new();

    check_header(proc, &mut errors);

    let params = check_params(proc, &mut errors);
    let (emit_set, emit_vec) = dedup_attrs(&proc.emit, "emit", &mut errors);
    let (consumes_set, consumes_vec) = dedup_attrs(&proc.consumes, "consumes", &mut errors);
    check_consumes_emitted(proc.kind, &emit_set, &consumes_vec, &mut errors);

    let mut blocks = Vec::with_capacity(proc.blocks.len());
    for block in &proc.blocks {
        // A block is always checked under its own natural kind, even if it
        // does not belong in this procedure (already reported by
        // `check_header`): that keeps its internal diagnostics — ambient
        // availability, `kill()` legality, output legality — meaningful
        // instead of cascading a second, confusing error out of the
        // mismatch.
        let block_kind_owner = block.kind.kind();
        let mut checker = Checker::new(block_kind_owner, Some(block.kind), &params, &emit_set, &consumes_set);
        let stmts = checker.check_stmts(&block.stmts);
        errors.append(&mut checker.errors);

        let covered = coverage(&stmts);
        for key in required_keys(block.kind, &emit_set) {
            if !covered.contains(&key) {
                errors.push(
                    IrError::contract(
                        block.span,
                        format!(
                            "`{}` is not assigned on every path through `{}`",
                            key.name(),
                            block.kind.name()
                        ),
                    )
                    .with_hint(
                        "assign it unconditionally, or make both arms of every `if` assign it \
                         — repeated assignment is fine, coverage is what's checked",
                    ),
                );
            }
        }

        blocks.push(TBlock {
            kind: block.kind,
            stmts,
            span: block.span,
        });
    }

    if errors.is_empty() {
        // Before the move: the classifier reads the checked blocks, and
        // `blocks` is about to become the struct's.
        let closed_form = is_closed_form(proc.kind, &emit_set, &blocks);
        let reads_beats = reads_beats(&blocks);
        Ok(Checked {
            name: proc.name.clone(),
            kind: proc.kind,
            topology: proc.topology,
            capacity: proc.capacity,
            blend: proc.blend,
            params: proc.params.clone(),
            emit: emit_vec.into_iter().map(|(a, _)| a).collect(),
            consumes: consumes_vec.into_iter().map(|(a, _)| a).collect(),
            blocks,
            cost: None,
            closed_form,
            reads_beats,
            span: proc.span,
        })
    } else {
        Err(errors)
    }
}

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::L1 => "L1",
        Kind::L4 => "L4",
    }
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

fn check_header(proc: &Proc, errors: &mut Vec<IrError>) {
    match proc.kind {
        Kind::L1 => {
            match &proc.capacity {
                None => errors.push(IrError::contract(
                    proc.span,
                    "L1 procedures require a `capacity` declaration",
                )),
                // The range is what a Set is allowed to be built at, so a
                // minimum of zero says a Set of no elements is legal. It is
                // not: the compaction scan has no level pyramid to build over
                // an empty buffer, and it asserts rather than degrading. That
                // assert is inside `Set::build`, which runs on the swap
                // worker — an internal panic on a background thread, where the
                // contract calls for a diagnostic against the declaration.
                Some(cap) if cap.min == 0 => errors.push(
                    IrError::contract(
                        cap.span,
                        "`capacity` minimum must be at least 1; a Set of no elements has nothing to run",
                    )
                    .with_hint("use `[1, …]`, or a minimum the procedure actually looks right at"),
                ),
                Some(_) => {}
            }
            if proc.topology.is_none() {
                errors.push(IrError::contract(
                    proc.span,
                    "L1 procedures require a `topology` declaration",
                ));
            }
            if proc.blend.is_some() {
                errors.push(
                    IrError::contract(proc.span, "`blend` is L4 only")
                        .with_hint("remove `blend`, or change `kind` to `L4`"),
                );
            }
            if proc.blocks.iter().all(|b| b.kind != BlockKind::Element) {
                errors.push(IrError::contract(
                    proc.span,
                    "L1 procedures require an `element` block",
                ));
            }
            // "`spawn` requires a spawn rate. Declare it as a parameter named
            // `spawn_rate`" — ir-spec, "Blocks". The engine reads that one
            // param specially and has nowhere else to get a count from, so a
            // `spawn` block without it is not a procedure that spawns slowly,
            // it is one that never spawns at all: it compiles, builds a Set,
            // and renders an empty frame forever. That is exactly the shape
            // this pass exists to refuse — checking clean and then coming up
            // short at runtime.
            if let Some(spawn) = proc.block(BlockKind::Spawn) {
                match proc.spawn_rate() {
                    None => errors.push(
                        IrError::contract(spawn.span, "a `spawn` block requires a `spawn_rate` param")
                            .with_hint(
                                "add `param spawn_rate : float [0.0, 40000.0] = 8000.0` — elements \
                                 per second, which the engine reads to decide how many elements \
                                 each step creates",
                            ),
                    ),
                    Some(p) if p.ty != Ty::Float => errors.push(
                        IrError::contract(
                            p.span,
                            format!("`spawn_rate` must be a `float`, not a `{}`", p.ty.name()),
                        )
                        .with_hint("the engine reads `spawn_rate` as elements per second"),
                    ),
                    Some(_) => {}
                }
            }
        }
        Kind::L4 => {
            if let Some(cap) = &proc.capacity {
                errors.push(
                    IrError::contract(cap.span, "`capacity` is L1 only")
                        .with_hint("remove `capacity`, or change `kind` to `L1`"),
                );
            }
            if proc.topology.is_some() {
                errors.push(
                    IrError::contract(proc.span, "`topology` is L1 only")
                        .with_hint("remove `topology`, or change `kind` to `L1`"),
                );
            }
            if proc.blend.is_none() {
                errors.push(IrError::contract(
                    proc.span,
                    "L4 procedures require a `blend` declaration",
                ));
            }
            if proc.blocks.iter().all(|b| b.kind != BlockKind::Vertex) {
                errors.push(IrError::contract(
                    proc.span,
                    "L4 procedures require a `vertex` block",
                ));
            }
            if proc.blocks.iter().all(|b| b.kind != BlockKind::Fragment) {
                errors.push(IrError::contract(
                    proc.span,
                    "L4 procedures require a `fragment` block",
                ));
            }
        }
    }

    for block in &proc.blocks {
        if block.kind.kind() != proc.kind {
            errors.push(
                IrError::contract(
                    block.span,
                    format!(
                        "a `{}` block is not valid in a `{}` procedure",
                        block.kind.name(),
                        kind_name(proc.kind)
                    ),
                )
                .with_hint(format!(
                    "`{}` belongs to {}",
                    block.kind.name(),
                    kind_name(block.kind.kind())
                )),
            );
        }
    }

    let mut seen = HashSet::new();
    for block in &proc.blocks {
        if !seen.insert(block.kind) {
            errors.push(IrError::contract(
                block.span,
                format!("a `{}` block is declared more than once", block.kind.name()),
            ));
        }
    }
}

/// Attributes, ambients, and stage outputs are a closed, reserved vocabulary
/// that no param or local may take on — see the module docs on shadowing.
fn check_reserved(name: &str, span: Span, kind_of_decl: &str, errors: &mut Vec<IrError>) {
    if name == "id" {
        errors.push(
            IrError::contract(span, format!("`id` is reserved and cannot be used as a {kind_of_decl} name"))
                .with_hint("there is no `id` — element identity is `seed`"),
        );
    } else if Attr::from_name(name).is_some() {
        errors.push(
            IrError::contract(span, format!("`{name}` shadows an attribute name"))
                .with_hint("pick a different name — attributes and params share one scope"),
        );
    } else if Ambient::from_name(name).is_some() {
        errors.push(IrError::contract(span, format!("`{name}` shadows an ambient value")));
    } else if Output::from_name(name).is_some() {
        errors.push(IrError::contract(span, format!("`{name}` shadows a stage output name")));
    }
}

fn check_params(proc: &Proc, errors: &mut Vec<IrError>) -> HashMap<String, Ty> {
    let mut map = HashMap::new();
    let empty_params: HashMap<String, Ty> = HashMap::new();
    let empty_attrs: HashSet<Attr> = HashSet::new();

    for p in &proc.params {
        if !matches!(p.ty, Ty::Float | Ty::Vec2 | Ty::Vec3) {
            errors.push(
                IrError::ty(
                    p.span,
                    format!(
                        "param `{}` has type `{}`; params may only be `float`, `vec2`, or `vec3`",
                        p.name,
                        p.ty.name()
                    ),
                ),
            );
        }
        check_reserved(&p.name, p.span, "param", errors);
        if map.contains_key(&p.name) {
            errors.push(IrError::contract(
                p.span,
                format!("param `{}` is declared more than once", p.name),
            ));
        }

        // Defaults are checked in an empty scope: no locals, no other
        // params, no attributes, no ambients. A default is meant to be a
        // constant-ish value (a literal or a constructor of literals), not
        // an expression referencing the rest of the procedure.
        let mut checker = Checker::new(proc.kind, None, &empty_params, &empty_attrs, &empty_attrs);
        if let Some(v) = checker.check_expr(&p.default) {
            if v.ty != p.ty {
                errors.push(IrError::ty(
                    p.default.span(),
                    format!(
                        "param `{}` default has type `{}`, expected `{}`",
                        p.name,
                        v.ty.name(),
                        p.ty.name()
                    ),
                ));
            }
        }
        errors.append(&mut checker.errors);

        map.insert(p.name.clone(), p.ty);
    }
    map
}

/// The declaration-order list is kept alongside the set, spans and all. The
/// set answers "is this attribute declared"; the list is what anything that
/// reports or generates walks, because both need a fixed order — buffer slot
/// order comes from it, and so does the order diagnostics come out in.
fn dedup_attrs(
    list: &[(Attr, Span)],
    label: &str,
    errors: &mut Vec<IrError>,
) -> (HashSet<Attr>, Vec<(Attr, Span)>) {
    let mut set = HashSet::new();
    let mut vec = Vec::new();
    for (a, span) in list {
        if set.insert(*a) {
            vec.push((*a, *span));
        } else {
            errors.push(IrError::contract(
                *span,
                format!("`{}` is listed in `{}` more than once", a.name(), label),
            ));
        }
    }
    (set, vec)
}

/// `consumes ⊆ emit`, checked directly with no derivation step. See the
/// module docs: this only runs within a single procedure, so it is only
/// meaningful for L1 (the only kind with an `emit` of its own to check
/// `consumes` against). An L4 procedure's `consumes` is left for
/// Set-composition time.
///
/// `velocity` and `age` get a different message from every other attribute:
/// `docs/ir-spec.md` describes a derivation rule for exactly those two (see
/// "Beyond v0.2 — specified, not implemented"), so a bare "not emitted"
/// verdict would read as the spec being wrong. Naming the unimplemented rule
/// keeps a regenerating model from concluding that and instead pointing it
/// at the one fix that works today: emit the attribute.
/// Walks `consumes` in declaration order rather than as a set, for two
/// reasons: the diagnostic points at the offending name instead of at the
/// whole procedure, and two missing attributes come out in the same order on
/// every run. Set iteration order is not stable, and a diagnostic is output —
/// the same source has to produce the same diagnostics, or a regeneration
/// loop is reacting to something that reshuffles under it.
fn check_consumes_emitted(
    kind: Kind,
    emit: &HashSet<Attr>,
    consumes: &[(Attr, Span)],
    errors: &mut Vec<IrError>,
) {
    if kind != Kind::L1 {
        return;
    }
    for &(attr, span) in consumes {
        if emit.contains(&attr) {
            continue;
        }
        let hint = format!("add `{}` to `emit`", attr.name());
        // `is_derivable` names the two attributes the spec still describes a
        // derivation rule for, purely so the message below can say so — it
        // does not change the verdict, since the rule is unimplemented for
        // both. See its doc comment in `ast.rs`.
        let message = if attr.is_derivable() {
            let source = match attr {
                Attr::Velocity => "a derivation from `position`",
                Attr::Age => "a derivation from spawn time",
                _ => unreachable!("is_derivable is true for exactly Velocity and Age"),
            };
            format!(
                "`{}` is consumed but not emitted; {source} is specified in `docs/ir-spec.md` \
                 but not implemented",
                attr.name()
            )
        } else {
            format!("`{}` is consumed but not emitted", attr.name())
        };
        errors.push(IrError::contract(span, message).with_hint(hint));
    }
}

// ---------------------------------------------------------------------------
// Closed form versus accumulating.
// ---------------------------------------------------------------------------

/// Whether this procedure is a pure function of `seed`, `t`, and its params —
/// so any `t` can be evaluated directly. See
/// [`Checked::closed_form`](crate::typed::Checked::closed_form) for what that
/// buys (no priming, and — the larger half — the material can be scrubbed
/// forwards, held, or run backwards) and `docs/ir-spec.md`, "Closed form
/// versus accumulating", for the whole of it.
///
/// Three things make a procedure accumulating, and only the first is the one
/// the name suggests:
///
/// 1. **It reads an attribute it emits.** `age = age + dt` reads one;
///    `position = f(seed, t)` does not. Every read counts, wherever it is —
///    inside an `if`, inside a `for`, or bound to a `let` and used later — so
///    this walks every expression in every block rather than trying to decide
///    which reads "reach" a write. A read that reaches an emitted attribute's
///    value through a local is still a read of that attribute, and it is the
///    read this looks at, not the local.
///
///    Within an L1 procedure `consumes ⊆ emit` holds
///    (`check_consumes_emitted`), so in practice *any* attribute read in an L1
///    block is a read of an emitted one. The membership test is still written
///    out, because it is the rule the property actually names and it stays
///    correct if the two lists are ever allowed to come apart.
///
/// 2. **It has a `spawn` block.** Spawning and closed form cannot coexist, and
///    the reason is not about attributes at all: an element that does not
///    exist yet cannot be stepped, and *whether it exists* is engine state —
///    the spawn accumulator, the seed counter, the live range — accumulated
///    from every frame since the Set started. Jumping to `t = 30` on a Set that
///    spawns 8000 elements a second does not produce 240,000 elements; it
///    produces the handful of them one frame's accumulator emits, at their
///    spawn state. The population is the state that had to be warmed, and it is
///    not reachable from `seed` and `t`. So a `spawn` block disqualifies,
///    however pure the `element` block is.
///
/// 3. **It can `kill()`.** A killed element stays killed, so the live set at
///    `t` is a function of every step taken to get there and not of `t`. Even a
///    kill condition written purely in `seed` and `t` is history-dependent in
///    the direction that matters: `if t > 5.0 && t < 5.1 { kill() }` removes
///    nothing at all if `t = 6.0` is arrived at in one step.
///
/// **The conservative direction is the safe one, and this errs into it
/// deliberately.** There is no diagnostic attached to this decision — nothing
/// is rejected either way — so the only way it can be wrong is silently. Wrong
/// in the strict direction costs a warm-up that was not needed: a slot primes
/// for a few seconds it could have skipped, and cannot be scrubbed when it
/// could have been. Wrong in the permissive direction is far worse, and gets
/// worse the more the property is used for. It puts a slot on air showing an
/// unwarmed image — particles being born, an integrator at its initial
/// condition — while telling the governor it needed no warming, so nothing
/// anywhere is looking for the problem. And once transport is built on this,
/// a wrongly-claimed closed form is a scrub that produces garbage rather than
/// merely a bad first second: seeking an accumulating procedure to an
/// arbitrary `t` evaluates it once from wherever it happened to be. That is
/// the failure this whole pass exists to refuse — checking clean and then
/// coming up short at runtime. Under-claim.
fn is_closed_form(kind: Kind, emit: &HashSet<Attr>, blocks: &[TBlock]) -> bool {
    // Vacuously true for L4, and said here rather than left to fall out of an
    // empty `emit`. An L4 procedure holds no per-element state: it reads what
    // L1 wrote and throws the result at a target, so there is nothing about it
    // to warm at any `t`. Nothing rejects a stray `emit` on an L4 — it has no
    // meaning there and no buffer behind it — and without this line such a
    // procedure reads its own `emit` list in `vertex`, is called accumulating,
    // and drags a Set that needs no priming into needing it.
    if kind == Kind::L4 {
        return true;
    }
    if blocks.iter().any(|b| b.kind == BlockKind::Spawn) {
        return false;
    }
    blocks
        .iter()
        .all(|b| !accumulates(&b.stmts, emit))
}

/// `kill()`, or a read of an emitted attribute, anywhere under `stmts`.
fn accumulates(stmts: &[TStmt], emit: &HashSet<Attr>) -> bool {
    stmts.iter().any(|s| match s {
        TStmt::Kill { .. } => true,
        TStmt::Let { value, .. } | TStmt::Var { value, .. } => reads_emitted(value, emit),
        TStmt::Assign { value, .. } => reads_emitted(value, emit),
        TStmt::If { cond, then, els, .. } => {
            reads_emitted(cond, emit) || accumulates(then, emit) || accumulates(els, emit)
        }
        TStmt::For { body, .. } => accumulates(body, emit),
    })
}

/// A read of an emitted attribute anywhere in one expression.
fn reads_emitted(e: &TExpr, emit: &HashSet<Attr>) -> bool {
    match &e.kind {
        TExprKind::Attr(a) => emit.contains(a),
        TExprKind::Lit(_)
        | TExprKind::Local(_)
        | TExprKind::Param(_)
        | TExprKind::Ambient(_) => false,
        TExprKind::Unary { value, .. } => reads_emitted(value, emit),
        TExprKind::Binary { lhs, rhs, .. } => {
            reads_emitted(lhs, emit) || reads_emitted(rhs, emit)
        }
        TExprKind::Builtin { args, .. } | TExprKind::Construct { args } => {
            args.iter().any(|a| reads_emitted(a, emit))
        }
        TExprKind::Swizzle { value, .. } => reads_emitted(value, emit),
    }
}

/// **Whether the procedure reads [`Ambient::Beats`] anywhere.**
///
/// Recorded because it decides what a *transport* may do to the slot, and it is
/// the one property there that the engine cannot work out for itself: a
/// procedure written against the grid already follows the room's tempo, and
/// putting it in a tempo-synced slot would make it follow twice — the slot's
/// clock scaled by the tempo, and the grid read on that scaled clock. The
/// result runs at roughly the square of the tempo ratio, which reads as a
/// broken artifact rather than as two controls doing the same job.
///
/// So this is a **fact, not a judgement**, and nothing is rejected for it.
/// Unlike [`is_closed_form`] there is no conservative direction to err into:
/// over-claiming greys out a control that would have worked, under-claiming
/// offers one that compounds. Both are wrong, and neither is safe, which is
/// why this walks the tree rather than approximating.
fn reads_beats(blocks: &[TBlock]) -> bool {
    fn in_stmts(stmts: &[TStmt]) -> bool {
        stmts.iter().any(|s| match s {
            TStmt::Kill { .. } => false,
            TStmt::Let { value, .. } | TStmt::Var { value, .. } => in_expr(value),
            TStmt::Assign { value, .. } => in_expr(value),
            TStmt::If { cond, then, els, .. } => {
                in_expr(cond) || in_stmts(then) || in_stmts(els)
            }
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
            | TExprKind::Ambient(_) => false,
            TExprKind::Unary { value, .. } => in_expr(value),
            TExprKind::Binary { lhs, rhs, .. } => in_expr(lhs) || in_expr(rhs),
            TExprKind::Builtin { args, .. } | TExprKind::Construct { args } => {
                args.iter().any(in_expr)
            }
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
enum CovKey {
    Attr(Attr),
    Output(Output),
}

impl CovKey {
    fn name(self) -> &'static str {
        match self {
            CovKey::Attr(a) => a.name(),
            CovKey::Output(o) => o.name(),
        }
    }
}

fn required_keys(block: BlockKind, emit: &HashSet<Attr>) -> Vec<CovKey> {
    match block {
        BlockKind::Spawn | BlockKind::Element => emit.iter().map(|a| CovKey::Attr(*a)).collect(),
        // `point_size` is required unconditionally here — see the module
        // docs on why the literal "when the topology is points" condition
        // cannot be evaluated from an L4 file alone.
        BlockKind::Vertex => vec![CovKey::Output(Output::Clip), CovKey::Output(Output::PointSize)],
        BlockKind::Fragment => vec![CovKey::Output(Output::Color)],
    }
}

fn coverage(stmts: &[TStmt]) -> HashSet<CovKey> {
    covered_from(stmts, &HashSet::new())
}

fn covered_from(stmts: &[TStmt], base: &HashSet<CovKey>) -> HashSet<CovKey> {
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

// ---------------------------------------------------------------------------
// Scope: locals only. Params, attributes, and ambients are looked up through
// the `Checker` directly since they are proc- or block-scoped, not nested.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct LocalInfo {
    ty: Ty,
    mutable: bool,
}

struct Scope {
    frames: Vec<HashMap<String, LocalInfo>>,
}

impl Scope {
    fn new() -> Scope {
        Scope {
            frames: vec![HashMap::new()],
        }
    }

    fn push(&mut self) {
        self.frames.push(HashMap::new());
    }

    fn pop(&mut self) {
        self.frames.pop();
    }

    fn lookup(&self, name: &str) -> Option<LocalInfo> {
        self.frames.iter().rev().find_map(|f| f.get(name).copied())
    }

    fn declare(&mut self, name: String, info: LocalInfo) {
        self.frames.last_mut().expect("at least one frame").insert(name, info);
    }
}

// ---------------------------------------------------------------------------
// Checker: expression and statement checking for one block (or, with
// `block: None`, one header expression such as a param default).
// ---------------------------------------------------------------------------

struct Checker<'a> {
    kind: Kind,
    block: Option<BlockKind>,
    params: &'a HashMap<String, Ty>,
    emit: &'a HashSet<Attr>,
    consumes: &'a HashSet<Attr>,
    scope: Scope,
    errors: Vec<IrError>,
}

enum TargetRes {
    Local(String, Ty, bool),
    Attr(Attr),
    Output(Output),
    /// Already reported; caller should not report again.
    Invalid,
}

impl<'a> Checker<'a> {
    fn new(
        kind: Kind,
        block: Option<BlockKind>,
        params: &'a HashMap<String, Ty>,
        emit: &'a HashSet<Attr>,
        consumes: &'a HashSet<Attr>,
    ) -> Checker<'a> {
        Checker {
            kind,
            block,
            params,
            emit,
            consumes,
            scope: Scope::new(),
            errors: Vec::new(),
        }
    }

    fn err(&mut self, stage: Stage, span: Span, msg: impl Into<String>) {
        self.errors.push(IrError::new(stage, span, msg));
    }

    fn err_hint(&mut self, stage: Stage, span: Span, msg: impl Into<String>, hint: impl Into<String>) {
        self.errors.push(IrError::new(stage, span, msg).with_hint(hint));
    }

    // -- statements ---------------------------------------------------------

    fn check_stmts(&mut self, stmts: &[Stmt]) -> Vec<TStmt> {
        stmts.iter().filter_map(|s| self.check_stmt(s)).collect()
    }

    fn check_stmt(&mut self, s: &Stmt) -> Option<TStmt> {
        match s {
            Stmt::Let { name, value, span } => self.check_let(name, value, *span, false),
            Stmt::Var { name, value, span } => self.check_let(name, value, *span, true),
            Stmt::Assign {
                target,
                op,
                value,
                span,
            } => self.check_assign(target, *op, value, *span),
            Stmt::If { cond, then, els, span } => self.check_if(cond, then, els, *span),
            Stmt::For {
                var,
                start,
                end,
                body,
                span,
            } => self.check_for(var, *start, *end, body, *span),
            Stmt::Kill { span } => self.check_kill(*span),
        }
    }

    fn check_let(&mut self, name: &str, value: &Expr, span: Span, mutable: bool) -> Option<TStmt> {
        let value_t = self.check_expr(value);
        self.check_declarable_name(name, span);
        let value_t = value_t?;
        self.scope.declare(
            name.to_string(),
            LocalInfo {
                ty: value_t.ty,
                mutable,
            },
        );
        if mutable {
            Some(TStmt::Var {
                name: name.to_string(),
                value: value_t,
                span,
            })
        } else {
            Some(TStmt::Let {
                name: name.to_string(),
                value: value_t,
                span,
            })
        }
    }

    /// `let`, `var`, and the loop variable may not shadow `id`, an
    /// attribute, an ambient, a stage output, a param, or another local —
    /// see the module docs on why this is stricter than the task checklist's
    /// paraphrase.
    fn check_declarable_name(&mut self, name: &str, span: Span) {
        if name == "id" {
            self.err_hint(
                Stage::Contract,
                span,
                "`id` is reserved and cannot be used as a name",
                "there is no `id` — element identity is `seed`",
            );
            return;
        }
        if Attr::from_name(name).is_some() {
            self.err(Stage::Contract, span, format!("`{name}` shadows an attribute name"));
            return;
        }
        if Ambient::from_name(name).is_some() {
            self.err(Stage::Contract, span, format!("`{name}` shadows an ambient value"));
            return;
        }
        if Output::from_name(name).is_some() {
            self.err(Stage::Contract, span, format!("`{name}` shadows a stage output name"));
            return;
        }
        if self.params.contains_key(name) {
            self.err(Stage::Contract, span, format!("`{name}` shadows a param"));
            return;
        }
        if self.scope.lookup(name).is_some() {
            self.err(Stage::Contract, span, format!("`{name}` shadows another local"));
        }
    }

    fn resolve_target(&mut self, name: &str, span: Span) -> TargetRes {
        if let Some(info) = self.scope.lookup(name) {
            return TargetRes::Local(name.to_string(), info.ty, info.mutable);
        }
        if self.params.contains_key(name) {
            self.err(
                Stage::Type,
                span,
                format!("cannot assign to param `{name}`; params are read-only"),
            );
            return TargetRes::Invalid;
        }
        if let Some(attr) = Attr::from_name(name) {
            let available = matches!(self.block, Some(BlockKind::Spawn) | Some(BlockKind::Element))
                && self.emit.contains(&attr);
            if !available {
                let hint = match self.block {
                    Some(BlockKind::Vertex) | Some(BlockKind::Fragment) => {
                        "attributes are read-only in vertex/fragment blocks — assign a stage \
                         output instead"
                            .to_string()
                    }
                    _ => format!("add `{name}` to `emit` to make it writable here"),
                };
                self.err_hint(
                    Stage::Contract,
                    span,
                    format!("attribute `{name}` is not writable here"),
                    hint,
                );
                return TargetRes::Invalid;
            }
            return TargetRes::Attr(attr);
        }
        if Ambient::from_name(name).is_some() {
            self.err(
                Stage::Type,
                span,
                format!("cannot assign to `{name}`; it is an ambient value"),
            );
            return TargetRes::Invalid;
        }
        if let Some(output) = Output::from_name(name) {
            if self.block != Some(output.block()) {
                self.err_hint(
                    Stage::Contract,
                    span,
                    format!("`{name}` belongs to the `{}` block", output.block().name()),
                    format!("write `{name}` inside `{}`, not here", output.block().name()),
                );
                return TargetRes::Invalid;
            }
            return TargetRes::Output(output);
        }
        self.err_hint(
            Stage::Type,
            span,
            format!("assigning to `{name}`, which was never declared"),
            "assigning to a name that was never declared is an error, not a declaration — \
             declare it first with `let` or `var`",
        );
        TargetRes::Invalid
    }

    fn check_assign(&mut self, target: &str, op: Option<BinOp>, value: &Expr, span: Span) -> Option<TStmt> {
        let value_t = self.check_expr(value);
        let res = self.resolve_target(target, span);

        if op.is_some() {
            match res {
                TargetRes::Attr(_) => {
                    self.err_hint(
                        Stage::Type,
                        span,
                        format!("compound assignment is not allowed on attribute `{target}`"),
                        format!(
                            "on an attribute this reads like accumulation but actually re-reads \
                             the previous frame's value every time — write `let v = ...;` then \
                             `{target} = v;`"
                        ),
                    );
                    return None;
                }
                TargetRes::Output(_) => {
                    self.err(
                        Stage::Type,
                        span,
                        format!("compound assignment is not allowed on stage output `{target}`"),
                    );
                    return None;
                }
                _ => {}
            }
        }

        match res {
            TargetRes::Invalid => None,
            TargetRes::Local(name, ty, mutable) => {
                if !mutable {
                    self.err_hint(
                        Stage::Type,
                        span,
                        format!("cannot assign to `{name}`; `let` bindings are immutable"),
                        "declare it with `var` instead if it needs to change",
                    );
                    return None;
                }
                let value_t = value_t?;
                let final_v = match op {
                    None => value_t,
                    Some(binop) => {
                        let lhs = TExpr::new(ty, span, TExprKind::Local(name.clone()));
                        self.desugar_compound(binop, lhs, value_t, span)?
                    }
                };
                if final_v.ty != ty {
                    self.err(
                        Stage::Type,
                        span,
                        format!(
                            "cannot assign `{}` to `{name}`, which has type `{}`",
                            final_v.ty.name(),
                            ty.name()
                        ),
                    );
                    return None;
                }
                Some(TStmt::Assign {
                    target: Target::Local(name),
                    value: final_v,
                    span,
                })
            }
            TargetRes::Attr(attr) => {
                let value_t = value_t?;
                if value_t.ty != attr.ty() {
                    self.err(
                        Stage::Type,
                        span,
                        format!(
                            "cannot assign `{}` to attribute `{}`, which has type `{}`",
                            value_t.ty.name(),
                            attr.name(),
                            attr.ty().name()
                        ),
                    );
                    return None;
                }
                Some(TStmt::Assign {
                    target: Target::Attr(attr),
                    value: value_t,
                    span,
                })
            }
            TargetRes::Output(output) => {
                let value_t = value_t?;
                if value_t.ty != output.ty() {
                    self.err(
                        Stage::Type,
                        span,
                        format!(
                            "cannot assign `{}` to `{}`, which has type `{}`",
                            value_t.ty.name(),
                            output.name(),
                            output.ty().name()
                        ),
                    );
                    return None;
                }
                Some(TStmt::Assign {
                    target: Target::Output(output),
                    value: value_t,
                    span,
                })
            }
        }
    }

    fn check_if(&mut self, cond: &Expr, then: &[Stmt], els: &[Stmt], span: Span) -> Option<TStmt> {
        let cond_t = self.check_expr(cond);
        if let Some(c) = &cond_t {
            if c.ty != Ty::Bool {
                self.err(
                    Stage::Type,
                    c.span,
                    format!("`if` condition must be `bool`, found `{}`", c.ty.name()),
                );
            }
        }
        self.scope.push();
        let then_t = self.check_stmts(then);
        self.scope.pop();
        self.scope.push();
        let els_t = self.check_stmts(els);
        self.scope.pop();

        let cond_t = cond_t.unwrap_or_else(|| poison(Ty::Bool, cond.span()));
        Some(TStmt::If {
            cond: cond_t,
            then: then_t,
            els: els_t,
            span,
        })
    }

    fn check_for(&mut self, var: &str, start: i32, end: i32, body: &[Stmt], span: Span) -> Option<TStmt> {
        self.scope.push();
        self.check_declarable_name(var, span);
        self.scope.declare(
            var.to_string(),
            LocalInfo {
                ty: Ty::Int,
                mutable: false,
            },
        );
        let body_t = self.check_stmts(body);
        self.scope.pop();
        Some(TStmt::For {
            var: var.to_string(),
            start,
            end,
            body: body_t,
            span,
        })
    }

    fn check_kill(&mut self, span: Span) -> Option<TStmt> {
        if !(self.kind == Kind::L1 && self.block == Some(BlockKind::Element)) {
            self.err_hint(
                Stage::Contract,
                span,
                "`kill()` is only legal in an L1 `element` block",
                "remove it, or move this logic into the `element` block",
            );
        }
        Some(TStmt::Kill { span })
    }

    // -- expressions ----------------------------------------------------------

    fn check_expr(&mut self, e: &Expr) -> Option<TExpr> {
        match e {
            Expr::Lit { value, span } => Some(TExpr::new(value.ty(), *span, TExprKind::Lit(*value))),
            Expr::Ident { name, span } => self.check_ident(name, *span),
            Expr::Unary { op, value, span } => {
                let v = self.check_expr(value)?;
                let ty = self.check_unary(*op, v.ty, *span)?;
                Some(TExpr::new(
                    ty,
                    *span,
                    TExprKind::Unary {
                        op: *op,
                        value: Box::new(v),
                    },
                ))
            }
            Expr::Binary { op, lhs, rhs, span } => {
                let l = self.check_expr(lhs);
                let r = self.check_expr(rhs);
                let (l, r) = (l?, r?);
                self.desugar_compound(*op, l, r, *span)
            }
            Expr::Call { name, args, span } => self.check_call(name, args, *span),
            Expr::Swizzle { value, components, span } => self.check_swizzle(value, components, *span),
        }
    }

    fn check_ident(&mut self, name: &str, span: Span) -> Option<TExpr> {
        if let Some(info) = self.scope.lookup(name) {
            return Some(TExpr::new(info.ty, span, TExprKind::Local(name.to_string())));
        }
        if let Some(ty) = self.params.get(name) {
            return Some(TExpr::new(*ty, span, TExprKind::Param(name.to_string())));
        }
        if let Some(attr) = Attr::from_name(name) {
            let available = match self.block {
                Some(BlockKind::Spawn) | Some(BlockKind::Element) => {
                    self.emit.contains(&attr) || self.consumes.contains(&attr)
                }
                Some(BlockKind::Vertex) | Some(BlockKind::Fragment) => self.consumes.contains(&attr),
                None => false,
            };
            if available {
                return Some(TExpr::new(attr.ty(), span, TExprKind::Attr(attr)));
            }
            let hint = match self.block {
                Some(BlockKind::Vertex) | Some(BlockKind::Fragment) => {
                    format!("add `{name}` to `consumes` to read it here")
                }
                Some(BlockKind::Spawn) | Some(BlockKind::Element) => {
                    format!("add `{name}` to `emit` to read it here")
                }
                None => "attributes are not available in a header expression".to_string(),
            };
            self.err_hint(
                Stage::Contract,
                span,
                format!("attribute `{name}` is not available here"),
                hint,
            );
            return None;
        }
        if let Some(ambient) = Ambient::from_name(name) {
            let available = match self.block {
                Some(block) => ambient.available_in(self.kind, block),
                None => false,
            };
            if available {
                return Some(TExpr::new(ambient.ty(), span, TExprKind::Ambient(ambient)));
            }
            self.err(
                Stage::Contract,
                span,
                format!("`{name}` is not available in this block"),
            );
            return None;
        }
        if Output::from_name(name).is_some() {
            self.err_hint(
                Stage::Contract,
                span,
                format!("reading `{name}` is not allowed; stage outputs are write-only"),
                "bind a `let` to the value you need before writing it, and read the `let` instead",
            );
            return None;
        }

        // Truly unresolved. The signal bus accepts any name (see the module
        // docs), so this is the only diagnosis available: either the author
        // meant to read a signal directly, which IR cannot do, or made a
        // typo, which the hint below will not fit as well but does not
        // actively mislead either.
        if self.block.is_some() {
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{name}` does not resolve to a local, a param, an attribute, or an ambient value"),
                format!(
                    "the signal bus is not readable from IR — declare `param {name}` and attach \
                     a `bind` record to it in the Set file"
                ),
            );
        } else {
            self.err(Stage::Type, span, format!("`{name}` is undefined"));
        }
        None
    }

    fn check_unary(&mut self, op: UnOp, ty: Ty, span: Span) -> Option<Ty> {
        match op {
            UnOp::Neg => {
                if matches!(ty, Ty::Float | Ty::Int | Ty::Vec2 | Ty::Vec3 | Ty::Vec4) {
                    Some(ty)
                } else {
                    self.err(Stage::Type, span, format!("`-` is not defined for `{}`", ty.name()));
                    None
                }
            }
            UnOp::Not => {
                if ty == Ty::Bool {
                    Some(Ty::Bool)
                } else {
                    self.err(
                        Stage::Type,
                        span,
                        format!("`!` requires `bool`, found `{}`", ty.name()),
                    );
                    None
                }
            }
        }
    }

    fn desugar_compound(&mut self, op: BinOp, lhs: TExpr, rhs: TExpr, span: Span) -> Option<TExpr> {
        let ty = self.binary_ty(op, lhs.ty, rhs.ty, span)?;
        Some(TExpr::new(
            ty,
            span,
            TExprKind::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
        ))
    }

    fn binary_ty(&mut self, op: BinOp, lhs: Ty, rhs: Ty, span: Span) -> Option<Ty> {
        if op.is_logical() {
            if lhs == Ty::Bool && rhs == Ty::Bool {
                return Some(Ty::Bool);
            }
            self.err(
                Stage::Type,
                span,
                format!(
                    "`{}` requires `bool` operands, found `{}` and `{}`",
                    op_symbol(op),
                    lhs.name(),
                    rhs.name()
                ),
            );
            return None;
        }
        if op.is_comparison() {
            let ok = if matches!(op, BinOp::Eq | BinOp::Ne) {
                lhs == rhs
            } else {
                lhs == rhs && matches!(lhs, Ty::Float | Ty::Int | Ty::Uint)
            };
            if ok {
                return Some(Ty::Bool);
            }
            self.err(
                Stage::Type,
                span,
                format!(
                    "`{}` is not defined for `{}` and `{}`",
                    op_symbol(op),
                    lhs.name(),
                    rhs.name()
                ),
            );
            return None;
        }

        // Arithmetic: same type, or `float`/vector broadcast, or the
        // `camera * vecN` special case (matrices have no other operation).
        if lhs == rhs && matches!(lhs, Ty::Float | Ty::Int | Ty::Uint | Ty::Vec2 | Ty::Vec3 | Ty::Vec4) {
            return Some(lhs);
        }
        match (lhs, rhs) {
            (v, Ty::Float) | (Ty::Float, v) if matches!(v, Ty::Vec2 | Ty::Vec3 | Ty::Vec4) => {
                return Some(v);
            }
            (Ty::Mat4, Ty::Vec4) if op == BinOp::Mul => return Some(Ty::Vec4),
            (Ty::Mat3, Ty::Vec3) if op == BinOp::Mul => return Some(Ty::Vec3),
            _ => {}
        }
        self.err(
            Stage::Type,
            span,
            format!(
                "`{}` is not defined for `{}` and `{}` — there are no implicit conversions",
                op_symbol(op),
                lhs.name(),
                rhs.name()
            ),
        );
        None
    }

    fn check_call(&mut self, name: &str, args: &[Expr], span: Span) -> Option<TExpr> {
        if let Some(builtin) = Builtin::from_name(name) {
            return self.check_builtin_call(builtin, args, span);
        }
        if let Some(ty) = Ty::from_name(name) {
            return self.check_constructor(ty, args, span);
        }
        self.err(
            Stage::Type,
            span,
            format!("`{name}` is not a builtin function or a type constructor"),
        );
        for a in args {
            self.check_expr(a);
        }
        None
    }

    fn check_builtin_call(&mut self, b: Builtin, args_ast: &[Expr], span: Span) -> Option<TExpr> {
        let sig = b.signature();
        if args_ast.len() != sig.args.len() {
            self.err(
                Stage::Type,
                span,
                format!(
                    "`{}` takes {} argument(s), found {}",
                    b.name(),
                    sig.args.len(),
                    args_ast.len()
                ),
            );
            for a in args_ast {
                self.check_expr(a);
            }
            return None;
        }

        for &i in sig.const_args {
            if let Some(a) = args_ast.get(i) {
                if !matches!(a, Expr::Lit { value: Lit::Int(_), .. }) {
                    self.err_hint(
                        Stage::Type,
                        a.span(),
                        format!("argument {} to `{}` must be a literal integer", i + 1, b.name()),
                        "octave counts are unrolled at lowering time, so they cannot be a \
                         runtime value",
                    );
                }
            }
        }

        let mut checked_args = Vec::with_capacity(args_ast.len());
        let mut ok = true;
        for a in args_ast {
            match self.check_expr(a) {
                Some(t) => checked_args.push(t),
                None => ok = false,
            }
        }
        if !ok {
            return None;
        }

        let mut same_ty: Option<Ty> = None;
        for (shape, actual) in sig.args.iter().zip(checked_args.iter()) {
            match shape {
                Shape::Exact(t) => {
                    if actual.ty != *t {
                        self.err(
                            Stage::Type,
                            actual.span,
                            format!("`{}` expects `{}`, found `{}`", b.name(), t.name(), actual.ty.name()),
                        );
                        ok = false;
                    }
                }
                Shape::Same => {
                    if !domain_allows(sig.domain, actual.ty) {
                        self.err(
                            Stage::Type,
                            actual.span,
                            format!("`{}` does not accept `{}`", b.name(), actual.ty.name()),
                        );
                        ok = false;
                    } else {
                        match same_ty {
                            None => same_ty = Some(actual.ty),
                            Some(t) if t == actual.ty => {}
                            Some(t) => {
                                self.err(
                                    Stage::Type,
                                    actual.span,
                                    format!(
                                        "`{}` expects every argument to be the same type; found \
                                         `{}` and `{}`",
                                        b.name(),
                                        t.name(),
                                        actual.ty.name()
                                    ),
                                );
                                ok = false;
                            }
                        }
                    }
                }
                Shape::Scalar => {
                    // No builtin puts `Scalar` in an argument position today
                    // (it only ever appears as `ret`); nothing to unify.
                }
            }
        }
        if !ok {
            return None;
        }

        let ret_ty = match sig.ret {
            Shape::Exact(t) => t,
            Shape::Same => same_ty.unwrap_or(Ty::Float),
            Shape::Scalar => Ty::Float,
        };
        Some(TExpr::new(
            ret_ty,
            span,
            TExprKind::Builtin {
                func: b,
                args: checked_args,
            },
        ))
    }

    fn check_constructor(&mut self, ty: Ty, args_ast: &[Expr], span: Span) -> Option<TExpr> {
        match ty {
            Ty::Float | Ty::Int | Ty::Uint => {
                if args_ast.len() != 1 {
                    self.err(
                        Stage::Type,
                        span,
                        format!("`{}(...)` takes exactly one argument", ty.name()),
                    );
                    for a in args_ast {
                        self.check_expr(a);
                    }
                    return None;
                }
                let a = self.check_expr(&args_ast[0])?;
                if !matches!(a.ty, Ty::Float | Ty::Int | Ty::Uint) {
                    self.err(
                        Stage::Type,
                        a.span,
                        format!("`{}(...)` converts a scalar number, found `{}`", ty.name(), a.ty.name()),
                    );
                    return None;
                }
                Some(TExpr::new(ty, span, TExprKind::Construct { args: vec![a] }))
            }
            Ty::Vec2 | Ty::Vec3 | Ty::Vec4 => {
                let n = ty.components().expect("vector has components") as usize;

                // Single-scalar broadcast: `vec3(0.0)`. Checked as its own
                // case because it is the one shape the general component-sum
                // rule below cannot express (a lone `float` sums to 1, not
                // `n`, for every `n` this branch handles).
                if args_ast.len() == 1 {
                    if let Some(a) = self.check_expr(&args_ast[0]) {
                        if a.ty == Ty::Float {
                            return Some(TExpr::new(ty, span, TExprKind::Construct { args: vec![a] }));
                        }
                        // Not a scalar: fall through to the general rule,
                        // which will accept it if it happens to already be
                        // exactly `n` components (e.g. a redundant
                        // `vec3(some_vec3)`) and reject it otherwise.
                        return self.check_constructor_concat(ty, n, vec![a], span);
                    }
                    return None;
                }

                // General rule: any mix of `float`/`vec2`/`vec3`/`vec4`
                // arguments whose component counts sum to exactly `n` —
                // e.g. `vec4(position, 1.0)` concatenates a `vec3` and a
                // `float`. This is GLSL's actual vector-constructor rule;
                // the spec's prose ("one scalar per component, or a single
                // scalar to broadcast") undersells it — the spec's own
                // `soft_points` example uses `vec4(position, 1.0)`, which
                // only this broader rule accepts. See the module docs.
                let mut checked = Vec::with_capacity(args_ast.len());
                let mut ok = true;
                for a in args_ast {
                    match self.check_expr(a) {
                        Some(t) => checked.push(t),
                        None => ok = false,
                    }
                }
                if !ok {
                    return None;
                }
                self.check_constructor_concat(ty, n, checked, span)
            }
            Ty::Bool | Ty::Mat3 | Ty::Mat4 => {
                self.err(Stage::Type, span, format!("`{}` cannot be constructed", ty.name()));
                for a in args_ast {
                    self.check_expr(a);
                }
                None
            }
        }
    }

    /// The shared tail of vector construction: every argument must be
    /// `float`/`vec2`/`vec3`/`vec4`, and their component counts must sum to
    /// exactly `n`.
    fn check_constructor_concat(&mut self, ty: Ty, n: usize, args: Vec<TExpr>, span: Span) -> Option<TExpr> {
        let mut total = 0usize;
        let mut ok = true;
        for a in &args {
            match a.ty.components() {
                Some(c) if matches!(a.ty, Ty::Float | Ty::Vec2 | Ty::Vec3 | Ty::Vec4) => total += c as usize,
                _ => {
                    self.err(
                        Stage::Type,
                        a.span,
                        format!(
                            "`{}(...)` arguments must be `float` or a vector, found `{}`",
                            ty.name(),
                            a.ty.name()
                        ),
                    );
                    ok = false;
                }
            }
        }
        if !ok {
            return None;
        }
        if total != n {
            self.err_hint(
                Stage::Type,
                span,
                format!(
                    "`{}(...)` arguments have {} component(s) total, expected {}",
                    ty.name(),
                    total,
                    n
                ),
                format!(
                    "e.g. `{0}(1.0, 0.0, 0.0)`, `{0}(0.0)` to broadcast, or `{0}(v, 1.0)` to \
                     extend a smaller vector",
                    ty.name()
                ),
            );
            return None;
        }
        Some(TExpr::new(ty, span, TExprKind::Construct { args }))
    }

    fn check_swizzle(&mut self, value: &Expr, components: &str, span: Span) -> Option<TExpr> {
        let v = self.check_expr(value)?;
        let width = match v.ty {
            Ty::Vec2 => 2u8,
            Ty::Vec3 => 3,
            Ty::Vec4 => 4,
            other => {
                self.err(
                    Stage::Type,
                    span,
                    format!("cannot swizzle `{}`; only vectors have components", other.name()),
                );
                return None;
            }
        };
        if components.is_empty() || components.len() > 4 {
            self.err(
                Stage::Type,
                span,
                format!("swizzle must have 1 to 4 components, found {}", components.len()),
            );
            return None;
        }

        let mut idx = Vec::with_capacity(components.len());
        let mut ok = true;
        for c in components.chars() {
            let i = match c {
                'x' => 0u8,
                'y' => 1,
                'z' => 2,
                'w' => 3,
                _ => {
                    self.err(
                        Stage::Type,
                        span,
                        format!("`{c}` is not a valid swizzle component; use `x`, `y`, `z`, or `w`"),
                    );
                    ok = false;
                    continue;
                }
            };
            if i >= width {
                self.err_hint(
                    Stage::Type,
                    span,
                    format!("`.{c}` is out of range for a {width}-component vector"),
                    format!("`{}` only has components `{}`", v.ty.name(), &"xyzw"[..width as usize]),
                );
                ok = false;
                continue;
            }
            idx.push(i);
        }
        if !ok {
            return None;
        }

        let ty = match idx.len() {
            1 => Ty::Float,
            2 => Ty::Vec2,
            3 => Ty::Vec3,
            4 => Ty::Vec4,
            _ => unreachable!("checked above"),
        };
        Some(TExpr::new(
            ty,
            span,
            TExprKind::Swizzle {
                value: Box::new(v),
                components: idx,
            },
        ))
    }
}

fn domain_allows(d: Domain, ty: Ty) -> bool {
    match d {
        Domain::None => false,
        Domain::FloatOrVector => matches!(ty, Ty::Float | Ty::Vec2 | Ty::Vec3 | Ty::Vec4),
        Domain::VectorOnly => matches!(ty, Ty::Vec2 | Ty::Vec3 | Ty::Vec4),
    }
}

fn op_symbol(op: BinOp) -> &'static str {
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

/// A well-typed placeholder used only for continued error recovery. The tree
/// it lands in is discarded whenever any error was recorded — `check` never
/// returns `Ok` otherwise — so this only has to keep `check_stmts` from
/// panicking, not represent anything meaningful.
fn poison(ty: Ty, span: Span) -> TExpr {
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
