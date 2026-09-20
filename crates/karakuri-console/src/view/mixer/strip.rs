use std::time::Duration;

use super::super::*;
use super::*;

/// `.trim .lbl`: the `g`, which is the only word in this bay that is neither
/// the deck's nor the engine's — it is the mock's.
pub(super) const TRIM_LABEL: &str = "g";

/// How long the panel has been animating, and the one value everything that
/// moves on it derives its own rate from.
///
/// # One phase, panel-wide, because two clocks drift and one does not
///
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
pub(super) const ROLL_STEPS: u32 = 12;

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

/// How far a pending presentation has got toward its destination this frame, as
/// a fraction of the distance to it: `0.0` at rest, [`ROLL_REACH`] at the top
/// of the travel, and never `1.0`.
///
/// One curve, two readers — the pitch between the tally's two words
/// ([`tally_into`]) and the gap between a fader's value and its mark
/// ([`reach`]).
///
/// A raised cosine over [`ROLL_TRAVEL`], flat for the rest of [`ROLL_PERIOD`].
/// It is that curve rather than a triangle for one reason worth having: it
/// leaves and arrives at zero *with zero velocity*, so the word does not snap
/// into the rest it holds for the next 600 ms, and the frame the travel begins
/// on is not a jump.
///
/// A pure function of the phase, which is the point of the phase being a value.
/// A test asserts it at a phase it chose; nothing samples a clock to find out
/// what the panel is doing.
pub fn roll_at(phase: Phase) -> f32 {
    let travel = ROLL_TRAVEL.as_secs_f32() / ROLL_PERIOD.as_secs_f32();
    let t = phase.cycle(ROLL_PERIOD);
    match t < travel {
        true => ROLL_REACH * 0.5 * (1.0 - (t / travel * std::f32::consts::TAU).cos()),
        false => 0.0,
    }
}

