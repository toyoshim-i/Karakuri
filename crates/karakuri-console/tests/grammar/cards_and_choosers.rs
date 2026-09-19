#![allow(unused_imports)]

use super::grammar_common::*;

/// The Sequencer's `+ lane` is `0 6`, and the number is the head's own count —
/// the mode pill, the four bank pills, then the chooser.
const ADD_LANE: &[usize] = &[HEAD, 6];

/// `enter` on `+ lane` puts the chooser down, and its entries are the rung
/// under it: a digit names the nth of them and the address descends
/// (ADR-0351).
#[test]
fn enter_on_add_lane_puts_the_chooser_down_and_the_address_descends_into_it() {
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "sequencer", ADD_LANE);
    assert!(
        !view.lane_open(),
        "the chooser was already down before anything was pressed"
    );

    // While the card is up there is no rung under `+ lane`, and the refusal
    // says which press draws it.
    match press(&mut view, &panel, Press::Digit(1)) {
        Asked::Nothing(why) => assert!(
            why.contains("enter") && why.contains("+ lane"),
            "a digit on `+ lane` with the card up declined without naming the press that puts \
             it down: {why}"
        ),
        other => panic!("a digit named an entry of a chooser that is not down: {other:?}"),
    }
    assert_eq!(at(&view, "sequencer"), vec![HEAD, 6]);

    assert_eq!(
        press(&mut view, &panel, Press::Enter),
        Asked::Moved,
        "`enter` on `+ lane` did not put the chooser down"
    );
    assert!(view.lane_open(), "the card is not down");

    // And now the digits count what is on it, from one.
    let offered = view.lane_choices().items.len();
    assert!(offered >= 2, "this console offers too few targets to walk");
    match press(&mut view, &panel, Press::Digit(offered + 1)) {
        Asked::Nothing(why) => assert!(
            why.contains("from one"),
            "a digit past the end of the chooser declined without saying what the digits \
             count: {why}"
        ),
        other => panic!("a digit named a target the chooser is not offering: {other:?}"),
    }
    assert_eq!(
        at(&view, "sequencer"),
        vec![HEAD, 6],
        "a refused digit descended anyway"
    );
    assert_eq!(press(&mut view, &panel, Press::Digit(2)), Asked::Moved);
    assert_eq!(
        at(&view, "sequencer"),
        vec![HEAD, 6, 2],
        "the address did not descend into the chooser"
    );
}

/// The chooser's entries are a column, so `↑↓` walk them and `←→` are refused
/// with the pair that works — the axis check one rung under the head.
#[test]
fn the_choosers_entries_are_a_column_the_arrows_walk() {
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "sequencer", ADD_LANE);
    press(&mut view, &panel, Press::Enter);
    let offered = view.lane_choices().items.len();

    match press(&mut view, &panel, Press::Arrow(Arrow::Right)) {
        Asked::Nothing(why) => assert!(
            why.contains("column") && why.contains("up and down"),
            "`→` in the chooser declined without naming the axis that works: {why}"
        ),
        other => panic!("`→` walked a column of targets: {other:?}"),
    }

    // The walk starts from the entry the chooser remembers, with no digit
    // pressed first, and the address follows it into the card.
    assert_eq!(
        press(&mut view, &panel, Press::Arrow(Arrow::Down)),
        Asked::Moved
    );
    assert_eq!(at(&view, "sequencer"), vec![HEAD, 6, 2]);

    // And it is walked and clamped rather than wrapped.
    for _ in 0..offered + 2 {
        press(&mut view, &panel, Press::Arrow(Arrow::Down));
    }
    assert_eq!(
        at(&view, "sequencer"),
        vec![HEAD, 6, offered],
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

/// `enter` on an entry points the lane at that target and takes the card away —
/// the same operation the pointer's pick emits, with the bank this bay drew.
#[test]
fn enter_on_a_chooser_entry_points_the_lane_and_takes_the_card_away() {
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "sequencer", ADD_LANE);
    press(&mut view, &panel, Press::Enter);
    let want = view.lane_choices().items[1].target.clone();
    press(&mut view, &panel, Press::Digit(2));

    assert_eq!(
        press(&mut view, &panel, Press::Enter),
        Asked::Emitted(Operation::PointLane {
            // The bank this bay is reading, which is `console`'s own — never
            // whichever is armed by the time the operation is performed.
            pattern: 2,
            target: want
        }),
        "`enter` on an entry of the chooser did not point the lane at what it names"
    );
    assert!(
        !view.lane_open(),
        "the card was left standing over a lane that has already been asked for"
    );
    assert_eq!(
        at(&view, "sequencer"),
        vec![HEAD, 6],
        "the address did not go back to `+ lane`"
    );
}

