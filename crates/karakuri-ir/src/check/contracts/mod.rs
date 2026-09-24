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

/// Returns true if `name` conflicts with a stage output writable by `kind`.
pub(crate) fn shadows_output(name: &str, kind: Kind) -> bool {
    Output::from_name(name).is_some_and(|o| output_block(o, kind).kind() == kind)
}

/// Returns the block kind that writes `output` for a procedure of kind `kind`.
pub(crate) fn output_block(output: Output, kind: Kind) -> BlockKind {
    match (output, kind) {
        (Output::Color, Kind::L5) => BlockKind::Frame,
        _ => output.block(),
    }
}

/// Validates that declared slot names do not conflict with reserved words, params, builtins, or other slots.
pub(crate) fn check_slot_names(proc: &Proc, errors: &mut Vec<IrError>) {
    for (at, u) in proc.uses.iter().enumerate() {
        check_reserved(&u.name, u.name_span, proc.kind, "slot", errors);
        // L5 reserves `src` and `held` in frame blocks.
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
        // Reject duplicate slot declarations.
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
        // Field slot names must not clash with builtins or type constructors.
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

/// Verifies that L1 procedures emit all consumed attributes (or emit sources for derived attributes).
pub(crate) fn check_consumes_emitted(
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
        // Handle derived attributes (e.g. velocity derived from position).
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
