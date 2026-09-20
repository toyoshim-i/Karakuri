//! Pipeline planning, wiring validation, and dependency graph resolution.

use karakuri_ir::typed::{Checked, Slot};
use karakuri_ir::Kind;

use crate::set::types::{Edge, Layering, Plan, Set, SetError, Wiring, BUILTIN_CAMERA};

/// Returns the first duplicate string in the slice, if any.
pub fn first_duplicate(names: &[String]) -> Option<String> {
    names
        .iter()
        .enumerate()
        .find(|(at, name)| names[..*at].contains(name))
        .map(|(_, name)| name.clone())
}

/// Returns the default salt for a source derived from `seed_salt` and `source` index.
pub fn derived_salt(seed_salt: u32, source: usize) -> u32 {
    seed_salt.wrapping_add((source as u32).wrapping_mul(0x9E37_79B9))
}

/// Validates requested capacity against declared range in the L1 procedure.
pub fn capacity_in_range(l1: &Checked, capacity: u32) -> Result<(), SetError> {
    let range = l1
        .capacity
        .ok_or_else(|| SetError::NoCapacity(l1.name.clone()))?;
    if !range.contains(capacity) {
        return Err(SetError::Capacity {
            proc: l1.name.clone(),
            requested: capacity,
            min: range.min,
            max: range.max,
        });
    }
    Ok(())
}

/// Resolves concrete node names for all procedures in the Set, ensuring distinct names.
pub fn resolve_node_names(
    l1s: &[(&Checked, u32)],
    l2s: &[&Checked],
    cameras: &[Option<&Checked>],
    l4s: &[&Checked],
    fields: &[&Checked],
    wiring: &Wiring<'_>,
) -> Result<Vec<String>, SetError> {
    let given = |at: usize, from: &[Option<String>]| from.get(at).cloned().flatten();
    let wanted: Vec<(Option<String>, Option<&Checked>)> = l1s
        .iter()
        .enumerate()
        .map(|(at, (l1, _))| (given(at, wiring.l1s), Some(*l1)))
        .chain(
            l2s.iter()
                .enumerate()
                .map(|(at, n)| (given(at, wiring.l2s), Some(*n))),
        )
        .chain(
            cameras
                .iter()
                .enumerate()
                .map(|(at, n)| (given(at, wiring.l3s), *n)),
        )
        .chain(
            l4s.iter()
                .enumerate()
                .map(|(at, n)| (given(at, wiring.l4s), Some(*n))),
        )
        .chain(
            fields
                .iter()
                .enumerate()
                .map(|(at, n)| (given(at, wiring.fields), Some(*n))),
        )
        .collect();

    let mut taken: Vec<String> = wanted.iter().filter_map(|(n, _)| n.clone()).collect();
    if let Some(dup) = first_duplicate(&taken) {
        return Err(SetError::DuplicateNodeName { name: dup });
    }
    let names: Vec<String> = wanted
        .into_iter()
        .map(|(name, node)| match name {
            Some(written) => written,
            None => {
                let mut candidate = node.map_or(BUILTIN_CAMERA, |n| n.name.as_str()).to_string();
                let mut at = 1;
                while taken.contains(&candidate) {
                    at += 1;
                    candidate =
                        format!("{}-{at}", node.map_or(BUILTIN_CAMERA, |n| n.name.as_str()));
                }
                taken.push(candidate.clone());
                candidate
            }
        })
        .collect();
    Ok(names)
}

/// The validated wiring bindings: `(field_bound, camera_bound, source_bound, far_at)`.
pub type WiringBindings = (
    Vec<(usize, String, usize)>,
    Vec<(usize, usize)>,
    Vec<(usize, String, usize)>,
    Option<usize>,
);

