use super::*;

// ---------------------------------------------------------------------------
// The tracker group: the latency offset, the tap and the octave
// ---------------------------------------------------------------------------

/// The word before the offset track — [`EXPOSURE_LABEL`]'s arrangement at the
/// other end of this row, and the mock's own word: a faint label, one
/// [`size::TRIM_GAP`], a horizontal [`fader`], another gap and the figure.
const OFFSET_LABEL: &str = "offset";

/// The unit, written into the figure and never left off.
///
/// `docs/manual/console.html` is plain about why: *"Two things on this panel
/// are called an offset, and they are not the same thing … The unit is what
/// tells them apart, so neither is ever drawn without it."* The other is the
/// deck head's anchor, which is in beats and is one deck's.
const OFFSET_UNIT: &str = " ms";

/// What is written in the tap capsule, and the whole of it — the mock writes
/// one word with no value beside it, because a tap has no state to read back.
/// It is [`TONEMAP_LABEL`]'s kind of capsule and not [`AUDIO_LABEL`]'s: no `▾`,
/// because nothing opens.
const TAP_LABEL: &str = "tap";

/// The two halves of the octave, in the mock's own marks — `&frac12;` and
/// `&times;2`, which are one character and two.
const HALVE_MARK: &str = "½";
const DOUBLE_MARK: &str = "×2";

/// The two ends of the latency offset, in milliseconds.
///
/// `karakuri_environment::audio::LATENCY_OFFSET_RANGE` is `-200.0..=200.0` and
/// this is that range restated, for [`EXPOSURE_STOPS`]' reason one control
/// along: this crate depends on nothing that could reach it (ADR-0156), and a
/// control has to know its own ends to lay a track out.
///
/// The copy is held to the original where both are visible, which is
/// `crates/karakuri` — the binary that has this crate and that one as
/// dependencies — rather than asserted here. That is the difference between
/// this restatement and [`EXPOSURE_STOPS`]': the exposure's ends are private to
/// `karakuri-cli` and nothing can compare them, and these are public.
pub const LATENCY_OFFSET_MIN_MS: f32 = -200.0;
pub const LATENCY_OFFSET_MAX_MS: f32 = 200.0;

/// What one press of an offset key moves it by, in milliseconds —
/// `karakuri_environment::audio::LATENCY_OFFSET_STEP_MS`, restated here for the
/// reason the two ends above are, and held to the original in the same place.
pub const LATENCY_OFFSET_STEP_MS: f32 = 5.0;

/// How many presses of an offset key cross the whole span: four hundred
/// milliseconds at five a press is eighty.
const OFFSET_PRESSES: f32 =
    (LATENCY_OFFSET_MAX_MS - LATENCY_OFFSET_MIN_MS) / LATENCY_OFFSET_STEP_MS;

/// The track, in pixels: one pixel a press — [`EXPOSURE_TRACK_W`]'s whole
/// argument, one control along and on a value that is a difference rather than
/// a ratio
/// ([ADR-0277](../../../../docs/adr/0277-the-latency-offset-is-a-track-because-a-capsule-cannot-name-a-value.md)).
///
/// A pointer on this track can ask for any of the eighty positions along it and
/// `o` and `p` can ask for any of the eighty values between the ends, so
/// neither surface can reach a value the other cannot — which is what stops a
/// track and a pair of keys nearly agreeing. `docs/manual/console.html` carries
/// the same sentence for the exposure and now for this.
pub const OFFSET_TRACK_W: f32 = OFFSET_PRESSES;

/// What a point `unit` of the way along the track asks for, in milliseconds.
///
/// Linear, and that is not the exposure's arithmetic. An exposure is a *ratio*
/// and its track is logarithmic for a stated reason — *"an additive step would
/// be enormous at 0.1 and invisible at 8.0"*. A latency offset is a difference
/// between two arrival times: five milliseconds is five milliseconds at either
/// end of the range, which is why the key steps by an addition and why equal
/// distances along this track are equal numbers of milliseconds. The middle of
/// the track is exactly zero, because the range is symmetric and the
/// subtraction is exact at a half.
pub fn offset_at(at: f32) -> f32 {
    LATENCY_OFFSET_MIN_MS + unit(at) * (LATENCY_OFFSET_MAX_MS - LATENCY_OFFSET_MIN_MS)
}

