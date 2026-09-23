use super::*;

mod aiming;
mod keeping;
mod telemetry;

pub(crate) use aiming::*;
pub(crate) use keeping::*;
pub(crate) use telemetry::*;

/// Initial look settings (ACES tonemapping, 1.0 exposure, 1.0 white point) configured at startup (ADR-0037).
pub(crate) const LOOK: Look = Look {
    op: TonemapOp::Aces,
    exposure: 1.0,
    white_point: 1.0,
};

/// The engine behind the Program bay: a deck of [`SLOTS`] Sets, the present
/// pass, and the two textures it lands in.
///
/// Scaffolding still in what it is wired to — no audio, no MIDI, no store, no
/// arguments — and a watcher on each slot, which is the one thing here that is
/// not the shortest path to texels and is there because the Staging lane's rows
/// are verdicts on builds ([`watched`]). What `karakuri-cli` does around this
/// is a program; what is here is the shortest path from two `.kir` files to
/// texels — and now back again, which is what a watched slot is.
///
/// The deck is full, and the slots are channels rather than exhibits. It has
/// every slot a `Deck` can hold, because a strip is a slot and a mixer is its
/// channels; what is *in* them is this program's one pair at four salts, which
/// is what a slot nobody has loaded anything into holds ([`Engine::new`]).
/// [`ON_AIR`] is Live and is the whole of the picture. Every other slot rests
/// at `Residency::Allocated` — contributing nothing to the mix, and stepped and
/// drawn into its own cell all the same — and [`ASKED_TO_PRIME`] is
/// additionally asked to warm up and parked by the budget in
/// [`Engine::ask_to_prime`], which is what puts a pending request on this panel
/// for the mixer's tally to draw. The three cost a step and a draw each, and
/// none of it reaches the governor, which reads a per-Set cost. This sentence
/// has been wrong twice in the same direction — it said the three cost nothing
/// while they were being drawn, and *a draw each and no step* while they were
/// being stepped — so what it is now is the whole of a frame for every slot,
/// which is what
/// [ADR-0269](../../../docs/adr/0269-a-slot-that-is-drawn-is-stepped-and-a-preview-runs-at-the-rooms-tempo.md)
/// makes it. The number that goes with it is not one this file can carry: it is
/// `karakuri-engine`'s `tests/deck.rs`, which prints a deck of four against a
/// deck of one on the machine reading it.
///
/// All four preview cells are on, whatever the decks are doing. A cell is drawn
/// because there is a slot behind it ([`Engine::aim`]), and this deck is full,
/// so four cells show four slots' own material, all four of them running: deck
/// A stepping on air, deck B warming or parked, C and D warming with nobody
/// having asked. That is
/// [ADR-0258](../../../docs/adr/0258-the-look-comes-before-the-fader-so-a-cell-draws-every-slot-and-says-which-nothing-it-is.md)
/// met on this surface — the operator watches a candidate's cell to decide
/// whether it is worth a fader, and then raises the fader. It used to be gated
/// on `Residency::Live`, which left the three cells worth looking at dark; the
/// gap ADR-0241 named was this line.
pub(crate) struct Engine {
    pub(crate) deck: Deck,
    /// Elements per geometry, read off the L1's own `capacity` declaration rather
    /// than named here — see [`Engine::new`]. Kept because the reading
    /// [`Costs::say`] prints names it, and a workload figure that is not the one
    /// the run used is worse than none.
    pub(crate) capacity: u32,
    pub(crate) present: Present,
    /// The Program bay's picture.
    pub(crate) picture: Presented,
    /// The four deck preview cells, one per deck slot.
    pub(crate) previews: [Presented; DECKS],
    /// Cached bind groups for each slot view into the tone-mapping pipeline.
    pub(crate) slot_bind_groups: [Option<wgpu::BindGroup>; DECKS],
    /// Active tonemapping and exposure look configuration applied on present passes (P-0090, ADR-0192).
    pub(crate) look: Look,
    /// What the master chain is set to, and [`Engine::look`]'s twin at the other
    /// end of that chain.
    ///
    /// Held here for `look`'s reason exactly: a press becomes
    /// `Operation::SetChainParam`, `AddChainEffect` or `RemoveChainEffect`, which
    /// become one
    /// `Record::MasterChain`, which [`apply`] writes here; the frame loop puts it
    /// on the `Present` and the slots' uniforms are written from it. Nothing calls
    /// that setter behind the record's back, which is P-0090 on this value.
    ///
    /// A list where the out is a bare `f32` on the deck, and the two are apart for
    /// the reason their records are: the level at the chain's entry is ridden by a
    /// fader and the chain's slots are moved by a press
    /// ([ADR-0317](../../../docs/adr/0317-the-master-chain-is-three-fixed-passes-and-feedback-reads-either-cut.md),
    /// [ADR-0340](../../../docs/adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md)).
    ///
    /// A description and not the built chain, which is the one thing that changed
    /// when the chain became a list: `karakuri_engine::Present` holds the compiled
    /// slots and is still the only writer of them, and this is what a record says
    /// the chain should be. The frame loop puts one on the other where the two
    /// differ.
    pub(crate) chain: Vec<karakuri_engine::SlotSpec>,
    /// Where a chain is compiled and where the chain it replaces is freed.
    ///
    /// The frame loop asks this for a build when [`Engine::chain`] and
    /// `Present::chain_spec` differ, and installs what it finished at the frame
    /// boundary before `compose`. Nothing here compiles a shader, creates a
    /// pipeline or allocates a target on this thread
    /// ([ADR-0354](../../../docs/adr/0354-a-chain-is-compiled-on-a-thread-of-its-own-and-lands-at-a-frame-boundary.md)).
    pub(crate) chain_swap: karakuri_engine::ChainSwap,
    /// How many registrations have been freed, over both textures. The atlas leak
    /// this exists to prevent is invisible from outside: a resize that registers
    /// without freeing leaves a bind group per drag frame and nothing says so, so
    /// the count is kept and `mod gpu` asserts on it. It is the whole engine's
    /// tally rather than either texture's, which is why it lives here and is handed
    /// to [`Presented::fit`].
    pub(crate) freed: usize,
    /// Target aim definitions and watch channels for each deck slot, indexed by slot.
    pub(crate) aimed: Vec<Aiming>,
    /// The run's wiring — every edge a `wire_input` has written, for the whole run
    /// and not per slot.
    ///
    /// One list because `--edge` is one list: an edge names the node that declares
    /// the input and what its procedure calls it, and a Set that has not got that
    /// node passes it over where it is built. See [`rewired`].
    ///
    /// What a rebuild carries and what a save records, which is why it lives here
    /// rather than inside a watcher: [`Aiming::re_aim`] restates it to the worker
    /// and [`playing_values`] writes it into the file, and those are one list or
    /// they are two answers to what the run is wired with.
    ///
    /// It lives beside the aims rather than beside the saves, and that is what lets
    /// a press reach it: a rewiring writes this list and re-aims a slot, and both
    /// halves are here. It was `Keeping`'s until 2026-09-09, when the Inspector's
    /// `uses` line gave the list a second writer that is not a model's request —
    /// see `docs/adr/0329-…`.
    pub(crate) edges: Vec<karakuri_engine::set::Edge>,
    /// Where this deck says which files its slots are running, for the readers that
    /// are not on this thread — see [`Aiming::pointing`], which is a clone of this,
    /// and [`karakuri_mcp::Slots`].
    ///
    /// The engine keeps it so that the two readers ask one handle. The MCP server
    /// was handed the launch working copies and the landing on a row of the edit
    /// history built a second `Slots` of its own out of the aims (ADR-0308); the
    /// first went stale on the first library load and the second was the workaround
    /// for it. There is one now, this is it, and [`restored`] reads it rather than
    /// rebuilding one.
    pub(crate) pointing: karakuri_mcp::Slots,
    /// Every node the run launched with, in file order, with the bytes each one was
    /// compiled from — see [`karakuri_environment::compile::Placed`].
    ///
    /// One list for four slots, because the four files hold the same bytes.
    /// [`working_copies`] writes the one pair the command line settled into every
    /// slot, so a node's layer, its index, its address and its source are the same
    /// answer four times; the only per-slot difference is the *path*, which each
    /// watcher is given from `slots[slot]` and which no part of a saved node
    /// carries. A second compile per slot would be four answers to one question
    /// with a window between them — see [`Placed::source`], which is where that
    /// hazard is written.
    ///
    /// This is what [`Playing`] is seeded from, and it is the reason a deck can be
    /// saved on the first frame rather than only after something has been rebuilt.
    pub(crate) placed: Vec<karakuri_environment::compile::Placed>,
}

