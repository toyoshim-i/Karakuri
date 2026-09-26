//! Integration tests for mixer strip tally chip interaction and residency cycling.

mod common;

use common::{point, rect_of, showing};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Op, Panel, GRAB};
use karakuri_console::room::size;
use karakuri_console::view::{mixer, program_bay, Level, Mask, Mixer, Strip, Tally};
use karakuri_layout::{Hit, Point};
use karakuri_operation::{BlendMode, Operation, Residency};

/// A strip that is where it was asked to be, at `tally` — the ordinary case,
/// and the one where a press cannot tell the two residencies apart.
fn settled(slot: usize, tally: Tally) -> Strip {
    Strip {
        name: ["drift_night", "lattice_veil", "glass_shell", "slow_tide"][slot].to_owned(),
        tally,
        requested: tally,
        gain: 0.2 + 0.15 * slot as f32,
        gain_to: None,
        opacity: 0.8 - 0.15 * slot as f32,
        opacity_to: None,
        blend: BlendMode::ALL[slot % BlendMode::ALL.len()],
        mask: Mask::None,
        mask_angle: 0.0,
        level: Some(Level {
            mean: 0.5,
            peak: 0.6,
        }),
        is_muted: false,
        is_soloed: false,
    }
}

/// Four strips, no two adjacent ones on the same residency, so that a press
/// answered from the wrong strip is a wrong answer rather than the right one by
/// luck. `Tally::ALL` is three and there are four decks, so deck A and deck D
/// share a residency and neither is beside the other.
fn strips() -> Vec<Strip> {
    (0..4)
        .map(|slot| settled(slot, Tally::ALL[slot % Tally::ALL.len()]))
        .collect()
}

/// The parked strip, which is the one state the engine can actually produce
/// where the two residencies disagree: `Deck::is_parked` is `requested ==
/// Priming && effective == Allocated` — asked to prime, and held at allocated
/// because the budget has not found room.
fn parked(slot: usize) -> Strip {
    Strip {
        tally: Tally::Allocated,
        requested: Tally::Priming,
        ..settled(slot, Tally::Allocated)
    }
}

use common::default_console as console;

fn bay<'a>(panel: &Panel, ctx: &egui::Context, strips: &'a [Strip]) -> Mixer<'a> {
    mixer(ctx, panel.layout(), strips).expect("the mixer bay draws its strips")
}

/// What a press at the centre of strip `slot`'s chip asks for. Panics where
/// there is no chip there, which is the failure worth reading.
fn pressed(bay: &Mixer, slot: usize) -> Operation {
    let at = bay.strip(slot).tally.center();
    bay.tally(point(at))
        .unwrap_or_else(|| panic!("nothing on the tally chip of strip {slot} at {at:?}"))
}

/// Translates console residency enums into vocabulary terms independently to verify mapping.
fn asked(tally: Tally) -> Residency {
    match tally {
        Tally::Live => Residency::Live,
        Tally::Priming => Residency::Priming,
        Tally::Allocated => Residency::Allocated,
    }
}

/// The residency the cycle arrives at from `tally`, as the operation names it —
/// `Tally::ALL` walked one step on, wrapping.
fn next(tally: Tally) -> Residency {
    let step = Tally::ALL
        .iter()
        .position(|one| *one == tally)
        .expect("`Tally::ALL` names every residency");
    asked(Tally::ALL[(step + 1) % Tally::ALL.len()])
}

// ---------------------------------------------------------------------------
// The cycle, and where each press arrives
// ---------------------------------------------------------------------------

/// Clicking the tally cycles through the residencies in order (live -> priming -> allocated -> live).
#[test]
fn a_press_moves_to_the_next_residency_and_the_last_wraps_to_the_first() {
    let (panel, ctx) = console();
    let mut strips = strips();

    for (step, from) in Tally::ALL.into_iter().enumerate() {
        let want = asked(Tally::ALL[(step + 1) % Tally::ALL.len()]);
        strips[0] = settled(0, from);
        let bay = bay(&panel, &ctx, &strips);
        assert_eq!(
            pressed(&bay, 0),
            Operation::SetResidency {
                deck: 0,
                residency: want,
            },
            "a press on a chip asked for `{}` did not ask for {want:?}",
            from.word()
        );
    }

    // The guard on the loop above: it has to have walked every residency there
    // is, or a cycle with a hole in it would pass by never being asked about
    // the residency in the hole.
    assert_eq!(
        Tally::ALL.len(),
        3,
        "`Tally::ALL` is no longer the three this walks, and the wrap it asserts is a \
         different wrap"
    );
    assert_eq!(
        Tally::ALL,
        [Tally::Live, Tally::Priming, Tally::Allocated],
        "the cycle is no longer the order the manual's tally tooltips state"
    );
}

