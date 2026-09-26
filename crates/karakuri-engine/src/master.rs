//! The master chain: an ordered sequence of L5 post-processing slots connecting mix to tone mapping.

use std::collections::BTreeMap;
use std::fmt;

use karakuri_ir::typed::Checked;

pub use crate::pass::{
    create_hdr_target, BoundImagePass, Clock, Cut, ImagePass, RenderPassNode, Retained,
    RetentionManager,
};

/// Describes an uncompiled slot specification within the master chain.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotSpec {
    pub procedure: String,
    pub cut: Option<Cut>,
    pub params: BTreeMap<String, f32>,
}

/// Errors occurring during master chain slot validation or construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotError {
    /// The procedure is not an L5 at all.
    NotL5 { proc: String, kind: String },
    /// A `cut` was answered for a procedure that never asked for one.
    CutWithoutRetains { proc: String, cut: Cut },
    /// The procedure declares `retains` and the slot answered no cut.
    RetainsWithoutCut { proc: String },
    /// The procedure declares `uses … : Texture`, which a chain slot has no
    /// `edge` to bind.
    TextureSlot { proc: String, slot: String },
}

impl fmt::Display for SlotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SlotError::NotL5 { proc, kind } => write!(
                f,
                "{proc} declares `kind {kind}` and a master chain slot holds an L5 — \
                 write `kind L5` with a `frame` block, or load this procedure over a Set"
            ),
            SlotError::CutWithoutRetains { proc, cut } => write!(
                f,
                "{proc} was given the `{}` cut and does not declare `retains` — \
                 drop the cut, or declare `retains` in the procedure",
                cut.name()
            ),
            SlotError::RetainsWithoutCut { proc } => write!(
                f,
                "{proc} declares `retains` and the slot answers no cut — \
                 give the slot `mix` or `exit`"
            ),
            SlotError::TextureSlot { proc, slot } => write!(
                f,
                "{proc} declares `uses {slot} : Texture` and a master chain slot has no \
                 `edge` to bind it — the chain's only fan-in is its order, so drop the \
                 slot or nest this procedure in a Set"
            ),
        }
    }
}

/// One declared parameter of a chain slot, as a surface reads it: the key the
/// procedure declares, the range it declares it over, and what the slot holds.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotParam {
    pub key: String,
    pub min: f32,
    pub max: f32,
    /// The value the slot is running at, clamped into the declared range.
    pub value: f32,
    /// What the procedure declares it at.
    pub default: f32,
}

/// Runtime inspection state of an active master chain slot.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotReading {
    /// The content address of the procedure's source.
    pub procedure: String,
    /// The name the procedure declares.
    pub name: String,
    /// Which cut this slot reads, and `None` where its procedure declares no
    /// `retains`.
    pub cut: Option<Cut>,
    /// Whether the procedure declares `retains`, which is whether the slot has a
    /// cut to be set at all.
    pub retains: bool,
    /// The declared parameters, in declaration order.
    pub params: Vec<SlotParam>,
}

/// Compiled pipeline and parameter state for one slot in the master chain.
pub struct Slot {
    /// Content address of the procedure source code.
    proc: String,
    /// Fullscreen image pass pipeline and uniform state.
    pass: ImagePass,
    /// Whether the procedure declares a retained history read.
    retains: bool,
    cut: Option<Cut>,
    params: BTreeMap<String, f32>,
    /// Declared uniform parameter names, ranges, and defaults.
    declared: Vec<Declared>,
    ops_per_fragment: u32,
}

pub struct Declared {
    pub name: String,
    pub min: f32,
    pub max: f32,
    pub default: f32,
}

impl fmt::Debug for Slot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Slot")
            .field("proc", &self.proc)
            .field("name", &self.name())
            .field("cut", &self.cut)
            .field("params", &self.params)
            .finish_non_exhaustive()
    }
}

