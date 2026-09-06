//! **A Set carried from a Library row onto a mixer strip**, end to end.
//!
//! `library.rs` is where the bay's rectangles are and which of them are
//! controls; `fader.rs` is what a hand does to a knob. This is the third kind
//! of drag: a row is picked up, nothing happens while it is carried, and the
//! strip it is let go over is what names the deck.
//!
//! **The whole of what it asserts is the difference from the fader.** A fader
//! knows its deck at the press, emits per changed value, cannot be cancelled
//! and reports no value at the release. A carry knows only its Set at the
//! press, emits nothing at all while it moves, is cancelled by being let go
//! over anything that is not a strip, and asks for its one operation at the
//! release. Every test here is one of those four sentences, and the fifth is
//! that nothing in the gesture reads what a deck is doing — *"Nothing refuses
//! it: what may be asked for is the instrument's to decide"*
//! (`docs/manual/console.html`, *How a Set reaches a deck*).
//!
//! **None of it needs a device.** The strips are laid out with the type in
//! them, which is the one thing here that needs `egui` — `mixer.rs`'s own
//! opening.
//!
//! # Where this stops
//!
//! Everything here ends at the operation, exactly as `fader.rs` does. What
//! carries the drop the rest of the way — the press handler that resolves the
//! strip at the release and the deck that is re-pointed afterwards — is
//! `crates/karakuri/src/main.rs`, and
//! `a_drop_on_a_strip_loads_the_strip_it_was_let_go_over` there is the test
//! that presses the gesture through the window loop's own routing.

mod common;

use common::{drawn_once, rect_of, showing, PLAUSIBLE};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Panel, Released};
use karakuri_console::room::size;
use karakuri_console::view::{library, mixer, LibraryBay, Mixer, Scope, Strip, Tally, View};
use karakuri_layout::Point;
use karakuri_operation::{BlendMode, Operation};

/// **The mock's own library, as names** — `library.rs`'s list, and the same
/// five: a drag that named the wrong row has four wrong answers to give.
fn sets() -> Vec<String> {
    [
        "drift_night",
        "lattice_veil",
        "glass_shell",
        "night01",
        "strand_bloom",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect()
}

/// Four strips, so that *which deck a drop named* is a question with three
/// wrong answers rather than none.
///
/// **Their residencies are all different and one of them is live**, which is
/// what makes `a_drop_on_a_live_deck_still_asks_for_the_load` a test of
/// something: a drop over deck A replaces what the room is watching, and this
/// bay is the one drawing that fact.
fn strips() -> Vec<Strip> {
    [Tally::Live, Tally::Priming, Tally::Allocated, Tally::Live]
        .into_iter()
        .enumerate()
        .map(|(slot, tally)| Strip {
            name: format!("slot{slot}"),
            tally,
            requested: tally,
            gain: 0.2 + 0.15 * slot as f32,
            gain_to: None,
            opacity: 0.8 - 0.15 * slot as f32,
            opacity_to: None,
            blend: BlendMode::Over,
            mask: karakuri_console::view::Mask::None,
            mask_angle: 0.0,
            level: None,
        })
        .collect()
}

/// A console with a store and a deck behind it: the mock's five rows, its four
/// scope chips, and four strips to drop onto.
fn console() -> (View, Panel, egui::Context) {
    let mut view = showing(&strips());
    view.library = sets();
    // The chips as well as the rows, because they are the same bay: a console
    // handed a listing and no scopes draws its list a scope row higher up.
    view.scopes = Scope::ALL.to_vec();
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    (view, panel, drawn_once())
}

fn bay(panel: &Panel, view: &View) -> LibraryBay {
    library(panel.layout(), &view.scopes, &view.library, view.opened())
        .expect("the library bay lists its rows")
}

fn strips_bay<'a>(panel: &Panel, ctx: &egui::Context, view: &'a View) -> Mixer<'a> {
    mixer(ctx, panel.layout(), &view.mixer).expect("the mixer bay draws its strips")
}

