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
use crate::mix::{Composite, Input};
use crate::present::Present;
use crate::probe::{MeasurementMethod, Probe};
use crate::set::{DT, MAX_STEPS};
use crate::swap::{Event, HotSwap};
use crate::transition::{Control, Selection, Transition};
use crate::transport::{Advance, Sync, Transport};
use crate::video_source::VideoSource;

/// Maximum number of slots a deck can hold.
pub const MAX_SLOTS: usize = 4;

/// Slot index within the deck mixer.
///
/// Distinct from [`crate::master::Slot`], which represents an effect pass in the master chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeckSlot(pub u8);

impl DeckSlot {
    /// Returns a validated slot index if `slot < count`.
    pub fn new(slot: u8, count: usize) -> Option<DeckSlot> {
        if (slot as usize) < count {
            Some(DeckSlot(slot))
        } else {
            None
        }
    }

    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

impl std::fmt::Display for DeckSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u8> for DeckSlot {
    fn from(slot: u8) -> DeckSlot {
        DeckSlot(slot)
    }
}

/// Clamps gain to non-negative finite values, treating NaN as zero.
fn clamp_gain(gain: f32) -> f32 {
    if gain.is_nan() {
        0.0
    } else {
        gain.max(0.0)
    }
}

/// Clamps opacity to the unit interval `[0.0, 1.0]`, treating NaN as zero.
fn clamp_opacity(opacity: f32) -> f32 {
    if opacity.is_nan() {
        0.0
    } else {
        opacity.clamp(0.0, 1.0)
    }
}

/// Spatial mask applied to a slot layer's opacity during composition.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Mask {
    kind: MaskKind,
    /// Orientation angle for linear masks in radians.
    angle: f32,
    /// Reveal progression in `[0.0, 1.0]`.
    position: f32,
    /// Transition edge softness in `[0.0, 1.0]`.
    softness: f32,
}

/// Shape geometry for a slot mask.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MaskKind {
    /// Full frame reveal without masking.
    #[default]
    None,
    /// Linear transition boundary across the frame.
    Linear,
    /// Centered radial circular mask.
    Radial,
}

impl MaskKind {
    /// All mask geometries in cycle order.
    pub const ALL: [MaskKind; 3] = [MaskKind::None, MaskKind::Linear, MaskKind::Radial];

    /// Wire and display name for this mask geometry.
    pub fn name(self) -> &'static str {
        match self {
            MaskKind::None => "none",
            MaskKind::Linear => "linear",
            MaskKind::Radial => "radial",
        }
    }

    /// Parses a mask geometry name, or returns None if unrecognised.
    pub fn from_name(name: &str) -> Option<MaskKind> {
        MaskKind::ALL.iter().copied().find(|k| k.name() == name)
    }

    /// Numeric index encoded for shader uniform consumption.
    pub(crate) fn index(self) -> u32 {
        match self {
            MaskKind::None => 0,
            MaskKind::Linear => 1,
            MaskKind::Radial => 2,
        }
    }
}

impl Default for Mask {
    fn default() -> Mask {
        Mask {
            kind: MaskKind::None,
            angle: 0.0,
            position: 1.0,
            softness: 0.0,
        }
    }
}

impl Mask {
    /// Creates a mask with values clamped to normalized shader ranges.
    pub fn new(kind: MaskKind, angle: f32, position: f32, softness: f32) -> Mask {
        Mask {
            kind,
            angle: if angle.is_finite() { angle } else { 0.0 },
            position: clamp_unit(position),
            softness: clamp_unit(softness),
        }
    }

    pub fn kind(self) -> MaskKind {
        self.kind
    }

    pub fn angle(self) -> f32 {
        self.angle
    }

    pub fn position(self) -> f32 {
        self.position
    }

    pub fn softness(self) -> f32 {
        self.softness
    }

    /// Returns a copy of the mask with its reveal position set to `position`.
    pub fn at(self, position: f32) -> Mask {
        Mask {
            position: clamp_unit(position),
            ..self
        }
    }

    /// Returns true if the mask completely hides the slot content.
    pub(crate) fn hides_everything(self) -> bool {
        self.kind != MaskKind::None && self.position == 0.0
    }
}

/// Clamps values to `[0.0, 1.0]`, treating NaN as zero.
fn clamp_unit(x: f32) -> f32 {
    if x.is_nan() {
        0.0
    } else {
        x.clamp(0.0, 1.0)
    }
}

