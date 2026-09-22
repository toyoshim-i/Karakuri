use super::common::*;

// ---------------------------------------------------------------------------
// `source`, and the slot a mask compares it against
// ---------------------------------------------------------------------------

/// A generator that varies with which geometry of the Set it is making.
const SOURCE_L1: &str = r#"
proc grain {
  kind     L1
  topology points
  capacity [1, 64] = 8

  emit position, tint

  element {
    position = vec3(hash1(seed), hash1(seed + 1u), 0.0);
    tint     = vec3(float(source % 7u) * 0.1, 0.0, 0.0);
  }
}
"#;

/// A deformation that reads it, in the block a mask lives in.
const SOURCE_L2: &str = r#"
proc shrink {
  kind L2

  consumes position, size

  mask   { strength = float(source % 2u); }
  deform { size = size * 0.5; }
}
"#;

/// A renderer that reads it, in both its stages.
const SOURCE_L4: &str = r#"
proc lit
{
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = float(source % 3u) + 1.0;
  }

  fragment {
    color = vec4(float(source % 5u) * 0.2, 1.0, 1.0, 1.0);
  }
}
"#;

/// **`source` is readable on the three kinds a Set instantiates per geometry**,
/// and it is a `uint`.
///
/// It is a per-source *uniform*, not the fourth implicit attribute the spec's
/// table once called it: a chain is instantiated per source, so an instance
/// knows statically which geometry it runs over and the value is the same for
/// every element it touches. Which is why nothing had to be carried to make it
/// readable — `Set::prepare` has been writing the salt into every one of these
/// uniform blocks all along.
#[test]
fn source_is_readable_in_a_generator_a_deformation_and_a_renderer() {
    for (src, block) in [
        (SOURCE_L1, BlockKind::Element),
        (SOURCE_L2, BlockKind::Mask),
        (SOURCE_L4, BlockKind::Fragment),
    ] {
        let checked = check_ok(src);
        let read = format!(
            "{:?}",
            checked.block(block).expect("the block under test").stmts
        );
        assert!(
            read.contains("Ambient(Source)"),
            "`source` has to resolve to the ambient in {block:?}: {read}"
        );
        assert!(
            !read.contains("Attr(Source"),
            "and not to a carried attribute — the value is a uniform: {read}"
        );
    }

    // A vertex stage reads it as well as a fragment one: it is per instance,
    // not per fragment.
    let vertex = format!(
        "{:?}",
        check_ok(SOURCE_L4)
            .block(BlockKind::Vertex)
            .expect("a vertex block")
            .stmts
    );
    assert!(vertex.contains("Ambient(Source)"), "{vertex}");
}

/// **A `uint`, and the checker types it as one.** `float(source)` is the
/// spelling a colour wants and `source == only` is the one a mask wants;
/// neither works if the read comes back the wrong width.
#[test]
fn source_is_a_uint() {
    assert_eq!(karakuri_ir::ast::Ambient::Source.ty(), Ty::Uint);
    assert_eq!(
        karakuri_ir::ast::Ambient::from_name("source"),
        Some(karakuri_ir::ast::Ambient::Source)
    );

    // Read into a `let`, so the type shows up on the binding rather than
    // inside an arithmetic that could have coerced it.
    let src = SOURCE_L2.replace(
        "strength = float(source % 2u);",
        "let s = source; strength = float(s % 2u);",
    );
    let checked = check_ok(&src);
    let stmts = &checked.block(BlockKind::Mask).expect("a mask").stmts;
    match &stmts[0] {
        TStmt::Let { value, .. } => assert_eq!(value.ty, Ty::Uint),
        other => panic!("expected the `let` first: {other:?}"),
    }
}

/// **Refused on the two kinds that run over no geometry**, on exactly the
/// grounds `seed` is refused there: an L3 runs once a frame over nothing and
/// the salt is a property of a Set's geometry, which a camera is not; a field
/// is a function of space, spliced into callers that may have no geometry at
/// all.
#[test]
fn source_is_refused_in_a_camera_and_in_a_field() {
    let camera = r#"
proc drift {
  kind L3

  camera {
    eye    = vec3(0.0, 0.0, float(source % 4u) + 6.0);
    target = vec3(0.0, 0.0, 0.0);
  }
}
"#;
    let errs = check_err(camera);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`source` is per geometry")
                && e.message.contains("a camera")),
        "an L3 has to refuse `source` with a sentence about itself: {errs:?}"
    );

    let field = r#"
proc blob {
  kind Field

  field {
    distance = sd_sphere(point, 1.0) + float(source % 2u);
  }
}
"#;
    let errs = check_err(field);
    assert!(
        errs.iter().any(
            |e| e.message.contains("`source` is per geometry") && e.message.contains("a field")
        ),
        "a field has to refuse `source` with a sentence about itself: {errs:?}"
    );
}

