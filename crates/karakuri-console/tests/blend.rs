//! Mixer strip blend chip click behaviors and `SetBlendMode` emission (ADR-0156, ADR-0187, P-0090).

mod common;

use common::{point, rect_of, showing};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Panel, GRAB};
use karakuri_console::room::size;
use karakuri_console::view::{mixer, Level, Mask, Mixer, Strip, Tally};
use karakuri_layout::{Hit, Point};
use karakuri_operation::{BlendMode, Operation};

/// Creates four strips with distinct, alternating blend modes.
fn strips() -> Vec<Strip> {
    ["drift_night", "lattice_veil", "glass_shell", "slow_tide"]
        .into_iter()
        .enumerate()
        .map(|(slot, name)| Strip {
            name: name.to_owned(),
            tally: Tally::Live,
            // Settled: the request and the effective residency agree, so
            // nothing in these strips is pending and nothing rolls. What a
            // strip whose two halves disagree draws is `parked.rs`.
            requested: Tally::Live,
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
        })
        .collect()
}

use common::default_console as console;

fn bay<'a>(panel: &Panel, ctx: &egui::Context, strips: &'a [Strip]) -> Mixer<'a> {
    mixer(ctx, panel.layout(), strips).expect("the mixer bay draws its strips")
}

/// What a press at the centre of strip `slot`'s chip asks for. Panics where
/// there is no chip there, which is the failure worth reading.
fn pressed(bay: &Mixer, slot: usize) -> Operation {
    let at = bay.strip(slot).blend.center();
    bay.blend(point(at))
        .unwrap_or_else(|| panic!("nothing on the blend chip of strip {slot} at {at:?}"))
}

// ---------------------------------------------------------------------------
// The cycle, and where each press arrives
// ---------------------------------------------------------------------------

/// Clicking the blend chip cycles through `BlendMode::ALL` in order and wraps around to the beginning.
#[test]
fn a_press_moves_to_the_next_mode_and_the_last_wraps_to_the_first() {
    let (panel, ctx) = console();
    let mut strips = strips();

    for (step, from) in BlendMode::ALL.into_iter().enumerate() {
        let want = BlendMode::ALL[(step + 1) % BlendMode::ALL.len()];
        strips[0].blend = from;
        let bay = bay(&panel, &ctx, &strips);
        assert_eq!(
            pressed(&bay, 0),
            Operation::SetBlendMode {
                deck: 0,
                blend: want,
            },
            "a press on a chip reading `{}` did not ask for `{}`",
            from.name(),
            want.name()
        );
    }

    // The guard on the loop above: it has to have walked every mode there is,
    // or a cycle with a hole in it would pass by never being asked about the
    // mode in the hole.
    assert_eq!(
        BlendMode::ALL.len(),
        3,
        "`BlendMode::ALL` is no longer the three this walks, and the wrap it asserts is a \
         different wrap"
    );
}

/// Emitted operations target the clicked strip's deck index and mode.
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
        let want = Operation::SetBlendMode {
            deck: slot as u8,
            blend: BlendMode::ALL[(slot + 1) % BlendMode::ALL.len()],
        };
        assert_eq!(
            pressed(&bay, slot),
            want,
            "the chip of strip {slot}, which reads `{}`, asked for the wrong thing",
            strip.blend.name()
        );
    }

    // And the four answers are four different operations, which is what says
    // the loop above compared anything: a chip that answered the same thing
    // everywhere would satisfy neither the deck nor the blend.
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

