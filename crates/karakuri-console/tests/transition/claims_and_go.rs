use super::transition_common::*;

// ---------------------------------------------------------------------------
// A control claims what it acts on and no more
// ---------------------------------------------------------------------------

/// A press elsewhere in the bay asks this row for nothing and is not claimed by
/// it.
///
/// `input`'s rule 4: *"a control claims what it acts on and no more."* The
/// ground either side of the pills, the gaps between them and the card under
/// the row are all painted by the console and none of them is a control, so
/// `egui` gets the event — which owns no widget there either, so the two
/// answers are the same nothing, arrived at without the panel claiming a press
/// it would throw away.
///
/// Both halves, because either alone is satisfiable by the wrong code. A row
/// that emitted nothing but was claimed would take presses it does nothing
/// with; one that emitted from anywhere would change what the next wipe means
/// from a press on the card beside it.
///
/// A strip's own controls are asserted the other way round, exactly as
/// `blend.rs` does with the tally and the mask: the panel claims each, and this
/// row must answer nothing for it.
#[test]
fn a_press_off_the_pills_asks_for_nothing_and_is_not_claimed() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let strips = strips();
    let settings = place(5, 0, 2);
    let mut showing = showing_at(&strips, settings);
    let at = row(&panel, &ctx, settings);
    let bay = mixer(&ctx, panel.layout(), &strips).expect("the strips");

    let probes = [
        // Two pixels left of the first pill, and **not** the middle of
        // `.xfade`'s own padding: that point is five in from the bay's left
        // edge and inside the pane boundary's `GRAB`, so rule 3 would answer
        // *the panel's* before this row was asked anything at all.
        (
            egui::pos2(at.shape.min.x - 2.0, at.shape.center().y),
            "the padding left of the first pill",
        ),
        (
            egui::pos2(at.shape.max.x + size::XROW_GAP * 0.5, at.shape.center().y),
            "the gap between the shape and the quantum",
        ),
        (
            egui::pos2(
                at.quantum.max.x + size::XROW_GAP * 0.5,
                at.quantum.center().y,
            ),
            "the gap between the quantum and the length",
        ),
        (
            egui::pos2(at.length.max.x + 4.0, at.length.center().y),
            "the card right of the last pill",
        ),
        (
            egui::pos2(at.shape.center().x, at.rect.min.y + size::HAIRLINE * 0.5),
            "the rule along the top of the row",
        ),
        (
            egui::pos2(at.shape.center().x, at.shape.max.y + 2.0),
            "the padding under the pills",
        ),
    ];
    for (probe, what) in probes {
        for pill in [at.shape, at.quantum, at.length] {
            assert!(
                !pill.contains(probe),
                "{what} is inside a pill, so this probe asserts nothing"
            );
        }
        assert!(!at.owns(point(probe)), "{what} was claimed by the row");
        assert_eq!(
            claim(&mut panel, &ctx, &showing, point(probe)),
            Claim::Egui,
            "{what} is being claimed as a control the panel acts on"
        );
    }

    // **The strip above, which is a control and is not this row's.** A hit
    // test that reached up would change what the next wipe means from a press
    // meant for a blend mode.
    for (probe, what) in [
        (bay.strip(0).blend.center(), "a strip's blend chip"),
        (bay.strip(0).mask.center(), "a strip's mask mini"),
        (bay.strip(1).name.center(), "a strip's name"),
    ] {
        assert!(!at.owns(point(probe)), "{what} was claimed by the row");
        assert_eq!(
            at.shape(point(probe)),
            None,
            "{what} asked the wipe shape to change"
        );
    }

    // The guard: the four capsules do both of the things the probes above do
    // neither of. Without this the test passes on a row that was never a
    // control at all.
    for (pill, which) in [
        (at.shape, "shape"),
        (at.quantum, "quantum"),
        (at.length, "length"),
        (at.go, "go"),
    ] {
        let on = pill.center();
        assert!(at.owns(point(on)), "the {which} pill asked for nothing");
        assert_eq!(
            claim(&mut panel, &ctx, &showing, point(on)),
            Claim::Panel,
            "the {which} pill is not the panel's, so it is drawn where it cannot be clicked"
        );
    }

    // And the row moves with the console's own value, which is what says the
    // probes above were taken against the row that was drawn: at a different
    // shape the pills are different widths.
    apply(&mut showing, place(4, 0, 2));
    let wider = row(&panel, &ctx, showing.transition());
    assert_ne!(
        wider.quantum.min.x, at.quantum.min.x,
        "`back diagonal` and `iris` laid the row out identically, so this file's widths say \
         nothing"
    );
}

