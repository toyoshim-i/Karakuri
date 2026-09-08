//! The third clause of the V1 assumption: **can we hot-swap it without
//! dropping a frame?**
//!
//! The headline is a timing claim and timing assertions are flaky, so it is
//! split. What is *asserted* here is structural and deterministic: that a swap
//! lands on a frame boundary and not inside one, that frames keep being
//! produced while a build is in flight, that a build which fails changes
//! nothing at all, and that a rollback restores the previous Set rather than
//! merely stopping the new one. What is *measured* is printed rather than
//! asserted — see `frame_times_across_a_swap_are_measured_and_reported` at the
//! bottom, and the numbers it prints.
//!
//! ## The harness waits for the GPU each frame, and the real one does not
//!
//! `Harness::frame` submits and then calls `device.poll(PollType::wait_indefinitely())`. That
//! is the submit-and-wait pattern the render thread must never use; it is here
//! for two reasons. It bounds a headless loop that would otherwise queue
//! thousands of command buffers ahead of the GPU, standing in for the vsync
//! that bounds a real window. And it makes the interval `HotSwap` measures
//! include the GPU actually finishing, rather than only the cost of recording
//! and handing over the work — which is what makes the printed numbers mean
//! something. `probe.rs` documents the same trade for the same reason: a host
//! clock around submit-and-wait is coarse and biased high, and it is a real
//! number.

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
mod gpu {
    use std::sync::mpsc::{self, Sender};
    use std::time::{Duration, Instant};

    use karakuri_engine::swap::{Event, HotSwap, Request, Source};
    use karakuri_engine::{Binding, Curve, Gpu, Present, Set, Signals, VideoSource};
    use karakuri_ir::typed::Checked;

    /// Deliberately small for the structural tests: what they check does not
    /// depend on the workload, and a dozen of them at a realistic size would make
    /// `cargo test` a coffee break. The measurement at the bottom of this file
    /// uses [`REAL`] instead, because its numbers do depend on it.
    const WIDTH: u32 = 256;
    const HEIGHT: u32 = 256;

    /// The two capacities the structural tests build at. They differ so that which
    /// Set is live is observable from outside: `capacity` is a Set-level dial, so
    /// the same pair of procedures at two capacities is two Sets and one artifact.
    const FIRST: u32 = 4096;
    const SECOND: u32 = 8192;

    /// The workload the reported numbers are taken at — the CLI's own defaults,
    /// so that they are comparable with the other host-clock figures in this
    /// repository rather than being a measurement of a toy. That one workload
    /// is the rule and not a coincidence: `docs/contributing.md` §1.
    const REAL: (u32, (u32, u32)) = (262_144, (1280, 720));

    /// A budget no frame in this harness will come near, for the tests that want a
    /// candidate accepted rather than rolled back.
    const GENEROUS_MS: f32 = 10_000.0;

    /// How long a test will spin waiting for the worker before giving up. Generous:
    /// it covers WGSL generation, two `create_shader_module` calls, pipeline
    /// creation, and a whole-capacity buffer upload, on whatever machine CI turns
    /// out to be.
    const PATIENCE: Duration = Duration::from_secs(30);

    const L1: &str = r#"
proc static_shell {
  kind     L1
  topology points
  capacity [1024, 262144] = 4096

  param radius : float [0.1, 8.0] = 2.5

  emit position, age

  element {
    let u = hash1(seed);
    let v = hash1(seed + 1000u);
    position = sphere_point(u, v) * radius;
    age      = age + dt;
  }
}
"#;

    const L4: &str = r#"
proc soft_points {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 1.0

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.015625;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    color = vec4(vec3(1.0, 1.0, 1.0) * exposure, max(0.0, 1.0 - d));
  }
}
"#;

    /// An L4 consuming an attribute `L1` does not emit **and nothing can
    /// synthesise**. `Set::build` refuses this pair — stage 6, the composition
    /// check — which is the cheapest way to get a build that fails *on the worker
    /// thread*, as opposed to one that fails earlier and never becomes a `Request`
    /// at all.
    ///
    /// **It used to want `velocity`, and that stopped working.** `velocity` and
    /// `age` have derivation rules now, so a pair missing either of them composes.
    /// Nothing about the tests below changed in intent; what changed is that their
    /// "cheapest failure" was quietly no longer a failure, and every one of them
    /// would have hung waiting for a rejection that was never coming. `normal` has
    /// no rule and is not going to acquire one — it is a property of a surface, and
    /// there is no surface to take it from.
    const L4_INCOMPATIBLE: &str = r#"
