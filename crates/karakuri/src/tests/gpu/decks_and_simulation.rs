use super::common::*;

mod gpu {
    use super::*;

    /// Verifies knob interaction round-trip: UI edits generate records that update live
    /// deck state, and strips reflect values read back from the deck.
    #[test]
    fn a_drag_moves_the_deck_and_the_strip_follows_the_deck() {
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

        // The strips, written the way the frame writes them.
        let material = vec![shipped().material(); engine.deck.slot_count()];
        let mut strips = Vec::new();
        mixer(&engine.deck, &material, &mut strips);
        assert_eq!(strips.len(), engine.deck.slot_count());
        let was = engine.deck.gain(karakuri_engine::DeckSlot(0));

        // A knob, taken hold of and dragged to the bottom of its track. The
        // context has to have drawn once, because a strip is laid out with the
        // type in it.
        let ctx = super::tests::drawn_once();
        let bay = mixer_bay(&ctx, panel.layout(), &strips).expect("the bay draws its strip");
        let at = bay.strip(0);
        let knob = at.trim_at(strips[0].gain).knob.center();
        let grab = bay
            .grab(Point::new(knob.x, knob.y))
            .expect("the trim's knob");
        panel.grab(Point::new(knob.x, knob.y), grab);
        let floor = Point::new(at.trim.min.x, knob.y);
        let Some(Dragged::Fader(operation)) = panel.moved(floor) else {
            panic!("a drag to the floor of the trim emitted nothing")
        };
        assert_eq!(operation, Operation::SetGain { deck: 0, gain: 0.0 });

        // Verify deck and strip values remain unchanged before operation application.
        assert_eq!(engine.deck.gain(karakuri_engine::DeckSlot(0)), was);
        let mut after = Vec::new();
        mixer(&engine.deck, &material, &mut after);
        assert_eq!(
            after, strips,
            "the strip moved before the deck did, so the console is keeping a value"
        );

        // The record, and the deck.
        let record = super::tests::only_record(&operation);
        assert!(apply(
            &record,
            &mut engine.deck,
            &mut engine.look,
            &mut engine.chain
        )
        .is_some());
        assert_eq!(
            engine.deck.gain(karakuri_engine::DeckSlot(0)),
            0.0,
            "the record was built and the deck did not move, so the control ends nowhere"
        );

        // And now the strip follows, because it is read off the deck.
        mixer(&engine.deck, &material, &mut after);
        assert_eq!(after[0].gain, 0.0);
        assert_ne!(
            after, strips,
            "the deck moved and the strip did not follow it"
        );

        // The other direction, so that *follows the deck* is not *always
        // zero*: something else writes the deck and the strip says so without
        // a pointer anywhere near it.
        engine.deck.set_gain(karakuri_engine::DeckSlot(0), 0.5);
        mixer(&engine.deck, &material, &mut after);
        assert_eq!(after[0].gain, 0.5);
    }
    /// Verifies mix key adjustments affect only the targeted deck and derive levels
    /// directly from live deck state rather than potentially stale strip copies (ADR-0333).
    #[test]
    fn a_mix_key_moves_the_deck_the_operator_selected_and_leaves_the_others_alone() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;
        /// Deck C.
        const SELECTED: u8 = 2;
        /// Two levels are the same level, allowing for the arithmetic: a tenth is not
        /// an `f32`, so `0.6 + GAIN_STEP` and `0.7` are two different numbers and
        /// neither of them is wrong.
        const CLOSE: f32 = 1e-6;

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
        let here = usize::from(SELECTED);
        let here_slot = EngineSlot(SELECTED);
        assert!(
            here < engine.deck.slot_count(),
            "this deck has {} slots and the test selects deck {SELECTED}, so there is nothing \
         here to move",
            engine.deck.slot_count()
        );

        // Seed distinct values across all slots to prevent false positive matches.
        let seeded = [
            (0.20_f32, 0.90_f32, Blend::Add),
            (0.40, 0.70, Blend::Max),
            (0.60, 0.50, Blend::Over),
            (0.80, 0.30, Blend::Add),
        ];
        for (slot, (gain, opacity, blend)) in seeded.iter().copied().enumerate() {
            let addr = EngineSlot(slot as u8);
            engine.deck.set_gain(addr, gain);
            engine.deck.set_opacity(addr, opacity);
            engine.deck.set_blend(addr, blend);
        }

