//! Hot swap: building a Set off the render thread, installing it on a frame
//! boundary, and taking it back out again if it does not hold up.
//!
//! This is the third clause of the V1 assumption — "and can we hot-swap it
//! without dropping a frame?" — and it is the one place where all four render
//! thread invariants have to hold at once:
//!
//! - **Never allocate on the render thread. Never compile shaders on it.**
//! - Pipelines are double-buffered; swaps happen only on frame boundaries.
//! - If a new pipeline exceeds the frame budget, **stop the slot and say so**
//!   — where *the frame budget* is what one frame of **that Set** may cost,
//!   and not what the deck's frames happen to be taking (ADR-0313). It used to
//!   roll back automatically, and does not since ADR-0316: the version stays
//!   where the operator put it and the slot stops updating.
//!
//! ## Compiling somewhere else
//!
//! `wgpu::Device` and `wgpu::Queue` are `Send + Sync` and cheap to clone, and
//! `Set` is `Send` — nothing in it is a raw handle, a `Rc`, or a cell (see the
//! static assertion at the bottom of this module, which is there so that a
//! future field which *is* one of those fails to compile here rather than
//! silently forcing this whole design back onto the render thread). So a
//! worker thread can do the entire expensive half of the job: parse-checked IR
//! in, WGSL out, shader modules, pipelines, buffers, bind groups, a whole
//! `Set`, handed over on a channel already finished.
//!
//! The render thread's side of that channel is `try_recv`, never `recv`, and
//! there is no `device.poll(PollType::wait_indefinitely())` anywhere on the frame path. A
//! frame that finds nothing waiting does nothing about it and renders.
//!
//! ## Where a swap lands, and what actually guarantees it
//!
//! [`HotSwap::begin_frame`] is the only place `live` is replaced, and it
//! returns `&mut Set` borrowed from `&mut self`. While a caller holds that
//! borrow the borrow checker will not let it call `begin_frame` — or anything
//! else on the `HotSwap` — again, so no *single* `&mut Set` can change
//! identity underneath a frame.
//!
//! **That is weaker than "a swap cannot land mid-frame", and the difference
//! matters.** The command encoder is the caller's, and it borrows nothing from
//! the `HotSwap`. A caller is free to open one encoder, drop the borrow, call
//! `begin_frame` again, and record a second Set's passes into the same
//! encoder; that compiles, and if a build arrived in between the encoder ends
//! up holding half of one Set's frame and half of another's. The property this
//! module wants is "one `begin_frame` per encoder", and nothing in *these*
//! types says so — it is a convention `karakuri-cli`'s frame loop and the
//! harness in `tests/hot_swap.rs` both happen to keep.
//!
//! Making it structural means giving `begin_frame` the encoder, and
//! [`crate::deck`] now does exactly that: `Deck::begin_frame` returns a guard
//! owning both the `&mut Deck` and the `CommandEncoder`, so a second frame
//! cannot be opened while one is and there is no way to record two generations
//! of Sets into one encoder. That is where a deck of one to four Sets is
//! composited, and it is the shape a caller with more than one Set has to use.
//!
//! A `HotSwap` driven on its own — which is what `karakuri-cli` and
//! `tests/hot_swap.rs` still do — keeps the weaker property, and the honest
//! claim for it is unchanged: **the live Set is only ever replaced at the top
//! of `begin_frame`**, and such a caller must call it exactly once per frame.
//!
//! ## What a swap does not do
//!
//! It transfers no state. A new procedure means new buffers, so the incoming
//! Set starts cold: `t` at zero, element buffers at their initial contents,
//! nothing primed. That is correct for V1. Warming a Set out of sight before
//! showing it is Priming — see [`crate::deck`] — and needs the deck and the
//! residency model to exist first; half of it built here would be a second,
//! worse answer that the deck would then have to remove.
//!
//! **The outgoing Set is not held to see whether it is needed again**, since
//! ADR-0316: nothing puts a version back, so it is retired at the install and
//! the graveyard has it before the verdict is reached. What survives of the
//! property it used to lean on is the one this module still needs — `t` is
//! simulation time and does not advance for a Set nothing is calling `prepare`
//! on, which is what makes **stopping a slot free**: a stopped slot skips its
//! `prepare` and its `render`, so it costs no step and no draw, and its target
//! goes on holding the frame it was stopped at. The same property
//! [`Allocated`](crate::deck::Residency::Allocated) residency has.
//!
//! ## What a verdict against does
//!
//! **The candidate stays in the slot and the slot stops updating**
//! ([`Event::Overloaded`], [`HotSwap::overloaded`], ADR-0316). The maintainer's
//! sentence is the whole of the reason: *"rolled backが分かりにくい。事情を知らな
//! いとバグってるようにしか見えないんだよね"* — a rollback is unreadable, and
//! without knowing the machinery it looks like a bug. A rollback restored the
//! *picture* and not the *file*, so the disk went on holding the version that
//! had been refused and the next unrelated save reinstalled it; what an
//! operator saw was their save not taking, twice, with nothing saying why.
//! A stopped slot is the same safety bought loudly instead of quietly, which
//! is `P-0094`'s third admissible answer in place of its second.
//!
//! ## A candidate is judged on the candidate's own cost
//!
//! What the verdict compares is **what one frame of this Set costs**, measured
//! on the worker at build time (or estimated at the output's size, where
//! anything has estimated it), against **one frame of the display** — the
//! refresh interval where the caller knew one, [`DEFAULT_BUDGET_MS`] where it
//! did not. Nothing about the other slots is on either side.
//!
//! **It used to be the deck's frame interval, and that was the defect**
//! ([ADR-0313](../../../docs/adr/0313-a-candidate-is-judged-on-its-own-cost-and-the-decks-period-is-a-deck-level-alarm.md)).
//! The interval between two `begin_frame` calls is one number for the whole
//! deck: since ADR-0269 it contains every slot's step and draw whatever its
//! residency, the composite, the cell presents, the picture, the `egui` pass
//! and the vsync wait. Judging a candidate on it means judging one slot's
//! material on the other three slots' cost plus the console's, and under Fifo
//! it is quantised to the display period, so one dropped vsync in the median
//! rolls a candidate back. Measured headless at 1280x720 on an M4 Pro, on the
//! host clock and biased high: the panel's default four slots 4.3 ms, the
//! reference Set in one of four slots 11.3 ms, the reference Set in all four
//! with A live 23.2 ms, all four live 69.9 ms — and on the panel each cell
//! present adds about 1.5 ms on top. So loading the reference Set into one slot
//! of four was about 18 ms of work against a 16.7 ms vsync, landed at 33 ms,
//! and rolled back a Set whose own frame costs about 9 ms.
//!
//! The maintainer's ruling on which quantity is wanted is quoted at
//! `HotSwap::judge`, where the comparison is: **the verdict on a candidate is
//! independent of what the other slots are carrying.** A share of the budget
//! divided by the slot count, or a share taken beside what the neighbours are
//! committed to, would both be the neighbours' load back on the left-hand side
//! by another route.
//!
//! **The number is a host-clock reading and says so.** `probe.rs` explains at
//! length why GPU timestamps are advertised, enabled and unreliable on this
//! crate's machines; the fallback brackets a submit-and-wait the GPU never
//! spent and reads biased high, which is what [`DEFAULT_BUDGET_MS`]'s slack is
//! now for. [`Measurement::method`] is which clock answered, and
//! [`crate::governor::Basis`] rides on the verdict so that a reader can see
//! which of the two numbers it was.
//!
//! ## The frame period is the deck's, and it is an alarm
//!
//! The interval between frames is still measured — [`HotSwap::frame_period_ms`],
//! a rolling median over [`PERIOD_FRAMES`] on a host clock. It can tell that
//! frames stopped arriving on time, which is the thing "the deck is not keeping
//! up" means on stage, and it is honest about being that and nothing more: it
//! **cannot be divided among the slots that produced it**, so nothing may read
//! it as a statement about any one of them.
//!
//! What it reaches is [`Deck::frame_period_ms`](crate::deck::Deck::frame_period_ms)
//! and [`Report::deck_over_period`](crate::governor::Report::deck_over_period),
//! a deck-level fact that **warns and does not act** — the same order
//! [`crate::governor`] takes with [`Report::over_budget`](crate::governor::Report::over_budget)
//! and this repository takes with the level meter: show the number first, and
//! decide later whether anything should move by itself. Whether the deck's
//! total should become what `over_budget` is about is `roadmap.md` M5.14 item 3
//! and is the maintainer's; this is the measurement that item needs.
//!
//! A **median** rather than a worst case, because on a host clock a single
//! sample carries whatever else the OS scheduler was doing — `Probe` makes the
//! same choice for the same reason. One hitch is not a reason to say the deck
//! has stopped keeping up; thirty frames that all miss is.
//!
//! ## The worker measures what it built
//!
//! The number the verdict above is reached on, and the number
//! [`crate::governor`] budgets from, is a **per-Set measurement taken here**, on
//! the worker thread, as part of building the Set. A governor has to add up what
//! several Sets cost and decide, so it needs per-slot numbers by construction —
//! and since ADR-0313 the watchdog needs exactly the same thing for exactly the
//! same reason, so the two read one number through one rule
//! ([`crate::governor::budgeted`]).
//!
//! This is the right place for three reasons and they are all already written
//! down in this repository:
//!
//! - The worker already does a submit-and-wait before handing a Set over (see
//!   "The window" above), so there is somewhere to measure that is not the
//!   render thread. [`crate::probe`]'s whole method is submit, wait, repeat.
//! - `crate::probe` already knows that GPU timestamps are advertised, enabled
//!   and unreliable on this machine, calibrates against a known-heavy workload
//!   rather than trusting the feature flag, and labels every number with how it
//!   was obtained. A governor steering on an unlabelled number is the failure
//!   mode a measurement that does not say how it was taken invites — see
//!   `docs/principles/0095-an-instrument-that-cannot-measure-says-so-rather-than-reporting-a-number.md`.
//! - A Set is the unit the governor budgets in, and it is the unit that gets
//!   built. Measuring it where it is built means the measurement travels with
//!   it and cannot get attached to the wrong one.
//!
//! **What the measurement is of**, exactly: one frame of this Set at
//! [`PROBE_STEPS`] simulation steps, at its real capacity and with its real
//! parameters and bindings already applied, rendered into an offscreen target
//! **at a size the caller names** — its L1 compute passes and its L4 draw, the
//! commands `VideoSource::render` records and nothing else.
//!
//! **The size used to be a constant here and is not any more**, which is
//! [ADR-0303](../../../docs/adr/0303-a-frames-cost-is-the-period-and-a-measurement-names-which-resolution-it-is-about.md).
//! `PROBE_RESOLUTION` was 1280x720, chosen because every other host-clock
//! figure in this repository is quoted there and argued as *comparable matters
//! more than absolute*. **This application has two resolutions and that was a
//! third**: the final output size, which the mix is composited once at and
//! which every output — the picture included — is a resize of (ADR-0247), and
//! the slot preview size, which each deck cell's own render is sized from. A
//! number taken at a size nothing draws is a number about a frame nobody sees,
//! and it only looked harmless while `karakuri`'s `CANVAS` happened to be the
//! same constant. So the size is named by whoever knows the layout —
//! [`HotSwap::set_measure_size`], seeded from the slot's own viewport by
//! [`crate::deck::Deck::new`] — and it travels out on
//! [`Measurement::resolution`] exactly as it always did.
//!
//! **What it cannot see**, and every one of these matters to whatever reads it:
//!
//! - **Resolution.** It is taken at the size it was told, which is not the
//!   deck's unless the caller said so. L4 cost is fill-rate bound, so a slot on
//!   a 4K output costs several times a preview-scale figure. The number is
//!   comparable *between slots measured at the same size* — which is what a
//!   budget needs — and is not a prediction of this machine's frame time.
//!   **This one now has an answer beside it**, and it is the only one of the
//!   four that does: [`crate::estimate`] fits `a + b·area` through two draws
//!   and evaluates it at the output's size, [`HotSwap::estimate_live`] is where
//!   a slot gets one, and the governor budgets on it in preference to this
//!   (ADR-0296). It is a second measurement rather than a correction to this
//!   one — both are kept, because they are statements about different sizes.
//! - **The composite, the meters, the present pass, and `prepare`.** None of
//!   them is inside `render`. The deck's own per-frame cost is not in here.
//! - **The future.** It is one measurement of a cold Set at one instant. A
//!   procedure whose population grows, whose points get bigger, or whose
//!   overdraw rises as it spreads costs more later, and nothing re-measures.
//! - **Anything host-side**, on the GPU path; and on the fallback path it
//!   includes submission and synchronization overhead and reads biased high.
//!   [`Measurement::method`] is which, and it is not decoration.
//!
//! Measuring means *stepping* the Set, which a Set about to be swapped in must
//! not arrive having done — it is documented as arriving cold. So the probe run
//! is followed by [`Set::rewind`], which puts the buffers, `t`, parity and the
//! spawn accumulator back to what `Set::build` left. That is another
//! whole-capacity upload on the worker thread, next to the one that was already
//! there.
//!
//! **The worker also flushes and waits for that upload before handing the Set
//! over.** A cold Set's first frames otherwise pay for the driver's first use of
//! each freshly created pipeline, first touch of freshly allocated buffers, and
//! — the big one — the whole-capacity element and alive buffer upload
//! `Set::build` leaves staged on the queue, megabytes at 262144 elements. Left
//! there, they would be flushed by whatever the render thread submitted next,
//! putting the upload inside the very frame the swap was supposed to be
//! invisible to. There used to be a `WARMUP_FRAMES` on this side as well,
//! discarding the first eight frames of a trial so that those costs could not
//! decide a verdict; it went with the trial (ADR-0313), because the verdict is
//! no longer taken from frames at all.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use karakuri_ir::typed::Checked;
use karakuri_ir::Kind;