/// Where an offset sits on the track, on `[0, 1]` — [`offset_at`] inverted, and
/// clamped to the ends for [`unit_of`]'s reason: a value past either end is
/// drawn at that end and the figure beside the track is what says the number.
/// Nothing on the way in is held to these ends — the clamp is
/// `karakuri_environment::audio`'s, at the one place an offset is applied — so
/// this is drawing a reading rather than enforcing a bound.
pub fn unit_of_offset(ms: f32) -> f32 {
    unit((ms - LATENCY_OFFSET_MIN_MS) / (LATENCY_OFFSET_MAX_MS - LATENCY_OFFSET_MIN_MS))
}

/// What the three controls beside the audio-in pill read this frame: the offset
/// the session is holding, and which way the grid can still be moved an octave.
///
/// # The same seam as [`View::transport`], and it is the beat lock's rather
/// than the engine's
///
/// Every field is a number or a `bool` and none of them is a device. Whether an
/// octave is available is `karakuri_audio`'s `BPM_RANGE` against the tempo the
/// session is running at, and the offset is a value
/// `karakuri_environment::audio::Audio` holds — `src/` has neither (ADR-0156),
/// so whoever owns them reads them and writes this per frame, exactly as
/// whoever owns the engine writes [`View::look`].
///
/// It carries what cannot be derived and nothing that can. The console holds
/// the tempo already — [`Transport::bpm`] — but not the range the tracker
/// searches, so *which half is live* is one of the two things it cannot work
/// out for itself. `docs/manual/console.html` says the panel can work it out
/// *before the press*, and this is what makes that true.
///
/// A second value rather than three more fields on [`Transport`], for
/// [`Look`]'s reason: a tempo and a frame cost are what the session is doing,
/// and these are what the *tracker* is doing. A console driving one and not the
/// other is a state the type should be able to say.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tracker {
    /// The latency offset the open session is holding, in milliseconds, or `None`
    /// where nothing is open.
    ///
    /// `None` draws no offset control at all rather than a track at a plausible
    /// figure, which is [`View::audio`]'s own rule one control to the left: an
    /// offset belongs to a session, and until one is open there is no value being
    /// held anywhere for a track to point at. The page says so.
    pub offset_ms: Option<f32>,
    /// Whether halving the grid is available at this tempo — the tracker's range
    /// against half of what the session is running at.
    pub halve: bool,
    /// And whether doubling is. Never both: the range is under two octaves wide,
    /// which is the mock's own note and is why this is two fields rather than a
    /// direction.
    pub double: bool,
}

/// The offset control, laid out: the word, the track, what the value fills of
/// it, the band a press has to land in, and the figure.
///
/// [`LookRow`]'s exposure half, on a linear value — and a type of its own
/// because it is the one part of [`TrackerGroup`] that is not always there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OffsetTrack {
    /// The faint `offset` before the track.
    pub label: Rect,
    /// The track, `.fader`'s 5px well lying down at [`OFFSET_TRACK_W`] long.
    pub track: Rect,
    /// What the offset fills of it, from the left — [`unit_of_offset`].
    pub fill: Rect,
    /// What a press has to land in to set the offset: the track grown to a line's
    /// height and no wider, which is [`LookRow::grip`]'s rule and its argument — a
    /// 5px-tall target is not something a hand finds, and a target reaching past
    /// either end would have two pixels of itself asking for the same value.
    pub grip: Rect,
    /// `−15 ms`, after the track. See [`tracker_group`] for why it is after it.
    pub value: Rect,
}

