//! The deck: several Sets resident at once, one to four of them composited.
//!
//! `docs/roadmap.md` names this once, in M2, so that Priming Sets, M5's
//! staging lane, M6's agent pool and M7's scheduled material are all the same
//! waiting area seen from different angles. This is its first slice and it is
//! deliberately narrow: N slots, each owning a [`HotSwap`], each Live slot
//! rendering into its own HDR target, and one composite pass folding those
//! targets together with a per-slot gain, opacity and [`Blend`] mode.
//!
//! What is **not** here, and is not half-here either: audio, MIDI, transitions,
//! and masks. Priming and the budget governor have since landed, and they are
//! the only automatic thing on this deck: gain, opacity, blend mode and what
//! goes on air stay manual — see "Residency: requested and effective" below for
//! what [`Deck::govern`] is and is not allowed to move.
//!
//! ## One target per slot
//!
//! Every Live slot renders into its own `Rgba16Float` target and the composite
//! reads them all. Rendering every Set into one shared target would have worked
//! while `add` was the only mode, because addition does not care who went
//! first. It stops working at [`Blend::Over`], which is a question about one
//! source against the accumulated rest, and a mix that has already been summed
//! cannot be taken apart again. Per-slot preview — M2's "live preview of any
//! slot's output" — wants the same thing.
//!
//! The bill for that, stated so it is on the record rather than rediscovered:
//! **four targets at 1280x720 `Rgba16Float` is 8 bytes a texel, 7.03 MB each,
//! 28.1 MB for the deck.** Plus whatever the present pass holds for the mix
//! itself. That is small next to the element buffers a Set at capacity 262144
//! carries, and it scales with slot count rather than with capacity, so the
//! VRAM budget M2's governor gets is dominated by the Sets and not by this.
//!
//! ## The frame guard
//!
//! `docs/roadmap.md` records this under M2's **Demands on earlier work**:
//!
//! > **One `begin_frame` per encoder has to become structural before there are
//! > several Sets.** [...] with a deck compositing up to four and priming the
//! > rest it is a frame built from two different simulations.
//!
//! Here is where it comes due, so [`Deck::begin_frame`] owns the encoder. It
//! returns a [`Frame`] holding `&mut Deck` *and* the `CommandEncoder`, and the
//! frame is recorded through that.
//!
//! **What that enforces, exactly:** while a `Frame` is alive it holds the only
//! `&mut Deck` there is, so `deck.begin_frame(..)` a second time does not
//! compile — and because the encoder lives inside the guard, there is no way
//! to reach a second generation of Sets and record it into the encoder the
//! first generation is being recorded into. Builds are installed once, at the
//! top of [`Deck::begin_frame`], before the encoder exists. The `compile_fail`
//! doc test on [`Deck::begin_frame`] is the assertion, with a `no_run` twin
//! beside it that differs only by the second borrow — so that the negative
//! cannot pass for an unrelated reason without the positive failing too.
//!
//! **What it does not enforce, stated because the alternative is a comment
//! that reads stronger than the code:**
//!
//! - It does not stop a caller recording *other* work into the frame's
//!   encoder through [`Frame::encoder`]. That is what the encoder is handed
//!   out for: the present pass has to go somewhere. What cannot get in there
//!   is a second frame's Sets.
//! - It does not make an early return safe by making it lossless in some other
//!   way — [`Frame`] submits on drop, so a frame abandoned halfway is
//!   submitted as far as it got rather than silently thrown away. Dropping the
//!   guard is ending the frame, not cancelling it.
//! - It says nothing about two *different* `Deck`s. Each has its own encoder
//!   and neither can record into the other's, so there is nothing to enforce,
//!   but "one open frame per process" is not the claim.
//! - [`HotSwap::begin_frame`] is still callable on its own, and is still a
//!   convention when it is. `karakuri-cli` and `tests/hot_swap.rs` use it
//!   directly; the guard is the deck's, not the swap's.
//! - **`mem::forget` on a [`Frame`] corrupts a Set permanently**, and this is
//!   the one entry here that is worse than losing a frame. Forgetting the
//!   guard ends the `&mut Deck` borrow — so the next `begin_frame` opens
//!   immediately — and drops the encoder unsubmitted. But the frame's effects
//!   are already half applied: `Set::prepare` has bumped `steps_taken` and its
//!   `queue.write_buffer`s went to the *queue*, not the encoder, so they land;
//!   and `Set::render` has already flipped `self.parity` on the host while the
//!   compute passes that were supposed to justify that flip are discarded.
//!   From then on `t`, parity and the element buffers disagree and L4 reads
//!   the wrong buffer every frame. Nothing closes this without moving the
//!   parity flip and the clock to where the submission is acknowledged, which
//!   is a redesign of `Set`, not of this guard. `mem::forget` is safe Rust and
//!   this is a real hole; it is named rather than hidden.
//!
//! ## Residency: requested and effective
//!
//! All three of the roadmap's levels are here now. [`Residency::Live`] is
//! stepped and composited; [`Residency::Priming`] is stepped and **not drawn**;
//! [`Residency::Allocated`] is compiled, buffers held, not stepping.
//!
//! **A slot has two of them, and keeping them apart is the whole of this
//! section.** They are different facts about different actors and the word
//! "residency" on its own used to mean both, which is how a demotion came to
//! destroy an operator's request:
//!
//! | | Who writes it | What it says |
//! |---|---|---|
//! | **Requested** | The operator, through [`Deck::set_residency`], and nothing else | What this slot is *for*: leave it off, warm it up, put it on air |
//! | **Effective** | [`Deck::govern`], from the request and the budget | What the engine is doing with it *this frame* |
//!
//! Read them with [`Deck::requested_residency`] and [`Deck::residency`]. The
//! frame loop reads the effective one and only the effective one; every
//! decision about what the deck is *for* reads the request.
//!
//! The effective residency is never more expensive than the request: the
//! governor may hold a slot below what was asked for, and may never put one
//! above it. Two consequences worth stating because the code depends on both:
//! effective Live and requested Live are the same set of slots, since nothing
//! demotes a Live slot and nothing promotes into Live; and a slot whose request
//! is Priming and whose effective residency is Allocated is **parked** —
//! waiting for room, not switched off.
//!
//! Parked is the state that has to survive, and it survives by *not being
//! stored*: the effective residency is recomputed from the request on every
//! [`Deck::govern`] pass, so a slot parked because the deck was full primes
//! again on the first pass after the deck empties, with no operator action and
//! nothing remembered. A transient over-budget frame therefore defers every
//! priming request and cancels none of them. [`Deck::is_parked`] is how a
//! status line tells that from a slot the operator actually turned off, and
//! showing them the same way is showing an operator a request they never
//! withdrew as though they had.
//!
//! The property underneath all three, and the reason the set of levels is this
//! set: **`t` advances through [`Set::prepare`](crate::set::Set::prepare) and
//! nowhere else.** A slot that is not handed one does not move. So Allocated
//! keeps its state and a slot taken off-air and put back resumes at the `t` it
//! stopped at rather than restarting — `swap.rs` already leans on exactly this
//! for rollback, parking the outgoing Set unstepped so it comes back where it
//! was left.
//!
//! ## Priming steps; it does not draw
//!
//! `docs/roadmap.md` describes Priming as "rendering hidden at reduced rate and
//! resolution". **That wording is superseded and this is the implementation, so
//! the reasoning belongs here.**
//!
//! What needs warming is per-element state, and **L1 owns all of it**. An L1
//! procedure's attributes live in the element buffers and are the only thing a
//! simulation accumulates; L4 is stateless by construction — the roadmap's own
//! layer table says so, `Set`'s L4 pipeline holds no per-element storage it
//! writes, and every frame it reads whatever L1 last wrote and throws the
//! result at a target. So a Set is warm exactly when its element buffers are
//! warm, and reaching that state needs [`Set::prepare`] and the L1 passes and
//! nothing else.
//!
//! A Priming slot therefore runs `prepare` and [`Set::step`], and **skips the
//! render entirely**. Not a smaller target: no target. There is no resolution
//! to reduce, no draw to pay for, and no composite term — the mix skips
//! anything that is not Live, which it already did for Allocated. That is
//! strictly cheaper than the roadmap's version and it warms exactly the same
//! state, because the state was never in the pixels.
//!
//! It also removes a trap the roadmap's version carries: a Set primed at a
//! reduced resolution is a Set whose L4 ran at a different `point_size`-to-pixel
//! ratio, and if any of that ever fed back into state the primed result would
//! not be the result of having been Live. Skipping the draw makes "primed for
//! thirty frames, then Live" *identical* to "Live for thirty frames" rather
//! than close to it, and `tests/priming.rs` asserts that bit for bit.
//!
//! **"Reduced rate" survives and means something real.** A Priming slot may
//! step on only some frames — one in `n`, decided by [`crate::governor`] from
//! the budget. Its `t` then advances slower than wall time, which is correct
//! and is the whole point: a priming slot is catching up, not keeping time.
//! Nothing downstream can tell a slot that primed slowly from one that primed
//! fast, because `t` is `steps_taken * dt` and both arrive at a given `t`
//! having taken the same steps.
//!
//! **That took one repair, and a Set with a signal binding is where it showed.**
//! A Priming slot used to be handed the *session's* [`Signals`] at this frame's
//! phase, because that is what every slot in the frame was handed. A slot
//! stepping one frame in `n` has taken a fraction of the session's steps, so
//! its `t` and the phase driving it came apart, and a bound param was sampled
//! at instants the slot's own clock never reached — "primed at one-in-two then
//! Live" was *not* bit-identical to "always Live" for a Set with a binding, and
//! `spawn_rate` is a bound param in the ir-spec's own answer to irregular
//! spawning, so it reached the population and not only the look.
//!
//! What closes it is a slot-local view of the one oscillator — the session's
//! grid read at the slot's own position along it, not a second oscillator.
//! [`Set::prepare_warming`](crate::set::Set::prepare_warming) is that view and
//! carries the argument; the position is expressed as a **lag in steps** rather
//! than as a time, so a slot that is not behind reads the session's oscillator
//! bit for bit and the identity is exact rather than nearly exact.
//!
//! Two things that repair does not reach, both named where the code is: a
//! tempo *correction* during a slow warm-up, since the grid is read as it
//! stands rather than as it was, and measured audio, which has no past value to
//! be read at. `tests/priming.rs` covers rate-invariance against a bound Set
//! and the full-rate identity against a Live one — the second is the test that
//! says which instant is the right one, and the first cannot see it.
//!
//! What Priming is *not* is a preview. Seeing a slot before it goes on air is
//! M2's "live preview of any slot's output" and wants a target and a draw; this
//! is the warming, and the two are separate features that a single "render
//! hidden" would have conflated.
//!
//! ## Determinism
//!
//! Every Live slot is advanced by the same `steps`, from the same tick, in the
//! same frame. Two runs with the same tick sequence and the same seeds
//! composite to the same pixels, and that depends on the *order* of the sum as
//! much as on the values: floating-point addition is not associative, which is
//! why this repository has order-preserving compaction at all.
//!
//! Priming is inside that invariant rather than beside it. Which frames a
//! Priming slot steps on is `frames_primed % prime_one_in`, counted from the
//! frame it entered Priming — engine state advanced by the same frame loop
//! everything else is, with no clock anywhere in it. The rate itself comes from
//! [`crate::governor`], whose inputs are measurements taken at build time and
//! residencies, and which visits slots in index order. So a record stream that
//! primes a slot for a while and then puts it on air reproduces exactly, which
//! `tests/priming.rs` asserts — the priming half of it. The governor's index
//! order is asserted separately, in `tests/governor.rs`, because `govern` is
//! not on the frame path and a bit-for-bit render comparison cannot see it.
//!
//! So the compositing order is the slot index, and nothing else. Slots live in
//! a `Vec` and are visited by index; there is no `HashMap` in the path, and
//! which slot last had a build land on it is not observable from the mix. The
//! shader folds its four terms unrolled, in slot order, for the same reason —
//! see `shaders/composite.wgsl`.
//!
//! **[`Blend::Over`] raises the stakes on that ordering rather than changing
//! it.** Under `add` the order was a rounding question and the answer was
//! nearly the same either way; under `over` a layer that covers is a layer that
//! hides, so slot order is now visible in the picture and not only in the last
//! bits. It is the same order it always was.
//!
//! ## The mix: gain, opacity, blend
//!
//! Three per-slot controls, and the reason they are three is [`Blend`]. Every
//! mode is `acc <- mix(acc, f(acc, gain * src), opacity)`: **`gain` is the
//! level the material arrives at**, touching colour alone, and **`opacity` is
//! the fader across the blend**, the only one of the two that scales what a
//! layer covers. Under `add` they collapse into one multiply, which is why the
//! deck carried them as one number for as long as `add` was all there was.
//!
//! **One exception, and it is the skip rather than the arithmetic.** A silenced
//! layer is not composited at all, coverage included — so a slot under `add` at
//! gain `1e-30` contributes its coverage to the mix's alpha and the same slot
//! at gain `0.0` does not. Alpha is therefore discontinuous in gain at exactly
//! zero. That is what silence means and it is the price of the skip being
//! total; nothing reads the mix's alpha today, and when M2's output routing
//! does, this is the sentence it needs.
//!
//! A slot at silence is skipped rather than blended at zero, because `0.0 *
//! NaN` is NaN and material generated by an LLM will be broken sometimes.
//! [`Blend::silent_at`] decides what silence is per mode, and the asymmetry it
//! records is worth knowing at the keyboard: **`opacity` at zero silences under
//! every mode; `gain` at zero does not silence `over`**, because an `over`
//! layer at zero gain is a black card and a black card covers.
//!
//! ## Signals
//!
//! **One local oscillator per session, and this is where it lives.** The
//! invariants call it the single source of truth for phase and tempo, so there
//! cannot be one per Set: four Sets would be four truths, and a beat would
//! land at four instants. The deck is the smallest thing that is one per
//! session and already has the frame, so [`Deck`] owns a [`Signals`] and
//! [`Frame::render`] advances it once, by the same `steps` every Live slot is
//! advanced by.
//!
//! That placement is what puts bindings inside the determinism invariant
//! rather than beside it: the oscillator's only input is the tick sequence,
//! the noise seed is explicit, and nothing here reads a clock — so the same
//! record stream and the same seed produce the same bound parameter values,
//! bit for bit, on every run.
//!
//! An `Allocated` slot is not prepared, so it reads no signals while it is off
//! air and its `t` stands still — but the session clock does not stop, because
//! it is the session's rather than the slot's. A slot brought back on air
//! rejoins the beat the rest of the deck is on rather than resuming a phase of
//! its own, which is the right answer for the same reason the tempo is shared.
//!
//! A `Priming` slot is the one case that reads the grid somewhere other than
//! the session's position on it. It steps on some frames and not others, so its
//! `t` falls behind at a rate the *governor* chose, and giving it the session's
//! phase would make that rate decide what it warms into. It reads the same
//! oscillator, held back by the steps it has not taken —
//! [`Set::prepare_warming`](crate::set::Set::prepare_warming) — and rejoins the
//! session's position the moment it goes on air, where nothing sees the jump
//! because the frame before was not drawn. A slot that is not behind is held
//! back by nothing and reads the session's oscillator itself, which is what
//! keeps the identity above bit-exact for bound material.
//!
//! ## Transport
//!
//! A Live slot's clock need not be the session's. [`crate::transport`] holds
//! the mapping and the argument for it; what the deck owes is that the mapping
//! is consulted in exactly one place — the Live branch of [`Frame::render`] —
//! and that a slot nobody has arranged reads [`Sync::Free`], which is the
//! session's own step count and is what every slot did before the module
//! existed.
//!
//! **Priming deliberately has no transport.** A warming slot is catching up on
//! a rate the *governor* chose, which is a different mechanism answering a
//! different question, and two things in charge of one clock is how a slot ends
//! up somewhere neither of them meant. The transport applies when a slot is on
//! air and at no other time.
//!
//! ## Level metering
//!
//! A deck can measure what each Live slot's target actually puts out — mean and
//! peak luminance, per slot, per frame — which is what a fader needs to mean
//! anything. It is off until [`Deck::enable_meters`] is called and costs
//! nothing at all until then, because an offscreen `--render` has no use for a
//! meter. The measurement never waits, so it lags; what it does and does not
//! promise is in [`crate::meter`], including what an off-air slot reads and
//! why. Nothing here acts on the number: gain is manual.
//!
//! Three things retire a slot's reading, and they are one thing: the meter
//! stops being able to vouch that what it measured is what the slot is
//! showing. Going off air, a resize, and **a build landing on the slot** — a
//! swap installs a cold Set and a rollback restores a parked one, and either
//! way the next frame's image has nothing to do with the last one's. All three
//! are handled where they happen: [`Deck::set_residency`], [`Deck::resize`],
//! and [`Deck::begin_frame`].

