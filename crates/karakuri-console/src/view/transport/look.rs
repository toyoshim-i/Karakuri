use super::super::*;
use super::*;

// ---------------------------------------------------------------------------
// The look: the tone map, and the level going into it
// ---------------------------------------------------------------------------

/// The tone map pill's first word, the mock's own two-part label one pill along
/// from `arr · night ▾` — and no chevron, which is what says this one acts on
/// the press instead of putting a card down. The two pills that open a menu
/// carry the mark and the three that do not (`learn`, `tap` — drawn since
/// 2026-09-08, and [`TAP_LABEL`] — and this) do not.
const TONEMAP_LABEL: &str = "tone";

/// The word before the exposure track — `.trim`'s `g` one row up, and the same
/// arrangement: a faint label, [`size::TRIM_GAP`], a horizontal [`fader`].
const EXPOSURE_LABEL: &str = "exp";

/// Every tone map operator, in the order this control walks them.
///
/// `karakuri_operation::Tonemap` has no `ALL` — nothing had read one until now
/// — so unlike the blend chip's cycle this is not a second copy of a list the
/// vocabulary keeps ([`after`] says what that costs). It is the only statement
/// of the order in this workspace apart from `karakuri-cli`'s own
/// `next_tonemap`, which walks the engine's `TonemapOp` in exactly this order,
/// and it is what [`next_tonemap`] is checked against.
///
/// The order is the engine's own declaration order, which is the order
/// [ADR-0037](../../../../docs/adr/0037-tone-mapping-is-a-uniform-and-the-default-is-chosen-by-looking.md)
/// compared them in: `Clamp` first, because it is *"not a tone mapper — the
/// control that shows what the other three are fixing"* and is where a
/// `Present` that nobody has written to starts. A cycle should start where the
/// thing it cycles starts, which is [`next_shape`]'s own reason.
const TONEMAPS: [Tonemap; 4] = [
    Tonemap::Clamp,
    Tonemap::Reinhard,
    Tonemap::Aces,
    Tonemap::AgX,
];

/// How many stops of exposure the track spans, and the whole of what decides
/// its two ends: 1.0 sits at the middle, so the track runs from `2^-6` to `2^6`
/// — a sixty-fourth to sixty-four.
///
/// It is `karakuri-cli`'s own interactive range, restated rather than shared
/// because `EXPOSURE_MIN` and `EXPOSURE_MAX` are private to that binary and
/// this crate depends on nothing that could reach them. What is stated there is
/// *"the interactive bounds, and only the interactive ones"* — the bounds a key
/// nudges inside, where `--exposure` is deliberately not held to them because
/// *"a batch render asks for something extreme on purpose"*. A control under a
/// hand is exactly the interactive case, so this is the same span for the same
/// reason.
const EXPOSURE_STOPS: f32 = 12.0;

/// The two ends of that span, which is [`exposure_at`] at each end of the
/// track. Written out so a reader meets the numbers rather than an exponent,
/// and asserted against the derivation in `tests/look.rs`.
pub const EXPOSURE_MIN: f32 = 1.0 / 64.0;
pub const EXPOSURE_MAX: f32 = 64.0;

/// How many presses of an exposure key cross the whole span.
///
/// `karakuri-cli`'s `EXPOSURE_STEP` is `1.189_207`, which is `2^(1/4)`, and it
/// says why it is a ratio rather than an addition: *"a stop is a ratio, and an
/// additive step would be enormous at 0.1 and invisible at 8.0. This is a
/// quarter of a stop, near enough."* Four presses to a stop over
/// [`EXPOSURE_STOPS`] stops is forty-eight.
const EXPOSURE_PRESSES: f32 = EXPOSURE_STOPS * 4.0;

/// The track width in pixels (one pixel per exposure press, 48px total per P-0094).
pub const EXPOSURE_TRACK_W: f32 = EXPOSURE_PRESSES;