use crate::binding::{Binding, Signals};
use crate::estimate::{estimate, Estimate};
use crate::governor::{self, Basis, Estimated};
use crate::probe::{Measurement, Probe};
use crate::set::{Authority, Set, SetError};

/// Frames the deck's period is taken over, as a rolling window. Half a second
/// at 60 Hz — long enough that the reading is not one sample's opinion on a
/// host clock, short enough that a deck which has stopped keeping up says so
/// within half a second.
///
/// **It used to be `JUDGE_FRAMES`, and it judges nothing now** (ADR-0313).
/// There was a `WARMUP_FRAMES` beside it, eight frames discarded after a swap
/// so that a cold Set's first frames — the driver's first use of each fresh
/// pipeline, first touch of fresh buffers, and the whole-capacity upload
/// `Set::build` leaves staged — did not decide a verdict. Both existed because
/// the verdict was a *frame interval*, which had to settle before it meant
/// anything. The verdict is now the candidate's own cost, taken on the worker
/// before the Set was ever handed over, and a number that was finished before
/// the first frame has nothing to settle: **the warmup is deleted rather than
/// kept for a reason it no longer has**. This window survives it because a
/// rolling median over a deck's frames is what the *alarm* is, and it is
/// rolling rather than restarted, so a swap's expensive frames are absorbed by
/// the window instead of being discarded by a count.
///
/// **Public because a caller reading [`HotSwap::frame_period_ms`] has to know
/// how many frames it waits for a first answer**, and because a test that
/// transcribed the number instead would get weaker rather than louder if this
/// moved.
pub const PERIOD_FRAMES: usize = 30;

/// The frame budget a candidate is judged against where nothing better is
/// known, in milliseconds: one 60 Hz frame plus slack.
///
/// **A fallback rather than the answer.** Since
/// [ADR-0313](../../../docs/adr/0313-a-candidate-is-judged-on-its-own-cost-and-the-decks-period-is-a-deck-level-alarm.md)
/// what a candidate is held against is *one frame of the display it is going
/// to be shown on*, and where the platform will say what that interval is the
/// caller passes it: `karakuri`'s window reads `refresh_rate_millihertz` and
/// hands it down, so a 120 Hz panel judges against 8.3 ms and a 60 Hz one
/// against 16.6. This is what is left when the platform names no monitor or no
/// refresh rate — a headless run, or a display `winit` cannot describe —
/// which is `P-0095`'s shape: an instrument that cannot say is not a licence to
/// invent a number, and the conservative reading of an unknown display is the
/// rate the simulation itself runs at.
///
/// **Why 20 and not 16.7, now that the quantisation argument is gone.** It used
/// to be a frame *interval* on a host clock, which under vsync quantises to
/// multiples of the refresh period, so the budget had to sit between one period
/// and two or a healthy frame would round its way into a rollback. That
/// argument died with the quantity: what is compared now is a probe reading of
/// one Set, taken offscreen, which is not quantised by anything. What replaces
/// it is the other half of the same instrument — on this crate's machines the
/// probe falls back to [`MeasurementMethod::HostWallClock`](crate::probe::MeasurementMethod),
/// which brackets a submit-and-wait the GPU never spent and **reads biased
/// high** (`docs/contributing.md` §1). A biased-high number against an exact
/// 16.7 ms line stops slots whose candidates would have fitted, which is the
/// false-reject direction this whole gate exists to stop doing. The slack is
/// now for the bias rather than for vsync, and it is the same three
/// milliseconds.
pub const DEFAULT_BUDGET_MS: f32 = 20.0;

/// Simulation steps per measured frame. One, because that is what a `tick`
/// carries in normal play; a Set that falls behind and substeps costs a
/// multiple of this, which is the caller's arithmetic rather than the probe's.
pub const PROBE_STEPS: u8 = 1;

/// How long a [`Source`] with nothing to report should block before returning.
/// The worker does nothing else between polls, so a source that returns
/// immediately turns it into a spin.
pub const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Everything the worker needs to build one Set. Owned, not borrowed: the
/// worker is on another thread and cannot hold anything belonging to the
/// render loop.
#[derive(Debug, Clone, Default)]
pub struct RequestNames {
    pub l1s: Vec<Option<String>>,
    pub l2s: Vec<Option<String>>,
    /// **A list, like the renderers'**, and one entry long for a Set with no
    /// camera procedure: the built-in orbit is a node too, and a caller may
    /// name it.
    pub l3s: Vec<Option<String>>,
    pub l4s: Vec<Option<String>>,
    /// **A list, like the renderers'** — a Set holds as many fields as its
    /// files declare, and each of them is a node an edge points at by name.
    pub fields: Vec<Option<String>>,
}

pub struct Request {
    /// The caller's own name for this build, echoed back on every event about
    /// it.
    ///
    /// **The engine never interprets it and never invents one.** What it is for
    /// is the thing a caller cannot otherwise do: say *which* build landed.
    /// `label` is for a human and repeats on every rebuild of the same pair,
    /// and the queue collapses superseded builds, so counting does not work
    /// either — a caller recording what a session actually played needs to
    /// match an outcome to the source that produced it, and this is the only
    /// thread between them.
    pub id: u64,
    /// **The geometry sources, each with the capacity it runs at.** A list
    /// because a Set holds a list — see `crate::set::Source` — and the capacity
    /// travels beside each one because each declares its own range, so one
    /// number cannot serve two.
    pub l1s: Vec<(Checked, u32)>,
    /// The deformations, in chain order — each reads what the one before wrote.
    /// Empty for a Set that draws its geometry as the L1 made it.
    pub l2s: Vec<Checked>,
    /// **The cameras, in node order.** Empty leaves the Set looking from the
    /// built-in orbit, which is a node of its own — so a Set has at least one
    /// camera whatever this list says, and `L3:0` addresses something in every
    /// Set.
    ///
    /// **A list, on the terms the fields and the renderers already had.** Which
    /// renderer draws from which camera is an `edge` and not a position, so
    /// several cost this nothing but a `Vec`.
    pub l3s: Vec<Checked>,
    /// **The fields, in node order.** Empty for a Set that evaluates none.
    pub fields: Vec<Checked>,
    /// The renderers, in draw order — see [`Set::build_many`]. A rebuild names
    /// every one of them rather than the one that changed, for the reason the
    /// bindings below are restated: a request that depended on what happens to be
    /// live is not reproducible from a record stream.
    pub l4s: Vec<Checked>,
    /// Whether the renderers overdraw or composite — see
    /// [`crate::set::Layering`]. Restated on every rebuild for the reason the
    /// bindings below are: a request that depended on what happens to be live is
    /// not reproducible from a record stream.
    pub layering: crate::set::Layering,
    /// **Which renderer the new Set is folded to**, applied as soon as it is
    /// built — where [`Request::camera`] is applied, and for its reason.
    /// `None` leaves every input live, which is what [`Set::build_many`] builds
    /// and what a Set nobody has selected in is.
    ///
    /// **Restated rather than carried over from the outgoing Set**, on the
    /// terms every other field here is. The symptom of leaving it out is the
    /// one [`Request::camera`] describes: a slot whose Set file recorded a
    /// selection comes back with every renderer folded in at once on the first
    /// rebuild, and nothing says so — a composited picture is not a broken one,
    /// it is a different one.
    ///
    /// **Ineffective under [`crate::set::Layering::Overdraw`]**, on
    /// [`Set::select_renderer`]'s terms and for its reason: the edges exist
    /// either way and nothing reads them without an L5.
    pub live: Option<u32>,
    pub seed_salt: u32,
    /// **One hash salt per geometry, and only what was assigned** — see
    /// [`Set::build_many`]. Empty, or an entry that is `None`, is a source
    /// whose salt is derived from `seed_salt` and its ordinal.
    ///
    /// **Restated on every rebuild**, on exactly the terms the bindings and the
    /// interface are, and with a sharper consequence than either: a rebuild that
    /// let the salts be derived afresh would re-salt a Set that was loaded with
    /// salts of its own, and every colour in it would change on the next save
    /// of a `.kir` that had nothing to do with the geometry.
    pub salts: Vec<Option<u32>>,
    /// **The built-in orbit's six numbers**, assigned to the new Set as soon as
    /// it is built — which is where the startup path puts a `camera` record,
    /// and before the params for that path's reason.
    ///
    /// **Restated on every rebuild**, on exactly the terms the salts and the
    /// bindings are, and with the sharpest consequence of any of them.
    /// [`Set::build_many`] starts every Set it builds from `Orbit::default()`,
    /// so while this field did not exist a `--load-set X --watch` put the
    /// camera back to its defaults on the first save of any `.kir`, saying
    /// nothing — and the live save that reads `Set::camera` faithfully then
    /// wrote those defaults into the operator's next preset. A rebuild that
    /// lets a value be re-derived is a rebuild that quietly discards what was
    /// loaded, and here the loss stopped being a wrong picture and became a
    /// file.
    ///
    /// **An `Orbit` rather than an `Option<Orbit>`, unlike the salts above.** A
    /// `None` salt means something: it derives from `seed_salt` and the
    /// source's ordinal, which is a different number from any a caller would
    /// have written. A `None` here would mean nothing — a Set holds a built-in
    /// camera whatever its files declare, and `build_many` starts it at exactly
    /// this default, so "no camera stated" and "the default stated" build the
    /// same Set. A caller that loaded none says so by sending the default,
    /// which is what it is.
    pub camera: crate::camera::Orbit,
    /// Applied to the new Set once it is built, and **what this build says a
    /// value is** — which is not the same as every value the Set will hold.
    ///
    /// **Parameter values are the one field here that is not restated on every
    /// rebuild**, and the one that could stop being. Everything else on this
    /// struct is restated because the engine would otherwise re-derive it: a
    /// salt, a camera, a fold, an edge and an authority all come back at some
    /// default the moment a request stops naming them. A parameter value does
    /// not — the outgoing Set is holding it, it knows which of its values
    /// somebody stated, and [`Set::carry_moved_from`] hands those across at the
    /// install. So the values an operator has moved travel on the Set, and this
    /// list is what a *caller* states: the values a slot was aimed with, on the
    /// build that aim causes. See `karakuri_environment::watch::Watch`'s
    /// `overrides`, which is that caller.
    ///
    /// **A stated value beats an inherited one**, and that clause is what
    /// separates a load from a rebuild: loading a Set file over a playing slot
    /// states every declaration of every node, which is what the file records,
    /// and the operator asked for that file rather than for the knobs they were
    /// holding. A rebuild of the same files states nothing and inherits
    /// everything somebody moved.
    ///
    /// **This does not make a request depend on what happens to be live.** The
    /// request is still a complete statement of what to build and what to state;
    /// what it no longer does is pretend to be a complete statement of what the
    /// Set will hold, which it never was — a rebuild's restatement writes no
    /// record either way
    /// (`docs/adr/0280-a-parameter-written-to-a-live-set-is-a-session-record.md`,
    /// §7). A replay does not come through here at all: it installs Sets through
    /// [`HotSwap::install`], which inherits nothing.
    pub params: Vec<crate::binding::ParamWrite>,
    /// **The interface, restated on every rebuild** for the reason the bindings
    /// and the edges are: a request that depended on what happens to be live is
    /// not reproducible from a record stream.
    ///
    /// Left out, this was worse than a lost surface. The bindings *are*
    /// restated, so a `control:` binding survived a swap and the control it
    /// named did not — and a binding whose source is gone leaves its param
    /// where it was, silently, for the rest of the run.
    pub published: Vec<crate::set::Published>,
    /// Attached to the new Set once its params are set, and **restated on every
    /// rebuild**: a rebuild that read its bindings out of whatever happened to
    /// be live would not be reproducible from a record stream. A binding is Set
    /// state, not Set structure — the new Set is a new value either way, and a
    /// `bind` record travels with it.
    ///
    /// **This is the field the ones around it point at**, and it used to be the
    /// params above: a binding is the restated field whose argument is still the
    /// plain one, where a parameter value has since become the one thing a Set
    /// can hand across a swap itself.
    ///
    /// **What this states is not the whole of what the new Set is bound by**,
    /// and that is the params' arrangement met one field along: an attachment
    /// made on a *live* slot — `Deck::bind`, a `source` record — is in no
    /// request, and [`crate::set::Set::carry_bound_from`] hands it across at
    /// the install for the reason [`crate::set::Set::carry_moved_from`] hands
    /// a ride across
    /// (`docs/adr/0339-a-rebuild-inherits-the-attachments-somebody-made.md`).
    /// A binding stated here still wins at its own address: what the request
    /// says is this build's answer, and only an address it did not name
    /// carries.
    pub bindings: Vec<Binding>,
    /// What a swap or verdict message calls this.
    /// What each node of the rebuilt Set is called, in the same per-layer shape
    /// the procedures are given in. Restated rather than carried over for the
    /// reason `bindings` is.
    pub names: RequestNames,
    /// **Which node fills each declared input slot** — see
    /// [`crate::set::Edge`]. Restated on every rebuild, and with a sharper
    /// consequence than the names beside it: a slot nothing binds is refused
    /// outright, so a request that left these out would turn every save of a
    /// morph's `.kir` into a rebuild that will not build.
    pub edges: Vec<crate::set::Edge>,
    /// **Who may move each node the operator has spoken for**, applied once the
    /// Set is built, and **restated on every rebuild** on exactly the terms the
    /// bindings, the names and the edges are: a request that
    /// depended on what happened to be live is not reproducible from a record
    /// stream.
    ///
    /// Left out, the loss is the one
    /// `docs/adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md`
    /// says the engine owes a field for: a node an operator granted to an agent
    /// — or took back from one — comes up at [`crate::set::Authority::default`]
    /// again on the next save of any `.kir`, saying nothing, and the arrangement
    /// between the operator and whatever else is in the room is silently a
    /// different one. `docs/principles/0078-…` is what a rebuild would be
    /// breaking, and a rebuild is not a surface, so no surface could refuse it.
    ///
    /// **Sparse, unlike [`Request::names`] and the salts.** An authority is
    /// what an operator *said*, one `Record::Authority` per saying, and a Set
    /// that nobody has spoken for states nothing here — an empty `Vec` is that
    /// Set, and every node of it lands on the default. A dense list would have
    /// to be as long as a node count the caller does not know until the build
    /// derives it, and would spell "nobody has said" and "manual" as two
    /// different entries when they are one arrangement.
    ///
    /// **A node this build no longer has is said and passed over**, on
    /// [`Request::live`]'s terms: the files were just recompiled and may name
    /// fewer nodes than the Set the authority was recorded against, which is a
    /// rebuild rather than an error.
    pub authorities: Vec<AuthorityAt>,
    pub label: String,
}

