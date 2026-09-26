//! Mixer strip mask mini button: shape cycling, angle retention, deck targeting, and hit bounds (ADR-0156, ADR-0185, ADR-0201, ADR-0203).

mod common;

use common::{point, rect_of, showing};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Op, Panel, GRAB};
use karakuri_console::room::size;
use karakuri_console::view::{mixer, program_bay, Level, Mask, Mixer, Strip, Tally};
use karakuri_layout::{Hit, Point};
use karakuri_operation::{BlendMode, Operation, WipeKind};

/// Cycle order of mask shapes matching UI progression.
const CYCLE: [Mask; 3] = [Mask::None, Mask::Linear, Mask::Radial];

/// The guard that [`CYCLE`] is every shape and not three of them: a match, so a
/// fourth variant of `Mask` does not compile until somebody has put it in the
/// list above and said where it goes.
fn step_of(mask: Mask) -> usize {
    match mask {
        Mask::None => 0,
        Mask::Linear => 1,
        Mask::Radial => 2,
    }
}

/// Maps console mask shape to vocabulary representation.
fn asked(mask: Mask) -> WipeKind {
    match mask {
        Mask::None => WipeKind::None,
        Mask::Linear => WipeKind::Linear,
        Mask::Radial => WipeKind::Radial,
    }
}

/// The shape the cycle arrives at from `mask`, as the operation names it —
/// [`CYCLE`] walked one step on, wrapping.
fn next(mask: Mask) -> WipeKind {
    asked(CYCLE[(step_of(mask) + 1) % CYCLE.len()])
}

/// Creates a strip configured with specified mask shape and angle.
fn wearing(slot: usize, mask: Mask, angle: f32) -> Strip {
    Strip {
        name: ["drift_night", "lattice_veil", "glass_shell", "slow_tide"][slot].to_owned(),
        tally: Tally::Live,
        // Settled: nothing here is pending and nothing rolls. What a strip
        // whose two halves disagree draws is `parked.rs`.
        requested: Tally::Live,
        gain: 0.2 + 0.15 * slot as f32,
        gain_to: None,
        opacity: 0.8 - 0.15 * slot as f32,
        opacity_to: None,
        blend: BlendMode::ALL[slot % BlendMode::ALL.len()],
        mask,
        mask_angle: angle,
        level: Some(Level {
            mean: 0.5,
            peak: 0.6,
        }),
        is_muted: false,
        is_soloed: false,
    }
}

/// Helper creating four strips with distinct, alternating mask shapes and angles.
fn strips() -> Vec<Strip> {
    const ANGLES: [f32; 4] = [0.9, -0.4, 2.75, 1.25];
    (0..4)
        .map(|slot| wearing(slot, CYCLE[slot % CYCLE.len()], ANGLES[slot]))
        .collect()
}

use common::default_console as console;

fn bay<'a>(panel: &Panel, ctx: &egui::Context, strips: &'a [Strip]) -> Mixer<'a> {
    mixer(ctx, panel.layout(), strips).expect("the mixer bay draws its strips")
}

/// What a press at the centre of strip `slot`'s mini asks for. Panics where
/// there is no mini there, which is the failure worth reading.
fn pressed(bay: &Mixer, slot: usize) -> Operation {
    let at = bay.strip(slot).mask.center();
    bay.mask(point(at))
        .unwrap_or_else(|| panic!("nothing on the mask mini of strip {slot} at {at:?}"))
}

// ---------------------------------------------------------------------------
// The cycle, and where each press arrives
// ---------------------------------------------------------------------------

/// Pressing mask mini cycles to the next shape in [`CYCLE`] and wraps to beginning.
#[test]
fn a_press_moves_to_the_next_shape_and_the_last_wraps_to_the_first() {
    let (panel, ctx) = console();
    let mut strips = strips();

    for (step, from) in CYCLE.into_iter().enumerate() {
        let want = asked(CYCLE[(step + 1) % CYCLE.len()]);
        strips[0] = wearing(0, from, 0.9);
        let bay = bay(&panel, &ctx, &strips);
        assert_eq!(
            pressed(&bay, 0),
            Operation::SetMaskShape {
                deck: 0,
                kind: want,
                angle: 0.9,
            },
            "a press on a mini showing {from:?} did not ask for {want:?}"
        );
    }

    // The guard on the loop above: it has to have walked every shape there is,
    // or a cycle with a hole in it would pass by never being asked about the
    // shape in the hole. `step_of` is a match, so this is exhaustive by the
    // compiler rather than by counting.
    for (step, mask) in CYCLE.into_iter().enumerate() {
        assert_eq!(
            step_of(mask),
            step,
            "`CYCLE` is no longer every shape in the order this walks"
        );
    }
    assert_eq!(
        CYCLE[0],
        Mask::None,
        "the cycle no longer starts at `none` — a cycle should start where a slot starts, \
         which is `MaskKind::ALL`'s own reason for that order"
    );
}

