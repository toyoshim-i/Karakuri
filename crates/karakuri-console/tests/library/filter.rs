use super::library_common::*;

// ---------------------------------------------------------------------------
// A row's badges, and what a procedure row is
// ---------------------------------------------------------------------------

/// The mock's own listing with a procedure in it, which is the bay ADR-0338
/// draws: four Sets carrying the layers their files fill, and `orbit_wide`
/// between two of them carrying its one `kind`.
fn with_a_procedure() -> (Vec<String>, Vec<RowKind>) {
    use karakuri_operation::Layer;
    let set = |badges: Vec<Layer>| RowKind {
        badges,
        procedure: false,
    };
    (
        [
            "drift_night",
            "lattice_veil",
            "orbit_wide",
            "glass_shell",
            "night01",
        ]
        .iter()
        .map(|s| (*s).to_owned())
        .collect(),
        vec![
            set(vec![Layer::L1, Layer::L2, Layer::L4]),
            set(vec![Layer::L1, Layer::L4]),
            RowKind {
                badges: vec![Layer::L3],
                procedure: true,
            },
            set(vec![Layer::L1, Layer::L4]),
            set(vec![Layer::L1, Layer::L2, Layer::L4]),
        ],
    )
}

/// Verifies that a row displays kind badges matching the layers it implements (ADR-0338).
#[test]
fn a_row_carries_the_words_of_the_layers_it_implements() {
    let (names, kinds) = with_a_procedure();
    let rows = Rows {
        names: &names,
        kinds: &kinds,
    };
    assert_eq!(rows.badges(0), vec!["L1", "L2", "L4"]);
    assert_eq!(rows.badges(2), vec!["L3"]);
    assert!(
        rows.procedure(2),
        "the procedure row does not say it is one"
    );
    assert!(!rows.procedure(0), "a Set row says it is a procedure");

    // **The star, the reading and the send take a Set and a procedure row hands
    // them nothing**, which is one question rather than three.
    assert_eq!(rows.set(0), Some("drift_night"));
    assert_eq!(rows.set(2), None, "a procedure row was offered as a Set id");
    assert_eq!(rows.set(9), None, "a row past the end was offered as a Set");

    let bare = listed(&names);
    assert!(
        bare.badges(0).is_empty(),
        "a row with no kinds drew a badge"
    );
    assert!(
        !bare.procedure(2),
        "a row with no kinds beside it is not a Set"
    );
    assert_eq!(bare.set(2), Some("orbit_wide"));
}

/// The badges are laid out from the right of the row, one gap apart, which is
/// where the mock puts them: after the name and before the row's own padding,
/// so a longer name never moves them.
#[test]
fn the_badges_end_where_the_rows_padding_starts() {
    let panel = console(PLAUSIBLE);
    let ctx = drawn_once();
    let bay = bay(&panel);
    let (names, kinds) = with_a_procedure();
    let rows = Rows {
        names: &names,
        kinds: &kinds,
    };
    let row = bay.row(0);
    let drawn: Vec<(&str, egui::Rect)> = bay.badges(&ctx, 0, &rows.badges(0)).collect();
    assert_eq!(
        drawn.iter().map(|(word, _)| *word).collect::<Vec<_>>(),
        vec!["L1", "L2", "L4"]
    );
    assert!(
        near(
            drawn[2].1.max.x,
            row.max.x - karakuri_console::room::size::LIB_ROW_PAD_X
        ),
        "the last badge ends at {} in a row ending at {}",
        drawn[2].1.max.x,
        row.max.x
    );
    for pair in drawn.windows(2) {
        assert!(
            near(
                pair[1].1.min.x - pair[0].1.max.x,
                karakuri_console::room::size::BADGE_GAP
            ),
            "{:?} and {:?} are not one gap apart",
            pair[0].0,
            pair[1].0
        );
    }
    assert!(
        near(drawn[0].1.height(), karakuri_console::room::size::BADGE_H)
            && drawn[0].1.center().y == row.center().y,
        "a badge is {:?} in a row centred at {}",
        drawn[0].1,
        row.center().y
    );
    // A row with no badges draws none, and asks `egui` for nothing to say so.
    assert_eq!(bay.badges(&ctx, 1, &[]).count(), 0);
}