/// `esc` takes the card away and leaves the address on `+ lane`, from inside the
/// chooser and from the control that opened it.
#[test]
fn esc_takes_the_chooser_away_and_leaves_the_address_on_add_lane() {
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "sequencer", ADD_LANE);
    press(&mut view, &panel, Press::Enter);
    press(&mut view, &panel, Press::Digit(1));
    assert_eq!(at(&view, "sequencer"), vec![HEAD, 6, 1]);

    assert!(
        view.focus_up(&panel),
        "`esc` inside the chooser acted on nothing"
    );
    assert!(!view.lane_open(), "`esc` left the card down");
    assert_eq!(
        at(&view, "sequencer"),
        vec![HEAD, 6],
        "`esc` did not leave the address on `+ lane`"
    );

    // And from `+ lane` itself, where the card is down and the address never
    // descended: the card goes and the address stays.
    press(&mut view, &panel, Press::Enter);
    assert!(view.lane_open());
    assert!(view.focus_up(&panel));
    assert!(!view.lane_open(), "`esc` on `+ lane` left the card down");
    assert_eq!(at(&view, "sequencer"), vec![HEAD, 6]);

    // With no card down it is the ordinary climb again.
    assert!(view.focus_up(&panel));
    assert_eq!(at(&view, "sequencer"), vec![HEAD]);
}

/// `space` sets and `enter` performs, so neither `+ lane` nor one of its entries
/// has a next state — and each refusal names the key that does run it.
#[test]
fn space_declines_on_the_chooser_and_names_enter() {
    let (panel, mut view) = console();
    walk_to(&mut view, &panel, "sequencer", ADD_LANE);
    for _ in 0..2 {
        match press(&mut view, &panel, Press::Space) {
            Asked::Nothing(why) => assert!(
                why.contains("enter"),
                "`space` on the chooser declined without naming the key that runs it: {why}"
            ),
            other => panic!("`space` on the chooser set something: {other:?}"),
        }
        press(&mut view, &panel, Press::Enter);
        press(&mut view, &panel, Press::Digit(1));
    }
}

// ---------------------------------------------------------------------------
// The Transport's two cards, and its tempo figure
// ---------------------------------------------------------------------------

/// The tempo figure, the audio-in pill and the arrangement pill, by the number a
/// digit names each of them with: the row's own order, left to right, which is
/// what the digits count.
const TEMPO: &[usize] = &[1];
const AUDIO_IN: &[usize] = &[7];
const ARRANGEMENT: &[usize] = &[8];

/// The host's half of a press that runs one of a card's rows: the card goes as
/// the operation is named, which is what `Readout::listened` and
/// `Readout::arranged` do at the pointer.
fn take_the_card_away(view: &mut View, asked: &Asked) {
    match asked {
        Asked::Listened(AudioAsk::Operation(_)) => {
            view.audio.as_mut().expect("an audio pill").shut();
        }
        Asked::Arranged(Ask::Operation(_)) => view.arrangement.shut(),
        Asked::Arranged(Ask::Name) => view.arrangement.asks_a_name(),
        other => panic!("this press did not run a row of a card: {other:?}"),
    }
}

/// The host's half of the press that puts a card down: the console says which
/// card and the program puts it there. Opening the audio-in card enumerates the
/// machine's inputs, and this crate takes no device (ADR-0156).
fn put_the_card_down(view: &mut View, asked: &Asked) {
    match asked {
        Asked::Listened(AudioAsk::Open) => view.audio.as_mut().expect("an audio pill").opened(),
        Asked::Arranged(Ask::Open) => view.arrangement.opened(),
        other => panic!("this press did not ask for a card: {other:?}"),
    }
}

/// The tempo figure is a track the arrows step — one press, one beat a minute,
/// named here and stepped by the host — and it has no value `space` returns it
/// to (ADR-0350).
#[test]
fn the_arrows_step_the_tempo_figure_and_space_declines_on_it() {
    let (panel, mut view) = transport();
    walk_to(&mut view, &panel, "transport", TEMPO);
    assert_eq!(
        press(&mut view, &panel, Press::Arrow(Arrow::Up)),
        Asked::Stepped {
            level: Level::Tempo,
            step: Step::Up
        },
        "`1 ↑` in the Transport did not ask the host to step the grid"
    );
    assert_eq!(
        press(&mut view, &panel, Press::Arrow(Arrow::Down)),
        Asked::Stepped {
            level: Level::Tempo,
            step: Step::Down
        }
    );
    match press(&mut view, &panel, Press::Space) {
        Asked::Nothing(why) => assert!(
            why.contains("declared"),
            "`space` on the tempo declined without saying it has no value to return to: {why}"
        ),
        other => panic!("`space` on the tempo figure set something: {other:?}"),
    }
    match press(&mut view, &panel, Press::Arrow(Arrow::Right)) {
        Asked::Nothing(why) => assert!(
            why.contains("up and down"),
            "`→` on the tempo declined without naming the pair that works: {why}"
        ),
        other => panic!("`→` stepped the tempo across: {other:?}"),
    }
    // And with no engine behind the console there is no grid to step.
    let (panel, mut view) = transport();
    view.transport = None;
    walk_to(&mut view, &panel, "transport", TEMPO);
    match press(&mut view, &panel, Press::Arrow(Arrow::Up)) {
        Asked::Nothing(why) => assert!(
            why.contains("grid"),
            "a tempo with no engine behind it declined without saying so: {why}"
        ),
        other => panic!("a console with no engine stepped a grid: {other:?}"),
    }
}