fn point(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

/// The middle of the `index`th drawn row.
fn row(bay: &LibraryBay, index: usize) -> Point {
    point(bay.row(index).center())
}

/// The middle of the `deck`th strip.
fn strip_at(bay: &Mixer<'_>, deck: u8) -> Point {
    point(
        bay.selected(deck)
            .unwrap_or_else(|| panic!("the bay draws a strip for deck {deck}"))
            .center(),
    )
}

// ---------------------------------------------------------------------------
// What a press takes in hand
// ---------------------------------------------------------------------------

/// **A press on a row takes that row's Set in hand, and nothing else does.**
///
/// Every drawn row is asked at its own middle and answers its own name, so a
/// walk that returned the cursor's row, the first row, or the row above has
/// four wrong answers to give. The list's own ground under the last row is
/// asked too: `.lib-list` runs to the foot and the rows stop where they stop,
/// so what is below them is nobody's — which is `input.rs`'s *a control claims
/// what it acts on and no more*.
#[test]
fn a_press_on_a_row_takes_that_rows_set_in_hand() {
    let (view, panel, _ctx) = console();
    let bay = bay(&panel, &view);
    assert!(
        bay.rows >= 3,
        "the bay listed {} rows at the plausible console, which is too few to tell one from \
         another",
        bay.rows
    );

    for index in 0..bay.rows {
        let taken = bay
            .take(&view.library, row(&bay, index))
            .unwrap_or_else(|| panic!("no row answered a press on row {index}"));
        assert_eq!(taken.row, index, "a press on row {index} took another row");
        assert_eq!(
            taken.set, view.library[index],
            "row {index} took a Set that is not the one on it"
        );
    }

    // **The ground under the last row**, which is where the list stops and the
    // foot has not started.
    //
    let below = bay.row(bay.rows - 1);
    let ground = Point::new(below.center().x, below.max.y + size::LIB_ROW_H * 0.5);
    assert!(
        ground.y < bay.foot.min.y,
        "the sweep is asking about the foot rather than about the list's ground"
    );
    assert_eq!(
        bay.take(&view.library, ground),
        None,
        "the list's own ground took a Set in hand"
    );

    // **And the row that is not drawn, asked of a library taller than its
    // list** — the mock's own `5 of 27`. Nothing scrolls, so the Sets past the
    // last row that fits are out of reach: the stride runs on past the bottom
    // of the list, the foot is drawn over where the next row would be, and a
    // press there must reach neither. This is the half the listing's own
    // length cannot answer, because there *is* a Set at that index.
    let long: Vec<String> = (0..80).map(|n| format!("set{n:02}")).collect();
    let tall = library(panel.layout(), &view.scopes, &long, None).expect("a bay with rows in it");
    assert!(
        tall.rows < long.len(),
        "the bay drew all {} rows, so there is no undrawn row to ask about",
        long.len()
    );
    let undrawn = point(tall.row(tall.rows).center());
    assert!(
        undrawn.y > tall.list.max.y,
        "the row after the last drawn one is still inside the list, so the rows do not fill it"
    );
    assert_eq!(
        tall.take(&long, undrawn),
        None,
        "a press below the list took a Set the bay never drew"
    );

    // **And a listing shorter than the rows drawn takes nothing**, which is
    // the refusal that makes handing the listing in worth doing: a row index
    // answered bare would name a Set nobody can see.
    assert_eq!(
        bay.take(&[], row(&bay, 0)),
        None,
        "a bay with nothing listed under it still handed a Set over"
    );
}

/// **A press on an open reading takes nothing in hand.**
///
/// The `read` chip opens a block of lines *between* two rows — *"one row is
/// open at a time, which is what keeps this a mode of the list rather than a
/// second list"* — and the rows below it move down by the whole block. So the
/// list has a region in the middle of it that is not a row, and a press there
/// must take nothing: a walk that divided the list by the row stride instead
/// of asking `LibraryBay::row` would answer with whichever row the block
/// happens to be sitting over, and a hand reading a Set would carry a
/// different one.
///
/// **The rows under the block still answer their own names**, which is the
/// other half: the block pushes them down and they are still the rows they
/// were.
#[test]
fn a_press_on_an_open_reading_takes_nothing_in_hand() {
    use karakuri_console::view::{Published, Reading};
    let (mut view, panel, _ctx) = console();
    view.read(Reading {
        id: view.library[0].clone(),
        knobs: ["radius", "turbulence", "amount"]
            .iter()
            .map(|key| Published {
                key: (*key).to_owned(),
                range: "0 – 1 · 0.5".to_owned(),
            })
            .collect(),
        capacity: None,
        emits: None,
        nodes: 1,
        described: 1,
    });
    let bay = bay(&panel, &view);
    let block = bay.reading.expect("the reading is open under the cursor");
    assert!(block.rows >= 3, "the block is too short to press into");

    for line in 0..block.rows {
        let at = point(block.row(line).center());
        assert!(
            at.y > bay.row(0).max.y,
            "line {line} of the reading is above the row it opened under"
        );
        assert_eq!(
            bay.take(&view.library, at),
            None,
            "a press on line {line} of the reading took a Set in hand"
        );
    }

    // **And the rows under it are still their own**, pushed down by the block.
    for index in (block.under)..bay.rows {
        let taken = bay
            .take(&view.library, row(&bay, index))
            .unwrap_or_else(|| panic!("row {index} under the block answered nothing"));
        assert_eq!(
            taken.set, view.library[index],
            "row {index} under the block took the wrong Set"
        );
    }
}

/// **A carry is in hand, and it is not a fader.**
///
/// `Panel::in_hand` is what a window loop asks to decide what a *move* meant,
/// and answering `Fader` for a carry would make every move of it emit the
/// last operation again. It is also what suppresses the resize cursor, which
/// is the other half of `view::View::cursor`'s rule.
#[test]
fn a_carry_is_in_hand_until_it_is_let_go() {
    use karakuri_console::panel::InHand;
    let (view, mut panel, _ctx) = console();
    let bay = bay(&panel, &view);
    let at = row(&bay, 1);

    assert_eq!(
        panel.in_hand(),
        None,
        "something was in hand before a press"
    );
    let taken = bay.take(&view.library, at).expect("row 1 is drawn");
    panel.carry(at, taken.set);
    assert_eq!(
        panel.in_hand(),
        Some(InHand::Carrying),
        "a carried Set is not what the panel says it has hold of"
    );
    assert!(panel.dragging(), "a carry is not a drag in hand");
    panel.released(None);
    assert_eq!(panel.in_hand(), None, "the release let go of nothing");
}

// ---------------------------------------------------------------------------
// What a move does, which is nothing
// ---------------------------------------------------------------------------

/// **Nothing is emitted while a Set is being carried.**
///
/// A fader emits per changed value, because the value is what it is moving. A
/// carry moves nothing: a Set half-way to a strip has not been loaded
/// anywhere, and an operation emitted on the way would name a deck the pointer
/// was merely passing over. So every move answers `None` — across the library,
/// over three strips in turn, and off the viewport.
#[test]
fn a_carry_emits_nothing_until_it_is_let_go() {
    let (view, mut panel, ctx) = console();
    let bay = bay(&panel, &view);
    let strips = strips_bay(&panel, &ctx, &view);
    let at = row(&bay, 0);
    let taken = bay.take(&view.library, at).expect("row 0 is drawn");
    panel.carry(at, taken.set);

    let mut asked = 0;
    for over in [
        row(&bay, 1),
        row(&bay, 2),
        strip_at(&strips, 0),
        strip_at(&strips, 1),
        strip_at(&strips, 3),
        Point::new(-40.0, -40.0),
    ] {
        assert_eq!(
            panel.moved(over),
            None,
            "a move with a Set in hand said something happened"
        );
        asked += 1;
    }
    assert_eq!(asked, 6, "not every move was asked");
    assert!(
        panel.dragging(),
        "the carry was let go of by a move rather than by a release"
    );
}

// ---------------------------------------------------------------------------
// What the drop asks for
// ---------------------------------------------------------------------------

/// **The right Set, on the deck it was let go over.**
///
/// Four strips and five rows, so a drop that named the selection, the strip
/// the press was over, or the first deck fails at once. The destination is
/// asked of the bay at the **release** point and never at the press: the whole
/// of what this route buys over the key is that the drop names the deck, and a
/// press in the Library bay is over no strip at all — which is asserted here,
/// because it is what makes resolving at the press impossible rather than
/// merely wrong.
#[test]
fn a_drop_on_a_strip_asks_to_load_that_strips_deck() {
    let (view, mut panel, ctx) = console();
    let bay = bay(&panel, &view);
    let strips = strips_bay(&panel, &ctx, &view);

    // **Where the press was made is over no strip**, so a destination
    // resolved there is `None` for every row and every deck.
    for index in 0..bay.rows {
        assert_eq!(
            strips.dropped(row(&bay, index)),
            None,
            "row {index} of the Library bay is over a mixer strip, so this test cannot tell a \
             destination resolved at the press from one resolved at the release"
        );
    }

    for deck in 0..4u8 {
        for index in 0..bay.rows {
            let at = row(&bay, index);
            let taken = bay.take(&view.library, at).expect("a drawn row");
            panel.carry(at, taken.set);
            let onto = strip_at(&strips, deck);
            assert_eq!(panel.moved(onto), None);
            assert_eq!(
                panel.released(strips.dropped(onto)),
                Some(Released::Dropped(Operation::LoadSet {
                    deck,
                    set: view.library[index].clone(),
                })),
                "row {index} let go over strip {deck} asked for something else"
            );
        }
    }
}

/// **A drop on nothing asks for nothing**, and that is the outcome a fader
/// does not have.
///
/// A boundary dragged off its track still lands somewhere legal and a fader
/// dragged past its end is still at its end; a row let go over the transport
/// row, over the Library bay it came from, or off the viewport has nowhere to
/// land. The nearest strip is not the answer, because a load aimed at it would
/// be a deck nobody pointed at.
#[test]
fn a_drop_on_nothing_asks_for_nothing() {
    let (view, mut panel, ctx) = console();
    let bay = bay(&panel, &view);
    let strips = strips_bay(&panel, &ctx, &view);
    let transport = rect_of(panel.layout(), "transport");

    let mut asked = 0;
    for over in [
        row(&bay, 0),
        Point::new(transport.x + transport.w * 0.5, transport.y + 4.0),
        Point::new(-40.0, -40.0),
    ] {
        let at = row(&bay, 0);
        let taken = bay.take(&view.library, at).expect("row 0 is drawn");
        panel.carry(at, taken.set);
        assert_eq!(panel.moved(over), None);
        assert_eq!(
            panel.released(strips.dropped(over)),
            Some(Released::Nowhere {
                set: view.library[0].clone(),
            }),
            "a drop at {over:?} asked for a load"
        );
        assert!(!panel.dragging(), "the carry is still in hand");
        asked += 1;
    }
    assert_eq!(asked, 3, "not every drop was asked");
}

/// **A drop on a live deck still asks for the load**, and this drag has no
/// reading of residency at all.
///
/// `console.html` rules on it in as many words — *"Nothing refuses it: what
/// may be asked for is the instrument's to decide, and a panel refusing what
/// the key allows would be a second rule kept in a second place"* — so the one
/// thing this control must not learn is what a deck is doing.
///
/// **Asserted twice over, because a refusal could hide in either half.** The
/// live strip's drop asks for its load like every other; and the same bay laid
/// out from strips whose residencies have all been changed answers the same
/// deck at the same point, which is what says the destination is a rectangle
/// and not a state.
#[test]
fn a_drop_on_a_live_deck_still_asks_for_the_load() {
    let (view, mut panel, ctx) = console();
    let bay = bay(&panel, &view);
    let strips = strips_bay(&panel, &ctx, &view);
    assert_eq!(
        view.mixer[0].tally,
        Tally::Live,
        "deck A is not on air, so this test is not about a drop on one that is"
    );

    let at = row(&bay, 0);
    let taken = bay.take(&view.library, at).expect("row 0 is drawn");
    panel.carry(at, taken.set);
    let onto = strip_at(&strips, 0);
    assert_eq!(
        panel.released(strips.dropped(onto)),
        Some(Released::Dropped(Operation::LoadSet {
            deck: 0,
            set: view.library[0].clone(),
        })),
        "a drop on the deck the room is watching was refused by the panel"
    );

    // **The same points, every residency turned over.** If anything in the
    // destination read a tally, one of these four would answer differently.
    let mut turned = view.mixer.clone();
    for (slot, strip) in turned.iter_mut().enumerate() {
        strip.tally = [
            Tally::Allocated,
            Tally::Live,
            Tally::Priming,
            Tally::Allocated,
        ][slot];
        strip.requested = strip.tally;
    }
    let other = showing(&turned);
    let after = strips_bay(&panel, &ctx, &other);
    for deck in 0..4u8 {
        assert_eq!(
            after.dropped(strip_at(&strips, deck)),
            Some(deck),
            "strip {deck} answered a different deck once its residency changed"
        );
    }
}

// ---------------------------------------------------------------------------
// The claim, and the mark on the row
// ---------------------------------------------------------------------------

/// **A press on a row is the panel's, and it keeps the pointer while the hand
/// crosses the console** — `input`'s rule 1 covering the third kind of drag
/// without a word being added to it.
///
/// The row is claimed on the way down, the carry keeps every event while the
/// pointer runs across two bays and off the viewport, and the same point is
/// `egui`'s again the moment the button comes up. That last is what says the
/// claim was the *gesture's* and not the position's.
#[test]
fn a_carry_keeps_its_claim_while_the_pointer_leaves_the_bay() {
    let (view, mut panel, ctx) = console();
    let bay = bay(&panel, &view);
    let strips = strips_bay(&panel, &ctx, &view);
    let at = row(&bay, 2);

    assert_eq!(
        claim(&mut panel, &ctx, &view, at),
        Claim::Panel,
        "a press on a library row went to `egui`"
    );
    let taken = bay.take(&view.library, at).expect("row 2 is drawn");
    panel.carry(at, taken.set);

    let mut asked = 0;
    for over in [
        strip_at(&strips, 1),
        Point::new(-40.0, -40.0),
        Point::new(PLAUSIBLE.w + 200.0, PLAUSIBLE.h * 0.5),
    ] {
        assert_eq!(
            claim(&mut panel, &ctx, &view, over),
            Claim::Panel,
            "the carry lost its claim at {over:?}"
        );
        panel.moved(over);
        asked += 1;
    }
    assert_eq!(asked, 3, "not every point was asked");

    let away = Point::new(-40.0, -40.0);
    panel.released(strips.dropped(away));
    assert_eq!(
        claim(&mut panel, &ctx, &view, away),
        Claim::Egui,
        "the panel kept the pointer after the gesture ended"
    );
}

/// **The row a hand takes is the row the cursor marks.**
///
/// The mock draws no ghost under a pointer and no lit strip, and it does draw
/// `.lib-row.cursor` — so the mark on the row is the whole of what a carry
/// can show, and the press is what moves it. It outlives the gesture on
/// purpose: the cursor is the operand `l` reads, so a drop that landed and a
/// carry let go over nothing both leave the keyboard aimed at the Set the hand
/// last touched.
///
/// **A row past the listing is refused rather than clamped**, which is
/// `View::select`'s rule and not `View::walk`'s: a press names a row outright,
/// and answering it with the nearest one would move the load somewhere nobody
/// pointed.
#[test]
fn the_row_a_hand_takes_is_the_row_the_cursor_marks() {
    let (mut view, panel, _ctx) = console();
    let bay = bay(&panel, &view);
    assert_eq!(view.cursor_row(), 0, "the cursor does not start at the top");

    for index in (0..bay.rows).rev() {
        let taken = bay
            .take(&view.library, row(&bay, index))
            .expect("a drawn row");
        let was = view.cursor_row();
        assert_eq!(
            view.point_at(taken.row),
            index != was,
            "the mark answered a move it did not make"
        );
        assert_eq!(
            view.cursor_row(),
            index,
            "the mark is not on the row the hand took"
        );
    }

    // The same row again is not a move, and costs no frame.
    assert!(
        !view.point_at(0),
        "pointing at the row the cursor is already on read as a move"
    );
    // And a row the listing does not have is refused rather than clamped.
    assert!(!view.point_at(view.library.len()));
    assert_eq!(
        view.cursor_row(),
        0,
        "a refused press moved the mark anyway"
    );
}