        // The console, with the strips the frame would have written into it —
        // `View::select` refuses a deck the mixer draws no strip for, so the
        // selection cannot be made before the readings are there.
        let mut readout = Readout::new(W as f32, H as f32);
        mixer(&engine.deck, &material, &mut readout.view.mixer);
        assert_eq!(
            readout.view.mixer.len(),
            engine.deck.slot_count(),
            "the view is not holding one strip per slot, so the selection below is being made \
         against something other than this deck"
        );
        assert_eq!(
            readout.view.selection(),
            0,
            "a run no longer opens on deck A, so selecting deck C proves nothing about a route \
         that ignores the selection"
        );
        assert!(
            readout.view.select(SELECTED),
            "deck C could not be selected on a four-slot deck"
        );
        assert_eq!(readout.view.selection(), SELECTED);

        /// Every value the three keys can move, per slot — the whole of what a press is
        /// allowed to have touched.
        fn snapshot(deck: &Deck) -> Vec<(f32, f32, Blend)> {
            (0..deck.slot_count())
                .map(|slot| {
                    let addr = EngineSlot(slot as u8);
                    (deck.gain(addr), deck.opacity(addr), deck.blend(addr))
                })
                .collect()
        }

        // -- the trim -------------------------------------------------------

        let was = snapshot(&engine.deck);
        let deck = readout.view.selection();
        let slot = held(&engine.deck, deck).expect("the selection is a slot this deck has");
        let gain = gain_key(Step::Up, engine.deck.gain(slot));
        let record = super::tests::only_record(&Operation::SetGain { deck, gain });
        assert!(apply(
            &record,
            &mut engine.deck,
            &mut engine.look,
            &mut engine.chain
        )
        .is_some());
        let now = snapshot(&engine.deck);
        assert!(
            (now[here].0 - (was[here].0 + GAIN_STEP)).abs() <= CLOSE,
            "an up press on deck C's addressed trim took it from {} to {} rather than one \
         {GAIN_STEP} up",
            was[here].0,
            now[here].0
        );
        for other in (0..now.len()).filter(|slot| *slot != here) {
            assert_eq!(
                now[other], was[other],
                "a press of `]` with deck C selected moved deck {other} as well — the route is \
             not addressed to the deck the operator picked"
            );
        }

        // -- the fader ------------------------------------------------------

        let was = snapshot(&engine.deck);
        let opacity = opacity_key(Step::Up, engine.deck.opacity(slot));
        let record = super::tests::only_record(&Operation::SetOpacity { deck, opacity });
        assert!(apply(
            &record,
            &mut engine.deck,
            &mut engine.look,
            &mut engine.chain
        )
        .is_some());
        let now = snapshot(&engine.deck);
        assert!(
            (now[here].1 - (was[here].1 + OPACITY_STEP)).abs() <= CLOSE,
            "an up press on deck C's addressed fader took it from {} to {} rather than one \
         {OPACITY_STEP} up",
            was[here].1,
            now[here].1
        );
        for other in (0..now.len()).filter(|slot| *slot != here) {
            assert_eq!(
                now[other], was[other],
                "a press on deck C's fader moved deck {other} as well"
            );
        }

        // -- the blend ------------------------------------------------------

        let was = snapshot(&engine.deck);
        let blend = karakuri_console::view::after(blend_mode(engine.deck.blend(slot)));
        assert_ne!(
            blend,
            blend_mode(was[here].2),
            "deck C's next mode is the one it is already on, so the assertion below would pass \
         on a press that did nothing"
        );
        let record = super::tests::only_record(&Operation::SetBlendMode { deck, blend });
        assert!(apply(
            &record,
            &mut engine.deck,
            &mut engine.look,
            &mut engine.chain
        )
        .is_some());
        let now = snapshot(&engine.deck);
        assert_eq!(
            blend_mode(now[here].2),
            blend,
            "`space` on deck C's addressed blend chip did not arrive at the mode the cycle \
         names"
        );
        for other in (0..now.len()).filter(|slot| *slot != here) {
            assert_eq!(
                now[other], was[other],
                "a press on deck C's blend chip moved deck {other} as well"
            );
        }

        // -- the guard ------------------------------------------------------