/// One node's authority, as a request states it — the engine's spelling of
/// `karakuri_operation::Operation::SetAuthority`'s `{ node, authority }`, with
/// the deck left off because a [`Request`] is already one slot's.
///
/// A struct rather than a bare tuple, so that the address and the level cannot
/// be swapped at a construction site, and addressed the way
/// [`crate::binding::ParamWrite`] is — except that `at` is not an `Option`
/// here: a bare name means *every node declaring it*, and there is no such
/// thing as an authority every node happens to declare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorityAt {
    /// Which node, `(layer, index)` — the address every surface in this system
    /// uses and the one `karakuri_operation::NodeAddress` carries.
    pub at: (Kind, u32),
    pub authority: Authority,
}

impl AuthorityAt {
    pub fn new(layer: Kind, index: u32, authority: Authority) -> AuthorityAt {
        AuthorityAt {
            at: (layer, index),
            authority,
        }
    }
}

/// **Why a source turned material down before anything was built**, on its
/// way to the render thread as [`Event::SourceRefused`].
///
/// # It is not a build, and every field here says so
///
/// There is no `id`. Every other thing the engine reports about carries one,
/// because a caller matches an outcome back to the [`Request`] that produced
/// it — and a refusal produced no request, put nothing in a store and filed no
/// version, so an id here would be a thread to a build that does not exist.
/// *Nothing compiled, so it is not a version* is the same rule the edit
/// history is gated on (`docs/adr/0089-history-is-gated-on-compiling-not-on-landing.md`).
///
/// # Formatted where the file was read, which is the worker
///
/// Both fields are `String`s the source built on the worker thread, beside the
/// four validation stages that produced them. The render thread moves them
/// into an [`Event`] and allocates nothing
/// (`docs/principles/0091-cost-is-known-before-it-is-paid.md`).
pub struct Refusal {
    /// **What the material calls itself** — the same slot in the sentence
    /// [`Request::label`] fills, and read the same way by whatever draws it.
    /// A source with no build to name it after names the file it was reading.
    pub label: String,
    /// **Every diagnostic, one line each**, in the order the checker produced
    /// them.
    ///
    /// One line rather than a rendered block because the readers on this side
    /// are a row and a status line; the full rendering, with the source line
    /// and the caret under it, is the source's own to print and it still does
    /// (`docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md`
    /// — every diagnostic at once, so the repair is one round trip).
    ///
    /// **Never empty**: a refusal with nothing to say is a silence with a
    /// message type, which is the thing this whole path exists to end.
    pub said: Vec<String>,
}

/// **What one poll answers**: material to build, or a refusal to report.
///
/// # Why the answer is one value and not two calls
///
/// `poll` returned `Option<Request>` until 2026-09-08, so a source that had
/// turned a file down had no way to say so: it printed its diagnostics and
/// answered `None`, and `None` is *nothing happened*. A `.kir` the checker
/// refuses is the opposite of nothing happening — it is the operator's newest
/// edit disagreeing with what is on screen — and the disagreement was said on
/// a terminal and on no surface at all
/// (`docs/adr/0310-a-source-can-say-it-refused-and-the-lane-draws-it.md`).
///
/// # Neither variant is boxed, and the lint that asks for it is answered here
///
/// A [`Request`] is the larger of the two by an order of magnitude, so
/// `clippy::large_enum_variant` asks for a `Box`. What that would buy is
/// nothing: exactly one of these exists at a time, on the worker's stack,
/// between a `poll` and the `match` that takes it apart — there is no
/// collection of them and nothing holds one. What it would cost is a heap
/// allocation and a deref on every build, and a `Box::new` at every
/// construction site in every implementation of the trait below, which is the
/// seam a caller writes against.
#[allow(clippy::large_enum_variant)]
pub enum Polled {
    /// Build this.
    Build(Request),
    /// **Nothing was built and nothing changed**, and here is why. The live
    /// Set keeps running, with its `t` and its live count untouched.
    Refused(Refusal),
}

/// Where the worker gets its work.
///
/// Implemented by the caller rather than here, because "what changed" is not
/// the engine's business: a file watcher, a record stream, and a queue of
/// generated material are the same shape from this side. A source with nothing
/// to say returns `None` and nothing happens; a source that *refused*
/// something — a `.kir` file the checker turned down — answers
/// [`Polled::Refused`], which builds nothing either and is reported rather than
/// passed over. That is how "a failed compile changes nothing" is still
/// enforced: a refusal never produces a [`Request`], so it cannot reach the
/// render thread as a Set to be rejected there.
///
/// Called only on the worker thread. `poll` is expected to block for roughly
/// [`POLL_INTERVAL`] when it has nothing to report.
pub trait Source: Send {
    fn poll(&mut self) -> Option<Polled>;
}

/// A channel is the simplest source there is: whoever holds the `Sender`
/// decides when a rebuild happens. Tests use this, and so would an agent
/// queueing generated material.
///
/// **It cannot refuse.** Whoever holds the `Sender` has a `Request` in hand,
/// which means whatever checking there was has already passed; a source that
/// wants to report a refusal implements the trait itself.
impl Source for Receiver<Request> {
    fn poll(&mut self) -> Option<Polled> {
        match self.recv_timeout(POLL_INTERVAL) {
            Ok(request) => Some(Polled::Build(request)),
            Err(RecvTimeoutError::Timeout) => None,
            // `Disconnected` returns immediately rather than after the
            // timeout, so sleep by hand — otherwise a dropped `Sender` turns
            // the worker into a spin until the `HotSwap` is dropped.
            Err(RecvTimeoutError::Disconnected) => {
                std::thread::sleep(POLL_INTERVAL);
                None
            }
        }
    }
}

