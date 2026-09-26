use super::*;

// ---------------------------------------------------------------------------
// The look: the tone map, and the level going into it
// ---------------------------------------------------------------------------

/// The tone map pill's first word, displayed as `tone · <name>`.
const TONEMAP_LABEL: &str = "tone";

/// The word before the exposure track — `.trim`'s `g` one row up, and the same
/// arrangement: a faint label, [`size::TRIM_GAP`], a horizontal [`fader`].
const EXPOSURE_LABEL: &str = "exp";

/// Available tonemapping operators in cycle order (ADR-0037).
const TONEMAPS: [Tonemap; 4] = [
    Tonemap::Clamp,
    Tonemap::Reinhard,
    Tonemap::Aces,
    Tonemap::AgX,
];

/// Number of exposure stops spanned by the slider track (ADR-0156).
const EXPOSURE_STOPS: f32 = 12.0;

/// The two ends of that span, which is [`exposure_at`] at each end of the
/// track. Written out so a reader meets the numbers rather than an exponent,
/// and asserted against the derivation in `tests/look.rs`.
pub const EXPOSURE_MIN: f32 = 1.0 / 64.0;
pub const EXPOSURE_MAX: f32 = 64.0;

/// Number of discrete keypress steps across the exposure slider track.
const EXPOSURE_PRESSES: f32 = EXPOSURE_STOPS * 4.0;

/// The track width in pixels (one pixel per exposure press, 48px total per P-0094).
pub const EXPOSURE_TRACK_W: f32 = EXPOSURE_PRESSES;

/// Current look settings (active tonemap operator and exposure multiplier) (ADR-0156, ADR-0192, ADR-0224).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// The transfer that is running. One of [`TONEMAPS`].
    pub tonemap: Tonemap,
    /// The level going into it. Unbounded on the way in — a `--exposure` flag is
    /// not held to the track's ends — and drawn wherever [`unit_of`] puts it, which
    /// is at an end for anything past one.
    pub exposure: f32,
}

/// Layout metrics for the tonemap selector pill and exposure slider track (ADR-0192).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LookRow {
    /// Bounding rectangle for the tonemap cycle pill.
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
    /// Vertical hit-testing band for the exposure slider track.
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

    /// Advances tonemap to the next operator in [`TONEMAPS`] if `p` clicks the pill (ADR-0187, P-0090).
    pub fn tonemap(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_tone(p).then(|| Operation::SetTonemap {
            tonemap: next_tonemap(self.values.tonemap),
        })
    }

    /// Computes [`Operation::SetExposure`] from the click coordinate along the track.
    pub fn exposure(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_exposure(p).then(|| Operation::SetExposure {
            exposure: exposure_at((p.x - self.track.min.x) / self.track.width()),
        })
    }
}

/// Returns the next tonemapping operator in the cycle order (P-0090).
pub(crate) fn next_tonemap(tonemap: Tonemap) -> Tonemap {
    match tonemap {
        Tonemap::Clamp => Tonemap::Reinhard,
        Tonemap::Reinhard => Tonemap::Aces,
        Tonemap::Aces => Tonemap::AgX,
        Tonemap::AgX => Tonemap::Clamp,
    }
}

/// Converts normalized slider position `[0, 1]` to logarithmic exposure multiplier.
pub fn exposure_at(unit: f32) -> f32 {
    ((crate::panel::unit(unit) - 0.5) * EXPOSURE_STOPS).exp2()
}

/// Converts exposure multiplier to normalized `[0, 1]` track position, clamped to bounds.
pub fn unit_of(exposure: f32) -> f32 {
    crate::panel::unit(exposure.log2() / EXPOSURE_STOPS + 0.5)
}

/// Computes layout rectangles for the tonemap pill and exposure slider track (ADR-0037, P-0064).
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

/// Formats the tonemap pill label as `tone · <name>`.
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

/// Paints the tonemap pill button and exposure slider track with value readout.
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
