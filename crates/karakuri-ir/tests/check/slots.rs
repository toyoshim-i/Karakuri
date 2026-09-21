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

const MORPH: &str = r#"
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

// ---------------------------------------------------------------------------
// A field, reached through a slot
// ---------------------------------------------------------------------------

/// A marcher that contains no shape: it declares the field it takes and calls
/// it by that name.
const LENS: &str = r#"
proc lens {
  kind  L4
  blend additive

  uses shape : Field

  fragment {
    var p = eye;
    var hit = 0.0;
    for i in 0..8 {
      let d = shape(p);
      if d < 0.005 {
        hit = 1.0;
      }
      p = p + ray * max(d, 0.005);
    }
    color = vec4(hit, hit, hit, 1.0);
  }
}
"#;

/// **A declared Field slot is callable, and the call carries the slot's name.**
///
/// The name is the whole change. `field(p)` named the one field by being the
/// one spelling there was; this resolves against what the header declared, so
/// what reaches the lowering is a call that says *which* field — and a lowering
/// that has the name can address a second one without any of this moving.
#[test]
fn a_declared_field_slot_is_called_and_carries_its_name() {
    let checked = check_ok(LENS);
    assert_eq!(
        checked.uses,
        vec![Slot {
            name: "shape".into(),
            ty: SlotTy::Field,
        }]
    );
    assert_eq!(checked.field_slots(), vec!["shape"]);
    assert_eq!(
        checked.geometry_slot(),
        None,
        "and it is not a geometry slot: the two answer different questions"
    );

    let fragment = checked.block(BlockKind::Fragment).expect("a fragment");
    let calls = format!("{:?}", fragment.stmts);
    assert!(
        calls.contains("Field") && calls.contains("shape"),
        "the call resolves to a field read naming its slot: {calls}"
    );

    // And its type is the field block's: one `vec3` in, a `float` out.
    let errs = check_err(&LENS.replace("shape(p)", "shape(p, 1.0)"));
    assert!(
        errs.iter().any(|e| e.message.contains("takes 1 argument")),
        "expected the arity to be the field's, got: {errs:?}"
    );
    let errs = check_err(&LENS.replace("shape(p)", "shape(1.0)"));
    assert!(
        errs.iter().any(|e| e.message.contains("expects `vec3`")),
        "expected a position to be required, got: {errs:?}"
    );
}

/// **Several Field slots on one procedure are legal**, which is the rule a
/// geometry slot does not follow.
///
/// A marcher wanting a shape and a cutter is the ordinary case, and it costs
/// nothing: a field has no node and no buffer, so a second slot is one more
/// body spliced under one more name. The count is a rule about *geometry* —
/// a second bound element buffer per node is what is not built — which is why
/// splitting the arity rule by type is what this notation needed.
#[test]
fn two_field_slots_on_one_procedure_are_accepted() {
    let checked = check_ok(
        r#"
proc carve {
  kind  L4
  blend additive

  uses shape  : Field
  uses cutter : Field

  fragment {
    let d = max(shape(eye), -cutter(eye + ray));
    color = vec4(d, d, d, 1.0);
  }
}
"#,
    );
    assert_eq!(checked.field_slots(), vec!["shape", "cutter"]);

    // **Counted apart**, which is what lets a Set multiply each by the field
    // bound to it rather than by whichever one it found first.
    let cost = karakuri_ir::cost::estimate(&checked).expect("a marcher of two fields costs");
    assert_eq!(
        cost.field_calls.slot("shape").map(|c| c.total),
        Some(1),
        "{:?}",
        cost.field_calls
    );
    assert_eq!(
        cost.field_calls.slot("cutter").map(|c| c.total),
        Some(1),
        "{:?}",
        cost.field_calls
    );

    // Two of one *name* is still refused: an edge names a slot, so two would be
    // one address for two inputs.
    let errs = check_err(
        r#"
proc twice {
  kind  L4
  blend additive

  uses shape : Field
  uses shape : Field

  fragment {
    let d = shape(eye);
    color = vec4(d, d, d, 1.0);
  }
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.message.contains("already a slot")),
        "expected two slots of one name to be refused, got: {errs:?}"
    );
}

