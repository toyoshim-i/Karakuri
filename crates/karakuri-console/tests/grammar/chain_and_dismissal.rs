#![allow(unused_imports)]

use super::grammar_common::*;

// ---------------------------------------------------------------------------
// The Master chain's list
// ---------------------------------------------------------------------------

/// The Master bay's items are the out fader, one per slot of the chain and
/// `+ add`, so `console`'s two-slot chain numbers them 1 to 4 — and a slot's
/// controls are its parameter rows, its cut chip and its `−` (ADR-0352).
const OUT: &[usize] = &[1];
const RETAINING_SLOT: &[usize] = &[2];
const PLAIN_SLOT: &[usize] = &[3];
const ADD_EFFECT: &[usize] = &[4];

/// `enter` on `+ add` puts the chooser down, and the `kind L5` procedures it
/// lists are the rung under it: a digit names the nth and the address descends
/// (ADR-0352).
#[test]
fn enter_on_add_effect_puts_the_chooser_down_and_the_address_descends_into_it() {
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "master", ADD_EFFECT);
    assert!(
        !view.chain_add_open(),
        "the chooser was already down before anything was pressed"
    );

    // While the card is up there is no rung under `+ add`, and the refusal
    // says which press draws it.
    match press(&mut view, &panel, Press::Digit(1)) {
        Asked::Nothing(why) => assert!(
            why.contains("enter") && why.contains("+ add"),
            "a digit on `+ add` with the card up declined without naming the press that puts it \
             down: {why}"
        ),
        other => panic!("a digit named a row of a chooser that is not down: {other:?}"),
    }
    assert_eq!(at(&view, "master"), vec![4]);

    assert_eq!(
        press(&mut view, &panel, Press::Enter),
        Asked::Moved,
        "`enter` on `+ add` did not put the chooser down"
    );
    assert!(view.chain_add_open(), "the card is not down");

    // A second `enter` declines and names the keys that reach what is on it.
    match press(&mut view, &panel, Press::Enter) {
        Asked::Nothing(why) => assert!(
            why.contains("already down"),
            "`enter` on a chooser that is down declined without saying so: {why}"
        ),
        other => panic!("`enter` put a card down twice: {other:?}"),
    }

    // And now the digits count what is on it, from one.
    let offered = view.chain_choices().items.len();
    assert_eq!(
        offered, 2,
        "this console offers the wrong number of procedures"
    );
    match press(&mut view, &panel, Press::Digit(offered + 1)) {
        Asked::Nothing(why) => assert!(
            why.contains("from one"),
            "a digit past the end of the chooser declined without saying what the digits count: \
             {why}"
        ),
        other => panic!("a digit named a procedure the chooser is not offering: {other:?}"),
    }
    assert_eq!(
        at(&view, "master"),
        vec![4],
        "a refused digit descended anyway"
    );
    assert_eq!(press(&mut view, &panel, Press::Digit(2)), Asked::Moved);
    assert_eq!(
        at(&view, "master"),
        vec![4, 2],
        "the address did not descend into the chooser"
    );
}

/// The `+ add` chooser's rows are a column, so `↑↓` walk them and `←→` are
/// refused with the pair that works — walked and clamped, never wrapped.
#[test]
fn the_add_choosers_rows_are_a_column_the_arrows_walk() {
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "master", ADD_EFFECT);
    press(&mut view, &panel, Press::Enter);
    let offered = view.chain_choices().items.len();

    match press(&mut view, &panel, Press::Arrow(Arrow::Right)) {
        Asked::Nothing(why) => assert!(
            why.contains("column") && why.contains("up and down"),
            "`→` in the chooser declined without naming the axis that works: {why}"
        ),
        other => panic!("`→` walked a column of procedures: {other:?}"),
    }

    // The walk starts from the row the chooser remembers, with no digit
    // pressed first, and the address follows it into the card.
    assert_eq!(
        press(&mut view, &panel, Press::Arrow(Arrow::Down)),
        Asked::Moved
    );
    assert_eq!(at(&view, "master"), vec![4, 2]);

    for _ in 0..offered + 2 {
        press(&mut view, &panel, Press::Arrow(Arrow::Down));
    }
    assert_eq!(
        at(&view, "master"),
        vec![4, offered],
        "the walk wrapped past the end of the chooser instead of stopping at it"
    );
    match press(&mut view, &panel, Press::Arrow(Arrow::Down)) {
        Asked::Nothing(why) => assert!(
            why.contains("end"),
            "a walk off the end of the chooser declined without saying so: {why}"
        ),
        other => panic!("the walk ran off the end of the chooser: {other:?}"),
    }
}

