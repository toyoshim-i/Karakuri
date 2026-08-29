//! **What a live region declares, and what the panel has to fit every
//! declaration into.**
//!
//! [P-0072](../../../docs/principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md)
//! has two halves and this module is the second one's *inputs*: **what must be
//! live during a performance is named, and each named thing declares two
//! numbers** — what its update costs, and how stale it may get. [`Declared`]
//! is that pair with the region's own name on it, and
//! [`crate::view::View::declares`] is what answers with them.
//!
//! # Nothing here schedules, and nothing here is read on a frame
//!
//! P-0072's arithmetic is two inequalities over the declarations:
//!
//! ```text
//! Σ (cost / staleness)  ≤  budget / frame interval
//! max(cost)             ≤  a small part of the budget
//! ```
//!
//! and the principle says outright what is done with them: *"They are sums
//! over the named regions, so **a test asserts them** rather than a stage
//! discovering them."* So the four numbers below are here and the arithmetic
//! is not — `tests/schedulable.rs` is the only place either sum is taken, and
//! nothing in this crate arbitrates between two regions at runtime.
//!
//! **There are two regions now and still nothing arbitrates, and the reason
//! has changed.** It used to be that choosing between one region and nothing
//! is an abstraction with one call site. What it is now is that **both fit**:
//! `Σ (cost / staleness)` is 0.0889 against 1.0, so a frame drawn for the
//! sooner deadline is in time for the other one and nothing has to be
//! refused. [`crate::view::View::animating`] takes the soonest staleness and
//! that is the whole policy. A scheduler is what a budget that cannot afford
//! everything needs, and this one can
//! ([ADR-0212](../../../docs/adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md)).
//!
//! # What is *not* on this budget
//!
//! **The Program bay's picture.** P-0072: *"What is inside it is the engine's
//! output, already accounted for by the governor, and counting it here would
//! count it twice."* What lands here is the composite — putting that texture
//! into the panel — *"drawn every frame at the highest priority regardless"*,
//! which is to say it is not scheduled and does not declare.
//!
//! **What the operator does.** A drag on a divider, a fold, a room toggle:
//! *"these are the operator's own load, they are transient, and they are not
//! budgeted"*. [`crate::repaint`] already reads that way — every arm on a
//! gesture is `Repaint::Now` — and this module is about the frames nobody is
//! touching.

use std::time::Duration;

/// **One live region's declaration**: what one update of it costs, and how
/// stale it may get.
///
/// The unit is a **region** and never a slice of time, which is P-0072 stated
/// as a type: a region is redrawn whole or not at all, so what declares is a
/// node of the arrangement rather than an animation, a control or a frame.
/// [`Declared::region`] is that node's name, and it is the name the
/// arrangement, the manual, the operations and this crate all address it by
/// (ADR-0156, ADR-0159) — so a declaration can be handed to
/// [`karakuri_layout::Layout::find`] and answered for, which is what
/// `tests/schedulable.rs` does with it.
///
/// **Three presentations inside one region are one declaration**, because the
/// region is the unit: the mixer strip's tally roll and the reach on each of
/// its two faders share a period, a curve and a staleness, so a parked slot
/// and eight armed fades declare once
/// ([ADR-0190](../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md),
/// [ADR-0206](../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)).
/// A second *rate* would be a second declaration; a second *user* of one rate
/// is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Declared {
    /// The arrangement's name for the region that is declaring —
    /// `"transport"` and `"mixer"` today, in that order, which is the order
    /// they are drawn down the panel.
    pub region: &'static str,
    /// **What one update of this region costs**, on the CPU. Today that is
    /// [`PANEL_PASS`] for every region, and the constant says why.
    pub cost: Duration,
    /// **How stale this region may get before what it shows misrepresents
    /// what the instrument is doing**, in wall time and never in frames — a
    /// tolerance stated in frames doubles when the rate halves, which is to
    /// say it becomes most permissive exactly when the machine is most loaded.
    pub staleness: Duration,
}

