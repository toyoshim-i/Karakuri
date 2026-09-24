//! Procedure header contract checks.

use std::collections::HashSet;

use super::{check_slot_names, kind_name};
use crate::ast::{BlockKind, Kind, Proc, SlotTy, Topology, Ty};
use crate::error::IrError;
use crate::span::Span;

/// Creates an error for invalid `uses … : Texture` declarations on non-L5 procedures.
pub(crate) fn texture_slot_is_l5s(span: Span, why: &str) -> IrError {
    IrError::contract(span, "`uses … : Texture` is L5 only").with_hint(format!(
        "remove it, or change `kind` to `L5`: {why}. A Texture slot is one input of a nested \
         L5, bound by an `edge` and read with `texel` and `tap`"
    ))
}

pub(crate) fn check_header(proc: &Proc, errors: &mut Vec<IrError>) {
    match proc.kind {
        Kind::L1 => {
            match &proc.capacity {
                None => errors.push(IrError::contract(
                    proc.span,
                    "L1 procedures require a `capacity` declaration",
                )),
                // Capacity minimum must be >= 1 for compaction pyramid allocation.
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
                // Fullscreen topology applies to renderers, not geometry generators.
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
                    // Field slots are permitted on L1, L2, L3, and L4.
                    SlotTy::Field => {}
                    // Cameras are permitted on L4 renderers only.
                    SlotTy::Camera => errors.push(
                        IrError::contract(u.span, "`uses … : Camera` is L4 only").with_hint(
                            "remove it: an L1 makes geometry and draws nothing, so there is \
                             no projection here for a camera to be the origin of",
                        ),
                    ),
                    // Source slots bind a uniform identity without buffer allocation.
                    SlotTy::Source => {}
                    SlotTy::Texture => errors.push(texture_slot_is_l5s(
                        u.span,
                        "an L1 makes geometry, and a picture is what the layers below it eventually \
                         produce",
                    )),
                }
            }
            if proc.blocks.iter().all(|b| b.kind != BlockKind::Element) {
                errors.push(IrError::contract(
                    proc.span,
                    "L1 procedures require an `element` block",
                ));
            }
            // Procedures with a spawn block must declare a spawn_rate parameter.
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
        // L2 procedures are stateless modulators without frame carry or kill capabilities.
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
            // `weight` parameter is read by code generation as the modulation scaling factor.
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
            // Geometry slot usage and amplification are mutually exclusive on L2.
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
            // L2 accepts at most one Geometry slot; multiple Geometry slots are prohibited.
            // Camera slots are L4 only; deformation nodes operate in world space.
            for u in proc.uses.iter().filter(|u| u.ty == SlotTy::Camera) {
                errors.push(
                    IrError::contract(u.span, "`uses … : Camera` is L4 only").with_hint(
                        "remove it: an L2 rewrites geometry in world space, and where that \
                         geometry is watched from is the renderer's question — one geometry \
                         may be drawn from two cameras at once",
                    ),
                );
            }
            // **The one kind whose slot rules are written as filters rather
            // than as a `match`**, so this refusal is a loop of its own rather
            // than an arm. Same sentence as the other four.
            for u in proc.uses.iter().filter(|u| u.ty == SlotTy::Texture) {
                errors.push(texture_slot_is_l5s(
                    u.span,
                    "an L2 rewrites elements, and a picture is what several layers below it \
                     eventually produce",
                ));
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
            // Amplify factor must be at least 2 (factor 0 discards elements, 1 is identity).
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
                            buffer is sized at the Set's whole `capacity` times the factor. This \
                            ceiling only keeps that product inside a `u32`: the cost pass \
                            multiplies this node's per-element cost by the factor, so a body that \
                            does real work is refused well below it and by that message instead",
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
                    // L3 produces a camera viewpoint and cannot bind camera slots.
                    SlotTy::Camera => errors.push(
                        IrError::contract(u.span, "`uses … : Camera` is L4 only").with_hint(
                            "remove it: an L3 *is* a camera — it writes `eye` and `target`, \
                             and `clip`, `eye` and `ray` are derived from what it writes",
                        ),
                    ),
                    // L3 viewpoints are independent of source element geometries.
                    SlotTy::Source => errors.push(
                        IrError::contract(
                            u.span,
                            "`uses … : Source` is for masking, and an L3 masks nothing",
                        )
                        .with_hint(
                            "remove it: a Source slot is the comparand for `source`, and an L3 \
                             runs once a frame over no geometry — there is no chain instance \
                             here for `source` to name",
                        ),
                    ),
                    SlotTy::Texture => errors.push(texture_slot_is_l5s(
                        u.span,
                        "an L3 produces a viewpoint, and a picture is what is seen from one rather \
                         than something a camera takes in",
                    )),
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
            // L3 cannot currently consume element attributes.
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
            // Field procedures evaluate spatial functions and cannot bind geometry slots.
            for u in &proc.uses {
                match u.ty {
                    SlotTy::Geometry => {
                        errors.push(IrError::contract(u.span, "`uses` is L2 only").with_hint(
                            "remove it: a field is a function of space — it is handed `point` \
                             and returns a distance, and there are no elements here for a \
                             second geometry to be read beside",
                        ))
                    }
                    // Fields cannot bind other fields to prevent recursive calls.
                    SlotTy::Field => errors.push(
                        IrError::contract(u.span, "a field cannot take a field").with_hint(
                            "inline what you wanted from it — a field is the one procedure \
                             whose whole body is an expression over `point`, and one bound to \
                             itself is a function calling itself",
                        ),
                    ),
                    // Field distance functions are camera-invariant.
                    SlotTy::Camera => errors.push(
                        IrError::contract(u.span, "`uses … : Camera` is L4 only").with_hint(
                            "remove it: a field is handed `point` and returns a distance, and \
                              the same point has the same distance from wherever it is seen",
                        ),
                    ),
                    // Field functions are independent of source geometry identities.
                    SlotTy::Source => errors.push(
                        IrError::contract(
                            u.span,
                            "`uses … : Source` is for masking, and a field masks nothing",
                        )
                        .with_hint(
                            "remove it: a field is handed `point` and returns a distance, and \
                             it is spliced into callers that may be running over different \
                             geometries — mask in the caller, where there is one source to name",
                        ),
                    ),
                    SlotTy::Texture => errors.push(texture_slot_is_l5s(
                        u.span,
                        "a field is handed `point` and returns a distance, and a picture has no \
                         distance at a point",
                    )),
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
                    // Legal, and on a fullscreen renderer too: `source` is per
                    // *instance* rather than per element, so a procedure with
                    // no element still knows whose chain it is running in.
                    SlotTy::Source => {}
                    SlotTy::Texture => errors.push(texture_slot_is_l5s(
                        u.span,
                        "a renderer draws *into* a picture rather than out of one — folding several \
                         together is what an L5 is",
                    )),
                }
            }
            // A renderer draws from a single camera slot.
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
            // Fullscreen L4 procedures without vertex blocks cannot consume element attributes.
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
            // Renderers draw pixels and cannot emit element attributes.
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
        // L5 procedures operate on textures and have no element geometry or vertex stages.
        Kind::L5 => {
            for (present, what, hint) in [
                (
                    proc.capacity.is_some(),
                    "capacity",
                    "an L5 covers the frame it is handed exactly once, and how much material \
                     is in that frame was settled several layers upstream",
                ),
                (
                    proc.topology.is_some(),
                    "topology",
                    "an L5 has no elements to be points or lines. It covers the frame, which \
                     is what `fullscreen` says about a renderer and is not a thing to declare \
                     here",
                ),
                (
                    proc.blend.is_some(),
                    "blend",
                    "an L5 writes one texel per texel and nothing overdraws, so there is \
                     nothing for two fragments on one texel to be combined by",
                ),
                (
                    proc.amplify.is_some(),
                    "amplify",
                    "an L5 makes no elements, so there is nothing to multiply",
                ),
            ] {
                if present {
                    errors.push(
                        IrError::contract(proc.span, format!("`{what}` is not an L5's"))
                            .with_hint(format!("remove `{what}`: {hint}")),
                    );
                }
            }
            for u in &proc.uses {
                match u.ty {
                    // L5 can fold any number of texture inputs.
                    SlotTy::Texture => {}
                    // L5 processes frame textures and cannot access element-level slots.
                    SlotTy::Geometry => {
                        errors.push(IrError::contract(u.span, "`uses` is L2 only").with_hint(
                            "remove it: an L5 is handed a picture, and the elements that drew \
                             it are no longer in front of it. Fold another picture in with \
                             `uses <name> : Texture` instead",
                        ))
                    }
                    SlotTy::Camera => errors.push(
                        IrError::contract(u.span, "`uses … : Camera` is L4 only").with_hint(
                            "remove it: the frame an L5 is handed may hold several decks\u{2019} \
                             material seen from several cameras, so there is no one viewpoint \
                             for it to name",
                        ),
                    ),
                    SlotTy::Source => errors.push(
                        IrError::contract(
                            u.span,
                            "`uses … : Source` is for masking, and an L5 masks nothing",
                        )
                        .with_hint(
                            "remove it: a Source slot is the comparand for `source`, and the \
                             frame an L5 is handed may hold several sources folded together — \
                             mask upstream, in a node that runs over one geometry",
                        ),
                    ),
                    // L5 operates on screen coordinates rather than 3D spatial points.
                    SlotTy::Field => errors.push(
                        IrError::contract(u.span, "`uses … : Field` is not an L5\u{2019}s")
                            .with_hint(
                            "remove it: a field is a distance at a *point in space*, and an L5 \
                             has only a frame coordinate — the viewpoint that would turn one \
                             into the other is exactly what an L5 has no way to name. March \
                             the field in a fullscreen L4 and hand this pass the picture",
                        ),
                    ),
                }
            }
            if !proc.emit.is_empty() || !proc.consumes.is_empty() {
                errors.push(
                    IrError::contract(
                        proc.span,
                        "`emit` and `consumes` are about elements, and an L5 has none",
                    )
                    .with_hint(
                        "remove them: an L5 reads the frame it is handed with `texel(src)` and \
                         `tap(src, uv)`, and the attributes of the elements that drew it are \
                         not something it can see",
                    ),
                );
            }
            if !proc.blocks.iter().any(|b| b.kind == BlockKind::Frame) {
                errors.push(
                    IrError::contract(proc.span, "L5 procedures require a `frame` block")
                        .with_hint("add `frame { … }`: it is the whole of what an L5 does"),
                );
            }
        }
    }

    // **`retains` is refused on every kind but L5**, at the declaration rather
    // than at the read, so that a procedure which declares it and never reads
    // `held` is turned away too: what it asks the engine for is a frame-sized
    // target, and nothing but a chain slot or a merge has a frame to retain.
    if let Some(r) = &proc.retains {
        if proc.kind != Kind::L5 {
            errors.push(
                IrError::contract(
                    r.span,
                    format!(
                        "`retains` is L5 only, and this is {} {}",
                        if matches!(proc.kind, Kind::Field) {
                            "a"
                        } else {
                            "an"
                        },
                        kind_name(proc.kind)
                    ),
                )
                .with_hint(
                    "remove `retains`: it says this procedure reads a retained cut of the \
                     previous *frame*, and a frame is what an L5 is handed",
                ),
            );
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