/// Asserts that `source` is rejected in a procedure declaring a geometry slot.
#[test]
fn source_is_refused_beside_a_geometry_slot() {
    let src = r#"
proc morph {
  kind L2

  uses far : Geometry

  consumes position

  mask   { strength = float(source % 2u); }
  deform { position = mix(position, far.position, vec3(0.5, 0.5, 0.5)); }
}
"#;
    let errs = check_err(src);
    let hit = errs
        .iter()
        .find(|e| e.message.contains("`source` is ambiguous"))
        .unwrap_or_else(|| panic!("expected `source` to be refused here: {errs:?}"));
    assert!(
        hit.message.contains("far"),
        "the refusal names the slot that made it ambiguous: {}",
        hit.message
    );
    assert!(
        hit.hint
            .as_ref()
            .is_some_and(|h| h.contains("uses <name> : Source")),
        "and points at the spelling that is not ambiguous: {:?}",
        hit.hint
    );

    // The same procedure without the geometry slot is fine, which is what
    // makes this a rule about the pair rather than about `mask`.
    let alone = src.replace("  uses far : Geometry\n\n", "").replace(
        "mix(position, far.position, vec3(0.5, 0.5, 0.5))",
        "position",
    );
    assert_eq!(check_ok(&alone).source_slots(), Vec::<&str>::new());
}

/// A mask that dissolves whichever geometry its `only` slot was bound to.
const DISSOLVE: &str = r#"
proc dissolve {
  kind L2

  uses only : Source

  consumes position, size

  mask {
    strength = 0.0;
    if source == only { strength = 1.0; }
  }

  deform { size = size * 0.2; }
}
"#;

/// **A Source slot is read as a value, alone**, which no other slot type is.
///
/// A geometry is not a value because the language has no type for a whole
/// source; a field is not one because it is a function; a camera is not one
/// because it is six numbers. This is a `uint`, and the language has one of
/// those — so `only` on its own is the bound geometry's identity and needs no
/// dot, no call and no member.
#[test]
fn a_declared_source_slot_is_read_as_a_value() {
    let checked = check_ok(DISSOLVE);
    assert_eq!(
        checked.uses,
        vec![Slot {
            name: "only".into(),
            ty: SlotTy::Source,
        }]
    );
    assert_eq!(checked.source_slots(), vec!["only"]);
    assert_eq!(
        checked.geometry_slot(),
        None,
        "and it is not a geometry slot: a mask reads no elements of it"
    );
    assert!(checked.field_slots().is_empty());
    assert_eq!(checked.camera_slot(), None);

    let mask = checked.block(BlockKind::Mask).expect("a mask block");
    let read = format!("{:?}", mask.stmts);
    assert!(
        read.contains(r#"Source { slot: "only" }"#),
        "`only` has to resolve to its own slot read, carrying the name: {read}"
    );
    assert!(
        read.contains("Ambient(Source)"),
        "and `source` beside it to the ambient: {read}"
    );
}

/// **Several are legal**, which is the difference from `Geometry` and `Camera`
/// and the reason the type is worth its own variant.
///
/// A `Geometry` slot binds an element buffer and is capped at one because a
/// second bound buffer is not built. This binds a `u32` in a uniform the
/// module already has, so `source == a || source == b` costs two of them and
/// nothing else.
#[test]
fn two_source_slots_on_one_node_are_accepted() {
    let src = r#"
proc pair {
  kind L2

  uses a : Source
  uses b : Source

  consumes position, size

  mask {
    strength = 0.0;
    if source == a || source == b { strength = 1.0; }
  }

  deform { size = size * 0.2; }
}
"#;
    let checked = check_ok(src);
    assert_eq!(checked.source_slots(), vec!["a", "b"]);
    let read = format!(
        "{:?}",
        checked.block(BlockKind::Mask).expect("a mask").stmts
    );
    assert!(
        read.contains(r#"Source { slot: "a" }"#) && read.contains(r#"Source { slot: "b" }"#),
        "each read carries its own slot, or one edge would answer for both: {read}"
    );

    // Two of one *name* is still refused, on the terms every other collision
    // here is: an edge names a slot, so two of one name is one address for two
    // inputs.
    let errs = check_err(&src.replace("uses b : Source", "uses a : Source"));
    assert!(
        errs.iter().any(|e| e.message.contains("is already a slot")),
        "expected the duplicate name to be refused: {errs:?}"
    );
}

/// **Legal on L1, L2 and L4 and refused on L3 and Field**, which is the
/// ambient's rule stated one level up — a slot exists to be compared against
/// `source`, so it belongs exactly where `source` does.
#[test]
fn a_source_slot_follows_the_ambients_kinds() {
    // The three that may.
    for src in [
        r#"
proc gen {
  kind     L1
  topology points
  capacity [1, 64] = 8

  uses only : Source

  emit position

  element {
    var k = 0.0;
    if source == only { k = 1.0; }
    position = vec3(k, 0.0, 0.0);
  }
}
"#,
        DISSOLVE,
        r#"
proc lit {
  kind  L4
  blend additive

  uses only : Source

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.016;
  }

  fragment {
    var k = 0.0;
    if source == only { k = 1.0; }
    color = vec4(k, 0.0, 0.0, 1.0);
  }
}
"#,
    ] {
        assert_eq!(check_ok(src).source_slots(), vec!["only"]);
    }

    // And the two that may not, each with a sentence about itself.
    for (src, about) in [
        (
            r#"
proc look {
  kind L3

  uses only : Source

  camera {
    eye    = vec3(0.0, 0.0, 6.0);
    target = vec3(0.0, 0.0, 0.0);
  }
}
"#,
            "an L3 masks nothing",
        ),
        (
            r#"
proc blob {
  kind Field

  uses only : Source

  field { distance = sd_sphere(point, 1.0); }
}
"#,
            "a field masks nothing",
        ),
    ] {
        let errs = check_err(src);
        assert!(
            errs.iter().any(|e| e.message.contains(about)),
            "expected a refusal saying `{about}`: {errs:?}"
        );
    }
}

/// **A fullscreen renderer reads it too**, and that is what separates this from
/// `seed`.
///
/// `seed` is per element and a fullscreen L4 has none, so reading it there is
/// refused. `source` is per *instance*: the chain is instantiated per geometry,
/// so a procedure with no element still knows whose chain it is running in, and
/// its uniform holds the salt like every other module's.
#[test]
fn source_is_readable_in_a_fullscreen_renderer_where_seed_is_not() {
    let src = r#"
proc march {
  kind  L4
  blend additive

  fragment {
    color = vec4(float(source % 3u) * 0.3, length(ray) * 0.0 + 0.5, 0.5, 1.0);
  }
}
"#;
    let checked = check_ok(src);
    let read = format!(
        "{:?}",
        checked
            .block(BlockKind::Fragment)
            .expect("a fragment")
            .stmts
    );
    assert!(read.contains("Ambient(Source)"), "{read}");

    // The mirror, unchanged: `seed` there is still refused.
    let errs = check_err(&src.replace("float(source % 3u)", "float(seed % 3u)"));
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`seed` is per element")),
        "expected `seed` to stay refused in a fullscreen procedure: {errs:?}"
    );
}

