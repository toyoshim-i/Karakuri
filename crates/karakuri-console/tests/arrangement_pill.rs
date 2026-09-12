//! The transport row's arrangement pill: the console's sixth control, and its
//! first with a menu.
//!
//! Six things, and the first two are why this file exists rather than a few
//! more assertions in `transport.rs`:
//!
//! 1. Where the control is, derived from the row's own geometry. 2. That it
//! clears every boundary's grab, which is `tests/outputs.rs`'s arithmetic over
//! a different control and is never inherited from it: the row is 48 and a
//! `.pill` is 16.5, so the clearance is 15.75 against a `GRAB` of 6 — and that
//! is measured here rather than reasoned from the Outputs row's 7.75. 3. That
//! an open menu keeps the pointer, which is the rule `karakuri_console::input`
//! gained with this control: the card is drawn across the boundary under the
//! row, so a rule that gave the boundary first refusal would leave rows of the
//! menu dead. 4. That the pill says what it was handed, and that the default
//! arrangement is not called `default`. 5. What each item of the menu asks for:
//! the reset, a restore by name, and the one item that asks for letters. 6.
//! That the name being typed is the console's own state and moves only through
//! the methods that move it.
//!
//! None of it needs a window, a device or a disk. It does need `egui`'s fonts,
//! because the capsule is as wide as the name in it — see `common::drawn_once`
//! — and a `Transport`, because the pill's place is one gap after the bar and a
//! row with no engine behind it draws nothing at all.

mod common;

use common::{drawn_once, near, rect_of, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Op, Outcome, Panel, GRAB};
use karakuri_console::room::{size, Room};
use karakuri_console::view::{arrangement, Arrangement, Ask, Item, Menu, Transport, View};
use karakuri_layout::{Point, Rect};
use karakuri_operation::Operation;

/// The mock's own transport, as numbers — `transport.rs`'s `mock`, which is
/// where the argument for each of them is. The pill sits one gap after the bar
/// this draws, so a row is needed to have a pill at all.
fn mock() -> Transport {
    common::mock_transport()
}

/// A view with an engine behind it and that arrangement in front of it — what
/// `View::draw` paints from and what `claim` hit-tests, one value.
fn view(arr: Arrangement) -> View {
    let mut view = View::new(Room::Day);
    view.transport = Some(mock());
    view.arrangement = arr;
    view
}

/// A panel at a viewport, solved, with a context that has drawn once — the pair
/// every test here starts from, and `outputs.rs`'s own opening.
fn console(viewport: Rect) -> (Panel, egui::Context) {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    (panel, drawn_once())
}