/// A procedure row has no star, and every load off it names `LoadProcedure` —
/// the button, the row menu's items and the carry, which are the three routes
/// ADR-0338 names.
#[test]
fn a_procedure_row_has_no_star_and_loads_one_layer() {
    let panel = console(PLAUSIBLE);
    let ctx = drawn_once();
    let bay = bay(&panel);
    let (names, kinds) = with_a_procedure();
    let rows = Rows {
        names: &names,
        kinds: &kinds,
    };
    let none = std::collections::BTreeSet::new();

    // The star on the procedure row answers nothing, and the star on the Set
    // row above it still does.
    let star = bay.star(2);
    assert_eq!(
        bay.starred(rows, &none, Point::new(star.center().x, star.center().y)),
        None,
        "a press on a procedure row's star asked for a favourite"
    );
    let star = bay.star(0);
    assert_eq!(
        bay.starred(rows, &none, Point::new(star.center().x, star.center().y)),
        Some(Operation::SetFavourite {
            id: "drift_night".to_owned(),
            favourite: true
        })
    );

    // The `load` button, with the cursor on the procedure row.
    let load = bay.load(&ctx, AIMED);
    assert_eq!(
        bay.aim(
            &ctx,
            viewport(&panel),
            AIMED,
            rows,
            2,
            Point::new(load.button.center().x, load.button.center().y)
        ),
        Some(Aim::Load(Operation::LoadProcedure {
            deck: 0,
            procedure: "orbit_wide".to_owned()
        })),
        "the `load` button named a Set load on a procedure row"
    );

    // A row menu's `Load to Slot B`, and the menu on such a row carries no
    // send at all — a bare `.kir` is not a Set and nothing takes one in.
    let at = karakuri_console::view::Menued {
        row: Some(2),
        decks: 4,
        sends: false,
    };
    let menu = bay
        .menu(&ctx, viewport(&panel), at)
        .expect("the menu is down");
    assert_eq!(menu.save, None, "a procedure row's menu carries a send");
    assert_eq!(menu.rule, None, "a procedure row's menu draws a separator");
    let item = menu.load(1);
    assert_eq!(
        bay.menu_ask(
            &ctx,
            viewport(&panel),
            at,
            rows,
            Point::new(item.center().x, item.center().y)
        ),
        Some(Picked::Load(Operation::LoadProcedure {
            deck: 1,
            procedure: "orbit_wide".to_owned()
        }))
    );

    // And the carry, which names the operand and says which load a drop will
    // ask for.
    let row = bay.row(2);
    let taken = bay
        .take(rows, Point::new(row.center().x, row.center().y))
        .expect("the row was not taken in hand");
    assert_eq!(taken.set, "orbit_wide");
    assert!(taken.procedure, "the carry does not say it is a procedure");
}

// ---------------------------------------------------------------------------
// The filter field and the six kind chips
// ---------------------------------------------------------------------------

/// Asserts the six filter chips match declared procedure material kinds and sets (ADR-0340).
#[test]
fn the_kind_chips_are_the_five_kinds_and_the_sets() {
    use karakuri_operation::Layer;
    for layer in [Layer::L1, Layer::L2, Layer::L3, Layer::L4, Layer::Field] {
        let spelled = match layer {
            Layer::L1 => "L1",
            Layer::L2 => "L2",
            Layer::L3 => "L3",
            Layer::L4 => "L4",
            Layer::Field => "FIELD",
            Layer::L5 => unreachable!("L5 is not on this row"),
        };
        let found = LAYERS
            .iter()
            .find(|(kind, _)| *kind == layer)
            .unwrap_or_else(|| panic!("{layer:?} has no word on the kind row"));
        assert_eq!(found.1, spelled, "{layer:?} is spelled two ways");
        assert_eq!(
            KindChip::Layer(layer).word(),
            spelled,
            "the chip and the badge spell {layer:?} two ways"
        );
    }
    assert_eq!(
        LAYERS.len(),
        5,
        "the row draws a chip per layer and no more"
    );
    assert_eq!(
        KindChip::ALL.len(),
        6,
        "the row is five kinds and the Sets — {} chips",
        KindChip::ALL.len()
    );
    assert_eq!(KindChip::Sets.word(), "SET");
}

