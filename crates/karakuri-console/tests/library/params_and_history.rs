use super::library_common::*;

// ---------------------------------------------------------------------------
// The reading, and the chip that opens it
// ---------------------------------------------------------------------------

/// A view over the mock's library with that reading open under the first row.
fn showing_reading() -> (View, Panel) {
    let (mut view, panel) = showing_mock();
    view.read(reading());
    (view, panel)
}

/// Asserts `params` chip emits `ReadSet` for the active cursor row and toggles reading expansion closed on second click.
#[test]
fn the_params_chip_asks_for_the_set_under_the_cursor() {
    let (mut view, panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    let chip = bay.params_chip(&ctx, AIMED);
    let probe = Point::new(chip.min.x + 2.0, chip.center().y);
    let ask = |view: &View, bay: &LibraryBay| {
        bay.read(&ctx, AIMED, view.rows().set(view.cursor_row()), probe)
    };

    assert_eq!(
        ask(&view, &bay),
        Some(Read::Open(Operation::ReadSet {
            id: "drift_night".to_owned()
        })),
        "the chip did not ask for the Set under the cursor"
    );

    assert!(
        view.walk(2, bay.drawn()),
        "the cursor did not move two rows"
    );
    assert_eq!(
        ask(&view, &bay),
        Some(Read::Open(Operation::ReadSet {
            id: "glass_shell".to_owned()
        })),
        "the cursor moved and the chip went on naming the row it started on"
    );

    // **The gap before the chip is nobody's**, which is `.lib-foot`'s `gap`
    // rather than a target: the count, the space beside it and the ground
    // between the two capsules take no press.
    let gap = Point::new(chip.min.x - size::LIB_FOOT_GAP * 0.5, chip.center().y);
    assert!(
        ask(&view, &bay).is_some(),
        "the chip itself stopped answering, so the gap beside it measures nothing"
    );
    assert_eq!(
        bay.read(&ctx, AIMED, Some("drift_night"), gap),
        None,
        "the gap between the `params` chip and the `load` button answered a press"
    );

    // **A library that lists nothing has no Set to read**, and the chip is
    // still drawn because the foot is what the count is in. A press on it asks
    // nothing rather than asking for a Set with no name.
    let empty =
        library(panel.layout(), SCOPES, &[], None, None, 0.0).expect("the bay draws its foot");
    let chip = empty.params_chip(&ctx, AIMED);
    assert_eq!(
        empty.read(
            &ctx,
            AIMED,
            None,
            Point::new(chip.center().x, chip.center().y)
        ),
        None,
        "a bay listing nothing asked for a Set"
    );

    // **And with a reading open the same press closes it**, which is the
    // block's own state read off the bay rather than off anything the chip is
    // told.
    let (view, panel) = showing_reading();
    let open = library(
        panel.layout(),
        SCOPES,
        &view.library,
        view.opened(),
        None,
        0.0,
    )
    .expect("the bay lists its rows");
    assert!(open.reading.is_some(), "nothing was open to close");
    let chip = open.params_chip(&ctx, AIMED);
    assert_eq!(
        open.read(
            &ctx,
            AIMED,
            Some("drift_night"),
            Point::new(chip.min.x + 2.0, chip.center().y)
        ),
        Some(Read::Shut),
        "a press with a reading open asked for it again instead of closing it"
    );
}

/// Asserts reading expands under cursor row, shifting lower rows down and adjusting foot count accordingly.
#[test]
fn a_reading_opens_under_the_cursor_row_and_pushes_the_rest_down() {
    let (view, panel) = showing_reading();
    let shut = bay(&panel);
    let open = library(
        panel.layout(),
        SCOPES,
        &view.library,
        view.opened(),
        None,
        0.0,
    )
    .expect("the bay lists its rows");
    let block = open.reading.expect("the reading is open under the cursor");

    // Ten rows: the head, six knobs, the capacity, what it emits, and the
    // foot.
    assert_eq!(reading().rows(), 10, "the reading is not ten lines");
    assert_eq!(block.rows, reading().rows(), "the block is not the reading");

    // The well: under the cursor's row by the block's top margin, inside the
    // list by its side margin, and as tall as its rows.
    assert!(
        near(
            block.well.min.y,
            open.row(0).max.y + size::READING_MARGIN_TOP
        ),
        "the well starts at {} and the cursor's row ends at {}",
        block.well.min.y,
        open.row(0).max.y
    );
    assert!(
        near(block.well.min.x, open.list.min.x + size::READING_MARGIN_X)
            && near(block.well.max.x, open.list.max.x - size::READING_MARGIN_X),
        "the well is {:?} in a list spanning {} to {}",
        block.well,
        open.list.min.x,
        open.list.max.x
    );
    assert!(
        near(block.well.height(), size::LIB_ROW_H * block.rows as f32),
        "the well is {} tall and ten rows are {}",
        block.well.height(),
        size::LIB_ROW_H * block.rows as f32
    );

    // Its rows tile it, at a `.lib-row`'s own height.
    for index in 1..block.rows {
        assert!(
            near(block.row(index).min.y, block.row(index - 1).max.y),
            "the reading's row {index} does not meet the one above it"
        );
    }
    assert!(
        near(block.row(block.rows - 1).max.y, block.well.max.y),
        "the reading's last row ends at {} and the well at {}",
        block.row(block.rows - 1).max.y,
        block.well.max.y
    );

    // The row above it has not moved, and the row below it starts under the
    // block's bottom margin.
    assert!(
        near(open.row(0).min.y, shut.row(0).min.y),
        "the row the reading is under moved when it opened"
    );
    assert!(
        near(
            open.row(1).min.y,
            block.well.max.y + size::READING_MARGIN_BOTTOM
        ),
        "the row under the reading is at {} and the block ends at {}",
        open.row(1).min.y,
        block.well.max.y
    );

    // **The five the mock lists all still fit**, which is what makes the
    // paragraph above about a block between two rows rather than about a
    // shorter list. The bay this column gives the Library is taller than five
    // rows and a reading of ten, and nothing was pushed out.
    assert_eq!(
        open.rows, shut.rows,
        "the mock's five rows and a ten-line reading fit the bay, and {} were listed",
        open.rows
    );

    // **A library taller than its list is where the block costs rows**, and
    // what it costs is the block's own height read in rows — the foot says so
    // in the words it already says a truncated listing in.
    let many: Vec<String> = (0..200).map(|n| format!("set_{n:03}")).collect();
    let mut deep = View::new(Room::Day);
    deep.library = many.clone();
    deep.scopes = Scope::ALL.to_vec();
    let mut of_first = reading();
    of_first.id = many[0].clone();
    deep.read(of_first);
    let full =
        library(panel.layout(), SCOPES, &many, None, None, 0.0).expect("the bay lists its rows");
    let cut = library(panel.layout(), SCOPES, &many, deep.opened(), None, 0.0)
        .expect("the bay lists its rows");
    let block = cut.reading.expect("the reading is open");
    assert!(
        full.rows < full.total,
        "200 names all fitted, so nothing here is measuring what a block costs"
    );
    let took = size::READING_MARGIN_TOP + block.well.height() + size::READING_MARGIN_BOTTOM;
    assert!(
        cut.rows < full.rows,
        "the bay listed {} rows with a {took}-tall block open and {} with none",
        cut.rows,
        full.rows
    );
    // **And it is a count of what fits and not a number chosen**: the last row
    // listed is inside the list and the next one would not have been, which is
    // `the_rows_tile_the_list_and_stay_inside_it`'s claim asked of a list with
    // a block in the middle of it.
    assert!(
        !cut.list.contains_rect(cut.row(cut.rows)),
        "row {} at {:?} would have fitted under the block in {:?} and was not drawn",
        cut.rows,
        cut.row(cut.rows),
        cut.list
    );
    assert_eq!(
        cut.count(),
        format!("{} of 200", cut.rows),
        "the foot stopped saying how many of how many"
    );
    assert!(
        cut.list.contains_rect(cut.row(cut.rows - 1)),
        "the last row the bay listed is outside the list"
    );
}

/// Asserts reading displays only under matching set and automatically closes if list is filtered or rewritten.
#[test]
fn a_reading_is_drawn_only_under_the_row_it_is_a_reading_of() {
    let (mut view, panel) = showing_reading();
    let listed = bay(&panel).rows;
    assert_eq!(
        view.opened().map(|open| open.at),
        Some(0),
        "the reading did not open under the row it was read of"
    );

    // The cursor moves and the reading does not follow it by itself: what is
    // under the cursor is a different Set, and a block left drawn there would
    // be `lattice_veil` described as `drift_night`.
    assert!(view.walk(1, 0..listed), "the cursor did not move");
    assert!(
        view.opened().is_none(),
        "the reading of `drift_night` is drawn under `lattice_veil`"
    );
    assert!(view.walk(-1, 0..listed), "the cursor did not move back");
    assert!(
        view.opened().is_some(),
        "the reading did not come back with the cursor"
    );

    // And a listing rewritten under it closes it for the same reason, even
    // where the cursor has not moved at all.
    view.library = vec!["night01".to_owned(), "drift_night".to_owned()];
    assert!(
        view.opened().is_none(),
        "the reading is drawn under whatever took the row it was read of"
    );

    // Putting it away is the console's own state and says whether there was
    // one to put away.
    assert!(view.shut_reading(), "there was nothing open to shut");
    assert!(
        !view.shut_reading(),
        "shutting nothing said it shut something"
    );
    assert!(view.opened().is_none());
}

/// Asserts `params` chip clears top/left/right boundaries while its bottom rim touches footer boundary grab.
#[test]
fn the_params_chip_clears_every_boundary_but_the_one_under_the_bay() {
    let (view, mut panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    let chip = bay.params_chip(&ctx, AIMED);
    let region = to_egui(rect_of(panel.layout(), "library"));

    // The three edges that clear it: the two ends of the capsule and its top.
    for probe in [
        Point::new(chip.min.x + 1.0, chip.center().y),
        Point::new(chip.max.x - 1.0, chip.center().y),
        Point::new(chip.center().x, chip.min.y + 1.0),
    ] {
        assert!(
            !matches!(
                panel.layout().hit(probe, GRAB),
                karakuri_layout::Hit::Divider { .. }
            ),
            "a boundary grabs {probe:?}, which is on the `params` chip"
        );
        assert_eq!(
            claim(&mut panel, &ctx, &view, probe),
            Claim::Panel,
            "the console gave `egui` a press at {probe:?} on the `params` chip"
        );
    }

    // And the rim that does not, with the arithmetic said out loud so that a
    // change to either number is a change to this sentence.
    let rim = region.max.y - chip.max.y;
    assert!(
        near(rim, (size::LIB_FOOT_H - size::PILL_H) * 0.5),
        "the chip's bottom rim is {rim} off the bay's edge and the foot leaves {}",
        (size::LIB_FOOT_H - size::PILL_H) * 0.5
    );
    assert!(
        rim < GRAB,
        "the chip clears the boundary by {rim} against a grab of {GRAB}, so this test is \
         measuring nothing"
    );
    assert!(
        matches!(
            panel
                .layout()
                .hit(Point::new(chip.center().x, chip.max.y - 0.5), GRAB),
            karakuri_layout::Hit::Divider { .. }
        ),
        "the pixel at the bottom of the chip is not the boundary's, so nothing here measured a \
         grab"
    );
}

/// Asserts reading renders declaration lines, unread markers, and node counts accurately.
#[test]
fn a_reading_draws_a_line_per_declaration_and_counts_what_it_could_not_read() {
    let (mut view, mut panel) = showing_reading();
    let open = library(
        panel.layout(),
        SCOPES,
        &view.library,
        view.opened(),
        None,
        0.0,
    )
    .expect("the bay lists its rows");
    let block = open.reading.expect("the reading is open");
    let drawn = shapes_inside(&mut view, &mut panel, block.well);
    let words: Vec<String> = drawn
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Text(at) => Some(at.galley.text().to_owned()),
            _ => None,
        })
        .collect();

    for want in [
        "declares",
        "6 knobs",
        "radius",
        "0 – 8 · 2",
        "exposure",
        "capacity",
        "16384 – 1048576 · 262144",
        "emits",
        "position, size, life",
        "5 nodes",
        "all described",
    ] {
        assert!(
            words.iter().any(|word| word == want),
            "the reading does not say `{want}`: {words:?}"
        );
    }
    // **The well itself**, so a box of type standing on the card cannot pass
    // this: the mock draws these rows on the recess the staging lane's
    // candidates stand on.
    assert!(
        drawn.iter().any(|shape| matches!(
            shape,
            egui::Shape::Rect(at) if near(at.rect.min.y, block.well.min.y)
                && near(at.rect.height(), block.well.height())
        )),
        "nothing filled the reading's well at {:?}: {drawn:#?}",
        block.well
    );
    // **And nothing in it is a figure this reading did not pay for.** The
    // element storage is bytes, and no line here says so.
    assert!(
        !words.iter().any(|word| word.contains("bytes")),
        "the reading draws an element-storage figure, which needs every source in the Set \
         fetched and compiled: {words:?}"
    );

    // A Set with a node the store holds no card for: the knob list is short by
    // whatever that node declared, and the foot is what says so.
    let mut short = reading();
    short.described = 3;
    view.read(short);
    let open = library(
        panel.layout(),
        SCOPES,
        &view.library,
        view.opened(),
        None,
        0.0,
    )
    .expect("the bay lists its rows");
    let block = open.reading.expect("the reading is open");
    let drawn = shapes_inside(&mut view, &mut panel, block.well);
    assert!(
        drawn.iter().any(|shape| matches!(
            shape,
            egui::Shape::Text(at) if at.galley.text() == "2 without a card"
        )),
        "two nodes declared nothing this could read and the foot did not say so: {drawn:#?}"
    );
}

