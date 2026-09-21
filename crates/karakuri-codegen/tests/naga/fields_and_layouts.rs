#![allow(unused_imports, dead_code)]

use super::naga_common::*;

// ---------------------------------------------------------------------------
// A spliced field, in each of the five modules that can carry one.
// ---------------------------------------------------------------------------

/// A field exercising everything a spliced body can reach: the clock, a param,
/// an SDF builtin, a `mod`, and `fbm` — which this crate unrolls in Rust, so its
/// requirement lands in the *field's* set and has to be absorbed by the caller.
const SPLICED: &str = r#"
proc wobble {
  kind Field

  param radius : float [0.1, 3.0] = 1.0

  field {
    let n = fbm(point, 3) * 0.1;
    let a = mod(point.x, 2.0);
    distance = sd_sphere(point, radius) + n + a * 0.0 + sin(t + beats) * 0.0;
  }
}
"#;

/// The field itself, not a splice of it: a generator is handed the `kind Field`
/// procedure now and splices it once per slot its caller declared, because the
/// function's name is the caller's name for it.
fn spliced() -> karakuri_ir::typed::Checked {
    let parsed = karakuri_ir::parse(SPLICED).expect("parses");
    karakuri_ir::check::check(&parsed).expect("checks")
}

fn compiled(src: &str) -> karakuri_ir::typed::Checked {
    let parsed = karakuri_ir::parse(src).expect("parses");
    karakuri_ir::check::check(&parsed).expect("checks")
}

/// **Every caller kind, through a real WGSL front end.**
///
/// The engine's own tests reach one shape — a fullscreen L4 — so three of the
/// five splice sites had no coverage at any level, and deleting the splice from
/// any of them left the whole workspace green. What breaks is not subtle: a
/// module that names `_field_shape_at` and does not define it.
///
/// The field above reads `t` and `beats`, which each caller spells its own way,
/// and calls `fbm`, whose unrolled `perlin` requirement belongs to the field and
/// has to reach the caller's prelude.
#[test]
fn a_spliced_field_validates_in_every_kind_of_caller() {
    let field = spliced();
    let layout = karakuri_ir::layout::generate_element_layout(
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
        &[],
    );

    // L1, in both of its blocks.
    let l1 = compiled(
        r#"
proc gen {
  kind     L1
  topology points
  capacity [1, 64] = 8

  uses shape : Field

  param spawn_rate : float [0.0, 100.0] = 10.0

  emit position

  spawn   { position = vec3(shape(vec3(0.0, 0.0, 0.0)), 0.0, 0.0); }
  element { position = position + vec3(0.0, shape(position), 0.0) * dt; }
}
"#,
    );
    validate(&karakuri_codegen::generate_l1(&l1, &[], &[("shape", &field)]).source);

    // L2, in both of its blocks.
    let l2 = compiled(
        r#"
proc warp {
  kind L2
  uses shape : Field
  consumes position
  mask   { strength = clamp(shape(position), 0.0, 1.0); }
  deform { position = position * (1.0 + shape(position) * 0.01); }
}
"#,
    );
    validate(
        &karakuri_codegen::generate_l2(
            &l2,
            &[Attr::Position],
            karakuri_ir::layout::Synthetic::NONE,
            &[],
            None,
            &[("shape", &field)],
        )
        .source,
    );

    // L3.
    let l3 = compiled(
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
    );
    validate(&karakuri_codegen::generate_l3(&l3, &[("shape", &field)]).source);

    // L4 with a vertex block — per element, which is a different generator from
    // the fullscreen one below.
    let l4 = compiled(
        r#"
proc dots {
  kind  L4
  blend additive

  uses shape : Field

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.008 + shape(position) * 0.0;
  }

  fragment {
    let d = shape(vec3(0.0, 0.0, 0.0));
    color = vec4(d, d, d, 1.0);
  }
}
"#,
    );
    validate(&karakuri_codegen::generate_l4(&l4, &layout, &[("shape", &field)]).source);

    // L4 with none — fullscreen.
    let full = compiled(
        r#"
proc marcher {
  kind  L4
  blend additive

  uses shape : Field

  fragment {
    let d = shape(eye + ray);
    color = vec4(d, d, d, 1.0);
  }
}
"#,
    );
    validate(&karakuri_codegen::generate_l4(&full, &layout, &[("shape", &field)]).source);
}