/// What happened, in the order it happened. Drained by the caller and printed;
/// the engine does not print, so that a test can assert on the same values a
/// user reads.
#[derive(Debug)]
pub enum Event {
    /// A build finished and is now the live Set.
    ///
    /// **The verdict follows in the same drain**, since ADR-0313: the number
    /// the watchdog decides on was taken on the worker before this Set was
    /// handed over, so there is nothing to wait for and no window in which a
    /// candidate is "on trial". A caller reading events in order sees this and
    /// then either [`Event::Accepted`] or [`Event::Overloaded`] about the same
    /// `id`.
    ///
    /// **It is the only event that changes which Set is live**, since
    /// ADR-0316. The verdict that follows decides whether that Set is
    /// *stepped*, and never whether it is there.
    Swapped { id: u64, label: Arc<str> },
    /// A build failed. **Nothing changed**: the running Set is still running,
    /// with its `t` and its live count untouched.
    Rejected {
        id: u64,
        label: Arc<str>,
        error: SetError,
    },
    /// **The source turned material down before anything was built** — see
    /// [`Refusal`]. Nothing changed here either, and the difference from
    /// `Rejected` is which side of the request the refusal is on: `Rejected`
    /// is *this Set would not build*, and this is *there was never a Set to
    /// build*. Two words for one state would be a name meaning two things, so
    /// they are two states with two words.
    ///
    /// **No `id`**, unlike every variant around it: nothing was requested, so
    /// there is nothing for a caller to match it back to.
    SourceRefused { label: Arc<str>, said: Vec<String> },
    /// **The candidate stays and the slot goes on running.** The outgoing Set
    /// is released.
    ///
    /// **Since ADR-0316 the candidate stays either way**, and what this word
    /// carries is the other half: this slot is being stepped and drawn, where
    /// [`Event::Overloaded`]'s is not.
    ///
    /// Two ways to arrive here and [`Event::Accepted::cost_ms`] tells them
    /// apart: the candidate's own cost fitted the budget, or **nothing could
    /// measure the candidate at all**, in which case it was not judged. See
    /// that field.
    Accepted {
        id: u64,
        label: Arc<str>,
        /// **What one frame of this candidate costs**, and the number the
        /// verdict was reached on — not a frame interval and not this frame.
        ///
        /// [`None`] is the case `P-0084` is about: the probe run on the worker
        /// panicked or was never taken, nothing estimated the Set either, and
        /// there is no number. A candidate is **not rolled back on a number it
        /// does not have** — an instrument that declined to answer has not said
        /// the answer is large, any more than it has said it is small
        /// (`P-0095`) — so the operator's material stays and this says the
        /// verdict was not reached rather than that it was passed.
        /// [`Event::Accepted::basis`] is [`Basis::Unbudgetable`] there.
        cost_ms: Option<f32>,
        /// **Which of the candidate's two numbers that was**, on
        /// [`Decision::basis`](crate::governor::Decision::basis)'s terms and
        /// through the same [`governor::budgeted`] rule the governor decides
        /// on.
        basis: Basis,
        /// Reported alongside, because "held the budget" is not a useful
        /// thing to read without the number it held against — see
        /// [`DEFAULT_BUDGET_MS`], and note that a caller may have passed the
        /// display's own interval instead.
        budget_ms: f32,
    },
    /// **The watchdog's verdict, against: the candidate stays in the slot and
    /// the slot stops updating.**
    ///
    /// One frame of this Set costs more than a frame may, so
    /// [`HotSwap::overloaded`] is set and
    /// [`crate::deck::Frame::render`] skips this slot's `prepare` and its
    /// `render` from the next frame on. The slot's target keeps the last image
    /// it drew, which the composite and the deck's cells already read, so the
    /// slot costs **zero step and zero draw** and goes on contributing exactly
    /// the frame it was stopped at.
    ///
    /// **Nothing is put back**, and that is the change ADR-0316 records.
    /// The outgoing Set is retired here as it is on the accepted side: there
    /// is no rollback target, in this type or on [`HotSwap`]. A rollback
    /// restored the *picture* and not the *file*, so the next unrelated save
    /// reinstalled the over-budget version and the operator was never told
    /// which of the two they were looking at. Leaving the version they asked
    /// for exactly where they put it, and stopping it loudly, is
    /// `P-0094`'s *be loud* in place of its *undo* — the undo was not one.
    ///
    /// **Three ways out and all of them are the operator's**: a fader to zero
    /// on that slot (ADR-0040's zero-skip takes it out of the mix), a previous
    /// version landed out of the history (ADR-0308's `Revision::Previous`,
    /// which is a build like any other), or the next save — the freeze belongs
    /// to the *installed version*, so any build landing in this slot clears
    /// it. A residency change does **not**: a stopped slot taken off air and
    /// put back is still stopped, because what stopped is the version and not
    /// the placement.
    Overloaded {
        id: u64,
        label: Arc<str>,
        /// **The candidate's own cost**, which exceeded `budget_ms`.
        ///
        /// **An `f32` and not an `Option<f32>`, unlike [`Event::Accepted`]'s**,
        /// and that asymmetry is the rule in the type: a slot can only ever be
        /// stopped *through* a number, so there is no freeze without one to
        /// print. Making both optional would let a future edit stop a slot for
        /// a cost nobody measured, which is the defect ADR-0313 exists to end
        /// in its second form.
        cost_ms: f32,
        /// Which of the two it was — never [`Basis::Unbudgetable`], for the
        /// reason on `cost_ms`.
        basis: Basis,
        budget_ms: f32,
    },
    /// The build worker is gone — it can only leave by panicking — so nothing
    /// will be built again for the rest of the run. Emitted once. The live Set
    /// keeps running; what stops is the ability to replace it.
    WorkerLost,
}

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Swapped { label, .. } => {
                write!(f, "swapped in `{label}` — cold, t back to zero")
            }
            Event::Rejected { label, error, .. } => {
                write!(f, "`{label}` was refused, nothing changed:\n{error}")
            }
            // **Every diagnostic, not the first one.** The reader on the other
            // end of this string is a terminal or a model in a loop, and both
            // have room for the lot; the one-line-and-a-count reading is the
            // lane's, where a row is a row (`docs/principles/0083-…`).
            Event::SourceRefused { label, said } => write!(
                f,
                "`{label}` did not compile, nothing was built:\n{}",
                said.join("\n")
            ),
            // **Two sentences and not one with a hole in it.** A candidate
            // that held the budget and a candidate nothing could measure are
            // different facts, and a line reading "held the budget: none ms"
            // would be the second pretending to be the first.
            Event::Accepted {
                label,
                id: _,
                cost_ms: Some(cost_ms),
                basis,
                budget_ms,
            } => write!(
                f,
                "`{label}` held the budget: {cost_ms:.2} ms for its own frame \
                 against {budget_ms:.2} ms ({})",
                said(*basis)
            ),
            Event::Accepted {
                label,
                id: _,
                cost_ms: None,
                basis: _,
                budget_ms,
            } => write!(
                f,
                "`{label}` was not judged and stays: nothing measured it and nothing \
                 estimated it, so there is no number to hold against {budget_ms:.2} ms \
                 (see docs/principles/0084-…)"
            ),
            // **The word says the state and the sentence says what to do
            // about it.** A reader of this line — a terminal, a model in a
            // loop, the MCP surface — is being told about a slot that is
            // still holding the version they asked for and has stopped
            // running it, which is a thing they can end and nothing else
            // will.
            Event::Overloaded {
                label,
                id: _,
                cost_ms,
                basis,
                budget_ms,
            } => write!(
                f,
                "`{label}` is overloaded: {cost_ms:.2} ms for its own frame exceeds the \
                 {budget_ms:.2} ms budget ({} — see docs/contributing.md, working style). \
                 It is still in the slot and the slot has stopped updating; a fader to \
                 zero, an earlier version, or a build that fits is what ends it",
                said(*basis)
            ),
            Event::WorkerLost => write!(
                f,
                "the build worker is gone; nothing more will be built this run \
                 (the live Set is unaffected)"
            ),
        }
    }
}

/// **What kind of number a verdict was reached on, in words.** `P-0095`: a
/// figure a reader cannot check the provenance of is a figure they have to
/// take on trust, and these two are taken at different sizes (ADR-0303).
fn said(basis: Basis) -> &'static str {
    match basis {
        Basis::Measured => "one draw at the size the caller named, host clock",
        Basis::Estimated => "a two-draw fit at the output's size",
        // Unreachable from `Overloaded` by construction and printed by the
        // `Accepted` arm above instead; here so that a third basis added later
        // fails at a `match` rather than being passed over.
        Basis::Unbudgetable => "no number",
    }
}

/// **What the worker sends back**, which is a finished build or a refusal that
/// never became one.
///
/// One channel and not two. A second channel would put the two on separate
/// queues, so a refusal and the build that superseded it could arrive in
/// either order and the lane would draw whichever won — where what the operator
/// needs is the newest thing the source said, in the order it said it. This is
/// an `mpsc` and `mpsc` is FIFO, so one queue is the ordering.
///
/// Unboxed for [`Polled`]'s reason, one step along: the wasted room is the
/// difference between a refusal and a build, once per refusal, in an `mpsc`
/// node that is allocated either way.
#[allow(clippy::large_enum_variant)]
enum Done {
    Built(Built),
    /// Reported and never installed: there is no Set here, and no candidate.
    Refused(Refusal),
}

/// A finished build on its way back to the render thread.
struct Built {
    id: u64,
    label: Arc<str>,
    result: Result<Set, SetError>,
    /// What the worker measured this Set at, if it got that far. Travels with
    /// the Set rather than being looked up later, so it cannot be attached to
    /// the wrong one — see "The worker also measures what it built".
    cost: Option<Measurement>,
}

/// **How long the frames this `HotSwap` is being driven through are taking**,
/// as a rolling median of the interval between successive boundaries on a host
/// clock.
///
/// **It is the deck's number and it judges nothing** — see "The frame period is
/// the deck's, and it is an alarm" in the module doc. One interval covers every
/// slot's step and draw, the composite, the cell presents, the picture, the
/// `egui` pass and the vsync wait, so it cannot be divided among the slots that
/// produced it. What it can say is that the deck as a whole is not keeping up.
///
/// **Rolling rather than windowed from a start.** There is no event this
/// restarts at: a swap's expensive frames are absorbed by a thirty-frame median
/// rather than discarded by a warmup count, which is why `WARMUP_FRAMES` could
/// go with the verdict it was serving.
///
/// A fixed array and not a `Vec`, so that `mark` on the frame path writes one
/// `f32` into a slot that already exists and `median_ms` sorts on the stack.
struct Period {
    samples: [f32; PERIOD_FRAMES],
    /// Where the next interval goes. Wrapping to zero is what fills the ring.
    next: usize,
    filled: bool,
    /// The previous boundary, or [`None`] before there has been one.
    last: Option<Instant>,
}

// **There is no `Parked` here, and its absence is a decision.** The Set a swap
// displaced used to be held — first as three fields on `HotSwap` (`previous`,
// `previous_cost`, `previous_estimate`) across a thirty-eight-frame trial, then
// as a local struct across the one call the verdict is reached in (ADR-0313).
// A verdict against no longer puts anything back (ADR-0316), so the displaced
// Set is retired on both sides of the `match` and there is nothing left to
// hold. `HotSwap::previous` existing *only* inside the trial window is what
// ADR-0071 and `P-0084` rest on when they refuse a panic key; there is now no
// rollback target at all, in any lifetime, which is the same argument with
// nothing left to qualify. What replaces the put-back is
// `Revision::Previous` — a version out of the history, rebuilt and installed
// like any other build (ADR-0308, ADR-0089) — which is the operator's act and
// not the engine's.

impl Period {
    fn new() -> Period {
        Period {
            samples: [0.0; PERIOD_FRAMES],
            next: 0,
            filled: false,
            last: None,
        }
    }

    /// One frame boundary. The interval that just ended is the *previous*
    /// frame's duration, so this reading is always one frame behind — it has to
    /// be: a frame's cost is not known until the next one starts.
    fn mark(&mut self, now: Instant) {
        if let Some(last) = self.last.replace(now) {
            self.samples[self.next] = now.duration_since(last).as_secs_f32() * 1_000.0;
            self.next = (self.next + 1) % PERIOD_FRAMES;
            self.filled |= self.next == 0;
        }
    }

    /// The median of the window, or [`None`] until it has filled.
    ///
    /// **A median rather than a worst case**, for the reason `probe.rs` gives:
    /// on a host clock a single sample carries whatever else the OS scheduler
    /// was doing. **And `None` rather than a median of fewer samples**, because
    /// a partly filled window has not established that the deck is keeping up
    /// (`P-0095`).
    fn median_ms(&self) -> Option<f32> {
        if !self.filled {
            return None;
        }
        let mut sorted = self.samples;
        sorted.sort_by(|a, b| a.partial_cmp(b).expect("frame intervals are finite"));
        Some(sorted[PERIOD_FRAMES / 2])
    }
}

/// A live Set, a worker building the next one, and a watchdog over the swap.
///
/// Construct with [`HotSwap::new`] to get the worker, or [`HotSwap::fixed`]
/// for a run that will never swap — the frame loop is then identical either
/// way, which is the point of routing both through here.
pub struct HotSwap {
    live: Set,
    /// What the worker measured [`HotSwap::live`] at, if anything did. `None`
    /// for a Set no worker built — [`HotSwap::fixed`]'s, and the one `new` was
    /// constructed with — until [`HotSwap::set_measured_cost`] supplies one.
    cost: Option<Measurement>,
    /// **What two small draws say this Set would cost at the output's size**,
    /// if anything estimated it — [`HotSwap::estimate_live`], and
    /// [`Deck::estimate_slots`](crate::deck::Deck::estimate_slots) deck-wide.
    ///
    /// A second measurement rather than a refinement of the first: `cost` is
    /// one draw at whatever size [`HotSwap::set_measure_size`] last named and
    /// this is a fit through two, so it answers for the size the deck is
    /// actually drawing. The governor prefers it where it answers — see "Two
    /// numbers, and which one is budgeted on" in [`crate::governor`].
    ///
    /// **Since ADR-0303 the two can be at different scales**, and a budget that
    /// sums one of each is summing an audition and a frame. Which way that
    /// goes is not settled here; see that record.
    ///
    /// **Dropped whenever it would stop being about this Set at this size**: a
    /// build landing (`install_if_ready`), a replay's `install`, and a
    /// `resize`, which changes the very target the number is for. There is no
    /// estimate on the worker's side to replace it with, because the worker
    /// does not know the output's size.
    estimate: Option<Estimate>,
    /// **What a candidate's own cost is held against**, in milliseconds of one
    /// frame — the display's own refresh interval where the caller knew one,
    /// [`DEFAULT_BUDGET_MS`] where it did not. Written by
    /// [`HotSwap::set_budget_ms`].
    budget_ms: f32,
    /// **The live Set costs more than one frame may, so it is not being
    /// stepped or drawn** — see [`HotSwap::overloaded`], which is the whole of
    /// what this is and where it is argued.
    ///
    /// **A property of the installed version and not of the slot**, which is
    /// what decides where it is written: [`HotSwap::judge`] sets it, and every
    /// path that replaces `live` clears it — `install_if_ready` before the
    /// verdict, and [`HotSwap::install`] for a replay. Nothing else writes it.
    /// A residency change is not one of those paths, deliberately: a stopped
    /// slot taken off air and put back is still stopped, because the version
    /// in it has not changed.
    overloaded: bool,
    viewport: (u32, u32),
    /// **The deck's frames, measured and never used as a verdict** — see
    /// [`Period`] and [`HotSwap::frame_period_ms`].
    period: Period,
    frames: u64,
    /// Sized for the most that can accumulate between two drains, and every
    /// `Event` owns its strings already, so pushing one allocates nothing. A
    /// caller that stops draining eventually makes this grow — which is a
    /// caller bug rather than a case to handle here, because the alternative
    /// is silently discarding a rollback nobody was told about.
    events: Vec<Event>,