/// `enter` on the audio-in pill puts its card down, the inputs are the rung under
/// it, and `enter` on one attaches that input — the card walked rather than
/// declined (ADR-0350).
#[test]
fn the_audio_in_card_is_walked_and_enter_attaches_the_input_it_is_on() {
    let (panel, mut view) = transport();
    walk_to(&mut view, &panel, "transport", AUDIO_IN);

    // While the card is up there is no rung under the pill, and the refusal
    // says which press draws it.
    match press(&mut view, &panel, Press::Digit(1)) {
        Asked::Nothing(why) => assert!(
            why.contains("enter"),
            "a digit under a card that is up declined without naming the press that puts it \
             down: {why}"
        ),
        other => panic!("a digit named a row of a card that is not down: {other:?}"),
    }
    assert_eq!(at(&view, "transport"), vec![7]);

    let asked = press(&mut view, &panel, Press::Enter);
    assert_eq!(
        asked,
        Asked::Listened(AudioAsk::Open),
        "`enter` on the audio-in pill did not ask for its card"
    );
    put_the_card_down(&mut view, &asked);

    // The digits count what is on the card, from one.
    match press(&mut view, &panel, Press::Digit(3)) {
        Asked::Nothing(why) => assert!(
            why.contains("from one"),
            "a digit past the end of the card declined without saying what the digits count: \
             {why}"
        ),
        other => panic!("a digit named an input the card is not drawing: {other:?}"),
    }
    assert_eq!(press(&mut view, &panel, Press::Digit(2)), Asked::Moved);
    assert_eq!(
        at(&view, "transport"),
        vec![7, 2],
        "the address did not descend into the card"
    );

    assert_eq!(
        press(&mut view, &panel, Press::Enter),
        Asked::Listened(AudioAsk::Operation(Operation::AttachBeatSource {
            source: BeatSource::AudioInput("MacBook Pro Microphone".to_owned()),
        })),
        "`enter` on a row of the card did not attach the input it names"
    );
    assert_eq!(
        at(&view, "transport"),
        vec![7],
        "the address was left inside a card the press takes away"
    );
}

/// The arrangement menu is *save*, *start a new one* and the names filed, in that
/// order — `enter` on the first saves under the name in use and asks for one where
/// there is none, `enter` on one of the names puts that arrangement back, and the
/// reset declines and names the key that reaches it (ADR-0350).
#[test]
fn the_arrangement_menu_is_walked_and_enter_saves_or_puts_one_back() {
    let (panel, mut view) = transport();
    view.arrangement.name = Some("night".to_owned());
    walk_to(&mut view, &panel, "transport", ARRANGEMENT);
    let asked = press(&mut view, &panel, Press::Enter);
    assert_eq!(asked, Asked::Arranged(Ask::Open));
    put_the_card_down(&mut view, &asked);

    // `1` is *save*, and with a name in use saving again means that name.
    assert_eq!(press(&mut view, &panel, Press::Digit(1)), Asked::Moved);
    let asked = press(&mut view, &panel, Press::Enter);
    assert_eq!(
        asked,
        Asked::Arranged(Ask::Operation(Operation::SaveArrangement {
            name: "night".to_owned(),
        })),
    );
    assert_eq!(
        at(&view, "transport"),
        vec![8],
        "the address was left inside a menu the press takes away"
    );
    take_the_card_away(&mut view, &asked);

    // `2` is *start a new one*, which is the reset: `r` reaches that row and
    // the grammar declines rather than reaching it a second way.
    let asked = press(&mut view, &panel, Press::Enter);
    put_the_card_down(&mut view, &asked);
    press(&mut view, &panel, Press::Digit(2));
    match press(&mut view, &panel, Press::Enter) {
        Asked::Nothing(why) => assert!(
            why.contains('r'),
            "the reset row declined without naming the key that reaches it: {why}"
        ),
        other => panic!("the menu's reset was performed from the grammar: {other:?}"),
    }

    // `3` and `4` are the names filed, in the order the store listed them, and
    // the arrows are how the address moves between rows of a card it has
    // already descended into — a digit reaches nothing under a row.
    press(&mut view, &panel, Press::Arrow(Arrow::Down));
    press(&mut view, &panel, Press::Arrow(Arrow::Down));
    assert_eq!(at(&view, "transport"), vec![8, 4]);
    assert_eq!(
        press(&mut view, &panel, Press::Enter),
        Asked::Arranged(Ask::Operation(Operation::RestoreArrangement {
            name: "wide".to_owned(),
        })),
        "`enter` on a filed name did not put that arrangement back"
    );
}

