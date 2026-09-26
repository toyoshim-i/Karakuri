use super::common::*;

mod gpu {
    use super::*;

    #[allow(dead_code)]
    fn channel_driven() {
        let _ = Gpu::headless();
    }

    #[allow(dead_code)]
    fn channel_driven_at() {
        let _ = Gpu::headless();
    }

    #[allow(dead_code)]
    fn new() {
        let _ = Gpu::headless();
    }

    /// Verifies that hot-swap builds compile in the background without blocking the render loop
    /// and install atomically at frame boundaries.
    #[test]
    fn a_build_runs_in_the_background_and_lands_between_two_frames() {
        let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);

        for _ in 0..10 {
            h.frame();
        }
        assert_eq!(h.swap.set().capacity(), FIRST);
        let t_before = h.swap.set().time();
        assert!(t_before > 0.0, "the first Set never stepped");

        tx.send(request(L4, SECOND, "second"))
            .expect("worker alive");
        let (in_flight, _) = h.frames_until(is_swapped, "the swap");

        assert!(
            in_flight > 1,
            "only {in_flight} frame(s) were produced between the request and the swap — \
         the render loop waited for the build instead of polling for it"
        );
        assert_eq!(
            h.swap.set().capacity(),
            SECOND,
            "the swap reported success but the live Set is still the old one"
        );

        // Verifies the swapped-in set starts cold with step count 1 on its first frame.
        assert_eq!(
            steps_taken(h.swap.set()),
            1,
            "the swapped-in Set inherited a `t`; a swap lands cold and Priming is the warming"
        );

