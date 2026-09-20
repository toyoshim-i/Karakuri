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

    /// The claim, operationally: **a build does not block the render loop, and its
    /// result appears between two frames rather than inside one.**
    ///
    /// "Does not block" is not directly observable — there is no such thing as
    /// asking a loop whether it was blocked. What is observable is that frames
    /// continued to be produced between the request going out and the swap coming
    /// back, which is the same statement from the outside. A `recv` instead of a
    /// `try_recv` on the frame path would make that count zero.
    ///
    /// "Not inside a frame" is checked by observing the live Set's identity at the
    /// top and at the bottom of every frame body. Note what that does *not* check:
    /// the borrow `begin_frame` returns stops a second call while it is held, but
    /// the encoder is the caller's and borrows nothing, so a caller that calls
    /// `begin_frame` twice inside one encoder still gets two Sets in one frame.
    /// This asserts the property for a frame loop shaped like the CLI's, which is
    /// the convention both callers keep — see the module doc on `swap.rs`.
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

        // No state transfer, by design: a new procedure means new buffers, so the
        // incoming Set starts cold. Priming a Set out of sight before showing it
        // is the deck's Priming, and a partial version of it here would be a
        // second answer to remove.
        //
        // One step, not zero: the frame the swap landed on is a rendered frame
        // like any other, and it stepped the Set that was live at its top — which
        // by then was the new one. The claim is that it started from zero, and one
        // step after ten frames of the old Set is that claim.
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

    /// A swapped-in Set carries the bindings the request stated.
    ///
    /// A binding is Set state and a swap builds a whole new Set, so it is carried
    /// by being restated — and losing it is silent: `--watch` would keep working,
    /// the picture would keep updating, and the only symptom would be a parameter
    /// that quietly stopped moving after the first save.
    ///
    /// **The params were the example this pointed at and are no longer.** A
    /// parameter value is the one thing the outgoing Set can hand over itself,
    /// and a rebuild inherits the ones somebody moved rather than restating them
    /// — see the tests at the bottom of this file.
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

    /// **A swapped-in Set is under the authority the request states**, and not
    /// back at the default because a `.kir` was saved.
    ///
    /// This is the field
    /// `docs/adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md`
    /// says the engine owes, and its consequences section names the failure it
    /// prevents: *"a rebuild that let a node's authority be re-derived would
    /// hand a node back to an agent an operator had taken it from, on the next
    /// save of any `.kir`, saying nothing."*
    ///
    /// **Both directions, because the loss is silent in both and only one of
    /// them is the sentence above.** A rebuild that drops the restatement puts
    /// every node back at `Authority::Manual`, so what it actually destroys is
    /// a grant — the operator who let an agent at `L1:0` finds it theirs again
    /// and nothing said so. The other half is the one the ADR names, and it is
    /// asserted here as *the default is not inherited*: `L4:0` is granted on
    /// one build, stated by nobody on the next, and must come up manual rather
    /// than carrying the previous Set's grant forward. A rebuild is not a
    /// surface, so no surface could have refused either
    /// (`docs/principles/0078-…`).
    ///
    /// Two nodes in two different layers, so that the address is exercised past
    /// `L1:0` and so that one rebuild can ask both questions at once: `L1:0`
    /// restated and `L4:0` not.
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

    /// **A swapped-in Set is aimed where the request says**, and not at
    /// `Orbit::default()`.
    ///
    /// The camera is Set state the way the bindings are, and it was the one piece
    /// of it a request did not carry. `Set::build_many` starts every
    /// Set it builds from the default orbit, so a swap silently re-aimed the slot —
    /// and it stayed silent, because the picture still moved and nothing was
    /// refused. Downstream of that, a caller that *records* `Set::camera` — which
    /// is what `karakuri-cli`'s live save does — writes the defaults into a file
    /// the operator asked to keep, so the loss outlives the run.
    ///
    /// Asserted on all six numbers rather than on the two a `camera` record spells:
    /// a request that carried half of them would be as wrong as one that carried
    /// none, and only quieter.
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

    /// A build that fails leaves the running Set **completely** untouched: not
    /// merely still rendering, but at the same `t`, with the same live count, and
    /// the same buffers. The composition check is the failure used here because it
    /// happens inside `Set::build`, on the worker thread, which is the case a
    /// render thread could plausibly mishandle — it is holding a `Result` and has
    /// to put the `Err` down without disturbing anything.
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

    /// **A build arrives measured, and still arrives cold.**
    ///
    /// The governor budgets against a per-Set measurement taken on the worker as
    /// part of building — see "The worker also measures what it built" in
    /// `swap.rs`. Two things have to hold together and each is easy to have
    /// without the other:
    ///
    /// - the measurement exists and travels with the Set, so a slot the operator
    ///   might want to prime is budgetable at all;
    /// - and measuring it left **no trace**. Measuring means stepping, and a
    ///   swapped-in Set is documented as arriving cold with `t` at zero. A probe
    ///   run that forgot to rewind would hand over a Set sixteen steps into its own
    ///   simulation, which nothing downstream would ever notice — the picture would
    ///   simply be slightly wrong on the first frame after every swap, forever.
    ///
    /// The cold half is asserted in step counts rather than in pixels for the
    /// reason `steps_taken` gives.
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
        // **The size the slot was told to measure at**, which is the deck's
        // own until somebody narrows it (ADR-0303) — and a `HotSwap` built
        // outside a deck is at its own viewport.
        assert_eq!(cost.resolution, h.swap.measure_size());
    }

    /// The other half of "a failed compile changes nothing", and the half that is
    /// enforced by shape rather than by handling: a `.kir` that does not compile
    /// never becomes a `Request`, so there is nothing for the render thread to
    /// reject. A [`Source`] that produces nothing is exactly what
    /// `karakuri-environment`'s watcher becomes on a parse error, and the running Set must
    /// not notice.
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
