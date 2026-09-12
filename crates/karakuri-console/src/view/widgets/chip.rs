use egui::Painter;

use super::super::*;

/// `.tally.off` went with it as the one rule only that strip used. The argument
/// is kept in the past tense rather than deleted, because it is still why there
/// is no fourth variant here — the three below are the residencies a `Deck`
/// has, and a fourth would have to be invented whether or not a mock is drawing
/// one.
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
    /// Every residency there is, so that anything which has to hold all three
    /// cannot be given two.
    ///
    /// One customer today and it is [`mixer`], which measures the chip against the
    /// widest word rather than the current one. A `match` cannot express *the
    /// widest of them*, and a list written at the call site would be a second list
    /// of the residencies — the exact drift a fourth variant would walk straight
    /// past. Here it is one line under the enum, and a fourth variant that is not
    /// added to it is a `[Tally; 3]` that no longer compiles.
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

/// `.tally`: a capsule with the residency's word in it, in the residency's own
/// colours.
///
/// - `.tally.live` — `color-mix(in srgb, var(--c-pink) 18%, transparent)`
///   behind `var(--c-pink)`, with `box-shadow: 0 0 10px var(--c-glowp)`. The
///   halo is an [`egui::epaint::Shadow`], the mechanism the transport's lit
///   beat and the Outputs row's dot both use.
/// - `.tally.priming` — a 20% wash of `var(--c-sun)` behind `var(--c-sun)`,
///   and no halo: priming is warming out of sight, not on air.
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
