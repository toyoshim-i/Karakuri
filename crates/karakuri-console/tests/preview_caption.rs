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
use egui::epaint::CircleShape;
use egui::Rect;
use karakuri_console::room::{size, Room};
use karakuri_console::view::{
    band_of, caption_of, preview_rects, Band, Basis, Budgeted, Picture, View, BAND_BLUE_MS,
    BAND_PURPLE_MS, BAND_RED_MS, BAND_YELLOW_MS, DECKS, DECK_LETTERS, MOCK_CANVAS,
    PREVIEW_MATERIAL, PREVIEW_NO_SLOT,
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

// ---------------------------------------------------------------------------
// The badge
// ---------------------------------------------------------------------------
//
// **What used to be here, and what it cost to retire it.**
// `the_badge_is_absent_rather_than_drawn_empty` asserted that a caption paints
// no shape but text — written when nothing in this workspace could say what a
// slot costs, so a dot of any kind would have been a reading nobody took. It
// **fails only when somebody draws a badge and never when a number arrives**,
// which is why the engine grew
// `a_governed_slot_now_has_a_number_a_band_can_be_predicted_from` beside it
// rather than trusting it (ADR-0296).
//
// It caught exactly one thing this file now has to catch some other way: a
// caption that paints a mark it has no value for. That is
// `a_slot_with_no_number_draws_no_dot` and
// `a_cell_with_no_slot_draws_no_dot_even_with_a_number` together, and they are
// **stronger** than what they replace — the old assertion could not tell a dot
// drawn from a number apart from a dot drawn by default, because it refused
// both. These two hold the refusal exactly where the manual puts it and the
// drawing everywhere else.

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

/// Every shape but text painted inside one caption, which is how the absence
/// of a badge is asserted: it fails for a circle, a filled rectangle, a ring
/// or a glyph standing in for one.
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

/// **The band is read from the number, and the table is the console's.**
///
/// `docs/manual/console.html`, *What a deck preview cell shows, and when*:
/// four slots share one frame, so 16.7 ms at 60 Hz is about 4 ms each, and the
/// five bands are green up to 4 ms, blue over it, yellow about 8, red about 12
/// and purple over 16.
///
/// **This is the test M5.14 item 5 is for**, and it is the console's half of
/// `karakuri-engine`'s
/// `a_governed_slot_now_has_a_number_a_band_can_be_predicted_from`: the engine
/// asserts a number arrives that a band can be predicted from, and this
/// asserts what the prediction is. The engine quotes the boundaries and does
/// not share them, because a millisecond is the engine's and *how many of
/// these at what rate* is this panel's.
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

/// **A value on a boundary rounds to the worse band.**
///
/// The page says it in those words, and it is the one rule of the table that
/// cannot be got right by accident: every comparison is `>=` and the
/// fall-through is green, so 4.0 is blue rather than green and 16.0 stops a
/// slot rather than nearly stopping one.
///
/// **The direction is the same one the whole chain rounds in.**
/// `karakuri_engine::estimate` rounds toward refusing at every step —
/// ADR-0293's correction is applied to the answer for exactly that reason —
/// and a badge that read a boundary kindly would be the one place in the chain
/// that did not.
///
/// The value just under each boundary is asserted beside it, because a table
/// written with `>` instead of `>=` passes every test that only checks the
/// interiors.
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

/// **The dot is drawn in its band's colour, at the far end of the caption.**
///
/// `.risk` is `width: 6px; height: 6px; border-radius: 999px; margin-left:
/// auto`, and the stylesheet's own comment says what the last of those is for:
/// *"pushed to the far end of the caption so the four badges line up down the
/// row and can be read as a column without reading a word"*. So the assertion
/// is not only that a dot exists — it is that the four are one column, which
/// is the whole reason the badge is worth drawing rather than printing.
///
/// Drawn in both rooms, because the five colours are five properties of the
/// palette and a room that took the other room's would be a badge in the wrong
/// five.
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
        // **The five colours are named here rather than asked of
        // [`Band::colour`], and that is the point of the assertion.** A test
        // that asked would compare the mapping with itself: point `Band::Red`
        // at `pal.pink` and both sides move together and it passes — which is
        // what the first draft of this test did, and it was run against
        // exactly that defect and did not fail. What holds the *palette* to
        // the stylesheet is `the_band_colours_are_the_rooms_own_five` in
        // `tests/transcribed_constants_cite_the_mock.rs`, so the two together
        // pin both halves: which room colour a band takes, and what that
        // colour is.
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

        // **The four are one reading**, which is what `margin-left: auto`
        // buys and what the stylesheet's comment is for: the row is four cells
        // side by side, so lining up means one height and one inset from each
        // cell's own far edge — an eye running along the row lands on four
        // dots at the same place in each caption and never on a dot that has
        // moved because the word beside it got longer.
        assert!(
            centres
                .windows(2)
                .all(|pair| near(pair[0].0, pair[1].0) && near(pair[0].1, pair[1].1)),
            "the four badges sit at {centres:?} (height, inset) and are meant to read as one"
        );
    }
}

/// **A slot with no number draws no dot.**
///
/// This is what `the_badge_is_absent_rather_than_drawn_empty` protected and it
/// is asserted the same way — no shape but text inside the caption — so it
/// fails for a circle, a filled rectangle, a ring, or a glyph standing in for
/// one. What has changed is that it now holds only where there is no number,
/// which is the half of that test that was ever a rule.
///
/// **Three nothings take this path and none of them is a small number.**
///
/// - Nobody has governed this deck, which is every other test in this crate.
/// - The governor found neither number for the slot —
///   `karakuri_engine::governor::Basis::Unbudgetable`, which is *not zero* and
///   is why it crosses this seam as an absent entry rather than as a `0.0`
///   ([`Budgeted`]). A `0.0` here would draw green, which is the exact defect.
/// - A number that is not finite. A failed reading is not a cheap Set, which
///   is the engine's own rule for `Unfit::FragmentTermNegative` said once more
///   at the drawing.
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

/// **A cell with no slot behind it draws no dot, whatever it was handed.**
///
/// The mock's D cell and its tooltip: *"No slot is no cost, so there is no
/// dot."* A dot beside `no slot` would be a cost for a thing that is not
/// there, and it is the one way the two halves of a caption could contradict
/// each other — so the picture gates the badge rather than the two being read
/// independently.
///
/// Handed a number deliberately: the caller is the one that can be wrong here,
/// and a console that trusted a stale `costs` entry against a cell whose slot
/// has gone is exactly the frame this defends.
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

/// **An estimate and a measurement draw the same dot, and P-0095 crosses the
/// seam anyway.**
///
/// [`Budgeted::basis`] is `karakuri_engine::governor::Basis`, which says
/// whether the number is the two-draw fit at this deck's own size or the
/// single draw at the reference resolution. **They are not the same
/// statement** — that is P-0095, and ADR-0296 §3 keeps both halves on the
/// decision for it — and this asserts that the console today draws them
/// identically.
///
/// **That is a decision and this is where it is written down.** The number is
/// the one the governor spent, so a badge that appeared only for estimated
/// slots would show a different quantity from the one the deck is governed on
/// and would vanish on every swap (ADR-0296 leaves an incoming Set
/// unestimated). And the mock has five classes on `.risk` and no sixth mark:
/// saying *which* is a page change, and the page moves first. If it moves,
/// **this test is what fails**, which is the point of writing it as an
/// assertion rather than as a comment.
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
