//! Parked tally roll animation when requested and effective residencies diverge (ADR-0190, ADR-0164, ADR-0189, P-0091).
//! Validates capsule sizing, phase-driven displacement, clipping bounds, and repaint deadlines.

mod common;

use std::time::Duration;

use common::{drawn_once, near, PLAUSIBLE};
use karakuri_console::panel::{Op, Panel};
use karakuri_console::repaint::{Change, Repaint};
use karakuri_console::room::{size, Room};
use karakuri_console::view::{
    mixer, roll_at, tally_into, Level, Mask, Phase, Strip, StripBox, Tally, View, ROLL_PERIOD,
    ROLL_REACH, ROLL_STALENESS, ROLL_TRAVEL,
};
use karakuri_layout::NodeId;
use karakuri_operation::BlendMode;

/// A strip that is where it was asked to be — the ordinary case, and what every
/// other test file in this crate is made of.
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
        is_muted: false,
        is_soloed: false,
    }
}

/// The parked strip, which is the one state the engine can actually produce:
/// `Deck::is_parked` is `requested == Priming && effective == Allocated` —
/// asked to prime, and held at allocated because the budget has not found room.
fn parked() -> Strip {
    Strip {
        tally: Tally::Allocated,
        requested: Tally::Priming,
        ..settled(Tally::Allocated)
    }
}

/// The console's arrangement at a plausible window, solved — every region laid
/// out, which is the state `View::animating` is asked in everywhere below
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

/// Collects galleys painted in the first strip's tally along with their active clipping rectangles.
fn words_at(strips: Vec<Strip>, phase: Phase) -> (StripBox, Vec<(String, egui::Rect, egui::Rect)>) {
    let at = box_of(&strips);
    let pal = Room::Day.palette();
    let ctx = drawn_once();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        let painter = ui.painter();
        for strip in &strips {
            tally_into(painter, &pal, at.tally, strip.tally, strip.pending(), phase);
        }
    });
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

/// Sizing the tally capsule to the widest residency word prevents geometry changes during rolls.
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

/// When residency states match, the strip renders a single word and declares no repaint (ADR-0164).
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

/// Displacement is verified against explicit phases (ADR-0189) using the raised-cosine roll curve.
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

/// Verifies the rolling word is clipped to the capsule rectangle to prevent bleed into adjacent controls.
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

/// Ensures the active and target words maintain visual separation throughout the entire roll travel.
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

/// A parked slot schedules `Repaint::After` deadlines; an idle panel stays at `Repaint::Never`.
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

/// Verifies that a parked slot in a folded mixer bay declares no animation frames,
/// and re-declares them upon unfolding (P-0091).
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