proc wants_normal {
  kind  L4
  blend additive

  consumes position, normal

  vertex {
    clip       = camera * vec4(position + normal, 1.0);
    point_rate = 0.015625;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;

    fn compile(src: &str) -> Checked {
        let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("{}", render(&e, src)));
        let checked =
            karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{}", render(&e, src)));
        karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("{}", render(&e, src)));
        checked
    }

    fn render(errs: &[karakuri_ir::IrError], src: &str) -> String {
        errs.iter()
            .map(|e| e.render(src))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// A second renderer over the same geometry: bigger sprites, its own
    /// `exposure`. Declaring that name twice in one Set is the thing a flat
    /// parameter map could not hold.
    const L4_WIDE: &str = r#"
proc wide_points {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 0.5

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.04296875;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    color = vec4(vec3(0.2, 0.9, 1.0) * exposure, max(0.0, 1.0 - d));
  }
}
"#;

    fn request(l4_src: &str, capacity: u32, label: &str) -> Request {
        request_many(&[l4_src], capacity, label)
    }

    fn request_many(l4_srcs: &[&str], capacity: u32, label: &str) -> Request {
        Request {
            names: karakuri_engine::swap::RequestNames::default(),
            edges: Vec::new(),
            id: 1,
            l1s: vec![(compile(L1), capacity)],
            l2s: Vec::new(),
            l3s: Vec::new(),
            fields: Vec::new(),
            layering: karakuri_engine::set::Layering::Overdraw,
            live: None,
            published: Vec::new(),
            l4s: l4_srcs.iter().map(|s| compile(s)).collect(),
            seed_salt: 19274,
            camera: karakuri_engine::camera::Orbit::default(),
            salts: Vec::new(),
            params: Vec::new(),
            bindings: Vec::new(),
            authorities: Vec::new(),
            label: label.to_string(),
        }
    }

    /// A source that never has anything to build — what a `.kir` file that failed
    /// to compile leaves behind. It still has to sleep, or it spins the worker.
    struct Silent;

    impl Source for Silent {
        fn poll(&mut self) -> Option<Request> {
            std::thread::sleep(karakuri_engine::swap::POLL_INTERVAL);
            None
        }
    }

    struct Harness {
        gpu: Gpu,
        present: Present,
        size: (u32, u32),
        swap: HotSwap,
        /// Every frame interval this harness has measured for itself, in
        /// milliseconds. Independent of the one `HotSwap` keeps, so the printed
        /// numbers are not the watchdog reporting on its own homework.
        intervals: Vec<f32>,
        last: Option<Instant>,
        /// `capacity` observed at the top and at the bottom of each frame body. If
        /// these ever differ, a swap landed in the middle of a frame.
        frame_capacities: Vec<(u32, u32)>,
    }

    impl Harness {
        fn new(
            budget_ms: f32,
            capacity: u32,
            size: (u32, u32),
            source: Box<dyn Source>,
        ) -> Harness {
            let gpu = Gpu::headless().expect("no GPU available");
            let present = Present::new(
                &gpu.device,
                wgpu::TextureFormat::Rgba16Float,
                size.0,
                size.1,
            );
            let set = Harness::build(&gpu, L4, capacity);
            let mut swap = HotSwap::new(&gpu.device, &gpu.queue, set, budget_ms, source);
            swap.resize(&gpu.device, size.0, size.1);
            Harness {
                gpu,
                present,
                size,
                swap,
                intervals: Vec::new(),
                last: None,
                frame_capacities: Vec::new(),
            }
        }

        /// A harness plus the `Sender` that drives it, for the tests that decide
        /// when a rebuild happens rather than watching a file for it.
        fn channel_driven(budget_ms: f32) -> (Harness, Sender<Request>) {
            Harness::channel_driven_at(budget_ms, FIRST, (WIDTH, HEIGHT))
        }

        fn channel_driven_at(
            budget_ms: f32,
            capacity: u32,
            size: (u32, u32),
        ) -> (Harness, Sender<Request>) {
            let (tx, rx) = mpsc::channel();
            (Harness::new(budget_ms, capacity, size, Box::new(rx)), tx)
        }

        fn build(gpu: &Gpu, l4_src: &str, capacity: u32) -> Set {
            Set::build(
                &gpu.device,
                &gpu.queue,
                &compile(L1),
                &compile(l4_src),
                capacity,
                19274,
            )
            .expect("the pair is compatible and the capacity is in range")
        }

        /// One frame, shaped exactly like the CLI's: `begin_frame`, then the whole
        /// frame recorded through the borrow it returns.
        fn frame(&mut self) {
            let now = Instant::now();
            if let Some(last) = self.last.replace(now) {
                self.intervals
                    .push(now.duration_since(last).as_secs_f32() * 1_000.0);
            }

            let hdr = self.present.hdr_view();
            let device = &self.gpu.device;
            let queue = &self.gpu.queue;

            let set = self.swap.begin_frame(device);
            let at_top = set.capacity();
            set.prepare(queue, 1, &Signals::default());
            let mut encoder = device.create_command_encoder(&Default::default());
            set.render(&mut encoder, hdr, 1);
            let at_bottom = set.capacity();
            queue.submit([encoder.finish()]);

            self.frame_capacities.push((at_top, at_bottom));
            // See the module doc: standing in for vsync, and what makes the
            // measured interval include the GPU rather than only the submission.
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("poll");
        }

        /// **Turn a knob on the Set that is playing**, which is what
        /// `Deck::write_param` does for a surface: `begin_frame` is the public
        /// road to a live `&mut Set` and the write is a uniform value, on screen
        /// at the next frame with no build
        /// (`docs/adr/0280-a-parameter-written-to-a-live-set-is-a-session-record.md`).
        ///
        /// It opens a frame it does not render, which is what a caller must not
        /// do in a loop and is exactly right here: the swap it might install is
        /// the thing under test in every caller below, and each of them has
        /// already waited for the one it was expecting.
        fn ride(&mut self, key: &str, value: f32) {
            let device = &self.gpu.device;
            let live = self.swap.begin_frame(device);
            let landed = live
                .write_param(&karakuri_engine::ParamWrite::everywhere(key, value))
                .expect("nothing here grants a node away, so no write crosses an authority");
            assert!(landed > 0, "nothing in the live Set declares `{key}`");
        }

        /// Render frames until `wanted` matches an event, and return how many
        /// frames that took. Every event seen along the way is collected, so a
        /// caller can check that nothing else happened either.
        fn frames_until(
            &mut self,
            wanted: impl Fn(&Event) -> bool,
            what: &str,
        ) -> (u64, Vec<String>) {
            let started = Instant::now();
            let from = self.swap.frames_rendered();
            let mut seen = Vec::new();
            loop {
                self.frame();
                let mut found = false;
                for event in self.swap.events() {
                    found |= wanted(&event);
                    seen.push(event.to_string());
                }
                if found {
                    return (self.swap.frames_rendered() - from, seen);
                }
                assert!(
                    started.elapsed() < PATIENCE,
                    "waited {PATIENCE:?} for {what} and it never happened; saw {seen:?}"
                );
            }
        }
    }

    fn is_swapped(event: &Event) -> bool {
        matches!(event, Event::Swapped { .. })
    }

    /// Simulation steps a Set has taken.
    ///
    /// `Set::time` is the only public witness of it, and it is `steps * dt` at
    /// `dt = 1/60` — a product that does not round to the same `f32` as the same
    /// count divided by sixty, which is exactly the kind of last-bit difference
    /// `Set` derives `t` from an integer counter to avoid in the first place. So
    /// the tests below compare step counts and let this recover the integer,
    /// rather than comparing two float expressions that mean the same thing.
    fn steps_taken(set: &Set) -> u64 {
        (set.time() * 60.0).round() as u64
    }

    // ---------------------------------------------------------------------------
    // Structural, asserted.
    // ---------------------------------------------------------------------------

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
            six(&h.swap.set().camera),
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
            six(&set.camera),
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
        assert!(
            !h.swap.on_trial(),
            "a failed build started a watchdog trial over nothing"
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
        assert_eq!(cost.resolution, karakuri_engine::swap::PROBE_RESOLUTION);
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
        assert!(!h.swap.on_trial());
    }

    /// Rollback, forced with an absurd budget rather than with a slow shader — a
    /// procedure heavy enough to miss the budget on one machine is comfortable on
    /// another, and a test that depends on which is which is not a test.
    ///
    /// The assertion that matters is not that the rollback *fired*; it is that
    /// what came back is the **same Set**, still holding the state it was parked
    /// with. `t` is the sharpest available witness of that: simulation time only
    /// advances through `prepare`, nothing called `prepare` on the outgoing Set
    /// while the candidate was on trial, so a restored Set must resume at exactly
    /// the `t` it stopped at. A Set that had been rebuilt, or reset, or kept
    /// stepping in the background would all show up here.
    #[test]
    fn the_watchdog_rolls_back_and_restores_the_previous_set_where_it_was_parked() {
        // Nothing is faster than zero milliseconds, so every candidate fails.
        let (mut h, tx) = Harness::channel_driven(0.0);

        for _ in 0..10 {
            h.frame();
        }
        tx.send(request(L4, SECOND, "second"))
            .expect("worker alive");

        // Where the outgoing Set was parked: its step count at the top of the
        // frame the swap landed on, which is the last moment anything stepped it.
        let mut parked = None;
        let started = Instant::now();
        while parked.is_none() {
            let before = steps_taken(h.swap.set());
            h.frame();
            for event in h.swap.events() {
                if is_swapped(&event) {
                    parked = Some(before);
                }
            }
            assert!(started.elapsed() < PATIENCE, "the swap never happened");
        }
        let parked = parked.expect("just set");
        assert_eq!(h.swap.set().capacity(), SECOND, "the candidate is live");
        assert!(h.swap.on_trial(), "the candidate is not being watched");

        let (frames, seen) =
            h.frames_until(|e| matches!(e, Event::RolledBack { .. }), "the rollback");

        assert_eq!(
            h.swap.set().capacity(),
            FIRST,
            "the rollback fired but did not restore the previous Set"
        );
        // `parked + 1`, not `parked`: the frame the rollback landed on stepped the
        // restored Set once on its way past, exactly as it would have stepped any
        // other live Set. The claim is that it resumed from where it stopped and
        // not from zero, and not from somewhere it drifted to while parked.
        assert_eq!(
            steps_taken(h.swap.set()),
            parked + 1,
            "the restored Set is not the one that was parked: it was left at {parked} steps \
         and came back at {}",
            steps_taken(h.swap.set())
        );
        assert!(
            !h.swap.on_trial(),
            "the trial did not end when the verdict came in"
        );
        // The verdict waited for a window rather than firing on the first frame —
        // a watchdog that judged frame one would roll back every candidate that
        // ever existed, because a cold Set's first frame pays for its own upload.
        assert!(
            frames > 8,
            "the verdict came after {frames} frames, which is inside the warmup"
        );

        let rollback = seen
            .iter()
            .find(|s| s.contains("rolled back"))
            .unwrap_or_else(|| panic!("no rollback message: {seen:?}"));
        assert!(
            rollback.contains("host clock"),
            "the rollback message does not say what kind of number it decided on: {rollback}"
        );
    }

    /// A candidate that fits is kept, and the old Set is released — the other
    /// branch of the same verdict, and the one that has to work for a hot swap to
    /// be useful rather than merely safe.
    #[test]
    fn a_candidate_that_holds_the_budget_is_kept() {
        let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);

        for _ in 0..5 {
            h.frame();
        }
        tx.send(request(L4, SECOND, "second"))
            .expect("worker alive");
        h.frames_until(is_swapped, "the swap");
        let (_, seen) = h.frames_until(|e| matches!(e, Event::Accepted { .. }), "the verdict");

        assert_eq!(h.swap.set().capacity(), SECOND, "the candidate was kept");
        assert!(!h.swap.on_trial());
        assert!(
            seen.iter().any(|s| s.contains("held the budget")),
            "no acceptance message: {seen:?}"
        );
    }

    /// **A rebuild can change how many renderers a Set has**, not only which ones.
    ///
    /// A `Request` restates the whole stack rather than naming the node that
    /// changed, for the same reason it restates the bindings: a request that
    /// depended on what happens to be live would not be reproducible from a
    /// record stream. So going from one renderer to two is an ordinary
    /// rebuild and needs nothing the swap path did not already have.
    ///
    /// Both renderers declare `exposure`, at different defaults. That pair is
    /// exactly what `SetError::ParamCollision` used to refuse, so this also pins
    /// that a *swapped-in* Set gets the per-node parameter maps and not a flat one.
    #[test]
    fn a_rebuild_can_add_a_renderer_over_the_same_geometry() {
        let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);

        for _ in 0..5 {
            h.frame();
        }
        tx.send(request_many(&[L4, L4_WIDE], SECOND, "two renderers"))
            .expect("worker alive");
        h.frames_until(is_swapped, "the swap");
        h.frames_until(|e| matches!(e, Event::Accepted { .. }), "the verdict");

        let set = h.swap.set();
        assert_eq!(set.capacity(), SECOND, "the candidate was not kept");
        let mut exposures: Vec<f32> = set
            .params()
            .filter(|(_, _, name, _)| *name == "exposure")
            .map(|(_, _, _, value)| value)
            .collect();
        exposures.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
        assert_eq!(
            exposures,
            vec![0.5, 1.0],
            "the swapped-in Set does not hold both renderers' `exposure`"
        );
    }

    /// Two saves during one judging window leave two finished builds behind it, and
    /// the channel is FIFO. Installing the front of that queue would put a
    /// superseded Set on screen for a whole window — thirty-eight frames of a `.kir`
    /// the operator has already replaced — before reaching the current one. The
    /// verdict frame has to drain to the newest.
    ///
    /// No frames are rendered while `b` and `c` build, so nothing can be installed
    /// and the trial over `a` cannot end: both results are guaranteed to be waiting
    /// when the window finally closes.
    #[test]
    fn the_build_installed_after_a_verdict_is_the_newest_one() {
        const THIRD: u32 = 12_288;
        const FOURTH: u32 = 16_384;

        let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);
        for _ in 0..5 {
            h.frame();
        }
        tx.send(request(L4, SECOND, "a")).expect("worker alive");
        h.frames_until(is_swapped, "the first swap");
        assert!(h.swap.on_trial(), "`a` is not being watched");

        tx.send(request(L4, THIRD, "b")).expect("worker alive");
        tx.send(request(L4, FOURTH, "c")).expect("worker alive");
        let waited = Instant::now();
        while waited.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(50));
        }

        let seen = h.frames_until(is_swapped, "the swap after the verdict").1;
        assert_eq!(
            h.swap.set().capacity(),
            FOURTH,
            "a superseded build was installed after the verdict; events: {seen:?}"
        );
        assert!(
            !seen.iter().any(|s| s.contains("`b`")),
            "`b` was superseded before it was ever live and should not have been shown: {seen:?}"
        );
    }

    /// The worker can only leave its loop by panicking, and when it does its end of
    /// the channel closes. `Disconnected` and `Empty` are otherwise the same thing
    /// to the render thread, so without a distinction a `--watch` session would go
    /// on rendering and silently ignore every save for the rest of the run. It is
    /// reported once and the live Set is untouched.
    #[test]
    fn a_worker_that_dies_is_reported_once_and_does_not_disturb_the_live_set() {
        struct Exploding(u32);
        impl Source for Exploding {
            fn poll(&mut self) -> Option<Request> {
                self.0 += 1;
                if self.0 > 2 {
                    panic!("deliberate: a build worker that will not come back");
                }
                std::thread::sleep(karakuri_engine::swap::POLL_INTERVAL);
                None
            }
        }

        let mut h = Harness::new(GENEROUS_MS, FIRST, (WIDTH, HEIGHT), Box::new(Exploding(0)));
        let started = Instant::now();
        let mut lost = 0;
        while started.elapsed() < Duration::from_secs(5) {
            h.frame();
            lost += h
                .swap
                .events()
                .filter(|e| matches!(e, Event::WorkerLost))
                .count();
            if lost > 0 && started.elapsed() > Duration::from_secs(1) {
                break;
            }
        }

        assert_eq!(
            lost, 1,
            "the dead worker was reported {lost} times, not once"
        );
        assert_eq!(h.swap.set().capacity(), FIRST, "the live Set was disturbed");
        assert!(!h.swap.on_trial());
    }

    // ---------------------------------------------------------------------------
    // Measured, reported.
    // ---------------------------------------------------------------------------

    /// Wall-clock frame intervals across a swap: worst case and median, before,
    /// during, and after. **Printed, not asserted** — see the module doc. Run with
    /// `cargo test -p karakuri-engine --test hot_swap -- --nocapture` to see them;
    /// they print labelled as the host-clock figures they are — see
    /// `docs/principles/0095-an-instrument-that-cannot-measure-says-so-rather-than-reporting-a-number.md`.
    #[test]
    fn frame_times_across_a_swap_are_measured_and_reported() {
        let (capacity, size) = REAL;
        // Both Sets at the same capacity, so that "before" and "after" are
        // measurements of the same workload and the only difference between them
        // is that a swap happened in between. Two capacities would confound the
        // question being asked.
        let (mut h, tx) = Harness::channel_driven_at(GENEROUS_MS, capacity, size);

        // Discarded, for the same reason the watchdog discards its own first
        // frames: the process's first frames pay for pipeline first-use, first
        // touch of the element buffers, and whatever the GPU's clocks were doing
        // before there was work. Leaving them in makes the "before" window read
        // slower than the "after" one and invites the conclusion that swapping
        // made things faster.
        for _ in 0..60 {
            h.frame();
        }
        h.intervals.clear();

        for _ in 0..120 {
            h.frame();
        }
        let before = h.intervals.len();

        tx.send(request(L4, capacity, "second"))
            .expect("worker alive");
        let in_flight = h.frames_until(is_swapped, "the swap").0;
        // One more frame before the slice indices are taken. `Harness::frame`
        // pushes an interval at the *top* of a frame, so when the `Swapped` event
        // is first seen the swap frame has begun but not ended and its interval is
        // not in the vector yet — the last entry is the frame before it. Rendering
        // one more frame is what puts the swap frame's own interval at the end.
        h.frame();
        let at_swap = h.intervals.len();

        for _ in 0..120 {
            h.frame();
        }

        let summarize = |label: &str, xs: &[f32]| {
            let mut sorted = xs.to_vec();
            sorted.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
            let median = sorted[sorted.len() / 2];
            let worst = sorted[sorted.len() - 1];
            eprintln!(
                "  {label:<30} n={:<4} median {median:.3} ms   worst {worst:.3} ms",
                sorted.len()
            );
        };

        eprintln!(
            "\nframe intervals at capacity {capacity}, {}x{}, host clock around \
         submit-and-wait; {in_flight} frames rendered between the request and the swap:",
            h.size.0, h.size.1
        );
        summarize("steady, before the request", &h.intervals[..before]);
        summarize(
            "while the build was in flight",
            &h.intervals[before..at_swap - 1],
        );
        // The frame the swap landed on gets its own line: it is the one frame that
        // could plausibly cost something, since it is where the live Set is
        // replaced and where the incoming pipelines are used for the first time.
        summarize("the swap frame itself", &h.intervals[at_swap - 1..at_swap]);
        summarize("steady, after the swap", &h.intervals[at_swap..]);
        eprintln!();
    }

    // ---------------------------------------------------------------------------
    // `Set::rewind`, directly.
    // ---------------------------------------------------------------------------

    /// Accumulating, spawning, and killing at once, so that a rewind has every
    /// kind of state to put back: element buffers that integrate, an `alive`
    /// buffer and a counts buffer the compaction scan rewrites, the spawn
    /// accumulator, and the seed counter.
    const L1_STATEFUL: &str = r#"