    /// **What the worker has finished with** — a build, or a refusal that
    /// never became one. See [`Done`], and `install_if_ready`, which is the
    /// only reader.
    done: Receiver<Done>,
    /// Sets the render thread is done with, waiting for the worker to drop
    /// them. Dropping a Set releases its buffers, bind groups and pipelines,
    /// and a deallocation on the render thread is the same invariant as an
    /// allocation on it.
    /// **The size the next measurement is taken at**, shared with the worker
    /// because the worker measures what it builds and the render thread is
    /// what knows the layout. Packed as `(width << 32) | height`, written by
    /// [`HotSwap::set_measure_size`] and by nothing else.
    ///
    /// **A resize does not write it.** The output size moving is not the same
    /// event as the size a measurement is *about* moving — a preview cell is
    /// sized from its cell and not from the output — so whoever named the
    /// measurement size names it again. [`crate::deck::Deck::new`] seeds it
    /// with the deck's own size, so a caller that never names one measures at
    /// the output size, which is one of this application's two real
    /// resolutions rather than a third (ADR-0303).
    measure_at: Arc<AtomicU64>,
    graveyard: Arc<Mutex<Vec<Set>>>,
    /// Retired but not yet handed over, because the worker held the graveyard
    /// lock this frame. Retried next frame rather than dropped here.
    retired: Vec<Set>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    /// The worker was expected and its channel is closed, and that has been
    /// reported once. A worker only closes it by panicking — `run_worker`
    /// otherwise runs until [`HotSwap::drop`] — and the symptom without this
    /// is a `--watch` session that silently stops responding to saves.
    worker_lost: bool,
}

