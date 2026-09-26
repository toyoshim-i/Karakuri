use super::common::*;

mod gpu {
    use super::*;

    /// Verifies saving active deck state creates a valid Set file containing current
    /// geometry capacity and salt, which can be reloaded accurately (ADR-0221).
    #[test]
    fn a_save_writes_what_the_deck_is_playing_as_a_set_file_that_loads_back() {
        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            wgpu::TextureFormat::Rgba8Unorm,
            egui_wgpu::RendererOptions::default(),
        );
        let mut panel = Panel::new(1440.0, 900.0);
        view::rearrange(&mut panel, CANVAS);
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

        let root = scratch_dir("kept");
        let mut keeping = keeping();
        keeping.playing = Playing::at_launch(&engine.placed, engine.deck.slot_count());
        // Deck B, so a hard-coded slot 0 fails here.
        keeping.save_set(
            &engine,
            &root,
            Asked::Operator,
            ASKED_TO_PRIME,
            Some("kept01".into()),
            None,
        );
        assert_eq!(keeping.in_flight, 1, "the save was never started");

        // The save is on a thread of its own and no frame waits for it; this is
        // what the end of a run does — see [`Keeping::awaited_saves`].
        keeping.awaited_saves();
        assert_eq!(keeping.in_flight, 0, "the save never came back");

        let store = Store::open(&root).expect("the store the save made");
        let loaded = setfile::load(&store, "kept01").expect("the file it wrote");
        assert_eq!(
            loaded.srcs.len(),
            engine.placed.len(),
            "the file does not name every node the slot is running"
        );
        // Verify saved capacity and salt reflect running engine slot values.
        assert_eq!(
            loaded.capacities,
            vec![Some(engine.capacity)],
            "the capacity written down is not the one the deck is drawing"
        );
        assert_eq!(
            loaded.salts,
            vec![Some(slot_salt(ASKED_TO_PRIME))],
            "the salt written down is not the one this slot is salted with"
        );