/// Kind chip presses emit the entire 6-chip filter state, toggling the clicked chip and updating all others (ADR-0338).
#[test]
fn a_press_on_a_kind_chip_names_all_six_and_turns_that_one_over() {
    use karakuri_operation::{LibraryKinds, Operation};
    let panel = console(PLAUSIBLE);
    let ctx = drawn_once();
    let bay = bay(&panel);
    let boxes: Vec<(KindChip, egui::Rect)> = bay.kind_chips(&ctx).collect();
    let mut at = LibraryKinds::EVERYTHING;
    for (chip, box_) in &boxes {
        let asked = bay
            .kind(
                &ctx,
                Filters {
                    holds: None,
                    kinds: at,
                },
                Point::new(box_.center().x, box_.center().y),
            )
            .expect("the chip did not answer a press on it");
        let Operation::FilterLibrary { kinds } = asked else {
            panic!("a kind chip named something other than `FilterLibrary`");
        };
        assert!(chip.on(kinds), "a press on {chip:?} did not turn it on");
        for (other, _) in &boxes {
            if other != chip {
                assert_eq!(
                    other.on(kinds),
                    other.on(at),
                    "a press on {chip:?} moved {other:?}"
                );
            }
        }
        at = kinds;
    }
    assert!(
        at.narrowing(),
        "six presses left the row showing everything"
    );
    // And back off again, chip by chip, to the state a run opens in.
    for (chip, box_) in &boxes {
        let asked = bay
            .kind(
                &ctx,
                Filters {
                    holds: None,
                    kinds: at,
                },
                Point::new(box_.center().x, box_.center().y),
            )
            .expect("the chip did not answer a press on it");
        let Operation::FilterLibrary { kinds } = asked else {
            panic!("a kind chip named something other than `FilterLibrary`");
        };
        assert!(
            !chip.on(kinds),
            "a press on a lit {chip:?} did not turn it off"
        );
        at = kinds;
    }
    assert_eq!(
        at,
        LibraryKinds::EVERYTHING,
        "the row cannot be pressed back to showing everything"
    );
}

/// The row's own ground answers nothing, which is the scope row's rule one band
/// down: the gaps between the chips and the padding either side are bare card,
/// and a bay with no kind row at all answers no press anywhere in it.
#[test]
fn the_kind_rows_own_ground_is_nobodys() {
    let panel = console(PLAUSIBLE);
    let ctx = drawn_once();
    let bay = bay(&panel);
    let row = bay.kinds.expect("the bay draws its kind row");
    let boxes: Vec<(KindChip, egui::Rect)> = bay.kind_chips(&ctx).collect();
    let gap = Point::new((boxes[0].1.max.x + boxes[1].1.min.x) * 0.5, row.center().y);
    assert_eq!(
        bay.kind(&ctx, Filters::NONE, gap),
        None,
        "the gap between two chips answered a press"
    );
    assert_eq!(
        bay.kind(
            &ctx,
            Filters::NONE,
            Point::new(row.min.x + 1.0, row.center().y)
        ),
        None,
        "the row's left padding answered a press"
    );

    let bare = library(panel.layout(), &[], &mock(), None, None, 0.0).expect("the bay lists rows");
    assert_eq!(bare.kinds, None, "a bay with no scope row drew a kind row");
    assert_eq!(
        bare.kind(&ctx, Filters::NONE, gap),
        None,
        "a bay with no kind row answered a press on one"
    );
}