/// What the look controls read this frame: the operator that is running, and
/// the level going into it.
///
/// # The same seam as [`View::transport`], and it is not the Master bay's `out`
///
/// The look is the engine's — `karakuri_engine::frame::Look`, which is exactly
/// what `Present::set_tonemap` is told — and `src/` has no engine (ADR-0156),
/// so whoever owns one reads it and writes this per frame.
///
/// `white_point` is not here, and it is not an oversight: it is in the record,
/// it is Reinhard's parameter alone, and no surface has a control for it, so a
/// field here would be a value this crate carries and no control names. That is
/// the vocabulary's own line — `Operation::SetExposure` asks for the level and
/// nothing else, and whoever writes the record fills the other two thirds in
/// from the look that is running
/// ([ADR-0192](../../../../docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)).
///
/// And `exposure` here is the tone mapper's, never `Deck::set_out`. The two are
/// levels that multiply in different places — the master out where the mix
/// writes the composited frame, this where the present pass reads it — and with
/// nothing in the master chain they are indistinguishable in every frame this
/// program can draw
/// ([ADR-0224](../../../../docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md)).
/// The Master bay's row is the other one and nothing can move it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// The transfer that is running. One of [`TONEMAPS`].
    pub tonemap: Tonemap,
    /// The level going into it. Unbounded on the way in — a `--exposure` flag is
    /// not held to the track's ends — and drawn wherever [`unit_of`] puts it, which
    /// is at an end for anything past one.
    pub exposure: f32,
}

/// The two look controls, laid out: the capsule that names the operator, and
/// the track that sets the level.
///
/// # One derivation, for [`Outputs`]' reason
///
/// [`View::draw`] paints exactly these rectangles and [`crate::input::claim`]
/// hit-tests exactly these rectangles. Two copies of the arithmetic is a track
/// that lights under a pointer that cannot set it.
///
/// # Two controls and one type, because they are one group in the row
///
/// They are two operations and two presses
/// ([ADR-0192](../../../../docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)),
/// and they are laid out together because the second's place is measured from
/// the first's — which is what a flex row is. Splitting them into two functions
/// would mean measuring the capsule twice, once to draw it and once to find out
/// where the track starts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LookRow {
    /// The capsule, which is what a press has to land in to move the tone map on.
    ///
    /// As wide as the widest of the four names whatever word is in it, which is
    /// [`StripBox::tally`]'s rule and not the blend chip's: a target sized to the
    /// word it is showing moves under the hand that is pressing it, and here it
    /// would take the exposure track with it.
    pub tone: Rect,
    /// Where `tone · aces` is painted, inside the capsule's padding. Narrower than
    /// the capsule for every operator but the widest, so the words are centred in
    /// it.
    pub tone_text: Rect,
    /// The faint `exp` before the track.
    pub label: Rect,
    /// The track, `.fader`'s 5px well lying down at [`EXPOSURE_TRACK_W`] long.
    pub track: Rect,
    /// What the level fills of it, from the left — [`unit_of`] of the exposure.
    pub fill: Rect,
    /// What a press has to land in to set the exposure: the track grown to a line's
    /// height and no wider.
    ///
    /// Grown, because a 5px-tall target is not something a hand finds — the
    /// `.mini`'s own argument (*"a 15px word is not something a hand finds, and
    /// `.mini`'s padding is what makes it one"*), met here by a band rather than by
    /// drawing a fatter track. No wider, because the value a press asks for is
    /// where along the track it landed, and a target that reached past either end
    /// would have two pixels of itself asking for the same value.
    pub grip: Rect,
    /// `1.00`, after the track. See [`look`] for why it is after it.
    pub value: Rect,
    /// The values these rectangles were measured from, carried for
    /// [`TransportRow::values`]' reason: whoever measured the type and whoever
    /// paints it are one statement.
    pub values: Look,
}

impl LookRow {
    /// Whether `p` is on the tone map's capsule.
    pub fn hit_tone(&self, p: karakuri_layout::Point) -> bool {
        self.tone.contains(Pos2::new(p.x, p.y))
    }

    /// Whether `p` is on the exposure track's band. See [`LookRow::grip`].
    pub fn hit_exposure(&self, p: karakuri_layout::Point) -> bool {
        self.grip.contains(Pos2::new(p.x, p.y))
    }