/// `enter` on one of the chooser's rows appends a slot of that procedure, takes
/// the card away and puts the address back on `+ add` — the cut is `Some`
/// exactly where the procedure declares `retains`.
#[test]
fn enter_on_an_add_chooser_row_appends_the_slot_and_takes_the_card_away() {
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "master", ADD_EFFECT);
    press(&mut view, &panel, Press::Enter);
    press(&mut view, &panel, Press::Digit(1));

    assert_eq!(
        press(&mut view, &panel, Press::Enter),
        Asked::Emitted(Operation::AddChainEffect {
            procedure: "sha256:feed".to_owned(),
            cut: Some(Cut::Mix),
        }),
        "`enter` on a row of the chooser did not append the procedure it names"
    );
    assert!(
        !view.chain_add_open(),
        "the card was left standing over a slot that has already been asked for"
    );
    assert_eq!(
        at(&view, "master"),
        vec![4],
        "the address did not go back to `+ add`"
    );

    // And the second row, whose procedure declares no `retains`, arrives with
    // no cut at all.
    press(&mut view, &panel, Press::Enter);
    press(&mut view, &panel, Press::Digit(2));
    assert_eq!(
        press(&mut view, &panel, Press::Enter),
        Asked::Emitted(Operation::AddChainEffect {
            procedure: "sha256:b100".to_owned(),
            cut: None,
        })
    );
}

/// A slot's parameter row is a level: `↑↓` step it a tenth of the range its
/// procedure declares and `space` returns it to the value that procedure
/// declared it at (ADR-0352).
#[test]
fn the_arrows_step_a_chain_parameter_by_a_tenth_of_its_declared_range() {
    let (panel, mut view) = console();
    // The first slot's first parameter row — `amount`, declared over
    // `0.0 .. 0.95` and holding `0.2`.
    walk_to(&mut view, &panel, "master", &[RETAINING_SLOT[0], 1]);

    match press(&mut view, &panel, Press::Arrow(Arrow::Up)) {
        Asked::Emitted(Operation::SetChainParam {
            at,
            param: ChainParam::Declared { key, value },
        }) => {
            assert_eq!(at, 0, "the step named the wrong slot of the chain");
            assert_eq!(key, "amount");
            assert!(
                (value - (0.2 + 0.095)).abs() < 1e-6,
                "`↑` did not step a tenth of the declared range: {value}"
            );
        }
        other => panic!("`↑` on a chain parameter row did not set it: {other:?}"),
    }
    match press(&mut view, &panel, Press::Arrow(Arrow::Down)) {
        Asked::Emitted(Operation::SetChainParam {
            param: ChainParam::Declared { value, .. },
            ..
        }) => assert!(
            (value - (0.2 - 0.095)).abs() < 1e-6,
            "`↓` did not step a tenth of the declared range: {value}"
        ),
        other => panic!("`↓` on a chain parameter row did not set it: {other:?}"),
    }
    // `space` is the value the procedure declared it at, which the reading
    // carries because the compiled slot has it.
    match press(&mut view, &panel, Press::Space) {
        Asked::Emitted(Operation::SetChainParam {
            param: ChainParam::Declared { value, .. },
            ..
        }) => assert!(
            value.abs() < 1e-6,
            "`space` did not return the row to what the procedure declared: {value}"
        ),
        other => panic!("`space` on a chain parameter row did not set it: {other:?}"),
    }
    // It performs nothing, so `enter` declines.
    match press(&mut view, &panel, Press::Enter) {
        Asked::Nothing(why) => assert!(
            why.contains("enter"),
            "a chain parameter row declined `enter` without saying why: {why}"
        ),
        other => panic!("a chain parameter row performed something: {other:?}"),
    }
}