/// A `karakuri_layout` point, from `egui`'s.
fn at(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

/// An arrangement in use, with three filed beside it.
fn filed() -> Arrangement {
    Arrangement {
        name: Some("night".to_owned()),
        filed: vec![
            "four_deck".to_owned(),
            "night".to_owned(),
            "rehearsal".to_owned(),
        ],
        menu: Menu::Shut,
    }
}

// ---------------------------------------------------------------------------
// Where the control is
// ---------------------------------------------------------------------------

/// The pill is the row's own geometry and the mock's own box, and every number
/// here is read off `style.css` rather than off the panel.
///
/// `.pill { border-radius: 999px; padding: 0 8px }` around text at 11px and
/// `line-height: 1.5`, in a `.transport` whose `gap` is 14 — so the capsule
/// starts one gap after the bar, is 16.5 tall whatever the row is, and holds
/// its words one padding in.
#[test]
fn the_pill_is_the_rows_own_geometry() {
    let (panel, ctx) = console(SMALLEST);
    let strip = rect_of(panel.layout(), "transport");
    let arr = filed();
    let pill = arrangement(&ctx, panel.layout(), Some(mock()), None, None, None, &arr)
        .expect("the row draws it");
    let row = karakuri_console::view::transport(&ctx, panel.layout(), Some(mock())).expect("a row");

    assert!(
        near(pill.pill.min.x, row.bar.max.x + size::TRANSPORT_GAP),
        "the capsule starts {} after the bar and `.transport`'s gap is {}",
        pill.pill.min.x - row.bar.max.x,
        size::TRANSPORT_GAP
    );
    assert!(
        near(pill.pill.height(), size::PILL_H),
        "the capsule is {} tall and a pill is 11 at line-height 1.5",
        pill.pill.height()
    );
    assert!(near(pill.pill.center().y, strip.y + strip.h * 0.5));

    // The words one padding in, and the chevron one padding from the far end.
    assert!(near(pill.text.min.x, pill.pill.min.x + size::PILL_PAD_X));
    assert!(near(pill.chevron.max.x, pill.pill.max.x - size::PILL_PAD_X));
    assert!(pill.chevron.min.x > pill.text.max.x);

    // The whole of it is inside the row it is drawn in, and clear of the
    // frame readout at the other end.
    assert!(
        egui::Rect::from_min_size(egui::pos2(strip.x, strip.y), egui::vec2(strip.w, strip.h))
            .contains_rect(pill.pill),
        "the capsule {:?} is not inside the row {strip:?}",
        pill.pill
    );
    assert!(
        pill.pill.max.x + size::TRANSPORT_GAP <= row.frame.min.x,
        "the capsule ends at {} and the frame readout starts at {}",
        pill.pill.max.x,
        row.frame.min.x
    );
}

/// The capsule is as wide as the name in it, which is what makes this a control
/// that has to be measured rather than a rectangle written down.
#[test]
fn the_capsule_is_as_wide_as_the_name_in_it() {
    let (panel, ctx) = console(PLAUSIBLE);
    let short = Arrangement {
        name: Some("a".to_owned()),
        ..Arrangement::NONE
    };
    let long = Arrangement {
        name: Some("a_very_long_arrangement_name".to_owned()),
        ..Arrangement::NONE
    };
    let of = |arr: &Arrangement| {
        arrangement(&ctx, panel.layout(), Some(mock()), None, None, None, arr)
            .expect("a pill")
            .pill
            .width()
    };
    assert!(
        of(&long) > of(&short),
        "a long name and a short one laid out to the same capsule: {} and {}",
        of(&long),
        of(&short)
    );
}

// ---------------------------------------------------------------------------
// The claim rule
// ---------------------------------------------------------------------------

/// The pill clears every boundary's grab, measured here and never inherited
/// from the Outputs row's sink.
///
/// `karakuri_console::input`'s hazard is the same one: `GRAB` widens every
/// boundary by six pixels either side, and those twelve pixels are inside the
/// bays, over whatever a bay draws at its edge.
///
/// The numbers say it clears by more than any other control on the panel: the
/// transport row is 48, a `.pill` is 16.5 and it is centred, so there is (48 -
/// 16.5) / 2 = 15.75 of row above the capsule and 15.75 below, against a grab
/// of 6. The boundary under the row gives up 9.75 pixels short of the control.
///
/// So this fails if the control moves, if the row gets shorter, or if `GRAB`
/// widens past 15.75 — and the last is the point: the fix then is to change the
/// rule in `input`, deliberately, rather than to nudge the pill.
#[test]
fn the_pill_clears_every_boundarys_grab() {
    for viewport in [SMALLEST, PLAUSIBLE] {
        let (mut panel, ctx) = console(viewport);
        let arr = filed();
        let pill = arrangement(&ctx, panel.layout(), Some(mock()), None, None, None, &arr)
            .expect("a pill");
        let strip = rect_of(panel.layout(), "transport");

        let clearance = (strip.h - size::PILL_H) * 0.5;
        assert!(
            near(clearance, 15.75),
            "the clearance is {clearance} and the row's 48 around a pill's 16.5 is 15.75"
        );
        assert!(
            clearance > GRAB,
            "the capsule has {clearance} of row above it and a boundary grabs {GRAB} — the \
             control is inside a boundary's grab, and `input`'s rule is what has to change"
        );
        assert!(near(pill.pill.min.y - strip.y, clearance));
        assert!(near(strip.y + strip.h - pill.pill.max.y, clearance));

        // And the hit test agrees, at every corner and edge of the capsule
        // rather than from the sum.
        let view = view(arr);
        for probe in [
            pill.pill.left_top(),
            pill.pill.right_top(),
            pill.pill.left_bottom(),
            pill.pill.right_bottom(),
            pill.pill.center(),
            pill.pill.center_top(),
            pill.pill.center_bottom(),
        ] {
            assert!(
                !matches!(
                    panel.layout().hit(at(probe), GRAB),
                    karakuri_layout::Hit::Divider { .. }
                ),
                "a boundary grabs {probe:?}, which is on the control"
            );
            assert_eq!(
                claim(&mut panel, &ctx, &view, at(probe)),
                Claim::Panel,
                "the panel does not get a press at {probe:?}, which is on its own control"
            );
        }

        // The band under it is still the boundary's, which is what says the
        // clearance is a clearance and not the grab having gone missing.
        assert!(
            matches!(
                panel
                    .layout()
                    .hit(Point::new(pill.pill.center().x, strip.y + strip.h), GRAB),
                karakuri_layout::Hit::Divider { .. }
            ),
            "the bottom edge of the transport row is not in the grab of the boundary under \
             it, so this test is no longer measuring the clearance it was written for"
        );
    }
}

/// The three readouts beside the pill are still readouts, and this test is what
/// says so as the row gains controls.
///
/// It named four until 2026-09-08. The tempo figure left the list when it
/// became the free-run tempo's control
/// ([ADR-0291](../../../docs/adr/0291-the-tempo-figure-is-the-track-and-the-band-is-a-guard-on-the-hand.md)),
/// and the point pressed for it here is now on the guard either side of the
/// band rather than at the figure's centre — a press a hand is not trusted to
/// have meant, which the row declines and `egui` gets. That is the one point in
/// this file where a readout and a control share a rectangle, and pressing the
/// centre would assert the opposite of what the row now does.
#[test]
fn only_the_pill_is_claimed_out_of_the_transport_row() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let arr = filed();
    let pill =
        arrangement(&ctx, panel.layout(), Some(mock()), None, None, None, &arr).expect("a pill");
    let row = karakuri_console::view::transport(&ctx, panel.layout(), Some(mock())).expect("a row");
    let view = view(arr);

    assert_eq!(
        claim(&mut panel, &ctx, &view, at(pill.pill.center())),
        Claim::Panel
    );
    for (probe, what) in [
        (
            egui::pos2(row.bpm.min.x + 1.0, row.bpm.center().y),
            "the guard at the low end of the tempo figure",
        ),
        (row.grid.center(), "the beat grid"),
        (row.bar.center(), "the bar"),
        (row.frame.center(), "the frame readout"),
        (
            egui::pos2(pill.pill.max.x + 20.0, pill.pill.center().y),
            "the empty middle of the row",
        ),
    ] {
        assert_eq!(
            claim(&mut panel, &ctx, &view, at(probe)),
            Claim::Egui,
            "{what} is being claimed as a control the panel acts on"
        );
    }
}