/// Blend mode used to fold a slot layer into the composite accumulation target.
///
/// Evaluated in linear HDR space before tone mapping:
/// - [`Blend::Add`]: Additive emission accumulation.
/// - [`Blend::Over`]: Premultiplied alpha over blending using slot coverage.
/// - [`Blend::Max`]: Component-wise maximum value.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Blend {
    /// Additive sum of colour values.
    #[default]
    Add,
    /// Alpha over blending against accumulated coverage.
    Over,
    /// Component-wise maximum value.
    Max,
}

impl Blend {
    /// All blend modes in cycle order.
    pub const ALL: [Blend; 3] = [Blend::Add, Blend::Over, Blend::Max];

    /// Wire and display name for this blend mode.
    pub fn name(self) -> &'static str {
        match self {
            Blend::Add => "add",
            Blend::Over => "over",
            Blend::Max => "max",
        }
    }

    /// Parses a blend mode name, or returns None if unrecognised.
    pub fn from_name(name: &str) -> Option<Blend> {
        Blend::ALL.iter().copied().find(|b| b.name() == name)
    }

    /// Numeric index encoded for shader uniform consumption.
    pub(crate) fn index(self) -> u32 {
        match self {
            Blend::Add => 0,
            Blend::Over => 1,
            Blend::Max => 2,
        }
    }

    /// Returns true if the given gain and opacity silence the slot in this blend mode.
    pub(crate) fn silent_at(self, gain: f32, opacity: f32) -> bool {
        opacity == 0.0 || (gain == 0.0 && self != Blend::Over)
    }
}

/// Lifecycle and composition state of a slot in the deck.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Residency {
    /// Stepped, drawn, and folded into the composite mix.
    Live,
    /// Stepped and drawn into preview targets, but excluded from the composite mix.
    Priming,
    /// Stepped and drawn for monitoring, but unbudgeted for on-air composition.
    Allocated,
}

/// Resident slot entry holding a Set, render targets, and mixer edge controls.
struct Slot {
    swap: HotSwap,
    /// Requested residency set by the operator.
    requested: Residency,
    /// Effective residency granted by the governor.
    effective: Residency,
    /// Linear colour gain applied before blending.
    gain: f32,
    /// Opacity fader scaling blend contribution in `[0.0, 1.0]`.
    opacity: f32,
    /// Blend mode for folding this slot into the accumulation target.
    blend: Blend,
    /// Spatial reveal mask.
    mask: Mask,
    /// Transport mapping governing clock advancement.
    transport: Transport,
    target: wgpu::Texture,
    view: wgpu::TextureView,
}

impl Slot {
    /// Constructs the composite input descriptor for this slot.
    fn edge(&self) -> Input {
        Input {
            gain: self.gain,
            opacity: self.opacity,
            blend: self.blend,
            mask: self.mask,
            live: true,
        }
    }
}

/// Multi-slot Set runtime managing presentation targets, mixing, and frame dispatch.
pub struct Deck {
    slots: Vec<Slot>,
    composite: Composite,
    out: f32,
    meters: Option<Meters>,
    signals: Signals,
    governor: Governor,
    frame_budget_ms: f32,
    transitions: Vec<Transition>,
    selections: Vec<Selection>,
    width: u32,
    height: u32,
    clock: Option<MeasurementMethod>,
    measure_at: (u32, u32),
}