    /// Whether `p` is on either of the two, which is what [`crate::input::claim`]
    /// asks.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.hit_tone(p) || self.hit_exposure(p)
    }

    /// What a press at `p` asks the tone map to become, or `None` where there is no
    /// capsule under it.
    ///
    /// # The capsule cycles, and the operation names where it arrived
    ///
    /// Click it and the look moves to the next of [`TONEMAPS`] — `clamp`,
    /// `reinhard`, `aces`, `agx`, wrapping — and what comes out is
    /// [`Operation::SetTonemap`] naming the destination, never a step, because
    /// there is no step in the vocabulary to name. The affordance is
    /// [`Mixer::blend`]'s exactly
    /// ([ADR-0187](../../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md))
    /// and so is the division it rests on: the cycle is [`next_tonemap`] here and
    /// nothing at all in `karakuri-operation`, which is P-0090's division: a toggle
    /// is an affordance, built over operations by whoever draws the control.
    ///
    /// What a map is offered is the four values, not the cycle — the same sentence
    /// ADR-0187 wrote about three, and the reason the manual's row says a map or a
    /// model *"would name the one it wants"*.
    ///
    /// # Why not a menu, now that there is one
    ///
    /// ADR-0187 could not have a popup: the blend chip is in a 53-wide strip, and a
    /// card had nowhere to be and no rule to be modal under. Both of those changed
    /// ([ADR-0225](../../../../docs/adr/0225-a-menu-is-a-gesture-in-hand-rather-than-a-rectangle-on-the-panel.md)),
    /// so the alternative is genuinely available here and it is still refused. A
    /// menu is a gesture in hand: it takes every pointer event on the console until
    /// it is shut, and a boundary cannot be dragged while it is down. That is a
    /// price worth paying for a list of files an operator named and did not pay for
    /// four words that never change — and it puts a second press in front of a
    /// control that is pressed on the beat.
    pub fn tonemap(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_tone(p).then(|| Operation::SetTonemap {
            tonemap: next_tonemap(self.values.tonemap),
        })
    }

    /// What a press at `p` asks the exposure to become, or `None` where there is no
    /// track under it.
    ///
    /// # The press sets it outright, and that is the decision
    ///
    /// Where along the track the press landed *is* the value, through
    /// [`exposure_at`], and what comes out is [`Operation::SetExposure`] naming it.
    /// The manual is where the words for that come from — *"the value is absolute
    /// and the keys are the nudge … because a control that could only be nudged is
    /// a control no fader can reach"* — and the whole span is reachable in one
    /// press.
    ///
    /// It is not [`Mixer::grab`], and the difference is not an inconsistency. A
    /// fader answers `None` for a press on its *track*, on purpose: it has a knob,
    /// the knob keeps the offset it was taken hold of at, and a press that jumped
    /// the value would move the mix under a hand mid-gesture. There is no knob here
    /// and no gesture to be mid-way through — the press is the whole of it — so the
    /// two rules are answers to two different questions rather than one rule
    /// applied twice. Nothing is drawn that looks like a handle, for the same
    /// reason.
    ///
    /// [`crate::input`]'s *a control claims what it acts on and no more* is kept
    /// exactly. Every point of [`LookRow::grip`] asks for a value, so nothing is
    /// claimed in order to be thrown away.
    ///
    /// # It says one thing per press, so it needs no coalescing
    ///
    ///
    /// [ADR-0207](../../../../docs/adr/0207-a-continuous-control-says-one-thing-per-frame.md)
    /// coalesces a swept MIDI fader to one operation a frame, and names the
    /// console's own fader as the other precedent — which emits only when the value
    /// changed, because it holds the value it last drew. Neither applies to a
    /// press: one press is one operation, and a second press in the same frame is a
    /// second thing an operator asked for.
    ///
    /// # And it can never be pending, which is why nothing marks a destination
    ///
    ///
    /// [ADR-0206](../../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)
    /// gives a fader a mark for where a scheduled move is going.
    /// `karakuri_engine::deck::Control` is per slot — a gain, an opacity and a mask
    /// front — so there is no transition that can be scheduled on the look at all,
    /// and a mark here would be a presentation of a state that cannot occur.
    pub fn exposure(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_exposure(p).then(|| Operation::SetExposure {
            exposure: exposure_at((p.x - self.track.min.x) / self.track.width()),
        })
    }
}