/// A `.kir` still writing `point_size` is refused by name, and the refusal says
/// what to write instead.
///
/// The rename is the whole reason this test exists. `point_size` is not a
/// declared name and not an output any more, so without a case of its own it
/// would fall out of `resolve_target` as "assigning to `point_size`, which was
/// never declared" — a sentence that is true and useless. Every `.kir` written
/// before the rename hits this line, and each of them needs the same two facts:
/// the new spelling, and that the number is no longer a pixel count.
#[test]
fn a_file_still_writing_point_size_is_refused_with_the_new_spelling() {
    let src = r#"
proc stale {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = vec4(position, 1.0);
    point_size = 4.0;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    let errs = check_err(src);
    let hit = errs
        .iter()
        .find(|e| e.message.contains("point_size"))
        .unwrap_or_else(|| panic!("no refusal named `point_size`: {errs:?}"));
    assert!(
        hit.message.contains("point_rate"),
        "the refusal does not name the new spelling: {}",
        hit.message
    );
    assert!(
        !hit.message.contains("never declared"),
        "the rename was reported as an undeclared name: {}",
        hit.message
    );
    let hint = hit
        .hint
        .as_deref()
        .unwrap_or_else(|| panic!("the refusal carries no hint: {hit:?}"));
    assert!(
        hint.contains("fraction") && hint.contains("height"),
        "the hint does not say what the unit became: {hint}"
    );
    // **No reference resolution in the hint.** The division is a fact about the
    // file being migrated, not about the language, and a number here would
    // become an anchor authors write against.
    assert!(
        !hint.contains("720"),
        "the hint anchors the unit to a resolution: {hint}"
    );
}

/// The same name met in an expression is refused the same way.
///
/// Reading a stage output was never allowed under either spelling, so the
/// interesting half is which sentence comes back: "does not resolve to a local,
/// a param, an attribute, or an ambient value" would send the author looking
/// for a typo in a name that is not a typo.
#[test]
fn reading_point_size_is_also_refused_with_the_new_spelling() {
    let src = r#"
proc stale_read {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = vec4(position, 1.0);
    point_rate = 0.004;
  }

  fragment {
    color = vec4(point_size, 1.0, 1.0, 1.0);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("point_size") && e.message.contains("point_rate")),
        "expected the rename refusal on a read as well: {errs:?}"
    );
}

/// `point_rate` is the required vertex output, under that name.
///
/// The coverage rule did not change with the rename, and this is what says so:
/// a `vertex` block that writes only `clip` is refused, and the diagnostic
/// names `point_rate` rather than the spelling it replaced.
#[test]
fn a_vertex_block_without_point_rate_is_refused_and_the_refusal_names_point_rate() {
    let src = r#"
proc no_rate {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip = vec4(position, 1.0);
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("point_rate") && e.message.contains("every path")),
        "expected a coverage diagnostic naming `point_rate`, got: {errs:?}"
    );
}
