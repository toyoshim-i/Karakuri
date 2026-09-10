//! **Focus: the ring `Tab` walks, the level `esc` leaves, and the one field
//! the console's three pointers now are.**
//!
//! [ADR-0259](../../../docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
//! decided the keyboard and built none of it;
//! [ADR-0332](../../../docs/adr/0332-focus-is-a-pointer-the-console-owns-and-the-three-pointers-are-instances-of-it.md)
//! is the first slice, and this is what holds it. Five claims:
//!
//! 1. **The ring is the arrangement's own walk** — down a column, then across —
//!    and it holds every bay and only the bays. `view::REGIONS` is what it is
//!    checked against rather than what it is derived from, which is the
//!    record's own instruction, so this file is where the constant does the
//!    checking.
//! 2. **`Tab` walks it forward and wraps; `shift-Tab` is that walk run
//!    backwards and nothing else.** The two have to agree about where they are,
//!    which is what a round trip asserts and what a *second rule* for the
//!    backward direction would have cost.
//! 3. **A folded bay is visited.** It is in the ring so that there is something
//!    to press to open it, not so that it can be operated — the narrow reason,
//!    and the one that decides this test.
//! 4. **`esc` leaves a level and stops at the bay.** There is no unfocused
//!    state to fall out into, so the key acts on nothing there and says so.
//!    That it does not quit is the window loop's half and is
//!    `karakuri/src/main.rs`'s `focus_keys`.
//! 5. **The deck selection, the library cursor and the marked scope are three
//!    readings of one field.** The test that says so moves one of them through
//!    the shared path — `Focus`, not `View::select` — and reads it back off the
//!    method that is supposed to be a reading of it.
//!
//! **No device and no `egui` pass**, which is the whole of this crate: the ring
//! is a walk of a tree and the mark is a rectangle.

mod common;

use common::PLAUSIBLE;
use karakuri_console::focus::{ring, Address, Focus, HEAD, LIBRARY, MIXER};
use karakuri_console::panel::Panel;
use karakuri_console::room::{size, Room};
use karakuri_console::view::{region, Mask, Scope, Strip, Tally, View, REGIONS};
use karakuri_operation::BlendMode;

/// **The nine bays, in the order the arrangement's own walk reaches them** —
/// the transport, then the left pane down, then the centre's, then the right
/// pane's, then the outputs row.
///
/// Written out rather than derived, because deriving it is what is under test.
/// It is the same order `REGIONS` is written in less the four regions that are
/// not bays, which is what `every_bay_and_only_the_bays_are_in_the_ring`
/// asserts separately — so a reordering of the arrangement fails here with the
/// order it now has, and a region that stopped being a bay fails there.
const WALK: &[&str] = &[
    "transport",
    "library",
    "staging",
    "program",
    "inspector",
    "mixer",
    "master",
    "sequencer",
    "outputs",
];

fn panel() -> Panel {
    Panel::new(PLAUSIBLE.w, PLAUSIBLE.h)
}

/// A console with four strips, which is the count `View::select` refuses a deck
/// against — its rule is the mixer's own length, and nothing in this file is
/// about what a strip draws.
fn four_strips(view: &mut View) {
    view.mixer.clear();
    for _ in 0..4 {
        view.mixer.push(Strip {
            name: String::from("a set"),
            tally: Tally::Live,
            requested: Tally::Live,
            gain: 1.0,
            gain_to: None,
            opacity: 1.0,
            opacity_to: None,
            blend: BlendMode::Add,
            mask: Mask::None,
            mask_angle: 0.0,
            level: None,
        });
    }
}

