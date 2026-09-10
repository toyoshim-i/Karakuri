//! **The deck head's six chips and the seven controls in them, and the first
//! controls on this console that are not in a row of their own.**
//!
//! Thirteen things, and the first two are why this is its own file rather than
//! a few more assertions in `view.rs`'s module tests:
//!
//! 1. Where they sit, as `.deck-head`'s flex row lays them out — the three on
//!    the left measured from the left and the three after the `.sep` measured
//!    leftwards from the fold.
//! 2. **That the controls clear every boundary's grab**, which is
//!    `tests/look.rs`'s arithmetic over a row that is not a row of the
//!    arrangement: a pane's deck head is measured against the **pane
//!    divider**, and the number that binds is `.deck-head`'s own 10 pixels of
//!    left padding against a `GRAB` of 6.
//! 3. That the sync chip cycles all three modes and comes back, once each.
//! 4. **That the cycle skips a mode the material cannot honour**, which is the
//!    one thing this cycle has that the mixer's two do not.
//! 5. **That the anchor asks for the mode the deck is already in**, which is
//!    the one thing the chip beside it structurally cannot ask for.
//! 6. That an arrow asks for a quarter beat, signed by which arrow it was.
//! 7. **That an inert scrub is drawn and not claimed**, which is *a control
//!    claims what it acts on and no more* answered by a state.
//! 8. **That the fold names the layering the deck is not in**, which is the
//!    chip that was drawn and claimed nothing until 2026-09-09 — see
//!    `docs/adr/0314-…`.
//! 9. **That the capacity chip steps every rung of its ladder once and wraps**,
//!    and that a slot running off the ladder steps *up* rather than back to the
//!    bottom — `docs/adr/0328-…`.
//! 10. **That a capacity with nowhere to step is drawn and claims nothing**,
//!     which is the inert scrub's arrangement one control along.
//! 11. **That `re-salt` asks for the salt it was handed**, which is the whole
//!     of what keeps a re-seed reproducible.
//! 12. **That a pane too narrow for the two build chips keeps the five that
//!     were here before them**, which is the one place this row drops part of
//!     itself rather than all of it — and the reason is a measurement, at the
//!     test.
//! 13. **The route a window loop actually takes** — `claim`, then the
//!     derivation that drew the control, then the operation.
//!
//! None of it needs a window, a device or a disk. It does need `egui`'s fonts,
//! because a chip is as wide as the word in it — see `common::drawn_once` —
//! and a `Pane`, because a console with no deck behind it has no pane to head.

mod common;

use common::{drawn_once, near, rect_of, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Panel, GRAB};
use karakuri_console::room::{size, Room};
use karakuri_console::view::{
    deck_head, inspector, Aimed, DeckHead, InspectorPane, Pane, View, PANES, PANE_NAMES,
    SCRUB_BEATS, SYNCS,
};
use karakuri_layout::{Point, Rect};
use karakuri_operation::{Operation, Sync};

/// **The mock's own deck B**: beat-synced, engaged at 128 BPM and sitting a
/// quarter beat ahead of the room, with nothing its material refuses. It is
/// deck B rather than deck A because that is the pane the mock draws every
/// part of this row live on — a tempo-synced deck's arrows are `.scrub.idle`,
/// which is its own test below.
fn mock() -> Pane {
    Pane {
        deck: 1,
        material: "lattice_veil".to_owned(),
        sync: Sync::Beat,
        allows: [true; SYNCS.len()],
        anchor_bpm: 128.0,
        scrub_beats: 0.25,
        composite: false,
        aimed: Some(aimed()),
        nodes: Vec::new(),
    }
}

/// **The mock deck B's build chips**, and every number in it is the mock's own:
/// `lattice_veil`'s geometry is `examples/lattice_shell.kir`, which declares
/// `capacity [4096, 262144] = 32768`, and the mock draws that default unlit
/// because nobody has asked this deck for another number.
///
/// **The ladder is the powers of two inside the declared range**, ascending,
/// which is the list a host reads off the material rather than a list this
/// console owns — so it is written out here as data rather than generated,
/// exactly as the mock writes it.
///
/// **The salt is any number and the test is that it is *this* one**: what a
/// press asks for is a value the host handed over, so a console that computed
/// one would be reproducible only by accident. `NEXT_SALT` is what
/// `karakuri_engine::set::derived_salt(0, 1)` is, which makes it a plausible
/// one to be handed.
fn aimed() -> Aimed {
    Aimed {
        capacity: 32768,
        stated: false,
        capacities: LADDER.to_vec(),
        salt: NEXT_SALT,
    }
}

/// The powers of two inside `lattice_shell`'s declared `[4096, 262144]`.
const LADDER: [u32; 7] = [4096, 8192, 16384, 32768, 65536, 131072, 262144];

/// The salt the mock's host hands the `re-salt` capsule.
const NEXT_SALT: u32 = 0x9E37_79B9;

/// That pane in some other mode, which is the only thing most of these tests
/// vary.
fn at_sync(sync: Sync) -> Pane {
    Pane { sync, ..mock() }
}

/// A view with a deck behind it — two panes, both showing `pane` — which is
/// what `View::draw` paints from and what `claim` hit-tests, one value.
fn view(pane: &Pane) -> View {
    let mut view = View::new(Room::Day);
    view.inspector = vec![pane.clone(); PANES];
    view
}

