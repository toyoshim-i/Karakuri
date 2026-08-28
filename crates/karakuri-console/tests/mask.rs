//! **The mixer strip's mask mini, pressed.**
//!
//! `mixer.rs` is where a strip's rectangles are and which of them are
//! controls; `blend.rs` is the chip 3px to the left of this one and `tally.rs`
//! is the chip four rows up. This is the fifth control (ADR-0203): the mini
//! cycles, and a press on it emits `Operation::SetMaskShape` naming the shape
//! it **arrived at**.
//!
//! **The angle is what makes this control different from the other two**, and
//! it is the decision this file exists to hold. `Operation::SetMaskShape`
//! carries a shape **and an angle** ([ADR-0201](../../../docs/adr/0201-the-mask-is-two-rows-because-a-control-change-can-only-set.md)),
//! and this chip names only the shape — *"`.mini` is a chip that says which
//! shape, and three numbers about that shape are the inspector's row, not this
//! one."* So a press has to carry a number the control does not control, and
//! the one it carries is `Strip::mask_angle`: the angle the slot is already
//! wearing. Sending `0.0` would make choosing a shape silently straighten a
//! diagonal wipe — a press that changed something nobody asked it to, and
//! invisible on the panel, because the mark a mini draws is the same mark at
//! any angle. `the_press_carries_the_angle_the_slot_is_already_wearing` is
//! that assertion, in both directions.
//!
//! **What is *not* here is the position and the softness.** Neither is in the
//! operation at all: `Record::Mask` is written whole and the half a shape
//! operation does not ask for is filled in where the record is written, from a
//! reading of the running mask. So there is nothing on this surface to assert
//! about them, and a strip that carried them would be this crate keeping half
//! a deck.
//!
//! **None of it needs a device.** Laying a strip out needs `egui`, because the
//! blend chip beside this one is as wide as the word in it, and what comes out
//! is `karakuri-operation`'s, which has no dependencies at all.
//!
//! # Where this stops
//!
//! At the operation, exactly as `blend.rs` and `tally.rs` do. Turning it into
//! a `Record::Mask` — whole, with the front and the soft edge read off the
//! deck — and applying it is the harness's, and `examples/panel.rs`'s
//! `a_press_on_the_mask_mini_chooses_a_shape_and_keeps_the_angle` is that end
//! of the same press. This crate has no deck (ADR-0156).

mod common;

use common::{drawn_once, rect_of, PLAUSIBLE};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Op, Panel, GRAB};
use karakuri_console::room::size;
use karakuri_console::view::{mixer, Level, Mask, Mixer, Strip, Tally};
use karakuri_layout::{Hit, Point};
use karakuri_operation::{BlendMode, Operation, WipeKind};

/// **Every shape there is, in the order a press walks them** — and the order
/// is `karakuri_engine::deck::MaskKind::ALL`'s, which states its own reason:
/// *"`None` first, because it is the default and a cycle should start where a
/// slot starts."*
///
/// **Written here rather than read off a `Mask::ALL`, because there is no
/// `Mask::ALL`.** `Tally::ALL` and `BlendMode::ALL` exist for readers —
/// `mixer` measures the tally capsule against the widest of the three words,
/// and a map file is offered the blend's three — and nothing reads a list of
/// mask shapes: the mini holds a mark rather than a word and is the same width
/// whichever shape it shows, and no map target names a shape, which is why
/// `karakuri_operation::WipeKind` has none either. So `view::next_shape` is
/// the only statement of the order in the crate, and this is the copy it is
/// checked against — which is what `blend.rs` and `tally.rs` get from walking
/// an `ALL`.
const CYCLE: [Mask; 3] = [Mask::None, Mask::Linear, Mask::Radial];

/// **The guard that [`CYCLE`] is every shape and not three of them**: a match,
/// so a fourth variant of `Mask` does not compile until somebody has put it in
/// the list above and said where it goes.
fn step_of(mask: Mask) -> usize {
    match mask {
        Mask::None => 0,
        Mask::Linear => 1,
        Mask::Radial => 2,
    }
}

