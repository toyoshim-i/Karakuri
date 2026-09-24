//! Stages 2 and 3: type checking and contract checking.
//!
//! Validates AST semantics, attribute bindings, kind contracts, and closed-form properties.

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

    // Resolve slot declarations for expression typing and resolution.
    let uses = proc
        .uses
        .iter()
        .find(|u| u.ty == SlotTy::Geometry)
        .map(|u| u.name.as_str());
    let fields: Vec<&str> = proc
        .uses
        .iter()
        .filter(|u| u.ty == SlotTy::Field)
        .map(|u| u.name.as_str())
        .collect();
    let camera = proc
        .uses
        .iter()
        .find(|u| u.ty == SlotTy::Camera)
        .map(|u| u.name.as_str());
    let sources: Vec<&str> = proc
        .uses
        .iter()
        .filter(|u| u.ty == SlotTy::Source)
        .map(|u| u.name.as_str())
        .collect();
    let textures: Vec<&str> = proc
        .uses
        .iter()
        .filter(|u| u.ty == SlotTy::Texture)
        .map(|u| u.name.as_str())
        .collect();
    let paired = proc
        .uses
        .iter()
        .find(|u| u.ty == SlotTy::Geometry)
        .map(|u| u.name.clone());
    let fullscreen =
        proc.kind == Kind::L4 && proc.blocks.iter().all(|b| b.kind != BlockKind::Vertex);
    let mut blocks = Vec::with_capacity(proc.blocks.len());
    for block in &proc.blocks {
        // Check blocks under their own natural kind to localize diagnostics.
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
                Kind::L2 | Kind::L3 | Kind::Field | Kind::L5 => None,
                Kind::L4 => Some(drawn_topology(&blocks)),
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

/// Infers the rendered primitive topology for an L4 procedure based on clip output assignments.
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