/// The three controls that finish the tracker group, laid out: the offset, the
/// tap and the two halves of the octave.
///
/// # One derivation, for [`LookRow`]'s reason
///
/// [`View::draw`] paints exactly these rectangles and [`crate::input::claim`]
/// hit-tests exactly these rectangles. They are laid end to end from the
/// audio-in pill's right edge, so where the octave is depends on how wide the
/// offset's figure is — three questions about one laid-out group, and a second
/// walk would put the chip a press lands on somewhere the mark is not.
///
/// # Why they are one group and not three controls in a row
///
/// `docs/manual/style.css` says it at `.tracker`, which is the class the mock
/// puts round exactly these four: *"the pill that says whether there is an
/// audio input, the latency offset … the tap, and the octave. None of them
/// means anything without an input, so they are one item at the pills' own 5px
/// rather than four at the row's 14."* So the gap inside this group is
/// [`size::PILL_GAP`] and the gap between the group and what follows it is
/// [`size::TRANSPORT_GAP`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackerGroup {
    /// The offset, or `None` where no session is open to be holding one.
    pub offset: Option<OffsetTrack>,
    /// The tap capsule, which is what a press has to land in to tap the beat. Drawn
    /// whatever the audio-in pill says, because what it asks for does not depend on
    /// a reading — a tap with no room to tap against is refused out loud, which is
    /// `Operation::TapBeat`'s own row on `docs/manual/operations.html`.
    pub tap: Rect,
    /// Where `tap` is painted, inside the capsule's padding.
    pub tap_text: Rect,
    /// The `½` half, drawn whether or not it is live.
    pub halve: Rect,
    /// The `×2` half. At most one of the two is ever live, which is the range being
    /// under two octaves wide, and the inert one keeps its shape rather than going
    /// away — `.octave i.idle`, and the deck head's scrub arrows one bay over.
    pub double: Rect,
    /// The values these rectangles were measured from, carried for
    /// [`TransportRow::values`]' reason: whoever measured the type and whoever
    /// paints it are one statement.
    pub values: Tracker,
}

impl TrackerGroup {
    /// Whether `p` is on the tap capsule.
    pub fn hit_tap(&self, p: karakuri_layout::Point) -> bool {
        self.tap.contains(Pos2::new(p.x, p.y))
    }

    /// Whether `p` is on the offset track's band. See [`OffsetTrack::grip`], and
    /// `false` where there is no offset drawn at all.
    pub fn hit_offset(&self, p: karakuri_layout::Point) -> bool {
        self.offset
            .is_some_and(|offset| offset.grip.contains(Pos2::new(p.x, p.y)))
    }

    /// Which half of the octave `p` is on, as the factor it asks for — or `None`
    /// off both, and `None` on either while that direction is refused.
    ///
    /// Inert is not claimed, which is [`DeckHead::arrow`]'s rule word for word and
    /// [`crate::input`]'s *a control claims what it acts on and no more*. Both
    /// halves are drawn on every tempo, because the range is under two octaves wide
    /// and a pair that vanished would move the rest of the row under the hand every
    /// time the grid crossed 100 or 120 BPM.
    fn half(&self, p: karakuri_layout::Point) -> Option<GridScale> {
        let p = Pos2::new(p.x, p.y);
        match (
            self.values.halve && self.halve.contains(p),
            self.values.double && self.double.contains(p),
        ) {
            (true, _) => Some(GridScale::Halve),
            (_, true) => Some(GridScale::Double),
            _ => None,
        }
    }