/// Empty readings display `0 knobs` and suppress empty declaration rows.
#[test]
fn a_reading_with_nothing_to_declare_says_so_rather_than_drawing_blanks() {
    let (mut view, mut panel) = showing_mock();
    view.read(Reading {
        id: "drift_night".to_owned(),
        knobs: Vec::new(),
        capacity: None,
        emits: None,
        nodes: 1,
        described: 1,
    });
    let open = library(
        panel.layout(),
        SCOPES,
        &view.library,
        view.opened(),
        None,
        0.0,
    )
    .expect("the bay lists its rows");
    let block = open.reading.expect("the reading is open");
    assert_eq!(block.rows, 2, "a head and a foot are the whole of it");

    let drawn = shapes_inside(&mut view, &mut panel, block.well);
    let words: Vec<String> = drawn
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Text(at) => Some(at.galley.text().to_owned()),
            _ => None,
        })
        .collect();
    assert!(
        words.iter().any(|word| word == "0 knobs") && words.iter().any(|word| word == "1 node"),
        "the head and the foot do not say what there is: {words:?}"
    );
    assert!(
        !words
            .iter()
            .any(|word| word == "capacity" || word == "emits"),
        "a row was drawn with nothing after it: {words:?}"
    );
}

/// Verifies that the parameters chip displays `params` and indicates lit state
/// while a reading is open (ADR-0312).
#[test]
fn the_params_chip_says_params_and_lights_while_a_reading_is_open() {
    let (mut view, mut panel) = showing_mock();
    for open in [false, true] {
        if open {
            view.read(reading());
        }
        let said = match open {
            true => "open",
            false => "shut",
        };
        let foot = bay(&panel).foot;
        let drawn = shapes_inside(&mut view, &mut panel, foot);
        assert!(
            drawn.iter().any(|shape| matches!(
                shape,
                egui::Shape::Text(at) if at.galley.text() == PARAMS_WORD
            )),
            "the foot does not say `{PARAMS_WORD}` with the reading {said}: {drawn:#?}"
        );
        let ctx = drawn_once();
        let chip = bay(&panel).params_chip(&ctx, AIMED);
        let filled = drawn.iter().any(|shape| {
            matches!(
                shape,
                egui::Shape::Rect(at) if near(at.rect.min.x, chip.min.x)
                    && near(at.rect.width(), chip.width())
                    && at.fill != egui::Color32::TRANSPARENT
            )
        });
        assert_eq!(
            filled,
            open,
            "the `params` chip is {} with the reading {said}, and a toggle says which of the two \
             the next press will be",
            match filled {
                true => "filled",
                false => "not filled",
            }
        );
    }
}

