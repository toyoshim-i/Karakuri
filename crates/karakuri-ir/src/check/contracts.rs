//! Semantic contracts and header validation.

use std::collections::{HashMap, HashSet};

use super::*;
use crate::ast::{
    Ambient, Attr, BlockKind, Kind, Output, Proc, SlotTy, Topology, Ty, TEXTURE_HELD, TEXTURE_SRC,
};
use crate::builtin::Builtin;
use crate::error::IrError;
use crate::span::Span;

pub(crate) fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::L1 => "L1",
        Kind::L2 => "L2",
        Kind::L3 => "L3",
        Kind::L4 => "L4",
        Kind::Field => "Field",
        Kind::L5 => "L5",
    }
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

/// A `uses … : Texture` slot on a kind that is handed no picture.
///
/// One function rather than five copies, because the refusal is one sentence
/// with one clause that varies: a texture is what an L5 folds, and every other
/// kind is handed elements or a position. `why` is that clause.
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
                    // **Legal here, and on L2 and L4** — the three kinds a Set
                    // instantiates per source, which is exactly where `source`
                    // is readable. What a Source slot binds is a `u32` in the
                    // uniform this module already has, so it adds an input to
                    // the file and no buffer, no bind group and nothing to the
                    // chain.
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
                    // **The same refusal `source` itself gets here, one level
                    // up.** A Source slot exists to be compared against
                    // `source`, and an L3 has no `source` to compare: it runs
                    // once a frame over nothing, and the identity is a property
                    // of a Set's geometry, which a camera is not. Refused at
                    // the declaration rather than left to the read, so that a
                    // camera which declares one and never reads it is turned
                    // away too — the Set would otherwise bind an edge to a
                    // value nothing here could ever use.
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
                    // **And a field has no `source` either**, for the reason it
                    // has no `seed`: it is spliced into every procedure that
                    // declares a slot for it, and two of those may be running
                    // over two different geometries — so a distance that varied
                    // with the source would be two shapes in one frame, out of
                    // one body.
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
        // **An L5 is handed a picture and declares nothing about the material
        // that made it.** Every geometry declaration is refused at the header,
        // on the terms every misplaced declaration is refused: an L5 counts
        // nothing, spawns nothing, kills nothing and stores nothing.
        //
        // **There is no `vertex` block to refuse here**, and that is the
        // block-owner check below rather than an omission: `vertex` belongs to
        // L4, so a `frame` procedure that declares one is turned away with a
        // sentence about which kind owns it. The absence of a vertex stage is
        // not a *declaration* on an L5 the way it is on an L4 — an L5 has no
        // per-element form for it to be a declaration against.
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
                    // **The one kind that may declare one**, and this is the
                    // nested role's fan-in: several pictures folded into the
                    // one `Texture` a Set outputs, each bound by an `edge`.
                    // Any number of them — a fold of three is as ordinary as a
                    // fold of two, and each costs one texture binding.
                    SlotTy::Texture => {}
                    // **All three are element-level, and an L5 is handed a
                    // picture rather than the material that made it.** One
                    // sentence for the three because it is one sentence: by the
                    // time a frame exists, the elements that drew it are gone
                    // and the camera they were seen from is one of possibly
                    // several.
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
                    // **The one refusal here that has to be argued rather than
                    // followed.** The four kinds that may evaluate a field are
                    // the four that have a position in space to evaluate it at.
                    // An L5 has a frame coordinate; `eye` and `ray` are refused
                    // above; and a field marched from a viewpoint an L5 cannot
                    // name would be a shape drawn against nothing.
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

/// Whether `name` is a stage output this kind of procedure can write.
///
/// Scoped to the layer, not global. `Output` grew from four names to ten when
/// the camera arrived, and a global reservation would have made `up`, `target`,
/// `near`, `far` and `fov_y` illegal as params and locals in *every* layer —
/// `let near = length(position)` in a marcher, `let up` in an L1, both
/// previously legal and neither shadowing anything reachable there. A name is
/// only ambiguous where the thing it names exists, and the diagnostic for the
/// global version named a block the procedure did not have.
///
/// `eye` stays refused everywhere, but as an *ambient* rather than an output —
/// a marching fragment reads it, so it genuinely is in scope in an L4.
pub(crate) fn shadows_output(name: &str, kind: Kind) -> bool {
    Output::from_name(name).is_some_and(|o| output_block(o, kind).kind() == kind)
}