/// How long until anything rolling on this panel next moves, from `phase` —
/// [`ROLL_STALENESS`] while the travel is under way, and the rest of the rest
/// while it is not.
///
/// # What it is for
///
/// [`roll_at`] is exactly `0.0` for the 600 ms of every [`ROLL_PERIOD`] that is
/// not [`ROLL_TRAVEL`], so a chip drawn at the start of the rest and a chip
/// drawn 33 ms later are the same picture, pixel for pixel. Servicing
/// [`ROLL_STALENESS`] through that stretch buys seventeen frames a second of
/// the panel being redrawn exactly as it already is — 31 asked for in a period
/// where 14 draw something, counted in `tests/moving.rs`. This is what the
/// declaration answers instead, and
/// [ADR-0283](../../../../docs/adr/0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)
/// is the argument.
///
/// It is not a second rate. ADR-0190 rejected *a fine deadline while the word
/// moves and a coarse one while it rests* on the grounds that two numbers is
/// two live regions wearing one name; the rest here is not a second tolerance
/// somebody chose but the same two constants read for when the curve leaves
/// zero, so there is nothing that could drift from the rate and nothing to
/// arbitrate between. [`ROLL_STALENESS`] is still the only staleness this
/// presentation declares, and it is still what the arithmetic sums.
///
/// # It never answers finer than the rate it declared
///
/// A rest with less than one step left of it answers one step, which is
/// [`crate::budget::Declared::moves_in`]'s invariant and costs at most the
/// first [`ROLL_STALENESS`] of the travel — one step out of twelve, and inside
/// the tolerance the presentation itself named. The alternative is a deadline
/// tending to zero as the period wraps, which is the spin [`crate::repaint`]
/// exists to refuse.
///
/// A pure function of the phase, for [`roll_at`]'s reason: a test chooses the
/// phase it asserts at, and nothing here reads a clock.
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
/// [`next_shape`]'s: a `match`, so a fourth shape does not compile until
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
    ///
    /// A `String`, and the harness's word rather than the engine's: nothing
    /// reachable from a `Deck` carries a name for the material in a slot.
    /// `Deck::slot` hands out a `HotSwap`, `HotSwap::set` hands out a `Set`, and a
    /// `Set` names its *nodes* (`Set::node_names`) and its *published controls*
    /// (`Published::name`) and has no name of its own — which is right, because a
    /// Set is built from a list of `.kir` files and only whoever passed that list
    /// knows what to call the result. So the caller names its own material, and a
    /// name invented in `src/` would be a label the console made up about somebody
    /// else's Set.
    ///
    /// Elided rather than wrapped where it does not fit, which is `.strip-name`'s
    /// own `overflow: hidden; text-overflow: ellipsis; white-space: nowrap`: a
    /// strip is 53 wide inside its padding and most real names are wider. An empty
    /// string draws no name at all, which is a harness with nothing to say rather
    /// than a strip with nothing in it.
    pub name: String,
    /// Where the slot sits — `Deck::residency`, which is the effective residency
    /// and not `requested_residency`. The governor moves a slot down without
    /// anybody asking it to, and a tally that did not follow it would be showing
    /// what was asked for over a slot doing something else.
    ///
    /// What the chip draws, and not what a press on it counts from.
    /// [`Mixer::tally`] steps from [`Strip::requested`]: the word says where the
    /// deck *is*, and the cycle is about what was last asked for. The two are the
    /// same value on every settled slot and they part exactly where it matters —
    /// see [`Mixer::tally`] for the parked case, which is the whole argument.
    pub tally: Tally,
    /// What was asked for — `Deck::requested_residency`, the other half of the pair
    /// [`Strip::tally`] is one of, and what a press on the chip counts from
    /// ([`Mixer::tally`]).
    ///
    /// # Why the strip carries both and derives nothing else
    ///
    ///
    /// [P-0087](../../../../docs/principles/0087-name-the-property-never-the-shape.md)
    /// asks a pending control to say three things — where it is, where it is going,
    /// and that it has not arrived — and the first two *are* these two values. The
    /// third is [`Strip::pending`], which is a comparison of them.
    ///
    /// There is deliberately no `parked: bool` beside them. A third field would be
    /// the same fact stored twice, and the copy is the one that goes stale: the
    /// harness could write a `tally` and a `requested` that disagree and a `parked`
    /// that says they do not, and nothing in this crate could tell. One derivation
    /// with several readers is this crate's habit — [`StripBox::fader_at`] is the
    /// same shape — and it is P-0087's *a pending state is derived every frame*
    /// read one level down, in the surface rather than in the engine.
    ///
    /// Two fields rather than one `Tally` grown into a pair. `Tally` is the mock's
    /// `.tally` and the engine's `Residency`, and both of those name *one* place a
    /// slot can sit; a variant meaning "allocated, and asked to prime" would be a
    /// fourth residency in a type whose own documentation says there are three and
    /// no fourth. The relation between two residencies is not a residency.
    pub requested: Tally,
    /// The trim — `Deck::gain`: linear, floored at zero, and deliberately open
    /// above 1.0 because the mix is HDR.
    ///
    /// Its own field and not the fader's twin. *"`gain` is a trim and `opacity` is
    /// the fader"*: opacity at zero silences under every blend mode and gain at
    /// zero does not silence `over`, which `karakuri-engine/src/mix.rs` states
    /// outright — so these are two controls, and the bay draws them as two rather
    /// than as two identical sliders. A `g` and a mini fader lying down against the
    /// tall one in the middle of the strip.
    pub gain: f32,
    /// Where a scheduled move is taking the trim, or `None` for a trim nothing is
    /// moving — `Deck::transitions_on(slot)`'s `Control::Gain` entry, and its
    /// `Transition::to`.
    ///
    /// # It is the destination and nothing else about the move
    ///
    /// A `Transition` also carries the instant it starts, how long it lasts and the
    /// curve it takes. None of the three is here, and the reason is the same seam
    /// every other field on this type is on: the console has no beat count, so a
    /// start in beats and a length in beats are two numbers it could not turn into
    /// anything a strip draws. What it can draw is where the control is going,
    /// which is the second of the three things P-0087 asks for and the whole of
    /// what the fader is being asked to say.
    ///
    /// Armed and running are the same state here, deliberately. A transition is in
    /// the deck's list from the moment it is scheduled until the beat it finishes
    /// on, and `Deck::transitions_on` answers with it throughout — so a fade
    /// quantised to the next bar and a fade halfway through are both *a request
    /// that has not arrived*, which is exactly the relation P-0087 is about. What
    /// separates them on the surface is that the value under the knob is moving in
    /// the second case, and the mark stands still in both.
    ///
    /// One per control, because the engine allows one: `Deck::schedule` cancels
    /// whatever was moving that pair before pushing, so a harness that finds two
    /// has read the wrong slot.
    ///
    /// # What it cannot say, and it is the trim's existing gap
    ///
    /// The track shows `[0, 1]` and the mix is HDR, so a move from 1.5 to 2.0 is
    /// two values the track draws in one place — see [`StripBox::trim_at`], where
    /// that gap is already written down. [`Strip::gain_pending`] answers `None`
    /// there rather than declaring a staleness for an animation that would not move
    /// a pixel.
    pub gain_to: Option<f32>,
    /// The fader — `Deck::opacity`: a proportion in `[0, 1]`, and the one control
    /// that silences a slot under every blend mode, which is what makes it the way
    /// out of material that has gone NaN.
    pub opacity: f32,
    /// Where a scheduled move is taking the fader, or `None` for a fader nothing is
    /// moving — `Deck::transitions_on(slot)`'s `Control::Opacity` entry, on
    /// [`Strip::gain_to`]'s terms and for its reasons.
    ///
    /// # And the third control has nowhere to say it
    ///
    /// `Control` has three members and this strip carries two of them.
    /// `Control::MaskPosition` — *how far a mask's front has travelled* — is the
    /// one a wipe is made of, and the strip has no control and no readout for it at
    /// all: the mask `.mini` names a *shape*, and the position, the angle and the
    /// softness are an inspector row that does not exist yet ([`Strip::mask`] says
    /// so, and [`Strip::mask_angle`] is the same fact from the other side). A
    /// destination carried here for it would be a number with nowhere to be drawn,
    /// and drawing it on the shape chip would say a shape was changing when none
    /// is.
    ///
    /// So an armed wipe is invisible on this panel, it is an under-draw rather than
    /// a decision that it does not matter, and it is
    /// [ADR-0206](../../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)'s
    /// named consequence: what closes it is a mask-position control, and it closes
    /// the same day that control lands.
    pub opacity_to: Option<f32>,
    /// The blend in force, as one of the vocabulary's three — and the word drawn on
    /// the chip is [`BlendMode::name`].
    ///
    /// # Why this is not the engine's word
    ///
    /// It was a `&'static str` — `Blend::name`, handed straight through, on the
    /// argument that the console has nothing to do with the blend but draw the
    /// engine's word. That stopped being true when the chip became a control
    /// ([ADR-0187](../../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md)):
    /// to say what the *next* mode is, [`Mixer::blend`] has to know which of the
    /// three this one is, and a string it cannot exhaustively match is the wrong
    /// carrier for that. A `&str` would make the chip's arithmetic a comparison
    /// against three literals with a fourth case that has no answer.
    ///
    /// So the type is the vocabulary's, and the harness converts.
    /// `karakuri-console` already depends on `karakuri-operation`, and
    /// `crates/karakuri/src/main.rs` turns `Deck::blend`'s
    /// `karakuri_engine::deck::Blend` into one of these with a `match`. The failure
    /// that buys is worth stating: a fourth engine blend mode with no operation
    /// variant stops compiling at the harness, rather than drawing a word on a chip
    /// no control can reach and no map can ask for. That is the vocabulary doing
    /// its job — P-0090's price, paid rather than avoided: a vocabulary that names
    /// a destination has to own the lists a destination is drawn from, or it is
    /// back to toggles (ADR-0180).
    ///
    /// The mock's own tooltip lists four — *"add, over, screen, multiply"* — and
    /// there are three. The disagreement was the mock's to settle and
    /// [ADR-0187](../../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md)
    /// settled it: `docs/manual/console.html` now lists the three that exist.
    pub blend: BlendMode,
    /// The mask in force — `Deck::mask(slot).kind()`. Its angle, position and
    /// softness are not drawn: `.mini` is a chip that says *which shape*, and three
    /// numbers about that shape are the inspector's row, not this one.
    ///
    /// What a press on the chip counts from ([`Mixer::mask`]), the way
    /// [`Strip::requested`] is what the tally's press counts from.
    pub mask: Mask,
    /// The angle the mask is already wearing — `Deck::mask(slot).angle()`, in
    /// radians. Read to build an operation, and drawn nowhere.
    ///
    /// # Why the strip carries a number no part of it paints
    ///
    /// [`Operation::SetMaskShape`] carries a shape and an angle, because the
    /// vocabulary's row is one an operator can say the whole of and a record is
    /// written whole
    /// ([ADR-0201](../../../../docs/adr/0201-the-mask-is-two-rows-because-a-control-change-can-only-set.md)).
    /// The chip names the shape and nothing on this strip names the angle — so a
    /// press has to carry a value the control does not control, and the only honest
    /// one is the value it already has. Sending `0.0` would make choosing a shape
    /// silently straighten a diagonal wipe: a press that changed something nobody
    /// asked it to, which is the class of failure
    /// [ADR-0192](../../../../docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)
    /// and ADR-0201 are both about, arriving one layer further out.
    ///
    /// It is [`Strip::requested`]'s arrangement exactly: a value the strip carries
    /// because a press has to be computed from it, written by whoever owns the deck
    /// like every other field here, and never a value this crate keeps. See
    /// [ADR-0203](../../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md).
    ///
    /// Not the position and not the softness, which the record also carries:
    /// neither is in the operation at all, and the half a shape operation does not
    /// ask for is filled in from a reading of the mask that is running, where the
    /// record is written — `karakuri_operation_record`'s `Current`. The angle is
    /// here because the *operation* names it; those two are not, because it does
    /// not.
    pub mask_angle: f32,
    /// What the slot's meter last read, or `None` for no reading at all.
    ///
    /// `Deck::level` is already `None` for *no meter, no measurement yet, a slot
    /// that is neither Live nor being auditioned, and a slot whose material was
    /// just replaced by a resize or a swap* — every case where a held reading would
    /// be about a different image. So `None` here draws the meter's well and
    /// nothing in it, which is the mock's own `alloc` strip: a `.vmeter` with no
    /// `b` and no `u` inside it.
    ///
    /// The one value on a strip that moves without a hand on anything, and it
    /// declares nothing — decided rather than missed, in [`View::mixer_declares`].
    pub level: Option<Level>,
    /// Whether this channel is currently muted.
    pub is_muted: bool,
    /// Whether this channel is currently soloed.
    pub is_soloed: bool,
}