/// The word the chip is drawn with, which is the mock's own and is not a
/// constant this crate exports: it is asserted here because a chip reading
/// something else is a control the note does not describe.
const PARAMS_WORD: &str = "params";

// ---------------------------------------------------------------------------
// The `history` scope: the fifth chip, its rows, and the landing
// ---------------------------------------------------------------------------

/// Three versions in newest-first host order (ADR-0156, ADR-0263).
fn versions() -> Vec<String> {
    [
        "20260908-143052-271_slot0_L4_beat_strokes",
        "20260908-142930-004_slot0_L1_drift_shell",
        "20260907-235959-999_slot2_L41_soft_points",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect()
}

/// Verifies the fifth chip is `history` and pressing it requests `Operation::WalkHistory` (ADR-0308).
#[test]
fn the_history_chip_is_the_fifth_and_a_press_on_it_asks_for_the_walk() {
    assert_eq!(
        Scope::ALL.last().copied(),
        Some(Scope::History),
        "`history` is not the last chip the bay draws"
    );
    assert_eq!(Scope::History.name(), "history");
    assert!(
        !Scope::History.lists_sets(),
        "a row of `history` is a version and not a Set"
    );

    let (mut view, mut panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    assert_eq!(
        chips(&mut view, &mut panel, &bay)
            .last()
            .map(String::as_str),
        Some("history"),
        "the row's last word is not the fifth chip's"
    );

    let (scope, chip) = bay
        .chips(&ctx, SCOPES)
        .last()
        .expect("the row has chips in it");
    assert_eq!(scope, Scope::History);
    let probe = egui::pos2(chip.min.x + 2.0, chip.center().y);
    let chosen = bay
        .chip(&ctx, SCOPES, Point::new(probe.x, probe.y))
        .expect("no chip answered a press on `history`");
    assert_eq!(chosen.scope, Scope::History);
    assert_eq!(
        chosen.asked(Some("night01")),
        Operation::WalkHistory {
            set: Some("night01".to_owned())
        },
        "the `history` chip asked for something other than `Walk the edit history`"
    );
    // **And the Set is the host's answer rather than the chip's**: the console
    // holds a deck letter, the id rides the aim, and a walk aimed at a deck
    // running the pair the run launched with names no Set — which lists
    // nothing rather than everything (ADR-0276, ADR-0308).
    assert_eq!(
        chosen.asked(None),
        Operation::WalkHistory { set: None },
        "a walk with no Set aimed at invented one"
    );

    assert!(view.select_scope(Scope::History), "the mark did not move");
    assert_eq!(
        marked(&mut view, &mut panel, &bay).as_deref(),
        Some("history"),
        "the chip was chosen and the wash stayed where it was"
    );
}

/// Verifies `history` preserves host order (ADR-0263) and foot counts rows correctly.
#[test]
fn a_history_listing_is_the_rows_the_host_handed_in_and_the_foot_counts_them() {
    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    view.library = versions();
    assert!(view.select_scope(Scope::History));
    let mut panel = console(PLAUSIBLE);
    let bay = library(panel.layout(), SCOPES, &view.library, None, None, 0.0)
        .expect("the library bay lists its rows");
    assert_eq!(bay.rows, versions().len(), "the bay drew a different count");
    assert_eq!(bay.count(), "3 of 3");

    let mut down: Vec<(f32, String)> = shapes_inside(&mut view, &mut panel, bay.list)
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Text(at) => Some((at.pos.y, at.galley.text().to_owned())),
            _ => None,
        })
        .collect();
    down.sort_by(|a, b| a.0.total_cmp(&b.0));
    let painted: Vec<String> = down.into_iter().map(|(_, text)| text).collect();
    assert_eq!(
        painted,
        versions(),
        "the rows were drawn in an order this bay invented"
    );

    // **A history taller than the bay**, which is the foot's second number
    // doing the only thing it is for: the rows are what fit and the total is
    // what the host handed over.
    let many: Vec<String> = (0..60)
        .map(|at| format!("20260908-1430{at:02}-000_slot0_L4_beat_strokes"))
        .collect();
    let tall =
        library(panel.layout(), SCOPES, &many, None, None, 0.0).expect("the bay draws its foot");
    assert!(
        tall.rows < many.len(),
        "sixty versions fit in the bay, so the foot has nothing to report"
    );
    assert_eq!(tall.count(), format!("{} of {}", tall.rows, many.len()));
}

