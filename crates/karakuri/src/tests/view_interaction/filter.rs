use super::*;

/// Clicking a library scope chip updates the active scope, emits `SelectScope`, and resets the cursor to the top (ADR-0213).
#[test]
fn a_press_on_a_scope_chip_names_the_library_the_bay_reads() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    // A console that has been told what libraries there are and handed a
    // listing for the one it opens on, which is what `resumed` does.
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec!["drift_night".to_owned(), "lattice_veil".to_owned()];
    assert!(readout.view.select_scope(Scope::MySets));

    // The capsule, asked of the derivation that draws it rather than
    // remembered — the rule the whole of `input` is written to.
    let chip = |readout: &mut Readout, want: Scope| {
        readout.panel.solve();
        let bay = library_bay(
            readout.panel.layout(),
            &readout.view.scopes,
            &readout.view.library,
            readout.view.opened(),
            readout.view.pointed(),
            readout.view.library_scroll(),
        )
        .expect("the bay lists its rows");
        let (_, at) = bay
            .chips(&ctx, &readout.view.scopes)
            .find(|(scope, _)| *scope == want)
            .expect("the scope is on the row");
        // Two pixels in from its own left edge: the last chip in the row
        // is clipped by the pane, so its centre can be off the row.
        Point::new(at.min.x + 2.0, at.center().y)
    };

    // Move cursor away from row 0 to test cursor reset on scope switch.
    assert!(readout.view.walk(1, 0..2), "the cursor did not move");
    assert_eq!(readout.view.cursor_row(), 1);

    let at = chip(&mut readout, Scope::Presets);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Emitted(Some(Operation::SelectScope { scope: Undecided })),
        "the press did not reach the chip"
    );
    assert!(
        !readout.panel.dragging(),
        "the press took a boundary in hand"
    );
    readout.pointer(&ctx, Pointer::Up);
    assert_eq!(
        readout.view.scope(),
        Some(Scope::Presets),
        "the press was routed and the mark stayed where it was"
    );
    assert_eq!(
        readout.view.cursor_row(),
        0,
        "the scope changed and the cursor is still pointing into the listing it left"
    );

    // Pressing the currently active chip maintains selection rather than clearing it.
    let at = chip(&mut readout, Scope::Presets);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Down).1,
        Acted::Emitted(Some(Operation::SelectScope { scope: Undecided })),
        "a press on the chip that is already marked was not answered"
    );
    readout.pointer(&ctx, Pointer::Up);
    assert_eq!(
        readout.view.scope(),
        Some(Scope::Presets),
        "a press on the marked chip stepped somewhere"
    );
}

/// Clicking a library row's star icon toggles its starred state without triggering row selection (ADR-0213, ADR-0299).
#[test]
fn a_press_on_a_star_names_the_state_the_row_is_not_in() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec!["drift_night".to_owned(), "lattice_veil".to_owned()];

    // The mark's box, asked of the derivation that draws it rather than
    // remembered — the rule the whole of `input` is written to.
    let star = |readout: &mut Readout, index: usize| {
        readout.panel.solve();
        let at = library_bay(
            readout.panel.layout(),
            &readout.view.scopes,
            &readout.view.library,
            readout.view.opened(),
            readout.view.pointed(),
            readout.view.library_scroll(),
        )
        .expect("the bay lists its rows")
        .star(index);
        Point::new(at.center().x, at.center().y)
    };

    // Unstarred row requests starring.
    let at = star(&mut readout, 1);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Emitted(Some(Operation::SetFavourite {
            id: "lattice_veil".to_owned(),
            favourite: true,
        })),
        "the press did not reach the star, or it named the wrong row"
    );
    assert!(
        !readout.panel.dragging(),
        "the press took a boundary in hand"
    );
    readout.pointer(&ctx, Pointer::Up);

    // Starred row requests unstarring when pressed.
    readout.view.starred.insert("lattice_veil".to_owned());
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Down).1,
        Acted::Emitted(Some(Operation::SetFavourite {
            id: "lattice_veil".to_owned(),
            favourite: false,
        })),
        "a starred row was asked to be starred again"
    );
    readout.pointer(&ctx, Pointer::Up);

    // Clicking the row body begins carrying without emitting a favorite operation (ADR-0265).
    readout.panel.solve();
    let row = library_bay(
        readout.panel.layout(),
        &readout.view.scopes,
        &readout.view.library,
        readout.view.opened(),
        readout.view.pointed(),
        readout.view.library_scroll(),
    )
    .expect("the bay lists its rows")
    .row(1);
    let ground = Point::new(row.max.x - 4.0, row.center().y);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Moved(ground)).0,
        Claim::Panel
    );
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert!(
        !matches!(did, Acted::Emitted(Some(Operation::SetFavourite { .. }))),
        "a press on the row's own ground was answered by the star in it: {did:?}"
    );
    readout.pointer(&ctx, Pointer::Up);
}