impl HotSwap {
    /// A Set with a worker behind it. `source` is polled on the worker thread;
    /// anything it produces is built there and offered to the render thread on
    /// the next frame boundary.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        live: Set,
        budget_ms: f32,
        source: Box<dyn Source>,
    ) -> HotSwap {
        let (done_tx, done_rx) = mpsc::channel();
        let graveyard = Arc::new(Mutex::new(Vec::with_capacity(GRAVEYARD_CAPACITY)));
        let stop = Arc::new(AtomicBool::new(false));
        // **The viewport, until somebody names a size.** `Set::build` leaves a
        // Set at 1x1 and `Deck::new` resizes every slot before anything can
        // measure, so the deck's own size is what a run that never calls
        // `set_measure_size` measures at — one of this application's two
        // resolutions, and never a third (ADR-0303).
        let measure_at = Arc::new(AtomicU64::new(packed(live.viewport())));

        let worker = {
            // Both are `Arc`s inside, so this is a refcount bump rather than a
            // second device — the worker builds against the same device the
            // render thread renders with, which is the whole reason the
            // finished pipelines are usable when they arrive.
            let device = device.clone();
            let queue = queue.clone();
            let graveyard = Arc::clone(&graveyard);
            let stop = Arc::clone(&stop);
            let measure_at = Arc::clone(&measure_at);
            std::thread::Builder::new()
                .name("karakuri-build".into())
                .spawn(move || {
                    run_worker(device, queue, source, done_tx, graveyard, stop, measure_at)
                })
                .expect("spawn build worker")
        };

        HotSwap {
            live,
            cost: None,
            estimate: None,
            budget_ms,
            overloaded: false,
            viewport: (1, 1),
            period: Period::new(),
            frames: 0,
            events: Vec::with_capacity(EVENT_CAPACITY),
            done: done_rx,
            measure_at,
            graveyard,
            retired: Vec::with_capacity(GRAVEYARD_CAPACITY),
            stop,
            worker: Some(worker),
            worker_lost: false,
        }
    }

    /// Put `set` in, now, with no worker and no trial.
    ///
    /// **For a replay, and only a replay.** A live run's swaps are judged on
    /// the candidate's own measured cost as they land; a replay is reading what
    /// a live run already decided, out of a `procedure` record, so there is
    /// nothing left to judge. Judging again would be worse than pointless — an
    /// offscreen render has no frame budget to fail, and a slot stopped here
    /// that the live run never stopped would put the replay on a still the
    /// performance never showed.
    ///
    /// The outgoing Set is retired, as it is on the live path: nothing here
    /// holds a version to put back. Its measured cost goes with it, so the governor treats the
    /// incoming one as unmeasured — which is what it is.
    pub fn install(&mut self, device: &wgpu::Device, mut set: Set) {
        set.resize(device, self.viewport.0, self.viewport.1);
        let outgoing = std::mem::replace(&mut self.live, set);
        self.retire(outgoing);
        // **The freeze is the outgoing version's and goes with it** — see
        // [`HotSwap::overloaded`]. A replay that installed a Set into a slot
        // stopped by a live run's verdict would draw nothing at all, which is
        // a picture the performance never showed.
        self.overloaded = false;
        self.cost = None;
        // The estimate went with the Set it was taken of. Nothing carries one
        // across a replaced Set: it is a fit through two draws of *that*
        // material, and the incoming Set is unestimated exactly as it is
        // unmeasured.
        self.estimate = None;
    }

    /// One Set and no worker, for `--render`, `--seq`, and a window run
    /// without `--watch`. Nothing will ever be swapped in, so nothing is ever
    /// measured against a budget either — but the frame loop is the same one.
    pub fn fixed(live: Set) -> HotSwap {
        // A receiver whose sender is already gone: `try_recv` says
        // `Disconnected` forever, which `install_if_ready` treats exactly like
        // "nothing waiting".
        let (_, done_rx) = mpsc::channel();
        let live_viewport = live.viewport();
        HotSwap {
            live,
            cost: None,
            estimate: None,
            budget_ms: f32::INFINITY,
            // Nothing will ever be judged here, so nothing can ever stop this
            // slot: `fixed` is a Set and no worker.
            overloaded: false,
            viewport: (1, 1),
            period: Period::new(),
            frames: 0,
            events: Vec::with_capacity(EVENT_CAPACITY),
            done: done_rx,
            measure_at: Arc::new(AtomicU64::new(packed(live_viewport))),
            graveyard: Arc::new(Mutex::new(Vec::new())),
            retired: Vec::new(),
            stop: Arc::new(AtomicBool::new(false)),
            worker: None,
            worker_lost: false,
        }
    }

    /// The top of a frame, and the only place the live Set is ever replaced.
    ///
    /// Everything that can change which Set is live happens here, before the
    /// borrow is handed out: the previous frame's interval is fed to the
    /// deck alarm, and then a finished build is installed and judged if one
    /// has arrived — which may stop this slot, and never takes it back. The caller records its whole frame through the
    /// returned reference, and cannot touch this `HotSwap` again until it
    /// drops it.
    ///
    /// **Call this exactly once per frame.** Calling it a second time inside
    /// one command encoder compiles, and records two Sets' passes into that
    /// encoder; see "Where a swap lands, and what actually guarantees it" in
    /// the module doc for why the borrow alone does not rule that out.
    /// [`crate::deck::Deck::begin_frame`] is the version that does rule it
    /// out, by owning the encoder — a `HotSwap` driven directly still relies
    /// on the caller.
    ///
    /// Allocates nothing, compiles nothing, and never blocks: the channel is
    /// polled with `try_recv` and the graveyard with `try_lock`.
    pub fn begin_frame(&mut self, device: &wgpu::Device) -> &mut Set {
        self.frame_boundary(device);
        &mut self.live
    }

    /// The same frame boundary for a slot that is **not on air**, and since
    /// ADR-0313 it is the same work: builds install, retired Sets go back to
    /// the worker, the candidate is judged, and the deck's frame period is
    /// marked. The only difference left is that no `&mut Set` comes back,
    /// because an off-air slot's Set is reached through
    /// [`crate::deck::Frame`] instead.
    ///
    /// **The two used to differ, and the difference was a symptom.** An off-air
    /// slot's trial was *frozen* here — no sample taken, `seen` not advanced,
    /// the verdict waiting until the slot was Live — because the number being
    /// judged was the whole deck's frame interval, and judging a candidate
    /// nobody was drawing against work its neighbours were doing accepts it on
    /// a budget it never spent and rolls it back for cost it never caused. That
    /// was as far as an interval-based watchdog could get, and it did not reach
    /// the case that mattered: a *Live* slot's candidate was still judged
    /// against its neighbours' cost.
    ///
    /// The verdict is now the candidate's own measured cost, which is the same
    /// number whether the slot is on air or not, so there is nothing left to
    /// freeze — and a candidate on an off-air slot no longer waits an unbounded
    /// time for a verdict it could have had at the install.
    pub(crate) fn begin_frame_parked(&mut self, device: &wgpu::Device) {
        self.frame_boundary(device);
    }

    fn frame_boundary(&mut self, device: &wgpu::Device) {
        // **Marked whatever this slot's residency is**, because what it
        // measures is not this slot: it is the interval between the caller's
        // frames, which in a deck is one number for four slots and is the
        // deck's. See [`Period`].
        self.period.mark(Instant::now());
        self.frames += 1;
        self.hand_over_retired();
        self.install_if_ready(device);
    }

    /// The live Set, outside a frame. Read-only, so it cannot be rendered
    /// through — recording a frame goes through [`HotSwap::begin_frame`], and
    /// that is deliberate.
    pub fn set(&self) -> &Set {
        &self.live
    }

    /// The live Set, mutably, with **none** of the frame-boundary work
    /// [`HotSwap::begin_frame`] does: no watchdog sample, no install, no
    /// handover.
    ///
    /// This exists for [`crate::deck::Frame`], which calls `begin_frame` on
    /// every slot at the top of a frame and then has to reach those same Sets
    /// again once its encoder is open. Splitting the two is safe there
    /// precisely because the deck's guard holds the encoder: between the
    /// install and this call there is no point at which a caller could have
    /// obtained a different Set.
    ///
    /// `pub(crate)`, and it should stay that way. Handing this out publicly
    /// would be a second way to render a Set, next to the one that installs
    /// builds on a frame boundary, and "which Sets is this frame made of"
    /// would stop having one answer.
    pub(crate) fn live_mut(&mut self) -> &mut Set {
        &mut self.live
    }

    /// The currently live Set. Read-only inspection of the Set this slot is playing.
    pub fn live(&self) -> &Set {
        &self.live
    }

    /// Frames begun since construction. What "a build does not block the
    /// render loop" is measured in: the count between a request going out and
    /// its [`Event::Swapped`] coming back is how many frames were produced
    /// while the worker was busy.
    pub fn frames_rendered(&self) -> u64 {
        self.frames
    }

    /// The budget the watchdog is holding candidates to, in milliseconds of
    /// **one frame of one candidate** — see [`DEFAULT_BUDGET_MS`].
    pub fn budget_ms(&self) -> f32 {
        self.budget_ms
    }

    /// **Whether the live Set has been stopped for costing more than one frame
    /// may** — [`Event::Overloaded`], and ADR-0316.
    ///
    /// **What a caller must do about it**: skip this slot's `prepare` and its
    /// `render`. [`crate::deck::Frame::render`] does, which is the whole of
    /// the behaviour on the deck path; a caller driving a `HotSwap` on its own
    /// — `karakuri-cli`, `tests/hot_swap.rs` — reads this or goes on stepping
    /// a Set the watchdog said is too expensive.
    ///
    /// **The Set is still there and the target still holds its last image.**
    /// A stopped slot is not an empty one: nothing was taken out, nothing was
    /// put back, and what the composite and the deck's cells read is the frame
    /// it was stopped at. That is why the word reaches every surface — an
    /// unmarked still is a preview that lies (ADR-0269).
    ///
    /// **It clears when a build lands and at no other time**, because the
    /// freeze belongs to the version rather than to the slot. A fader to zero
    /// takes a stopped slot out of the mix and leaves it stopped; a residency
    /// change moves it and leaves it stopped; landing an earlier version out
    /// of the history clears it, because that is a build.
    pub fn overloaded(&self) -> bool {
        self.overloaded
    }

    /// **Say what one frame of the display this is being shown on may cost.**
    ///
    /// A `HotSwap` is constructed before there is a window on many paths — the
    /// deck is built from the launch pair and the surface is created after it —
    /// so the budget starts at whatever the constructor was given and is
    /// narrowed here once the platform will say. `karakuri`'s window does that
    /// once, when it opens, through
    /// [`Deck::set_frame_budget_ms`](crate::deck::Deck::set_frame_budget_ms).
    ///
    /// **It changes no verdict already reported.** A verdict is reached in the
    /// call the swap lands in, so there is no candidate part-way through a
    /// window whose budget could move underneath it (ADR-0313).
    ///
    /// A non-finite or non-positive budget is refused rather than stored: it
    /// would make every candidate fail or every candidate pass without saying
    /// so, which is the silent shape `P-0095` rules out. [`HotSwap::fixed`]'s
    /// infinity is set by the constructor and is a statement that there is no
    /// worker and nothing to judge, not a budget a caller passed.
    pub fn set_budget_ms(&mut self, budget_ms: f32) {
        if budget_ms.is_finite() && budget_ms > 0.0 {
            self.budget_ms = budget_ms;
        }
    }

    /// **How long the frames this is being driven through are taking**, as a
    /// median over the last [`PERIOD_FRAMES`] of them on a host clock, or
    /// [`None`] until that window has filled.
    ///
    /// **It is the caller's frame and not this slot's**, and in a deck that
    /// means it is the same number on every slot: one interval covers every
    /// slot's step and draw, the composite, the cell presents, the picture, the
    /// `egui` pass and the vsync wait. **Nothing may divide it among the slots
    /// that produced it**, which is why it is no longer a candidate's verdict
    /// (ADR-0313) and why the only thing entitled to read it is a deck-level
    /// alarm: [`Deck::frame_period_ms`](crate::deck::Deck::frame_period_ms) and
    /// [`Report::deck_over_period`](crate::governor::Report::deck_over_period).
    ///
    /// Under vsync it quantises to multiples of the refresh period, because a
    /// frame that misses one lands at the next and there is nothing in between.
    pub fn frame_period_ms(&self) -> Option<f32> {
        self.period.median_ms()
    }

    /// **What one frame of the live Set costs**, as measured when it was built
    /// — not a frame interval, and not this frame.
    ///
    /// `None` means nothing has measured this Set, which is not the same as
    /// "it is free": [`HotSwap::fixed`] and the Set [`HotSwap::new`] is
    /// constructed with arrive unmeasured, and stay that way until
    /// [`HotSwap::measure_live`] is called. [`crate::governor`] treats an
    /// unmeasured Set as unbudgetable rather than as zero, which is the only
    /// safe reading — **including when the slot is Live**, where it makes the
    /// deck's committed cost unknown and suspends priming. Measuring the Sets
    /// a caller built itself is therefore a startup step rather than an
    /// optional refinement; see
    /// [`Deck::measure_slots`](crate::deck::Deck::measure_slots).
    ///
    /// Read [`Measurement::method`] before trusting the number; read
    /// "The worker also measures what it built" in the module doc for what it
    /// is a measurement of and what it cannot see.
    pub fn measured_cost(&self) -> Option<Measurement> {
        self.cost
    }

    /// **Measure the Set that is live now**, and keep the result.
    ///
    /// For a Set no worker built — [`HotSwap::fixed`]'s, or the one handed to
    /// [`HotSwap::new`] — which is otherwise invisible to the governor, and
    /// which makes the whole deck unbudgetable for as long as it is on air and
    /// unmeasured. This is the reachable form of [`measure`]: the live `Set` is
    /// not handed out mutably (that is `live_mut`, and it is `pub(crate)` for
    /// good reasons), so without this a caller cannot measure a Set it has
    /// already put into a `HotSwap`.
    ///
    /// **Never on the render thread and never inside a frame.** It submits and
    /// waits, once per sample, and it steps the Set and rewinds it — which is
    /// another whole-capacity upload. Startup is where it belongs.
    ///
    /// **On a Set that has already stepped, this is destructive.** [`measure`]
    /// ends in [`Set::rewind`], which restores what `Set::build` left rather
    /// than what this call found: `t` back to zero, element buffers cold. That
    /// is exactly right for the cold Set it is meant for and is a reset for any
    /// other, so measure before the first frame.
    /// [`Deck::measure_slots`](crate::deck::Deck::measure_slots) skips a slot
    /// that has stepped for this reason; a caller reaching this directly is
    /// holding the check itself.
    ///
    /// `probe` is taken rather than constructed for the reason [`measure`]
    /// gives: calibration is expensive and two probes can disagree about this
    /// adapter, producing numbers a governor would then be summing.
    /// [`Deck::measure_slots`](crate::deck::Deck::measure_slots) is the
    /// deck-wide version and constructs one probe for all of them.
    pub fn measure_live(
        &mut self,
        probe: &mut Probe,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Measurement {
        let at = self.measure_size();
        let cost = measure(probe, device, queue, &mut self.live, at);
        self.cost = Some(cost);
        cost
    }

    /// Attach a measurement to the Set that is live now.
    ///
    /// [`HotSwap::measure_live`] is the one that takes it; this is for a caller
    /// that has a number from somewhere else — a test pinning exact
    /// arithmetic, or a measurement carried across a rebuild of the same Set.
    ///
    /// Replaced wholesale by the next build that lands, since that
    /// measurement is of a different Set.
    pub fn set_measured_cost(&mut self, cost: Measurement) {
        self.cost = Some(cost);
    }

    /// **What the live Set is estimated to cost at the size it was estimated
    /// for**, or [`None`] where nothing has estimated it.
    ///
    /// The [`Estimate`] and not a millisecond figure, because
    /// [`Estimate::ms`] alone is never enough to act on: `P-0095` puts the
    /// instrument, the two rungs, the floor it was placed against
    /// ([`Estimate::floor_from`]) and how strictly that floor was read
    /// ([`Estimate::floored`]) on the same object, and a consumer that cannot
    /// see them cannot check the number. [`crate::governor`] reads a Copy
    /// summary of exactly those; the whole record stays here.
    ///
    /// `None` is not "free" and not "cheap", on the same terms as
    /// [`HotSwap::measured_cost`]. It means the slot is budgeted on its
    /// measurement instead, and where there is no measurement either it is
    /// unbudgetable.
    pub fn estimated_cost(&self) -> Option<&Estimate> {
        self.estimate.as_ref()
    }

    /// **Estimate the Set that is live now at `target`**, and keep the result.
    ///
    /// The two-draw counterpart of [`HotSwap::measure_live`], and it carries
    /// every one of that call's warnings: **never on the render thread and
    /// never inside a frame** — it submits and waits, now twice over — and
    /// **destructive on a Set that has stepped**, because
    /// [`crate::estimate::estimate_above_floor`] ends in
    /// [`Set::rewind`](crate::set::Set::rewind).
    /// [`Deck::estimate_slots`](crate::deck::Deck::estimate_slots) is the
    /// deck-wide version, holds the "has not stepped" check, and constructs one
    /// probe for every slot.
    ///
    /// `target` is the size the number is *for* — the output's, which since
    /// ADR-0246 is a per-output question rather than a global one. It is
    /// recorded on [`Estimate::target`], and [`HotSwap::resize`] drops the
    /// estimate when that size moves.
    ///
    /// **An estimate that refuses is stored too**, and deliberately: the
    /// refusal names what would have to change, carries the rungs and the
    /// floor, and is what a status line shows instead of a number. What it is
    /// not is a licence — the governor falls back to the measurement, and to
    /// [`Reason::Unmeasured`](crate::governor::Reason::Unmeasured) where there
    /// is not one.
    pub fn estimate_live(
        &mut self,
        probe: &mut Probe,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: (u32, u32),
    ) -> &Estimate {
        let e = estimate(probe, device, queue, &mut self.live, target);
        self.estimate.insert(e)
    }

    /// Attach an estimate to the Set that is live now.
    ///
    /// [`HotSwap::estimate_live`] is the one that takes it; this is for a
    /// caller that has one from somewhere else — a test pinning exact
    /// arithmetic, which is what most of the governor's is pinned with.
    ///
    /// Replaced wholesale by the next build that lands and dropped by the next
    /// [`HotSwap::resize`], for the reasons on the field.
    pub fn set_estimated_cost(&mut self, estimate: Estimate) {
        self.estimate = Some(estimate);
    }

    /// Everything that has happened since this was last called.
    pub fn events(&mut self) -> std::vec::Drain<'_, Event> {
        self.events.drain(..)
    }

    /// The same events, read without taking them.
    ///
    /// [`HotSwap::events`] is the caller's — it drains, because a caller that
    /// reads an event twice would print one verdict twice. Something that has
    /// to *react* to a swap rather than report it cannot use that without
    /// stealing it, so this is the read-only view: `Deck::begin_frame` notes
    /// the length before the frame boundary and looks at what was appended,
    /// which is how it knows a build landed on a slot and its level meter is
    /// now measuring different material. It changes nothing and consumes
    /// nothing.
    pub fn pending_events(&self) -> &[Event] {
        &self.events
    }

    /// Remembered as well as forwarded: a Set built while the window was one
    /// size must not arrive on screen still believing it.
    ///
    /// **There is no parked Set to resize any more.** A rollback target used to
    /// live here across a thirty-frame trial and had to be resized with the
    /// live one; the verdict is reached in the same call the swap lands in
    /// since ADR-0313 and puts nothing back since ADR-0316, so an outgoing Set
    /// is released before this can be called again. **A stopped slot is
    /// resized like any other**: it is not drawing, and the day it is drawn
    /// again — the next build — it must already be the right size.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.viewport = (width, height);
        self.live.resize(device, width, height);
        // **An estimate is a number *at a target*, and the target just
        // moved.** `Estimate::target` says which size it answered for, so a
        // kept one would be a right number about a frame nobody is drawing any
        // more — and the governor would spend it. Dropping it puts the slot
        // back on its measurement until something estimates it again, which is
        // the fallback the whole wiring is built around.
        self.estimate = None;
    }

    /// **Name the size the next measurement is taken at.**
    ///
    /// The caller is whoever knows the layout, because nothing in this crate
    /// does: this application has an output size and a preview size and no
    /// third one (ADR-0247, ADR-0303), and which of the two a number is about
    /// is a fact about the number. It reaches the build worker as well as the
    /// render thread, so a candidate built mid-session is measured at the same
    /// size as the Set it is a candidate for.
    ///
    /// **It is not written by [`HotSwap::resize`]**, and that is the rule
    /// rather than an omission: the output size moving is a different event
    /// from the size a measurement is about moving, and a preview cell is
    /// sized from its cell. A caller that narrows this to a preview says so
    /// again after a resize — `karakuri`'s window does it once a frame, where
    /// it aims the cells.
    ///
    /// A degenerate size is refused rather than stored: a zero-area target
    /// measures nothing and `wgpu` will not allocate one. The previous size
    /// stands, which is the conservative direction — the alternative is a
    /// measurement of a texture one pixel across.
    pub fn set_measure_size(&mut self, at: (u32, u32)) {
        if at.0 == 0 || at.1 == 0 {
            return;
        }
        self.measure_at.store(packed(at), Ordering::Relaxed);
    }

    /// The size [`HotSwap::set_measure_size`] last named, or the viewport this
    /// was constructed at.
    pub fn measure_size(&self) -> (u32, u32) {
        unpacked(self.measure_at.load(Ordering::Relaxed))
    }

    /// **Judge the candidate that has just been installed, on the candidate's
    /// own number**, and stop the slot if it does not fit.
    ///
    /// # What a verdict against does, and what it deliberately does not
    ///
    /// It sets [`HotSwap::overloaded`], and that is the whole of it: the
    /// candidate stays live, the displaced Set is retired exactly as it is on
    /// the other side, and the slot's `prepare` and `render` are skipped from
    /// the next frame on. **Nothing is put back** (ADR-0316). A rollback put
    /// the previous *Set* back and could not put the previous *file* back, so
    /// the picture and the disk disagreed with nothing saying so and the next
    /// unrelated save reinstalled the over-budget version; a stopped slot says
    /// what it is, on three surfaces, and ends when the operator ends it.
    ///
    /// # What is compared, and what is deliberately not
    ///
    /// The left-hand side is what one frame of *this* Set costs — the estimate
    /// where one answers, the probe measurement the worker took where it does
    /// not, through [`governor::budgeted`], which is the same rule
    /// [`Governor::decide`](crate::governor::Governor::decide) sums the
    /// committed cost with and admits priming slots on. One rule and not two,
    /// so a Set cannot be stopped here on one reading and admitted there on
    /// another (`P-0085`).
    ///
    /// The right-hand side is [`HotSwap::budget_ms`]: one frame of the display,
    /// where the caller knew what that is.
    ///
    /// **Nothing about the other slots appears on either side**, and that is
    /// the decision rather than a simplification. *"判定は他のスロットのロードとは
    /// 独立にあるべきだね"* — the verdict on a candidate must be independent of
    /// what the other slots are carrying. A budget divided by the live slot
    /// count, or a share taken beside what the neighbours are committed to,
    /// would both put a heavy neighbour back on the left-hand side by another
    /// route: the same candidate would be kept on an empty deck and thrown out
    /// on a busy one, which is what this is repairing. What the deck's total
    /// costs is a deck-level question and has a deck-level answer —
    /// [`Report::deck_over_period`](crate::governor::Report::deck_over_period),
    /// which warns and never acts, on
    /// [`crate::governor`]'s "What it does not touch" terms.
    ///
    /// # The candidate with no number
    ///
    /// The probe run on the worker is caught rather than propagated, so a build
    /// can arrive with `cost: None`; nothing has estimated an incoming Set
    /// either. A candidate in that state is **kept, run, and reported as not
    /// judged**. Its slot is not stopped, because stopping it would be
    /// deciding it is over a budget on a number it does not have, which is
    /// `P-0084`'s confident wrong judgement exactly — and `P-0095`'s: an
    /// instrument that declined to answer has not said the answer is large.
    ///
    /// **That is the opposite direction from [`crate::governor`]'s**, which
    /// parks an unmeasured slot with
    /// [`Reason::Unmeasured`](crate::governor::Reason::Unmeasured), and the two
    /// are not in conflict because they are answering different questions. The
    /// governor is deciding whether to *spend* budget nobody asked it to spend,
    /// where refusing costs a warm-up. This is deciding whether to take away
    /// material the operator asked for, where refusing costs the thing they
    /// asked for — and `P-0094` will not buy safety with the operator's
    /// authority. The deck alarm still fires if the frames actually stop
    /// arriving.
    fn judge(&mut self, id: u64, label: Arc<str>) {
        let (basis, budgeted) =
            governor::budgeted(self.cost, self.estimate.as_ref().map(Estimated::from));
        // **A number that is not a number is not a number.** `is_nan` is
        // checked rather than left to fall out of the comparison below, which
        // would answer `false` for a NaN and keep the candidate anyway — the
        // same outcome, silently and by accident. Here it is the *unjudged*
        // outcome and says so, which is a different fact from having fitted.
        let judged = budgeted.filter(|ms| ms.is_finite());
        let basis = match judged {
            Some(_) => basis,
            None => Basis::Unbudgetable,
        };
        match judged {
            Some(cost_ms) if cost_ms > self.budget_ms => {
                // **The candidate stays and the slot stops.** Nothing is
                // replaced here: `live` is the Set the operator asked for and
                // goes on being what this slot holds, its measurement is the
                // one the governor budgets on, and the only state that changes
                // is this flag. The displaced Set was already retired at the
                // install — there is no rollback target to keep.
                self.overloaded = true;
                self.events.push(Event::Overloaded {
                    id,
                    label,
                    cost_ms,
                    basis,
                    budget_ms: self.budget_ms,
                });
            }
            _ => {
                self.events.push(Event::Accepted {
                    id,
                    label,
                    cost_ms: judged,
                    basis,
                    budget_ms: self.budget_ms,
                });
            }
        }
    }

    /// Install a finished build, if one is waiting, and judge it.
    ///
    /// **There is no longer a gate at the top of this.** A candidate on trial
    /// used to hold the channel shut, because `previous` was the one rollback
    /// target and admitting a second candidate would have lost the only Set
    /// known to work; a build that finished during a trial waited in the
    /// channel, and so did a *refusal* behind it — thirty-eight frames of
    /// latency on a drawn slot and, on a parked one whose trial was frozen,
    /// however long the slot stayed off air. ADR-0313 removed the trial, so a
    /// build and a refusal are both reported on the first boundary after they
    /// arrive. ADR-0316 removed the rollback target itself: the displaced Set
    /// is retired here, on both sides of the verdict.
    ///
    /// **A build landing clears [`HotSwap::overloaded`]**, which is the third
    /// of the three ways out of a stopped slot and the only one the engine
    /// takes by itself. It happens before the verdict, so a candidate that is
    /// itself over budget stops the slot again on its own number rather than
    /// inheriting the last one's.
    fn install_if_ready(&mut self, device: &wgpu::Device) {
        // `try_recv`, never `recv`, and drained to the *newest* result rather
        // than stopping at the first. An `mpsc` channel is FIFO and two saves
        // can finish between two frames, so taking the front of the queue would
        // put a superseded Set on screen for a frame before reaching the one
        // the operator is waiting for.
        // A superseded error is still reported — a diagnostic is the point of
        // an error, superseded or not — and a superseded Set is retired to the
        // worker rather than dropped here.
        let mut newest: Option<Built> = None;
        loop {
            match self.done.try_recv() {
                // **A refusal is reported where it is read and never held as
                // the newest anything.** Nothing was built, so it is not a
                // candidate to be superseded and it does not take the place of
                // one: a save that does not compile followed by a save that
                // does is a refusal and then a swap, in that order, and both
                // are said. The strings are moved rather than copied, so this
                // costs the render thread a `Vec` push.
                Ok(Done::Refused(refusal)) => self.events.push(Event::SourceRefused {
                    label: refusal.label.into(),
                    said: refusal.said,
                }),
                Ok(Done::Built(next)) => {
                    if let Some(stale) = newest.replace(next) {
                        match stale.result {
                            Ok(set) => self.retire(set),
                            Err(error) => self.events.push(Event::Rejected {
                                id: stale.id,
                                label: stale.label,
                                error,
                            }),
                        }
                    }
                }
                // `Empty` and `Disconnected` are the same thing from here: no
                // Set to install, so render and move on. A worker that is gone
                // while one was expected is reported once, below.
                Err(mpsc::TryRecvError::Disconnected) => {
                    if self.worker.is_some() && !self.worker_lost {
                        self.worker_lost = true;
                        self.events.push(Event::WorkerLost);
                    }
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => break,
            }
        }
        let Some(built) = newest else {
            return;
        };

        match built.result {
            Err(error) => self.events.push(Event::Rejected {
                id: built.id,
                label: built.label,
                error,
            }),
            Ok(mut candidate) => {
                candidate.resize(device, self.viewport.0, self.viewport.1);
                // **What the operator's hands are on comes across; what the code
                // declares does not** — see [`Set::carry_moved_from`], which is
                // the whole of the rule and the reason `Set` remembers which of
                // its values were stated.
                //
                // **Here, and not in [`HotSwap::install`].** This is the live
                // path: a build a worker produced while a Set was playing, and
                // the only moment both Sets exist in one hand. `install` is the
                // replay path, where there is no operator and no live write to
                // inherit — a replay builds from the record stream and the
                // `ride` records land at the frames they were made at, so
                // inheriting there would be a second, unrecorded source of the
                // same value
                // (`docs/adr/0280-a-parameter-written-to-a-live-set-is-a-session-record.md`).
                //
                // **On the render thread and allocation-bounded**: it writes
                // into a map that already holds the key and inserts a `String`
                // per key it carries, once per swap rather than once per frame.
                // A swap is already the frame that resizes render targets.
                candidate.carry_moved_from(&self.live);
                // **And what an operator attached, beside what they moved** —
                // [`Set::carry_bound_from`], which is the same rule about a
                // different writer and is here for the same reason: this is
                // the only moment both Sets exist in one hand. A binding this
                // request states is left where the loop above put it; one the
                // outgoing Set holds at an address the request did not name is
                // an attachment made on the live slot, and it carries.
                //
                // **Not in [`HotSwap::install`]**, on the line above's terms
                // exactly: that is the replay path, where a `source` record
                // lands at the frame it was made at and inheriting would be a
                // second, unrecorded source of the same attachment
                // (`docs/adr/0339-a-rebuild-inherits-the-attachments-somebody-made.md`).
                candidate.carry_bound_from(&self.live);
                let outgoing = std::mem::replace(&mut self.live, candidate);
                self.cost = built.cost;
                // The worker measures what it built and cannot estimate it —
                // an estimate is taken against the output's size, which the
                // worker does not know. So the incoming Set arrives with none
                // and the outgoing one's goes with the Set it was taken of;
                // there is no rollback for it to be held against any more.
                self.estimate = None;
                // **Retired here rather than after the verdict** (ADR-0316).
                // Both verdicts leave the candidate live, so neither of them
                // wants this Set back, and holding it across the call would be
                // a rollback target kept for a rollback that cannot happen.
                self.retire(outgoing);
                // **The freeze belongs to the version that was in this slot,
                // and that version has just left.** Cleared before the verdict
                // so that an over-budget candidate sets it again on its own
                // number — see [`HotSwap::overloaded`].
                self.overloaded = false;
                self.events.push(Event::Swapped {
                    id: built.id,
                    label: Arc::clone(&built.label),
                });
                // **The verdict, in the same call.** The number it is reached
                // on was taken on the worker before this Set was handed over,
                // so there is nothing to wait for; a caller draining events
                // sees the swap and then its verdict, in that order.
                self.judge(built.id, built.label);
            }
        }
    }

    /// Queue a Set for the worker to drop. Never drops it here.
    fn retire(&mut self, set: Set) {
        self.retired.push(set);
        self.hand_over_retired();
    }

    /// `try_lock`, never `lock`: the render thread waits on the worker for
    /// nothing. A Set that misses this frame's handover is handed over next
    /// frame instead, and holding one for a few extra frames costs only the
    /// memory it was already using.
    fn hand_over_retired(&mut self) {
        if self.retired.is_empty() {
            return;
        }
        if let Ok(mut graveyard) = self.graveyard.try_lock() {
            graveyard.append(&mut self.retired);
        }
    }
}