/// Before anything has been drawn there is no control, and a row that is not
/// drawn has no pill: a capsule is as wide as the name in it, and a press
/// cannot be on something that has never been on screen.
#[test]
fn a_control_that_has_not_been_drawn_is_not_there() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let fresh = egui::Context::default();
    assert_eq!(fresh.cumulative_pass_nr(), 0);
    assert_eq!(
        arrangement(
            &fresh,
            panel.layout(),
            Some(mock()),
            None,
            None,
            None,
            &filed()
        ),
        None
    );

    // And with no engine behind the console there is no row at all, so there
    // is nothing for the pill to sit after — `transport`'s answer, not a
    // second one.
    let ctx = drawn_once();
    assert_eq!(
        arrangement(&ctx, panel.layout(), None, None, None, None, &filed()),
        None
    );
}

// ---------------------------------------------------------------------------
// What the pill says
// ---------------------------------------------------------------------------

/// The default arrangement has no name, and the word for it is not one.
///
/// ADR-0221: the default reaches code rather than a file, an operator may save
/// an arrangement called `default`, and it shadows nothing. A pill that wrote
/// `default` for the built-in would read the same for two different states —
/// and the word chosen instead has a space in it, so nothing can ever be filed
/// under it.
#[test]
fn the_default_arrangement_has_no_name_and_is_not_called_default() {
    let none = Arrangement::NONE;
    let word = none.word();
    assert!(
        word.contains(' '),
        "`{word}` is a name an operator could file an arrangement under, so the pill would \
         read the same for the built-in and for that file"
    );
    assert_ne!(word, "default");
    assert_eq!(
        filed().word(),
        "night",
        "the pill is not saying the name in use"
    );
}

