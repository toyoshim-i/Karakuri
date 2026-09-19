use super::common::*;

mod gpu {
    use super::*;

    /// The whole loop, closed on a real deck: a hand moves a knob and the strip
    /// follows because the *deck* changed.
    ///
    /// `tests/fader.rs` asserts everything up to the operation and one thing past
    /// it — that the console keeps no value of its own — and it does all of that
    /// with no deck anywhere, which is the point of that file. This is the other
    /// end, and it needs a device because a `Deck` does: the operation becomes a
    /// `Record`, the record moves the deck, and the strips are read back off the
    /// deck by [`mixer`] exactly as the frame reads them.
    ///
    /// The middle step is the one worth the device. Between the drag and the record
    /// the strip must *not* have moved — if it had, the console would be showing a
    /// number it kept rather than one the deck holds, and every assertion after it
    /// would pass over a second copy of the deck's state.
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

        // **Nothing has been told anything yet**, so the deck is where it was
        // and so is the strip the frame would draw.
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
    /// A mix key moves the deck the operator selected, and leaves the other three
    /// exactly where they were — the three key routes closed on a real deck, which
    /// is why this is here rather than beside the helpers' own tests.
    ///
    /// `tests::stepping_the_trim_…` and the one beside it assert [`gain_key`] and
    /// [`opacity_key`] with no deck anywhere, which is the point of those two, and
    /// `karakuri-console/tests/grammar.rs` asserts which control a press lands on
    /// with no deck either. This is the other end of the same chain, and it needs a
    /// device because a `Deck` does: four slots exist, the selection is one of
    /// them, the level the press steps from is read off that slot, the operation
    /// becomes a `Record` and the record moves one slot.
    ///
    /// What separates it from a plausible wrong answer is which deck it lands on.
    /// Deck C is selected — not the default and not the last, so a selection
    /// ignored in either direction lands somewhere this test can see — and no two
    /// slots are seeded alike, which is `tests/blend.rs`'s rule at the far end of
    /// the same chain. Both halves are asserted: the one slot that moved and the
    /// three that did not, because only the second catches a route that acted on a
    /// deck of its own.
    ///
    /// And the level comes off the deck rather than off `view::Strip`, which the
    /// arms' own comment claims and nothing measured. A strip is that same reading
    /// copied once a frame, so the two disagree the moment anything moves the deck
    /// without the frame having run again — a scheduled fade landing between the
    /// two is one way and the only one the comment names, but it is the *staleness*
    /// that matters and not how it arose, so it is made here the cheap way. What is
    /// asserted is that the two sources give different answers and that the deck's
    /// is the one that lands right.
    ///
    /// # What this cannot reach, and what does
    ///
    /// This doc named three arms inline in `App::window_event` until 2026-09-10,
    /// when ADR-0333 moved the trim and the fader's half of that chain into two
    /// free functions, [`held`] and [`answered`], reached from the grammar rather
    /// than from three letters. `answered` is not inline in `window_event` any
    /// more, but it is still out of this test's reach for a narrower reason: it
    /// takes `&mut Gfx`, which bundles a live `winit::window::Window` and a
    /// `wgpu::Surface`, and nothing in this workspace builds one off-screen for a
    /// test the way [`Engine`] is built here for a bare `Deck`. So this presses
    /// [`held`], [`gain_key`] and [`opacity_key`] by hand, in the order
    /// `answered`'s `Trim`/`Fader` arm calls them, rather than calling `answered`
    /// itself.
    /// `holding_reads_the_addressed_decks_own_state_and_never_a_strip_that_predates_it`,
    /// below, presses [`holding`] — `answered`'s neighbour and the other function
    /// ADR-0333 named — directly, because [`holding`] takes only `&Deck` and needs
    /// no window at all. Between the two, every function the grammar's mix answers
    /// call on a deck is pressed by something; only `answered`'s own dispatch —
    /// that it calls them in this order, on the deck the address named — is still
    /// asserted by hand here rather than by entering the function that actually
    /// does it.
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

        // **No two slots alike, in all three values.** An answer read off the
        // wrong slot is then a wrong answer rather than the right one by luck,
        // and the blends are seeded so the selected deck's next mode is not
        // where it already was.
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