/// No pill of the transition row is inside a boundary's [`GRAB`].
///
/// `input`'s rule 3 comes before rule 4, so a control under a boundary's grab
/// band is a control that cannot be clicked, with nothing on screen saying so.
/// This row is the lowest thing in the Mixer bay — 24 pixels above the boundary
/// between the mixer and the master chain — where the blend chip measured for
/// this is at the bottom of a *strip*, so it is a different clearance against
/// the same boundary and is measured rather than inherited.
///
/// It asks `Layout::hit` directly as well as `claim`, which is ADR-0185's
/// caught test: `claim` says *the panel's* for a boundary and for a control, so
/// a version of this that only asked `claim` would pass with `GRAB` widened to
/// 60.
#[test]
fn no_pill_is_inside_a_boundarys_grab() {
    let strips = strips();
    for viewport in [PLAUSIBLE, SMALLEST] {
        let (mut panel, ctx) = console(viewport);
        let settings = place(4, 1, 2);
        let showing = showing_at(&strips, settings);
        let at = row(&panel, &ctx, settings);
        for (pill, which) in [
            (at.shape, "shape"),
            (at.quantum, "quantum"),
            (at.length, "length"),
            // **The `go` capsule is the one at the other end of the row**, so
            // its clearance is off the bay's right edge rather than its left
            // and is measured rather than inherited from the three: it is
            // `.xfade`'s own `XFADE_PAD_X` of 10 against a `GRAB` of 6.
            (at.go, "go"),
        ] {
            for probe in [
                pill.left_top(),
                pill.right_top(),
                pill.left_bottom(),
                pill.right_bottom(),
                pill.center(),
            ] {
                assert!(
                    !matches!(panel.layout().hit(point(probe), GRAB), Hit::Divider { .. }),
                    "a boundary grabs {probe:?}, which is on the {which} pill — the control is \
                     dead there, and `input`'s rule 3 is what would have to change"
                );
                assert_eq!(
                    claim(&mut panel, &ctx, &showing, point(probe)),
                    Claim::Panel,
                    "the {which} pill is not the panel's at {probe:?}"
                );
            }
        }

        // The guard: the ground under the bay *is* inside a boundary's grab,
        // which is what says the answers above are the clearance rather than
        // the grab having gone missing.
        let region = rect_of(panel.layout(), "mixer");
        let below = Point::new(region.x + region.w * 0.5, region.y + region.h + GRAB * 0.5);
        assert!(
            matches!(panel.layout().hit(below, GRAB), Hit::Divider { .. }),
            "the ground under the mixer is not in the grab of the boundary there, so this test \
             is no longer measuring the clearance it was written for"
        );
        assert!(
            !at.owns(below),
            "a point in the ground under the bay pressed a pill"
        );
    }
}

// ---------------------------------------------------------------------------
// The `go` capsule
// ---------------------------------------------------------------------------