impl Deck {
    /// A deck over `swaps`, in that order. Every slot comes up [`Live`] at
    /// unity gain and full opacity, and the master out comes up at 1.0, so a
    /// deck of one is a bare Set with two multiplies by 1.0 in front of it —
    /// both exact, which is what keeps that a bit-for-bit claim rather than a
    /// close one.
    ///
    /// Allocates: one HDR target per slot, a pipeline, a bind group. None of
    /// that may happen on the render thread, which is why it happens here and
    /// in [`Deck::resize`] and nowhere else.
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
            // **The output size, until somebody names the other one.** This
            // application has two resolutions — the output the mix is
            // composited once at (ADR-0247) and the preview cell each slot is
            // auditioned in — and a deck that has not been told which one a
            // measurement is about answers with the one it knows. It is never
            // a third size, which is what ADR-0303 removed.
            //
            // **Told to the slots below rather than written here**, because a
            // `HotSwap` seeds itself from the viewport of the Set it was
            // handed and `Deck::new` is what resizes that Set — so a slot left
            // to its own seed would measure at whatever `Set::build` left,
            // which is 1x1.
            measure_at: (width, height),
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
            if let Some(meters) = &mut self.meters {
                meters.retire(slot.index());
            }
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
        let mut report = self.governor.decide(&states);
        report.frame_period_ms = self.frame_period_ms();
        report.frame_budget_ms = Some(self.frame_budget_ms);
        for decision in &report.decisions {
            self.set_effective(DeckSlot(decision.slot as u8), decision.effective);
        }
        report
    }

    pub fn gain(&self, slot: DeckSlot) -> f32 {
        self.slots[slot.index()].gain
    }

    /// Sets the linear colour gain for a slot, cancelling active gain transitions.
    pub fn set_gain(&mut self, slot: DeckSlot, gain: f32) {
        self.cancel(slot, Control::Gain);
        self.slots[slot.index()].gain = clamp_gain(gain);
    }

    pub fn opacity(&self, slot: DeckSlot) -> f32 {
        self.slots[slot.index()].opacity
    }

    /// Sets the blend opacity fader for a slot in `[0.0, 1.0]`, cancelling active opacity transitions.
    pub fn set_opacity(&mut self, slot: DeckSlot, opacity: f32) {
        self.cancel(slot, Control::Opacity);
        self.slots[slot.index()].opacity = clamp_opacity(opacity);
    }

    pub fn out(&self) -> f32 {
        self.out
    }

    /// Sets the master output scaling level applied to the composited mix.
    pub fn set_out(&mut self, out: f32) {
        self.out = clamp_gain(out);
    }

    pub fn blend(&self, slot: DeckSlot) -> Blend {
        self.slots[slot.index()].blend
    }

    pub fn mask(&self, slot: DeckSlot) -> Mask {
        self.slots[slot.index()].mask
    }

    /// Sets the mask shape geometry and angle without altering its current reveal position.
    pub fn set_mask_shape(&mut self, slot: DeckSlot, kind: MaskKind, angle: f32) {
        let mask = self.slots[slot.index()].mask;
        self.slots[slot.index()].mask = Mask::new(kind, angle, mask.position(), mask.softness());
    }

    /// Sets the mask reveal position, cancelling any active mask position transitions.
    pub fn set_mask_position(&mut self, slot: DeckSlot, position: f32) {
        self.cancel(slot, Control::MaskPosition);
        let mask = self.slots[slot.index()].mask;
        self.slots[slot.index()].mask = mask.at(position);
    }

    /// Sets the full mask specification for a slot, cancelling active mask position transitions.
    pub fn set_mask(&mut self, slot: DeckSlot, mask: Mask) {
        self.set_mask_shape(slot, mask.kind(), mask.angle());
        self.set_mask_position(slot, mask.position());
        let at = self.slots[slot.index()].mask;
        self.slots[slot.index()].mask =
            Mask::new(at.kind(), at.angle(), at.position(), mask.softness());
    }

    /// Schedules an animated transition on a slot control.
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

    /// Sets the blend mode for the specified slot.
    pub fn set_blend(&mut self, slot: DeckSlot, blend: Blend) {
        self.slots[slot.index()].blend = blend;
    }

    /// Returns a reference to the slot's render target texture.
    pub fn slot_target(&self, slot: DeckSlot) -> &wgpu::Texture {
        &self.slots[slot.index()].target
    }
}

/// Staged slot control state evaluated during frame rendering.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StagedSlotControl {
    pub(crate) slot: usize,
    pub(crate) gain: f32,
    pub(crate) opacity: f32,
    pub(crate) mask: Mask,
}

/// Active frame recording guard holding the command encoder and exclusive deck access.
pub struct Frame<'a> {
    deck: &'a mut Deck,
    queue: &'a wgpu::Queue,
    encoder: Option<wgpu::CommandEncoder>,
    rendered: bool,
    staged_signals: Option<Signals>,
    staged_transitions: Option<Vec<Transition>>,
    staged_selections: Option<Vec<Selection>>,
    staged_slot_controls: Option<Vec<StagedSlotControl>>,
}

