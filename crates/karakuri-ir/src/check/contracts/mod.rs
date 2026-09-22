//! Semantic contracts and header validation.

use std::collections::{HashMap, HashSet};

use super::*;
use crate::ast::{
    Ambient, Attr, BlockKind, Kind, Output, Proc, SlotTy, Ty, TEXTURE_HELD, TEXTURE_SRC,
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

mod header;
pub(crate) use header::*;

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