use crate::binding::Signals;
use crate::governor::{Governor, Report, SlotState};
use crate::meter::{Level, Meters};
use crate::present::Present;
use crate::probe::Probe;
use crate::set::{DT, MAX_STEPS};
use crate::swap::{Event, HotSwap, PROBE_RESOLUTION};
use crate::transport::{Advance, Sync, Transport};
use crate::video_source::VideoSource;

/// How many slots a deck can hold. The roadmap's number: "one to four members
/// are Live and composited; the rest are resident". Four is also what the
/// composite shader binds, so raising it is an edit there as well as here.
///
/// This is a layout constant only because the governor that should be
/// deciding it does not exist yet — "deck size is an output of this, not a
/// layout constant", says M2 of the budget governor.
pub const MAX_SLOTS: usize = 4;

/// Bytes in the composite's uniform block: four `vec4`s, one per slot-indexed
/// field — gain, opacity, blend mode, live flag. See `shaders/composite.wgsl`.
const MIX_UNIFORM_SIZE: u64 = 64;

/// **How a slot's layer meets the ones under it**, at L5.
///
/// Three, and the set is chosen by what survives an unbounded linear HDR mix
/// rather than by what a VJ mixer usually lists. `screen` is `d + s - d*s` and
/// `multiply` is `d * s`; both assume their inputs are display-referred and in
/// `[0, 1]`, and at this point in the pipeline nothing has tone mapped —
/// `screen` of two 2.0s is 0.0, which is not a blend mode, it is a bug with a
/// familiar name. They belong after the transfer curve or not at all.
///
/// The mix is `acc <- mix(acc, f(acc, gain * src), opacity)` for all three,
/// which is what finally separates the deck's two per-slot numbers: `gain` is
/// the level the material arrives at and touches colour only, `opacity` is the
/// fader across the blend and is the only one that scales coverage — with the
/// one exception the module doc names, that a layer [`Blend::silent_at`] skips
/// contributes no coverage either. Under
/// [`Blend::Add`] the two collapse into one multiply, which is why they were
/// one number for as long as `add` was the only mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Blend {
    /// Sum. Emissive material stacking, and what every slot came up in before
    /// there was a choice. A deck of one Live slot at unity gain and full
    /// opacity under this mode reproduces a bare Set bit for bit.
    ///
    /// **In colour, unconditionally; in alpha, for material whose coverage is
    /// a coverage.** The mix saturates what it reads into `[0, 1]` and a bare
    /// Set's target is whatever L4 accumulated, so an L4 writing an alpha of
    /// 1.5 leaves the two disagreeing in that channel and nowhere else. The
    /// deck's answer is the right one — it is the number `Over` composites
    /// against — and a bare Set has no mix to bound it. Nothing reads either,
    /// which is why this is a caveat recorded here rather than a defect: the
    /// way to remove it is for L4 to bound the coverage where it produces it,
    /// which is a change to the lowering and not to this.
    #[default]
    Add,
    /// Alpha over, against the coverage the L4 pass accumulated. **The only
    /// mode in which one layer hides another**, which is what a mixer is for
    /// and what a transition that is not a crossfade will need.
    ///
    /// Sparse material barely covers, so `over` on a thin point cloud looks
    /// close to `add` at the same fader — which is not a defect: a handful of
    /// sprites genuinely does not occlude anything, and a mode that pretended
    /// otherwise would be keying on something nobody drew.
    Over,
    /// Per-channel maximum. Stacking without the blowout `add` gives, since
    /// four layers of the same bright material stay that bright instead of
    /// reaching four times it. Scale-free, so it means the same thing wherever
    /// the material's exposure sits.
    Max,
}

