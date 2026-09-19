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

/// **Refused in a procedure that declares a geometry slot**, because there the
/// reading would be silently one of two.
///
/// A pairing Set is one source made of two simulations: the far one feeds the
/// slot and shares the near one's uniform, so there is one salt for two
/// geometries and `source` would answer for the near one without saying so.
/// The refusal names the slot, which is the half that helps — a hint saying
/// *which* reading was ambiguous beats a rule the author has to infer.
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

// ---------------------------------------------------------------------------
// kind L5 — the frame effect
// ---------------------------------------------------------------------------

/// The shortest legal L5, and the negative control every refusal below is
/// measured against.
///
/// It is here rather than repeated in each test because a refusal test that
/// carries no positive is a test that passes when the whole kind is broken:
/// `kind L5` refused outright would make every "is it refused" assertion below
/// green.
const LEGAL_FRAME: &str = r#"
proc dim {
  kind L5

  param amount : float [0.0, 1.0] = 0.5

  frame {
    let s = texel(src);
    color = vec4(s.xyz * amount, s.w);
  }
}
"#;

#[test]
fn a_legal_frame_block_is_accepted() {
    let checked = check_ok(LEGAL_FRAME);
    assert_eq!(checked.kind, Kind::L5);
    assert!(checked.block(BlockKind::Frame).is_some());
    assert!(!checked.retains, "no `retains` was declared");
    // **Nothing about a topology**, and that is the fact `cost::fragment_ceiling`
    // had to be taught: an L5 covers the frame exactly once and never says so.
    assert_eq!(checked.topology, None);
}

/// **`seed` names the fix**, which is the whole of what a refusal owes here: a
/// fullscreen pass has no element, and the sentence says what to reach for
/// instead rather than restating the rule.
#[test]
fn an_l5_reading_seed_is_refused_with_the_fix_named() {
    let src = r#"
proc speckle {
  kind L5

  frame {
    let s = texel(src);
    color = vec4(s.xyz * hash1(seed), s.w);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`seed` is not available to an L5")),
        "the refusal has to name the value and the kind: {errs:?}"
    );
    let hint = errs
        .iter()
        .find_map(|e| e.hint.clone())
        .expect("the refusal carries a hint");
    assert!(
        hint.contains("a fullscreen pass has no element"),
        "the hint has to say why: {hint}"
    );
    assert!(
        hint.contains("point_coord") && hint.contains("param"),
        "the hint has to name what to reach for instead: {hint}"
    );

    // The negative control: the same shape without `seed` is accepted, so this
    // test cannot pass by the kind being refused outright.
    check_ok(LEGAL_FRAME);
}

/// The other seven, each by name and each with its own sentence.
///
/// Together with `seed` above they are the eight `docs/ir-spec.md` lists, and
/// the assertion is that none of them falls through to *"not available in this
/// block"* — which would send an author looking at the block when what is
/// wrong is the kind.
#[test]
fn every_ambient_an_l5_refuses_gets_its_own_sentence() {
    for (name, expr, because) in [
        ("copy", "float(copy)", "has no element"),
        ("source", "float(source)", "several decks"),
        ("point", "point.x", "field"),
        ("capacity", "float(capacity)", "how much material"),
        (
            "camera",
            "(camera * vec4(0.0, 0.0, 0.0, 1.0)).x",
            "several cameras",
        ),
        ("eye", "eye.x", "marcher"),
        ("ray", "ray.x", "marcher"),
    ] {
        let src = format!(
            r#"
proc probe {{
  kind L5

  frame {{
    let s = texel(src);
    color = vec4(s.xyz * {expr}, s.w);
  }}
}}
"#
        );
        let errs = check_err(&src);
        assert!(
            errs.iter()
                .any(|e| e.message == format!("`{name}` is not available to an L5")),
            "`{name}` must be refused with a sentence about the kind, got: {errs:?}"
        );
        let hint = errs
            .iter()
            .find(|e| e.message == format!("`{name}` is not available to an L5"))
            .and_then(|e| e.hint.clone())
            .unwrap_or_default();
        assert!(
            hint.contains(because),
            "`{name}`'s hint has to say why: {hint}"
        );
    }
}

/// **`held` is refused with a sentence about the *declaration*, not the name.**
///
/// The name is right and the header is what is missing, so the refusal points
/// at `retains` — and the negative control beside it is the same file with the
/// declaration added, which checks clean and carries the flag.
#[test]
fn held_outside_retains_is_refused_and_retains_makes_it_readable() {
    let without = r#"
proc echo {
  kind L5

  param amount : float [0.0, 1.0] = 0.5

  frame {
    let s = texel(src);
    let h = texel(held);
    color = vec4(s.xyz + amount * h.xyz, s.w);
  }
}
"#;
    let errs = check_err(without);
    assert!(
        errs.iter().any(|e| e
            .message
            .contains("`held` is available only under `retains`")),
        "the refusal has to name the declaration: {errs:?}"
    );
    let hint = errs.iter().find_map(|e| e.hint.clone()).unwrap_or_default();
    assert!(
        hint.contains("add a bare `retains`"),
        "the hint has to say what to add: {hint}"
    );
    // **And that the cut is not the file's**, which is the half a reader is
    // most likely to try to write down.
    assert!(
        hint.contains("mix") && hint.contains("exit"),
        "the hint has to say the cut is answered elsewhere: {hint}"
    );

    let with = without.replace("kind L5", "kind L5\n  retains");
    let checked = check_ok(&with);
    assert!(checked.retains, "a bare `retains` is carried through");
    // **Reading the previous frame is accumulation**, whatever it is spelled
    // with: the trail at `t` is every frame that led to it.
    assert!(
        !checked.closed_form,
        "a procedure that reads a retained frame cannot be evaluated at any `t`"
    );
}

/// **`retains` is refused on every kind but L5**, at the declaration.
#[test]
fn retains_is_refused_off_an_l5() {
    let src = r#"
proc drift {
  kind L4
  retains
  blend additive

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`retains` is L5 only")),
        "{errs:?}"
    );
}

/// **The one refusal that has to be argued rather than followed.**
///
/// The four kinds that may evaluate a field are the four with a position in
/// space to evaluate it at. An L5 has a frame coordinate, `eye` and `ray` are
/// refused, and a field marched from a viewpoint it cannot name would be a
/// shape drawn against nothing.
#[test]
fn a_field_slot_on_an_l5_is_refused_and_a_texture_slot_is_not() {
    let field = r#"
proc carve {
  kind L5

  uses shape : Field

  frame {
    let s = texel(src);
    color = vec4(s.xyz * shape(vec3(0.0, 0.0, 0.0)), s.w);
  }
}
"#;
    let errs = check_err(field);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`uses … : Field` is not an L5")),
        "{errs:?}"
    );
    let hint = errs.iter().find_map(|e| e.hint.clone()).unwrap_or_default();
    assert!(
        hint.contains("frame coordinate"),
        "the hint has to say what an L5 has instead of a point: {hint}"
    );

    // The negative control, and the one slot type an L5 *may* declare: a
    // nested merge's fan-in, `uses` plus `edge` and no new mechanism.
    let texture = r#"
proc over {
  kind L5

  param mix_amount : float [0.0, 1.0] = 0.5

  uses under : Texture

  frame {
    let a = texel(src);
    let b = tap(under, point_coord);
    color = vec4(a.xyz + (b.xyz - a.xyz) * mix_amount, a.w);
  }
}
"#;
    let checked = check_ok(texture);
    assert_eq!(
        checked.uses,
        vec![Slot {
            name: "under".into(),
            ty: SlotTy::Texture,
        }]
    );
    assert_eq!(checked.texture_slots(), vec!["under"]);
}