impl Strip {
    /// Where this slot has been asked to go and has not got to, or `None` for a
    /// slot that is where it was asked to be.
    ///
    /// The one derivation the pending presentation reads, and it answers a *word*
    /// rather than a `bool` because that is what the surface has to draw: the
    /// second of P-0087's three is where the control is going, which ADR-0188
    /// states as *identifiable from the surface itself* — so the thing worth
    /// deriving is the destination and not the fact that there is one.
    ///
    /// # Why it is an inequality and not the engine's pair
    ///
    /// `Deck::is_parked` is exactly `requested == Residency::Priming && effective
    /// == Residency::Allocated`, and today this answers `Some` on precisely those
    /// slots — `deck.rs`'s module doc closes the other cases itself: *"the governor
    /// may hold a slot below what was asked for, and may never put one above it"*,
    /// and *"effective Live and requested Live are the same set of slots"*. So the
    /// two forms agree slot for slot, and there is no frame on which they differ.
    ///
    /// They differ in what a second kind of disagreement would do to them. Written
    /// as the engine's pair, a surface matching `Priming` over `Allocated` draws a
    /// settled chip over any other outstanding request — an under-draw, silent, and
    /// exactly the failure the rule exists to name. Written as an inequality it
    /// draws the new one without being taught, because the presentation was never
    /// about *parked*: it is about a request that has not landed, which is
    /// [P-0087](../../../../docs/principles/0087-name-the-property-never-the-shape.md)
    /// — the property rather than the one shape it currently takes.
    ///
    /// `park` is still the word, and it is the status line's: `karakuri-cli` spells
    /// it out for an operator in prose, which is the same clause met by a different
    /// means (ADR-0188).
    pub fn pending(&self) -> Option<Tally> {
        match self.requested == self.tally {
            true => None,
            false => Some(self.requested),
        }
    }

    /// Where the trim is going and has not got to, or `None` for a trim that is
    /// where it has been asked to be.
    ///
    /// [`Strip::pending`] one control along, and the same shape: the field is the
    /// request, the derivation is the third clause, and nothing is stored that says
    /// *a move is pending* — that fact is `self.gain_to` against `self.gain`, read
    /// every frame off values the harness rewrote this frame.
    ///
    /// # It compares what the track can show, and that is not the same as the raw
    /// pair
    ///
    /// The trim draws `[0, 1]` and gain is open above unity, so a scheduled move
    /// from 1.5 to 2.0 is two values with one position on this track
    /// ([`StripBox::trim_at`]). Compared raw it is pending; compared through
    /// [`unit`] it is not, and this compares through [`unit`].
    ///
    /// The reason is [`View::animating`] rather than the mark: a `Some` here
    /// declares a staleness, and a staleness bought for an animation that cannot
    /// move a pixel is the cost ADR-0193 refused to pay for a bay nobody can see,
    /// arriving through the other door. It is still an under-draw and it is the
    /// trim's existing gap, not a new one — the same track that cannot show 1.0
    /// against 3.0 cannot show a move between them, and both close together, on the
    /// pass that lets a hand push the control past the top.
    ///
    /// A destination that is not a number is zero, which is [`unit`]'s rule and is
    /// where a NaN out of an engine stops being a `NaN`-wide band.
    pub fn gain_pending(&self) -> Option<f32> {
        self.gain_to
            .filter(|to| unit(*to) != unit(self.gain))
            .map(unit)
    }

    /// Where the fader is going and has not got to, on [`Strip::gain_pending`]'s
    /// terms.
    ///
    /// Opacity is a proportion, so the clamp is the identity on every value a deck
    /// can hold and the comparison is the plain one. It is written through [`unit`]
    /// anyway, because a harness writing a destination the engine never held is the
    /// case both of these are the last line of defence for — and two derivations
    /// that agree except in the case nobody tests are worse than one shape written
    /// twice.
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
    ///
    /// A field, and not a fill stored beside its track — the thing [`StripBox`]
    /// refuses. The fill says where the value *is* and says nothing about how far
    /// it could go; the zero end is recoverable from the fill (a row's
    /// `fill.min.x`, a column's `fill.max.y`, neither of which moves) and the
    /// length is not recoverable from anything here. It is what turns a pointer
    /// back into a value — see [`Mixer::grab`] — so the derivation that draws the
    /// fader and the one that grabs it are one.
    pub travel: f32,
}

