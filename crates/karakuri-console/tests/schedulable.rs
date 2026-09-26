//! Schedulability verification for rendering budget conditions (ADR-0164, ADR-0193, ADR-0210, ADR-0212, P-0094).
//! Validates `Σ (cost / staleness) ≤ budget / interval` and `max(cost) ≤ max_budget` under maximum load.

mod common;

use std::time::Duration;

use common::PLAUSIBLE;
use karakuri_console::budget::{Declared, BUDGET, FRAME_INTERVAL, PANEL_PASS, SMALL_PART};
use karakuri_console::panel::{Op, Panel};
use karakuri_console::room::Room;
use karakuri_console::view::{
    Level, Mask, Phase, Strip, Tally, Transport, View, BEAT_STALENESS, DECKS, REGIONS, ROLL_PERIOD,
    ROLL_STALENESS,
};
use karakuri_layout::NodeId;
use karakuri_operation::BlendMode;

// ---------------------------------------------------------------------------
// The console, and the most it can ever declare
// ---------------------------------------------------------------------------

/// The console's arrangement at a plausible window, solved — every region laid
/// out, which is the state everything below is asked in except where a fold is
/// the thing under test.
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

/// Helper constructing a settled strip with no active transitions.
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

/// Creates a mixer strip with all pending animations armed under a single declaration (ADR-0190).
fn pending() -> Strip {
    Strip {
        tally: Tally::Allocated,
        requested: Tally::Priming,
        gain_to: Some(0.9),
        opacity_to: Some(0.8),
        ..settled()
    }
}

/// Configures mock transport values representing an active, live engine state.
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
        // Running recording state included to test worst-case shape budget.
        rec: Some(karakuri_console::view::Rec::Running),
    }
}

/// Configures the worst-case console load: active engine, fully pending strips, and all regions visible.
fn worst_case() -> View {
    let mut view = View::new(Room::Day);
    view.transport = Some(running());
    view.mixer = (0..DECKS).map(|_| pending()).collect();
    view
}

/// What [`View::declares`] answers, collected so it can be summed twice.
fn declared(view: &View, panel: &Panel) -> Vec<Declared> {
    view.declares(panel.layout()).collect()
}

// ---------------------------------------------------------------------------
// The two conditions
// ---------------------------------------------------------------------------

/// Computes `Σ (cost / staleness)` as the fractional frame budget demanded across active regions.
fn load(declared: &[Declared]) -> f64 {
    declared
        .iter()
        .map(|live| live.cost.as_secs_f64() / live.staleness.as_secs_f64())
        .sum()
}

/// Computes `max(cost)` representing the largest indivisible update cost across declared regions.
fn peak(declared: &[Declared]) -> Duration {
    declared
        .iter()
        .map(|live| live.cost)
        .max()
        .unwrap_or(Duration::ZERO)
}

/// `budget / frame interval` — the fraction of each frame the panel may have.
fn capacity() -> f64 {
    BUDGET.as_secs_f64() / FRAME_INTERVAL.as_secs_f64()
}

/// `a small part of the budget`, which is [`SMALL_PART`] of it.
fn small_part() -> Duration {
    BUDGET.mul_f32(SMALL_PART)
}

/// Verifies both ADR-0164 schedulability conditions hold under maximum declared console load.
#[test]
fn both_conditions_hold_at_the_most_this_console_declares() {
    let panel = arrangement();
    let view = worst_case();
    let declared = declared(&view, &panel);

    assert!(
        !declared.is_empty(),
        "a console with everything pending declared nothing, so neither \
         condition below is about anything"
    );

    let load = load(&declared);
    assert!(
        load <= capacity(),
        "Σ (cost / staleness) is {load:.4} against a budget / frame interval of {:.4}: \
         {} live region(s) declaring {:?} each at {:?}, and the panel cannot keep up on \
         average",
        capacity(),
        declared.len(),
        declared.first().map(|live| live.cost),
        declared.first().map(|live| live.staleness),
    );

    let peak = peak(&declared);
    assert!(
        peak <= small_part(),
        "max(cost) is {peak:?} against {:?}, which is {SMALL_PART} of a {BUDGET:?} budget: \
         an update is not divisible, so a region this expensive is a traffic jam of one — \
         every frame it runs, nothing else can",
        small_part(),
    );
}