impl Drop for HotSwap {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            // Up to one `POLL_INTERVAL`, because the worker is most likely
            // sitting inside `Source::poll`.
            let _ = worker.join();
        }
    }
}

/// How many Sets the handover between the render thread and the worker is
/// sized for. A frame retires at most the superseded builds it drained plus one
/// verdict's Set, and the graveyard is emptied by the worker every poll; four is
/// slack, and the capacity exists so that the render thread's `push` and
/// `append` never have to grow anything.
const GRAVEYARD_CAPACITY: usize = 4;

/// Likewise for events. A frame can now emit a swap *and* its verdict, which
/// are no longer mutually exclusive (ADR-0313), plus a refusal that arrived
/// beside them.
const EVENT_CAPACITY: usize = 4;

/// Measure one frame of `set` and leave it exactly as it was found.
///
/// **Never on the render thread.** It submits and waits, once per sample, which
/// is what makes the number mean anything and is the pattern the frame loop must
/// never use; and it steps the Set, so it has to rewind it afterwards, which is
/// another whole-capacity upload. The worker calls this as part of building; a
/// caller with a [`HotSwap::fixed`] Set reaches it through
/// [`HotSwap::measure_live`], or through
/// [`Deck::measure_slots`](crate::deck::Deck::measure_slots) for a whole deck
/// at once, before the first frame.
///
/// `probe` is taken rather than constructed because constructing one runs a
/// calibration workload ten times over (see [`Probe::new`]), and because two
/// independently constructed probes can land on different
/// [`MeasurementMethod`](crate::probe::MeasurementMethod)s and produce numbers
/// that are not comparable — which is precisely what a governor summing them
/// would then be doing.
///
/// `at` is the offscreen size the run is taken at, and it is the caller's because
/// nothing here can know it: this application has one output size and one
/// preview size and no third one, and which of those a measurement is about is
/// a fact about the number rather than about the probe (ADR-0303). It comes
/// back on [`Measurement::resolution`].
///
/// The Set is resized to `at` for the run and **resized back**
/// before this returns. That is not decoration: the viewport is what the
/// camera's aspect ratio is derived from in [`Set::prepare`], it is the one
/// piece of a Set's state a probe run touches that [`Set::rewind`] has no
/// business resetting — `build` leaves it at 1×1 and a caller's size is not a
/// thing to rewind to — and a Set left at 720p renders a different picture on
/// its first frame in a deck of any other shape. `HotSwap::install_if_ready`
/// happens to resize an arriving candidate anyway, which is what kept this
/// invisible; a caller measuring its own [`HotSwap::fixed`] Set has no such
/// second chance. (*"a Set left at 720p"* is the shape of the failure rather
/// than the number now; it is left at whatever `at` was.)
/// `(width, height)` in one `u64`, so the render thread can hand the worker a
/// size with a single store and the worker can read it with a single load.
/// Two `AtomicU32`s would let a worker read a width from one frame and a
/// height from the next, which is a size nothing ever drew.
fn packed((width, height): (u32, u32)) -> u64 {
    (u64::from(width) << 32) | u64::from(height)
}

