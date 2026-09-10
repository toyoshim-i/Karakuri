//! **The six keys inside a bay: what a digit names, what the arrows walk, what
//! `space` cycles and what `enter` performs.**
//!
//! [ADR-0259](../../../docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
//! designed the grammar and
//! [ADR-0333](../../../docs/adr/0333-the-console-resolves-the-address-and-the-window-loop-names-the-operation.md)
//! builds it in two bays. This holds the console's half: the address a press
//! moves and what it says the press landed on. That the window loop then names
//! the right operation is `crates/karakuri/src/main.rs`'s `key_column`, and
//! that a press on a running panel moves a deck is `mod gpu`'s.
//!
//! Six claims:
//!
//! 1. **A digit names the nth thing one level below the address and `0` the
//!    head**, counting what the bay drew from one.
//! 2. **Naming a strip is the deck selection**, which is why that row keeps a
//!    key badge rather than losing one — and it goes through `View::select`,
//!    so a deck the mixer draws no strip for is refused and the address does
//!    not descend.
//! 3. **The arrows take the neighbour along the axis the bay draws its items
//!    on**, and the next value of a level along the other.
//! 4. **`space` is the addressed thing's next state**, and it is the same cycle
//!    the chip walks — asked of the same three functions rather than restated.
//! 5. **`enter` is the act the addressed thing is for**, and it declines in a
//!    bay whose items perform nothing.
//! 6. **A refusal says why.** A key that declines and a key that is not bound
//!    are the same experience, so every `Nothing` carries a sentence.
//!
//! **No device and no `egui` pass.** A press is a walk of a path.

mod common;

use common::PLAUSIBLE;
use karakuri_console::focus::{self, Arrow, Asked, Control, Grammar, Level, Press, Step, HEAD};
use karakuri_console::panel::Panel;
use karakuri_console::room::Room;
use karakuri_console::view::{Mask, Scope, Strip, Tally, View};
use karakuri_operation::{BlendMode, Operation, Residency, Undecided, WipeKind};

/// The mixer's four strips, at the values this file steps from. Every one of
/// them differs from its neighbours in the three states, so a cycle that
/// answered from the wrong strip comes out wrong rather than right by luck.
fn strip(at: usize) -> Strip {
    Strip {
        name: format!("set {at}"),
        tally: Tally::Live,
        requested: [Tally::Live, Tally::Priming, Tally::Allocated, Tally::Live][at],
        gain: 1.0,
        gain_to: None,
        opacity: 1.0,
        opacity_to: None,
        blend: [
            BlendMode::Add,
            BlendMode::Over,
            BlendMode::Max,
            BlendMode::Add,
        ][at],
        mask: [Mask::None, Mask::Linear, Mask::Radial, Mask::None][at],
        mask_angle: 0.25,
        level: None,
    }
}

fn console() -> (Panel, View) {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let mut view = View::new(Room::Day);
    view.mixer = (0..4).map(strip).collect();
    view.library = ["one", "two", "three"]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    view.scopes = Scope::ALL.to_vec();
    (panel, view)
}

/// **What the deck is holding**, answered off the strips this file built —
/// which is what `crates/karakuri` reads off the *deck* at the press. Here they
/// are the same values, and which of the two a console reads is that file's
/// question rather than this one's.
fn holding(view: &View, deck: u8) -> Option<focus::Held> {
    let strip = view.mixer.get(usize::from(deck))?;
    Some(focus::Held {
        requested: strip.requested,
        blend: strip.blend,
        mask: strip.mask,
        mask_angle: strip.mask_angle,
    })
}

/// Put focus on `bay`, in at most one turn of the ring — bounded for
/// `tests/focus.rs`'s reason: a bay a walk cannot reach is a hang and not a
/// failure.
fn focus_on(view: &mut View, panel: &Panel, bay: &str) {
    for _ in 0..10 {
        if view.focused(panel).map(|found| found.name) == Some(bay) {
            return;
        }
        view.tab(panel, 1);
    }
    panic!("`Tab` did not reach `{bay}` in one turn of the ring");
}

/// One press, with the deck's own readings behind it.
fn press(view: &mut View, panel: &Panel, key: Press) -> Asked {
    let held: Vec<Option<focus::Held>> = (0..4).map(|deck| holding(view, deck)).collect();
    focus::press(view, panel, key, |deck| {
        held.get(usize::from(deck)).copied().flatten()
    })
}

fn at(view: &View, bay: &str) -> Vec<usize> {
    view.focus()
        .address(bay)
        .map_or_else(Vec::new, |address| address.at().to_vec())
}

// ---------------------------------------------------------------------------
// The table
// ---------------------------------------------------------------------------