        // [`held`] guards deck slot indexing to avoid panics on out-of-range addresses.
        let past = engine.deck.slot_count();
        for slot in 0..past {
            assert_eq!(
                held(&engine.deck, slot as u8),
                Some(EngineSlot(slot as u8)),
                "slot {slot} is one this deck has and the guard refused it, so every press \
             would return without doing anything"
            );
        }
        assert_eq!(
            held(&engine.deck, past as u8),
            None,
            "the guard let a selection past this deck's {past} slots through"
        );
        assert_eq!(held(&engine.deck, u8::MAX), None);
        let quiet = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let read = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            engine.deck.gain(EngineSlot(past as u8))
        }));
        std::panic::set_hook(quiet);
        assert!(
            read.is_err(),
            "reading past this deck's slots answered {read:?} instead of panicking, so the \
         guard above is standing in front of nothing and this test says nothing about why \
         it is there"
        );

        // -- the reading is the deck's, not the strip's ----------------------

        // The strips as a frame would have left them, and then the deck moved
        // with no frame in between — which is what a scheduled fade landing
        // between the two does, arrived at the cheap way because it is the
        // staleness that matters rather than how it arose.
        let mut strips = Vec::new();
        mixer(&engine.deck, &material, &mut strips);
        assert_eq!(
            strips[here].gain,
            engine.deck.gain(here_slot),
            "the strips were just read off the deck and already disagree with it"
        );
        engine.deck.set_gain(here_slot, 0.25);
        assert_ne!(
            strips[here].gain,
            engine.deck.gain(here_slot),
            "the deck moved and the copy the console is holding moved with it, so there is no \
         stale reading here to tell the two sources apart"
        );
        let off_the_deck = gain_key(Step::Up, engine.deck.gain(here_slot));
        let off_the_strip = gain_key(Step::Up, strips[here].gain);
        assert_ne!(
            off_the_deck, off_the_strip,
            "a step counted from the deck and a step counted from the strip came out at the \
         same place, so this test cannot tell which source a press used"
        );
        let record = super::tests::only_record(&Operation::SetGain {
            deck,
            gain: off_the_deck,
        });
        assert!(apply(
            &record,
            &mut engine.deck,
            &mut engine.look,
            &mut engine.chain
        )
        .is_some());
        assert!(
            (engine.deck.gain(here_slot) - (0.25 + GAIN_STEP)).abs() <= CLOSE,
            "a press stepping from the deck's own reading landed at {} rather than at {}",
            engine.deck.gain(here_slot),
            0.25 + GAIN_STEP
        );
        assert!(
            (engine.deck.gain(here_slot) - off_the_strip).abs() > CLOSE,
            "the press landed where a step off the stale strip would have put it"
        );
    }

    /// Verifies `holding` reads live deck mixer states (tally, blend, mask, angle)
    /// directly rather than stale previous strip copies (ADR-0156).
    #[test]
    fn holding_reads_the_addressed_decks_own_state_and_never_a_strip_that_predates_it() {
        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            wgpu::TextureFormat::Rgba8Unorm,
            egui_wgpu::RendererOptions::default(),
        );
        let mut panel = Panel::new(1440.0, 900.0);
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
        /// Deck C — not the default and not the last, [`held`]'s own reason for the mix
        /// key test above.
        const AT: u8 = 2;
        let slot = held(&engine.deck, AT).expect("this deck has a slot 2");

        // A frame's worth of strips, read before anything below moves the
        // deck — the copy `holding` must answer differently from once the
        // deck has moved on.
        let mut strips = Vec::new();
        mixer(&engine.deck, &material, &mut strips);
        let before = strips[usize::from(AT)].clone();

        // Every state `holding` reports, moved to something the frame above
        // never saw.
        engine.deck.set_residency(slot, Residency::Priming);
        engine.deck.set_blend(slot, Blend::Max);
        engine
            .deck
            .set_mask(slot, Mask::new(MaskKind::Radial, 0.75, 0.0, 0.0));
        assert_ne!(
            tally(engine.deck.requested_residency(slot)),
            before.requested,
            "deck C's next residency is the one the frame above already drew, so the assertion \
         below would pass on a read that never moved"
        );
        assert_ne!(
            blend_mode(engine.deck.blend(slot)),
            before.blend,
            "deck C's next blend is the one the frame above already drew"
        );
        assert_ne!(
            masked(engine.deck.mask(slot).kind()),
            before.mask,
            "deck C's next mask is the one the frame above already drew"
        );
        assert_ne!(
            engine.deck.mask(slot).angle(),
            before.mask_angle,
            "deck C's next mask angle is the one the frame above already drew"
        );

        let now = holding(&engine.deck, AT).expect("this deck has a slot 2");
        assert_eq!(
            now.requested,
            tally(engine.deck.requested_residency(slot)),
            "`holding` answered a residency other than the one the deck holds now"
        );
        assert_eq!(
            now.blend,
            blend_mode(engine.deck.blend(slot)),
            "`holding` answered a blend other than the one the deck holds now"
        );
        assert_eq!(
            now.mask,
            masked(engine.deck.mask(slot).kind()),
            "`holding` answered a mask shape other than the one the deck holds now"
        );
        assert_eq!(
            now.mask_angle,
            engine.deck.mask(slot).angle(),
            "`holding` answered a mask angle other than the one the deck holds now"
        );

        // And none of the three states agrees with the strip a frame drew
        // before the move — the stale reading `holding` must not be
        // answering from.
        assert_ne!(now.requested, before.requested);
        assert_ne!(now.blend, before.blend);
        assert_ne!(now.mask, before.mask);
        assert_ne!(now.mask_angle, before.mask_angle);
    }

    /// Verifies that distinct slots derive independent simulation salts and render independently.
    #[test]
    fn every_slot_is_its_own_simulation() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());
        let mut panel = Panel::new(1440.0, 900.0);
        panel.solve();
        let engine = Engine::new(
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

        assert_eq!(
            engine.deck.slot_count(),
            SLOTS,
            "the deck is not built full, so the mixer has tracks with no channel in them"
        );
        let salts: Vec<u32> = (0..engine.deck.slot_count())
            .map(|slot| {
                let salts = engine
                    .deck
                    .slot(EngineSlot(slot as u8))
                    .set()
                    .source_salts();
                assert_eq!(
                    salts.len(),
                    1,
                    "the pair builds one geometry, so one salt is the whole of a slot's seed"
                );
                salts[0]
            })
            .collect();
        assert_eq!(
            salts[ON_AIR], SEED_SALT,
            "deck A is not seeded at the salt `karakuri-cli`'s tests use, so this program's \
         picture is not the one they look at"
        );
        let distinct: std::collections::BTreeSet<u32> = salts.iter().copied().collect();
        assert_eq!(
            distinct.len(),
            salts.len(),
            "two slots are the same simulation, so bringing a second channel up draws the \
         first one again: {salts:?}"
        );
    }

    /// Verifies compute budget governance can park a deck into `NoHeadroom` when live workload
    /// exceeds capacity, propagating parking residencies to UI strips (ADR-0190).
    #[test]
    fn the_budget_parks_a_deck_and_the_strip_carries_both_residencies() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());
        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        // Uses reference workload pairs to test compute budget exhaustion rather than closed-form Sets.
        let reference = reference();
        let slots: Vec<Sources> = std::iter::repeat_n(reference.clone(), SLOTS).collect();
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            HEADLESS_PICTURE_FORMAT,
            &slots,
            panel.layout(),
            1.0,
            None,
            None,
            mcp::Slots::unpointed(),
        );
        let material = vec![reference.material(); engine.deck.slot_count()];

        // Initial open state: deck A is Live and other decks are Allocated with agreeing residencies.
        let mut before = Vec::new();
        mixer(&engine.deck, &material, &mut before);
        // Asserts channel presence across all console tracks corresponding to decks (ADR-0178).
        assert_eq!(
            before.len(),
            DECKS,
            "the deck does not fill the bay's tracks, so the mixer draws tracks with no \
         channel in them"
        );
        assert_eq!(
            before[ON_AIR].tally,
            view::Tally::Live,
            "the slot the Program bay draws is not live"
        );
        assert!(
            before
                .iter()
                .enumerate()
                .all(|(slot, strip)| slot == ON_AIR || strip.is_muted),
            "a slot other than ON_AIR opened unmuted: {:?}",
            before
                .iter()
                .map(|strip| strip.is_muted)
                .collect::<Vec<_>>()
        );
        assert!(
            before.iter().all(|strip| strip.pending().is_none()),
            "a strip was pending before anything had asked for anything"
        );

        for slot in 0..SLOTS {
            if slot != ON_AIR {
                engine
                    .deck
                    .set_residency(EngineSlot(slot as u8), Residency::Allocated);
            }
        }

        let governed = engine.ask_to_prime(&gpu);

        // The engine's predicate first, since the console's is derived from
        // the same two values.
        assert!(
            engine.deck.is_parked(EngineSlot(ASKED_TO_PRIME as u8)),
            "the request was granted rather than parked — {governed}"
        );
        let parked: Vec<_> = governed.parked().collect();
        assert_eq!(parked.len(), 1, "{governed}");
        assert_eq!(parked[0].slot, ASKED_TO_PRIME);
        assert_eq!(
            parked[0].reason,
            Reason::NoHeadroom,
            "deck B is parked for a reason that is not the budget — {governed}"
        );
        assert_eq!(
            engine.deck.residency(EngineSlot(ON_AIR as u8)),
            Residency::Live,
            "the governor took the picture off air"
        );

        // Unrequested resting slots report `OffAir` rather than `NoHeadroom`.
        let resting: Vec<usize> = governed
            .decisions
            .iter()
            .filter(|decision| decision.reason == Reason::OffAir)
            .map(|decision| decision.slot)
            .collect();
        assert_eq!(
            resting,
            (0..SLOTS)
                .filter(|slot| *slot != ON_AIR && *slot != ASKED_TO_PRIME)
                .collect::<Vec<_>>(),
            "the slots this program asked nothing of are not the ones the governor left \
         alone — {governed}"
        );

        // Committed cost sums only Live slots; allocated slots add zero compute overhead.
        assert!(
            !governed.over_budget,
            "one live slot is already over the budget this program set — {governed}"
        );
        // Evaluates against budgeted compute allowance rather than raw measurements (ADR-0296).
        let budgeted = governed
            .decisions
            .iter()
            .find(|decision| decision.slot == ON_AIR)
            .and_then(|decision| decision.budgeted_ms)
            .expect("deck A is governed and has a number to be budgeted on");
        assert!(
            (governed.committed_ms - budgeted).abs() < f32::EPSILON,
            "the committed cost is not deck A's alone, so a slot nobody asked for is being \
         budgeted as if it were on air — {governed}"
        );

        // Logs hypothetical four-slot live compute costs under probe resolution for diagnostics (ADR-0191, P-0095).
        let costs: Vec<f32> = (0..SLOTS)
            .filter_map(|slot| engine.deck.slot(EngineSlot(slot as u8)).measured_cost())
            .map(|cost| cost.ms)
            .collect();
        println!(
            "  the deck's {} slots measured {:?} ms, summing to {:.3} ms against a \
         DEFAULT_COMPUTE_BUDGET_MS of {} ms — which is what the governor would hold \
         four LIVE slots against, and over which it warns rather than acts",
            costs.len(),
            costs,
            costs.iter().sum::<f32>(),
            karakuri_engine::governor::DEFAULT_COMPUTE_BUDGET_MS
        );

        // And both residencies cross the seam, which is what the roll is drawn
        // from: the strip carries the pair and the view derives the rest.
        let mut strips = Vec::new();
        mixer(&engine.deck, &material, &mut strips);
        assert_eq!(strips[ON_AIR].tally, view::Tally::Live);
        assert_eq!(
            strips[ON_AIR].pending(),
            None,
            "the live strip is pending something"
        );
        assert_eq!(strips[ASKED_TO_PRIME].tally, view::Tally::Allocated);
        assert_eq!(strips[ASKED_TO_PRIME].requested, view::Tally::Priming);
        assert_eq!(
            strips[ASKED_TO_PRIME].pending(),
            Some(view::Tally::Priming),
            "the strip's two residencies agree, so the chip has nothing to roll toward"
        );

        // Panel remains active while residencies diverge and the containing bay is laid out.
        let mut readout = Readout::new(1440.0, 900.0);
        readout.view.mixer = strips;
        assert_eq!(
            readout.view.animating(readout.panel.layout()),
            Some(view::ROLL_STALENESS),
            "a parked deck declared no staleness, so the roll never gets a frame"
        );
    }
}