impl Slot {
    /// Compiles a single master chain slot against the shared bind group layout.
    ///
    /// Validates that `checked` is an L5 procedure without unbound texture slots,
    /// and that cut retention requirements are satisfied.
    pub fn build(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        proc: impl Into<String>,
        checked: &Checked,
        cut: Option<Cut>,
        params: BTreeMap<String, f32>,
    ) -> Result<Slot, SlotError> {
        let proc = proc.into();
        if checked.kind != karakuri_ir::Kind::L5 {
            return Err(SlotError::NotL5 {
                proc: checked.name.clone(),
                kind: format!("{:?}", checked.kind),
            });
        }
        if let Some(slot) = checked.texture_slots().first() {
            return Err(SlotError::TextureSlot {
                proc: checked.name.clone(),
                slot: (*slot).to_string(),
            });
        }
        match (checked.retains, cut) {
            (false, Some(cut)) => {
                return Err(SlotError::CutWithoutRetains {
                    proc: checked.name.clone(),
                    cut,
                })
            }
            (true, None) => {
                return Err(SlotError::RetainsWithoutCut {
                    proc: checked.name.clone(),
                })
            }
            _ => {}
        }

        let (pass, _shader) = ImagePass::from_l5(device, layout, checked);
        let ops_per_fragment = karakuri_ir::cost::estimate(checked)
            .map(|c| u32::try_from(c.ops_per_fragment).unwrap_or(u32::MAX))
            .unwrap_or(0);

        Ok(Slot {
            proc,
            pass,
            retains: checked.retains,
            cut,
            params,
            declared: checked
                .params
                .iter()
                .map(|p| Declared {
                    name: p.name.clone(),
                    min: p.min,
                    max: p.max,
                    default: p.default_scalar().unwrap_or(0.0),
                })
                .collect(),
            ops_per_fragment,
        })
    }

    /// The content address of this slot's procedure.
    pub fn proc(&self) -> &str {
        &self.proc
    }

    /// The procedure's declared name.
    pub fn name(&self) -> &str {
        self.pass.name()
    }

    /// Access the underlying unified image pass abstraction.
    pub fn pass(&self) -> &ImagePass {
        &self.pass
    }

    /// Pair this slot's pass with an active bind group to form an executable [`RenderPassNode`].
    pub fn bound<'a>(&'a self, bind_group: &'a wgpu::BindGroup) -> BoundImagePass<'a> {
        self.pass.bound(bind_group)
    }

    /// Which cut this slot answers, where its procedure declares `retains`.
    pub fn cut(&self) -> Option<Cut> {
        self.cut
    }

    /// Whether this slot reads a retained history frame.
    pub fn retains(&self) -> bool {
        self.retains
    }

    /// What this slot's fragment costs, per texel.
    pub fn ops_per_fragment(&self) -> u32 {
        self.ops_per_fragment
    }

    /// Returns parameter values clamped to their declared ranges.
    pub fn resolved(&self) -> BTreeMap<String, f32> {
        self.declared
            .iter()
            .map(|d| {
                let asked = self.params.get(&d.name).copied().unwrap_or(d.default);
                let value = if asked.is_nan() {
                    d.min
                } else {
                    asked.clamp(d.min, d.max)
                };
                (d.name.clone(), value)
            })
            .collect()
    }

    /// Replaces this slot's parameters.
    fn set_params(&mut self, params: BTreeMap<String, f32>) {
        self.params = params;
    }

    /// Writes clock, viewport, and parameters into the slot's uniform buffer.
    fn write_uniform(&mut self, queue: &wgpu::Queue, clock: Clock, viewport: [f32; 2]) {
        let resolved = self.resolved();
        let declared = &self.declared;
        let params: Vec<(&str, f32)> = declared
            .iter()
            .map(|d| {
                (
                    d.name.as_str(),
                    resolved.get(&d.name).copied().unwrap_or(d.default),
                )
            })
            .collect();
        self.pass.write_uniform(queue, clock, viewport, params);
    }

    /// Updates only the clock portion of the slot's uniform buffer.
    fn write_clock(&self, queue: &wgpu::Queue, clock: Clock) {
        self.pass.write_clock(queue, clock);
    }

