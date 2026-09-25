//! Strip state, animation phase, fader and meter readings for the Mixer bay.

use std::time::Duration;

use egui::Rect;

use super::super::*;

/// `.trim .lbl`: the `g`, which is the only word in this bay that is neither
/// the deck's nor the engine's — it is the mock's.
pub(crate) const TRIM_LABEL: &str = "g";

/// How long the panel has been animating, and the one value everything that
/// moves on it derives its own rate from.
///
/// # One phase, panel-wide, because two clocks drift and one does not
///
/// [ADR-0190](../../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md)
/// decided exactly this: *"two controls moving out of step looks broken rather
/// than informative, and N animations are N deadlines where one phase is one."*
/// Two strips parked at once are two rolls, and they are the same roll because
/// they are read off the same number.
///
/// # It is elapsed time and not a wrapped fraction
///
/// A phase already reduced to *where we are in the cycle* fixes the cycle, and
/// then a second presentation at another rate cannot be derived from it at all
/// — it would need a phase of its own, which is the drift this exists to
/// prevent. So the value is the whole elapsed interval and each presentation
/// takes its own [`Phase::cycle`] out of it. The console has one moving thing
/// today; it is about to have the beat and the rest of the mixer, and *this is
/// where the second one is either free or a second clock*.
///
/// # It is a `Duration` and not an `Instant`, which is the whole seam
///
/// `src/` reads no clock. [`Transport`]'s own doc says why in the general case
/// — *"an `Instant` here would put a clock in it, and then the row would be
/// reading wall time in a repository whose first principle is that nothing
/// does"*
/// ([P-0092](../../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md))
/// — and every `Instant::now` in this crate is in
/// `crates/karakuri/src/main.rs`, which owns the window. An `Instant` is a
/// *reading*; a `Duration` is a number, and a number is what a caller writes
/// and a test chooses. So this arrives per frame the way [`Transport`] and
/// [`View::mixer`] arrive
/// ([ADR-0156](../../../../docs/adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)),
/// and a test asserting an animation writes the phase it wants rather than
/// catching one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Phase(Duration);

impl Phase {
    /// The origin, and what a console with no clock behind it is at — every test in
    /// this crate, and the first frame of a window.
    pub const ZERO: Phase = Phase(Duration::ZERO);

    /// A phase from the interval that has elapsed since whoever owns the clock
    /// started counting. Where the origin is does not matter and is not asked:
    /// every presentation is periodic in it.
    pub const fn since(elapsed: Duration) -> Phase {
        Phase(elapsed)
    }

    /// Where this phase sits inside one `period`, as a fraction in `[0, 1)`.
    ///
    /// The one operation a presentation performs on a phase, and the reason the
    /// carrier is an interval: a rate is chosen here, by the thing that moves,
    /// rather than baked into the value the harness writes.
    ///
    /// In `f64` and returned as `f32`. A session runs for hours and a phase is
    /// seconds from an origin — at 3600 s an `f32` step is already coarser than a
    /// millisecond, and the fractional part is what survives the division, so the
    /// division is the one place the extra bits are worth having.
    pub fn cycle(self, period: Duration) -> f32 {
        let period = period.as_secs_f64();
        match period > 0.0 {
            true => (self.0.as_secs_f64() / period).fract() as f32,
            false => 0.0,
        }
    }
}

/// How often anything pending on this panel sets off toward where it is going,
/// and the period every other number in the presentation is a fraction of.
///
/// One second, which is the roll's *"about once a second"* and is the rate that
/// reads as *waiting* rather than as a fault. See
/// [ADR-0190](../../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md).
///
/// Two users, one rate, and that is the rule rather than a coincidence: the
/// tally's word rolling toward a residency, and a fader's fill reaching toward
/// a scheduled value ([`Reach`],
/// [ADR-0206](../../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)).
/// ADR-0190 asks for one phase panel-wide because two controls moving out of
/// step looks broken rather than informative, and a second period here is how
/// that would happen with nothing failing to compile.
pub const ROLL_PERIOD: Duration = Duration::from_millis(1000);

