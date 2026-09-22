//! Multi-slot Set compositing, frame synchronization, and residency management.
//!
//! A [`Deck`] manages up to [`MAX_SLOTS`] resident [`HotSwap`] instances, rendering
//! live slots into dedicated `Rgba16Float` HDR targets and compositing them into a single
//! linear HDR target using per-slot gain, opacity, blend modes ([`Blend`]), and masks ([`Mask`]).
//!
//! # Architecture
//!
//! - **Slot Isolation**: Each slot renders into an isolated texture target. The composite pass
//!   folds active targets in fixed slot index order.
//! - **Frame Guard**: [`Deck::begin_frame`] acquires exclusive access to the deck and returns
//!   a [`Frame`] holding the `wgpu::CommandEncoder`. This guarantees single-encoder recording
//!   and supports two-phase atomic commit on submit or discard.
//! - **Residency**: Slots operate under [`Residency::Live`] (simulated, drawn, composited),
//!   [`Residency::Priming`] (simulated, drawn for preview, not composited), or
//!   [`Residency::Allocated`] (drawn for preview, not composited). The [`Governor`] computes
//!   effective residencies from operator requests and frame budgets.
//! - **Signals**: The deck owns the session [`Signals`] oscillator, advancing it once per
//!   frame so that bound parameters and transport mappings stay synchronized across slots.

use crate::binding::Signals;
use crate::governor::{Estimated, Governor, Report, SlotState};
use crate::meter::{Level, Meters};
use crate::mix::Composite;
use crate::probe::{MeasurementMethod, Probe};
use crate::set::{DT, MAX_STEPS};
use crate::swap::{Event, HotSwap};
use crate::transition::{Control, Selection, Transition};
use crate::transport::{Sync, Transport};

pub mod frame;
pub mod mixer;
pub mod types;

pub use frame::Frame;
pub(crate) use frame::{make_slot_target, StagedSlotControl};
pub use types::*;

/// Multi-slot Set runtime managing presentation targets, mixing, and frame dispatch.
pub struct Deck {
    pub(crate) slots: Vec<Slot>,
    pub(crate) composite: Composite,
    pub(crate) out: f32,
    pub(crate) meters: Option<Meters>,
    pub(crate) signals: Signals,
    pub(crate) governor: Governor,
    pub(crate) frame_budget_ms: f32,
    /// The running master chain's summed `ops_per_fragment`, charged against the
    /// frame beside the slots and against no one of them. Zero while the chain
    /// is empty. Written once a frame by `crate::frame::compose` from the
    /// `Present` that holds the chain; the milliseconds are derived at
    /// [`Deck::chain_ms`] against this deck's current size, so a resize moves
    /// the charge with no second write.
    pub(crate) chain_ops_per_fragment: u32,
    pub(crate) transitions: Vec<Transition>,
    pub(crate) selections: Vec<Selection>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) clock: Option<MeasurementMethod>,
    pub(crate) measure_at: (u32, u32),
    pub(crate) muted: [bool; MAX_SLOTS],
    pub(crate) solo: Option<usize>,
    pub(crate) revision: u64,
}