    /// Binds source, held, and sampler into the slot's bind group.
    fn bind(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        src: &wgpu::TextureView,
        held: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        self.pass.bind(
            device,
            layout,
            src,
            held,
            sampler,
            Some("master chain slot"),
        )
    }
}

/// Configuration snapshot from [`Present`](crate::Present) required for background chain compilation.
#[derive(Clone)]
pub struct ChainWorkshop {
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    retention_layout: wgpu::BindGroupLayout,
    /// The view the `exit` cut is held from — the `Present`'s HDR target.
    out: wgpu::TextureView,
    at: (u32, u32),
}

impl ChainWorkshop {
    /// The bind group layout every chain slot's pipeline is built against.
    pub fn layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }

    /// The frame size this workshop's targets are made at.
    pub fn at(&self) -> (u32, u32) {
        self.at
    }
}

/// Intermediate GPU render targets, ping-pong textures, and bind groups for a master chain.
pub struct ChainTargets {
    at: (u32, u32),
    entry: Option<wgpu::TextureView>,
    ping: Vec<wgpu::TextureView>,
    binds: Vec<wgpu::BindGroup>,
    retained: Retained,
}

impl ChainTargets {
    /// Allocates the entry target, the ping-pong targets, the retention
    /// history and one bind group per slot, for `slots` at the workshop's size.
    ///
    /// An empty slot list allocates nothing.
    pub fn build(device: &wgpu::Device, workshop: &ChainWorkshop, slots: &[Slot]) -> ChainTargets {
        let total = slots.len();
        let (width, height) = workshop.at;
        if total == 0 {
            return ChainTargets {
                at: workshop.at,
                entry: None,
                ping: Vec::new(),
                binds: Vec::new(),
                retained: Retained::default(),
            };
        }
        let entry = create_hdr_target(device, width, height, Some("master chain entry"));
        let pings = (total - 1).min(2);
        let ping: Vec<wgpu::TextureView> = (0..pings)
            .map(|i| {
                create_hdr_target(
                    device,
                    width,
                    height,
                    Some(&format!("master chain ping {i}")),
                )
            })
            .collect();
        let cuts: Vec<Cut> = {
            let mut c: Vec<Cut> = slots.iter().filter_map(|s| s.cut).collect();
            c.sort_unstable();
            c.dedup();
            c
        };
        let retained = RetentionManager::build(
            &workshop.retention_layout,
            device,
            width,
            height,
            &cuts,
            Some(&entry),
            Some(&workshop.out),
        );
        let held_for = |at: usize, src: &wgpu::TextureView| -> wgpu::BindGroup {
            let held: &wgpu::TextureView = slots[at]
                .cut
                .and_then(|cut| retained.held(cut))
                .unwrap_or(src);
            slots[at].bind(device, &workshop.layout, src, held, &workshop.sampler)
        };
        let binds: Vec<wgpu::BindGroup> = (0..total)
            .map(|at| {
                let src = if at == 0 {
                    &entry
                } else {
                    &ping[(at - 1) % ping.len().max(1)]
                };
                held_for(at, src)
            })
            .collect();
        ChainTargets {
            at: workshop.at,
            entry: Some(entry),
            ping,
            binds,
            retained,
        }
    }

    /// The frame size these targets were allocated at.
    pub fn at(&self) -> (u32, u32) {
        self.at
    }
}

/// Retired GPU pipelines and intermediate textures scheduled for background deallocation (ADR-0033).
#[derive(Default)]
pub struct RetiredChain {
    slots: Vec<Slot>,
    targets: Vec<ChainTargets>,
}

impl RetiredChain {
    /// Whether this holds nothing, in which case there is nothing to retire.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty() && self.targets.iter().all(|t| t.entry.is_none())
    }
}

impl From<Chain> for RetiredChain {
    /// A chain that was built and never installed is retired exactly as one
    /// that was.
    fn from(chain: Chain) -> RetiredChain {
        RetiredChain {
            slots: chain.slots,
            targets: chain.targets.into_iter().collect(),
        }
    }
}

