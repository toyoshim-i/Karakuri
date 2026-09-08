//! The two small draws, end to end on a device: that they are taken at
//! [`rungs`]'s two sizes, that they leave the Set as it was found, and that a
//! Set whose floor is not knowable is refused without anything being drawn.
//!
//! **What is asserted here and what is not.** The arithmetic of the fit is
//! unit-tested in `src/estimate.rs` against synthetic rungs built from a known
//! `a` and `b`, and needs no adapter. What needs one is everything a caller
//! would discover the hard way: a probe left pointing at the wrong size, a Set
//! left at a rung's size so its first on-air frame draws the wrong aspect
//! ratio, a Set left stepped so the slot arrives warm when the design says it
//! arrives cold. Each of those is silent, and each is one line in
//! `estimate_above_floor`.
//!
//! **No timing is asserted.** `docs/contributing.md` §1 is why: this machine's
//! GPU timestamps demote to a host clock, the figure is biased high by
//! submission and synchronization, and a threshold on it would be a test that
//! passes for the wrong reason on a faster machine. What *is* asserted about
//! the numbers is their structure — that the terms sum to the answer, and that
//! either a fit or a named refusal comes back. The measurements the rule was
//! chosen against are in `examples/small_draw.rs` and quoted in the module doc
//! with the instrument that took them.

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
mod gpu {
    use karakuri_engine::estimate::{
        estimate, estimate_above_floor, rungs, sub_pixel_floor_rows, Floor, Unfit,
        PREPARATION_RESOLUTION,
    };
    use karakuri_engine::set::{Edge, Layering, Wiring};
    use karakuri_engine::swap::PROBE_RESOLUTION;
    use karakuri_engine::{Gpu, Probe, Set};
    use karakuri_ir::rate::Bound;
    use karakuri_ir::typed::Checked;
    use karakuri_ir::Topology;

    const CAPACITY: u32 = 4_096;
    const SEED: u32 = 19_274;
    /// A caller's own viewport, deliberately neither the probe's nor either
    /// rung's, so "put it back" is distinguishable from "leave it".
    const CALLER_VIEWPORT: (u32, u32) = (800, 600);
    /// The rate `plain_dots` writes, as a literal, so the floor this file uses
    /// is the procedure's own rather than a number picked for the test.
    const PLAIN_DOTS_RATE: f32 = 0.004;

    fn gpu() -> Gpu {
        Gpu::headless().expect("no GPU available")
    }

    fn compile(src: &str) -> Checked {
        let proc = karakuri_ir::parse(src).expect("parse");
        karakuri_ir::check::check(&proc).expect("check")
    }

