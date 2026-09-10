use super::*;

// ---------------------------------------------------------------------------
// The Mixer bay
// ---------------------------------------------------------------------------

/// **The word at the head of the Mixer bay**, in the source's own
/// capitalisation for [`Kind::Bay`]'s reason: the mock upper-cases in CSS, and
/// that is done at paint time so the word a reader searches for is the word in
/// the source.
pub(super) const MIXER_TITLE: &str = "Mixer";

/// `.trim .lbl`: the `g`, which is the only word in this bay that is neither
/// the deck's nor the engine's — it is the mock's.
const TRIM_LABEL: &str = "g";

/// **How long the panel has been animating**, and the one value everything
/// that moves on it derives its own rate from.
///
/// # One phase, panel-wide, because two clocks drift and one does not
///
/// [ADR-0190](../../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md)
/// decided exactly this: *"two controls moving out of step looks broken
/// rather than informative, and N animations are N deadlines where one phase
/// is one."* Two strips parked at once
/// are two rolls, and they are the same roll because they are read off the
/// same number.
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
/// `src/` reads no clock. [`Transport`]'s own doc says why in the general
/// case — *"an `Instant` here would put a clock in it, and then the row would
/// be reading wall time in a repository whose first principle is that nothing
/// does"*
/// ([P-0092](../../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md))
/// — and every `Instant::now` in this crate is in `crates/karakuri/src/main.rs`, which
/// owns the window. An `Instant` is a *reading*; a `Duration` is a number, and
/// a number is what a caller writes and a test chooses. So this arrives per
/// frame the way [`Transport`] and [`View::mixer`] arrive
/// ([ADR-0156](../../../../docs/adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)),
/// and a test asserting an animation writes the phase it wants rather than
/// catching one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Phase(Duration);

impl Phase {
    /// The origin, and what a console with no clock behind it is at — every
    /// test in this crate, and the first frame of a window.
    pub const ZERO: Phase = Phase(Duration::ZERO);

    /// A phase from the interval that has elapsed since whoever owns the clock
    /// started counting. Where the origin is does not matter and is not
    /// asked: every presentation is periodic in it.
    pub const fn since(elapsed: Duration) -> Phase {
        Phase(elapsed)
    }

    /// **Where this phase sits inside one `period`**, as a fraction in
    /// `[0, 1)`.
    ///
    /// The one operation a presentation performs on a phase, and the reason
    /// the carrier is an interval: a rate is chosen here, by the thing that
    /// moves, rather than baked into the value the harness writes.
    ///
    /// In `f64` and returned as `f32`. A session runs for hours and a phase is
    /// seconds from an origin — at 3600 s an `f32` step is already coarser
    /// than a millisecond, and the fractional part is what survives the
    /// division, so the division is the one place the extra bits are worth
    /// having.
    pub fn cycle(self, period: Duration) -> f32 {
        let period = period.as_secs_f64();
        match period > 0.0 {
            true => (self.0.as_secs_f64() / period).fract() as f32,
            false => 0.0,
        }
    }
}

/// **How often anything pending on this panel sets off toward where it is
/// going**, and the period every other number in the presentation is a
/// fraction of.
///
/// One second, which is the roll's *"about once a second"* and is the rate
/// that reads as *waiting* rather than as a fault. See
/// [ADR-0190](../../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md).
///
/// **Two users, one rate, and that is the rule rather than a coincidence**:
/// the tally's word rolling toward a residency, and a fader's fill reaching
/// toward a scheduled value ([`Reach`],
/// [ADR-0206](../../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)).
/// ADR-0190 asks for one phase panel-wide because two controls moving out of
/// step looks broken rather than informative, and a second period here is how
/// that would happen with nothing failing to compile.
pub const ROLL_PERIOD: Duration = Duration::from_millis(1000);

/// **How much of the period the word is moving for**: 400 ms out and back,
/// and 600 ms at rest.
///
/// The rest is not slack. It is what makes the roll read as *an attempt that
/// keeps being made* rather than as a chip that wobbles — and it is what makes
/// most of a parked strip's frames the strip's settled appearance, so the
/// operator reads the effective residency off a still chip nearly two thirds
/// of the time — the first of the three things P-0087 asks a pending control
/// to say, which is where it *is*.
pub const ROLL_TRAVEL: Duration = Duration::from_millis(400);

/// **How far toward the destination the moving thing gets**: a fraction of the
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

/// **How many steps the travel is drawn in**, which is the only reason
/// [`ROLL_STALENESS`] is a number at all.
pub(super) const ROLL_STEPS: u32 = 12;

/// **How stale the roll may get**, which is what a presentation declares under
/// [P-0091](../../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)
/// and what the harness turns into a deadline.
///
/// [`ROLL_TRAVEL`] in [`ROLL_STEPS`] steps — 33.33 ms, about thirty a second.
/// It is **one number for the whole period** rather than a fine one while the
/// word moves and a coarse one while it rests: two numbers is two live regions
/// wearing one name, and choosing between them frame by frame is a scheduler's
/// job (ADR-0164's second half) rather than a presentation's. A presentation
/// declares what it needs; what the panel can afford is decided somewhere
/// else.
pub const ROLL_STALENESS: Duration =
    Duration::from_micros(ROLL_TRAVEL.as_millis() as u64 * 1000 / ROLL_STEPS as u64);

/// **How far a pending presentation has got toward its destination this
/// frame**, as a fraction of the distance to it: `0.0` at rest, [`ROLL_REACH`]
/// at the top of the travel, and never `1.0`.
///
/// One curve, two readers — the pitch between the tally's two words
/// ([`tally_into`]) and the gap between a fader's value and its mark
/// ([`reach`]).
///
/// A raised cosine over [`ROLL_TRAVEL`], flat for the rest of
/// [`ROLL_PERIOD`]. It is that curve rather than a triangle for one reason
/// worth having: it leaves and arrives at zero *with zero velocity*, so the
/// word does not snap into the rest it holds for the next 600 ms, and the
/// frame the travel begins on is not a jump.
///
/// **A pure function of the phase, which is the point of the phase being a
/// value.** A test asserts it at a phase it chose; nothing samples a clock to
/// find out what the panel is doing.
pub fn roll_at(phase: Phase) -> f32 {
    let travel = ROLL_TRAVEL.as_secs_f32() / ROLL_PERIOD.as_secs_f32();
    let t = phase.cycle(ROLL_PERIOD);
    match t < travel {
        true => ROLL_REACH * 0.5 * (1.0 - (t / travel * std::f32::consts::TAU).cos()),
        false => 0.0,
    }
}

/// **How long until anything rolling on this panel next moves**, from
/// `phase` — [`ROLL_STALENESS`] while the travel is under way, and the rest of
/// the rest while it is not.
///
/// # What it is for
///
/// [`roll_at`] is exactly `0.0` for the 600 ms of every [`ROLL_PERIOD`] that
/// is not [`ROLL_TRAVEL`], so a chip drawn at the start of the rest and a chip
/// drawn 33 ms later are the same picture, pixel for pixel. Servicing
/// [`ROLL_STALENESS`] through that stretch buys seventeen frames a second of
/// the panel being redrawn exactly as it already is — 31 asked for in a period
/// where 14 draw something, counted in `tests/moving.rs`. This is what the
/// declaration answers instead, and
/// [ADR-0283](../../../../docs/adr/0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)
/// is the argument.
///
/// **It is not a second rate.** ADR-0190 rejected *a fine deadline while the
/// word moves and a coarse one while it rests* on the grounds that two numbers
/// is two live regions wearing one name; the rest here is not a second
/// tolerance somebody chose but the same two constants read for when the curve
/// leaves zero, so there is nothing that could drift from the rate and nothing
/// to arbitrate between. [`ROLL_STALENESS`] is still the only staleness this
/// presentation declares, and it is still what the arithmetic sums.
///
/// # It never answers finer than the rate it declared
///
/// A rest with less than one step left of it answers one step, which is
/// [`crate::budget::Declared::moves_in`]'s invariant and costs at most the
/// first [`ROLL_STALENESS`] of the travel — one step out of twelve, and inside
/// the tolerance the presentation itself named. The alternative is a deadline
/// tending to zero as the period wraps, which is the spin
/// [`crate::repaint`] exists to refuse.
///
/// **A pure function of the phase**, for [`roll_at`]'s reason: a test chooses
/// the phase it asserts at, and nothing here reads a clock.
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

/// **Where a slot sits between compiled and composited**, which is the mock's
/// `.tally` and `karakuri_engine`'s `Residency` — three states and no fourth.
///
/// The words are the mock's abbreviations and the variants are the engine's
/// names, because each document owns one end: `live`, `prim` and `alloc` are
/// what fits in a strip 53 wide, and `Live`, `Priming` and `Allocated` are
/// what `Deck::residency` answers.
///
/// **The mock drew a fourth and it was not a residency.** Its `.strip.empty`
/// read `empty` through a `.tally.off` rule, and a `Deck` has no such slot:
/// every one of its `slot_count` slots holds a `HotSwap`. An empty strip is a
/// *track on the page nothing fills*, and this bay draws nothing at all in one
/// rather than a strip full of dashes — see [`mixer`].
///
/// **The page has since agreed**: `39f1e6b` took that strip out under
/// ADR-0178, and `.tally.off` went with it as the one rule only that strip
/// used. The argument is kept in the past tense rather than deleted, because
/// it is still why there is no fourth variant here — the three below are the
/// residencies a `Deck` has, and a fourth would have to be invented whether or
/// not a mock is drawing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tally {
    /// `.tally.live` — stepped and composited. On air.
    Live,
    /// `.tally.priming` — stepped and warming its buffers, drawn into its own
    /// cell, and asked for.
    Priming,
    /// `.tally.alloc` — stepped and drawn like a priming slot, and asked of
    /// nothing. Off air, not at rest: every slot runs at the room's tempo so
    /// that its cell is a preview rather than a still (ADR-0269).
    Allocated,
}

impl Tally {
    /// **Every residency there is**, so that anything which has to hold all
    /// three cannot be given two.
    ///
    /// One customer today and it is [`mixer`], which measures the chip against
    /// the widest word rather than the current one. A `match` cannot express
    /// *the widest of them*, and a list written at the call site would be a
    /// second list of the residencies — the exact drift a fourth variant would
    /// walk straight past. Here it is one line under the enum, and a fourth
    /// variant that is not added to it is a `[Tally; 3]` that no longer
    /// compiles.
    pub const ALL: [Tally; 3] = [Tally::Live, Tally::Priming, Tally::Allocated];

    /// The mock's own word, lower-case here and upper-cased at paint time
    /// because `.tally` is `text-transform: uppercase` — the rule
    /// [`Kind::Bay`]'s title states.
    pub fn word(self) -> &'static str {
        match self {
            Tally::Live => "live",
            Tally::Priming => "prim",
            Tally::Allocated => "alloc",
        }
    }
}

/// **What shape of the frame a layer reaches**: the second `.mini` in a strip,
/// and `karakuri_engine`'s `MaskKind` — three kinds and no fourth.
///
/// # It is a mark rather than a word, and the mark is drawn rather than typed
///
/// The mock writes `&#9711;` and `&#9681;` — a circle, and a circle with one
/// half filled — and it has to: a strip is 53 wide inside its padding, and
/// `over` beside `linear` is 75 before either mini's border. So the mask says
/// its state in a shape.
///
/// **Drawn rather than set as a glyph**, which is [`grip_dots`]'s argument one
/// control along: whether `◯` and `◑` are in `egui`'s default face is a
/// question with no good answer, and a circle is the same mark either way. It
/// also settles the third one. The mock names `◯` in its own tooltip —
/// *"Mask: none"* — and draws `◑` on the strip beside it without saying which
/// kind that is, and it draws no third mark anywhere; a glyph for the third
/// would be a character picked out of a font, where a **mark** is the shape
/// the mask makes and can be argued from the mask.
///
/// # There is no `ALL` beside it, where [`Tally`] and `BlendMode` have one
///
/// Both of those constants exist for a **reader**: `Tally::ALL` is what
/// [`mixer`] measures the tally capsule against, because a `match` cannot
/// express *the widest of them*, and `BlendMode::ALL` is what a map file is
/// offered. Nothing measures this chip against its three shapes — it holds a
/// mark rather than a word and is [`size::MINI_SIZE`] wide whichever shape it
/// is showing — and no map target names a shape, which is why
/// `karakuri_operation::WipeKind` has no `ALL` either and says so at
/// [`WipeKind::name`]. A list written here would be a second statement of the
/// order with no reader, and the order is [`next_shape`]'s: a `match`, so a
/// fourth shape does not compile until somebody says what follows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mask {
    /// No mask: the layer reaches the whole frame. The mock's `◯`.
    None,
    /// A straight edge across the frame — the mock's `◑`, which is what a hard
    /// edge down the middle of a circle looks like.
    Linear,
    /// A circle. The same outline with a filled centre, which is the same
    /// family as the mock's two and is the shape a radial mask makes.
    Radial,
}

/// **What a slot's meter last read**, which is two of the four numbers
/// `karakuri_engine::meter::Level` carries.
///
/// Plain numbers, for [`Transport`]'s reason: this crate takes no engine
/// (ADR-0156), so whoever owns the deck reads a level and writes this.
///
/// # What is left out, and why each
///
/// - **`frames_behind`.** `Deck::level` reads back without ever waiting, so a
///   reading is a few frames old and the engine says by how many. It is not
///   carried, and not because it does not matter: the engine's own
///   measurements are **1** frame behind under pacing, *"2 or 3"* on a vsync
///   window, and *"tens: 13 to 101"* with nothing pacing the loop at all — and
///   all three are normal. A staleness threshold picked in this crate would
///   blank a healthy meter on one loop and pass a dead one on another, because
///   the console does not know which loop it is on. What the meter draws is a
///   fact about a frame that has already been drawn, exactly as
///   [`Transport::frame_ms`] is, and it does not go stale by standing still
///   the way a *rate* does — which is the whole of why `frame_ms` is a number
///   and `fps` is an `Option`. So **handing over no reading at all is the
///   caller's decision**, on the same terms `fps: None` is, and the caller is
///   the one thing that knows its own loop.
/// - **`bad_texels`.** It is the denominator the mean was taken over rather
///   than a reading — *"`mean` and `peak` are computed over the texels this
///   did not count"* — and the mock's 6px meter has nowhere to say it. A meter
///   that drew it would be reporting on the measurement instead of on the
///   image.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Level {
    /// Mean luminance over the whole frame — *"the figure to match faders
    /// on"*, and what the meter's column is as tall as.
    pub mean: f32,
    /// The brightest single texel. What the peak mark sits on.
    pub peak: f32,
}

