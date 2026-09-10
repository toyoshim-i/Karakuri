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
use karakuri_console::view::{
    library, mixer, program_bay, rearrange, LibraryBay, Mixer, ProgramBay, Rows, Scope, Strip,
    Tally, View,
};
use karakuri_layout::Point;
use karakuri_operation::{BlendMode, Operation};

/// **A listing of Sets and nothing else**, which is what every test in this
/// file is about: a row with no entry in the kinds beside it is a Set with no
/// badge, which is the seam's own default (`view::RowKind`).
fn listed(names: &[String]) -> Rows<'_> {
    Rows { names, kinds: &[] }
}

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
    library(
        panel.layout(),
        &view.scopes,
        &view.library,
        view.opened(),
        None,
        0.0,
    )
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
            .take(listed(&view.library), row(&bay, index))
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
        bay.take(listed(&view.library), ground),
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
    let tall = library(panel.layout(), &view.scopes, &long, None, None, 0.0)
        .expect("a bay with rows in it");
    assert!(
        tall.rows < long.len(),
        "the bay drew all {} rows, so there is no undrawn row to ask about",
        long.len()
    );
    // **The first row whose whole box is below the list**, which is the one
    // after the last drawn one where the rows fill the list exactly and the one
    // after that where a cut row is drawn in the leftover. A cut row *is*
    // pressable — `LibraryBay::drawn` includes it and the paint clips it — so
    // the row this asks about is the first that is not drawn at all.
    let undrawn = point(
        (tall.rows..long.len())
            .map(|index| tall.row(index))
            .find(|row| row.min.y >= tall.list.max.y)
            .expect("every row of a listing of eighty reaches the list")
            .center(),
    );
    assert!(
        undrawn.y > tall.list.max.y,
        "the row after the last drawn one is still inside the list, so the rows do not fill it"
    );
    assert_eq!(
        tall.take(listed(&long), undrawn),
        None,
        "a press below the list took a Set the bay never drew"
    );

    // **And a listing shorter than the rows drawn takes nothing**, which is
    // the refusal that makes handing the listing in worth doing: a row index
    // answered bare would name a Set nobody can see.
    assert_eq!(
        bay.take(Rows::NONE, row(&bay, 0)),
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
            bay.take(listed(&view.library), at),
            None,
            "a press on line {line} of the reading took a Set in hand"
        );
    }

    // **And the rows under it are still their own**, pushed down by the block.
    for index in (block.under)..bay.rows {
        let taken = bay
            .take(listed(&view.library), row(&bay, index))
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
    let taken = bay.take(listed(&view.library), at).expect("row 1 is drawn");
    panel.carry(at, taken.set, taken.procedure);
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
    let taken = bay.take(listed(&view.library), at).expect("row 0 is drawn");
    panel.carry(at, taken.set, taken.procedure);

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
            let taken = bay.take(listed(&view.library), at).expect("a drawn row");
            panel.carry(at, taken.set, taken.procedure);
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
        let taken = bay.take(listed(&view.library), at).expect("row 0 is drawn");
        panel.carry(at, taken.set, taken.procedure);
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
    let taken = bay.take(listed(&view.library), at).expect("row 0 is drawn");
    panel.carry(at, taken.set, taken.procedure);
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
    let taken = bay.take(listed(&view.library), at).expect("row 2 is drawn");
    panel.carry(at, taken.set, taken.procedure);

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
            .take(listed(&view.library), row(&bay, index))
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

// ---------------------------------------------------------------------------
// The other set of rectangles a drop can land on
// ---------------------------------------------------------------------------

/// A console with a store, a deck of `strips.len()` slots, and the Program
/// bay arranged so its four cells have places.
///
/// [`console`] does not rearrange, because nothing it tests reads the Program
/// bay; every rectangle in that bay is read from the bit `view::rearrange`
/// writes, so a cell asked for without it is a cell read out of one of the two
/// arrangements only.
fn with_cells(strips: &[Strip]) -> (View, Panel, egui::Context) {
    let mut view = showing(strips);
    view.library = sets();
    view.scopes = Scope::ALL.to_vec();
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    rearrange(&mut panel, view.canvas);
    panel.solve();
    (view, panel, drawn_once())
}

fn program(panel: &Panel, view: &View) -> ProgramBay {
    program_bay(panel.layout(), view.canvas).expect("the Program bay is on screen")
}

/// The middle of the `deck`th preview cell's image.
fn cell_at(bay: &ProgramBay, deck: u8) -> Point {
    point(bay.cells.expect("the preview row is on screen")[usize::from(deck)].center())
}

/// **A drop on a deck preview cell asks to load that cell's deck**, which is
/// the same command a release on that deck's strip asks for.
///
/// Four cells and five rows, so a drop that named the selection, the row's
/// index or the first deck has three wrong answers to give. **The strips are
/// asked at every cell too**, and answer `None` at all four: that is what
/// makes this a second set of rectangles rather than a second reading of the
/// first, and it is what *at most one rectangle is marked* rests on — a point
/// inside one set is outside the other, so nothing has to arbitrate.
#[test]
fn a_drop_on_a_preview_cell_asks_to_load_that_cells_deck() {
    let (view, mut panel, ctx) = with_cells(&strips());
    let bay = bay(&panel, &view);
    let cells = program(&panel, &view);
    let strips = strips_bay(&panel, &ctx, &view);

    for deck in 0..4u8 {
        let onto = cell_at(&cells, deck);
        assert_eq!(
            strips.dropped(onto),
            None,
            "cell {deck} is inside a mixer strip, so a release there has two answers"
        );
        for index in 0..bay.rows {
            let at = row(&bay, index);
            let taken = bay.take(listed(&view.library), at).expect("a drawn row");
            panel.carry(at, taken.set, taken.procedure);
            assert_eq!(panel.moved(onto), None);
            assert_eq!(
                panel.released(cells.dropped(onto, view.mixer.len())),
                Some(Released::Dropped(Operation::LoadSet {
                    deck,
                    set: view.library[index].clone(),
                })),
                "row {index} let go over cell {deck} asked for something else"
            );
        }
    }
}

/// **A cell whose letter names no slot takes no drop**, and it is the only
/// rectangle in either set that is drawn and is not a target.
///
/// The mixer draws one strip per slot, so a three-slot deck has three strips
/// and a fourth cell with nothing behind the letter on it — the mock's own
/// deck D. A release there has no deck to load into and asks for nothing,
/// which is the refusal `3` already gets from the keyboard and the same
/// reading behind it: `View::select` off `View::mixer`'s length.
///
/// **The three cells beside it still answer**, which is what makes this a
/// reading of the slot count rather than a bay that stopped taking drops; and
/// the mixer is asked at the fourth cell too, so a `None` there cannot be the
/// strips answering for it.
#[test]
fn a_cell_whose_letter_names_no_slot_takes_no_drop() {
    let three: Vec<Strip> = strips().into_iter().take(3).collect();
    let (view, mut panel, ctx) = with_cells(&three);
    let bay = bay(&panel, &view);
    let cells = program(&panel, &view);
    let strips = strips_bay(&panel, &ctx, &view);
    assert_eq!(view.mixer.len(), 3, "this deck is not three slots");
    assert_eq!(
        cells.cells.expect("the preview row is on screen").len(),
        4,
        "the row is not four cells, so there is no cell without a slot to test"
    );

    // The fourth cell is drawn, and `cell` still answers for it — what has
    // changed is what a *release* there means.
    let onto = cell_at(&cells, 3);
    assert_eq!(cells.cell(onto), Some(3), "the fourth cell is not drawn");
    assert!(cells.owns(onto), "the fourth cell is not claimed any more");
    assert_eq!(strips.dropped(onto), None);
    assert_eq!(
        cells.dropped(onto, view.mixer.len()),
        None,
        "a drop on deck D's cell named a deck this console has no slot for"
    );

    let at = row(&bay, 0);
    let taken = bay.take(listed(&view.library), at).expect("row 0 is drawn");
    panel.carry(at, taken.set, taken.procedure);
    assert_eq!(
        panel.released(cells.dropped(onto, view.mixer.len())),
        Some(Released::Nowhere {
            set: view.library[0].clone(),
        }),
        "a drop on a cell with no slot behind its letter asked for a load"
    );

    // And the three that do name a slot are unaffected.
    for deck in 0..3u8 {
        let onto = cell_at(&cells, deck);
        assert_eq!(
            cells.dropped(onto, view.mixer.len()),
            Some(deck),
            "cell {deck} stopped naming its deck"
        );
    }
}

// ---------------------------------------------------------------------------
// What the carry is drawn as
// ---------------------------------------------------------------------------

/// Every drop mark the console paints on one frame, as the rectangle it is
/// round.
///
/// **Recognised by the ink and the side of the edge it is on**, which is what
/// `style.css` gives it and nothing else on this panel has: `--c-text` is the
/// one colour the four states do not use, and `.strip.focus`'s lavender ring
/// is inset where this is an `outline`. So a mark counted here cannot be the
/// selection, a hairline, or a pill's border.
fn drop_marks(view: &mut View, panel: &mut Panel) -> Vec<egui::Rect> {
    let ink = view.room.palette().text;
    let ctx = drawn_once();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, panel));
    out.textures_delta.clear();
    out.shapes
        .into_iter()
        .filter_map(|clipped| match clipped.shape {
            egui::Shape::Rect(rect)
                if rect.stroke.color == ink
                    && rect.stroke.width == size::DROP_RING
                    && rect.stroke_kind == egui::StrokeKind::Outside =>
            {
                Some(rect.rect)
            }
            _ => None,
        })
        .collect()
}