/// Which block writes `output` in a procedure of this kind.
///
/// [`Output::block`] answers for the four kinds that had one each, and `color`
/// is now written by two: a `fragment` block on an L4 and a `frame` block on an
/// L5. It is the same output — `vec4`, linear, unclamped, the same value at the
/// next node down — which is exactly why it is one name rather than two, and
/// why this is a redirection here rather than a fifth `Output` variant.
pub(crate) fn output_block(output: Output, kind: Kind) -> BlockKind {
    match (output, kind) {
        (Output::Color, Kind::L5) => BlockKind::Frame,
        _ => output.block(),
    }
}

/// Attributes, ambients, and stage outputs are a closed, reserved vocabulary
/// that no param, local or declared geometry slot may take on — see the module
/// docs on shadowing.
///
/// A slot name goes through here for the same reason a param's does: it is read
/// the way a local is — `far.position`, `shape(p)` — so one spelling would
/// otherwise mean two things depending on what follows it. What it is *not* is
/// a reserved word of its own — the name belongs to the procedure that declared
/// it, and reserving one language-wide is what caps a procedure at one input.
///
/// A *callable* slot has one more collision than this function knows about, and
/// it is checked beside the call in [`check_slot_names`] rather than added
/// here: a param named `sin` is still a perfectly good param. What every slot
/// name has to be, whatever type it was declared with.
///
/// Outside the per-kind arms above, because it is not a rule about a kind: a
/// Field slot is legal on four of the five, so a check that lived in L2's arm
/// would leave the other three able to declare a slot called `position` — and
/// the arm it lived in is exactly where nobody would look for it.
pub(crate) fn check_slot_names(proc: &Proc, errors: &mut Vec<IrError>) {
    for (at, u) in proc.uses.iter().enumerate() {
        // **The slot name shares one scope with everything else nameable
        // here.** It is read the way a local is — `far.position`, `shape(p)` —
        // so a slot called `position` or `t` would make one spelling mean two
        // things depending on what follows it.
        check_reserved(&u.name, u.name_span, proc.kind, "slot", errors);
        // **The two names an L5's `frame` block already holds.** `src` is the
        // incoming picture and `held` is the retained one, and both are fetched
        // the way a Texture slot is — so a slot called either would be one
        // spelling for two bindings, in the one kind where a Texture slot is
        // legal at all. Asked of the kind rather than reserved language-wide,
        // because `src` is an ordinary name in every other layer.
        if proc.kind == Kind::L5 && matches!(u.name.as_str(), TEXTURE_SRC | TEXTURE_HELD) {
            errors.push(
                IrError::contract(
                    u.name_span,
                    format!("`{}` is a texture an L5 is already handed", u.name),
                )
                .with_hint(format!(
                    "rename the slot: `{}` names {} in a `frame` block, and `texel`/`tap` \
                     would have one name for two pictures",
                    u.name,
                    if u.name == TEXTURE_SRC {
                        "the incoming frame"
                    } else {
                        "the retained cut of the previous frame"
                    }
                )),
            );
        }
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

pub(crate) fn check_reserved(
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

pub(crate) fn check_params(proc: &Proc, errors: &mut Vec<IrError>) -> HashMap<String, Ty> {
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
            &[],
            &[],
            false,
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

/// The declaration-order list is kept alongside the set, spans and all. The set
/// answers "is this attribute declared"; the list is what anything that reports
/// or generates walks, because both need a fixed order — buffer slot order
/// comes from it, and so does the order diagnostics come out in.
pub(crate) fn dedup_attrs(
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
/// is not the same as `consumes ⊆ emit`.
///
/// Two attributes have a rule: `age` and `velocity`. An L1 may consume either
/// without emitting it, and the engine provides it — see `docs/ir-spec.md`,
/// "Attribute derivation". So what is checked here is narrower than it was and
/// is genuinely a one-file question: `velocity` is synthesised from `position`,
/// and whether *this* procedure emits `position` is something one file can
/// answer. Whether anybody emits `velocity` is not, and that half moved to
/// `Set::build_many`, which is the first point holding every procedure at once.
///
/// L1 only, for the reason it always was: an L1 is the whole of what is
/// available to it, where an L2 and an L4 read what is available at their
/// position in a chain.
pub(crate) fn check_consumes_emitted(
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