/// The other half of [`packed`].
fn unpacked(at: u64) -> (u32, u32) {
    ((at >> 32) as u32, at as u32)
}

pub fn measure(
    probe: &mut Probe,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    set: &mut Set,
    at: (u32, u32),
) -> Measurement {
    let capacity = set.capacity();
    let viewport = set.viewport();
    // **The probe follows the size rather than the size following the
    // probe.** `Probe::resize` keeps the calibration verdict and replaces only
    // the attachment, which is the whole reason a small draw is affordable —
    // and a second `Probe` here could land on a different clock and make two
    // slots' numbers incomparable.
    probe.resize(device, at);
    set.resize(device, at.0, at.1);
    // The uniforms have never been written otherwise — `build` allocates them
    // and leaves them at whatever the driver's fresh buffer holds — so the
    // measured frame has to be preceded by a real `prepare`, exactly as an
    // on-air frame is. The session's signals are `default` here: a probe run is
    // not part of a session and has no tick sequence of its own, and a binding
    // resolved against a phase of zero is the same shape of work as one
    // resolved against any other phase.
    set.prepare(queue, PROBE_STEPS, &Signals::default());
    let measurement = probe.run(device, queue, set, PROBE_STEPS, capacity);
    // Back to cold. `Set::rewind` is documented for this one caller.
    set.rewind(device, queue);
    set.resize(device, viewport.0, viewport.1);
    measurement
}

/// The build worker.
///
/// A poll loop rather than a blocking one, because it has two jobs: asking the
/// source for work, and freeing whatever the render thread retired. A `recv`
/// that blocked until the next request would leave a rolled-back Set's buffers
/// resident until the operator happened to edit a file again.
fn run_worker(
    device: wgpu::Device,
    queue: wgpu::Queue,
    mut source: Box<dyn Source>,
    out: Sender<Done>,
    graveyard: Arc<Mutex<Vec<Set>>>,
    stop: Arc<AtomicBool>,
    // **Read per build and never cached.** The render thread narrows it to a
    // preview cell and a drag moves that cell, so a size read once at spawn
    // would measure every candidate of a session at whatever the window
    // happened to be when it opened.
    measure_at: Arc<AtomicU64>,
) {
    // Constructed once, on first use, and reused for every build this worker
    // ever does. Once because calibration is expensive (ten heavy runs) and
    // once because two probes can disagree with each other about whether this
    // adapter's timestamps work — see `Probe::new`. Lazily because a worker
    // that is never given anything to build should not allocate an offscreen
    // target and half a second of calibration for nothing.
    let mut probe: Option<Probe> = None;

    while !stop.load(Ordering::Relaxed) {
        // Take the retired Sets out from under the lock before dropping them:
        // releasing a Set's GPU resources is not instant, and the render
        // thread's `try_lock` would fail for the whole of it. `drain` leaves
        // the graveyard's capacity intact, which is what keeps the render
        // thread's side allocation-free.
        let condemned: Vec<Set> = match graveyard.lock() {
            Ok(mut held) => held.drain(..).collect(),
            // Poisoned: the render thread panicked holding it. There is
            // nothing to recover here and the process is going down anyway.
            Err(_) => Vec::new(),
        };
        // `held` is gone by here, so the lock is free while the expensive part
        // — actually releasing the GPU resources — happens.
        drop(condemned);

        let request = match source.poll() {
            Some(Polled::Build(request)) => request,
            // **Straight back, with nothing built and nothing measured.** The
            // sentences were formatted by the source on this thread, beside
            // the stages that produced them, so what the render thread does
            // with this is move two allocations into an `Event`.
            Some(Polled::Refused(refusal)) => {
                if out.send(Done::Refused(refusal)).is_err() {
                    // The render thread is gone.
                    break;
                }
                continue;
            }
            None => continue,
        };
        let id = request.id;
        let label: Arc<str> = request.label.into();

        // Caught, not allowed to propagate. A panic here would take the whole
        // worker with it, and the render thread's only symptom would be that
        // nothing is ever built again — no error, no event, just a `--watch`
        // that quietly stops responding to saves. That is the worst shape a
        // failure can take on stage. `Set::build` creates shader modules, and
        // wgpu's default handler for an uncaptured validation error is a
        // panic, so this is reachable from any generated WGSL naga refuses:
        // a compiler bug, but one that must not be an unexplained silence.
        let build = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Set::build_many(
                &device,
                &queue,
                &request.l1s.iter().map(|(p, c)| (p, *c)).collect::<Vec<_>>(),
                &request.l2s.iter().collect::<Vec<_>>(),
                &request.l3s.iter().collect::<Vec<_>>(),
                &request.fields.iter().collect::<Vec<_>>(),
                &request.l4s.iter().collect::<Vec<_>>(),
                request.layering,
                request.seed_salt,
                &request.salts,
                // **Restated by the request**, on the same terms its bindings
                // and its interface are: a rebuild that took the names off the
                // outgoing Set would depend on what happened to be live, and a
                // request has to be reproducible from a record stream.
                crate::set::Wiring {
                    l1s: &request.names.l1s,
                    l2s: &request.names.l2s,
                    l3s: &request.names.l3s,
                    l4s: &request.names.l4s,
                    fields: &request.names.fields,
                    edges: &request.edges,
                },
            )
            .map(|mut set| {
                // **The camera the request states, into the node it is about**,
                // at the point the startup path assigns a `camera` record — see
                // `Request::camera`.
                //
                // **Before the params, and the order is a contest now.** It was
                // not while the built-in orbit declared no parameters: nothing
                // in `params` could address it, and the two paths agreeing
                // about where this landed was the whole of what the order
                // bought. Since ADR-0318 the orbit's `radius`, `speed` and
                // `height` are that node's parameters, so a `--param
                // L3:0:radius` in this request is a statement about the same
                // number and has to land after the camera it overrides — which
                // is where it already was.
                set.aim_camera(request.camera);
                // **The fold the request states, beside the camera it states**
                // and for the same reason — see [`Request::live`]. A renderer
                // this build no longer has is said and passed over: the files
                // were just recompiled and may name fewer renderers than the
                // Set that was loaded, which is a rebuild rather than an error.
                if let Some(at) = request.live {
                    if !set.select_renderer(at as usize) {
                        eprintln!(
                            "  this build has no renderer {at} to fold to — every renderer is live"
                        );
                    }
                }
                for write in &request.params {
                    // Addressed or not — `Set::write_param` is the one place
                    // that decides, so this path and the command line's cannot
                    // disagree about what a bare name means. That includes the
                    // refusal it carries: a bare name over nodes that are not
                    // under one authority is refused here in the words a
                    // `--param` is refused in
                    // (`crate::set::CrossesAuthority`).
                    //
                    // **A restatement cannot meet it, because the authorities
                    // this request states are applied below.** Every node of a
                    // freshly built Set is at `Authority::default`, so the
                    // landing is uniform whatever the request goes on to grant,
                    // and a build applies the values it was given rather than
                    // re-asking permission for them. That is deliberate: what is
                    // stated here is what a caller said this Set's values are —
                    // the file a slot was pointed at, or the flags it was
                    // started with — which is a fact rather than a new write.
                    //
                    // **Where the operator left the controls is not here**, and
                    // has not been since it stopped needing to be: it travels on
                    // the outgoing Set and lands at the install — see
                    // `Set::carry_moved_from` and `Request::params`. A key this
                    // loop writes is marked as stated on the new Set, which is
                    // exactly what tells that carry to leave it alone.
                    match set.write_param(write) {
                        Ok(0) => eprintln!("  no parameter named `{}`, ignoring", write.key),
                        Ok(_) => {}
                        Err(refused) => eprintln!("  {refused}"),
                    }
                }
                // **Before the bindings**, because a macro is a binding whose
                // source is a published control: a binding attached before the
                // control existed would be attached to a name nothing answers,
                // and would hold its param where it found it for the rest of
                // the run.
                for control in request.published {
                    let name = control.name.clone();
                    if let Err(e) = set.publish(control) {
                        eprintln!("  `{name}` is not published: {e}");
                    }
                }
                // After the params, because a binding blends from a param's
                // value: applying them the other way round would leave the
                // first frame after a swap blending from the `.kir` default.
                for binding in request.bindings {
                    let (layer, key) = (binding.layer, binding.key.clone());
                    let signal = binding.signal.clone();
                    match set.bind(binding) {
                        crate::set::Bound::Yes => {}
                        crate::set::Bound::NoSuchParam => {
                            eprintln!("  no {layer:?} parameter named `{key}` to bind, ignoring")
                        }
                        crate::set::Bound::NoSuchControl => eprintln!(
                            "  `{signal}` is not published by this Set, so `{layer:?} {key}` \
                             is not bound"
                        ),
                    }
                }
                // **Who may move each node, as the request states it** — see
                // `Request::authorities`. Order-free, unlike everything above
                // it: an authority is a permission granted forward and is not
                // an input to a param, a control or a binding, so nothing here
                // contends with any of them. It is last because the loop that
                // has nothing to say about ordering should not sit between two
                // that do.
                //
                // A node this build no longer has is said and passed over, on
                // the fold's terms: a recompile may name fewer nodes than the
                // Set the authority was recorded against.
                for stated in &request.authorities {
                    let (layer, index) = stated.at;
                    if !set.set_authority(layer, index, stated.authority) {
                        eprintln!(
                            "  this build has no {layer:?} node {index} to give authority to, \
                             ignoring"
                        );
                    }
                }
                set
            })
        }));
        let result = match build {
            Ok(result) => result,
            Err(payload) => Err(SetError::Panicked {
                label: label.to_string(),
                detail: panic_detail(&payload),
            }),
        };

        let mut result = result;
        let mut cost = None;
        if let Ok(set) = &mut result {
            // `Set::build` leaves the whole element and alive buffer contents
            // staged on the queue — megabytes at a real capacity. Left there,
            // they would be flushed by whatever the *render thread* submits
            // next, putting the upload inside the first frame the swap was
            // supposed to be invisible to. Flushing it here gives it its own
            // submission, and waiting for it here means the Set is resident
            // before it is handed over. This is the submit-and-wait the render
            // thread must never do; a worker thread is exactly where it
            // belongs.
            queue.submit([]);
            let _ = device.poll(wgpu::PollType::wait_indefinitely());

            // And then measure it, on the same thread and for the same reason.
            // Caught for the same reason the build is: this reaches driver code
            // through a freshly generated pipeline, and a panic here would take
            // the worker with it and leave a `--watch` session silently
            // unable to build anything again. A build that could not be
            // measured is still a build — it travels without a measurement and
            // the governor declines to budget for it, which is the conservative
            // reading rather than a failure.
            let at = unpacked(measure_at.load(Ordering::Relaxed));
            let probe = probe.get_or_insert_with(|| {
                Probe::new(
                    &device,
                    &queue,
                    device.features().contains(wgpu::Features::TIMESTAMP_QUERY),
                    at,
                )
            });
            cost = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                measure(probe, &device, &queue, set, at)
            }))
            .ok();
            if cost.is_none() {
                eprintln!("  `{label}` could not be measured; it will not be budgeted for");
            }

            // And flush again, for the reason the first flush exists. `measure`
            // ends in `Set::rewind`, which re-stages the *whole* element and
            // alive buffers — the same megabytes `build` staged, put back a
            // second time — and nothing in `measure` submits after it. Left
            // here they would ride out on the render thread's next submission,
            // which is precisely the cost the flush above was added to keep out
            // of the swap frame. The flush before the measurement does not
            // cover an upload the measurement itself creates.
            queue.submit([]);
            let _ = device.poll(wgpu::PollType::wait_indefinitely());
        }

        if out
            .send(Done::Built(Built {
                id,
                label,
                result,
                cost,
            }))
            .is_err()
        {
            // The render thread is gone.
            break;
        }
    }
}

/// The message out of a caught panic, which arrives as a `Box<dyn Any>` and is
/// a `&str` or a `String` for every panic the standard macros produce.
fn panic_detail(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "no message".to_string()
    }
}

/// `Set` must stay `Send`, because the whole design above rests on building
/// one on a worker thread and moving it to the render thread.
///
/// It is today: every field is a wgpu handle (all `Send + Sync` in wgpu 30),
/// a `Vec`, a `HashMap`, or a plain number. Nothing is a `Rc`, a `Cell`, or a
/// raw pointer. This assertion is here so that a field which *is* one of those
/// fails to compile at the line that explains why it matters, rather than
/// somewhere inside `HotSwap::new` where the fix looks like "wrap it in a
/// mutex" instead of "do not put that in a Set".
const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<Set>();
    assert_send::<Request>();
};