/// The `go` capsule is at the right end of the row, which is what `.sep`'s
/// `flex: 1` puts it — measured back from `.xfade`'s own padding on that side
/// rather than forward from the length pill.
///
/// The three settings grow with their words and this one does not move, which
/// is the property a separator has and a fourth pill laid end to end would not:
/// at `no shape · now · cut` and at `back diagonal · next beat · 8 beats` the
/// capsule is in the same place, and the gap between it and the length pill is
/// what changes.
#[test]
fn the_go_capsule_sits_against_the_rows_right_padding() {
    let (panel, ctx) = console(PLAUSIBLE);
    let narrow = row(&panel, &ctx, place(0, 2, 3));
    let wide = row(&panel, &ctx, place(4, 1, 2));

    for (at, which) in [(narrow, "the narrowest words"), (wide, "the widest")] {
        assert!(
            near(at.go.max.x, at.rect.max.x - size::XFADE_PAD_X),
            "with {which} the `go` capsule does not finish on `.xfade`'s right padding: \
             {} against {}",
            at.go.max.x,
            at.rect.max.x - size::XFADE_PAD_X
        );
        assert!(
            near(
                at.go.min.y,
                at.rect.min.y + size::HAIRLINE + size::XFADE_PAD_TOP
            ),
            "the `go` capsule is not on the row's line"
        );
        assert!(
            near(at.go.height(), size::XPILL_H),
            "the `go` capsule is {} tall and a pill on this row is {}",
            at.go.height(),
            size::XPILL_H
        );
        assert!(
            at.length.max.x + size::XROW_GAP <= at.go.min.x + common::EPS,
            "`.sep` is not a gap: the length pill runs into the `go` capsule"
        );
    }

    // The whole of what `.sep` buys, stated as the difference: the capsule
    // does not move when the words to its left do, and the separator is what
    // takes up the slack. Without this the two halves above are satisfied by a
    // fourth pill laid end to end at a window this wide.
    assert!(
        near(narrow.go.min.x, wide.go.min.x),
        "the `go` capsule moved with the words beside it, so it is laid end to end rather \
         than pushed to the end by `.sep`"
    );
    assert!(
        wide.length.max.x > narrow.length.max.x,
        "the two settings this test compares laid the row out identically, so it is \
         asserting nothing"
    );

    // And the word is painted inside the capsule that answers for it, which is
    // this crate's rule for every control it has.
    let strips = strips();
    let settings = place(5, 0, 2);
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let ctx = drawn_once();
    let at = row(&panel, &ctx, settings);
    let mut view = showing_at(&strips, settings);
    let painted = texts(&mut view, &mut panel);
    assert!(
        painted
            .iter()
            .any(|(pos, text)| text == "go" && at.go.contains(*pos)),
        "`go` is not painted inside the capsule that answers for it"
    );
}

/// A press on `go` asks for a wipe naming the addressed deck and the next one
/// round, and the last deck wraps to the first.
///
/// `Operation::Wipe` carries *the deck being covered* and *the deck arriving
/// over it*, and which two those are is a surface's translation — the
/// vocabulary carries both decks precisely so that it is not the operation's.
/// This row names them `karakuri-cli`'s way: the selection is covered and the
/// next slot round arrives over it, which is ADR-0259's *fade, crossfade and
/// wipe are acts on the addressed strip*.
///
/// The wrap is not a case in the assertion: the loop's last step is the last
/// strip and the expected answer is the first, reached by the same modulo every
/// other step uses.
#[test]
fn a_press_on_go_covers_the_addressed_deck_with_the_next_one_round() {
    let strips = strips();
    let (panel, ctx) = console(PLAUSIBLE);
    let settings = place(5, 0, 2);
    let at = row(&panel, &ctx, settings);
    let decks = strips.len();
    assert!(
        decks >= 2,
        "a one-strip mixer cannot wipe and this test needs one that can"
    );
    for from in 0..decks {
        let mut view = showing_at(&strips, settings);
        // `View::select` answers whether the selection *moved*, so a `false`
        // for deck A is a press that changed nothing rather than a refusal —
        // what this test needs is where the pointer ended up.
        view.select(from as u8);
        assert_eq!(
            view.selection(),
            from as u8,
            "the console refused a selection this test needs it to be on"
        );
        assert_eq!(
            at.go(point(at.go.center()), view.selection(), view.mixer.len()),
            Some(Go::Wipe(Operation::Wipe {
                from: from as u8,
                to: ((from + 1) % decks) as u8,
            })),
            "a press on `go` with deck {from} addressed did not cover it with deck {}",
            (from + 1) % decks
        );
    }
}

/// A press on `go` with no shape chosen is refused, and the refusal says which
/// of the two it is.
///
/// The refusal is the *surface's* and not the conversion's: `Operation::Wipe`
/// says *"Refused with no shape chosen"* at its own definition,
/// `karakuri-operation-record`'s three answers contain no refusal at all, and
/// the shape is this console's own setting — so this is the only place a wipe
/// with nothing to move can be turned away. `karakuri-cli`'s `c` is the
/// precedent and refuses the same two, in this order.
///
/// `WipeKind::None` is the first entry of the cycle and is where a run begins,
/// so this is the state a console nobody has pressed anything on is in rather
/// than one a test had to arrange.
#[test]
fn a_press_on_go_with_no_shape_chosen_is_refused() {
    let strips = strips();
    let (panel, ctx) = console(PLAUSIBLE);
    let view = showing_at(&strips, TransitionSettings::START);
    assert_eq!(
        view.transition().kind,
        WipeKind::None,
        "a run does not begin with no shape chosen, so this test is not in the state it says"
    );
    let at = row(&panel, &ctx, view.transition());
    assert_eq!(
        at.go(point(at.go.center()), view.selection(), view.mixer.len()),
        Some(Go::NoShape),
        "a press on `go` with the shape pill reading `no shape` was not refused for it"
    );
    assert!(
        !at.runs(view.mixer.len()),
        "the capsule is drawn lit over a press that is refused"
    );

    // The guard: the same press with a shape chosen and nothing else changed
    // is a wipe. Without it this passes on a control that refuses everything.
    let armed = place(5, 0, 2);
    let mut chosen = showing_at(&strips, armed);
    chosen.select(0);
    assert_eq!(chosen.selection(), 0);
    let at = row(&panel, &ctx, armed);
    assert_eq!(
        at.go(
            point(at.go.center()),
            chosen.selection(),
            chosen.mixer.len()
        ),
        Some(Go::Wipe(Operation::Wipe { from: 0, to: 1 })),
        "the same press with an `iris` chosen is still not a wipe"
    );
    assert!(
        at.runs(chosen.mixer.len()),
        "the capsule is not lit over a press that runs"
    );
}