/// **What one mixer strip reads this frame**: what the deck is playing, where
/// the slot sits, and the four numbers in front of it.
///
/// # The same seam as [`Transport`], and it is not crossed
///
/// Every field is a string, a number or a word, and none of them is a `Deck`.
/// `src/` takes no device and no engine (ADR-0156), so whoever owns the deck
/// reads `Deck::residency`, `gain`, `opacity`, `blend`, `mask` and `level` and
/// writes one of these per slot per frame — exactly as whoever owns the device
/// registers a texture and writes [`View::picture`].
///
/// **No repaint arm for the values arriving**, which is [`Transport`]'s
/// reason and **not the picture's**: these move when the engine draws a
/// frame, and the engine draws a frame on the frames this panel is drawn on —
/// so a value that has arrived is a value already on screen, whether or not
/// there is a picture two bays along asking for frames of its own. The one
/// that has to be said out loud is [`Strip::level`], because it moves on
/// every one of those frames and declares nothing: see
/// [`View::mixer_declares`], where that is decided, and
/// [ADR-0290](../../../../docs/adr/0290-the-level-meter-moves-only-when-a-frame-is-drawn-so-it-declares-nothing.md).
///
/// **A drag on one of the two faders is a different thing and does have an
/// arm** — [`crate::repaint::Change::Emitted`]. That is not the value
/// arriving; it is an operation leaving, on a gesture an operator is making,
/// and it is on the list because [`crate::repaint::Change`] is one list of
/// everything that can change what the console shows and a fader that reached
/// no repaint would leave the strip drawn at the value before the drag.
///
/// # Five of these are controls and the rest are readouts, and the source
/// says which
///
/// **The two faders are played.** A press on the trim's knob or the fader's
/// knob takes it in hand ([`Mixer::grab`]), and dragging it emits
/// [`Operation::SetGain`] or `SetOpacity` for the caller to turn into a
/// record. Nothing here applies it, and nothing here keeps the value: the
/// fields below are still written by whoever owns the deck, every frame, so a
/// fader that has just been dragged shows the new number **because the deck
/// changed**.
///
/// **The blend chip is the third, and it cycles.** A press on it emits
/// [`Operation::SetBlendMode`] naming the mode after this one ([`Mixer::blend`]),
/// and nothing here applies that either. It is one control emitting three
/// operations, which is the affordance P-0090 leaves to whoever draws the
/// control — see
/// [ADR-0187](../../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md).
///
/// **The tally chip is the fourth, and it cycles too.** A press on it emits
/// [`Operation::SetResidency`] naming the next of the three ([`Mixer::tally`])
/// — counted from [`Strip::requested`] rather than from [`Strip::tally`],
/// which is what makes a press on a parked chip the withdrawal of its own
/// prime request without a case in the code for it
/// ([ADR-0195](../../../../docs/adr/0195-the-tally-chip-cycles-from-the-request-so-the-parked-case-needs-no-case.md)).
///
/// **The mask mini is the fifth, and it cycles too.** A press on it emits
/// [`Operation::SetMaskShape`] naming the shape after this one ([`Mixer::mask`])
/// — **and the angle this strip is already wearing**, which is
/// [`Strip::mask_angle`]: the chip chooses a shape and the angle is not its
/// business, so it hands back the one it was given rather than a default that
/// would straighten a diagonal front
/// ([ADR-0203](../../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)).
///
/// **Everything else in the bay is a readout.** The meter and the number are
/// drawn from what the deck says and a press on either reaches nothing — and
/// so does a press on a fader's *track*, off the knob, which would otherwise
/// be a jump nobody asked for. `tests/mixer.rs` asserts both directions rather
/// than leaving either to be inferred from the absence of a hit test.
///
/// **[`Strip::gain_to`] and [`Strip::opacity_to`] are readouts too**, and the
/// order is the tally's: the roll was drawn a commit before the chip became a
/// control, because a control that could not yet say it was pending would look
/// dead for exactly as long as that commit. Nothing here schedules a move,
/// nothing here cancels one, and no press on a mark reaches anything — a
/// scheduled fade is armed from a key, a map or an MCP call, and what this
/// strip owes it is that it is not invisible until it happens
/// ([ADR-0206](../../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)).
#[derive(Debug, Clone, PartialEq)]
pub struct Strip {
    /// **What the deck is playing**, in `.strip-name`.
    ///
    /// A `String`, and **the harness's word rather than the engine's**:
    /// nothing reachable from a `Deck` carries a name for the material in a
    /// slot. `Deck::slot` hands out a `HotSwap`, `HotSwap::set` hands out a
    /// `Set`, and a `Set` names its *nodes* (`Set::node_names`) and its
    /// *published controls* (`Published::name`) and has no name of its own —
    /// which is right, because a Set is built from a list of `.kir` files and
    /// only whoever passed that list knows what to call the result. So the
    /// caller names its own material, and a name invented in `src/` would be a
    /// label the console made up about somebody else's Set.
    ///
    /// Elided rather than wrapped where it does not fit, which is
    /// `.strip-name`'s own `overflow: hidden; text-overflow: ellipsis;
    /// white-space: nowrap`: a strip is 53 wide inside its padding and most
    /// real names are wider. An empty string draws no name at all, which is a
    /// harness with nothing to say rather than a strip with nothing in it.
    pub name: String,
    /// Where the slot sits — `Deck::residency`, which is the **effective**
    /// residency and not `requested_residency`. The governor moves a slot down
    /// without anybody asking it to, and a tally that did not follow it would
    /// be showing what was asked for over a slot doing something else.
    ///
    /// **What the chip draws, and not what a press on it counts from.**
    /// [`Mixer::tally`] steps from [`Strip::requested`]: the word says where
    /// the deck *is*, and the cycle is about what was last asked for. The two
    /// are the same value on every settled slot and they part exactly where it
    /// matters — see [`Mixer::tally`] for the parked case, which is the whole
    /// argument.
    pub tally: Tally,
    /// **What was asked for** — `Deck::requested_residency`, the other half of
    /// the pair [`Strip::tally`] is one of, and **what a press on the chip
    /// counts from** ([`Mixer::tally`]).
    ///
    /// # Why the strip carries both and derives nothing else
    ///
    /// [P-0087](../../../../docs/principles/0087-name-the-property-never-the-shape.md)
    /// asks a pending control to say three things — where it is, where it is
    /// going, and that it has not arrived — and the first two *are* these two
    /// values. The third is [`Strip::pending`], which is a comparison of them.
    ///
    /// **There is deliberately no `parked: bool` beside them.** A third field
    /// would be the same fact stored twice, and the copy is the one that goes
    /// stale: the harness could write a `tally` and a `requested` that
    /// disagree and a `parked` that says they do not, and nothing in this
    /// crate could tell. One derivation with several readers is this crate's
    /// habit — [`StripBox::fader_at`] is the same shape — and it is P-0087's
    /// *a pending state is derived every frame* read one level down, in the surface
    /// rather than in the engine.
    ///
    /// **Two fields rather than one `Tally` grown into a pair.** `Tally` is
    /// the mock's `.tally` and the engine's `Residency`, and both of those
    /// name *one* place a slot can sit; a variant meaning "allocated, and
    /// asked to prime" would be a fourth residency in a type whose own
    /// documentation says there are three and no fourth. The relation between
    /// two residencies is not a residency.
    pub requested: Tally,
    /// **The trim** — `Deck::gain`: linear, floored at zero, and deliberately
    /// open above 1.0 because the mix is HDR.
    ///
    /// Its own field and not the fader's twin. *"`gain` is a trim and
    /// `opacity` is the fader"*: opacity at zero silences under every blend
    /// mode and gain at zero does not silence `over`, which
    /// `karakuri-engine/src/mix.rs` states outright — so these are two
    /// controls, and the bay draws them as two rather than as two identical
    /// sliders. A `g` and a mini fader lying down against the tall one in the
    /// middle of the strip.
    pub gain: f32,
    /// **Where a scheduled move is taking the trim**, or `None` for a trim
    /// nothing is moving — `Deck::transitions_on(slot)`'s
    /// `Control::Gain` entry, and its `Transition::to`.
    ///
    /// # It is the destination and nothing else about the move
    ///
    /// A `Transition` also carries the instant it starts, how long it lasts
    /// and the curve it takes. None of the three is here, and the reason is
    /// the same seam every other field on this type is on: **the console has
    /// no beat count**, so a start in beats and a length in beats are two
    /// numbers it could not turn into anything a strip draws. What it can draw
    /// is where the control is going, which is the second of the three things
    /// P-0087 asks for and the whole of what the fader is being asked to say.
    ///
    /// **Armed and running are the same state here, deliberately.** A
    /// transition is in the deck's list from the moment it is scheduled until
    /// the beat it finishes on, and `Deck::transitions_on` answers with it
    /// throughout — so a fade quantised to the next bar and a fade halfway
    /// through are both *a request that has not arrived*, which is exactly the
    /// relation P-0087 is about. What separates them on the surface is that
    /// the value under the knob is moving in the second case, and the mark
    /// stands still in both.
    ///
    /// **One per control, because the engine allows one**: `Deck::schedule`
    /// cancels whatever was moving that pair before pushing, so a harness that
    /// finds two has read the wrong slot.
    ///
    /// # What it cannot say, and it is the trim's existing gap
    ///
    /// The track shows `[0, 1]` and the mix is HDR, so a move from 1.5 to 2.0
    /// is two values the track draws in one place — see [`StripBox::trim_at`],
    /// where that gap is already written down. [`Strip::gain_pending`] answers
    /// `None` there rather than declaring a staleness for an animation that
    /// would not move a pixel.
    pub gain_to: Option<f32>,
    /// **The fader** — `Deck::opacity`: a proportion in `[0, 1]`, and the one
    /// control that silences a slot under every blend mode, which is what
    /// makes it the way out of material that has gone NaN.
    pub opacity: f32,
    /// **Where a scheduled move is taking the fader**, or `None` for a fader
    /// nothing is moving — `Deck::transitions_on(slot)`'s `Control::Opacity`
    /// entry, on [`Strip::gain_to`]'s terms and for its reasons.
    ///
    /// # And the third control has nowhere to say it
    ///
    /// `Control` has three members and this strip carries two of them.
    /// `Control::MaskPosition` — *how far a mask's front has travelled* — is
    /// the one a wipe is made of, and **the strip has no control and no
    /// readout for it at all**: the mask `.mini` names a *shape*, and the
    /// position, the angle and the softness are an inspector row that does not
    /// exist yet ([`Strip::mask`] says so, and [`Strip::mask_angle`] is the
    /// same fact from the other side). A destination carried here for it would
    /// be a number with nowhere to be drawn, and drawing it on the shape chip
    /// would say a shape was changing when none is.
    ///
    /// So an armed wipe is invisible on this panel, it is an under-draw rather
    /// than a decision that it does not matter, and it is
    /// [ADR-0206](../../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)'s
    /// named consequence: what closes it is a mask-position control, and it
    /// closes the same day that control lands.
    pub opacity_to: Option<f32>,
    /// **The blend in force**, as one of the vocabulary's three — and the word
    /// drawn on the chip is [`BlendMode::name`].
    ///
    /// # Why this is not the engine's word
    ///
    /// It was a `&'static str` — `Blend::name`, handed straight through, on
    /// the argument that the console has nothing to do with the blend but draw
    /// the engine's word. **That stopped being true when the chip became a
    /// control** ([ADR-0187](../../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md)):
    /// to say what the *next* mode is, [`Mixer::blend`] has to know which of
    /// the three this one is, and a string it cannot exhaustively match is the
    /// wrong carrier for that. A `&str` would make the chip's arithmetic a
    /// comparison against three literals with a fourth case that has no
    /// answer.
    ///
    /// **So the type is the vocabulary's, and the harness converts.**
    /// `karakuri-console` already depends on `karakuri-operation`, and
    /// `crates/karakuri/src/main.rs` turns `Deck::blend`'s `karakuri_engine::deck::Blend`
    /// into one of these with a `match`. The failure that buys is worth
    /// stating: a fourth engine blend mode with no operation variant stops
    /// compiling **at the harness**, rather than drawing a word on a chip no
    /// control can reach and no map can ask for. That is the vocabulary doing
    /// its job — P-0090's price, paid rather than avoided: a vocabulary that
    /// names a destination has to own the lists a destination is drawn from,
    /// or it is back to toggles (ADR-0180).
    ///
    /// The mock's own tooltip lists four — *"add, over, screen, multiply"* —
    /// and there are three. The disagreement was the mock's to settle and
    /// [ADR-0187](../../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md)
    /// settled it: `docs/manual/console.html` now lists the three that exist.
    pub blend: BlendMode,
    /// The mask in force — `Deck::mask(slot).kind()`. Its angle, position and
    /// softness are not drawn: `.mini` is a chip that says *which shape*, and
    /// three numbers about that shape are the inspector's row, not this one.
    ///
    /// **What a press on the chip counts from** ([`Mixer::mask`]), the way
    /// [`Strip::requested`] is what the tally's press counts from.
    pub mask: Mask,
    /// **The angle the mask is already wearing** — `Deck::mask(slot).angle()`,
    /// in radians. **Read to build an operation, and drawn nowhere.**
    ///
    /// # Why the strip carries a number no part of it paints
    ///
    /// [`Operation::SetMaskShape`] carries a shape **and an angle**, because
    /// the vocabulary's row is one an operator can say the whole of and a
    /// record is written whole
    /// ([ADR-0201](../../../../docs/adr/0201-the-mask-is-two-rows-because-a-control-change-can-only-set.md)).
    /// The chip names the shape and nothing on this strip names the angle — so
    /// a press has to carry a value the control does not control, and the only
    /// honest one is **the value it already has**. Sending `0.0` would make
    /// choosing a shape silently straighten a diagonal wipe: a press that
    /// changed something nobody asked it to, which is the class of failure
    /// [ADR-0192](../../../../docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)
    /// and ADR-0201 are both about, arriving one layer further out.
    ///
    /// **It is [`Strip::requested`]'s arrangement exactly**: a value the strip
    /// carries because a press has to be computed from it, written by whoever
    /// owns the deck like every other field here, and never a value this crate
    /// keeps. See
    /// [ADR-0203](../../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md).
    ///
    /// **Not the position and not the softness**, which the record also
    /// carries: neither is in the operation at all, and the half a shape
    /// operation does not ask for is filled in from a reading of the mask that
    /// is running, where the record is written — `karakuri_operation_record`'s
    /// `Current`. The angle is here because the *operation* names it; those
    /// two are not, because it does not.
    pub mask_angle: f32,
    /// **What the slot's meter last read**, or `None` for no reading at all.
    ///
    /// `Deck::level` is already `None` for *no meter, no measurement yet, a
    /// slot that is neither Live nor being auditioned, and a slot whose
    /// material was just replaced by a resize or a swap* — every case where a
    /// held reading would be about a different image. So `None` here draws the
    /// meter's well and nothing in it, which is the mock's own `alloc` strip:
    /// a `.vmeter` with no `b` and no `u` inside it.
    ///
    /// **The one value on a strip that moves without a hand on anything, and
    /// it declares nothing** — decided rather than missed, in
    /// [`View::mixer_declares`].
    pub level: Option<Level>,
}