// ---------------------------------------------------------------------------
// The menu
// ---------------------------------------------------------------------------

/// A press on the pill opens the menu, and a press on it again shuts it.
#[test]
fn a_press_on_the_pill_opens_the_menu_and_a_press_again_shuts_it() {
    let (panel, ctx) = console(PLAUSIBLE);
    let mut arr = filed();
    let shut =
        arrangement(&ctx, panel.layout(), Some(mock()), None, None, None, &arr).expect("a pill");
    assert_eq!(
        shut.menu, None,
        "the menu is drawn with nothing asking for it"
    );
    assert_eq!(shut.rows, 0);
    assert_eq!(shut.ask(&arr, at(shut.pill.center())), Some(Ask::Open));

    arr.opened();
    let open =
        arrangement(&ctx, panel.layout(), Some(mock()), None, None, None, &arr).expect("a pill");
    let card = open.menu.expect("the menu is down and nothing is drawn");
    assert_eq!(
        open.rows,
        2 + arr.filed.len(),
        "the menu is save, start a new one, and one row per name filed"
    );
    assert!(
        card.min.y > open.pill.max.y,
        "the card is at {:?} and the pill ends at {}",
        card.min,
        open.pill.max.y
    );
    assert_eq!(open.ask(&arr, at(open.pill.center())), Some(Ask::Shut));

    // A press on the card but on no row — its padding — is the dismissal
    // rather than a press that did nothing at all.
    let padding = egui::pos2(card.center().x, card.min.y + 1.0);
    assert_eq!(open.item(at(padding)), None);
    assert_eq!(open.ask(&arr, at(padding)), Some(Ask::Shut));
}

/// An open menu keeps every pointer event, boundary or no boundary.
///
/// This is `input`'s rule 2, and it is the reason the rule exists: the card
/// hangs out of the transport row and down over the bays, so it crosses the
/// boundary under that row. Under a rule that gave the boundary first refusal,
/// the rows of the menu behind that band would be dead — silently, in the
/// middle of a list an operator is reading.
#[test]
fn an_open_menu_keeps_the_pointer_and_a_shut_one_gives_the_boundary_back() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let strip = rect_of(panel.layout(), "transport");
    // A point in the grab of the boundary under the transport row, and one
    // out in a bay where nothing the console draws is.
    let boundary = Point::new(strip.x + strip.w * 0.5, strip.y + strip.h);
    let elsewhere = Point::new(strip.x + strip.w * 0.5, strip.y + strip.h + 300.0);

    let mut arr = filed();
    assert_eq!(
        claim(&mut panel, &ctx, &view(arr.clone()), boundary),
        Claim::Panel,
        "the boundary under the row is not the panel's with the menu shut"
    );
    assert_eq!(
        claim(&mut panel, &ctx, &view(arr.clone()), elsewhere),
        Claim::Egui
    );

    arr.opened();
    let open = view(arr.clone());
    assert_eq!(claim(&mut panel, &ctx, &open, boundary), Claim::Panel);
    assert_eq!(
        claim(&mut panel, &ctx, &open, elsewhere),
        Claim::Panel,
        "a point out in a bay went to `egui` with a menu open over the console"
    );

    // And it is the menu and not something that stuck: shut it, and the bay
    // is `egui`'s again.
    arr.shut();
    assert_eq!(
        claim(&mut panel, &ctx, &view(arr), elsewhere),
        Claim::Egui,
        "the menu was shut and the console is still holding every event"
    );
}