/// **What one update of a region on this console costs: a whole panel pass.**
///
/// # Why every region declares the same number
///
/// `egui` is immediate mode. There is no retained tree, so *redraw the mixer
/// bay* is not an operation this panel has — what repaints is the panel and
/// not the chip, which
/// [ADR-0188](../../../docs/adr/0188-a-pending-transition-says-it-is-pending-and-no-surface-holds-the-rule.md)
/// and
/// [ADR-0190](../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md)
/// both state as the honest cost of the roll. So the price of servicing any
/// one region's deadline is the price of the whole immediate-mode pass, and a
/// per-region figure would be a number describing work this console cannot
/// do.
///
/// **It is therefore a deliberate over-declaration**, which is the direction
/// ADR-0190 already chose for the staleness beside it: *"A presentation
/// declares what it needs; what the panel can afford is decided elsewhere …
/// a deliberate over-declaration that a scheduler can refine downwards."* It
/// stops being one on the day a region can be drawn once into a texture and
/// composited after — P-0072's own remedy for a region that cannot meet the
/// second condition — and that day this constant becomes a per-region number
/// and `Declared::cost` stops being written from one place.
///
/// # Where the number comes from, and why it is not measured at runtime
///
/// **A declaration rather than a measurement**, which is
/// [ADR-0164](../../../docs/adr/0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md):
/// *"a measured schedule reorders itself with the machine's noise, so the same
/// state behaves differently frame to frame for reasons the operator cannot
/// see, and when a declaration is wrong it is wrong somewhere a person can
/// read."* Nothing reads this at runtime and nothing samples a clock to get
/// it.
///
/// **1.26 ms, taken on 2026-08-28** over five runs of `crates/karakuri/src/main.rs` on
/// an Apple M4 Pro at 1440x900 logical, host clock, debug profile with
/// dependencies at opt-level 3, with nothing touching the window. It is
/// `ui + textures + buffers + record` — the whole immediate-mode pass, plus
/// the panel's own texture and geometry uploads and the recording of its
/// render pass. The five read per-frame medians of 1.047, 1.224, 1.259, 1.281
/// and 1.340 ms, and this is the middle of them. **Read it as an order of
/// magnitude**: the same machine reports about a sixth of these numbers with
/// its other cores loaded, which is why what holds this figure honest allows
/// a factor of two either way rather than a digit.
///
/// **Three numbers on that frame are deliberately not in it.** The engine
/// pass, because the picture is the governor's and counting it here would
/// count it twice (P-0072). The vsync wait, because a frame blocked in
/// `get_current_texture` is not paying for anything. And the submission —
/// about 2.4 ms, larger than this whole figure — because it carries the
/// engine's half of the frame as well and is paid whenever anything is
/// submitted, so charging it to a region would charge a region for a frame it
/// did not ask for.
///
/// # How it is kept honest
///
/// **`crates/karakuri/src/main.rs` holds this figure against the run it has just
/// taken** and says so when the two have parted company, which is the pattern
/// already in that file for `WRITTEN_ALLOCS` and the reason that one exists:
/// a number written into prose reads as current forever, and this repository
/// has watched exactly that happen. It **reports** rather than fails, and that
/// is a decision rather than a lapse: this machine reports about a sixth of
/// these numbers with its other cores loaded, so an assertion on a
/// millisecond would be a gate nobody could keep passing, and *flaky is worse
/// than broken*.
///
/// What **is** asserted, without a window or a clock, is the other half:
/// `tests/schedulable.rs` holds every declaration's cost to this one
/// constant, so a per-region figure cannot be invented while the panel is
/// still redrawn whole.
pub const PANEL_PASS: Duration = Duration::from_micros(1260);

/// **What the panel may spend on a frame**, and the divisor in P-0072's
/// second condition.
///
/// **It is the console's only written-down budget and it is the whole frame's,
/// which is worth reading twice.** `view::Transport::budget_ms` is *"what a
/// frame has to fit in"* — the `/16.6` the transport row draws — and
/// `crates/karakuri/src/main.rs` fills it from the display's refresh interval, because
/// the surface is `PresentMode::Fifo` and a frame longer than one interval is
/// a frame that misses a vsync. Nothing anywhere writes down a **panel's
/// share** of that. So the conditions are asserted in their most permissive
/// form — the panel is allowed the whole frame — and what that costs is that
/// they cannot catch a panel which fits the frame while leaving the engine
/// nothing. Naming a share is a decision, and it belongs with whoever takes
/// it rather than with the test that noticed it was missing.
///
/// **Fixed at the 60 Hz interval rather than read from the display**, which is
/// ADR-0164 in one number: *"the panel's absolute budget does not grow when
/// the frame lengthens. The extra time belongs to the engine, which is what is
/// actually struggling."* So this stays put while [`FRAME_INTERVAL`] moves,
/// and the panel's share of a lengthened frame shrinks rather than growing —
/// which is the whole point of the two being separate constants that happen to
/// be equal today.
pub const BUDGET: Duration = Duration::from_nanos(16_666_667);

/// **How long a frame is**, and the divisor in P-0072's first condition.
///
/// 60 Hz: the rate the mock's transport is drawn against (`12.4/16.6 ms`), the
/// rate `crates/karakuri/src/main.rs` reads off the monitor it opens on, and the rate
/// every figure in this repository is taken at.
///
/// **The same number as [`BUDGET`] and not the same quantity.** This one is
/// what the window is actually held to and it moves — a 120 Hz display halves
/// it, and `PresentMode::Fifo` quantising a missed frame doubles it. The
/// budget does not move with it. `Σ (cost / staleness) ≤ budget / frame
/// interval` is therefore a *fraction of each frame the panel may have*, and
/// it gets smaller as the frame gets longer.
pub const FRAME_INTERVAL: Duration = Duration::from_nanos(16_666_667);

/// **How much of the [`BUDGET`] one region's update may be**, and the whole of
/// P-0072's second condition: `max(cost) ≤ a small part of the budget`.
///
/// The clause exists because *an update is not divisible*: a region costing
/// most of the budget is a traffic jam of one — every frame it runs, nothing
/// else can, and the readouts that had to be live miss their deadlines. A
/// region that cannot meet it is split, or its unchanging part is drawn once
/// into a texture and composited after.
///
/// # A quarter, and the fraction is a preference
///
/// Marked as
/// [P-0056](../../../docs/principles/0056-mark-what-is-preference-so-it-can-be-revisited.md)
/// asks. **Forced**: the fraction is well under a half, because *most of the
/// budget* is the failure the clause names and a half is not distinguishable
/// from it. **Preference**: which fraction under a half.
///
/// A quarter, on two readings that bracket it. A tenth is 1.67 ms against a
/// panel pass measured at 1.26, whose five runs already spread 1.05 to 1.34 —
/// a rule that passes by a third of what the measurement itself moves by is a
/// rule about this laptop rather than about the console. A quarter is 4.17 ms
/// and leaves a factor of 3.3, which is the same order of headroom
/// `crates/karakuri/src/main.rs` allows its own quoted figure and for the same reason.
///
/// **A second region declares now and this number did not have to move**,
/// because `max(cost)` is still one number: both of them declare one whole
/// [`PANEL_PASS`], which is what immediate mode makes true. It is the day a
/// region declares a cost **of its own** that `max` starts choosing, and that
/// is the day to revisit this fraction.
pub const SMALL_PART: f32 = 0.25;