/// Asserts clicking a filter field advances its cycle through unset and emits full `ListSets` filter state.
#[test]
fn a_press_on_a_filter_field_steps_it_and_names_where_it_arrived() {
    let panel = console(PLAUSIBLE);
    let bay = bay(&panel);
    let holds: Vec<String> = ["drift_shell", "soft_points"]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    let at = |field: Field| {
        let box_ = bay.field(field).expect("the field is drawn");
        Point::new(box_.center().x, box_.center().y)
    };

    // The `holds` field, over the two candidates and back to unset — and with
    // the kind row narrowed, which the press must leave exactly where it is:
    // `ListSets` carries no kinds, and the two controls on this row are two
    // questions (ADR-0338).
    let narrowed = karakuri_operation::LibraryKinds {
        l3: true,
        ..karakuri_operation::LibraryKinds::EVERYTHING
    };
    let mut set = None;
    for want in [Some("drift_shell"), Some("soft_points"), None] {
        let asked = bay
            .filter(
                &holds,
                Filters {
                    holds: set,
                    kinds: narrowed,
                },
                at(Field::Holds),
            )
            .expect("the `holds` field did not answer a press on it");
        assert_eq!(
            asked,
            karakuri_operation::Operation::ListSets {
                holds: want.map(str::to_owned),
                // **The layer goes out unset and the field it came from is
                // gone**: `Operation::ListSets` keeps the field for
                // `list_sets` and `--list-sets`, and this console stopped
                // asking it (ADR-0338).
                layer: None
            },
            "the `holds` field stepped from {set:?} to something else"
        );
        set = want;
    }
}

/// A `holds` field with nothing to step to asks the listing again, which is the
/// state every console in this crate that has not been handed candidates is in
/// — and it is a question rather than a no-op, the same one a press on the chip
/// that is already marked asks.
#[test]
fn a_holds_field_with_nothing_to_step_to_asks_the_listing_again() {
    let panel = console(PLAUSIBLE);
    let bay = bay(&panel);
    let box_ = bay.field(Field::Holds).expect("the field is drawn");
    assert_eq!(
        bay.filter(
            &[],
            Filters::NONE,
            Point::new(box_.center().x, box_.center().y)
        ),
        Some(karakuri_operation::Operation::ListSets {
            holds: None,
            layer: None
        }),
        "a press on `holds…` with no candidates behind it was not answered"
    );
}

/// Asserts filter fields claim input while padding and inter-field gaps fall through.
#[test]
fn the_filter_fields_answer_a_press_and_the_row_around_them_does_not() {
    let mut panel = console(PLAUSIBLE);
    let ctx = drawn_once();
    let bay = bay(&panel);
    let row = bay.filters.expect("the bay draws its filter row");
    let mut view = View::new(karakuri_console::room::Room::Day);
    view.scopes = SCOPES.to_vec();
    view.library = mock();
    view.holds = vec!["drift_shell".to_owned()];

    for field in Field::ALL {
        let box_ = bay.field(field).expect("the field is drawn");
        let on = Point::new(box_.center().x, box_.center().y);
        assert!(
            bay.filter(&view.holds, view.filters(), on).is_some(),
            "{field:?} did not answer a press in the middle of it"
        );
        assert_eq!(
            claim(&mut panel, &ctx, &view, on),
            Claim::Panel,
            "{field:?} is drawn and `egui` was given the press on it"
        );
    }

    // The gap between the two, which is 5 wide and bare card.
    let holds = bay.field(Field::Holds).expect("the `holds` field");
    let gap = Point::new(holds.max.x + size::LIB_FILTERS_GAP * 0.5, holds.center().y);
    assert_eq!(
        bay.filter(&view.holds, view.filters(), gap),
        None,
        "the gap between the two fields answered a press"
    );
    assert_eq!(
        claim(&mut panel, &ctx, &view, gap),
        Claim::Egui,
        "the gap between the two fields is claimed by the panel"
    );

    // And the row's own left padding, which is 9 of it.
    let pad = Point::new(row.min.x + 1.0, row.center().y);
    assert_eq!(
        bay.filter(&view.holds, view.filters(), pad),
        None,
        "the row's padding answered a press"
    );
}

