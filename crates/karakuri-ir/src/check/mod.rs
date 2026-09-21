//! Stages 2 and 3: type checking and contract checking.
//!
//! Validates the syntax tree produced by the parser and returns a typed, checked
//! representation ([`Checked`](crate::typed::Checked)) or a set of diagnostic errors.
//!
//! ### Key Responsibilities
//! - Validates attribute references, local bindings, and shadowing restrictions.
//! - Checks semantic contracts for procedure kinds (L1-L5), topology, and blend modes.
//! - Determines whether a procedure is closed-form (evaluable at arbitrary `t`)
//!   via [`is_closed_form`](crate::check::coverage::is_closed_form).

pub mod context;
pub mod contracts;
pub mod coverage;
pub mod eval;

pub(crate) use context::*;
pub(crate) use contracts::*;
pub(crate) use coverage::*;

use std::collections::HashSet;

use crate::ast::{Attr, BlockKind, Kind, Proc, SlotTy, Topology};
use crate::error::{IrError, IrResult};
use crate::typed::{Checked, InputPort, Slot, TBlock};

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
    // **The fifth, and the one that makes a name a bare value.** A Source slot
    // is read as `only` and nothing else, so a block checker that could not see
    // the header would resolve it against the signal-bus hint and tell the
    // author to declare a param.
    //
    // A list, like the field one and for the same reason: what a Source slot
    // costs is a `u32` in a uniform block the module already has, so nothing
    // caps it at one and `source == a || source == b` is the ordinary case.
    let sources: Vec<&str> = proc
        .uses
        .iter()
        .filter(|u| u.ty == SlotTy::Source)
        .map(|u| u.name.as_str())
        .collect();
    // **The sixth such fact, and the one that makes a name fetchable.** A
    // Texture slot is read only as `texel(<name>)` or `tap(<name>, uv)`, so
    // what the header declared has to reach the call checker or the call
    // resolves against the builtin table and finds nothing.
    //
    // A list, like the field and source ones: a fold of three pictures is as
    // ordinary as a fold of two, and nothing caps it.
    let textures: Vec<&str> = proc
        .uses
        .iter()
        .filter(|u| u.ty == SlotTy::Texture)
        .map(|u| u.name.as_str())
        .collect();
    // **Whether this procedure declares a geometry slot**, which is what makes
    // `source` ambiguous rather than merely present — see
    // [`Checker::paired`].
    let paired = proc
        .uses
        .iter()
        .find(|u| u.ty == SlotTy::Geometry)
        .map(|u| u.name.clone());
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
            &sources,
            &textures,
            proc.retains.is_some(),
            paired.as_deref(),
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
        let closed_form = is_closed_form(proc.kind, proc.retains.is_some(), &carried, &blocks);
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
                // **An L5 covers the frame exactly once and declares nothing
                // about it.** `Topology::Fullscreen` describes what a
                // *renderer* answered by having no `vertex` block, and an L5
                // has no per-element form for its absence to be an answer
                // against — so the field stays empty and `cost::fragment_ceiling`
                // asks the kind instead.
                Kind::L5 => None,
            },
            capacity: proc.capacity,
            amplify: proc.amplify.map(|a| a.factor),
            uses: proc
                .uses
                .iter()
                .map(|u| Slot {
                    name: InputPort(u.name.clone()),
                    ty: u.ty,
                })
                .collect(),
            retains: proc.retains.is_some(),
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
