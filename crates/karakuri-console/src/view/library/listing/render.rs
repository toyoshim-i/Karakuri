use egui::{Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, Stroke, Ui};

use super::super::super::*;
use super::*;

// ---------------------------------------------------------------------------
// Library listing painting
// ---------------------------------------------------------------------------

/// What the list is drawing this frame: the rows, and which of them the store
/// has starred.
///
/// One argument because they are one reading — the host's `listing` writes the
/// two halves together, and a row and its mark drawn from two answers could
/// disagree about a Set that arrived between them. It is [`Filters`]' shape one
/// row down, borrowed for the same reason: nothing is cloned to draw a frame.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Listed<'a> {
    /// The names the bay lists and what each of them is — [`View::rows`].
    pub(crate) rows: Rows<'a>,
    /// The ids the store has starred — [`View::starred`].
    pub(crate) starred: &'a std::collections::BTreeSet<String>,
}

/// A star's mark, drawn rather than typed — [`arrow_mark`]'s reason one bay
/// along, and the case is sharper here: this mark's whole job is saying
/// *starred* or *not starred*, and a face with no `★` would answer it with a
/// tofu in both states.
///
/// `across` wide and the same tall, which is [`arrow_mark`]'s rule for a mark
/// that stands in for a glyph: the box is the size the glyph would have been,
/// so the name beside it starts in the same place whichever way this is drawn.
///
/// Ten rim points at two radii, the outer at the top and the rest every 36°
/// round — [`STAR_WAIST`] is the inner one. `filled` is `.star` and the outline
/// is `.star.off`, which is the mock's own pair.
///
/// A fan from the centre and not a polygon, because a five-pointed star is not
/// convex: `egui::Shape::convex_polygon` fans from the first vertex, which for
/// this outline puts triangles outside the ink. A star *is* star-shaped about
/// its own centre, so a fan anchored there is exact — which is
/// [`mixer::mask_mark`]'s answer to `epaint` having no arc, one shape along.
fn star_mark(painter: &egui::Painter, centre: Pos2, across: f32, colour: Color32, filled: bool) {
    let outer = across * 0.5;
    let rim: Vec<Pos2> = (0..10)
        .map(|step| {
            // **From straight up, and clockwise**, which is where a star's
            // point is drawn: `egui`'s y runs down the screen, so the turn is
            // taken as a positive angle off `-y`.
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

/// A reading, painted: the well, and a row of it per line.
///
/// Where the box goes is [`library`]'s and where each row in it goes is
/// [`Block::row`]'s, so this paints and derives nothing — [`library_into`]'s
/// own rule one box out.
///
/// Term for term from `style.css` and from the markup the mock sets inline:
///
/// - the box — `background: var(--c-well)` at [`size::READING_RADIUS`], which
///   is the well a candidate row stands on in the staging lane.
/// - `.lib-row` with its left padding overridden — a name at [`size::BASE`],
///   one [`size::READING_PAD_X`] in from the left of the well, centred across
///   the row's own height.
/// - `.lib-row .dim` — `color: var(--c-faint)` at [`size::LIB_FOOT_SIZE`],
///   `margin-left: auto`, so the value ends one [`size::LIB_ROW_PAD_X`] in
///   from the right of the well. The same 10px the foot's count is drawn
///   at, which is the mock's own reading: a declaration is what the row is
///   *about* and the range beside it is the small type this bay uses for
///   everything a row is not named by.
/// - `.addr` — `color: var(--c-lav)`, on the head's word alone. The weight is
///   not honoured and cannot be, which is [`room`](crate::room)'s own sentence:
///   `egui`'s default proportional face has no bold, so a `font-weight: 700`
///   is a colour and a size here.
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
    // **The head is the one lav in the box**, which is `.addr`'s own colour
    // and the mark that says this is a reading of the row above rather than
    // a sixth Set.
    line(READING_HEAD, &reading.knobs_word(), pal.lav);
    for knob in &reading.knobs {
        line(&knob.key, &knob.range, pal.dim);
    }
    // **The capacity is drawn in a knob's shape**, because that is how a
    // procedure declares it — and it is the declaration rather than this
    // Set's own number, which nothing on this panel says before a load. The
    // mock's note carries that as owed rather than answered here.
    if let Some(capacity) = &reading.capacity {
        line(READING_CAPACITY, capacity, pal.dim);
    }
    if let Some(emits) = &reading.emits {
        line(READING_EMITS, emits, pal.dim);
    }
    // **The foot counts the nodes and says how many of them could be read**,
    // which is the one thing that keeps a knob missing for want of a card
    // from being a knob missing in silence.
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
    // `.lib-row.cursor` — `background: color-mix(in srgb, var(--c-lav)
    // 13%, transparent)` and `color: var(--c-text)`, where every other row
    // is `var(--c-dim)` over the bare card. **The wash is the whole of the
    // mark**: the mock puts no rule, no caret and no chevron on the row,
    // so a row that is not under the cursor is drawn exactly as it was
    // before this line existed.
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
    // **The star before the name**, and its ink is the store's answer
    // rather than the cursor's: `.lib-row .star` is `--c-sun` whatever
    // else the row is wearing, and `.star.off` is `--c-faint` — the one
    // mark in this bay that a row's own state colours and the wash above
    // does not.
    //
    // **A procedure row draws none at all**, which is `.star.none`'s
    // `visibility: hidden` in the mock: the column is kept so every name
    // starts in the same place, and there is nothing in it because a star
    // is a control over a Set this store holds (ADR-0299, ADR-0338).
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
    // **The badges, at the right of the row.** `.badge` is a hairline round
    // `--c-faint` and `.badge.kind` — the one badge a procedure row wears —
    // is `--c-line` round `--c-dim`, which is `style.css`'s own pair and
    // carries a real distinction: on a procedure row the single badge is
    // what the row *is*, where a Set's badges are a list of what it holds.
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
    // **`.pill.lav`, and it is the one pill on this panel with no border**:
    // `border-color: transparent; color: var(--c-lav); background:
    // color-mix(in srgb, var(--c-lav) 15%, transparent)`. Every other capsule
    // here is [`pill_at`]'s hairline round `--c-dim`, and the difference is
    // the point — `console.html`: *"`load` is lav because it is the press this
    // bay exists for, and a second lav chip beside it would make the colour
    // mean two things at a width of eight characters"*.
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

    // **The `→` between the two capsules is drawn and is on neither of them**,
    // which is the whole of what [`LOAD_ARROW`] is: the mock's `&rarr;` was
    // typed here and `egui`'s default face has no U+2192, so the row read
    // `load □ A` — a label saying how to read two controls, with a tofu where
    // the reading was. It is `--c-faint`, which is `.lib-foot`'s own colour
    // and the colour the count at the other end of the row is in: it is the
    // foot's furniture rather than either control's, and a mark in the lav
    // would put this bay's accent on a thing nobody can press.
    arrow_mark(&painter, load.arrow.center(), LOAD_ARROW, pal.faint, false);

    // **The pulldown, drawn as the `params` chip is and not as the button is.**
    // `console.html`: *"It is deliberately not lavender. Lavender here is the
    // deck the keys are addressed to, and this is the one letter on the
    // console that is allowed to name a different one."* So it is
    // [`pill_at`]'s hairline round `--c-dim` with the chevron the two menu
    // pills in the transport row already carry.
    pill_at(ui, pal, load.deck, at.letter());
    chevron_down(&painter, load.chevron, pal.dim);
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

    // **The `params` chip**, between the count and the `load` button, and it
    // is a toggle drawn as one: `.pill.armed`'s mint while a reading is open
    // and `.pill`'s hairline round `--c-dim` while none is.
    //
    // **Which of the two is read off the block this bay is *drawing***, and
    // off nothing else — the same `LibraryBay::reading` that
    // [`LibraryBay::read`] matches on to decide what a press asks for. So the
    // capsule an operator is looking at and the answer the press gives cannot
    // come apart, which is what a toggle owes and is why this is not a second
    // reading of `View::reading` (ADR-0312).
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

/// The pulldown's list, painted — the card and a row per deck the mixer is
/// drawing a strip for.
///
/// Where everything goes is [`Load`]'s, so this paints and derives nothing.
/// Drawn from [`View::draw`] after the bays for the arrangement menu's reason:
/// the card hangs out of the foot it belongs to and over this bay's own list,
/// so a card painted from inside the Library arm would go on before the rows
/// and end up under them.
///
/// The card is the arrangement menu's card term for term — the panel's own
/// fill, a hairline and the shadow — because it is the same object one bay
/// along and a second treatment would be a second answer to *what does a list
/// hanging off a capsule look like* (`console.html` draws neither, which is
/// what makes this the console's own).
///
/// The row the target is on is drawn in `--c-text` and the rest in `--c-dim`,
/// which is the arrangement menu's own reading of *which of these is in use*.
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

/// A row's menu, painted — the card, a row per deck the mixer is drawing a
/// strip for, the separator, and the send under it.
///
/// Where everything goes is [`RowMenu`]'s, so this paints and derives nothing,
/// which is [`deck_list_into`]'s own sentence one control along. Drawn from
/// [`View::draw`] after the bays for that card's reason: it hangs out of the
/// row it belongs to and over the rows under it, so a card painted from inside
/// the Library arm would go on before them and end up underneath.
///
/// The card is the deck pulldown's card term for term — the panel's own fill, a
/// hairline and the shadow — because it is the same object one control along
/// and a second treatment would be a second answer to *what does a card hanging
/// off something look like*. `.rowmenu` in `docs/manual/style.css` says the
/// same thing from the mock's side.
///
/// Every item is drawn in `--c-dim`, and none of them is in `--c-text`. The
/// pulldown's card marks the row the target is on, because that list is a mark
/// being moved; this one is six acts and none of them is a state, so there is
/// nothing here for an ink to say. `.rowmenu .item` carries `color:
/// var(--c-dim)` and no second rule.
///
/// The separator is drawn and is not an item: one hairline in `--c-hair` across
/// the band, inset from the card's edge, which is the mock's `.rowmenu .rule`
/// and the same pixel every other rule on this panel is drawn at.
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
    // **The separator and the send are drawn where there is one**, which is a
    // Set row: a procedure cannot be written out as a `.kbset` — nothing takes
    // a bare `.kir` in — so its menu is the loads and stops (ADR-0338).
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
