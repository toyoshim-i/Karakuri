//! The pulldown on an Inspector pane's head: *point this pane at another deck*
//! —
//! [ADR-0338](../../../docs/adr/0338-a-procedure-is-a-row-of-the-library-and-one-loaded-over-a-layer-makes-a-set-with-no-name.md),
//! decision 5.
//!
//! The mock has drawn the `▾` between `deck A · drift_night` and the count
//! since the Inspector was drawn, and
//! [ADR-0292](../../../docs/adr/0292-the-pane-heads-name-takes-letters-and-the-keep-capsule-stays-a-stamp.md)
//! reserved its rectangle without painting it. This is that rectangle made
//! live.
//!
//! Nine things:
//!
//! 1. Where the mark is: one `.half-head` gap after the run, at the same
//! measure every other `▾` on this console is drawn at — and that the run now
//! stops short of it rather than over it. 2. That the card offers the decks the
//! mixer is drawing strips for and no others, which is `View::select`'s refusal
//! read a fourth time rather than a fourth rule. 3. That a pick emits
//! `PointPane` naming that pane and the deck the row was on. 4. That the pick
//! moves that pane and no other pointer — not the deck selection, not the pane
//! next door, not the Library bay's load target. That is the whole of what this
//! mark is for. 5. That a deck the mixer draws no strip for is refused rather
//! than clamped. 6. That one card is down at a time, and that a pick puts it
//! away. 7. That a console which has not drawn has no mark.
//!
//! What is not here and cannot be: that `input::claim` gives the panel a press
//! on the mark and every press while the card is down, and that the window
//! re-reads the pane the pick moved. Those are `input::PROBES`' row, `claim`'s
//! rule 2 and `crates/karakuri/src/main.rs`'s `pointed_pane`.

mod common;

use common::{drawn_once, near, PLAUSIBLE};
use karakuri_console::panel::Panel;
use karakuri_console::room::{size, Room};
use karakuri_console::view::{
    deck_name, inspector, to_egui, Mask, Pane, Strip, Tally, View, DECK_LETTERS, PANES, PANE_NAMES,
    SYNCS,
};
use karakuri_layout::Point;
use karakuri_operation::{BlendMode, Operation, Sync};

/// One mixer strip, which is what makes a deck a row of this card.
fn strip(name: &str) -> Strip {
    Strip {
        name: name.to_owned(),
        tally: Tally::Allocated,
        requested: Tally::Allocated,
        gain: 0.44,
        gain_to: None,
        opacity: 0.30,
        opacity_to: None,
        blend: BlendMode::Over,
        mask: Mask::Linear,
        mask_angle: 0.0,
        level: None,
    }
}

/// A pane showing `deck`, with no groups in it — this file is about the head.
fn pane(deck: usize, material: &str) -> Pane {
    Pane {
        deck,
        material: material.to_owned(),
        sync: Sync::Tempo,
        allows: [true; SYNCS.len()],
        anchor_bpm: 128.0,
        scrub_beats: 0.0,
        composite: true,
        aimed: None,
        nodes: Vec::new(),
    }
}

/// A console with `decks` strips and two panes on the first two of them, which
/// is what a run opens with.
fn console(decks: usize) -> (Panel, egui::Context, View) {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let mut view = View::new(Room::Day);
    view.mixer = (0..decks).map(|d| strip(DECK_LETTERS[d])).collect();
    view.inspector = (0..PANES.min(decks))
        .map(|d| pane(d, DECK_LETTERS[d]))
        .collect();
    (panel, drawn_once(), view)
}