/// A meter, laid out: the well, the column the mean fills, and the peak's mark.
///
/// # It is not a [`Fader`], and the difference is not the handle alone
///
/// They share exactly one thing and it is [`filled`] — a value turned into a
/// length up a track — which is why that is a function of its own with three
/// call sites rather than a shared type. What they do not share is what each
/// is: a fader's knob is a grab target and [`Mixer::grab`] is what makes it
/// one, while a peak mark is a reading and never will be. A shared type would
/// be a type half of whose fields are about a gesture the other half can never
/// have — and [`Fader::travel`], which is the length a *hand* moves the value
/// along, is the field that says so.
///
/// They disagree about the track as well. `.vfader b` sits
/// [`size::VFADER_INSET`] inside its well and `.vmeter b` fills its own edge to
/// edge; `.vfader`'s fill is a capsule and `.vmeter` is `overflow: hidden`
/// around a square column; and a knob is meant to stand proud of its track
/// where nothing in a meter may leave the well.
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
///
/// The presentation
/// [P-0087](../../../../docs/principles/0087-name-the-property-never-the-shape.md)
/// is met with on a track, decided in
/// [ADR-0206](../../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md):
/// the knob and the fill go on saying where the control is, a mark says where
/// it is going, and the fill keeps setting off toward the mark and falling
/// back. Nothing about the value moves, which is a stricter first clause than
/// the tally's roll manages — the word there is lifted off its centre line and
/// this is not.
///
/// # Two rectangles and no phase
///
/// It is a [`Fader`]'s neighbour and it is measured the same way: a pure
/// function of the two values and the displacement, with no clock and no
/// `Phase` in it — [`roll_at`] is applied by whoever is painting, exactly as it
/// is for the tally's word. A test asks for a reach at a displacement it chose.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reach {
    /// The destination, as a hairline across the track at the position the knob
    /// would sit at if the move had landed.
    ///
    /// As long as the knob is wide, so it is the knob's own footprint reduced to a
    /// line — and so it always has the same two pixels of strip to be read against
    /// that the knob stands proud on ([`size::VFADER_KNOB_OUT`]), whatever the fill
    /// is doing underneath. A mark the width of the *track* would cross the fill's
    /// own lavender end in its own colour on a move down from the top.
    ///
    /// It is painted over the knob rather than under it. Where the two nearly
    /// coincide the knob would otherwise swallow it — 9 pixels of knob on a
    /// 98-pixel travel is every move under about a tenth of the fader, and a
    /// quarter of the trim — and a mark that disappears exactly when the control is
    /// nearly there says *arrived* at the one moment it must not.
    pub mark: Rect,
    /// What the reach has covered at this displacement: from the value's own edge
    /// toward [`Reach::mark`], and never as far as it — [`ROLL_REACH`] of the way
    /// at the top of the travel, nothing at rest.
    ///
    /// Across the fill's own width rather than the track's, because it is the fill
    /// reaching. Empty at rest, which is a rectangle with no area and nothing
    /// painted.
    pub band: Rect,
}

/// One strip's furniture: a rectangle for each of the six things stacked in it,
/// and the two tracks a value rides.
///
/// The fills and the knobs are not fields, because each is a function of a
/// value this already knows where to put — see [`StripBox::trim_at`],
/// [`StripBox::fader_at`] and [`StripBox::meter_at`]. A fill stored beside its
/// track is two statements about one number.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StripBox {
    /// `.strip` itself: the 9px well everything else is inside.
    pub rect: Rect,
    /// `.strip-name`, the full width of the strip's content box because the CSS
    /// says `width: 100%`.
    pub name: Rect,
    /// `.tally`'s capsule, as wide as the widest of the three residency words
    /// inside its padding — the same box whichever one it is showing, so the chip
    /// does not resize when the deck moves and does not resize under a word rolling
    /// through it. See [`mixer`], where it is measured.
    ///
    /// The word is centred in it ([`tally_into`]), and the capsule is centred in
    /// the strip, so widening the box does not move the word: it grows
    /// symmetrically around type that was already on the strip's centre line.
    ///
    /// It is the chip a press acts on and not only the box a word is painted into —
    /// [`Mixer::tally`] hit-tests exactly this rectangle, the way
    /// [`StripBox::blend`] is hit-tested. Being the widest word's width rather than
    /// the shown word's is what the blend chip cannot say: this target stands still
    /// while the deck moves under it and while a word rolls through it.
    pub tally: Rect,
    /// The SOLO toggle button rect on the left of the tally capsule.
    pub solo: Rect,
    /// The MUTE toggle button rect on the right of the tally capsule.
    pub mute: Rect,
    /// The `g` in `.trim`.
    pub trim_label: Rect,
    /// `.trim`'s `.fader`: the horizontal track, [`size::FADER_H`] tall.
    pub trim: Rect,
    /// `.vfader`: the tall track, [`size::FADER_COL_H`] high.
    pub fader: Rect,
    /// `.vmeter`, beside it.
    pub meter: Rect,
    /// `.strip-num`: the opacity as a number.
    pub num: Rect,
    /// The blend `.mini`, which is the chip a press acts on and not only the box a
    /// word is painted into — [`Mixer::blend`] hit-tests exactly this rectangle. As
    /// wide as the word in it, inside `.mini`'s padding and border.
    pub blend: Rect,
    /// The mask `.mini`, which is the chip a press acts on and not only the box a
    /// mark is drawn into — [`Mixer::mask`] hit-tests exactly this rectangle, the
    /// way [`StripBox::blend`] and [`StripBox::tally`] are. It holds a mark rather
    /// than a word, so it is the same width whichever shape it is showing: a target
    /// that stands still while the deck moves under it, which the tally's capsule
    /// buys by being the widest word's and the blend chip cannot say at all.
    pub mask: Rect,
}

impl StripBox {
    /// The trim at a gain, which is [`fader`] on the horizontal track.
    ///
    /// The gain is shown over `[0, 1]`, and that range is read off
    /// `karakuri-midi`'s `GAIN_RANGE` rather than chosen here: *"`[0, 1]` for gain
    /// even though the mix is HDR and values above 1.0 are ordinary … a fader whose
    /// top is unity is what a fader means."* So a gain pushed past unity fills the
    /// track and stops, and this bay cannot yet show the difference between 1.0 and
    /// 3.0 — a real gap, and it belongs with the pass that lets a hand push the
    /// control past the top.
    pub fn trim_at(&self, gain: f32) -> Fader {
        fader(
            self.trim,
            Axis::Row,
            gain,
            0.0,
            egui::vec2(size::FADER_KNOB_W, size::FADER_KNOB_H),
        )
    }