/// A slot's cut chip is a state `space` cycles, and it is addressed on every
/// slot: one whose procedure declares no `retains` draws none and declines with
/// that sentence, so a digit means the same control down the whole chain
/// (ADR-0352).
#[test]
fn space_on_a_slots_cut_chip_cycles_it_and_declines_where_the_slot_retains_nothing() {
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "master", &[RETAINING_SLOT[0], 2]);
    assert_eq!(
        press(&mut view, &panel, Press::Space),
        Asked::Emitted(Operation::SetChainParam {
            at: 0,
            param: ChainParam::Cut(Cut::Exit),
        }),
        "`space` on a cut chip did not name the other of the two cuts"
    );
    // A closed list has no axis, so the arrows decline and name `space`.
    match press(&mut view, &panel, Press::Arrow(Arrow::Down)) {
        Asked::Nothing(why) => assert!(
            why.contains("space"),
            "an arrow on the cut chip declined without naming the key that cycles it: {why}"
        ),
        other => panic!("an arrow walked a closed list: {other:?}"),
    }

    // The second slot declares no `retains`, and its chip keeps the number.
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "master", &[PLAIN_SLOT[0], 2]);
    match press(&mut view, &panel, Press::Space) {
        Asked::Nothing(why) => assert!(
            why.contains("retains"),
            "a slot with no cut declined without saying why it has none: {why}"
        ),
        other => panic!("a slot with no retained frame answered with a cut: {other:?}"),
    }
}

/// `enter` on the `−` at the end of a slot's row takes that slot out of the
/// chain, by the position the bay drew it at.
#[test]
fn enter_on_a_slots_minus_takes_it_out_of_the_chain() {
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "master", &[RETAINING_SLOT[0], 3]);
    assert_eq!(
        press(&mut view, &panel, Press::Enter),
        Asked::Emitted(Operation::RemoveChainEffect { at: 0 })
    );
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "master", &[PLAIN_SLOT[0], 3]);
    assert_eq!(
        press(&mut view, &panel, Press::Enter),
        Asked::Emitted(Operation::RemoveChainEffect { at: 1 }),
        "the `−` on the second slot named the wrong position in the chain"
    );

    // A digit past what a slot draws says what the digits count.
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "master", PLAIN_SLOT);
    match press(&mut view, &panel, Press::Digit(4)) {
        Asked::Nothing(why) => assert!(
            why.contains("this slot") && why.contains("from one"),
            "a digit past a slot's controls declined without saying what the digits count: {why}"
        ),
        other => panic!("a digit named a control a slot does not draw: {other:?}"),
    }

    // And the out fader is a level with nothing under it.
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "master", OUT);
    match press(&mut view, &panel, Press::Digit(1)) {
        Asked::Nothing(why) => assert!(
            why.contains("esc"),
            "a digit below the out fader declined without saying how to go back: {why}"
        ),
        other => panic!("the out fader drew a rung: {other:?}"),
    }
}