impl Blend {
    /// Every mode there is, in cycle order. `Add` first, because it is the
    /// default and a cycle should start where a slot starts.
    pub const ALL: [Blend; 3] = [Blend::Add, Blend::Over, Blend::Max];

    /// The wire and status-line spelling. A match rather than a table, so a
    /// mode added to the enum does not compile until it has a name.
    pub fn name(self) -> &'static str {
        match self {
            Blend::Add => "add",
            Blend::Over => "over",
            Blend::Max => "max",
        }
    }

    /// The spelling back, or `None`. Derived from [`Blend::name`] over
    /// [`Blend::ALL`], so the two directions cannot disagree.
    pub fn from_name(name: &str) -> Option<Blend> {
        Blend::ALL.iter().copied().find(|b| b.name() == name)
    }

    /// What the shader switches on. The numbers are the wire format between
    /// `deck.rs` and `composite.wgsl` and nothing else — a record carries the
    /// name, not this.
    fn index(self) -> u32 {
        match self {
            Blend::Add => 0,
            Blend::Over => 1,
            Blend::Max => 2,
        }
    }

    /// **Whether a fader at this setting is silence**, and therefore whether
    /// the composite can skip the slot outright.
    ///
    /// The skip is the operator's way out of broken material: a slot whose L4
    /// divided by zero holds a NaN, and `0.0 * NaN` is NaN, so a fader that
    /// multiplied rather than skipped would put one slot's NaN into every
    /// channel of the mix. `docs/roadmap.md` records that as the first thing
    /// this milestone taught.
    ///
    /// `opacity` at zero is silence under every mode, and it is the escape
    /// that always works. `gain` at zero is silence under `add` and `max`,
    /// where a layer contributing nothing is a layer contributing nothing —
    /// and is **not** silence under `over`, where zero gain is a black card
    /// and a black card covers. That asymmetry is real rather than an
    /// oversight, so it is stated here and in the key that moves the fader:
    /// pull `opacity`, not `gain`, to get out of trouble.
    fn silent_at(self, gain: f32, opacity: f32) -> bool {
        opacity == 0.0 || (gain == 0.0 && self != Blend::Over)
    }
}

/// Where a slot sits between compiled and composited.
///
/// All three of the roadmap's levels. The ordering of the variants is the
/// ordering of how much a slot costs, which is the order
/// [`crate::governor`] moves slots down.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Residency {
    /// Stepped and composited.
    Live,
    /// **Stepped and not drawn.** Warming its element buffers out of sight, on
    /// some or all frames — see "Priming steps; it does not draw" in the module
    /// doc. Contributes nothing to the mix and reads no level.
    Priming,
    /// Compiled, buffers held, not stepping. Keeps its `t`.
    Allocated,
}

impl Residency {
    /// Whether a slot at this level advances its `t` at all. Priming does, on
    /// the frames it steps.
    pub fn steps(self) -> bool {
        !matches!(self, Residency::Allocated)
    }
}

