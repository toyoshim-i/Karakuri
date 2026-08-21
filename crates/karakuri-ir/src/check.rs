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
//! - **Attribute derivation splits the `consumes ⊆ emit` check in two, and
//!   this pass keeps only the half one file can answer.** `age` and
//!   `velocity` are synthesised where nothing emits them: the engine gives
//!   `age` a `birth_t` slot and `velocity` a slot the L1 writes, so an L1
//!   consuming either without emitting it is asking for something the Set
//!   will provide rather than making a contradiction. What stays here is the
//!   part that is genuinely local — `velocity` is derived *from* `position`,
//!   and whether this procedure emits `position` is a one-file question. See
//!   [`check_consumes_emitted`]. Whether anybody at all emits a consumed
//!   attribute is `Set::build_many`'s check, at the first point that holds
//!   every procedure at once.
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
//!   never declares and this pass never sees. It stays unconditional now that
//!   `lines` exists, because it means something under both: a sprite's extent
//!   and a stroke's width are the same number in the same units.
//! - **What an L4 draws is inferred here, not declared.** Assigning
//!   [`Output::ClipB`](crate::ast::Output::ClipB) — a segment's far end — is
//!   the only thing that could make a procedure a line renderer, so this pass
//!   reads it off the `vertex` block and records it in
//!   [`Checked::topology`](crate::typed::Checked::topology), which is
//!   otherwise an L1 field. Whether it agrees with the L1 it is paired with is
//!   **not checked anywhere**, and deliberately: a segment gets both of its
//!   ends from attributes the L4 consumes, so a renderer needs nothing of the
//!   geometry beyond what the `emit`/`consumes` check at Set composition
//!   already covers. Requiring agreement would forbid one L1 being drawn as
//!   sprites by one L4 and as strokes by another.
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
    Ambient, Attr, BinOp, BlockKind, Expr, Kind, Lit, Output, Proc, SlotTy, Stmt, Topology, Ty,
    UnOp,
};
use crate::builtin::{Builtin, Domain, Shape};
use crate::error::{IrError, IrResult, Stage};
use crate::span::Span;
use crate::typed::{Checked, Slot, TBlock, TExpr, TExprKind, TStmt, Target};

