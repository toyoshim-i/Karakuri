//! The small draw, end to end on a device: that it is taken at
//! [`PREPARATION_RESOLUTION`], that it leaves the Set as it found it, and that
//! the number it hands out is the measurement times the ratio of the areas and
//! nothing else.
//!
//! **What is asserted here and what is not.** The arithmetic of the rule is
//! unit-tested in `src/estimate.rs` against figures the ladder actually
//! produced, and needs no adapter. What needs one is everything a caller would
//! discover the hard way: a probe left pointing at the wrong size, a Set left
//! at 640x360 so its first on-air frame draws the wrong aspect ratio, a Set
//! left stepped so the slot arrives warm when the design says it arrives cold.
//! Each of those is silent, and each is one line in `estimate`.
//!
//! **No timing is asserted.** `docs/contributing.md` §1 is why: this machine's
//! GPU timestamps demote to a host clock, the figure is biased high by
//! submission and synchronization, and a threshold on it would be a test that
//! passes for the wrong reason on a faster machine. The measurements this
//! module's rule was chosen against are in `examples/small_draw.rs` and quoted
//! in the module doc with the instrument that took them.

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
mod gpu {
    use karakuri_engine::estimate::{estimate, PREPARATION_RESOLUTION};
    use karakuri_engine::set::{Edge, Layering, Wiring};
    use karakuri_engine::swap::PROBE_RESOLUTION;
    use karakuri_engine::{Gpu, Probe, Set};
    use karakuri_ir::typed::Checked;
    use karakuri_ir::Topology;

    const CAPACITY: u32 = 4_096;
    const SEED: u32 = 19_274;
    /// A caller's own viewport, deliberately neither the probe's nor the
    /// preparation size, so "put it back" is distinguishable from "leave it".
    const CALLER_VIEWPORT: (u32, u32) = (800, 600);

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
        let mut set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&l1, CAPACITY)],
            &[],
            &[],
            &[],
            &[&l4],
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

    /// **The rule, on a real measurement.** The number handed out is the one
    /// the probe took, times the ratio of the target's area to the probe's, and
    /// the measurement it was derived from travels with it — instrument,
    /// capacity and size — so a consumer can check what it was given.
    #[test]
    fn a_small_draw_is_scaled_to_the_target_and_carries_what_took_it() {
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

        eprintln!("estimated via {:?}: {:.3} ms", e.measured.method, e.ms);
        assert_eq!(
            e.measured.resolution, PREPARATION_RESOLUTION,
            "the draw has to be the small one, or there is no estimate here at all"
        );
        assert_eq!(e.measured.capacity, CAPACITY);
        assert_eq!(e.target, PROBE_RESOLUTION);
        assert_eq!(e.topologies, vec![Topology::Points]);
        assert_eq!(e.area_ratio, 4.0);
        assert!(
            (e.ms - e.measured.ms * 4.0).abs() < 1e-3,
            "{} is not {} times four",
            e.ms,
            e.measured.ms
        );
    }

    /// **The Set is left exactly as it was found**, which is the whole of what
    /// makes this safe to call on a slot that is about to go on air: the
    /// caller's viewport back (the camera derives its aspect ratio from it, so
    /// a Set left at 640x360 draws a different picture on its first frame), and
    /// `t` back to zero (the design says a primed Set arrives cold, and
    /// `Set::rewind` is what puts it there).
    #[test]
    fn the_set_is_left_at_its_own_size_and_cold() {
        let gpu = gpu();
        let mut probe = Probe::new(&gpu.device, &gpu.queue, gpu.timestamps, PROBE_RESOLUTION);
        let mut set = points_set(&gpu);

        estimate(
            &mut probe,
            &gpu.device,
            &gpu.queue,
            &mut set,
            PROBE_RESOLUTION,
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
    /// probe can answer on a different scale; `estimate` therefore moves the
    /// one it is given and leaves it at the small size, which is what makes a
    /// deck's worth of slots cost one reallocation rather than one each.
    #[test]
    fn the_probe_is_moved_to_the_small_size_and_left_there() {
        let gpu = gpu();
        let mut probe = Probe::new(&gpu.device, &gpu.queue, gpu.timestamps, PROBE_RESOLUTION);
        assert_eq!(probe.resolution(), PROBE_RESOLUTION);
        let mut set = points_set(&gpu);

        let first = estimate(
            &mut probe,
            &gpu.device,
            &gpu.queue,
            &mut set,
            PROBE_RESOLUTION,
        );
        assert_eq!(probe.resolution(), PREPARATION_RESOLUTION);

        // And a second call finds it already there and still answers the same
        // way, which is the property that makes the resize an optimisation
        // rather than a state a caller has to track.
        let second = estimate(
            &mut probe,
            &gpu.device,
            &gpu.queue,
            &mut set,
            PROBE_RESOLUTION,
        );
        assert_eq!(second.measured.resolution, PREPARATION_RESOLUTION);
        assert_eq!(second.measured.method, first.measured.method);
    }

    /// A fullscreen procedure is labelled as one. The rule does not branch on
    /// it — the module doc says why — but the overshoot the module doc reports
    /// is per topology, so the topology has to survive the trip.
    #[test]
    fn a_fullscreen_renderer_is_recorded_as_fullscreen() {
        let gpu = gpu();
        let mut probe = Probe::new(&gpu.device, &gpu.queue, gpu.timestamps, PROBE_RESOLUTION);
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
        let mut set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&l1, CAPACITY)],
            &[],
            &[],
            &[],
            &[&l4],
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

        let e = estimate(
            &mut probe,
            &gpu.device,
            &gpu.queue,
            &mut set,
            PROBE_RESOLUTION,
        );
        assert_eq!(e.topologies, vec![Topology::Fullscreen]);
    }
}
