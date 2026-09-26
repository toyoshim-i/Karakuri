use super::*;

// ---------------------------------------------------------------------------
// The tracker group: the latency offset, the tap and the octave
// ---------------------------------------------------------------------------

/// The word before the offset track — [`EXPOSURE_LABEL`]'s arrangement at the
/// other end of this row, and the mock's own word: a faint label, one
/// [`size::TRIM_GAP`], a horizontal [`fader`], another gap and the figure.
const OFFSET_LABEL: &str = "offset";

/// Suffix unit appended to the latency offset readout.
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

/// Latency offset range limits in milliseconds (ADR-0156).
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

/// Width of the latency offset slider track in pixels (ADR-0277).
pub const OFFSET_TRACK_W: f32 = OFFSET_PRESSES;

/// Maps normalized slider position `[0, 1]` to linear millisecond latency offset.
pub fn offset_at(at: f32) -> f32 {
    LATENCY_OFFSET_MIN_MS + unit(at) * (LATENCY_OFFSET_MAX_MS - LATENCY_OFFSET_MIN_MS)
}

/// Maps millisecond latency offset to normalized `[0, 1]` track position.
pub fn unit_of_offset(ms: f32) -> f32 {
    unit((ms - LATENCY_OFFSET_MIN_MS) / (LATENCY_OFFSET_MAX_MS - LATENCY_OFFSET_MIN_MS))
}

/// Transport tracker control state (latency offset, octaving availability) (ADR-0156).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tracker {
    /// Active latency offset in milliseconds, or `None` if unconfigured.
    pub offset_ms: Option<f32>,
    /// Whether halving the grid is available at this tempo — the tracker's range
    /// against half of what the session is running at.
    pub halve: bool,
    /// And whether doubling is. Never both: the range is under two octaves wide,
    /// which is the mock's own note and is why this is two fields rather than a
    /// direction.
    pub double: bool,
}

/// Layout bounding boxes and hit zones for the latency offset track control.
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

/// Layout metrics for the tracker controls group (offset, tap tempo, octave multipliers).
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

    /// Returns the scale factor if `p` lands on an active octave button, or `None`.
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

    /// Emits [`Operation::TapBeat`] if `p` hits the tap tempo capsule (ADR-0156).
    pub fn tapped(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_tap(p).then_some(Operation::TapBeat)
    }

    /// What a press at `p` asks the grid to do: [`Operation::ScaleGrid`] naming the
    /// direction, or `None` off both halves and on a refused one.
    pub fn octave(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.half(p).map(|by| Operation::ScaleGrid { by })
    }

    /// Computes [`Operation::SetLatencyOffset`] based on click position on the offset track.
    pub fn nudge(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let offset = self.offset?;
        self.hit_offset(p).then(|| Operation::SetLatencyOffset {
            ms: offset_at((p.x - offset.track.min.x) / offset.track.width()),
        })
    }
}

/// Computes layout for the tracker group controls (offset track, tap tempo, octave multipliers).
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
    // Anchor following the audio-in pill or the transport bar.
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

/// Formats the millisecond offset with explicit sign and unit (`+0 ms`, `−15 ms`).
fn offset_text(ms: f32) -> String {
    // `-0.0 == 0.0` in IEEE, so this is the whole of the normalisation.
    let whole = match ms.round() == 0.0 {
        true => 0.0,
        false => ms.round(),
    };
    format!("{whole:+.0}{OFFSET_UNIT}")
}

/// Paints the tracker controls (offset track, tap capsule, octave buttons).
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
