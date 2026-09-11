//! The master chain: an ordered list of L5 slots between the mix and the tone
//! map.
//!
//! # What it is
//!
//! **A list, not three passes.** Each entry is one `kind L5` procedure with its
//! params and — where the procedure declares `retains` — the cut it reads back.
//! `src` of the first slot is what the mix wrote, `src` of every other is what
//! the slot before it wrote, and what the last writes is what the tone map
//! reads. That is
//! `docs/adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md`,
//! and the three passes this module used to hold in WGSL ship as
//! `examples/feedback.kir`, `examples/bloom.kir` and `examples/rgb_shift.kir`.
//!
//! **The default chain is empty**, so the default look is bit-identical for
//! free: with no slot the mix writes straight into the target the present pass
//! reads and this module is not in the frame at all. **A slot that is in the
//! chain runs, at zero as at one** — ADR-0317's zero-skip is not carried
//! forward, because a list spells *no pass* as *no slot*.
//!
//! # Where it sits
//!
//! Between the two multiplications ADR-0224 separated. [`crate::mix`] applies
//! `out` where it **writes** the composited frame, this chain reads that frame,
//! and [`crate::present`] applies `exposure` where the tone mapper **reads**
//! what this chain wrote.
//!
//! Everything here is linear HDR (`Rgba16Float`), unclamped, upstream of the
//! one tone map and the one sRGB encode
//! (`docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md`).
//! Nothing in this module encodes anything, so P-0064's *one call site* is the
//! same one call site.
//!
//! # What it costs, before it is paid
//!
//! **Memory, and only where a chain has slots in it.** The entry — what the mix
//! writes into and what the first slot reads — plus at most two targets the
//! rest ping-pong between, plus **one per retained cut some slot asked for**.
//! 8 bytes a texel, so 7.03 MB each at 1280x720: nothing for an empty chain,
//! 7.03 MB for one slot, 21.1 MB for three, and 7.03 MB more per cut. They are
//! allocated when the list is installed and when the frame is resized, never
//! when a parameter moves
//! (`docs/principles/0091-cost-is-known-before-it-is-paid.md`).
//!
//! **The entry is not one of the ping-pong pair**, and that is what makes the
//! retention rule position-independent. `master.rs` used to copy the `mix` cut
//! *between* two passes, because the mix's target was also the second pass's
//! destination and the copy had to be recorded while it was still true. With
//! the entry held apart, the frame as the mix wrote it survives the whole
//! chain, so both cuts are copied at the chain's end and a slot reading `mix`
//! reads the previous frame's mix wherever it sits in the list. ADR-0317's two
//! copies are the same bytes they were; what is gone is the ordering
//! constraint, which a list could not have honoured.
//!
//! **Time, per slot**: one fullscreen pass at the frame's area, priced on
//! `ops_per_fragment` by `karakuri_ir::cost` before anything is built, and a
//! chain's cost is the sum over its slots — which is not the addition
//! [ADR-0013](../../../docs/adr/0013-cost-has-three-axes-that-must-not-be-added.md)
//! forbids, because these are rates against one quantity. Plus one frame-sized
//! texture copy per retained cut.
//!
//! # Determinism
//!
//! A slot that reads a retained frame reads part of the state a replay has to
//! reproduce
//! (`docs/principles/0092-the-same-inputs-produce-the-same-frame.md`). It is,
//! and by construction rather than by care: the retained frame is a copy of a
//! target this chain wrote on the previous frame, the copy is a
//! `copy_texture_to_texture` and not an arithmetic pass, the parameters come
//! from records, and a freshly allocated target reads as zero — so a run and a
//! replay of the same records see the same history at every frame, including
//! the first.
//!
//! **Nothing here reads a clock.** The clock a `frame` block can read is
//! [`Clock`], written by the host from the session's own time; a chain nobody
//! has handed one to runs at zero, which is what an offscreen render does.

use std::collections::BTreeMap;
use std::fmt;

use karakuri_ir::typed::Checked;

pub use crate::pass::{
    create_hdr_target, BoundImagePass, Clock, Cut, ImagePass, RenderPassNode, RetentionManager,
};

/// **One slot of the master chain, described rather than built.**
///
/// What a record carries — an address, a cut and a map of params — with the
/// cut already read back into [`Cut`]. It is the engine's type rather than the
/// decoder's because it is also what a running chain reads *back* as: a
/// `Present` is the one writer of what the chain is (ADR-0317), so a surface
/// asking what is running asks it, and a copy on a host struct beside it would
/// be the second writer that arrangement exists to refuse.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotSpec {
    pub procedure: String,
    pub cut: Option<Cut>,
    pub params: BTreeMap<String, f32>,
}

/// **Why a procedure cannot be a chain slot.** Each names the fix rather than
/// the rule (P-0083).
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

