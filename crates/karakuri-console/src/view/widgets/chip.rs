use egui::Painter;

use super::super::*;

/// Tally states representing the three valid residencies a [`Deck`] can have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tally {
    /// `.tally.live` — stepped and composited. On air.
    Live,
    /// `.tally.priming` — stepped and warming its buffers, drawn into its own cell,
    /// and asked for.
    Priming,
    /// `.tally.alloc` — stepped and drawn like a priming slot, and asked of
    /// nothing. Off air, not at rest: every slot runs at the room's tempo so that
    /// its cell is a preview rather than a still (ADR-0269).
    Allocated,
}

impl Tally {
    /// All tally variants, used for measuring bounds against the widest word.
    pub const ALL: [Tally; 3] = [Tally::Live, Tally::Priming, Tally::Allocated];

    /// The mock's own word, lower-case here and upper-cased at paint time because
    /// `.tally` is `text-transform: uppercase` — the rule [`Kind::Bay`]'s title
    /// states.
    pub const fn word(self) -> &'static str {
        match self {
            Tally::Live => "live",
            Tally::Priming => "prim",
            Tally::Allocated => "alloc",
        }
    }
}

/// A galley centred in a box, both ways — which is `align-items: center` on a
/// flex column done where the type's real height is known.
pub fn centre_galley(
    painter: &Painter,
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

/// A badge: a hairline-stroked rectangular tag with centered typography.
pub fn badge(painter: &Painter, rect: Rect, text: &str, ring: Color32, word: Color32) {
    painter.rect_stroke(
        rect,
        CornerRadius::same(size::BADGE_RADIUS as u8),
        Stroke::new(size::HAIRLINE, ring),
        StrokeKind::Inside,
    );
    let galley = painter.layout_no_wrap(
        text.to_owned(),
        FontId::new(size::BADGE_SIZE, FontFamily::Proportional),
        word,
    );
    centre_galley(painter, rect, galley, word);
}

/// An interactive or stateful toggle capsule chip with centered typography.
pub fn toggle_chip(
    painter: &Painter,
    rect: Rect,
    on: bool,
    text: &str,
    font_size: f32,
    pal: &Palette,
    active_ink: Color32,
) {
    let radius = CornerRadius::same((rect.height() * 0.5) as u8);
    let ink = match on {
        true => {
            painter.rect_filled(rect, radius, tint(active_ink, 15));
            active_ink
        }
        false => {
            painter.rect_stroke(
                rect,
                radius,
                Stroke::new(size::HAIRLINE, pal.line),
                StrokeKind::Inside,
            );
            pal.faint
        }
    };
    let galley = painter.layout_no_wrap(
        text.to_owned(),
        FontId::new(font_size, FontFamily::Proportional),
        ink,
    );
    centre_galley(painter, rect, galley, ink);
}

/// A scope tab pill with left-padded and vertically centered typography.
pub fn scope_tab(painter: &Painter, rect: Rect, marked: bool, text: &str, pal: &Palette) {
    let ink = match marked {
        true => {
            painter.rect_filled(
                rect,
                CornerRadius::same((rect.height() * 0.5) as u8),
                tint(pal.lav, 15),
            );
            pal.lav
        }
        false => pal.faint,
    };
    let galley = painter.layout_no_wrap(
        text.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        ink,
    );
    painter.galley(
        Pos2::new(
            rect.min.x + size::SCOPE_PAD_X,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        ink,
    );
}

/// The tally's word as one laid-out run, so that measuring it and painting it
/// cannot be two different runs of type.
pub fn tally_job(tally: Tally, colour: Color32) -> LayoutJob {
    spaced(
        &tally.word().to_uppercase(),
        size::TALLY_SIZE,
        colour,
        size::TALLY_TRACKING,
    )
}

/// Draws `.tally`: a capsule with the residency word in residency colours.
/// If `pending` is set, rolls toward the target residency per [ADR-0190](../../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md).
pub fn tally_into(
    painter: &Painter,
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

    // Clip the capsule to the words alone; the outer halo is intended to spill.
    let painter = painter.with_clip_rect(rect.intersect(painter.clip_rect()));

    let galley = painter.layout_job(tally_job(tally, ink_now));
    // Pitch between words is at least the box height to prevent overlap during roll.
    let pitch = size::TALLY_H.max(galley.size().y);
    let rolled = match pending {
        Some(_) => roll_at(phase) * pitch,
        None => 0.0,
    };
    let word_into = |painter: &Painter, galley: std::sync::Arc<egui::Galley>, colour, dy| {
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
    // Draw destination word below in its target colour when rolling.
    if let Some(to) = pending {
        let (_, ink_to) = ink(to);
        let galley = painter.layout_job(tally_job(to, ink_to));
        word_into(&painter, galley, ink_to, pitch - rolled);
    }
}