        // **[`held`] is what stands between an event handler and an abort**,
        // and this is the device half of that sentence: `Deck::gain` indexes
        // its slots, so the thing the guard refuses is a real panic and not a
        // supposed one. A panic here would take the process with it rather
        // than unwinding into a message — see the module documentation — which
        // is why the arms ask before they read.
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

    /// `holding` reads the addressed deck's own state, and never a strip that
    /// predates it — the two functions'
    /// `the_grammars_mix_answers_act_on_the_addressed_deck_and_read_it_off_the_deck`
    /// used to hold as a claim about this file's text, read as a claim about what
    /// runs.
    ///
    /// [`holding`] hands the console the three states a mixer strip cycles —
    /// [`tally`], [`blend_mode`] and [`masked`] applied to
    /// `Deck::requested_residency`, `Deck::blend` and `Deck::mask` — plus the angle
    /// carried through unchanged. A text scan can only say those calls are *spelled
    /// somewhere above the tests*; this presses [`holding`] itself and checks the
    /// *values* it hands back, against a slot moved after a strip had already
    /// copied its old ones — the same staleness
    /// `a_mix_key_moves_the_deck_operator_selected_and_leaves_the_others_alone`
    /// presses [`gain_key`] against, above.
    ///
    /// Nothing here reaches for a strip because nothing here has one to reach for.
    /// [`holding`]'s only parameters are `&Deck` and a slot number, and
    /// `karakuri-engine` does not depend on `karakuri-console` (ADR-0156): there is
    /// no `view::Strip` in scope for a function with this signature to name, by
    /// accident or otherwise. That half of the old claim is a fact about the crate
    /// graph, settled the day this file stopped being allowed to import the
    /// engine's own compositor into the console — not something either the old scan
    /// or this test has to hold at runtime. What is worth pressing is the other
    /// half: that the values [`holding`] reports are the ones on the deck now.
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

    /// Every slot is its own simulation of the one procedure, which is what keeps a
    /// mixer of four channels from being one picture drawn four times.
    ///
    /// [`slot_salt`] is the derivation and this asserts it where it lands: the salt
    /// is read back off the `Set` the deck actually built, which
    /// `Set::source_salts` exists for — *"a caller that assigned none finds out
    /// what it got"* — so a slot built at the wrong seed, or four slots built at
    /// one, fails here rather than in a picture only an operator with two channels
    /// up would ever notice. Read off the deck rather than by calling `slot_salt`
    /// again, which would be the test agreeing with itself about the one thing it
    /// checks.
    ///
    /// It is deck A's salt that is named against a constant, because that one is a
    /// claim about a *value* — 7 is what `karakuri-cli`'s own tests use, so this
    /// program's picture looks like theirs. The rest is a claim about distinctness,
    /// and distinctness is what is asserted.
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

    /// A parked deck is reachable by running this window, and both of its
    /// residencies reach the strips.
    ///
    ///
    /// [ADR-0190](../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md)
    /// drew a chip that rolls while a request is outstanding and `tests/parked.rs`
    /// asserts every pixel of it — from strips written by hand. This is the other
    /// question, and it was the one answered `no`: whether the engine can put this
    /// panel in that state at all. A `Deck` grants every residency it is asked for
    /// until something governs, so before [`Engine::ask_to_prime`] the two halves
    /// of the pair could not disagree here however long anybody ran the program,
    /// and the animation that is fully tested was unreachable in the one place a
    /// person would look at it.
    ///
    /// Every step is the product's: two Sets are built, `Deck::measure_slots`
    /// measures them with a real probe, the budget is set from what it measured,
    /// [`Deck::govern`] refuses, and [`mixer`] reads the two residencies back off
    /// the deck the way the frame does. The reason is asserted and not only the
    /// park, because three of the four reasons that satisfy `Deck::is_parked` mean
    /// this program forgot to do something — `Unmeasured` and `CommittedUnknown`
    /// are a probe that never ran, and `NoPrimingNeeded` is a closed-form Set that
    /// never needed warming. Only `NoHeadroom` is the budget refusing.
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
        // **The reference workload's pair rather than the shipped one**, and
        // that is the whole of what keeps this test about the budget: see
        // [`reference`]. `examples/star_vortex.kset`'s pair is closed-form, so
        // this deck built from it parks deck B for `NoPrimingNeeded` and the
        // assertion below would be reading a different refusal.
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