/// `esc` takes the `+ add` chooser away and leaves the address on `+ add`, from
/// inside the card and from the control that opened it.
#[test]
fn esc_takes_the_add_chooser_away_and_leaves_the_address_on_add_effect() {
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "master", ADD_EFFECT);
    press(&mut view, &panel, Press::Enter);
    press(&mut view, &panel, Press::Digit(1));
    assert_eq!(at(&view, "master"), vec![4, 1]);

    assert!(
        view.focus_up(&panel),
        "`esc` inside the chooser acted on nothing"
    );
    assert!(!view.chain_add_open(), "`esc` left the card down");
    assert_eq!(
        at(&view, "master"),
        vec![4],
        "`esc` did not leave the address on `+ add`"
    );

    // And from `+ add` itself, where the card is down and the address never
    // descended: the card goes and the address stays.
    press(&mut view, &panel, Press::Enter);
    assert!(view.chain_add_open());
    assert!(view.focus_up(&panel));
    assert!(
        !view.chain_add_open(),
        "`esc` on `+ add` left the card down"
    );
    assert_eq!(at(&view, "master"), vec![4]);

    // With no card down it is the ordinary climb again.
    assert!(view.focus_up(&panel));
    assert_eq!(at(&view, "master"), Vec::<usize>::new());
}

// ---------------------------------------------------------------------------
// A card that is not on the address's path
// ---------------------------------------------------------------------------

/// `esc` climbs the focused bay hierarchy and only dismisses cards along that path (ADR-0332).
#[test]
fn esc_in_another_bay_leaves_the_lane_chooser_alone() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    press(&mut view, &panel, Press::Digit(2));
    assert!(view.open_lane(), "the chooser did not go down");

    assert!(view.focus_up(&panel), "`esc` acted on nothing");
    assert!(
        view.lane_open(),
        "`esc` took a card that is not on the address's path"
    );
    assert_eq!(
        at(&view, "mixer"),
        Vec::<usize>::new(),
        "`esc` did not go up one level of the focused bay's address"
    );
}

/// [`esc_in_another_bay_leaves_the_lane_chooser_alone`]'s rule, one bay along.
#[test]
fn esc_in_another_bay_leaves_the_add_chooser_alone() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    press(&mut view, &panel, Press::Digit(2));
    assert!(view.open_chain_add(), "the chooser did not go down");

    assert!(view.focus_up(&panel), "`esc` acted on nothing");
    assert!(
        view.chain_add_open(),
        "`esc` took a card that is not on the address's path"
    );
    assert_eq!(at(&view, "mixer"), Vec::<usize>::new());
}

/// [`esc_in_another_bay_leaves_the_lane_chooser_alone`]'s rule, on the audio-in
/// pill's card.
#[test]
fn esc_in_another_bay_leaves_the_audio_in_card_alone() {
    let (panel, mut view) = transport();
    focus_on(&mut view, &panel, "mixer");
    press(&mut view, &panel, Press::Digit(2));
    view.audio.as_mut().expect("an audio pill").opened();

    assert!(view.focus_up(&panel), "`esc` acted on nothing");
    assert!(
        view.audio.as_ref().expect("an audio pill").open(),
        "`esc` took a card that is not on the address's path"
    );
    assert_eq!(at(&view, "mixer"), Vec::<usize>::new());
}

/// [`esc_in_another_bay_leaves_the_lane_chooser_alone`]'s rule, on the
/// arrangement pill's menu.
#[test]
fn esc_in_another_bay_leaves_the_arrangement_menu_alone() {
    let (panel, mut view) = transport();
    focus_on(&mut view, &panel, "mixer");
    press(&mut view, &panel, Press::Digit(2));
    view.arrangement.opened();

    assert!(view.focus_up(&panel), "`esc` acted on nothing");
    assert!(
        view.arrangement.open(),
        "`esc` took a card that is not on the address's path"
    );
    assert_eq!(at(&view, "mixer"), Vec::<usize>::new());
}

