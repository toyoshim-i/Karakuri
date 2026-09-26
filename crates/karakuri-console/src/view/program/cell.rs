use super::*;

/// Draws one deck preview cell's image: well background, optional texture, and hairline border.
pub fn preview(ui: &Ui, pal: &Palette, cell: Rect, picture: Option<Picture>) {
    let radius = CornerRadius::same(size::PREVIEW_RADIUS as u8);
    // Clipped to the cell for the reason the picture is clipped to its region:
    // the rectangle in `picture` came from outside, and a stale one is a
    // thumbnail painted across the bay rather than a wrong thumbnail.
    let painter = ui.painter().with_clip_rect(cell);
    painter.rect_filled(cell, radius, pal.well);

    if let Some(picture) = picture {
        painter.image(picture.id, picture.rect, WHOLE_TEXTURE, Color32::WHITE);
    }
    // Inset hairline stroke on every cell; master mix monitors per ADR-0240.
    painter.rect_stroke(
        cell,
        radius,
        Stroke::new(size::HAIRLINE, pal.hair),
        StrokeKind::Inside,
    );
}

/// Returns the status label for a preview cell (`material`, `overloaded`, or `no slot`).
fn state_word(picture: Option<Picture>, overloaded: bool) -> &'static str {
    match (picture, overloaded) {
        (None, _) => PREVIEW_NO_SLOT,
        (Some(_), true) => PREVIEW_OVERLOADED,
        (Some(_), false) => PREVIEW_MATERIAL,
    }
}

/// A cell showing its slot's own material. See [`state_word`].
pub const PREVIEW_MATERIAL: &str = "material";

/// A cell showing the last frame a stopped slot drew — the word
/// `swap::Event::Overloaded` and [`Stage::Overloaded`] carry, said here because
/// this is where the still is. See [`state_word`] and [`View::overloaded`].
pub const PREVIEW_OVERLOADED: &str = "overloaded";

/// A cell with no deck slot behind it. See [`state_word`].
pub const PREVIEW_NO_SLOT: &str = "no slot";

/// Draws the deck preview cell caption: deck letter, state label, and optional performance risk badge.
///
/// Missing slots or unbudgeted decks omit the risk badge per [ADR-0200](../../../../docs/adr/0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md) and ADR-0296.
#[allow(clippy::too_many_arguments)]
pub fn caption_into(
    ui: &Ui,
    pal: &Palette,
    image: Rect,
    deck: usize,
    picture: Option<Picture>,
    // **Whether this slot has stopped updating** — [`View::overloaded`]. It
    // changes the word and nothing else: not the letter, not the badge, and
    // not the image above, which is the still it is about.
    overloaded: bool,
    cost: Option<Budgeted>,
    marked: bool,
) {
    let at = caption_of(image);
    let painter = ui.painter().with_clip_rect(at);
    let font = FontId::new(size::PREVIEW_SIZE, FontFamily::Proportional);

    let ink = match marked {
        true => pal.text,
        false => pal.dim,
    };
    let letter = painter.layout_no_wrap(DECK_LETTERS[deck].to_owned(), font.clone(), ink);
    let width = letter.size().x;
    painter.galley(
        Pos2::new(at.min.x, at.center().y - letter.size().y * 0.5),
        letter,
        ink,
    );

    let word = painter.layout_no_wrap(state_word(picture, overloaded).to_owned(), font, pal.faint);
    painter.galley(
        Pos2::new(
            at.min.x + width + size::PREVIEW_CAPTION_GAP_X,
            at.center().y - word.size().y * 0.5,
        ),
        word,
        pal.faint,
    );

    // Draw the risk badge dot if a slot exists and has a valid budget.
    let dot = picture
        .and(cost)
        .filter(|budgeted| budgeted.ms.is_finite())
        .map(|budgeted| band_of(budgeted.ms));
    if let Some(band) = dot {
        // `margin-left: auto` — the far end of the caption, centred in its
        // height, and `border-radius: 999px` on a 6px box is a circle of half
        // that across.
        let radius = size::PREVIEW_RISK * 0.5;
        painter.circle_filled(
            Pos2::new(at.max.x - radius, at.center().y),
            radius,
            band.colour(pal),
        );
    }
}
