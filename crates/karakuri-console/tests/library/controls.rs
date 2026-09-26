use super::library_common::*;

// ---------------------------------------------------------------------------
// Which of this bay's parts answer a press, and nothing is stored
// ---------------------------------------------------------------------------

/// Asserts that scope chips, filter fields, foot capsules, and list rows claim pointer input, while surrounding bay ground claims nothing (ADR-0305).
#[test]
fn the_chips_the_fields_the_params_chip_and_the_rows_are_the_bays_controls_and_nothing_else_is() {
    let (view, mut panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    let region = to_egui(rect_of(panel.layout(), "library"));
    let row = bay.scopes.expect("the bay was handed scopes");

    // **Every chip, asked two pixels in from its own left edge.** Not the
    // centre: the last chip runs out past the bay and its centre can be
    // outside the row, which is the clip this row is drawn with and is
    // [`a_chip_is_pressed_only_where_it_is_drawn`]'s subject.
    let chips: Vec<(Scope, egui::Rect)> = bay.chips(&ctx, SCOPES).collect();
    assert_eq!(
        chips.len(),
        SCOPES.len(),
        "the bay was handed {} scopes and laid out {} chips",
        SCOPES.len(),
        chips.len()
    );
    for (scope, chip) in &chips {
        let probe = egui::pos2(chip.min.x + 2.0, chip.center().y);
        assert!(
            row.contains(probe),
            "`{}`'s own left edge is outside the scope row, so this test is asking about a \
             capsule nobody drew",
            scope.name()
        );
        assert!(
            !matches!(
                panel.layout().hit(Point::new(probe.x, probe.y), GRAB),
                karakuri_layout::Hit::Divider { .. }
            ),
            "a boundary grabs {probe:?}, which is on the `{}` chip — rule 3 gives it first \
             refusal and the chip would be dead there",
            scope.name()
        );
        assert_eq!(
            claim(&mut panel, &ctx, &view, Point::new(probe.x, probe.y)),
            Claim::Panel,
            "the console gave `egui` a press on the `{}` chip",
            scope.name()
        );
    }

    // Surrounding bay ground and non-interactive areas fall through to egui, inset beyond GRAB zones.
    let inset = GRAB + 2.0;
    let mut points = vec![
        egui::pos2(region.min.x + inset, region.min.y + inset),
        egui::pos2(region.max.x - inset, region.min.y + inset),
        egui::pos2(region.min.x + inset, region.max.y - inset),
        egui::pos2(region.max.x - inset, region.max.y - inset),
        region.center(),
    ];
    // **The scope row's own ground**: its left padding, the gap between the
    // first two chips, and the band above the capsules. `.scope` is a capsule
    // and not a cell — the row is not a segmented control — so the space
    // between two of them belongs to nobody.
    points.push(egui::pos2(
        row.min.x + size::SCOPES_PAD_X * 0.5,
        row.center().y,
    ));
    points.push(egui::pos2(
        chips[0].1.max.x + size::SCOPES_GAP * 0.5,
        row.center().y,
    ));
    points.push(egui::pos2(
        chips[0].1.center().x,
        row.min.y + size::SCOPES_PAD_Y * 0.5,
    ));
    // **The filter row's own ground**: its left padding and the gap between the
    // two fields. `.field` is a capsule the same way `.scope` is, so what is
    // between two of them belongs to nobody.
    let filters = bay.filters.expect("the bay draws its filter row");
    let holds = bay.field(Field::Holds).expect("the `holds` field");
    points.push(egui::pos2(
        filters.min.x + size::LIB_FILTERS_PAD_X * 0.5,
        filters.center().y,
    ));
    points.push(egui::pos2(
        holds.max.x + size::LIB_FILTERS_GAP * 0.5,
        filters.center().y,
    ));
    let (left, right) = (
        bay.foot.min.x + size::LIB_FOOT_PAD_X,
        bay.foot.max.x - size::LIB_FOOT_PAD_X,
    );
    assert!(
        right - left > 0.0,
        "the foot's content box is {left} to {right}, which is no box to sweep"
    );
    // Foot capsules are verified separately; checks foot ground, count, and inert arrow label.
    let chip = bay.params_chip(&ctx, AIMED);
    let load = bay.load(&ctx, AIMED);
    let mut swept = 0;
    for step in 0..=10 {
        let t = step as f32 / 10.0;
        let p = egui::pos2(left + (right - left) * t, bay.foot.center().y);
        if chip.contains(p) || load.button.contains(p) || load.deck.contains(p) {
            continue;
        }
        points.push(p);
        swept += 1;
    }
    assert!(
        swept >= 4,
        "only {swept} points of the foot's ground were swept, and the three capsules cannot be \
         most of a row this wide"
    );
    // **The label's own box**, asked explicitly rather than left to the sweep
    // above: it is between two controls and a press on it must belong to
    // neither of them.
    points.push(load.arrow.center());
    // **The list's own ground under the last row**, which is where the rows
    // stop and the foot has not started: a `.lib-row` is a stride and the list
    // is whatever is left of the bay, so what is below the last of them is
    // nobody's. The rows themselves are the panel's and are asked below.
    let last = bay.row(bay.rows - 1);
    let ground = egui::pos2(last.center().x, last.max.y + size::LIB_ROW_H * 0.5);
    assert!(
        ground.y < bay.foot.min.y,
        "the sweep is asking about the foot rather than about the list's ground"
    );
    points.push(ground);

    let mut asked = 0;
    for p in &points {
        assert_eq!(
            claim(&mut panel, &ctx, &view, Point::new(p.x, p.y)),
            Claim::Egui,
            "the console took the pointer at {p:?}, which is inside the Library bay and is not \
             on a scope chip"
        );
        asked += 1;
    }
    // **And every drawn row is the panel's**, asked at three points across it:
    // a press on one takes that Set in hand, and a row that went to `egui`
    // would be the one gesture this bay exists for reaching nothing at all.
    for index in 0..bay.rows {
        let at = bay.row(index);
        for probe in [
            egui::pos2(at.min.x + size::LIB_ROW_PAD_X, at.center().y),
            at.center(),
            egui::pos2(at.max.x - size::LIB_ROW_PAD_X, at.center().y),
        ] {
            assert_eq!(
                claim(&mut panel, &ctx, &view, Point::new(probe.x, probe.y)),
                Claim::Panel,
                "the console gave `egui` a press on row {index} at {probe:?}"
            );
        }
    }

    // Foot capsules claim panel input when hit slightly inset from their left edges.
    for (what, capsule) in [("read", chip), ("load", load.button), ("deck", load.deck)] {
        let probe = egui::pos2(capsule.min.x + 2.0, capsule.center().y);
        assert_eq!(
            claim(&mut panel, &ctx, &view, Point::new(probe.x, probe.y)),
            Claim::Panel,
            "the console gave `egui` a press on the `{what}` capsule at {probe:?}"
        );
    }

    // A guard, so this cannot pass by testing nothing — and a floor under the
    // reach, so the foot's sweep, the scope row's ground and the rows cannot
    // quietly go away.
    assert_eq!(asked, points.len(), "not every point was asked");
    assert!(
        points.len() >= 1 + 5 + 3 + 2 + 5,
        "only {} points asked: the list's own ground, the bay's corners, the scope row's ground, \
         the filter row's and the foot's are the floor, and fewer is a sweep that has stopped \
         covering the foot",
        points.len()
    );
}

/// The foot arrow label (`→`) takes no press, while list rows initiate drag operations (ADR-0305).
#[test]
fn the_label_between_the_two_capsules_takes_no_press_and_a_row_is_where_the_drag_begins() {
    let (mut view, mut panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    view.mixer = std::iter::repeat_with(strip).take(4).collect();

    // **The three, as wide as what is in them** — asked of `LibraryBay::load`,
    // which is the derivation the paint uses, so the boxes swept here are the
    // boxes drawn.
    let load = bay.load(&ctx, view.target());
    for (what, box_) in [
        ("load", load.button),
        ("→", load.arrow),
        ("deck", load.deck),
    ] {
        assert!(
            bay.foot.contains_rect(box_),
            "the `{what}` box at {box_:?} is not inside the foot at {:?}",
            bay.foot
        );
    }

    // Tests across label centerline; capsule top/bottom edges lie within boundary grab zones (GRAB 6).
    let points: Vec<egui::Pos2> = (0..=4)
        .map(|step| {
            let t = step as f32 / 4.0;
            egui::pos2(
                load.arrow.min.x + load.arrow.width() * t,
                load.arrow.center().y,
            )
        })
        .collect();
    assert!(
        bay.rows > 0,
        "the bay drew no rows, so the contrast below is measuring nothing"
    );

    for p in &points {
        assert_eq!(
            claim(&mut panel, &ctx, &view, Point::new(p.x, p.y)),
            Claim::Egui,
            "the console took the pointer at {p:?} — the `→` says how to read the two capsules \
             either side of it and is not one of them"
        );
    }

    // **And the contrast, on the two capsules the label sits between**, so
    // that this cannot pass by a foot that takes no press anywhere: the label
    // is `egui`'s and both capsules are the panel's, in one console in one
    // state.
    for (what, capsule) in [("load", load.button), ("deck", load.deck)] {
        let at = capsule.center();
        assert_eq!(
            claim(&mut panel, &ctx, &view, Point::new(at.x, at.y)),
            Claim::Panel,
            "the `{what}` capsule beside the label went to `egui`, so the sweep above is \
             measuring a foot that takes no press at all"
        );
    }

    // **And the row the drag begins on**, which is the half of this test that
    // did not move.
    let first = bay.row(0).center();
    assert_eq!(
        claim(&mut panel, &ctx, &view, Point::new(first.x, first.y)),
        Claim::Panel,
        "the row the drag begins on went to `egui`, so the sweep above is measuring a bay that \
         takes no press at all"
    );
}

/// Star icon is positioned within the row, clears divider grab zones, and toggles row state (ADR-0299).
#[test]
fn a_star_is_inside_its_row_and_names_the_state_the_row_is_not_in() {
    let (view, mut panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    assert!(bay.rows >= 2, "the bay drew {} rows", bay.rows);

    for index in 0..bay.rows {
        let star = bay.star(index);
        let row = bay.row(index);
        assert!(
            row.contains_rect(star),
            "the star at {star:?} is not inside row {index} at {row:?}"
        );
        assert!(
            star.min.x - bay.list.min.x >= 0.0,
            "the star at {star:?} hangs off the left of the list at {:?}",
            bay.list
        );
        // **A boundary would take the press before any control did**, which is
        // `input::claim`'s rule 3 and is what the two filter fields are
        // measured against one row up.
        for p in [
            egui::pos2(star.min.x, star.center().y),
            egui::pos2(star.max.x, star.center().y),
        ] {
            assert!(
                !matches!(
                    panel.layout().hit(Point::new(p.x, p.y), GRAB),
                    karakuri_layout::Hit::Divider { .. }
                ),
                "a boundary grabs {p:?}, which is on row {index}'s star"
            );
        }
        assert_eq!(
            claim(
                &mut panel,
                &ctx,
                &view,
                Point::new(star.center().x, star.center().y)
            ),
            Claim::Panel,
            "the console gave `egui` a press on row {index}'s star"
        );
    }

    // **Nothing starred: every press asks for the star to go on.**
    let none = std::collections::BTreeSet::new();
    let at = bay.star(1).center();
    assert_eq!(
        bay.starred(listed(&mock()), &none, Point::new(at.x, at.y)),
        Some(Operation::SetFavourite {
            id: "lattice_veil".to_owned(),
            favourite: true,
        }),
        "the star did not name the row it is drawn on"
    );

    // **And with that row starred it asks for the star to come off**, which is
    // the same derivation reading the state it is drawn from.
    let one: std::collections::BTreeSet<String> =
        std::iter::once("lattice_veil".to_owned()).collect();
    assert_eq!(
        bay.starred(listed(&mock()), &one, Point::new(at.x, at.y)),
        Some(Operation::SetFavourite {
            id: "lattice_veil".to_owned(),
            favourite: false,
        }),
        "a starred row was asked to be starred again"
    );

    // **The row's own ground is not the star's**, which is rule 4's *a control
    // claims what it acts on and no more*: the far end of the same row is
    // where the carry begins and the mark answers nothing there.
    let ground = egui::pos2(bay.row(1).max.x - size::LIB_ROW_PAD_X, at.y);
    assert_eq!(
        bay.starred(listed(&mock()), &none, Point::new(ground.x, ground.y)),
        None,
        "the star answered a press at the far end of its row"
    );

    // **And a listing shorter than the rows drawn takes nothing**, which is
    // `take`'s refusal rather than a clamp: a star answered bare would name a
    // Set nobody can see.
    assert_eq!(
        bay.starred(Rows::NONE, &none, Point::new(at.x, at.y)),
        None,
        "the star named a Set in a listing with nothing in it"
    );
}

/// Scope chip presses emit `SelectScope` naming the targeted chip directly rather than relative stepping (P-0090).
#[test]
fn a_press_names_the_chip_it_landed_on_and_never_the_next_one() {
    let (mut view, panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    let row = bay.scopes.expect("the bay was handed scopes");

    for (scope, chip) in bay.chips(&ctx, SCOPES) {
        let probe = egui::pos2(chip.min.x + 2.0, chip.center().y);
        assert!(
            row.contains(probe),
            "`{}`'s left edge is off the row",
            scope.name()
        );
        let chosen = bay
            .chip(&ctx, SCOPES, Point::new(probe.x, probe.y))
            .unwrap_or_else(|| panic!("no chip answered a press on `{}`", scope.name()));
        assert_eq!(
            chosen.scope,
            scope,
            "a press on `{}` asked for `{}`",
            scope.name(),
            chosen.scope.name()
        );
        // History chip triggers history navigation rather than set scope selection.
        assert_eq!(
            chosen.asked(Some("night01")),
            match scope {
                Scope::History => karakuri_operation::Operation::WalkHistory {
                    set: Some("night01".to_owned())
                },
                _ => karakuri_operation::Operation::SelectScope {
                    scope: karakuri_operation::Undecided
                },
            },
            "a press on `{}` asked for the wrong row of the page",
            scope.name()
        );
    }

    // **The chip that is already marked asks for itself**, where the key would
    // step off it. The two are asked of one console in one state, so this is
    // the contrast and not two facts side by side.
    assert!(view.select_scope(Scope::MySets));
    let (_, chip) = bay
        .chips(&ctx, SCOPES)
        .find(|(scope, _)| *scope == Scope::MySets)
        .expect("`my sets` is on the row");
    let probe = egui::pos2(chip.min.x + 2.0, chip.center().y);
    assert_eq!(
        bay.chip(&ctx, SCOPES, Point::new(probe.x, probe.y))
            .map(|chosen| chosen.scope),
        Some(Scope::MySets),
        "a press on the marked chip stepped somewhere"
    );
    assert!(view.step_scope());
    assert_eq!(
        view.scope(),
        Some(Scope::Presets),
        "the key does not step where the pointer names, so this contrast is measuring nothing"
    );
}

/// Scope chips clipped at bay boundaries respond to input only within visible areas (ADR-0299, ADR-0308).
#[test]
fn a_chip_is_pressed_only_where_it_is_drawn() {
    let mut layout = solved(PLAUSIBLE);
    let left = id_of(&layout, "left-pane");
    let split = layout.parent(left).expect("left-pane has a parent split");
    layout.set_divider(split, 0, 190.0);
    layout.solve();
    let ctx = drawn_once();
    let bay =
        library(&layout, SCOPES, &mock(), None, None, 0.0).expect("the library bay lists its rows");
    let row = bay.scopes.expect("the bay was handed scopes");

    let (scope, last) = bay
        .chips(&ctx, SCOPES)
        .find(|(_, chip)| chip.min.x < row.max.x && chip.max.x > row.max.x)
        .expect(
            "no chip straddles the row's right edge — the pane is wide enough to hold every \
             one of them, so there is no clipped capsule to ask about",
        );

    let over = egui::pos2((last.max.x + row.max.x) * 0.5, last.center().y);
    assert!(
        last.contains(over) && !row.contains(over),
        "{over:?} is not on the part of `{}` that hangs outside the row",
        scope.name()
    );
    assert_eq!(
        bay.chip(&ctx, SCOPES, Point::new(over.x, over.y)),
        None,
        "the console answered a press on the part of `{}` it does not draw",
        scope.name()
    );

    // **And the ground of the row answers nothing either** — its left padding
    // and the gap between two capsules, which is what makes the row a row of
    // chips rather than a segmented control.
    for (p, what) in [
        (
            egui::pos2(row.min.x + size::SCOPES_PAD_X * 0.5, row.center().y),
            "the row's left padding",
        ),
        (
            egui::pos2(row.min.x + 1.0, row.min.y + 1.0),
            "the row's top-left corner",
        ),
    ] {
        assert_eq!(
            bay.chip(&ctx, SCOPES, Point::new(p.x, p.y)),
            None,
            "a chip answered a press on {what}"
        );
    }
}

/// Verifies that scope chip hit-test capsules exactly match painted wash boundaries.
#[test]
fn the_capsule_a_press_lands_on_is_the_capsule_the_wash_is_drawn_in() {
    let (mut view, mut panel) = showing_mock();
    let ctx = drawn_once();
    let bay = bay(&panel);
    let row = bay.scopes.expect("the bay was handed scopes");

    for want in Scope::ALL {
        assert!(view.select_scope(want) || view.scope() == Some(want));
        let washes: Vec<egui::Rect> = shapes_across(&mut view, &mut panel, row)
            .iter()
            .filter_map(|shape| match shape {
                egui::Shape::Rect(at) => Some(at.rect),
                _ => None,
            })
            .collect();
        assert_eq!(
            washes.len(),
            1,
            "`{}` is marked and {} chips are washed",
            want.name(),
            washes.len()
        );
        let (_, chip) = bay
            .chips(&ctx, SCOPES)
            .find(|(scope, _)| *scope == want)
            .expect("the marked scope is on the row");
        assert!(
            near(washes[0].min.x, chip.min.x)
                && near(washes[0].min.y, chip.min.y)
                && near(washes[0].width(), chip.width())
                && near(washes[0].height(), chip.height()),
            "`{}` is washed at {:?} and hit-tested at {chip:?}",
            want.name(),
            washes[0]
        );
    }
}

/// Scope chips clear top/left boundary grab zones, with any overrunning trailing edge clipped at the divider.
#[test]
fn the_chips_clear_every_boundary_but_the_one_the_row_is_clipped_by() {
    let panel = console(PLAUSIBLE);
    let ctx = drawn_once();
    let bay = bay(&panel);
    let region = to_egui(rect_of(panel.layout(), "library"));
    let row = bay.scopes.expect("the bay was handed scopes");

    let above = row.min.y + size::SCOPES_PAD_Y - region.min.y;
    assert!(
        near(above, size::HEAD_H + size::SCOPES_PAD_Y),
        "the chips are {above} below the bay's top edge and the bay head plus `.scopes`' padding \
         is {}",
        size::HEAD_H + size::SCOPES_PAD_Y
    );
    assert!(
        above > GRAB,
        "the chips are {above} below the bay's top edge and a boundary grabs {GRAB}"
    );

    for (scope, chip) in bay.chips(&ctx, SCOPES) {
        let left = chip.min.x - region.min.x;
        assert!(
            left >= size::SCOPES_PAD_X,
            "`{}` starts {left} in from the bay's left edge and `.scopes`' padding is {}",
            scope.name(),
            size::SCOPES_PAD_X
        );
        assert!(
            left > GRAB,
            "`{}` starts {left} in from the bay's left edge and a boundary grabs {GRAB}",
            scope.name()
        );
        // **Where the chip is drawn, no boundary has it.** The tail of the
        // last one is outside the row and is nobody's business here.
        let drawn = chip.intersect(row);
        if drawn.width() <= 0.0 {
            continue;
        }
        let probe = egui::pos2(drawn.min.x + 1.0, drawn.center().y);
        assert!(
            !matches!(
                panel.layout().hit(Point::new(probe.x, probe.y), GRAB),
                karakuri_layout::Hit::Divider { .. }
            ),
            "a boundary grabs {probe:?}, which is on `{}` where it is drawn",
            scope.name()
        );
    }

    // **And the band at the far end of the row is still a boundary's**, which
    // is what says the clearances above are clearances rather than the grab
    // having gone missing.
    let into = Point::new(row.max.x - 1.0, row.center().y);
    assert!(
        matches!(
            panel.layout().hit(into, GRAB),
            karakuri_layout::Hit::Divider { .. }
        ),
        "the pixel at the far end of the scope row is not the pane divider's, so nothing here \
         measured a grab"
    );
}

/// Bay representation is dynamically derived each frame rather than cached.
#[test]
fn the_names_are_the_harnesss_and_are_stored_nowhere() {
    let panel = console(PLAUSIBLE);
    let one = vec!["morph01".to_owned()];
    let two = vec!["morph01".to_owned(), "night01".to_owned()];
    assert_eq!(
        library(panel.layout(), SCOPES, &one, None, None, 0.0).map(|b| b.total),
        Some(1)
    );
    assert_eq!(
        library(panel.layout(), SCOPES, &two, None, None, 0.0).map(|b| b.total),
        Some(2)
    );
    assert_eq!(
        library(panel.layout(), SCOPES, &one, None, None, 0.0).map(|b| b.total),
        Some(1)
    );

    // And what a `View` holds is what it was handed, unchanged by drawing it.
    let mut view = View::new(Room::Day);
    view.library = two.clone();
    let mut panel = console(PLAUSIBLE);
    let ctx = drawn_once();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, &mut panel));
    out.textures_delta.clear();
    assert_eq!(
        view.library, two,
        "the frame rewrote the listing it was given"
    );
}