/// The next tone map round the cycle, wrapping from the last back to the first
/// — the whole of the affordance the capsule is.
///
/// [`after`]'s division, one row up: the cycle is four lines here and nothing
/// in `karakuri-operation`, which owns the four operators and not the order a
/// pointer walks them in (P-0090).
///
/// A match rather than an index into [`TONEMAPS`], for [`after`]'s reason: a
/// fifth operator does not compile until somebody says what follows it. The
/// price is that the order is written twice — here and in [`TONEMAPS`] — so
/// `tests/look.rs` walks the array through this and asserts they are the same
/// cycle, which is `tests/blend.rs`'s own measurement.
///
/// `karakuri-cli` has this function over the engine's `TonemapOp` and in this
/// order. Two crates naming the same order is what P-0090 costs a vocabulary
/// that depends on nothing, and the test above is what stops the two drifting
/// on this side.
pub(crate) fn next_tonemap(tonemap: Tonemap) -> Tonemap {
    match tonemap {
        Tonemap::Clamp => Tonemap::Reinhard,
        Tonemap::Reinhard => Tonemap::Aces,
        Tonemap::Aces => Tonemap::AgX,
        Tonemap::AgX => Tonemap::Clamp,
    }
}

/// What a point `unit` of the way along the track asks for, in exposure.
///
/// `2^((unit − ½) · stops)`: the middle of the track is `2^0`, which is exactly
/// 1.0, and the two ends are exactly [`EXPOSURE_MIN`] and [`EXPOSURE_MAX`] —
/// the subtraction is exact at a half and `exp2` of a whole number is a power
/// of two with no rounding in it.
///
/// # Logarithmic, and that is not a preference
///
/// Exposure is a ratio: `karakuri-cli`'s step is a *factor* and says why — *"an
/// additive step would be enormous at 0.1 and invisible at 8.0"*. A linear
/// track is that same mistake drawn in space rather than in time: over
/// [`EXPOSURE_MIN`] to [`EXPOSURE_MAX`] unity would sit 0.75 of a pixel from
/// the left-hand end of [`EXPOSURE_TRACK_W`], so the half of the range that
/// darkens the picture would be one pixel wide and the other half would be the
/// whole control. Equal distances along this track are equal ratios, which is
/// the only reading under which both halves are usable.
pub fn exposure_at(unit: f32) -> f32 {
    ((crate::panel::unit(unit) - 0.5) * EXPOSURE_STOPS).exp2()
}

/// Where an exposure sits on the track, on `[0, 1]` — [`exposure_at`] inverted,
/// and clamped to the ends.
///
/// A value past either end is drawn at that end, which is deliberate and is
/// stated rather than hidden: `--exposure 200` is accepted unclamped by the
/// flag on purpose, and a fill that ran off the track would be a picture of a
/// value nobody could point at. The figure beside the track is what says the
/// number in that case, which is [`LookRow::value`]'s whole job. Zero and
/// negative — which the flag refuses everywhere — take `log2` to negative
/// infinity and land at the floor rather than anywhere surprising, and a NaN is
/// zero for [`crate::panel::unit`]'s reason.
pub fn unit_of(exposure: f32) -> f32 {
    crate::panel::unit(exposure.log2() / EXPOSURE_STOPS + 0.5)
}