impl Strip {
    /// **Where this slot has been asked to go and has not got to**, or `None`
    /// for a slot that is where it was asked to be.
    ///
    /// The one derivation the pending presentation reads, and it answers a
    /// *word* rather than a `bool` because that is what the surface has to
    /// draw: the second of P-0087's three is where the control is going, which
    /// ADR-0188 states as *identifiable from the surface itself* — so the thing
    /// worth deriving is the destination
    /// and not the fact that there is one.
    ///
    /// # Why it is an inequality and not the engine's pair
    ///
    /// `Deck::is_parked` is exactly `requested == Residency::Priming &&
    /// effective == Residency::Allocated`, and today this answers `Some` on
    /// precisely those slots — `deck.rs`'s module doc closes the other cases
    /// itself: *"the governor may hold a slot below what was asked for, and
    /// may never put one above it"*, and *"effective Live and requested Live
    /// are the same set of slots"*. So the two forms agree slot for slot, and
    /// there is no frame on which they differ.
    ///
    /// They differ in what a **second** kind of disagreement would do to them.
    /// Written as the engine's pair, a surface matching `Priming` over
    /// `Allocated` draws a settled chip over any other outstanding request —
    /// an under-draw, silent, and exactly the failure the rule exists to name.
    /// Written as an inequality it draws the new one without being taught,
    /// because the presentation was never about *parked*: it is about a
    /// request that has not landed, which is
    /// [P-0087](../../../../docs/principles/0087-name-the-property-never-the-shape.md)
    /// — the property rather than the one shape it currently takes.
    ///
    /// **`park` is still the word**, and it is the status line's: `karakuri-cli`
    /// spells it out for an operator in prose, which is the same clause met by
    /// a different means (ADR-0188).
    pub fn pending(&self) -> Option<Tally> {
        match self.requested == self.tally {
            true => None,
            false => Some(self.requested),
        }
    }

    /// **Where the trim is going and has not got to**, or `None` for a trim
    /// that is where it has been asked to be.
    ///
    /// [`Strip::pending`] one control along, and the same shape: the field is
    /// the request, the derivation is the third clause, and nothing is stored
    /// that says *a move is pending* — that fact is `self.gain_to` against
    /// `self.gain`, read every frame off values the harness rewrote this
    /// frame.
    ///
    /// # It compares what the track can show, and that is not the same as the
    /// raw pair
    ///
    /// The trim draws `[0, 1]` and gain is open above unity, so a scheduled
    /// move from 1.5 to 2.0 is two values with **one position** on this track
    /// ([`StripBox::trim_at`]). Compared raw it is pending; compared through
    /// [`unit`] it is not, and this compares through [`unit`].
    ///
    /// The reason is [`View::animating`] rather than the mark: a `Some` here
    /// declares a staleness, and a staleness bought for an animation that
    /// cannot move a pixel is the cost ADR-0193 refused to pay for a bay
    /// nobody can see, arriving through the other door. **It is still an
    /// under-draw and it is the trim's existing gap, not a new one** — the
    /// same track that cannot show 1.0 against 3.0 cannot show a move between
    /// them, and both close together, on the pass that lets a hand push the
    /// control past the top.
    ///
    /// A destination that is not a number is zero, which is [`unit`]'s rule
    /// and is where a NaN out of an engine stops being a `NaN`-wide band.
    pub fn gain_pending(&self) -> Option<f32> {
        self.gain_to
            .filter(|to| unit(*to) != unit(self.gain))
            .map(unit)
    }

    /// **Where the fader is going and has not got to**, on
    /// [`Strip::gain_pending`]'s terms.
    ///
    /// Opacity is a proportion, so the clamp is the identity on every value a
    /// deck can hold and the comparison is the plain one. It is written
    /// through [`unit`] anyway, because a harness writing a destination the
    /// engine never held is the case both of these are the last line of
    /// defence for — and two derivations that agree except in the case nobody
    /// tests are worse than one shape written twice.
    pub fn opacity_pending(&self) -> Option<f32> {
        self.opacity_to
            .filter(|to| unit(*to) != unit(self.opacity))
            .map(unit)
    }
}

/// **A fader, laid out**: the track, the length of it the value fills, and the
/// knob sitting on the value.
///
/// One type and one derivation for both of a strip's faders — see [`fader`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fader {
    /// The well: `.fader`'s 5px capsule lying down, or `.vfader`'s 17px one
    /// standing up.
    pub track: Rect,
    /// Which way it runs. [`Axis::Row`] fills from the left and
    /// [`Axis::Column`] fills from the **bottom**, because a fader stands up.
    pub axis: Axis,
    /// What the value fills, from the track's own zero.
    pub fill: Rect,
    /// The knob, centred on the fill's moving edge.
    pub knob: Rect,
    /// **How far the knob's centre moves between the two ends**: the track
    /// less the inset the fill sits inside it by, along the axis.
    ///
    /// A field, and not a fill stored beside its track — the thing
    /// [`StripBox`] refuses. The fill says where the value *is* and says
    /// nothing about how far it could go; the zero end is recoverable from the
    /// fill (a row's `fill.min.x`, a column's `fill.max.y`, neither of which
    /// moves) and the length is not recoverable from anything here. It is what
    /// turns a pointer back into a value — see [`Mixer::grab`] — so the
    /// derivation that draws the fader and the one that grabs it are one.
    pub travel: f32,
}

/// **A meter, laid out**: the well, the column the mean fills, and the peak's
/// mark.
///
/// # It is not a [`Fader`], and the difference is not the handle alone
///
/// They share exactly one thing and it is [`filled`] — a value turned into a
/// length up a track — which is why that is a function of its own with three
/// call sites rather than a shared type. What they do not share is what each
/// **is**: a fader's knob is a grab target and [`Mixer::grab`] is what makes
/// it one, while a peak mark is a reading and never will be. A shared type
/// would be a type half of whose fields are about a gesture the other half can
/// never have — and [`Fader::travel`], which is the length a *hand* moves the
/// value along, is the field that says so.
///
/// They disagree about the track as well. `.vfader b` sits
/// [`size::VFADER_INSET`] inside its well and `.vmeter b` fills its own edge
/// to edge; `.vfader`'s fill is a capsule and `.vmeter` is `overflow: hidden`
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

/// **A scheduled move on a fader, laid out**: the mark on the destination, and
/// how far this frame's attempt to get there has reached.
///
/// The presentation
/// [P-0087](../../../../docs/principles/0087-name-the-property-never-the-shape.md)
/// is met with on a track, decided in
/// [ADR-0206](../../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md):
/// **the knob and the fill go on saying where the control is, a mark says
/// where it is going, and the fill keeps setting off toward the mark and
/// falling back**. Nothing about the value moves, which is a stricter first
/// clause than the tally's roll manages — the word there is lifted off its
/// centre line and this is not.
///
/// # Two rectangles and no phase
///
/// It is a [`Fader`]'s neighbour and it is measured the same way: a pure
/// function of the two values and the displacement, with no clock and no
/// `Phase` in it — [`roll_at`] is applied by whoever is painting, exactly as
/// it is for the tally's word. A test asks for a reach at a displacement it
/// chose.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reach {
    /// **The destination**, as a hairline across the track at the position the
    /// knob would sit at if the move had landed.
    ///
    /// **As long as the knob is wide**, so it is the knob's own footprint
    /// reduced to a line — and so it always has the same two pixels of strip
    /// to be read against that the knob stands proud on
    /// ([`size::VFADER_KNOB_OUT`]), whatever the fill is doing underneath. A
    /// mark the width of the *track* would cross the fill's own lavender end
    /// in its own colour on a move down from the top.
    ///
    /// It is painted **over** the knob rather than under it. Where the two
    /// nearly coincide the knob would otherwise swallow it — 9 pixels of knob
    /// on a 98-pixel travel is every move under about a tenth of the fader,
    /// and a quarter of the trim — and a mark that disappears exactly when the
    /// control is nearly there says *arrived* at the one moment it must not.
    pub mark: Rect,
    /// **What the reach has covered at this displacement**: from the value's
    /// own edge toward [`Reach::mark`], and never as far as it — [`ROLL_REACH`]
    /// of the way at the top of the travel, nothing at rest.
    ///
    /// Across the fill's own width rather than the track's, because it is the
    /// fill reaching. Empty at rest, which is a rectangle with no area and
    /// nothing painted.
    pub band: Rect,
}

/// **One strip's furniture**: a rectangle for each of the six things stacked
/// in it, and the two tracks a value rides.
///
/// The fills and the knobs are **not** fields, because each is a function of a
/// value this already knows where to put — see [`StripBox::trim_at`],
/// [`StripBox::fader_at`] and [`StripBox::meter_at`]. A fill stored beside its
/// track is two statements about one number.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StripBox {
    /// `.strip` itself: the 9px well everything else is inside.
    pub rect: Rect,
    /// `.strip-name`, the full width of the strip's content box because the
    /// CSS says `width: 100%`.
    pub name: Rect,
    /// `.tally`'s capsule, as wide as **the widest** of the three residency
    /// words inside its padding — the same box whichever one it is showing,
    /// so the chip does not resize when the deck moves and does not resize
    /// under a word rolling through it. See [`mixer`], where it is measured.
    ///
    /// The word is centred in it ([`tally_into`]), and the capsule is centred
    /// in the strip, so widening the box does not move the word: it grows
    /// symmetrically around type that was already on the strip's centre line.
    ///
    /// **It is the chip a press acts on** and not only the box a word is
    /// painted into — [`Mixer::tally`] hit-tests exactly this rectangle, the
    /// way [`StripBox::blend`] is hit-tested. Being the widest word's width
    /// rather than the shown word's is what the blend chip cannot say: this
    /// target stands still while the deck moves under it and while a word
    /// rolls through it.
    pub tally: Rect,
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
    /// The blend `.mini`, which is **the chip a press acts on** and not only
    /// the box a word is painted into — [`Mixer::blend`] hit-tests exactly
    /// this rectangle. As wide as the word in it, inside `.mini`'s padding and
    /// border.
    pub blend: Rect,
    /// The mask `.mini`, which is **the chip a press acts on** and not only
    /// the box a mark is drawn into — [`Mixer::mask`] hit-tests exactly this
    /// rectangle, the way [`StripBox::blend`] and [`StripBox::tally`] are.
    /// It holds a mark rather than a word, so it is the same width whichever
    /// shape it is showing: a target that stands still while the deck moves
    /// under it, which the tally's capsule buys by being the widest word's and
    /// the blend chip cannot say at all.
    pub mask: Rect,
}

impl StripBox {
    /// **The trim at a gain**, which is [`fader`] on the horizontal track.
    ///
    /// The gain is shown over `[0, 1]`, and that range is read off
    /// `karakuri-midi`'s `GAIN_RANGE` rather than chosen here: *"`[0, 1]` for
    /// gain even though the mix is HDR and values above 1.0 are ordinary …
    /// a fader whose top is unity is what a fader means."* So a gain pushed
    /// past unity fills the track and stops, and this bay cannot yet show the
    /// difference between 1.0 and 3.0 — a real gap, and it belongs with the
    /// pass that lets a hand push the control past the top.
    pub fn trim_at(&self, gain: f32) -> Fader {
        fader(
            self.trim,
            Axis::Row,
            gain,
            0.0,
            egui::vec2(size::FADER_KNOB_W, size::FADER_KNOB_H),
        )
    }

    /// **The fader at an opacity**, which is [`fader`] on the vertical track.
    pub fn fader_at(&self, opacity: f32) -> Fader {
        fader(
            self.fader,
            Axis::Column,
            opacity,
            size::VFADER_INSET,
            egui::vec2(size::VFADER_KNOB_W, size::VFADER_KNOB_H),
        )
    }

    /// **The trim's scheduled move**, from the gain it is at to the one it was
    /// asked for, `rolled` of the way — which is [`reach`] on the horizontal
    /// track.
    ///
    /// Off [`StripBox::trim_at`] twice rather than off any arithmetic of its
    /// own: the destination's mark is *where the knob would be*, so it is the
    /// same derivation asked a second question, and a mark that drifted from
    /// the knob it stands for would be a fader with two ideas of what a value
    /// looks like.
    pub fn trim_reach(&self, gain: f32, to: f32, rolled: f32) -> Reach {
        reach(self.trim_at(gain), self.trim_at(to), rolled)
    }

    /// **The fader's scheduled move**, on [`StripBox::trim_reach`]'s terms and
    /// off [`StripBox::fader_at`].
    pub fn fader_reach(&self, opacity: f32, to: f32, rolled: f32) -> Reach {
        reach(self.fader_at(opacity), self.fader_at(to), rolled)
    }

    /// **The meter at a reading.**
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

/// **The mixer's strips, laid out**: the values, and where each one goes.
///
/// # It borrows the values rather than carrying a copy
///
/// [`TransportRow`] carries the [`Transport`] it was measured from, for
/// [`Picture`]'s reason: whoever measured the type and whoever paints it are
/// then one statement, so a row laid out for one value and painted with
/// another cannot be written by accident. A [`Strip`] carries a name, so it is
/// not `Copy` and a copy per frame would be a `String` allocated per strip per
/// frame. The borrow says the same thing for nothing — these boxes were
/// measured from *these* strips — and the compiler holds it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mixer<'a> {
    /// **The values these rectangles were measured from**, in slot order.
    pub strips: &'a [Strip],
    /// Where each of them is. `None` past the last strip: a track on the page
    /// no deck fills, which is drawn as nothing at all.
    boxes: [Option<StripBox>; DECKS],
}

impl<'a> Mixer<'a> {
    /// How many strips there are — the deck's slot count, as far as one page
    /// of this bay reaches.
    pub fn count(&self) -> usize {
        self.boxes.iter().filter(|at| at.is_some()).count()
    }

    /// One strip's furniture. Panics on a strip this bay has not got, which is
    /// a caller having invented a slot — the rule [`TransportRow::dot`] states
    /// about a dot of a grid.
    pub fn strip(&self, index: usize) -> StripBox {
        let count = self.count();
        self.boxes
            .get(index)
            .copied()
            .flatten()
            .unwrap_or_else(|| panic!("strip {index} of a mixer of {count}"))
    }