/// Clicking a parked tally withdraws the prime request (returning to allocated) rather than advancing to live.
#[test]
fn a_parked_chip_asks_for_the_withdrawal_rather_than_going_on_air() {
    let (panel, ctx) = console();
    let strips = vec![parked(0)];
    let strip = &strips[0];

    // The guard: this strip really is the state the test is about. Without it
    // the assertions below hold trivially on a settled strip, where the two
    // residencies are the same value and nothing can tell them apart.
    assert_eq!(
        strip.pending(),
        Some(Tally::Priming),
        "the strip is not pending, so nothing here distinguishes the request from the readout"
    );
    assert_ne!(strip.tally, strip.requested);

    let bay = bay(&panel, &ctx, &strips);
    assert_eq!(
        pressed(&bay, 0),
        Operation::SetResidency {
            deck: 0,
            residency: Residency::Allocated,
        },
        "a press on a parked chip did not ask for the prime request to be withdrawn"
    );
    assert_ne!(
        pressed(&bay, 0),
        Operation::SetResidency {
            deck: 0,
            residency: next(strip.tally),
        },
        "the chip cycled from the residency it is *showing* rather than from the one that was \
         *requested*, so a press on a parked deck puts it on air instead of withdrawing the \
         prime request"
    );

    // And the two really are different answers, which is what says the
    // assertion above is about the chip rather than about two names for one
    // value.
    assert_ne!(next(strip.tally), next(strip.requested));
}

/// Operations identify the specific deck and its requested residency rather than defaulting to deck 0.
#[test]
fn the_operation_names_the_strips_own_deck() {
    let (panel, ctx) = console();
    let strips = strips();
    let bay = bay(&panel, &ctx, &strips);
    assert_eq!(
        bay.count(),
        4,
        "the bay is not the four strips it was given"
    );

    for (slot, strip) in strips.iter().enumerate() {
        let want = Operation::SetResidency {
            deck: slot as u8,
            residency: next(strip.requested),
        };
        assert_eq!(
            pressed(&bay, slot),
            want,
            "the chip of strip {slot}, which was asked for `{}`, asked for the wrong thing",
            strip.requested.word()
        );
    }

    // Confirms operations differ across strips by deck identity and requested residency.
    let asked: Vec<Operation> = (0..strips.len()).map(|slot| pressed(&bay, slot)).collect();
    for (slot, one) in asked.iter().enumerate() {
        for other in &asked[slot + 1..] {
            assert_ne!(one, other, "two strips' chips asked for the same operation");
        }
    }
}

// ---------------------------------------------------------------------------
// A control claims what it acts on and no more
// ---------------------------------------------------------------------------