/// Resolve names, type every expression, and enforce the contracts.
pub fn check(proc: &Proc) -> IrResult<Checked> {
    let mut errors = Vec::new();

    check_header(proc, &mut errors);

    let params = check_params(proc, &mut errors);
    let (emit_set, emit_vec) = dedup_attrs(&proc.emit, "emit", &mut errors);
    let (consumes_set, consumes_vec) = dedup_attrs(&proc.consumes, "consumes", &mut errors);
    check_consumes_emitted(proc.kind, &emit_set, &consumes_vec, &mut errors);

    // **What makes an L4 a marcher**, and the one procedure-wide fact a block
    // checker needs: `eye` and `ray` are defined by the ray prologue a
    // fullscreen fragment stage opens with, and by nothing else.
    //
    // The declared slot name is the second such fact: a `deform` writing
    // `far.position` is reading a geometry the *header* named, and a block does
    // not see its own header. The geometry one specifically — `far.position` is
    // a read of an element, and a slot of some other type has no element in it.
    let uses = proc
        .uses
        .iter()
        .find(|u| u.ty == SlotTy::Geometry)
        .map(|u| u.name.as_str());
    // **The third such fact, and the one that makes a name callable.** A Field
    // slot is read as `shape(p)`, so what the header declared has to reach the
    // expression checker or the call resolves against the builtin table and
    // finds nothing.
    //
    // A list where the geometry one is an option, because the two arities
    // differ: there is at most one second element buffer per node, and a
    // marcher wanting a shape and a cutter is the ordinary case.
    let fields: Vec<&str> = proc
        .uses
        .iter()
        .filter(|u| u.ty == SlotTy::Field)
        .map(|u| u.name.as_str())
        .collect();
    // **The fourth, and the one that makes a name a value with parts.** A
    // Camera slot is read as `view.clip`, which arrives at the swizzle checker
    // shaped exactly like `far.position` — so the header has to reach it or the
    // read resolves against the attribute table and is refused for not being an
    // attribute, which is a sentence about the wrong thing.
    //
    // An option where the field one is a list, and for the reason the geometry
    // one is: a renderer draws one picture and a picture is seen from one
    // place. `check_header` is where that arity is refused.
    let camera = proc
        .uses
        .iter()
        .find(|u| u.ty == SlotTy::Camera)
        .map(|u| u.name.as_str());
    let fullscreen =
        proc.kind == Kind::L4 && proc.blocks.iter().all(|b| b.kind != BlockKind::Vertex);
    let mut blocks = Vec::with_capacity(proc.blocks.len());
    for block in &proc.blocks {
        // A block is always checked under its own natural kind, even if it
        // does not belong in this procedure (already reported by
        // `check_header`): that keeps its internal diagnostics — ambient
        // availability, `kill()` legality, output legality — meaningful
        // instead of cascading a second, confusing error out of the
        // mismatch.
        let block_kind_owner = block.kind.kind();
        let mut checker = Checker::new(
            block_kind_owner,
            Some(block.kind),
            fullscreen,
            uses,
            &fields,
            camera,
            &params,
            &emit_set,
            &consumes_set,
        );
        let stmts = checker.check_stmts(&block.stmts);
        errors.append(&mut checker.errors);

        let covered = coverage(&stmts);
        for key in required_keys(block.kind, &emit_set, assigns_clip_b(&stmts)) {
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
        let carried: HashSet<Attr> = emit_set
            .iter()
            .copied()
            .chain(
                consumes_vec
                    .iter()
                    .map(|(a, _)| *a)
                    .filter(|a| a.is_derivable()),
            )
            .collect();
        let closed_form = is_closed_form(proc.kind, &carried, &blocks);
        let reads_beats = reads_beats(&blocks);
        Ok(Checked {
            name: proc.name.clone(),
            kind: proc.kind,
            // Declared on an L1, inferred on an L4 — see `Output::ClipB` and
            // `Checked::topology`. An L4 that declared one was rejected by
            // `check_header`, so this never overwrites something a file said.
            topology: match proc.kind {
                Kind::L1 => proc.topology,
                // An L2 draws nothing, so it reads as nothing. What the
                // geometry is *meant to read as* stays the L1's declaration all
                // the way down a chain — a deformation moves elements about and
                // does not turn a cloud into strands.
                Kind::L2 => None,
                // An L3 has no geometry at all — it produces a viewpoint.
                Kind::L3 => None,
                Kind::L4 => Some(drawn_topology(&blocks)),
                // A field is a function of space and has no elements at all.
                Kind::Field => None,
            },
            capacity: proc.capacity,
            amplify: proc.amplify.map(|a| a.factor),
            uses: proc
                .uses
                .iter()
                .map(|u| Slot {
                    name: u.name.clone(),
                    ty: u.ty,
                })
                .collect(),
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

/// What an L4 procedure draws, read off its `vertex` block: a second endpoint
/// means a segment, and there is nothing else it could mean.
///
/// A vertex block is required of every L4 and its absence was already reported,
/// so a procedure with none falls back to `points` rather than being given a
/// second diagnostic about a block it does not have.
fn drawn_topology(blocks: &[TBlock]) -> Topology {
    let Some(vertex) = blocks.iter().find(|b| b.kind == BlockKind::Vertex) else {
        // No per-element position to compute, so nothing per element to draw:
        // the frame, once. See `Topology::Fullscreen`.
        return Topology::Fullscreen;
    };
    if assigns_clip_b(&vertex.stmts) {
        Topology::Lines
    } else {
        Topology::Points
    }
}

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::L1 => "L1",
        Kind::L2 => "L2",
        Kind::L3 => "L3",
        Kind::L4 => "L4",
        Kind::Field => "Field",
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
            match proc.topology {
                None => errors.push(IrError::contract(
                    proc.span,
                    "L1 procedures require a `topology` declaration",
                )),
                // The value exists because an L1's declaration and an L4's
                // inferred answer share one field, not because geometry can be
                // fullscreen. Refused where it is written rather than left to
                // mean something arbitrary downstream.
                Some(Topology::Fullscreen) => errors.push(
                    IrError::contract(proc.span, "`fullscreen` describes a renderer, not geometry")
                        .with_hint(
                            "use `points` or `lines` here. An L4 draws the whole frame by \
                         having no `vertex` block, which is the only way to say it",
                        ),
                ),
                Some(_) => {}
            }
            if proc.blend.is_some() {
                errors.push(
                    IrError::contract(proc.span, "`blend` is L4 only")
                        .with_hint("remove `blend`, or change `kind` to `L4`"),
                );
            }
            if let Some(amp) = &proc.amplify {
                errors.push(
                    IrError::contract(amp.span, "`amplify` is L2 only").with_hint(
                        "remove `amplify`: how many elements an L1 makes is `capacity`, \
                            which a Set turns. Amplification is a *multiplier on what reaches \
                            it*, which is a thing only a stage with an input can be",
                    ),
                );
            }
            for u in &proc.uses {
                match u.ty {
                    SlotTy::Geometry => {
                        errors.push(IrError::contract(u.span, "`uses` is L2 only").with_hint(
                            "remove it: an L1 makes geometry rather than taking any, so there \
                             is nothing for a second one to be blended with",
                        ))
                    }
                    // **A Field slot is legal here, and on L2, L3 and L4** —
                    // the four kinds the engine already walks when it decides
                    // who evaluates a field. A field has no node and no
                    // elements, so naming one adds an input to the file and
                    // nothing to the chain: what an L1 may not take is
                    // *geometry*, and that is what the arm above says.
                    SlotTy::Field => {}
                    // **A camera is for drawing with, and an L1 draws
                    // nothing.** It makes the elements; where they are looked
                    // at from is settled two layers down, by the renderer that
                    // draws them — and a Set may draw one geometry from two
                    // cameras at once, so a viewpoint baked into the geometry
                    // would be a viewpoint one of those two renderers has to
                    // disagree with.
                    SlotTy::Camera => errors.push(
                        IrError::contract(u.span, "`uses … : Camera` is L4 only").with_hint(
                            "remove it: an L1 makes geometry and draws nothing, so there is \
                             no projection here for a camera to be the origin of",
                        ),
                    ),
                }
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
                        IrError::contract(
                            spawn.span,
                            "a `spawn` block requires a `spawn_rate` param",
                        )
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
        // **An L2 is stateless by rule, and this is the rule.**
        //
        // It is the decision the whole layer rests on — `docs/ir-spec.md`, "L2
        // and L3". A stateless modulator is freely stackable, keeps
        // `closed_form` and priming questions the L1 alone answers, and is
        // legal for a graph compiler to fuse rather than merely plausible to.
        // A stateful one would make every one of those a question about the
        // chain, and there would be no way back.
        //
        // Stateless has a precise meaning here and each half is checked below:
        // a `deform` may not read back what it wrote *from the previous frame*
        // — there is no previous frame, since its output is rebuilt each time —
        // and it may not `kill()`.
        Kind::L2 => {
            if let Some(cap) = &proc.capacity {
                errors.push(
                    IrError::contract(cap.span, "`capacity` is L1 only").with_hint(
                        "remove `capacity`: an L2 gets as many elements as reach it, and \
                             how many that is belongs to the L1 that made them",
                    ),
                );
            }
            if proc.topology.is_some() {
                errors.push(
                    IrError::contract(proc.span, "`topology` is L1's").with_hint(
                        "remove `topology`: a deformation moves elements about and does \
                             not turn a cloud into strands, so what the geometry reads as \
                             stays what the L1 declared",
                    ),
                );
            }
            if proc.blend.is_some() {
                errors.push(
                    IrError::contract(proc.span, "`blend` is L4 only")
                        .with_hint("remove `blend`: an L2 rewrites geometry and draws nothing"),
                );
            }
            // **`weight` is read by the lowering**, on the same terms
            // `spawn_rate` is read by the engine: a name the layer gives a
            // meaning to, stated here so that a procedure declaring it as
            // something else is refused rather than silently scaled.
            if let Some(p) = proc.params.iter().find(|p| p.name == "weight") {
                if p.ty != Ty::Float {
                    errors.push(
                        IrError::contract(
                            p.span,
                            format!("`weight` must be a `float`, not a `{}`", p.ty.name()),
                        )
                        .with_hint(
                            "an L2's `weight` is how much of the deformation applies, and the \
                             lowering multiplies the whole modulation by it",
                        ),
                    );
                }
            }
            if !proc.blocks.iter().any(|b| b.kind == BlockKind::Deform) {
                errors.push(
                    IrError::contract(proc.span, "L2 procedures require a `deform` block")
                        .with_hint("add `deform { … }`: it is the whole of what an L2 does"),
                );
            }
            // **Not both at once.** The two break the endomorphism on different
            // axes and each is a change to what the invocation index *means*:
            // an amplifier's index walks the input while the output is
            // `factor` times as long, and a pairing node's index has to be the
            // slot both sources share. A node that did both would need one
            // index to be two things.
            //
            // Refused rather than resolved, because nothing wants it yet and a
            // rule invented for no case is a rule nobody can check against one.
            // **A geometry slot specifically.** What the two break is the
            // meaning of the invocation index, and a Field slot does not touch
            // it: a field has no elements to walk, so an amplifier that
            // evaluates one is an ordinary amplifier.
            if let (Some(u), Some(_)) = (
                proc.uses.iter().find(|u| u.ty == SlotTy::Geometry),
                &proc.amplify,
            ) {
                errors.push(
                    IrError::contract(u.span, "`uses` and `amplify` cannot both apply").with_hint(
                        "split them: a node that takes a second geometry, and a node below \
                             it that amplifies what the first one produced",
                    ),
                );
            }
            // **A second slot is refused with a sentence, not overwritten.**
            // One name would silently win and the other's reads would resolve
            // against the wrong geometry — the shape this whole notation
            // exists to end. What a second one needs is a second bound buffer
            // on the node and a second `edge` per Set, neither of which is
            // built; refusing here is what keeps a file that asks for it a
            // refusal rather than a picture that is quietly wrong.
            // **The count is a rule about geometry, not about `uses`**, which
            // is why it asks the type. A second *Field* slot is the ordinary
            // case — a marcher wanting a shape and a cutter — and costs
            // nothing, because a field has no node and no buffer: it is a body
            // spliced in once more under a second name.
            // **A deformation moves elements and does not project them.**
            // Where an element ends up on screen is the renderer's arithmetic,
            // and it is settled *after* this node has run — so a deformation
            // that read a camera would be deciding the picture from a layer
            // that does not know how many renderers there are, let alone which
            // camera each of them draws with.
            for u in proc.uses.iter().filter(|u| u.ty == SlotTy::Camera) {
                errors.push(
                    IrError::contract(u.span, "`uses … : Camera` is L4 only").with_hint(
                        "remove it: an L2 rewrites geometry in world space, and where that \
                         geometry is watched from is the renderer's question — one geometry \
                         may be drawn from two cameras at once",
                    ),
                );
            }
            let mut geometries = proc.uses.iter().filter(|u| u.ty == SlotTy::Geometry);
            if let Some(first) = geometries.next() {
                for u in geometries {
                    errors.push(
                        IrError::contract(u.span, "a node takes one second geometry").with_hint(
                            format!(
                                "`{}` is already declared. Chain two nodes instead — each takes \
                                 one and passes the result down",
                                first.name
                            ),
                        ),
                    );
                }
            }
            // **A factor below two is refused, and the two cases are refused
            // for different reasons.** Zero is a stage that discards every
            // element, and liveness is the one thing the layer is not permitted
            // to decide — it is settled by the compaction that runs once after
            // L1, and nothing below a deformation reconsiders it. One is the
            // endomorphism, which is what an L2 already is without the
            // declaration: it would allocate a second buffer, a second set of
            // liveness flags and a second `Counts` to produce, element for
            // element, exactly what reached it. A spelling whose presence
            // changes nothing observable is a spelling that will be read as
            // meaning something.
            if let Some(amp) = &proc.amplify {
                if amp.factor < 2 {
                    let (why, hint) = if amp.factor == 0 {
                        (
                            "a factor of 0 discards every element",
                            "an L2 cannot decide liveness — that is settled by the compaction \
                             which runs once after L1, and nothing downstream of a deformation \
                             reconsiders it",
                        )
                    } else {
                        (
                            "a factor of 1 is the endomorphism an L2 already is",
                            "remove `amplify`: a stage that makes one element per element is \
                             what every L2 does, and declaring it would buy a second buffer \
                             and a second set of liveness flags holding a copy of the first",
                        )
                    };
                    errors.push(
                        IrError::contract(amp.span, format!("`amplify` must be at least 2; {why}"))
                            .with_hint(hint),
                    );
                }
                const MAX_AMPLIFY: u32 = 1024;
                if amp.factor > MAX_AMPLIFY {
                    errors.push(
                        IrError::contract(
                            amp.span,
                            format!(
                                "`amplify {}` is above the ceiling of {MAX_AMPLIFY}",
                                amp.factor
                            ),
                        )
                        .with_hint(
                            "every element reaching this node becomes that many, and the derived \
                            buffer is sized at the Set's whole `capacity` times the factor",
                        ),
                    );
                }
            }
        }
        // **An L3 declares nothing about geometry, because it has none.** It
        // produces the six numbers a camera is; what is drawn with them is the
        // renderer's business, and how many elements there are is the L1's.
        Kind::L3 => {
            if let Some(cap) = &proc.capacity {
                errors.push(
                    IrError::contract(cap.span, "`capacity` is L1 only").with_hint(
                        "remove `capacity`: an L3 produces one viewpoint per frame and \
                             has no elements of its own",
                    ),
                );
            }
            if proc.topology.is_some() {
                errors.push(
                    IrError::contract(proc.span, "`topology` is L1's")
                        .with_hint("remove `topology`: an L3 is a viewpoint, not geometry"),
                );
            }
            if proc.blend.is_some() {
                errors.push(
                    IrError::contract(proc.span, "`blend` is L4 only")
                        .with_hint("remove `blend`: an L3 draws nothing"),
                );
            }
            if let Some(amp) = &proc.amplify {
                errors.push(
                    IrError::contract(amp.span, "`amplify` is L2 only")
                        .with_hint("remove `amplify`: an L3 produces one viewpoint, not elements"),
                );
            }
            for u in &proc.uses {
                match u.ty {
                    SlotTy::Geometry => errors.push(
                        IrError::contract(u.span, "`uses` is L2 only")
                            .with_hint("remove it: an L3 produces a viewpoint, not geometry"),
                    ),
                    // Legal: a camera that frames a shape evaluates a field,
                    // and evaluating one is not producing geometry.
                    SlotTy::Field => {}
                    // **An L3 produces a viewpoint and has no use for one.**
                    // Every value a camera slot offers is a *derivation* of the
                    // six numbers this procedure is being asked to write, so a
                    // camera reading one would be reading its own output — last
                    // frame's, since there is no other, which is the frame
                    // ordering an L3 is deliberately not given a way to depend
                    // on.
                    SlotTy::Camera => errors.push(
                        IrError::contract(u.span, "`uses … : Camera` is L4 only").with_hint(
                            "remove it: an L3 *is* a camera — it writes `eye` and `target`, \
                             and `clip`, `eye` and `ray` are derived from what it writes",
                        ),
                    ),
                }
            }
            if !proc.emit.is_empty() {
                errors.push(
                    IrError::contract(proc.span, "`emit` is for procedures that write elements")
                        .with_hint(
                            "remove `emit`: what an L3 produces is a camera, and the six values \
                             it writes are named by the `camera` block rather than declared",
                        ),
                );
            }
            // **Refused rather than ignored, and it will not always be.**
            // `docs/ir-spec.md` settles that an L3 may read geometry — a camera
            // that follows an element is the first thing anyone asks a camera to
            // do — and what it reads is a *reduction* or element zero rather
            // than a per-element attribute, which is syntax this language does
            // not have yet. Until it does, `consumes position` would check
            // clean and lower to a camera that ignores it.
            if !proc.consumes.is_empty() {
                errors.push(
                    IrError::contract(proc.span, "an L3 cannot consume attributes yet").with_hint(
                        "remove `consumes`: a camera that reads geometry points at a reduction \
                         — a centroid, or element zero — and that addressing is specified in \
                         `docs/ir-spec.md` but not built. A camera on the clock alone works today",
                    ),
                );
            }
            if !proc.blocks.iter().any(|b| b.kind == BlockKind::Camera) {
                errors.push(
                    IrError::contract(proc.span, "L3 procedures require a `camera` block")
                        .with_hint("add `camera { … }`: it is the whole of what an L3 does"),
                );
            }
        }
        // **A field declares nothing about geometry, because it is not
        // geometry.** It takes a position and returns a distance; there are no
        // elements to count, to draw, to emit or to consume.
        Kind::Field => {
            for (present, what, hint) in [
                (
                    proc.capacity.is_some(),
                    "capacity",
                    "a field has no elements to allocate",
                ),
                (
                    proc.topology.is_some(),
                    "topology",
                    "a field is a function, not geometry",
                ),
                (proc.blend.is_some(), "blend", "a field draws nothing"),
                (
                    proc.amplify.is_some(),
                    "amplify",
                    "a field makes no elements, so there is nothing to multiply",
                ),
            ] {
                if present {
                    errors.push(
                        IrError::contract(proc.span, format!("`{what}` is not a field's"))
                            .with_hint(format!("remove `{what}`: {hint}")),
                    );
                }
            }
            // **A field takes no geometry either**, which is the one geometry
            // declaration this arm used to let past. It is refused for the
            // reason the four above are refused: a field is a function of
            // space, and a slot is a second set of *elements* to read beside
            // the ones a node runs over.
            //
            // Nothing below would have honoured it. A Set resolves an `edge`
            // against what an L2 declares, so the slot was invisible exactly
            // where it would have been bound, and the edge naming it came back
            // as an unknown slot — a refusal a page away from the line that
            // caused it.
            for u in &proc.uses {
                match u.ty {
                    SlotTy::Geometry => {
                        errors.push(IrError::contract(u.span, "`uses` is L2 only").with_hint(
                            "remove it: a field is a function of space — it is handed `point` \
                             and returns a distance, and there are no elements here for a \
                             second geometry to be read beside",
                        ))
                    }
                    // **A field is the one kind that may not take one**, and
                    // the reason is the one the recursion refusal here used to
                    // carry: a field bound to itself is a function calling
                    // itself, which WGSL forbids outright and which reached the
                    // driver as a shader-module panic from a `.kir` that
                    // checked clean.
                    //
                    // Two fields naming each other is the same failure at one
                    // remove, and telling that apart from a legal chain of
                    // shapes is a walk over every edge in the Set — a graph
                    // question, answerable where the Set is built and nowhere
                    // in this file. Refused whole rather than half-checked,
                    // because the half a single file can check is the half
                    // nobody writes by accident.
                    SlotTy::Field => errors.push(
                        IrError::contract(u.span, "a field cannot take a field").with_hint(
                            "inline what you wanted from it — a field is the one procedure \
                             whose whole body is an expression over `point`, and one bound to \
                             itself is a function calling itself",
                        ),
                    ),
                    // **A field is a function of space and knows nothing about
                    // where it is watched from.** It is spliced into every
                    // procedure that declares a slot for it, and two of those
                    // may draw from two different cameras — so a distance that
                    // varied with the viewpoint would be two different shapes
                    // in one frame.
                    SlotTy::Camera => errors.push(
                        IrError::contract(u.span, "`uses … : Camera` is L4 only").with_hint(
                            "remove it: a field is handed `point` and returns a distance, and \
                             the same point has the same distance from wherever it is seen",
                        ),
                    ),
                }
            }
            if !proc.emit.is_empty() || !proc.consumes.is_empty() {
                errors.push(
                    IrError::contract(
                        proc.span,
                        "`emit` and `consumes` are about elements, and a field has none",
                    )
                    .with_hint(
                        "remove them: a field is handed `point` and returns `distance`, and \
                         the material that happens to be at that point is not something it \
                         can see",
                    ),
                );
            }
            if !proc.blocks.iter().any(|b| b.kind == BlockKind::Field) {
                errors.push(
                    IrError::contract(proc.span, "Field procedures require a `field` block")
                        .with_hint("add `field { … }`: it is the whole of what a field does"),
                );
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
                    IrError::contract(proc.span, "`topology` is declared on L1 and inferred on L4")
                        .with_hint(
                            "remove `topology`: an L4 draws segments by assigning `clip_b` in \
                             `vertex` and sprites by not assigning it, so declaring it here \
                             would be a second place for the same fact to be wrong",
                        ),
                );
            }
            if proc.blend.is_none() {
                errors.push(IrError::contract(
                    proc.span,
                    "L4 procedures require a `blend` declaration",
                ));
            }
            if let Some(amp) = &proc.amplify {
                errors.push(
                    IrError::contract(amp.span, "`amplify` is L2 only").with_hint(
                        "remove `amplify`: a renderer draws what reaches it and makes no \
                            elements. Several copies of one element is a deformation that \
                            amplifies, above the renderer rather than inside it",
                    ),
                );
            }
            for u in &proc.uses {
                match u.ty {
                    SlotTy::Geometry => {
                        errors.push(IrError::contract(u.span, "`uses` is L2 only").with_hint(
                            "remove it: a renderer draws what reaches it. Taking a second \
                             geometry is a deformation, above the renderer rather than inside it",
                        ))
                    }
                    // Legal, and this is the kind it is most for: a marcher
                    // that contains no shape at all takes one here.
                    SlotTy::Field => {}
                    // **The one kind that may declare one**, and the reason is
                    // the reason the value exists: `clip`, `eye` and `ray` are
                    // what a renderer projects and marches with, and no other
                    // layer does either.
                    SlotTy::Camera => {}
                }
            }
            // **A renderer looks from one place.** Two would each need their
            // own bind group in one pipeline and their own edge per Set,
            // neither of which is built — but the reason to refuse rather than
            // build is that nothing has asked for the picture it would make: a
            // frame drawn twice from two viewpoints is two renderers, which is
            // exactly what this commit makes possible. Refused with a sentence
            // so that a file asking for it is turned away at the declaration
            // rather than drawn from whichever slot happened to be first.
            let mut cameras = proc.uses.iter().filter(|u| u.ty == SlotTy::Camera);
            if let Some(first) = cameras.next() {
                for u in cameras {
                    errors.push(
                        IrError::contract(u.span, "a renderer draws from one camera").with_hint(
                            format!(
                                "`{}` is already declared. Two viewpoints in one frame are two \
                                 renderers, each bound to its own camera",
                                first.name
                            ),
                        ),
                    );
                }
            }
            // **A `vertex` block is what makes an L4 per-element**, and an L4
            // without one draws the whole frame instead — see
            // `Topology::Fullscreen`. So its absence is a declaration rather
            // than an omission, and what it declares brings one rule with it:
            // with no vertex block there is nowhere to read an element from, so
            // `consumes` must be empty. Stated as a rule rather than left as a
            // consequence, because it is what lets the engine skip the paired
            // L1's simulation — an optimisation that is provable with this and
            // merely plausible without it.
            if proc.blocks.iter().all(|b| b.kind != BlockKind::Vertex) && !proc.consumes.is_empty()
            {
                errors.push(
                    IrError::contract(
                        proc.span,
                        "an L4 with no `vertex` block draws the whole frame and cannot \
                         consume attributes",
                    )
                    .with_hint(
                        "remove `consumes`, or add a `vertex` block — a fullscreen procedure \
                         has no element to read an attribute from, and it is what lets the \
                         paired L1 skip its simulation entirely",
                    ),
                );
            }
            if proc.blocks.iter().all(|b| b.kind != BlockKind::Fragment) {
                errors.push(IrError::contract(
                    proc.span,
                    "L4 procedures require a `fragment` block",
                ));
            }
            // **A renderer emits nothing**, and this was the one layer where
            // saying so was left out. An L3 and a field both refuse `emit` by
            // name; an L4's was accepted, given no buffer, and read back by
            // `is_closed_form` — which had to state "vacuously true for L4"
            // partly to stop a stray `emit` dragging a Set into needing to be
            // primed. Refusing it is the same fact said once instead of
            // compensated for downstream.
            if let Some((_, span)) = proc.emit.first() {
                errors.push(
                    IrError::contract(*span, "`emit` is not an L4 declaration").with_hint(
                        "a renderer draws what reaches it and stores nothing: it has no \
                             element buffer to emit into. `consumes` is how an L4 says what \
                             it reads",
                    ),
                );
            }
        }
    }

    check_slot_names(proc, errors);

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

/// Whether `name` is a stage output **this kind of procedure can write**.
///
/// **Scoped to the layer, not global.** `Output` grew from four names to ten
/// when the camera arrived, and a global reservation would have made `up`,
/// `target`, `near`, `far` and `fov_y` illegal as params and locals in *every*
/// layer — `let near = length(position)` in a marcher, `let up` in an L1, both
/// previously legal and neither shadowing anything reachable there. A name is
/// only ambiguous where the thing it names exists, and the diagnostic for the
/// global version named a block the procedure did not have.
///
/// `eye` stays refused everywhere, but as an *ambient* rather than an output —
/// a marching fragment reads it, so it genuinely is in scope in an L4.
fn shadows_output(name: &str, kind: Kind) -> bool {
    Output::from_name(name).is_some_and(|o| o.block().kind() == kind)
}

/// Attributes, ambients, and stage outputs are a closed, reserved vocabulary
/// that no param, local or declared geometry slot may take on — see the module
/// docs on shadowing.
///
/// **A slot name goes through here for the same reason a param's does**: it is
/// read the way a local is — `far.position`, `shape(p)` — so one spelling would
/// otherwise mean two things depending on what follows it. What it is *not* is a
/// reserved word of its own — the name belongs to the procedure that declared
/// it, and reserving one language-wide is what caps a procedure at one input.
///
/// A *callable* slot has one more collision than this function knows about, and
/// it is checked beside the call in [`check_slot_names`] rather than added here:
/// a param named `sin` is still a perfectly good param.
/// **What every slot name has to be, whatever type it was declared with.**
///
/// Outside the per-kind arms above, because it is not a rule about a kind: a
/// Field slot is legal on four of the five, so a check that lived in L2's arm
/// would leave the other three able to declare a slot called `position` — and
/// the arm it lived in is exactly where nobody would look for it.
fn check_slot_names(proc: &Proc, errors: &mut Vec<IrError>) {
    for (at, u) in proc.uses.iter().enumerate() {
        // **The slot name shares one scope with everything else nameable
        // here.** It is read the way a local is — `far.position`, `shape(p)` —
        // so a slot called `position` or `t` would make one spelling mean two
        // things depending on what follows it.
        check_reserved(&u.name, u.name_span, proc.kind, "slot", errors);
        if proc.params.iter().any(|p| p.name == u.name) {
            errors.push(
                IrError::contract(u.name_span, format!("`{}` is already a param", u.name))
                    .with_hint("a slot and a param share one scope — rename one of them"),
            );
        }
        // **Two slots of one name**, which nothing could ask before: one slot
        // per procedure made this unreachable, and several Field slots make it
        // the first thing a second declaration gets wrong. Refused rather than
        // resolved by position, on the terms every other collision here is —
        // one name that means two inputs is the failure the whole notation
        // exists to end, arriving through the notation itself.
        if let Some(first) = proc.uses[..at].iter().find(|p| p.name == u.name) {
            errors.push(
                IrError::contract(u.name_span, format!("`{}` is already a slot", u.name))
                    .with_hint(format!(
                        "it is declared as `{}` above — an edge names a slot, so two of one \
                         name would be one address for two inputs",
                        first.ty.name()
                    )),
            );
        }
        // **A Field slot's name is *called*, so it has one more way to
        // collide.** `check_reserved` asks about names that are read — an
        // attribute, an ambient, a stage output — and a builtin is none of
        // those: `sin` was a perfectly good slot name while a slot was only
        // ever read with a dot after it. It is not one now, because a call
        // resolves against the header first and `sin(x)` would stop meaning
        // the sine.
        //
        // Refused rather than ordered around, because either order is a
        // spelling that silently means something else than it says.
        if u.ty == SlotTy::Field {
            let clashes_with = if Builtin::from_name(&u.name).is_some() {
                Some("a builtin function")
            } else if Ty::from_name(&u.name).is_some() {
                Some("a type constructor")
            } else {
                None
            };
            if let Some(what) = clashes_with {
                errors.push(
                    IrError::contract(
                        u.name_span,
                        format!("`{}` is {what}, and a Field slot is called", u.name),
                    )
                    .with_hint(format!(
                        "a field is evaluated as `{}(p)`, so the name would have to mean two \
                         things at one call site — rename the slot",
                        u.name
                    )),
                );
            }
        }
    }
}

fn check_reserved(
    name: &str,
    span: Span,
    kind: Kind,
    kind_of_decl: &str,
    errors: &mut Vec<IrError>,
) {
    if name == "id" {
        errors.push(
            IrError::contract(
                span,
                format!("`id` is reserved and cannot be used as a {kind_of_decl} name"),
            )
            .with_hint("there is no `id` — element identity is `seed`"),
        );
    } else if Attr::from_name(name).is_some() {
        errors.push(
            IrError::contract(span, format!("`{name}` shadows an attribute name"))
                .with_hint("pick a different name — attributes and params share one scope"),
        );
    } else if Ambient::from_name(name).is_some() {
        errors.push(IrError::contract(
            span,
            format!("`{name}` shadows an ambient value"),
        ));
    } else if shadows_output(name, kind) {
        errors.push(IrError::contract(
            span,
            format!("`{name}` shadows a stage output name"),
        ));
    }
}

fn check_params(proc: &Proc, errors: &mut Vec<IrError>) -> HashMap<String, Ty> {
    let mut map = HashMap::new();
    let empty_params: HashMap<String, Ty> = HashMap::new();
    let empty_attrs: HashSet<Attr> = HashSet::new();

    for p in &proc.params {
        if !matches!(p.ty, Ty::Float | Ty::Vec2 | Ty::Vec3) {
            errors.push(IrError::ty(
                p.span,
                format!(
                    "param `{}` has type `{}`; params may only be `float`, `vec2`, or `vec3`",
                    p.name,
                    p.ty.name()
                ),
            ));
        }
        check_reserved(&p.name, p.span, proc.kind, "param", errors);
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
        let mut checker = Checker::new(
            proc.kind,
            None,
            false,
            None,
            &[],
            None,
            &empty_params,
            &empty_attrs,
            &empty_attrs,
        );
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

/// `consumes` against what this procedure has, which since attribute derivation
/// is **not** the same as `consumes ⊆ emit`.
///
/// Two attributes have a rule: `age` and `velocity`. An L1 may consume either
/// without emitting it, and the engine provides it — see
/// `docs/ir-spec.md`, "Attribute derivation". So what is checked here is
/// narrower than it was and is genuinely a one-file question: **`velocity` is
/// synthesised from `position`**, and whether *this* procedure emits `position`
/// is something one file can answer. Whether anybody emits `velocity` is not,
/// and that half moved to `Set::build_many`, which is the first point holding
/// every procedure at once.
///
/// L1 only, for the reason it always was: an L1 is the whole of what is
/// available to it, where an L2 and an L4 read what is available at their
/// position in a chain.
fn check_consumes_emitted(
    kind: Kind,
    emit: &HashSet<Attr>,
    consumes: &[(Attr, Span)],
    errors: &mut Vec<IrError>,
) {
    // **L1 only.** An L1 is the whole of what is available to it, so consuming
    // something it does not emit is a contradiction inside one file. An L2 and
    // an L4 read what is available *at their position* in a chain, which no
    // single procedure can know — that is `Set::build`'s check, against the
    // pair or the chain.
    if kind != Kind::L1 {
        return;
    }
    for &(attr, span) in consumes {
        if emit.contains(&attr) {
            continue;
        }
        // **A rule is a satisfaction, not a softer refusal.** `age` and
        // `velocity` are synthesised where nothing emits them — the Set decides
        // that, since it is the first point holding every procedure at once, and
        // an L1 asking for one of them is asking for something the Set will
        // provide. `velocity` additionally needs `position`, and that *is* a
        // one-file question: the rule reads it off this procedure's own element.
        if let Some(rule) = attr.derivation() {
            match rule.source() {
                None => continue,
                Some(from) if emit.contains(&from) => continue,
                Some(from) => {
                    errors.push(
                        IrError::contract(
                            span,
                            format!(
                                "`{}` is derived from `{}`, which this procedure does not emit",
                                attr.name(),
                                from.name()
                            ),
                        )
                        .with_hint(format!(
                            "add `{}` to `emit`, or emit `{}` and compute it yourself",
                            from.name(),
                            attr.name()
                        )),
                    );
                    continue;
                }
            }
        }
        errors.push(
            IrError::contract(
                span,
                format!("`{}` is consumed but not emitted", attr.name()),
            )
            .with_hint(format!("add `{}` to `emit`", attr.name())),
        );
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
///    **The set tested against is what the procedure *carries*, not what it
///    emits**, and the difference arrived with attribute derivation: an L1 may
///    consume `age` or `velocity` without emitting either, and both are
///    per-element state carried across frames. Reading one is reading where the
///    element has been, which is exactly what this property is about. This
///    paragraph used to say `consumes ⊆ emit` held inside an L1 and that the
///    membership test was therefore belt-and-braces; it no longer holds, and a
///    permissive answer here is the one the block below calls far worse — a
///    scrub that produces garbage, and material put on air unwarmed.
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
fn is_closed_form(kind: Kind, carried: &HashSet<Attr>, blocks: &[TBlock]) -> bool {
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
fn accumulates(stmts: &[TStmt], carried: &HashSet<Attr>) -> bool {
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

/// A read of a **carried** attribute anywhere in one expression — one this
/// procedure emits, or one the engine derives for it.
///
/// **The second half is not a detail.** A derived attribute is per-element state
/// carried across frames exactly as an emitted one is: `age` is the clock minus
/// a stored spawn instant, `velocity` is a stored difference. A procedure
/// reading one is reading where it has been, which is what `closed_form` is
/// asking about — and the block above this one used to say that could not
/// happen, because `consumes ⊆ emit` held inside an L1. It does not any more.
fn reads_carried(e: &TExpr, carried: &HashSet<Attr>) -> bool {
    match &e.kind {
        // **Both sides.** A paired read is a read of the other source's carried
        // state, which is state all the same.
        TExprKind::Attr(a) | TExprKind::Far(a) => carried.contains(a),
        TExprKind::Lit(_) | TExprKind::Local(_) | TExprKind::Param(_) | TExprKind::Ambient(_) => {
            false
        }
        TExprKind::Unary { value, .. } => reads_carried(value, carried),
        TExprKind::Binary { lhs, rhs, .. } => {
            reads_carried(lhs, carried) || reads_carried(rhs, carried)
        }
        TExprKind::Builtin { args, .. } | TExprKind::Construct { args } => {
            args.iter().any(|a| reads_carried(a, carried))
        }
        // **The argument, and nothing behind it.** A field is a function of the
        // position it is handed and holds no state of its own, so what decides
        // this is whatever the caller computed the point from.
        TExprKind::Field { point, .. } => reads_carried(point, carried),
        TExprKind::Swizzle { value, .. } => reads_carried(value, carried),
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

/// Whether the block assigns [`Output::ClipB`] **anywhere**, including on a
/// path coverage does not count — one arm of an `if`, or a `for` body.
///
/// Deliberately not the same question coverage asks. Mentioning `clip_b` is
/// what makes it *required*, and then coverage decides whether it was assigned
/// on every path: a procedure that writes a second endpoint under some
/// condition and not others is drawing a segment sometimes and an
/// uninitialised one the rest of the time, which is a diagnostic rather than a
/// picture. Asking only the coverage question would silently accept it as a
/// points procedure with a dead store.
fn assigns_clip_b(stmts: &[TStmt]) -> bool {
    stmts.iter().any(|s| match s {
        TStmt::Assign { target, .. } => matches!(target, Target::Output(Output::ClipB)),
        TStmt::If { then, els, .. } => assigns_clip_b(then) || assigns_clip_b(els),
        TStmt::For { body, .. } => assigns_clip_b(body),
        TStmt::Let { .. } | TStmt::Var { .. } | TStmt::Kill { .. } => false,
    })
}

fn required_keys(block: BlockKind, emit: &HashSet<Attr>, draws_lines: bool) -> Vec<CovKey> {
    match block {
        BlockKind::Spawn | BlockKind::Element => emit.iter().map(|a| CovKey::Attr(*a)).collect(),
        // The whole of what a field produces, and there is nothing optional
        // beside it — a field that assigned nothing would be a function with no
        // return value.
        BlockKind::Field => vec![CovKey::Output(Output::Distance)],
        // `point_size` is required unconditionally here — see the module
        // docs on why the literal "when the topology is points" condition
        // cannot be evaluated from an L4 file alone. It stays required now
        // that `lines` exists, because a segment has a width for the same
        // reason a sprite has a size.
        BlockKind::Vertex => {
            let mut keys = vec![
                CovKey::Output(Output::Clip),
                CovKey::Output(Output::PointSize),
            ];
            if draws_lines {
                keys.push(CovKey::Output(Output::ClipB));
            }
            keys
        }
        BlockKind::Fragment => vec![CovKey::Output(Output::Color)],
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
        self.frames
            .last_mut()
            .expect("at least one frame")
            .insert(name, info);
    }
}

// ---------------------------------------------------------------------------
// Checker: expression and statement checking for one block (or, with
// `block: None`, one header expression such as a param default).
// ---------------------------------------------------------------------------

struct Checker<'a> {
    kind: Kind,
    block: Option<BlockKind>,
    /// Whether this procedure draws the whole frame — an L4 with no `vertex`
    /// block, which is the only way to say so.
    ///
    /// **Not derivable from `kind` and `block`**, which is why it is carried
    /// here rather than folded into [`Ambient::available_in`]: it is a fact
    /// about the procedure, and a block checker otherwise sees only its own
    /// block. `eye` and `ray` need it — see [`Checker::marching_only`].
    fullscreen: bool,
    /// **What this procedure calls the second geometry it takes**, from
    /// `uses far : Geometry`, and `None` for a procedure that takes none.
    ///
    /// It is what makes `far.position` mean anything, and it is per *procedure*
    /// rather than per block for the reason [`Checker::fullscreen`] is: the
    /// declaration is in the header and a block checker sees only its own
    /// block.
    uses: Option<&'a str>,
    /// **What this procedure calls the fields it evaluates**, from every
    /// `uses <name> : Field` in its header.
    ///
    /// The list is what makes `shape(p)` mean anything, and it is consulted
    /// ahead of the builtin table: a call resolves against what the header
    /// declared, which is the whole of this notation. Several, because a
    /// procedure may want a shape and a cutter, and neither of them is "the"
    /// field.
    fields: &'a [&'a str],
    /// **What this procedure calls the camera it draws from**, from `uses view
    /// : Camera`, and `None` for one that declares none — which then reads the
    /// Set's camera as `camera`, `eye` and `ray`.
    ///
    /// It is what makes `view.clip` mean anything, and it is per procedure for
    /// the reason the two above are: the declaration is in the header and a
    /// block checker sees only its own block.
    camera: Option<&'a str>,
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
    // Nine, and each one is a fact about the *procedure* that a block checker
    // cannot see for itself — the header is not in the block. Bundling them
    // into a struct would be the same nine fields under one name, and the
    // struct would have exactly one constructor and one use.
    #[allow(clippy::too_many_arguments)]
    fn new(
        kind: Kind,
        block: Option<BlockKind>,
        fullscreen: bool,
        uses: Option<&'a str>,
        fields: &'a [&'a str],
        camera: Option<&'a str>,
        params: &'a HashMap<String, Ty>,
        emit: &'a HashSet<Attr>,
        consumes: &'a HashSet<Attr>,
    ) -> Checker<'a> {
        Checker {
            kind,
            block,
            fullscreen,
            uses,
            fields,
            camera,
            params,
            emit,
            consumes,
            scope: Scope::new(),
            errors: Vec::new(),
        }
    }

    /// **`eye` and `ray` exist only where the lowering defines them**, which is
    /// the ray prologue a fullscreen fragment stage opens with. A per-element
    /// L4 has a `vertex` block, gets no prologue, and reading either there
    /// lowered to a bare identifier nothing declared — so the `.kir` checked
    /// clean, `generate_l4` produced WGSL naga refuses, and wgpu's uncaptured
    /// error handler panicked the thread that built it. On the swap worker that
    /// is a `SetError::Panicked`; at startup it takes the process down.
    ///
    /// A rule about the *procedure* rather than the block, which is why it is
    /// not in [`Ambient::available_in`]: what makes an L4 a marcher is the
    /// absence of a `vertex` block, and a block does not know its siblings.
    fn marching_only(&self, amb: Ambient) -> bool {
        !matches!(amb, Ambient::Eye | Ambient::Ray) || self.fullscreen
    }

    /// **The mirror of [`Checker::marching_only`], and it fails the same way.**
    /// `seed` and `copy` are per-element identity; a fullscreen L4 has no
    /// element, no element buffer bound, and a vertex stage the procedure did
    /// not write. Reading either lowered to `in.seed` against a `VsOut` with no
    /// such field — WGSL naga refuses, and wgpu's uncaptured error handler takes
    /// the process down at startup or fails the swap worker.
    ///
    /// This is the same rule `consumes` already states for the same reason, and
    /// the reason it needed a second statement is that `seed` is not a
    /// `consumes`: it is available everywhere an element is, which is exactly
    /// the sentence a fullscreen procedure falsifies.
    fn element_only(&self, amb: Ambient) -> bool {
        !matches!(amb, Ambient::Seed | Ambient::Copy) || !self.fullscreen
    }

    fn err(&mut self, stage: Stage, span: Span, msg: impl Into<String>) {
        self.errors.push(IrError::new(stage, span, msg));
    }

    fn err_hint(
        &mut self,
        stage: Stage,
        span: Span,
        msg: impl Into<String>,
        hint: impl Into<String>,
    ) {
        self.errors
            .push(IrError::new(stage, span, msg).with_hint(hint));
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
            Stmt::If {
                cond,
                then,
                els,
                span,
            } => self.check_if(cond, then, els, *span),
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
            self.err(
                Stage::Contract,
                span,
                format!("`{name}` shadows an attribute name"),
            );
            return;
        }
        if self.uses == Some(name) {
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{name}` is the geometry this procedure uses"),
                "a slot and a local share one scope — rename one of them",
            );
            return;
        }
        if self.fields.contains(&name) {
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{name}` is a field this procedure uses"),
                "a slot and a local share one scope — rename one of them",
            );
            return;
        }
        if self.camera == Some(name) {
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{name}` is the camera this procedure draws from"),
                "a slot and a local share one scope — rename one of them",
            );
            return;
        }
        if Ambient::from_name(name).is_some() {
            self.err(
                Stage::Contract,
                span,
                format!("`{name}` shadows an ambient value"),
            );
            return;
        }
        if shadows_output(name, self.kind) {
            self.err(
                Stage::Contract,
                span,
                format!("`{name}` shadows a stage output name"),
            );
            return;
        }
        if self.params.contains_key(name) {
            self.err(Stage::Contract, span, format!("`{name}` shadows a param"));
            return;
        }
        if self.scope.lookup(name).is_some() {
            self.err(
                Stage::Contract,
                span,
                format!("`{name}` shadows another local"),
            );
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
            let available = match self.block {
                // **A field has no element**, so no attribute is writable in
                // one. It is a function of space, and the material that happens
                // to be at a point is not something it is given.
                Some(BlockKind::Field) => false,
                Some(BlockKind::Spawn) | Some(BlockKind::Element) => self.emit.contains(&attr),
                // **A `deform` writes what it consumes as well as what it
                // emits**, and rewriting is the more common of the two:
                // `position = position + …` is what a modulator is for. An L1
                // has one buffer and one list; an L2 has an input edge and an
                // output one, and it may write anything that reaches the output
                // — which is everything it was given, plus everything it adds.
                Some(BlockKind::Deform) => {
                    self.emit.contains(&attr) || self.consumes.contains(&attr)
                }
                Some(BlockKind::Vertex)
                | Some(BlockKind::Fragment)
                | Some(BlockKind::Camera)
                | Some(BlockKind::Mask)
                | None => false,
            };
            if !available {
                let hint = match self.block {
                    Some(BlockKind::Vertex) | Some(BlockKind::Fragment) => {
                        "attributes are read-only in vertex/fragment blocks — assign a stage \
                         output instead"
                            .to_string()
                    }
                    Some(BlockKind::Deform) => format!(
                        "add `{name}` to `consumes` to rewrite what reaches this node, or to \
                         `emit` to add it to what leaves"
                    ),
                    Some(BlockKind::Mask) => "a mask says where the deformation applies and \
                         writes nothing but `strength` — rewrite the attribute in `deform`"
                        .to_string(),
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
        // **In a `camera` block, `eye` is the output rather than the ambient.**
        // The two name one thing — where the camera is — written here and read
        // in a marching fragment stage, the same way `position` is written by
        // an L1 and read by an L4. Ambients are tried first below, which is
        // right everywhere else and wrong in exactly this block: without this,
        // the one word an L3 author writes most reports "cannot assign to an
        // ambient value".
        if self.block == Some(BlockKind::Camera) {
            if let Some(output) = Output::from_name(name) {
                if output.block() == BlockKind::Camera {
                    return TargetRes::Output(output);
                }
            }
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
                    format!(
                        "write `{name}` inside `{}`, not here",
                        output.block().name()
                    ),
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

    fn check_assign(
        &mut self,
        target: &str,
        op: Option<BinOp>,
        value: &Expr,
        span: Span,
    ) -> Option<TStmt> {
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

    fn check_for(
        &mut self,
        var: &str,
        start: i32,
        end: i32,
        body: &[Stmt],
        span: Span,
    ) -> Option<TStmt> {
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
            // **An L2 gets its own reason**, because "move it into the `element`
            // block" names a block an L2 does not have and would send an author
            // looking for one. The rule there is structural: compaction runs
            // once, after L1, and nothing downstream of a deformation
            // reconsiders liveness — so a `kill()` here would remove an element
            // from a buffer whose live range had already been decided.
            let hint = if self.kind == Kind::L2 {
                "an L2 rewrites elements and never removes them: compaction runs once, after L1, \
                so liveness is settled before a `deform` sees anything. Fade it out instead — \
                write `size` or `color`'s alpha — or kill it in the L1"
            } else {
                "remove it, or move this logic into the `element` block"
            };
            self.err_hint(
                Stage::Contract,
                span,
                "`kill()` is only legal in an L1 `element` block",
                hint,
            );
        }
        Some(TStmt::Kill { span })
    }

    // -- expressions ----------------------------------------------------------

    fn check_expr(&mut self, e: &Expr) -> Option<TExpr> {
        match e {
            Expr::Lit { value, span } => {
                Some(TExpr::new(value.ty(), *span, TExprKind::Lit(*value)))
            }
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
            Expr::Swizzle {
                value,
                components,
                span,
            } => self.check_swizzle(value, components, *span),
        }
    }

    fn check_ident(&mut self, name: &str, span: Span) -> Option<TExpr> {
        if let Some(info) = self.scope.lookup(name) {
            return Some(TExpr::new(
                info.ty,
                span,
                TExprKind::Local(name.to_string()),
            ));
        }
        if let Some(ty) = self.params.get(name) {
            return Some(TExpr::new(*ty, span, TExprKind::Param(name.to_string())));
        }
        if let Some(attr) = Attr::from_name(name) {
            let available = match self.block {
                // **A `spawn` block reads only what this procedure emits.**
                // An attribute it merely `consumes` is one the engine derives,
                // and every rule reads state an element does not have yet: the
                // spawn instant is written after this block runs, and last
                // step's position is a step this element has not lived. Read
                // here, both would be whatever the slot held for the element
                // that last occupied it.
                //
                // Refused rather than substituted, because there is no value to
                // substitute. It is also refused rather than left to the
                // lowering, which had no field to name and produced WGSL naga
                // rejects — a `.kir` that checked clean and took the process
                // down, which is the one shape this pass exists to prevent.
                // Read side of the same rule: no element, no attributes.
                Some(BlockKind::Field) => false,
                Some(BlockKind::Spawn) => self.emit.contains(&attr),
                Some(BlockKind::Element) => {
                    self.emit.contains(&attr) || self.consumes.contains(&attr)
                }
                // **Both lists, and they mean different things here.** A
                // `deform` reads what it `consumes` from upstream and reads
                // back what it `emit`s, because an L2 that widens the element —
                // adding a `tint` nothing produced — has to be able to read the
                // field it is writing. Same shape as an `element` block, for a
                // different reason: there the two lists are one buffer, here
                // they are the input edge and the output one.
                Some(BlockKind::Deform) => {
                    self.emit.contains(&attr) || self.consumes.contains(&attr)
                }
                // **`consumes` only, unlike the `deform` beside it.** A mask
                // decides where the deformation applies, which is a question
                // about what *reaches* this node — and an `emit`ted attribute
                // has not been written yet when the mask runs, so reading one
                // here would read the zero the pass-through left rather than a
                // value. Refusing it is better than a rule an author has to
                // remember.
                Some(BlockKind::Mask) => self.consumes.contains(&attr),
                Some(BlockKind::Vertex) | Some(BlockKind::Fragment) => {
                    self.consumes.contains(&attr)
                }
                // An L3 has no element in hand — see `check_header`.
                Some(BlockKind::Camera) | None => false,
            };
            if available {
                return Some(TExpr::new(attr.ty(), span, TExprKind::Attr(attr)));
            }
            let hint = match self.block {
                Some(BlockKind::Field) => format!(
                    "a field is a function of space: it is handed `point` and nothing else, \
                     so `{name}` — a property of an element — has no meaning here"
                ),
                Some(BlockKind::Vertex) | Some(BlockKind::Fragment) => {
                    format!("add `{name}` to `consumes` to read it here")
                }
                Some(BlockKind::Spawn) if attr.is_derivable() && self.consumes.contains(&attr) => {
                    format!(
                        "`{name}` is derived, and a derivation reads state an element being \
                         spawned does not have yet — add `{name}` to `emit` and write it here, \
                         or read it in `element`"
                    )
                }
                Some(BlockKind::Spawn) | Some(BlockKind::Element) => {
                    format!("add `{name}` to `emit` to read it here")
                }
                Some(BlockKind::Deform) => format!(
                    "add `{name}` to `consumes` to read what reaches this node, or to \
                     `emit` to add it to what leaves"
                ),
                Some(BlockKind::Mask) => format!(
                    "add `{name}` to `consumes` — a mask reads what reaches this node, and an \
                     `emit`ted attribute has not been written when it runs"
                ),
                Some(BlockKind::Camera) => "a camera reads the clock and its params, not \
                     elements — pointing one at geometry means naming a reduction or element \
                     zero, which `docs/ir-spec.md` specifies and nothing builds yet"
                    .to_string(),
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
                Some(block) => {
                    ambient.available_in(self.kind, block)
                        && self.marching_only(ambient)
                        && self.element_only(ambient)
                }
                None => false,
            };
            if available {
                return Some(TExpr::new(ambient.ty(), span, TExprKind::Ambient(ambient)));
            }
            // A marcher's values get their own sentence, because "not available
            // in this block" would send an author looking at the block when what
            // is wrong is that the procedure has a `vertex` block at all.
            // The converse sentence, for the converse mistake — and it names
            // the `vertex` block too, because that is the thing to add rather
            // than the thing to remove.
            if matches!(ambient, Ambient::Seed | Ambient::Copy) && self.fullscreen {
                self.err_hint(
                    Stage::Contract,
                    span,
                    format!("`{name}` is per element, and this procedure draws the whole frame"),
                    "an L4 with no `vertex` block covers the frame and has no element to have \
                     an identity. Add a `vertex` block to draw elements, or drive the \
                     picture from `eye`, `ray` and `point_coord`, which are what a \
                     marcher has",
                );
                return None;
            }
            // The same shape once more, from the third side. A field has no
            // element for a reason an author cannot see from the block: it is
            // not a node, so "which element" has no answer at the point the
            // splice lands.
            if ambient == Ambient::Seed && self.kind == Kind::Field {
                self.err_hint(
                    Stage::Contract,
                    span,
                    "`seed` is per element, and a field has none",
                    "a field is a function of space — it is handed `point` and nothing else, \
                     and it is spliced into every procedure that declares a slot for it, \
                     including fragment stages that have no element at all. Vary it with `t`, `beats` \
                     or a `param` instead",
                );
                return None;
            }
            if matches!(ambient, Ambient::Eye | Ambient::Ray) && self.kind == Kind::L4 {
                self.err_hint(
                    Stage::Contract,
                    span,
                    format!("`{name}` is only available to a procedure that draws the whole frame"),
                    "a `vertex` block is what makes an L4 per-element, and a ray through a \
                     fragment is not something a sprite has. Remove the `vertex` block to \
                     march, or read the attributes this procedure `consumes` instead",
                );
                return None;
            }
            self.err(
                Stage::Contract,
                span,
                format!("`{name}` is not available in this block"),
            );
            return None;
        }
        // **A geometry is not a value.** `far` on its own is the whole second
        // source, which this language has no type for and no way to pass — the
        // one thing that can be said about it is what one of its elements
        // holds.
        //
        // **Before the stage outputs**, and that is not an ordering
        // convenience: `near` and `far` are the camera's clip planes, so they
        // are output names on an L3 and ordinary names everywhere else — which
        // is exactly what `shadows_output` already says by asking about the
        // kind. Asked in the other order, an L2 slot called `far` would be
        // declarable and unreadable.
        if self.uses == Some(name) {
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{name}` is a geometry, not a value"),
                format!(
                    "read an attribute of the element it is paired with — `{name}.position` \
                     — which is the whole of what a used geometry offers"
                ),
            );
            return None;
        }
        // **A field is not a value either**, and for the mirror reason: it is a
        // function of space, so what can be had from it is its value *at* a
        // point. The language has no function type and nothing to pass one to.
        if self.fields.contains(&name) {
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{name}` is a field, not a value"),
                format!(
                    "evaluate it at a point — `{name}(p)` — which is the whole of what a \
                     field offers"
                ),
            );
            return None;
        }
        // **A camera is not a value either.** It is six numbers and three
        // derivations of them, and the language has no type for any of that —
        // what can be had is one of the parts, which is what the members are.
        if self.camera == Some(name) {
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{name}` is a camera, not a value"),
                format!(
                    "read one of its parts — `{name}.clip`, `{name}.eye`, `{name}.ray` — \
                     which is the whole of what a camera offers a renderer"
                ),
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
                    self.err(
                        Stage::Type,
                        span,
                        format!("`-` is not defined for `{}`", ty.name()),
                    );
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
        if lhs == rhs
            && matches!(
                lhs,
                Ty::Float | Ty::Int | Ty::Uint | Ty::Vec2 | Ty::Vec3 | Ty::Vec4
            )
        {
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

    /// **What the header declared, before the table the language ships.**
    ///
    /// A field is reached through a slot, so the name at a call site is the
    /// procedure's own — and this branch is the whole of that. It is first for
    /// the reason it is a branch at all: a call resolves against what the file
    /// said it takes, and asking the builtin table first would make the
    /// language's vocabulary quietly outrank the header. Nothing is hidden by
    /// the order, since a slot named after a builtin is refused where it is
    /// declared — see `check_slot_names`.
    fn check_call(&mut self, name: &str, args: &[Expr], span: Span) -> Option<TExpr> {
        if self.fields.contains(&name) {
            return self.check_field_call(name, args, span);
        }
        if let Some(builtin) = Builtin::from_name(name) {
            return self.check_builtin_call(builtin, args, span);
        }
        if let Some(ty) = Ty::from_name(name) {
            return self.check_constructor(ty, args, span);
        }
        // **`field` is an ordinary name now**, and this is the sentence the
        // person migrating a file reads. It was the reserved word a field was
        // reached through, and reserving one is exactly what capped a procedure
        // at a single input — so it is gone rather than kept as an alias for
        // "the one field, if there is exactly one", which is the rule the slot
        // exists to remove.
        let hint = (name == "field").then_some(
            "`field(p)` is no longer the way to evaluate one: declare which field this \
             procedure takes — `uses shape : Field` — and call it by that name, `shape(p)`. \
             The Set binds it with `--edge <node>.shape=<field>`",
        );
        match hint {
            Some(hint) => self.err_hint(
                Stage::Type,
                span,
                format!("`{name}` is not a builtin function or a type constructor"),
                hint,
            ),
            None => self.err(
                Stage::Type,
                span,
                format!("`{name}` is not a builtin function or a type constructor"),
            ),
        }
        for a in args {
            self.check_expr(a);
        }
        None
    }

    /// **A field's signature is the field block's**: one `vec3` in, a `float`
    /// out. It is not read off the bound procedure, because no single file
    /// holds one — every `kind Field` has exactly this shape, which is what
    /// makes a slot bindable at all.
    fn check_field_call(&mut self, slot: &str, args: &[Expr], span: Span) -> Option<TExpr> {
        if args.len() != 1 {
            self.err_hint(
                Stage::Type,
                span,
                format!("`{slot}` takes 1 argument, found {}", args.len()),
                "a field is handed a position and returns the distance at it",
            );
            for a in args {
                self.check_expr(a);
            }
            return None;
        }
        let point = self.check_expr(&args[0])?;
        if point.ty != Ty::Vec3 {
            self.err(
                Stage::Type,
                point.span,
                format!("`{slot}` expects `vec3`, found `{}`", point.ty.name()),
            );
            return None;
        }
        Some(TExpr::new(
            Ty::Float,
            span,
            TExprKind::Field {
                slot: slot.to_string(),
                point: Box::new(point),
            },
        ))
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
                if !matches!(
                    a,
                    Expr::Lit {
                        value: Lit::Int(_),
                        ..
                    }
                ) {
                    self.err_hint(
                        Stage::Type,
                        a.span(),
                        format!(
                            "argument {} to `{}` must be a literal integer",
                            i + 1,
                            b.name()
                        ),
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
                            format!(
                                "`{}` expects `{}`, found `{}`",
                                b.name(),
                                t.name(),
                                actual.ty.name()
                            ),
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
                        format!(
                            "`{}(...)` converts a scalar number, found `{}`",
                            ty.name(),
                            a.ty.name()
                        ),
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
                            return Some(TExpr::new(
                                ty,
                                span,
                                TExprKind::Construct { args: vec![a] },
                            ));
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
                self.err(
                    Stage::Type,
                    span,
                    format!("`{}` cannot be constructed", ty.name()),
                );
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
    fn check_constructor_concat(
        &mut self,
        ty: Ty,
        n: usize,
        args: Vec<TExpr>,
        span: Span,
    ) -> Option<TExpr> {
        let mut total = 0usize;
        let mut ok = true;
        for a in &args {
            match a.ty.components() {
                Some(c) if matches!(a.ty, Ty::Float | Ty::Vec2 | Ty::Vec3 | Ty::Vec4) => {
                    total += c as usize
                }
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

    /// `<slot>.<attr>` — an attribute of the far element, from the geometry
    /// bound to the slot this procedure declared.
    ///
    /// **Only for something the node consumes.** The far geometry is an input
    /// edge: this node reads it and writes its own output, so what is readable
    /// there is what the node declared it takes.
    ///
    /// `slot` is the name the header gave it, carried in only so the
    /// diagnostics are written in the author's own spelling. Which node fills
    /// it is not decided here and never can be — it is the Set's answer.
    fn check_far(&mut self, slot: &str, name: &str, span: Span) -> Option<TExpr> {
        let Some(attr) = Attr::from_name(name) else {
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{slot}.{name}` is not an attribute"),
                format!(
                    "`{slot}` is a geometry, so it has the attributes an element has — \
                     `{slot}.position`, `{slot}.tint` — and nothing else"
                ),
            );
            return None;
        };
        if !matches!(self.block, Some(BlockKind::Deform) | Some(BlockKind::Mask)) {
            self.err(
                Stage::Contract,
                span,
                format!("`{slot}` is readable in a `deform` or a `mask`, and nowhere else"),
            );
            return None;
        }
        if !self.consumes.contains(&attr) {
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{slot}.{name}` is not consumed"),
                format!(
                    "add `{name}` to `consumes`: a used geometry is an input edge, and one \
                     `consumes` covers both sides of it — a node reads the same attribute \
                     from each"
                ),
            );
            return None;
        }
        Some(TExpr::new(attr.ty(), span, TExprKind::Far(attr)))
    }

    /// `<slot>.<member>` — one part of the camera bound to the slot this
    /// procedure declared.
    ///
    /// **The second resolution path, and the reason a Camera slot cost the
    /// checker anything at all.** `far.position` resolves against the attribute
    /// table, because a geometry's parts *are* attributes; a camera's are not
    /// parts of anything else, so this asks the slot's *type* what it has.
    /// Three members, and each is a value an L4 could already read — which is
    /// what makes this a new spelling rather than a new capability: the
    /// unnamed forms are [`Ambient::Camera`], [`Ambient::Eye`] and
    /// [`Ambient::Ray`], and they mean the Set's camera where no slot was
    /// declared.
    ///
    /// **So the stage rules are the ambients' own**, asked rather than
    /// restated: `.clip` is readable wherever an L4 projects, and `.eye` and
    /// `.ray` only in the fragment stage of a procedure that draws the whole
    /// frame — because they are defined by the ray prologue such a stage opens
    /// with and by nothing else. A second copy of those rules here would be a
    /// second place for them to be wrong, and this one would be the copy
    /// nobody looks at.
    fn check_camera_member(&mut self, slot: &str, name: &str, span: Span) -> Option<TExpr> {
        let amb = match name {
            "clip" => Ambient::Camera,
            "eye" => Ambient::Eye,
            "ray" => Ambient::Ray,
            _ => {
                self.err_hint(
                    Stage::Contract,
                    span,
                    format!("`{slot}.{name}` is not part of a camera"),
                    format!(
                        "a camera offers a renderer three things — `{slot}.clip`, the \
                         projection to multiply a position by; `{slot}.eye`, where it is; \
                         and `{slot}.ray`, the direction through this fragment. Its `target`, \
                         `up`, `fov_y`, `near` and `far` are the L3's to write and no \
                         renderer can read them yet — see `docs/roadmap.md`"
                    ),
                );
                return None;
            }
        };
        let available = match self.block {
            Some(block) => amb.available_in(self.kind, block) && self.marching_only(amb),
            None => false,
        };
        if available {
            return Some(TExpr::new(amb.ty(), span, TExprKind::Ambient(amb)));
        }
        // The same sentence the bare ambient gets, because it is the same
        // mistake: a `vertex` block is what makes an L4 per element, and a ray
        // through a fragment is not something a sprite has.
        if matches!(amb, Ambient::Eye | Ambient::Ray) && !self.fullscreen {
            self.err_hint(
                Stage::Contract,
                span,
                format!(
                    "`{slot}.{name}` is only available to a procedure that draws the whole frame"
                ),
                format!(
                    "a `vertex` block is what makes an L4 per-element, and a ray through a \
                     fragment is not something a sprite has. Remove the `vertex` block to \
                     march, or project with `{slot}.clip` instead"
                ),
            );
            return None;
        }
        self.err_hint(
            Stage::Contract,
            span,
            format!("`{slot}.{name}` is not available in this block"),
            format!(
                "`{slot}.eye` and `{slot}.ray` are the fragment stage's — they are built by \
                 the ray prologue it opens with, and a vertex stage has no fragment to send \
                 one through"
            ),
        );
        None
    }

    fn check_swizzle(&mut self, value: &Expr, components: &str, span: Span) -> Option<TExpr> {
        // **`far.position` is not a swizzle**, and it arrives here because it
        // is *shaped* like one — `expr . ident` is the grammar, and the parser
        // is right not to decide which it is. Deciding here costs no new
        // syntactic category, which is the whole reason a read of the second
        // geometry is spelled this way: `uses` adds one header declaration and
        // one name, and nothing else in the language moves.
        //
        // **The base name is the procedure's own**, so what reaches here is a
        // comparison against what the header declared rather than against a
        // reserved word. That is the difference the whole notation is: a file
        // that had to spell it `other` could only ever have one.
        if let Expr::Ident { name, .. } = value {
            if self.uses == Some(name.as_str()) {
                return self.check_far(name, components, span);
            }
            // **The same shape a third time, resolved a second way.** A
            // geometry slot's members are attributes and a camera slot's are
            // not, so this cannot go through `check_far`: what decides which
            // members exist is the *type* the header declared, which is the
            // whole of what the type on a slot is for.
            if self.camera == Some(name.as_str()) {
                return self.check_camera_member(name, components, span);
            }
        }
        let v = self.check_expr(value)?;
        let width = match v.ty {
            Ty::Vec2 => 2u8,
            Ty::Vec3 => 3,
            Ty::Vec4 => 4,
            other => {
                self.err(
                    Stage::Type,
                    span,
                    format!(
                        "cannot swizzle `{}`; only vectors have components",
                        other.name()
                    ),
                );
                return None;
            }
        };
        if components.is_empty() || components.len() > 4 {
            self.err(
                Stage::Type,
                span,
                format!(
                    "swizzle must have 1 to 4 components, found {}",
                    components.len()
                ),
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
                        format!(
                            "`{c}` is not a valid swizzle component; use `x`, `y`, `z`, or `w`"
                        ),
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
                    format!(
                        "`{}` only has components `{}`",
                        v.ty.name(),
                        &"xyzw"[..width as usize]
                    ),
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