/// The look controls, derived: the capsule that names the operator, the track
/// that sets the level, and the figure beside it.
///
/// # Where they sit, and why it is after the arrangement pill
///
/// `docs/manual/console.html`'s `.transport` is a flex row and these are the
/// last two items in it before the `.sep`, immediately after `arr · night ▾`.
/// The page's own argument for the place is that this row is already four
/// groups with a reason each — the clock, the four things that need an audio
/// input, the three that say what this console is *set up* as, and health past
/// the spacer — and a look is none of them: it needs no input, it is no file,
/// and it is played rather than arranged. So it is a fourth group at the end of
/// the left half, and the row then reads in the order a frame does, since the
/// tone map is the one transfer the pipeline ends in, applied immediately
/// before the single encode at the output
/// ([P-0064](../../../../docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md),
/// [ADR-0037](../../../../docs/adr/0037-tone-mapping-is-a-uniform-and-the-default-is-chosen-by-looking.md)).
///
/// Nothing already drawn moves, which was a consequence rather than the reason
/// and is worth reading with its date on it: this said [`arrangement`] was
/// *"still one [`size::TRANSPORT_GAP`] after the bar"*, and it is not, as of
/// 2026-09-08 — [`tracker_group`] draws three of the items the mock puts
/// between the two, and the pill is laid out from the last of them. What has
/// not changed is that this group is measured from the pill's right edge, so
/// where it sits is still one answer read one item further along.
///
/// # The order inside the group, and where the figure goes
///
/// The operator first and its level second, which is `Record::Look`'s own order
/// and the manual's row order — the exposure is *the level going into that
/// transfer*, so it reads as belonging to the thing on its left.
///
/// The figure is after the track, and that is the one piece of this layout
/// decided by what a hand does rather than by what the mock draws. It is the
/// only part of the group whose width moves with its value, so putting it last
/// leaves the capsule and the track exactly where they were: a target that
/// walked away from the pointer as it was set would be worst at the moment it
/// was being used most precisely. The capsule ahead of it is held to the widest
/// of the four names for the same reason ([`LookRow::tone`]).
///
/// # No row and no pill means none of this
///
/// `None` wherever [`arrangement`] answers `None` — which is wherever
/// [`transport`] does — and `None` for a console with no engine behind it,
/// which is every test in this crate that does not hand a look in. The place is
/// measured from the pill's right edge, so a console that cannot draw the pill
/// cannot draw what comes after it either; that is one answer to *does this row
/// fit* rather than a second one.
///
/// # What it costs to ask
///
/// Six galley lookups of its own: the four operator names, because the capsule
/// is as wide as the widest of them; the `exp` label; and the figure.
///
/// And it re-derives the row and the pill, which is the honest cost of being
/// laid out from something else's right edge rather than from the row's:
/// [`transport`] is asked twice more per frame and [`arrangement`] once more,
/// where a paint-as-you-go pass that carried a cursor along the row would ask
/// each once. That is deliberate and it is [`Outputs`]' rule — the derivation
/// that draws a control is the one that hit-tests it, so a control cannot be
/// painted anywhere a press cannot reach — and it is what makes every rectangle
/// here something `tests/look.rs` can ask about without a device. The measured
/// price of the whole panel pass is a median 1518 allocations a frame
/// (`crates/karakuri`'s `WRITTEN_ALLOCS`, taken 2026-08-31); this adds on the
/// order of forty, which is inside the factor of two that file will quote a
/// figure across and is why the figure has not been re-taken here — re-taking
/// one needs a window and three seconds of nobody touching it.
///
/// Paid on a pointer event and on a frame, and a console with no look behind it
/// pays none of it: the `look?` is the first line.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one.
// **Eight, where clippy's line is seven**, and it went past it when the row
// gained `learn` and `map`. Every one is a thing the *row before this one* is
// laid out from — the tempo, the input, the tracker, the map, the arrangement
// — and this group is the last item in a flex row, so it is laid out from all
// of them. A struct carrying them would be `View` itself with the fields no
// pill in this row reads left out, which is `App::new`'s sentence one crate
// along.
#[allow(clippy::too_many_arguments)]
pub fn look(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    audio: Option<&AudioIn>,
    tracker: Option<Tracker>,
    // Handed straight to [`arrangement`], which is the item before this one
    // and the only reason this needs it — see that function's own note.
    map: Option<&MapPill>,
    arr: &Arrangement,
    look: Option<Look>,
) -> Option<LookRow> {
    let look = look?;
    // The pill, asked once and for both things it answers: whether the row
    // exists at all, and where the group before this one ended. `transport`
    // answers the first for the whole module and `arrangement` is laid out
    // from it, so this is that one answer read one item further along.
    let pill = arrangement(ctx, layout, values, audio, tracker, map, arr)?;
    let row = transport(ctx, layout, values)?;
    let strip = to_egui(layout.rect(layout.find("transport")?));
    let width = |text: &str| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size::BASE, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };

    let mid = strip.center().y;
    let span_h = size::BASE * size::LINE;
    // The capsule is the widest of the four whatever it says — see
    // `LookRow::tone`.
    let widest = TONEMAPS
        .iter()
        .map(|op| width(&tone_text(*op)))
        .fold(0.0, f32::max);
    let tone = Rect::from_min_size(
        Pos2::new(
            pill.pill.max.x + size::TRANSPORT_GAP,
            mid - size::PILL_H * 0.5,
        ),
        egui::vec2(size::PILL_PAD_X * 2.0 + widest, size::PILL_H),
    );
    // Centred in a capsule it does not fill, which is what *as wide as the
    // widest* comes to for the other three.
    let words = tone_text(look.tonemap);
    let tone_text = Rect::from_center_size(
        Pos2::new(tone.center().x, mid),
        egui::vec2(width(&words), size::PILL_H),
    );

    let label = Rect::from_min_size(
        Pos2::new(tone.max.x + size::TRANSPORT_GAP, mid - span_h * 0.5),
        egui::vec2(width(EXPOSURE_LABEL), span_h),
    );
    let track = Rect::from_min_size(
        Pos2::new(label.max.x + size::TRIM_GAP, mid - size::FADER_H * 0.5),
        egui::vec2(EXPOSURE_TRACK_W, size::FADER_H),
    );
    let grip = Rect::from_min_max(
        Pos2::new(track.min.x, mid - size::PILL_H * 0.5),
        Pos2::new(track.max.x, mid + size::PILL_H * 0.5),
    );
    let value = Rect::from_min_size(
        Pos2::new(track.max.x + size::TRIM_GAP, mid - span_h * 0.5),
        egui::vec2(width(&exposure_text(look.exposure)), span_h),
    );

    // The rule `arrangement` and `outputs_row` both state: a control that does
    // not fit in the row it is drawn in is no control at all, rather than half
    // of one over the frame readout.
    if !strip.contains_rect(tone)
        || !strip.contains_rect(grip)
        || value.max.x + size::TRANSPORT_GAP > row.frame.min.x
    {
        return None;
    }

    Some(LookRow {
        tone,
        tone_text,
        label,
        track,
        fill: filled(track, Axis::Row, unit_of(look.exposure)),
        grip,
        value,
        values: look,
    })
}