/// One deck member: a Set with its own hot-swap worker, its own HDR target,
/// and its own place in the mix.
struct Slot {
    swap: HotSwap,
    /// **What was asked for.** Only [`Deck::set_residency`] writes this; the
    /// governor never does. See "Residency: requested and effective".
    requested: Residency,
    /// **What is happening.** Derived from `requested` and the budget by
    /// [`Deck::govern`], and the only one the frame loop reads.
    effective: Residency,
    /// Linear gain, applied to this slot's own colour before the blend. This is
    /// the roadmap's "per-Set linear gain" at L5 — how this material balances
    /// against the others — and it is not the `param exposure` inside a
    /// procedure and not tone mapping. All three get called exposure and only
    /// the first belongs to the artifact.
    gain: f32,
    /// Layer opacity, the mixer fader — how much of this layer's blend lands,
    /// `[0, 1]`.
    ///
    /// **Two fields rather than one, and now they earn it.** Under `add` this
    /// and `gain` collapse into a single multiply and always did; under
    /// [`Blend::Over`] and [`Blend::Max`] they do not, because `gain` scales
    /// the colour and `opacity` scales the blend — including the coverage the
    /// layer hides with. See [`Blend`].
    opacity: f32,
    /// How this layer meets the ones under it. Slot order is the stacking
    /// order, so this is the only per-slot control whose meaning depends on
    /// where the slot sits.
    blend: Blend,
    /// While [`Residency::Priming`], step one frame in this many. `1` is every
    /// frame. Set by [`crate::governor`] out of the budget, or by hand.
    prime_one_in: u32,
    /// Frames this slot has spent Priming, counted from the frame it entered.
    /// The step decision is `prime_phase % prime_one_in == 0`, so it is a pure
    /// function of engine state and the same record stream primes on the same
    /// frames every run — see "Determinism" in the module doc.
    ///
    /// Reset whenever the rate or the residency changes, so that a rate change
    /// takes effect from a defined frame rather than from wherever a running
    /// counter happened to be.
    prime_phase: u32,
    /// **What this slot's clock does with the session's** — free, tempo-synced
    /// or beat-locked. Read only while [`Residency::Live`]: a Priming slot is
    /// catching up on the governor's rate, which is a different mechanism for a
    /// different reason, and giving it a transport as well would put two things
    /// in charge of one clock.
    transport: Transport,
    target: wgpu::Texture,
    view: wgpu::TextureView,
}

/// Several Sets resident, one to four of them composited into one HDR target.
///
/// Construct with [`Deck::new`], record frames through [`Deck::begin_frame`].
pub struct Deck {
    /// **The compositing order.** Index order, fixed, never reordered — see
    /// "Determinism" in the module doc.
    slots: Vec<Slot>,
    composite: Composite,
    /// `None` until [`Deck::enable_meters`]. Metering is opt-in because a
    /// caller that will never read a level should not pay for one — see
    /// "Level metering" in the module doc.
    meters: Option<Meters>,
    /// **The session's one local oscillator**, and the seed every noise stream
    /// comes off. See "Signals" in the module doc.
    signals: Signals,
    /// The compute budget priming is decided against. Holds no per-slot state;
    /// [`Deck::govern`] is a pure function of the slots plus this.
    governor: Governor,
    width: u32,
    height: u32,
}