    /// **What a press at `p` takes hold of**, or `None` where there is nothing
    /// under it that a hand can move.
    ///
    /// # It is the knob, and the track is deliberately not a target
    ///
    /// **A press on the track, off the knob, does nothing.** A fader at 0.3
    /// whose top is clicked would jump to 1.0 — a change to the mix nobody
    /// asked for, made on stage — and this bay's controls are played during a
    /// performance. So the answer is `None` there, and it is a decision rather
    /// than a hit test that stops at the knob by accident.
    ///
    /// It also decides who the *event* belongs to, since
    /// [`crate::input::claim`]'s rule 3 asks this: **the panel claims what it
    /// acts on**, so a press on a track goes to `egui`, which owns no widget
    /// there and does nothing with it — which is the same nothing, arrived at
    /// without the panel claiming a press it would throw away.
    ///
    /// # One derivation, asked twice
    ///
    /// [`crate::input::claim`] asks this and so does the caller that acts on
    /// the press, exactly as [`Outputs::op`] is asked after [`Outputs::hit`]
    /// (ADR-0176). Two copies of *where the knob is* is a control drawn where
    /// it cannot be grabbed, with nothing on screen saying so.
    ///
    /// **The value is part of the geometry here**, which the Outputs chip does
    /// not have to deal with: the knob sits on the fill's moving edge, so
    /// where it is depends on what the deck said this frame. That is the same
    /// [`Strip`] the bay was laid out from, so the knob a hand sees and the
    /// knob it grabs are the same one.
    pub fn grab(&self, p: karakuri_layout::Point) -> Option<Grab> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (strip, at))| {
                let at = at?;
                // The manual's *deck* is the code's *slot*, and a deck holds
                // `MAX_SLOTS` of them — `DECKS`, which is 4 — so the index is
                // a `u8` with room to spare. See `Knob::operation`.
                let deck = index as u8;
                grabbed(at.trim_at(strip.gain), Knob::Trim { deck }, p)
                    .or_else(|| grabbed(at.fader_at(strip.opacity), Knob::Fader { deck }, p))
            })
    }

    /// **What a press at `p` asks the blend to become**, or `None` where
    /// there is no blend chip under it.
    ///
    /// # The chip cycles, and the operation names where it arrived
    ///
    /// Click it and the deck's blend moves to the next of
    /// [`BlendMode::ALL`], wrapping from the last back to the first. What
    /// comes out is [`Operation::SetBlendMode`] naming the **destination** —
    /// never a step, because there is no step in the vocabulary to name.
    ///
    /// That is the affordance P-0090 leaves to whoever draws the control
    /// rather than an exception to it: a toggle is built over operations by
    /// whoever draws them, and a mini that cycles the blend is one control
    /// emitting three — the operator sees a toggle and the vocabulary never
    /// does. The cycle is [`after`], which is four lines in this file and
    /// nothing at all in `karakuri-operation`.
    ///
    /// **What a MIDI map is offered is the three values, not the cycle** —
    /// [ADR-0187](../../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md),
    /// which is the decision this affordance forces and the reason it is
    /// recorded at all.
    ///
    /// # One derivation, asked twice, and the whole chip is the target
    ///
    /// [`crate::input::claim`]'s rule 3 asks this and so does the caller that
    /// acts on the press — the arrangement [`Outputs::op`] and [`Mixer::grab`]
    /// both have, where the derivation that claims a press is asked again
    /// rather than copied. [`StripBox::blend`] is the chip's own rectangle,
    /// the one the word is painted into, so a chip a hand sees and a chip it
    /// clicks are the same one.
    ///
    /// **The whole chip is the target and not the glyphs in it**, which is
    /// [`Outputs::sink`]'s rule one bay along: a 15px word is not something a
    /// hand finds, and `.mini`'s padding is what makes it one.
    ///
    /// # It names the strip's own deck
    ///
    /// The slot index, cast the way [`Mixer::grab`] casts it — the manual's
    /// *deck* is the code's *slot* (ADR-0180), and a deck holds `MAX_SLOTS` of
    /// them, so the index is a `u8` with room to spare.
    pub fn blend(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (strip, at))| {
                at.filter(|at| at.blend.contains(p))
                    .map(|_| Operation::SetBlendMode {
                        deck: index as u8,
                        blend: after(strip.blend),
                    })
            })
    }

    /// **What a press at `p` asks the residency to become**, or `None` where
    /// there is no tally chip under it.
    ///
    /// # The chip cycles, and it cycles from what was *requested*
    ///
    /// Click it and the deck is asked for the next of [`Tally::ALL`] —
    /// `live`, `prim`, `alloc`, wrapping — and what comes out is
    /// [`Operation::SetResidency`] naming that **destination**. The
    /// affordance is the blend chip's ([`Mixer::blend`], ADR-0187) and so is
    /// the division it rests on: the cycle is [`next`] here and nothing at all
    /// in `karakuri-operation`, which is P-0090's division: a toggle is an
    /// affordance, built over operations by whoever draws the control.
    ///
    /// **The step is taken from [`Strip::requested`] and not from
    /// [`Strip::tally`]**, and that is the decision rather than a detail. The
    /// two disagree exactly while a request has not landed, and cycling from
    /// the request is what makes that case come out right **with no case in
    /// the code for it**: a parked slot's request is `Priming`, so the next is
    /// `Allocated` — which *is* the withdrawal of the prime request, said by
    /// the ordinary arithmetic. Cycling from the effective residency would
    /// answer `Live` there, and a press meant to take a request back would put
    /// the deck on air.
    ///
    /// That is what
    /// [P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
    /// permits a surface: *"a press on a control whose transition is pending
    /// may ask for the withdrawal"* — a control choosing which destination a
    /// press names, arrived at out of one rule rather than a branch, and
    /// **not** a lock. The chip refuses nothing; every request is handed over
    /// and the engine decides.
    ///
    /// It is also what `karakuri-cli`'s `w` already does: `toggle_priming`
    /// reads `Deck::requested_residency` to choose its direction, so a parked
    /// slot's `w` withdraws rather than re-asking. Two surfaces reading
    /// different halves of the pair would disagree about what a press means
    /// on exactly the slots where it matters.
    ///
    /// # One derivation, asked twice, and the whole chip is the target
    ///
    /// [`crate::input::claim`]'s rule 3 asks this and so does the caller that
    /// acts on the press — [`Outputs::op`], [`Mixer::grab`] and
    /// [`Mixer::blend`] are the same arrangement. [`StripBox::tally`] is the
    /// capsule the word is painted into, and it is **the widest of the three
    /// words whatever it is showing** ([`mixer`]), so this target does not
    /// move when the deck moves under it and does not move while a word is
    /// rolling through it — which the blend chip, sized to the word it shows,
    /// cannot say.
    ///
    /// # It names the strip's own deck
    ///
    /// The slot index, cast the way [`Mixer::grab`] and [`Mixer::blend`] cast
    /// it — the manual's *deck* is the code's *slot* (ADR-0180), and a deck
    /// holds `MAX_SLOTS` of them, so the index is a `u8` with room to spare.
    pub fn tally(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (strip, at))| {
                at.filter(|at| at.tally.contains(p))
                    .map(|_| Operation::SetResidency {
                        deck: index as u8,
                        residency: residency(next(strip.requested)),
                    })
            })
    }

    /// **What a press at `p` asks the mask's shape to become**, or `None`
    /// where there is no mask mini under it.
    ///
    /// # The chip cycles, and the operation names where it arrived
    ///
    /// Click it and the deck's mask moves to the next shape — none, linear,
    /// radial, wrapping — and what comes out is
    /// [`Operation::SetMaskShape`] naming that **destination**. The affordance
    /// is the blend chip's ([`Mixer::blend`], ADR-0187) and the tally's
    /// (ADR-0195), and so is the division under it: the cycle is
    /// [`next_shape`] here and nothing at all in `karakuri-operation`, which
    /// is P-0090's division: a toggle is an affordance, built over operations
    /// by whoever draws the control.
    ///
    /// **The order starts at `None`**, which is
    /// `karakuri_engine::deck::MaskKind::ALL`'s and is written down there:
    /// *"`None` first, because it is the default and a cycle should start
    /// where a slot starts."* This crate has no engine (ADR-0156), so the
    /// order is restated in [`next_shape`] rather than read from it.
    ///
    /// # It carries the angle it does not control, and that is the decision
    ///
    /// [`Operation::SetMaskShape`] is a shape **and an angle**, and this chip
    /// names only the shape — *"`.mini` is a chip that says which shape, and
    /// three numbers about that shape are the inspector's row, not this one."*
    /// So the angle handed back is [`Strip::mask_angle`], the one the slot is
    /// already wearing: a press chooses a shape and changes nothing else.
    /// Sending `0.0` would make choosing a shape straighten a diagonal front,
    /// which is a surface asking for a change nobody made — see
    /// [ADR-0203](../../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md).
    ///
    /// **The position and the softness are not here at all.** The operation
    /// does not name them, and the record that does is filled in from a
    /// reading of the running mask where the record is written (ADR-0201).
    /// A surface carrying a value no operation asks for would be this crate
    /// keeping half a deck.
    ///
    /// # One derivation, asked twice, and the whole chip is the target
    ///
    /// [`crate::input::claim`]'s rule 3 asks this and so does the caller that
    /// acts on the press — [`Outputs::op`], [`Mixer::grab`], [`Mixer::blend`]
    /// and [`Mixer::tally`] are the same arrangement. [`StripBox::mask`] is
    /// the chip's own rectangle, the one the mark is drawn into, and it is
    /// [`size::MINI_SIZE`] wide inside `.mini`'s padding whichever shape it is
    /// showing — so this target, like the tally's capsule and unlike the blend
    /// chip's, does not move under the value it draws.
    ///
    /// # It names the strip's own deck
    ///
    /// The slot index, cast the way [`Mixer::grab`], [`Mixer::blend`] and
    /// [`Mixer::tally`] cast it — the manual's *deck* is the code's *slot*
    /// (ADR-0180), and a deck holds `MAX_SLOTS` of them, so the index is a
    /// `u8` with room to spare.
    pub fn mask(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (strip, at))| {
                at.filter(|at| at.mask.contains(p))
                    .map(|_| Operation::SetMaskShape {
                        deck: index as u8,
                        kind: wipe_kind(next_shape(strip.mask)),
                        angle: strip.mask_angle,
                    })
            })
    }

    /// **Which deck a press at `p` selects**, or `None` where no strip is
    /// under it.
    ///
    /// # The strip is the control, and it is the last question this bay asks
    ///
    /// A strip's rectangle contains the trim, the fader, the two chips and the
    /// mask mini, so this would answer for a press on any of them if it were
    /// asked first. It is asked **last**: whoever routes a press tries the
    /// four questions that name something inside the column, and this is what
    /// is left over — a press on the strip's name, on its number, on the
    /// ground between its rows. That is the affordance `console.html` states
    /// — *"a press anywhere on a strip that no knob under the pointer
    /// claimed"* — and it is why the bay needs no sixth control drawn to carry
    /// it.
    ///
    /// **It is one of rule 4's probes like every other control here**, and it
    /// is asked twice for the same reason they are: once by
    /// [`crate::input::claim`] to decide the press is the panel's, and once by
    /// whoever acts on it. It went eight days claimed by nobody —
    /// `input::on_strip` asked the four inside the column and not this one, so
    /// the press reached `egui`, which owns no widget on the console and did
    /// nothing with it. `tests/mixer.rs` is what says otherwise now.
    ///
    /// [`Operation::SelectDeck`] writes no record and is the console's own
    /// pointer, so whoever emits it performs it: there is nothing on the deck
    /// for it to move. What it moves is [`View::selection`], which the ring
    /// above and the Library bay's pill both read.
    pub fn select(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.deck_at(p).map(|deck| Operation::SelectDeck { deck })
    }

    /// **Which deck a carry let go at `p` lands on**, or `None` where no strip
    /// is under it — which is a drop that asks for nothing at all
    /// ([`crate::panel::Released::Nowhere`]).
    ///
    /// # It is a release where every other question in this bay is a press
    ///
    /// [`Mixer::grab`] and the four beside it answer *what does a press here
    /// ask for*, and they are asked twice — once by [`crate::input::claim`] to
    /// decide whose event it is, and once by whoever acts on it. Nothing asks
    /// this one first: a carry already holds the pointer under `claim`'s rule
    /// 1, so there is no claim to decide, and what is left is the second ask
    /// on its own. What that buys is the destination resolved against the
    /// geometry the frame *last drew* rather than against the geometry the
    /// press was made on — the strips can have moved under a carry that took a
    /// boundary with it on the way, and where the row lands is where the strip
    /// is now.
    ///
    /// **The whole strip, where [`Mixer::select`] is what is left over.** The
    /// two answer off one derivation ([`Mixer::deck_at`]) and differ in
    /// nothing else, and that is the point: a press is offered the strip only
    /// after the knobs and chips inside it have declined, because a press on a
    /// knob is a press on that knob — but a Set let go over a knob is a Set
    /// let go over **that deck**, since none of the five controls is a place a
    /// Set could go instead. So this is asked of no other question first.
    ///
    /// **It reads no residency and no reading of any kind.** A drop on a live
    /// deck asks for the load and the instrument decides
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md),
    /// and `console.html`'s *"Nothing refuses it"*), so
    /// [`Strip::tally`] is not consulted here and a strip's values reach this
    /// only as the rectangle they were laid out into.
    ///
    /// **It is one of two answers now, and never both.** The four deck
    /// preview cells take the drop as well ([`ProgramBay::dropped`]), and the
    /// two bays are in two regions of the panel — so a point is inside one
    /// set of rectangles or the other or neither, and *at most one rectangle
    /// is marked* is a fact about where the pointer is rather than a rule
    /// either of them enforces
    /// ([ADR-0273](../../../../docs/adr/0273-the-carry-lands-on-two-sets-of-rectangles-and-wears-a-face.md)).
    pub fn dropped(&self, p: karakuri_layout::Point) -> Option<u8> {
        self.deck_at(p)
    }

    /// **Which strip's rectangle `p` is inside**, as a deck.
    ///
    /// Private, and the one derivation [`Mixer::select`] and
    /// [`Mixer::dropped`] both read: *which column is the pointer in* is one
    /// question, and two spellings of it would be a press that selects one
    /// deck and a drop that loads another from the same point. The manual's
    /// *deck* is the code's *slot* and a deck holds `MAX_SLOTS` of them, so
    /// the index is a `u8` with room to spare — [`Mixer::grab`]'s note.
    fn deck_at(&self, p: karakuri_layout::Point) -> Option<u8> {
        let p = Pos2::new(p.x, p.y);
        self.boxes
            .iter()
            .enumerate()
            .find_map(|(index, at)| at.filter(|at| at.rect.contains(p)).map(|_| index as u8))
    }

    /// **Where the deck selection's ring goes**, and `None` for a selection
    /// this bay has no strip for.
    ///
    /// The one box in this bay that is the strip's *whole* rectangle rather
    /// than something inside it, because what the selection addresses is the
    /// deck and not any one of the six readings in the column. Answered from
    /// the same `boxes` every other rectangle here comes off, so the ring is
    /// painted round the strip a press on that track would select
    /// ([`crate::input::claim`]) and never round a neighbour.
    pub fn selected(&self, deck: u8) -> Option<Rect> {
        self.boxes
            .get(usize::from(deck))
            .copied()
            .flatten()
            .map(|at| at.rect)
    }

    /// Every strip and its box, in slot order.
    pub fn placed(&self) -> impl Iterator<Item = (&'a Strip, StripBox)> {
        let boxes = self.boxes;
        self.strips
            .iter()
            .zip(boxes)
            .filter_map(|(strip, at)| at.map(|at| (strip, at)))
    }
}

