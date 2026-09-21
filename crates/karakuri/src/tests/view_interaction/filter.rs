use super::*;

/// A press on a scope chip, through the window loop's own routing — which is
/// what ADR-0213 makes the *panel* badge mean.
///
/// `karakuri-console`'s `tests/library.rs` asserts everything up to the
/// operation with no window anywhere: that the chips answer a press, that a
/// press names the chip it landed on, and that nothing else in the bay takes
/// one. This is the half that badge is actually about — *"the row is claimed
/// the day a person who launched the instrument can perform that operation from
/// the panel in front of them"* — and a control demonstrated in that crate and
/// never wired here would pass there and be a lie this page tells on its own
/// authority.
///
/// What is asserted is the whole press and not the routing alone: the mark
/// moves to the chip that was pressed, the operation that leaves is
/// `SelectScope` with the payload it is specified to carry, and the library
/// cursor goes back to the top — because the listing under a new scope is a
/// listing this cursor has never seen, and a cursor left where it was would sit
/// on a Set nobody chose under a pill saying a press will load it.
///
/// And the chip that is already marked is pressed too, because that is the case
/// a step cannot reach: a step would go somewhere else, and a pointer names —
/// so the press is answered rather than refused, and the mark stays where it
/// is.
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

    // **The cursor is somewhere other than the top**, so that the move
    // back to it is a move and not the state it was already in.
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

    // **The marked chip, pressed** — answered rather than refused, and the
    // mark does not step off it the way the key would.
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

/// A press on the star at the left of a library row, through the window loop's
/// own routing — the half of the badge ADR-0213 makes a badge mean, beside
/// [`a_press_on_a_filter_field_asks_the_store_for_a_narrower_listing`].
///
/// `karakuri-console`'s `tests/library.rs` says where the mark is and that it
/// answers a press; this says an operator reaches it — a control demonstrated
/// in that crate and never wired here would pass there and be a lie the page
/// tells on its own authority.
///
/// What is asserted is that the press names a state and not a step (ADR-0299):
/// the same mark pressed twice asks for two different things, because the
/// control reads the row's present mark and asks for the other one. That is the
/// failure a toggle hides completely — a press that always emitted `true` would
/// pass every assertion about the first press and never take a star off.
///
/// And that the star has not swallowed the row it sits in: a press on the row's
/// own ground still takes the Set in hand and names no operation, which is rule
/// 4's *a control claims what it acts on and no more* asked of the two boxes
/// that overlap.
///
/// A CPU test: a `Readout` takes no device.
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

    // **Nothing starred, so the press asks for the star to go on.**
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

    // **And with the row starred it asks for the star to come off**, which
    // is the same control reading the state it is drawn from. The marks
    // are the host's answer, so this is what `listing` would have written
    // after the write.
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

    // **The row's own ground is still the row's.** A press at the far end
    // of the same row takes the Set in hand and names no operation, which
    // is what a carry is (ADR-0265).
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

/// A press on one of the Library bay's two filter fields, through the window
/// loop's own routing — the half of the badge ADR-0213 makes a badge mean, one
/// row under [`a_press_on_a_scope_chip_names_the_library_the_bay_reads`].
///
/// `karakuri-console`'s `tests/library.rs` asserts everything up to the
/// operation with no window anywhere: where the two fields are, that each steps
/// its own cycle, that the operation names where it arrived and carries the
/// other field untouched, and that nothing between them takes a press. This is
/// the half that says an operator reaches it — a control demonstrated in that
/// crate and never wired here would pass there and be a lie the page tells on
/// its own authority, which is exactly what `press_handler` was written after.
///
/// What is asserted is the whole press. The operation that leaves is `ListSets`
/// carrying the step; `View::narrow` has been called, so the field the panel
/// draws next frame reads the new value; and the library cursor is back at the
/// top, because the listing under a narrower filter is one this cursor has
/// never seen.
///
/// Both fields, because the operation carries both halves: the press on `layer`
/// has to come back with the `holds` the press before it set, and a route that
/// rebuilt the operation from one field would lose it.
///
/// A CPU test: a `Readout` takes no device.
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
    // **The cursor is somewhere other than the top**, so that the move back
    // to it is a move and not the state it was already in.
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
    // **And one kind chip's, asked of the walk that paints them**, which
    // is the same rule one band down: a chip is as wide as the word in it,
    // so where it is is `egui`'s answer and never a remembered number.
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

    // **And the kind chips under it, which have to carry the field
    // through**: a press on a chip names all six and says nothing about
    // `holds`, so what the console is narrowed to afterwards is the pair as
    // it now stands (ADR-0338).
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