impl Frame<'_> {
    /// Advances live slots by `steps`, renders each into its target, and composites into `target`.
    pub fn render(&mut self, target: &wgpu::TextureView, target_size: (u32, u32), steps: u8) {
        assert!(
            !self.rendered,
            "one `render` per frame: a second one would advance every Live slot by \
             another `steps` off the same tick"
        );
        self.rendered = true;

        let mut signals = self.deck.signals;
        signals.advance(steps.min(MAX_STEPS), DT);
        self.staged_signals = Some(signals);

        assert_eq!(
            target_size,
            (self.deck.width, self.deck.height),
            "the mix target is {target_size:?} and the deck's slots are {:?} — \
             resize both or the composite silently reads out of range and mixes black",
            (self.deck.width, self.deck.height)
        );

        let encoder = self
            .encoder
            .as_mut()
            .expect("the encoder is open until `finish` or drop");

        let beats = signals.oscillator().beats();
        let (staged_transitions, staged_controls) = self.deck.stage_transitions(beats);
        self.staged_transitions = Some(staged_transitions);
        self.staged_slot_controls = Some(staged_controls);
        let staged_selections = self.deck.stage_selections(beats);
        self.staged_selections = Some(staged_selections);

        for (i, slot) in self.deck.slots.iter_mut().enumerate() {
            let stopped = slot.swap.overloaded();
            match slot.effective {
                Residency::Live if stopped => {
                    if let Some(meters) = &mut self.deck.meters {
                        meters.record(i, encoder);
                    }
                }
                Residency::Live => {
                    let view = &slot.view;
                    let set = slot.swap.live_mut();
                    let steps = match slot.transport.advance(steps, signals.oscillator(), DT) {
                        Advance::Steps(n) => n,
                        Advance::SeekTo(target) => {
                            set.seek(target.saturating_sub(1));
                            1
                        }
                    };
                    set.prepare(self.queue, steps, &signals);
                    set.render(encoder, view, steps);
                    if let Some(meters) = &mut self.deck.meters {
                        meters.record(i, encoder);
                    }
                }
                Residency::Priming | Residency::Allocated if stopped => {}
                Residency::Priming | Residency::Allocated => {
                    let view = &slot.view;
                    let set = slot.swap.live_mut();
                    set.prepare_warming(self.queue, steps, &signals);
                    set.render(encoder, view, steps);
                }
            }
        }

        let mut edges: Vec<Input> = Vec::with_capacity(self.deck.slots.len());
        for (i, slot) in self.deck.slots.iter().enumerate() {
            let mut edge = slot.edge();
            if let Some(controls) = &self.staged_slot_controls {
                if let Some(c) = controls.iter().find(|c| c.slot == i) {
                    edge.gain = c.gain;
                    edge.opacity = c.opacity;
                    edge.mask = c.mask;
                }
            }
            edges.push(Input {
                live: slot.effective == Residency::Live,
                ..edge
            });
        }
        self.deck
            .composite
            .write_uniform(self.queue, &edges, self.deck.out);
        self.deck.composite.record(encoder, target);
    }

    /// Returns a mutable reference to the open frame command encoder.
    pub fn encoder(&mut self) -> &mut wgpu::CommandEncoder {
        self.encoder
            .as_mut()
            .expect("the encoder is open until `finish` or drop")
    }

    /// Submits the command buffer and commits staged state transitions to the deck.
    pub fn finish(mut self) {
        self.submit();
    }

    /// Discards the frame without submitting commands or committing staged state.
    pub fn discard(mut self) {
        let _ = self.encoder.take();
        self.staged_signals = None;
        self.staged_transitions = None;
        self.staged_selections = None;
        self.staged_slot_controls = None;
        for slot in &mut self.deck.slots {
            slot.swap.live_mut().discard();
        }
    }

    fn submit(&mut self) {
        if let Some(encoder) = self.encoder.take() {
            self.queue.submit([encoder.finish()]);
            if let Some(signals) = self.staged_signals.take() {
                self.deck.signals = signals;
            }
            if let Some(transitions) = self.staged_transitions.take() {
                self.deck.transitions = transitions;
            }
            if let Some(selections) = self.staged_selections.take() {
                self.deck.selections = selections;
            }
            if let Some(controls) = self.staged_slot_controls.take() {
                for c in controls {
                    self.deck.slots[c.slot].gain = c.gain;
                    self.deck.slots[c.slot].opacity = c.opacity;
                    self.deck.slots[c.slot].mask = c.mask;
                }
            }
            for slot in &mut self.deck.slots {
                slot.swap.live_mut().commit();
            }
            if let Some(meters) = &mut self.deck.meters {
                meters.arm();
            }
        }
    }
}

impl Drop for Frame<'_> {
    fn drop(&mut self) {
        self.submit();
    }
}

/// Creates an HDR texture target and corresponding view for a deck slot.
fn make_slot_target(
    device: &wgpu::Device,
    slot: usize,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(&format!("deck slot {slot}")),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: Present::HDR_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    (texture, view)
}