// ---------------------------------------------------------------------------
// The Mixer bay's transition row
// ---------------------------------------------------------------------------

/// **What the next scheduled move means**: the shape a wipe's front takes and
/// which way it runs, the musical grid the move starts on, and how long it
/// lasts.
///
/// # It is the console's own model of record, and that is not a convenience
///
/// Every other value this bay draws is a reading handed in by whoever owns the
/// deck. These three are not, and [`Operation::SetTransition`] is why: it
/// *"changes nothing you can see and writes nothing to the stream"*, it is
/// `Written::Silent(Silent::Surface)`, and no record in
/// `karakuri-store` carries any of the three. So there is nothing downstream
/// that could be asked what the quantum is, and a host that kept a copy would
/// be keeping the console's state on its behalf — which is
/// [`View::selection`]'s argument arriving at a fourth pointer.
///
/// **What reads it is the wipe.** `karakuri_operation_record::Current::transition`
/// wants the quantum, the length and the front's shape together, and
/// [`Operation::Wipe`] is converted against them — so this is what a host
/// hands that conversion, and `karakuri-cli` holds the same four numbers as
/// `quantum`, `fade_beats`, `mask_kind` and `mask_angle` for the same reason.
///
/// # One value and not four fields on [`View`]
///
/// [`Operation::SetTransition`] is one operation with a three-armed payload,
/// and the manual has the row as one heading — *"This row is three operations
/// and the manual has it as one"*, which is `TransitionSetting`'s own
/// sentence. Four loose fields would be that sum taken apart in the one crate
/// that draws the row it is a sum for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionSettings {
    /// The shape the next wipe's front takes. [`WipeKind::None`] is *no
    /// shape*, under which `karakuri-cli`'s `c` is refused — which is the
    /// vocabulary's own sentence at that variant, not a rule this row holds.
    pub kind: WipeKind,
    /// Which way a linear front runs, in radians.
    ///
    /// **Its own field beside the kind, because the operation carries both
    /// together**: `TransitionSetting::WipeShape { kind, angle }` is one
    /// setting, and a shape chosen without an angle would be a surface
    /// straightening a diagonal front nobody touched — [`Mixer::mask`]'s
    /// argument (ADR-0203) one row down. Here the two move together because
    /// the pill's cycle names both, which is what [`WIPE_SHAPES`] is.
    pub angle: f32,
    /// The grid the next scheduled move starts on, in beats: 4 for the next
    /// bar, 1 for the next beat, 0 for now.
    pub quantum: f64,
    /// How long the next scheduled move lasts, in beats. Zero is a cut.
    pub length: f64,
}

/// **The shapes the row's first pill offers, in cycle order, with the angle
/// each runs at and the word the pill reads.**
///
/// # The vocabulary has no list, so a surface curates one
///
/// `karakuri_operation::TransitionSetting` has no `ALL` and no `name`, and
/// `WipeKind` has an `ALL` for nobody: a wipe shape is **a kind and an
/// angle**, so the thing a control cycles is a set of *pairs* and no
/// enumeration of a kind can be it. `WipeKind`'s own documentation says so —
/// *"an arbitrary angle is a dial, and a dial with nowhere to show its value
/// is a control an operator cannot read … the curation is a keyboard's
/// compromise, not the operation"* — and names `karakuri-cli`'s `MASK_SHAPES`
/// as the surface that curates one. This is the second, and the six pairs are
/// that one's exactly, so two surfaces stepping this setting arrive at the
/// same six places.
///
/// **The words are not that one's**, and they are the only half that differs:
/// `MASK_SHAPES` writes a sentence into a status line (*"linear, left to
/// right"*) where this writes into a capsule the width of its own word.
/// `docs/manual/console.html` draws the row's shape pill reading `iris`, so
/// the register is the mock's.
///
/// **`WipeKind::None` reads `no shape` and never `off`**, which is not a
/// preference: `off` is a *residency* word on this console — the one a deck
/// preview's caption said until ADR-0240 — and `tests/preview_caption.rs`
/// asserts it is painted nowhere on the panel, because it is a state the
/// program cannot be in. `no shape` is `MASK_SHAPES`' own sentence for the
/// same entry (*"`c` needs a shape"*) in this console's register, beside
/// `no slot`.
///
/// **`None` first**, which is `MASK_SHAPES`' own reason and
/// `karakuri_engine::deck::MaskKind::ALL`'s before it: *"a deck nobody has
/// touched wipes with nothing and says so rather than doing something."* It
/// is also [`TransitionSettings::START`], so a console nobody has pressed
/// anything on and a program nobody has pressed anything on begin in the same
/// place.
///
/// **A table rather than a `match`**, where [`after`], [`next`] and
/// [`next_shape`] are all matches. Those cycle a *vocabulary* enumeration, and
/// a match is what stops a fourth variant compiling until somebody says what
/// follows it. This cycles a curation — six of an unbounded set of pairs —
/// and there is no enumeration for the compiler to hold it against, so a table
/// is the honest shape: the list is the decision.
const WIPE_SHAPES: [(WipeKind, f32, &str); 6] = [
    (WipeKind::None, 0.0, "no shape"),
    (WipeKind::Linear, 0.0, "left"),
    (WipeKind::Linear, std::f32::consts::FRAC_PI_2, "up"),
    (WipeKind::Linear, std::f32::consts::FRAC_PI_4, "diagonal"),
    (
        WipeKind::Linear,
        -std::f32::consts::FRAC_PI_4,
        "back diagonal",
    ),
    (WipeKind::Radial, 0.0, "iris"),
];

/// **The grids the row's second pill offers, in cycle order**, with the word
/// the pill reads.
///
/// The three are `karakuri-cli`'s `QUANTA`, values and order, for
/// [`WIPE_SHAPES`]' reason: two surfaces stepping one setting arrive at the
/// same places. **A bar is four beats here**, which is that constant's own
/// assumption rather than a measurement — nothing in the signal bus knows a
/// time signature.
///
/// The words are shortened to the mock's own, which draws this pill reading
/// `next bar`.
const QUANTA: [(f64, &str); 3] = [(4.0, "next bar"), (1.0, "next beat"), (0.0, "now")];

/// **The lengths the row's third pill offers, in cycle order**, with the word
/// the pill reads.
///
/// `karakuri-cli`'s `FADE_BEATS`, values and order: *"a bar, half a bar, two
/// bars, and a cut — the four an operator reaches for, in the order they are
/// reached for."* The mock draws this pill reading `8 beats`, which is the
/// third of them.
///
/// **Zero reads `cut` rather than `0 beats`**, because that is what a fade of
/// no length is and is the word `karakuri-cli` prints for it.
const FADE_BEATS: [(f64, &str); 4] = [
    (4.0, "4 beats"),
    (2.0, "2 beats"),
    (8.0, "8 beats"),
    (0.0, "cut"),
];

impl TransitionSettings {
    /// **Where a run starts**: no shape, the next bar, four beats — the first
    /// entry of each of the three cycles.
    ///
    /// It is `karakuri-cli`'s own opening state (`MASK_SHAPES[0]`, `QUANTA[0]`
    /// and `FADE_BEATS[0]` at `Live::new`), so two surfaces that step the same
    /// three settings also begin at the same three values. A console that
    /// opened on the mock's `iris · next bar · 8 beats` would be starting a
    /// run somewhere a hand had to have put it.
    pub const START: TransitionSettings = TransitionSettings {
        kind: WIPE_SHAPES[0].0,
        angle: WIPE_SHAPES[0].1,
        quantum: QUANTA[0].0,
        length: FADE_BEATS[0].0,
    };

    /// Where this shape sits in [`WIPE_SHAPES`].
    ///
    /// **The fallback is unreachable while [`View::set_transition`] is the
    /// only way in**, because that setter refuses a value no entry of the
    /// table names — see it for why. It falls back rather than panicking for
    /// the reason nothing on the frame path panics: the cost of being wrong
    /// here is one pill reading the wrong word, and the cost of being right
    /// about it is a console that stops drawing.
    fn shape_at(&self) -> usize {
        WIPE_SHAPES
            .iter()
            .position(|(kind, angle, _)| *kind == self.kind && *angle == self.angle)
            .unwrap_or(0)
    }

    fn quantum_at(&self) -> usize {
        QUANTA
            .iter()
            .position(|(beats, _)| *beats == self.quantum)
            .unwrap_or(0)
    }

    fn length_at(&self) -> usize {
        FADE_BEATS
            .iter()
            .position(|(beats, _)| *beats == self.length)
            .unwrap_or(0)
    }

    /// **What the shape pill reads.**
    pub fn shape_word(&self) -> &'static str {
        WIPE_SHAPES[self.shape_at()].2
    }

    /// **What the quantum pill reads.**
    pub fn quantum_word(&self) -> &'static str {
        QUANTA[self.quantum_at()].1
    }

    /// **What the length pill reads.**
    pub fn length_word(&self) -> &'static str {
        FADE_BEATS[self.length_at()].1
    }

    /// **Whether a shape is chosen at all**, which is what draws the pill
    /// armed: `.pill.armed` is *armed, bound, live in the good sense*, and a
    /// row whose shape reads `no shape` has nothing armed to say.
    ///
    /// It is not a rule about what a wipe may do. `WipeKind::None` refuses a
    /// wipe where the record is applied and not here — every way in meets the
    /// same wall
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    pub fn armed(&self) -> bool {
        self.kind != WipeKind::None
    }

    /// **The next shape round the cycle**, as the setting an operation
    /// carries.
    pub(crate) fn next_wipe_shape(&self) -> TransitionSetting {
        let (kind, angle, _) = WIPE_SHAPES[(self.shape_at() + 1) % WIPE_SHAPES.len()];
        TransitionSetting::WipeShape { kind, angle }
    }

    /// **The next quantum round the cycle**, as the setting an operation
    /// carries.
    pub(crate) fn next_quantum(&self) -> TransitionSetting {
        TransitionSetting::Quantum {
            beats: QUANTA[(self.quantum_at() + 1) % QUANTA.len()].0,
        }
    }

    /// **The next length round the cycle**, as the setting an operation
    /// carries.
    pub(crate) fn next_length(&self) -> TransitionSetting {
        TransitionSetting::Length {
            beats: FADE_BEATS[(self.length_at() + 1) % FADE_BEATS.len()].0,
        }
    }

    /// **Take a setting, and answer whether anything moved** — the whole of
    /// what [`View::set_transition`] does, kept beside the three tables that
    /// decide it.
    ///
    /// `false` for a setting no entry of the cycles names, and for one that
    /// names where the row already is.
    fn take(&mut self, setting: TransitionSetting) -> bool {
        let was = *self;
        match setting {
            TransitionSetting::WipeShape { kind, angle } => {
                if !WIPE_SHAPES
                    .iter()
                    .any(|(k, a, _)| *k == kind && *a == angle)
                {
                    return false;
                }
                self.kind = kind;
                self.angle = angle;
            }
            TransitionSetting::Quantum { beats } => {
                if !QUANTA.iter().any(|(b, _)| *b == beats) {
                    return false;
                }
                self.quantum = beats;
            }
            TransitionSetting::Length { beats } => {
                if !FADE_BEATS.iter().any(|(b, _)| *b == beats) {
                    return false;
                }
                self.length = beats;
            }
        }
        *self != was
    }
}

/// **The Mixer bay's transition row, laid out**: `.xfade` under the strips,
/// the three setting pills in it, and the `go` capsule at the right end.
///
/// # It carries the settings it was measured from
///
/// [`Mixer`] borrows the strips for [`Picture`]'s reason and this carries a
/// copy for the same one: a pill is as wide as the word in it and the word is
/// the setting's, so whoever measured the row and whoever paints it are one
/// statement. [`TransitionSettings`] is four scalars and `Copy`, so there is
/// no borrow to take.
///
/// # The `go` pill is drawn now, and what changed is not this crate
///
/// It was left out while [`Operation::Wipe`] could not be converted anywhere:
/// a wipe is written against `karakuri_operation_record::Current::transition`
/// **and** `Current::mix`, `crates/karakuri/src/main.rs` answered `None` for
/// both, and a capsule the mock lights `on` that a press does nothing with is
/// the scaffolding this module refuses. That window supplies both readings
/// now, so the capsule is a control and is drawn.
///
/// **`.sep` is honoured rather than drawn.** It is `flex: 1` and paints
/// nothing at all; what it does is push the `go` capsule to the right end of
/// the row, which is where [`transition`] puts it — measured in from
/// `.xfade`'s own padding, the way the shape pill is measured in from the
/// other side.
///
/// # What the mock draws here and this does not
///
/// - **The `wipe` pill beside the shape.** The mock draws the shape as two
///   spans — an armed `wipe` and the shape's own word — and that is one
///   statement about one setting: *a wipe shape is armed, and it is an iris*.
///   There is one setting in `TransitionSetting` for it, so there is one
///   control here, and the armed treatment the mock puts on the first span is
///   carried by the pill that names the shape
///   ([`TransitionSettings::armed`]).
/// - **The row's tooltip.** Every control on this console is a painted shape
///   and a tooltip needs `egui` to own a widget — the sentence [`outputs`]
///   writes about a control, three bays along.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionRow {
    /// `.xfade` itself: the block under the strips, the full width of the bay,
    /// with its rule along the top edge.
    pub rect: Rect,
    /// The shape pill, which is **the capsule a press acts on** and not only
    /// the box a word is painted into — [`TransitionRow::shape`] hit-tests
    /// exactly this rectangle, the way [`StripBox::blend`] is. As wide as the
    /// word in it, inside `.pill`'s padding and border.
    pub shape: Rect,
    /// The quantum pill, on the same terms.
    pub quantum: Rect,
    /// The length pill, on the same terms.
    pub length: Rect,
    /// **The `go` capsule**, at the right end of the row with `.sep`'s
    /// `flex: 1` between it and the length pill. On the same terms as the
    /// three: it is the rectangle a press acts on, and
    /// [`TransitionRow::go`] hit-tests exactly it.
    pub go: Rect,
    /// **The settings these rectangles were measured from.**
    pub settings: TransitionSettings,
}