/// How much of the period the word is moving for: 400 ms out and back, and 600
/// ms at rest.
///
/// The rest is not slack. It is what makes the roll read as *an attempt that
/// keeps being made* rather than as a chip that wobbles — and it is what makes
/// most of a parked strip's frames the strip's settled appearance, so the
/// operator reads the effective residency off a still chip nearly two thirds of
/// the time — the first of the three things P-0087 asks a pending control to
/// say, which is where it *is*.
pub const ROLL_TRAVEL: Duration = Duration::from_millis(400);

/// How far toward the destination the moving thing gets: a fraction of the
/// pitch between the two words on the tally, and a fraction of the gap between
/// the two values on a fader.
///
/// Less than one, and that is the presentation: *"a slot machine that never
/// quite lands"*. Landing is what arrival looks like, so a roll that reached
/// 1.0 would state the opposite of the truth once a second — and a band that
/// covered the gap to a fader's mark would say a scheduled fade had already
/// run. At 0.4 the current word keeps most of its rows and the destination
/// shows enough of its own to be read.
pub const ROLL_REACH: f32 = 0.4;

/// How many steps the travel is drawn in, which is the only reason
/// [`ROLL_STALENESS`] is a number at all.
pub(crate) const ROLL_STEPS: u32 = 12;

/// How stale the roll may get, which is what a presentation declares under
/// [P-0091](../../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)
/// and what the harness turns into a deadline.
///
/// [`ROLL_TRAVEL`] in [`ROLL_STEPS`] steps — 33.33 ms, about thirty a second.
/// It is one number for the whole period rather than a fine one while the word
/// moves and a coarse one while it rests: two numbers is two live regions
/// wearing one name, and choosing between them frame by frame is a scheduler's
/// job (ADR-0164's second half) rather than a presentation's. A presentation
/// declares what it needs; what the panel can afford is decided somewhere else.
pub const ROLL_STALENESS: Duration =
    Duration::from_micros(ROLL_TRAVEL.as_millis() as u64 * 1000 / ROLL_STEPS as u64);

/// How far a pending presentation has progressed toward its destination this frame,
/// as a fraction of distance: `0.0` at rest, [`ROLL_REACH`] at peak, never `1.0`.
///
/// Implemented as a raised cosine over [`ROLL_TRAVEL`], remaining zero for the
/// rest of [`ROLL_PERIOD`] to ensure zero velocity at boundaries.
pub fn roll_at(phase: Phase) -> f32 {
    let travel = ROLL_TRAVEL.as_secs_f32() / ROLL_PERIOD.as_secs_f32();
    let t = phase.cycle(ROLL_PERIOD);
    match t < travel {
        true => ROLL_REACH * 0.5 * (1.0 - (t / travel * std::f32::consts::TAU).cos()),
        false => 0.0,
    }
}

/// Duration until the next presentation movement from `phase` (ADR-0283).
///
/// Returns [`ROLL_STALENESS`] during travel, or the remaining duration until the
/// next travel window begins, clamped to at least [`ROLL_STALENESS`].
pub fn roll_moves_in(phase: Phase) -> Duration {
    let travel = ROLL_TRAVEL.as_secs_f32() / ROLL_PERIOD.as_secs_f32();
    let t = phase.cycle(ROLL_PERIOD);
    match t < travel {
        true => ROLL_STALENESS,
        // `cycle` is `[0, 1)`, so this is positive and no longer than the
        // whole period.
        false => ROLL_PERIOD.mul_f32(1.0 - t).max(ROLL_STALENESS),
    }
}

