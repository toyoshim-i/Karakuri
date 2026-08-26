//! **The mixer strip's blend chip, clicked.**
//!
//! `mixer.rs` is where a strip's rectangles are and which of them are
//! controls; `fader.rs` is what a hand does to the two knobs. This is the
//! third control (ADR-0187): the chip cycles, and a press on it emits
//! `Operation::SetBlendMode` naming the mode it **arrived at**.
//!
//! **The cycle is the affordance and the operation is the destination**, which
//! is P-0074's own worked example — *"a mini that cycles the blend is one
//! control emitting three. The operator sees a toggle; the vocabulary never
//! does."* So what is asserted here is a named destination per press, the wrap
//! from the last mode back to the first, and that the operation names the
//! **strip's own deck** rather than a fixed one.
//!
//! **None of it needs a device.** Laying a strip out needs `egui`, because the
//! chip is as wide as the word in it — `mixer.rs`'s own opening — and what
//! comes out is `karakuri-operation`'s, which has no dependencies at all.
//!
//! # Where this stops
//!
//! At the operation, exactly as `fader.rs` does. Turning it into a
//! `Record::Blend` and moving a deck with it is the harness's —
//! `examples/panel.rs`, where there is a deck — and this crate has none
//! (ADR-0156).

mod common;

use common::{drawn_once, rect_of, PLAUSIBLE};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Panel, GRAB};
use karakuri_console::room::size;
use karakuri_console::view::{mixer, Level, Mask, Mixer, Strip, Tally};
use karakuri_layout::{Hit, Point};
use karakuri_operation::{BlendMode, Operation};

/// Four strips, **no two adjacent ones on the same blend**, so that a press
/// answered from the wrong strip is a wrong answer rather than the right one
/// by luck. `BlendMode::ALL` is three and there are four decks, so deck A and
/// deck D share a mode and neither is beside the other.
///
/// The other values are apart from each other for `fader.rs`'s reason: a bay
/// that read the wrong strip anywhere says so.
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
            opacity: 0.8 - 0.15 * slot as f32,
            blend: BlendMode::ALL[slot % BlendMode::ALL.len()],
            mask: Mask::None,
            level: Some(Level {
                mean: 0.5,
                peak: 0.6,
            }),
        })
        .collect()
}

/// A panel at a viewport, solved, with a context that has drawn once — the
/// pair `mixer.rs`, `fader.rs` and `transport.rs` all open with.
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

/// **A press moves the blend to the next mode, and the last wraps to the
/// first.**
///
/// Asserted against `BlendMode::ALL` walked in order rather than against three
/// literals, so this is *the cycle is `ALL`'s order* and not *the cycle is the
/// three lines somebody wrote in `view.rs`*. `view::after` is a match — a
/// fourth mode does not compile until somebody says what follows it — and the
/// price of a match is that the order is written twice; this is the
/// measurement that keeps the two copies from drifting.
///
/// **The wrap is not a special case in the assertion.** The loop's last step
/// is `max` and the expected answer is `ALL[0]`, reached by the same modulo
/// every other step uses, so a cycle that ran off the end would fail here
/// rather than in a test of its own that could be forgotten.
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

/// **The operation names the strip's own deck, and the mode that strip is
/// on** — not deck 0, and not the first strip's blend.
///
/// This is the test a hard-coded `deck: 0` has to fail: every strip is pressed
/// and each one's answer carries its own index. The strips are seeded so that
/// **no two adjacent decks share a mode**, so an answer read off the wrong
/// strip is wrong in the blend as well as in the deck — one of the two would
/// catch a wrong index even if the other were removed.
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

