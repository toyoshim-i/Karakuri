//! **A residency request that has not landed, and what the strip does about
//! it.**
//!
//! `Deck::residency` is what a slot is doing and `Deck::requested_residency`
//! is what it was asked to do, and the mixer's tally is the first control on
//! this panel whose readback can disagree with what was asked for. What it has
//! to say then is
//! [P-0087](../../../docs/principles/0087-name-the-property-never-the-shape.md)'s
//! — where it is, where it is going, and that it has not arrived — and the
//! presentation is
//! [ADR-0190](../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md)'s:
//! the word rolls part of the way toward the one that was asked for, once a
//! second, and never lands.
//!
//! Seven claims, and each one is a thing that could quietly not be true:
//!
//! 1. **The chip is as wide as the widest residency word**, whichever it is
//!    showing — a capsule sized to the current word resizes when the deck
//!    moves, and would resize under a word rolling through it. *This one
//!    failed before the change that added the rest of this file.*
//! 2. **A strip whose two halves agree is still**, draws one word, and asks
//!    for no repaint at all. P-0072's first clause is the one thing an
//!    animation is most likely to cost by accident.
//! 3. **The roll is a function of the phase it was handed** — asserted at
//!    phases this file chose, which is the whole reason the phase is a value
//!    written by the harness rather than a clock read in `src/`.
//! 4. **The rolling word never leaves the chip.** The clip is new here;
//!    nothing called `with_clip_rect` on a tally before.
//! 5. **The two words never meet**, whatever the displacement: a blank band of
//!    the chip's own slack separates them, and at 9px two words with nothing
//!    between them are mud.
//! 6. **The panel asks for a deadline while a slot is parked and for `Never`
//!    when none is** — the declaration end of P-0072, from the view, which is
//!    what knows the rate.
//! 7. **And for `Never` while the bay the chip is in is out of the layout**,
//!    however parked the slot behind it is. A declaration answered off the
//!    deck alone bought a 30 Hz deadline for a chip nobody could see, which is
//!    P-0072's *what must be live* read as *what is pending*.
//!
//! None of it needs a window, a device or a clock.

mod common;

use std::time::Duration;

use common::{drawn_once, near, PLAUSIBLE};
use karakuri_console::panel::{Op, Panel};
use karakuri_console::repaint::{Change, Repaint};
use karakuri_console::room::{size, Room};
use karakuri_console::view::{
    mixer, roll_at, Level, Mask, Phase, Strip, StripBox, Tally, View, ROLL_PERIOD, ROLL_REACH,
    ROLL_STALENESS, ROLL_TRAVEL,
};
use karakuri_layout::NodeId;
use karakuri_operation::BlendMode;

/// A strip that is where it was asked to be — the ordinary case, and what
/// every other test file in this crate is made of.
fn settled(tally: Tally) -> Strip {
    Strip {
        name: "glass_shell".to_owned(),
        tally,
        requested: tally,
        gain: 0.44,
        gain_to: None,
        opacity: 0.3,
        opacity_to: None,
        blend: BlendMode::Over,
        mask: Mask::Linear,
        mask_angle: 0.0,
        level: Some(Level {
            mean: 0.12,
            peak: 0.12,
        }),
    }
}

/// **The parked strip**, which is the one state the engine can actually
/// produce: `Deck::is_parked` is `requested == Priming && effective ==
/// Allocated` — asked to prime, and held at allocated because the budget has
/// not found room.
fn parked() -> Strip {
    Strip {
        tally: Tally::Allocated,
        requested: Tally::Priming,
        ..settled(Tally::Allocated)
    }
}

/// **The console's arrangement at a plausible window, solved** — every region
/// laid out, which is the state `View::animating` is asked in everywhere below
/// except where a fold is the thing under test.
fn arrangement() -> Panel {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    panel
}

/// The node the arrangement knows by `name`.
fn node(panel: &Panel, name: &str) -> NodeId {
    panel
        .layout()
        .find(name)
        .unwrap_or_else(|| panic!("the arrangement names `{name}`"))
}

/// One strip's box, off a panel at a plausible window.
fn box_of(strips: &[Strip]) -> StripBox {
    let panel = arrangement();
    let ctx = drawn_once();
    mixer(&ctx, panel.layout(), strips)
        .expect("the mixer bay draws its strips")
        .strip(0)
}