/// What shape of the frame a layer reaches: the second `.mini` in a strip, and
/// `karakuri_engine`'s `MaskKind` — three kinds and no fourth.
///
/// # It is a mark rather than a word, and the mark is drawn rather than typed
///
/// The mock writes `&#9711;` and `&#9681;` — a circle, and a circle with one
/// half filled — and it has to: a strip is 53 wide inside its padding, and
/// `over` beside `linear` is 75 before either mini's border. So the mask says
/// its state in a shape.
///
/// Drawn rather than set as a glyph, which is [`grip_dots`]'s argument one
/// control along: whether `◯` and `◑` are in `egui`'s default face is a
/// question with no good answer, and a circle is the same mark either way. It
/// also settles the third one. The mock names `◯` in its own tooltip — *"Mask:
/// none"* — and draws `◑` on the strip beside it without saying which kind that
/// is, and it draws no third mark anywhere; a glyph for the third would be a
/// character picked out of a font, where a mark is the shape the mask makes and
/// can be argued from the mask.
///
/// # There is no `ALL` beside it, where [`Tally`] and `BlendMode` have one
///
/// Both of those constants exist for a reader: `Tally::ALL` is what [`mixer`]
/// measures the tally capsule against, because a `match` cannot express *the
/// widest of them*, and `BlendMode::ALL` is what a map file is offered. Nothing
/// measures this chip against its three shapes — it holds a mark rather than a
/// word and is [`size::MINI_SIZE`] wide whichever shape it is showing — and no
/// map target names a shape, which is why `karakuri_operation::WipeKind` has no
/// `ALL` either and says so at [`WipeKind::name`]. A list written here would be
/// a second statement of the order with no reader, and the order is
/// [`next_shape`](super::paint::next_shape)'s: a `match`, so a fourth shape does not compile until
/// somebody says what follows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mask {
    /// No mask: the layer reaches the whole frame. The mock's `◯`.
    None,
    /// A straight edge across the frame — the mock's `◑`, which is what a hard edge
    /// down the middle of a circle looks like.
    Linear,
    /// A circle. The same outline with a filled centre, which is the same family as
    /// the mock's two and is the shape a radial mask makes.
    Radial,
}

/// What a slot's meter last read, which is two of the four numbers
/// `karakuri_engine::meter::Level` carries.
///
/// Plain numbers, for [`Transport`]'s reason: this crate takes no engine
/// (ADR-0156), so whoever owns the deck reads a level and writes this.
///
/// # What is left out, and why each
///
/// - `frames_behind`. `Deck::level` reads back without ever waiting, so a
///   reading is a few frames old and the engine says by how many. It is not
///   carried, and not because it does not matter: the engine's own
///   measurements are 1 frame behind under pacing, *"2 or 3"* on a vsync
///   window, and *"tens: 13 to 101"* with nothing pacing the loop at all — and
///   all three are normal. A staleness threshold picked in this crate would
///   blank a healthy meter on one loop and pass a dead one on another, because
///   the console does not know which loop it is on. What the meter draws is a
///   fact about a frame that has already been drawn, exactly as
///   [`Transport::frame_ms`] is, and it does not go stale by standing still
///   the way a *rate* does — which is the whole of why `frame_ms` is a number
///   and `fps` is an `Option`. So handing over no reading at all is the
///   caller's decision, on the same terms `fps: None` is, and the caller is
///   the one thing that knows its own loop.
/// - `bad_texels`. It is the denominator the mean was taken over rather
///   than a reading — *"`mean` and `peak` are computed over the texels this
///   did not count"* — and the mock's 6px meter has nowhere to say it. A meter
///   that drew it would be reporting on the measurement instead of on the
///   image.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Level {
    /// Mean luminance over the whole frame — *"the figure to match faders on"*, and
    /// what the meter's column is as tall as.
    pub mean: f32,
    /// The brightest single texel. What the peak mark sits on.
    pub peak: f32,
}