/// A panel at a viewport, solved, with a context that has drawn once.
fn console(viewport: Rect) -> (Panel, egui::Context) {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    (panel, drawn_once())
}

/// The laid-out pane, and the laid-out chips of its deck head inside it.
fn chips(
    panel: &Panel,
    ctx: &egui::Context,
    index: usize,
    pane: &Pane,
) -> (InspectorPane, DeckHead) {
    let at = inspector(panel.layout(), index, pane, 0.0).expect("a pane with room in it");
    let head = deck_head(ctx, &at, pane).expect("a deck head with room for its chips");
    (at, head)
}

/// A `karakuri_layout` point, from `egui`'s.
fn at(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

/// Every mode, and the order is deliberately **not** the cycle's: what is
/// asserted below is that the cycle visits each of these once, and a list in
/// the cycle's own order could not tell that from a cycle that had lost one.
const EVERY: [Sync; 3] = [Sync::Beat, Sync::Free, Sync::Tempo];

// ---------------------------------------------------------------------------
// Where the controls are
// ---------------------------------------------------------------------------

/// **The head is `.deck-head`'s own flex row**: the mode chip against the
/// left-hand padding, the anchor one gap along, the two arrows one gap after
/// that with `.scrub`'s own 3 between them, and the fold hard against the
/// right-hand padding.
#[test]
fn the_deck_head_is_the_rows_own_geometry() {
    let pane = mock();
    let (panel, ctx) = console(SMALLEST);
    let (pane_at, head) = chips(&panel, &ctx, 0, &pane);
    let row = pane_at.deck_head;
    let mid = row.center().y;

    assert!(near(head.mode.min.x, row.min.x + size::DECK_HEAD_PAD_X));
    assert!(near(head.mode.height(), size::MINI_H));
    assert!(near(head.mode.center().y, mid));

    let anchor = head.anchor.expect("a beat-synced deck reads an anchor");
    assert!(near(anchor.min.x, head.mode.max.x + size::DECK_HEAD_GAP));
    assert!(
        near(anchor.height(), size::MINI_H),
        "the anchor is {} tall and the chips either side of it are {} — a bare 9px run is not a \
         target a hand finds",
        anchor.height(),
        size::MINI_H
    );
    assert!(near(anchor.center().y, mid));

    assert!(near(head.back.min.x, anchor.max.x + size::DECK_HEAD_GAP));
    assert!(near(head.forward.min.x, head.back.max.x + size::SCRUB_GAP));
    for arrow in [head.back, head.forward] {
        assert!(
            near(
                arrow.width(),
                size::SCRUB_SIZE + size::SCRUB_PAD_X * 2.0 + size::HAIRLINE * 2.0
            ),
            "an arrow is {} wide and `.scrub i` is a 9px mark inside 4 of padding and its own \
             border",
            arrow.width()
        );
        assert!(near(arrow.height(), size::SCRUB_H));
        assert!(near(arrow.center().y, mid));
    }

    assert!(
        near(head.composite.max.x, row.max.x - size::DECK_HEAD_PAD_X),
        "the fold ends {} from the right of the row and `.deck-head`'s padding is {}",
        row.max.x - head.composite.max.x,
        size::DECK_HEAD_PAD_X
    );
    assert!(row.contains_rect(head.mode) && row.contains_rect(head.composite));
    assert!(
        head.forward.max.x + size::DECK_HEAD_GAP <= head.composite.min.x,
        "the arrows end at {} and the fold begins at {}",
        head.forward.max.x,
        head.composite.min.x
    );
    assert_eq!(
        head.aim,
        None,
        "a pane at the smallest window this arrangement claims drew the two build chips, and \
         there is not room for them: the row is {} wide and the five that were here already end \
         {} from the fold",
        row.width(),
        head.composite.min.x - head.forward.max.x
    );
}

/// **The two build chips are measured leftwards from the fold**, which is what
/// `.sep`'s `flex: 1` does to everything after it: the fold against the
/// right-hand padding, `re-salt` one gap before it, and the capacity one gap
/// before that.
///
/// At a plausible window rather than the smallest one, because
/// `the_deck_head_is_the_rows_own_geometry` above is what says they are not
/// there at the smallest — the two facts are the same measurement read at two
/// widths.
#[test]
fn the_build_chips_are_measured_leftwards_from_the_fold() {
    let pane = mock();
    let (panel, ctx) = console(PLAUSIBLE);
    let (pane_at, head) = chips(&panel, &ctx, 0, &pane);
    let row = pane_at.deck_head;
    let aim = head.aim.expect("a wide pane draws the two build chips");

    assert!(near(
        aim.salt.max.x,
        head.composite.min.x - size::DECK_HEAD_GAP
    ));
    assert!(near(aim.size.max.x, aim.salt.min.x - size::DECK_HEAD_GAP));
    for chip in [aim.size, aim.salt] {
        assert!(near(chip.height(), size::MINI_H));
        assert!(near(chip.center().y, row.center().y));
        assert!(row.contains_rect(chip));
    }
    assert!(
        head.forward.max.x + size::DECK_HEAD_GAP <= aim.size.min.x,
        "the arrows end at {} and the leftmost of the three on the right begins at {}",
        head.forward.max.x,
        aim.size.min.x
    );
}

/// **A pane too narrow for the two build chips keeps the five that were here
/// before them**, which is the one place this row answers *a control that does
/// not fit is no control at all* by dropping part of the row rather than all of
/// it.
///
/// **The measurement is the argument.** An Inspector pane at the console's
/// declared minimum window is 237.5 pixels wide and the row needs about 296 for
/// all seven, so a row that took all of them or none would draw **nothing** at
/// the width this arrangement claims to work at — trading two controls that
/// were never there for four that were. The threshold is a window of about 1360
/// with two panes open, and the console page says so.
#[test]
fn a_pane_too_narrow_for_the_build_chips_keeps_the_rest_of_the_row() {
    let pane = mock();
    let (panel, ctx) = console(PLAUSIBLE);
    let (pane_at, head) = chips(&panel, &ctx, 0, &pane);
    let full = pane_at.deck_head;
    let aim = head.aim.expect("a wide pane draws the two build chips");
    // **Everything after the `.sep` moves left with the row's right edge**, so
    // narrowing by the slack between the arrows and the capacity chip is
    // exactly the width at which the seven stop fitting.
    let slack = aim.size.min.x - (head.forward.max.x + size::DECK_HEAD_GAP);
    let narrowed = |w: f32| InspectorPane {
        deck_head: egui::Rect::from_min_size(full.min, egui::vec2(w, full.height())),
        ..pane_at
    };
    let fits = narrowed(full.width() - slack);
    assert!(
        deck_head(&ctx, &fits, &pane)
            .expect("a row exactly wide enough for the seven")
            .aim
            .is_some(),
        "a row exactly wide enough for the seven dropped two of them, so this test cannot tell \
         a fit from a drop"
    );
    let tight = narrowed(full.width() - slack - 1.0);
    let head = deck_head(&ctx, &tight, &pane).expect("the five that fit are still drawn");
    assert_eq!(
        head.aim, None,
        "a row with no room for the two build chips drew them anyway, and `inspector_into`'s \
         clip is what would cut them in half"
    );
    assert!(
        near(
            head.composite.max.x,
            tight.deck_head.max.x - size::DECK_HEAD_PAD_X
        ),
        "the fold did not stay against the right-hand padding when the chips beside it were \
         dropped"
    );
    assert!(
        head.forward.max.x + size::DECK_HEAD_GAP <= head.composite.min.x,
        "the five that were kept no longer fit each other"
    );
}

/// **A free deck draws no anchor, and the row closes up rather than leaving a
/// hole where one would have been** — which is what a flex row does and is why
/// the arrows are measured from whichever of the two came last.
#[test]
fn a_free_deck_draws_no_anchor_and_the_row_closes_up() {
    let pane = at_sync(Sync::Free);
    let (panel, ctx) = console(PLAUSIBLE);
    let (_, head) = chips(&panel, &ctx, 0, &pane);
    assert_eq!(
        head.anchor, None,
        "a free deck drew an anchor, and free is the absence of a transport rather than a setting"
    );
    assert!(near(head.back.min.x, head.mode.max.x + size::DECK_HEAD_GAP));
}

/// **A row that cannot hold its chips draws none of them**, which is `look`'s
/// rule one bay up: half a control is a picture of something that cannot be
/// pressed.
///
/// The narrow pane is built here rather than solved for, because the panel's
/// own minimum width is wider than this: below 990 the solve stops honouring
/// minima and scales the whole console down together, so there is no viewport
/// that reaches this state. The rule is stated anyway, and this is what says
/// it holds.
#[test]
fn a_row_too_narrow_for_its_chips_draws_none_of_them() {
    let pane = mock();
    let (panel, ctx) = console(PLAUSIBLE);
    let (pane_at, head) = chips(&panel, &ctx, 0, &pane);
    let full = pane_at.deck_head;
    // Wide enough for what is in it, and one pixel narrower than that.
    let width =
        head.forward.max.x + size::DECK_HEAD_GAP + head.composite.width() + size::DECK_HEAD_PAD_X
            - full.min.x;
    let narrowed = |w: f32| InspectorPane {
        deck_head: egui::Rect::from_min_size(full.min, egui::vec2(w, full.height())),
        ..pane_at
    };
    assert!(
        deck_head(&ctx, &narrowed(width), &pane).is_some(),
        "a row exactly wide enough for its chips drew none, so this test cannot tell a fit \
         from a refusal"
    );
    assert_eq!(
        deck_head(&ctx, &narrowed(width - 1.0), &pane),
        None,
        "a row one pixel too narrow drew its chips anyway, and `inspector_into`'s clip is what \
         would cut them in half"
    );
}

// ---------------------------------------------------------------------------
// The claim rule
// ---------------------------------------------------------------------------

/// **All three controls clear every boundary's grab**, measured here off their
/// own rectangles and never inherited from the transport row's.
///
/// **The nearest boundary is the pane divider, not the one under the row.** A
/// deck head is the second row *inside* an inspector pane, so what a chip has
/// to clear sideways is the bar between the two panes and the bay's own edges,
/// and the tightest of those is `.deck-head`'s left-hand padding: the mode
/// chip starts **10** pixels in from the pane's edge, against a `GRAB` of
/// **6**. Down the row the clearance is far larger — a pane's head and the bay
/// head are above it — and that is asserted rather than assumed.
///
/// **So it fails if a chip moves, if the row's padding shrinks, or if `GRAB`
/// widens past 10** — and the last is the point: 10 is the tightest clearance
/// on this console, so the deck head is what goes first, and the fix is then
/// to change the rule in `input`, deliberately.
#[test]
fn every_control_clears_every_boundarys_grab() {
    let pane = mock();
    for viewport in [SMALLEST, PLAUSIBLE] {
        let mut panel = console(viewport).0;
        let ctx = drawn_once();
        let view = view(&pane);
        for (index, name) in PANE_NAMES.iter().enumerate() {
            let (pane_at, head) = chips(&panel, &ctx, index, &pane);
            let region = pane_at.deck_head;
            let anchor = head.anchor.expect("a beat-synced deck reads an anchor");

            let clearance = head.mode.min.x - region.min.x;
            assert!(
                near(clearance, size::DECK_HEAD_PAD_X),
                "the mode chip is {clearance} in from the pane's edge and `.deck-head`'s \
                 padding is {}",
                size::DECK_HEAD_PAD_X
            );
            assert!(
                clearance > GRAB,
                "the leftmost chip is {clearance} in from the pane's edge and a boundary grabs \
                 {GRAB} — the control is inside a boundary's grab, and `input`'s rule is what \
                 has to change"
            );

            // **And down the row, which is where a deck head is unlike every
            // control before it.** A pane's top edge is the bay's, and there
            // are two whole rows above this one: the bay head painted over the
            // region, the pane's own `.half-head`, and then this row's
            // padding. 27 + 27.5 + 5 = **59.5**, against a `GRAB` of 6. The
            // bottom is not a constant — a pane is as tall as the bay lets it
            // be — so it is asserted as a clearance rather than as a number.
            let bay = rect_of(panel.layout(), name);
            let head_top = head.mode.min.y - bay.y;
            assert!(
                near(
                    head_top,
                    size::HEAD_H + size::HALF_HEAD_H + size::DECK_HEAD_PAD_Y
                ),
                "the chips are {head_top} down from the pane's top edge, and the bay head, the \
                 pane head and this row's padding come to {}",
                size::HEAD_H + size::HALF_HEAD_H + size::DECK_HEAD_PAD_Y
            );
            let under = bay.y + bay.h - head.mode.max.y;
            assert!(
                head_top > GRAB && under > GRAB,
                "the chips have {head_top} of pane above them and {under} below, against a \
                 grab of {GRAB}"
            );

            for (target, what) in [
                (head.mode, "the sync chip"),
                (anchor, "the anchor"),
                (head.back, "the scrub's back arrow"),
                (head.forward, "the scrub's forward arrow"),
            ] {
                for probe in [
                    target.left_top(),
                    target.right_top(),
                    target.left_bottom(),
                    target.right_bottom(),
                    target.center(),
                    target.center_top(),
                    target.center_bottom(),
                ] {
                    assert!(
                        !matches!(
                            panel.layout().hit(at(probe), GRAB),
                            karakuri_layout::Hit::Divider { .. }
                        ),
                        "a boundary grabs {probe:?}, which is on {what} of pane {index}"
                    );
                    assert_eq!(
                        claim(&mut panel, &ctx, &view, at(probe)),
                        Claim::Panel,
                        "the panel does not get a press at {probe:?}, which is on {what} of \
                         pane {index}"
                    );
                }
            }
        }
    }
}

/// **The band beside the panes is still the boundary's**, which is what says
/// the clearance above is a clearance and not the grab having gone missing.
#[test]
fn the_band_the_chips_clear_is_still_a_boundarys() {
    let pane = mock();
    let (panel, ctx) = console(PLAUSIBLE);
    let (pane_at, _) = chips(&panel, &ctx, 1, &pane);
    // **Inside the pane and still the divider's**, which is what a grab *is*:
    // the band reaches `GRAB` in over whatever the pane draws at its edge, and
    // the ten pixels of `.deck-head` padding are what put the first chip past
    // it. A probe on the pane's own edge would pass with no grab at all and
    // would measure nothing.
    let half = size::DECK_HEAD_PAD_X * 0.5;
    assert!(
        half < GRAB,
        "half the row's padding is {half} and a boundary grabs {GRAB} — the probe below is no \
         longer inside the band it is meant to be inside"
    );
    let into = Point::new(pane_at.deck_head.min.x + half, pane_at.deck_head.center().y);
    assert!(
        matches!(
            panel.layout().hit(into, GRAB),
            karakuri_layout::Hit::Divider { .. }
        ),
        "{half} pixels into the second pane is not in the grab of the divider beside it, so \
         this file is no longer measuring the clearance it was written for"
    );
}

/// **A control claims what it acts on and no more.** The gaps between the
/// chips are not controls, and neither is an arrow on a deck the scrub is
/// inert on.
///
/// **The fold moved from the second list to the first on 2026-09-09**, which
/// is the whole of what ADR-0314 changed about this row: it was drawn and
/// claimed nothing, on the argument that layering is a build decision the
/// engine has no setter for. It has none, and a press re-aims the slot
/// instead. **The two build chips joined it the same day** (ADR-0328), and
/// they are the same shape one field of the aim along.
#[test]
fn nothing_beside_the_six_controls_is_claimed() {
    let pane = mock();
    let (mut panel, ctx) = console(PLAUSIBLE);
    let view = view(&pane);
    let (_, head) = chips(&panel, &ctx, 0, &pane);
    let anchor = head.anchor.expect("a beat-synced deck reads an anchor");
    let aim = head.aim.expect("a wide pane draws the two build chips");

    for (probe, what) in [
        (head.mode.center(), "the sync chip"),
        (anchor.center(), "the anchor"),
        (head.back.center(), "the back arrow"),
        (head.forward.center(), "the forward arrow"),
        (aim.size.center(), "the capacity chip"),
        (aim.salt.center(), "the `re-salt` capsule"),
        (head.composite.center(), "the fold"),
    ] {
        assert_eq!(
            claim(&mut panel, &ctx, &view, at(probe)),
            Claim::Panel,
            "{what} is not claimed by the panel"
        );
    }
    for (probe, what) in [
        (
            egui::pos2(
                head.mode.max.x + size::DECK_HEAD_GAP * 0.5,
                head.mode.center().y,
            ),
            "the gap between the mode chip and the anchor",
        ),
        (
            egui::pos2(
                head.back.max.x + size::SCRUB_GAP * 0.5,
                head.back.center().y,
            ),
            "the gap between the two arrows",
        ),
        (
            egui::pos2(
                aim.salt.max.x + size::DECK_HEAD_GAP * 0.5,
                aim.salt.center().y,
            ),
            "the gap between the `re-salt` capsule and the fold",
        ),
    ] {
        assert_eq!(
            claim(&mut panel, &ctx, &view, at(probe)),
            Claim::Egui,
            "{what} is being claimed as a control the panel acts on"
        );
        assert!(!head.owns(at(probe)));
    }
}

/// **An inert scrub is drawn and not claimed.** A deck that is not beat-synced
/// keeps both arrows — the row would move under the hand every time the chip
/// beside them was pressed otherwise — and a press on one asks for nothing.
#[test]
fn an_inert_scrub_keeps_its_shape_and_claims_nothing() {
    for sync in [Sync::Free, Sync::Tempo] {
        let pane = at_sync(sync);
        let (mut panel, ctx) = console(PLAUSIBLE);
        let view = view(&pane);
        let (_, head) = chips(&panel, &ctx, 0, &pane);
        let live = chips(&panel, &ctx, 0, &mock()).1;
        assert!(
            head.back.width() > 0.0 && head.forward.width() > 0.0,
            "an inert scrub drew no arrows under `{}`, and it is meant to keep its shape",
            sync.name()
        );
        if sync == Sync::Tempo {
            assert_eq!(
                (head.back.size(), head.forward.size()),
                (live.back.size(), live.forward.size()),
                "an inert arrow is a different box from a live one, so the row moves when the \
                 mode changes"
            );
        }
        for arrow in [head.back, head.forward] {
            assert_eq!(
                head.scrub(at(arrow.center())),
                None,
                "an arrow asked for a scrub under `{}`, and only beat sync reads the offset",
                sync.name()
            );
            assert!(!head.owns(at(arrow.center())));
            assert_eq!(
                claim(&mut panel, &ctx, &view, at(arrow.center())),
                Claim::Egui,
                "an inert arrow claimed a press under `{}`",
                sync.name()
            );
        }
    }
}

/// **Before anything has been drawn there is no control**, and neither is
/// there without a deck: a chip is as wide as the word in it, and a pane is
/// what a deck head heads.
#[test]
fn a_control_that_has_not_been_drawn_is_not_there() {
    let pane = mock();
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();

    let fresh = egui::Context::default();
    assert_eq!(fresh.cumulative_pass_nr(), 0);
    let pane_at = inspector(panel.layout(), 0, &pane, 0.0).expect("a pane with room in it");
    assert_eq!(deck_head(&fresh, &pane_at, &pane), None);

    let ctx = drawn_once();
    let empty = View::new(Room::Day);
    assert!(empty.inspector.is_empty());
    let head = deck_head(&ctx, &pane_at, &pane).expect("a deck head");
    assert_eq!(
        claim(&mut panel, &ctx, &empty, at(head.mode.center())),
        Claim::Egui,
        "a console with no deck behind it claimed a press on a chip nothing draws"
    );
}

// ---------------------------------------------------------------------------
// What a press asks for
// ---------------------------------------------------------------------------

/// **The chip cycles all three modes and comes back, once each** — which is
/// `tests/blend.rs`'s measurement over the deck's clock, and is what keeps
/// `SYNCS` and the cycle from drifting apart.
///
/// The step is asserted **as an operation**, so what is checked is the thing a
/// map or a model would be offered: `SetSync` naming the destination, never a
/// step.
#[test]
fn the_chip_cycles_every_mode_once_and_wraps() {
    let (panel, ctx) = console(PLAUSIBLE);
    let mut seen = Vec::new();
    let mut at_now = Sync::Free;
    for _ in 0..EVERY.len() {
        let pane = at_sync(at_now);
        let (_, head) = chips(&panel, &ctx, 0, &pane);
        let asked = head
            .sync(at(head.mode.center()))
            .expect("a press on the sync chip");
        let Operation::SetSync { deck, sync } = asked else {
            panic!("a press on the sync chip asked for {asked:?}")
        };
        assert_eq!(deck, pane.deck as u8, "the press named another deck");
        assert_ne!(
            sync, at_now,
            "the chip asked for the mode that is already running, and nothing this deck's \
             material refuses"
        );
        seen.push(sync);
        at_now = sync;
    }
    assert_eq!(
        at_now,
        Sync::Free,
        "three presses from `free` did not come back to it, so the cycle is not three long"
    );
    for sync in EVERY {
        assert_eq!(
            seen.iter().filter(|seen| **seen == sync).count(),
            1,
            "`{}` is reached {} times in one turn of the cycle",
            sync.name(),
            seen.iter().filter(|seen| **seen == sync).count()
        );
    }
}

/// **The cycle skips a mode this material cannot honour rather than offering
/// it**, which is the one thing this cycle has that the mixer's two do not —
/// and the reading it skips on is the engine's, handed in.
#[test]
fn the_cycle_skips_a_mode_the_material_cannot_honour() {
    let (panel, ctx) = console(PLAUSIBLE);
    let asked = |pane: &Pane| {
        let (_, head) = chips(&panel, &ctx, 0, pane);
        match head.sync(at(head.mode.center())) {
            Some(Operation::SetSync { sync, .. }) => sync,
            other => panic!("a press on the sync chip asked for {other:?}"),
        }
    };

    // Material that accumulates: `beat` is refused, so `tempo` steps past it
    // to `free` rather than offering a mode the engine would turn down.
    let accumulates = Pane {
        allows: [true, true, false],
        ..at_sync(Sync::Tempo)
    };
    assert_eq!(asked(&accumulates), Sync::Free);
    assert_eq!(
        asked(&Pane {
            sync: Sync::Free,
            ..accumulates.clone()
        }),
        Sync::Tempo
    );

    // Material that accumulates **and** reads the beat: only `free` is left,
    // so the one mode a cycle can reach is the mode it is in — which
    // re-anchors, and is the one case where re-anchoring does nothing.
    let both = Pane {
        allows: [true, false, false],
        ..at_sync(Sync::Free)
    };
    assert_eq!(
        asked(&both),
        Sync::Free,
        "a cycle over one available mode landed somewhere else, so it offered a mode the deck \
         would refuse"
    );
}

/// **The anchor asks for the mode the deck is already in**, which re-anchors —
/// and it is the one thing the chip beside it can never say, because a cycle
/// starts at the mode *after* the one you are on.
#[test]
fn the_anchor_asks_for_the_mode_the_deck_is_already_in() {
    let (panel, ctx) = console(PLAUSIBLE);
    for sync in [Sync::Tempo, Sync::Beat] {
        let pane = at_sync(sync);
        let (_, head) = chips(&panel, &ctx, 0, &pane);
        let anchor = head.anchor.expect("a synced deck reads an anchor");
        assert_eq!(
            head.reanchor(at(anchor.center())),
            Some(Operation::SetSync {
                deck: pane.deck as u8,
                sync
            }),
            "the anchor asked for something other than the mode the deck is in"
        );
        // And the chip cannot: with nothing refused it always steps away.
        let stepped = match head.sync(at(head.mode.center())) {
            Some(Operation::SetSync { sync, .. }) => sync,
            other => panic!("a press on the sync chip asked for {other:?}"),
        };
        assert_ne!(
            stepped, sync,
            "the cycle reached the mode the deck is in, so the anchor is not the only control \
             that can re-anchor and this file's reason for it is wrong"
        );
    }
}

/// **A free deck has no anchor and nothing to re-ask for.** Free reads no
/// anchor at all, so there is nothing a press could re-anchor to.
#[test]
fn a_free_deck_has_nothing_to_re_anchor() {
    let pane = at_sync(Sync::Free);
    let (panel, ctx) = console(PLAUSIBLE);
    let (pane_at, head) = chips(&panel, &ctx, 0, &pane);
    assert_eq!(head.anchor, None);
    for probe in [
        head.mode.right_top(),
        egui::pos2(head.mode.max.x + 1.0, head.mode.center().y),
        pane_at.deck_head.center(),
    ] {
        assert_eq!(head.reanchor(at(probe)), None);
    }
}

/// **An arrow asks for a quarter beat, and which arrow it was is the sign** —
/// the one control on this panel that moves by an amount, because there is no
/// destination in the vocabulary for it to name.
#[test]
fn the_arrows_ask_for_a_quarter_beat_each_way() {
    let pane = mock();
    let (panel, ctx) = console(PLAUSIBLE);
    let (_, head) = chips(&panel, &ctx, 0, &pane);
    assert_eq!(
        head.scrub(at(head.back.center())),
        Some(Operation::ScrubDeck {
            deck: pane.deck as u8,
            beats: -SCRUB_BEATS
        })
    );
    assert_eq!(
        head.scrub(at(head.forward.center())),
        Some(Operation::ScrubDeck {
            deck: pane.deck as u8,
            beats: SCRUB_BEATS
        })
    );
    assert_eq!(
        SCRUB_BEATS, 0.25,
        "the row this fills is titled *Scrub a deck a quarter beat*"
    );

    // **The amount does not depend on where the offset already is**, which is
    // what *relative* means and is why the record needs a reading.
    let far = Pane {
        scrub_beats: -12.75,
        ..pane.clone()
    };
    let (_, moved) = chips(&panel, &ctx, 0, &far);
    assert_eq!(
        moved.scrub(at(moved.forward.center())),
        Some(Operation::ScrubDeck {
            deck: far.deck as u8,
            beats: SCRUB_BEATS
        })
    );
}

/// **A press is one of the six or none**, so no point on the row asks two
/// questions.
#[test]
fn a_press_is_one_control_or_none() {
    let pane = mock();
    let (panel, ctx) = console(PLAUSIBLE);
    let (pane_at, head) = chips(&panel, &ctx, 0, &pane);
    let anchor = head.anchor.expect("an anchor");
    let aim = head.aim.expect("a wide pane draws the two build chips");
    for probe in [
        head.mode.center(),
        anchor.center(),
        head.back.center(),
        head.forward.center(),
        aim.size.center(),
        aim.salt.center(),
        head.composite.center(),
        pane_at.deck_head.left_center(),
        pane_at.deck_head.right_center(),
    ] {
        let asked = [
            head.sync(at(probe)),
            head.reanchor(at(probe)),
            head.scrub(at(probe)),
            head.resized(at(probe)),
            head.re_salted(at(probe)),
            head.compositing(at(probe)),
        ];
        let answered = asked.iter().filter(|a| a.is_some()).count();
        assert!(
            answered <= 1,
            "a press at {probe:?} asked {answered} of the six controls for something"
        );
        assert_eq!(
            head.owns(at(probe)),
            answered == 1,
            "`owns` and what a press asks for disagree at {probe:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// The route a window loop takes
// ---------------------------------------------------------------------------

/// **The fold names the layering the deck is not in, and never a step.**
///
/// Two panes, one compositing and one overdrawing, and the same chip on each:
/// what leaves is `SetCompositing` carrying the destination, computed from the
/// state the frame that laid the row out drew — which is
/// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
/// and `Mixer::blend`'s division. Nothing in the vocabulary says *toggle*, and
/// a control that could only step would leave two surfaces disagreeing about
/// where the deck is.
///
/// **It does not claim reachability**: whether the press re-aims the slot is
/// `crates/karakuri/src/main.rs`'s, which this crate cannot depend on
/// (ADR-0156). ADR-0314 is the record.
#[test]
fn the_fold_asks_for_the_layering_the_deck_is_not_in() {
    for composite in [false, true] {
        let pane = Pane {
            composite,
            ..mock()
        };
        let (mut panel, ctx) = console(PLAUSIBLE);
        let view = view(&pane);
        let (_, head) = chips(&panel, &ctx, 0, &pane);
        let probe = head.composite.center();

        assert_eq!(
            claim(&mut panel, &ctx, &view, at(probe)),
            Claim::Panel,
            "the fold is drawn and not claimed on a deck that composites: {composite}"
        );
        assert_eq!(
            head.compositing(at(probe)),
            Some(Operation::SetCompositing {
                deck: pane.deck as u8,
                compositing: !composite,
            }),
            "a press on the fold of a deck compositing {composite} did not ask for the other \
             layering"
        );
        // The other three answer nothing for it, so the destination cannot be
        // reached twice by one press — `a_press_is_one_control_or_none` is the
        // same fact over the whole row.
        assert_eq!(head.sync(at(probe)), None);
        assert_eq!(head.reanchor(at(probe)), None);
        assert_eq!(head.scrub(at(probe)), None);
    }
}

/// **The capacity chip steps the powers of two inside the declared range, once
/// each, and wraps through the bottom** — and what leaves is the number it
/// arrived at rather than a step, which is what a second surface would need to
/// agree with it (P-0090).
///
/// **The whole ladder is walked** rather than one press asserted, because a
/// cycle that had lost a rung, repeated one or stopped at the top would all
/// pass a single-press test.
#[test]
fn the_capacity_chip_steps_every_rung_once_and_wraps() {
    let (panel, ctx) = console(PLAUSIBLE);
    let mut at_capacity = LADDER[0];
    let mut visited = Vec::new();
    for _ in 0..LADDER.len() {
        let pane = Pane {
            aimed: Some(Aimed {
                capacity: at_capacity,
                ..aimed()
            }),
            ..mock()
        };
        let (_, head) = chips(&panel, &ctx, 0, &pane);
        let aim = head.aim.expect("a wide pane draws the two build chips");
        let asked = head.resized(at(aim.size.center()));
        let Some(Operation::SetProperty {
            deck,
            property: karakuri_operation::Property::Capacity { elements },
        }) = asked
        else {
            panic!("a press on the capacity chip at {at_capacity} asked for {asked:?}");
        };
        assert_eq!(deck, pane.deck as u8);
        visited.push(elements);
        at_capacity = elements;
    }
    let mut once = visited.clone();
    once.sort_unstable();
    once.dedup();
    assert_eq!(
        once,
        LADDER.to_vec(),
        "walking the ladder from its bottom visited {visited:?}, which is not every rung once"
    );
    assert_eq!(
        at_capacity, LADDER[0],
        "a full walk did not come back to the bottom, so the wrap goes somewhere else"
    );
}

/// **A slot running at a number that is not on the ladder steps *up*, not back
/// to the bottom.** `examples/beat_strands.kir` declares `capacity [4096,
/// 1048576] = 81920`, and a Set file may record anything the range allows, so
/// this is an ordinary state rather than a corner: a press that read as *one
/// step* and dropped the slot from 81920 to 4096 would be a control that
/// reallocated every element buffer in the wrong direction.
#[test]
fn a_capacity_off_the_ladder_steps_up() {
    let (panel, ctx) = console(PLAUSIBLE);
    let pane = Pane {
        aimed: Some(Aimed {
            capacity: 81_920,
            stated: true,
            capacities: LADDER.to_vec(),
            salt: NEXT_SALT,
        }),
        ..mock()
    };
    let (_, head) = chips(&panel, &ctx, 0, &pane);
    let aim = head.aim.expect("a wide pane draws the two build chips");
    assert_eq!(
        head.resized(at(aim.size.center())),
        Some(Operation::SetProperty {
            deck: pane.deck as u8,
            property: karakuri_operation::Property::Capacity { elements: 131_072 },
        })
    );
}

/// **A chip with nowhere to step is drawn and claims nothing**, which is the
/// inert scrub's arrangement one control along: two geometries whose declared
/// ranges do not overlap have no capacity one re-aim could send, and the number
/// the slot is running at is still worth reading.
#[test]
fn a_capacity_with_no_shared_range_is_drawn_and_claims_nothing() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let pane = Pane {
        aimed: Some(Aimed {
            capacities: Vec::new(),
            ..aimed()
        }),
        ..mock()
    };
    let view = view(&pane);
    let (_, head) = chips(&panel, &ctx, 0, &pane);
    let aim = head.aim.expect("the number is still drawn");
    assert!(
        aim.size.width() > 0.0,
        "a chip with nothing to step to lost its shape as well as its press"
    );
    assert_eq!(aim.resize, None);
    assert_eq!(head.resized(at(aim.size.center())), None);
    assert!(!head.owns(at(aim.size.center())));
    assert_eq!(
        claim(&mut panel, &ctx, &view, at(aim.size.center())),
        Claim::Egui,
        "a chip that asks for nothing is being claimed as a control the panel acts on"
    );
}

/// **The `re-salt` capsule asks for the salt it was handed**, and that is the
/// whole assertion: a console that derived one would be producing a picture a
/// later run could reproduce only by accident
/// ([P-0092](../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)).
/// The number here is arbitrary on purpose — nothing in this crate can compute
/// it, so nothing in this crate can agree with a computation by luck.
#[test]
fn the_re_salt_capsule_asks_for_the_salt_it_was_handed() {
    let (panel, ctx) = console(PLAUSIBLE);
    let pane = mock();
    let (_, head) = chips(&panel, &ctx, 0, &pane);
    let aim = head.aim.expect("a wide pane draws the two build chips");
    assert_eq!(
        head.re_salted(at(aim.salt.center())),
        Some(Operation::SetProperty {
            deck: pane.deck as u8,
            property: karakuri_operation::Property::Seed { salt: NEXT_SALT },
        })
    );
    // The other five answer nothing for it, which is the same fact
    // `a_press_is_one_control_or_none` states over the whole row.
    assert_eq!(head.resized(at(aim.salt.center())), None);
    assert_eq!(head.compositing(at(aim.salt.center())), None);
}

/// **The whole route, as the window loop drives it**: `claim` first, then the
/// pane and the head derived a second time, then the operation.
///
/// **It does not claim reachability.** Whether a claimed press becomes one of
/// these operations is `crates/karakuri/src/main.rs`'s, which this crate
/// cannot depend on (ADR-0156).
#[test]
fn a_press_reaches_both_operations_the_way_the_window_loop_reaches_them() {
    let pane = mock();
    let (mut panel, ctx) = console(PLAUSIBLE);
    let view = view(&pane);
    let (_, laid) = chips(&panel, &ctx, 0, &pane);
    let anchor = laid.anchor.expect("an anchor");

    for (probe, expected) in [
        (
            laid.mode.center(),
            Operation::SetSync {
                deck: pane.deck as u8,
                sync: Sync::Free,
            },
        ),
        (
            anchor.center(),
            Operation::SetSync {
                deck: pane.deck as u8,
                sync: Sync::Beat,
            },
        ),
        (
            laid.back.center(),
            Operation::ScrubDeck {
                deck: pane.deck as u8,
                beats: -SCRUB_BEATS,
            },
        ),
        (
            laid.forward.center(),
            Operation::ScrubDeck {
                deck: pane.deck as u8,
                beats: SCRUB_BEATS,
            },
        ),
        (
            laid.composite.center(),
            Operation::SetCompositing {
                deck: pane.deck as u8,
                compositing: !pane.composite,
            },
        ),
    ] {
        assert_eq!(
            claim(&mut panel, &ctx, &view, at(probe)),
            Claim::Panel,
            "`claim` gives a press at {probe:?} to `egui`, so no route into the operation \
             exists however the control is drawn"
        );
        let again = inspector(panel.layout(), 0, &pane, 0.0)
            .and_then(|at| deck_head(&ctx, &at, &pane))
            .expect("the same head `claim` hit-tested");
        let asked = again
            .sync(at(probe))
            .or_else(|| again.reanchor(at(probe)))
            .or_else(|| again.scrub(at(probe)))
            .or_else(|| again.compositing(at(probe)))
            .expect("a press the panel claimed on a control it draws asks for something");
        assert_eq!(
            asked, expected,
            "the press was claimed and asked for something else, so the control that claims a \
             press and the control that acts on it have come apart"
        );
    }
}
