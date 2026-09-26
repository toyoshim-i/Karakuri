use super::common::*;

// Closed form versus accumulating procedure tests.

/// A procedure whose position is a function of `seed` and `t` alone is closed
/// form: any `t` can be evaluated directly, so it needs no priming.
#[test]
fn a_procedure_that_never_reads_what_it_emits_is_closed_form() {
    let src = r#"
proc pure_shell {
  kind     L1
  topology points
  capacity [1, 1024] = 256

  param radius : float [0.1, 8.0] = 2.0

  emit position

  element {
    let u = hash1(seed);
    let v = hash1(seed + 1000u);
    position = sphere_point(u, v) * radius * (1.0 + sin(t));
  }
}
"#;
    assert!(
        check_ok(src).closed_form,
        "a pure function of seed, t and params was classified as accumulating"
    );
}

/// The one the name is about: `age = age + dt` reads what it emits, so the
/// state at `t` is the sum of every step taken to get there.
#[test]
fn a_procedure_that_reads_what_it_emits_is_accumulating() {
    let src = r#"
proc accumulator {
  kind     L1
  topology points
  capacity [1, 1024] = 256

  emit position, age

  element {
    position = vec3(hash1(seed), 0.0, 0.0);
    age      = age + dt;
  }
}
"#;
    assert!(
        !check_ok(src).closed_form,
        "`age = age + dt` reads `age`, which is emitted — this is the definition \
         of accumulating and it was classified closed form"
    );
}

/// Verifies that reading an emitted attribute within nested control flow marks the procedure accumulating.
#[test]
fn a_read_inside_a_nested_if_in_a_for_still_counts() {
    let src = r#"
proc buried_read {
  kind     L1
  topology points
  capacity [1, 1024] = 256

  emit position, age

  element {
    var acc = 0.0;
    for i in 0..4 {
      if age > float(i) * 0.25 {
        acc = acc + 0.1;
      }
    }
    position = vec3(acc, 0.0, 0.0);
    age      = float(1.0);
  }
}
"#;
    assert!(
        !check_ok(src).closed_form,
        "the only read of `age` is in an `if` condition inside a `for`, and it was \
         not found — a classifier that misses it calls this seekable"
    );
}

/// The same read through a local. `let prev = position;` is a read of
/// `position`, and the fact that what is assigned back is spelled `prev` does
/// not make it one.
#[test]
fn a_read_that_reaches_the_value_through_a_local_still_counts() {
    let src = r#"
proc laundered_read {
  kind     L1
  topology points
  capacity [1, 1024] = 256

  emit position

  element {
    let prev = position;
    let drift = vec3(0.0, 0.01, 0.0);
    position = prev + drift;
  }
}
"#;
    assert!(
        !check_ok(src).closed_form,
        "the read of `position` was laundered through a `let` and got past the \
         classifier"
    );
}

/// Asserts that declaring a `spawn` block disqualifies closed-form determination.
#[test]
fn a_spawn_block_disqualifies_even_a_pure_element_block() {
    let src = r#"
proc pure_fountain {
  kind     L1
  topology points
  capacity [1, 1024] = 256

  param spawn_rate : float [0.0, 40000.0] = 100.0

  emit position

  spawn {
    position = vec3(0.0, 0.0, 0.0);
  }

  element {
    position = sphere_point(hash1(seed), hash1(seed + 7u)) * t;
  }
}
"#;
    let checked = check_ok(src);
    assert!(
        !checked.closed_form,
        "a procedure that spawns was called seekable; the elements that would have \
         been born on the way to `t` do not exist when `t` is jumped to"
    );
}

/// `kill()` disqualifies for the mirror-image reason: a killed element stays
/// killed, so which elements are alive at `t` is a function of every step taken
/// to get there rather than of `t`.
#[test]
fn a_kill_disqualifies_even_a_pure_element_block() {
    let src = r#"
proc pure_cull {
  kind     L1
  topology points
  capacity [1, 1024] = 256

  emit position

  element {
    position = sphere_point(hash1(seed), hash1(seed + 7u)) * 2.0;
    if t > 5.0 && t < 5.1 {
      kill();
    }
  }
}
"#;
    assert!(
        !check_ok(src).closed_form,
        "a procedure that can `kill()` was called seekable; arriving at t = 6 in one \
         step removes nothing this would have removed at t = 5.05"
    );
}

/// An L4 procedure holds no per-element state at all, so the property is
/// vacuously true of it — and a Set is closed form when both of its procedures
/// are, which in practice means when its L1 is.
#[test]
fn an_l4_procedure_is_vacuously_closed_form() {
    let src = include_str!("../fixtures/soft_points.kir");
    assert!(
        check_ok(src).closed_form,
        "L4 is stateless and emits nothing, so there is nothing for it to warm"
    );
}

/// The canonical examples, as a check that the classifier's answers are the
/// ones the specification's own material deserves: `drift_shell` integrates a
/// velocity and spawns, and is accumulating on both counts.
#[test]
fn drift_shell_is_accumulating() {
    let checked = check_ok(include_str!("../fixtures/drift_shell.kir"));
    assert!(!checked.closed_form);
}

