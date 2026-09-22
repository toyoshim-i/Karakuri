use super::common::*;

// ---------------------------------------------------------------------------
// `kind Field`: a spatial function, and the only kind with no pass of its own.
// ---------------------------------------------------------------------------

const BLOB: &str = r#"
proc blob {
  kind Field

  param ball : float [0.1, 2.0] = 0.8

  field {
    let sph = sd_sphere(point - vec3(0.9, 0.0, 0.0), ball);
    let bx  = sd_box(point + vec3(0.9, 0.0, 0.0), vec3(0.7, 0.7, 0.7));
    distance = op_smooth_union(sph, bx, 0.55);
  }
}
"#;

/// A field is a procedure like any other — a `kind`, a block, `param`s an
/// operator rides — and declares nothing about geometry, because it is not
/// geometry.
#[test]
fn a_field_checks_clean_and_carries_no_geometry() {
    let checked = check_ok(BLOB);
    assert_eq!(checked.kind, Kind::Field);
    assert_eq!(
        checked.topology, None,
        "a field is a function, not geometry"
    );
    assert!(checked.emit.is_empty() && checked.consumes.is_empty());
    assert_eq!(checked.params.len(), 1);
    assert!(checked.block(BlockKind::Field).is_some());
}

/// **`point` is a field's only input, and no other block has one.** Every other
/// block is handed an element or a fragment; this one is handed a position.
#[test]
fn point_is_readable_in_a_field_block_and_nowhere_else() {
    check_ok(BLOB);

    let errs = check_err(
        r#"
proc reads_point {
  kind     L1
  topology points
  capacity [1, 1] = 1

  emit position

  element {
    position = point;
  }
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.message.contains("point")),
        "expected `point` to be refused outside a field, got: {errs:?}"
    );
}

/// `distance` is the whole of what a field produces, so a field that assigns
/// nothing is a function with no return value.
#[test]
fn a_field_must_assign_distance() {
    let errs = check_err(
        r#"
proc silent {
  kind Field

  field {
    let d = sd_sphere(point, 1.0);
  }
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.message.contains("distance")),
        "expected `distance` to be required, got: {errs:?}"
    );
}

/// **A field has no element**, so nothing an element carries is readable in
/// one. The diagnostic says why rather than telling the author to declare it,
/// because declaring it is not available and would not help.
#[test]
fn attributes_are_refused_in_a_field_block() {
    let errs = check_err(
        r#"
proc peeks {
  kind Field

  field {
    distance = length(position) - 1.0;
  }
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.message.contains("position")
            && e.hint
                .as_deref()
                .unwrap_or_default()
                .contains("function of space")),
        "expected a diagnostic explaining a field has no element, got: {errs:?}"
    );
}

/// **A renderer emits nothing**, and it was the one layer that did not say so.
///
/// An L3 and a field both refuse `emit` by name. An L4's was accepted, given no
/// buffer, and then read back by `is_closed_form` — which states "vacuously
/// true for L4" partly to stop a stray `emit` making a Set look accumulating
/// and dragging it into needing to be primed. That is a compensation for a
/// declaration that should not have parsed.
#[test]
fn emit_is_refused_on_a_renderer() {
    let errs = check_err(
        r#"
proc draws {
  kind L4
  blend additive

  emit position

  fragment {
    color = vec4(1.0);
  }
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.message.contains("emit")
            && e.hint.as_deref().unwrap_or_default().contains("consumes")),
        "expected `emit` to be refused on an L4, and `consumes` offered, got: {errs:?}"
    );
}

/// **`seed` is per element, and a field has none** — the same sentence
/// `attributes_are_refused_in_a_field_block` states, for the one per-element
/// value that is not an attribute.
///
/// It needed its own test for the reason it needed its own rule: `seed` is
/// available in every block *an element reaches*, and that sentence is
/// falsified twice — by a fullscreen L4 and by a field. The first was found by
/// running it; this one checked clean and reached `FieldResolver::read_seed`,
/// which is an `unreachable!`, so a `.kir` nobody could see anything wrong with
/// panicked the thread that compiled it.
#[test]
fn seed_is_refused_in_a_field_block() {
    let errs = check_err(
        r#"
proc jitter {
  kind Field

  field {
    distance = length(point) - 1.0 + hash1(seed) * 0.01;
  }
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.message.contains("seed")
            && e.hint
                .as_deref()
                .unwrap_or_default()
                .contains("function of space")),
        "expected `seed` to be refused with a field's own reason, got: {errs:?}"
    );
}

/// Every geometry declaration is refused where it is written, on the same terms
/// an L3 refuses them: a field counts nothing, draws nothing and carries
/// nothing.
#[test]
fn a_field_refuses_every_geometry_declaration() {
    for (decl, word) in [
        ("capacity [1, 8] = 4", "capacity"),
        ("topology points", "topology"),
        ("blend additive", "blend"),
        ("amplify 4", "amplify"),
    ] {
        let src = format!(
            r#"
proc wrong {{
  kind Field
  {decl}

  field {{
    distance = length(point) - 1.0;
  }}
}}
"#
        );
        let errs = check_err(&src);
        assert!(
            errs.iter().any(|e| e.message.contains(word)),
            "expected `{word}` to be refused on a field, got: {errs:?}"
        );
    }

    let errs = check_err(
        r#"
proc wrong {
  kind Field

  emit position

  field {
    distance = length(point) - 1.0;
  }
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.message.contains("emit")),
        "expected `emit` to be refused on a field, got: {errs:?}"
    );
}