    /// Whether `p` is on any of the three, which is what [`crate::input::claim`]
    /// asks.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.hit_tap(p) || self.hit_offset(p) || self.half(p).is_some()
    }

    /// What a press at `p` asks for when it lands on the tap capsule:
    /// [`Operation::TapBeat`], which carries nothing because a tap is an instant
    /// and the instant is when the operation arrives.
    ///
    /// It is emitted with no input open too, and that is the decision rather than a
    /// gap: the operations page says the refusal *"says where to open one rather
    /// than doing nothing, because a key that declines and a key that is not bound
    /// are the same experience"*, and the surface that could see there was no room
    /// is not this one — this crate has no device (ADR-0156). So the press names
    /// the operation and whoever performs it says what happened, exactly as `b`
    /// already does.
    pub fn tapped(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_tap(p).then_some(Operation::TapBeat)
    }

    /// What a press at `p` asks the grid to do: [`Operation::ScaleGrid`] naming the
    /// direction, or `None` off both halves and on a refused one.
    pub fn octave(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.half(p).map(|by| Operation::ScaleGrid { by })
    }

    /// What a press at `p` asks the offset to become, or `None` where there is no
    /// track under it.
    ///
    /// # The press sets it outright, and that is the decision
    ///
    /// Where along the track the press landed *is* the value, through
    /// [`offset_at`], and what comes out is [`Operation::SetLatencyOffset`] naming
    /// it. The manual is where the words come from — *"The value is absolute and
    /// the keys are the nudge … because a control that could only be nudged is a
    /// control no fader can reach"* — and it is [`LookRow::exposure`]'s argument
    /// arriving one control earlier, because that control was built from this
    /// sentence.
    ///
    /// It is not [`Mixer::grab`], for [`LookRow::exposure`]'s reason: there is no
    /// knob here and no gesture to be mid-way through, so a press that names a
    /// value cannot move the mix under a hand.
    pub fn nudge(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let offset = self.offset?;
        self.hit_offset(p).then(|| Operation::SetLatencyOffset {
            ms: offset_at((p.x - offset.track.min.x) / offset.track.width()),
        })
    }
}

/// The tracker's other three controls, derived: the offset, the tap and the
/// octave.
///
/// # Where they sit, and why it is after the audio-in pill
///
/// `docs/manual/console.html`'s `.tracker` is the four things that need an
/// input, in this order: the pill that says whether there is one, the offset,
/// the tap, the octave. [`audio_in`] is the head of it and this is the rest,
/// laid out from that pill's right edge at [`size::PILL_GAP`] — the group's own
/// tighter gap and not the row's — which is exactly what [`audio_in`]'s own
/// documentation said would happen the day these were drawn.
///
/// With no pill it lands after the bar, at [`size::TRANSPORT_GAP`], which is
/// where [`arrangement`] used to land and is the same fallback: a console told
/// about the tracker and not about audio is a state nothing in this workspace
/// produces, and it is laid out rather than refused because a derivation that
/// panicked on it would be answering a question about a device on its own
/// authority.
///
/// # The order inside the group, and where the figure goes
///
/// The mock's, unchanged. The figure is after the track for [`look`]'s reason,
/// which is the one piece of this layout decided by what a hand does rather
/// than by what the mock draws: it is the only part whose width moves with its
/// value, so putting it last leaves the track and everything after it where
/// they were, and a target that walked away from the pointer as it was set
/// would be worst at the moment it was being used most precisely.
///
/// # No row and no values means none of this
///
/// `None` wherever [`transport`] answers `None` — the row folded away, soloed
/// away, too narrow, or a console with no engine behind it — and `None` again
/// where `tracker` is `None`, which is a console nobody has told anything about
/// the beat tracker and is every test in this crate that does not say
/// otherwise. Drawing a tap on a console with no tracker behind it would be the
/// scaffolding this module refuses.
///
/// # What it costs to ask
///
/// Five galley lookups of its own, and two of them go with the offset: the
/// `tap` word, the `offset` word, the figure — which is the one whose width
/// moves with its value — and one for each of `½` and `×2`, since a capsule
/// here is as wide as the mark in it. A console with nothing open pays three of
/// the five.
///
/// And it re-derives the row and the pill, which is [`look`]'s honest cost one
/// group to the left and for the same reason: the derivation that draws a
/// control is the one that hit-tests it.
///
/// Paid on a pointer event and on a frame, and a console with no tracker behind
/// it pays none of it: the `tracker?` is the first line.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one.
pub fn tracker_group(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    audio: Option<&AudioIn>,
    tracker: Option<Tracker>,
) -> Option<TrackerGroup> {
    let tracker = tracker?;
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
    let mark = |text: &str| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size::OCTAVE_SIZE, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };

    let mid = strip.center().y;
    let span_h = size::BASE * size::LINE;
    // **Where the group's head ended.** The pill where the console has been
    // told about audio, and the bar where it has not — [`arrangement`]'s own
    // one-answer arrangement, read one item earlier in the same row.
    let (after, gap) = match audio_in(ctx, layout, values, audio) {
        Some(pill) => (pill.pill.max.x, size::PILL_GAP),
        None => (row.bar.max.x, size::TRANSPORT_GAP),
    };

    // The offset, where a session is open to be holding one.
    let mut at = after + gap;
    let offset = tracker.offset_ms.map(|ms| {
        let label = Rect::from_min_size(
            Pos2::new(at, mid - span_h * 0.5),
            egui::vec2(width(OFFSET_LABEL), span_h),
        );
        let track = Rect::from_min_size(
            Pos2::new(label.max.x + size::TRIM_GAP, mid - size::FADER_H * 0.5),
            egui::vec2(OFFSET_TRACK_W, size::FADER_H),
        );
        let grip = Rect::from_min_max(
            Pos2::new(track.min.x, mid - size::PILL_H * 0.5),
            Pos2::new(track.max.x, mid + size::PILL_H * 0.5),
        );
        let value = Rect::from_min_size(
            Pos2::new(track.max.x + size::TRIM_GAP, mid - span_h * 0.5),
            egui::vec2(width(&offset_text(ms)), span_h),
        );
        at = value.max.x + size::PILL_GAP;
        OffsetTrack {
            label,
            track,
            fill: filled(track, Axis::Row, unit_of_offset(ms)),
            grip,
            value,
        }
    });

    let tap_w = size::PILL_PAD_X * 2.0 + width(TAP_LABEL);
    let tap = Rect::from_min_size(
        Pos2::new(at, mid - size::PILL_H * 0.5),
        egui::vec2(tap_w, size::PILL_H),
    );
    let tap_text = Rect::from_min_size(
        Pos2::new(tap.min.x + size::PILL_PAD_X, mid - size::PILL_H * 0.5),
        egui::vec2(width(TAP_LABEL), size::PILL_H),
    );

    let chip = |x: f32, text: &str| {
        Rect::from_min_size(
            Pos2::new(x, mid - size::OCTAVE_H * 0.5),
            egui::vec2(
                mark(text) + size::OCTAVE_PAD_X * 2.0 + size::HAIRLINE * 2.0,
                size::OCTAVE_H,
            ),
        )
    };
    let halve = chip(tap.max.x + size::PILL_GAP, HALVE_MARK);
    let double = chip(halve.max.x + size::OCTAVE_GAP, DOUBLE_MARK);

    // The rule [`arrangement`], [`look`] and `outputs_row` all state: a control
    // that does not fit in the row it is drawn in is no control at all, rather
    // than half of one over the frame readout.
    if !strip.contains_rect(tap)
        || !strip.contains_rect(double)
        || double.max.x + size::TRANSPORT_GAP > row.frame.min.x
    {
        return None;
    }

    Some(TrackerGroup {
        offset,
        tap,
        tap_text,
        halve,
        double,
        values: tracker,
    })
}