    /// The fader at an opacity, which is [`fader`] on the vertical track.
    pub fn fader_at(&self, opacity: f32) -> Fader {
        fader(
            self.fader,
            Axis::Column,
            opacity,
            size::VFADER_INSET,
            egui::vec2(size::VFADER_KNOB_W, size::VFADER_KNOB_H),
        )
    }

    /// The trim's scheduled move, from the gain it is at to the one it was asked
    /// for, `rolled` of the way — which is [`reach`] on the horizontal track.
    ///
    /// Off [`StripBox::trim_at`] twice rather than off any arithmetic of its own:
    /// the destination's mark is *where the knob would be*, so it is the same
    /// derivation asked a second question, and a mark that drifted from the knob it
    /// stands for would be a fader with two ideas of what a value looks like.
    pub fn trim_reach(&self, gain: f32, to: f32, rolled: f32) -> Reach {
        reach(self.trim_at(gain), self.trim_at(to), rolled)
    }

    /// The fader's scheduled move, on [`StripBox::trim_reach`]'s terms and off
    /// [`StripBox::fader_at`].
    pub fn fader_reach(&self, opacity: f32, to: f32, rolled: f32) -> Reach {
        reach(self.fader_at(opacity), self.fader_at(to), rolled)
    }

    /// The meter at a reading.
    pub fn meter_at(&self, level: Level) -> Meter {
        // **The peak mark stays inside the well.** `.vmeter u` is placed by
        // its own `bottom`, so a 2px bar at a peak of 1.0 would sit from the
        // top of the well to two pixels above it — and `.vmeter` is
        // `overflow: hidden`, which clips it away exactly at the reading that
        // matters most. Its travel is the well's height less its own, which is
        // the clamp `Transport::beat` makes on the far end of a grid.
        let travel = (self.meter.height() - size::VMETER_PEAK_H).max(0.0);
        let top = self.meter.max.y - size::VMETER_PEAK_H - travel * unit(level.peak);
        Meter {
            well: self.meter,
            fill: filled(self.meter, Axis::Column, level.mean),
            peak: Rect::from_min_size(
                Pos2::new(self.meter.min.x, top),
                egui::vec2(self.meter.width(), size::VMETER_PEAK_H),
            ),
        }
    }
}
/// Where the strips go: the `.mixer-strips` grid inside a mixer region.
///
/// The region less [`size::HEAD_H`] for the bay head painted over the top of
/// it, inset by [`size::STRIPS_PAD`] left, right and top, and exactly
/// [`size::STRIP_H`] tall rather than whatever is left over. The bay is taller
/// than its strips by design — the reservation for `.xfade` is the other 61 of
/// it, of which [`xfade_row`] carves [`size::XFADE_H`] and 23.5 is the
/// crossfader row the mock no longer draws — and a mixer with room to grow (it
/// is the only visible child of a soloed right pane, and a fixed child with
/// room takes it, ADR-0157) grows the bay and not the strips: a `.strip` is a
/// column of fixed type around a `.fader-col` whose 104 is stated in the CSS,
/// so there is nothing in it that gets bigger any more than there is anything
/// that gets smaller.
///
/// `None` where the region cannot hold it — folded away, soloed away, or a
/// window too small — which is [`picture_rect`]'s rule stated on a row of
/// strips.
pub fn strips_row(region: Rect) -> Option<Rect> {
    let row = Rect::from_min_size(
        Pos2::new(
            region.min.x + size::STRIPS_PAD,
            region.min.y + size::HEAD_H + size::STRIPS_PAD,
        ),
        egui::vec2(region.width() - size::STRIPS_PAD * 2.0, size::STRIP_H),
    );
    match row.width() > 0.0 && region.contains_rect(row) {
        true => Some(row),
        false => None,
    }
}