        for (i, (top, bottom)) in h.frame_capacities.iter().enumerate() {
            assert_eq!(
                top, bottom,
                "frame {i} began with a Set of capacity {top} and ended with one of {bottom}: \
             a swap landed in the middle of a frame"
            );
        }
        // And exactly one changeover happened across the whole run, at a frame
        // boundary — not one Set for the compute pass and another for the draw.
        let changes = h
            .frame_capacities
            .windows(2)
            .filter(|w| w[0].0 != w[1].0)
            .count();
        assert_eq!(changes, 1, "expected exactly one swap, saw {changes}");
    }

    /// Verifies that a swapped-in Set preserves external bindings specified in the request.
    #[test]
    fn a_swapped_in_set_carries_the_bindings_the_request_stated() {
        let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);

        for _ in 0..3 {
            h.frame();
        }
        assert!(h.swap.set().bindings().is_empty());

        let mut req = request(L4, SECOND, "bound");
        // The param moved by hand *and* bound, since the two travel together and
        // the binding has to be applied after the override to blend from it.
        req.params = vec![karakuri_engine::ParamWrite::everywhere("radius", 4.0)];
        req.bindings = vec![Binding::new(
            karakuri_ir::Kind::L1,
            "radius",
            "beat",
            Curve::Pow2,
            [1.0, 5.0],
        )];
        tx.send(req).expect("worker alive");
        h.frames_until(is_swapped, "the swap");

        let set = h.swap.set();
        assert_eq!(set.capacity(), SECOND, "the swap did not land");
        assert_eq!(
            set.param("radius").expect("declared"),
            4.0,
            "the override did not survive"
        );
        assert_eq!(
            set.bindings().len(),
            1,
            "the swapped-in Set lost the binding the request stated"
        );
        assert_eq!(set.bindings()[0].signal, "beat");
    }

    /// Verifies that a swapped-in Set inherits authority specified in the request (ADR-0211).
    #[test]
    fn a_swapped_in_set_carries_the_authority_the_request_stated() {
        use karakuri_engine::swap::AuthorityAt;
        use karakuri_engine::Authority;
        use karakuri_ir::Kind;

        let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);

        for _ in 0..3 {
            h.frame();
        }
        assert_eq!(
            h.swap.set().authority(Kind::L1, 0),
            Some(Authority::Manual),
            "a Set nobody has spoken for should come up the operator's"
        );

        let mut req = request(L4, SECOND, "granted");
        req.authorities = vec![
            AuthorityAt::new(Kind::L1, 0, Authority::Automatic),
            AuthorityAt::new(Kind::L4, 0, Authority::Suggesting),
        ];
        tx.send(req).expect("worker alive");
        h.frames_until(is_swapped, "the swap");

        let set = h.swap.set();
        assert_eq!(set.capacity(), SECOND, "the swap did not land");
        assert_eq!(
            set.authority(Kind::L1, 0),
            Some(Authority::Automatic),
            "the swapped-in Set lost the authority the request stated for L1:0 — \
             a grant an operator made is gone and nothing said so"
        );
        assert_eq!(
            set.authority(Kind::L4, 0),
            Some(Authority::Suggesting),
            "the swapped-in Set lost the authority the request stated for L4:0"
        );

        // And the other half: a node this request says nothing about lands on
        // the default rather than on whatever the previous build had. `L1:0` is
        // restated and `L4:0` is not, so one rebuild asks both questions.
        let mut req = request(L4, FIRST, "taken back");
        req.authorities = vec![AuthorityAt::new(Kind::L1, 0, Authority::Automatic)];
        tx.send(req).expect("worker alive");
        h.frames_until(is_swapped, "the second swap");

        let set = h.swap.set();
        assert_eq!(set.capacity(), FIRST, "the second swap did not land");
        assert_eq!(
            set.authority(Kind::L1, 0),
            Some(Authority::Automatic),
            "the restatement stopped working on the second rebuild"
        );
        assert_eq!(
            set.authority(Kind::L4, 0),
            Some(Authority::Manual),
            "a node nobody spoke for on this build kept the previous build's \
             authority — an agent was handed a node the request did not give it"
        );
    }

    /// Verifies that a swapped-in Set preserves camera orbit orientation stated in the request.
    #[test]
    fn a_swapped_in_set_is_aimed_where_the_request_states() {
        let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);

        for _ in 0..3 {
            h.frame();
        }
        // Six numbers no default produces, so a Set aimed by `Orbit::default()`
        // cannot pass this by accident.
        let aimed = karakuri_engine::camera::Orbit {
            radius: 3.25,
            speed: 0.75,
            height: -1.5,
            fov_y: 0.9,
            near: 0.25,
            far: 250.0,
        };
        let six = |o: &karakuri_engine::camera::Orbit| {
            (o.radius, o.speed, o.height, o.fov_y, o.near, o.far)
        };
        assert_ne!(
            six(&h.swap.set().orbit()),
            six(&aimed),
            "the live Set was already aimed there, so this asserts nothing"
        );

        tx.send(Request {
            camera: aimed,
            ..request(L4, SECOND, "aimed")
        })
        .expect("worker alive");
        h.frames_until(is_swapped, "the swap");

        let set = h.swap.set();
        assert_eq!(set.capacity(), SECOND, "the swap did not land");
        assert_eq!(
            six(&set.orbit()),
            six(&aimed),
            "the swapped-in Set was aimed somewhere the request did not ask for"
        );
    }

    /// Verifies that worker build failures leave running Set state, buffers, and clock unaffected.
    #[test]
    fn a_build_that_fails_changes_nothing() {
        let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);

        for _ in 0..10 {
            h.frame();
        }
        let live_before = h.swap.set().live_count(&h.gpu.device, &h.gpu.queue);
        let frames_before = h.swap.frames_rendered();

        tx.send(request(L4_INCOMPATIBLE, SECOND, "wants_normal"))
            .expect("worker alive");
        let (elapsed, seen) =
            h.frames_until(|e| matches!(e, Event::Rejected { .. }), "the rejection");

        assert_eq!(
            h.swap.set().capacity(),
            FIRST,
            "a failed build replaced the live Set"
        );
        assert_eq!(
            h.swap.set().live_count(&h.gpu.device, &h.gpu.queue),
            live_before,
            "a failed build disturbed the live Set's element buffers"
        );
        // `t` advanced by exactly the frames that were rendered, one step each:
        // the running Set was neither reset nor paused while the build failed.
        assert_eq!(
            steps_taken(h.swap.set()),
            frames_before + elapsed,
            "the running Set's clock did not advance normally through the rejection"
        );

        let rejection = seen
            .iter()
            .find(|s| s.contains("wants_normal"))
            .unwrap_or_else(|| panic!("no diagnostic naming the candidate: {seen:?}"));
        assert!(
            rejection.contains("normal"),
            "the rejection does not say what was wrong: {rejection}"
        );
    }

    /// Verifies that swapped-in Sets carry worker performance measurements while arriving unstepped (rewound).
    #[test]
    fn a_build_arrives_measured_and_still_arrives_cold() {
        let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);

        for _ in 0..10 {
            h.frame();
        }
        assert!(
            h.swap.measured_cost().is_none(),
            "the Set the harness was constructed with was never built by a worker, so \
         nothing can have measured it"
        );

        tx.send(request(L4, SECOND, "measured"))
            .expect("worker alive");
        h.frames_until(is_swapped, "the swap");

        assert_eq!(h.swap.set().capacity(), SECOND);
        assert_eq!(
            steps_taken(h.swap.set()),
            1,
            "the swapped-in Set arrived having already been stepped; the probe run \
         that measured it was not rewound"
        );

        let cost = h
            .swap
            .measured_cost()
            .expect("a build that succeeded was not measured");
        assert!(
            cost.ms > 0.0 && cost.ms.is_finite(),
            "a measurement of {} ms is not a measurement",
            cost.ms
        );
        assert_eq!(
            cost.capacity, SECOND,
            "the measurement is labelled with a capacity the Set was not built at"
        );
        // Measurement size defaults to deck resolution unless constrained (ADR-0303).
        // outside a deck is at its own viewport.
        assert_eq!(cost.resolution, h.swap.measure_size());
    }

    /// Verifies that idle/unproductive sources leave the running Set executing normally.
    #[test]
    fn a_source_that_produces_nothing_leaves_the_running_set_running() {
        let mut h = Harness::new(GENEROUS_MS, FIRST, (WIDTH, HEIGHT), Box::new(Silent));

        // Long enough that the worker has polled its source many times over.
        for _ in 0..60 {
            h.frame();
        }

        let events: Vec<String> = h.swap.events().map(|e| e.to_string()).collect();
        assert!(events.is_empty(), "something happened: {events:?}");
        assert_eq!(h.swap.set().capacity(), FIRST);
        assert_eq!(steps_taken(h.swap.set()), 60);
    }
}