/// The cursor the console asked for on one frame.
fn cursor(view: &mut View, panel: &mut Panel) -> egui::CursorIcon {
    let ctx = drawn_once();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, panel));
    out.textures_delta.clear();
    out.platform_output.cursor_icon
}

/// **At most one rectangle is marked, and it is the one the release would
/// name.**
///
/// The pointer is in one place, so this is a fact about where the hand is
/// rather than a rule either bay keeps — and it is asserted over both sets of
/// rectangles and over the ground between them. Four strips and four cells,
/// so a mark drawn round the wrong one of the eight has seven wrong answers
/// to give.
///
/// **Nothing is marked with nothing in hand**, which is what separates this
/// from a hover: the same points are drawn with no carry and carry no ring.
#[test]
fn one_rectangle_is_marked_and_it_is_the_one_the_release_names() {
    let (mut view, mut panel, ctx) = with_cells(&strips());
    let bay = bay(&panel, &view);
    let cells = program(&panel, &view);
    // **The eight rectangles, taken off the two bays before anything is
    // drawn**: `Mixer` borrows the strips the view holds, and `View::draw`
    // takes it by `&mut`.
    let targets: Vec<Point> = {
        let strips = strips_bay(&panel, &ctx, &view);
        (0..4u8)
            .flat_map(|deck| [strip_at(&strips, deck), cell_at(&cells, deck)])
            .collect()
    };

    // Nothing in hand: the pointer is over strip A and nothing is ringed.
    panel.moved(targets[0]);
    assert_eq!(
        drop_marks(&mut view, &mut panel).len(),
        0,
        "a rectangle is ringed with nothing in hand — the mark is a hover"
    );

    let mut asked = 0;
    for onto in targets {
        let at = row(&bay, 0);
        let taken = bay.take(listed(&view.library), at).expect("row 0 is drawn");
        panel.carry(at, taken.set, taken.procedure);
        panel.moved(onto);
        let marks = drop_marks(&mut view, &mut panel);
        assert_eq!(
            marks.len(),
            1,
            "{} rectangles are ringed with the pointer at {onto:?}",
            marks.len()
        );
        assert!(
            marks[0].contains(egui::Pos2::new(onto.x, onto.y)),
            "the ring at {:?} is not round the rectangle under the pointer at {onto:?}",
            marks[0]
        );
        panel.released(None);
        asked += 1;
    }
    assert_eq!(asked, 8, "not every rectangle was asked");
}