/// What one mixer strip reads this frame: what the deck is playing, where the
/// slot sits, and the four numbers in front of it.
///
/// # The same seam as [`Transport`], and it is not crossed
///
/// Every field is a string, a number or a word, and none of them is a `Deck`.
/// `src/` takes no device and no engine (ADR-0156), so whoever owns the deck
/// reads `Deck::residency`, `gain`, `opacity`, `blend`, `mask` and `level` and
/// writes one of these per slot per frame — exactly as whoever owns the device
/// registers a texture and writes [`View::picture`].
///
/// No repaint arm for the values arriving, which is [`Transport`]'s reason and
/// not the picture's: these move when the engine draws a frame, and the engine
/// draws a frame on the frames this panel is drawn on — so a value that has
/// arrived is a value already on screen, whether or not there is a picture two
/// bays along asking for frames of its own. The one that has to be said out
/// loud is [`Strip::level`], because it moves on every one of those frames and
/// declares nothing: see [`View::mixer_declares`], where that is decided, and
/// [ADR-0290](../../../../docs/adr/0290-the-level-meter-moves-only-when-a-frame-is-drawn-so-it-declares-nothing.md).
///
/// A drag on one of the two faders is a different thing and does have an arm —
/// [`crate::repaint::Change::Emitted`]. That is not the value arriving; it is
/// an operation leaving, on a gesture an operator is making, and it is on the
/// list because [`crate::repaint::Change`] is one list of everything that can
/// change what the console shows and a fader that reached no repaint would
/// leave the strip drawn at the value before the drag.
///
/// # Five of these are controls and the rest are readouts, and the source says
/// which
///
/// The two faders are played. A press on the trim's knob or the fader's knob
/// takes it in hand ([`Mixer::grab`]), and dragging it emits
/// [`Operation::SetGain`] or `SetOpacity` for the caller to turn into a record.
/// Nothing here applies it, and nothing here keeps the value: the fields below
/// are still written by whoever owns the deck, every frame, so a fader that has
/// just been dragged shows the new number because the deck changed.
///
/// The blend chip is the third, and it cycles. A press on it emits
/// [`Operation::SetBlendMode`] naming the mode after this one
/// ([`Mixer::blend`]), and nothing here applies that either. It is one control
/// emitting three operations, which is the affordance P-0090 leaves to whoever
/// draws the control — see
/// [ADR-0187](../../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md).
///
/// The tally chip is the fourth, and it cycles too. A press on it emits
/// [`Operation::SetResidency`] naming the next of the three ([`Mixer::tally`])
/// — counted from [`Strip::requested`] rather than from [`Strip::tally`], which
/// is what makes a press on a parked chip the withdrawal of its own prime
/// request without a case in the code for it
/// ([ADR-0195](../../../../docs/adr/0195-the-tally-chip-cycles-from-the-request-so-the-parked-case-needs-no-case.md)).
///
/// The mask mini is the fifth, and it cycles too. A press on it emits
/// [`Operation::SetMaskShape`] naming the shape after this one
/// ([`Mixer::mask`]) — and the angle this strip is already wearing, which is
/// [`Strip::mask_angle`]: the chip chooses a shape and the angle is not its
/// business, so it hands back the one it was given rather than a default that
/// would straighten a diagonal front
/// ([ADR-0203](../../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)).
///
/// Everything else in the bay is a readout. The meter and the number are drawn
/// from what the deck says and a press on either reaches nothing — and so does
/// a press on a fader's *track*, off the knob, which would otherwise be a jump
/// nobody asked for. `tests/mixer.rs` asserts both directions rather than
/// leaving either to be inferred from the absence of a hit test.
///
/// [`Strip::gain_to`] and [`Strip::opacity_to`] are readouts too, and the order
/// is the tally's: the roll was drawn a commit before the chip became a
/// control, because a control that could not yet say it was pending would look
/// dead for exactly as long as that commit. Nothing here schedules a move,
/// nothing here cancels one, and no press on a mark reaches anything — a
/// scheduled fade is armed from a key, a map or an MCP call, and what this
/// strip owes it is that it is not invisible until it happens
/// ([ADR-0206](../../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)).
#[derive(Debug, Clone, PartialEq)]
pub struct Strip {
    /// What the deck is playing, in `.strip-name`.
    pub name: String,
    /// Where the slot sits — `Deck::residency`, which is the effective residency
    /// and not `requested_residency`.
    pub tally: Tally,
    /// What was asked for — `Deck::requested_residency`, the other half of the pair
    /// [`Strip::tally`] is one of, and what a press on the chip counts from
    /// ([`Mixer::tally`]).
    pub requested: Tally,
    /// The trim — `Deck::gain`: linear, floored at zero, and deliberately open
    /// above 1.0 because the mix is HDR.
    pub gain: f32,
    /// Where a scheduled move is taking the trim, or `None` for a trim nothing is
    /// moving — `Deck::transitions_on(slot)`'s `Control::Gain` entry, and its
    /// `Transition::to`.
    pub gain_to: Option<f32>,
    /// The fader — `Deck::opacity`: a proportion in `[0, 1]`, and the one control
    /// that silences a slot under every blend mode, which is what makes it the way
    /// out of material that has gone NaN.
    pub opacity: f32,
    /// Where a scheduled move is taking the fader, or `None` for a fader nothing is
    /// moving — `Deck::transitions_on(slot)`'s `Control::Opacity` entry, on
    /// [`Strip::gain_to`]'s terms and for its reasons.
    pub opacity_to: Option<f32>,
    /// The blend in force, as one of the vocabulary's three — and the word drawn on
    /// the chip is [`BlendMode::name`].
    pub blend: BlendMode,
    /// The mask in force (`Deck::mask(slot).kind()`).
    pub mask: Mask,
    /// The angle the mask is already wearing — `Deck::mask(slot).angle()`, in
    /// radians. Read to build an operation, and drawn nowhere.
    pub mask_angle: f32,
    /// What the slot's meter last read, or `None` for no reading at all.
    pub level: Option<Level>,
    /// Whether this channel is currently muted.
    pub is_muted: bool,
    /// Whether this channel is currently soloed.
    pub is_soloed: bool,
}