/// **What a press on the `go` capsule comes to**: a wipe, or the reason there
/// is not one.
///
/// # A refusal is an answer this control has and the record layer does not
///
/// [`Operation::Wipe`] says *"Refused with no shape chosen"* at its own
/// definition and `karakuri-operation-record`'s arm says the same from the
/// other side: `Written` has three answers and none of them is a refusal,
/// because the shape is **a surface's own setting** and a surface is the only
/// thing that can see it is unset. `karakuri-cli`'s `c` is the precedent for
/// both of these arms — it turns a one-slot deck and a chosen-nothing shape
/// away before it asks — and this is that key's two refusals as a value,
/// because this crate has nowhere to print.
///
/// **It says which refusal and not what to do about it**, which is where the
/// seam between this crate and the window that runs it falls: the sentence an
/// operator reads is `crates/karakuri/src/main.rs`'s, and
/// [P-0083](../../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)
/// is what that sentence owes — the shape pill is two capsules to the left,
/// and a second deck is a `--set` away.
///
/// **Not `Option<Operation>`**, which would make a press on a row with no
/// shape chosen indistinguishable from a press on the card beside it. A
/// control that claims a press has acted on it
/// ([`crate::input::claim`]'s rule 4), and the act here is the refusal.
#[derive(Debug, Clone, PartialEq)]
pub enum Go {
    /// **Run it**: [`Operation::Wipe`] naming the deck being covered and the
    /// deck arriving over it. See [`TransitionRow::go`] for which is which.
    Wipe(Operation),
    /// **There is nowhere for the wipe to come from**: the mixer draws fewer
    /// than two strips, so the deck the selection is on is the only deck
    /// there is. `karakuri-cli`'s *"a wipe needs somewhere to come from —
    /// this deck holds one slot"*.
    NoOtherDeck,
    /// **No shape is chosen**, so there is nothing for the front to be. The
    /// shape pill on this row is where one is picked, and
    /// [`TransitionSettings::armed`] is the same fact drawn.
    NoShape,
}

impl TransitionRow {
    /// **What a press at `p` asks the wipe shape to become**, or `None` where
    /// there is no shape pill under it.
    ///
    /// # The pill cycles, and the operation names where it arrived
    ///
    /// Click it and the shape moves to the next of [`WIPE_SHAPES`], wrapping
    /// from the last back to the first, and what comes out is
    /// [`Operation::SetTransition`] naming the **destination** — never a step,
    /// because there is no step in the vocabulary to name.
    ///
    /// That is the affordance
    /// [P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
    /// leaves to whoever draws the control, and [`Mixer::blend`] is the
    /// precedent (ADR-0187): a pill that cycles is one control emitting six,
    /// the operator sees a toggle and the vocabulary never does. What P-0090
    /// forbids is an operation that says *step*, and this emits none.
    ///
    /// **The cycle is curated here because the vocabulary has no list to
    /// cycle** — see [`WIPE_SHAPES`], where that is the whole argument, and
    /// `karakuri-cli`'s `MASK_SHAPES`, which is the precedent for a surface
    /// curating one.
    ///
    /// # One derivation, asked twice, and the whole pill is the target
    ///
    /// [`crate::input::claim`]'s rule 4 asks this and so does the caller that
    /// acts on the press — the arrangement [`Mixer::blend`], [`Mixer::mask`]
    /// and [`Outputs::op`] are all in. [`TransitionRow::shape`] is the pill's
    /// own rectangle, the one the word is painted into, so a pill a hand sees
    /// and a pill it clicks are the same one, and the padding is what makes a
    /// word a hand can find ([`Outputs::sink`]'s rule).
    ///
    /// # It names no deck, and there is nothing missing
    ///
    /// The settings decide what the *next* move means wherever it lands, so
    /// [`Operation::SetTransition`] carries a setting and no slot — the one
    /// row of this bay that does, for [`Operation::SetMasterOut`]'s reason
    /// one bay down.
    pub fn shape(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.pressed(self.shape, p, self.settings.next_wipe_shape())
    }

    /// **What a press at `p` asks the quantum to become**, or `None` where
    /// there is no quantum pill under it. [`TransitionRow::shape`]'s
    /// affordance over [`QUANTA`].
    pub fn quantum(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.pressed(self.quantum, p, self.settings.next_quantum())
    }

    /// **What a press at `p` asks the length to become**, or `None` where
    /// there is no length pill under it. [`TransitionRow::shape`]'s affordance
    /// over [`FADE_BEATS`].
    pub fn length(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.pressed(self.length, p, self.settings.next_length())
    }

    /// **What a press at `p` on the `go` capsule comes to**, or `None` where
    /// there is no capsule under it.
    ///
    /// # Which two decks a wipe names
    ///
    /// [`Operation::Wipe`] carries *the deck being covered* and *the deck
    /// arriving over it*, and this row names them the way `karakuri-cli`'s `c`
    /// does: **the deck the selection is on is covered, and the next one round
    /// arrives over it.** *The next deck* is the surface's translation and
    /// never the operation — the vocabulary's own sentence at that variant —
    /// and here the addressed deck is [`View::selection`], which is the ring
    /// this bay draws round a strip and the letter the library's `load` pill
    /// reads. `decks` is how many strips the mixer has, which is
    /// [`View::mixer`]'s length and the same count [`View::select`] refuses a
    /// selection against, so the wrap cannot name a deck with no strip.
    ///
    /// **Neither deck is decided here beyond that.** What the wipe *does* to
    /// them — the mask at 0, the put-on-air, whether `over` is written at all
    /// — is the conversion's and the deck's
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    ///
    /// # The two refusals are this control's, and they are its own
    ///
    /// A one-deck mixer and a shape reading `no shape` are both turned away
    /// here, in `karakuri-cli`'s order, and see [`Go`] for why a refusal is a
    /// value rather than a `None`.
    pub fn go(&self, p: karakuri_layout::Point, selection: u8, decks: usize) -> Option<Go> {
        if !self.go.contains(Pos2::new(p.x, p.y)) {
            return None;
        }
        if decks < 2 {
            return Some(Go::NoOtherDeck);
        }
        if !self.settings.armed() {
            return Some(Go::NoShape);
        }
        let from = usize::from(selection).min(decks - 1);
        Some(Go::Wipe(Operation::Wipe {
            from: from as u8,
            to: ((from + 1) % decks) as u8,
        }))
    }

    /// **Whether a press on the `go` capsule would run a wipe**, which is what
    /// draws it lit: `.pill.on` is the mock's *this is the press that does the
    /// thing*, and a capsule lit over a refusal would be the row saying it can
    /// do something it cannot.
    ///
    /// It is [`TransitionSettings::armed`] with the deck count beside it —
    /// exactly the two conditions [`TransitionRow::go`] refuses on, asked
    /// again rather than copied, so the pill a hand sees lit and the press
    /// that runs cannot come apart.
    pub fn runs(&self, decks: usize) -> bool {
        decks >= 2 && self.settings.armed()
    }

    /// One pill's hit test, written once because the three differ only in
    /// which rectangle and which cycle — the shape [`Mixer`]'s five share by
    /// being five questions about one laid-out strip.
    fn pressed(
        &self,
        pill: Rect,
        p: karakuri_layout::Point,
        setting: TransitionSetting,
    ) -> Option<Operation> {
        pill.contains(Pos2::new(p.x, p.y))
            .then_some(Operation::SetTransition { setting })
    }

    /// **Whether a press at `p` is on any of the row's four capsules**, which
    /// is what [`crate::input::claim`] asks: the panel claims what it acts on,
    /// and the row's own ground between two pills is not something it acts on.
    ///
    /// [`MasterRow::owns`]'s shape one bay up, and asked of the four
    /// derivations rather than of [`TransitionRow::rect`] — a press on the
    /// card either side of the pills reaches nothing, so claiming it would be
    /// taking an event to throw away. **`.sep` is ground and not a control**,
    /// for exactly that reason: it is the gap the `go` capsule is pushed to
    /// the end by, and there is nothing there to press.
    ///
    /// **The `go` capsule is claimed whether or not a wipe would run**, which
    /// is the one place this differs from the three pills: a press that is
    /// refused *is* acted on — the window says why — so claiming it is the
    /// rule met rather than bent.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.shape(p).is_some()
            || self.quantum(p).is_some()
            || self.length(p).is_some()
            || self.go.contains(Pos2::new(p.x, p.y))
    }
}

/// **Where the transition row goes**: the `.xfade` block inside a mixer
/// region.
///
/// Directly under `.mixer-strips`, which is [`strips_row`] plus the
/// [`size::STRIPS_PAD`] below it, and exactly [`size::XFADE_H`] tall rather
/// than whatever the bay has left — the bay is taller than its contents by
/// design, and what is under this block is the 23.5 the crossfader took with
/// it when the mixer was decided to have none. It is the **full width of the
/// bay**, because `.xfade` is a child of `.bay` and its own padding is
/// [`size::XFADE_PAD_X`]; the rule along its top spans the card the way a bay
/// head's does.
///
/// `None` where the region cannot hold it — folded away, soloed away, or a
/// window too small — which is [`strips_row`]'s rule stated on the row under
/// it.
fn xfade_row(region: Rect) -> Option<Rect> {
    let block = Rect::from_min_size(
        Pos2::new(
            region.min.x,
            region.min.y + size::HEAD_H + size::STRIPS_PAD * 2.0 + size::STRIP_H,
        ),
        egui::vec2(region.width(), size::XFADE_H),
    );
    match block.width() > 0.0 && region.contains_rect(block) {
        true => Some(block),
        false => None,
    }
}

/// **The Mixer bay's transition row, derived**: the block under the strips,
/// the three setting pills laid end to end from its left padding, and the `go`
/// capsule against its right one.
///
/// # It is drawn with or without a deck, and that is not the strips' rule
///   broken
///
/// [`mixer`] answers `None` for a console with no deck behind it, because six
/// readings a slot with no slot to read is ADR-0177's row of zeroes. **These
/// three are not readings.** They are the console's own pointer
/// ([`TransitionSettings`]) and it always has a value — the same thing that is
/// true of [`View::selection`] and of the arrangement pill, which is drawn
/// with no store behind it. A row blanked for want of a deck would be a
/// setting an operator cannot make until something else has happened.
///
/// **The `go` capsule is drawn there too**, and what it says about a deckless
/// console is said in the paint and in the answer rather than by leaving it
/// out: it is not lit ([`TransitionRow::runs`]) and a press on it is
/// [`Go::NoOtherDeck`]. A capsule that vanished with the strips would be the
/// one control on this row an operator has to discover.
///
/// # All four capsules or none
///
/// The three settings are laid from the left of the block's padding, one
/// [`size::XROW_GAP`] apart; `go` is measured back from the padding on the
/// other side, which is `.sep`'s `flex: 1` — the separator takes whatever is
/// between them and paints nothing. If the three would reach it the whole row
/// answers `None`. **Not the chips-that-fit
/// rule** `.scopes` and `.rend-row` are drawn under: those are a *list* whose
/// length is a value, and what is dropped off the end is one more of the same
/// question. This is three different settings and the press that runs them,
/// and one of them silently missing is a control with nothing on screen saying
/// where it went. The bay's
/// track is a fixed 400 wide (`lib.rs`), so the case is a window below the
/// arrangement's own minimum rather than an ordinary narrow console.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one. `ctx` is asked for the type, because a pill is as wide as the word in
/// it.
pub fn transition(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    settings: TransitionSettings,
) -> Option<TransitionRow> {
    // Fonts are not valid until `egui` has run a pass, exactly as in `mixer`,
    // `outputs` and `transport`.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let region = to_egui(layout.rect(layout.find("mixer")?));
    let block = xfade_row(region)?;
    let mut x = block.min.x + size::XFADE_PAD_X;
    let top = block.min.y + size::HAIRLINE + size::XFADE_PAD_TOP;
    let right = block.max.x - size::XFADE_PAD_X;
    let mut pill = |text: &str| {
        // `.pill`'s padding either side of the word, and its own border,
        // which `pill_width` does not count — see `size::XPILL_H`, where the
        // two pixels are argued.
        let w = pill_width(ctx, text) + size::HAIRLINE * 2.0;
        let at = Rect::from_min_size(Pos2::new(x, top), egui::vec2(w, size::XPILL_H));
        x = at.max.x + size::XROW_GAP;
        at
    };
    let shape = pill(settings.shape_word());
    let quantum = pill(settings.quantum_word());
    let length = pill(settings.length_word());
    // **`go` from the other end**, which is what `.sep`'s `flex: 1` puts it:
    // the separator absorbs whatever is left between the length pill and this
    // one, so the capsule's place is measured off the block's right padding
    // and never off the words to its left.
    let go_w = pill_width(ctx, GO) + size::HAIRLINE * 2.0;
    let go = Rect::from_min_size(
        Pos2::new(right - go_w, top),
        egui::vec2(go_w, size::XPILL_H),
    );
    // The separator is `flex: 1` and so is never negative: where the three
    // settings would reach the capsule there is no row, for the reason the
    // header gives. One `.xrow` gap is the least `.sep` can be and still be a
    // gap between two pills rather than two capsules touching.
    if length.max.x + size::XROW_GAP > go.min.x {
        return None;
    }
    Some(TransitionRow {
        rect: block,
        shape,
        quantum,
        length,
        go,
        settings,
    })
}

/// **What the `go` capsule reads**, which is the mock's own word and the only
/// one on this row that is not a setting's.
///
/// It is a *verb* where the three beside it are values — `docs/manual/console.html`
/// draws `iris · next bar · 8 beats · go` — so it has no cycle behind it and
/// no table to come out of. Written here rather than inline because
/// [`transition`] measures the capsule from it and [`transition_into`] paints
/// it from it, which is this module's rule about every word it draws.
const GO: &str = "go";