/// Asserts that a Field procedure rejects a Geometry slot declaration.
#[test]
fn a_field_refuses_a_geometry_slot() {
    let errs = check_err(
        r#"
proc wrong {
  kind Field

  uses far : Geometry

  field {
    distance = length(point) - 1.0;
  }
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.message.contains("`uses` is L2 only")),
        "expected `uses` to be refused on a field, got: {errs:?}"
    );
}

/// **`is_static` asks whether anything ever moves an element between slots**,
/// which is two questions and used to be one.
///
/// A `spawn` block allocates; `kill()` makes the next step's scan compact the
/// survivors down. Either one and `seed` stops being the slot index — which is
/// the property a caller wants it for, and which the `kill`-only case
/// falsified while the answer stayed true.
#[test]
fn a_procedure_that_kills_is_not_static() {
    let lattice = r#"
proc lattice {
  kind     L1
  topology points
  capacity [64, 64] = 64

  emit position

  element {
    position = vec3(float(seed % 8u), float(seed / 8u), 0.0);
  }
}
"#;
    assert!(
        check_ok(lattice).is_static(),
        "nothing spawns and nothing dies"
    );

    // The same procedure, killing. Every element is live at frame zero and not
    // after it, so the old sentence was true and the answer was wrong.
    let culled = lattice.replace(
        "    position = vec3(float(seed % 8u), float(seed / 8u), 0.0);",
        "    position = vec3(float(seed % 8u), float(seed / 8u), 0.0);\n    if seed == 3u { kill(); }",
    );
    assert!(
        !check_ok(&culled).is_static(),
        "a `kill()` compacts, and compaction moves every element after the gap"
    );

    let spawning = r#"
proc fountain {
  kind     L1
  topology points
  capacity [64, 64] = 64

  param spawn_rate : float [0.0, 100.0] = 10.0

  emit position

  spawn   { position = vec3(0.0, 0.0, 0.0); }
  element { position = position; }
}
"#;
    assert!(
        !check_ok(spawning).is_static(),
        "and a `spawn` block allocates"
    );
}

// ---------------------------------------------------------------------------
// `uses`: an L2 that declares a named geometry input.
// ---------------------------------------------------------------------------

pub(crate) const MORPH: &str = r#"
proc morph {
  kind L2
  uses far : Geometry

  param k : float [0.0, 1.0] = 0.5

  consumes position

  deform {
    position = mix(position, far.position, vec3(k, k, k));
  }
}
"#;

/// Asserts that a used Geometry slot is checked and recorded on `Checked`.
#[test]
fn a_used_geometry_checks_clean_and_carries_its_name() {
    let checked = check_ok(MORPH);
    assert_eq!(
        checked.uses,
        vec![Slot {
            name: "far".into(),
            ty: SlotTy::Geometry,
        }]
    );
    assert_eq!(checked.geometry_slot(), Some("far"));
    assert_eq!(checked.kind, Kind::L2);

    let deform = checked.block(BlockKind::Deform).expect("a deform");
    let reads_far = |stmts: &[TStmt]| {
        stmts.iter().any(|s| match s {
            TStmt::Assign { value, .. } => format!("{value:?}").contains("Far"),
            _ => false,
        })
    };
    assert!(reads_far(&deform.stmts), "`far.position` is a far read");
}

/// **The base name is the procedure's own**, so a read through a name it did
/// not declare resolves to nothing rather than to the second geometry.
///
/// This is the whole difference from the reserved `other` this replaced: a
/// spelling means the far side because the header said so, not because the
/// language reserved a word — which is what let there be only ever one.
#[test]
fn a_read_through_an_undeclared_name_is_refused() {
    let errs = check_err(&MORPH.replace("  uses far : Geometry\n", ""));
    assert!(
        errs.iter().any(|e| e.message.contains("`far`")),
        "expected the name to be reported as resolving to nothing, got: {errs:?}"
    );

    // And a procedure that declares one slot does not get a second by writing
    // a different name.
    let errs = check_err(&MORPH.replace("far.position", "near.position"));
    assert!(
        errs.iter().any(|e| e.message.contains("`near`")),
        "expected `near` to resolve to nothing, got: {errs:?}"
    );
}

/// **One `consumes` covers both sides.** The far geometry is an input edge, and
/// a node reads the same attribute from each — so an attribute this node does
/// not take is not readable on either side.
#[test]
fn a_far_read_takes_only_what_the_node_consumes() {
    let errs = check_err(&MORPH.replace("far.position", "far.tint"));
    assert!(
        errs.iter()
            .any(|e| e.message.contains("tint") && e.message.contains("not consumed")),
        "expected `far.tint` to need `tint` in `consumes`, got: {errs:?}"
    );
}