/// Clicking off the tally chip inside the strip selects the deck without modifying residency.
#[test]
fn a_press_off_the_chip_asks_for_nothing_and_selects_the_deck_instead() {
    let (mut panel, ctx) = console();
    let strips = strips();
    let bay = bay(&panel, &ctx, &strips);
    let at = bay.strip(1);

    // A point one pixel outside each edge of the capsule, plus its neighbours
    // up and down the strip. The capsule is centred and narrower than the
    // strip, so a point beside it is inside the strip and outside the control.
    let probes = [
        (at.name.center(), "the name above it"),
        (at.trim_label.center(), "the trim's label below it"),
        (at.num.center(), "the number"),
        (at.meter.center(), "the meter"),
        (
            egui::pos2(at.tally.min.x - 1.0, at.tally.center().y),
            "one pixel left of the chip",
        ),
        (
            egui::pos2(at.tally.max.x + 1.0, at.tally.center().y),
            "one pixel right of the chip",
        ),
        (
            egui::pos2(at.tally.center().x, at.tally.min.y - 1.0),
            "one pixel above the chip",
        ),
        (
            egui::pos2(at.tally.center().x, at.tally.max.y + 1.0),
            "one pixel below the chip",
        ),
    ];
    for (probe, what) in probes {
        assert!(
            !at.tally.contains(probe),
            "{what} is inside the chip, so this probe asserts nothing"
        );
        assert_eq!(
            bay.tally(point(probe)),
            None,
            "{what} asked the residency to change"
        );
        assert_eq!(
            bay.select(point(probe)),
            Some(Operation::SelectDeck { deck: 1 }),
            "{what} is inside strip B and selects no deck, so the press reaches nothing"
        );
        assert_eq!(
            claim(&mut panel, &ctx, &showing(&strips), point(probe)),
            Claim::Panel,
            "{what} is not the panel's, so a press there goes to `egui`, which owns no \
             widget anywhere on this console"
        );
    }

    // Mode row chips claim their own clicks and do not trigger tally residency changes.
    for (probe, what) in [
        (at.blend.center(), "the blend chip"),
        (at.mask.center(), "the mask mini"),
    ] {
        assert_eq!(
            bay.tally(point(probe)),
            None,
            "{what} asked the residency to change"
        );
        assert_eq!(
            claim(&mut panel, &ctx, &showing(&strips), point(probe)),
            Claim::Panel,
            "{what} stopped being a control, so this is asserting nothing about the tally"
        );
    }

    // The guard: the chip itself does both of the things the probes above do
    // neither of. Without this the test passes on a chip that was never a
    // control at all.
    let on = at.tally.center();
    assert!(bay.tally(point(on)).is_some(), "the chip asked for nothing");
    assert_eq!(
        claim(&mut panel, &ctx, &showing(&strips), point(on)),
        Claim::Panel,
        "the chip is not the panel's, so it is drawn where it cannot be clicked"
    );
}

/// Tallies in folded bays or folded panes cannot be hit-tested or clicked.
#[test]
fn a_folded_mixer_bay_has_no_chip_to_press() {
    for enclosing in [false, true] {
        let (mut panel, ctx) = console();
        let strips = strips();
        let where_it_was = bay(&panel, &ctx, &strips).strip(0).tally.center();
        assert!(
            bay(&panel, &ctx, &strips)
                .tally(point(where_it_was))
                .is_some(),
            "the chip is not a control before anything is folded"
        );

        let bay_id = panel
            .layout()
            .find("mixer")
            .expect("the arrangement names the mixer");
        let folds = match enclosing {
            true => panel
                .layout()
                .find("right-pane")
                .expect("the arrangement names the right pane"),
            false => bay_id,
        };
        panel.op(Op::Fold(folds));
        assert!(
            !panel.layout().visible(bay_id),
            "folding {} left the mixer bay laid out",
            match enclosing {
                true => "the right pane",
                false => "the mixer bay",
            }
        );

        assert!(
            mixer(&ctx, panel.layout(), &strips).is_none(),
            "a folded mixer bay still laid its strips out (enclosing: {enclosing})"
        );
        let view = showing(&strips);
        // Whoever is standing in that ground now, asked of the arrangement
        // rather than assumed — see this test's own documentation.
        let cell =
            program_bay(panel.layout(), view.canvas).and_then(|bay| bay.cell(point(where_it_was)));
        assert_eq!(
            claim(&mut panel, &ctx, &view, point(where_it_was)),
            match cell {
                Some(_) => Claim::Panel,
                None => Claim::Egui,
            },
            "where the chip used to be is claimed by the wrong thing with the bay folded \
             away — the cell there is {cell:?} (enclosing: {enclosing})"
        );

        // And back, so that the answer above is the fold rather than the
        // control having gone.
        panel.op(Op::Unfold(folds));
        assert!(
            bay(&panel, &ctx, &strips)
                .tally(point(where_it_was))
                .is_some(),
            "unfolding the bay left the chip dead (enclosing: {enclosing})"
        );
    }
}

// ---------------------------------------------------------------------------
// The boundary gets first refusal, and the chip clears its band
// ---------------------------------------------------------------------------