/// Verifies pressing a history row restores that version to the target deck (ADR-0308).
#[test]
fn a_press_on_a_history_row_lands_that_version_on_the_pulldowns_deck() {
    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    view.library = versions();
    assert!(view.select_scope(Scope::History));
    view.mixer = std::iter::repeat_with(strip).take(4).collect();
    assert!(view.select(2), "the keys did not go to deck C");
    assert!(view.aim_at(1), "the load was not aimed at deck B");

    let panel = console(PLAUSIBLE);
    let bay = library(panel.layout(), SCOPES, &view.library, None, None, 0.0)
        .expect("the library bay lists its rows");
    let row = bay.row(1);
    let probe = Point::new(row.center().x, row.center().y);

    assert_eq!(
        bay.land(view.versions(), view.target(), probe),
        Some(Operation::RestoreProcedure {
            deck: 1,
            revision: karakuri_operation::Revision::Picked(versions()[1].clone()),
        }),
        "the press did not land the row's own version on the pulldown's deck"
    );
    assert_eq!(
        view.selection(),
        2,
        "landing a version moved the deck selection"
    );
    assert_eq!(
        bay.take(view.rows(), probe),
        None,
        "a row of `history` was taken in hand as though it were a Set"
    );

    // **And the other way round**: back on a library scope the rows are Sets,
    // so the carry answers and the landing does not.
    assert!(view.select_scope(Scope::AllSets));
    view.library = mock();
    assert!(
        bay.take(view.rows(), probe).is_some(),
        "a row of `all` was not taken in hand"
    );
    assert_eq!(
        bay.land(view.versions(), view.target(), probe),
        None,
        "a row of `all` was landed as though it were a version"
    );
}

/// Verifies an empty history draws no rows and ignores presses (ADR-0276).
#[test]
fn a_deck_running_no_set_draws_no_history_rows() {
    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    assert!(view.select_scope(Scope::History));
    assert!(view.library.is_empty());

    let panel = console(PLAUSIBLE);
    let bay = library(panel.layout(), SCOPES, &view.library, None, None, 0.0)
        .expect("the bay draws its scopes and its foot");
    assert_eq!(bay.rows, 0, "a scope with nothing in it drew rows");
    assert_eq!(bay.count(), "0 of 0");
    assert_eq!(
        bay.land(
            view.versions(),
            view.target(),
            Point::new(bay.list.center().x, bay.list.min.y + 1.0)
        ),
        None,
        "a press on the empty list landed a version nobody can see"
    );
}
