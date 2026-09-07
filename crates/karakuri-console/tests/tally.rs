//! **The mixer strip's tally chip, pressed.**
//!
//! `mixer.rs` is where a strip's rectangles are and which of them are
//! controls; `parked.rs` is what the chip *draws* while a request has not
//! landed; `blend.rs` is the chip one row down. This is the fourth control
//! (ADR-0195): the tally cycles, and a press on it emits
//! `Operation::SetResidency` naming the residency it **arrived at**.
//!
//! **The step is taken from the residency that was requested**, which is the
//! decision this file exists to hold. The requested and the effective
//! residency are the same value on every settled slot and they part on exactly
//! one state the engine can produce — **parked**, asked to prime and held at
//! allocated — so that state is where cycling from the request and cycling
//! from the readout give different answers, and it is the state
//! `a_parked_chip_asks_for_the_withdrawal_rather_than_going_on_air` is written
//! at. From the request the next is `Allocated`, which *is* the withdrawal of
//! the prime request; from the readout it would be `Live`, and a press meant
//! to take a request back would put the deck on air.
//!
//! That is the affordance a surface owns and not a lock it holds
//! ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)):
//! the chip refuses nothing, and `karakuri-cli`'s `w` — `toggle_priming`,
//! which reads `Deck::requested_residency` to choose its direction — is the
//! same choice on another surface.
//!
//! **None of it needs a device.** Laying a strip out needs `egui`, because the
//! capsule is as wide as the widest of the three words, and what comes out is
//! `karakuri-operation`'s, which has no dependencies at all.
//!
//! # Where this stops
//!
//! At the operation, exactly as `blend.rs` and `fader.rs` do. Turning it into
//! a `Record::Residency`, applying it and letting the governor answer is the
//! harness's — `crates/karakuri/src/main.rs`, where there is a deck — and this crate has
//! none (ADR-0156).

mod common;

use common::{drawn_once, rect_of, showing, PLAUSIBLE};
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
    }
}

/// **Four strips, no two adjacent ones on the same residency**, so that a
/// press answered from the wrong strip is a wrong answer rather than the right
/// one by luck. `Tally::ALL` is three and there are four decks, so deck A and
/// deck D share a residency and neither is beside the other.
fn strips() -> Vec<Strip> {
    (0..4)
        .map(|slot| settled(slot, Tally::ALL[slot % Tally::ALL.len()]))
        .collect()
}

/// **The parked strip**, which is the one state the engine can actually
/// produce where the two residencies disagree: `Deck::is_parked` is
/// `requested == Priming && effective == Allocated` — asked to prime, and held
/// at allocated because the budget has not found room.
fn parked(slot: usize) -> Strip {
    Strip {
        tally: Tally::Allocated,
        requested: Tally::Priming,
        ..settled(slot, Tally::Allocated)
    }
}

/// A panel at a viewport, solved, with a context that has drawn once — the
/// pair `mixer.rs`, `blend.rs` and `fader.rs` all open with.
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
    let at = bay.strip(slot).tally.center();
    bay.tally(point(at))
        .unwrap_or_else(|| panic!("nothing on the tally chip of strip {slot} at {at:?}"))
}

/// **The vocabulary's word for a console residency, written out again here.**
///
/// `view`'s own conversion is private, and a test that asked it for the
/// expected answer would be asserting that it agrees with itself. Two arms
/// swapped in either copy is a chip that asks for the wrong state, and it
/// fails here.
fn asked(tally: Tally) -> Residency {
    match tally {
        Tally::Live => Residency::Live,
        Tally::Priming => Residency::Priming,
        Tally::Allocated => Residency::Allocated,
    }
}

/// The residency the cycle arrives at from `tally`, as the operation names it
/// — `Tally::ALL` walked one step on, wrapping.
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

/// **A press moves the residency to the next of the three, and the last wraps
/// to the first.**
///
/// The order is the mock's own, which `docs/manual/console.html` states on
/// every tally tooltip — *"one of three residencies — live, priming,
/// allocated"* — and which `Tally::ALL` is in. Asserted against `ALL` walked in
/// order rather than against three literals, so this is *the cycle is `ALL`'s
/// order* and not *the cycle is the three lines somebody wrote in `view.rs`*:
/// `view::next` is a match, so a fourth residency does not compile until
/// somebody says what follows it, and the price of a match is that the order
/// is written twice.
///
/// **The wrap is not a special case in the assertion.** The loop's last step
/// is `alloc` and the expected answer is `ALL[0]`, reached by the same modulo
/// every other step uses, so a cycle that ran off the end would fail here
/// rather than in a test of its own that could be forgotten.
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

/// **A press on a parked chip asks for the withdrawal of its own prime
/// request, and not for the next of the residency it is showing.**
///
/// This is the test that separates *cycles from the request* from *cycles from
/// the readout*, and the parked slot is the only state the engine can produce
/// where the two differ. A parked slot shows `alloc` and was asked for `prim`:
///
/// - from the **request**, the next is `alloc` — which is the prime request
///   withdrawn, arrived at by the ordinary arithmetic with no case in the code
///   for it, and what `karakuri-cli`'s `w` does on the same slot;
/// - from the **readout**, the next would be `live` — a press meant to take a
///   request back putting the deck on air, in front of an audience.
///
/// **Both are asserted**, the second by name: the answer must be the one and
/// must not be the other, so a rewrite that read `Strip::tally` fails here
/// with a message that says which mistake it made rather than a mismatch of
/// two enum values.
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