impl Deck {
    /// A deck over `swaps`, in that order. Every slot comes up [`Live`] at
    /// unity gain and full opacity, so a deck of one is a bare Set with a
    /// multiply by 1.0 in front of it.
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
                swap.resize(width, height);
                Slot {
                    swap,
                    requested: Residency::Live,
                    effective: Residency::Live,
                    gain: 1.0,
                    opacity: 1.0,
                    blend: Blend::default(),
                    prime_one_in: 1,
                    prime_phase: 0,
                    transport: Transport::default(),
                    target,
                    view,
                }
            })
            .collect();

        Deck {
            slots,
            composite,
            meters: None,
            signals: Signals::default(),
            governor: Governor::default(),
            width,
            height,
        }
    }

    /// The session's signals — the local oscillator every binding reads, and
    /// the seed every noise stream comes off.
    pub fn signals(&self) -> &Signals {
        &self.signals
    }

    /// Replace the session's signals, tempo and seed together.
    ///
    /// **Before the first frame.** A `Signals` carries the phase as well as
    /// the tempo, so this restarts the session clock at zero rather than
    /// retuning one that is running; a tempo that can move mid-set is what
    /// M2's PLL correction is for, and it belongs in the oscillator rather
    /// than in a wholesale replacement here.
    pub fn set_signals(&mut self, signals: Signals) {
        self.signals = signals;
    }

    /// Start measuring what each slot puts out. Off by default.
    ///
    /// Allocates a pipeline, two buffers and a ring of staging buffers per
    /// slot, so never on the render thread — same terms as [`Deck::new`] and
    /// [`Deck::resize`]. Calling it twice replaces the meters, which discards
    /// every reading and starts over rather than pretending the old ones
    /// survived.
    ///
    /// Levels arrive a few frames later and are read with [`Deck::level`].
    /// Nothing in the deck acts on them: gain is manual, and see
    /// [`crate::meter`] for why that is a decision rather than an omission.
    pub fn enable_meters(&mut self, device: &wgpu::Device) {
        let views: Vec<&wgpu::TextureView> = self.slots.iter().map(|s| &s.view).collect();
        self.meters = Some(Meters::new(device, &views));
    }

    /// The meters, if there are any. `None` from [`Deck::level`] means "no
    /// reading"; this is what tells that apart from "no meter", and it is also
    /// where [`Meters::skipped`] is reached from.
    ///
    /// Read-only: recording, arming and collecting are the frame's, and a
    /// caller that could drive them out of that order would get a reading of
    /// the wrong frame.
    pub fn meters(&self) -> Option<&Meters> {
        self.meters.as_ref()
    }

    /// The most recent level measured for a slot, or `None` if there is none —
    /// no meter, no measurement yet, a slot that is not Live, or a slot whose
    /// material was just replaced by a resize or a swap. Never waits.
    ///
    /// A level is a few frames old and says so: [`Level::frames_behind`]. What
    /// it is never is a reading of an image the slot is no longer showing; see
    /// "Level metering" in the module doc for the three things that retire one.
    pub fn level(&self, slot: usize) -> Option<Level> {
        self.meters.as_ref().and_then(|m| m.level(slot))
    }

    /// The top of a frame: installs whatever the workers finished, opens the
    /// encoder, and hands both out together behind a guard.
    ///
    /// Every slot gets its frame boundary here — Allocated ones included, so
    /// that a build landing on an off-air slot still installs, and so that
    /// retired Sets keep being handed back to the worker to be freed. What an
    /// off-air slot does *not* get is a watchdog sample: it renders nothing,
    /// so this frame's interval is entirely other slots' cost, and judging a
    /// candidate against it would accept or reject it on a number it had no
    /// part in. `HotSwap::begin_frame_parked` freezes the trial instead, and
    /// the verdict waits until the slot is Live. What is still not separated
    /// is one Live slot's cost from its neighbours' — that needs a per-Set
    /// measurement, which is M2's budget governor and is not in this slice.
    ///
    /// Allocates nothing, compiles nothing, blocks on nothing: everything
    /// under here is `try_recv` and `try_lock`, and creating a command encoder
    /// is not a GPU allocation.
    ///
    /// # One frame at a time, by construction
    ///
    /// The guard holds `&mut self` and the encoder together, so a second frame
    /// cannot be opened while one is:
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
    /// The same function with one more `begin_frame` in it does not compile.
    /// It is spelled out rather than described because a claim about what the
    /// borrow checker refuses is only worth what a compiler says about it, and
    /// the `no_run` twin above differs from it by exactly the second borrow —
    /// so a `compile_fail` that started failing for some unrelated reason
    /// would take that one down with it:
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
        for (i, slot) in self.slots.iter_mut().enumerate() {
            // Read before the boundary so that what it appends can be told
            // apart from what the caller has not drained yet. `pending_events`
            // takes nothing: the events are still the caller's to read through
            // `Deck::events`.
            let seen = slot.swap.pending_events().len();
            match slot.effective {
                // The returned borrow is dropped immediately: what is wanted
                // here is the frame-boundary work, and the Sets are reached
                // again inside `Frame::render`, after the encoder exists. That
                // is the whole point of the guard — between these two moments
                // nothing the caller can hold reaches a Set.
                Residency::Live => {
                    let _ = slot.swap.begin_frame();
                }
                // Installs and retirement, but no watchdog sample: this frame
                // is not this slot's to be judged by.
                //
                // Priming is on this side of the line and not the other, which
                // is worth stating because it steps. The watchdog's question is
                // "are frames still arriving with this Set on screen", and this
                // Set is not on screen — it draws nothing, so the interval the
                // deck is producing is entirely the Live slots' cost. Worse, a
                // slot priming at one frame in four is not even paying its own
                // cost on three frames out of four, so a window of intervals
                // attributed to it would be three parts noise. The trial is
                // frozen and the verdict waits until the slot is Live, exactly
                // as it does for a parked one. What judges a priming slot is
                // the per-Set measurement `crate::governor` budgets against,
                // which is a different question and a different number.
                Residency::Priming | Residency::Allocated => slot.swap.begin_frame_parked(),
            }
            // A build landing, and a rollback putting the outgoing Set back,
            // both replace what the slot is drawing — a swapped-in Set is
            // cold, `t` at zero, and a restored one resumes at a different `t`
            // than the candidate had. A measurement of the Set that was there
            // is a measurement of a different image, on exactly the terms a
            // pre-resize measurement is, so it is retired on those terms too.
            // Nothing else in `Event` changes the material: `Accepted` only
            // ends a trial, and `Rejected` and `WorkerLost` change nothing at
            // all.
            let replaced = slot.swap.pending_events()[seen..]
                .iter()
                .any(|e| matches!(e, Event::Swapped { .. } | Event::RolledBack { .. }));
            if replaced {
                if let Some(meters) = &mut self.meters {
                    meters.retire(i);
                }
            }
        }
        // Take delivery of whatever the GPU has finished measuring. A
        // non-blocking `Poll` and a `try_recv` per slot — the same shape as
        // the installs above, and for the same reason: a frame boundary is
        // where results are allowed to arrive, and waiting for one is not.
        if let Some(meters) = &mut self.meters {
            meters.collect(device);
        }
        Frame {
            deck: self,
            queue,
            encoder: Some(device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("deck frame"),
            })),
            rendered: false,
        }
    }

    /// Reallocates every slot target. Never from inside a frame — the borrow
    /// checker sees to that — and never on the render thread mid-frame, on the
    /// same terms as [`Present::resize`].
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if (width, height) == (self.width, self.height) {
            return;
        }
        for (i, slot) in self.slots.iter_mut().enumerate() {
            let (target, view) = make_slot_target(device, i, width, height);
            slot.target = target;
            slot.view = view;
            // Forwarded as well as remembered, for the reason
            // `HotSwap::resize` gives: a Set built at one size must not arrive
            // on screen still believing it.
            slot.swap.resize(width, height);
        }
        let views: Vec<&wgpu::TextureView> = self.slots.iter().map(|s| &s.view).collect();
        self.composite.rebind(device, &views);
        // The meters point at the old textures otherwise — and would keep
        // measuring them, since a bind group holds its views alive, so the
        // symptom would be a level frozen at whatever was on screen before the
        // window was dragged. `Meters::rebind` retires every reading for the
        // same reason.
        if let Some(meters) = &mut self.meters {
            meters.rebind(device, &views);
        }
        self.width = width;
        self.height = height;
    }

    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// How many slots are composited right now. One to [`MAX_SLOTS`] in normal
    /// use; zero is legal and mixes to black.
    pub fn live_slots(&self) -> usize {
        self.slots
            .iter()
            .filter(|s| s.effective == Residency::Live)
            .count()
    }

    /// Read-only access to a slot's hot swap — its live `Set`, its `t`, its
    /// budget. Deliberately not mutable: [`HotSwap::begin_frame`] is the one
    /// place a Set is replaced, and letting a caller reach it here would put
    /// the hole the frame guard closes straight back.
    pub fn slot(&self, slot: usize) -> &HotSwap {
        &self.slots[slot].swap
    }

    /// Everything that has happened to one slot since this was last called.
    /// Draining is the only mutation a caller gets on a `HotSwap` through the
    /// deck, and it cannot change which Set is live.
    pub fn events(&mut self, slot: usize) -> std::vec::Drain<'_, Event> {
        self.slots[slot].swap.events()
    }

    /// **What the slot is doing** — the effective residency. What the frame
    /// loop acts on, and what a status line shows as the slot's current state.
    ///
    /// [`Deck::requested_residency`] is the other half and they are not
    /// interchangeable: a slot reading Allocated here may be one the operator
    /// asked to prime and the budget has parked. See "Residency: requested and
    /// effective" in the module doc.
    pub fn residency(&self, slot: usize) -> Residency {
        self.slots[slot].effective
    }

    /// **What was asked for.** Written only by [`Deck::set_residency`] and
    /// never by [`Deck::govern`], so it survives any number of demotions and is
    /// still there when there is room again.
    pub fn requested_residency(&self, slot: usize) -> Residency {
        self.slots[slot].requested
    }

    /// **Asked to prime, and not priming**: the request stands and the budget
    /// has not allowed it yet. Not the same as a slot the operator parked, and
    /// a surface that shows them alike is telling the operator their request
    /// was discarded when it was only deferred.
    ///
    /// Why it is waiting is [`Reason`](crate::governor::Reason), on the last
    /// [`Report`] rather than here — the deck holds no memory of a pass.
    pub fn is_parked(&self, slot: usize) -> bool {
        let slot = &self.slots[slot];
        slot.requested == Residency::Priming && slot.effective == Residency::Allocated
    }

    /// Move a slot between residency levels.
    ///
    /// Nothing is freed and nothing is reset at any level: a slot moved to
    /// [`Residency::Allocated`] holds its `t` until it steps again, and one
    /// moved to [`Residency::Priming`] resumes stepping from wherever it was
    /// left rather than starting over.
    ///
    /// **This is the request, and the only thing that writes it.** Putting a
    /// slot into Priming asks for it to be warmed; whether the budget allows
    /// it, and how fast, is [`Deck::govern`]'s to answer, and it may hold the
    /// slot at Allocated — parked, with the request intact, primed as soon as
    /// there is room. Nothing here consults the budget, so the request takes
    /// effect immediately and a caller that never calls `govern` primes at full
    /// rate and is responsible for its own arithmetic.
    pub fn set_residency(&mut self, slot: usize, residency: Residency) {
        self.slots[slot].requested = residency;
        // Effective follows the request until a governor pass says otherwise:
        // a deck with no governor driving it behaves exactly as it did when
        // there was one field, and `govern` is what introduces the gap.
        self.set_effective(slot, residency);
    }

    /// Write the effective residency, with the two things that have to happen
    /// whenever it moves. The one place it is written — [`Deck::govern`] and
    /// [`Deck::set_residency`] both come through here, so neither can forget
    /// half of it.
    fn set_effective(&mut self, slot: usize, residency: Residency) {
        if self.slots[slot].effective == residency {
            return;
        }
        // From a defined frame, so that "one step in four" counts from the
        // moment priming started rather than from wherever a counter that had
        // been running through an unrelated stretch happened to be. Two runs of
        // the same record stream then prime on the same frames.
        self.slots[slot].prime_phase = 0;
        self.slots[slot].effective = residency;
        // Off air is off the meter, immediately and including whatever is
        // still in flight: an Allocated slot's target holds whatever it last
        // drew, and reporting that as a level would be a stale number
        // presented as a live one. `Deck::level` reads `None` until it is back
        // on air and a fresh measurement has landed.
        if residency != Residency::Live {
            if let Some(meters) = &mut self.meters {
                meters.retire(slot);
            }
        }
    }

    /// What this slot's clock is doing with the session's.
    pub fn transport(&self, slot: usize) -> &Transport {
        &self.slots[slot].transport
    }

    /// **Put a slot's clock under a sync mode**, or say why it cannot go there.
    ///
    /// The refusal is here, at the moment the operator asks, and not at the
    /// frame where it would misbehave. Both refusals are silent failures
    /// otherwise: beat sync on accumulating material evaluates the procedure
    /// once from wherever it happened to be, and tempo sync on material that
    /// reads `beats` runs it at the square of the tempo ratio. Neither raises
    /// anything on its own; both look like a broken artifact.
    ///
    /// All three values are applied verbatim, because this is the apply half of
    /// a record and a replay that recomputed one of them from the machine it is
    /// running on would not be a replay. [`Transport::engaged`] is what decides
    /// them when an operator engages a mode by hand.
    pub fn set_transport(
        &mut self,
        slot: usize,
        sync: Sync,
        anchor_bpm: f32,
        offset_beats: f64,
    ) -> Result<(), crate::transport::Refusal> {
        self.sync_allowed(slot, sync)?;
        self.slots[slot].transport.set(sync, anchor_bpm, offset_beats);
        Ok(())
    }

    /// Whether a mode is available for the Set currently in this slot, and why
    /// not when it is not. **What a surface greys a control out on**, and it
    /// answers before anything is pressed.
    ///
    /// A property of the Set, so it changes when a build lands in the slot. A
    /// swap that replaces closed-form material with accumulating material can
    /// therefore make the mode a slot is *already in* unavailable; nothing here
    /// resolves that, and whatever wires swapping to this owes it.
    pub fn sync_allowed(
        &self,
        slot: usize,
        sync: Sync,
    ) -> Result<(), crate::transport::Refusal> {
        let set = self.slots[slot].swap.set();
        Transport::allows(sync, set.is_closed_form(), set.reads_beats())
    }

    /// How many slots are warming out of sight right now. Effective, so a
    /// parked slot is not one of them however much it was asked for.
    pub fn priming_slots(&self) -> usize {
        self.slots
            .iter()
            .filter(|s| s.effective == Residency::Priming)
            .count()
    }

    /// How many slots asked to prime and are waiting for room — see
    /// [`Deck::is_parked`].
    pub fn parked_slots(&self) -> usize {
        (0..self.slots.len()).filter(|&i| self.is_parked(i)).count()
    }

    /// One step every this many frames while the slot is Priming. `1` is every
    /// frame, which is what a slot comes up at.
    pub fn prime_one_in(&self, slot: usize) -> u32 {
        self.slots[slot].prime_one_in
    }

    /// Slow a Priming slot down, or speed it up. Clamped to at least 1 — a rate
    /// of zero is not "never", it is a modulus of zero.
    ///
    /// Ordinarily [`Deck::govern`]'s to set. Exposed because a rate is a
    /// legitimate manual choice ("warm this gently while I mix") and because a
    /// caller that does not want a governor at all should still be able to
    /// prime.
    pub fn set_prime_one_in(&mut self, slot: usize, one_in: u32) {
        let one_in = one_in.max(1);
        if self.slots[slot].prime_one_in != one_in {
            // From a defined frame, for the reason `set_residency` resets it.
            self.slots[slot].prime_phase = 0;
            self.slots[slot].prime_one_in = one_in;
        }
    }

    /// The compute budget priming is decided against, in milliseconds of
    /// measured per-Set cost.
    ///
    /// **Not the watchdog's `--budget-ms`**, which is a frame interval on a
    /// host clock; see [`crate::governor::DEFAULT_COMPUTE_BUDGET_MS`] for why
    /// the two are not comparable. Spelled `compute_budget_ms` rather than
    /// `budget_ms` for exactly that reason: every slot on this deck carries a
    /// [`HotSwap::budget_ms`] of the other kind, and `deck.budget_ms()` sitting
    /// beside `deck.slot(i).budget_ms()` is an invitation to pass one where the
    /// other belongs. Two quantities that share a unit and nothing else should
    /// not share a name.
    pub fn compute_budget_ms(&self) -> f32 {
        self.governor.budget_ms()
    }

    pub fn set_compute_budget_ms(&mut self, budget_ms: f32) {
        self.governor.set_budget_ms(budget_ms);
    }

    /// **Measure every slot nothing has measured yet.** Returns how many it
    /// measured.
    ///
    /// **Call this at startup, before the first frame**, where a stall is free
    /// — and never on the render thread or inside a frame, which the borrow
    /// checker sees to. It submits and waits, once per sample per slot, which
    /// is exactly the pattern the frame path must never use.
    ///
    /// This is what a deck owes [`Deck::govern`] before governing means
    /// anything. A Set no worker built — every [`HotSwap::fixed`] one, and the
    /// one [`HotSwap::new`] is constructed with — arrives with no measurement,
    /// and an unmeasured *Live* slot makes the deck's committed cost unknown,
    /// which suspends priming entirely: see "Why an unmeasured Set is not a
    /// free one" in [`crate::governor`]. A slot the build worker already
    /// measured is skipped, so calling this is idempotent and cheap after the
    /// first time.
    ///
    /// **A slot whose Set has already stepped is skipped too, and that is the
    /// reason "at startup" is not decoration.** Measuring means stepping, so it
    /// ends in [`Set::rewind`](crate::set::Set::rewind), which puts the Set back
    /// to what `Set::build` left — cold, `t` at zero. On a Set that has been
    /// running that is not a restoration, it is a reset, and on an on-air slot
    /// it would be a visible one. So the cold ones are measured and the running
    /// ones are left alone: a deck measured late stays partly unbudgetable,
    /// which the governor reports, rather than losing an hour of simulation to
    /// a status line.
    ///
    /// One [`Probe`] for the whole deck, deliberately: constructing one runs a
    /// calibration workload ten times over, and two probes can land on
    /// different [`MeasurementMethod`](crate::probe::MeasurementMethod)s and
    /// produce numbers that are not comparable — which is precisely what the
    /// governor summing them would then be doing.
    pub fn measure_slots(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> usize {
        // `t == 0` is "has not stepped": `t` is `steps_taken * dt` and advances
        // through `Set::prepare` alone, so a Set at zero is one a rewind cannot
        // take anything away from.
        let measurable = |slot: &Slot| {
            slot.swap.measured_cost().is_none() && slot.swap.set().time() == 0.0
        };
        if !self.slots.iter().any(measurable) {
            // Before the probe, not after: `Probe::new` allocates a 720p target
            // and spends half a second calibrating, and a deck with nothing to
            // measure should pay neither.
            return 0;
        }
        let mut probe = Probe::new(
            device,
            queue,
            device.features().contains(wgpu::Features::TIMESTAMP_QUERY),
            PROBE_RESOLUTION,
        );
        let mut measured = 0;
        for slot in &mut self.slots {
            if measurable(slot) {
                slot.swap.measure_live(&mut probe, device, queue);
                measured += 1;
            }
        }
        measured
    }

    /// **Decide what may prime and how fast, and apply it.**
    ///
    /// Reads each slot's *requested* residency, the per-Set measurement its
    /// `HotSwap` is carrying, and whether its Set is closed form; hands them to
    /// [`Governor::decide`]; and writes the *effective* residencies and the
    /// rates back. The request is never written — that is the operator's, and a
    /// governor that overwrote it would turn "not now" into "no" and make a
    /// single over-budget pass cancel every priming request on the deck. Live
    /// slots are read and never written at all — see "What it does not touch"
    /// in [`crate::governor`].
    ///
    /// Returns the whole [`Report`], including the numbers it decided on and
    /// the one warning it can raise, so a status line has something to print
    /// and a test has something to assert. **Nothing is printed here**, on the
    /// same terms as `swap.rs`'s events: the engine does not print, so a test
    /// can assert on the same values a user reads.
    ///
    /// Allocates — one `Vec` per call — so **not from inside a frame**, which
    /// the borrow checker sees to anyway, and not once per frame by habit. This
    /// is a beat- or edit-granularity decision: call it when a residency
    /// changed, when a build landed, or when the budget moved.
    pub fn govern(&mut self) -> Report {
        let states: Vec<SlotState> = self
            .slots
            .iter()
            .map(|s| SlotState {
                // The request, not what the slot is currently doing. A pass
                // fed its own last verdict would make every park permanent —
                // the parked slot would read as Allocated, be reported as
                // "not asked", and never be reconsidered.
                requested: s.requested,
                cost: s.swap.measured_cost(),
                closed_form: s.swap.set().is_closed_form(),
            })
            .collect();
        let report = self.governor.decide(&states);
        for decision in &report.decisions {
            // Through `set_effective`, not around it: that is where a meter is
            // retired and where the priming phase is reset, and a governor that
            // wrote the field directly would leave a level reported for a slot
            // it had just taken off air. Not `set_residency` — that writes the
            // request too, which is the one thing this pass may not do.
            self.set_effective(decision.slot, decision.effective);
            self.set_prime_one_in(decision.slot, decision.prime_one_in);
        }
        report
    }

    pub fn gain(&self, slot: usize) -> f32 {
        self.slots[slot].gain
    }

    /// Per-slot linear gain, applied to that slot's colour before the blend.
    /// Linear, not perceptual, and **deliberately not clamped above 1.0**: the
    /// pipeline is HDR and values above 1.0 are expected. That is the one way
    /// this differs from [`Deck::set_opacity`], which is a proportion and is
    /// bounded at both ends.
    ///
    /// **Floored at zero, and NaN reads as zero**, for the same reason and in
    /// the same place: a record is how a *replay* drives the deck and a stream
    /// is allowed to say anything. A negative gain subtracts one slot's light
    /// from another's — which is a blend mode, not a fader — and a NaN gain
    /// puts a NaN in every channel of the mix from one slot, which is the
    /// failure the fader exists to prevent and which no fader can undo once the
    /// gain has done it.
    ///
    /// Not the fader. See [`Deck::set_opacity`], and [`Blend`] for why the
    /// difference is only visible under a mode that is not `add`.
    pub fn set_gain(&mut self, slot: usize, gain: f32) {
        self.slots[slot].gain = if gain.is_nan() { 0.0 } else { gain.max(0.0) };
    }

    pub fn opacity(&self, slot: usize) -> f32 {
        self.slots[slot].opacity
    }

    /// **The fader.** How much of this layer's blend lands, and the one control
    /// that silences a slot under every blend mode — which makes it the way out
    /// of material that has gone NaN. See [`Blend`].
    ///
    /// **Clamped to `[0, 1]` here rather than trusted**, because opacity is a
    /// proportion of a blend and there is no such thing as 1.4 of one: past 1.0
    /// an `over` layer subtracts more than it covers and the mix goes negative.
    /// Unlike [`Deck::set_gain`], which is a level into an HDR mix and is
    /// deliberately open above 1.0, every value outside this range has exactly
    /// one sensible reading. The clamp is here and not at the key press because
    /// a replayed `opacity` record reaches this and not that.
    ///
    /// **NaN silences**, and is handled before the clamp rather than by it:
    /// `f32::clamp` passes a NaN straight through. A fader whose value is not a
    /// number is a broken control, and of the two available readings — "this
    /// slot goes dark" and "the whole mix goes dark" — only one of them is a
    /// fader.
    pub fn set_opacity(&mut self, slot: usize, opacity: f32) {
        self.slots[slot].opacity = if opacity.is_nan() {
            0.0
        } else {
            opacity.clamp(0.0, 1.0)
        };
    }

    pub fn blend(&self, slot: usize) -> Blend {
        self.slots[slot].blend
    }

    /// How this slot's layer meets the ones under it. See [`Blend`].
    pub fn set_blend(&mut self, slot: usize, blend: Blend) {
        self.slots[slot].blend = blend;
    }

    /// The target a slot renders into, for a readback or a preview. The mix is
    /// written to whatever [`Frame::render`] is handed, which is the present
    /// pass's HDR target; the deck owns no mix target of its own, because
    /// owning one would mean copying it into the present pass's.
    pub fn slot_target(&self, slot: usize) -> &wgpu::Texture {
        &self.slots[slot].target
    }
}

