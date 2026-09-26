//! Deck preview caption layout, label positioning, and performance risk badges.

mod common;

use common::{arranged, near};
use egui::epaint::CircleShape;
use egui::Rect;
use karakuri_console::room::{size, Room};
use karakuri_console::view::{
    band_of, caption_of, preview_rects, Band, Basis, Budgeted, Picture, View, BAND_BLUE_MS,
    BAND_PURPLE_MS, BAND_RED_MS, BAND_YELLOW_MS, DECKS, DECK_LETTERS, MOCK_CANVAS,
    PREVIEW_MATERIAL, PREVIEW_NO_SLOT, PREVIEW_OVERLOADED,
};

/// A window with the cells in the row under the picture, and the four
/// rectangles the bay put them in.
fn cells() -> (karakuri_console::panel::Panel, [Rect; DECKS]) {
    let panel = arranged(common::SMALLEST, MOCK_CANVAS);
    let cells = preview_rects(panel.layout(), MOCK_CANVAS).expect("the cells are on screen");
    (panel, cells)
}

/// A console with something behind every cell. The texture id is never resolved
/// — nothing here renders — and what it stands for is the only thing under
/// test: a cell with material in it.
fn with_material() -> View {
    let mut view = View::new(Room::Day);
    view.previews = std::array::from_fn(|deck| {
        Some(Picture {
            id: egui::TextureId::Managed(deck as u64 + 1),
            rect: Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(16.0, 9.0)),
        })
    });
    view
}

/// Every shape the console paints on one frame, with its clip rectangle
/// dropped.
fn shapes(view: &mut View, panel: &mut karakuri_console::panel::Panel) -> Vec<egui::Shape> {
    let ctx = common::drawn_once();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, panel));
    out.textures_delta.clear();
    out.shapes.into_iter().map(|c| c.shape).collect()
}