/// Start a new one is the reset, and it is the same operation `r` performs.
///
/// Not *like* the reset: the identical `Op`, applied to a panel that has been
/// folded about, giving the identical arrangement. A control that reset through
/// some path of its own is how the two stop being one operation.
#[test]
fn start_a_new_one_is_the_reset_the_r_key_performs() {
    let (panel, ctx) = console(PLAUSIBLE);
    let mut arr = filed();
    arr.opened();
    let pill =
        arrangement(&ctx, panel.layout(), Some(mock()), None, None, None, &arr).expect("a pill");

    let row = pill.row(1);
    assert_eq!(pill.item(at(row.center())), Some(Item::New));
    assert_eq!(
        pill.ask(&arr, at(row.center())),
        Some(Ask::Panel(Op::Reset))
    );

    // The same operation, over the same fold, reaching the same arrangement.
    let fold = |panel: &mut Panel| {
        let id = panel.layout().find("library").expect("the library bay");
        panel.op(Op::Fold(id));
    };
    let mut by_key = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    fold(&mut by_key);
    let mut by_pill = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    fold(&mut by_pill);

    assert_eq!(by_key.op(Op::Reset), Outcome::Reset);
    let Some(Ask::Panel(op)) = pill.ask(&arr, at(row.center())) else {
        panic!("the item did not ask for an operation of the panel");
    };
    assert_eq!(by_pill.op(op), Outcome::Reset);
    by_key.solve();
    by_pill.solve();
    assert_eq!(
        common::rects(by_key.layout()),
        common::rects(by_pill.layout()),
        "the menu's reset and `r`'s reset reached two different arrangements"
    );
}

/// Load is the list, and a pick asks for that name back.
///
/// The names are the ones handed in, in the order they were handed in, and the
/// operation carries the name the row was drawn with — not the row's index,
/// which is exactly the handle ADR-0221 refused.
#[test]
fn the_menu_lists_the_names_handed_in_and_a_pick_puts_that_one_back() {
    let (panel, ctx) = console(PLAUSIBLE);
    let mut arr = filed();
    arr.opened();
    let pill =
        arrangement(&ctx, panel.layout(), Some(mock()), None, None, None, &arr).expect("a pill");

    for (index, name) in arr.filed.iter().enumerate() {
        let row = pill.row(2 + index);
        assert_eq!(pill.item(at(row.center())), Some(Item::Filed(index)));
        assert_eq!(
            pill.ask(&arr, at(row.center())),
            Some(Ask::Operation(Operation::RestoreArrangement {
                name: name.clone()
            })),
            "row {index} asks for the wrong arrangement back"
        );
    }

    // A console with nothing filed lists nothing and still offers the two
    // verbs: there is nothing to put back, and saying so is drawing no rows
    // rather than an empty list.
    let mut none = Arrangement::NONE;
    none.opened();
    let empty =
        arrangement(&ctx, panel.layout(), Some(mock()), None, None, None, &none).expect("a pill");
    assert_eq!(empty.rows, 2);
}

