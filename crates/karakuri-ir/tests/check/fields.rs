use super::common::*;
use super::slots::MORPH;

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

/// Verifies that a declared Field slot is callable and retains the slot name in typed IR.
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

/// Verifies that multiple Field slots can be declared on a single procedure.
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

    // Multiple field slots are estimated independently.
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

/// Verifies that Field procedures cannot declare Field slots.
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

/// Verifies that the legacy `field(p)` syntax is rejected with a migration hint.
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

/// Verifies that Field slot names cannot shadow builtins or type constructors.
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

/// Verifies that declared Camera slots expose camera members (.clip, .eye, .ray).
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

/// Verifies that Camera slots can only be declared on L4 renderers.
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

/// Verifies that declaring multiple Camera slots on one renderer is rejected.
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

/// Verifies that accessing invalid camera members is rejected with hints.
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

/// Verifies that camera members respect the stage restrictions of their underlying ambients.
#[test]
fn the_members_keep_the_ambients_stage_rules() {
    // Ambient stage accessibility rules verified within fragment stage.
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

/// Verifies that Camera slot names undergo standard shadowing checks.
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