/// **Every word painted in the first strip's tally, with the clip it was
/// painted under.**
///
/// `mixer.rs`'s `shapes_inside` drops the clip — it maps each `ClippedShape`
/// to its shape — and the clip is half of what this file asserts, so the whole
/// `ClippedShape` is kept here. Filtered by the galley's *position* rather
/// than by its bounds, because a word halfway out of the chip is exactly what
/// this is looking for.
fn words_at(strips: Vec<Strip>, phase: Phase) -> (StripBox, Vec<(String, egui::Rect, egui::Rect)>) {
    let at = box_of(&strips);
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let mut view = View::new(Room::Day);
    view.mixer = strips;
    view.phase = phase;
    let ctx = drawn_once();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, &mut panel));
    out.textures_delta.clear();
    let words = out
        .shapes
        .into_iter()
        .filter_map(|clipped| match clipped.shape {
            egui::Shape::Text(text) if at.tally.x_range().contains(text.pos.x) => {
                let bounds = egui::Rect::from_min_size(text.pos, text.galley.size());
                (bounds.center().y - at.tally.center().y)
                    .abs()
                    .lt(&(size::TALLY_H * 2.0))
                    .then(|| (text.galley.text().to_owned(), bounds, clipped.clip_rect))
            }
            _ => None,
        })
        .collect();
    (at, words)
}

/// The distance one word is displaced from where it sits at rest, positive
/// upward — which is what the roll moves.
fn lifted(bounds: egui::Rect, at: &StripBox) -> f32 {
    at.tally.center().y - bounds.center().y
}

// ---------------------------------------------------------------------------
// 1. The chip's width
// ---------------------------------------------------------------------------

/// **The tally's capsule is the widest residency word's, whatever residency it
/// is showing.**
///
/// It was the current word's, and that was a defect rather than a
/// simplification: `LIVE` is 34.06 wide, `PRIM` 37.59 and `ALLOC` 44.53
/// (galley plus two `TALLY_PAD_X`), so a slot going from allocated to live
/// shrank its own capsule by ten and a half pixels while the strip around it
/// stood still. A word rolling through a box that resizes as it rolls would be
/// the same defect in motion.
///
/// **The word does not move when the box grows**, and that is asserted here
/// too: the capsule is centred in the strip and the word is centred in the
/// capsule, so widening it grows the capsule symmetrically around type that
/// was already on the strip's centre line. It is why fixing this changed the
/// chip and nothing inside it.
#[test]
fn the_chip_is_the_widest_words_width_whatever_it_shows() {
    let ctx = drawn_once();
    let widest = Tally::ALL
        .into_iter()
        .map(|tally| {
            let mut job = egui::text::LayoutJob::default();
            job.append(
                &tally.word().to_uppercase(),
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::new(size::TALLY_SIZE, egui::FontFamily::Proportional),
                    extra_letter_spacing: size::TALLY_TRACKING,
                    color: egui::Color32::PLACEHOLDER,
                    ..Default::default()
                },
            );
            ctx.fonts_mut(|f| f.layout_job(job).size().x)
        })
        .fold(0.0f32, f32::max)
        + size::TALLY_PAD_X * 2.0;

    for tally in Tally::ALL {
        let strips = vec![settled(tally)];
        let at = box_of(&strips);
        assert!(
            near(at.tally.width(), widest),
            "a {tally:?} chip is {} wide and the widest of the three words needs {widest}",
            at.tally.width()
        );

        // And the word itself is where it always was: centred on the strip.
        let (at, words) = words_at(strips, Phase::ZERO);
        let (word, bounds, _) = words.first().expect("the chip draws its word");
        assert_eq!(word, &tally.word().to_uppercase());
        assert!(
            near(bounds.center().x, at.rect.center().x),
            "a {tally:?} word is centred at {} and the strip's centre is {}",
            bounds.center().x,
            at.rect.center().x
        );
    }
}

// ---------------------------------------------------------------------------
// 2. A settled strip
// ---------------------------------------------------------------------------