/// **One slot of the chain, compiled.**
///
/// Built off the render thread — an [`ImagePass`] pipeline and buffer, which is what a Set's
/// build makes on its worker — and installed at a frame boundary by
/// [`Present::set_chain`]. What it does *not* own is a target or a bind group:
/// those name the frame's size, so they belong to the thing that is resized.
pub struct Slot {
    /// **The content address of the procedure's source**, which is what a
    /// record carries and what a slot is recognised by — see [`Chain::shape`].
    proc: String,
    /// The unified fullscreen image pass pipeline and uniform state.
    pass: ImagePass,
    /// **Whether this slot reads a retained frame**, off the procedure's
    /// declaration rather than off the cut: the file says it reads one, the
    /// slot says which.
    retains: bool,
    cut: Option<Cut>,
    params: BTreeMap<String, f32>,
    /// The declaration names, which are the uniform's own field names, with the
    /// range each was declared under — the wall is here, where the value is
    /// applied, so every route in meets the same one (P-0090).
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
    /// **Generate, compile and price one chain slot.**
    ///
    /// `proc` is the content address the record carries; `checked` is what that
    /// address resolves to. `layout` is the chain's own bind group layout —
    /// [`Present::chain_layout`] — taken rather than made here so that a slot
    /// built on a worker binds the targets the `Present` owns.
    ///
    /// **Three refusals, and all three are about the chain rather than about
    /// the language**: a `kind` that is not L5, a `cut` answered where the file
    /// declares no `retains` (and the other way round), and a Texture slot,
    /// which the chain has no `edge` to bind. `docs/ir-spec.md`'s *L5's chain*
    /// is where each is written down.
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
                // `Debug` is the kind's own spelling — `L1`, `Field` — and
                // there is no `Kind::name` to borrow, which is a gap in the IR
                // rather than a decision this module should take.
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
        // **The price, taken from the same estimator every other artifact is
        // priced by.** A chain's cost is the sum of these; a slot that somehow
        // reached here without an estimate is counted as zero rather than
        // guessed at, because stage 4 already refused one over the ceiling.
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

    /// **Whether this slot's procedure reads a retained frame**, off the file's
    /// bare `retains` and not off the cut. The two agree by construction —
    /// [`Slot::build`] refuses each without the other — and both are carried
    /// because they are two different facts: the file's declaration and the
    /// slot's answer.
    pub fn retains(&self) -> bool {
        self.retains
    }

    /// What this slot's fragment costs, per texel.
    pub fn ops_per_fragment(&self) -> u32 {
        self.ops_per_fragment
    }

    /// **The params, as they will be written** — every declared name, brought
    /// into the range its declaration gave it.
    ///
    /// A NaN floors to the declared minimum, which is `Chain::clamped`'s answer
    /// one design back: a value that cannot be compared is not a value, and the
    /// bottom of the range is the one an operator can see is wrong.
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

    /// Replace this slot's params. A `queue.write_buffer` at the caller, and no
    /// allocation: the buffer was sized when the slot was built.
    fn set_params(&mut self, params: BTreeMap<String, f32>) {
        self.params = params;
    }

    /// **This slot's whole uniform block**: the clock, the frame's size, and
    /// its own params.
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

    /// **The clock alone, at the offsets the layout gives it.**
    fn write_clock(&self, queue: &wgpu::Queue, clock: Clock) {
        self.pass.write_clock(queue, clock);
    }