/// **The operation names the strip's own deck, and the residency that strip
/// was asked for** — not deck 0, and not the first strip's residency.
///
/// This is the test a hard-coded `deck: 0` has to fail: every strip is pressed
/// and each one's answer carries its own index. The strips are seeded so that
/// **no two adjacent decks share a residency**, so an answer read off the
/// wrong strip is wrong in the residency as well as in the deck — one of the
/// two would catch a wrong index even if the other were removed.
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

    // And the four answers are not all the same operation, which is what says
    // the loop above compared anything: a chip that answered the same thing
    // everywhere would satisfy neither the deck nor the residency. Every pair
    // differs, because the deck alone differs even where two strips share a
    // residency.
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

/// **A press off the chip asks for nothing, and what it does instead is select
/// the deck.**
///
/// `input`'s rule 4: *"a control claims what it acts on and no more."* The
/// name above it, the trim below it, the number and the meter are painted by
/// the console and none of them is the tally chip, so this chip answers
/// nothing for any of them. They are all inside the strip, though, and the
/// strip is itself a control (`Mixer::select`) — so the press is the panel's
/// and it means *address the keys to this deck*.
///
/// **Both halves, because either alone is satisfiable by the wrong code.** A
/// chip that asked from anywhere would move a deck's residency from a press on
/// the name above it; a chip whose neighbours answered nothing at all would be
/// a column an operator cannot select by pressing.
///
/// **This test required `Claim::Egui` at those points until 2026-09-07.** That
/// was true when it was written and stopped being true on 2026-08-30, when
/// `Mixer::select` made the whole column a control and `input::on_strip` went
/// on asking four questions instead of five. The rule has not changed; what is
/// *left over* after the four inside the column is no longer nothing.
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

    // **The mode row's two chips are controls**, so they are asserted
    // separately and in the other direction: the panel claims each, and the
    // tally answers nothing for either. A hit test that reached across would
    // change a residency from a press meant for the blend or for the mask
    // shape.
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

/// **A chip in a bay that is not laid out is not a control**, however much
/// residency the strips behind it carry.
///
/// The strips are written every frame from the `Deck` and say nothing about
/// the arrangement, so *is there a chip here* is `mixer`'s question and not
/// theirs — and the answer is `None` for a bay with no room for its row of
/// strips, before any rectangle is hit-tested. That is where a folded bay is
/// handled, and it is handled once for every control in it rather than per
/// control.
///
/// **Both folds, because they are one question with two ways in** — the
/// mixer bay itself, and the pane that encloses it (`parked.rs` makes the same
/// pair for the declaration). And unfolding puts the control back, because
/// nothing here is a latch.
///
/// # What claims the ground afterwards is not always nothing
///
/// This used to assert that the point goes to `egui` once the bay is folded,
/// and that stopped being true the day the deck preview cells became controls.
/// Folding the **right pane** gives its width to the centre, which is enough
/// for the Program bay to put its four cells down the sides of the picture
/// instead of under it (ADR-0182) — and the right-hand column lands in the
/// ground the mixer had. So a press where the chip was is the panel's again,
/// for a control that moved in rather than for the one that went.
///
/// **That is a fact about the arrangement and not about this bay**, so it is
/// derived rather than written down per arm — the same repair `mask.rs` owed
/// one chip along, and for the same reason. What this test still asserts about
/// the chip is what it always did, one line above: `mixer` answers `None`, so
/// there is no chip to press whatever else is on the screen.
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

/// **No tally chip is inside a boundary's [`GRAB`].**
///
/// `input`'s rule 2 comes before rule 3, so a control under a boundary's grab
/// band is a control that cannot be clicked, with nothing on screen saying so.
/// The Outputs sink is measured for this, and so are the two knobs and the
/// blend chip — **and this is measured too rather than inherited from any of
/// them**. The nearest boundary to either chip is the pane divider down the
/// left of the bay, and the two clear it by different numbers for different
/// reasons: the blend chip is as wide as its whole mode row and clears it by
/// **8.97**, while the tally's capsule is 44.53 in a 61 track and clears it by
/// **14.23**, both against a [`GRAB`] of 6. Widening the grab to 9 kills the
/// blend chip and the tally goes on working until 14.25 — which is the shape
/// of the mistake this test exists to stop, since a clearance inherited from
/// the control beside it would read as measured.
///
/// It asks `Layout::hit` directly as well as `claim`, which is ADR-0185's
/// caught test: `claim` says *the panel's* for a boundary **and** for a
/// control, so a version of this that only asked `claim` passes with `GRAB`
/// widened to 60.
///
/// The guard on itself is the same one `fader.rs` and `blend.rs` carry: the
/// ground under the bay *is* inside a grab, which is what says the answers
/// above are the clearance rather than the grab having gone missing — and the
/// chips have to have been found at all, or a bay with no strips passes this
/// trivially.
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

/// **The chip a hand aims at stands still whatever the deck is doing.**
///
/// The capsule is sized to the widest of the three words rather than to the
/// one it is showing (`parked.rs` asserts that width against the three
/// galleys, and `mixer` measures it once for the bay), so this control has
/// something the blend chip cannot claim: `ALLOC` is ten and a half pixels
/// wider than `LIVE`, and a chip sized to the word would move its own left
/// edge by five every time a deck went on air — and would move it *while a
/// word rolls through it* on a parked slot, which is a target sliding under a
/// finger already on its way down.
///
/// So the rectangle is asserted **identical** across all three residencies and
/// across the parked pair, and one fixed point — the far left of the capsule
/// at `alloc`, the widest — is asserted to be on the control in every one of
/// them.
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
