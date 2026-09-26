use super::mixer_common::*;

/// Verifies strip areas are claimed for panel interaction/drops while alleys yield to egui (ADR-0176).
#[test]
fn the_whole_strip_is_claimed_and_the_alley_between_two_is_not() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let strips = mock_strips();
    let bay = bay(&panel, &ctx, &strips);
    let at = bay.strip(0);
    let trim = at.trim_at(mock().gain);
    let fader = at.fader_at(mock().opacity);

    // The mock's trim is at 0.72 and its fader at 1.00, so the far end of the
    // trim and the floor of the fader are both track and neither is knob.
    let track_end = egui::pos2(at.trim.max.x - 1.0, at.trim.center().y);
    let track_floor = egui::pos2(at.fader.center().x, at.fader.max.y - 1.0);

    // The five controls and the ground between them, and the answer is the
    // same for all of them: a press anywhere in the column is the panel's.
    let ground = [
        (at.rect.center(), "the strip"),
        (at.name.center(), "the name"),
        (track_end, "the trim's track, past the knob"),
        (track_floor, "the fader's track, below the knob"),
        (at.meter.center(), "the meter"),
        (at.num.center(), "the number"),
    ];
    let controls = [
        (trim.knob.center(), "the trim's knob"),
        (fader.knob.center(), "the fader's knob"),
        (at.blend.center(), "the blend chip"),
        (at.tally.center(), "the tally chip"),
        (at.mask.center(), "the mask mini"),
    ];
    for (probe, what) in controls.iter().chain(ground.iter()) {
        assert_eq!(
            claim(&mut panel, &ctx, &showing(&strips), point(*probe)),
            Claim::Panel,
            "{what} is not being claimed, so a press there reaches nothing at all — \
             `egui` owns no widget anywhere on this console"
        );
    }

    // **Which control it is is still the derivation's**, and it has to be
    // asked here rather than left to the claim: the strip's rectangle contains
    // every one of the five, so `Claim::Panel` at a knob no longer says the
    // knob was hit-tested.
    assert!(
        bay.grab(point(trim.knob.center())).is_some(),
        "the trim's knob is claimed by the strip around it and by nothing of its own"
    );
    assert!(
        bay.grab(point(fader.knob.center())).is_some(),
        "the fader's knob is claimed by the strip around it and by nothing of its own"
    );
    assert!(
        bay.blend(point(at.blend.center())).is_some(),
        "the blend chip is claimed by the strip around it and by nothing of its own"
    );
    assert!(
        bay.tally(point(at.tally.center())).is_some(),
        "the tally chip is claimed by the strip around it and by nothing of its own"
    );
    assert!(
        bay.mask(point(at.mask.center())).is_some(),
        "the mask mini is claimed by the strip around it and by nothing of its own"
    );

    // And on the ground none of the four answers, so what is left over is the
    // strip — the deck this column is, named as an operation.
    for (probe, what) in ground {
        let p = point(probe);
        assert!(
            bay.grab(p).is_none()
                && bay.blend(p).is_none()
                && bay.tally(p).is_none()
                && bay.mask(p).is_none(),
            "{what} is one of the four controls inside the column, so it is not the \
             leftover the selection is made of"
        );
        assert_eq!(
            bay.select(p),
            Some(Operation::SelectDeck { deck: 0 }),
            "{what} is in deck A's strip and selects no deck"
        );
    }

    // **The alley between two strips is nobody's**: `.mixer-strips` has a
    // `gap`, it is a real place to press, and a claim there would be the panel
    // taking an event for a deck the pointer is not on.
    let alley = {
        let a = bay.selected(0).expect("a strip for deck A");
        let b = bay.selected(1).expect("a strip for deck B");
        egui::pos2((a.max.x + b.min.x) * 0.5, a.center().y)
    };
    assert!(
        bay.select(point(alley)).is_none(),
        "the alley is inside a strip, so the assertion under it is asking nothing"
    );
    assert_eq!(
        claim(&mut panel, &ctx, &showing(&strips), point(alley)),
        Claim::Egui,
        "the alley between two strips is being claimed, and there is no deck there to \
         address the keys to"
    );

    // The two points that are track rather than knob have to actually be off
    // the knob, or the paragraph above is asserting nothing.
    assert!(!trim.knob.contains(track_end));
    assert!(!fader.knob.contains(track_floor));
    // And the mask mini has to actually be off the blend chip, or the list
    // above would be asserting one control twice and never asking about the
    // other.
    assert!(!at.blend.contains(at.mask.center()));

    // The boundary **under** the bay still has its grab, which is what says
    // the answers above are about the controls rather than the rule having
    // gone missing.
    let region = rect_of(panel.layout(), "mixer");
    let below = Point::new(region.x + region.w * 0.5, region.y + region.h);
    assert_eq!(
        claim(&mut panel, &ctx, &showing(&strips), below),
        Claim::Panel,
        "the bottom edge of the mixer is not in the grab of the boundary under it, so \
         this test is no longer measuring what it was written for"
    );
}