proc fountain {
  kind     L1
  topology points
  capacity [1024, 262144] = 4096

  param spawn_rate : float [0.0, 40000.0] = 700.0
  param speed      : float [0.0, 16.0]    = 4.0

  emit position, velocity, age

  spawn {
    position = vec3(0.0, 0.0, 0.0);
    velocity = sphere_point(hash1(seed), hash1(seed + 7u)) * speed;
    age      = 0.0;
  }

  element {
    position = position + velocity * dt;
    velocity = velocity + vec3(0.0, -2.0, 0.0) * dt;
    age      = age + dt;
    if age > 0.35 {
      kill();
    }
  }
}
"#;

    /// Build one Set of [`L1_STATEFUL`] at the harness size.
    fn stateful(gpu: &Gpu) -> Set {
        let mut set = Set::build(
            &gpu.device,
            &gpu.queue,
            &compile(L1_STATEFUL),
            &compile(L4),
            FIRST,
            19274,
        )
        .expect("the pair is compatible and the capacity is in range");
        set.resize(&gpu.device, WIDTH, HEIGHT);
        set
    }

    /// One ordinary frame against a caller-owned target, exactly as the deck
    /// records one.
    fn drive(gpu: &Gpu, set: &mut Set, view: &wgpu::TextureView, steps: u8) {
        set.prepare(&gpu.queue, steps, &Signals::default());
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        set.render(&mut encoder, view, steps);
        gpu.queue.submit([encoder.finish()]);
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
    }

    fn pixels(gpu: &Gpu, texture: &wgpu::Texture) -> Vec<u16> {
        let (width, height) = (texture.width(), texture.height());
        let bytes_per_row = width * 8;
        assert_eq!(bytes_per_row % 256, 0, "row pitch must be 256-byte aligned");
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rewind readback"),
            size: u64::from(bytes_per_row * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        gpu.queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let data = slice.get_mapped_range().expect("map");
        let out = data
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        drop(data);
        buffer.unmap();
        out
    }

    /// **A rewound Set is a fresh Set**, in every way anything downstream can see.
    ///
    /// This is the claim `Set::rewind` has to hold up and the one that is silent
    /// when it does not: a swapped-in Set that kept a trace of its own probe run is
    /// wrong from its first frame and nothing reports it. Asserted by *equivalence*
    /// rather than field by field — one Set is probed and rewound, another never
    /// is, and then both are driven through the same frames and compared on their
    /// element bytes, their live count, their `t` and their pixels. A field
    /// `rewind` forgot shows up in one of those or it was not state.
    #[test]
    fn a_rewound_set_is_indistinguishable_from_one_that_was_never_stepped() {
        use karakuri_engine::probe::Probe;
        use karakuri_engine::swap::{measure, PROBE_RESOLUTION};

        let gpu = Gpu::headless().expect("no GPU available");
        let probed_target = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let fresh_target = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);

        let mut probed = stateful(&gpu);
        let mut fresh = stateful(&gpu);

        // `timestamps: false` takes the host-clock path deliberately: what is
        // being asserted is what the probe run does to the Set, not what it
        // measured, and calibration is half a second of GPU time for a number this
        // test never reads.
        let mut probe = Probe::new(&gpu.device, &gpu.queue, false, PROBE_RESOLUTION);
        let measurement = measure(&mut probe, &gpu.device, &gpu.queue, &mut probed);
        assert!(measurement.ms.is_finite());

        assert_eq!(steps_taken(&probed), 0, "the probe run left `t` advanced");
        assert_eq!(
            probed.viewport(),
            (WIDTH, HEIGHT),
            "the probe run left the Set at its own reference resolution; the camera's \
         aspect ratio comes off this and the next frame would be framed wrong"
        );

        for _ in 0..24 {
            drive(&gpu, &mut probed, probed_target.hdr_view(), 1);
            drive(&gpu, &mut fresh, fresh_target.hdr_view(), 1);
        }

        assert_eq!(steps_taken(&probed), steps_taken(&fresh));
        assert_eq!(
            probed.live_count(&gpu.device, &gpu.queue),
            fresh.live_count(&gpu.device, &gpu.queue),
            "the probed Set holds a different population; the spawn accumulator, the \
         seed counter or the counts buffer survived the rewind"
        );
        assert_eq!(
            probed.read_elements(&gpu.device, &gpu.queue),
            fresh.read_elements(&gpu.device, &gpu.queue),
            "the probed Set's element buffer differs from a fresh one's after the same \
         frames; something the probe run touched was not put back"
        );
        let (a, b) = (
            pixels(&gpu, probed_target.hdr_texture()),
            pixels(&gpu, fresh_target.hdr_texture()),
        );
        assert!(
            a.chunks_exact(4).any(|p| p[0] != 0),
            "neither Set drew anything, so this comparison is two black frames"
        );
        assert_eq!(
            a, b,
            "a probed-and-rewound Set renders differently from a fresh one"
        );
    }

    /// **A swap carries the Set's interface, and a macro survives it.**
    ///
    /// The bindings are restated on every rebuild for the reason `Request::bindings`
    /// gives, and the interface was not — so a `control:` binding survived a swap
    /// and the control it named did not. A source that is gone leaves its param
    /// where it was, silently, for the rest of the run: the picture would simply
    /// stop responding to a knob, with nothing said.
    ///
    /// The order matters as much as the presence. Publishing happens *before* the
    /// bindings are attached, because a binding on a name nothing answers is
    /// refused — which is the diagnostic, and would fire on every rebuild if the
    /// two were the other way round.
    #[test]
    fn a_swap_carries_the_interface_a_macro_is_bound_to() {
        let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);
        for _ in 0..5 {
            h.frame();
        }

        let mut next = request(L4, SECOND, "with_interface");
        next.published.push(karakuri_engine::set::Published {
            name: "level".to_string(),
            at: None,
            key: "exposure".to_string(),
            range: [0.0, 4.0],
        });
        next.bindings.push(karakuri_engine::Binding::new(
            karakuri_ir::Kind::L4,
            "exposure",
            "control:level",
            karakuri_engine::binding::Curve::Lin,
            [0.0, 8.0],
        ));
        tx.send(next).expect("worker alive");
        h.frames_until(|e| matches!(e, Event::Swapped { .. }), "the build to land");

        let set = h.swap.set();
        assert_eq!(
            set.published()
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            vec!["level"],
            "the interface did not cross the swap"
        );
        assert_eq!(
            set.bindings().len(),
            1,
            "the binding was refused, so the order is wrong"
        );

        // And it drives. The binding resolved on the first frame after the swap,
        // from the control at whatever the `.kir` default left it — what matters
        // here is that it resolved from the *control* at all, which a binding
        // holding its manual value would not have.
        let driven = set.bindings()[0].value();
        let expected = set
            .published_value("level")
            .expect("the control holds a value")
            * 2.0;
        assert!(
            (driven - expected).abs() < 1e-3,
            "the macro resolved to {driven}, where the control at {} maps to {expected}",
            set.published_value("level").unwrap()
        );
    }

    // ---------------------------------------------------------------------------
    // What a rebuild does with the values: the declared ones come from the code,
    // the moved ones come from the Set that is playing.
    // ---------------------------------------------------------------------------

    /// `L1` with its `radius` default edited, which is the whole of what an
    /// author does between two saves. Everything else about the procedure is the
    /// same text, so the only thing a rebuild can be answering with is the
    /// declaration.
    const L1_EDITED_DEFAULT: &str = r#"
