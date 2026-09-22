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