/// One open frame: the encoder, and exclusive access to the deck for as long
/// as it is open.
///
/// Submits on [`Frame::finish`] and on drop, so an early return out of a frame
/// body ends the frame rather than losing the work recorded so far. See "The
/// frame guard" in the module doc for what this does and does not enforce.
pub struct Frame<'a> {
    deck: &'a mut Deck,
    queue: &'a wgpu::Queue,
    /// `None` once submitted. `Option` rather than a flag because
    /// `CommandEncoder::finish` consumes the encoder and both [`Frame::finish`]
    /// and [`Drop`] have to be able to do it, exactly once between them.
    encoder: Option<wgpu::CommandEncoder>,
    rendered: bool,
}

impl Frame<'_> {
    /// Advance every Live slot by `steps`, render each into its own target,
    /// and mix them into `target`.
    ///
    /// `steps` comes from a `tick` record, never from a measurement, and every
    /// Live slot gets the same number from the same tick — a deck whose
    /// members advanced by different amounts would not be one frame of one
    /// session.
    ///
    /// `target` is linear HDR (`Rgba16Float`) and is left that way: tone
    /// mapping and the sRGB encode are the present pass's, downstream of here.
    /// It must be the size the deck was built or resized to, since the mix is
    /// a texel-for-texel read of the slot targets.
    ///
    /// Nothing here allocates: `Set::prepare` packs into storage sized at
    /// build time, and the composite's uniform is written from a stack array.
    pub fn render(&mut self, target: &wgpu::TextureView, target_size: (u32, u32), steps: u8) {
        assert!(
            !self.rendered,
            "one `render` per frame: a second one would advance every Live slot by \
             another `steps` off the same tick"
        );
        self.rendered = true;

        // The session clock, advanced once, here, by exactly what every Live
        // slot is about to be advanced by — clamped the same way `Set::prepare`
        // clamps it, or a frame the simulation was allowed to fall behind on
        // would move the oscillator further than the material it drives. The
        // `rendered` assert above is what makes "once" structural: a second
        // `render` in one frame cannot reach this.
        //
        // Before the slots, so that a binding reads the phase at the instant of
        // this frame's last substep — the same instant `Set::time` reports.
        self.deck.signals.advance(steps.min(MAX_STEPS), DT);

        // A view carries no dimensions, so the size comes alongside it and is
        // checked here. Silence is the reason: the composite reads its sources
        // with `textureLoad`, and an out-of-range `textureLoad` is *defined* to
        // return zero rather than to fault — so a caller that resized `Present`
        // and not the deck would get a black mix, every frame, with nothing
        // logged and nothing to catch. `Present::size` is where the pair comes
        // from; wanting them apart is wanting a bug.
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

        // Index order, and one branch per residency level.
        // Borrowed out of the deck before the slots are: every Live slot reads
        // the same signals, from the same frame, which is the same reason they
        // are all advanced by the same `steps`.
        let signals = &self.deck.signals;
        for (i, slot) in self.deck.slots.iter_mut().enumerate() {
            // The effective residency, and only ever that: what the frame does
            // is what the governor last allowed, not what was asked for.
            match slot.effective {
                Residency::Live => {
                    let view = &slot.view;
                    let set = slot.swap.live_mut();
                    // **What this slot's clock does with the session's.** Free
                    // is the session's own `steps` and is what every slot did
                    // before the transport existed, so a deck nothing has
                    // arranged records exactly the frame it used to.
                    let steps = match slot.transport.advance(steps, signals.oscillator(), DT) {
                        Advance::Steps(n) => n,
                        // A seek: put the clock on the target and evaluate
                        // there with a single pass. One is enough because the
                        // transport only offers this mode on closed-form
                        // material, whose state at `t` does not depend on how
                        // it got there — `Set::seek` carries that argument and
                        // the refusal that enforces it.
                        //
                        // One short of the target, because `prepare` bumps the
                        // counter by the steps it is given.
                        Advance::SeekTo(target) => {
                            set.seek(target.saturating_sub(1));
                            1
                        }
                    };
                    set.prepare(self.queue, steps, signals);
                    set.render(encoder, view, steps);
                    // After the render pass, into the same encoder, so the
                    // measurement is of this frame's image. Live slots only: a
                    // slot that is not Live rendered nothing, so there is
                    // nothing of this frame's to measure — see
                    // `Deck::set_residency`.
                    if let Some(meters) = &mut self.deck.meters {
                        meters.record(i, encoder);
                    }
                }
                // Warming, out of sight. `prepare` and the L1 passes, and
                // **no render pass at all** — no target, no draw, nothing
                // reaching the composite. See "Priming steps; it does not
                // draw" in the module doc for why that warms everything the
                // roadmap's hidden render would have.
                //
                // On a frame it does not step, nothing happens: `prepare` is
                // what advances `t`, so the slot's clock simply runs slower
                // than wall time. It is catching up, not keeping time.
                Residency::Priming => {
                    let step_now = slot.prime_phase % slot.prime_one_in == 0;
                    slot.prime_phase = slot.prime_phase.wrapping_add(1);
                    if step_now {
                        let set = slot.swap.live_mut();
                        // `prepare_warming`, not `prepare`: this slot's clock
                        // is behind the session's by however much the governor
                        // has slowed it, and handing it the session's phase
                        // would let the priming *rate* decide what it warms
                        // into. See `Set::prepare_warming`.
                        set.prepare_warming(self.queue, steps, signals);
                        set.step(encoder, steps);
                    }
                }
                // Not stepped and not drawn, which is the whole of what
                // Allocated means: `t` only advances through `prepare`.
                Residency::Allocated => {}
            }
        }

        self.deck
            .composite
            .record(self.queue, encoder, target, &self.deck.slots);
    }

    /// The frame's encoder, for whatever else the caller records into this
    /// frame — the present pass, a readback copy. Deliberately reachable only
    /// through the guard: an encoder the caller owned is what let two Sets
    /// into one frame.
    pub fn encoder(&mut self) -> &mut wgpu::CommandEncoder {
        self.encoder
            .as_mut()
            .expect("the encoder is open until `finish` or drop")
    }

    /// Submit, and end the frame. Dropping the guard does the same thing; this
    /// exists so that the common case says so at the call site.
    pub fn finish(mut self) {
        self.submit();
    }

    fn submit(&mut self) {
        if let Some(encoder) = self.encoder.take() {
            self.queue.submit([encoder.finish()]);
            // Only now: `map_async` resolves against the submissions
            // outstanding when it is called, so arming a staging buffer before
            // the copy that fills it has been submitted would deliver whatever
            // that buffer held from three frames ago as if it were this
            // frame's. `Meters::arm` says the same thing from the other side.
            if let Some(meters) = &mut self.deck.meters {
                meters.arm();
            }
        }
    }
}