/// **Which keys act in which bay is derived from what the bay is made of**, not
/// listed beside it — so this is the derivation held against the two bays the
/// record describes.
#[test]
fn the_dispatch_table_is_the_two_bays_the_record_builds() {
    let reaches = focus::reaches();
    assert_eq!(
        reaches,
        vec![
            ("mixer", Grammar::Digit),
            ("mixer", Grammar::Arrows),
            ("mixer", Grammar::Space),
            ("library", Grammar::Digit),
            ("library", Grammar::Arrows),
            ("library", Grammar::Space),
            ("library", Grammar::Enter),
        ],
        "the grammar the console declares is not the one ADR-0333 builds. `enter` reaches nothing \
         in the Mixer — that bay's acts are its transition row's, which is not addressable — and \
         all four reach the Library, whose rows are an act"
    );
    assert_eq!(
        focus::BUILT.len(),
        2,
        "two of the nine bays have the grammar; the seven that do not are absent from the table \
         rather than present and empty, because a row with nothing under it is a claim that the \
         six keys reach that bay"
    );
}

// ---------------------------------------------------------------------------
// A digit
// ---------------------------------------------------------------------------

#[test]
fn a_digit_names_the_nth_thing_below_the_address_and_zero_the_head() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    assert_eq!(at(&view, "mixer"), Vec::<usize>::new());

    // `2` is the second strip — deck B — and naming it is the deck selection.
    assert_eq!(
        press(&mut view, &panel, Press::Digit(2)),
        Asked::Emitted(Operation::SelectDeck { deck: 1 }),
        "a digit that names a strip did not name the deck selection with it. ADR-0259: *naming a \
         strip is the deck selection*, which is why that row keeps a key badge"
    );
    assert_eq!(at(&view, "mixer"), vec![2], "the address did not descend");
    assert_eq!(view.selection(), 1, "the selection did not move with it");

    // `3` under it is that strip's third control — the fader. `2 3` is deck B's
    // fader, which is the record's own example.
    assert_eq!(press(&mut view, &panel, Press::Digit(3)), Asked::Moved);
    assert_eq!(at(&view, "mixer"), vec![2, 3]);
    assert_eq!(
        press(&mut view, &panel, Press::Arrow(Arrow::Up)),
        Asked::Stepped {
            deck: 1,
            level: Level::Fader,
            step: Step::Up
        },
        "`2 3` is not deck B's fader — the digits count the controls a strip draws, in the order \
         it draws them: the tally, the trim, the fader, the blend chip and the mask mini"
    );
}

#[test]
fn a_digit_past_what_the_bay_drew_is_refused_and_the_address_stays_put() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    view.mixer.truncate(2);
    let refused = press(&mut view, &panel, Press::Digit(4));
    assert!(
        matches!(refused, Asked::Nothing(_)),
        "deck D was named at a two-strip mixer and was not refused: {refused:?}"
    );
    assert_eq!(
        at(&view, "mixer"),
        Vec::<usize>::new(),
        "the address descended onto a strip the mixer is not drawing. `View::select` is what \
         refuses a deck there is no strip for, and a press it turns down is a press that named \
         nothing"
    );
    assert_eq!(
        view.selection(),
        0,
        "the selection moved on a refused press"
    );
}

#[test]
fn zero_names_the_head_and_the_mixers_head_holds_nothing() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "library");
    assert_eq!(press(&mut view, &panel, Press::Digit(HEAD)), Asked::Moved);
    assert_eq!(at(&view, "library"), vec![HEAD]);

    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    let said = press(&mut view, &panel, Press::Digit(HEAD));
    assert!(
        matches!(said, Asked::Nothing(_)),
        "the Mixer's head draws no control and the press did not say so: {said:?}"
    );
    assert_eq!(
        at(&view, "mixer"),
        vec![HEAD],
        "the address did not reach the head. It is there and empty, which is a different thing \
         from a digit that named nothing — esc goes back"
    );
}

// ---------------------------------------------------------------------------
// The arrows
// ---------------------------------------------------------------------------

/// **The mixer's strips are a row and the library's rows are a column**, so
/// each bay answers one pair and declines the other — which is ADR-0259's *the
/// neighbour of the addressed thing, along the axis it is drawn on*.
#[test]
fn the_arrows_walk_a_bays_items_along_the_axis_it_draws_them_on() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    assert_eq!(
        press(&mut view, &panel, Press::Arrow(Arrow::Right)),
        Asked::Emitted(Operation::SelectDeck { deck: 1 }),
        "`→` did not walk the strips"
    );
    let up = press(&mut view, &panel, Press::Arrow(Arrow::Up));
    assert!(
        matches!(up, Asked::Nothing(_)),
        "`↑` walked a row of strips: {up:?}"
    );

    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "library");
    assert_eq!(
        press(&mut view, &panel, Press::Arrow(Arrow::Down)),
        Asked::Moved
    );
    assert_eq!(view.cursor_row(), 1, "`↓` did not walk the library's rows");
    let right = press(&mut view, &panel, Press::Arrow(Arrow::Right));
    assert!(
        matches!(right, Asked::Nothing(_)),
        "`→` walked a column of rows: {right:?}"
    );
}