proc static_shell {
  kind     L1
  topology points
  capacity [1024, 262144] = 4096

  param radius : float [0.1, 8.0] = 5.0

  emit position, age

  element {
    let u = hash1(seed);
    let v = hash1(seed + 1000u);
    position = sphere_point(u, v) * radius;
    age      = age + dt;
  }
}
"#;

    /// A renderer that declares no `exposure` — the name a rebuild **dropped**.
    const L4_NO_EXPOSURE: &str = r#"
proc plain_points {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.015625;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    color = vec4(1.0, 1.0, 1.0, max(0.0, 1.0 - d));
  }
}
"#;

    /// A request over an L1 the caller chooses, so that an edited declaration is
    /// something a test can send.
    fn request_from(l1_src: &str, l4_srcs: &[&str], label: &str) -> Request {
        let mut r = request_many(l4_srcs, FIRST, label);
        r.l1s = vec![(compile(l1_src), FIRST)];
        r
    }

    fn value_at(set: &Set, layer: karakuri_ir::Kind, index: u32, key: &str) -> Option<f32> {
        set.params()
            .find(|(l, i, k, _)| *l == layer && *i == index && *k == key)
            .map(|(_, _, _, v)| v)
    }

    /// **An author edits a declared default and saves; the picture moves.** That
    /// is the one thing `--watch` exists to do, and it is the half of the rule
    /// that a rebuild inheriting *every* value by name would break — a Set holds
    /// one number per key, so carrying them all would carry the outgoing
    /// declaration forward and an edit would show nothing, forever
    /// (`docs/adr/0280-…`, §6, which is why that section said the information was
    /// not there).
    ///
    /// Nobody has touched `radius` here, so nothing about it was ever stated: the
    /// value comes from the code because the code is the only thing that has
    /// spoken.
    #[test]
    fn an_edited_declaration_lands_on_a_value_nobody_moved() {
        let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);
        for _ in 0..5 {
            h.frame();
        }
        assert_eq!(
            value_at(h.swap.set(), karakuri_ir::Kind::L1, 0, "radius"),
            Some(2.5),
            "the Set did not start at the declaration"
        );

        tx.send(request_from(L1_EDITED_DEFAULT, &[L4], "edited"))
            .expect("worker alive");
        h.frames_until(is_swapped, "the rebuild to land");

        assert_eq!(
            value_at(h.swap.set(), karakuri_ir::Kind::L1, 0, "radius"),
            Some(5.0),
            "the author edited `radius` to 5.0 and saved, and the rebuild came back \
             holding the value the outgoing Set was built with — an edit to a default \
             that changes nothing is `--watch` doing the one thing it is for"
        );
    }

    /// **A knob is ridden and then a `.kir` is saved; the knob stays where the
    /// operator left it.** The other half, and the one that was broken: the
    /// rebuild used to restate what the slot was *loaded* with, so a ride was
    /// walked back on the next save of any file in the slot, silently.
    ///
    /// The same save also carries an edited declaration for the parameter nobody
    /// touched, so one assertion pair covers both directions of the rule at once
    /// — which is the point of it being one rule.
    #[test]
    fn a_ridden_value_crosses_a_rebuild_and_a_declared_one_does_not() {
        let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);
        for _ in 0..5 {
            h.frame();
        }
        h.ride("exposure", 3.25);
        h.frame();

        tx.send(request_from(L1_EDITED_DEFAULT, &[L4], "saved"))
            .expect("worker alive");
        h.frames_until(is_swapped, "the rebuild to land");

        let set = h.swap.set();
        assert_eq!(
            value_at(set, karakuri_ir::Kind::L4, 0, "exposure"),
            Some(3.25),
            "the operator's hand was on `exposure` and the rebuild put it back to \
             what the file declares"
        );
        assert_eq!(
            value_at(set, karakuri_ir::Kind::L1, 0, "radius"),
            Some(5.0),
            "`radius` was never moved, so the rebuild owed it the new declaration"
        );
    }

    /// **A value this build states beats a value the outgoing Set was holding**,
    /// which is what separates *loading a Set* from *rebuilding one*. A slot
    /// pointed at a Set file states every declaration of every node, because that
    /// is what a live save writes; an operator who loads a preset over a slot
    /// they have been riding asked for the preset.
    #[test]
    fn a_value_the_request_states_beats_the_one_the_operator_moved() {
        let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);
        for _ in 0..5 {
            h.frame();
        }
        h.ride("exposure", 3.25);
        h.frame();

        let mut next = request(L4, FIRST, "loaded");
        next.params.push(karakuri_engine::ParamWrite::at(
            karakuri_ir::Kind::L4,
            0,
            "exposure",
            0.125,
        ));
        tx.send(next).expect("worker alive");
        h.frames_until(is_swapped, "the load to land");

        assert_eq!(
            value_at(h.swap.set(), karakuri_ir::Kind::L4, 0, "exposure"),
            Some(0.125),
            "the request said what `exposure` is and the ride was carried over it"
        );
    }

    /// **A name the rebuild dropped lands nowhere, and a node that is new comes
    /// up at its own declaration.** Two of the five cases the rule has to answer,
    /// in one save: the renderer that declared `exposure` is replaced by one that
    /// does not, and a second renderer appears behind it.
    ///
    /// The carry is addressed by `(layer, index)` — `ParamWrite::at`'s spelling —
    /// so the ridden `L4:0 exposure` is offered to `L4:0` and to nothing else. It
    /// declares no such name, so the value is gone; `L4:1` is a node the outgoing
    /// Set never had and takes what `wide_points` declares.
    #[test]
    fn a_dropped_name_is_gone_and_a_new_node_starts_at_its_declaration() {
        let (mut h, tx) = Harness::channel_driven(GENEROUS_MS);
        for _ in 0..5 {
            h.frame();
        }
        h.ride("exposure", 3.25);
        h.frame();

        tx.send(request_from(L1, &[L4_NO_EXPOSURE, L4_WIDE], "reshaped"))
            .expect("worker alive");
        h.frames_until(is_swapped, "the rebuild to land");

        let set = h.swap.set();
        assert_eq!(
            value_at(set, karakuri_ir::Kind::L4, 0, "exposure"),
            None,
            "`plain_points` declares no `exposure`, so a carried one is a value in a \
             Set nothing can address"
        );
        assert_eq!(
            value_at(set, karakuri_ir::Kind::L4, 1, "exposure"),
            Some(0.5),
            "`wide_points` is a node the outgoing Set never had, so it owes its own \
             declaration and not the ride from the renderer beside it"
        );
    }
}