/// **The Mixer bay's strips, derived**: one per slot the deck has, and none at
/// all where there is no deck.
///
/// # The page has [`DECKS`] tracks whatever the deck holds
///
/// `.mixer-strips` is `grid-template-columns: repeat(4, 1fr)`, and that four
/// is the same four [`DECKS`] is — `MAX_SLOTS` is 4, so no deck can fill a
/// fifth. **A strip is one of four tracks wide even where there is one
/// strip**, and that is read off the arrangement rather than chosen here: the
/// right pane's minimum width is written as *"four mixer strips still side by
/// side … four of them with three 4px gaps inside `.mixer-strips`' 6 + 6 is
/// 172"*. Tracks that followed the strip *count* would make a one-slot deck's
/// strip 244 wide in a pane sized for four 61-wide ones, and would re-derive
/// that minimum every time a slot was installed.
///
/// So a track with no strip in it **draws nothing at all** — not an empty
/// strip. That is the opposite of what [`preview`] does with a cell that has
/// no slot behind it, and the two are not in tension: a preview cell is the
/// region's own face and its caption says `no slot`, where a strip is six
/// readings and an empty one is six readings nobody took. The mock drew such a
/// strip — `.strip.empty`, with `—` for a name, `empty` for a tally and both
/// tracks bare — until `39f1e6b` took it out under ADR-0178; inventing one
/// here would be ADR-0177's row of zeroes with a different glyph, which is why
/// the strip going does not make this rule the mock's rather than the
/// record's.
///
/// **The example's deck has one slot, so it draws one strip**, and that is the
/// example rather than a gap in it — exactly as three of its preview cells
/// read `no slot`.
///
/// # What is in the mock's bay and is deliberately not here
///
/// - **The head's `3 of 3 · page 1`.** *"The strip is a paged list whose
///   length is a number, the header says how long the list is, and it says
///   which page you are on"* — so both halves of that pill are about paging,
///   and there is none. Every strip the deck has is drawn, on the one page, so
///   the pill could only ever read `n of n · page 1`: two numbers that are
///   always equal and a third that is always 1. That is [`Kind::Bay`]'s own
///   rule about a pill stating a value the console does not have, and the
///   number that *is* known — how many
///   strips there are — is on the face of the bay already. It arrives with
///   paging.
/// - **`.xfade`, the transition row under the strips.** It is drawn now, and
///   it is [`transition`] rather than this: the three settings are the
///   console's own pointer and not six readings a slot, so the row survives a
///   console with no deck behind it where these strips do not. **The A/B
///   track that used to sit above it is not a row this console owes**: the
///   mixer has no crossfader, and `console.html`'s *The mixer has no
///   crossfader* is the argument. The block is 37.5 of the mock's bay
///   ([`size::XFADE_H`], which is what this console now carves) and the bay
///   still reserves 61 for it — the 16.5 row and the 7 gap the crossfader
///   took with it. That height has still not been re-derived, and
///   `tests/mixer.rs` now states the leftover in the row's own terms rather
///   than as a subtraction.
/// - **`.wfocus`, which is the second of the mock's two focuses**: keyboard
///   focus, transient, wherever tab lands. The mock draws it as a dashed sun
///   outline and the deck selection as a solid lavender ring, on purpose,
///   because *"Drawing them the same way would erase which of the two a reader
///   is looking at"*. **The selection exists here now** and is drawn —
///   [`View::selection`], and [`mixer_into`] for the ring — which is the
///   sentence ADR-0219 recorded as owed. Keyboard focus does not: nothing in
///   this console takes it, so a dashed outline would be drawn around a state
///   that is not kept.
/// - **Every tooltip.** Four of this bay's controls carry one, and a tooltip
///   needs `egui` to own a widget — the sentence [`outputs`] writes about a
///   control, one bay along.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one. `ctx` is asked for the type, because the tally's capsule and the
/// blend's mini are as wide as the words in them.
pub fn mixer<'a>(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    strips: &'a [Strip],
) -> Option<Mixer<'a>> {
    // **No deck behind the console, so there are no strips.** Every test in
    // this crate is here, and so is the whole of `cargo test -p
    // karakuri-console`. Drawing four empty strips would be inventing six
    // readings a slot; this is `View::picture`'s rule, one bay along.
    if strips.is_empty() {
        return None;
    }
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // `outputs` and `transport`.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let region = to_egui(layout.rect(layout.find("mixer")?));
    let row = strips_row(region)?;
    let width = |job: LayoutJob| ctx.fonts_mut(|f| f.layout_job(job).size().x);
    let label = width(span_at(
        TRIM_LABEL,
        size::TRIM_LABEL_SIZE,
        Color32::PLACEHOLDER,
    ));
    // **The chip is as wide as the widest residency, not as the one it is
    // showing.** Measured once for the bay rather than per strip: it is the
    // same three words in every strip, so a per-strip measurement would be
    // three galley layouts a strip for one answer.
    //
    // It read `strip.tally` alone until the tally learned to say that a
    // request had not landed, and that was a defect rather than a
    // simplification: a chip sized to the current word is a capsule that
    // changes width when the deck moves under it, and one that changes width
    // *while a word is rolling through it* would be a control resizing on its
    // own animation. What the mock does not have is the reason it survived —
    // a still page draws each chip once, so shrink-to-fit and this are the
    // same picture there and only one of them is the same picture over time.
    let tally = Tally::ALL
        .into_iter()
        .map(|tally| width(tally_job(tally, Color32::PLACEHOLDER)))
        .fold(0.0f32, f32::max);
    let mut boxes = [None; DECKS];
    for (index, strip) in strips.iter().take(DECKS).enumerate() {
        let blend = width(span_at(
            strip.blend.name(),
            size::MINI_SIZE,
            Color32::PLACEHOLDER,
        ));
        boxes[index] = strip_box(
            track(row, DECKS, index, size::STRIP_GAP, Axis::Row),
            label,
            tally,
            blend,
        );
    }
    Some(Mixer { strips, boxes })
}