/// The arithmetic of one strip, away from the type it measures and the layout
/// it reads.
///
/// Term for term from `.strip` and what is in it, in `style.css`:
///
/// - `.strip { display: flex; flex-direction: column; align-items: center;
///   gap: 5px; padding: 7px 4px }` — six things stacked from the top of the
///   strip's content box, one [`size::STRIP_GAP_Y`] between each pair, each
///   centred across the strip rather than filling it. Two are the
///   exception, and the CSS states both: `.strip-name` and `.trim` are
///   `width: 100%`.
/// - `.trim { gap: 5px; padding: 0 3px }` with `.trim .fader { flex: 1 }` —
///   the `g`, then the track, which takes what is left and is centred in the
///   row because the label is the taller of the two.
/// - `.fader-col { display: flex; gap: 6px; height: 104px }` around a
///   `.vfader` of 17 and a `.vmeter` of 6, both `height: 100%` — so the column
///   is 29 wide, centred, and its `align-items: flex-end` has nothing left to
///   align.
/// - `.strip-mode { gap: 3px }` — two minis, centred.
///
/// `None` where the track is too narrow to hold what is in it, which is
/// [`picture_rect`]'s rule stated on a strip: the fader column is the widest
/// fixed thing in it, and a strip that cannot hold that has no readings to
/// show. At the right pane's own minimum of 172 a track is exactly 37 and the
/// column is exactly 29 inside 4 + 4 of padding, so the arrangement's minimum
/// and this are one number or neither.
pub(super) fn strip_box(track: Rect, label_w: f32, tally_w: f32, blend_w: f32) -> Option<StripBox> {
    let inner = Rect::from_min_max(
        Pos2::new(
            track.min.x + size::STRIP_PAD_X,
            track.min.y + size::STRIP_PAD_Y,
        ),
        Pos2::new(
            track.max.x - size::STRIP_PAD_X,
            track.max.y - size::STRIP_PAD_Y,
        ),
    );
    let column_w = size::VFADER_W + size::FADER_COL_GAP + size::VMETER_W;
    if inner.width() < column_w {
        return None;
    }
    // Each row starts one gap after the one before it ended, and is as tall as
    // its own type — which is what a flex column is.
    let mut y = inner.min.y;
    let mut row = |h: f32| {
        let at = Rect::from_min_size(Pos2::new(inner.min.x, y), egui::vec2(inner.width(), h));
        y = at.max.y + size::STRIP_GAP_Y;
        at
    };
    let name = row(size::STRIP_NAME_SIZE * size::LINE);
    let tally = centred_in(row(size::TALLY_H), tally_w + size::TALLY_PAD_X * 2.0);
    let gap = 4.0;
    let btn_w = (tally.width() - gap) * 0.5;
    let solo = Rect::from_min_size(tally.min, egui::vec2(btn_w, tally.height()));
    let mute = Rect::from_min_size(
        Pos2::new(tally.max.x - btn_w, tally.min.y),
        egui::vec2(btn_w, tally.height()),
    );
    let trim = row(size::TRIM_H);
    let column = centred_in(row(size::FADER_COL_H), column_w);
    let num = row(size::STRIP_NUM_SIZE * size::LINE);
    let blend_w = blend_w + size::MINI_PAD_X * 2.0 + size::HAIRLINE * 2.0;
    // The mask's mini holds a mark rather than a word, and the mark is drawn
    // at the size the glyph it stands in for would have been — see `Mask`.
    let mask_w = size::MINI_SIZE + size::MINI_PAD_X * 2.0 + size::HAIRLINE * 2.0;
    let mode = centred_in(row(size::MINI_H), blend_w + size::MODE_GAP + mask_w);

    let trim_label = Rect::from_min_size(
        Pos2::new(trim.min.x + size::TRIM_PAD_X, trim.min.y),
        egui::vec2(label_w, trim.height()),
    );
    let trim_track = Rect::from_min_size(
        Pos2::new(
            trim_label.max.x + size::TRIM_GAP,
            trim.center().y - size::FADER_H * 0.5,
        ),
        egui::vec2(
            trim.max.x - size::TRIM_PAD_X - trim_label.max.x - size::TRIM_GAP,
            size::FADER_H,
        ),
    );
    match trim_track.width() > 0.0 {
        true => Some(StripBox {
            rect: track,
            name,
            tally,
            solo,
            mute,
            trim_label,
            trim: trim_track,
            fader: Rect::from_min_size(column.min, egui::vec2(size::VFADER_W, column.height())),
            meter: Rect::from_min_size(
                Pos2::new(column.max.x - size::VMETER_W, column.min.y),
                egui::vec2(size::VMETER_W, column.height()),
            ),
            num,
            blend: Rect::from_min_size(mode.min, egui::vec2(blend_w, mode.height())),
            mask: Rect::from_min_size(
                Pos2::new(mode.max.x - mask_w, mode.min.y),
                egui::vec2(mask_w, mode.height()),
            ),
        }),
        false => None,
    }
}

/// A box `w` wide centred across `row`, which is `align-items: center` on one
/// child of a flex column.
pub(super) fn centred_in(row: Rect, w: f32) -> Rect {
    Rect::from_min_size(
        Pos2::new(row.center().x - w * 0.5, row.min.y),
        egui::vec2(w, row.height()),
    )
}

/// A scheduled move on a laid-out fader: the mark on where it is going, and the
/// band reaching `rolled` of the way from where it is.
///
/// Two faders in and no value arithmetic, which is what keeps this honest:
/// `now` and `to` are the same track measured at the two values, so the mark
/// lands exactly where the knob would and the band starts exactly where the
/// fill ends. The knob's centre is the fill's moving edge — [`fader`] says so —
/// and that one number is the whole of what this reads out of each.
///
/// The displacement is a fraction of the gap rather than a distance, so the
/// reach is in proportion to the move: a fade across the fader sets off a long
/// way and a fade of a hundredth sets off a pixel. That is the honest picture
/// and it is also the limit — under about a hundredth of the travel the whole
/// disagreement is a pixel, and what carries the message there is the mark
/// rather than the motion. See
/// [ADR-0206](../../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md).
///
/// Two call sites the day it is written, which is this repository's rule about
/// an abstraction: the trim and the opacity fader, exactly as [`fader`] itself
/// has.
fn reach(now: Fader, to: Fader, rolled: f32) -> Reach {
    let (from, dest) = (now.knob.center(), to.knob.center());
    match now.axis {
        Axis::Row => {
            let head = from.x + (dest.x - from.x) * rolled;
            Reach {
                mark: Rect::from_center_size(
                    Pos2::new(dest.x, now.track.center().y),
                    egui::vec2(size::HAIRLINE, now.knob.height()),
                ),
                band: Rect::from_min_max(
                    Pos2::new(from.x.min(head), now.fill.min.y),
                    Pos2::new(from.x.max(head), now.fill.max.y),
                ),
            }
        }
        Axis::Column => {
            let head = from.y + (dest.y - from.y) * rolled;
            Reach {
                mark: Rect::from_center_size(
                    Pos2::new(now.track.center().x, dest.y),
                    egui::vec2(now.knob.width(), size::HAIRLINE),
                ),
                band: Rect::from_min_max(
                    Pos2::new(now.fill.min.x, from.y.min(head)),
                    Pos2::new(now.fill.max.x, from.y.max(head)),
                ),
            }
        }
    }
}

/// The next blend mode round the cycle, wrapping from the last back to the
/// first — and the whole of the affordance the blend chip is.
///
/// It is four lines here and nothing at all in `karakuri-operation`, which is
/// P-0090's division: a toggle is an affordance, built over operations by
/// whoever draws the control, and it belongs there. The vocabulary owns the
/// three values; this owns the order a pointer walks them in.
///
/// A match rather than an index into [`BlendMode::ALL`], for the reason
/// [`BlendMode::name`] is one: a fourth mode does not compile until somebody
/// says what follows it. The cost is that the order is written twice — here and
/// in `ALL` — so `tests/blend.rs` walks `ALL` through this and asserts they are
/// the same cycle, which is the measurement that keeps the two from drifting
/// rather than a comment promising they will not.
///
/// # It is the workspace's only blend cycle, and it is `pub` for that
///
/// `crates/karakuri/src/main.rs` had one of its own — `after_blend`, what one
/// press of `m` asked for — saying the same three modes in the same order, with
/// a test each. Under the grammar `space` on an addressed blend chip asks
/// [`crate::focus::press`], which asks this, so the key and the chip are one
/// cycle and the copy is gone
/// ([ADR-0333](../../../../docs/adr/0333-the-console-resolves-the-address-and-the-window-loop-names-the-operation.md)).
/// What is left in that file is a device test asserting where the cycle arrives
/// on a real deck, and it asks this rather than restating it — which is the
/// whole of why this is `pub` where [`next`] and [`next_shape`] beside it are
/// `pub(crate)`.
pub fn after(blend: BlendMode) -> BlendMode {
    match blend {
        BlendMode::Add => BlendMode::Over,
        BlendMode::Over => BlendMode::Max,
        BlendMode::Max => BlendMode::Add,
    }
}