/// Collects all painted text elements paired with their positions.
fn texts(view: &mut View, panel: &mut karakuri_console::panel::Panel) -> Vec<(egui::Pos2, String)> {
    shapes(view, panel)
        .into_iter()
        .filter_map(|shape| match shape {
            egui::Shape::Text(at) => Some((at.pos, at.galley.text().to_owned())),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Where the caption is
// ---------------------------------------------------------------------------

/// Verifies that the preview caption is placed strictly below the preview image
/// and fits within the preview row bounds.
#[test]
fn the_caption_is_outside_the_image() {
    let (panel, cells) = cells();
    let row = common::rect_of(panel.layout(), "deck-previews");
    let floor = row.y + row.h - size::PROGRAM_BODY_PAD;
    for (deck, cell) in cells.iter().enumerate() {
        let caption = caption_of(*cell);
        assert!(
            caption.min.y >= cell.max.y,
            "cell {deck}'s caption starts at {} and the image ends at {}",
            caption.min.y,
            cell.max.y
        );
        assert!(
            !caption.intersects(*cell),
            "cell {deck}'s caption {caption:?} overlaps its image {cell:?}"
        );
        // `.cell`'s gap between them, and `.caption`'s own height.
        assert!(near(caption.min.y - cell.max.y, size::PREVIEW_CAPTION_GAP));
        assert!(near(caption.height(), size::PREVIEW_CAPTION_H));
        // The image's width, because the image is centred in its track and a
        // caption at the track's left edge would sit under the ground beside
        // the cell it names.
        assert!(near(caption.min.x, cell.min.x) && near(caption.max.x, cell.max.x));
        // And the whole cell — image and caption — is inside the row the bay
        // laid out, less the padding under it.
        assert!(
            cell.min.y >= row.y && caption.max.y <= floor,
            "cell {deck} runs {} to {} and the row holds {} to {floor}",
            cell.min.y,
            caption.max.y,
            row.y
        );
    }
}

/// The letter is painted inside the caption below rather than over the preview image.
#[test]
fn the_letter_is_not_painted_over_the_material() {
    let (mut panel, cells) = cells();
    let mut view = with_material();
    let painted = texts(&mut view, &mut panel);

    for (deck, cell) in cells.iter().enumerate() {
        let over: Vec<&String> = painted
            .iter()
            .filter(|(pos, _)| cell.contains(*pos))
            .map(|(_, text)| text)
            .collect();
        assert!(
            over.is_empty(),
            "cell {deck} has {over:?} written over its material"
        );
    }

    for (deck, letter) in DECK_LETTERS.iter().enumerate() {
        let caption = caption_of(cells[deck]);
        assert!(
            painted
                .iter()
                .any(|(pos, text)| text == letter && caption.contains(*pos)),
            "deck {letter}'s letter is not in its caption"
        );
    }
}

// ---------------------------------------------------------------------------
// What the caption says
// ---------------------------------------------------------------------------

/// Verifies that preview captions distinguish slots using explicit state words
/// (ADR-0240, ADR-0269, ADR-0316).
#[test]
fn the_state_word_is_what_the_cell_actually_distinguishes() {
    // The words themselves, and not only the constants that hold them: a
    // rename that took both sides with it would leave every assertion below
    // comparing the code with itself.
    assert_eq!(PREVIEW_MATERIAL, "material");
    assert_eq!(PREVIEW_NO_SLOT, "no slot");
    assert_eq!(PREVIEW_OVERLOADED, "overloaded");

    let (mut panel, cells) = cells();

    // With a slot behind every cell.
    let mut view = with_material();
    let painted = texts(&mut view, &mut panel);
    for (deck, cell) in cells.iter().enumerate() {
        let words = in_caption(&painted, caption_of(*cell));
        assert!(
            words.contains(&PREVIEW_MATERIAL.to_owned()),
            "cell {deck} shows material and its caption says {words:?}"
        );
    }

    // Overloaded slot updates caption text without affecting adjacent cells.
    let mut view = with_material();
    view.overloaded[1] = true;
    let painted = texts(&mut view, &mut panel);
    for (deck, cell) in cells.iter().enumerate() {
        let words = in_caption(&painted, caption_of(*cell));
        let wanted = match deck {
            1 => PREVIEW_OVERLOADED,
            _ => PREVIEW_MATERIAL,
        };
        assert!(
            words.contains(&wanted.to_owned()),
            "cell {deck} should read `{wanted}` and its caption says {words:?}"
        );
    }

    // Cells without assigned slots display `no slot` (ADR-0258).
    let mut view = View::new(Room::Day);
    view.overloaded = [true; 4];
    let painted = texts(&mut view, &mut panel);
    for (deck, cell) in cells.iter().enumerate() {
        let words = in_caption(&painted, caption_of(*cell));
        assert!(
            words.contains(&PREVIEW_NO_SLOT.to_owned()),
            "cell {deck} has no slot behind it and its caption says {words:?}"
        );
    }

    // With none behind any of them, which is a console with no engine.
    let mut view = View::new(Room::Day);
    let painted = texts(&mut view, &mut panel);
    for (deck, cell) in cells.iter().enumerate() {
        let words = in_caption(&painted, caption_of(*cell));
        assert!(
            words.contains(&PREVIEW_NO_SLOT.to_owned()),
            "cell {deck} has no slot behind it and its caption says {words:?}"
        );
    }

    // And neither word the cell does not distinguish, anywhere on the panel.
    for (_, text) in &painted {
        assert!(
            text != "off"
                && text != "empty"
                && !DECK_LETTERS.iter().any(|l| text == &format!("{l} · off")),
            "{text:?} is drawn, and it is a state the program cannot be in"
        );
    }
}

/// Every word painted inside one caption.
fn in_caption(painted: &[(egui::Pos2, String)], caption: Rect) -> Vec<String> {
    painted
        .iter()
        .filter(|(pos, _)| caption.contains(*pos))
        .map(|(_, text)| text.clone())
        .collect()
}

// ---------------------------------------------------------------------------
// The badge (ADR-0296)
// ---------------------------------------------------------------------------

/// A slot the governor budgeted at `ms` on the better of its two numbers.
fn estimated(ms: f32) -> Option<Budgeted> {
    Some(Budgeted {
        ms,
        basis: Basis::Estimated,
    })
}

/// Every dot painted inside one caption: a filled circle, which is what
/// `.risk`'s `border-radius: 999px` on a 6px box is.
fn dots_in(
    view: &mut View,
    panel: &mut karakuri_console::panel::Panel,
    caption: Rect,
) -> Vec<CircleShape> {
    shapes(view, panel)
        .into_iter()
        .filter_map(|shape| match shape {
            egui::Shape::Circle(dot) => Some(dot),
            _ => None,
        })
        .filter(|dot| caption.contains(dot.center))
        .collect()
}

/// Every shape but text painted inside one caption, which is how the absence of
/// a badge is asserted: it fails for a circle, a filled rectangle, a ring or a
/// glyph standing in for one.
fn marks_in(
    view: &mut View,
    panel: &mut karakuri_console::panel::Panel,
    caption: Rect,
) -> Vec<egui::Shape> {
    shapes(view, panel)
        .into_iter()
        .filter(|shape| {
            let bounds = shape.visual_bounding_rect();
            bounds.is_finite() && caption.contains_rect(bounds)
        })
        .filter(|shape| !matches!(shape, egui::Shape::Text(_)))
        .collect()
}

/// Maps slot processing times into risk bands (Green, Blue, Yellow, Red, Purple) per `console.html`.
#[test]
fn the_band_is_read_from_the_number() {
    // Inside each band, at a value an operator could actually see.
    assert_eq!(band_of(0.0), Band::Green);
    assert_eq!(band_of(1.6), Band::Green);
    assert_eq!(band_of(5.5), Band::Blue);
    assert_eq!(band_of(8.9), Band::Yellow);
    assert_eq!(band_of(12.4), Band::Red);
    assert_eq!(band_of(21.0), Band::Purple);

    // The boundaries themselves, and the constants they are named by.
    assert_eq!(BAND_BLUE_MS, 4.0);
    assert_eq!(BAND_YELLOW_MS, 8.0);
    assert_eq!(BAND_RED_MS, 12.0);
    assert_eq!(BAND_PURPLE_MS, 16.0);

    // The five words the mock's classes name them by — `.risk.green` and its
    // four siblings, which is also what the engine's own band prediction
    // spells and how the guard finds the sentence in the page.
    let words: Vec<&str> = Band::ALL.iter().map(|band| band.word()).collect();
    assert_eq!(words, ["green", "blue", "yellow", "red", "purple"]);

    // The measured frame this repository has written down: a four-slot deck at
    // 21 ms warm, which is one slot's worth of a frame all by itself.
    assert_eq!(band_of(21.0 / 4.0), Band::Blue);
}

/// Values on band boundaries round toward the higher risk band (ADR-0293).
#[test]
fn a_boundary_rounds_to_the_worse_band() {
    for (boundary, worse, better) in [
        (BAND_BLUE_MS, Band::Blue, Band::Green),
        (BAND_YELLOW_MS, Band::Yellow, Band::Blue),
        (BAND_RED_MS, Band::Red, Band::Yellow),
        (BAND_PURPLE_MS, Band::Purple, Band::Red),
    ] {
        assert_eq!(
            band_of(boundary),
            worse,
            "{boundary} ms is on a boundary and rounds to the worse band"
        );
        assert_eq!(
            band_of(boundary - 0.001),
            better,
            "just under {boundary} ms is the better band, or the table is written with `>`"
        );
    }

    // ADR-0293 §2's binding case, which is where `FLOORED_SHARE_ALLOWED` comes
    // from: a truth of 12 ms corrected by 4/3 is reported as 16, one band
    // worse and exactly the edge of the band that stops a slot. A fifth more
    // and the correction alone would move red into purple.
    assert_eq!(band_of(12.0), Band::Red);
    assert_eq!(band_of(12.0 * 4.0 / 3.0), Band::Purple);
}

/// Renders the risk dot using its band colour, right-aligned to align across cells.
#[test]
fn the_dot_is_drawn_in_its_bands_colour_at_the_far_end_of_the_caption() {
    let (mut panel, cells) = cells();
    let want = [Band::Green, Band::Blue, Band::Red, Band::Purple];
    let at = [1.0, 5.0, 13.0, 30.0];

    for room in [Room::Day, Room::Night] {
        let mut view = with_material();
        view.room = room;
        view.costs = std::array::from_fn(|deck| estimated(at[deck]));
        let pal = room.palette();
        // Explicitly verifies color mappings against room palettes to prevent self-referential tautologies.
        let ink = [pal.band_green, pal.band_blue, pal.band_red, pal.band_purple];

        let mut centres = Vec::new();
        for (deck, cell) in cells.iter().enumerate() {
            let caption = caption_of(*cell);
            let painted = dots_in(&mut view, &mut panel, caption);
            let [dot] = painted[..] else {
                panic!(
                    "cell {deck} was budgeted at {} ms and its caption paints {} dot(s)",
                    at[deck],
                    painted.len()
                );
            };
            assert_eq!(
                dot.fill,
                ink[deck],
                "cell {deck} at {} ms is {:?} in the {} room",
                at[deck],
                want[deck],
                room.word()
            );
            // `width: 6px`, as a circle of half that across.
            assert!(near(dot.radius * 2.0, size::PREVIEW_RISK));
            // `margin-left: auto`: hard against the caption's far edge, and
            // centred in its height.
            assert!(near(dot.center.x + dot.radius, caption.max.x));
            assert!(near(dot.center.y, caption.center().y));
            // Inside the caption, which is under the image and never on it —
            // this file's first assertion, reached from the badge's side.
            assert!(caption.contains_rect(dot.visual_bounding_rect()));
            centres.push((dot.center.y, caption.max.x - dot.center.x));
        }

        // Verifies dots across all cells share identical vertical and right-inset coordinates.
        assert!(
            centres
                .windows(2)
                .all(|pair| near(pair[0].0, pair[1].0) && near(pair[0].1, pair[1].1)),
            "the four badges sit at {centres:?} (height, inset) and are meant to read as one"
        );
    }
}

/// Verifies that a slot without a valid budgeted execution cost draws no status dot.
#[test]
fn a_slot_with_no_number_draws_no_dot() {
    let (mut panel, cells) = cells();

    for costs in [
        // A console nobody has governed, and a governed deck the governor had
        // neither number for — the same absent entry, because
        // `Basis::Unbudgetable` is written as one rather than as a zero.
        [None; DECKS],
        // A number that is not a number.
        std::array::from_fn(|_| estimated(f32::NAN)),
        std::array::from_fn(|_| estimated(f32::INFINITY)),
    ] {
        let mut view = with_material();
        view.costs = costs;
        for (deck, cell) in cells.iter().enumerate() {
            let caption = caption_of(*cell);
            let drawn = marks_in(&mut view, &mut panel, caption);
            assert!(
                drawn.is_empty(),
                "cell {deck} has no number and its caption draws {} shape(s) that are not \
                 its two words: {drawn:?}",
                drawn.len()
            );
        }
    }
}

/// Cells without an active slot omit the cost risk dot, ignoring any stale cost data.
#[test]
fn a_cell_with_no_slot_draws_no_dot_even_with_a_number() {
    let (mut panel, cells) = cells();
    let mut view = View::new(Room::Day);
    view.costs = std::array::from_fn(|_| estimated(1.0));

    for (deck, cell) in cells.iter().enumerate() {
        let caption = caption_of(*cell);
        let drawn = marks_in(&mut view, &mut panel, caption);
        assert!(
            drawn.is_empty(),
            "cell {deck} has no slot behind it and its caption draws {drawn:?}"
        );
    }

    // And the word is the one that says so, which is what makes the pair a
    // contradiction rather than two independent readings.
    let painted = texts(&mut view, &mut panel);
    for cell in &cells {
        assert!(in_caption(&painted, caption_of(*cell)).contains(&PREVIEW_NO_SLOT.to_owned()));
    }
}

/// Both estimated and measured budgets render identical risk dots (P-0095, ADR-0296, ADR-0356).
#[test]
fn an_estimate_and_a_measurement_draw_the_same_dot() {
    let (mut panel, cells) = cells();
    let ms = 9.0;
    assert_eq!(band_of(ms), Band::Yellow);

    let mut drawn = Vec::new();
    for basis in [Basis::Estimated, Basis::Measured] {
        let mut view = with_material();
        view.costs = std::array::from_fn(|_| Some(Budgeted { ms, basis }));
        drawn.push(dots_in(&mut view, &mut panel, caption_of(cells[0])));
    }
    let [estimate, measurement] = &drawn[..] else {
        unreachable!()
    };
    assert_eq!(estimate.len(), 1, "an estimated slot draws one dot");
    assert_eq!(
        estimate[0].fill, measurement[0].fill,
        "the two bases are drawn in two colours, and the page does not say they differ"
    );
    assert_eq!(estimate[0].center, measurement[0].center);
    assert_eq!(estimate[0].radius, measurement[0].radius);
}