impl Drop for Frame<'_> {
    /// Submits if `finish` did not. An early return out of a frame body is
    /// then a frame that ends where it was abandoned, rather than a frame's
    /// recorded work silently disappearing along with the encoder.
    fn drop(&mut self) {
        self.submit();
    }
}

/// One slot's linear HDR target.
///
/// **8 bytes a texel.** At 1280x720 that is 7.03 MB per slot and 28.1 MB for a
/// full deck of four, which is the price of keeping the sources separate — see
/// "One target per slot" in the module doc.
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
        // COPY_SRC is not for the frame path: it is what lets a test read one
        // slot's output back, and what M2's per-slot preview will want.
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    (texture, view)
}

/// The mix pass: one fullscreen triangle folding up to four slot targets.
struct Composite {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl Composite {
    fn new(device: &wgpu::Device, views: &[&wgpu::TextureView]) -> Composite {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("composite"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/composite.wgsl").into()),
        });

        let mut entries = vec![wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }];
        for slot in 0..MAX_SLOTS {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 1 + slot as u32,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    // Not filterable, because the mix does not sample: it
                    // loads the texel under the fragment. No sampler is bound
                    // here at all, which is what keeps a deck of one exact.
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
        }
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("composite"),
            entries: &entries,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("composite"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("composite"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    // Linear HDR out. The mix is folded in the shader, in slot
                    // order, so there is no blend state here: hardware
                    // blending would put the order in the hands of whatever
                    // sequence the passes happened to be recorded in — and
                    // `over` makes that order visible in the picture rather
                    // than only in the last bits.
                    format: Present::HDR_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("composite mix"),
            size: MIX_UNIFORM_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = Composite::bind(device, &layout, &uniform, views);
        Composite {
            pipeline,
            layout,
            uniform,
            bind_group,
        }
    }

    /// The shader binds [`MAX_SLOTS`] textures whatever the deck's size is, so
    /// a deck of fewer slots fills the spare bindings with slot 0's view. The
    /// live flag for those is zero and the shader skips them, so nothing is
    /// read through them; binding a view twice is cheaper and simpler than a
    /// second pipeline per deck size, and far simpler than allocating four
    /// targets for a deck of one.
    fn bind(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        uniform: &wgpu::Buffer,
        views: &[&wgpu::TextureView],
    ) -> wgpu::BindGroup {
        let mut entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform.as_entire_binding(),
        }];
        for slot in 0..MAX_SLOTS {
            entries.push(wgpu::BindGroupEntry {
                binding: 1 + slot as u32,
                resource: wgpu::BindingResource::TextureView(views[slot.min(views.len() - 1)]),
            });
        }
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("composite"),
            layout,
            entries: &entries,
        })
    }

    fn rebind(&mut self, device: &wgpu::Device, views: &[&wgpu::TextureView]) {
        self.bind_group = Composite::bind(device, &self.layout, &self.uniform, views);
    }

    /// Write this frame's weights and record the mix.
    ///
    /// The uniform write is a fixed 64 bytes off the stack — the render thread
    /// does not allocate, and this is the render thread.
    fn record(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        slots: &[Slot],
    ) {
        let mut bytes = [0u8; MIX_UNIFORM_SIZE as usize];
        for (i, slot) in slots.iter().enumerate() {
            // Gain and opacity travel apart now — the blend mode decides what
            // each one multiplies, and only `add` multiplies them together.
            // The blend mode is the shader's index rather than its name; the
            // name is what a record carries.
            //
            // A slot at silence is *skipped*, not blended at zero, on exactly
            // the terms an off-air slot is: `0.0 * x` is only zero for finite
            // `x`, and a slot's own target may hold an infinity or a NaN — a
            // generated L4 that divides by zero or takes a root of a negative
            // is a compiling procedure, not a broken build. Blending that at
            // zero puts a NaN in every channel of the mix, so a fader pulled to
            // silence would take the whole deck down with it. Which settings
            // count as silence is the mode's answer: see [`Blend::silent_at`].
            let live = u32::from(
                slot.effective == Residency::Live && !slot.blend.silent_at(slot.gain, slot.opacity),
            );
            let fields = [
                slot.gain.to_le_bytes(),
                slot.opacity.to_le_bytes(),
                slot.blend.index().to_le_bytes(),
                live.to_le_bytes(),
            ];
            for (field, value) in fields.iter().enumerate() {
                let at = field * 16 + i * 4;
                bytes[at..at + 4].copy_from_slice(value);
            }
        }
        queue.write_buffer(&self.uniform, 0, &bytes);

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    // The triangle covers the whole target, so this only
                    // matters for a deck with nothing Live in it — which mixes
                    // to black, and should say so rather than showing whatever
                    // was there last frame. `TRANSPARENT` rather than `BLACK`
                    // because the alpha channel is coverage: an empty deck
                    // covers nothing, and `BLACK` would claim it covered
                    // everything opaquely.
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}