/// Folded or invisible regions omit declarations and contribute nothing to budget sums (ADR-0193).
#[test]
fn a_folded_region_is_in_neither_sum_and_unfolding_puts_it_back() {
    for enclosing in [false, true] {
        let mut panel = arrangement();
        let bay = node(&panel, "mixer");
        let folds = match enclosing {
            true => node(&panel, "right-pane"),
            false => bay,
        };
        let view = worst_case();

        let drawn = declared(&view, &panel);
        assert_eq!(
            drawn.len(),
            2,
            "the mixer bay is drawn and everything in it is pending, and the console \
             declared {drawn:?}"
        );

        panel.op(Op::Fold(folds));
        assert!(
            !panel.layout().visible(bay),
            "folding {} left the mixer bay laid out",
            match enclosing {
                true => "the right pane",
                false => "the mixer bay",
            }
        );

        let folded = declared(&view, &panel);
        assert!(
            !folded.iter().any(|live| live.region == "mixer"),
            "a folded region declared {folded:?} (enclosing: {enclosing})"
        );
        // The transport row is untouched by either fold and goes on
        // declaring, so what the fold took out of the sums is exactly the
        // mixer's own term: 1.26 ms every 33.333 is 0.0378.
        assert_eq!(
            load(&declared(&view, &arrangement())) - load(&folded),
            PANEL_PASS.as_secs_f64() / ROLL_STALENESS.as_secs_f64(),
            "folding the mixer bay took something other than its own term out of \
             Σ (cost / staleness) (enclosing: {enclosing})"
        );

        panel.op(Op::Unfold(folds));
        assert_eq!(
            declared(&view, &panel),
            drawn,
            "unfolding the bay left the region out of both sums while the same request is \
             still outstanding (enclosing: {enclosing})"
        );
    }
}

/// An idle panel without an engine declares no regions, yielding zero for both sums (ADR-0164).
#[test]
fn a_still_panel_is_zero_in_both_sums() {
    let panel = arrangement();
    let mut view = View::new(Room::Day);

    // No deck at all.
    assert!(
        declared(&view, &panel).is_empty(),
        "an empty panel declared"
    );

    // Settled strips with no active transitions declare zero frame budget.
    view.mixer = (0..DECKS).map(|_| settled()).collect();
    let declared = declared(&view, &panel);
    assert!(
        declared.is_empty(),
        "a deck with nothing outstanding declared {declared:?}"
    );
    assert_eq!(load(&declared), 0.0);
    assert_eq!(peak(&declared), Duration::ZERO);
}

// ---------------------------------------------------------------------------
// The beat, which declares for a different reason from everything else here
// ---------------------------------------------------------------------------

/// Live engine transport beat grids declare unconditionally to signal liveness (P-0094, ADR-0212).
#[test]
fn the_beat_declares_while_the_console_is_live_and_nothing_is_pending() {
    let panel = arrangement();
    let mut view = View::new(Room::Day);
    view.transport = Some(running());

    let declared = declared(&view, &panel);
    assert_eq!(
        declared,
        vec![Declared {
            region: "transport",
            cost: PANEL_PASS,
            staleness: BEAT_STALENESS,
            // The beat's two numbers are one number: the light travels on
            // every frame the session advances, so it is never at rest and
            // never asks for anything but its own rate (ADR-0283).
            moves_in: BEAT_STALENESS,
        }],
        "a live console with nothing pending declared {declared:?}, so nothing on this \
         panel is moving and a stopped panel looks exactly like this one"
    );

    // Settled decks do not alter the transport's beat declarations.
    view.mixer = (0..DECKS).map(|_| settled()).collect();
    assert_eq!(
        declared,
        self::declared(&view, &panel),
        "a settled deck changed what the beat declares"
    );

    // The mixer's own declaration is the one that comes and goes with what is
    // outstanding, and it arrives beside this rather than instead of it.
    view.mixer = (0..DECKS).map(|_| pending()).collect();
    assert_eq!(
        self::declared(&view, &panel).len(),
        2,
        "a parked deck on a live console is two live regions"
    );
}

