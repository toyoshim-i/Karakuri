//! **The deck preview caption: what is under a cell and what is not on it.**
//!
//! A cell used to write its letter inside the image, at the bottom-left, over
//! whatever the deck was making. `docs/manual/console.html` moved it out on
//! 2026-09-02 — the letter, a word for what the cell is showing, and a badge
//! for what the slot costs, in a row under the image — and the reason is the
//! one the mock states beside the rule: **a letter laid over the material is
//! unreadable exactly when the deck is live and the material is bright**,
//! which is the one moment the row is read fastest.
//!
//! Each test below is named after the sentence it defends, and every one of
//! them was run against the defect it exists for before it was written down.

mod common;

use common::{arranged, near};
use egui::Rect;
use karakuri_console::room::{size, Room};
use karakuri_console::view::{
    caption_of, preview_rects, Picture, View, DECKS, DECK_LETTERS, MOCK_CANVAS, PREVIEW_MATERIAL,
    PREVIEW_NO_SLOT,
};

/// A window with the cells in the row under the picture, and the four
/// rectangles the bay put them in.
fn cells() -> (karakuri_console::panel::Panel, [Rect; DECKS]) {
    let panel = arranged(common::SMALLEST, MOCK_CANVAS);
    let cells = preview_rects(panel.layout(), MOCK_CANVAS).expect("the cells are on screen");
    (panel, cells)
}

/// **A console with something behind every cell.** The texture id is never
/// resolved — nothing here renders — and what it stands for is the only thing
/// under test: a cell with material in it.
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

/// Every text the console paints, with where it was painted.
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

/// **The caption is outside the image, and inside the row.**
///
/// The rectangle a caption is drawn in and the rectangle a texture is drawn in
/// share an edge and no area at all — `preview_rects` is what the engine sizes
/// a slot's texture from, so a caption rectangle that overlapped it would be a
/// caption under texels rather than under the image.
///
/// **And the row is what pays for it.** The caption band comes off the track
/// before the image is fitted into it, so the image plus its caption is the
/// row's height exactly. The defect this exists for is that subtraction being
/// dropped: the image is then fitted into the whole track, `caption_of` puts
/// the caption under it as it always does, and the caption hangs off the
/// bottom of the row into `.program-body`'s padding and the bay's edge. It is
/// the half that is not true by construction, and it is the half that fails.
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

/// **The letter is not painted over the material.**
///
/// Drawn with a picture behind every cell — which is the case the move was
/// made for — nothing at all is written inside any of the four images, and the
/// four letters are all written inside the four captions.
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

/// **The state word is what the cell actually distinguishes.**
///
/// `View::previews` is `Option<Picture>` and nothing else, so a cell has a
/// slot behind it or it has none — and those are the two words. The two words
/// it is *not* are the ones this file exists to keep out:
///
/// - **`off`** is residency, and residency has not gated a cell since
///   ADR-0240. It is what `view::preview` said until this pass, copied out of
///   a `C · off` the mock stopped drawing.
/// - **`empty`** is a slot that exists with nothing loaded into it. The manual
///   names it; `Deck::new` builds a slot per `HotSwap` and `Deck::slot_view`
///   is `None` only past `slot_count`, so the engine cannot be in it and
///   nothing may draw it.
#[test]
fn the_state_word_is_what_the_cell_actually_distinguishes() {
    // The words themselves, and not only the constants that hold them: a
    // rename that took both sides with it would leave every assertion below
    // comparing the code with itself.
    assert_eq!(PREVIEW_MATERIAL, "material");
    assert_eq!(PREVIEW_NO_SLOT, "no slot");

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
            text != "off" && text != "empty" && !text.ends_with("· off"),
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

/// **The badge is absent rather than drawn empty.**
///
/// The mock's `.risk` dot is the slot's estimated cost in five bands and
/// **nothing in this workspace estimates what a slot costs**, so the caption
/// draws the letter, the word and nothing else. A dot drawn hollow, grey or
/// green by default would assert a reading nobody took, which is ADR-0200's
/// rule and ADR-0191's before it.
///
/// Asserted as *no shape but text inside a caption*, because that is what
/// fails for every way of drawing it: a circle, a filled rectangle, a ring, or
/// a glyph standing in for one.
#[test]
fn the_badge_is_absent_rather_than_drawn_empty() {
    let (mut panel, cells) = cells();
    let mut view = with_material();
    let painted = shapes(&mut view, &mut panel);

    for (deck, cell) in cells.iter().enumerate() {
        let caption = caption_of(*cell);
        let drawn: Vec<&egui::Shape> = painted
            .iter()
            .filter(|shape| {
                let bounds = shape.visual_bounding_rect();
                bounds.is_finite() && caption.contains_rect(bounds)
            })
            .filter(|shape| !matches!(shape, egui::Shape::Text(_)))
            .collect();
        assert!(
            drawn.is_empty(),
            "cell {deck}'s caption draws {} shape(s) that are not its two words, and the \
             cost estimate they would be standing for does not exist: {drawn:?}",
            drawn.len()
        );
    }
}