/// Asserts that Field slots are permitted on L1, L2, L3, and L4 procedures.
#[test]
fn a_field_slot_is_legal_on_every_kind_that_can_evaluate_one() {
    for src in [
        r#"
proc gen {
  kind     L1
  topology points
  capacity [1, 64] = 8

  uses shape : Field

  emit position

  element { position = vec3(shape(position), 0.0, 0.0); }
}
"#,
        r#"
proc warp {
  kind L2

  uses shape : Field

  consumes position

  deform { position = position * (1.0 + shape(position) * 0.01); }
}
"#,
        r#"
proc look {
  kind L3

  uses shape : Field

  camera {
    eye    = vec3(0.0, 0.0, 4.0 + shape(vec3(0.0, 0.0, 0.0)));
    target = vec3(0.0, 0.0, 0.0);
  }
}
"#,
        LENS,
    ] {
        let checked = check_ok(src);
        assert_eq!(
            checked.field_slots(),
            vec!["shape"],
            "in `{}`",
            checked.name
        );
    }
}

/// **A field may not take a field**, and the reason is the one the recursion
/// refusal it replaced carried: a field bound to itself is a function calling
/// itself, which WGSL forbids outright.
///
/// Two fields naming each other is the same failure at one remove, and telling
/// that apart from a legal chain of shapes is a walk over every edge in the
/// Set. That is a graph question, answerable where the Set is built and nowhere
/// in one file — so the whole thing is refused rather than half-checked here.
#[test]
fn a_field_refuses_a_field_slot() {
    let errs = check_err(
        r#"
proc wrong {
  kind Field

  uses other : Field

  field {
    distance = other(point);
  }
}
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.message.contains("a field cannot take a field")),
        "expected a field taking a field to be refused, got: {errs:?}"
    );
}

/// **`field(p)` no longer resolves**, and the sentence says what to write
/// instead.
///
/// It was a reserved word, which is exactly what capped a procedure at one
/// field: a second would have had nothing to be called. Keeping it as an alias
/// for "the one field, if there is exactly one" would be that rule reinstated
/// under a new spelling, so it is gone — and this is a breaking change to the
/// language, which is why the refusal is written for the person migrating a
/// file rather than for the compiler.
#[test]
fn field_is_no_longer_a_reserved_word() {
    let errs = check_err(&LENS.replace("shape(p)", "field(p)"));
    assert!(
        errs.iter().any(|e| e
            .message
            .contains("`field` is not a builtin function or a type constructor")),
        "expected `field(p)` to resolve to nothing, got: {errs:?}"
    );
    assert!(
        errs.iter().any(|e| e
            .hint
            .as_deref()
            .is_some_and(|h| h.contains("uses shape : Field"))),
        "and the hint has to say what replaced it, got: {errs:?}"
    );

    // It is an ordinary name again, in both directions: a slot may be called
    // `field`, and then `field(p)` means that slot.
    let checked = check_ok(&LENS.replace("shape", "field"));
    assert_eq!(checked.field_slots(), vec!["field"]);
}

/// **A Field slot's name is called, so it may not be a builtin's.**
///
/// `check_reserved` asks about names that are *read* — an attribute, an
/// ambient, a stage output — and a builtin is none of those: `sin` was a
/// perfectly good slot name while a slot was only ever read with a dot after
/// it. A call resolves against the header first, so `sin(x)` would stop meaning
/// the sine, and either resolution order is a spelling that quietly means
/// something other than it says.
#[test]
fn a_field_slot_may_not_be_named_after_a_builtin_or_a_type() {
    for (name, what) in [
        ("sin", "a builtin function"),
        ("vec3", "a type constructor"),
    ] {
        let errs = check_err(&LENS.replace("shape", name));
        assert!(
            errs.iter().any(|e| e.message.contains(what)),
            "expected `{name}` to be refused as a Field slot, got: {errs:?}"
        );
    }

    // A *geometry* slot is not called, so it keeps the name: the refusal is
    // about the type, not about `uses`.
    let checked = check_ok(
        &MORPH
            .replace("uses far : Geometry", "uses sin : Geometry")
            .replace("far.position", "sin.position"),
    );
    assert_eq!(checked.geometry_slot(), Some("sin"));
}

// ---------------------------------------------------------------------------
// A camera, reached through a slot
// ---------------------------------------------------------------------------

