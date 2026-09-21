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
        use karakuri_engine::swap::measure;

        /// Test fixture resolution (1280x720) for probe measurement tests (ADR-0303).
        const AT: (u32, u32) = (1280, 720);

        let gpu = Gpu::headless().expect("no GPU available");
        let probed_target = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
        let fresh_target = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);

        let mut probed = stateful(&gpu);
        let mut fresh = stateful(&gpu);

        // `timestamps: false` takes the host-clock path deliberately: what is
        // being asserted is what the probe run does to the Set, not what it
        // measured, and calibration is half a second of GPU time for a number this
        // test never reads.
        let mut probe = Probe::new(&gpu.device, &gpu.queue, false, AT);
        let measurement = measure(&mut probe, &gpu.device, &gpu.queue, &mut probed, AT);
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