/// **A press off the chip emits nothing and is not claimed.**
///
/// `input`'s rule 3: *"a control claims what it acts on and no more."* The
/// mask mini beside it, the number above it, the strip's own well and the
/// ground between two strips are all painted by the console and none of them
/// is a control, so `egui` gets the event — which owns no widget there either,
/// so the two answers are the same nothing, arrived at without the panel
/// claiming a press it would throw away.
///
/// **Both halves, because either alone is satisfiable by the wrong code.** A
/// chip that emitted nothing but was claimed would take presses it does
/// nothing with; a chip that emitted from anywhere would change the mix from a
/// press on the mask.
#[test]
fn a_press_off_the_chip_asks_for_nothing_and_is_not_claimed() {
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
        (at.mask.center(), "the mask mini"),
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
            claim(&mut panel, &ctx, &strips, point(probe)),
            Claim::Egui,
            "{what} is being claimed as a control the panel acts on"
        );
    }

    // **The tally chip is the one neighbour that is a control** (ADR-0195), so
    // it is asserted separately and in the other direction: the panel claims
    // it, and the blend answers nothing for it. A hit test that reached across
    // the two would change a blend mode from a press meant for the residency.
    assert_eq!(
        bay.blend(point(at.tally.center())),
        None,
        "the tally chip asked the blend to change"
    );
    assert_eq!(
        claim(&mut panel, &ctx, &strips, point(at.tally.center())),
        Claim::Panel,
        "the tally chip stopped being a control, so this is asserting nothing about the blend"
    );

    // The guard: the chip itself does both of the things the probes above do
    // neither of. Without this the test passes on a chip that was never a
    // control at all.
    let on = at.blend.center();
    assert!(bay.blend(point(on)).is_some(), "the chip asked for nothing");
    assert_eq!(
        claim(&mut panel, &ctx, &strips, point(on)),
        Claim::Panel,
        "the chip is not the panel's, so it is drawn where it cannot be clicked"
    );
}

// ---------------------------------------------------------------------------
// The boundary gets first refusal, and the chip clears its band
// ---------------------------------------------------------------------------

/// **No blend chip is inside a boundary's [`GRAB`].**
///
/// `input`'s rule 2 comes before rule 3, so a control under a boundary's grab
/// band is a control that cannot be clicked, with nothing on screen saying so.
/// The Outputs sink is measured for this and so are the two knobs — **and this
/// is measured too rather than inherited from them**, because the chip is at
/// the *bottom* of a strip and the knobs are in the middle of one, which is a
/// different clearance against a different boundary.
///
/// It asks `Layout::hit` directly as well as `claim`, which is ADR-0185's
/// caught test: `claim` says *the panel's* for a boundary **and** for a
/// control, so a version of this that only asked `claim` passed with `GRAB`
/// widened to 60.
///
/// The guard on itself is the same one `fader.rs` carries: the ground under
/// the bay *is* inside a grab, which is what says the answers above are the
/// clearance rather than the grab having gone missing — and the chips have to
/// have been found at all, or a bay with no strips passes this trivially.
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
                claim(&mut panel, &ctx, &strips, point(probe)),
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

/// **The mode row stays inside the strip's track, at every mode.**
///
/// `.strip-mode` is two minis centred across the strip, and **it already
/// overflows the strip's content box** — a strip is 53 wide inside `.strip`'s
/// `padding: 7px 4px` and the row is 55.06, 57.84 or 56.84 depending on the
/// word, so it spills 1.03 to 2.42 each side into that 4px of padding. Nothing
/// clamps it. That is fine today and it is fine by 1.58 at the worst mode,
/// which is the width of the *gap* between two strips being what stops a chip
/// reaching its neighbour.
///
/// **A fact that holds by 1.58 is a test rather than a sentence.** A fourth
/// mode with a longer word, a bigger `MINI_SIZE`, more `MINI_PAD_X` or a wider
/// mask chip each move the row outward, and the failure without this test is
/// silent: one strip's chip painted over the next strip's, hit-tested by
/// whichever `Mixer::blend` reaches first.
///
/// It asserts containment in the **track** — the 61 the grid gives a strip,
/// which is `StripBox::rect` — because that is the boundary that matters: the
/// tracks tile with `STRIP_GAP` between them, so a row inside its own track
/// cannot be inside anyone else's.
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

        // The overflow is real and is the reason this test exists: the row is
        // wider than the strip's content box at every mode. If that ever
        // stops being true the clearance above is no longer the tight thing
        // this was written to watch, and the message says so rather than the
        // test quietly going on measuring nothing.
        let at = bay.strip(0);
        let content = at.rect.width() - size::STRIP_PAD_X * 2.0;
        let row = at.mask.max.x - at.blend.min.x;
        assert!(
            row > content,
            "at `{}` the mode row is {row:.4} and the strip's content box is {content:.4} — \
             the row no longer overflows, so re-read this test before trusting it",
            blend.name()
        );
    }

    assert_eq!(
        measured,
        BlendMode::ALL.len() * strips.len(),
        "this test measured no rows, so it is asserting nothing"
    );
}