/// The figure after the track: the offset, signed both ways and never without
/// its unit — `−15 ms`, and `+0 ms` at the middle.
///
/// The sign is always drawn, which is the page's own instruction: *"the sign is
/// the half that gets read wrong at two in the morning, so the panel says it in
/// words rather than leaving −15 ms to be interpreted."* The words are
/// `karakuri`'s, in the line it prints; the mark is this control's, and a `+`
/// that only appeared above zero would be a control that says less exactly
/// where it matters.
///
/// Whole milliseconds, because the step is five of them and the ends are two
/// hundred: a decimal place here would be a precision no surface can ask for.
///
/// The sign is written in ASCII, as the deck head's anchor writes its own
/// signed offset — the `−` in the mock and in this comment is the page's
/// typography, and the panel paints what a `{:+}` writes.
///
/// And the rounding is taken before the sign, so there is no `-0 ms`. A press a
/// fraction of a pixel left of the middle asks for a value that rounds to zero,
/// and `{:+.0}` writes `-0` for it: two spellings of the one value an operator
/// is most likely to be aiming at.
fn offset_text(ms: f32) -> String {
    // `-0.0 == 0.0` in IEEE, so this is the whole of the normalisation.
    let whole = match ms.round() == 0.0 {
        true => 0.0,
        false => ms.round(),
    };
    format!("{whole:+.0}{OFFSET_UNIT}")
}

