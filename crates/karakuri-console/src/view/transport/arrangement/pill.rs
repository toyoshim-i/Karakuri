use super::super::*;

// ---------------------------------------------------------------------------
// learn, and the map it writes into
// ---------------------------------------------------------------------------

/// Readout information for the active MIDI/control surface map file (ADR-0156).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MapPill {
    /// What the map is called — the file's stem, which is the name an operator gave
    /// it and not its path (P-0087).
    pub name: Option<String>,
}

impl MapPill {
    /// A surface with no map, which draws `map · none`. It is a state and not an
    /// absence: a run that opened a port and found no file to load is one an
    /// operator can still learn into.
    pub const NONE: MapPill = MapPill { name: None };

    /// What the pill says after `map ·`.
    pub fn word(&self) -> &str {
        self.name.as_deref().unwrap_or(NO_MAP)
    }
}

/// What the pill says where no map is loaded, and it is deliberately not a
/// name: `none` is a word for a state, where a name would be a reading this
/// console invented. [`NO_ARRANGEMENT`]'s argument one pill along, and the same
/// shape [`audio_text`] uses for an input nobody opened.
const NO_MAP: &str = "none";

/// The `learn` pill's word, which is the whole of its label.
const LEARN_LABEL: &str = "learn";

/// The `map` pill's first word, the mock's own abbreviation — `map · <name>`.
const MAP_LABEL: &str = "map";

/// Layout and state for the transport MIDI learn toggle button.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LearnPill {
    /// The control: the capsule a press has to land in.
    pub pill: Rect,
    /// Whether learn is armed, read off [`View::learn`] and kept nowhere here.
    pub armed: bool,
}

impl LearnPill {
    /// Whether `p` is on the control.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.pill.contains(Pos2::new(p.x, p.y))
    }

    /// Returns the inverted arming state requested by clicking the learn pill (ADR-0236, ADR-0336, P-0090).
    pub fn next(&self) -> bool {
        !self.armed
    }
}

/// Layout bounding rectangle for the map filename readout pill.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapRow {
    /// The capsule, painted.
    pub pill: Rect,
}

/// Computes layout for the `learn` pill button, or `None` if unconfigured or insufficient room.
pub fn learn_pill(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    audio: Option<&AudioIn>,
    tracker: Option<Tracker>,
    map: Option<&MapPill>,
    armed: bool,
) -> Option<LearnPill> {
    map?;
    let row = transport(ctx, layout, values)?;
    let strip = to_egui(layout.rect(layout.find("transport")?));
    let text_w = width_of(ctx, LEARN_LABEL);
    let pill_w = size::PILL_PAD_X * 2.0 + text_w;
    let mid = strip.center().y;
    // Layout following the tracker group, audio-in pill, or bar.
    let after = match (
        tracker_group(ctx, layout, values, audio, tracker),
        audio_in(ctx, layout, values, audio),
    ) {
        (Some(group), _) => group.double.max.x,
        (None, Some(before)) => before.pill.max.x,
        (None, None) => row.bar.max.x,
    };
    let pill = Rect::from_min_size(
        Pos2::new(after + size::TRANSPORT_GAP, mid - size::PILL_H * 0.5),
        egui::vec2(pill_w, size::PILL_H),
    );
    // [`outputs_row`]'s rule: a capsule that does not fit in its row is no
    // control at all rather than half of one over the frame readout.
    if !strip.contains_rect(pill) || pill.max.x + size::TRANSPORT_GAP > row.frame.min.x {
        return None;
    }
    Some(LearnPill { pill, armed })
}

/// Computes layout for the `map` readout pill, placed after the learn button or preceding group.
pub fn map_pill(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    audio: Option<&AudioIn>,
    tracker: Option<Tracker>,
    map: Option<&MapPill>,
) -> Option<MapRow> {
    let map = map?;
    let row = transport(ctx, layout, values)?;
    let strip = to_egui(layout.rect(layout.find("transport")?));
    let words = format!("{MAP_LABEL} · {}", map.word());
    let pill_w = size::PILL_PAD_X * 2.0 + width_of(ctx, &words);
    let mid = strip.center().y;
    let after = match (
        learn_pill(ctx, layout, values, audio, tracker, Some(map), false),
        tracker_group(ctx, layout, values, audio, tracker),
        audio_in(ctx, layout, values, audio),
    ) {
        (Some(learn), _, _) => learn.pill.max.x,
        (None, Some(group), _) => group.double.max.x,
        (None, None, Some(before)) => before.pill.max.x,
        (None, None, None) => row.bar.max.x,
    };
    let pill = Rect::from_min_size(
        Pos2::new(after + size::TRANSPORT_GAP, mid - size::PILL_H * 0.5),
        egui::vec2(pill_w, size::PILL_H),
    );
    if !strip.contains_rect(pill) || pill.max.x + size::TRANSPORT_GAP > row.frame.min.x {
        return None;
    }
    Some(MapRow { pill })
}

/// One text measurement, the way every pill in this row takes one.
fn width_of(ctx: &egui::Context, text: &str) -> f32 {
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            text.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    })
}

/// Paints the learn pill button, highlighting in lavender when armed.
pub(crate) fn learn_into(ui: &Ui, pal: &Palette, pill: &LearnPill) {
    let painter = ui.painter();
    let radius = CornerRadius::same((size::PILL_H * 0.5) as u8);
    let ink = match pill.armed {
        true => pal.lav,
        false => pal.dim,
    };
    if pill.armed {
        painter.rect_filled(pill.pill, radius, pal.tint);
    }
    painter.rect_stroke(
        pill.pill,
        radius,
        Stroke::new(size::HAIRLINE, ink),
        StrokeKind::Inside,
    );
    let galley = painter.layout_no_wrap(
        LEARN_LABEL.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        ink,
    );
    painter.galley(
        Pos2::new(
            pill.pill.min.x + size::PILL_PAD_X,
            pill.pill.center().y - galley.size().y * 0.5,
        ),
        galley,
        ink,
    );
}

/// Paints the map name readout pill without dropdown chevron.
pub(crate) fn map_into(ui: &Ui, pal: &Palette, pill: &MapRow, map: &MapPill) {
    let painter = ui.painter();
    painter.rect_stroke(
        pill.pill,
        CornerRadius::same((size::PILL_H * 0.5) as u8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    let galley = painter.layout_no_wrap(
        format!("{MAP_LABEL} · {}", map.word()),
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.dim,
    );
    painter.galley(
        Pos2::new(
            pill.pill.min.x + size::PILL_PAD_X,
            pill.pill.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.dim,
    );
}
