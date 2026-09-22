use super::deck_head_common::*;

// ---------------------------------------------------------------------------
// Where the controls are
// ---------------------------------------------------------------------------

/// The head is `.deck-head`'s own flex row: the mode chip against the left-hand
/// padding, the anchor one gap along, the two arrows one gap after that with
/// `.scrub`'s own 3 between them, and the fold hard against the right-hand
/// padding.
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

/// The two build chips are measured leftwards from the fold, which is what
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

/// A pane too narrow for the two build chips keeps the five that were here
/// before them, which is the one place this row answers *a control that does
/// not fit is no control at all* by dropping part of the row rather than all of
/// it.
///
/// The measurement is the argument. An Inspector pane at the console's declared
/// minimum window is 237.5 pixels wide and the row needs about 296 for all
/// seven, so a row that took all of them or none would draw nothing at the
/// width this arrangement claims to work at — trading two controls that were
/// never there for four that were. The threshold is a window of about 1360 with
/// two panes open, and the console page says so.
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

/// A free deck draws no anchor, and the row closes up rather than leaving a
/// hole where one would have been — which is what a flex row does and is why
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

/// A row that cannot hold its chips draws none of them, which is `look`'s rule
/// one bay up: half a control is a picture of something that cannot be pressed.
///
/// The narrow pane is built here rather than solved for, because the panel's
/// own minimum width is wider than this: below 990 the solve stops honouring
/// minima and scales the whole console down together, so there is no viewport
/// that reaches this state. The rule is stated anyway, and this is what says it
/// holds.
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

/// All three controls clear every boundary's grab, measured here off their own
/// rectangles and never inherited from the transport row's.
///
/// The nearest boundary is the pane divider, not the one under the row. A deck
/// head is the second row *inside* an inspector pane, so what a chip has to
/// clear sideways is the bar between the two panes and the bay's own edges, and
/// the tightest of those is `.deck-head`'s left-hand padding: the mode chip
/// starts 10 pixels in from the pane's edge, against a `GRAB` of 6. Down the
/// row the clearance is far larger — a pane's head and the bay head are above
/// it — and that is asserted rather than assumed.
///
/// So it fails if a chip moves, if the row's padding shrinks, or if `GRAB`
/// widens past 10 — and the last is the point: 10 is the tightest clearance on
/// this console, so the deck head is what goes first, and the fix is then to
/// change the rule in `input`, deliberately.
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

/// The band beside the panes is still the boundary's, which is what says the
/// clearance above is a clearance and not the grab having gone missing.
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

/// A control claims what it acts on and no more. The gaps between the chips are
/// not controls, and neither is an arrow on a deck the scrub is inert on.
///
/// The fold moved from the second list to the first on 2026-09-09, which is the
/// whole of what ADR-0314 changed about this row: it was drawn and claimed
/// nothing, on the argument that layering is a build decision the engine has no
/// setter for. It has none, and a press re-aims the slot instead. The two build
/// chips joined it the same day (ADR-0328), and they are the same shape one
/// field of the aim along.
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

/// An inert scrub is drawn and not claimed. A deck that is not beat-synced
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

/// Before anything has been drawn there is no control, and neither is there
/// without a deck: a chip is as wide as the word in it, and a pane is what a
/// deck head heads.
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