/// The tracker's three controls, painted.
///
/// Where everything goes is [`tracker_group`]'s, so this paints and derives nothing.
/// Term for term from `style.css`:
///
/// - the `offset` before the track — `style="color:var(--c-faint)"` in the
///   markup, which is `pal.faint`, and is [`look_into`]'s treatment of `exp`.
/// - `.fader` and `.fader b` — the well and the ramp [`look_into`] paints, and
///   `.fader s` is deliberately not drawn for its reason: the mock's knob is a
///   handle and this control has none.
/// - `.pill` — the tap capsule, which is [`arrangement_into`]'s treatment with
///   no `▾`.
/// - `.octave i` — `border: 1px solid var(--c-line); color: var(--c-dim)` at
///   `border-radius: 999px`, and `.octave i.idle` is `color: var(--c-faint);
///   border-color: var(--c-hair)`. Never grey without a reason is the
///   stylesheet's own comment, and the reason is on the mock's tooltip.
pub(crate) fn tracker_into(ui: &Ui, pal: &Palette, group: &TrackerGroup) {
    let painter = ui.painter();
    let centred = |rect: Rect, galley: std::sync::Arc<egui::Galley>, colour: Color32| {
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            colour,
        );
    };

    if let Some(offset) = group.offset {
        centred(
            offset.label,
            painter.layout_no_wrap(
                OFFSET_LABEL.to_owned(),
                FontId::new(size::BASE, FontFamily::Proportional),
                pal.faint,
            ),
            pal.faint,
        );
        slider_track_into(painter, pal, offset.track, offset.fill, Axis::Row);
        centred(
            offset.value,
            painter.layout_no_wrap(
                offset_text(group.values.offset_ms.unwrap_or_default()),
                FontId::new(size::BASE, FontFamily::Proportional),
                pal.text,
            ),
            pal.text,
        );
    }

    painter.rect_stroke(
        group.tap,
        CornerRadius::same((size::PILL_H * 0.5) as u8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    centred(
        group.tap_text,
        painter.layout_no_wrap(
            TAP_LABEL.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.dim,
        ),
        pal.dim,
    );

    let half = |rect: Rect, text: &str, live: bool| {
        let (ink, edge) = match live {
            true => (pal.dim, pal.line),
            false => (pal.faint, pal.hair),
        };
        painter.rect_stroke(
            rect,
            CornerRadius::same((size::OCTAVE_H * 0.5) as u8),
            Stroke::new(size::HAIRLINE, edge),
            StrokeKind::Inside,
        );
        let galley = painter.layout_no_wrap(
            text.to_owned(),
            FontId::new(size::OCTAVE_SIZE, FontFamily::Proportional),
            ink,
        );
        painter.galley(
            Pos2::new(
                rect.center().x - galley.size().x * 0.5,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            ink,
        );
    };
    half(group.halve, HALVE_MARK, group.values.halve);
    half(group.double, DOUBLE_MARK, group.values.double);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies offset formatting: signed value, units, and no `-0 ms`.
    #[test]
    fn the_offset_figure_is_signed_carries_its_unit_and_has_one_zero() {
        assert_eq!(offset_text(-15.0), "-15 ms");
        assert_eq!(offset_text(15.0), "+15 ms");
        assert_eq!(offset_text(LATENCY_OFFSET_MIN_MS), "-200 ms");
        assert_eq!(offset_text(LATENCY_OFFSET_MAX_MS), "+200 ms");
        // Either side of the middle, and exactly on it.
        assert_eq!(offset_text(0.0), "+0 ms");
        assert_eq!(offset_text(-0.4), "+0 ms");
        assert_eq!(offset_text(0.4), "+0 ms");
        // And the first value on each side that is not zero still says which
        // side it is on, so the normalisation above has not swallowed a sign.
        assert_eq!(offset_text(-0.6), "-1 ms");
        assert_eq!(offset_text(0.6), "+1 ms");
    }
}