/// **The vocabulary's word for a console mask shape, written out again here.**
///
/// `view`'s own conversion is private, and a test that asked it for the
/// expected answer would be asserting that it agrees with itself. Two arms
/// swapped in either copy is a chip that asks for the wrong shape, and it
/// fails here.
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

/// A strip wearing `mask` at `angle`.
///
/// **The angle is never zero on any strip these tests build**, and that is
/// deliberate: a fixture at zero cannot tell a press that carries the slot's
/// angle from one that carries a default, which is the whole difference this
/// file is about.
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
    }
}

/// **Four strips, no two adjacent ones wearing the same shape, and every one
/// at a different angle** — so that a press answered from the wrong strip is a
/// wrong answer rather than the right one by luck. There are three shapes and
/// four decks, so deck A and deck D share one and neither is beside the other;
/// the angles are all different, so even those two answer differently.
///
/// One of the angles is **negative**, because an angle is a direction in
/// radians and nothing about it is a proportion — a copy that clamped or
/// unit-ed it on the way through would pass on four positive numbers.
fn strips() -> Vec<Strip> {
    const ANGLES: [f32; 4] = [0.9, -0.4, 2.75, 1.25];
    (0..4)
        .map(|slot| wearing(slot, CYCLE[slot % CYCLE.len()], ANGLES[slot]))
        .collect()
}

/// A panel at a viewport, solved, with a context that has drawn once — the
/// pair `mixer.rs`, `blend.rs` and `tally.rs` all open with.
fn console() -> (Panel, egui::Context) {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    (panel, drawn_once())
}

fn bay<'a>(panel: &Panel, ctx: &egui::Context, strips: &'a [Strip]) -> Mixer<'a> {
    mixer(ctx, panel.layout(), strips).expect("the mixer bay draws its strips")
}

fn point(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
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

/// **A press moves the mask to the next shape, and the last wraps to the
/// first.**
///
/// Asserted against [`CYCLE`] walked in order rather than against three
/// literals in three assertions, so this is *the cycle is that order* and not
/// *the cycle is the three lines somebody wrote in `view.rs`*. `view::next_shape`
/// is a match — a fourth shape does not compile until somebody says what
/// follows it — and this is the second copy that keeps it honest, since there
/// is no `Mask::ALL` for it to be checked against.
///
/// **The wrap is not a special case in the assertion.** The loop's last step
/// is `radial` and the expected answer is `CYCLE[0]`, reached by the same
/// modulo every other step uses, so a cycle that ran off the end would fail
/// here rather than in a test of its own that could be forgotten.
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

/// **The press carries the angle the slot is already wearing, and never a
/// default.**
///
/// This is the test that separates this design from the wrong one, and the
/// wrong one is tidy: the chip names a *shape*, the angle is not its business,
/// so send `0.0` and let the shape be the whole of what a press means. It is
/// wrong because `Record::Mask` is written whole out of what the operation
/// says — so a press that named a zero angle would **straighten a diagonal
/// wipe**, on a control whose entire visible business is choosing between a
/// circle and a straight edge, and the mark would look the same afterwards.
///
/// **Both directions, and at every shape.** The answer must carry the strip's
/// own angle and must not carry zero, so an implementation that sent a default
/// fails here with a message saying which mistake it made rather than a
/// mismatch of two operations.
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

/// **The operation names the strip's own deck, the shape after that strip's
/// own, and that strip's own angle** — not deck 0, and not the first strip's
/// values.
///
/// This is the test a hard-coded `deck: 0` has to fail: every strip is pressed
/// and each answer carries its own index. The strips are seeded so that **no
/// two adjacent decks wear the same shape and no two decks share an angle**,
/// so an answer read off the wrong strip is wrong in the payload as well as in
/// the deck — any one of the three would catch a wrong index even if the other
/// two were removed.
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

/// **A press off the mini asks for nothing and is not claimed.**
///
/// `input`'s rule 3: *"a control claims what it acts on and no more."* The
/// number above it, the meter, the strip's own well and the ground between two
/// strips are all painted by the console and none of them is a control, so
/// `egui` gets the event — which owns no widget there either, so the two
/// answers are the same nothing, arrived at without the panel claiming a press
/// it would throw away.
///
/// **Both halves, because either alone is satisfiable by the wrong code.** A
/// mini that asked for nothing but was claimed would take presses it does
/// nothing with; a mini that asked from anywhere would change a deck's mask
/// from a press on the number above it.
#[test]
fn a_press_off_the_mini_asks_for_nothing_and_is_not_claimed() {
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
            claim(&mut panel, &ctx, &strips, point(probe)),
            Claim::Egui,
            "{what} is being claimed as a control the panel acts on"
        );
    }

    // **The two neighbours that are controls** — the blend chip 3px to the
    // left and the tally chip four rows up — are asserted separately and in
    // the other direction: the panel claims each, and the mask answers nothing
    // for either. A hit test that reached across would change a deck's mask
    // from a press meant for the blend or the residency.
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
            claim(&mut panel, &ctx, &strips, point(probe)),
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
        claim(&mut panel, &ctx, &strips, point(on)),
        Claim::Panel,
        "the mini is not the panel's, so it is drawn where it cannot be clicked"
    );
}