/// Validates slot wirings and dependencies between nodes.
///
/// Returns `(field_bound, camera_bound, source_bound, far_at)`.
#[allow(clippy::too_many_arguments)]
pub fn validate_wiring<'a>(
    nodes: &[Option<&'a Checked>],
    names: &[String],
    edges: &[Edge],
    l1s: &[(&'a Checked, u32)],
    l2s: &[&'a Checked],
    fields: &[&'a Checked],
    camera_range: std::ops::Range<usize>,
    field_range: std::ops::Range<usize>,
) -> Result<WiringBindings, SetError> {
    let node_at = |name: &str| names.iter().position(|n| n == name);
    let geometry_at = |name: &str| node_at(name).filter(|at| *at < l1s.len());
    let holds = || names.join(", ");
    let sources = || names[..l1s.len()].join(", ");
    let camera_ordinal = |at: usize| camera_range.contains(&at).then(|| at - camera_range.start);
    let holds_cameras = || names[camera_range.clone()].join(", ");
    let field_ordinal = |at: usize| field_range.contains(&at).then(|| at - field_range.start);
    let holds_fields = || match field_range.is_empty() {
        true => "none — this Set holds no `kind Field` procedure".to_string(),
        false => names[field_range.clone()].join(", "),
    };
    let layer_of = |at: usize| -> &'static str {
        if at < l1s.len() {
            "an L1"
        } else if at < l1s.len() + l2s.len() {
            "an L2"
        } else if camera_range.contains(&at) {
            "a camera"
        } else if field_range.contains(&at) {
            "a field"
        } else {
            "an L4"
        }
    };

    // Validate that all edges targeting nodes in this Set connect to declared input slots.
    for edge in edges {
        let Some(at) = node_at(&edge.node) else {
            continue;
        };
        let declared: &[Slot] = nodes[at].map_or(&[], |n| n.uses.as_slice());
        if !declared.iter().any(|slot| slot.name == edge.slot) {
            return Err(SetError::NoSuchSlot {
                node: edge.node.clone(),
                slot: edge.slot.to_string(),
                declares: match declared.is_empty() {
                    true => String::new(),
                    false => format!(
                        "; `{}` declares {}",
                        edge.node,
                        declared
                            .iter()
                            .map(|s| format!("`{}`", s.name))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                },
            });
        }
    }

    // Validate pairing geometry slot and far geometry binding.
    let pairing = l2s.iter().position(|n| n.geometry_slot().is_some());
    let far_at: Option<usize> = match pairing {
        None => None,
        Some(at) => {
            let l2 = l2s[at];
            let node = names[l1s.len() + at].clone();
            let slot = l2
                .geometry_slot()
                .expect("`pairing` is the position of a node that declares a slot")
                .to_string();
            if at != 0 {
                return Err(SetError::PairingNotFirst {
                    l2: l2.name.clone(),
                    at,
                });
            }
            if l1s.len() != 2 {
                return Err(SetError::PairingArity {
                    l2: l2.name.clone(),
                    sources: l1s.len(),
                });
            }
            let mut bound = edges
                .iter()
                .filter(|e| e.node == node && e.slot.as_str() == slot);
            let Some(edge) = bound.next() else {
                return Err(SetError::SlotUnbound {
                    node,
                    slot,
                    takes: karakuri_ir::SlotTy::Geometry.name(),
                    holds: holds(),
                });
            };
            if let Some(second) = bound.next() {
                return Err(SetError::SlotBoundTwice {
                    node,
                    slot,
                    first: edge.to.clone(),
                    second: second.to.clone(),
                });
            }
            let Some(far_at) = geometry_at(&edge.to) else {
                return Err(match node_at(&edge.to) {
                    Some(other) => SetError::EdgeToNotGeometry {
                        node,
                        slot,
                        to: edge.to.clone(),
                        layer: match other < l1s.len() + l2s.len() {
                            true => "an L2",
                            false => "a node that holds no elements",
                        },
                        sources: sources(),
                    },
                    None => SetError::EdgeToUnknown {
                        node,
                        slot,
                        to: edge.to.clone(),
                        holds: holds(),
                    },
                });
            };
            for (l1, _) in l1s {
                if !l1.is_static() {
                    return Err(SetError::PairingNotStatic {
                        l2: l2.name.clone(),
                        l1: l1.name.clone(),
                    });
                }
            }
            let near_at = (0..l1s.len())
                .find(|at| *at != far_at)
                .expect("two sources, one of them bound");
            if l1s[near_at].1 != l1s[far_at].1 {
                return Err(SetError::PairingCapacity {
                    l2: l2.name.clone(),
                    a: l1s[near_at].1,
                    b: l1s[far_at].1,
                });
            }
            Some(far_at)
        }
    };

    // Validate and record bindings for all Field slots across nodes.
    let mut field_bound: Vec<(usize, String, usize)> = Vec::new();
    for (at, node) in nodes.iter().enumerate() {
        let Some(node) = node else { continue };
        for slot in node
            .uses
            .iter()
            .filter(|s| s.ty == karakuri_ir::SlotTy::Field)
        {
            let node_name = names[at].clone();
            let mut bound = edges
                .iter()
                .filter(|e| e.node == node_name && e.slot == slot.name);
            let Some(edge) = bound.next() else {
                return Err(SetError::SlotUnbound {
                    node: node_name,
                    slot: slot.name.to_string(),
                    takes: slot.ty.name(),
                    holds: holds(),
                });
            };
            if let Some(second) = bound.next() {
                return Err(SetError::SlotBoundTwice {
                    node: node_name,
                    slot: slot.name.to_string(),
                    first: edge.to.clone(),
                    second: second.to.clone(),
                });
            }
            match node_at(&edge.to).map(|to| (to, field_ordinal(to))) {
                Some((_, Some(ordinal))) => {
                    field_bound.push((at, slot.name.to_string(), ordinal));
                }
                Some((other, None)) => {
                    return Err(SetError::EdgeToNotField {
                        node: node_name,
                        slot: slot.name.to_string(),
                        to: edge.to.clone(),
                        layer: layer_of(other),
                        fields: holds_fields(),
                    })
                }
                None => {
                    return Err(SetError::EdgeToUnknown {
                        node: node_name,
                        slot: slot.name.to_string(),
                        to: edge.to.clone(),
                        holds: holds(),
                    })
                }
            }
        }
    }

    // Validate and record bindings for all Camera slots across nodes.
    let mut camera_bound: Vec<(usize, usize)> = Vec::new();
    for (at, node) in nodes.iter().enumerate() {
        let Some(node) = node else { continue };
        for slot in node
            .uses
            .iter()
            .filter(|s| s.ty == karakuri_ir::SlotTy::Camera)
        {
            let node_name = names[at].clone();
            let mut bound = edges
                .iter()
                .filter(|e| e.node == node_name && e.slot == slot.name);
            let Some(edge) = bound.next() else {
                return Err(SetError::SlotUnbound {
                    node: node_name,
                    slot: slot.name.to_string(),
                    takes: slot.ty.name(),
                    holds: holds(),
                });
            };
            if let Some(second) = bound.next() {
                return Err(SetError::SlotBoundTwice {
                    node: node_name,
                    slot: slot.name.to_string(),
                    first: edge.to.clone(),
                    second: second.to.clone(),
                });
            }
            match node_at(&edge.to).map(|to| (to, camera_ordinal(to))) {
                Some((_, Some(ordinal))) => camera_bound.push((at, ordinal)),
                Some((other, None)) => {
                    return Err(SetError::EdgeToNotCamera {
                        node: node_name,
                        slot: slot.name.to_string(),
                        to: edge.to.clone(),
                        layer: layer_of(other),
                        cameras: holds_cameras(),
                    })
                }
                None => {
                    return Err(SetError::EdgeToUnknown {
                        node: node_name,
                        slot: slot.name.to_string(),
                        to: edge.to.clone(),
                        holds: holds(),
                    })
                }
            }
        }
    }

    // Validate and record bindings for all Source slots across nodes.
    let mut source_bound: Vec<(usize, String, usize)> = Vec::new();
    for (at, node) in nodes.iter().enumerate() {
        let Some(node) = node else { continue };
        for slot in node
            .uses
            .iter()
            .filter(|s| s.ty == karakuri_ir::SlotTy::Source)
        {
            let node_name = names[at].clone();
            let mut bound = edges
                .iter()
                .filter(|e| e.node == node_name && e.slot == slot.name);
            let Some(edge) = bound.next() else {
                return Err(SetError::SlotUnbound {
                    node: node_name,
                    slot: slot.name.to_string(),
                    takes: slot.ty.name(),
                    holds: holds(),
                });
            };
            if let Some(second) = bound.next() {
                return Err(SetError::SlotBoundTwice {
                    node: node_name,
                    slot: slot.name.to_string(),
                    first: edge.to.clone(),
                    second: second.to.clone(),
                });
            }
            match node_at(&edge.to) {
                Some(to) => match geometry_at(&edge.to) {
                    Some(l1_at) => source_bound.push((at, slot.name.to_string(), l1_at)),
                    None => {
                        return Err(SetError::EdgeToNotSource {
                            node: node_name,
                            slot: slot.name.to_string(),
                            to: edge.to.clone(),
                            layer: layer_of(to),
                            sources: sources(),
                        })
                    }
                },
                None => {
                    return Err(SetError::EdgeToUnknown {
                        node: node_name,
                        slot: slot.name.to_string(),
                        to: edge.to.clone(),
                        holds: holds(),
                    })
                }
            }
        }
    }

    // Validate Field instruction budgets.
    if !fields.is_empty() {
        let per_evaluation: Vec<u64> = fields
            .iter()
            .map(|f| {
                karakuri_ir::cost::estimate(f)
                    .map(|c| c.ops_per_evaluation)
                    .unwrap_or(0)
            })
            .collect();
        for (at, caller) in nodes
            .iter()
            .enumerate()
            .filter(|(at, _)| !field_range.contains(at))
            .filter_map(|(at, n)| n.map(|n| (at, n)))
        {
            let per_slot = |slot: &str| {
                field_bound
                    .iter()
                    .find(|(node, name, _)| *node == at && name == slot)
                    .map(|(_, _, ordinal)| per_evaluation[*ordinal])
                    .unwrap_or(0)
            };
            if let Err(over) = karakuri_ir::cost::check_with_field(caller, &per_slot) {
                let blamed = field_bound
                    .iter()
                    .find(|(node, name, _)| *node == at && *name == over.slot)
                    .map(|(_, _, ordinal)| fields[*ordinal].name.clone())
                    .unwrap_or_default();
                return Err(SetError::FieldTooExpensive {
                    caller: caller.name.clone(),
                    slot: over.slot,
                    field: blamed,
                    detail: over
                        .errors
                        .first()
                        .map(|e| e.message.clone())
                        .unwrap_or_default(),
                });
            }
        }
    }

    Ok((field_bound, camera_bound, source_bound, far_at))
}

/// Arguments for planning geometry sources and deriving attributes.
#[derive(Clone, Copy)]
pub struct PlanSourcesCtx<'a> {
    pub l1s: &'a [(&'a Checked, u32)],
    pub l2s: &'a [&'a Checked],
    pub l4s: &'a [&'a Checked],
    pub heads: &'a [usize],
    pub far_at: Option<usize>,
    pub pairing: Option<usize>,
    pub salts: &'a [Option<u32>],
    pub seed_salt: u32,
}