/// **Over anything that is not a target, nothing is marked.**
///
/// The alley between two strips is the case the rule was written for:
/// `.mixer-strips` has a `gap: 4px` and it is a real place to let go, so a
/// mark that snapped to the nearest strip would name a deck nobody pointed at
/// and the release after it would load one. The bay head above the strips,
/// the transition row under them, the Library bay the Set came out of and a
/// point off the viewport are the other four.
///
/// **The fourth cell of a three-slot deck is the fifth**, and it is the one
/// that is a rectangle rather than ground: a release there loads nothing
/// (`a_cell_whose_letter_names_no_slot_takes_no_drop`), so a ring on it would
/// be the mark promising a landing the release does not make.
#[test]
fn nothing_is_marked_where_a_release_would_load_nothing() {
    let three: Vec<Strip> = strips().into_iter().take(3).collect();
    let (mut view, mut panel, ctx) = with_cells(&three);
    let bay = bay(&panel, &view);
    let cells = program(&panel, &view);

    // The alley: half way between strip A's right edge and strip B's left.
    // Taken before anything is drawn, for the reason above.
    let alley = {
        let strips = strips_bay(&panel, &ctx, &view);
        let a = strips.selected(0).expect("a strip for deck A");
        let b = strips.selected(1).expect("a strip for deck B");
        let alley = Point::new((a.max.x + b.min.x) * 0.5, a.center().y);
        assert_eq!(strips.dropped(alley), None, "the alley is inside a strip");
        alley
    };
    let mixer_region = rect_of(panel.layout(), "mixer");
    let transport = rect_of(panel.layout(), "transport");

    let mut asked = 0;
    for over in [
        alley,
        // The bay's head, above the strips.
        Point::new(mixer_region.x + mixer_region.w * 0.5, mixer_region.y + 4.0),
        Point::new(transport.x + transport.w * 0.5, transport.y + 4.0),
        row(&bay, 0),
        Point::new(-40.0, -40.0),
        cell_at(&cells, 3),
    ] {
        let at = row(&bay, 0);
        let taken = bay.take(listed(&view.library), at).expect("row 0 is drawn");
        panel.carry(at, taken.set, taken.procedure);
        panel.moved(over);
        let marks = drop_marks(&mut view, &mut panel);
        assert_eq!(
            marks.len(),
            0,
            "{:?} is ringed with the pointer at {over:?}, where a release loads nothing",
            marks
        );
        panel.released(None);
        asked += 1;
    }
    assert_eq!(asked, 6, "not every point was asked");
}