    /// The smallest honest per-element pairing: one geometry, one sprite
    /// renderer, at a capacity that keeps this test a check rather than a
    /// benchmark.
    fn points_set(gpu: &Gpu) -> Set {
        let l1 = compile(
            r#"
proc dots {
  kind     L1
  topology points
  capacity [1024, 65536] = 4096

  param radius : float [0.1, 8.0] = 2.0

  emit position, velocity, age

  element {
    let u = hash1(seed);
    let v = hash1(seed + 1000u);
    position = sphere_point(u, v) * radius;
    velocity = vec3(0.0, 0.0, 0.0);
    age      = age + dt;
  }
}
"#,
        );
        let l4 = compile(
            r#"
proc plain_dots {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.004;
  }

  fragment {
    color = vec4(0.2, 0.2, 0.2, 1.0);
  }
}
"#,
        );
        build(gpu, &l1, &l4)
    }

    fn build(gpu: &Gpu, l1: &Checked, l4: &Checked) -> Set {
        let mut set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(l1, CAPACITY)],
            &[],
            &[],
            &[],
            &[l4],
            Layering::Overdraw,
            SEED,
            &[],
            Wiring {
                edges: &[] as &[Edge],
                ..Default::default()
            },
        )
        .expect("build");
        set.resize(&gpu.device, CALLER_VIEWPORT.0, CALLER_VIEWPORT.1);
        set
    }

    fn fullscreen_set(gpu: &Gpu) -> Set {
        let l1 = compile(
            r#"
proc dots {
  kind     L1
  topology points
  capacity [1024, 65536] = 4096

  emit position, velocity, age

  element {
    position = vec3(hash1(seed), hash1(seed + 1u), hash1(seed + 2u));
    velocity = vec3(0.0, 0.0, 0.0);
    age      = age + dt;
  }
}
"#,
        );
        let l4 = compile(
            r#"
proc wash {
  kind  L4
  blend additive

  param level : float [0.0, 1.0] = 0.2

  fragment {
    color = vec4(level, level * 0.5, level, 1.0);
  }
}
"#,
        );
        build(gpu, &l1, &l4)
    }

    /// **A renderer whose rate nothing can bound**: `size` is an attribute, so
    /// there is no declaration to read it against. `examples/hard_dots.kir`
    /// ships the same expression.
    fn unbounded_set(gpu: &Gpu) -> Set {
        let l1 = compile(
            r#"
proc dots {
  kind     L1
  topology points
  capacity [1024, 65536] = 4096

  emit position, velocity, age, size

  element {
    position = vec3(hash1(seed), hash1(seed + 1u), hash1(seed + 2u));
    velocity = vec3(0.0, 0.0, 0.0);
    age      = age + dt;
    size     = hash1(seed + 3u);
  }
}
"#,
        );
        let l4 = compile(
            r#"
proc loose_dots {
  kind  L4
  blend additive

  param dot_scale : float [0.005, 0.05] = 0.01

  consumes position, size

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = dot_scale * size;
  }

  fragment {
    color = vec4(0.2, 0.2, 0.2, 1.0);
  }
}
"#,
        );
        build(gpu, &l1, &l4)
    }

    /// **A renderer whose rate is one param**, so a write to that param is the
    /// whole of what the bound rests on.
    fn scaled_set(gpu: &Gpu) -> Set {
        let l1 = compile(
            r#"
proc dots {
  kind     L1
  topology points
  capacity [1024, 65536] = 4096

  emit position, velocity, age

  element {
    position = vec3(hash1(seed), hash1(seed + 1u), hash1(seed + 2u));
    velocity = vec3(0.0, 0.0, 0.0);
    age      = age + dt;
  }
}
"#,
        );
        let l4 = compile(
            r#"
proc scaled_dots {
  kind  L4
  blend additive

  param point_scale : float [0.005, 0.05] = 0.01

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = point_scale;
  }

  fragment {
    color = vec4(0.2, 0.2, 0.2, 1.0);
  }
}
"#,
        );
        build(gpu, &l1, &l4)
    }

    /// The floor `plain_dots` implies, and the two rungs that fit under a
    /// 720-row target above it. `0.004` is one pixel at 250 rows, which leaves
    /// the quarter-height rung below the floor and the half-height one above
    /// it — so this file exercises the raise rather than the default pair.
    fn floor() -> u32 {
        sub_pixel_floor_rows(PLAIN_DOTS_RATE).expect("a positive rate has a floor")
    }

    /// **The rule, on real draws.** Two rungs come back, low area first, at the
    /// sizes `rungs` places; the fit's two terms sum to the answer it hands
    /// out; and the measurements travel with it, instrument and capacity and
    /// size, so a consumer can check what it was given.
    #[test]
    fn two_small_draws_are_fitted_and_carry_what_took_them() {
        let gpu = gpu();
        let mut probe = Probe::new(&gpu.device, &gpu.queue, gpu.timestamps, PROBE_RESOLUTION);
        let mut set = points_set(&gpu);

        let e = estimate_above_floor(
            &mut probe,
            &gpu.device,
            &gpu.queue,
            &mut set,
            PROBE_RESOLUTION,
            floor(),
        );

        let placed = rungs(PROBE_RESOLUTION, floor()).expect("two rungs fit under 720 rows");
        let taken = e.rungs.expect("both draws recorded");
        assert_eq!(
            [taken[0].resolution, taken[1].resolution],
            placed,
            "the draws have to be the two small ones, or there is no fit here at all"
        );
        assert_eq!(taken[0].capacity, CAPACITY);
        assert_eq!(taken[1].capacity, CAPACITY);
        assert_eq!(
            taken[0].method, taken[1].method,
            "one probe, one instrument"
        );
        assert_eq!(e.target, PROBE_RESOLUTION);
        assert_eq!(e.topologies, vec![Topology::Points]);
        assert_eq!(e.floor, Some(floor()));

        // The fit may legitimately fail on this machine — a quarter of a
        // million primitives is not what this Set is, but thermal drift and a
        // host clock's noise can still put the slope below zero, and that is a
        // refusal by design rather than a fault. What must hold either way is
        // that the answer is the two terms and nothing else.
        match e.fit {
            Ok(f) => {
                eprintln!(
                    "fitted via {:?}: {:.3} ms = {:.3} invariant + {:.3} fragment",
                    e.method().expect("a fit was drawn"),
                    f.ms,
                    f.invariant_ms,
                    f.fragment_ms
                );
                assert!(f.invariant_ms >= 0.0 && f.fragment_ms >= 0.0);
                assert!(
                    (f.ms - (f.invariant_ms + f.fragment_ms)).abs() < 1e-2,
                    "{} is not {} + {}",
                    f.ms,
                    f.invariant_ms,
                    f.fragment_ms
                );
                assert_eq!(e.ms(), Some(f.ms));
            }
            Err(why) => {
                eprintln!("refused, which is a valid outcome on a noisy clock: {why:?}");
                assert!(matches!(
                    why,
                    Unfit::FragmentTermNegative { .. } | Unfit::InvariantTermNegative { .. }
                ));
                assert_eq!(e.ms(), None, "a refusal hands out no number");
            }
        }
    }

    /// **The Set is left exactly as it was found**, which is the whole of what
    /// makes this safe to call on a slot that is about to go on air: the
    /// caller's viewport back (the camera derives its aspect ratio from it, so
    /// a Set left at a rung's size draws a different picture on its first
    /// frame), and `t` back to zero (the design says a primed Set arrives cold,
    /// and `Set::rewind` is what puts it there). **Two draws rather than one
    /// doubles the number of places that can leak.**
    #[test]
    fn the_set_is_left_at_its_own_size_and_cold() {
        let gpu = gpu();
        let mut probe = Probe::new(&gpu.device, &gpu.queue, gpu.timestamps, PROBE_RESOLUTION);
        let mut set = points_set(&gpu);

        estimate_above_floor(
            &mut probe,
            &gpu.device,
            &gpu.queue,
            &mut set,
            PROBE_RESOLUTION,
            floor(),
        );

        assert_eq!(
            set.viewport(),
            CALLER_VIEWPORT,
            "the caller's viewport was not restored"
        );
        assert_eq!(set.time(), 0.0, "the Set was left stepped");
    }

    /// **One probe, resized rather than replaced.** `Probe::run` demotes itself
    /// to a host clock for life on the first implausible sample, so a second
    /// probe can answer on a different scale and the two rungs would then be
    /// two numbers the fit subtracts. `estimate` therefore moves the one it is
    /// given and leaves it at the **upper** rung, which is the larger of the
    /// two and the cheaper end to alternate from.
    #[test]
    fn the_probe_is_moved_to_the_upper_rung_and_left_there() {
        let gpu = gpu();
        let mut probe = Probe::new(&gpu.device, &gpu.queue, gpu.timestamps, PROBE_RESOLUTION);
        assert_eq!(probe.resolution(), PROBE_RESOLUTION);
        let mut set = points_set(&gpu);

        let first = estimate_above_floor(
            &mut probe,
            &gpu.device,
            &gpu.queue,
            &mut set,
            PROBE_RESOLUTION,
            floor(),
        );
        let [_, upper] = rungs(PROBE_RESOLUTION, floor()).expect("two rungs fit");
        assert_eq!(upper, PREPARATION_RESOLUTION);
        assert_eq!(probe.resolution(), upper);

        // And a second call finds it already there and still answers the same
        // way, which is the property that makes the resize an optimisation
        // rather than a state a caller has to track.
        let second = estimate_above_floor(
            &mut probe,
            &gpu.device,
            &gpu.queue,
            &mut set,
            PROBE_RESOLUTION,
            floor(),
        );
        assert_eq!(
            second.rungs.expect("drawn")[1].resolution,
            first.rungs.expect("drawn")[1].resolution
        );
        assert_eq!(second.method(), first.method());
    }

    /// **A per-element Set states its own floor now, and `estimate` draws.**
    /// `point_rate` is still a vertex-stage expression nothing on this side
    /// evaluates; what changed is that `karakuri_ir::rate` *bounds* it, and
    /// `plain_dots` writes a literal, so the bound is the literal and the floor
    /// is the 250 rows it implies.
    ///
    /// This is the case that answered `Unfit::FloorUnknown` before ADR-0285 —
    /// with the same Set, the same probe and the same call.
    #[test]
    fn a_per_element_set_states_its_own_floor_and_is_drawn() {
        let gpu = gpu();
        let mut probe = Probe::new(&gpu.device, &gpu.queue, gpu.timestamps, PROBE_RESOLUTION);
        let mut set = points_set(&gpu);

        let e = estimate(
            &mut probe,
            &gpu.device,
            &gpu.queue,
            &mut set,
            PROBE_RESOLUTION,
        );

        assert_eq!(e.floor, Some(floor()), "the floor is the procedure's own");
        assert_eq!(e.topologies, vec![Topology::Points]);
        let taken = e.rungs.expect("both rungs drawn");
        assert_eq!(
            [taken[0].resolution, taken[1].resolution],
            rungs(PROBE_RESOLUTION, floor()).expect("two rungs fit"),
        );

        // **And it says where the floor came from**, which is what makes the
        // number checkable: one bound per renderer, naming the procedure and
        // the declarations it rests on.
        match &e.floor_from {
            Floor::Analysed {
                bounds,
                contradicted,
            } => {
                assert_eq!(contradicted, &None);
                assert_eq!(bounds.len(), 1);
                assert_eq!(bounds[0].procedure, "plain_dots");
                assert_eq!(bounds[0].rate(), Some(PLAIN_DOTS_RATE));
            }
            other => panic!("the floor was read off the Set, not stated: {other:?}"),
        }
        assert_eq!(set.viewport(), CALLER_VIEWPORT);
        assert_eq!(set.time(), 0.0);
    }

    /// **A rate nothing can bound is still refused, and still without a draw.**
    /// `size` is an attribute — whatever the simulation left in it — so there
    /// is no declaration to read and no height at which every primitive is a
    /// pixel across. `hard_dots` ships this exact expression.
    ///
    /// The refusal is the same word it always was, and the *working* is on
    /// `floor_from`: an analysis that guesses here is worse than one that
    /// refuses, and this is the test that the refusal is reachable.
    #[test]
    fn a_rate_read_from_an_attribute_is_refused_without_being_drawn() {
        let gpu = gpu();
        let mut probe = Probe::new(&gpu.device, &gpu.queue, gpu.timestamps, PROBE_RESOLUTION);
        let mut set = unbounded_set(&gpu);

        let e = estimate(
            &mut probe,
            &gpu.device,
            &gpu.queue,
            &mut set,
            PROBE_RESOLUTION,
        );

        assert_eq!(e.fit, Err(Unfit::FloorUnknown));
        assert_eq!(e.floor, None);
        assert_eq!(e.rungs, None, "nothing may be drawn for a refusal");
        assert_eq!(e.ms(), None);
        assert_eq!(e.topologies, vec![Topology::Points]);
        match &e.floor_from {
            Floor::Analysed { bounds, .. } => assert!(
                matches!(bounds[0].bound, Bound::Unbounded { .. }),
                "the refusal has to name what could not be bounded: {bounds:?}"
            ),
            other => panic!("expected an analysed floor, got {other:?}"),
        }
        assert_eq!(
            probe.resolution(),
            PROBE_RESOLUTION,
            "the probe was moved for a draw that never happened"
        );
        assert_eq!(set.viewport(), CALLER_VIEWPORT);
    }

    /// **A held value outside its declaration falsifies the bound taken over
    /// it, and the estimate refuses rather than using it.**
    ///
    /// The bound is over the *declared* range so that it survives a fader, and
    /// **nothing in this engine clamps a write to a declared range** — a
    /// `--param` below the minimum is accepted and reaches the uniform. A floor
    /// computed from a declaration the run is not honouring would be wrong in
    /// the one direction ADR-0245 forbids, so it is not used at all.
    #[test]
    fn a_param_written_below_its_declaration_refuses_the_floor() {
        let gpu = gpu();
        let mut probe = Probe::new(&gpu.device, &gpu.queue, gpu.timestamps, PROBE_RESOLUTION);
        let mut set = scaled_set(&gpu);

        // In range first: the same Set, answered. **The floor is the declared
        // minimum's and not this value's** — 0.005 is one pixel at 200 rows,
        // and 201 because the nearest `f32` to 0.005 is a shade under it and
        // the floor rounds up.
        assert!(set.set_param("point_scale", 0.01) > 0);
        let ok = estimate(
            &mut probe,
            &gpu.device,
            &gpu.queue,
            &mut set,
            PROBE_RESOLUTION,
        );
        assert_eq!(ok.floor, Some(201));

        // A tenth of the declared minimum, which no fader could reach.
        assert!(set.set_param("point_scale", 0.0005) > 0);
        let e = estimate(
            &mut probe,
            &gpu.device,
            &gpu.queue,
            &mut set,
            PROBE_RESOLUTION,
        );
        assert_eq!(e.fit, Err(Unfit::FloorUnknown));
        assert_eq!(e.floor, None);
        assert_eq!(e.rungs, None, "nothing may be drawn for a refusal");
        match &e.floor_from {
            Floor::Analysed { contradicted, .. } => {
                let (param, value, declared) =
                    contradicted.as_ref().expect("the write has to be named");
                assert_eq!(param, "point_scale");
                assert_eq!(*value, 0.0005);
                assert_eq!(*declared, [0.005, 0.05]);
            }
            other => panic!("expected an analysed floor, got {other:?}"),
        }
    }

    /// **A fullscreen Set has no primitive, so `estimate` answers it.** The
    /// topology is not the fit's discriminator — the module doc says why — but
    /// it is what decides whether there is a sub-pixel floor at all, and a
    /// procedure with no `vertex` block emits no `point_rate` to fall under
    /// one.
    #[test]
    fn a_fullscreen_set_has_no_floor_so_both_rungs_are_drawn() {
        let gpu = gpu();
        let mut probe = Probe::new(&gpu.device, &gpu.queue, gpu.timestamps, PROBE_RESOLUTION);
        let mut set = fullscreen_set(&gpu);

        let e = estimate(
            &mut probe,
            &gpu.device,
            &gpu.queue,
            &mut set,
            PROBE_RESOLUTION,
        );

        assert_eq!(e.topologies, vec![Topology::Fullscreen]);
        assert_eq!(e.floor, Some(1));
        let taken = e.rungs.expect("a fullscreen Set is drawn twice");
        assert_eq!(
            [taken[0].resolution, taken[1].resolution],
            [(320, 180), PREPARATION_RESOLUTION],
            "half and a quarter of the target's height"
        );
        assert_eq!(set.viewport(), CALLER_VIEWPORT);
        assert_eq!(set.time(), 0.0);
        eprintln!("fullscreen: {:?}", e.fit);
    }
}