/// The capsule's words: `tone · aces`.
///
/// One string rather than two galleys laid end to end, for [`pill_text`]'s
/// reason: the mock writes one run of text and laying it out as one keeps the
/// spaces round the `·` the type's own.
fn tone_text(tonemap: Tonemap) -> String {
    format!("{TONEMAP_LABEL} · {}", tonemap.name())
}

/// The level, as the Master bay's own row writes it — `out 1.00`, two places.
/// The two are the same kind of reading and are deliberately written the same
/// way; where they stop being the same *number* is
/// [ADR-0224](../../../../docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md).
fn exposure_text(exposure: f32) -> String {
    format!("{exposure:.2}")
}

/// The look controls, painted.
///
/// Where everything goes is [`look`]'s, so this paints and derives nothing.
/// Term for term from `style.css`:
///
/// - `.pill` — the capsule, which is [`arrangement_into`]'s treatment and this
///   is the same pill three items along. No `▾`, because nothing opens.
/// - the `exp` before the track — `style="color:var(--c-faint)"` in the
///   markup, which is `pal.faint`, and it is `.trim .lbl`'s job one bay up.
/// - `.fader` — `background: var(--c-well)` with
///   `box-shadow: inset 0 0 0 1px var(--c-hair)` and `border-radius: 999px`,
///   which on a 5px box is a capsule.
/// - `.fader b` — `linear-gradient(90deg, var(--c-mint), var(--c-lav))`,
///   through [`gradient`], which is the one place that ramp is drawn and is
///   what the mixer's own faders are painted with.
/// - the figure — `.val`, `pal.text`, *a value*.
///
/// `.fader s` is deliberately not drawn. The mock's knob is a handle and
/// this control has none — see [`LookRow::exposure`].
pub(crate) fn look_into(ui: &Ui, pal: &Palette, row: &LookRow) {
    let painter = ui.painter();
    painter.rect_stroke(
        row.tone,
        CornerRadius::same((size::PILL_H * 0.5) as u8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    let centred = |rect: Rect, galley: std::sync::Arc<egui::Galley>, colour: Color32| {
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            colour,
        );
    };
    let words = tone_text(row.values.tonemap);
    centred(
        row.tone_text,
        painter.layout_no_wrap(
            words,
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.dim,
        ),
        pal.dim,
    );
    centred(
        row.label,
        painter.layout_no_wrap(
            EXPOSURE_LABEL.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.faint,
        ),
        pal.faint,
    );

    slider_track_into(painter, pal, row.track, row.fill, Axis::Row);

    centred(
        row.value,
        painter.layout_no_wrap(
            exposure_text(row.values.exposure),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.text,
        ),
        pal.text,
    );
}
