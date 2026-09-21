use super::*;

/// A press on the outputs dot, through the window loop's own routing.
///
/// The other half of the test above: that one is a boundary the panel claims
/// and `egui` never sees, and this is the console's one control, which the
/// panel claims for a different reason — `egui` owns no widget anywhere here,
/// so a press routed to it would reach nothing at all.
///
/// What is asserted is the round trip an operator makes: the picture is on
/// screen, a click on the dot folds it away by name, and a click on the same
/// dot brings it back. The dot is where it is drawn and the press is the
/// panel's at every step.
#[test]
fn a_press_on_the_outputs_dot_folds_the_picture_and_unfolds_it() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    let picture = readout
        .panel
        .layout()
        .find("program-view")
        .expect("program-view");
    let dot = |readout: &mut Readout| {
        readout.panel.solve();
        let row =
            outputs(&ctx, readout.panel.layout(), Open::CLOSED).expect("the row draws its sink");
        (Point::new(row.sink.center().x, row.sink.center().y), row.on)
    };

    let (at, on) = dot(&mut readout);
    assert!(on, "the picture is on screen, so the sink is on");

    // The pointer arrives, and the control is the panel's.
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Operated(Outcome::Folded {
            id: picture,
            folded: true,
            root: false
        }),
        "the press did not reach the sink"
    );
    assert!(
        !readout.panel.dragging(),
        "the press took a boundary in hand"
    );
    readout.pointer(&ctx, Pointer::Up);

    // And the dot is dark, where it still is, and turns the picture back
    // on rather than unfolding whatever else is folded.
    let (at, on) = dot(&mut readout);
    assert!(!on, "the picture is folded and the sink is still lit");
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Down).1,
        Acted::Operated(Outcome::Folded {
            id: picture,
            folded: false,
            root: false
        }),
        "the dark dot did not turn the picture back on"
    );
    assert!(
        dot(&mut readout).1,
        "the picture is back and the dot is dark"
    );
}