        // **Before the pass, and this is the deck this program opens with.**
        // `Deck::new` brings every slot up Live and [`Engine::new`] rests all
        // but deck A at Allocated, so the two residencies agree on every slot
        // and nothing is pending: a park is something the governor does below,
        // and nothing has asked for anything yet.
        let mut before = Vec::new();
        mixer(&engine.deck, &material, &mut before);
        // **`DECKS` rather than `SLOTS`**, which is the claim rather than the
        // definition: a strip is a slot, the bay draws four tracks whatever
        // the deck has (ADR-0178), and what is being asserted is that every
        // track this console lays out has a channel in it. Held against
        // `SLOTS` it would be `Deck::new`'s argument compared with
        // `Deck::slot_count`, which is the engine agreeing with itself.
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
                .all(|(slot, strip)| slot == ON_AIR || strip.tally == view::Tally::Allocated),
            "a slot nobody asked anything of opened somewhere other than allocated, so this \
         deck steps and folds material the operator never called for: {:?}",
            before.iter().map(|strip| strip.tally).collect::<Vec<_>>()
        );
        assert_eq!(
            engine.deck.live_slots(),
            1,
            "more than one slot is live before anything was asked for, so the picture is a \
         sum of simulations nobody chose"
        );
        assert!(
            before.iter().all(|strip| strip.pending().is_none()),
            "a strip was pending before anything had asked for anything"
        );

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

        // **The slots nobody asked anything of come back `OffAir`**, which is
        // the governor saying it was not asked about them — and it is a
        // different word from `NoHeadroom` on purpose: a park stands and is
        // reconsidered every pass, and a slot at rest carries no request to
        // stand. These are the ones the legend counts off this report rather
        // than naming.
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

        // **And the deck's committed cost is deck A's alone.** That is what
        // keeps the arithmetic in `ask_to_prime` the arithmetic it was with
        // two slots — `committed_ms` sums the **Live** slots, and three more
        // allocated ones add nothing to it — and it is also the answer to
        // whether four slots of the reference workload fit the frame budget:
        // they are not being asked to.
        assert!(
            !governed.over_budget,
            "one live slot is already over the budget this program set — {governed}"
        );
        // **Against what deck A is *budgeted* on rather than what it was
        // measured at**, which since
        // [ADR-0296](../../../docs/adr/0296-the-governor-budgets-on-the-estimate-where-it-answers-and-on-the-measurement-where-it-does-not.md)
        // are two different numbers: the governor spends the estimate where a
        // Set has one and the measurement where it does not, and this test is
        // about *which slots* are in the sum rather than about which reading
        // each of them contributed. Taking it off the decision is also what
        // stops this assertion passing by arithmetic coincidence the day the
        // estimate stops arriving — the number it compares against moves with
        // the same rule the sum is built from.
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

        // **What four of these would cost if they were all Live, printed
        // rather than asserted.** It is this machine's number and a threshold
        // on it would be a test that passes here and fails on the next machine
        // — ADR-0191 measured this same Set at 3.9 ms and at 9.8 ms in two
        // runs of one program, and `DEFAULT_COMPUTE_BUDGET_MS` is 16.7. So the
        // measurement is taken where it can be taken and reported;
        // `cargo test -p karakuri -- --nocapture` is where to read it.
        // P-0095: it carries how it was taken — `Deck::measure_slots`, one
        // `Probe` for the deck, at the probe's own resolution rather than at
        // `CANVAS`.
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

        // And the panel is live for as long as they disagree **and the bay
        // the chip is in is laid out**, which is the declaration
        // `tests/parked.rs` asserts against strips and folds of its own. The
        // panel here is this program's own, unfolded, which is the arrangement
        // this window opens on.
        let mut readout = Readout::new(1440.0, 900.0);
        readout.view.mixer = strips;
        assert_eq!(
            readout.view.animating(readout.panel.layout()),
            Some(view::ROLL_STALENESS),
            "a parked deck declared no staleness, so the roll never gets a frame"
        );
    }
}