impl Engine {
    /// Sized from the arrangement rather than from the window, by the same two
    /// calls the frame aims with — see [`aims`]. The window this opens at gives the
    /// picture and deck A's cell their first rectangles, so no frame has to correct
    /// a guess and there is no second derivation here to drift from the one in
    /// [`Engine::aim`].
    // Eight, for [`watched`]'s reason: the last three are the run-wide handles
    // this constructor hands every watcher it makes, and each has a different
    // owner in [`main`].
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        // **What the present pass draws into, and what every texture here is
        // in.** Handed in rather than named here: it is the first sRGB format
        // the console's own surface offers, read off it once in
        // [`App::resumed`] — see [`crate::Gfx::picture_format`]. `mod gpu` has
        // no surface and names a headless stand-in.
        picture_format: wgpu::TextureFormat,
        slots: &[Sources],
        layout: &karakuri_layout::Layout,
        scale: f32,
        // **Where every watcher puts what it builds, and where it reports it.**
        // `None` is a harness with no store to write into, which is every test
        // under `mod gpu` below: nothing there saves, and a run that created a
        // store to draw four cells would be the side effect
        // `karakuri_environment::scratch` refuses for a `--render`.
        stored: Option<(std::sync::Arc<Store>, std::sync::mpsc::Sender<watch::Built>)>,
        // **The run's one edit history**, made and seeded in [`main`] and
        // handed to every watcher this makes — see [`watched`]. `None` is a
        // harness with no store, which is every test under `mod gpu` below, on
        // `stored`'s terms exactly.
        //
        // **One `Shared` for the run and not one per window.** A window remade
        // makes these watchers again, and a second `Snapshots` would have an
        // empty dedup memory: the first rebuild after a remake would file every
        // untouched procedure as a new version.
        snapshots: Option<history::Shared>,
        // **Where this deck says which files each of its slots is running**,
        // handed in rather than made here: [`main`] gives the same handle to
        // [`karakuri_mcp::serve`] and to this, so a model's
        // address and the file a watcher is polling are one answer — which is
        // exactly the arrangement the opening already has. It is not
        // `Option` and does not depend on `--mcp`, because the landing on a row
        // of the edit history reads it too ([`restored`]).
        pointing: karakuri_mcp::Slots,
    ) -> Engine {
        assert!(
            slots.len() == SLOTS,
            "a deck of {SLOTS} slots was handed {} pairs to run from",
            slots.len()
        );
        // **Parsed once and built [`SLOTS`] times, and that is a fact rather
        // than an assumption now.** Every entry in `slots` is a copy of the
        // one pair the command line settled ([`working_copies`]), so the four
        // files hold the same bytes at startup and one `Checked` is the same
        // answer four times. What is *not* the same is the path each slot's
        // watcher polls, which is the whole of what per-slot copies buy and is
        // read off `slots[slot]` in the loop below.
        // **Compiled through the sort every other surface compiles through**,
        // which is what this used to do by hand and is the whole of what a save
        // needed: `checked` gave back a `Checked` and dropped the bytes it read,
        // and a node's address is a function of exactly those bytes
        // ([`karakuri_environment::compile::Placed::source`]). Re-reading the
        // path later to hash it is the defect that function's own doc is
        // written against — between here and the first frame sit a device, four
        // `Set::build`s and, now, an MCP server.
        //
        // **Once, for slot 0, and used by all four.** See [`Engine::placed`].
        let (material, placed) = karakuri_environment::compile::sort_slot(
            ON_AIR,
            &karakuri_environment::compile::Named::bare(&slots[ON_AIR].l1),
            &[karakuri_environment::compile::Named::bare(
                &slots[ON_AIR].l4,
            )],
        );
        let l1 = material
            .l1s
            .first()
            .expect("the launch pair declares a geometry")
            .clone();
        let l4 = material
            .l4s
            .first()
            .expect("the launch pair declares a renderer")
            .clone();
        let capacity = capacity_of(&l1);
        // **The same material in every slot, at its own salt and in its own
        // file.** A slot cannot hold *nothing*: `HotSwap::new` takes a live
        // `Set` and `Deck::new` takes one `HotSwap` per slot, so an empty slot
        // is not a state this engine has and the nearest thing to it is a slot
        // holding material nobody has asked for. What this program has to give
        // them is one pair — [`Sources`] is the whole command line — so each
        // gets it at its own salt ([`slot_salt`]): four slots of one procedure
        // at four seeds are four simulations, and four slots at one seed would
        // be one picture drawn four times, which is not a mixer either.
        // Building three of them from other `.kir` files would be this program
        // choosing material for the operator, which is the library's job and
        // not a constructor's
        // ([ADR-0228](../../../docs/adr/0228-a-library-load-re-points-the-slots-source-and-never-installs-a-set.md)).
        //
        // **The salt is no longer the only difference, and that is the fix.**
        // Each slot runs from its own copy of that pair ([`working_copies`]),
        // so the four are the same *material* and four different *files* — an
        // edit reaches the deck whose file it is.
        let built = |salt| {
            Set::build(&gpu.device, &gpu.queue, &l1, &l4, capacity, salt)
                .expect("the pair builds a Set")
        };
        // **A watcher per slot, over that slot's own two files**, which is
        // `karakuri-cli`'s wiring and not a second one — one `HotSwap::new`
        // over a `watch::Watch`, at the engine's own default budget.
        //
        // **`slots[slot]` and not one pair repeated**, which is the sentence
        // this comment used to be the other way round. It said every slot was
        // spelled with the same pair and quoted `watch`'s *"Two slots given
        // the same files both rebuild, which is right: the same edit reached
        // both of them"* — true of that module, and the wrong thing for this
        // program to be doing, because it made one save rebuild four slots and
        // fill the Staging lane with three rows that can never leave (a parked
        // slot's trial is frozen, so it reaches no verdict). Each watcher now
        // polls the copy made for its own slot, and no watcher can see
        // another's file at all, which is what `watch`'s *"a slot is the unit
        // that gets replaced"* asks for.
        //
        // **This is the Staging lane's producer**, and it is the whole of what
        // it took. `HotSwap::fixed` keeps a `Receiver` whose `Sender` was
        // dropped at construction, so nothing is ever installed and no
        // `swap::Event` of any variant is emitted — which is why the lane drew
        // its empty state and could reach no other. Nothing about the frame
        // path changed: `install_if_ready` polls the same channel with
        // `try_recv` either way, and everything a rebuild costs — the file
        // read, the four validation stages, the compile and `Set::build` — is
        // on the worker thread this spawns (P-0091).
        //
        // **In slot order, and the loop is the whole of what four slots
        // took**: a `Vec` of `HotSwap` is what `Deck::new` has always taken,
        // and `Engine::aimed` is documented as being in the same order the
        // strips and the preview cells are.
        let mut swaps = Vec::with_capacity(SLOTS);
        let mut aimed = Vec::with_capacity(SLOTS);
        for (slot, running) in slots.iter().enumerate().take(SLOTS) {
            let salt = slot_salt(slot);
            let (swap, aim) = watched(
                gpu,
                running,
                built(salt),
                slot,
                salt,
                stored
                    .as_ref()
                    .map(|(store, tx)| (std::sync::Arc::clone(store), tx.clone())),
                pointing.clone(),
                snapshots.clone(),
            );
            swaps.push(swap);
            aimed.push(aim);
        }
        let mut deck = Deck::new(&gpu.device, swaps, CANVAS.0, CANVAS.1);
        // **Every slot but deck A opens muted.**
        //
        // `Deck::new` brings every slot up Live, and that is right for a deck
        // built to play what is in it. This deck is built full rather than
        // built to play one, so leaving all four unmuted would put four
        // simulations summed under `Blend::Add` at unity at startup.
        // Slots other than `ON_AIR` start muted, so only deck A is on air in the
        // composite mix initially, while the remaining slots can be brought in by unmuting.
        for slot in 0..SLOTS {
            if slot != ON_AIR {
                deck.set_mute(EngineSlot(slot as u8), true);
            }
        }
        // **The meters are on, and that is a decision rather than a default.**
        // Five of the six things a mixer strip shows are settings the deck was
        // told; the meter is the only one that is a *measurement*, so with it
        // off this bay would draw five readouts that never move beside a well
        // that is always empty — which is the scaffolding-that-looks-finished
        // this panel refuses, read from the other side. It costs a pipeline,
        // two buffers and a ring of staging buffers per slot, allocated here
        // and never on the render thread, which is the same terms `Deck::new`
        // above is on; there are [`SLOTS`] slots, so it is four of each. The
        // three that are not Live report no level — `Deck::level` is `None`
        // for a slot that is not being drawn — which is the meter saying what
        // it measured rather than a strip with a gap in it.
        deck.enable_meters(&gpu.device);
        let present = Present::new(&gpu.device, picture_format, CANVAS.0, CANVAS.1);
        let (picture_at, preview_ats) = aims(layout, present.size());
        let picture = Presented::new(
            gpu,
            renderer,
            "program view",
            picture_format,
            picture_at,
            scale,
        );
        let previews = [
            Presented::new(
                gpu,
                renderer,
                "deck A preview",
                picture_format,
                preview_ats.and_then(|c| c.first().copied()),
                scale,
            ),
            Presented::new(
                gpu,
                renderer,
                "deck B preview",
                picture_format,
                preview_ats.and_then(|c| c.get(1).copied()),
                scale,
            ),
            Presented::new(
                gpu,
                renderer,
                "deck C preview",
                picture_format,
                preview_ats.and_then(|c| c.get(2).copied()),
                scale,
            ),
            Presented::new(
                gpu,
                renderer,
                "deck D preview",
                picture_format,
                preview_ats.and_then(|c| c.get(3).copied()),
                scale,
            ),
        ];
        let slot_bind_groups = std::array::from_fn(|slot| {
            deck.slot_view(EngineSlot(slot as u8))
                .map(|view| present.create_bind_group_for(&gpu.device, view))
        });
        Engine {
            deck,
            // **Nothing wired by hand yet**, which is the state a run begins
            // in: the launch aims carry whatever `--edge` said, and this is the
            // list a rewiring writes and every later re-aim restates.
            edges: Vec::new(),
            capacity,
            present,
            picture,
            previews,
            slot_bind_groups,
            look: LOOK,
            // **Nothing in it at all**, which is the default chain: with no
            // slot the mix writes straight into the target the present pass
            // reads, so the frame this program opens on is the frame it drew
            // before the chain existed — bit for bit and for free.
            chain: Vec::new(),
            chain_swap: karakuri_engine::ChainSwap::new(&gpu.device, &gpu.queue),
            freed: 0,
            aimed,
            pointing,
            placed,
        }
    }

    /// Ask deck B to warm up, and let the budget answer. The one governor pass this
    /// program makes, taken at startup where the stall it costs is free, and the
    /// whole of why a strip on this panel can read one residency and have been
    /// asked for another.
    ///
    /// The other two slots are not in this, and that is the change. Deck B used to
    /// be the only other slot there was, so *the deck has a second slot* and *the
    /// panel can show a park* were one sentence; they are two now. C and D rest at
    /// `Residency::Allocated` — [`Engine::new`] says why — were asked for nothing,
    /// and come back from the pass as `Reason::OffAir`, which is the governor
    /// reporting that it was not asked about them. They cost the arithmetic below
    /// nothing: `committed_ms` is the sum over Live slots and deck A is the only
    /// one, so this sets the same budget it set with two slots, off the same
    /// measurement, for the same reason.
    ///
    /// It is `karakuri-cli`'s order rather than a second one: measure every slot
    /// before anything is decided about any of them, ask through
    /// [`Deck::set_residency`], and call [`Deck::govern`], which is the only thing
    /// in the engine that writes an *effective* residency. `set_residency` writes
    /// the request and grants it, so a harness that never governs has a deck whose
    /// two residencies agree on every slot and every frame — which is what this
    /// file was, and is why the roll ADR-0190 drew was tested and unreachable. The
    /// report comes back whole for the same reason `karakuri-cli`'s
    /// `report_governing` prints one: nothing is printed in the engine, so what an
    /// operator reads and what a test asserts are the same values.
    ///
    /// # The budget is what moves, and it is moved from what was measured
    ///
    /// A deck starts on `governor::DEFAULT_COMPUTE_BUDGET_MS` — one 60 Hz frame of
    /// measured per-Set cost — and what one of these Sets measures at is this
    /// machine's business rather than anything this file can know. So the budget is
    /// set from the measurement that was just taken: what deck A is already
    /// committed to, plus half of what warming deck B was measured at. Half of a
    /// cost is not that cost, so the request cannot fit — the refusal is arithmetic
    /// on every machine rather than on the ones where the numbers happen to come
    /// out.
    ///
    /// It used to be an eighth of it, and the change is ADR-0269's. The governor
    /// could once admit a slot at one step in `SLOWEST_PRIME_ONE_IN` frames and
    /// charge `cost / n` for it, so parking a request meant leaving headroom under
    /// `cost / 8`. There is no rate left to undercut: a drawn slot steps every
    /// frame, every slot is drawn, and a request either fits at its whole measured
    /// cost or is parked.
    ///
    /// A number computed from the measurement rather than a constant, because a
    /// constant is the fixture the product cannot produce (`docs/contributing.md`
    /// §3, *a check you have not watched fail is guessing*, read from the other
    /// side): a budget typed in here parks the request on this machine and admits
    /// it on a faster one, and a *measurement* typed in —
    /// `HotSwap::set_measured_cost` is public and would take one — is this file
    /// writing down the number the probe exists to take.
    ///
    /// Nothing here writes a residency, a strip or a park. The deck is asked and
    /// the governor answers; [`mixer`] reads both residencies back off the deck the
    /// way it reads the gain, and `view::Strip::pending` derives the disagreement.
    /// `Deck::is_parked` is not called in this file at all outside `mod gpu`.
    ///
    /// Where a measurement is missing the budget is left where it was, and the
    /// Startup initialization for the engine before the first frame.
    ///
    /// Measures and estimates all cold slots so their costs are known,
    /// and runs the initial governor pass against normal budget.
    /// Slots other than ON_AIR remain cleanly resting at `Residency::Allocated`.
    pub(crate) fn startup(&mut self, gpu: &Gpu) -> Report {
        self.deck.measure_slots(&gpu.device, &gpu.queue);
        self.deck.estimate_slots(&gpu.device, &gpu.queue);
        self.deck.govern()
    }

    /// governor parks the request anyway for a different and more serious reason —
    /// an unmeasured Live slot means the committed cost is unknown, which suspends
    /// priming wholesale. The caller prints the reason it got rather than the one
    /// this comment expects.
    #[allow(dead_code)]
    pub(crate) fn ask_to_prime(&mut self, gpu: &Gpu) -> Report {
        // **Before the first frame, and this is the only place it can be.**
        // Measuring means stepping and ends in a rewind, so `measure_slots`
        // skips any slot whose Set has already run — a deck measured late
        // stays unbudgetable rather than losing what it has simulated.
        self.deck.measure_slots(&gpu.device, &gpu.queue);
        // **And the estimate beside the measurement**, on the same terms: two
        // draws per cold slot, before the first frame and never on the render
        // thread. The governor budgets on this where it answers and on the
        // measurement where it does not
        // ([ADR-0296](../../../docs/adr/0296-the-governor-budgets-on-the-estimate-where-it-answers-and-on-the-measurement-where-it-does-not.md)),
        // and it is taken at the deck's own size, so a deck on a small output
        // stops being judged against a frame nobody is drawing.
        //
        // **A startup step and only that.** Every Set loaded afterwards
        // arrives with its own estimate, taken on the build worker at the
        // output's size and installed with the Set, and a resize re-reads what
        // a slot holds rather than dropping it
        // ([ADR-0356](../../../docs/adr/0356-the-worker-estimates-what-it-built-and-an-estimate-is-a-fit-rather-than-a-number-at-one-size.md)).
        self.deck.estimate_slots(&gpu.device, &gpu.queue);
        self.deck
            .set_residency(EngineSlot(ASKED_TO_PRIME as u8), Residency::Priming);
        // Compute budgeted cost per slot using build estimate if available, falling back to measurement (ADR-0296, ADR-0303).
        let budgeted = |slot: &HotSwap| {
            slot.estimated_cost()
                .and_then(Estimate::ms)
                .or_else(|| slot.measured_cost().map(|cost| cost.ms))
        };
        if let (Some(committed), Some(warming)) = (
            budgeted(self.deck.slot(EngineSlot(ON_AIR as u8))),
            budgeted(self.deck.slot(EngineSlot(ASKED_TO_PRIME as u8))),
        ) {
            self.deck.set_compute_budget_ms(committed + warming / 2.0);
        }
        self.deck.govern()
    }

    /// Aim every sink at its own rectangle, and hand back what the console should
    /// draw in each — the picture, and one entry per preview cell.
    ///
    /// # A cell is aimed because there is a slot behind it, and residency has
    /// nothing to do with it
    ///
    /// This read `Deck::preview` once and aimed the one preview sink at the cell of
    /// the deck the output was auditioning: the sinks all took the same composited
    /// frame, so a sink left in deck A's cell would have drawn deck C's material
    /// under the letter `A` the moment somebody auditioned C. ADR-0240 retired the
    /// audition — the picture is the master mix and every cell is its own deck's
    /// monitor — and the sinks stopped taking the composited frame: each cell is
    /// drawn from `Deck::slot_view` for the slot it is lettered for, which is a
    /// texture that cannot be of the wrong deck.
    ///
    /// Then it gated the aim on `Residency::Live`, and that was the defect this
    /// pass removes. The reason given was that an off-air slot "is not stepping and
    /// has nothing new in its view", which was true only because the engine refused
    /// to draw one. It is the exact case
    /// [ADR-0258](../../../docs/adr/0258-the-look-comes-before-the-fader-so-a-cell-draws-every-slot-and-says-which-nothing-it-is.md)
    /// exists for: an operator decides whether to put a candidate on air by
    /// watching its cell, and a cell that is dark until the candidate is already on
    /// air answers the question after it stops being asked. The deck draws every
    /// slot into its own target on every frame now, so there is something new in
    /// every view, every frame.
    ///
    /// What is left to decide is whether there is a slot at all, and that is
    /// `slot_bind_groups[slot]`: `Deck::slot_view` is `None` past `slot_count`, so
    /// a deck of fewer than [`DECKS`] slots leaves the surplus cells with nothing
    /// to sample. The aim asks the same question the draw asks, so the two cannot
    /// disagree — a cell aimed but not drawn would be a texture from an earlier
    /// frame held under a letter, and a cell drawn but not aimed is a pass into
    /// nothing.
    pub(crate) fn aim(
        &mut self,
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        layout: &karakuri_layout::Layout,
        scale: f32,
        // **The projector window's size, or `None` while it is closed.** The
        // second output, and the second half of what [`render_size`] is a
        // maximum over — handed in because the window is `Gfx`'s and this is
        // the engine.
        projector: Option<(u32, u32)>,
    ) -> (Option<Picture>, [Option<Picture>; DECKS]) {
        let (picture_at, preview_ats) = aims(layout, CANVAS);
        let picture = self
            .picture
            .aim(gpu, renderer, picture_at, scale, &mut self.freed);
        // Which cells have a slot behind them, read once for the frame: the
        // loop below both aims and reports off the same answer, and the draw in
        // `compose` reads the same `slot_bind_groups`.
        let behind: [bool; DECKS] =
            std::array::from_fn(|slot| self.slot_bind_groups[slot].is_some());
        let mut previews = [None; DECKS];
        let freed = &mut self.freed;
        for (slot, (sink, out)) in self
            .previews
            .iter_mut()
            .zip(previews.iter_mut())
            .enumerate()
        {
            let at = match behind[slot] {
                true => preview_ats.and_then(|cells| cells.get(slot).copied()),
                false => None,
            };
            let pic = sink.aim(gpu, renderer, at, scale, freed);
            if behind[slot] {
                *out = pic;
            }
        }
        // **What a slot's measurement is a measurement of**, told to the deck
        // here because this is the statement that knows it.
        //
        // This application has two resolutions and no third one: the output
        // size, which the mix is composited once at and which the picture and
        // every other sink is a resize of (ADR-0247), and the size of a deck
        // cell, which is what a slot is auditioned in. A per-slot measurement
        // used to be taken at a constant 1280x720 — a size nothing renders at
        // — and ADR-0303 removed it, so the size is named by whoever knows
        // the layout, which is this file and not the engine.
        //
        // **The first aimed cell, and every slot is told the same one.** The
        // four cells are one row of equal boxes and a probe measures a deck
        // with one target; a per-slot size would make four slots' numbers
        // incomparable, which is precisely what a governor summing them must
        // not have. With no cell aimed at all — the Program bay folded away —
        // nothing is said and the last size stands, because a bay that is not
        // laid out is not a statement that a measurement is about nothing.
        //
        // **Once a frame, and it is a store rather than a measurement.** The
        // cell moves when a divider moves or a bay folds, and the build worker
        // reads this per build, so a size taken once at startup would measure
        // every candidate of a session against whatever the window opened at.
        let cell = self.previews.iter().find(|p| p.aimed).map(|p| p.size);
        if let Some(cell) = cell {
            self.deck.set_measure_size(cell);
        }

        // **The frame's own size, derived from the outputs that are on** —
        // ADR-0247, and [`render_size`] is where the rule is. It is taken here
        // rather than anywhere else because this is the statement that has
        // just decided the picture's rectangle, and the picture's rectangle
        // *is* its size as an output.
        //
        // **A reallocation, so it is the frame's first act and not something
        // done mid-pass**, which is [`Presented::fit`]'s sentence one level
        // out: `aim` runs before `compose`, and P-0091 is about the render
        // thread. What it costs was measured on 2026-09-09 rather than
        // argued — **0.145 ms** for `Deck::resize` and `Present::resize`
        // together on a four-slot deck, host clock, biased high, and within a
        // few percent of the same at 466x262 and at 1920x1080 because what it
        // pays for is nine objects rather than their texels. It is paid on the
        // frames a size changed and on no others: every frame of a divider
        // drag on the Program bay's height is one of them, and it is the same
        // frame `Presented::fit` was already remaking the picture's texture
        // on.
        //
        // **Both, always, and in one statement.** `Frame::render` checks its
        // sizes and panics at the call site, so a deck resized without the
        // present pass is a loud failure a frame later — which is the right
        // failure and the wrong place to find out.
        let outputs = [self.picture.aimed.then_some(self.picture.size), projector];
        if let Some(at) = render_size(&outputs) {
            if at != self.present.size() {
                // **Nothing is printed here**, and that is the frame path
                // rather than reticence: every frame of a divider drag on the
                // Program bay's height changes this size, so a line would be
                // sixty a second and sixty formats a second with it. What the
                // frame is composited at is a *readout* — `Costs::say` prints
                // it beside the numbers it is about, which is what P-0095 asks
                // of a measurement, and the console page's own size pill is
                // where an operator reads it.
                self.present.resize(&gpu.device, &gpu.queue, at.0, at.1);
                self.deck.resize(&gpu.device, at.0, at.1);
                // **And every cell's bind group, because `Deck::resize`
                // replaced the views they were made from.**
                //
                // A bind group holds its view alive, so a stale one samples a
                // texture nothing draws into any more: the cells would go on
                // showing the last frame at the old size, under letters
                // saying their decks are running. That is the silent wrong
                // picture P-0094 refuses, and it is exactly the symptom
                // `Deck::resize`'s own comment names for the meters one line
                // along. It was found by
                // `every_cell_with_a_slot_behind_it_is_aimed_whatever_its_residency`
                // the first time the deck was resized from here.
                self.slot_bind_groups = std::array::from_fn(|slot| {
                    self.deck
                        .slot_view(EngineSlot(slot as u8))
                        .map(|view| self.present.create_bind_group_for(&gpu.device, view))
                });
            }
        }
        (picture, previews)
    }
}