/// **A mini in a bay that is not laid out is not a control**, whatever mask
/// the strips behind it carry.
///
/// The strips are written every frame from the `Deck` and say nothing about
/// the arrangement, so *is there a mini here* is `mixer`'s question and not
/// theirs — and the answer is `None` for a bay with no room for its row of
/// strips, before any rectangle is hit-tested. That is where a folded bay is
/// handled, once for every control in it rather than per control.
///
/// **Both folds, because they are one question with two ways in** — the mixer
/// bay itself, and the pane that encloses it. And unfolding puts the control
/// back, because nothing here is a latch.
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
        assert_eq!(
            claim(&mut panel, &ctx, &strips, point(where_it_was)),
            Claim::Egui,
            "where the mini used to be is still claimed with the bay folded away \
             (enclosing: {enclosing})"
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

/// **No mask mini is inside a boundary's [`GRAB`].**
///
/// `input`'s rule 2 comes before rule 3, so a control under a boundary's grab
/// band is a control that cannot be clicked, with nothing on screen saying so.
/// The Outputs sink, the two knobs, the blend chip and the tally chip are each
/// measured for this — **and this is measured too rather than inherited from
/// the chip 3px to its left**, which is the mistake the numbers make easy: the
/// blend chip clears the pane divider down the left of the bay by **8.97** and
/// this one clears it by **41.03**, because it sits at the far end of the same
/// row.
///
/// **It is also the one control whose nearest boundary is not the same one on
/// every strip.** On deck A the pane divider is nearest, at 41.03; on the other
/// three the nearest is the boundary *under* the bay, **74.50** below the mode
/// row. A clearance inherited from the control beside it would have named the
/// wrong boundary and the wrong number at once.
///
/// It asks `Layout::hit` directly as well as `claim`, which is ADR-0185's
/// caught test: `claim` says *the panel's* for a boundary **and** for a
/// control, so a version of this that only asked `claim` passes with [`GRAB`]
/// widened to 60.
///
/// The guard on itself is `fader.rs`'s, `blend.rs`'s and `tally.rs`'s: the
/// ground under the bay *is* inside a grab, which is what says the answers
/// above are the clearance rather than the grab having gone missing — and the
/// minis have to have been found at all, or a bay with no strips passes this
/// trivially.
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
                claim(&mut panel, &ctx, &strips, point(probe)),
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

/// **The mini a hand aims at stands still whatever shape the deck is
/// wearing.**
///
/// The mark is drawn rather than typed and the box is `MINI_SIZE` wide inside
/// `.mini`'s padding whichever shape it is showing — so this control has what
/// the tally's capsule buys by being sized to the widest word, and what the
/// blend chip, sized to the word it shows, cannot claim at all.
///
/// The rectangle is asserted **identical** across all three shapes, and one
/// fixed point on it is a control in every one of them.
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