/// The next residency round the cycle, wrapping from the last back to the first
/// — the whole of the affordance the tally chip is, and the order the mock's
/// own tooltip lists: *"one of three residencies — live, priming, allocated"*.
///
/// [`after`]'s division, one control along: the cycle is three lines here and
/// nothing in `karakuri-operation`, which owns the three values and not the
/// order a pointer walks them in (P-0090).
///
/// A match rather than an index into [`Tally::ALL`], for [`after`]'s reason: a
/// fourth residency does not compile until somebody says what follows it. The
/// price is that the order is written twice — here and in `ALL` — so
/// `tests/tally.rs` walks `ALL` through this and asserts they are the same
/// cycle.
///
/// What it is asked about is [`Strip::requested`], and why is [`Mixer::tally`].
pub(crate) fn next(tally: Tally) -> Tally {
    match tally {
        Tally::Live => Tally::Priming,
        Tally::Priming => Tally::Allocated,
        Tally::Allocated => Tally::Live,
    }
}

/// The console's word for a residency, as the vocabulary's — and it is the
/// whole of what the console has to know about the difference.
///
/// [`Tally`] is the mock's `.tally` and the engine's `Residency` seen from the
/// surface; [`karakuri_operation::Residency`] is what an operation may name.
/// They are the same three states and two crates' words for them, so the
/// translation is a match — and a match rather than a cast so that a fourth
/// state on either side stops the build here, where the two lists meet, rather
/// than at a chip drawing a word no operation can carry.
///
/// The mirror image of it is `crates/karakuri/src/main.rs`'s `tally`, which
/// turns the *engine's* `Residency` into a [`Tally`] on the way in. Three names
/// for three states is the cost `karakuri-operation` pays for depending on
/// nothing (P-0090), and this is one of the two places it is paid.
pub(crate) fn residency(tally: Tally) -> Residency {
    match tally {
        Tally::Live => Residency::Live,
        Tally::Priming => Residency::Priming,
        Tally::Allocated => Residency::Allocated,
    }
}

/// The next mask shape round the cycle, wrapping from the last back to the
/// first — the whole of the affordance the mask mini is.
///
/// [`after`]'s division, one control along: the cycle is three lines here and
/// nothing in `karakuri-operation`, which owns the three shapes and not the
/// order a pointer walks them in (P-0090).
///
/// A match, for [`after`]'s reason: a fourth shape does not compile until
/// somebody says what follows it. Unlike [`after`] and [`next`] the price is
/// not a second copy of the order — [`Mask`] has no `ALL` and neither does
/// `karakuri_operation::WipeKind`, because nothing reads one (see [`Mask`]) —
/// so this is the only statement of the order in this crate, and
/// `tests/mask.rs` is what checks it against the order it is a copy of.
///
/// It starts at [`Mask::None`], which is
/// `karakuri_engine::deck::MaskKind::ALL`'s own order and its own reason:
/// *"`None` first, because it is the default and a cycle should start where a
/// slot starts."* A cycle that began at `Linear` would be a control whose first
/// press on a fresh slot moves it somewhere it has never been.
pub(crate) fn next_shape(mask: Mask) -> Mask {
    match mask {
        Mask::None => Mask::Linear,
        Mask::Linear => Mask::Radial,
        Mask::Radial => Mask::None,
    }
}

/// The console's word for a mask shape, as the vocabulary's — [`residency`] one
/// control along, and for its reason exactly.
///
/// [`Mask`] is the mock's second `.mini` and the engine's `MaskKind` seen from
/// the surface; [`karakuri_operation::WipeKind`] is what an operation may name.
/// They are the same three shapes and two crates' words for them, so the
/// translation is a match — and a match rather than a cast so that a fourth
/// shape on either side stops the build here, where the two lists meet, rather
/// than at a chip drawing a mark no operation can carry.
///
/// The mirror image of it is `crates/karakuri/src/main.rs`'s reading of
/// `Deck::mask(slot).kind()`, which turns the *engine's* `MaskKind` into a
/// [`Mask`] on the way in.
pub(crate) fn wipe_kind(mask: Mask) -> WipeKind {
    match mask {
        Mask::None => WipeKind::None,
        Mask::Linear => WipeKind::Linear,
        Mask::Radial => WipeKind::Radial,
    }
}