/// Plans geometry sources, checks attribute derivations and compositions, and assigns salts.
///
/// Returns `(source_salts, derived_per_head)`.
pub fn plan_sources<'a>(
    ctx: PlanSourcesCtx<'a>,
) -> Result<(Vec<u32>, Vec<Vec<karakuri_ir::Attr>>), SetError> {
    let PlanSourcesCtx {
        l1s,
        l2s,
        l4s,
        heads,
        far_at,
        pairing,
        salts,
        seed_salt,
    } = ctx;
    let salt_of = |at: usize| -> u32 {
        salts
            .get(at)
            .copied()
            .flatten()
            .unwrap_or_else(|| derived_salt(seed_salt, at))
    };
    let source_salts: Vec<u32> = (0..l1s.len()).map(salt_of).collect();
    let mut derived_per_head: Vec<Vec<karakuri_ir::Attr>> = Vec::with_capacity(heads.len());

    for &at in heads {
        let (l1, capacity) = l1s[at];
        let emitted: Vec<karakuri_ir::Attr> = l1
            .emit
            .iter()
            .chain(l2s.iter().flat_map(|n| n.emit.iter()))
            .copied()
            .collect();
        let mut derived: Vec<karakuri_ir::Attr> = Vec::new();
        let mut blocked: Vec<(karakuri_ir::Attr, karakuri_ir::Attr)> = Vec::new();
        {
            let mut seen: Vec<karakuri_ir::Attr> = l1.emit.clone();
            for node in std::iter::once(&l1).chain(l2s.iter()).chain(l4s.iter()) {
                for &attr in &node.consumes {
                    if seen.contains(&attr) || derived.contains(&attr) || emitted.contains(&attr) {
                        continue;
                    }
                    let Some(rule) = attr.derivation() else {
                        continue;
                    };
                    if let Some(from) = rule.source().filter(|from| !l1.emit.contains(from)) {
                        blocked.push((attr, from));
                        continue;
                    }
                    derived.push(attr);
                }
                for &attr in &node.emit {
                    if !seen.contains(&attr) {
                        seen.push(attr);
                    }
                }
            }
        }

        let mut available: Vec<karakuri_ir::Attr> = l1.emit.clone();
        available.extend(derived.iter().copied());
        let check_against = |node: &Checked, available: &[karakuri_ir::Attr]| {
            let missing: Vec<String> = node
                .consumes
                .iter()
                .filter(|a| !available.contains(a))
                .map(|a| format!("`{}`", a.name()))
                .collect();
            if missing.is_empty() {
                return None;
            }
            let hint = node
                .consumes
                .iter()
                .find_map(|a| blocked.iter().find(|(attr, _)| attr == a))
                .map(|(attr, from)| {
                    format!(
                        "`{}` is synthesised from `{}`, and `{}` emits neither. Add `{}` to \
                         `{}`'s `emit` and `{}` follows",
                        attr.name(),
                        from.name(),
                        l1.name,
                        from.name(),
                        l1.name,
                        attr.name()
                    )
                })
                .unwrap_or_else(|| {
                    format!(
                        "add {} to `{}`'s `emit`, or pair `{}` with an L1 that emits it. \
                         `age` and `velocity` are synthesised where nothing emits them; nothing \
                         else is",
                        missing.join(", "),
                        l1.name,
                        node.name
                    )
                });
            Some(SetError::Composition {
                l1: l1.name.clone(),
                l4: node.name.clone(),
                missing: missing.join(", "),
                hint,
            })
        };
        for l2 in l2s {
            if l2.kind != Kind::L2 {
                return Err(SetError::WrongKind {
                    slot: "L2",
                    expected: Kind::L2,
                    actual: l2.kind,
                });
            }
            if let Some(e) = check_against(l2, &available) {
                return Err(e);
            }
            for &attr in &l2.emit {
                if !available.contains(&attr) {
                    available.push(attr);
                }
            }
        }
        for l4 in l4s {
            if l4.kind != Kind::L4 {
                return Err(SetError::WrongKind {
                    slot: "L4",
                    expected: Kind::L4,
                    actual: l4.kind,
                });
            }
            if let Some(e) = check_against(l4, &available) {
                return Err(e);
            }
        }

        if let [only] = l4s {
            if only.blend == Some(karakuri_ir::Blend::Weighted)
                && only.topology == Some(karakuri_ir::Topology::Fullscreen)
            {
                return Err(SetError::WeightedFullscreen {
                    l4: only.name.clone(),
                });
            }
        }

        capacity_in_range(l1, capacity)?;
        if let Some(far_at) = far_at {
            let (far, far_capacity) = l1s[far_at];
            for attr in &derived {
                if attr
                    .derivation()
                    .and_then(|d| d.source())
                    .is_some_and(|from| !far.emit.contains(&from))
                {
                    return Err(SetError::PairingDerivation {
                        l2: l2s[pairing.expect("a bound geometry is a node that declared a slot")]
                            .name
                            .clone(),
                        l1: far.name.clone(),
                        attr: attr.name().to_string(),
                    });
                }
            }
            capacity_in_range(far, far_capacity)?;
        }
        derived_per_head.push(derived);
    }

    Ok((source_salts, derived_per_head))
}