/// A card down in the bay that has focus is still not `esc`'s unless the address
/// is on the control it hangs from: the key goes up one level of the address it
/// is given, and a card the address is not on is not one of those levels.
#[test]
fn esc_on_another_control_of_the_same_bay_leaves_the_card_alone() {
    let (panel, mut view) = console();
    // `0 1` is the grid mode pill, which hangs no card, and the pointer put
    // the chooser down while the address was on it.
    walk_to(&mut view, &panel, "sequencer", &[HEAD, 1]);
    assert!(view.open_lane(), "the chooser did not go down");

    assert!(view.focus_up(&panel), "`esc` acted on nothing");
    assert!(
        view.lane_open(),
        "`esc` on a control that hangs no card took the card away"
    );
    assert_eq!(
        at(&view, "sequencer"),
        vec![HEAD],
        "`esc` did not go up one level"
    );
}

/// An address on something the bay has stopped drawing goes back to the bay,
/// rather than acting on whatever has taken that position — which is
/// `View::point_at`'s rule about a row past the listing, one level up.
#[test]
fn an_address_on_something_that_is_gone_goes_back_to_the_bay() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    press(&mut view, &panel, Press::Digit(4));
    press(&mut view, &panel, Press::Digit(3));
    assert_eq!(at(&view, "mixer"), vec![4, 3]);

    view.mixer.truncate(2);
    let said = press(&mut view, &panel, Press::Space);
    assert!(
        matches!(said, Asked::Nothing(_)),
        "a press on an address the bay has stopped drawing acted on something: {said:?}"
    );
    assert_eq!(
        at(&view, "mixer"),
        Vec::<usize>::new(),
        "the address stayed on a strip that is gone"
    );

    // Verify item-level address paths are also cleared when the targeted item is removed.
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    press(&mut view, &panel, Press::Digit(4));
    assert_eq!(at(&view, "mixer"), vec![4]);
    view.mixer.truncate(2);
    let said = press(&mut view, &panel, Press::Digit(1));
    assert!(
        matches!(said, Asked::Nothing(_)),
        "a press on an item the bay has stopped drawing acted on something: {said:?}"
    );
    assert_eq!(at(&view, "mixer"), Vec::<usize>::new());
}

/// The floor under the whole file: a `Control` list that had gone empty would
/// satisfy most of it by refusing everything.
#[test]
fn the_bays_are_made_of_what_the_record_walks() {
    let mixer = focus::built("mixer").expect("the mixer's grammar");
    assert_eq!(
        mixer.item().first,
        &[
            Control::Tally,
            Control::Trim,
            Control::Fader,
            Control::Blend,
            Control::Mask
        ],
        "a strip's controls are not the five the bay draws, in the order it draws them"
    );
    assert_eq!(
        mixer.head,
        &[
            Control::Shape,
            Control::Quantum,
            Control::Length,
            Control::Go
        ],
        "the Mixer's head is the transition row: the settings are about the bay rather than \
         about any one strip, which is what a head is (ADR-0343)"
    );
    assert!(mixer.selects && mixer.across);

    let library = focus::built("library").expect("the library's grammar");
    assert_eq!(library.head, &[Control::Scope]);
    assert_eq!(library.item().first, &[Control::Star, Control::Params]);
    assert!(!library.selects && !library.across);
    assert!(library.act.is_some(), "a library row is an act");

    let inspector = focus::built("inspector").expect("the inspector's grammar");
    assert_eq!(inspector.item().first, &[Control::DeckHead]);
    assert_eq!(inspector.item().then, Some(Control::Node));
    assert_eq!(
        inspector.beneath(Control::DeckHead).first,
        &[Control::Sync, Control::Anchor, Control::Composite]
    );
    assert_eq!(inspector.beneath(Control::Node).then, Some(Control::Param));

    // `Undecided` is what a scope step carries, and it is named here so that a
    // payload that grew a value fails against the page rather than silently.
    assert_eq!(
        Operation::SelectScope { scope: Undecided },
        Operation::SelectScope { scope: Undecided }
    );
    // And the scrub is a quarter beat, which is the deck head's own arrows and
    // the amount `↑↓` on an anchor ask for.
    assert_eq!(SCRUB_BEATS, 0.25);
}