/// Save means the name in use, and asks for one where there is none.
///
/// The manual: *"Once a name is in use, saving again means that name: saving
/// over it is what saving it again is."* So the same row is two different asks,
/// decided by what the pill was handed and not by anything it kept.
#[test]
fn save_means_the_name_in_use_and_asks_for_one_where_there_is_none() {
    let (panel, ctx) = console(PLAUSIBLE);

    let mut in_use = filed();
    in_use.opened();
    let pill = arrangement(
        &ctx,
        panel.layout(),
        Some(mock()),
        None,
        None,
        None,
        &in_use,
    )
    .expect("a pill");
    let row = pill.row(0);
    assert_eq!(pill.item(at(row.center())), Some(Item::Save));
    assert_eq!(
        pill.ask(&in_use, at(row.center())),
        Some(Ask::Operation(Operation::SaveArrangement {
            name: "night".to_owned()
        }))
    );

    let mut fresh = Arrangement::NONE;
    fresh.opened();
    let pill =
        arrangement(&ctx, panel.layout(), Some(mock()), None, None, None, &fresh).expect("a pill");
    assert_eq!(pill.ask(&fresh, at(pill.row(0).center())), Some(Ask::Name));
}

// ---------------------------------------------------------------------------
// The name being typed
// ---------------------------------------------------------------------------

/// The buffer is the console's own state and moves only through the two methods
/// that move it, one character at a time — because `src/` has no key events to
/// read (ADR-0156) and whoever holds the keyboard is on the other side of that
/// seam.
///
/// Nothing typed is checked, which is the half that matters: a name that is not
/// one path component is refused where the file is written, in one sentence,
/// and a pill that dropped the characters it did not like would be a rule an
/// operator could only find by experiment (P-0090).
#[test]
fn typing_fills_the_name_and_nothing_in_it_is_checked() {
    let mut arr = Arrangement::NONE;
    assert_eq!(arr.naming(), None, "a shut menu is asking for a name");
    assert!(!arr.typed('n'), "a shut menu took a character");
    assert!(!arr.rubbed_out());

    arr.asks_a_name();
    assert_eq!(arr.naming(), Some(""));
    for c in "night".chars() {
        assert!(arr.typed(c));
    }
    assert_eq!(arr.naming(), Some("night"));

    // A character an arrangement name may not hold still goes in: the wall is
    // where the file is written and not here.
    assert!(arr.typed('/'));
    assert_eq!(arr.naming(), Some("night/"));
    assert!(arr.rubbed_out());
    assert_eq!(arr.naming(), Some("night"));

    // A control character is not a letter — a newline arriving as text is the
    // commit, and it must not land in the name.
    assert!(!arr.typed('\n'));
    assert_eq!(arr.naming(), Some("night"));

    // Rubbing out an empty name changes nothing, which is what earns no frame.
    for _ in 0..5 {
        arr.rubbed_out();
    }
    assert_eq!(arr.naming(), Some(""));
    assert!(!arr.rubbed_out());

    // And shutting the menu takes the half-typed name with it: the buffer is
    // the gesture, and the gesture ended.
    arr.typed('x');
    arr.shut();
    assert_eq!(arr.naming(), None);
    arr.asks_a_name();
    assert_eq!(arr.naming(), Some(""));
}

/// A menu asking for a name is a field and not a list, so there is nothing to
/// pick and no row to hand out a rectangle for.
#[test]
fn a_menu_asking_for_a_name_has_no_rows_to_pick() {
    let (panel, ctx) = console(PLAUSIBLE);
    let mut arr = filed();
    arr.asks_a_name();
    let pill =
        arrangement(&ctx, panel.layout(), Some(mock()), None, None, None, &arr).expect("a pill");
    let card = pill.menu.expect("a field to type into");
    assert_eq!(pill.rows, 0, "a field is being drawn as a list of items");
    assert_eq!(pill.item(at(card.center())), None);
    // A press inside it is not a pick, and it is not nothing either.
    assert_eq!(pill.ask(&arr, at(card.center())), Some(Ask::Shut));
}

// ---------------------------------------------------------------------------
// The values are the harness's
// ---------------------------------------------------------------------------