/// **A strip whose request and effective residency agree draws one word, at
/// rest, and asks for nothing.**
///
/// Both halves matter and they fail differently. A chip that painted the
/// second word unconditionally would draw it clipped away and cost a galley a
/// frame for nothing — invisible, and the kind of thing that is discovered by
/// a profiler years later. A panel that declared a staleness unconditionally
/// would end the still panel outright: P-0072's first clause is *a panel with
/// nothing changing on it is paid for once and not again*, and the whole
/// console is on the other side of it.
#[test]
fn a_settled_strip_is_still_and_asks_for_nothing() {
    for tally in Tally::ALL {
        // Whatever the phase, because nothing here reads it.
        for millis in [0, 137, 250, 900] {
            let (at, words) = words_at(
                vec![settled(tally)],
                Phase::since(Duration::from_millis(millis)),
            );
            assert_eq!(
                words.len(),
                1,
                "a settled {tally:?} strip painted {} words in its chip at {millis} ms",
                words.len()
            );
            let (word, bounds, _) = &words[0];
            assert_eq!(word, &tally.word().to_uppercase());
            assert!(
                near(lifted(*bounds, &at), 0.0),
                "a settled {tally:?} word is lifted {} at {millis} ms and nothing is pending",
                lifted(*bounds, &at)
            );
        }

        let mut panel = arrangement();
        let mut view = View::new(Room::Day);
        view.mixer = vec![settled(tally)];
        // **Whether the bay is drawn or folded**, because a slot that is not
        // parked has nothing to declare either way — the fold is what stops a
        // *pending* thing being paid for, and it is not a second reason to be
        // still.
        assert_eq!(
            view.animating(panel.layout()),
            None,
            "a settled {tally:?} strip declared a staleness with the bay drawn"
        );
        assert_eq!(
            Change::Animating(view.animating(panel.layout())).repaint(),
            Repaint::Never,
            "a console with nothing pending asked for a frame"
        );
        panel.op(Op::Fold(node(&panel, "mixer")));
        assert_eq!(
            view.animating(panel.layout()),
            None,
            "a settled {tally:?} strip declared a staleness with the bay folded"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. The roll is the phase's
// ---------------------------------------------------------------------------

/// **The displacement is a function of the phase the view was handed**, and
/// this asserts it at phases chosen here.
///
/// That is the point of the phase being a value rather than a clock: a test
/// *chooses* the phase it asserts at and never catches one, which is
/// [ADR-0189](../../../docs/adr/0189-motion-may-carry-the-meaning-and-a-stopped-animation-is-a-fault-to-report.md)'s
/// own argument. There is no sampling here and no tolerance for one.
///
/// Two phases, and each is a different thing about the curve:
///
/// - **100 ms** is a quarter of the way through the 400 ms travel, where the
///   raised cosine is at `ROLL_REACH * 0.5 * (1 - cos(τ/4))` — half its reach.
/// - **200 ms** is the top of the travel, and it is `ROLL_REACH` exactly:
///   the furthest the word ever gets, and it is not 1.0, because landing is
///   what arrival looks like.
///
/// And two more that are about the shape rather than a value: **700 ms** is in
/// the 600 ms the word rests for, and one whole period later is the same
/// answer, which is what makes it a phase.
#[test]
fn the_roll_is_a_function_of_the_phase_it_was_handed() {
    let quarter = Phase::since(Duration::from_millis(100));
    let top = Phase::since(Duration::from_millis(200));

    assert!(
        near(roll_at(quarter), ROLL_REACH * 0.5),
        "a quarter of the way through the travel is {} and the curve says {}",
        roll_at(quarter),
        ROLL_REACH * 0.5
    );
    assert!(
        near(roll_at(top), ROLL_REACH),
        "the top of the travel is {} and the reach is {ROLL_REACH}",
        roll_at(top)
    );
    assert!(
        roll_at(top) < 1.0,
        "the roll reached {} of the way and a roll that lands says it arrived",
        roll_at(top)
    );
    assert!(
        near(roll_at(Phase::since(Duration::from_millis(700))), 0.0),
        "the word is not at rest between one roll and the next"
    );
    assert!(
        near(
            roll_at(Phase::since(ROLL_PERIOD + Duration::from_millis(100))),
            roll_at(quarter)
        ),
        "one period later is a different answer, so this is not a phase"
    );

    // And the chip is painted at it. The pitch is the box, so the lift is the
    // fraction of a box height the curve says.
    for (phase, what) in [(quarter, "a quarter through"), (top, "at the top")] {
        let (at, words) = words_at(vec![parked()], phase);
        assert_eq!(
            words.len(),
            2,
            "a parked chip draws the word and its destination"
        );
        let lift = lifted(words[0].1, &at);
        assert!(
            near(lift, roll_at(phase) * size::TALLY_H),
            "{what} the word is lifted {lift} and the curve at that phase asks for {}",
            roll_at(phase) * size::TALLY_H
        );
    }

    // Two phases, two different pictures — which is the claim the two exact
    // values above are evidence for, stated so that a roll frozen at one
    // displacement fails here and not only there.
    let (at_a, a) = words_at(vec![parked()], quarter);
    let (at_b, b) = words_at(vec![parked()], top);
    assert!(
        lifted(a[0].1, &at_a) < lifted(b[0].1, &at_b),
        "the word is at the same height at two phases, so it is not reading the phase"
    );
}

// ---------------------------------------------------------------------------
// 4. The clip
// ---------------------------------------------------------------------------

/// **Nothing the roll paints leaves the capsule.**
///
/// The destination word is a whole box height below the settled one and the
/// roll never brings it more than [`ROLL_REACH`] of the way up, so most of it
/// is outside the chip on every frame it is drawn — and directly over the trim
/// row underneath, which is 5px away. Without the clip a parked strip paints
/// `PRIM` across its own `g` and fader.
///
/// The clip is asserted on the `ClippedShape` rather than by looking at where
/// the ink lands, because that is the mechanism: `epaint` clips at tessellation
/// and the shape's own bounds are unchanged by it. **Vertically, and not
/// horizontally**: the capsule is the widest word's, so nothing ever needs
/// clipping sideways, and asserting a bound that is never approached would be
/// asserting the wrong thing.
#[test]
fn the_rolling_words_never_leave_the_chip() {
    for millis in [0, 50, 100, 150, 200, 250, 300, 350, 399, 500, 999] {
        let phase = Phase::since(Duration::from_millis(millis));
        let (at, words) = words_at(vec![parked()], phase);
        assert_eq!(
            words.len(),
            2,
            "a parked chip draws two words at {millis} ms"
        );
        for (word, _, clip) in &words {
            assert!(
                clip.min.y >= at.tally.min.y - common::EPS
                    && clip.max.y <= at.tally.max.y + common::EPS,
                "{word} at {millis} ms is clipped to {clip:?}, which is outside the chip {:?}",
                at.tally
            );
        }
        // And the clip is doing something on every one of these frames, so a
        // clip that was never exercised could not pass this by accident.
        let outside = words
            .iter()
            .filter(|(_, bounds, _)| !at.tally.contains_rect(*bounds))
            .count();
        assert!(
            outside > 0,
            "no word at {millis} ms was outside the chip, so the clip is not being asked anything"
        );
    }
}

// ---------------------------------------------------------------------------
// 5. The band between the two words
// ---------------------------------------------------------------------------

/// **The two words never touch, at any displacement.**
///
/// The pitch is the travel's rather than the geometry's, and it is at least
/// the box height: at the natural 10.0 row pitch the two words are both partly
/// visible with nothing between them, and at 9px that is mud rather than two
/// words. One box height apart leaves a blank band of exactly the slack the
/// chip already has — [`size::TALLY_H`]'s 13.5 less a 10.0 ink row is 3.5 —
/// and it is the same band at every displacement, because it is the difference
/// of two constants rather than a function of how far the roll has got.
///
/// **It costs the chip nothing**, which is the other half of why it is a
/// property of the travel: the second word is a whole box below the first at
/// rest, which is outside the capsule, which is clipped away.
#[test]
fn the_two_words_never_meet() {
    for millis in (0..400).step_by(5) {
        let phase = Phase::since(Duration::from_millis(millis));
        let (_, words) = words_at(vec![parked()], phase);
        let (_, now, _) = &words[0];
        let (_, to, _) = &words[1];
        let band = to.min.y - now.max.y;
        assert!(
            band > 0.0,
            "the two words are {band} apart at {millis} ms, and nothing between them is mud"
        );
        assert!(
            near(band, size::TALLY_H - now.height()),
            "the band is {band} at {millis} ms and the chip's own slack is {}",
            size::TALLY_H - now.height()
        );
    }
    // The travel is a quarter of the way through the period at 400 ms and the
    // words rest for the rest of it, so the loop above is every frame on which
    // anything moves.
    assert_eq!(ROLL_TRAVEL, Duration::from_millis(400));
}

// ---------------------------------------------------------------------------
// 6. What the panel asks for
// ---------------------------------------------------------------------------

/// **A parked slot ends the still panel, and nothing else does.**
///
/// The declaration is the view's — it is what knows the rate — and
/// `Change::Animating` is where it becomes a deadline. A **deadline** and not
/// a frame: `Repaint::After` names *draw then, and not before*, and a
/// once-a-second animation that asked for a frame instead would become a spin
/// at whatever rate the loop can manage.
///
/// The other direction is the one worth having a test for at all. A panel with
/// no parked slot must cost exactly what it cost before any of this existed —
/// `Repaint::Never`, and a window that sleeps — and an animation is the most
/// likely thing to take that away by accident.
#[test]
fn the_panel_asks_for_a_deadline_only_while_something_is_parked() {
    let panel = arrangement();
    let mut view = View::new(Room::Day);

    // No deck at all: the bay draws nothing and nothing moves.
    assert_eq!(view.animating(panel.layout()), None);
    assert_eq!(
        Change::Animating(view.animating(panel.layout())).repaint(),
        Repaint::Never
    );

    // Four settled strips.
    view.mixer = Tally::ALL
        .into_iter()
        .chain(std::iter::once(Tally::Live))
        .map(settled)
        .collect();
    assert_eq!(
        view.animating(panel.layout()),
        None,
        "a deck with nothing parked declared a staleness"
    );
    assert_eq!(
        Change::Animating(view.animating(panel.layout())).repaint(),
        Repaint::Never,
        "a still panel asked for a frame"
    );

    // One of them parked, and the panel is live for as long as it is.
    view.mixer[2] = parked();
    assert_eq!(
        view.animating(panel.layout()),
        Some(ROLL_STALENESS),
        "a parked slot did not declare the roll's staleness"
    );
    assert_eq!(
        Change::Animating(view.animating(panel.layout())).repaint(),
        Repaint::After(ROLL_STALENESS),
        "a parked slot did not ask for a deadline"
    );

    // Two parked is one panel-wide phase and one deadline, not two.
    view.mixer[0] = parked();
    assert_eq!(view.animating(panel.layout()), Some(ROLL_STALENESS));

    // And the request landing puts the panel back to sleep.
    view.mixer[0] = settled(Tally::Priming);
    view.mixer[2] = settled(Tally::Priming);
    assert_eq!(
        view.animating(panel.layout()),
        None,
        "the request landed and the panel is still asking for frames"
    );
}

/// **A parked slot in a bay that is folded away declares nothing**, and
/// unfolding it declares again.
///
/// The strips are rewritten every frame from the `Deck`, so *is anything
/// pending* is a fact about the deck and not about the panel. Answering off
/// that alone bought a 30 Hz deadline forever for a chip nobody could see:
/// measured with the picture and the preview row folded, the window sat at
/// 28.7 to 29.0 frames a second, and folding the mixer bay on top of that
/// moved the price of a frame — 432 allocations to 260 — and not the rate at
/// all. That is P-0072 read backwards: the rule is *what **must be live**
/// declares a cost and a staleness*, and a bay the operator has folded away is
/// not live.
///
/// **Both folds, because they are one question with two ways in.** `f` over
/// the bay folds the mixer itself; `g` over it folds the split that encloses
/// it, and the whole right pane goes with it. `Layout::visible` walks the
/// ancestors and answers both — asking `is_collapsed` on the bay alone would
/// pass the first of these and fail the second, silently, at the moment an
/// operator folded a pane rather than a bay.
///
/// **The fold is not a latch**, which is the half that a `bool` written once
/// would get wrong: the declaration is re-derived every frame from the
/// arrangement as it now is, so putting the bay back puts the deadline back
/// while the slot is still parked.
#[test]
fn a_folded_mixer_bay_declares_nothing_and_unfolding_declares_again() {
    for enclosing in [false, true] {
        let mut panel = arrangement();
        let bay = node(&panel, "mixer");
        let folds = match enclosing {
            true => node(&panel, "right-pane"),
            false => bay,
        };

        let mut view = View::new(Room::Day);
        view.mixer = vec![settled(Tally::Live), parked()];

        // Drawn, and the chip is rolling on screen: the panel declares.
        assert_eq!(
            view.animating(panel.layout()),
            Some(ROLL_STALENESS),
            "a parked slot in a drawn mixer bay declared nothing"
        );

        // Folded away, with the same slot still parked behind it.
        panel.op(Op::Fold(folds));
        assert!(
            !panel.layout().visible(bay),
            "folding {} left the mixer bay laid out",
            match enclosing {
                true => "the right pane",
                false => "the mixer bay",
            }
        );
        assert_eq!(
            view.animating(panel.layout()),
            None,
            "a parked slot declared a staleness with its bay folded away (enclosing: {enclosing})"
        );
        assert_eq!(
            Change::Animating(view.animating(panel.layout())).repaint(),
            Repaint::Never,
            "a folded bay's parked slot still asked the window for a frame"
        );

        // And back: the fold is not a latch.
        panel.op(Op::Unfold(folds));
        assert_eq!(
            view.animating(panel.layout()),
            Some(ROLL_STALENESS),
            "unfolding the bay left the roll declared dead while the slot is still parked"
        );
        assert_eq!(
            Change::Animating(view.animating(panel.layout())).repaint(),
            Repaint::After(ROLL_STALENESS),
            "unfolding the bay did not get the deadline back"
        );
    }
}