fn at(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

/// The pulldown in pane `index`'s head, as the paint and the press both ask for
/// it.
fn pulldown(
    panel: &Panel,
    ctx: &egui::Context,
    view: &View,
    index: usize,
) -> karakuri_console::view::PaneTarget {
    let pane = &view.inspector[index];
    let laid = inspector(panel.layout(), index, pane, view.scroll_in(index))
        .expect("a pane with room in it");
    view.pane_pulldown(ctx, &laid, pane, index)
        .expect("a head with room for the mark")
}

/// The card's room, which is the whole viewport.
fn room(panel: &Panel) -> egui::Rect {
    to_egui(panel.layout().viewport())
}

// ---------------------------------------------------------------------------
// Where the control is
// ---------------------------------------------------------------------------

/// One `.half-head` gap after the run, and the run stops short of it.
///
/// The second half is what changed when the mark became a control: the run was
/// clipped to whatever was next along the row, so its rectangle and the
/// chevron's could overlap the count in a narrow head. The chevron's room now
/// comes off the run's limit, which is `.half-head`'s own order — `showing`,
/// the run, `▾`, `.sep`, the count, `keep`.
#[test]
fn the_mark_sits_one_gap_after_the_run() {
    let (panel, ctx, view) = console(4);
    for index in 0..PANES {
        let pane = &view.inspector[index];
        let laid = inspector(panel.layout(), index, pane, 0.0).expect("a pane with room in it");
        let named = deck_name(&ctx, &laid, pane, None).expect("a head with room for the run");
        let target = pulldown(&panel, &ctx, &view, index);
        assert_eq!(target.chevron, named.chevron);
        assert!(
            near(target.chevron.min.x, named.name.max.x + size::HALF_HEAD_GAP),
            "the mark starts at {} and the run ends one gap before {}",
            target.chevron.min.x,
            named.name.max.x + size::HALF_HEAD_GAP
        );
        assert!(
            laid.head.contains_rect(target.chevron),
            "the mark is not inside the head it is drawn in"
        );
        assert_eq!(target.pane, index);
    }
}

/// Before the first frame there is nothing here to press, which is
/// `deck_name`'s guard read through: the mark hangs off the run and there is no
/// run until a pass has laid one out.
#[test]
fn a_console_that_has_not_drawn_has_no_mark() {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let mut view = View::new(Room::Day);
    view.mixer = vec![strip("A")];
    view.inspector = vec![pane(0, "drift_night")];
    let laid = inspector(panel.layout(), 0, &view.inspector[0], 0.0).expect("a pane");
    let ctx = egui::Context::default();
    assert!(view
        .pane_pulldown(&ctx, &laid, &view.inspector[0], 0)
        .is_none());
}

// ---------------------------------------------------------------------------
// What the card offers
// ---------------------------------------------------------------------------

/// The card is shut until a press opens it, and a shut card has no rows — which
/// is what stops `PaneTarget::row` handing out a rectangle for a list nobody
/// opened.
#[test]
fn a_shut_card_offers_nothing() {
    let (panel, ctx, view) = console(4);
    let target = pulldown(&panel, &ctx, &view, 0);
    assert_eq!(target.rows, 0);
    assert_eq!(target.list(room(&panel)), None);
}

/// It offers the decks the mixer is drawing strips for and no others.
///
/// `View::select`'s refusal read a fourth time rather than a fourth rule: a
/// deck with no strip is a deck a pane pointed at it would draw nothing for.
#[test]
fn the_card_offers_only_the_decks_the_mixer_draws() {
    for decks in 1..=DECK_LETTERS.len() {
        let (panel, ctx, mut view) = console(decks);
        assert!(view.open_pane_target(0), "the card did not come down");
        let target = pulldown(&panel, &ctx, &view, 0);
        assert_eq!(
            target.rows, decks,
            "a console drawing {decks} strips offered {} rows",
            target.rows
        );
        let card = target.list(room(&panel)).expect("a card with rows in it");
        // Every row is inside the card, and a point below the last one is on
        // no row at all.
        for index in 0..decks {
            let row = target.row(card, index);
            assert!(card.contains_rect(row), "row {index} is outside the card");
            assert_eq!(
                target.picked(room(&panel), at(row.center())),
                Some(Operation::PointPane {
                    pane: PANE_NAMES[0].to_owned(),
                    deck: index as u8,
                }),
                "row {index} named a different deck than the one it is drawn for"
            );
        }
        assert_eq!(
            target.picked(
                room(&panel),
                at(egui::Pos2::new(card.center().x, card.max.y + 4.0))
            ),
            None,
            "a point under the card was answered with a deck"
        );
    }
}

// ---------------------------------------------------------------------------
// What a pick moves
// ---------------------------------------------------------------------------

/// A pick moves that pane and no other pointer: not the deck selection, not the
/// pane next door, not the Library bay's load target.
#[test]
fn a_pick_moves_that_pane_and_nothing_else() {
    let (_panel, _ctx, mut view) = console(4);
    let selection = view.selection();
    let load = view.target_deck();
    let other = view.pane_deck(1);
    assert!(view.point_pane(0, 2), "the pick did not move the pane");
    assert_eq!(view.pane_deck(0), 2, "the pane is not showing deck C");
    assert_eq!(
        view.selection(),
        selection,
        "the pick moved the deck selection"
    );
    assert_eq!(
        view.target_deck(),
        load,
        "the pick moved the Library bay's load target"
    );
    assert_eq!(
        view.pane_deck(1),
        other,
        "the pick moved the pane next door"
    );
}

/// Two panes are two answers, which is what makes slots C and D reachable at
/// all: a run with four decks can show any two of them.
#[test]
fn the_two_panes_are_pointed_apart() {
    let (_panel, _ctx, mut view) = console(4);
    assert!(view.point_pane(0, 2));
    assert!(view.point_pane(1, 3));
    assert_eq!([view.pane_deck(0), view.pane_deck(1)], [2, 3]);
}

/// A deck the mixer draws no strip for is refused rather than clamped, which is
/// `View::select`'s and `View::aim_at`'s rule read a third time: a pick of deck
/// D on a two-slot deck means *deck D*, and clamping would point the pane at
/// deck B.
#[test]
fn a_deck_with_no_strip_is_refused() {
    let (_panel, _ctx, mut view) = console(2);
    let was = view.pane_deck(0);
    assert!(!view.point_pane(0, 3), "deck D was accepted on two strips");
    assert_eq!(
        view.pane_deck(0),
        was,
        "a refused pick moved the pane anyway"
    );
}

/// A pane index past the panes there are is refused too, and it is a caller's
/// error rather than a state.
#[test]
fn a_pane_that_is_not_there_is_refused() {
    let (_panel, _ctx, mut view) = console(4);
    assert!(!view.point_pane(PANES, 1));
    assert!(!view.open_pane_target(PANES));
    assert_eq!(view.pane_target_open(), None);
}

// ---------------------------------------------------------------------------
// One card at a time
// ---------------------------------------------------------------------------

/// One card is down at a time, and a pick puts it away, which is
/// `View::aim_at`'s clause: a pick is one gesture, and a card left down would
/// go on claiming every press on the console.
#[test]
fn one_card_is_down_and_a_pick_puts_it_away() {
    let (_panel, _ctx, mut view) = console(4);
    assert!(view.open_pane_target(0));
    assert_eq!(view.pane_target_open(), Some(0));
    assert!(
        view.open_pane_target(1),
        "the second head's card did not open"
    );
    assert_eq!(
        view.pane_target_open(),
        Some(1),
        "two cards were down at once"
    );
    assert!(view.point_pane(1, 0));
    assert_eq!(
        view.pane_target_open(),
        None,
        "the card is still down after a pick"
    );
    // **Put away even where the pane did not move** — picking the deck a pane
    // already shows is still a hand finishing what it started.
    assert!(view.open_pane_target(1));
    assert!(view.point_pane(1, 0), "the pick reported nothing moved");
    assert_eq!(view.pane_target_open(), None);
    assert!(!view.shut_pane_target(), "there was a card to shut");
}