/// Folding the transport row stops its frame declarations (ADR-0193, ADR-0204).
#[test]
fn a_folded_transport_row_declares_nothing_and_unfolding_puts_it_back() {
    let mut panel = arrangement();
    let mut view = View::new(Room::Day);
    view.transport = Some(running());
    let row = node(&panel, "transport");

    let drawn = declared(&view, &panel);
    assert_eq!(drawn.len(), 1, "a live console declared {drawn:?}");

    panel.op(Op::Fold(row));
    assert!(
        !panel.layout().visible(row),
        "folding the transport row left it laid out"
    );

    let folded = declared(&view, &panel);
    assert!(
        folded.is_empty(),
        "a folded transport row declared {folded:?}, so the panel is asking for frames \
         to move a beat grid that is not on screen"
    );
    assert_eq!(load(&folded), 0.0);
    assert_eq!(peak(&folded), Duration::ZERO);

    panel.op(Op::Unfold(row));
    assert_eq!(
        declared(&view, &panel),
        drawn,
        "unfolding the transport row left the beat out of both sums while the console \
         is still live"
    );

    // Mixer region declarations remain independent when transport row is folded.
    view.mixer = (0..DECKS).map(|_| pending()).collect();
    panel.op(Op::Fold(row));
    let folded = declared(&view, &panel);
    assert_eq!(
        folded.iter().map(|live| live.region).collect::<Vec<_>>(),
        vec!["mixer"],
        "folding the transport row moved what the mixer bay declares"
    );
}

// ---------------------------------------------------------------------------
// What a declaration is allowed to say
// ---------------------------------------------------------------------------

/// Verifies that every declared cost is exactly one whole panel pass (ADR-0188, ADR-0190).
#[test]
fn every_declared_cost_is_one_whole_panel_pass() {
    let panel = arrangement();
    let view = worst_case();

    for live in declared(&view, &panel) {
        assert_eq!(
            live.cost, PANEL_PASS,
            "`{}` declared a cost of its own; under immediate mode there is one pass and \
             every region costs it",
            live.region
        );
    }
}

/// Verifies each declaration targets an identifiable layout region (ADR-0210).
#[test]
fn a_declaration_names_a_region_of_this_arrangement() {
    let panel = arrangement();
    let view = worst_case();
    let declared = declared(&view, &panel);

    assert_eq!(
        declared.len(),
        2,
        "the console declares {declared:?}; it had two live regions when this was written, \
         and a third one wants both sums re-read"
    );

    for live in &declared {
        assert!(
            panel.layout().find(live.region).is_some(),
            "`{}` is declared and the arrangement does not name it",
            live.region
        );
        assert!(
            REGIONS.iter().any(|region| region.name == live.region),
            "`{}` is declared and the panel draws no face for it",
            live.region
        );
    }

    // And the two there are, in `REGIONS`' order: the transport row at the
    // beat's rate, and the mixer bay at the roll's — the tally's and both
    // faders', which share it.
    assert_eq!(
        declared[0],
        Declared {
            region: "transport",
            cost: PANEL_PASS,
            staleness: BEAT_STALENESS,
            moves_in: BEAT_STALENESS,
        }
    );
    assert_eq!(
        declared[1],
        Declared {
            region: "mixer",
            cost: PANEL_PASS,
            staleness: ROLL_STALENESS,
            // `Phase::ZERO`, which is where every view in this file is: the
            // roll is at the foot of its travel, so it is moving and the two
            // numbers agree. `tests/moving.rs` is where they part company.
            moves_in: ROLL_STALENESS,
        }
    );
}

// ---------------------------------------------------------------------------
// The frame moves one of the three numbers, and only in one direction
// ---------------------------------------------------------------------------

/// Change detection may delay but never accelerates frames beyond declared staleness (ADR-0283).
#[test]
fn a_declaration_never_asks_for_a_frame_sooner_than_the_staleness_it_declared() {
    let panel = arrangement();
    let mut view = worst_case();

    let at_rest = declared(&view, &panel);
    let (load_at_rest, peak_at_rest) = (load(&at_rest), peak(&at_rest));

    // A whole period, one millisecond at a time: the travel, the rest, and the
    // wrap between them.
    for millis in 0..ROLL_PERIOD.as_millis() as u64 {
        view.phase = Phase::since(Duration::from_millis(millis));
        let declared = declared(&view, &panel);
        for live in &declared {
            assert!(
                live.moves_in >= live.staleness,
                "`{}` asked for a frame in {:?} at {millis} ms, against the {:?} it \
                 declared — a region may only ever wait longer than it declared",
                live.region,
                live.moves_in,
                live.staleness
            );
        }
        assert_eq!(
            (load(&declared), peak(&declared)),
            (load_at_rest, peak_at_rest),
            "the two sums moved at {millis} ms, so the schedulability arithmetic is a \
             function of when it was asked rather than of what this console declares"
        );
    }
}
