use egui::{Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, Stroke, Ui};

use super::*;

// ---------------------------------------------------------------------------
// Library listing painting
// ---------------------------------------------------------------------------

/// View of rows and starred items being rendered for the current frame.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Listed<'a> {
    /// The names the bay lists and what each of them is — [`View::rows`].
    pub(crate) rows: Rows<'a>,
    /// The ids the store has starred — [`View::starred`].
    pub(crate) starred: &'a std::collections::BTreeSet<String>,
}

/// Draws a 5-pointed star mark (outline or filled).
fn star_mark(painter: &egui::Painter, centre: Pos2, across: f32, colour: Color32, filled: bool) {
    let outer = across * 0.5;
    let rim: Vec<Pos2> = (0..10)
        .map(|step| {
            // Clockwise from top.
            let angle = std::f32::consts::PI * 0.2 * step as f32;
            let r = match step % 2 {
                0 => outer,
                _ => outer * STAR_WAIST,
            };
            Pos2::new(centre.x + r * angle.sin(), centre.y - r * angle.cos())
        })
        .collect();
    if !filled {
        painter.add(egui::Shape::closed_line(
            rim,
            Stroke::new(size::HAIRLINE, colour),
        ));
        return;
    }
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(centre, colour);
    for point in &rim {
        mesh.colored_vertex(*point, colour);
    }
    for step in 0..10u32 {
        mesh.add_triangle(0, step + 1, (step + 1) % 10 + 1);
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// Renders an expanded set/procedure reading block within the library listing.
pub(crate) fn reading_into(
    painter: &egui::Painter,
    pal: &Palette,
    block: &Block,
    reading: &Reading,
) {
    painter.rect_filled(
        block.well,
        CornerRadius::same(size::READING_RADIUS as u8),
        pal.well,
    );
    let mut at = 0usize;
    let mut line = |left: &str, right: &str, ink: Color32| {
        let row = block.row(at);
        at += 1;
        let galley = painter.layout_job(span_at(left, size::BASE, ink));
        painter.galley(
            Pos2::new(
                row.min.x + size::READING_PAD_X,
                row.center().y - galley.size().y * 0.5,
            ),
            galley,
            ink,
        );
        let value = painter.layout_job(span_at(right, size::LIB_FOOT_SIZE, pal.faint));
        painter.galley(
            Pos2::new(
                row.max.x - size::LIB_ROW_PAD_X - value.size().x,
                row.center().y - value.size().y * 0.5,
            ),
            value,
            pal.faint,
        );
    };
    // Reading header label styled in lavender ink.
    line(READING_HEAD, &reading.knobs_word(), pal.lav);
    for knob in &reading.knobs {
        line(&knob.key, &knob.range, pal.dim);
    }
    // Capacity readout rendered in parameter row style.
    if let Some(capacity) = &reading.capacity {
        line(READING_CAPACITY, capacity, pal.dim);
    }
    if let Some(emits) = &reading.emits {
        line(READING_EMITS, emits, pal.dim);
    }
    // Foot summary readout displaying total nodes and readable cards.
    line(&reading.nodes_word(), &reading.cards_word(), pal.dim);
}

/// One row of the Library bay, painted: the wash if under the cursor, the star,
/// the name and the badges.
pub(crate) fn row_into(
    ui: &Ui,
    pal: &Palette,
    bay: &LibraryBay,
    rows: Rows<'_>,
    starred: &std::collections::BTreeSet<String>,
    cursor: usize,
    index: usize,
) {
    let Some(name) = rows.name(index) else {
        return;
    };
    let painter = ui.painter().with_clip_rect(bay.list);
    let row = bay.row(index);
    // Cursor row highlight: lavender wash and highlighted text.
    let ink = match index == cursor {
        true => {
            painter.rect_filled(
                row,
                CornerRadius::same(size::LIB_ROW_RADIUS as u8),
                tint(pal.lav, 13),
            );
            pal.text
        }
        false => pal.dim,
    };
    // Star indicator: sun/faint for sets, hidden for procedure rows (ADR-0299, ADR-0338).
    if !rows.procedure(index) {
        let on = starred.contains(name);
        star_mark(
            &painter,
            bay.star(index).center(),
            STAR_SIZE,
            match on {
                true => pal.sun,
                false => pal.faint,
            },
            on,
        );
    }
    let galley = painter.layout_job(span_at(name, size::BASE, ink));
    painter.galley(
        Pos2::new(bay.named(index), row.center().y - galley.size().y * 0.5),
        galley,
        ink,
    );
    // Badges at right: distinct outline and text style for procedure kind vs set contents.
    let (ring, word) = match rows.procedure(index) {
        true => (pal.line, pal.dim),
        false => (pal.hair, pal.faint),
    };
    for (badge_text, box_) in bay.badges(ui.ctx(), index, &rows.badges(index)) {
        badge(&painter, box_, badge_text, ring, word);
    }
}

/// The Library bay's rows, painted into the list area.
pub(crate) fn rows_into(
    ui: &Ui,
    pal: &Palette,
    bay: &LibraryBay,
    listed: Listed<'_>,
    cursor: usize,
) {
    let Listed { rows, starred } = listed;
    for index in bay.drawn() {
        row_into(ui, pal, bay, rows, starred, cursor, index);
    }
}

/// The foot's load control, painted: the button, the arrow label and the deck pulldown.
pub(crate) fn load_into(ui: &Ui, pal: &Palette, load: &Load, at: Target) {
    let painter = ui.painter();
    // Primary load button rendered with borderless lavender capsule wash.
    painter.rect_filled(
        load.button,
        // `border-radius: 999px` on a box this short is a capsule.
        CornerRadius::same((size::PILL_H * 0.5) as u8),
        tint(pal.lav, 15),
    );
    let galley = painter.layout_no_wrap(
        LOAD_PILL.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.lav,
    );
    painter.galley(load.text.min, galley, pal.lav);

    // Arrow indicator between load button and deck pulldown.
    arrow_mark(painter, load.arrow.center(), LOAD_ARROW, pal.faint, false);

    // Deck selection pulldown button with downward chevron.
    pill_at(ui, pal, load.deck, at.letter());
    chevron_down(painter, load.chevron, pal.dim);
}

/// The foot of the Library bay, painted: the count, the params chip, and the load controls.
pub(crate) fn foot_into(ui: &Ui, pal: &Palette, bay: &LibraryBay, at: Target) {
    let painter = ui.painter().with_clip_rect(bay.foot);
    let rule = bay.foot.min.y + size::HAIRLINE * 0.5;
    painter.line_segment(
        [
            Pos2::new(bay.foot.min.x, rule),
            Pos2::new(bay.foot.max.x, rule),
        ],
        Stroke::new(size::HAIRLINE, pal.hair),
    );
    let galley = painter.layout_job(span_at(&bay.count(), size::LIB_FOOT_SIZE, pal.faint));
    painter.galley(
        Pos2::new(
            bay.foot.min.x + size::LIB_FOOT_PAD_X,
            bay.foot.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.faint,
    );

    // Params toggle chip in mint armed state when reading is open (ADR-0312).
    pill_into(
        ui,
        pal,
        bay.params_chip(ui.ctx(), at),
        PARAMS_PILL,
        bay.reading.is_some(),
    );

    let load = bay.load(ui.ctx(), at);
    load_into(ui, pal, &load, at);
}

/// Paints the deck pulldown selection menu overlay.
pub(crate) fn deck_list_into(ui: &Ui, pal: &Palette, load: &Load, at: Target, card: Rect) {
    let painter = ui.painter();
    popup_card(painter, pal, card);
    // `take` rather than a range, because the rows are the letters: a list
    // longer than [`DECK_LETTERS`] is a deck this crate has no letter for, and
    // `View::aim_at` is what stops one being asked for.
    for (index, letter) in DECK_LETTERS.iter().enumerate().take(load.rows) {
        let row = load.row(card, index);
        let ink = match index == usize::from(at.deck) {
            true => pal.text,
            false => pal.dim,
        };
        let galley = painter.layout_no_wrap(
            (*letter).to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            ink,
        );
        painter.galley(
            Pos2::new(
                row.min.x + size::LIB_ROW_PAD_X,
                row.center().y - galley.size().y * 0.5,
            ),
            galley,
            ink,
        );
    }
}

/// Paints the row context popup menu for loading to decks or saving (ADR-0338).
pub(crate) fn row_menu_into(ui: &Ui, pal: &Palette, menu: &RowMenu) {
    let painter = ui.painter();
    popup_card(painter, pal, menu.card);
    let word = |at: Rect, text: String| {
        let galley = painter.layout_no_wrap(
            text,
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.dim,
        );
        painter.galley(
            Pos2::new(
                at.min.x + size::LIB_ROW_PAD_X,
                at.center().y - galley.size().y * 0.5,
            ),
            galley,
            pal.dim,
        );
    };
    // `take` rather than a range, for the deck list's reason: the items are
    // the letters, and `Menued::decks` past `DECKS` is a deck this crate has
    // no letter for.
    for index in 0..menu.loads.min(DECKS) {
        word(menu.load(index), load_item(index as u8));
    }
    // Separator rule and save action for sets (omitted for procedures per ADR-0338).
    if let Some(band) = menu.rule {
        let rule = band.center().y;
        painter.line_segment(
            [
                Pos2::new(band.min.x + size::LIB_ROW_PAD_X, rule),
                Pos2::new(band.max.x - size::LIB_ROW_PAD_X, rule),
            ],
            Stroke::new(size::HAIRLINE, pal.hair),
        );
    }
    if let Some(save) = menu.save {
        word(save, MENU_SAVE.to_owned());
    }
}
