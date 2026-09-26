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

        // Advances transport timeline past the first beat to prevent quantum quantization collisions.
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

        // Verifies mask wipe position records are populated using live deck shape, angle, and softness (ADR-0334, ADR-0341).
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

    /// Verifies clicking a parked tally chip withdraws outstanding prime requests and returns the deck to allocated state (ADR-0191, ADR-0195).
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

        // Re-requesting prime under an unchanged budget re-triggers governor parking rather than granting live status (ADR-0191).
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

    /// Verifies mask shape selection cycles shape type while preserving current angle, softness, and direction (ADR-0201, ADR-0203).
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
}