/// A renderer that says which camera it draws from, and reads the projection
/// through it.
const THROUGH: &str = r#"
proc through {
  kind  L4
  blend additive

  uses view : Camera

  consumes position

  vertex {
    clip       = view.clip * vec4(position, 1.0);
    point_rate = 0.012;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;

/// A marcher that says which camera it marches from.
const MARCH_THROUGH: &str = r#"
proc march_through {
  kind  L4
  blend additive

  uses view : Camera

  fragment {
    let p = view.eye + view.ray;
    color = vec4(p.x, p.y, p.z, 1.0);
  }
}
"#;

/// **A declared Camera slot is read as a member, and the members are the
/// ambients under another name.**
///
/// That is the whole of what this notation is: an L4 could already read the
/// Set's camera as `camera`, `eye` and `ray`, and what it could not say was
/// *which* camera. So the read resolves to the same three values — a new
/// spelling for an old capability, which is why nothing below the checker had
/// to move.
#[test]
fn a_declared_camera_slot_is_read_as_a_member() {
    let checked = check_ok(THROUGH);
    assert_eq!(
        checked.uses,
        vec![Slot {
            name: "view".into(),
            ty: SlotTy::Camera,
        }]
    );
    assert_eq!(checked.camera_slot(), Some("view"));
    assert_eq!(
        checked.geometry_slot(),
        None,
        "and it is not a geometry slot: the two answer different questions"
    );
    assert!(checked.field_slots().is_empty());

    // `view.clip` is the `camera` ambient, so what reaches the lowering is
    // exactly what a renderer that named no slot produces.
    let vertex = checked.block(BlockKind::Vertex).expect("a vertex block");
    let read = format!("{:?}", vertex.stmts);
    assert!(
        read.contains("Ambient(Camera)"),
        "`view.clip` has to resolve to the camera ambient: {read}"
    );
    assert!(
        !read.contains("Far("),
        "and not to an attribute of a far element: {read}"
    );

    // The other two, in the stage that has them.
    let marching = format!(
        "{:?}",
        check_ok(MARCH_THROUGH)
            .block(BlockKind::Fragment)
            .expect("a fragment block")
            .stmts
    );
    assert!(
        marching.contains("Ambient(Eye)") && marching.contains("Ambient(Ray)"),
        "`view.eye` and `view.ray` have to resolve to the marcher's two: {marching}"
    );
}

/// **A Camera slot is L4's alone**, and each of the other four refuses it with
/// a sentence about itself rather than about `uses`.
///
/// An L3 *is* a camera — every member is a derivation of the six numbers it
/// writes — an L1 and an L2 work in world space and do not project, and a field
/// is a function of space whose answer cannot depend on where it is watched
/// from.
#[test]
fn a_camera_slot_is_l4s_alone() {
    for src in [
        r#"
proc gen {
  kind     L1
  topology points
  capacity [1, 64] = 8

  uses view : Camera

  emit position

  element { position = vec3(0.0, 0.0, 0.0); }
}
"#,
        r#"
proc warp {
  kind L2

  uses view : Camera

  consumes position

  deform { position = position * 1.5; }
}
"#,
        r#"
proc look {
  kind L3

  uses view : Camera

  camera {
    eye    = vec3(0.0, 0.0, 4.0);
    target = vec3(0.0, 0.0, 0.0);
  }
}
"#,
        r#"
proc blob {
  kind Field

  uses view : Camera

  field { distance = length(point) - 1.0; }
}
"#,
    ] {
        let errs = check_err(src);
        assert!(
            errs.iter()
                .any(|e| e.message.contains("`uses … : Camera` is L4 only")),
            "expected a Camera slot to be refused here, got: {errs:?}"
        );
    }

    // And an L4 takes one, so none of the above is a refusal of the type as
    // such.
    assert_eq!(check_ok(THROUGH).camera_slot(), Some("view"));
}

/// **A renderer draws from one camera.** Two slots would need two bind groups
/// in one pipeline and two edges per Set, and neither is what anybody asked
/// for: a frame drawn from two viewpoints is two renderers, which is exactly
/// what an edge per renderer makes possible.
///
/// Refused rather than resolved by position, on the terms every other collision
/// in a header is.
#[test]
fn two_camera_slots_on_one_renderer_are_refused() {
    let errs = check_err(
        r#"
proc twice {
  kind  L4
  blend additive

  uses left  : Camera
  uses right : Camera

  consumes position

  vertex {
    clip       = left.clip * vec4(position, 1.0);
    point_rate = 0.012;
  }

  fragment { color = vec4(1.0, 1.0, 1.0, 1.0); }
}
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.message.contains("a renderer draws from one camera")),
        "expected a second Camera slot to be refused, got: {errs:?}"
    );

    // Two of one *name* is refused as well, and by the check every slot type
    // goes through: an edge names a slot, so two would be one address for two
    // inputs.
    let errs = check_err(&THROUGH.replace(
        "uses view : Camera",
        "uses view : Camera\n  uses view : Camera",
    ));
    assert!(
        errs.iter().any(|e| e.message.contains("already a slot")),
        "expected two slots of one name to be refused, got: {errs:?}"
    );
}

/// **A camera's members are resolved against its type**, so a name that is not
/// one of the three is refused with the three named — and with the five values
/// an L4 still cannot read, which is unbuilt by decision rather than by
/// oversight:
/// `docs/adr/0153-a-renderer-reads-three-camera-members-and-the-l3s-five-stay-unreadable.md`.
#[test]
fn a_member_a_camera_has_not_got_is_refused() {
    let errs = check_err(&THROUGH.replace("view.clip", "view.target"));
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`view.target` is not part of a camera")),
        "expected an unknown member to be refused, got: {errs:?}"
    );
    assert!(
        errs.iter()
            .any(|e| e.hint.as_deref().is_some_and(|h| h.contains("view.clip"))),
        "and the hint has to name the members there are, got: {errs:?}"
    );

    // A camera is not a value on its own either: what can be had from it is one
    // of its parts.
    let errs = check_err(&THROUGH.replace("view.clip *", "view *"));
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`view` is a camera, not a value")),
        "expected the bare slot name to be refused, got: {errs:?}"
    );
}