/// Everything on the pill came in through the argument and the console keeps
/// none of it — the seam `View::picture` is on, which is the reason this crate
/// can be asked about a console with no store anywhere near it.
///
/// Asked twice with two arrangements and once more with the first: a console
/// that had kept anything would answer the third call with the second's.
#[test]
fn the_pill_is_the_harnesss_and_is_stored_nowhere() {
    let (panel, ctx) = console(PLAUSIBLE);
    let one = filed();
    let two = Arrangement {
        name: None,
        filed: vec!["only_one".to_owned()],
        menu: Menu::Open,
    };

    let first =
        arrangement(&ctx, panel.layout(), Some(mock()), None, None, None, &one).expect("a pill");
    let other =
        arrangement(&ctx, panel.layout(), Some(mock()), None, None, None, &two).expect("a pill");
    assert_ne!(
        first, other,
        "two different arrangements drew the same pill, so something in it is not coming \
         from the argument"
    );
    assert_eq!(first.rows, 0, "a shut menu has rows");
    assert_eq!(other.rows, 3, "one name filed is two verbs and one row");
    assert_eq!(
        arrangement(&ctx, panel.layout(), Some(mock()), None, None, None, &one).expect("a pill"),
        first,
        "the pill remembered the arrangement it was last asked about"
    );

    // And the view holds exactly what a caller put there — a plain field, not
    // a copy the console maintains.
    let mut view = View::new(Room::Day);
    assert_eq!(view.arrangement, Arrangement::NONE);
    view.arrangement = one.clone();
    assert_eq!(view.arrangement, one);
}

/// The menu lists as many names as fit and says how many it left out, rather
/// than the list quietly ending where the window does.
///
/// The Library bay's own answer to the same question, at the same `.lib-row`
/// box — a count in a foot, which is the difference between a list that is
/// short and a list that has been cut.
#[test]
fn the_menu_lists_what_fits_and_says_how_many_it_left_out() {
    let ctx = drawn_once();
    // A window with room for the row and very little under it, so the card
    // runs out of console long before it runs out of names.
    let mut panel = Panel::new(SMALLEST.w, SMALLEST.h);
    panel.solve();
    let arr = Arrangement {
        name: None,
        filed: (0..200).map(|n| format!("arrangement_{n}")).collect(),
        menu: Menu::Open,
    };
    let pill =
        arrangement(&ctx, panel.layout(), Some(mock()), None, None, None, &arr).expect("a pill");
    let card = pill.menu.expect("a menu");
    assert_eq!(pill.of, 202, "two verbs and two hundred names");
    assert!(
        pill.rows < pill.of,
        "the card claims to be showing all {} of them in a window {} tall",
        pill.of,
        SMALLEST.h
    );
    assert!(pill.rows > 0, "the card shows nothing at all");
    assert!(
        card.max.y <= panel.layout().viewport().h + 0.001,
        "the card ends at {} and the console is {} tall",
        card.max.y,
        panel.layout().viewport().h
    );
    // Every row it claims has a rectangle inside the card.
    for index in 0..pill.rows {
        assert!(
            card.contains_rect(pill.row(index)),
            "row {index} is drawn outside its own card"
        );
    }
}

/// A folded, soloed or narrow row draws no pill, which is `transport`'s answer
/// rather than a second one: where there is no row there is no control.
#[test]
fn a_folded_or_soloed_or_narrow_row_draws_no_pill() {
    let ctx = drawn_once();
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let arr = filed();
    assert!(arrangement(&ctx, panel.layout(), Some(mock()), None, None, None, &arr).is_some());

    let row = panel.layout().find("transport").expect("transport");
    panel.op(Op::Fold(row));
    panel.solve();
    assert_eq!(
        arrangement(&ctx, panel.layout(), Some(mock()), None, None, None, &arr),
        None,
        "the transport row is folded away and its control is still being drawn"
    );
    panel.op(Op::UnfoldAll);
    panel.solve();

    let library = panel.layout().find("library").expect("library");
    panel.op(Op::Solo(library));
    panel.solve();
    assert_eq!(
        arrangement(&ctx, panel.layout(), Some(mock()), None, None, None, &arr),
        None,
        "a solo left the transport row invisible and its control still drawn"
    );
}