fn named(bays: &[&'static karakuri_console::view::Region]) -> Vec<&'static str> {
    bays.iter().map(|bay| bay.name).collect()
}

#[test]
fn the_ring_is_the_arrangements_own_walk_down_a_column_then_across() {
    let panel = panel();
    assert_eq!(
        named(&ring(panel.layout())),
        WALK,
        "the tab ring is not the arrangement's walk. ADR-0259: a column's \
         children are taken top to bottom, a row's left to right, and the walk \
         stops at a bay rather than descending into it — so the transport, the \
         left pane's two, the centre's two, the right pane's three and the \
         outputs row, in that order. A scanline over the three panes is the \
         alternative that record rejected"
    );
}

/// **The ring against the constant**, which is the direction ADR-0259 asks for:
/// *"a ring derived from the solved tree is the honest implementation and the
/// constant is a thing to check against, not the source"*.
#[test]
fn every_bay_and_only_the_bays_are_in_the_ring() {
    let panel = panel();
    let walked = named(&ring(panel.layout()));
    let listed: Vec<&str> = REGIONS
        .iter()
        .filter(|region| karakuri_console::focus::is_bay(region.kind))
        .map(|region| region.name)
        .collect();
    assert_eq!(
        walked, listed,
        "the ring and `REGIONS` disagree about which regions are bays, or about \
         their order. `REGIONS`' own doc says the order is the arrangement's, \
         top to bottom and left to right, so the two are supposed to be the \
         same reading twice — and the four regions that are not bays (the \
         picture, the preview row and the two inspector panes) are a bay's \
         *items*, reached by a digit and never by `Tab`"
    );
    assert_eq!(
        walked.len(),
        9,
        "the manual's *What each region is standing on* lists nine bays and the \
         ring holds {}",
        walked.len()
    );
}

#[test]
fn tab_walks_the_ring_forward_and_wraps() {
    let panel = panel();
    let mut focus = Focus::default();
    let mut seen = vec![
        focus
            .bay(panel.layout())
            .expect("this arrangement has bays")
            .name,
    ];
    for _ in 1..WALK.len() {
        assert!(focus.tab(panel.layout(), 1), "`Tab` moved nothing");
        seen.push(focus.bay(panel.layout()).expect("still in the ring").name);
    }
    assert_eq!(
        seen, WALK,
        "`Tab` from where focus starts does not walk the ring in its order. \
         Focus starts on the first bay the traversal reaches — the rule rather \
         than the bay, so that an operator who has moved the transport has \
         moved the starting position with it"
    );
    assert!(focus.tab(panel.layout(), 1), "the last `Tab` moved nothing");
    assert_eq!(
        focus.bay(panel.layout()).map(|bay| bay.name),
        Some(WALK[0]),
        "`Tab` off the end of the ring did not wrap to its start — nine bays is \
         a ring and not a list with an end"
    );
}

#[test]
fn shift_tab_is_that_walk_run_backwards() {
    let panel = panel();
    let mut focus = Focus::default();
    let mut seen = vec![focus.bay(panel.layout()).expect("bays").name];
    for _ in 1..WALK.len() {
        assert!(focus.tab(panel.layout(), -1), "`shift-Tab` moved nothing");
        seen.push(focus.bay(panel.layout()).expect("bays").name);
    }
    let mut backwards: Vec<&str> = WALK.to_vec();
    backwards[1..].reverse();
    assert_eq!(
        seen, backwards,
        "`shift-Tab` is not the forward walk reversed. From the top of a column \
         it goes to the **bottom** of the one before, not to its top — the other \
         reading is the one that would make the two keys disagree about where \
         they are"
    );
}

/// The round trip, which is the whole of why the backward key is kept: *"an
/// operator who overshoots gets back exactly where they were"*.
#[test]
fn a_tab_and_a_shift_tab_come_back_to_the_same_bay() {
    let panel = panel();
    let mut focus = Focus::default();
    for _ in 0..WALK.len() {
        let was = focus.bay(panel.layout()).expect("bays").name;
        focus.tab(panel.layout(), 1);
        focus.tab(panel.layout(), -1);
        assert_eq!(
            focus.bay(panel.layout()).map(|bay| bay.name),
            Some(was),
            "a `Tab` and a `shift-Tab` did not cancel"
        );
        focus.tab(panel.layout(), 1);
    }
}

/// **A folded bay stays in the ring.**
///
/// The Master bay is the one `console.html` draws the folded mark on, so it is
/// the one folded here. It is in the ring so that there is something to press
/// to open it — not so that it can be operated, which is the narrow reading
/// ADR-0259 took and the one that keeps this test from being about a bay's
/// contents.
#[test]
fn a_folded_bay_is_still_in_the_ring() {
    let mut panel = panel();
    let master = panel.layout().find("master").expect("the master bay");
    panel.layout_mut().collapse(master);
    panel.solve();
    assert!(
        !panel.layout().visible(master),
        "the fold did not take — everything below is asserting nothing"
    );
    assert_eq!(
        named(&ring(panel.layout())),
        WALK,
        "a folded bay left the tab ring. It is in the ring so that there is \
         something to press to open it: a bay a key cannot reach is a bay that \
         can only be unfolded with a pointer, and `z` unfolds everything"
    );

    // **And `Tab` reaches it**, in at most one turn of the ring. The bound is
    // the assertion and not a guard: a walk that does not reach every bay in
    // `WALK.len()` presses is a ring with a bay missing from it or a `Tab` that
    // has stopped moving, and a `while` here would hang on either rather than
    // say which.
    let mut focus = Focus::default();
    let reached = (0..WALK.len()).any(|at| {
        if at > 0 {
            focus.tab(panel.layout(), 1);
        }
        focus.bay(panel.layout()).map(|bay| bay.name) == Some("master")
    });
    assert!(
        reached,
        "`Tab` did not reach the folded bay in {} presses, which is one turn of \
         the ring",
        WALK.len()
    );
}

/// **A folded pane's bays stay in the ring too**, which is the same rule one
/// level up and is the case that says the walk is over the tree and not over
/// what is on screen.
#[test]
fn a_folded_pane_keeps_its_bays_in_the_ring() {
    let mut panel = panel();
    let pane = panel.layout().find("left-pane").expect("the left pane");
    panel.layout_mut().collapse(pane);
    panel.solve();
    assert_eq!(
        named(&ring(panel.layout())),
        WALK,
        "folding the left pane took the library and the staging lane out of the \
         ring. The walk is the arrangement's tree and not the rectangles it \
         solved to — `Layout::children` and never `placed_children`"
    );
}

#[test]
fn esc_leaves_a_level_and_stops_at_the_bay() {
    let panel = panel();
    let mut focus = Focus::default();
    // Deck B's fader: `2 3` in the Mixer, which is ADR-0259's own example.
    focus.put(panel.layout(), MIXER);
    let address = focus.address_mut(MIXER);
    address.down(2);
    address.down(3);
    assert_eq!(focus.address(MIXER).map(Address::at), Some(&[2, 3][..]));

    assert!(focus.up(panel.layout()), "`esc` did not leave the control");
    assert_eq!(focus.address(MIXER).map(Address::at), Some(&[2][..]));
    assert!(focus.up(panel.layout()), "`esc` did not leave the item");
    assert_eq!(focus.address(MIXER).map(Address::at), Some(&[][..]));
    assert!(
        !focus.up(panel.layout()),
        "`esc` answered *moved* at bay level. There is no rung below the bay and \
         no unfocused state to fall out into, so it acts on nothing — and the \
         caller is what says so, because a key that declines silently and a key \
         that is not bound are the same experience"
    );
    assert_eq!(
        focus.bay(panel.layout()).map(|bay| bay.name),
        Some(MIXER),
        "`esc` left the ring. It goes up one level and no further"
    );
}

/// **`esc` at the bay leaves the remembered address alone**, which is the pair
/// of fields earning their keep: the address `esc` popped is where the ring is
/// drawn, and what the bay *remembers* is what the deck selection is.
#[test]
fn esc_does_not_forget_where_the_bay_was() {
    let panel = panel();
    let mut focus = Focus::default();
    focus.put(panel.layout(), MIXER);
    focus.address_mut(MIXER).down(2);
    // Up to the bay, and then one more press that answers *nothing to leave*.
    assert!(focus.up(panel.layout()));
    assert!(!focus.up(panel.layout()));
    assert_eq!(
        focus.address(MIXER).and_then(|a| a.remembered(&[])),
        Some(2),
        "`esc` took the bay's remembered address with it. The dashed ring is \
         where the address is now and the solid one is where it was, which is \
         why they are two fields"
    );
}

#[test]
fn the_ring_is_drawn_on_the_focused_head_and_on_no_other() {
    let mut panel = panel();
    panel.solve();
    let mut view = View::new(Room::Day);

    let head = view.focus_mark(&panel).expect("a bay to mark");
    let transport = panel.layout().find("transport").expect("the transport");
    assert_eq!(
        view.focused(&panel).map(|bay| bay.name),
        Some("transport"),
        "focus does not start where the traversal starts"
    );
    // The transport is headless, so the row itself stands in for the head —
    // ADR-0259 reads its `0` the same way.
    assert_eq!(
        head.height(),
        panel.layout().rect(transport).h,
        "the mark on a headless bay is not the whole row. The transport and the \
         Outputs row draw no head (ADR-0159), and ADR-0259 says `0` names the \
         row itself there — so the row is what stands in for the head"
    );

    view.tab(&panel, 1);
    let library = panel.layout().find("library").expect("the library");
    let mark = view.focus_mark(&panel).expect("a bay to mark");
    assert_eq!(
        mark.height(),
        size::HEAD_H,
        "the mark on a bay that draws a head is not its head. `console.html` \
         puts the dashed ring on the bay head and nowhere else"
    );
    assert_eq!(
        mark.min.y,
        panel.layout().rect(library).y,
        "the mark is not at the top of the bay it is on"
    );
    assert_ne!(
        mark, head,
        "the mark did not move with focus — one ring, on the bay that has it"
    );
}

/// **A folded bay has no rectangle, so nothing is drawn** — which is the
/// drawing ADR-0259 leaves open and `console.html` carries beside its note.
#[test]
fn a_folded_bay_takes_focus_and_is_not_ringed() {
    let mut panel = panel();
    let master = panel.layout().find("master").expect("the master bay");
    panel.layout_mut().collapse(master);
    panel.solve();
    let mut view = View::new(Room::Day);
    // Bounded at one turn of the ring, for `a_folded_bay_is_still_in_the_ring`'s
    // reason: a bay a walk cannot reach is what this file asserts against, and a
    // `while` would hang instead of failing.
    let reached = (0..WALK.len()).any(|at| {
        if at > 0 {
            view.tab(&panel, 1);
        }
        view.focused(&panel).map(|bay| bay.name) == Some("master")
    });
    assert!(
        reached,
        "`Tab` did not reach the folded bay in {} presses",
        WALK.len()
    );
    assert!(
        view.focus_mark(&panel).is_none(),
        "a ring was drawn on a folded bay. A folded region has no rectangle and \
         no divider beside it (ADR-0204), so there is nothing on the panel for a \
         ring to sit on; the mark that is owed is the head alone, and \
         `console.html` draws it beside the note that defines it"
    );
}

/// **The three pointers are three readings of one field.**
///
/// Each of them is moved through the method that is its only writer and read
/// back out of the **shared** path — `View::focus`, the one field they are all
/// in — and then out of its own reader. Two directions of one claim: the
/// storage is one thing, and what each of the three *means* is unchanged.
#[test]
fn the_three_pointers_are_one_mechanism() {
    let mut view = View::new(Room::Day);
    four_strips(&mut view);
    view.library = ["a", "b", "c"].iter().map(|s| (*s).to_owned()).collect();
    view.scopes = Scope::ALL.to_vec();

    assert_eq!(view.selection(), 0, "the mixer does not start on deck A");
    assert_eq!(view.cursor_row(), 0, "the library does not start on row 1");
    assert_eq!(
        view.scope(),
        Some(Scope::ALL[0]),
        "the library does not start on the first chip"
    );

    // Digits count from one, because they count what the bay drew: deck D is
    // the Mixer's fourth item and the third library row is its third.
    assert!(view.select(3), "deck D was refused at a four-strip mixer");
    assert_eq!(
        view.focus().address(MIXER).and_then(|a| a.remembered(&[])),
        Some(4),
        "`View::select` did not write the Mixer bay's remembered item — the deck \
         selection is that bay's address and not a field of its own"
    );
    assert_eq!(view.selection(), 3, "and the reading did not come back");

    assert!(
        view.point_at(2),
        "row three was refused at a three-row listing"
    );
    assert_eq!(
        view.focus()
            .address(LIBRARY)
            .and_then(|a| a.remembered(&[])),
        Some(3),
        "`View::point_at` did not write the Library bay's remembered item"
    );
    assert_eq!(view.cursor_row(), 2, "and the reading did not come back");

    assert!(
        view.select_scope(Scope::ALL[2]),
        "the third chip was refused"
    );
    assert_eq!(
        view.focus()
            .address(LIBRARY)
            .and_then(|a| a.remembered(&[HEAD])),
        Some(3),
        "`View::select_scope` did not write the control remembered under the \
         Library's head. The scope chips are the bay's head — the controls that \
         are about the bay rather than about anything in it — which is how one \
         bay holds two of these"
    );
    assert_eq!(
        view.scope(),
        Some(Scope::ALL[2]),
        "and the reading did not come back"
    );

    // **And the deck selection survived the hands going into the library**,
    // which is the sentence this whole mechanism exists for.
    assert_eq!(view.selection(), 3);
}

/// **The Library's two do not collide**, which is the one thing a single path
/// per bay could not have carried: the cursor is which *item* the bay is on and
/// the scope is a control of its *head*, so moving one leaves the other alone.
#[test]
fn the_library_remembers_its_row_and_its_chip_separately() {
    let mut view = View::new(Room::Day);
    view.library = ["a", "b", "c"].iter().map(|s| (*s).to_owned()).collect();
    view.scopes = Scope::ALL.to_vec();
    assert!(view.point_at(2));
    assert!(view.step_scope());
    // A scope press puts the cursor back at the top on purpose — the listing is
    // about to be a different listing — so the row is checked before it.
    assert_eq!(view.cursor_row(), 0);
    assert!(view.point_at(1));
    assert_eq!(
        view.scope(),
        Some(Scope::ALL[1]),
        "moving the library cursor moved the marked scope with it. They are two \
         levels of one address — an item and a control of the head — and not one \
         number"
    );
    assert_eq!(view.cursor_row(), 1);
}

/// **The head is `0` and is never remembered**, which is what keeps the two
/// above apart: a bay that filed *the head* under the same key as *the third
/// strip* would lose the deck selection the first time an operator pressed `0`.
#[test]
fn the_head_is_not_one_of_the_things_a_bay_remembers() {
    let mut address = Address::default();
    assert!(address.remember(&[], 2));
    assert!(
        !address.remember(&[], HEAD),
        "`0` reported that it moved something"
    );
    assert_eq!(
        address.remembered(&[]),
        Some(2),
        "descending into the head overwrote the item the bay was on"
    );
    address.down(HEAD);
    assert_eq!(
        address.at(),
        &[HEAD],
        "the address did not descend into the head. It is a rung the address \
         goes through — which is how the Library's scope is reached — and only \
         the *memory* leaves it out"
    );
    assert_eq!(address.remembered(&[]), Some(2));
}

/// The floor under the two that walk a ring: an arrangement with no bay in it
/// would satisfy every loop above by iterating over nothing.
#[test]
fn the_arrangement_this_file_walks_has_the_bays_it_names() {
    for name in WALK {
        let found =
            region(name).unwrap_or_else(|| panic!("`{name}` is not a region this console draws"));
        assert!(
            karakuri_console::focus::is_bay(found.kind),
            "`{name}` is in this file's walk and is not a bay"
        );
        assert!(
            panel().layout().find(name).is_some(),
            "`{name}` is not in the arrangement"
        );
    }
}
