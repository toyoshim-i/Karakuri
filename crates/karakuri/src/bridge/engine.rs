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

/// The rendering engine driving the Program bay: manages deck slots, presentation passes, and output textures.
///
/// Steps and draws all active preview cells regardless of live status (ADR-0258, ADR-0269).
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
    /// Active master chain specification, updated via operations and applied to `Present` (ADR-0317, ADR-0340, P-0090).
    pub(crate) chain: Vec<karakuri_engine::SlotSpec>,
    /// Background worker channel for asynchronously compiling and swapping master chains (ADR-0354).
    pub(crate) chain_swap: karakuri_engine::ChainSwap,
    /// Count of freed texture registrations to verify against leaked bind groups across resizes.
    pub(crate) freed: usize,
    /// Target aim definitions and watch channels for each deck slot, indexed by slot.
    pub(crate) aimed: Vec<Aiming>,
    /// Global input wiring edges across all slots, maintained across rebuilds and saves (ADR-0329).
    pub(crate) edges: Vec<karakuri_engine::set::Edge>,
    /// Shared handle tracking active files running in each slot for external readers (ADR-0308).
    pub(crate) pointing: karakuri_mcp::Slots,
    /// Initial node placements and source bytes compiled at startup, used to seed [`Playing`].
    pub(crate) placed: Vec<karakuri_environment::compile::Placed>,
}

impl Engine {
    /// Initializes the engine, sizing presentations from layout arrangement and spawning slot watchers.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        // Presentation texture format matching the surface sRGB format.
        picture_format: wgpu::TextureFormat,
        slots: &[Sources],
        layout: &karakuri_layout::Layout,
        scale: f32,
        // Optional store destination and build report channel for slot watchers.
        stored: Option<(std::sync::Arc<Store>, std::sync::mpsc::Sender<watch::Built>)>,
        // Shared edit history across windows to prevent duplicate version snapshots.
        snapshots: Option<history::Shared>,
        // Shared slot-file mappings shared with MCP server and history restores.
        pointing: karakuri_mcp::Slots,
    ) -> Engine {
        assert!(
            slots.len() == SLOTS,
            "a deck of {SLOTS} slots was handed {} pairs to run from",
            slots.len()
        );
        // Compile the initial launch sources once for slot 0 and reuse placements across all slots.
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
        // Each slot runs an independent working copy and simulation salt (ADR-0228).
        let built = |salt| {
            Set::build(&gpu.device, &gpu.queue, &l1, &l4, capacity, salt)
                .expect("the pair builds a Set")
        };
        // Spawn a background watcher per slot over its distinct working files (P-0091).
        // Produces swap events for the Staging lane.
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
        // Only `ON_AIR` (deck A) starts unmuted; other slots start muted in composite mix.
        for slot in 0..SLOTS {
            if slot != ON_AIR {
                deck.set_mute(EngineSlot(slot as u8), true);
            }
        }
        // Enable level meters across all slots for live mixer telemetry.
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

    /// Startup initialization for the engine before the first frame.
    ///
    /// Measures and estimates all cold slots so their costs are known,
    /// and runs the initial governor pass against normal budget.
    pub(crate) fn startup(&mut self, gpu: &Gpu) -> Report {
        self.deck.measure_slots(&gpu.device, &gpu.queue);
        self.deck.estimate_slots(&gpu.device, &gpu.queue);
        self.deck.govern()
    }

    /// Requests priming for deck B with an artificially restricted budget to verify park behavior (ADR-0269, ADR-0296).
    #[allow(dead_code)]
    pub(crate) fn ask_to_prime(&mut self, gpu: &Gpu) -> Report {
        // Before the first frame: measure slot stepping costs.
        self.deck.measure_slots(&gpu.device, &gpu.queue);
        // Cold slot estimate recorded at deck size before first frame (ADR-0296, ADR-0356).
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

    /// Aims picture and per-slot preview sinks at layout rectangles (ADR-0240, ADR-0258).
    ///
    /// Aims all cells having backing slots regardless of residency state.
    pub(crate) fn aim(
        &mut self,
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        layout: &karakuri_layout::Layout,
        scale: f32,
        // The projector window's size, or `None` while closed.
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
        // Record preview cell target size on deck for governor workload benchmarking (ADR-0247, ADR-0303).
        let cell = self.previews.iter().find(|p| p.aimed).map(|p| p.size);
        if let Some(cell) = cell {
            self.deck.set_measure_size(cell);
        }

        // Resize composite targets and deck when output resolution changes (ADR-0247, P-0091).
        let outputs = [self.picture.aimed.then_some(self.picture.size), projector];
        if let Some(at) = render_size(&outputs) {
            if at != self.present.size() {
                // Suppress per-frame resize logging during interactive divider drags (P-0095).
                self.present.resize(&gpu.device, &gpu.queue, at.0, at.1);
                self.deck.resize(&gpu.device, at.0, at.1);
                // Recreate preview bind groups for newly allocated slot views on resize (P-0094).
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
