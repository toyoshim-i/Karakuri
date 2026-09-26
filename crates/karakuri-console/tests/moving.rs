//! Animation deadline declarations: regions request repaints only during active motion (ADR-0164, ADR-0283, P-0094).

mod common;

use std::time::Duration;

use common::PLAUSIBLE;
use karakuri_console::panel::{Op, Panel};
use karakuri_console::repaint::{Change, Repaint};
use karakuri_console::room::Room;
use karakuri_console::view::{
    roll_at, roll_moves_in, Level, Mask, Phase, Strip, Tally, Transport, View, BEAT_STALENESS,
    ROLL_PERIOD, ROLL_STALENESS, ROLL_TRAVEL,
};
use karakuri_operation::BlendMode;

/// The console's arrangement at a plausible window, solved — every region laid
/// out, so that nothing here is answering ADR-0193's question by accident.
fn arrangement() -> Panel {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    panel
}

/// Helper creating a settled strip with active meters (ADR-0290).
fn settled() -> Strip {
    Strip {
        name: "glass_shell".to_owned(),
        tally: Tally::Live,
        requested: Tally::Live,
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
/// asked to prime, held at allocated because the budget found no room.
fn parked() -> Strip {
    Strip {
        tally: Tally::Allocated,
        requested: Tally::Priming,
        ..settled()
    }
}

/// The mock's own transport, which is what makes a console live: without it
/// there is no beat grid, and the beat is the one thing on this panel that
/// never rests.
fn running() -> Transport {
    Transport {
        bpm: 128.0,
        beats: 144.0,
        beats_per_bar: 4,
        fps: Some(58.0),
        frame_ms: 12.4,
        budget_ms: Some(16.6),
        chain_ms: None,
        health: Some(karakuri_console::view::Stage::Landed),
        // Unset recording state omits the rec pill.
        rec: None,
    }
}

/// Simulates window event loop frame pacing across a specified time window.
fn frames_over(view: &mut View, panel: &Panel, over: Duration) -> Vec<Duration> {
    let mut drawn = vec![Duration::ZERO];
    let mut at = Duration::ZERO;
    loop {
        view.phase = Phase::since(at);
        let Some(wait) = view.animating(panel.layout()) else {
            break;
        };
        at += wait;
        if at >= over {
            break;
        }
        drawn.push(at);
    }
    drawn
}

// ---------------------------------------------------------------------------
// 1. What a parked panel asks for
// ---------------------------------------------------------------------------

/// Parked roll requests frames only during active travel (400ms), pausing during rest (ADR-0283).
#[test]
fn a_parked_panel_asks_for_frames_only_while_the_word_is_travelling() {
    let panel = arrangement();
    let mut view = View::new(Room::Day);
    view.mixer = vec![settled(), parked()];

    let drawn = frames_over(&mut view, &panel, ROLL_PERIOD);
    // During rest phases, `roll_moves_in` indicates remaining sleep time rather than roll staleness.
    let resting: Vec<&Duration> = drawn
        .iter()
        .filter(|at| roll_moves_in(Phase::since(**at)) > ROLL_STALENESS)
        .collect();
    // Baseline frame count based purely on declared staleness rate across one second (ADR-0283).
    let at_the_declared_rate = (0..)
        .map(|step| ROLL_STALENESS * step)
        .take_while(|at| *at < ROLL_PERIOD)
        .count();

    assert_eq!(
        at_the_declared_rate, 31,
        "a second holds {at_the_declared_rate} frames at the roll's declared staleness, \
         and the arithmetic in this file is written for 31"
    );
    assert_eq!(
        drawn.len(),
        14,
        "a parked panel drew {} frames in a second and {} of them were drawn while the \
         roll was at rest",
        drawn.len(),
        resting.len()
    );

    // Exactly one frame is rendered at rest to discover the resting state following travel.
    assert_eq!(
        resting.len(),
        1,
        "{} of the fourteen frames were drawn while the roll was at rest",
        resting.len()
    );

    // The sleep that replaced seventeen frames, stated as itself.
    let rest_began = *resting[0];
    assert!(
        ROLL_PERIOD - rest_began > Duration::from_millis(500),
        "the rest was serviced in pieces: it began at {rest_began:?} and the panel was \
         woken again before the period was out"
    );

    // And it is the *panel's* answer that is longer, not this file's: the arm
    // that turns it into a deadline says the same thing.
    view.phase = Phase::since(rest_began);
    assert_eq!(
        Change::Animating(view.animating(panel.layout())).repaint(),
        Repaint::After(roll_moves_in(Phase::since(rest_began))),
        "the deadline the window waits on is not the one the region asked for"
    );
}

// ---------------------------------------------------------------------------
// 2. What the frames that stopped being drawn would have drawn
// ---------------------------------------------------------------------------

/// Verifies that frame rate reductions during roll rest do not miss visual state updates.
#[test]
fn the_frames_the_rest_no_longer_asks_for_would_have_drawn_the_same_chip() {
    let began = ROLL_TRAVEL;
    let was = roll_at(Phase::since(began));
    assert_eq!(
        was, 0.0,
        "the roll is at {was} where its travel ends, so the rest is not a rest"
    );

    // Every step the old schedule would have taken through the rest.
    let mut at = began;
    let mut skipped = 0;
    while at + ROLL_STALENESS < ROLL_PERIOD {
        at += ROLL_STALENESS;
        skipped += 1;
        let now = roll_at(Phase::since(at));
        assert_eq!(
            now, was,
            "the roll is at {now} at {at:?} and was at {was} when the rest began, so a \
             frame drawn at the old rate would have drawn something new"
        );
    }
    assert_eq!(
        skipped, 18,
        "the rest holds {skipped} steps of the declared staleness, and this file's \
         arithmetic about what was bought is written for 18 of them"
    );

    // And the region says so itself: at the foot of the rest it asks to be
    // left alone until the period wraps, and nothing sooner.
    let asked = roll_moves_in(Phase::since(began));
    assert!(
        began + asked >= ROLL_PERIOD,
        "the rest began at {began:?} and the region asked to be woken in {asked:?}, which \
         is inside its own rest"
    );
    assert!(
        asked >= ROLL_STALENESS,
        "a region asked for {asked:?}, sooner than the {ROLL_STALENESS:?} it declared"
    );

    // The travel keeps every one of its twelve steps: a step is what
    // `ROLL_STALENESS` is defined as, and this is the half of the declaration
    // that is honest and is not touched.
    for step in 0..12 {
        let during = Phase::since(ROLL_STALENESS * step);
        assert_eq!(
            roll_moves_in(during),
            ROLL_STALENESS,
            "the roll asked for something other than its own rate at step {step} of its \
             travel"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. What may not be dropped
// ---------------------------------------------------------------------------

/// Beat light declaration maintains continuous animation rate independent of roll phase (P-0094).
#[test]
fn the_beat_keeps_its_rate_through_the_rolls_rest() {
    let panel = arrangement();
    let mut view = View::new(Room::Day);
    view.transport = Some(running());
    view.mixer = vec![settled(), parked()];

    for millis in (0..ROLL_PERIOD.as_millis() as u64).step_by(10) {
        view.phase = Phase::since(Duration::from_millis(millis));
        assert_eq!(
            view.animating(panel.layout()),
            Some(BEAT_STALENESS),
            "a live console asked for {:?} at {millis} ms into the roll, and the beat \
             declares {BEAT_STALENESS:?} whatever else the panel is doing",
            view.animating(panel.layout())
        );
    }

    // Folding the transport row reveals the mixer's resting animation schedule.
    let mut panel = panel;
    let transport = panel
        .layout()
        .find("transport")
        .expect("the arrangement names `transport`");
    panel.op(Op::Fold(transport));
    view.phase = Phase::since(ROLL_TRAVEL);
    let asked = view.animating(panel.layout());
    assert!(
        asked.is_some_and(|asked| asked > ROLL_STALENESS),
        "with the beat folded away the panel asked for {asked:?}, so the roll's rest is \
         being serviced at the travel's rate after all"
    );
}