/// **The pointer is a grab for as long as the Set is in hand, wherever it
/// is** — the one thing that says a gesture is still running while the hand is
/// over nothing at all.
///
/// **Asserted over a boundary above all.** A carry crosses every divider
/// between the Library bay and the mixer, and the arm this replaces answered
/// `None` there and fell through to the hit test — so a resize cursor flicked
/// on over each of them, for a gesture that was not happening. The boundary is
/// asked here with nothing in hand as well, where it is still a resize: that
/// is what makes this the *carry* suppressing it rather than the hit test
/// having stopped working.
#[test]
fn the_pointer_is_a_grab_while_a_set_is_in_hand() {
    let (mut view, mut panel, ctx) = with_cells(&strips());
    let bay = bay(&panel, &view);
    let on_strip = {
        let strips = strips_bay(&panel, &ctx, &view);
        strip_at(&strips, 1)
    };

    // A vertical boundary, found the way the panel finds one: the left edge of
    // the mixer's own region is a divider between two panes.
    let mixer_region = rect_of(panel.layout(), "mixer");
    let on_boundary = Point::new(mixer_region.x, mixer_region.y + mixer_region.h * 0.5);
    panel.moved(on_boundary);
    let resize = cursor(&mut view, &mut panel);
    assert!(
        matches!(
            resize,
            egui::CursorIcon::ResizeHorizontal | egui::CursorIcon::ResizeVertical
        ),
        "the point picked is not on a boundary at all ({resize:?}), so the assertion below \
         would pass against nothing"
    );

    let at = row(&bay, 2);
    let taken = bay.take(listed(&view.library), at).expect("row 2 is drawn");
    panel.carry(at, taken.set, taken.procedure);

    let mut asked = 0;
    for over in [
        on_boundary,
        on_strip,
        row(&bay, 0),
        Point::new(-40.0, -40.0),
    ] {
        panel.moved(over);
        assert_eq!(
            cursor(&mut view, &mut panel),
            egui::CursorIcon::Grabbing,
            "the pointer is not a grab at {over:?} with a Set in hand"
        );
        asked += 1;
    }
    assert_eq!(asked, 4, "not every point was asked");

    // And it is gone with the gesture: the boundary is a resize again.
    panel.moved(on_boundary);
    panel.released(None);
    assert_eq!(
        cursor(&mut view, &mut panel),
        resize,
        "the grab outlived the carry"
    );
}