        std::fs::remove_dir_all(&root).expect("clean up");
    }

    /// Verifies swap outcome reports and staging lane states match across live execution,
    /// handling both successful runs and budget overload stops consistently (ADR-0313, ADR-0316).
    #[test]
    fn the_swap_report_says_what_the_lane_says() {
        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            wgpu::TextureFormat::Rgba8Unorm,
            egui_wgpu::RendererOptions::default(),
        );
        let mut panel = Panel::new(1440.0, 900.0);
        view::rearrange(&mut panel, CANVAS);

        // Isolated working copies allow live mutation of .kir sources.
        let root = scratch_dir("swapped");
        let (_, running) =
            working_copies(&root, &shipped(), SLOTS).expect("the copies this deck runs from");
        let store = std::sync::Arc::new(Store::open(&root).expect("store"));
        let (built_tx, built) = std::sync::mpsc::channel();
        // Shared slot handle between engine watchers and MCP server.
        let pointing = mcp::Slots::of(
            running
                .iter()
                .map(|pair| (pair.l1.clone(), vec![pair.l4.clone()]))
                .collect(),
        );
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            HEADLESS_PICTURE_FORMAT,
            &running,
            panel.layout(),
            1.0,
            Some((std::sync::Arc::clone(&store), built_tx)),
            None,
            pointing.clone(),
        );

        let reporter = mcp::serve(
            0,
            pointing,
            root.clone(),
            true,
            Opening::closed(),
            karakuri_environment::SlotPolicies::default(),
        )
        .expect("an ephemeral port");
        let port = reporter.port();

        let mut keeping = keeping();
        keeping.built = built;
        keeping.playing = Playing::at_launch(&engine.placed, engine.deck.slot_count());
        let was = keeping.playing.at(ON_AIR).expect("seeded at launch")[0].hash;
        keeping.mcp = Some(reporter);

        // An edit the watcher will pick up: the same procedure, one comment
        // longer, so it compiles and its bytes are different.
        let l1 = &running[ON_AIR].l1;
        let edited = format!(
            "{}\n// an edit\n",
            std::fs::read_to_string(l1).expect("read")
        );
        std::fs::write(l1, edited).expect("write");

        // The worker polls every hundred milliseconds and wants two polls of
        // quiet before it builds, then compiles; the swap lands at a frame
        // boundary, which is `begin_frame`.
        let mut rows: Vec<view::Candidate> = Vec::new();
        // The transport's health capsule, off the same drain, so this test
        // reads both halves of what one verdict writes.
        let mut health: Option<view::Stage> = None;
        // Staging applies rebuild diffs and updates active playback (ADR-0326).
        let mut landed = false;
        let deadline = Instant::now() + Duration::from_secs(30);
        while !landed && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
            drop(engine.deck.begin_frame(&gpu.device, &gpu.queue));
            landed |= staging(
                &mut engine.deck,
                &mut keeping,
                &engine.aimed,
                &mut rows,
                &mut health,
            );
        }
        assert!(
            landed,
            "nothing was built in 30s — the watcher never saw the edit"
        );
        // Query deck overload state directly to validate against surface reports.
        let stopped = engine.deck.overloaded(EngineSlot(ON_AIR as u8));
        let row = rows.iter().find(|row| row.deck == ON_AIR);

        // Query swap outcome from MCP server to verify reported verdict.
        let (failed, said) = call(port, "swap_outcome", serde_json::json!({}));
        assert!(!failed, "swap_outcome refused: {said}");
        assert!(
            said.contains(&format!("slot {ON_AIR}:")),
            "the server was told about a slot this test did not rebuild: {said}"
        );
        // The swap itself, which happened either way and is what `took` above
        // is an entry of.
        assert!(
            said.contains("swapped in"),
            "the swap reached the lane and did not reach the server: {said}"
        );

        // Asserts transport health capsule displays the swap verdict drained from the device.
        match stopped {
            // Candidate met budget: row cleared from staging and capsule indicates landed swap.
            false => {
                assert!(
                    row.is_none(),
                    "the build held the budget and its row is still on the lane, \
                 which is a lane that never empties: {:?}",
                    row.map(|row| row.stage)
                );
                assert_eq!(
                    health,
                    Some(view::Stage::Landed),
                    "the lane was told what the build did and the transport row was not"
                );
                assert!(
                    said.contains("held the budget") || said.contains("was not judged"),
                    "the lane settled this build and the server was told something \
                 else about it: {said}"
                );
            }
            // Candidate stopped due to budget limit (ADR-0316).
            true => {
                let row = row.expect(
                    "the slot was stopped for cost and the lane drew no row, which is \
                 the disagreement this lane exists to say",
                );
                assert_eq!(
                    row.stage,
                    view::Stage::Overloaded,
                    "the slot is stopped and its row says otherwise"
                );
                assert_eq!(
                    health,
                    Some(view::Stage::Overloaded),
                    "the lane says the slot is stopped and the transport row does not"
                );
                assert!(
                    said.contains("is overloaded"),
                    "the lane says the slot is stopped and the server was told \
                 something else: {said}"
                );
                assert!(
                    said.contains(&row.name),
                    "the server was told about a build the row was not: {said} \
                 against `{}`",
                    row.name
                );
            }
        }

        // Verifies still rendering is applied only to the failing slot following budget overruns.
        let mut cells = [false; view::DECKS];
        cells[ON_AIR] = stopped;
        assert_eq!(
            stopped_slots(&engine.deck),
            cells,
            "the cells do not say what the deck says about which slot is stopped"
        );

        // And the slot is playing the new bytes, so a save would write them —
        // taken up inside `staging`, where the diff that made the rows above
        // was taken.
        let now = keeping.playing.at(ON_AIR).expect("still addressable")[0].hash;
        assert_ne!(
            was, now,
            "the build landed and the slot is still addressed as what it launched with"
        );
        // Asserts staging row reports only the specific node modified in the edit (ADR-0326).
        if let Some(row) = rows.iter().find(|row| row.deck == ON_AIR) {
            assert_eq!(
                row.addr, "L1:0",
                "the edit was to the L1 and the row says `{}`",
                row.addr
            );
            assert_eq!(
                rows.iter().filter(|row| row.deck == ON_AIR).count(),
                1,
                "one file changed and the lane drew a row for every node of the stack"
            );
        }

        std::fs::remove_dir_all(&root).expect("clean up");
    }

    /// Verifies inspector pane layout against a real Set, ensuring node resolution
    /// correctly disambiguates author parameters from built-in camera parameters (ADR-0318).
    #[test]
    fn a_pane_reads_a_running_set() {
        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            wgpu::TextureFormat::Rgba8Unorm,
            egui_wgpu::RendererOptions::default(),
        );
        let mut panel = Panel::new(1440.0, 900.0);
        view::rearrange(&mut panel, CANVAS);
        let sources = shipped();
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
        // One name per slot, which is what `Gfx::material` is: every slot
        // opens on the same pair, and a load is what makes them differ.
        let material = vec![sources.material(); engine.deck.slot_count()];
        let mut panes = Vec::new();
        // Uses lines are populated from slot aims (ADR-0329).
        inspector(
            &engine.deck,
            &material,
            &engine.aimed,
            view::PANE_DECKS,
            &mut panes,
        );

        // One pane per slot, up to the panes the arrangement has.
        assert_eq!(panes.len(), view::PANES);
        let pane = &panes[0];
        assert_eq!(pane.deck, 0);
        assert_eq!(pane.material, material[0]);
        // `Set::build` takes an L1 and an L4 and no merge, so the Set
        // overdraws — `Set::layering` read rather than assumed.
        assert!(
            !pane.composite,
            "the pair builds with no L5, so there is nothing to fold"
        );

        // Asserts deck build chips derive capacity ranges directly from material declarations (ADR-0328).
        let set = engine.deck.slot(karakuri_engine::DeckSlot(0)).set();
        let aimed = pane
            .aimed
            .as_ref()
            .expect("a slot with a geometry has a capacity and a salt");
        assert_eq!(
            aimed.capacity,
            set.source_capacities()[0],
            "the chip reads a capacity the first geometry is not running at"
        );
        let declared = set.declared_capacities();
        assert_eq!(
            aimed.stated,
            aimed.capacity != declared[0][2],
            "lit and unlit disagree with whether the slot is on its material's own default"
        );
        assert!(
            !aimed.capacities.is_empty(),
            "one geometry declaring a range gave a chip with nothing to step to"
        );
        assert!(
            aimed.capacities.windows(2).all(|two| two[0] < two[1]),
            "the ladder is not ascending, so the step is not a step: {:?}",
            aimed.capacities
        );
        for rung in &aimed.capacities {
            assert!(
                rung.is_power_of_two(),
                "{rung} is on the ladder and is not a power of two"
            );
            for at in declared {
                assert!(
                    (at[0]..=at[1]).contains(rung),
                    "the chip offers {rung} and a geometry declares [{}, {}], so the build this \
                 press asks for is one the engine refuses by name",
                    at[0],
                    at[1]
                );
            }
        }
        // Verifies capacity stepping operates smoothly even when active values are non-power-of-two defaults.
        assert!(
            (declared[0][0]..=declared[0][1]).contains(&aimed.capacity),
            "the pair runs at {} and its geometry declares [{}, {}]",
            aimed.capacity,
            declared[0][0],
            declared[0][1]
        );
        let above = aimed
            .capacities
            .iter()
            .find(|rung| **rung > aimed.capacity)
            .or_else(|| aimed.capacities.first());
        assert!(
            above.is_some(),
            "a slot running at {} has nowhere to step inside {:?}",
            aimed.capacity,
            aimed.capacities
        );
        // Verify offered salt is derived by the engine from the active slot salt (P-0092).
        let running = set.source_salts()[0];
        assert_eq!(
            aimed.salt,
            karakuri_engine::set::derived_salt(running, 1),
            "the salt offered is not the next in this slot's sequence"
        );
        assert_ne!(
            aimed.salt, running,
            "a press would ask for the salt the slot is already on, which is a rebuild that \
         changes no pixel"
        );

        // The L1 is its own group and the renderers fold into one, which is
        // the mock's `L1:0` beside its bare `L4`.
        let addrs: Vec<&str> = pane.nodes.iter().map(|n| n.addr.as_str()).collect();
        assert!(addrs.contains(&"L1:0"), "{addrs:?}");
        assert!(addrs.contains(&"L4"), "{addrs:?}");

        // Unconfigured nodes default to manual authority.
        for node in &pane.nodes {
            assert_eq!(
                node.authority.map(|a| a.level),
                Some(karakuri_operation::Authority::Manual),
                "{} reads something other than the default nobody has changed",
                node.addr
            );
            // Authority chip retains underlying node identity (ADR-0286).
            assert!(
                node.authority.is_some(),
                "{} draws a chip with no node behind it",
                node.addr
            );
        }

        // One chip per renderer, and none of them live: `Input::live` is
        // *"empty of meaning under Overdraw"*, so it is not passed on.
        let renderers = pane
            .nodes
            .iter()
            .find(|n| n.addr == "L4")
            .expect("the renderers group");
        assert_eq!(renderers.renderers.len(), 1);
        assert!(
            !renderers.renderers[0].live,
            "a deck that overdraws has no live renderer to mark"
        );

        // Verify published controls map to nodes with contiguous ordinals across groups.
        let published = engine
            .deck
            .slot(karakuri_engine::DeckSlot(0))
            .set()
            .published()
            .len();
        assert!(published > 0, "the pair publishes what it declares");
        // Procedures declaring no inputs omit uses lines.
        assert!(
            pane.nodes.iter().all(|node| node.uses.is_empty()),
            "a node of the launch pair drew a `uses` line, and the pair declares no input"
        );

        // Full interface is published with numbered rows (ADR-0329).
        let mut ords: Vec<usize> = pane
            .nodes
            .iter()
            .flat_map(|node| node.params.iter().filter_map(|param| param.ord))
            .collect();
        assert_eq!(
            ords.len(),
            pane.nodes
                .iter()
                .map(|node| node.params.len())
                .sum::<usize>(),
            "a row of this pane carries no interface position, and nothing has narrowed this deck"
        );
        ords.sort_unstable();
        assert_eq!(
            ords,
            (1..=published).collect::<Vec<_>>(),
            "a published control lost its group, so a row this Set publishes is not drawn"
        );
    }
}