impl Deck {
    /// Creates a new Deck over `swaps`.
    ///
    /// Initializes all slots as [`Live`] at unity gain and full opacity with master out at 1.0.
    /// Allocates per-slot HDR targets, pipeline, and bind groups.
    ///
    /// [`Live`]: Residency::Live
    pub fn new(device: &wgpu::Device, swaps: Vec<HotSwap>, width: u32, height: u32) -> Deck {
        assert!(
            !swaps.is_empty() && swaps.len() <= MAX_SLOTS,
            "a deck holds 1 to {MAX_SLOTS} slots, not {}",
            swaps.len()
        );

        let targets: Vec<(wgpu::Texture, wgpu::TextureView)> = (0..swaps.len())
            .map(|i| make_slot_target(device, i, width, height))
            .collect();
        let views: Vec<&wgpu::TextureView> = targets.iter().map(|(_, v)| v).collect();
        let composite = Composite::new(device, &views);

        let slots = swaps
            .into_iter()
            .zip(targets)
            .map(|(mut swap, (target, view))| {
                swap.resize(device, width, height);
                Slot {
                    swap,
                    requested: Residency::Live,
                    effective: Residency::Live,
                    gain: 1.0,
                    opacity: 1.0,
                    blend: Blend::default(),
                    mask: Mask::default(),
                    transport: Transport::default(),
                    online: true,
                    target,
                    view,
                }
            })
            .collect();

        let mut deck = Deck {
            slots,
            composite,
            out: 1.0,
            meters: None,
            signals: Signals::default(),
            governor: Governor::default(),
            // The engine's constant until a caller that can ask the platform
            // says otherwise, which is `set_frame_budget_ms` (ADR-0313).
            frame_budget_ms: crate::swap::DEFAULT_BUDGET_MS,
            // The default chain is empty, so a deck that is never handed one
            // is charged nothing for it (ADR-0340).
            chain_ops_per_fragment: 0,
            // At its bound from the start: at most one per `(slot, control)`,
            // so this never grows and `schedule` never allocates. That matters
            // on the replay path, where a scheduled move arrives inside the
            // per-frame callback rather than from a key press.
            transitions: Vec::with_capacity(MAX_SLOTS * Control::ALL.len()),
            // At its bound too, and the bound is one per slot.
            selections: Vec::with_capacity(MAX_SLOTS),
            width,
            height,
            // Nothing has been probed yet, and a deck that says it is on a
            // host clock before anything has asked the adapter would be
            // reporting a verdict nobody took.
            clock: None,
            measure_at: (width, height),
            muted: [false; MAX_SLOTS],
            solo: None,
            revision: 0,
        };
        deck.set_measure_size((width, height));
        deck
    }

    /// Sets the measurement resolution for all slot build workers and profilers.
    pub fn set_measure_size(&mut self, at: (u32, u32)) {
        if at.0 == 0 || at.1 == 0 {
            return;
        }
        self.measure_at = at;
        for slot in &mut self.slots {
            slot.swap.set_measure_size(at);
        }
    }

    /// Returns the current measurement resolution.
    pub fn measure_size(&self) -> (u32, u32) {
        self.measure_at
    }

    /// Returns the measurement method used by the slot profiler, if measured.
    pub fn clock(&self) -> Option<MeasurementMethod> {
        self.clock
    }

    /// Returns a reference to the session's audio and timing signals.
    pub fn signals(&self) -> &Signals {
        &self.signals
    }

    /// Replaces the session timing signals and resets the clock phase.
    pub fn set_signals(&mut self, signals: Signals) {
        self.signals = signals;
    }

    /// Enables luminance level metering across all slots.
    pub fn enable_meters(&mut self, device: &wgpu::Device) {
        let views: Vec<&wgpu::TextureView> = self.slots.iter().map(|s| &s.view).collect();
        self.meters = Some(Meters::new(device, &views));
    }

    /// Returns a reference to the luminance meters, if enabled.
    pub fn meters(&self) -> Option<&Meters> {
        self.meters.as_ref()
    }

    /// Returns the most recent measured luminance level for the given slot.
    pub fn level(&self, slot: DeckSlot) -> Option<Level> {
        self.meters.as_ref().and_then(|m| m.level(slot.index()))
    }

