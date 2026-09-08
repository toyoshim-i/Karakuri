//! The two small draws, end to end on a device: that they are taken at
//! [`rungs`]'s two sizes, that they leave the Set as it was found, and that a
//! Set whose floor is not knowable is drawn under the loosened reading with the
//! share it can hide on the record (ADR-0293). `mod corpus` at the end is the
//! same question asked of `examples/` and needs no device.
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
        FLOORED_SHARE_ALLOWED, PREPARATION_RESOLUTION,
    };
    use karakuri_engine::set::{Edge, Layering, Wiring};
    use karakuri_engine::{Gpu, Probe, Set};
    use karakuri_ir::rate::Bound;
    use karakuri_ir::typed::Checked;
    use karakuri_ir::Topology;

    /// **A size to measure at**, and a fixture rather than a reference.
    ///
    /// It was `swap::PROBE_RESOLUTION` until ADR-0303, which removed that
    /// constant: this application has an output size and a preview size and no
    /// third one, so the size a measurement is taken at is named by whoever
    /// knows the layout. Nothing here has a layout, so these tests name one
    /// and it is 1280x720 because that is what they were written against.
    const AT: (u32, u32) = (1280, 720);

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
        let mut probe = Probe::new(&gpu.device, &gpu.queue, gpu.timestamps, AT);
        let mut set = points_set(&gpu);

        let e = estimate_above_floor(&mut probe, &gpu.device, &gpu.queue, &mut set, AT, floor());

        let placed = rungs(AT, floor()).expect("two rungs fit under 720 rows");
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
        assert_eq!(e.target, AT);
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
        let mut probe = Probe::new(&gpu.device, &gpu.queue, gpu.timestamps, AT);
        let mut set = points_set(&gpu);

        estimate_above_floor(&mut probe, &gpu.device, &gpu.queue, &mut set, AT, floor());

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
        let mut probe = Probe::new(&gpu.device, &gpu.queue, gpu.timestamps, AT);
        assert_eq!(probe.resolution(), AT);
        let mut set = points_set(&gpu);

        let first =
            estimate_above_floor(&mut probe, &gpu.device, &gpu.queue, &mut set, AT, floor());
        let [_, upper] = rungs(AT, floor()).expect("two rungs fit");
        assert_eq!(upper, PREPARATION_RESOLUTION);
        assert_eq!(probe.resolution(), upper);

        // And a second call finds it already there and still answers the same
        // way, which is the property that makes the resize an optimisation
        // rather than a state a caller has to track.
        let second =
            estimate_above_floor(&mut probe, &gpu.device, &gpu.queue, &mut set, AT, floor());
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
        let mut probe = Probe::new(&gpu.device, &gpu.queue, gpu.timestamps, AT);
        let mut set = points_set(&gpu);

        let e = estimate(&mut probe, &gpu.device, &gpu.queue, &mut set, AT);

        assert_eq!(e.floor, Some(floor()), "the floor is the procedure's own");
        assert_eq!(e.topologies, vec![Topology::Points]);
        let taken = e.rungs.expect("both rungs drawn");
        assert_eq!(
            [taken[0].resolution, taken[1].resolution],
            rungs(AT, floor()).expect("two rungs fit"),
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

    /// **A rate nothing can bound is drawn now, and says so.** `size` is an
    /// attribute — whatever the simulation left in it — so there is no
    /// declaration to read and no height at which every primitive is a pixel
    /// across. `hard_dots` ships this exact expression, and this answered
    /// `Unfit::FloorUnknown` without drawing until ADR-0293.
    ///
    /// **A floor that is not known is the greatest floor there is**, which is
    /// a rung placement rather than a refusal: the share ADR-0245's flooring
    /// can hide does not depend on the rates at all, so an unbounded one is
    /// covered by the same arithmetic as a bounded one. What the estimate owes
    /// in exchange is to say it — `floor` is [`None`], `floored` carries the
    /// share and the correction, and the working is still on `floor_from`.
    #[test]
    fn a_rate_nothing_can_bound_is_drawn_under_the_loosened_floor() {
        let gpu = gpu();
        let mut probe = Probe::new(&gpu.device, &gpu.queue, gpu.timestamps, AT);
        let mut set = unbounded_set(&gpu);

        let e = estimate(&mut probe, &gpu.device, &gpu.queue, &mut set, AT);

        assert_eq!(e.floor, None, "nothing bounded the rate, and it says so");
        assert_eq!(e.topologies, vec![Topology::Points]);
        let taken = e
            .rungs
            .expect("an unknown floor is drawn under, not refused");
        assert_eq!(
            [taken[0].resolution, taken[1].resolution],
            rungs(AT, u32::MAX).expect("the accurate pair"),
            "an unknown floor is passed on as the greatest floor there is"
        );
        let floored = e.floored.expect("the lower rung is under an unknown floor");
        assert!(
            floored.share <= FLOORED_SHARE_ALLOWED && floored.share > 0.0,
            "{floored:?} is not a share inside the allowance"
        );
        assert!(
            (floored.correction - 1.0 / (1.0 - floored.share)).abs() < 1e-9,
            "the correction has to be the share's: {floored:?}"
        );
        match &e.floor_from {
            Floor::Analysed { bounds, .. } => assert!(
                matches!(bounds[0].bound, Bound::Unbounded { .. }),
                "what could not be bounded still has to be named: {bounds:?}"
            ),
            other => panic!("expected an analysed floor, got {other:?}"),
        }
        // **The correction is on the answer and not on the line.** A caller
        // reading `ms` gets the number that rounds toward refusing; one
        // reading the two terms gets the line that was fitted.
        if let Ok(f) = e.fit {
            let raw = f.invariant_ms + f.fragment_ms;
            assert!(f.ms > raw, "{} is not the corrected form of {raw}", f.ms);
            assert!(
                (f64::from(f.ms) - f64::from(raw) * floored.correction).abs() < 1e-2,
                "{} is not {raw} times {}",
                f.ms,
                floored.correction
            );
        }
        assert_eq!(set.viewport(), CALLER_VIEWPORT);
        assert_eq!(set.time(), 0.0);
    }

    /// **A held value outside its declaration falsifies the bound taken over
    /// it, so the floor becomes unknown — and an unknown floor is placed
    /// under, not refused.**
    ///
    /// `Set::rate_bounds` is over the *declared* range, and **nothing in this
    /// engine clamps a write to one** — a `--param` below the minimum is
    /// accepted and reaches the uniform. A floor computed from a declaration
    /// the run is not honouring would be wrong in the one direction ADR-0245
    /// forbids, so it is still not used. What changed with ADR-0293 is what
    /// happens next: the estimate places the accurate pair and pays the
    /// correction, rather than handing back no number at all.
    #[test]
    fn a_param_written_below_its_declaration_makes_the_floor_unknown() {
        let gpu = gpu();
        let mut probe = Probe::new(&gpu.device, &gpu.queue, gpu.timestamps, AT);
        let mut set = scaled_set(&gpu);

        // In range first: the same Set, answered. **The floor is the declared
        // minimum's and not this value's** — 0.005 is one pixel at 200 rows,
        // and 201 because the nearest `f32` to 0.005 is a shade under it and
        // the floor rounds up.
        assert!(set.set_param("point_scale", 0.01) > 0);
        let ok = estimate(&mut probe, &gpu.device, &gpu.queue, &mut set, AT);
        assert_eq!(ok.floor, Some(201));

        // A tenth of the declared minimum, which no fader could reach.
        assert!(set.set_param("point_scale", 0.0005) > 0);
        let e = estimate(&mut probe, &gpu.device, &gpu.queue, &mut set, AT);
        assert_eq!(
            e.floor, None,
            "the bound was falsified, so there is no floor"
        );
        assert!(
            e.rungs.is_some(),
            "an unknown floor is drawn under rather than refused"
        );
        let floored = e.floored.expect("the lower rung is under an unknown floor");
        assert!(floored.share > 0.0 && floored.share <= FLOORED_SHARE_ALLOWED);
        // And the in-range estimate above was clear of its floor, so the two
        // are different statements about the same Set — which is the whole of
        // why `floored` is on the record.
        assert_eq!(ok.floored, None, "201 rows is under the half-height rung");
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
        let mut probe = Probe::new(&gpu.device, &gpu.queue, gpu.timestamps, AT);
        let mut set = fullscreen_set(&gpu);

        let e = estimate(&mut probe, &gpu.device, &gpu.queue, &mut set, AT);

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

/// **What the floor does to the material this instrument ships**, which is the
/// only place the loosening's worth can be settled.
///
/// `crates/karakuri-ir/tests/rate.rs` asks what each renderer's rate *bounds*
/// to. This asks the question above it: with that bound in hand, does
/// [`rungs`] place a pair at the reference target, and how much can the
/// flooring hide from the fit that follows. The answers are written out one
/// procedure at a time rather than counted, because a count that moved would
/// say nothing about which way.
///
/// **Both readings, side by side.** ADR-0282 settled which state a number
/// about a Set is taken over — a declared value is the value in the untouched
/// state, and where somebody moved one the held value *is* the value — and
/// `Set::rate_bounds` does not follow it yet: it is computed once at build
/// over the whole declared range. So each row carries both, and the gap
/// between the two columns is exactly what following ADR-0282 here would buy.
/// ADR-0293 says what it would take.
///
/// No device: this is [`rungs`] and [`floored_share`], which are arithmetic.
mod corpus {
    use std::collections::{BTreeMap, HashMap};
    use std::path::{Path, PathBuf};

    use karakuri_engine::estimate::{
        floored_share, rungs, sub_pixel_floor_rows, FLOORED_SHARE_ALLOWED,
    };
    use karakuri_ir::rate::{point_rate_bound, point_rate_bound_at, Bound};
    use karakuri_ir::typed::Checked;
    use karakuri_ir::Kind;

    /// **A size to measure at**, and a fixture rather than a reference.
    ///
    /// It was `swap::PROBE_RESOLUTION` until ADR-0303, which removed that
    /// constant: this application has an output size and a preview size and no
    /// third one, so the size a measurement is taken at is named by whoever
    /// knows the layout. Nothing here has a layout, so these tests name one
    /// and it is 1280x720 because that is what they were written against.
    const AT: (u32, u32) = (1280, 720);

    /// What [`rungs`] and [`floored_share`] between them answer for a floor.
    #[derive(Debug, PartialEq)]
    enum Placed {
        /// Both rungs clear the floor: nothing is rounded up at either and the
        /// fit is exact in ADR-0245's terms.
        Clear,
        /// The lower rung is under it, and this is the share of the target's
        /// fragment cost that can hide there — never above
        /// [`FLOORED_SHARE_ALLOWED`], because that is what the upper rung was
        /// moved to guarantee.
        Floored(f64),
        /// No pair could be placed. Nothing in `examples/` reaches this after
        /// ADR-0293, and every per-element procedure did before it.
        Refused,
    }

    /// The floor a bound implies, with `u32::MAX` for *not known* — which is
    /// what [`karakuri_engine::estimate::estimate`] passes on, and is the worst
    /// case rather than a sentinel.
    fn floor_of(bound: &Bound) -> u32 {
        match bound {
            Bound::NoPrimitive => 1,
            Bound::AtLeast { rate, .. } => {
                sub_pixel_floor_rows(*rate).expect("a bounded rate is positive and finite")
            }
            Bound::Unbounded { .. } => u32::MAX,
        }
    }

    fn placed(floor: u32) -> Placed {
        match rungs(AT, floor) {
            Err(_) => Placed::Refused,
            Ok([low, high]) => {
                let share = floored_share(low, high, AT, floor);
                if share == 0.0 {
                    Placed::Clear
                } else {
                    Placed::Floored(share)
                }
            }
        }
    }

    fn kir_files() -> Vec<PathBuf> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "kir"))
            .collect();
        found.sort();
        found
    }

    fn renderers() -> BTreeMap<String, Checked> {
        let mut found = BTreeMap::new();
        for path in kir_files() {
            let src = std::fs::read_to_string(&path).expect("read");
            let proc = karakuri_ir::parse(&src)
                .unwrap_or_else(|e| panic!("{} does not parse: {e:?}", path.display()));
            let checked = karakuri_ir::check::check(&proc)
                .unwrap_or_else(|e| panic!("{} does not check: {e:?}", path.display()));
            if checked.kind == Kind::L4 {
                found.insert(checked.name.clone(), checked);
            }
        }
        assert!(!found.is_empty(), "no renderers found in examples/");
        found
    }

    /// **The state as it stands, for a Set nobody has touched.** Every param at
    /// its declared default, which is what `Set::params` is holding the moment
    /// `Set::build` returns — ADR-0282's *a declared value is the value in the
    /// untouched state*. A vector param has no scalar default and is left to
    /// its declared range, which `point_rate_bound_at` documents.
    fn as_it_stands(proc: &Checked) -> HashMap<String, f32> {
        proc.params
            .iter()
            .filter_map(|p| p.default_scalar().map(|v| (p.name.clone(), v)))
            .collect()
    }

    /// **Every shipped renderer, both readings, written out.**
    ///
    /// Under ADR-0285 the last two columns would have read `Refused` for
    /// twelve of these fifteen — eight `NoRoomBelowTheTarget` and four
    /// `FloorUnknown` — and `Clear` for the three that draw no primitive.
    #[test]
    fn every_shipped_renderer_is_placed_under_both_readings() {
        // (procedure, floor over the declared range, floor over the state as
        //  it stands). `u32::MAX` is *not known*.
        const CORPUS: &[(&str, u32, u32)] = &[
            // No `vertex` block, so no primitive and no floor.
            ("field_lens", 1, 1),
            ("field_march", 1, 1),
            ("glow_march", 1, 1),
            // `point_scale`, declared from 0.0014 and held at 0.00417.
            ("plain_points", 715, 240),
            // The same, times `max(size, 1.0)`; held at 0.0076.
            ("star_flares", 715, 132),
            // The constant low end of the procedure's own `clamp`, which no
            // param moves.
            ("sheet_shade", 720, 720),
            // `width`, declared from 0.00069 and held at 0.00139.
            ("speed_lines", 1450, 720),
            // `point_scale`'s minimum times the 0.35 an element at rest gets.
            ("soft_points", 4141, 514),
            ("second_eye", 4141, 514),
            ("glass_shell", 4141, 343),
            // The same shape on `width`.
            ("drift_streaks", 4141, 1711),
            // `width_var` is declared up to 1.0, where the rate reaches zero;
            // held at 0.45 it does not, which is the one procedure the
            // narrower reading rescues from *not known*.
            ("strand_strokes", u32::MAX, 164),
            // An attribute with no declaration to read, either way.
            ("hard_dots", u32::MAX, u32::MAX),
            ("beat_strokes", u32::MAX, u32::MAX),
            // A rate that genuinely reaches zero, either way.
            ("beat_bloom", u32::MAX, u32::MAX),
        ];

        let renderers = renderers();
        assert_eq!(
            renderers.len(),
            CORPUS.len(),
            "examples/ ships {} L4 procedures and this table carries {}",
            renderers.len(),
            CORPUS.len()
        );

        for (name, declared, held) in CORPUS {
            let checked = renderers
                .get(*name)
                .unwrap_or_else(|| panic!("examples/ no longer ships `{name}`"));
            assert_eq!(
                floor_of(&point_rate_bound(checked).bound),
                *declared,
                "`{name}`'s floor over the declared range moved"
            );
            assert_eq!(
                floor_of(&point_rate_bound_at(checked, &as_it_stands(checked)).bound),
                *held,
                "`{name}`'s floor over the state as it stands moved"
            );
            for (reading, floor) in [("declared", declared), ("as it stands", held)] {
                match placed(*floor) {
                    Placed::Refused => {
                        panic!("`{name}` is refused at the reference target, reading {reading}")
                    }
                    Placed::Floored(share) => assert!(
                        share <= FLOORED_SHARE_ALLOWED,
                        "`{name}` hides {share} reading {reading}, over the allowance"
                    ),
                    Placed::Clear => {}
                }
            }
        }
    }

    /// **The coverage figure, stated as a figure**, because *how much does it
    /// answer for* is the question ADR-0293 was written to move and a reader
    /// should not have to count the table above.
    ///
    /// - **Before, under ADR-0285:** 3 answered and 12 refused, whichever way
    ///   the bound was read. Every per-element procedure in `examples/` was
    ///   `NoRoomBelowTheTarget` or `FloorUnknown`.
    /// - **After, over the declared range:** 15 answered — 3 clear, 12 floored
    ///   at 0.2492 and corrected by 1.3319.
    /// - **After, over the state as it stands:** 15 answered — 7 clear, 8
    ///   floored, of which `soft_points` and `second_eye` hide only 0.1618 and
    ///   are corrected by 1.1930, their 514-row floor sitting between the two
    ///   rungs where the worst case cannot be reached.
    ///
    /// So the narrower reading is worth **four procedures' worth of
    /// exactness** and a third of the probe's fragment work on each of them.
    /// It is no longer worth an *answer*, which is the difference ADR-0293
    /// makes to ADR-0285's *three of the eight would place rungs at 720 rows
    /// instead of none*.
    #[test]
    fn the_loosened_floor_answers_for_all_fifteen_either_way() {
        let renderers = renderers();
        let mut declared = (0, 0, 0);
        let mut stands = (0, 0, 0);
        for checked in renderers.values() {
            let held = as_it_stands(checked);
            for (tally, floor) in [
                (&mut declared, floor_of(&point_rate_bound(checked).bound)),
                (
                    &mut stands,
                    floor_of(&point_rate_bound_at(checked, &held).bound),
                ),
            ] {
                match placed(floor) {
                    Placed::Clear => tally.0 += 1,
                    Placed::Floored(_) => tally.1 += 1,
                    Placed::Refused => tally.2 += 1,
                }
            }
        }
        assert_eq!(renderers.len(), 15, "examples/ no longer ships fifteen L4s");
        assert_eq!(
            declared,
            (3, 12, 0),
            "clear/floored/refused over the declared range"
        );
        assert_eq!(
            stands,
            (7, 8, 0),
            "clear/floored/refused over the state as it stands"
        );
    }

    /// **What the correction costs the two procedures a narrower bound helps
    /// most**, spelled out rather than left in the tally above. A floor
    /// between the rungs cannot reach the peak of `u(x)/x²`, so
    /// [`floored_share`] answers less than the worst case — which is what
    /// makes the bound tight rather than merely sound.
    #[test]
    fn a_floor_between_the_rungs_is_corrected_by_less_than_the_worst_case() {
        let worst = match placed(u32::MAX) {
            Placed::Floored(share) => share,
            other => panic!("an unknown floor is floored, not {other:?}"),
        };
        // `soft_points` and `second_eye` over the state as it stands.
        let between = match placed(514) {
            Placed::Floored(share) => share,
            other => panic!("a 514-row floor is floored, not {other:?}"),
        };
        assert!(
            (worst - 0.2492).abs() < 1e-3 && (between - 0.1618).abs() < 1e-3,
            "the two shares are {worst} and {between}"
        );
        assert!(
            1.0 / (1.0 - between) < 1.20 && 1.0 / (1.0 - worst) > 1.33,
            "the corrections are {} and {}",
            1.0 / (1.0 - between),
            1.0 / (1.0 - worst)
        );
    }
}