impl Strip {
    /// Where this slot has been asked to go and has not reached, or `None` if settled.
    ///
    /// Returns `Some(requested)` when `requested != tally` (P-0087, ADR-0188).
    pub fn pending(&self) -> Option<Tally> {
        match self.requested == self.tally {
            true => None,
            false => Some(self.requested),
        }
    }

    /// Where the trim is going and has not got to, or `None` for a trim that is
    /// where it has been asked to be.
    pub fn gain_pending(&self) -> Option<f32> {
        self.gain_to
            .filter(|to| unit(*to) != unit(self.gain))
            .map(unit)
    }

    /// Where the fader is going and has not got to, on [`Strip::gain_pending`]'s
    /// terms.
    pub fn opacity_pending(&self) -> Option<f32> {
        self.opacity_to
            .filter(|to| unit(*to) != unit(self.opacity))
            .map(unit)
    }
}

impl Default for Strip {
    fn default() -> Self {
        Strip {
            name: String::new(),
            tally: Tally::Allocated,
            requested: Tally::Allocated,
            gain: 0.0,
            gain_to: None,
            opacity: 0.0,
            opacity_to: None,
            blend: BlendMode::Add,
            mask: Mask::None,
            mask_angle: 0.0,
            level: None,
            is_muted: false,
            is_soloed: false,
        }
    }
}

/// A fader, laid out: the track, the length of it the value fills, and the knob
/// sitting on the value.
///
/// One type and one derivation for both of a strip's faders — see [`fader`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fader {
    /// The well: `.fader`'s 5px capsule lying down, or `.vfader`'s 17px one
    /// standing up.
    pub track: Rect,
    /// Which way it runs. [`Axis::Row`] fills from the left and [`Axis::Column`]
    /// fills from the bottom, because a fader stands up.
    pub axis: Axis,
    /// What the value fills, from the track's own zero.
    pub fill: Rect,
    /// The knob, centred on the fill's moving edge.
    pub knob: Rect,
    /// How far the knob's centre moves between the two ends: the track less the
    /// inset the fill sits inside it by, along the axis.
    pub travel: f32,
}

/// A meter, laid out: the well, the column the mean fills, and the peak's mark.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Meter {
    /// `.vmeter`'s 6px capsule.
    pub well: Rect,
    /// [`Level::mean`], up from the bottom.
    pub fill: Rect,
    /// [`Level::peak`], a [`size::VMETER_PEAK_H`] bar across the well.
    pub peak: Rect,
}

/// A scheduled move on a fader, laid out: the mark on the destination, and how
/// far this frame's attempt to get there has reached.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reach {
    /// The destination, as a hairline across the track at the position the knob
    /// would sit at if the move had landed.
    pub mark: Rect,
    /// What the reach has covered at this displacement: from the value's own edge
    /// toward [`Reach::mark`], and never as far as it — [`ROLL_REACH`] of the way
    /// at the top of the travel, nothing at rest.
    pub band: Rect,
}