/// `.strip-name` as one laid-out run: `overflow: hidden; text-overflow:
/// ellipsis; white-space: nowrap` is exactly one row, broken anywhere, with an
/// ellipsis standing for what did not fit.
fn name_job(name: &str, width: f32, colour: Color32) -> LayoutJob {
    let mut job = span_at(name, size::STRIP_NAME_SIZE, colour);
    job.wrap = egui::epaint::text::TextWrapping {
        max_width: width,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    job
}

/// One strip, term for term from `style.css`:
///
/// - `.strip` — `background: var(--c-well)`, `border-radius: 9px`.
/// - `.strip-name` — `font-size: 10px`, `var(--c-dim)`, centred, elided.
/// - `.tally` — three washes and three inks, which is [`tally_into`]'s.
/// - `.trim .lbl` — `font-size: 9px`, `var(--c-faint)`.
/// - `.fader` and `.vfader` — [`fader_into`]'s.
/// - `.vmeter` — [`meter_into`]'s.
/// - `.strip-num` — `var(--c-text)`, *a value*, and `font-weight: 500` is not
///   honoured because `egui`'s default proportional face has no bold.
/// - `.strip-mode` — two [`mini_into`]s.
pub(super) fn strip_into(ui: &Ui, pal: &Palette, strip: &Strip, at: StripBox, phase: Phase) {
    // Clipped to the strip and half the gap around it: a fader's knob is
    // meant to stand proud of its track, and `.mixer-strips`' 4px gap is where
    // that goes — but nothing in one strip may reach the strip beside it.
    let painter = ui
        .painter()
        .with_clip_rect(at.rect.expand(size::STRIP_GAP * 0.5));
    painter.rect_filled(
        at.rect,
        CornerRadius::same(size::STRIP_RADIUS as u8),
        pal.well,
    );

    if !strip.name.is_empty() {
        let galley = painter.layout_job(name_job(&strip.name, at.name.width(), pal.dim));
        centre_galley(&painter, at.name, galley, pal.dim);
    }

    solo_button_into(&painter, pal, at.solo, strip.is_soloed);
    mute_button_into(&painter, pal, at.mute, strip.is_muted);

    let galley = painter.layout_job(span_at(TRIM_LABEL, size::TRIM_LABEL_SIZE, pal.faint));
    painter.galley(
        Pos2::new(
            at.trim_label.min.x,
            at.trim_label.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.faint,
    );
    // **The same displacement for both faders and for the chip above them**,
    // because it is the same phase: ADR-0190's *one phase, not one per
    // animation*, which on this strip is three presentations reading one
    // number. A strip with a fade on each fader reaches twice, in step.
    let rolled = roll_at(phase);
    fader_into(
        &painter,
        pal,
        at.trim_at(strip.gain),
        false,
        strip
            .gain_pending()
            .map(|to| at.trim_reach(strip.gain, to, rolled)),
    );
    fader_into(
        &painter,
        pal,
        at.fader_at(strip.opacity),
        strip.tally == Tally::Live,
        strip
            .opacity_pending()
            .map(|to| at.fader_reach(strip.opacity, to, rolled)),
    );
    meter_into(&painter, pal, at.meter, strip.level.map(|l| at.meter_at(l)));

    // `.strip-num`: the opacity to two places, which is what the mock writes.
    // The value itself and not the fader's clamped one — the fader clamps
    // because a track has ends and a number does not.
    let galley = painter.layout_job(span_at(
        &format!("{:.2}", strip.opacity),
        size::STRIP_NUM_SIZE,
        pal.text,
    ));
    centre_galley(&painter, at.num, galley, pal.text);

    mini_into(&painter, pal, at.blend, true, |painter, colour| {
        let galley = painter.layout_job(span_at(strip.blend.name(), size::MINI_SIZE, colour));
        centre_galley(painter, at.blend, galley, colour);
    });
    mini_into(&painter, pal, at.mask, false, |painter, colour| {
        mask_mark(painter, at.mask.center(), colour, strip.mask);
    });
}

/// The meter: the well, the mean's column and the peak's mark — and nothing
/// in the well where there is no reading, which is the mock's own `alloc`
/// strip.
///
/// - `.vmeter` — the fader's well, at 6 wide.
/// - `.vmeter b` — `linear-gradient(0deg, var(--c-mint), var(--c-sun))`, and
///   square rather than a capsule: `.vmeter` is `overflow: hidden` and the
///   column is clipped by the well rather than rounded itself.
/// - `.vmeter u` — `background: var(--c-pink)`, *on air*, which is the colour
///   the peak shares with the live tally and the lit beat.
fn meter_into(painter: &egui::Painter, pal: &Palette, well: Rect, meter: Option<Meter>) {
    let radius = CornerRadius::same((well.width() * 0.5) as u8);
    painter.rect_filled(well, radius, pal.well);
    painter.rect_stroke(
        well,
        radius,
        Stroke::new(size::HAIRLINE, pal.hair),
        StrokeKind::Inside,
    );
    let Some(meter) = meter else {
        return;
    };
    let painter = painter.with_clip_rect(meter.well);
    gradient(&painter, meter.fill, Axis::Column, pal.mint, pal.sun, false);
    painter.rect_filled(meter.peak, CornerRadius::ZERO, pal.pink);
}

/// The mask's mark, drawn rather than typed — see [`Mask`].
///
/// A circle [`size::MINI_SIZE`] across, which is the size the glyph it stands
/// in for would have been, and then what the mask does to it: nothing, one half
/// filled, or a filled centre.
pub fn mask_mark(painter: &egui::Painter, centre: Pos2, colour: Color32, mask: Mask) {
    let r = size::MINI_SIZE * 0.5;
    match mask {
        Mask::None => {}
        // A half-disc is a circle with half of it not painted, so it is a clip
        // rather than a path: `epaint` has no arc and a polygon of one would
        // be a curve this file approximated by hand.
        Mask::Linear => {
            painter
                .with_clip_rect(Rect::from_min_max(
                    Pos2::new(centre.x, centre.y - r),
                    Pos2::new(centre.x + r, centre.y + r),
                ))
                .circle_filled(centre, r, colour);
        }
        Mask::Radial => {
            painter.circle_filled(centre, r * 0.5, colour);
        }
    }
    painter.circle_stroke(centre, r, Stroke::new(size::HAIRLINE, colour));
}

/// Renders the SOLO toggle button on the mixer channel strip.
fn solo_button_into(painter: &egui::Painter, pal: &Palette, rect: Rect, active: bool) {
    let radius = CornerRadius::same((rect.height() * 0.5) as u8);
    let (fill, stroke_col, ink) = match active {
        true => (
            super::super::widgets::fader::tint(pal.sun, 25),
            pal.sun,
            pal.sun,
        ),
        false => (pal.well, pal.line, pal.dim),
    };
    painter.rect_filled(rect, radius, fill);
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(size::HAIRLINE, stroke_col),
        StrokeKind::Inside,
    );
    let galley = painter.layout_job(span_at("S", size::TALLY_SIZE, ink));
    centre_galley(painter, rect, galley, ink);
}

/// Renders the MUTE toggle button on the mixer channel strip.
fn mute_button_into(painter: &egui::Painter, pal: &Palette, rect: Rect, active: bool) {
    let radius = CornerRadius::same((rect.height() * 0.5) as u8);
    let (fill, stroke_col, ink) = match active {
        true => (
            super::super::widgets::fader::tint(pal.pink, 25),
            pal.pink,
            pal.pink,
        ),
        false => (pal.well, pal.line, pal.faint),
    };
    painter.rect_filled(rect, radius, fill);
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(size::HAIRLINE, stroke_col),
        StrokeKind::Inside,
    );
    let galley = painter.layout_job(span_at("M", size::TALLY_SIZE, ink));
    centre_galley(painter, rect, galley, ink);
}