/// Both fields clear every boundary's grab, which the chip at the end of the
/// scope row above them does not — `.lib-filters` is padded 9 in from each side
/// edge against a `GRAB` of 6, and the row above and the list below are both
/// the bay's own.
#[test]
fn the_filter_fields_clear_every_boundary() {
    let panel = console(PLAUSIBLE);
    let bay = bay(&panel);
    let region = to_egui(rect_of(panel.layout(), "library"));

    for field in Field::ALL {
        let box_ = bay.field(field).expect("the field is drawn");
        for probe in [
            Point::new(box_.min.x + 1.0, box_.center().y),
            Point::new(box_.max.x - 1.0, box_.center().y),
            Point::new(box_.center().x, box_.min.y + 1.0),
            Point::new(box_.center().x, box_.max.y - 1.0),
        ] {
            assert!(
                !matches!(
                    panel.layout().hit(probe, GRAB),
                    karakuri_layout::Hit::Divider { .. }
                ),
                "a boundary grabs {probe:?}, which is inside {field:?}"
            );
        }
        assert!(
            box_.min.x - region.min.x >= size::LIB_FILTERS_PAD_X - f32::EPSILON
                && region.max.x - box_.max.x >= size::LIB_FILTERS_PAD_X - f32::EPSILON,
            "{field:?} is {:?} in a bay spanning {} to {}",
            box_,
            region.min.x,
            region.max.x
        );
    }

    // **And the pixel past the row's right-hand padding is still a
    // boundary's**, which is what says the clearances above are clearances
    // rather than the grab having gone missing.
    let row = bay.filters.expect("the bay draws its filter row");
    assert!(
        matches!(
            panel
                .layout()
                .hit(Point::new(row.max.x - 1.0, row.center().y), GRAB),
            karakuri_layout::Hit::Divider { .. }
        ),
        "the pixel at the far end of the filter row is not the pane divider's, so nothing here \
         measured a grab"
    );
}

/// Asserts filter state roundtrips through view and out-of-bounds candidate indices fallback to unset.
#[test]
fn the_fields_read_what_is_set_and_a_stale_candidate_reads_as_unset() {
    use karakuri_operation::LibraryKinds;
    let cameras = LibraryKinds {
        l3: true,
        ..LibraryKinds::EVERYTHING
    };
    let mut view = View::new(karakuri_console::room::Room::Day);
    assert_eq!(view.filters().holds_word(), HOLDS_UNSET);
    assert_eq!(view.filters().kinds, LibraryKinds::EVERYTHING);

    view.holds = vec!["drift_shell".to_owned(), "soft_points".to_owned()];
    assert!(view.narrow(Some("soft_points"), cameras));
    assert_eq!(view.filters().holds_word(), "soft_points");
    assert_eq!(view.filters().kinds, cameras);
    assert!(
        !view.narrow(Some("soft_points"), cameras),
        "narrowing to what it was already narrowed to moved something"
    );

    // **A `holds` this console cannot draw is refused, and refused whole**:
    // the kinds beside it are not written either.
    assert!(
        !view.narrow(Some("no_such_node"), LibraryKinds::EVERYTHING),
        "a filter the field cannot draw was accepted"
    );
    assert_eq!(view.filters().holds_word(), "soft_points");
    assert_eq!(view.filters().kinds, cameras);

    // **The candidates go, and the field reads unset** — not `drift_shell`,
    // which is what clamping would have answered.
    view.holds = vec!["drift_shell".to_owned()];
    assert_eq!(view.filters().holds_word(), HOLDS_UNSET);
    assert_eq!(view.filters().holds, None);
    assert_eq!(
        view.filters().kinds,
        cameras,
        "the kinds are the console's own and did not survive the candidates going"
    );
}

/// A console that was told about no library draws no filter row, which is the
/// scope row's own condition rather than a second one: a filter is a question
/// about a listing, and there is no listing to ask it of.
#[test]
fn a_console_with_no_scopes_draws_no_filter_row() {
    let panel = console(PLAUSIBLE);
    let bay =
        library(panel.layout(), &[], &mock(), None, None, 0.0).expect("the bay lists its rows");
    assert_eq!(bay.scopes, None);
    assert_eq!(
        bay.filters, None,
        "a bay with no scope row drew a filter row"
    );
    assert_eq!(bay.field(Field::Holds), None);
    assert_eq!(
        bay.filter(
            &[],
            Filters::NONE,
            Point::new(bay.list.min.x, bay.list.min.y)
        ),
        None,
        "a bay with no filter row answered a press on one"
    );
}
