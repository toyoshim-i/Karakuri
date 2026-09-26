//! Transport row `learn` and `map` pills: surface learning state and map display (ADR-0236, ADR-0336).

mod common;

use common::{at, PLAUSIBLE};
use karakuri_console::input::{claim, Claim};
use karakuri_console::room::Room;
use karakuri_console::view::{arrangement, learn_pill, map_pill, MapPill, Transport, View};

fn mock() -> Transport {
    common::mock_transport()
}

use common::console;

/// A view with an engine behind it, and a surface in front of it or not.
fn view(map: Option<MapPill>, armed: bool) -> View {
    let mut view = View::new(Room::Day);
    view.transport = Some(mock());
    view.map = map;
    view.learn = armed;
    view
}

/// When no surface is configured (`View::map` is `None`), neither pill is drawn and layout is unchanged.
#[test]
fn no_surface_means_neither_pill_and_an_arrangement_that_has_not_moved() {
    let (panel, ctx) = console(PLAUSIBLE);
    let untold = view(None, false);
    assert!(
        learn_pill(
            &ctx,
            panel.layout(),
            untold.transport,
            None,
            None,
            untold.map.as_ref(),
            untold.learn
        )
        .is_none(),
        "a console with no surface drew a learn pill"
    );
    assert!(
        map_pill(
            &ctx,
            panel.layout(),
            untold.transport,
            None,
            None,
            untold.map.as_ref()
        )
        .is_none(),
        "a console with no surface drew a map pill"
    );

    let before = arrangement(
        &ctx,
        panel.layout(),
        untold.transport,
        None,
        None,
        None,
        &untold.arrangement,
    )
    .expect("the row draws the arrangement pill")
    .pill;
    let told = view(Some(MapPill::NONE), false);
    let after = arrangement(
        &ctx,
        panel.layout(),
        told.transport,
        None,
        None,
        told.map.as_ref(),
        &told.arrangement,
    )
    .expect("the row still draws it")
    .pill;
    assert!(
        after.min.x > before.min.x,
        "the arrangement pill did not move for the two pills before it: {before:?} then {after:?}"
    );
}

/// The mock's order, and each laid out from the one before it — `learn`, then
/// `map`, then `arr`, left to right with the row's own gap between them.
#[test]
fn the_three_pills_are_in_the_mocks_order() {
    let (panel, ctx) = console(PLAUSIBLE);
    let told = view(
        Some(MapPill {
            name: Some("default".to_owned()),
        }),
        false,
    );
    let learn = learn_pill(
        &ctx,
        panel.layout(),
        told.transport,
        None,
        None,
        told.map.as_ref(),
        told.learn,
    )
    .expect("a learn pill");
    let map = map_pill(
        &ctx,
        panel.layout(),
        told.transport,
        None,
        None,
        told.map.as_ref(),
    )
    .expect("a map pill");
    let arr = arrangement(
        &ctx,
        panel.layout(),
        told.transport,
        None,
        None,
        told.map.as_ref(),
        &told.arrangement,
    )
    .expect("an arrangement pill")
    .pill;
    assert!(
        learn.pill.max.x <= map.pill.min.x,
        "learn is not before map: {:?} then {:?}",
        learn.pill,
        map.pill
    );
    assert!(
        map.pill.max.x <= arr.min.x,
        "map is not before the arrangement pill: {:?} then {arr:?}",
        map.pill
    );
    // All three sit on the row's own middle, which is what makes them one
    // group rather than three controls that happen to be near each other.
    assert!((learn.pill.center().y - map.pill.center().y).abs() < 0.01);
    assert!((map.pill.center().y - arr.center().y).abs() < 0.01);
}

/// The `learn` pill toggles learning state on press, while `map` functions as a readout (ADR-0236, ADR-0336).
#[test]
fn learn_names_the_other_state_and_both_pills_take_the_pointer() {
    let (panel, ctx) = console(PLAUSIBLE);
    let told = view(Some(MapPill::NONE), false);
    let off = learn_pill(
        &ctx,
        panel.layout(),
        told.transport,
        None,
        None,
        told.map.as_ref(),
        false,
    )
    .expect("a learn pill");
    assert!(!off.armed);
    assert!(off.next(), "a press on an unarmed pill has to arm it");

    let armed = view(Some(MapPill::NONE), true);
    let on = learn_pill(
        &ctx,
        panel.layout(),
        armed.transport,
        None,
        None,
        armed.map.as_ref(),
        true,
    )
    .expect("a learn pill");
    assert!(on.armed);
    assert!(!on.next(), "a press on an armed pill has to disarm it");
    // Arming does not move the control, which is what lets a second press
    // land where the first one did.
    assert_eq!(off.pill, on.pill);
    assert!(off.hit(at(off.pill.center())));

    // **Both are reached by the pointer**, and the `map` pill is one of them
    // even though a press on it asks for nothing: a press that fell through to
    // whatever is behind a control the panel drew is the defect `claim`'s rule
    // 4 exists to stop.
    let map = map_pill(
        &ctx,
        panel.layout(),
        told.transport,
        None,
        None,
        told.map.as_ref(),
    )
    .expect("a map pill");
    let mut panel = panel;
    assert_eq!(
        claim(&mut panel, &ctx, &told, at(off.pill.center())),
        Claim::Panel
    );
    assert_eq!(
        claim(&mut panel, &ctx, &told, at(map.pill.center())),
        Claim::Panel
    );
}

/// When no map is assigned, the map pill displays `none`.
#[test]
fn the_map_pill_names_the_file_and_says_none_where_there_is_not_one() {
    assert_eq!(MapPill::NONE.word(), "none");
    assert_eq!(
        MapPill {
            name: Some("nanoKONTROL2".to_owned())
        }
        .word(),
        "nanoKONTROL2"
    );
    // A longer name makes a wider capsule, which is what makes the pill a
    // readout of the file rather than a fixed box with a word in it.
    let (panel, ctx) = console(PLAUSIBLE);
    let short = view(Some(MapPill::NONE), false);
    let long = view(
        Some(MapPill {
            name: Some("a_very_long_controller_name".to_owned()),
        }),
        false,
    );
    let narrow = map_pill(
        &ctx,
        panel.layout(),
        short.transport,
        None,
        None,
        short.map.as_ref(),
    )
    .expect("a pill");
    let wide = map_pill(
        &ctx,
        panel.layout(),
        long.transport,
        None,
        None,
        long.map.as_ref(),
    )
    .expect("a pill");
    assert!(
        wide.pill.width() > narrow.pill.width(),
        "the capsule did not follow the name it was handed"
    );
}