/// **The walk starts from the item the bay remembers, at bay level**, which is
/// what keeps the two keys the Library already had (ADR-0333).
#[test]
fn the_arrows_walk_from_the_remembered_item_without_a_digit_first() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "library");
    assert_eq!(at(&view, "library"), Vec::<usize>::new());
    press(&mut view, &panel, Press::Arrow(Arrow::Down));
    press(&mut view, &panel, Press::Arrow(Arrow::Down));
    assert_eq!(
        view.cursor_row(),
        2,
        "two presses of `↓` with the address at the bay did not walk two rows. Before the \
         grammar these were `up` and `down` and needed nothing pressed first; a walk that \
         declined until a digit had been pressed would take that away"
    );
    assert_eq!(
        at(&view, "library"),
        Vec::<usize>::new(),
        "walking at bay level descended. The arrows move along a level and never into one"
    );
}

/// **A walk is not a cycle**, which is `View::walk`'s rule arriving at the row
/// of strips: a key held down must not jump the length of it.
#[test]
fn the_strips_are_walked_and_clamped_rather_than_wrapped() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    for _ in 0..8 {
        press(&mut view, &panel, Press::Arrow(Arrow::Right));
    }
    assert_eq!(
        view.selection(),
        3,
        "the walk did not stop at the last strip"
    );
    for _ in 0..8 {
        press(&mut view, &panel, Press::Arrow(Arrow::Left));
    }
    assert_eq!(view.selection(), 0, "the walk did not stop at the first");
}

/// **A level takes the arrows and a state does not.** A closed list has no
/// axis, so the arrows decline on the three chips and say why.
#[test]
fn the_arrows_step_a_level_and_decline_on_a_state() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    press(&mut view, &panel, Press::Digit(1));
    // The trim is a strip's second control, the tally its first.
    press(&mut view, &panel, Press::Digit(2));
    assert_eq!(
        press(&mut view, &panel, Press::Arrow(Arrow::Down)),
        Asked::Stepped {
            deck: 0,
            level: Level::Trim,
            step: Step::Down
        }
    );
    assert!(view.focus().address("mixer").map(|a| a.at()) == Some(&[1, 2][..]));

    // Back up to the strip and onto the tally, which is a state.
    view.focus_up(&panel);
    press(&mut view, &panel, Press::Digit(1));
    let refused = press(&mut view, &panel, Press::Arrow(Arrow::Up));
    assert!(
        matches!(refused, Asked::Nothing(_)),
        "an arrow stepped a residency, whose three values are a closed list rather than a \
         continuum: {refused:?}"
    );
}

// ---------------------------------------------------------------------------
// `space`
// ---------------------------------------------------------------------------

/// **The key and the chip are one cycle.** The three states are asked of the
/// same three functions `view::Mixer`'s chips ask, so a press and a click
/// cannot disagree about which state comes next.
#[test]
fn space_on_a_state_names_the_state_the_chip_would_name() {
    let cases: [(usize, Operation); 3] = [
        (
            1,
            // Deck B's requested residency is `priming`, so the next is
            // `allocated` — which is the withdrawal of the prime request, said
            // by the ordinary arithmetic.
            Operation::SetResidency {
                deck: 1,
                residency: Residency::Allocated,
            },
        ),
        (
            4,
            Operation::SetBlendMode {
                deck: 1,
                blend: BlendMode::Max,
            },
        ),
        (
            5,
            Operation::SetMaskShape {
                deck: 1,
                kind: WipeKind::Radial,
                angle: 0.25,
            },
        ),
    ];
    for (control, want) in cases {
        let (panel, mut view) = console();
        focus_on(&mut view, &panel, "mixer");
        press(&mut view, &panel, Press::Digit(2));
        press(&mut view, &panel, Press::Digit(control));
        assert_eq!(
            press(&mut view, &panel, Press::Space),
            Asked::Emitted(want.clone()),
            "`space` on deck B's control {control} did not ask for `{want:?}`"
        );
    }
}