/// **A caller that mentions no field carries none of it**, which is what keeps
/// a bad field body from taking down shaders with nothing to do with it — and
/// keeps every node in the Set from growing the field's params.
#[test]
fn a_caller_that_evaluates_no_field_is_not_spliced() {
    let field = spliced();
    let plain = compiled(
        r#"
proc plain {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.008;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#,
    );
    let layout = karakuri_ir::layout::generate_element_layout(
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
        &[],
    );
    let src = karakuri_codegen::generate_l4(&plain, &layout, &[("shape", &field)]).source;
    validate(&src);
    assert!(
        !src.contains("_field_shape_at"),
        "the function is not here: {src}"
    );
    assert!(
        !src.contains("field_shape_radius"),
        "and neither are its params: {src}"
    );
}

/// **The clock is the caller's own spelling**, passed at the call site. An L1
/// reads `t` per substep from `step_args` where everything else reads `u` — one
/// spliced body cannot say both, and a body that assumed either produced a
/// module naming a field that module does not have.
#[test]
fn a_spliced_field_takes_the_clock_from_its_caller() {
    let field = spliced();
    let l1 = compiled(
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
    );
    let src = karakuri_codegen::generate_l1(&l1, &[], &[("shape", &field)]).source;
    assert!(
        src.contains("step_args.t"),
        "an L1 passes its per-substep clock: {src}"
    );

    let layout = karakuri_ir::layout::generate_element_layout(
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
        &[],
    );
    let full = compiled(
        r#"
proc marcher {
  kind  L4
  blend additive

  uses shape : Field

  fragment {
    let d = shape(eye);
    color = vec4(d, d, d, 1.0);
  }
}
"#,
    );
    let src = karakuri_codegen::generate_l4(&full, &layout, &[("shape", &field)]).source;
    assert!(
        src.contains("_field_shape_at(") && src.contains("u.t"),
        "and a renderer passes `u.t`: {src}"
    );
}

// ---------------------------------------------------------------------------
// Element struct memory layout validation against Naga alignment arithmetic.
// Compares host-side `ElementSlot::offset` against Naga type layout offsets.
// ---------------------------------------------------------------------------

/// naga's own placement for one generated struct: each member's name and byte
/// offset in declaration order, and the stride it gives `array<name>`.
///
/// **The array stride rather than the struct's span**, because the stride is
/// the number the engine actually multiplies a capacity by. WGSL rounds a
/// struct's size up to its own alignment to get it, so the two agree — and the
/// caller asserts that they do, since a front end that disagreed with itself
/// there would make every other assertion here meaningless.
fn naga_placement(source: &str, name: &str) -> (Vec<(String, u32)>, u32, u32) {
    let module = naga::front::wgsl::parse_str(source).unwrap_or_else(|e| {
        panic!(
            "WGSL failed to parse:\n{}\n\n---- source ----\n{source}",
            e.emit_to_string(source)
        )
    });
    let (handle, members, span) = module
        .types
        .iter()
        .find_map(|(handle, ty)| match (&ty.name, &ty.inner) {
            (Some(n), naga::TypeInner::Struct { members, span }) if n == name => {
                Some((handle, members, *span))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no `struct {name}` in the emitted module:\n{source}"));
    let stride = module
        .types
        .iter()
        .find_map(|(_, ty)| match ty.inner {
            naga::TypeInner::Array { base, stride, .. } if base == handle => Some(stride),
            _ => None,
        })
        .unwrap_or_else(|| panic!("`{name}` is never bound as an array:\n{source}"));
    let placed = members
        .iter()
        .map(|m| {
            (
                m.name.clone().unwrap_or_else(|| "<unnamed>".to_string()),
                m.offset,
            )
        })
        .collect();
    (placed, span, stride)
}

/// Every slot of `layout`, at the name and offset naga put it at, and the
/// stride naga gives an array of it.
fn assert_naga_agrees(source: &str, name: &str, layout: &karakuri_ir::layout::ElementLayout) {
    let (placed, span, stride) = naga_placement(source, name);
    let ours: Vec<(String, u32)> = layout
        .slots
        .iter()
        .map(|s| (s.name.to_string(), s.offset))
        .collect();
    assert_eq!(
        placed, ours,
        "naga placed `{name}`'s members differently from `ElementLayout`:\n{source}"
    );
    assert_eq!(
        stride, layout.stride,
        "naga's `array<{name}>` stride is not `ElementLayout::stride`:\n{source}"
    );
    assert_eq!(
        span, stride,
        "a struct's span and its array stride must be the same number"
    );
}

fn l1_emitting(emit: &str) -> karakuri_ir::typed::Checked {
    let assignments: String = emit
        .split(", ")
        .map(|attr| match attr {
            "position" | "velocity" | "normal" | "tint" => {
                format!("    {attr} = vec3(0.0, 0.0, 0.0);\n")
            }
            "uv" => "    uv = vec2(0.0, 0.0);\n".to_string(),
            other => format!("    {other} = 0.0;\n"),
        })
        .collect();
    compiled(&format!(
        r#"
proc placed {{
  kind     L1
  topology points
  capacity [1, 1] = 1

  emit {emit}

  element {{
{assignments}  }}
}}
"#
    ))
}

/// **The one case the whole saving comes from: a `vec3` followed by a scalar.**
///
/// `position` is 16-byte aligned and 12 bytes long, so 28..32 is addressable
/// and `size` is placed there rather than at 32 — one 16-byte block for the
/// pair rather than two. That is the packing rule this project relies on, and
/// the assertion is that WGSL agrees it is a rule and not a hope: get it wrong
/// and every element after the first reads four bytes into its predecessor.
#[test]
fn naga_agrees_a_scalar_lands_in_the_padding_a_vec3_leaves() {
    let l1 = l1_emitting("position, size");
    let shader = karakuri_codegen::generate_l1(&l1, &[], &[]);
    validate(&shader.source);
    assert_naga_agrees(&shader.source, "Element", &shader.element_layout);
    // **And the number itself, because agreement is not enough on its own.**
    // The struct's *text* is written from the same slots the offsets are, so a
    // wrong element type — `size` declared as a `vec3` — moves the declaration
    // and the offset together and naga agrees about the wrong thing. These two
    // are what says which packing was agreed on.
    assert_eq!(shader.element_layout.offset_of("size"), 28);
    assert_eq!(shader.element_layout.stride, 32);
}

/// The same check across the shapes a real `emit` list takes: nothing but the
/// two unconditional scalars, a lone vector, two vectors with a scalar closing
/// the second's padding, and a `vec2` — the one alignment between 4 and 16.
#[test]
fn naga_agrees_with_the_element_layout_for_every_emit_shape() {
    for emit in [
        "position",
        "position, velocity, age",
        "position, uv, size",
        "uv, age",
        "position, normal, tint",
    ] {
        let l1 = l1_emitting(emit);
        let shader = karakuri_codegen::generate_l1(&l1, &[], &[]);
        validate(&shader.source);
        assert_naga_agrees(&shader.source, "Element", &shader.element_layout);
    }
}

/// **An L4 addresses the very buffer an L1 wrote**, under a struct it declares
/// itself, so the two have to place every member identically — and this asks a
/// front end rather than comparing the generator to itself.
#[test]
fn naga_agrees_with_the_element_layout_in_a_renderer() {
    let layout = layout_for(&drift_shell());
    let shader = karakuri_codegen::generate_l4(&soft_points(), &layout, &[]);
    validate(&shader.source);
    assert_naga_agrees(&shader.source, "Element", &layout);
}

/// **Both of an L2's structs, which are different shapes in one module.**
///
/// The output carries `copy` where the input does not, so the two disagree
/// about everything after the first eight bytes — and a single struct checked
/// twice would not notice a generator that emitted the input's shape under the
/// output's name.
#[test]
fn naga_agrees_with_both_element_layouts_in_a_deform() {
    let shader = compiled_l2(
        MIRROR,
        &[Attr::Position, Attr::Size],
        karakuri_ir::layout::Synthetic::NONE,
    );
    validate(&shader.source);
    let input = karakuri_ir::layout::generate_element_layout(
        &[Attr::Position, Attr::Size],
        karakuri_ir::layout::Synthetic::NONE,
        &[],
    );
    assert_naga_agrees(&shader.source, "ElementIn", &input);
    assert_naga_agrees(&shader.source, "ElementOut", &shader.element_layout);
    assert!(
        shader.element_layout.has_slot("copy"),
        "an amplifier's output carries the copy index: {:?}",
        shader.element_layout
    );
}

/// **A derived attribute's stored slot is placed by the same rules**, and it is
/// the one slot no `emit` list mentions: `velocity` exists here because a
/// downstream consumer named it, with the `velocity_lived` flag beside it in
/// the four bytes that `vec3` leaves. A slot nothing declares is exactly where
/// a placement rule is easiest to get wrong unnoticed.
#[test]
fn naga_agrees_about_a_slot_no_procedure_declared() {
    let l1 = l1_emitting("position");
    let shader = karakuri_codegen::generate_l1(&l1, &[Attr::Velocity], &[]);
    validate(&shader.source);
    assert!(shader.element_layout.has_slot("velocity_lived"));
    assert_naga_agrees(&shader.source, "Element", &shader.element_layout);
}

/// **`source` and a Source slot both lower to a uniform read, in every kind
/// that has one**, and the result is WGSL a front end accepts.
///
/// The two halves are one claim about where the value lives. `source` is
/// `u.seed_salt` — the field `Set::prepare` has been writing the geometry's
/// salt into all along, which is why the read needed no new plumbing — and a
/// declared slot is a `u32` of its own beside it, holding whatever geometry an
/// edge named. Neither is per element: nothing lands in a varying, nothing
/// lands in the element struct, and a fullscreen renderer with no element at
/// all reads both.
#[test]
fn source_and_a_source_slot_lower_to_uniform_reads() {
    let layout = karakuri_ir::layout::generate_element_layout(
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
        &[],
    );

    let l1 = compiled(
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
    );
    let out = karakuri_codegen::generate_l1(&l1, &[], &[]);
    assert!(
        out.source.contains("u.seed_salt == u.source_only"),
        "`source == only` is two uniform loads: {}",
        out.source
    );
    validate(&out.source);

    let l2 = r#"
proc dissolve {
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
    let out = compiled_l2(
        l2,
        &[Attr::Position, Attr::Size],
        karakuri_ir::layout::Synthetic::NONE,
    );
    // **Two slots, two fields.** One would be an edge answering for both, which
    // is the failure the name on the slot exists to prevent.
    assert!(
        out.source.contains("u.source_a") && out.source.contains("u.source_b"),
        "each slot reads its own uniform field: {}",
        out.source
    );
    validate(&out.source);

    // A per-element renderer, and a fullscreen one — which has no element and
    // reads it all the same, because the value is per chain instance.
    for src in [
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
        r#"
proc march {
  kind  L4
  blend additive

  uses only : Source

  fragment {
    var k = 0.0;
    if source == only { k = 1.0; }
    color = vec4(k, length(ray) * 0.0, 0.0, 1.0);
  }
}
"#,
    ] {
        let l4 = compiled(src);
        let out = karakuri_codegen::generate_l4(&l4, &layout, &[]);
        assert!(
            out.source.contains("u.seed_salt == u.source_only"),
            "an L4 reads both out of its uniform: {}",
            out.source
        );
        assert!(
            !out.source.contains("in.source") && !out.source.contains(".source;"),
            "and neither is a varying or an element field: {}",
            out.source
        );
        validate(&out.source);
    }
}

/// **A procedure that declares no Source slot carries no such uniform field**,
/// which is what keeps this from being a `u32` every module in every Set pays
/// for.
#[test]
fn a_procedure_with_no_source_slot_declares_no_field_for_one() {
    let layout = karakuri_ir::layout::generate_element_layout(
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
        &[],
    );
    let l4 = compiled(
        r#"
proc plain {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.016;
  }

  fragment { color = vec4(float(source % 3u) * 0.3, 0.0, 0.0, 1.0); }
}
"#,
    );
    let out = karakuri_codegen::generate_l4(&l4, &layout, &[]);
    assert!(
        !out.source.contains("source_"),
        "no slot, no field: {}",
        out.source
    );
    // And `source` itself still reads, out of the field that was always there.
    assert!(out.source.contains("u.seed_salt"), "{}", out.source);
    assert!(
        out.uniform_layout
            .fields
            .iter()
            .all(|f| !f.name.starts_with("source\u{1}")),
        "the layout carries no slot key either: {:?}",
        out.uniform_layout.fields
    );
    validate(&out.source);
}