/// Clicks outside the chip claim the strip for deck selection without altering the blend mode.
#[test]
fn a_press_off_the_chip_asks_for_nothing_and_selects_the_deck_instead() {
    let (mut panel, ctx) = console();
    let strips = strips();
    let bay = bay(&panel, &ctx, &strips);
    let at = bay.strip(1);

    // A point one pixel outside each edge of the chip, plus the neighbours in
    // the same row and the rows above it. The gap between the two minis is
    // `MODE_GAP`, which is 3, so a point in the middle of it is off both.
    let gap = egui::pos2(at.blend.max.x + size::MODE_GAP * 0.5, at.blend.center().y);
    let probes = [
        (gap, "the gap between the two minis"),
        (at.num.center(), "the number"),
        (at.name.center(), "the name"),
        (at.meter.center(), "the meter"),
        (
            egui::pos2(at.blend.min.x - 1.0, at.blend.center().y),
            "one pixel left of the chip",
        ),
        (
            egui::pos2(at.blend.center().x, at.blend.min.y - 1.0),
            "one pixel above the chip",
        ),
        (
            egui::pos2(at.blend.center().x, at.blend.max.y + 1.0),
            "one pixel below the chip",
        ),
    ];
    for (probe, what) in probes {
        assert!(
            !at.blend.contains(probe),
            "{what} is inside the chip, so this probe asserts nothing"
        );
        assert_eq!(
            bay.blend(point(probe)),
            None,
            "{what} asked the blend to change"
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

    // Distinct control hits: neighboring tally (ADR-0195) and mask mini (ADR-0203) ignore blend claims.
    for (probe, what) in [
        (at.tally.center(), "the tally chip"),
        (at.mask.center(), "the mask mini"),
    ] {
        assert_eq!(
            bay.blend(point(probe)),
            None,
            "{what} asked the blend to change"
        );
        assert_eq!(
            claim(&mut panel, &ctx, &showing(&strips), point(probe)),
            Claim::Panel,
            "{what} stopped being a control, so this is asserting nothing about the blend"
        );
    }

    // The guard: the chip itself does both of the things the probes above do
    // neither of. Without this the test passes on a chip that was never a
    // control at all.
    let on = at.blend.center();
    assert!(bay.blend(point(on)).is_some(), "the chip asked for nothing");
    assert_eq!(
        claim(&mut panel, &ctx, &showing(&strips), point(on)),
        Claim::Panel,
        "the chip is not the panel's, so it is drawn where it cannot be clicked"
    );
}

// ---------------------------------------------------------------------------
// The boundary gets first refusal, and the chip clears its band
// ---------------------------------------------------------------------------

/// Blend chips clear boundary grab zones, verified via direct hit testing (ADR-0185).
#[test]
fn no_blend_chip_is_inside_a_boundarys_grab() {
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
        let chip = bay.strip(slot).blend;
        assert!(
            bay.blend(point(chip.center())).is_some(),
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
                "a boundary grabs {probe:?}, which is on the blend chip of strip {slot} — the \
                 control is dead there, and `input`'s rule 2 is what would have to change"
            );
            assert_eq!(
                claim(&mut panel, &ctx, &showing(&strips), point(probe)),
                Claim::Panel,
                "the blend chip of strip {slot} is not the panel's at {probe:?}"
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
        bay.blend(below),
        None,
        "a point in the ground under the bay pressed a blend chip"
    );
}

// ---------------------------------------------------------------------------
// The row the chip sits in
// ---------------------------------------------------------------------------

/// Mode rows remain strictly inside their strip track bounds across all modes without overflowing neighbouring tracks.
#[test]
fn the_mode_row_stays_inside_its_track() {
    let (panel, ctx) = console();
    let mut strips = strips();

    let mut measured = 0;
    for blend in BlendMode::ALL {
        for strip in strips.iter_mut() {
            strip.blend = blend;
        }
        let bay = bay(&panel, &ctx, &strips);
        for slot in 0..strips.len() {
            let at = bay.strip(slot);
            let row = egui::Rect::from_min_max(at.blend.min, at.mask.max);
            measured += 1;
            assert!(
                at.rect.contains_rect(row),
                "at `{}` the mode row of strip {slot} is {:?}, which is outside its {:.2}-wide \
                 track {:?} — a chip is now over its neighbour",
                blend.name(),
                row,
                at.rect.width(),
                at.rect
            );
            // Centred, which is what `align-items: center` on `.strip` is and
            // is why the spill is even and not all to one side.
            assert!(
                (row.center().x - at.rect.center().x).abs() <= common::EPS,
                "the mode row of strip {slot} is not centred across its track"
            );
        }

        let at = bay.strip(0);
        let row = at.mask.max.x - at.blend.min.x;
        assert!(
            row <= at.rect.width(),
            "at `{}` the mode row is {row:.4} and the strip's track is {:.4}",
            blend.name(),
            at.rect.width()
        );
    }

    assert_eq!(
        measured,
        BlendMode::ALL.len() * strips.len(),
        "this test measured no rows, so it is asserting nothing"
    );
}