/// **A Texture slot is L5's and nowhere else's**, and each refusal says which
/// kind it is talking about.
#[test]
fn a_texture_slot_is_refused_on_every_other_kind() {
    for (kind, body) in [
        (
            "L1",
            "topology points\n  capacity [1, 1024] = 512\n\n  emit position\n\n  element {\n    position = vec3(0.0, 0.0, 0.0);\n  }",
        ),
        ("L2", "deform {\n    let a = 1.0;\n  }"),
        ("L3", "camera {\n    eye = vec3(0.0, 0.0, 6.0);\n    target = vec3(0.0, 0.0, 0.0);\n  }"),
        (
            "L4",
            "blend additive\n\n  fragment {\n    color = vec4(1.0, 1.0, 1.0, 1.0);\n  }",
        ),
        ("Field", "field {\n    distance = sd_sphere(point, 1.0);\n  }"),
    ] {
        let src = format!(
            r#"
proc probe {{
  kind {kind}

  uses back : Texture

  {body}
}}
"#
        );
        let errs = refusals(&src);
        assert!(
            errs.iter()
                .any(|e| e.message.contains("`uses … : Texture` is L5 only")),
            "a {kind} must refuse a Texture slot: {errs:?}"
        );
    }
}

/// **A texture is not a value, and it is the one slot type that can never
/// become one.** What it offers is a fetch, and the refusal says which.
#[test]
fn reading_a_texture_as_a_value_is_refused_with_the_two_builtins_named() {
    let src = r#"
proc grab {
  kind L5

  frame {
    let s = src;
    color = vec4(s, s, s, 1.0);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`src` is a texture, not a value")),
        "{errs:?}"
    );
    let hint = errs.iter().find_map(|e| e.hint.clone()).unwrap_or_default();
    assert!(
        hint.contains("texel(src)") && hint.contains("tap(src, uv)"),
        "the hint has to name both fetches: {hint}"
    );
}

/// **The three builtins are refused outside a `kind L5`**, and `frame_step` is
/// the one that needed saying: its two neighbours refuse themselves for want of
/// a texture name, and it would type-check anywhere and lower to a read of a
/// `viewport` field no other module carries.
#[test]
fn the_frame_builtins_are_refused_outside_an_l5() {
    let src = r#"
proc smear {
  kind L4
  blend additive

  fragment {
    let d = frame_step(0.01);
    color = vec4(d.x, d.y, 0.0, 1.0);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`frame_step` is an L5 builtin")),
        "{errs:?}"
    );

    // The negative control: the same call in a `frame` block is accepted.
    check_ok(
        r#"
proc smear {
  kind L5

  frame {
    let d = frame_step(0.01);
    color = tap(src, point_coord + d);
  }
}
"#,
    );
}

/// **An L5 declares nothing about the material that made the picture**, and
/// every one of those declarations is refused at the header.
#[test]
fn an_l5_declaring_geometry_is_refused() {
    let src = r#"
proc wrong {
  kind L5
  topology points
  capacity [1, 16] = 8
  blend additive
  amplify 2

  emit position
  consumes position

  frame {
    color = texel(src);
  }
}
"#;
    let errs = check_err(src);
    for what in [
        "`topology` is not an L5",
        "`capacity` is not an L5",
        "`blend` is not an L5",
        "`amplify` is not an L5",
    ] {
        assert!(
            errs.iter().any(|e| e.message.contains(what)),
            "expected {what}: {errs:?}"
        );
    }
    assert!(
        errs.iter().any(|e| e
            .message
            .contains("`emit` and `consumes` are about elements, and an L5 has none")),
        "{errs:?}"
    );
}

/// **A `vertex` block in an L5 is refused by the block-owner rule**, which is
/// where it belongs: `vertex` is L4's, and the sentence says so rather than
/// inventing a second rule about frame passes.
#[test]
fn a_vertex_block_in_an_l5_is_refused_as_l4s() {
    let src = r#"
proc wrong {
  kind L5

  vertex {
    clip       = vec4(0.0, 0.0, 0.0, 1.0);
    point_rate = 0.01;
  }

  frame {
    color = texel(src);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter().any(|e| e
            .message
            .contains("a `vertex` block is not valid in a `L5` procedure")),
        "{errs:?}"
    );
}

/// **An L5 requires its one block**, and the message names it.
#[test]
fn an_l5_without_a_frame_block_is_refused() {
    let errs = check_err("proc empty {\n  kind L5\n}\n");
    assert!(
        errs.iter()
            .any(|e| e.message.contains("L5 procedures require a `frame` block")),
        "{errs:?}"
    );
}

/// **`color` is written in `frame` on an L5 and in `fragment` on an L4**, one
/// name because it is one value at the next node down — and being an output
/// *here* is what makes it reserved here.
///
/// `shadows_output` is scoped to the layer, so before an L5 owned `color` a
/// procedure could declare `param color` and have the param silently outrank
/// the output in an assignment target. That is exactly the class of bug the
/// shadowing rule exists to prevent, and this is the assertion that it reaches
/// the sixth kind.
#[test]
fn color_is_reserved_in_an_l5() {
    let src = r#"
proc wrong {
  kind L5

  param color : float [0.0, 1.0] = 0.5

  frame {
    color = texel(src);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`color` shadows a stage output name")),
        "{errs:?}"
    );
}

/// **The two reserved names of a `frame` block cannot also name a slot**, in
/// the one kind where a Texture slot is legal at all.
#[test]
fn a_texture_slot_may_not_be_called_src_or_held() {
    for name in ["src", "held"] {
        let src = format!(
            r#"
proc collide {{
  kind L5
  retains

  uses {name} : Texture

  frame {{
    color = texel(src);
  }}
}}
"#
        );
        let errs = check_err(&src);
        assert!(
            errs.iter()
                .any(|e| e.message.contains("is a texture an L5 is already handed")),
            "`{name}` must be refused as a slot name: {errs:?}"
        );
    }
}

/// **`texel` takes no coordinate on purpose**, and the refusal says why rather
/// than reporting an arity.
#[test]
fn texel_with_a_coordinate_is_refused_with_the_reason() {
    let src = r#"
proc resample {
  kind L5

  frame {
    color = texel(src, point_coord);
  }
}
"#;
    let errs = check_err(src);
    let hint = errs.iter().find_map(|e| e.hint.clone()).unwrap_or_default();
    assert!(
        hint.contains("invitation to resample"),
        "the hint has to give the reason: {hint}"
    );
    assert!(
        hint.contains("tap(<texture>, uv)"),
        "the hint has to name the filtered read: {hint}"
    );
}