/// Emitted operation preserves existing slot angle rather than resetting to zero.
#[test]
fn the_press_carries_the_angle_the_slot_is_already_wearing() {
    let (panel, ctx) = console();

    // Four angles, none of them zero, one of them negative and one of them
    // past a right angle: an angle is a direction and not a proportion.
    for angle in [0.9f32, -0.4, 2.75, 1.25] {
        for from in CYCLE {
            let strips = vec![wearing(0, from, angle)];
            let bay = bay(&panel, &ctx, &strips);
            assert_eq!(
                pressed(&bay, 0),
                Operation::SetMaskShape {
                    deck: 0,
                    kind: next(from),
                    angle,
                },
                "a press on a {from:?} mini at {angle} rad did not carry that angle"
            );
            assert_ne!(
                pressed(&bay, 0),
                Operation::SetMaskShape {
                    deck: 0,
                    kind: next(from),
                    angle: 0.0,
                },
                "the press sent a default angle rather than the {angle} rad the slot is \
                 wearing, so choosing a shape straightens a diagonal wipe and nothing on \
                 the panel says so"
            );
        }
    }

    // And the strips these assertions were made against really were at a
    // non-zero angle, which is what says they are about the angle rather than
    // about a fixture that was at zero all along.
    assert_ne!(wearing(0, Mask::Linear, 0.9).mask_angle, 0.0);
}

/// Emitted operations target the clicked strip's deck index, shape, and angle.
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
        let want = Operation::SetMaskShape {
            deck: slot as u8,
            kind: next(strip.mask),
            angle: strip.mask_angle,
        };
        assert_eq!(
            pressed(&bay, slot),
            want,
            "the mini of strip {slot}, which wears {:?} at {} rad, asked for the wrong thing",
            strip.mask,
            strip.mask_angle
        );
    }

    // And the four answers are not all the same operation, which is what says
    // the loop above compared anything: a mini that answered the same thing
    // everywhere would satisfy neither the deck nor the payload.
    let asked: Vec<Operation> = (0..strips.len()).map(|slot| pressed(&bay, slot)).collect();
    for (slot, one) in asked.iter().enumerate() {
        for other in &asked[slot + 1..] {
            assert_ne!(one, other, "two strips' minis asked for the same operation");
        }
    }
}

// ---------------------------------------------------------------------------
// A control claims what it acts on and no more
// ---------------------------------------------------------------------------