/// Clicking library filter fields cycles their criteria and emits `ListSets` with combined filter parameters (ADR-0213).
#[test]
fn a_press_on_a_filter_field_asks_the_store_for_a_narrower_listing() {
    use karakuri_console::view::Field;

    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    // A console told what libraries there are, handed the listing for the
    // one it opens on and told what that listing's Sets are made of —
    // which is what `listing` does on this side of the seam.
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec!["drift_night".to_owned(), "lattice_veil".to_owned()];
    readout.view.holds = vec!["drift_shell".to_owned(), "soft_points".to_owned()];
    assert!(readout.view.select_scope(Scope::MySets));
    // Move cursor away from index 0 to verify reset behavior on filter application.
    assert!(readout.view.walk(1, 0..2), "the cursor did not move");

    // The box, asked of the derivation that draws it rather than
    // remembered — the rule the whole of `input` is written to.
    let field = |readout: &mut Readout, which: Field| {
        readout.panel.solve();
        let at = library_bay(
            readout.panel.layout(),
            &readout.view.scopes,
            &readout.view.library,
            readout.view.opened(),
            readout.view.pointed(),
            readout.view.library_scroll(),
        )
        .expect("the bay lists its rows")
        .field(which)
        .expect("the bay draws its filter field");
        Point::new(at.center().x, at.center().y)
    };
    // Kind chip rect derived dynamically from current text layout.
    let kind_chip = |readout: &mut Readout, which: usize| {
        readout.panel.solve();
        let bay = library_bay(
            readout.panel.layout(),
            &readout.view.scopes,
            &readout.view.library,
            readout.view.opened(),
            readout.view.pointed(),
            readout.view.library_scroll(),
        )
        .expect("the bay lists its rows");
        let at = bay
            .kind_chips(&ctx)
            .nth(which)
            .expect("the bay draws six kind chips")
            .1;
        Point::new(at.center().x, at.center().y)
    };

    let at = field(&mut readout, Field::Holds);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Emitted(Some(Operation::ListSets {
            holds: Some("drift_shell".to_owned()),
            layer: None,
        })),
        "the press did not reach the `holds` field"
    );
    assert!(
        !readout.panel.dragging(),
        "the press took a boundary in hand"
    );
    readout.pointer(&ctx, Pointer::Up);
    assert_eq!(
        readout.view.filters().holds,
        Some("drift_shell"),
        "the press was routed and the field it stepped does not read it"
    );
    assert_eq!(
        readout.view.cursor_row(),
        0,
        "the listing narrowed and the cursor is still pointing into the one it left"
    );

    // Kind chip filter preserves existing field filters such as `holds` (ADR-0338).
    let at = kind_chip(&mut readout, 2);
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Down).1,
        Acted::Emitted(Some(Operation::FilterLibrary {
            kinds: karakuri_operation::LibraryKinds {
                l3: true,
                ..karakuri_operation::LibraryKinds::EVERYTHING
            },
        })),
        "the press did not reach the `L3` chip, or it named something other than all six"
    );
    readout.pointer(&ctx, Pointer::Up);
    assert!(readout.view.filters().kinds.l3);
    assert_eq!(readout.view.filters().holds, Some("drift_shell"));
}