/// **Where the strips go**: the `.mixer-strips` grid inside a mixer region.
///
/// The region less [`size::HEAD_H`] for the bay head painted over the top of
/// it, inset by [`size::STRIPS_PAD`] left, right and top, and **exactly
/// [`size::STRIP_H`] tall** rather than whatever is left over. The bay is
/// taller than its strips by design — the reservation for `.xfade` is the
/// other 61 of it, of which [`xfade_row`] carves [`size::XFADE_H`] and 23.5
/// is the crossfader row the mock no longer draws — and a
/// mixer with room to grow (it is the only visible child of a soloed right
/// pane, and a fixed child with room takes it, ADR-0157) grows the bay and not
/// the strips: a `.strip` is a column of fixed type around a `.fader-col`
/// whose 104 is stated in the CSS, so there is nothing in it that gets bigger
/// any more than there is anything that gets smaller.
///
/// `None` where the region cannot hold it — folded away, soloed away, or a
/// window too small — which is [`picture_rect`]'s rule stated on a row of
/// strips.
pub(super) fn strips_row(region: Rect) -> Option<Rect> {
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
///   **centred across the strip** rather than filling it. Two are the
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
fn strip_box(track: Rect, label_w: f32, tally_w: f32, blend_w: f32) -> Option<StripBox> {
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
fn centred_in(row: Rect, w: f32) -> Rect {
    Rect::from_min_size(
        Pos2::new(row.center().x - w * 0.5, row.min.y),
        egui::vec2(w, row.height()),
    )
}

/// **A scheduled move on a laid-out fader**: the mark on where it is going,
/// and the band reaching `rolled` of the way from where it is.
///
/// **Two faders in and no value arithmetic**, which is what keeps this honest:
/// `now` and `to` are the same track measured at the two values, so the mark
/// lands exactly where the knob would and the band starts exactly where the
/// fill ends. The knob's centre is the fill's moving edge — [`fader`] says so
/// — and that one number is the whole of what this reads out of each.
///
/// **The displacement is a fraction of the gap rather than a distance**, so
/// the reach is in proportion to the move: a fade across the fader sets off a
/// long way and a fade of a hundredth sets off a pixel. That is the honest
/// picture and it is also the limit — under about a hundredth of the travel
/// the whole disagreement is a pixel, and what carries the message there is
/// the mark rather than the motion. See
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

/// **The next blend mode round the cycle**, wrapping from the last back to the
/// first — and the whole of the affordance the blend chip is.
///
/// It is four lines here and nothing at all in `karakuri-operation`, which is
/// P-0090's division: a toggle is an affordance, built over operations by
/// whoever draws the control, and it belongs there. The vocabulary owns the
/// three values; this owns the order a pointer walks them in.
///
/// **A match rather than an index into [`BlendMode::ALL`]**, for the reason
/// [`BlendMode::name`] is one: a fourth mode does not compile until somebody
/// says what follows it. The cost is that the order is written twice — here
/// and in `ALL` — so `tests/blend.rs` walks `ALL` through this and asserts
/// they are the same cycle, which is the measurement that keeps the two from
/// drifting rather than a comment promising they will not.
///
/// # It is the workspace's only blend cycle, and it is `pub` for that
///
/// `crates/karakuri/src/main.rs` had one of its own — `after_blend`, what one
/// press of `m` asked for — saying the same three modes in the same order,
/// with a test each. Under the grammar `space` on an addressed blend chip asks
/// [`crate::focus::press`], which asks this, so the key and the chip are one
/// cycle and the copy is gone
/// ([ADR-0333](../../../../docs/adr/0333-the-console-resolves-the-address-and-the-window-loop-names-the-operation.md)).
/// What is left in that file is a device test asserting where the cycle
/// arrives on a real deck, and it asks this rather than restating it — which
/// is the whole of why this is `pub` where [`next`] and [`next_shape`] beside
/// it are `pub(crate)`.
pub fn after(blend: BlendMode) -> BlendMode {
    match blend {
        BlendMode::Add => BlendMode::Over,
        BlendMode::Over => BlendMode::Max,
        BlendMode::Max => BlendMode::Add,
    }
}

/// **The next residency round the cycle**, wrapping from the last back to the
/// first — the whole of the affordance the tally chip is, and the order the
/// mock's own tooltip lists: *"one of three residencies — live, priming,
/// allocated"*.
///
/// [`after`]'s division, one control along: the cycle is three lines here and
/// nothing in `karakuri-operation`, which owns the three values and not the
/// order a pointer walks them in (P-0090).
///
/// **A match rather than an index into [`Tally::ALL`]**, for [`after`]'s
/// reason: a fourth residency does not compile until somebody says what
/// follows it. The price is that the order is written twice — here and in
/// `ALL` — so `tests/tally.rs` walks `ALL` through this and asserts they are
/// the same cycle.
///
/// **What it is asked about is [`Strip::requested`]**, and why is
/// [`Mixer::tally`].
pub(crate) fn next(tally: Tally) -> Tally {
    match tally {
        Tally::Live => Tally::Priming,
        Tally::Priming => Tally::Allocated,
        Tally::Allocated => Tally::Live,
    }
}

/// **The console's word for a residency, as the vocabulary's** — and it is the
/// whole of what the console has to know about the difference.
///
/// [`Tally`] is the mock's `.tally` and the engine's `Residency` seen from the
/// surface; [`karakuri_operation::Residency`] is what an operation may name.
/// They are the same three states and two crates' words for them, so the
/// translation is a match — and a match rather than a cast so that a fourth
/// state on either side stops the build here, where the two lists meet, rather
/// than at a chip drawing a word no operation can carry.
///
/// The mirror image of it is `crates/karakuri/src/main.rs`'s `tally`, which turns the
/// *engine's* `Residency` into a [`Tally`] on the way in. Three names for
/// three states is the cost `karakuri-operation` pays for depending on nothing
/// (P-0090), and this is one of the two places it is paid.
pub(crate) fn residency(tally: Tally) -> Residency {
    match tally {
        Tally::Live => Residency::Live,
        Tally::Priming => Residency::Priming,
        Tally::Allocated => Residency::Allocated,
    }
}

/// **The next mask shape round the cycle**, wrapping from the last back to the
/// first — the whole of the affordance the mask mini is.
///
/// [`after`]'s division, one control along: the cycle is three lines here and
/// nothing in `karakuri-operation`, which owns the three shapes and not the
/// order a pointer walks them in (P-0090).
///
/// **A match, for [`after`]'s reason**: a fourth shape does not compile until
/// somebody says what follows it. Unlike [`after`] and [`next`] the price is
/// not a second copy of the order — [`Mask`] has no `ALL` and neither does
/// `karakuri_operation::WipeKind`, because nothing reads one (see [`Mask`]) —
/// so this is the only statement of the order in this crate, and
/// `tests/mask.rs` is what checks it against the order it is a copy of.
///
/// **It starts at [`Mask::None`]**, which is `karakuri_engine::deck::MaskKind::ALL`'s
/// own order and its own reason: *"`None` first, because it is the default and
/// a cycle should start where a slot starts."* A cycle that began at `Linear`
/// would be a control whose first press on a fresh slot moves it somewhere it
/// has never been.
pub(crate) fn next_shape(mask: Mask) -> Mask {
    match mask {
        Mask::None => Mask::Linear,
        Mask::Linear => Mask::Radial,
        Mask::Radial => Mask::None,
    }
}

/// **The console's word for a mask shape, as the vocabulary's** — [`residency`]
/// one control along, and for its reason exactly.
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

/// The tally's word as one laid-out run, so that measuring it and painting it
/// cannot be two different runs of type.
fn tally_job(tally: Tally, colour: Color32) -> LayoutJob {
    spaced(
        &tally.word().to_uppercase(),
        size::TALLY_SIZE,
        colour,
        size::TALLY_TRACKING,
    )
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

/// **The Mixer bay's strips, painted.**
///
/// Where everything goes is [`mixer`]'s, so this paints and derives nothing.
///
/// **`marked` is the strip a carried Set would land on**, and it is handed in
/// rather than asked here for the reason `selection` is: it is a *pointer*,
/// and one of the two the panel holds. [`View::draw`] resolves it off
/// [`Mixer::dropped`] — the derivation the release asks — so the ring is
/// painted round the strip that release would name and never round a
/// neighbour.
pub(super) fn mixer_into(
    ui: &Ui,
    pal: &Palette,
    mixer: &Mixer,
    phase: Phase,
    selection: u8,
    marked: Option<u8>,
) {
    for (strip, at) in mixer.placed() {
        strip_into(ui, pal, strip, at, phase);
    }
    // `.strip.drop` — `outline: 2px solid var(--c-text)` at `outline-offset:
    // 0`, the rectangle a Set in hand lands on if it is let go here. Drawn
    // **before** the selection below and outside the strip's own edge where
    // that one is inset, which is how one strip wears both at once — the mock
    // draws deck A wearing exactly that pair.
    if let Some(rect) = marked.and_then(|deck| mixer.selected(deck)) {
        drop_ring(ui, pal, rect, size::STRIP_RADIUS);
    }
    // `.strip.focus` — `box-shadow: inset 0 0 0 2px var(--c-lav)`, the deck
    // selection, drawn **after every strip** because an inset shadow is over a
    // strip's contents and not under them. It is a solid ring where the mock's
    // keyboard focus is a dashed outline, which is `console.html`'s *Two
    // focuses, and they do not look alike*: drawn the same way, a reader could
    // not tell which of the two they were looking at. Nothing draws the dashed
    // one — this console takes no keyboard focus.
    //
    // `None` where the selection names a slot this bay is not drawing, which
    // [`View::select`] refuses at the source and this answers again because a
    // rectangle is what it has: a bay one frame behind a deck that lost a slot
    // would otherwise ring a track no strip is in.
    if let Some(rect) = mixer.selected(selection) {
        ui.painter().with_clip_rect(rect).rect_stroke(
            rect,
            CornerRadius::same(size::STRIP_RADIUS as u8),
            Stroke::new(size::STRIP_FOCUS_RING, pal.lav),
            StrokeKind::Inside,
        );
    }
}

/// **The Mixer bay's transition row, painted**, term for term from
/// `style.css`:
///
/// - `.xfade` — `border-top: 1px solid var(--c-hair)`, the rule that separates
///   the row from the strips above it, drawn inside the block's own top edge
///   the way `.scopes`' is drawn inside its bottom one.
/// - `.xrow` — three `.pill`s laid from the left, one `.xrow` gap apart, and
///   the `go` capsule at the right end with `.sep`'s `flex: 1` between. The
///   separator has no background and no border, so honouring it is placing the
///   capsule and painting nothing.
/// - `.pill.armed` on the shape while a shape is chosen, and the plain
///   `.pill` on it while it reads `no shape` — see
///   [`TransitionSettings::armed`].
///   The two settings between them are always plain, which is what the mock
///   draws.
/// - `.pill.on` on `go` while a press on it would run a wipe, and the plain
///   `.pill` otherwise — see [`TransitionRow::runs`], which is the same pair
///   of conditions [`TransitionRow::go`] refuses on. The mock draws it lit
///   because the row it draws is armed with two decks under it.
///
/// Where everything goes is [`transition`]'s, so this paints and derives
/// nothing — [`mixer_into`]'s own sentence, one row up. **`decks` is the one
/// thing it is told rather than measured**: whether the capsule is lit is a
/// fact about the mixer above it, and it is the same count
/// [`TransitionRow::go`] is asked with.
pub(super) fn transition_into(ui: &Ui, pal: &Palette, row: &TransitionRow, decks: usize) {
    // The rule is inside the block rather than above it, which is what keeps
    // the pills where `transition` put them: `size::XFADE_H` counts the
    // hairline as the first pixel of the block.
    let rule = row.rect.min.y + size::HAIRLINE * 0.5;
    ui.painter().line_segment(
        [
            Pos2::new(row.rect.min.x, rule),
            Pos2::new(row.rect.max.x, rule),
        ],
        Stroke::new(size::HAIRLINE, pal.hair),
    );
    pill_into(
        ui,
        pal,
        row.shape,
        row.settings.shape_word(),
        row.settings.armed(),
    );
    pill_into(ui, pal, row.quantum, row.settings.quantum_word(), false);
    pill_into(ui, pal, row.length, row.settings.length_word(), false);
    match row.runs(decks) {
        true => on_pill_at(ui, pal, row.go, GO),
        false => pill_at(ui, pal, row.go, GO),
    }
}

/// **One strip**, term for term from `style.css`:
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
fn strip_into(ui: &Ui, pal: &Palette, strip: &Strip, at: StripBox, phase: Phase) {
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

    tally_into(&painter, pal, at.tally, strip.tally, strip.pending(), phase);

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

/// A galley centred in a box, both ways — which is `align-items: center` on a
/// flex column done where the type's real height is known.
fn centre_galley(
    painter: &egui::Painter,
    rect: Rect,
    galley: std::sync::Arc<egui::Galley>,
    colour: Color32,
) {
    painter.galley(
        Pos2::new(
            rect.center().x - galley.size().x * 0.5,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        colour,
    );
}

/// `.tally`: a capsule with the residency's word in it, in the residency's own
/// colours.
///
/// - `.tally.live` — `color-mix(in srgb, var(--c-pink) 18%, transparent)`
///   behind `var(--c-pink)`, with `box-shadow: 0 0 10px var(--c-glowp)`. The
///   halo is an [`egui::epaint::Shadow`], the mechanism the transport's lit
///   beat and the Outputs row's dot both use.
/// - `.tally.priming` — a 20% wash of `var(--c-sun)` behind `var(--c-sun)`,
///   and **no halo**: priming is warming out of sight, not on air.
/// - `.tally.alloc` — `var(--c-tint)` behind `var(--c-dim)`, which is the
///   palette's own *"the wash behind a node head, and the allocated tally"* —
///   the first use of `--c-tint` in this crate, and it was transcribed against
///   this day.
///
/// # And the roll, where the request has not landed
///
/// `pending` is [`Strip::pending`] — the residency this slot was asked for and
/// has not reached. While it is `Some`, the word rolls part of the way toward
/// it and falls back, once a second, and never arrives: [`roll_at`] is the
/// displacement and [ADR-0190](../../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md)
/// is why it is a roll and not two lamps side by side — 53 pixels, and the
/// pair the rule exists for is 85.125 of them.
///
/// # The clip is new, not narrowed
///
/// Nothing called `with_clip_rect` on a tally before this: the chip painted a
/// filled rect and a galley that fitted inside it, so there was nothing to
/// clip and no clip to get wrong. A second word travelling through the box is
/// the first thing here that is drawn to be cut off, and the cut is what makes
/// the roll a roll rather than two words overlapping the trim row underneath.
/// It is introduced deliberately and it is on the type alone — see the note at
/// the clip itself, because the halo has to go on spilling.
pub(super) fn tally_into(
    painter: &egui::Painter,
    pal: &Palette,
    rect: Rect,
    tally: Tally,
    pending: Option<Tally>,
    phase: Phase,
) {
    let radius = CornerRadius::same((size::TALLY_H * 0.5) as u8);
    let ink = |tally| match tally {
        Tally::Live => (tint(pal.pink, 18), pal.pink),
        Tally::Priming => (tint(pal.sun, 20), pal.sun),
        Tally::Allocated => (pal.tint, pal.dim),
    };
    let (fill, ink_now) = ink(tally);
    if tally == Tally::Live {
        painter.add(
            egui::epaint::Shadow {
                offset: [0, 0],
                blur: size::TALLY_GLOW,
                spread: 0,
                color: pal.glow_pink,
            }
            .as_shape(rect, radius),
        );
    }
    painter.rect_filled(rect, radius, fill);

    // **The clip is the capsule, and it is on the words alone.** The halo is
    // drawn to spill — `.tally.live`'s `box-shadow: 0 0 10px` is 10px of it
    // outside the box — so a clip taken before the shadow would trim the one
    // shape in this chip that is meant to leave it. Everything after this
    // line is type that may be halfway out of the box on purpose.
    let painter = painter.with_clip_rect(rect.intersect(painter.clip_rect()));

    let galley = painter.layout_job(tally_job(tally, ink_now));
    // **The pitch is the travel's, not the geometry's**, and it is at least
    // the box: at the natural row pitch the two words would both be partly
    // visible with nothing between them, which at 9px is mud. One box height
    // apart leaves a blank band of exactly the slack the chip already has —
    // 13.5 less a 10.0 ink row is 3.5 — for every displacement, because the
    // band is the difference of two constants and not a function of how far
    // the roll has got. It costs the chip nothing: the second word is
    // outside the capsule at rest and clipped away.
    let pitch = size::TALLY_H.max(galley.size().y);
    let rolled = match pending {
        Some(_) => roll_at(phase) * pitch,
        None => 0.0,
    };
    let word_into = |painter: &egui::Painter, galley: std::sync::Arc<egui::Galley>, colour, dy| {
        painter.galley(
            Pos2::new(
                rect.center().x - galley.size().x * 0.5,
                rect.center().y - galley.size().y * 0.5 + dy,
            ),
            galley,
            colour,
        );
    };
    word_into(&painter, galley, ink_now, -rolled);
    // **The destination in its own ink**, which is the second half of what is
    // being said: the word names where the slot is going and the colour is the
    // one that slot will be drawn in when it gets there. It comes up from
    // below — one pitch under the settled word — so a still frame of a chip
    // that is not rolling is the chip as it was.
    if let Some(to) = pending {
        let (_, ink_to) = ink(to);
        let galley = painter.layout_job(tally_job(to, ink_to));
        word_into(&painter, galley, ink_to, pitch - rolled);
    }
}

/// The meter: the well, the mean's column and the peak's mark — and **nothing
/// in the well where there is no reading**, which is the mock's own `alloc`
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
/// in for would have been, and then what the mask does to it: nothing, one
/// half filled, or a filled centre.
pub(super) fn mask_mark(painter: &egui::Painter, centre: Pos2, colour: Color32, mask: Mask) {
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

impl View {
    /// **What the next fade, crossfade or wipe means** — see
    /// [`View::transition`] the field, which is where the argument is.
    ///
    /// Read back bare, where [`View::cursor_row`] and [`View::scope`] are both
    /// answered against a listing: there is no listing under this one to fall
    /// out of, because the three cycles are the console's own and
    /// [`View::set_transition`] refuses anything that is not on them.
    pub fn transition(&self) -> TransitionSettings {
        self.transition
    }

    /// **Take one of the three settings, and answer whether that moved
    /// anything.**
    ///
    /// **A setting no pill can draw is refused**, which is [`View::select`]'s
    /// rule and the same reasoning: the row is three capsules and each reads
    /// the word its cycle gives the value it is on, so a shape or a beat count
    /// off the cycle would be a pill with nothing to say — and, once it was
    /// there, a press on it stepping from wherever the fallback in
    /// [`TransitionSettings::shape_at`] landed rather than from what the pill
    /// reads. The cycles are [`WIPE_SHAPES`], [`QUANTA`] and [`FADE_BEATS`],
    /// and each is a *curation*: an angle between two of them is a value the
    /// operation can carry and this row cannot show
    /// (`karakuri_operation::WipeKind`'s own *"a dial with nowhere to show its
    /// value is a control an operator cannot read"*).
    ///
    /// **It refuses rather than clamping**, for [`View::select`]'s reason: the
    /// nearest curated angle is not the angle that was asked for, and a wipe
    /// that ran at it would be a move nobody asked for made on stage.
    ///
    /// **This is not a lock, and P-0090 is what says so.** What may be asked
    /// of the *instrument* is decided where the record is applied; nothing is
    /// applied here at all. The console is the model of record for these three
    /// ([`View::transition`] the field), and a model of record refusing a value
    /// it cannot hold is not a surface deciding what may be asked.
    ///
    /// The `bool` is [`View::select`]'s: a caller repaints on a move and not
    /// on a press, so a press that changed nothing costs no frame (P-0091).
    pub fn set_transition(&mut self, setting: TransitionSetting) -> bool {
        self.transition.take(setting)
    }

    /// **What the mixer bay declares**: the roll's staleness while anything in
    /// it is pending *and* the bay is laid out, and nothing otherwise — with
    /// the deadline it asks for taken from where the roll has got to rather
    /// than from the fact that something is pending.
    ///
    /// Three pending things, one rate — see [`View::declares`], which carries
    /// the whole argument.
    ///
    /// # Pending is when it declares; moving is when it asks for a frame
    ///
    /// The three presentations in this bay are one curve ([`roll_at`]) off one
    /// [`Phase`], and that curve is **exactly zero** for the 600 ms of every
    /// [`ROLL_PERIOD`] that is not [`ROLL_TRAVEL`]. So *is anything pending*
    /// is the right question for whether this region is live — it is what
    /// separates a strip that has somewhere to go from one that has arrived —
    /// and it is the wrong question for whether a frame is owed thirty
    /// milliseconds from now. [`roll_moves_in`] answers the second one, and
    /// [ADR-0283](../../../../docs/adr/0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)
    /// is where the two are separated.
    ///
    /// **The region stays in both sums throughout**, rest included: a parked
    /// slot is a live region for as long as it is parked, and a schedule
    /// admitted on the 40% of the period that moves would be a schedule that
    /// could not afford the thing it admitted.
    ///
    /// # The level meter is in this bay and is not in this declaration
    ///
    /// [`Strip::level`] is the one thing on a strip that is a *measurement*
    /// rather than a setting, and it moves on every frame the engine renders
    /// — so *nothing in this bay changes while the roll rests* is a sentence
    /// about the roll and not about the bay. It declares nothing anyway, and
    /// the reason is **where the reading comes from**: `Deck::level` moves
    /// inside `Deck::begin_frame`, which is the only caller of
    /// `Meters::collect`, and a caller calls it once per composed frame. The
    /// meter's picture is therefore a function of **the frames this panel is
    /// drawn on** rather than of wall time, and there is no moment between two
    /// frames at which what is on screen is not the newest reading taken —
    /// because no reading is taken between two frames.
    ///
    /// That is what separates it from [`roll_at`], which is different at
    /// 400 ms from what it was at 399 whether or not anybody drew anything and
    /// can therefore say when it next moves. It is also what separates it from
    /// the beat, which moves per composed frame as well and declares anyway on
    /// P-0094's forced clause: *something is moving continuously while the
    /// console is live*. There is no such clause for a meter, and a deadline
    /// for one would be asking for the frames that produce the readings it
    /// would then draw — measured in
    /// [ADR-0290](../../../../docs/adr/0290-the-level-meter-moves-only-when-a-frame-is-drawn-so-it-declares-nothing.md),
    /// which is where the alternatives are, and held in `tests/metered.rs`.
    ///
    /// **What would change it is ballistics.** A peak that is held and decays,
    /// or a fill that falls at a rate rather than following the reading, is a
    /// function of the clock exactly as the roll is: it would move between two
    /// frames, and it would then have to declare. `tests/metered.rs` asserts
    /// that premise — the meter's geometry is a function of the reading and of
    /// nothing else — so the day a meter grows a fall time is a test failure
    /// rather than a readout that freezes with nothing saying so.
    pub(super) fn mixer_declares(&self, layout: &karakuri_layout::Layout) -> Option<Declared> {
        let bay = layout.find("mixer").is_some_and(|id| layout.visible(id));
        let moving = bay
            && self.mixer.iter().any(|strip| {
                strip.pending().is_some()
                    || strip.gain_pending().is_some()
                    || strip.opacity_pending().is_some()
            });
        moving.then_some(Declared {
            region: "mixer",
            cost: PANEL_PASS,
            staleness: ROLL_STALENESS,
            moves_in: roll_moves_in(self.phase),
        })
    }
}