// ---------------------------------------------------------------------------
// The values are the harness's
// ---------------------------------------------------------------------------

/// Verifies mixer bay state derives purely from arguments with no internal state persistence.
#[test]
fn the_values_are_the_harnesss_and_are_stored_nowhere() {
    let (panel, ctx) = console(PLAUSIBLE);
    let one = mock_strips();
    let two = vec![mock()];

    let first = mixer(&ctx, panel.layout(), &one).expect("a bay");
    let other = mixer(&ctx, panel.layout(), &two).expect("a bay");
    assert_ne!(
        first, other,
        "two different decks drew the same bay, so something in it is not coming from the \
         argument"
    );
    assert_eq!(first.count(), 3);
    assert_eq!(other.count(), 1);
    // The bay carries what it was measured from, so whoever measured and
    // whoever paints are one statement.
    assert_eq!(first.strips, one.as_slice());

    // Asked again with the first, and it is the first answer: nothing was
    // written down in between.
    assert_eq!(
        mixer(&ctx, panel.layout(), &one).expect("a bay"),
        first,
        "the bay remembered the deck it was last asked about"
    );

    // And the view holds exactly what a caller put there — a plain field, not
    // a copy the console maintains.
    let mut view = View::new(Room::Day);
    assert!(view.mixer.is_empty());
    view.mixer = one.clone();
    assert_eq!(view.mixer, one);
    view.mixer.clear();
    assert!(view.mixer.is_empty());
}

/// Verifies deck selection draws a single solid ring surrounding the selected strip.
#[test]
fn the_selection_is_one_ring_and_it_is_round_the_strip_it_names() {
    let (mut panel, _ctx) = console(PLAUSIBLE);
    let region = rect_of(panel.layout(), "mixer");
    let row = strips_row(region);

    // The strips twice: once for the view to draw from and once for the
    // derivation to be asked about, so the boxes this test names are not a
    // borrow of the thing it is about to move the selection on.
    let strips: Vec<Strip> = std::iter::repeat_with(mock).take(DECKS).collect();
    let mut view = View::new(Room::Day);
    view.mixer = strips.clone();
    let ctx = drawn_once();
    let at = bay(&panel, &ctx, &strips);

    for deck in 0..DECKS {
        assert!(
            view.select(deck as u8) || deck == 0,
            "deck {deck} could not be selected with {DECKS} strips"
        );
        let drawn = shapes_inside(&mut view, &mut panel, row);
        assert_eq!(
            ringed(&drawn, &at),
            vec![deck],
            "the selection is deck {deck} and the ring is somewhere else, or there is more than \
             one of it"
        );
    }
}

/// Verifies that deck indices without strips cannot be selected (refuses instead of clamping; ADR-0305).
#[test]
fn a_deck_with_no_strip_cannot_be_selected() {
    let mut view = View::new(Room::Day);
    assert!(
        !view.select(0),
        "a console with no deck behind it selected one"
    );

    view.mixer = std::iter::repeat_with(mock).take(2).collect();
    assert!(!view.select(1) || view.selection() == 1);
    assert_eq!(view.selection(), 1);
    for deck in 2..DECKS as u8 {
        assert!(
            !view.select(deck),
            "deck {deck} was selected on a two-slot deck"
        );
        assert_eq!(
            view.selection(),
            1,
            "the refused press moved the selection anyway"
        );
    }
}

/// Verifies pressing strip body selects the deck while control clicks do not trigger selection.
#[test]
fn a_press_on_a_strip_selects_that_deck() {
    let (panel, ctx) = console(PLAUSIBLE);
    let strips = mock_strips();
    let at = bay(&panel, &ctx, &strips);

    for index in 0..at.count() {
        let box_ = at.strip(index);
        assert_eq!(
            at.select(point(box_.name.center())),
            Some(karakuri_operation::Operation::SelectDeck { deck: index as u8 }),
            "a press on deck {index}'s name did not select it"
        );
        // And every one of the four questions asked before it answers `None`
        // there, which is what makes the ordering safe rather than lucky.
        let p = point(box_.name.center());
        assert_eq!(at.grab(p), None);
        assert_eq!(at.blend(p), None);
        assert_eq!(at.tally(p), None);
        assert_eq!(at.mask(p), None);
    }

    // Outside every strip there is no deck to select. The row's own left edge
    // is `.mixer-strips`' padding, which belongs to the bay and not to a
    // strip.
    let outside = egui::pos2(
        at.strip(0).rect.min.x - size::STRIPS_PAD * 0.5,
        at.strip(0).rect.center().y,
    );
    assert_eq!(at.select(point(outside)), None);
}