/// **The members keep the stage rules the ambients have**, asked rather than
/// restated: `.eye` and `.ray` are built by the ray prologue a fullscreen
/// fragment stage opens with, and a per-element renderer has no such prologue.
///
/// A `.kir` that read them anyway used to check clean and lower to a bare
/// identifier nothing declared — WGSL naga refuses it, and wgpu's uncaptured
/// error handler takes the thread down. Reaching them through a slot must not
/// be a way back to that.
#[test]
fn the_members_keep_the_ambients_stage_rules() {
    // **Per element, and in the `fragment` stage**, which is where the ambient
    // itself is legal: what refuses this is the procedure having a `vertex`
    // block at all, so reading it anywhere else would be refused by the block
    // rule instead and prove nothing about this one.
    let errs = check_err(&THROUGH.replace(
        "color = vec4(1.0, 1.0, 1.0, 1.0)",
        "color = vec4(view.ray, 1.0)",
    ));
    assert!(
        errs.iter().any(|e| e
            .message
            .contains("`view.ray` is only available to a procedure that draws the whole frame")),
        "expected a per-element `.ray` to be refused, got: {errs:?}"
    );

    // Fullscreen: the same read is exactly what a marcher is for.
    assert_eq!(check_ok(MARCH_THROUGH).camera_slot(), Some("view"));

    // And `.clip` is readable in a vertex stage, which is where a projection is
    // used — so the rule is the ambient's own and not a blanket one.
    assert_eq!(check_ok(THROUGH).camera_slot(), Some("view"));
}

/// **A Camera slot's name goes through the same reserved-word check every slot
/// name does**, and it is *not* checked against the builtins.
///
/// The builtin rule is Field-only by design — a slot that is *called* is the
/// one with that collision, and `uses sin : Camera` is read `sin.clip`, which
/// cannot be confused with the sine. The same reasoning a geometry slot
/// already followed.
#[test]
fn a_camera_slot_name_shares_the_scope_every_slot_name_does() {
    let errs = check_err(&THROUGH.replace("view", "position"));
    assert!(
        errs.iter()
            .any(|e| e.message.contains("shadows an attribute name")),
        "expected a slot named after an attribute to be refused, got: {errs:?}"
    );
    let errs = check_err(&THROUGH.replace("view", "beats"));
    assert!(
        errs.iter()
            .any(|e| e.message.contains("shadows an ambient value")),
        "expected a slot named after an ambient to be refused, got: {errs:?}"
    );

    // A builtin's name is still a name here, because a camera slot is read and
    // not called.
    let checked = check_ok(&THROUGH.replace("view", "sin"));
    assert_eq!(checked.camera_slot(), Some("sin"));
}