impl Set {
    /// Validates procedure compatibility, topology, and slot bindings without requiring a GPU device.
    ///
    /// Returns a [`Plan`] containing resolved node names, bindings, and memory allocations.
    #[allow(clippy::too_many_arguments)]
    pub fn validate<'a>(
        l1s: &[(&'a Checked, u32)],
        l2s: &[&'a Checked],
        l3s: &[&'a Checked],
        fields: &[&'a Checked],
        l4s: &[&'a Checked],
        layering: Layering,
        seed_salt: u32,
        salts: &[Option<u32>],
        wiring: Wiring<'_>,
    ) -> Result<Plan<'a>, SetError> {
        let Some(&(first_l1, _)) = l1s.first() else {
            return Err(SetError::NoGeometry);
        };
        if l4s.is_empty() {
            return Err(SetError::NoRenderer {
                l1: first_l1.name.clone(),
            });
        }
        if layering == Layering::Composite && l4s.len() > crate::deck::MAX_SLOTS {
            return Err(SetError::TooManyInputs {
                l1: first_l1.name.clone(),
                count: l4s.len(),
                max: crate::deck::MAX_SLOTS,
            });
        }

        // Camera nodes: one per L3 procedure, ending with the built-in orbit camera.
        let cameras: Vec<Option<&Checked>> = l3s
            .iter()
            .copied()
            .map(Some)
            .chain(std::iter::once(None))
            .collect();

        let names = resolve_node_names(l1s, l2s, &cameras, l4s, fields, &wiring)?;

        let nodes: Vec<Option<&Checked>> = l1s
            .iter()
            .map(|(l1, _)| Some(*l1))
            .chain(l2s.iter().copied().map(Some))
            .chain(cameras.iter().copied())
            .chain(l4s.iter().copied().map(Some))
            .chain(fields.iter().copied().map(Some))
            .collect();

        let field_range = names.len() - fields.len()..names.len();
        let camera_range = l1s.len() + l2s.len()..l1s.len() + l2s.len() + cameras.len();

        let (field_bound, camera_bound, source_bound, far_at) = validate_wiring(
            &nodes,
            &names,
            wiring.edges,
            l1s,
            l2s,
            fields,
            camera_range.clone(),
            field_range,
        )?;

        for (l1, _) in l1s {
            if l1.kind != Kind::L1 {
                return Err(SetError::WrongKind {
                    slot: "L1",
                    expected: Kind::L1,
                    actual: l1.kind,
                });
            }
        }
        for f in fields {
            if f.kind != Kind::Field {
                return Err(SetError::WrongKind {
                    slot: "Field",
                    expected: Kind::Field,
                    actual: f.kind,
                });
            }
        }
        for l3 in l3s {
            if l3.kind != Kind::L3 {
                return Err(SetError::WrongKind {
                    slot: "L3",
                    expected: Kind::L3,
                    actual: l3.kind,
                });
            }
        }

        let heads: Vec<usize> = (0..l1s.len()).filter(|at| Some(*at) != far_at).collect();
        let pairing = l2s.iter().position(|n| n.geometry_slot().is_some());

        let (source_salts, derived) = plan_sources(PlanSourcesCtx {
            l1s,
            l2s,
            l4s,
            heads: &heads,
            far_at,
            pairing,
            salts,
            seed_salt,
        })?;

        Ok(Plan {
            names,
            cameras,
            camera_range,
            field_bound,
            camera_bound,
            source_bound,
            far_at,
            heads,
            source_salts,
            derived,
            l1s: l1s.to_vec(),
            l2s: l2s.to_vec(),
            fields: fields.to_vec(),
        })
    }
}