    /// Bind source, held, and sampler into the slot's bind group.
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

/// **The whole of what the master chain is**: an ordered list of slots.
///
/// Not `Copy` and not `Clone`, which is the shape a list of compiled pipelines
/// has: installing a chain moves it into the [`Present`], and what a surface or
/// a record carries is the *description* — an address, a cut and a map of
/// params per slot — which is `karakuri_store::record::Record::MasterChain`.
#[derive(Debug, Default)]
pub struct Chain {
    slots: Vec<Slot>,
}

impl Chain {
    /// **Nothing, and the frame is the frame with no chain in it.**
    pub fn new(slots: Vec<Slot>) -> Chain {
        Chain { slots }
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

    /// **What the chain costs per texel**, which is the sum over its slots.
    ///
    /// That is not the addition ADR-0013 forbids: those are three different
    /// quantities, these are rates against one — the frame's texels, covered
    /// once by every slot. It is said out loud because it looks like the
    /// forbidden thing.
    pub fn ops_per_fragment(&self) -> u32 {
        self.slots
            .iter()
            .fold(0u32, |sum, s| sum.saturating_add(s.ops_per_fragment))
    }

    /// **What identifies this list, ignoring what its params are set to.**
    ///
    /// The pair a slot is recognised by — its procedure's address and its cut —
    /// so that a chain arriving with the same procedures in the same order is a
    /// parameter move and not a rebuild. That is the whole of how P-0091's
    /// *allocated at build and at resize, never when a parameter moves* is kept
    /// with a list that is written whole.
    pub fn shape(&self) -> Vec<(&str, Option<Cut>)> {
        self.slots
            .iter()
            .map(|s| (s.proc.as_str(), s.cut))
            .collect()
    }

    /// The cuts some slot in this list asked for — at most two, and each is a
    /// frame-sized target the engine then holds.
    pub fn cuts(&self) -> Vec<Cut> {
        let mut cuts: Vec<Cut> = self.slots.iter().filter_map(|s| s.cut).collect();
        cuts.sort_unstable();
        cuts.dedup();
        cuts
    }

    /// **The most feedback there is, and it is short of 1.0 on purpose.**
    ///
    /// Kept as the vocabulary's number rather than the engine's wall: the wall
    /// is now the range `examples/feedback.kir` declares, and this is what
    /// `karakuri_operation::Feedback::MAX` is held against by the one package
    /// that depends on both. Under `Cut::Exit` a slot that adds `a` of its own
    /// output is an accumulator — `frame / (1 - a)` on material that is not
    /// moving — so at 0.95 the ceiling is twenty times the frame, which a tone
    /// mapper has an answer for.
    pub const FEEDBACK_MAX: f32 = 0.95;
}

/// The chain's targets, its bind groups and the slots that read them.
pub(crate) struct MasterChain {
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    /// **The retention manager**, which manages history targets and sanitized retention passes.
    retention: RetentionManager,
    slots: Vec<Slot>,
    /// One per slot, in list order: `(src, held)` resolved for that position.
    binds: Vec<wgpu::BindGroup>,
    /// **Where the mix writes**, held apart from the ping-pong so that the
    /// frame as the mix wrote it survives the whole chain — see the module doc.
    /// `None` for an empty chain, which is what makes the default look cost
    /// nothing at all.
    entry: Option<wgpu::TextureView>,
    /// What the slots after the first ping-pong between: none for a chain of
    /// one, one for a chain of two, two for anything longer.
    ping: Vec<wgpu::TextureView>,
    clock: Clock,
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
            clock: Clock::default(),
            width,
            height,
        }
    }