/// Tally chips clear boundary grab zones, verified against both `Layout::hit` and `claim` (ADR-0185).
#[test]
fn no_tally_chip_is_inside_a_boundarys_grab() {
    let (mut panel, ctx) = console();
    let strips = strips();
    let bay = bay(&panel, &ctx, &strips);

    // The other half of the guard, up front: there are four chips to measure.
    // A bay that had lost them would satisfy the loop below by never entering
    // it.
    assert_eq!(
        bay.count(),
        strips.len(),
        "the bay is not the four strips it was given, so this test has no chips to measure"
    );

    let mut chips = 0;
    for slot in 0..strips.len() {
        let chip = bay.strip(slot).tally;
        assert!(
            bay.tally(point(chip.center())).is_some(),
            "strip {slot}'s chip is not a control, so measuring its clearance asserts nothing"
        );
        chips += 1;
        // The corners as well as the centre: a grab is a band either side of a
        // boundary, so it is an edge that reaches one first.
        for probe in [
            chip.left_top(),
            chip.right_top(),
            chip.left_bottom(),
            chip.right_bottom(),
            chip.center(),
        ] {
            assert!(
                !matches!(panel.layout().hit(point(probe), GRAB), Hit::Divider { .. }),
                "a boundary grabs {probe:?}, which is on the tally chip of strip {slot} — the \
                 control is dead there, and `input`'s rule 2 is what would have to change"
            );
            assert_eq!(
                claim(&mut panel, &ctx, &showing(&strips), point(probe)),
                Claim::Panel,
                "the tally chip of strip {slot} is not the panel's at {probe:?}"
            );
        }
    }
    assert_eq!(
        chips,
        strips.len(),
        "this test found no chips to measure, so it is asserting nothing"
    );

    // The guard: the ground under the bay *is* inside a boundary's grab.
    let region = rect_of(panel.layout(), "mixer");
    let below = Point::new(region.x + region.w * 0.5, region.y + region.h + GRAB * 0.5);
    assert!(
        matches!(panel.layout().hit(below, GRAB), Hit::Divider { .. }),
        "the ground under the mixer is not in the grab of the boundary there, so this test \
         is no longer measuring the clearance it was written for"
    );
    assert_eq!(
        bay.tally(below),
        None,
        "a point in the ground under the bay pressed a tally chip"
    );
}

// ---------------------------------------------------------------------------
// The target does not move under its own value
// ---------------------------------------------------------------------------

/// Sizing the tally to the widest residency word keeps the target stationary across state changes.
#[test]
fn the_chip_does_not_move_under_the_value_it_shows() {
    let (panel, ctx) = console();

    let states: Vec<(String, Strip)> = Tally::ALL
        .into_iter()
        .map(|tally| (tally.word().to_owned(), settled(0, tally)))
        .chain([("parked".to_owned(), parked(0))])
        .collect();

    let mut chip = None;
    for (what, strip) in &states {
        let strips = vec![strip.clone()];
        let at = bay(&panel, &ctx, &strips).strip(0).tally;
        match chip {
            None => chip = Some(at),
            Some(first) => assert_eq!(
                at, first,
                "a `{what}` chip is {at:?} and the first was {first:?} — the target moves with \
                 the value it shows"
            ),
        }
    }
    let chip = chip.expect("there is at least one state to measure");

    // A point a hand could be aiming at that is only *on* the control because
    // the capsule is the widest word's: one pixel inside its left edge.
    let edge = egui::pos2(chip.min.x + 1.0, chip.center().y);
    for (what, strip) in &states {
        let strips = vec![strip.clone()];
        assert!(
            bay(&panel, &ctx, &strips).tally(point(edge)).is_some(),
            "the left edge of a `{what}` chip is not a control, so the target moved"
        );
    }

    // And the capsule really is wider than the narrowest word needs, or the
    // probe above is not testing anything: `LIVE` is 34.06 wide with its
    // padding and `ALLOC` is 44.53.
    let narrow = vec![settled(0, Tally::Live)];
    let word = bay(&panel, &ctx, &narrow).strip(0).tally;
    assert!(
        word.width() > size::TALLY_PAD_X * 2.0 + 1.0,
        "the capsule is padding and nothing else"
    );
}