/// A press on `go` with one strip in the bay is refused, and it is the other
/// refusal.
///
/// `karakuri-cli`'s *"a wipe needs somewhere to come from — this deck holds one
/// slot"*, and it is asked before the shape for that program's reason: a deck
/// with nowhere to go is refused whatever is armed.
///
/// A console with no deck at all is the same answer, which is the row being
/// drawn without one ([`a_console_with_no_deck_still_draws_the_row`]): the
/// capsule is there, it is not lit, and a press on it says why rather than
/// reaching nothing.
#[test]
fn a_press_on_go_with_nowhere_to_come_from_is_refused() {
    let (panel, ctx) = console(PLAUSIBLE);
    let armed = place(5, 0, 2);
    let one: Vec<Strip> = strips().into_iter().take(1).collect();
    for (mixer, what) in [
        (one, "a one-strip mixer"),
        (Vec::new(), "a console with no deck"),
    ] {
        let view = showing_at(&mixer, armed);
        let at = row(&panel, &ctx, armed);
        assert!(
            view.transition().armed(),
            "{what} is not armed, so this test would be refused for the shape instead"
        );
        assert_eq!(
            at.go(point(at.go.center()), view.selection(), view.mixer.len()),
            Some(Go::NoOtherDeck),
            "a press on `go` at {what} was not refused for having nowhere to come from"
        );
        assert!(!at.runs(view.mixer.len()), "the capsule is lit at {what}");
    }

    // The guard: two strips and the same settings run. Without it this passes
    // on a control that refuses every deck count.
    let two: Vec<Strip> = strips().into_iter().take(2).collect();
    let view = showing_at(&two, armed);
    let at = row(&panel, &ctx, armed);
    assert_eq!(
        at.go(point(at.go.center()), view.selection(), view.mixer.len()),
        Some(Go::Wipe(Operation::Wipe { from: 0, to: 1 })),
        "two strips is somewhere to come from and the press was still refused"
    );
}

/// The `go` capsule answers for itself and for none of the three settings, and
/// none of them answers for it.
///
/// [`a_pill_answers_for_its_own_setting_and_no_other`] with the fourth capsule
/// added, and it is the direction that matters most on this row: a press meant
/// for a length that ran a wipe would put a deck on air.
#[test]
fn the_go_capsule_answers_for_itself_and_no_setting_answers_for_it() {
    let strips = strips();
    let (panel, ctx) = console(PLAUSIBLE);
    let settings = place(5, 2, 3);
    let view = showing_at(&strips, settings);
    let at = row(&panel, &ctx, settings);

    let on_go = point(at.go.center());
    assert_eq!(at.shape(on_go), None, "a press on `go` asked for a shape");
    assert_eq!(at.quantum(on_go), None, "a press on `go` asked for a grid");
    assert_eq!(at.length(on_go), None, "a press on `go` asked for a length");
    assert!(at.owns(on_go), "the `go` capsule is not the row's");

    for (probe, what) in [
        (at.shape.center(), "the shape pill"),
        (at.quantum.center(), "the quantum pill"),
        (at.length.center(), "the length pill"),
        (
            egui::pos2(at.go.min.x - size::XROW_GAP * 0.5, at.go.center().y),
            "`.sep`, which is ground and not a control",
        ),
    ] {
        assert_eq!(
            at.go(point(probe), view.selection(), view.mixer.len()),
            None,
            "{what} ran a wipe"
        );
    }
}