/// The filter row narrows what the bay lists, through the summary.
///
/// The other half of the same press: `a_press_on_a_filter_field_…` says the
/// operation reaches `View::narrow`, and this says the listing that comes back
/// afterwards is a narrower one — which is the whole point, and was impossible
/// while this side asked `Store::list_sets` for names.
///
/// A procedure is a row of `all` and of `presets`, with its kind on it — and of
/// neither `my sets` nor `folder` (ADR-0338, decision 1).
///
/// A CPU test: two tiers on a disk, a `View`, and no window.
#[test]
fn the_two_tiers_list_procedures_beside_sets_and_two_scopes_do_not() {
    use karakuri_operation::LibraryKinds;
    use karakuri_store::hash::Hash;
    use karakuri_store::ndjson::Line;
    use karakuri_store::record::Layer as Written;

    let root = scratch_dir("procedure-listing");
    let store = Store::open(&root).expect("a store to list");
    store
        .write_set(
            "night01",
            &[Line::new(Record::Slot {
                at: karakuri_store::record::NodeAddress {
                    layer: Written::L1,
                    index: 0,
                },
                name: Some("drift_shell".to_owned()),
                proc_hash: Hash::of(b"drift_shell"),
            })],
        )
        .expect("a Set to list");
    std::fs::write(
        root.join(Store::PROCEDURES).join("orbit_wide.kir"),
        "proc orbit_wide {\n  kind L3\n}\n",
    )
    .expect("a kept procedure");
    let shipped = root.join("shipped");
    std::fs::create_dir_all(&shipped).expect("mkdir");
    std::fs::write(shipped.join("beat_glow.kset"), "{}\n").expect("a shipped Set");
    std::fs::write(shipped.join("tunnel_eye.kir"), "  kind L3\n").expect("a shipped procedure");
    let presets = karakuri_environment::places::presets(Some(&shipped))
        .expect("the root resolves")
        .expect("a root");

    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();

    // `all`: the Set and the kept procedure, and the procedure carries the
    // kind its `kind` line declares while the Set carries its slots'.
    let said = listing(&mut view, &root, Some(&presets), None, None);
    assert!(
        view.library.contains(&"orbit_wide".to_owned())
            && view.library.contains(&"night01".to_owned()),
        "`all` lists {:?} — {said}",
        view.library
    );
    let at = view
        .library
        .iter()
        .position(|row| row == "orbit_wide")
        .expect("the procedure is a row");
    assert_eq!(
        view.kinds[at],
        RowKind {
            badges: vec![karakuri_operation::Layer::L3],
            procedure: true
        },
        "the procedure row's badge is not its kind"
    );
    let at = view
        .library
        .iter()
        .position(|row| row == "night01")
        .expect("the Set is a row");
    assert_eq!(
        view.kinds[at],
        RowKind {
            badges: vec![karakuri_operation::Layer::L1],
            procedure: false
        },
        "the Set row's badges are not the layers its slots fill"
    );

    // `presets`: the shipped Set and the shipped procedure, in name order.
    assert!(view.select_scope(Scope::Presets));
    listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(
        view.library,
        vec!["beat_glow".to_owned(), "tunnel_eye".to_owned()]
    );
    assert!(view.kinds[1].procedure, "the shipped `.kir` is not a row");

    // `my sets` lists no procedure, because a star is refused on anything
    // `sets/` does not hold; `folder` lists none, because a folder row is a
    // take and nothing takes a bare `.kir` in.
    assert!(view.select_scope(Scope::MySets));
    listing(&mut view, &root, Some(&presets), None, None);
    assert!(
        !view.library.contains(&"orbit_wide".to_owned()),
        "`my sets` lists a procedure: {:?}",
        view.library
    );
    assert!(view.select_scope(Scope::Folder));
    listing(&mut view, &root, Some(&presets), Some(&shipped), None);
    assert!(
        !view.library.contains(&"tunnel_eye".to_owned()),
        "`folder` lists a procedure: {:?}",
        view.library
    );

    // **The kind chips narrow by OR, and none on is everything.**
    assert!(view.select_scope(Scope::AllSets));
    let cameras = LibraryKinds {
        l3: true,
        ..LibraryKinds::EVERYTHING
    };
    assert!(view.narrow(None, cameras));
    listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(
        view.library,
        vec!["orbit_wide".to_owned()],
        "`L3` on lists {:?}",
        view.library
    );
    let sets_only = LibraryKinds {
        sets: true,
        ..LibraryKinds::EVERYTHING
    };
    assert!(view.narrow(None, sets_only));
    listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(
        view.library,
        vec!["night01".to_owned()],
        "`SET` on lists {:?}",
        view.library
    );
    assert!(view.narrow(
        None,
        LibraryKinds {
            l3: true,
            sets: true,
            ..LibraryKinds::EVERYTHING
        }
    ));
    listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(
        view.library.len(),
        2,
        "an OR of two lists {:?}",
        view.library
    );
    assert!(view.narrow(None, LibraryKinds::EVERYTHING));
    listing(&mut view, &root, Some(&presets), None, None);
    assert_eq!(
        view.library.len(),
        2,
        "none on is not everything: {:?}",
        view.library
    );

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// What it narrows is the store's own listing, which is `all` and is what *List
/// what the store holds* lists. `my sets` is that listing starred (ADR-0299),
/// so the same retain applies to it and the row is not a control over one chip.
///
/// It is the same retain the MCP tool applies, over the same
/// `setfile::summarise`, which is what keeps one operation from being answered
/// two ways by two surfaces.
///
/// A CPU test: a store, a `View`, and no window.
#[test]
fn the_filter_row_narrows_the_stores_listing_through_the_summary() {
    use karakuri_store::hash::Hash;
    use karakuri_store::ndjson::Line;
    use karakuri_store::record::Layer as Written;

    let root = scratch_dir("filter-listing");
    let store = Store::open(&root).expect("a store to list");
    let slot = |layer: Written, name: &str| {
        Line::new(Record::Slot {
            at: karakuri_store::record::NodeAddress { layer, index: 0 },
            name: Some(name.to_owned()),
            proc_hash: Hash::of(name.as_bytes()),
        })
    };
    store
        .write_set("night01", &[slot(Written::L1, "drift_shell")])
        .expect("a Set to list");
    store
        .write_set("veil02", &[slot(Written::L4, "soft_points")])
        .expect("a second Set to list");

    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    assert_eq!(view.scope(), Some(Scope::AllSets));

    // Unnarrowed: both Sets, and the candidates are what their nodes are
    // called — sorted, deduplicated, and read off the *unfiltered* listing.
    let said = listing(&mut view, &root, None, None, None);
    assert_eq!(view.library.len(), 2, "the bay lists {:?}", view.library);
    assert_eq!(
        view.holds,
        vec!["drift_shell".to_owned(), "soft_points".to_owned()],
        "the `holds` field can be stepped to {:?}",
        view.holds
    );
    assert!(!said.contains("holding"), "{said}");

    // Narrowed by what a node is called.
    assert!(view.narrow(
        Some("drift_shell"),
        karakuri_operation::LibraryKinds::EVERYTHING
    ));
    let said = listing(&mut view, &root, None, None, None);
    assert_eq!(view.library, vec!["night01".to_owned()], "{said}");
    assert!(
        said.contains("1 of 2") && said.contains("drift_shell"),
        "{said}"
    );

    // **A filter that matched nothing is a different nothing from an empty
    // store**, and the line says which: the store is not empty, and what to
    // do about it is press a field rather than save a Set.
    assert!(view.narrow(
        Some("drift_shell"),
        karakuri_operation::LibraryKinds {
            l3: true,
            ..karakuri_operation::LibraryKinds::EVERYTHING
        },
    ));
    let said = listing(&mut view, &root, None, None, None);
    assert!(view.library.is_empty(), "the bay lists {:?}", view.library);
    assert!(
        said.contains("none of the 2 Sets here")
            && !said.contains(why_nothing(Scope::AllSets, false, false)),
        "{said}"
    );

    // **And the same retain applies to `my sets`**, which is this listing
    // starred: star one Set, mark the subset, and the filter that named
    // the other one leaves it with nothing — the narrowing is over what
    // the store holds and not over which chip is marked.
    assert!(view.narrow(None, karakuri_operation::LibraryKinds::EVERYTHING));
    favourite(
        &root,
        Asked::Operator,
        &Operation::SetFavourite {
            id: "night01".to_owned(),
            favourite: true,
        },
    )
    .expect("`favourite` answered nothing for a `SetFavourite`");
    assert!(view.select_scope(Scope::MySets));
    let said = listing(&mut view, &root, None, None, None);
    assert_eq!(view.library, vec!["night01".to_owned()], "{said}");
    assert!(view.narrow(
        Some("soft_points"),
        karakuri_operation::LibraryKinds::EVERYTHING
    ));
    let said = listing(&mut view, &root, None, None, None);
    assert!(
        view.library.is_empty(),
        "`my sets` lists {:?} under a filter that names the Set that is not starred",
        view.library
    );
    assert!(said.contains("none of the 2 Sets here"), "{said}");

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A press on the Program bay's `solo` pill, through the window loop's own
/// routing.
///
/// `karakuri-console`'s `tests/solo_pill.rs` and `tests/vocabulary.rs` assert
/// everything up to the operation with no window anywhere; this is the half
/// ADR-0213 makes the badge mean — *"the row is claimed the day a person who
/// launched the instrument can perform that operation from the panel in front
/// of them"* — and a control demonstrated in that crate and never wired here
/// would pass there and be a lie the page tells.
///
/// Both directions, because the pill is both. A solo takes every other control
/// off the screen, so the pill is the only thing left to press and the undo has
/// to come from it. What is asserted is the round trip an operator makes: the
/// picture is one region among many, a click on the pill leaves it holding the
/// window, and a click on the same pill — found again where it is now drawn,
/// because the solo moved every rectangle on the console — puts everything
/// back.
#[test]
fn a_press_on_the_solo_pill_solos_the_picture_and_undoes_it() {
    let ctx = drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    let picture = readout
        .panel
        .layout()
        .find("program-view")
        .expect("program-view");
    let library = readout.panel.layout().find("library").expect("library");
    // The capsule, asked of the derivation that draws it rather than
    // remembered — which is the rule the whole of `input` is written to,
    // and here it is load-bearing twice over.
    let pill = |readout: &mut Readout| {
        readout.panel.solve();
        let head = program_head(&ctx, readout.panel.layout(), Open::CLOSED)
            .expect("the bay draws its pill");
        (
            Point::new(head.solo.center().x, head.solo.center().y),
            head.soloed,
        )
    };

    let (at, soloed) = pill(&mut readout);
    assert!(!soloed, "something is soloed before anything was pressed");
    assert!(
        readout.panel.layout().visible(library),
        "the library is off the screen already, so soloing would prove nothing"
    );

    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    let (claim, did) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(
        did,
        Acted::Operated(Outcome::Soloed(picture)),
        "the press did not reach the pill"
    );
    assert!(
        !readout.panel.dragging(),
        "the press took a boundary in hand"
    );
    readout.pointer(&ctx, Pointer::Up);
    readout.panel.solve();
    assert!(
        !readout.panel.layout().visible(library),
        "the picture is soloed and the library is still on the screen"
    );

    // And the same pill, where it is now, undoes it.
    let (at, soloed) = pill(&mut readout);
    assert!(soloed, "the picture is soloed and the pill does not say so");
    assert_eq!(readout.pointer(&ctx, Pointer::Moved(at)).0, Claim::Panel);
    assert_eq!(
        readout.pointer(&ctx, Pointer::Down).1,
        Acted::Operated(Outcome::Unsoloed { was: true }),
        "the pill did not undo the solo it made"
    );
    readout.pointer(&ctx, Pointer::Up);
    readout.panel.solve();
    assert!(
        readout.panel.layout().visible(library),
        "undoing the solo left the library folded"
    );
}