/// Verifies that L4 renderers are treated as closed form regardless of consumed attributes.
#[test]
fn an_l4_is_vacuously_closed_form() {
    let src = r#"
proc odd_l4 {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.016;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    let checked = check_ok(src);
    assert_eq!(checked.kind, karakuri_ir::Kind::L4);
    assert!(
        checked.closed_form,
        "an L4 holds no per-element state at all, so the flag is supposed to be \
         vacuously true — a renderer reading what it draws made the whole Set \
         look like it needed priming"
    );
}

// ---------------------------------------------------------------------------
// `topology lines`: what an L4 draws is inferred from `clip_b`, not declared.
// ---------------------------------------------------------------------------

/// An L4 whose `vertex` block assigns a second endpoint is drawing segments,
/// and the check pass is where that is decided — nothing downstream re-derives
/// it, so a wrong answer here silently picks the wrong quad expansion.
#[test]
fn an_l4_that_assigns_clip_b_is_inferred_to_draw_lines() {
    let src = r#"
proc streaks {
  kind  L4
  blend additive

  consumes position, velocity

  vertex {
    clip       = camera * vec4(position, 1.0);
    clip_b     = camera * vec4(position - velocity, 1.0);
    point_rate = 0.008;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    let checked = check_ok(src);
    assert_eq!(
        checked.topology,
        Some(Topology::Lines),
        "a second endpoint is the only thing that could make a procedure draw segments"
    );
}

/// The control for the test above, and not a redundant one: an inference that
/// answered `Lines` unconditionally would satisfy it, and this is what says
/// the answer depends on the source.
#[test]
fn an_l4_without_clip_b_is_inferred_to_draw_points() {
    let src = r#"
proc sprites {
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
"#;
    let checked = check_ok(src);
    assert_eq!(checked.topology, Some(Topology::Points));
}

/// A procedure either draws segments or it does not. Assigning the far end
/// under a condition would leave it uninitialised on the other path, which is
/// a stroke laid along whatever the register happened to hold.
#[test]
fn clip_b_assigned_on_only_one_path_is_rejected() {
    let src = r#"
proc sometimes {
  kind  L4
  blend additive

  consumes position, velocity

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.008;
    if length(velocity) > 0.5 {
      clip_b = camera * vec4(position - velocity, 1.0);
    }
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("clip_b") && e.message.contains("every path")),
        "expected a coverage diagnostic naming `clip_b`, got: {errs:?}"
    );
}

/// `clip_b` is a vertex output, and the fragment stage has no second endpoint
/// to place. Reusing the existing output-legality rule rather than a new one is
/// the point — a new output that quietly escaped it would be assignable in a
/// block that cannot lower it.
#[test]
fn clip_b_in_a_fragment_block_is_rejected() {
    let src = r#"
proc misplaced {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.008;
  }

  fragment {
    clip_b = vec4(1.0, 1.0, 1.0, 1.0);
    color  = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter().any(|e| e.message.contains("clip_b")),
        "expected a diagnostic naming `clip_b`, got: {errs:?}"
    );
}

/// An L4 still may not declare `topology`. The inference is not a second way
/// of saying it — it is the only way, and a header field would be a place for
/// the file to contradict its own `vertex` block.
#[test]
fn an_l4_that_declares_topology_is_still_rejected() {
    let src = r#"
proc declared {
  kind     L4
  topology lines
  blend    additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    clip_b     = camera * vec4(position, 1.0);
    point_rate = 0.008;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter().any(|e| e.message.contains("topology")
            && e.hint.as_deref().unwrap_or_default().contains("clip_b")),
        "expected the diagnostic to point at `clip_b` as the way to say it, got: {errs:?}"
    );
}

/// An L1 may declare `topology lines`, and it survives checking as itself.
/// Nothing lowers from it — see `Set::build` — but a declaration that silently
/// became `points` would make the geometry's own statement of what it is a lie.
#[test]
fn an_l1_may_declare_topology_lines() {
    let src = r#"
proc strands {
  kind     L1
  topology lines
  capacity [1, 64] = 8

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#;
    let checked = check_ok(src);
    assert_eq!(checked.topology, Some(Topology::Lines));
}

// ---------------------------------------------------------------------------
// `fullscreen`: an L4 with no `vertex` block draws the frame.
// ---------------------------------------------------------------------------

/// The declaration is the absence. A procedure with nothing to place per
/// element has nothing for a vertex block to do, so not having one is how it
/// says it covers the frame.
#[test]
fn an_l4_with_no_vertex_block_is_inferred_to_draw_the_whole_frame() {
    let src = r#"
proc marcher {
  kind  L4
  blend additive

  fragment {
    let d = length(ray) + length(eye) + point_coord.x;
    color = vec4(d, d, d, 1.0);
  }
}
"#;
    let checked = check_ok(src);
    assert_eq!(checked.topology, Some(Topology::Fullscreen));
}

/// Consumes declaration requires vertex stage element access.
#[test]
fn a_fullscreen_l4_that_consumes_attributes_is_rejected() {
    let src = r#"
proc marcher {
  kind  L4
  blend additive

  consumes position

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter().any(|e| e.message.contains("whole frame")
            && e.hint.as_deref().unwrap_or_default().contains("vertex")),
        "expected a diagnostic pairing the two, got: {errs:?}"
    );
}