    /// Begins recording a frame, returning an exclusive frame guard owning the command encoder.
    ///
    /// Installs completed candidate builds, marks frame boundaries for all slots, and collects
    /// pending meter samples without blocking.
    ///
    /// # Exclusivity
    ///
    /// The returned [`Frame`] retains exclusive borrow of the deck for the duration of the frame,
    /// preventing concurrent frame recordings:
    ///
    /// ```no_run
    /// # use karakuri_engine::deck::Deck;
    /// fn one_frame(
    ///     deck: &mut Deck,
    ///     device: &wgpu::Device,
    ///     queue: &wgpu::Queue,
    ///     hdr: &wgpu::TextureView,
    ///     size: (u32, u32),
    /// ) {
    ///     let mut frame = deck.begin_frame(device, queue);
    ///     frame.render(hdr, size, 1);
    ///     frame.finish();
    /// }
    /// ```
    ///
    /// Attempting to call `begin_frame` again while a frame is alive fails compilation:
    ///
    /// ```compile_fail
    /// # use karakuri_engine::deck::Deck;
    /// fn two_frames(
    ///     deck: &mut Deck,
    ///     device: &wgpu::Device,
    ///     queue: &wgpu::Queue,
    ///     hdr: &wgpu::TextureView,
    ///     size: (u32, u32),
    /// ) {
    ///     let mut first = deck.begin_frame(device, queue);
    ///     let mut second = deck.begin_frame(device, queue); // E0499
    ///     first.render(hdr, size, 1);
    ///     second.render(hdr, size, 1);
    /// }
    /// ```
    pub fn begin_frame<'a>(
        &'a mut self,
        device: &wgpu::Device,
        queue: &'a wgpu::Queue,
    ) -> Frame<'a> {
        for slot in self.slots.iter_mut() {
            slot.swap.live_mut().discard();
        }
        for (i, slot) in self.slots.iter_mut().enumerate() {
            let seen = slot.swap.pending_events().len();
            match slot.effective {
                Residency::Live => {
                    let _ = slot.swap.begin_frame(device);
                }
                Residency::Priming | Residency::Allocated => slot.swap.begin_frame_parked(device),
            }
            let replaced = slot.swap.pending_events()[seen..]
                .iter()
                .any(|e| matches!(e, Event::Swapped { .. }));
            if replaced {
                if let Some(meters) = &mut self.meters {
                    meters.retire(i);
                }
            }
        }
        if let Some(meters) = &mut self.meters {
            meters.collect(device);
        }
        Frame {
            deck: self,
            queue,
            encoder: Some(
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("deck frame"),
                }),
            ),
            rendered: false,
            staged_signals: None,
            staged_transitions: None,
            staged_selections: None,
            staged_slot_controls: None,
        }
    }

    /// Reallocates all slot render targets and updates composite bind groups.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if (width, height) == (self.width, self.height) {
            return;
        }
        for (i, slot) in self.slots.iter_mut().enumerate() {
            let (target, view) = make_slot_target(device, i, width, height);
            slot.target = target;
            slot.view = view;
            slot.swap.resize(device, width, height);
        }
        let views: Vec<&wgpu::TextureView> = self.slots.iter().map(|s| &s.view).collect();
        self.composite.rebind(device, &views);
        if let Some(meters) = &mut self.meters {
            meters.rebind(device, &views);
        }
        self.width = width;
        self.height = height;
    }

    /// Returns the total number of slots in the deck.
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// Returns the number of slots currently composited into the mix.
    pub fn live_slots(&self) -> usize {
        self.slots
            .iter()
            .filter(|s| s.effective == Residency::Live)
            .count()
    }

    /// Installs a compiled Set directly into a slot.
    pub fn install(&mut self, device: &wgpu::Device, slot: DeckSlot, set: crate::set::Set) {
        self.slots[slot.index()].swap.install(device, set);
    }

    /// Returns a reference to the hot-swap manager for a slot.
    pub fn slot(&self, slot: DeckSlot) -> &HotSwap {
        &self.slots[slot.index()].swap
    }

    /// Writes a parameter value into the live Set of the specified slot.
    ///
    /// Returns the number of declarations updated, or an error if the write crosses authorities.
    pub fn write_param(
        &mut self,
        slot: DeckSlot,
        write: &crate::binding::ParamWrite,
    ) -> Result<usize, crate::set::CrossesAuthority> {
        self.slots[slot.index()].swap.live_mut().write_param(write)
    }

    /// Attaches a dynamic signal binding to a parameter in the specified slot.
    pub fn bind(&mut self, slot: DeckSlot, binding: crate::binding::Binding) -> crate::set::Bound {
        self.slots[slot.index()].swap.live_mut().bind(binding)
    }

    /// Detaches a dynamic signal binding from a parameter in the specified slot.
    ///
    /// Returns true if an existing binding was removed.
    pub fn unbind(
        &mut self,
        slot: DeckSlot,
        layer: karakuri_ir::Kind,
        index: Option<u32>,
        key: &str,
    ) -> bool {
        self.slots[slot.index()]
            .swap
            .live_mut()
            .unbind(layer, index, key)
    }

    /// Sets parameter mutation authority on a node within the specified slot.
    ///
    /// Returns false if the referenced node does not exist.
    pub fn set_authority(
        &mut self,
        slot: DeckSlot,
        layer: karakuri_ir::Kind,
        index: u32,
        authority: crate::set::Authority,
    ) -> bool {
        self.slots[slot.index()]
            .swap
            .live_mut()
            .set_authority(layer, index, authority)
    }

    /// Returns the texture view for a slot's linear HDR render target.
    pub fn slot_view(&self, slot: DeckSlot) -> Option<&wgpu::TextureView> {
        self.slots.get(slot.index()).map(|s| &s.view)
    }

    /// Drains lifecycle events that occurred on the specified slot.
    pub fn events(&mut self, slot: DeckSlot) -> std::vec::Drain<'_, Event> {
        self.slots[slot.index()].swap.events()
    }

    /// Returns the effective residency of the specified slot.
    pub fn residency(&self, slot: DeckSlot) -> Residency {
        self.slots[slot.index()].effective
    }

    /// Returns the requested residency of the specified slot.
    pub fn requested_residency(&self, slot: DeckSlot) -> Residency {
        self.slots[slot.index()].requested
    }

    /// Returns true if the slot requested priming but was deferred due to compute budget limits.
    pub fn is_parked(&self, slot: DeckSlot) -> bool {
        let slot = &self.slots[slot.index()];
        slot.requested == Residency::Priming && slot.effective == Residency::Allocated
    }

    /// Returns true if the slot has been stopped by the performance watchdog.
    pub fn overloaded(&self, slot: DeckSlot) -> bool {
        self.slots[slot.index()].swap.overloaded()
    }

    /// Requests a residency level for the specified slot.
    pub fn set_residency(&mut self, slot: DeckSlot, residency: Residency) {
        self.slots[slot.index()].requested = residency;
        self.set_effective(slot, residency);
    }

    /// Updates the effective residency of a slot, retiring meters when taken off air.
    fn set_effective(&mut self, slot: DeckSlot, residency: Residency) {
        if self.slots[slot.index()].effective == residency {
            return;
        }
        self.slots[slot.index()].effective = residency;
        if residency != Residency::Live {
            self.slots[slot.index()].online = false;
            if let Some(meters) = &mut self.meters {
                meters.retire(slot.index());
            }
        } else {
            self.slots[slot.index()].online = match self.solo {
                Some(s) => slot.index() == s,
                None => !self.muted.get(slot.index()).copied().unwrap_or(false),
            };
        }
    }

    /// Returns the transport configuration for the specified slot.
    pub fn transport(&self, slot: DeckSlot) -> &Transport {
        &self.slots[slot.index()].transport
    }

    /// Configures transport synchronization mode for the specified slot.
    pub fn set_transport(
        &mut self,
        slot: DeckSlot,
        sync: Sync,
        anchor_bpm: f32,
        scrub_beats: f64,
    ) -> Result<(), crate::transport::Refusal> {
        self.sync_allowed(slot, sync)?;
        self.slots[slot.index()]
            .transport
            .set(sync, anchor_bpm, scrub_beats);
        Ok(())
    }

    /// Checks whether the given transport sync mode is valid for the current Set.
    pub fn sync_allowed(
        &self,
        slot: DeckSlot,
        sync: Sync,
    ) -> Result<(), crate::transport::Refusal> {
        let set = self.slots[slot.index()].swap.set();
        Transport::allows(sync, set.is_closed_form(), set.reads_beats())
    }

    /// Returns the number of slots currently in the Priming residency state.
    pub fn priming_slots(&self) -> usize {
        self.slots
            .iter()
            .filter(|s| s.effective == Residency::Priming)
            .count()
    }

    /// Returns the number of slots whose priming request is currently parked.
    pub fn parked_slots(&self) -> usize {
        (0..self.slots.len())
            .filter(|&i| self.is_parked(DeckSlot(i as u8)))
            .count()
    }

    /// Returns the aggregate compute budget allocated for priming evaluation in milliseconds.
    pub fn compute_budget_ms(&self) -> f32 {
        self.governor.budget_ms()
    }

    /// Sets the aggregate compute budget allocated for priming evaluation in milliseconds.
    pub fn set_compute_budget_ms(&mut self, budget_ms: f32) {
        self.governor.set_budget_ms(budget_ms);
    }

    /// Sets the target frame duration budget in milliseconds across all slot watchdogs.
    pub fn set_frame_budget_ms(&mut self, budget_ms: f32) {
        if !budget_ms.is_finite() || budget_ms <= 0.0 {
            return;
        }
        self.frame_budget_ms = budget_ms;
        for slot in &mut self.slots {
            slot.swap.set_budget_ms(budget_ms);
        }
    }

    /// Returns the target frame duration budget in milliseconds.
    pub fn frame_budget_ms(&self) -> f32 {
        self.frame_budget_ms
    }

    /// Records what the running master chain costs per texel — the sum over its
    /// slots of `ops_per_fragment`, which is `Present::chain_ops_per_fragment`.
    ///
    /// Zero for an empty chain. It is a rate and not a duration: what it costs
    /// in milliseconds is [`Deck::chain_ms`], taken against this deck's current
    /// size, so a resize needs no second call.
    pub fn set_chain_ops_per_fragment(&mut self, ops: u32) {
        self.chain_ops_per_fragment = ops;
    }

    /// What [`Deck::set_chain_ops_per_fragment`] last recorded.
    pub fn chain_ops_per_fragment(&self) -> u32 {
        self.chain_ops_per_fragment
    }

    /// What the running master chain costs this frame, in milliseconds at the
    /// size this deck renders at.
    ///
    /// Charged against the frame beside the slots and against no one of them
    /// (ADR-0340), which is what [`Deck::govern`] spends it as.
    pub fn chain_ms(&self) -> f32 {
        crate::estimate::chain_ms(self.chain_ops_per_fragment, (self.width, self.height))
    }

    /// The clock a master chain slot reads on a frame that advances by `steps`.
    ///
    /// `t` and `beats` are the session clock at that frame's last substep — the
    /// instant every Live slot's own last substep lands on — and `dt` is the
    /// fixed simulation step, the same number every other layer reads. All
    /// three come from the `tick` this frame was committed with and none from a
    /// wall clock
    /// ([P-0092](../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)).
    ///
    /// Nothing is advanced here: the value is what [`Frame::render`] will have
    /// advanced the session to, so it may be written before the frame's encoder
    /// exists. `seed_salt` is zero — an L5 reads no `seed`.
    pub fn chain_clock(&self, steps: u8) -> crate::pass::Clock {
        let mut signals = self.signals;
        signals.advance(steps.min(MAX_STEPS), DT);
        crate::pass::Clock {
            t: signals.oscillator().t() as f32,
            beats: signals.oscillator().beats() as f32,
            dt: DT,
            seed_salt: 0,
        }
    }

    /// Returns the rolling median frame duration across recent frames, or None if unavailable.
    pub fn frame_period_ms(&self) -> Option<f32> {
        self.slots.first()?.swap.frame_period_ms()
    }

    /// Measures uncalibrated slots at startup, returning the number of slots evaluated.
    pub fn measure_slots(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> usize {
        let measurable =
            |slot: &Slot| slot.swap.measured_cost().is_none() && slot.swap.set().time() == 0.0;
        if !self.slots.iter().any(measurable) {
            return 0;
        }
        let mut probe = Probe::new(
            device,
            queue,
            device.features().contains(wgpu::Features::TIMESTAMP_QUERY),
            self.measure_at,
        );
        let mut measured = 0;
        for slot in &mut self.slots {
            if measurable(slot) {
                slot.swap.measure_live(&mut probe, device, queue);
                measured += 1;
            }
        }
        self.clock = Some(probe.method());
        measured
    }

    /// Estimates render cost at the deck's target resolution across uncalibrated slots.
    pub fn estimate_slots(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> usize {
        let target = (self.width, self.height);
        let estimable =
            |slot: &Slot| slot.swap.estimated_cost().is_none() && slot.swap.set().time() == 0.0;
        if !self.slots.iter().any(estimable) {
            return 0;
        }
        let mut probe = Probe::new(
            device,
            queue,
            device.features().contains(wgpu::Features::TIMESTAMP_QUERY),
            self.measure_at,
        );
        let mut estimated = 0;
        for slot in &mut self.slots {
            if estimable(slot) {
                slot.swap.estimate_live(&mut probe, device, queue, target);
                estimated += 1;
            }
        }
        self.clock = Some(probe.method());
        estimated
    }

    /// Evaluates slot residency and compute budget allocations, applying effective states.
    pub fn govern(&mut self) -> Report {
        let states: Vec<SlotState> = self
            .slots
            .iter()
            .map(|s| SlotState {
                requested: s.requested,
                cost: s.swap.measured_cost(),
                estimate: s.swap.estimated_cost().map(Estimated::from),
                closed_form: s.swap.set().is_closed_form(),
            })
            .collect();
        // The chain first: it is not one of the slots. A chain covers the
        // whole frame once per slot and belongs to no deck, so it is reserved
        // out of the budget ahead of every slot rather than summed into
        // `committed_ms` (ADR-0340). It is taken here so that a resize moves it
        // without a second writer.
        let chain_ms = self.chain_ms();
        self.governor.set_chain_ms(chain_ms);
        let mut report = self.governor.decide(&states);
        report.frame_period_ms = self.frame_period_ms();
        report.frame_budget_ms = Some(self.frame_budget_ms);
        for decision in &report.decisions {
            self.set_effective(DeckSlot(decision.slot as u8), decision.effective);
        }
        report
    }

    pub fn schedule(&mut self, transition: Transition) {
        assert!(
            transition.slot() < self.slots.len(),
            "no slot {}: this deck holds slots 0-{}",
            transition.slot(),
            self.slots.len() - 1
        );
        self.cancel(DeckSlot(transition.slot() as u8), transition.control());
        self.transitions.push(transition);
    }

    /// Cancels any scheduled transition on the given slot control.
    pub fn cancel(&mut self, slot: DeckSlot, control: Control) {
        self.transitions
            .retain(|t| !(t.slot() == slot.index() && t.control() == control));
    }

    /// Returns an iterator over active transitions on the specified slot.
    pub fn transitions_on(&self, slot: DeckSlot) -> impl Iterator<Item = &Transition> {
        self.transitions
            .iter()
            .filter(move |t| t.slot() == slot.index())
    }

    /// Schedules an animated selection switch for a slot's renderer.
    pub fn schedule_selection(&mut self, selection: Selection) {
        assert!(
            selection.slot() < self.slots.len(),
            "no slot {}: this deck holds slots 0-{}",
            selection.slot(),
            self.slots.len() - 1
        );
        self.selections.retain(|s| s.slot() != selection.slot());
        self.selections.push(selection);
    }

    /// Returns an iterator over pending selections for the specified slot.
    pub fn selections_on(&self, slot: DeckSlot) -> impl Iterator<Item = &Selection> {
        self.selections
            .iter()
            .filter(move |s| s.slot() == slot.index())
    }

    /// Stages due renderer selections without modifying deck state before commit.
    fn stage_selections(&mut self, beats: f64) -> Vec<Selection> {
        let mut staged_selections = self.selections.clone();
        for s in &staged_selections {
            if !s.due(beats) {
                continue;
            }
            self.slots[s.slot()]
                .swap
                .live_mut()
                .stage_select_renderer(s.renderer());
        }
        staged_selections.retain(|s| !s.due(beats));
        staged_selections
    }

    /// Calculates staged transitions for the given beat position without mutating slot fields.
    fn stage_transitions(&self, beats: f64) -> (Vec<Transition>, Vec<StagedSlotControl>) {
        let mut staged_transitions = self.transitions.clone();
        let mut slot_controls: Vec<StagedSlotControl> = Vec::new();
        for t in &staged_transitions {
            let Some(value) = t.value_at(beats) else {
                continue;
            };
            let (mut gain, mut opacity, mut mask) = slot_controls
                .iter()
                .find(|c| c.slot == t.slot())
                .map(|c| (c.gain, c.opacity, c.mask))
                .unwrap_or_else(|| {
                    let slot = &self.slots[t.slot()];
                    (slot.gain, slot.opacity, slot.mask)
                });
            match t.control() {
                Control::Gain => {
                    gain = clamp_gain(value);
                }
                Control::Opacity => {
                    opacity = clamp_opacity(value);
                }
                Control::MaskPosition => {
                    mask = mask.at(value);
                }
            }
            if let Some(entry) = slot_controls.iter_mut().find(|c| c.slot == t.slot()) {
                entry.gain = gain;
                entry.opacity = opacity;
                entry.mask = mask;
            } else {
                slot_controls.push(StagedSlotControl {
                    slot: t.slot(),
                    gain,
                    opacity,
                    mask,
                });
            }
        }
        staged_transitions.retain(|t| !t.finished(beats));
        (staged_transitions, slot_controls)
    }

    /// Returns a reference to the slot's render target texture.
    pub fn slot_target(&self, slot: DeckSlot) -> &wgpu::Texture {
        &self.slots[slot.index()].target
    }
}