/// **On a level the one state worth naming is the value it was declared at**,
/// which is the clause ADR-0259 buys with an argument rather than finds.
#[test]
fn space_on_a_level_is_the_value_it_was_declared_at() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    press(&mut view, &panel, Press::Digit(1));
    for (control, level) in [(2, Level::Trim), (3, Level::Fader)] {
        press(&mut view, &panel, Press::Digit(control));
        assert_eq!(
            press(&mut view, &panel, Press::Space),
            Asked::Stepped {
                deck: 0,
                level,
                step: Step::Default
            }
        );
        view.focus_up(&panel);
    }
}

#[test]
fn space_on_the_librarys_head_is_the_scope() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "library");
    press(&mut view, &panel, Press::Digit(HEAD));
    press(&mut view, &panel, Press::Digit(1));
    assert_eq!(
        press(&mut view, &panel, Press::Space),
        Asked::Scope,
        "`0 1 space` in the Library is not the scope. ADR-0259: *`0` is the head and its controls \
         are the scope chips, so `space` there cycles the scope*"
    );
}

/// **`space` on a bay is the fold and is not bound**, which is ADR-0333's own
/// clause: folding is a rule about all nine bays and binding it in two would
/// put a badge on *Fold a bay away* naming two of the nine places it works.
#[test]
fn space_on_a_bay_says_the_fold_is_not_bound() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    let said = press(&mut view, &panel, Press::Space);
    match said {
        Asked::Nothing(why) => assert!(
            why.contains("fold"),
            "`space` on a bay declined without saying the fold is what it is for: {why}"
        ),
        other => panic!("`space` on a bay did something: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// `enter`
// ---------------------------------------------------------------------------

#[test]
fn enter_on_a_library_row_is_the_load_and_on_a_strip_is_nothing() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "library");
    press(&mut view, &panel, Press::Digit(2));
    assert_eq!(
        press(&mut view, &panel, Press::Enter),
        Asked::Load,
        "`enter` on a library row is not the load"
    );
    assert_eq!(
        view.cursor_row(),
        1,
        "the digit that named the row did not move the cursor the load reads"
    );

    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "mixer");
    press(&mut view, &panel, Press::Digit(1));
    let refused = press(&mut view, &panel, Press::Enter);
    assert!(
        matches!(refused, Asked::Nothing(_)),
        "`enter` performed something on a strip. ADR-0259's walk: *enter reaches nothing in \
         Mixer* — a strip's five controls all set rather than perform: {refused:?}"
    );
}

// ---------------------------------------------------------------------------
// The refusals, and the bays that have no grammar
// ---------------------------------------------------------------------------

/// **Every refusal carries the sentence that says why**, which is P-0083 and
/// the page's own rule that a key that declines and a key that is not bound are
/// the same experience.
#[test]
fn every_refusal_says_why() {
    let (panel, mut view) = console();
    focus_on(&mut view, &panel, "transport");
    for key in [
        Press::Digit(1),
        Press::Arrow(Arrow::Up),
        Press::Space,
        Press::Enter,
    ] {
        match press(&mut view, &panel, key) {
            Asked::Nothing(why) => {
                assert!(
                    why.len() > 20 && why.contains("mixer"),
                    "a bay with no grammar declined `{key:?}` without saying where the six keys \
                     are: {why}"
                );
            }
            other => panic!(
                "the Transport answered `{key:?}` with {other:?}, and its grammar is not built"
            ),
        }
    }
}

/// **An address on something the bay has stopped drawing goes back to the
/// bay**, rather than acting on whatever has taken that position — which is
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

    // **And the same one rung up**, where the path names an item and no control
    // under it: a strip that has gone is a strip that has gone at either depth,
    // and clamping onto the nearest one would put a press on a deck nobody
    // addressed.
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
fn the_two_built_bays_are_made_of_what_the_record_walks() {
    let mixer = focus::built("mixer").expect("the mixer's grammar");
    assert_eq!(
        mixer.item,
        &[
            Control::Tally,
            Control::Trim,
            Control::Fader,
            Control::Blend,
            Control::Mask
        ],
        "a strip's controls are not the five the bay draws, in the order it draws them"
    );
    assert!(mixer.head.is_empty() && mixer.selects && mixer.across);
    let library = focus::built("library").expect("the library's grammar");
    assert_eq!(library.head, &[Control::Scope]);
    assert!(library.item.is_empty() && !library.selects && !library.across);
    assert!(library.act.is_some(), "a library row is an act");
    // `Undecided` is what a scope step carries, and it is named here so that a
    // payload that grew a value fails against the page rather than silently.
    assert_eq!(
        Operation::SelectScope { scope: Undecided },
        Operation::SelectScope { scope: Undecided }
    );
}