/// Ordered list of slots comprising the master chain, and — where it was built
/// off the render thread — the targets they run through.
#[derive(Default)]
pub struct Chain {
    slots: Vec<Slot>,
    /// Targets allocated alongside the slots. `None` means the installing
    /// `Present` allocates them, which is an allocation on whichever thread
    /// installs.
    targets: Option<ChainTargets>,
}

impl fmt::Debug for Chain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Chain")
            .field("slots", &self.slots)
            .field("targets", &self.targets.as_ref().map(ChainTargets::at))
            .finish()
    }
}

impl Chain {
    /// Creates a master chain from the given slots, with no targets.
    ///
    /// The `Present` this is installed on allocates them.
    pub fn new(slots: Vec<Slot>) -> Chain {
        Chain {
            slots,
            targets: None,
        }
    }

    /// Creates a master chain from slots and the targets built beside them.
    ///
    /// The targets are used only where their size still matches the `Present`
    /// at install time; a resize in between retires them unused.
    pub fn resident(slots: Vec<Slot>, targets: ChainTargets) -> Chain {
        Chain {
            slots,
            targets: Some(targets),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn slots(&self) -> &[Slot] {
        &self.slots
    }

    /// Returns the sum of fragment operations per texel across all slots.
    pub fn ops_per_fragment(&self) -> u32 {
        self.slots
            .iter()
            .fold(0u32, |sum, s| sum.saturating_add(s.ops_per_fragment))
    }

    /// Returns identifying procedure addresses and cuts for all slots in order.
    pub fn shape(&self) -> Vec<(&str, Option<Cut>)> {
        self.slots
            .iter()
            .map(|s| (s.proc.as_str(), s.cut))
            .collect()
    }

    /// Returns deduplicated cuts requested by slots in this chain.
    pub fn cuts(&self) -> Vec<Cut> {
        let mut cuts: Vec<Cut> = self.slots.iter().filter_map(|s| s.cut).collect();
        cuts.sort_unstable();
        cuts.dedup();
        cuts
    }

    /// Maximum allowed feedback gain factor (0.95) to prevent unbounded accumulation.
    pub const FEEDBACK_MAX: f32 = 0.95;
}

/// GPU target textures, bind groups, and pipelines for the master chain.
pub(crate) struct MasterChain {
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    /// Retention manager for history buffers and copies.
    retention: RetentionManager,
    slots: Vec<Slot>,
    /// Bind groups per slot in order: `(src, held)`.
    binds: Vec<wgpu::BindGroup>,
    /// Intermediate texture view where mix output is written when the chain is active.
    entry: Option<wgpu::TextureView>,
    /// Intermediate ping-pong texture views between slots.
    ping: Vec<wgpu::TextureView>,
    /// The clock last written to every slot's uniform, repacked whenever a slot's
    /// whole uniform block is rewritten. A frame writes it through
    /// [`MasterChain::set_clock`], which takes `&self` because writing it is a
    /// `queue.write_buffer` into a buffer sized at build and nothing else.
    clock: std::cell::Cell<Clock>,
    width: u32,
    height: u32,
}

impl MasterChain {
    pub(crate) fn new(device: &wgpu::Device, width: u32, height: u32) -> MasterChain {
        let layout = ImagePass::create_bind_group_layout(device, Some("master chain"));
        let sampler = ImagePass::create_sampler(device, Some("master chain"));
        let retention = RetentionManager::new(device);
        MasterChain {
            layout,
            sampler,
            retention,
            slots: Vec::new(),
            binds: Vec::new(),
            entry: None,
            ping: Vec::new(),
            clock: std::cell::Cell::new(Clock::default()),
            width,
            height,
        }
    }

    pub(crate) fn layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }

    /// Resizes intermediate targets to `(w, h)`.
    pub(crate) fn resize(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        out: &wgpu::TextureView,
        w: u32,
        h: u32,
    ) {
        self.width = w;
        self.height = h;
        self.allocate(device, queue, out);
    }

    /// Installs an incoming master chain, returning displaced resources in [`RetiredChain`].
    #[must_use = "the outgoing chain's GPU objects are freed where this is dropped"]
    pub(crate) fn set(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        out: &wgpu::TextureView,
        chain: Chain,
    ) -> RetiredChain {
        let mut retired = RetiredChain {
            slots: std::mem::take(&mut self.slots),
            targets: vec![self.take_targets()],
        };
        self.slots = chain.slots;
        match chain.targets {
            Some(targets) if targets.at == (self.width, self.height) => {
                self.install_targets(queue, targets)
            }
            stale => {
                retired.targets.extend(stale);
                self.allocate(device, queue, out);
            }
        }
        retired
    }

    /// Everything a chain build needs from this `Present`'s chain, in one
    /// value: the two layouts, the sampler, the view the `exit` cut is held
    /// from, and the size to build at.
    pub(crate) fn workshop(&self, out: &wgpu::TextureView) -> ChainWorkshop {
        ChainWorkshop {
            layout: self.layout.clone(),
            sampler: self.sampler.clone(),
            retention_layout: self.retention.layout().clone(),
            out: out.clone(),
            at: (self.width, self.height),
        }
    }

    /// Detaches the running targets, leaving the chain with none.
    fn take_targets(&mut self) -> ChainTargets {
        ChainTargets {
            at: (self.width, self.height),
            entry: self.entry.take(),
            ping: std::mem::take(&mut self.ping),
            binds: std::mem::take(&mut self.binds),
            retained: self.retention.take(),
        }
    }

    /// Adopts intermediate targets and updates uniform blocks across all chain slots.
    fn install_targets(&mut self, queue: &wgpu::Queue, targets: ChainTargets) {
        self.entry = targets.entry;
        self.ping = targets.ping;
        self.binds = targets.binds;
        self.retention.install(targets.retained);
        let viewport = [self.width.max(1) as f32, self.height.max(1) as f32];
        let clock = self.clock.get();
        for slot in &mut self.slots {
            slot.write_uniform(queue, clock, viewport);
        }
    }

    /// Updates parameters for running slots without reallocating textures.
    pub(crate) fn set_params(
        &mut self,
        queue: &wgpu::Queue,
        shape: &[(String, Option<Cut>)],
        params: &[BTreeMap<String, f32>],
    ) -> bool {
        if shape.len() != self.slots.len() || params.len() != self.slots.len() {
            return false;
        }
        if !self
            .slots
            .iter()
            .zip(shape)
            .all(|(slot, (proc, cut))| slot.proc == *proc && slot.cut == *cut)
        {
            return false;
        }
        let viewport = [self.width.max(1) as f32, self.height.max(1) as f32];
        let clock = self.clock.get();
        for (slot, want) in self.slots.iter_mut().zip(params) {
            slot.set_params(want.clone());
            slot.write_uniform(queue, clock, viewport);
        }
        true
    }

    /// Updates the clock uniform across all active chain slots.
    pub(crate) fn set_clock(&self, queue: &wgpu::Queue, clock: Clock) {
        self.clock.set(clock);
        for slot in &self.slots {
            slot.write_clock(queue, clock);
        }
    }

    /// The clock last written to this chain's slots.
    pub(crate) fn clock(&self) -> Clock {
        self.clock.get()
    }

    pub(crate) fn chain_len(&self) -> usize {
        self.slots.len()
    }

    pub(crate) fn shape(&self) -> Vec<(String, Option<Cut>)> {
        self.slots.iter().map(|s| (s.proc.clone(), s.cut)).collect()
    }

    /// Returns what each slot of the running chain is, for a surface to draw.
    pub(crate) fn reading(&self) -> Vec<SlotReading> {
        self.slots
            .iter()
            .map(|s| {
                let resolved = s.resolved();
                SlotReading {
                    procedure: s.proc.clone(),
                    name: s.name().to_string(),
                    cut: s.cut,
                    retains: s.retains,
                    params: s
                        .declared
                        .iter()
                        .map(|d| SlotParam {
                            key: d.name.clone(),
                            min: d.min,
                            max: d.max,
                            value: resolved.get(&d.name).copied().unwrap_or(d.default),
                            default: d.default,
                        })
                        .collect(),
                }
            })
            .collect()
    }

    /// Returns current slot specifications with resolved parameter values.
    pub(crate) fn spec(&self) -> Vec<SlotSpec> {
        self.slots
            .iter()
            .map(|s| SlotSpec {
                procedure: s.proc.clone(),
                cut: s.cut,
                params: s.resolved(),
            })
            .collect()
    }

    pub(crate) fn ops_per_fragment(&self) -> u32 {
        self.slots
            .iter()
            .fold(0u32, |sum, s| sum.saturating_add(s.ops_per_fragment))
    }

    /// Returns cuts currently tracked in the retention manager.
    pub(crate) fn retained(&self) -> Vec<Cut> {
        self.retention.active_cuts()
    }

    /// Returns total count of intermediate textures held by the chain.
    pub(crate) fn targets(&self) -> usize {
        usize::from(self.entry.is_some()) + self.ping.len() + self.retention.count()
    }

    /// Returns intermediate texture view for mix output, or `None` if empty.
    pub(crate) fn entry(&self) -> Option<&wgpu::TextureView> {
        self.entry.as_ref()
    }

    /// Records execution of all chain slots and history copies into `encoder`.
    pub(crate) fn record(&self, encoder: &mut wgpu::CommandEncoder, out: &wgpu::TextureView) {
        let total = self.slots.len();
        if self.entry.is_none() {
            return;
        }
        for (at, slot) in self.slots.iter().enumerate() {
            let target = if at + 1 == total {
                out
            } else {
                &self.ping[at % self.ping.len().max(1)]
            };
            let pass = slot.bound(&self.binds[at]);
            pass.record(encoder, target);
        }

        self.retention.record(encoder);
    }

    /// Allocates intermediate targets, ping-pong views, and slot bind groups on the calling thread.
    fn allocate(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, out: &wgpu::TextureView) {
        let targets = ChainTargets::build(device, &self.workshop(out), &self.slots);
        self.install_targets(queue, targets);
    }
}

impl RenderPassNode for MasterChain {
    fn record(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        self.record(encoder, target);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that all Cut variants round-trip through their string names.
    #[test]
    fn a_cut_is_spelled_one_way_and_read_back() {
        for cut in Cut::ALL {
            assert_eq!(Cut::parse(cut.name()), Some(cut));
        }
        assert_eq!(Cut::parse("previous"), None);
    }

    /// Verifies that an empty chain allocates zero targets and incurs zero ops.
    #[test]
    fn the_default_chain_is_empty() {
        assert!(Chain::default().is_empty());
        assert_eq!(Chain::default().len(), 0);
        assert_eq!(Chain::default().ops_per_fragment(), 0);
        assert!(Chain::default().cuts().is_empty());
    }

    /// Verifies that the feedback ceiling constant is bounded below 1.0.
    #[test]
    fn the_feedback_ceiling_is_twenty_times_the_frame() {
        const { assert!(Chain::FEEDBACK_MAX < 1.0) };
        let ceiling = 1.0 / (1.0 - Chain::FEEDBACK_MAX);
        assert!((ceiling - 20.0).abs() < 0.001, "{ceiling}");
    }

    /// Verifies that slot error display strings provide actionable diagnostic messages.
    #[test]
    fn a_slot_refusal_names_the_fix() {
        let cut = SlotError::CutWithoutRetains {
            proc: "bloom".into(),
            cut: Cut::Exit,
        }
        .to_string();
        assert!(cut.contains("exit") && cut.contains("retains"), "{cut}");

        let bare = SlotError::RetainsWithoutCut {
            proc: "feedback".into(),
        }
        .to_string();
        assert!(bare.contains("mix") && bare.contains("exit"), "{bare}");

        let tex = SlotError::TextureSlot {
            proc: "merge".into(),
            slot: "far".into(),
        }
        .to_string();
        assert!(tex.contains("edge") && tex.contains("far"), "{tex}");

        let kind = SlotError::NotL5 {
            proc: "drift_shell".into(),
            kind: "L1".into(),
        }
        .to_string();
        assert!(kind.contains("kind L5"), "{kind}");
    }
}