/// Clicks outside mask mini claim the strip for deck selection without changing mask shape.
#[test]
fn a_press_off_the_mini_asks_for_nothing_and_selects_the_deck_instead() {
    let (mut panel, ctx) = console();
    let strips = strips();
    let bay = bay(&panel, &ctx, &strips);
    let at = bay.strip(1);

    // The gap between the two minis is `MODE_GAP`, which is 3, so a point in
    // the middle of it is off both.
    let gap = egui::pos2(at.mask.min.x - size::MODE_GAP * 0.5, at.mask.center().y);
    let probes = [
        (gap, "the gap between the two minis"),
        (at.num.center(), "the number"),
        (at.name.center(), "the name"),
        (at.meter.center(), "the meter"),
        (at.trim_label.center(), "the trim's label"),
        (
            egui::pos2(at.mask.max.x + 1.0, at.mask.center().y),
            "one pixel right of the mini",
        ),
        (
            egui::pos2(at.mask.center().x, at.mask.min.y - 1.0),
            "one pixel above the mini",
        ),
        (
            egui::pos2(at.mask.center().x, at.mask.max.y + 1.0),
            "one pixel below the mini",
        ),
    ];
    for (probe, what) in probes {
        assert!(
            !at.mask.contains(probe),
            "{what} is inside the mini, so this probe asserts nothing"
        );
        assert_eq!(
            bay.mask(point(probe)),
            None,
            "{what} asked the mask to change"
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

    // Neighboring controls (blend chip and tally chip) ignore mask mini claims.
    for (probe, what) in [
        (at.blend.center(), "the blend chip"),
        (at.tally.center(), "the tally chip"),
    ] {
        assert_eq!(
            bay.mask(point(probe)),
            None,
            "{what} asked the mask to change"
        );
        assert_eq!(
            claim(&mut panel, &ctx, &showing(&strips), point(probe)),
            Claim::Panel,
            "{what} stopped being a control, so this is asserting nothing about the mask"
        );
    }

    // The guard: the mini itself does both of the things the probes above do
    // neither of. Without this the test passes on a mini that was never a
    // control at all.
    let on = at.mask.center();
    assert!(bay.mask(point(on)).is_some(), "the mini asked for nothing");
    assert_eq!(
        claim(&mut panel, &ctx, &showing(&strips), point(on)),
        Claim::Panel,
        "the mini is not the panel's, so it is drawn where it cannot be clicked"
    );
}

/// Mask minis in unarranged or folded bays decline hits across all folds (ADR-0182).
#[test]
fn a_folded_mixer_bay_has_no_mini_to_press() {
    for enclosing in [false, true] {
        let (mut panel, ctx) = console();
        let strips = strips();
        let where_it_was = bay(&panel, &ctx, &strips).strip(0).mask.center();
        assert!(
            bay(&panel, &ctx, &strips)
                .mask(point(where_it_was))
                .is_some(),
            "the mini is not a control before anything is folded"
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
            "where the mini used to be is claimed by the wrong thing with the bay folded \
             away — the cell there is {cell:?} (enclosing: {enclosing})"
        );

        // And back, so that the answer above is the fold rather than the
        // control having gone.
        panel.op(Op::Unfold(folds));
        assert!(
            bay(&panel, &ctx, &strips)
                .mask(point(where_it_was))
                .is_some(),
            "unfolding the bay left the mini dead (enclosing: {enclosing})"
        );
    }
}

// ---------------------------------------------------------------------------
// The boundary gets first refusal, and the mini clears its band
// ---------------------------------------------------------------------------

/// Mask minis clear boundary grab zones across all strips (ADR-0185).
#[test]
fn no_mask_mini_is_inside_a_boundarys_grab() {
    let (mut panel, ctx) = console();
    let strips = strips();
    let bay = bay(&panel, &ctx, &strips);

    // The other half of the guard, up front: there are four minis to measure.
    // A bay that had lost them would satisfy the loop below by never entering
    // it.
    assert_eq!(
        bay.count(),
        strips.len(),
        "the bay is not the four strips it was given, so this test has no minis to measure"
    );

    let mut minis = 0;
    for slot in 0..strips.len() {
        let chip = bay.strip(slot).mask;
        assert!(
            bay.mask(point(chip.center())).is_some(),
            "strip {slot}'s mini is not a control, so measuring its clearance asserts nothing"
        );
        minis += 1;
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
                "a boundary grabs {probe:?}, which is on the mask mini of strip {slot} — the \
                 control is dead there, and `input`'s rule 2 is what would have to change"
            );
            assert_eq!(
                claim(&mut panel, &ctx, &showing(&strips), point(probe)),
                Claim::Panel,
                "the mask mini of strip {slot} is not the panel's at {probe:?}"
            );
        }
    }
    assert_eq!(
        minis,
        strips.len(),
        "this test found no minis to measure, so it is asserting nothing"
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
        bay.mask(below),
        None,
        "a point in the ground under the bay pressed a mask mini"
    );
}

// ---------------------------------------------------------------------------
// The target does not move under its own value
// ---------------------------------------------------------------------------

/// Mini button bounds remain constant at `MINI_SIZE` regardless of displayed mask shape.
#[test]
fn the_mini_does_not_move_under_the_shape_it_shows() {
    let (panel, ctx) = console();

    let mut chip = None;
    for mask in CYCLE {
        let strips = vec![wearing(0, mask, 0.9)];
        let at = bay(&panel, &ctx, &strips).strip(0).mask;
        match chip {
            None => chip = Some(at),
            Some(first) => assert_eq!(
                at, first,
                "a {mask:?} mini is {at:?} and the first was {first:?} — the target moves \
                 with the shape it shows"
            ),
        }
    }
    let chip = chip.expect("there is at least one shape to measure");

    let edge = egui::pos2(chip.min.x + 1.0, chip.center().y);
    for mask in CYCLE {
        let strips = vec![wearing(0, mask, 0.9)];
        assert!(
            bay(&panel, &ctx, &strips).mask(point(edge)).is_some(),
            "the left edge of a {mask:?} mini is not a control, so the target moved"
        );
    }
}