/// **A geometry is not a value.** `far` alone is the whole second source, which
/// this language has no type for and no way to pass — so the one thing that can
/// be said about it is what one of its elements holds.
///
/// The complement of the read above: the new name appears in expression
/// position, and every position it can appear in either produces a value or is
/// refused with its own reason.
#[test]
fn a_used_geometry_is_not_a_value_on_its_own() {
    let errs = check_err(&MORPH.replace("far.position", "far"));
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`far` is a geometry, not a value")),
        "expected a used geometry to be unreadable as a value, got: {errs:?}"
    );
}

/// **And it is not assignable.** The far side is an input edge: this node reads
/// it and writes its own output, so a `.kir` that assigned to it would be
/// asking to write another source's buffer.
///
/// Refused by the grammar rather than by the checker — an assignment target is
/// a bare name — which is a refusal all the same, and the one that matters is
/// that it never reaches the generator.
#[test]
fn a_used_geometry_cannot_be_assigned_to() {
    let errs = refusals(&MORPH.replace(
        "position = mix(position, far.position, vec3(k, k, k));",
        "far.position = position;",
    ));
    assert!(
        !errs.is_empty(),
        "writing to the far side has to be refused"
    );
}

/// `uses` is L2's, on the same terms `amplify` is: an L1 makes geometry rather
/// than taking any, an L3 makes a viewpoint, and an L4 draws what reaches it.
#[test]
fn uses_is_refused_outside_an_l2() {
    let l1 = r#"
proc gen {
  kind     L1
  topology points
  capacity [1, 8] = 4
  uses far : Geometry

  emit position

  element { position = vec3(0.0, 0.0, 0.0); }
}
"#;
    let l4 = r#"
proc dots {
  kind  L4
  blend additive
  uses far : Geometry

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.016;
  }

  fragment { color = vec4(1.0, 1.0, 1.0, 1.0); }
}
"#;
    for src in [l1, l4] {
        let errs = check_err(src);
        assert!(
            errs.iter().any(|e| e.message.contains("`uses` is L2 only")),
            "expected `uses` to be refused, got: {errs:?}"
        );
    }
}

/// **A node takes one second geometry**, and a second `uses` is refused with a
/// sentence rather than quietly overwriting the first.
///
/// One name would win and the other's reads would resolve against the wrong
/// geometry — which is the shape this notation exists to end, arriving through
/// the notation itself.
#[test]
fn a_second_used_geometry_is_refused() {
    let errs = check_err(&MORPH.replace(
        "  uses far : Geometry\n",
        "  uses far : Geometry\n  uses other : Geometry\n",
    ));
    assert!(
        errs.iter()
            .any(|e| e.message.contains("one second geometry")),
        "expected a second slot to be refused, got: {errs:?}"
    );
}

/// **`Geometry` is what a slot can be**, and anything else is refused by name
/// rather than accepted and ignored.
///
/// The type is written even though there is one of them, so that the slot that
/// takes a camera or a field — which is what this notation is for next — does
/// not have to grow a type incompatibly.
#[test]
fn an_unknown_slot_type_is_refused() {
    let errs = refusals(&MORPH.replace("uses far : Geometry", "uses far : Points"));
    assert!(
        errs.iter()
            .any(|e| e.message.contains("unknown input type `Points`")),
        "expected the type to be refused, got: {errs:?}"
    );
}

/// **A slot shares one scope with everything else nameable here.** It is read
/// the way a local is — `far.position` — so a slot called `position` would make
/// one spelling mean two things depending on whether a dot follows it.
#[test]
fn a_slot_name_cannot_shadow_or_be_shadowed() {
    // An attribute.
    let errs = check_err(&MORPH.replace("uses far : Geometry", "uses position : Geometry"));
    assert!(
        errs.iter()
            .any(|e| e.message.contains("shadows an attribute name")),
        "expected a slot named after an attribute to be refused, got: {errs:?}"
    );

    // A param declared by this same procedure.
    let errs = check_err(&MORPH.replace("param k :", "param far :"));
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`far` is already a param")),
        "expected a slot and a param of one name to collide, got: {errs:?}"
    );

    // And a local, from the other direction.
    let errs = check_err(&MORPH.replace(
        "    position = mix",
        "    let far = 1.0;\n    position = mix",
    ));
    assert!(
        errs.iter().any(|e| e
            .message
            .contains("`far` is the geometry this procedure uses")),
        "expected a local to collide with the slot, got: {errs:?}"
    );
}

/// **`other` is an ordinary name again.** It was reserved everywhere because it
/// was the one spelling a paired read could have; a slot is named by the
/// procedure now, so reserving a word would be reserving one nothing means.
#[test]
fn other_is_no_longer_a_reserved_name() {
    let checked = check_ok(
        r#"
proc shadow {
  kind L2

  consumes position

  deform {
    let other = length(position);
    position = position * other;
  }
}
"#,
    );
    assert!(checked.uses.is_empty(), "and it declares no geometry slot");
}
