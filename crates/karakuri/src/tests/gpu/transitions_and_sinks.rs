use super::common::*;

mod gpu {
    use super::*;

    /// Verifies that pressing go triggers a wipe transition converting settings into operation records (ADR-0201).
    #[test]
    fn the_go_pill_runs_a_wipe_against_the_settings_the_row_is_on() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;
        /// The deck the wipe covers, and the one the selection is put on.
        const UNDER: usize = 1;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());
        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            HEADLESS_PICTURE_FORMAT,
            &shipped_slots(),
            panel.layout(),
            1.0,
            None,
            None,
            mcp::Slots::unpointed(),
        );
        let material = vec![shipped().material(); engine.deck.slot_count()];
        let over = (UNDER + 1) % engine.deck.slot_count();
        let over_slot = EngineSlot(over as u8);
        assert!(
            engine.deck.slot_count() >= 2,
            "a deck of one slot cannot wipe and this test needs one that can"
        );

        // **The grid is run on before anything is scheduled**, and it is the
        // one piece of setup here that is not the product's own path.
        // `quantise` answers `ceil(beats / quantum) * quantum`, so at beat
        // zero every quantum agrees on zero and a record built with the wrong
        // one is indistinguishable from a record built with the row's. Four
        // seconds of grid is past the first bar at the default tempo, and the
        // guard below is what says this line is still doing its job.
        let mut signals = karakuri_engine::binding::Signals::default();
        signals.advance(u8::MAX, 1.0 / 60.0);
        engine.deck.set_signals(signals);
        assert!(
            engine.deck.signals().oscillator().beats() > 1.0,
            "the grid is still on the first beat, so every quantum quantises to the same \
         instant and the start below says nothing about the row"
        );

        let mut view = View::new(Room::Day);
        mixer(&engine.deck, &material, &mut view.mixer);
        view.select(UNDER as u8);
        assert_eq!(view.selection(), UNDER as u8);

        // **The row walked to `iris · now · 2 beats`, through the door the
        // panel has.** `now` rather than the next bar so the record's start is
        // the beat the grid is on and the arithmetic below has nothing to wait
        // for; every value is still one the pills can reach.
        for setting in [
            karakuri_operation::TransitionSetting::WipeShape {
                kind: karakuri_operation::WipeKind::Radial,
                angle: 0.0,
            },
            karakuri_operation::TransitionSetting::Quantum { beats: 0.0 },
            karakuri_operation::TransitionSetting::Length { beats: 2.0 },
        ] {
            let said = scheduled(&mut view, &Operation::SetTransition { setting })
                .expect("a transition setting said nothing at all");
            assert!(
                !said.contains("refused"),
                "the console refused `{setting:?}`, which this test needs it to take: {said}"
            );
        }
        let settings = view.transition();
        assert!(
            settings.armed(),
            "the row is not armed, so the press below would be refused for the shape"
        );

        // The press, at the centre of the `go` capsule.
        let ctx = super::tests::drawn_once();
        let row = transition_row(&ctx, panel.layout(), settings).expect("the transition row");
        let centre = row.go.center();
        let asked = row
            .go(
                Point::new(centre.x, centre.y),
                view.selection(),
                view.mixer.len(),
            )
            .expect("a press on the `go` capsule");
        let Go::Wipe(operation) = asked else {
            panic!(
                "the press was refused with the row armed and {} strips: {asked:?}",
                view.mixer.len()
            )
        };
        assert_eq!(
            operation,
            Operation::Wipe {
                from: UNDER as u8,
                to: over as u8,
            },
            "the press did not cover the addressed deck with the next one round"
        );

        // **Nothing has been told anything yet**, which is the middle step the
        // device is worth: the console asks and the deck moves when the record
        // does.
        let before = engine.deck.mask(over_slot);
        assert_eq!(
            engine.deck.transitions_on(over_slot).count(),
            0,
            "something was already moving on the deck this wipe arrives on"
        );

        // The reading, and the answer that used to be a sentence.
        let current = reading(
            &operation,
            &engine.deck,
            &engine.look,
            &engine.chain,
            settings,
            // No lane holds anything here: the banks a run starts with hold one
            // muted lane, and a muted lane holds no control (ADR-0323).
            &karakuri_pattern::Banks::default(),
        );
        let wiped = written(&operation, &current);
        assert!(
            matches!(wiped, Written::Records(_)),
            "a wipe off this window's own reading did not write records: {wiped:?} — the \
         two readings a wipe converts against are what this asserts are handed over"
        );
        let Written::Records(records) = &wiped else {
            unreachable!("just matched")
        };

        // **The transition the row is on, and not one this test spelled.**
        // `now` is a quantum of 0, which `quantise` answers with the beat the
        // grid is on, so the start is read off the same oscillator the
        // conversion read.
        let start = karakuri_engine::transition::quantise(
            engine.deck.signals().oscillator().beats(),
            settings.quantum,
        );
        assert!(
            records.contains(&Record::Transition {
                slot: DeckSlot(over as u8),
                control: "mask".to_owned(),
                to: 1.0,
                start,
                beats: settings.length,
                curve: "smooth".to_owned(),
            }),
            "the wipe's scheduled move is not the one the row is set to — {records:?}"
        );
        // **The guard on the start**, and it is what a green run means here: a
        // quantum this test did not choose would put the move on a different
        // instant, and without a grid that has been running it would put it on
        // the same one. `next bar` is the pill's other end of the same cycle.
        assert_ne!(
            start,
            karakuri_engine::transition::quantise(engine.deck.signals().oscillator().beats(), 4.0),
            "`now` and `next bar` quantise to the same instant on this grid, so the start \
         above is satisfied by a conversion reading a quantum nobody chose"
        );
        // And the mask, written whole out of the shape the *row* holds and the
        // position and soft edge the *deck* is wearing. The second of the two
        // puts the front at 0, which is what makes the move a wipe.
        assert!(
            records.contains(&Record::Mask {
                slot: DeckSlot(over as u8),
                kind: "radial".to_owned(),
                angle: 0.0,
                position: 0.0,
                softness: before.softness(),
            }),
            "the front is not put to 0 at the shape the row holds, with the deck's own soft \
         edge — {records:?}"
        );

        // And the deck follows.
        for record in records {
            apply(
                record,
                &mut engine.deck,
                &mut engine.look,
                &mut engine.chain,
            );
        }
        assert_eq!(
            engine.deck.mask(over_slot).kind(),
            MaskKind::Radial,
            "the records were built and the arriving deck is not wearing the row's shape"
        );
        assert_eq!(
            engine.deck.transitions_on(over_slot).count(),
            1,
            "the wipe wrote its records and nothing is moving on the deck it arrives on"
        );

        // **And the gap that was here until 2026-09-10, closed off the same
        // deck.** [`reading`] answered for a shape and for a wipe's arriving
        // deck and not for a position, so `written` came back
        // `Owed(NotRead(Mask))` and nothing moved — ADR-0334 recorded it and
        // ADR-0341 fixed it with one arm. This is the other side of that
        // assertion: the position is written whole, out of the number the
        // operation carries and the shape, the angle and the soft edge the
        // *deck* is wearing.
        //
        // **Read off this deck rather than spelled**, which is what makes it
        // the half `an_operation_whose_record_is_owed_is_said_rather_than_swallowed`
        // cannot make: a conversion that took a default here would pass
        // against a hand-written `Current` and put a shape nobody chose on a
        // deck mid-wipe.
        let wearing = engine.deck.mask(over_slot);
        let (kind, angle, softness) = (wearing.kind(), wearing.angle(), wearing.softness());
        let front = Operation::SetMaskPosition {
            deck: over as u8,
            position: 0.5,
        };
        assert_eq!(
            written(
                &front,
                &reading(
                    &front,
                    &engine.deck,
                    &engine.look,
                    &engine.chain,
                    settings,
                    &karakuri_pattern::Banks::default(),
                )
            ),
            Written::Records(vec![Record::Mask {
                slot: DeckSlot(over as u8),
                kind: wipe_kind(kind).name().to_string(),
                angle,
                position: 0.5,
                softness,
            }]),
            "a mask position off a real deck did not come back as the deck's own mask with \
         the asked-for front in it — the reading ADR-0341 added is not being taken, or \
         it is being taken off the wrong slot"
        );
    }

    /// The whole loop, closed on a parked deck: a press on the tally chip withdraws
    /// the prime request the governor could not grant, and the strip stops rolling
    /// because the *deck* changed.
    ///
    /// `tests/tally.rs` asserts everything up to the operation with no deck
    /// anywhere, which is the point of that file. This is the other end, and it
    /// needs a device because a `Deck` does — and because the state under test is
    /// one only a governor pass can produce
    /// ([ADR-0191](../../../docs/adr/0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md)):
    /// the request is asked for through the product's own path, refused by a budget
    /// computed from what the probe measured, and read back off the deck by
    /// [`mixer`] exactly as the frame reads it.
    ///
    /// What separates this from a plausible wrong answer is which residency the
    /// press names. The parked strip *shows* `alloc` and was *asked for* `prim`. A
    /// chip cycling from what it shows would ask for `live` and put deck B on air;
    /// cycling from the request asks for `allocated`, which is the withdrawal — and
    /// both are asserted here, on the operation and again on the deck, because the
    /// two are the same mistake at two removes (ADR-0195).
    ///
    /// The middle step is the one worth the device, as in the fader's test: between
    /// the press and the record the deck must not have moved, or the console would
    /// be applying what it is only supposed to ask for.
    #[test]
    fn a_press_on_a_parked_tally_withdraws_the_request_and_the_strip_follows_the_deck() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());
        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            HEADLESS_PICTURE_FORMAT,
            &shipped_slots(),
            panel.layout(),
            1.0,
            None,
            None,
            mcp::Slots::unpointed(),
        );
        let material = vec![shipped().material(); engine.deck.slot_count()];

        // The state, produced by the governor and not written here.
        let governed = engine.ask_to_prime(&gpu);
        assert!(
            engine.deck.is_parked(EngineSlot(ASKED_TO_PRIME as u8)),
            "the request was granted rather than parked — {governed}"
        );
        let mut strips = Vec::new();
        mixer(&engine.deck, &material, &mut strips);
        assert_eq!(
            strips[ASKED_TO_PRIME].pending(),
            Some(view::Tally::Priming),
            "the strip is not pending, so this test cannot tell the request from the readout"
        );

        // The press, at the centre of that strip's chip.
        let ctx = super::tests::drawn_once();
        let bay = mixer_bay(&ctx, panel.layout(), &strips).expect("the bay draws its strips");
        let chip = bay.strip(ASKED_TO_PRIME).tally.center();
        let operation = bay
            .tally(Point::new(chip.x, chip.y))
            .expect("the tally chip of the parked strip");
        assert_eq!(
            operation,
            Operation::SetResidency {
                deck: ASKED_TO_PRIME as u8,
                residency: karakuri_operation::Residency::Allocated,
            },
            "a press on the parked chip did not ask for the prime request to be withdrawn"
        );
        assert_ne!(
            operation,
            Operation::SetResidency {
                deck: ASKED_TO_PRIME as u8,
                residency: karakuri_operation::Residency::Live,
            },
            "the chip cycled from the residency it is showing, so a press meant to withdraw a \
         request would have put deck B on air"
        );

        // **Nothing has been told anything yet**, so the deck is where the
        // governor left it and so is the strip the frame would draw.
        assert!(engine.deck.is_parked(EngineSlot(ASKED_TO_PRIME as u8)));
        let mut after = Vec::new();
        mixer(&engine.deck, &material, &mut after);
        assert_eq!(
            after, strips,
            "the strip moved before the deck did, so the console is keeping a value"
        );

        // The record, and the deck. `apply` governs after it, which is what
        // stops this from being a residency the budget never granted.
        let record = super::tests::only_record(&operation);
        assert_eq!(
            record,
            Record::Residency {
                slot: DeckSlot(ASKED_TO_PRIME as u8),
                level: "allocated".to_owned(),
            }
        );
        assert!(apply(
            &record,
            &mut engine.deck,
            &mut engine.look,
            &mut engine.chain
        )
        .is_some());
        assert_eq!(
            engine
                .deck
                .requested_residency(EngineSlot(ASKED_TO_PRIME as u8)),
            Residency::Allocated,
            "the record was built and the request did not move, so the control ends nowhere"
        );
        assert_eq!(
            engine.deck.residency(EngineSlot(ASKED_TO_PRIME as u8)),
            Residency::Allocated,
            "the withdrawal left the slot somewhere other than where it was being held"
        );
        assert!(
            !engine.deck.is_parked(EngineSlot(ASKED_TO_PRIME as u8)),
            "the slot is still parked, so the request was not withdrawn"
        );
        assert_eq!(
            engine.deck.residency(EngineSlot(ON_AIR as u8)),
            Residency::Live,
            "withdrawing deck B's request took deck A off air"
        );

        // And now the strip follows, because it is read off the deck: nothing
        // is pending, so nothing rolls and the panel is still again.
        mixer(&engine.deck, &material, &mut after);
        assert_eq!(
            after[ASKED_TO_PRIME].pending(),
            None,
            "the deck stopped being parked and the strip went on rolling"
        );
        assert_ne!(
            after, strips,
            "the deck moved and the strip did not follow it"
        );
        let mut readout = Readout::new(W as f32, H as f32);
        readout.view.mixer = after.clone();
        assert_eq!(
            readout.view.animating(readout.panel.layout()),
            None,
            "the request is withdrawn and the panel is still asking for frames to roll a chip"
        );

        // And the cycle goes on from what the deck now holds rather than from
        // anything the console remembered: the next press asks for `live`.
        let bay = mixer_bay(&ctx, panel.layout(), &after).expect("the bay draws its strips");
        assert_eq!(
            bay.tally(Point::new(chip.x, chip.y)),
            Some(Operation::SetResidency {
                deck: ASKED_TO_PRIME as u8,
                residency: karakuri_operation::Residency::Live,
            }),
            "the second press did not carry on round the cycle from the deck's own request"
        );

        // **And the governor pass in `apply` is what makes a request a
        // request.** Asked to prime again — the record the chip writes when
        // the cycle comes round to it — the budget is still the budget, so the
        // slot is parked again rather than granted. Without the pass
        // `Deck::set_residency` would grant it on the spot and this panel
        // would draw a primed deck the governor never admitted, which is
        // ADR-0191 read forwards.
        let again = Record::Residency {
            slot: DeckSlot(ASKED_TO_PRIME as u8),
            level: "priming".to_owned(),
        };
        assert!(apply(
            &again,
            &mut engine.deck,
            &mut engine.look,
            &mut engine.chain
        )
        .is_some());
        assert_eq!(
            engine
                .deck
                .requested_residency(EngineSlot(ASKED_TO_PRIME as u8)),
            Residency::Priming,
            "the request was not written"
        );
        assert!(
            engine.deck.is_parked(EngineSlot(ASKED_TO_PRIME as u8)),
            "the prime request was granted rather than parked, so nothing governed the record \
         and the panel is drawing a residency the budget never allowed"
        );
    }

    /// The whole loop, closed on a masked deck: a press on the mask mini chooses
    /// the next shape and leaves the front, the soft edge and — the one this test
    /// exists for — the *angle* exactly where they were.
    ///
    /// `tests/mask.rs` asserts everything up to the operation with no deck
    /// anywhere, which is the point of that file. This is the other end, and it
    /// needs a device because a `Deck` does.
    ///
    /// What separates this from the plausible wrong answer is the angle. The chip
    /// names a *shape*; `Operation::SetMaskShape` carries a shape and an angle
    /// (ADR-0201), so a press must carry an angle it does not control. Carrying the
    /// one the slot already wears is the whole of
    /// [ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md);
    /// carrying `0.0` would look like a chip minding its own business and would
    /// straighten a diagonal wipe on every press, with nothing on the panel saying
    /// so — the mark is the same mark at any angle.
    ///
    /// The middle step is the one worth the device, as in the fader's test and the
    /// tally's: between the press and the record the deck must not have moved, or
    /// the console would be applying what it is only supposed to ask for.
    #[test]
    fn a_press_on_the_mask_mini_chooses_a_shape_and_keeps_the_angle() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;
        /// A diagonal front, in radians — not zero, and not the default, which is the
        /// only reason this test can tell the two designs apart.
        const ANGLE: f32 = 0.9;
        /// Half way across, and a soft edge, so that a record written from the
        /// operation alone would show up in these two as well.
        const FRONT: f32 = 0.4;
        const SOFTNESS: f32 = 0.05;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());
        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            HEADLESS_PICTURE_FORMAT,
            &shipped_slots(),
            panel.layout(),
            1.0,
            None,
            None,
            mcp::Slots::unpointed(),
        );
        let material = vec![shipped().material(); engine.deck.slot_count()];

        // A wipe in progress on the deck that is on air: a straight front,
        // running at an angle, part of the way across.
        engine.deck.set_mask(
            EngineSlot(ON_AIR as u8),
            Mask::new(MaskKind::Linear, ANGLE, FRONT, SOFTNESS),
        );
        let mut strips = Vec::new();
        mixer(&engine.deck, &material, &mut strips);
        assert_eq!(
            strips[ON_AIR].mask,
            view::Mask::Linear,
            "the strip is not showing the mask the deck is wearing"
        );
        assert_eq!(
            strips[ON_AIR].mask_angle, ANGLE,
            "the strip did not carry the angle off the deck, so a press has nothing to \
         hand back and this test cannot tell the two designs apart"
        );

        // The press, at the centre of that strip's mini.
        let ctx = super::tests::drawn_once();
        let bay = mixer_bay(&ctx, panel.layout(), &strips).expect("the bay draws its strips");
        let chip = bay.strip(ON_AIR).mask.center();
        let operation = bay
            .mask(Point::new(chip.x, chip.y))
            .expect("the mask mini of the masked strip");
        assert_eq!(
            operation,
            Operation::SetMaskShape {
                deck: ON_AIR as u8,
                kind: karakuri_operation::WipeKind::Radial,
                angle: ANGLE,
            },
            "the press did not ask for the next shape at the angle the deck is wearing"
        );
        assert_ne!(
            operation,
            Operation::SetMaskShape {
                deck: ON_AIR as u8,
                kind: karakuri_operation::WipeKind::Radial,
                angle: 0.0,
            },
            "the press sent a default angle, so choosing a shape straightens a diagonal \
         wipe and nothing on the panel says so"
        );

        // **Nothing has been told anything yet**, so the deck is where it was
        // and so is the strip the frame would draw.
        assert_eq!(
            engine.deck.mask(EngineSlot(ON_AIR as u8)).kind(),
            MaskKind::Linear
        );
        let mut after = Vec::new();
        mixer(&engine.deck, &material, &mut after);
        assert_eq!(
            after, strips,
            "the strip moved before the deck did, so the console is keeping a value"
        );

        // Verify operation record generation matches current deck readings (ADR-0201).
        let written = written(
            &operation,
            &reading(
                &operation,
                &engine.deck,
                &engine.look,
                &engine.chain,
                TransitionSettings::START,
                &karakuri_pattern::Banks::default(),
            ),
        );
        let Written::Records(records) = &written else {
            panic!("a press on the mask mini wrote no record: {written:?}")
        };
        assert_eq!(
            records.as_slice(),
            [Record::Mask {
                slot: DeckSlot(ON_AIR as u8),
                kind: "radial".to_owned(),
                angle: ANGLE,
                position: FRONT,
                softness: SOFTNESS,
            }],
            "the record is not the whole mask with only the shape changed"
        );

        // And the deck.
        assert!(apply(
            &records[0],
            &mut engine.deck,
            &mut engine.look,
            &mut engine.chain
        )
        .is_some());
        let mask = engine.deck.mask(EngineSlot(ON_AIR as u8));
        assert_eq!(
            mask.kind(),
            MaskKind::Radial,
            "the record was built and the shape did not move, so the control ends nowhere"
        );
        assert_eq!(
            mask.angle(),
            ANGLE,
            "choosing a shape straightened the front — the angle the chip does not control \
         was rewritten by a press meant to choose a shape"
        );
        assert_eq!(
            mask.position(),
            FRONT,
            "choosing a shape moved the front, which is the half of the mask this operation \
         does not name"
        );
        assert_eq!(mask.softness(), SOFTNESS, "the soft edge was rewritten");

        // And the strip follows, because it is read off the deck rather than
        // remembered — and the next press carries on round the cycle from what
        // the deck now holds, still at the same angle.
        mixer(&engine.deck, &material, &mut after);
        assert_eq!(after[ON_AIR].mask, view::Mask::Radial);
        assert_eq!(after[ON_AIR].mask_angle, ANGLE);
        let bay = mixer_bay(&ctx, panel.layout(), &after).expect("the bay draws its strips");
        assert_eq!(
            bay.mask(Point::new(chip.x, chip.y)),
            Some(Operation::SetMaskShape {
                deck: ON_AIR as u8,
                kind: karakuri_operation::WipeKind::None,
                angle: ANGLE,
            }),
            "the second press did not wrap round to `none` at the angle the deck still holds"
        );
    }

    /// The whole loop, closed on the look: a press on the tone map capsule chooses
    /// the next operator and keeps the level, and a press on the exposure track
    /// sets the level and keeps the operator.
    ///
    /// `tests/look.rs` asserts everything up to the operation with no engine
    /// anywhere, which is the point of that file. This is the other end, and it
    /// needs a device because [`Engine`] does — and because the value being moved
    /// is [`Engine::look`], which is what every sink is drawn under.
    ///
    /// What separates this from the plausible wrong answer is the third of the
    /// record neither press names. `Record::Look` is an operator, a level and a
    /// white point; each control asks for one of the first two and [`reading`]
    /// supplies the rest (ADR-0192). A build that filled the missing thirds from a
    /// default would cycle the tone map and silently reset the exposure — and would
    /// rewrite `white_point`, which is on no surface at all and would therefore
    /// change with nothing saying so. So the look this starts from has none of the
    /// three at its default.
    ///
    /// The middle step is the one worth the device, as in the mask's test: between
    /// the press and the record the look must not have moved, or the console would
    /// be applying what it is only supposed to ask for.
    #[test]
    fn a_press_on_the_look_controls_moves_the_look_every_sink_is_drawn_under() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;
        /// Not `LOOK`'s three, so a press that dropped a third of the record and filled
        /// it from a default is visible in every one of them.
        const STARTS_AT: Look = Look {
            op: TonemapOp::Reinhard,
            exposure: 0.5,
            white_point: 3.5,
        };

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());
        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            HEADLESS_PICTURE_FORMAT,
            &shipped_slots(),
            panel.layout(),
            1.0,
            None,
            None,
            mcp::Slots::unpointed(),
        );
        engine.look = STARTS_AT;

        // What the console reads this frame, off the look the engine holds.
        let ctx = super::tests::drawn_once();
        let mut view = View::new(karakuri_console::room::Room::Day);
        // The mock's own transport, which is what `tests/transport.rs` and
        // the console's own tests read: the group is measured from the
        // arrangement pill and the pill from the bar, so a row is needed to
        // have either.
        view.transport = Some(view::Transport {
            bpm: 128.0,
            beats: 144.0,
            beats_per_bar: karakuri_signal::oscillator::BEATS_PER_BAR,
            fps: Some(58.0),
            frame_ms: 12.4,
            budget_ms: Some(16.6),
            chain_ms: None,
            health: Some(view::Stage::Landed),
            rec: Some(view::Rec::Idle),
        });
        view.look = Some(look(&engine.look));
        assert_eq!(
            view.look,
            Some(view::Look {
                tonemap: karakuri_operation::Tonemap::Reinhard,
                exposure: 0.5,
            }),
            "the console is not reading the look the engine is drawing under"
        );

        let row = look_row(
            &ctx,
            panel.layout(),
            view.transport,
            view.audio.as_ref(),
            view.tracker,
            None,
            &view.arrangement,
            view.look,
        )
        .expect("the transport row draws the look controls");

        // ---- the capsule: the next operator, at the level that is running --
        let capsule = row.tone.center();
        let operation = row
            .tonemap(Point::new(capsule.x, capsule.y))
            .expect("a press on the tone map capsule");
        assert_eq!(
            operation,
            Operation::SetTonemap {
                tonemap: karakuri_operation::Tonemap::Aces,
            },
            "the press did not ask for the operator after `reinhard`"
        );
        // **Nothing has been told anything yet.**
        assert_eq!(
            engine.look, STARTS_AT,
            "the look moved before the record did"
        );

        // **The transition settings are where a run begins and this press
        // does not read them**: `Current::transition` is a wipe's, a fade's, a
        // crossfade's and a selection's, and none of the three conversions
        // below is one of those. Handed in because `reading` takes them, and
        // `START` rather than a chosen value so that nothing here can look
        // like a setting the test needed.
        let chosen = written(
            &operation,
            &reading(
                &operation,
                &engine.deck,
                &engine.look,
                &engine.chain,
                TransitionSettings::START,
                &karakuri_pattern::Banks::default(),
            ),
        );
        let Written::Records(records) = &chosen else {
            panic!("a press on the tone map capsule wrote no record: {chosen:?}")
        };
        assert_eq!(
            records.as_slice(),
            [Record::Look {
                op: "aces".to_owned(),
                exposure: 0.5,
                white_point: 3.5,
            }],
            "the record is not the whole look with only the operator changed"
        );
        assert!(apply(
            &records[0],
            &mut engine.deck,
            &mut engine.look,
            &mut engine.chain
        )
        .is_some());
        assert_eq!(
            engine.look,
            Look {
                op: TonemapOp::Aces,
                ..STARTS_AT
            },
            "cycling the tone map did not leave the level and the white point alone"
        );

        // ---- the track: the level under the press, at the operator running -
        let row = look_row(
            &ctx,
            panel.layout(),
            view.transport,
            view.audio.as_ref(),
            view.tracker,
            None,
            &view.arrangement,
            Some(look(&engine.look)),
        )
        .expect("the group is still drawn");
        let middle = row.grip.center();
        let operation = row
            .exposure(Point::new(middle.x, middle.y))
            .expect("a press on the exposure track");
        assert_eq!(
            operation,
            Operation::SetExposure { exposure: 1.0 },
            "a press at the middle of the track did not ask for unity"
        );

        // **The transition settings are where a run begins and this press
        // does not read them**: `Current::transition` is a wipe's, a fade's, a
        // crossfade's and a selection's, and none of the three conversions
        // below is one of those. Handed in because `reading` takes them, and
        // `START` rather than a chosen value so that nothing here can look
        // like a setting the test needed.
        let levelled = written(
            &operation,
            &reading(
                &operation,
                &engine.deck,
                &engine.look,
                &engine.chain,
                TransitionSettings::START,
                &karakuri_pattern::Banks::default(),
            ),
        );
        let Written::Records(records) = &levelled else {
            panic!("a press on the exposure track wrote no record: {levelled:?}")
        };
        assert_eq!(
            records.as_slice(),
            [Record::Look {
                op: "aces".to_owned(),
                exposure: 1.0,
                white_point: 3.5,
            }],
            "the record is not the whole look with only the level changed — the operator the \
         press cannot name, or the white point no surface can, was rewritten"
        );
        assert!(apply(
            &records[0],
            &mut engine.deck,
            &mut engine.look,
            &mut engine.chain
        )
        .is_some());
        assert_eq!(
            engine.look,
            Look {
                op: TonemapOp::Aces,
                exposure: 1.0,
                white_point: 3.5,
            },
            "the level did not land, or it took the operator or the white point with it"
        );

        // And the console follows, because it is read off the engine rather
        // than remembered.
        assert_eq!(
            look(&engine.look),
            view::Look {
                tonemap: karakuri_operation::Tonemap::Aces,
                exposure: 1.0,
            }
        );
    }

    /// Which rectangle each sink's texture is sized from, and where the console
    /// then draws it — asked of the call the frame actually makes.
    ///
    /// This is the hole `docs/roadmap.md` recorded, closed. The decision used to be
    /// two `match`es inside `App::window_event`, and `winit` will not hand a test
    /// an `ActiveEventLoop`, so nothing could call it: `mod gpu` asserted what
    /// `Engine::new` did and not what the frame chose. Sizing deck A's texture from
    /// the picture's rectangle was injected there and every test still passed. It
    /// is [`aims`] and [`Engine::aim`] now, which take a solved layout and a scale
    /// factor and touch no window, and this asks them at a viewport and a scale
    /// neither of which `Engine::new` was given — so what is asserted is what `aim`
    /// decided rather than what construction left behind.
    ///
    /// Every half of it fails silently. A texture sized from the wrong rectangle
    /// looks perfectly correct — the cell is drawn at whatever size it is and the
    /// texture fills it — and is four to twenty times the texels the cell needs,
    /// per frame, for as long as the deck runs. A `Picture` carrying an id from
    /// before a resize is a freed registration, which `egui` draws as nothing at
    /// all. And a folded region whose sink still acquires is the manual's *"no
    /// state where it is hidden and still costing a pass"* quietly stopping being
    /// true.
    #[test]
    fn the_frame_aims_each_sink_at_its_own_rectangle() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;
        /// A display of a different scale, because the size is the rectangle and the
        /// scale and a test at 1.0 cannot tell them apart.
        const SCALE: f32 = 2.0;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());

        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            HEADLESS_PICTURE_FORMAT,
            &shipped_slots(),
            panel.layout(),
            1.0,
            None,
            None,
            mcp::Slots::unpointed(),
        );

        // A different window on a different display, so nothing asserted below
        // can be what construction happened to leave in place — and at 1760
        // wide it is a window past the crossover, so what is asserted below is
        // the arrangement with the four cells **beside** the picture. The
        // rearrangement is the frame's own first act and this test makes it in
        // the same order.
        panel.set_viewport(W as f32 + 320.0, H as f32 - 120.0);
        view::rearrange(&mut panel, CANVAS);
        assert!(
            panel
                .layout()
                .is_set_aside(panel.layout().find("deck-previews").expect("the row")),
            "1760 is not past the crossover, so the cells are still in the row"
        );
        let rect = picture_rect(panel.layout(), CANVAS).expect("the picture is on screen");
        let cell = preview_rects(panel.layout(), CANVAS).expect("the preview row is on screen")[0];
        let (picture, previews) = engine.aim(&gpu, &mut renderer, panel.layout(), SCALE, None);

        // **Each texture is the size of its own rectangle, at this scale.**
        assert_eq!(
            engine.picture.size,
            physical(rect, SCALE),
            "the picture's texture is not the size of the picture's region"
        );
        assert_eq!(
            engine.previews[0].size,
            physical(cell, SCALE),
            "deck A's texture is not the size of deck A's cell — it was sized from some \
         other rectangle, and nothing on screen would say so"
        );
        assert_ne!(
            engine.previews[0].size, engine.picture.size,
            "deck A's texture is the picture's size"
        );
        // **Half the picture in each direction, and it is half rather than the
        // quarter this used to ask for.** A quarter of the width was the row's
        // arithmetic — four tracks across the bay — and beside the picture a
        // cell is half a column, so it is about half the picture each way and
        // a quarter of its texels. The claim being made is the one that
        // catches the defect either way: a cell sized from the picture's
        // rectangle, or from the window, is *larger* than this and not
        // smaller.
        assert!(
            engine.previews[0].size.0 * 2 <= engine.picture.size.0
                && engine.previews[0].size.1 * 2 <= engine.picture.size.1,
            "a preview cell is not much smaller than the picture: {:?} against {:?}",
            engine.previews[0].size,
            engine.picture.size
        );

        // **And where the console draws it is the same statement**: the
        // rectangle the texture was just sized from, and the id the sizing may
        // have just replaced.
        let drawn = picture.expect("the picture is on screen and the frame aimed nothing at it");
        assert_eq!(
            drawn.rect, rect,
            "the picture is drawn somewhere other than the region \
         its texture was sized from"
        );
        assert_eq!(
            drawn.id, engine.picture.id,
            "the view carries the id from before the resize, which is a freed registration"
        );
        assert!(renderer.texture(&drawn.id).is_some());
        let monitored = previews[0].expect("deck A has a slot and the frame aimed nothing at it");
        assert_eq!(monitored.rect, cell);
        assert_eq!(monitored.id, engine.previews[0].id);
        assert!(renderer.texture(&monitored.id).is_some());
        // **All four, and residency has nothing to do with it.** Deck A is the
        // only Live slot on this engine and the other three rest at
        // `Allocated`; every one of them is drawn into its own target and
        // every one of them is aimed at a cell, which is ADR-0258 —
        // `every_cell_with_a_slot_behind_it_is_aimed_whatever_its_residency`
        // is where that is asserted across all four residency arrangements.
        assert!(
            previews.iter().all(Option::is_some),
            "a cell with a deck slot behind it was not aimed: an operator watches a \
         candidate's cell to decide whether to put it on air, so a cell that waits \
         for Live is dark at the one moment it is wanted"
        );

        // **Aimed is what `Sink::acquire` answers from**, and that is the
        // whole of what `compose` asks either of them.
        assert_eq!(engine.picture.acquire(&gpu), Ok(()));
        assert_eq!(engine.previews[0].acquire(&gpu), Ok(()));

        // **Fold the picture away and its sink has no target** — so `compose`
        // records no present pass into it, the deck still advances, and the
        // four cells go on monitoring underneath. Both halves matter: a fold
        // that took the cells with it is the console going dark from one
        // keystroke.
        let picture_node = panel.layout().find("program-view").expect("program-view");
        assert!(
            matches!(
                panel.op(Op::Fold(picture_node)),
                Outcome::Folded { folded: true, .. }
            ),
            "the picture did not fold"
        );
        // **The bay rearranges around the fold, and this is the guard rule
        // reached through the frame's own call.** The cells were beside the
        // picture; with the picture gone the row comes back under it, because
        // a bay whose only laid-out child is set aside can use nothing at all
        // and would claim no height. Reading a rectangle without this is
        // reading one from before the fold — the same contract `Layout::rect`
        // has about a stale solve.
        view::rearrange(&mut panel, CANVAS);
        assert!(picture_rect(panel.layout(), CANVAS).is_none());
        assert!(
            !panel
                .layout()
                .is_set_aside(panel.layout().find("deck-previews").expect("the row")),
            "the picture is folded and the row is still set aside, so the Program bay \
         claims nothing and has gone from the panel"
        );
        let (picture, previews) = engine.aim(&gpu, &mut renderer, panel.layout(), SCALE, None);
        assert!(
            picture.is_none(),
            "the picture is folded away and the frame still gave the console one to draw"
        );
        assert_eq!(
            engine.picture.acquire(&gpu),
            Err(Skip::Transient),
            "the picture is folded away and its sink still took the frame, so a present \
         pass is recorded into a texture nothing shows"
        );
        assert!(
            previews.iter().all(Option::is_some),
            "folding the picture away stopped the cells monitoring under it"
        );
        assert_eq!(
            engine.previews[0].acquire(&gpu),
            Ok(()),
            "folding the picture away stopped deck A's cell taking the frame"
        );
    }

    /// ADR-0155's bet, as an assertion.
    ///
    /// The record chose `egui` and paid a `wgpu` major version for it on the
    /// grounds that the panel and the engine share one `Device`. This builds the
    /// console's frame with an `egui` context, tessellates it, renders it through
    /// `egui-wgpu` into a texture on a device `karakuri-engine` created, and reads
    /// the texels back. If the two ever resolve different `wgpu`s it does not
    /// compile; if the render path breaks, `poll` gives a device-side complaint
    /// somewhere to surface; and if what lands is not the console, the two pixels
    /// below say so.
    ///
    /// The pixels are the point. A frame that renders without complaining and is
    /// the wrong colour is the failure that is easy to ship: `egui`'s shader writes
    /// gamma-encoded texels because it is told the target is gamma space, so an
    /// sRGB target encodes a second time and the whole panel washes out — with no
    /// error anywhere. So a bay's body is asserted to be exactly `--c-panel` and a
    /// divider is asserted not to be.
    #[test]
    fn egui_paints_the_console_onto_a_device() {
        // Gamma space, not sRGB: see above, and the window's own choice of
        // surface format, which is made for this reason.
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        // 1408 rather than 1440 for one reason: `copy_texture_to_buffer` wants
        // `bytes_per_row` a multiple of 256, and 1408 * 4 is 5632.
        const W: u32 = 1408;
        const H: u32 = 900;
        const ROOM: Room = Room::Night;

        let gpu = Gpu::headless().expect("no GPU");
        let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("console probe"),
            size: wgpu::Extent3d {
                width: W,
                height: H,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());

        let ctx = egui::Context::default();
        let mut panel = Panel::new(W as f32, H as f32);
        let mut view = View::new(ROOM);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(W as f32, H as f32),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| view.draw(ui, &mut panel));
        let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
        // The console has thirteen regions and seven headings, so a frame that
        // tessellated to nothing is a frame that drew nothing.
        assert!(
            !primitives.is_empty(),
            "the console tessellated to no primitives at all"
        );

        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [W, H],
            pixels_per_point: 1.0,
        };
        for (id, deltas) in &output.textures_delta.set {
            for delta in deltas {
                renderer.update_texture(&gpu.device, &gpu.queue, *id, delta);
            }
        }
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("console probe"),
            });
        let user =
            renderer.update_buffers(&gpu.device, &gpu.queue, &mut encoder, &primitives, &screen);
        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("console probe"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            renderer.render(&mut pass.forget_lifetime(), &primitives, &screen);
        }
        for id in &output.textures_delta.free {
            renderer.free_texture(id);
        }
        output.textures_delta.clear();

        let row = W * 4;
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("console probe"),
            size: (row * H) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(H),
                },
            },
            wgpu::Extent3d {
                width: W,
                height: H,
                depth_or_array_layers: 1,
            },
        );
        gpu.queue.submit(user.into_iter().chain([encoder.finish()]));
        readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the device stopped");
        let texels = readback.slice(..).get_mapped_range().expect("readback");
        let at = |p: karakuri_layout::Point| {
            let i = (p.y as u32 * row + p.x as u32 * 4) as usize;
            [texels[i], texels[i + 1], texels[i + 2]]
        };

        let pal = ROOM.palette();
        let panel_rgb = [pal.panel.r(), pal.panel.g(), pal.panel.b()];

        // A bay's body, well clear of its head and its edges.
        panel.solve();
        let library = panel
            .layout()
            .rect(panel.layout().find("library").expect("library"));
        let inside =
            karakuri_layout::Point::new(library.x + library.w * 0.5, library.y + library.h * 0.5);
        assert_eq!(
            at(inside),
            panel_rgb,
            "an empty bay's body is not --c-panel; the colour space or the palette is wrong"
        );

        // A divider: the ground shows through. Not `--c-ground` exactly, and
        // that is right rather than a tolerance — the bays either side cast
        // their shadow into the gap, as they do in the mock. So the assertion
        // is which of the two colours it is nearer, which is the question
        // "does the ground show through" and is not a threshold anybody has to
        // tune.
        let ground_rgb = [pal.ground.r(), pal.ground.g(), pal.ground.b()];
        let (split, index) = panel.layout().boundaries().next().expect("no boundary");
        let gap = panel.layout().boundary(split, index).expect("no pair");
        let in_gap = karakuri_layout::Point::new(gap.x + gap.w * 0.5, gap.y + gap.h * 0.5);
        let found = at(in_gap);
        let away = |from: [u8; 3]| -> i32 {
            (0..3)
                .map(|i| (found[i] as i32 - from[i] as i32).abs())
                .sum()
        };
        assert!(
            away(ground_rgb) < away(panel_rgb),
            "a divider at {found:?} is nearer the bay {panel_rgb:?} than the ground \
         {ground_rgb:?}, so the ground is not showing through"
        );
    }
}