    pub(crate) fn layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }

    /// Reallocation, so never from the render thread mid-frame — the same shape
    /// as [`Present::resize`], which is its one caller.
    ///
    /// **A retained frame does not survive it**, and it is stated rather than
    /// fixed: the new one is a new texture and reads as zero, so a trail starts
    /// again from the frame after a resize. Scaling the old one would be
    /// inventing texels a replay would have to reproduce exactly.
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

    /// **Install a list.** Allocates the targets the list needs, binds each
    /// slot to its position, and writes every uniform.
    ///
    /// **This is a build and it is spelled as one.** It is where the chain's
    /// memory is taken and given back, which is why a parameter move does not
    /// come through here — see [`MasterChain::set_params`].
    pub(crate) fn set(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        out: &wgpu::TextureView,
        chain: Chain,
    ) {
        self.slots = chain.slots;
        self.allocate(device, queue, out);
    }

    /// **A parameter move: uniforms and nothing else.**
    ///
    /// Refuses — `false`, and the caller installs a list instead — where the
    /// shape it was handed is not the shape that is running. That is what keeps
    /// the whole-record shape of `Record::MasterChain` from costing an
    /// allocation every time a fader lands on one of its slots (P-0091).
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
        let clock = self.clock;
        for (slot, want) in self.slots.iter_mut().zip(params) {
            slot.set_params(want.clone());
            slot.write_uniform(queue, clock, viewport);
        }
        true
    }

    /// The clock the chain's `frame` blocks read. A write per slot into storage
    /// sized at build, so it is safe on the render thread and costs what the
    /// tone map's own per-frame write costs.
    pub(crate) fn set_clock(&mut self, queue: &wgpu::Queue, clock: Clock) {
        self.clock = clock;
        for slot in &self.slots {
            slot.write_clock(queue, clock);
        }
    }

    pub(crate) fn chain_len(&self) -> usize {
        self.slots.len()
    }

    pub(crate) fn shape(&self) -> Vec<(String, Option<Cut>)> {
        self.slots.iter().map(|s| (s.proc.clone(), s.cut)).collect()
    }

    /// **The running chain, described** — what it would be recorded as. The
    /// params are [`Slot::resolved`]'s, so what comes back is what is actually
    /// running rather than what was asked for, which is the reading a record is
    /// completed from.
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

    /// **Which cuts are actually held**, which is a fact about memory and is
    /// readable so that a test can assert on it: a retention is a frame-sized
    /// target, and *only where a slot's answer names one, at most two ever* is
    /// the rule P-0091 is met by.
    pub(crate) fn retained(&self) -> Vec<Cut> {
        self.retention.active_cuts()
    }

    /// How many frame-sized targets this chain is holding — the entry, the
    /// ping-pong pair and the retentions. Zero for an empty chain, which is
    /// what makes the default look cost nothing at all.
    pub(crate) fn targets(&self) -> usize {
        usize::from(self.entry.is_some()) + self.ping.len() + self.retention.count()
    }

    /// **Where the mix writes**: this chain's entry when it holds a slot, and
    /// `None` when it does not — in which case the caller hands the mix the
    /// present pass's own target and this module is not in the frame at all.
    pub(crate) fn entry(&self) -> Option<&wgpu::TextureView> {
        self.entry.as_ref()
    }

    /// **Record the chain**, from the entry into `out`.
    ///
    /// Nothing at all for an empty chain, so the caller's `out` is what the mix
    /// already wrote into. Each slot is recorded via the unified [`RenderPassNode`] abstraction.
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

        // **Both cuts are copied here, once every pass has run**, sanitized through
        // the unified retention manager.
        self.retention.record(encoder);
    }

    /// **Every target this list needs and no other**, plus the bind groups over
    /// them and every slot's uniform.
    ///
    /// Called from exactly two places — a list being installed and a resize —
    /// which is P-0091's *at build and at resize and never when a parameter
    /// moves* held by there being nowhere else to call it from.
    fn allocate(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, out: &wgpu::TextureView) {
        let total = self.slots.len();
        if total == 0 {
            self.entry = None;
            self.ping.clear();
            self.retention.clear();
            self.binds.clear();
            return;
        }
        self.entry = Some(create_hdr_target(
            device,
            self.width,
            self.height,
            Some("master chain entry"),
        ));
        // One intermediate per slot that is not the last, capped at two: with
        // three or more the pair is ping-ponged between, and with one the slot
        // writes straight into the present pass's target.
        let pings = (total - 1).min(2);
        self.ping = (0..pings)
            .map(|i| {
                create_hdr_target(
                    device,
                    self.width,
                    self.height,
                    Some(&format!("master chain ping {i}")),
                )
            })
            .collect();
        let cuts: Vec<Cut> = {
            let mut c: Vec<Cut> = self.slots.iter().filter_map(|s| s.cut).collect();
            c.sort_unstable();
            c.dedup();
            c
        };

        let entry_view: &wgpu::TextureView = self.entry.as_ref().expect("just allocated");
        self.retention.allocate(
            device,
            self.width,
            self.height,
            &cuts,
            Some(entry_view),
            Some(out),
        );

        self.binds = (0..total)
            .map(|at| {
                let src = if at == 0 {
                    entry_view
                } else {
                    &self.ping[(at - 1) % self.ping.len().max(1)]
                };
                // **`src` bound twice where the slot reads no history**, which
                // is the composite's own trick and is why there is one layout
                // here rather than two: the module does not name binding 2, so
                // what is behind it is never read.
                let held: &wgpu::TextureView = self.slots[at]
                    .cut
                    .and_then(|cut| self.retention.held(cut))
                    .unwrap_or(src);
                self.slots[at].bind(device, &self.layout, src, held, &self.sampler)
            })
            .collect();

        let viewport = [self.width.max(1) as f32, self.height.max(1) as f32];
        let clock = self.clock;
        for slot in &mut self.slots {
            slot.write_uniform(queue, clock, viewport);
        }
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

    /// **Every cut has a name and the name round-trips**, which is what a
    /// record carries.
    #[test]
    fn a_cut_is_spelled_one_way_and_read_back() {
        for cut in Cut::ALL {
            assert_eq!(Cut::parse(cut.name()), Some(cut));
        }
        assert_eq!(Cut::parse("previous"), None);
    }

    /// **A chain nobody has put a slot in draws nothing**, which is the whole
    /// of what makes the default look unchanged: not a pass that multiplies by
    /// zero, but no pass.
    #[test]
    fn the_default_chain_is_empty() {
        assert!(Chain::default().is_empty());
        assert_eq!(Chain::default().len(), 0);
        assert_eq!(Chain::default().ops_per_fragment(), 0);
        assert!(Chain::default().cuts().is_empty());
    }

    /// **Feedback stops short of 1.0** — the number the vocabulary is held
    /// against. `1 / (1 - a)` is the ceiling on still material under the exit
    /// cut.
    #[test]
    fn the_feedback_ceiling_is_twenty_times_the_frame() {
        const { assert!(Chain::FEEDBACK_MAX < 1.0) };
        let ceiling = 1.0 / (1.0 - Chain::FEEDBACK_MAX);
        assert!((ceiling - 20.0).abs() < 0.001, "{ceiling}");
    }

    /// **Each refusal names the fix rather than the rule**, which is P-0083 at
    /// the width of one message.
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