/// With no arrangement in use, *save* asks for a name — the one flow on this
/// panel that takes letters, which the field then has the keyboard for.
#[test]
fn save_with_no_name_in_use_asks_for_one() {
    let (panel, mut view) = transport();
    assert!(view.arrangement.name.is_none());
    walk_to(&mut view, &panel, "transport", ARRANGEMENT);
    let asked = press(&mut view, &panel, Press::Enter);
    put_the_card_down(&mut view, &asked);
    press(&mut view, &panel, Press::Digit(1));
    assert_eq!(
        press(&mut view, &panel, Press::Enter),
        Asked::Arranged(Ask::Name),
        "*save* with no name in use did not ask for one"
    );
}

/// A card's rows are a column, so `↑↓` walk them and `←→` are refused with the
/// pair that works — and the walk starts from the pill with no digit pressed
/// first, which is the bay-level rule one rung down.
#[test]
fn a_cards_rows_are_a_column_the_arrows_walk() {
    let (panel, mut view) = transport();
    walk_to(&mut view, &panel, "transport", AUDIO_IN);
    let asked = press(&mut view, &panel, Press::Enter);
    put_the_card_down(&mut view, &asked);

    match press(&mut view, &panel, Press::Arrow(Arrow::Right)) {
        Asked::Nothing(why) => assert!(
            why.contains("column") && why.contains("up and down"),
            "`→` in a card declined without naming the axis that works: {why}"
        ),
        other => panic!("`→` walked a column of rows: {other:?}"),
    }
    assert_eq!(
        press(&mut view, &panel, Press::Arrow(Arrow::Down)),
        Asked::Moved
    );
    assert_eq!(at(&view, "transport"), vec![7, 2]);
    // Walked and clamped rather than wrapped, which is `View::walk`'s rule one
    // bay over.
    match press(&mut view, &panel, Press::Arrow(Arrow::Down)) {
        Asked::Nothing(why) => assert!(
            why.contains("end"),
            "a walk off the end of a card declined without saying so: {why}"
        ),
        other => panic!("the walk ran off the end of the card: {other:?}"),
    }
    assert_eq!(at(&view, "transport"), vec![7, 2]);
}

/// `Tab` takes the card away too: a card the address descends into goes when
/// focus moves.
#[test]
fn tab_takes_a_transport_card_away() {
    let (panel, mut view) = transport();
    walk_to(&mut view, &panel, "transport", AUDIO_IN);
    let asked = press(&mut view, &panel, Press::Enter);
    put_the_card_down(&mut view, &asked);
    assert!(view.audio.as_ref().expect("an audio pill").open());

    assert!(view.tab(&panel, 1), "`tab` did not move focus");
    assert!(
        !view.audio.as_ref().expect("an audio pill").open(),
        "`tab` left a card standing over a bay the keys have left"
    );
    // The bay keeps where it was, which is every other thing `Tab` leaves
    // alone.
    assert_eq!(at(&view, "transport"), vec![7]);
}

/// `esc` takes the card away and leaves the address on the pill it hangs from,
/// from inside the card and from the pill itself.
#[test]
fn esc_takes_a_transport_card_away_and_leaves_the_address_on_its_pill() {
    let (panel, mut view) = transport();
    walk_to(&mut view, &panel, "transport", ARRANGEMENT);
    let asked = press(&mut view, &panel, Press::Enter);
    put_the_card_down(&mut view, &asked);
    press(&mut view, &panel, Press::Digit(3));
    assert_eq!(at(&view, "transport"), vec![8, 3]);

    assert!(
        view.focus_up(&panel),
        "`esc` inside a card acted on nothing"
    );
    assert!(!view.arrangement.open(), "`esc` left the menu down");
    assert_eq!(
        at(&view, "transport"),
        vec![8],
        "`esc` did not leave the address on the pill"
    );

    // And from the pill itself, where the card is down and the address never
    // descended: the card goes and the address stays.
    let asked = press(&mut view, &panel, Press::Enter);
    put_the_card_down(&mut view, &asked);
    assert!(view.focus_up(&panel));
    assert!(
        !view.arrangement.open(),
        "`esc` on the pill left the menu down"
    );
    assert_eq!(at(&view, "transport"), vec![8]);

    // With no card down it is the ordinary climb again.
    assert!(view.focus_up(&panel));
    assert_eq!(at(&view, "transport"), Vec::<usize>::new());
}
