//! **P-0072's two schedulability conditions, asserted over what this console
//! declares.**
//!
//! [P-0072](../../../docs/principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md)
//! states them and states what is done with them in the same breath:
//!
//! ```text
//! Σ (cost / staleness)  ≤  budget / frame interval
//! max(cost)             ≤  a small part of the budget
//! ```
//!
//! > A panel is schedulable only if both hold. **They are sums over the named
//! > regions, so a test asserts them rather than a stage discovering them.**
//!
//! This is that test. It is the whole of the arithmetic in this repository:
//! nothing in `src/` takes either sum, because a panel that discovered it was
//! over budget at runtime would be discovering it during a performance.
//!
//! **The first says there is enough capacity on average.** The second is the
//! one P-0072 says gets forgotten: an update is not divisible, so a region
//! costing most of the budget is a traffic jam of one — every frame it runs,
//! nothing else can, and the readouts that had to be live miss their
//! deadlines.
//!
//! # What is summed, and it is what declares rather than what is named
//!
//! [`View::declares`] answers with the regions that are declaring *on this
//! frame*, which is where
//! [ADR-0193](../../../docs/adr/0193-a-region-that-is-not-laid-out-declares-nothing-rather-than-being-dropped-later.md)
//! lands in the arithmetic: a region the operator has folded away is in
//! neither sum, because it is not showing anything and so cannot be showing
//! anything out of date. The worst case this file asserts against is therefore
//! *everything pending, with every region laid out* — the most a console with
//! this arrangement can ever declare.
//!
//! # One region, and that is counted rather than assumed
//!
//! The mixer bay is the only live region on this panel. `docs/roadmap.md` has
//! read *four* — the picture, the beat grid, the mixer's readouts and whatever
//! is pending — and three of those four do not declare anything and two of
//! them are not this principle's business at all: the picture is the engine's
//! output and *"counting it here would count it twice"*, the beat grid moves
//! because the panel is being redrawn for something else rather than because
//! anything decided it must (which is exactly what
//! [P-0077](../../../docs/principles/0077-continuous-motion-is-how-a-stopped-panel-announces-itself.md)
//! is still waiting for), and the mixer's readouts change when a hand changes
//! them, which P-0072 does not budget. What is left is one region with three
//! presentations in it. `a_declaration_names_a_region_of_this_arrangement`
//! holds the count where the next person will see it move.
//!
//! Nothing here needs a window, a device or a clock.

mod common;

use std::time::Duration;

use common::PLAUSIBLE;
use karakuri_console::budget::{Declared, BUDGET, FRAME_INTERVAL, PANEL_PASS, SMALL_PART};
use karakuri_console::panel::{Op, Panel};
use karakuri_console::room::Room;
use karakuri_console::view::{Level, Mask, Strip, Tally, View, DECKS, REGIONS, ROLL_STALENESS};
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

/// A strip with nothing outstanding on it: where it was asked to be, and no
/// transition armed on either fader.
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
    }
}

/// **A strip with everything the panel can have outstanding on it at once**: a
/// residency request the governor has not granted, and a fade armed on each of
/// the two faders.
///
/// Three presentations, and P-0075 has them move together off one phase — so
/// this is what makes the point that they are one declaration and not three.
fn pending() -> Strip {
    Strip {
        tally: Tally::Allocated,
        requested: Tally::Priming,
        gain_to: Some(0.9),
        opacity_to: Some(0.8),
        ..settled()
    }
}

/// **The most this console can ever declare**: every strip a deck can hold,
/// each of them pending in all three ways, with the whole arrangement laid
/// out.
///
/// The worst case rather than a plausible one, because a schedulability
/// condition that only held for the panel somebody happened to be looking at
/// would be a condition about that panel.
fn worst_case() -> View {
    let mut view = View::new(Room::Day);
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

/// **`Σ (cost / staleness)`** — what the live regions ask for on average, as a
/// fraction of a frame.
///
/// Dimensionless: a cost is a time and a staleness is a time, and the ratio is
/// *how much of every millisecond this region needs*. In `f64` because a term
/// is a small ratio of two small durations and the sum is compared against
/// another one.
fn load(declared: &[Declared]) -> f64 {
    declared
        .iter()
        .map(|live| live.cost.as_secs_f64() / live.staleness.as_secs_f64())
        .sum()
}

/// **`max(cost)`** — the largest single update, which is the one that cannot
/// be divided.
///
/// Zero for a panel that declares nothing, which is the right answer and not a
/// missing one: nothing is going to run, so nothing is going to block.
fn peak(declared: &[Declared]) -> Duration {
    declared
        .iter()
        .map(|live| live.cost)
        .max()
        .unwrap_or(Duration::ZERO)
}

/// **`budget / frame interval`** — the fraction of each frame the panel may
/// have.
fn capacity() -> f64 {
    BUDGET.as_secs_f64() / FRAME_INTERVAL.as_secs_f64()
}

/// **`a small part of the budget`**, which is [`SMALL_PART`] of it.
fn small_part() -> Duration {
    BUDGET.mul_f32(SMALL_PART)
}

/// **Both conditions hold for this console, at the most it can ever declare.**
///
/// This is the assertion P-0072 asks for, and the numbers it comes out at are
/// in the messages so that a failure says how far out it is rather than that
/// it is out.
///
/// **Raising a declared cost past what the budget allows fails it**, which is
/// the defect this was run against: `PANEL_PASS` at 5 ms breaks the second
/// condition alone, and at 40 ms it breaks both — the second is the binding
/// one on this console by a factor of eight, which is P-0072's own point about
/// which of the two gets forgotten.
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

/// **A region that is not laid out is in neither sum**, and putting it back
/// puts it back into both.
///
/// [ADR-0193](../../../docs/adr/0193-a-region-that-is-not-laid-out-declares-nothing-rather-than-being-dropped-later.md)
/// decided this at the declaration rather than downstream of it, and the
/// arithmetic is why the record gives as its second reason: *"Discarding it is
/// not arbitration."* The two sums are about **capacity** — which of several
/// true deadlines fit in a frame — and a term dropped because the thing is
/// invisible is not that computation.
///
/// **Both ways in**, because they are one question: `f` over the bay folds the
/// mixer itself, `g` over it folds the split that encloses it and the whole
/// right pane goes with it. Asking `is_collapsed` on the bay alone passes the
/// first and fails the second, silently — and that is one of the defects this
/// was run against, along with dropping the visibility term altogether.
///
/// **The fold is not a latch**: the declaration is re-derived from the
/// arrangement as it now is, so unfolding declares again while the same
/// request is still outstanding.
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
            1,
            "the mixer bay is drawn and everything in it is pending, and it declared {drawn:?}"
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
            folded.is_empty(),
            "a folded region declared {folded:?} (enclosing: {enclosing})"
        );
        assert_eq!(
            load(&folded),
            0.0,
            "a folded region contributed to Σ (cost / staleness) (enclosing: {enclosing})"
        );
        assert_eq!(
            peak(&folded),
            Duration::ZERO,
            "a folded region contributed to max(cost) (enclosing: {enclosing})"
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

/// **A still panel declares nothing, and both sums are zero.**
///
/// P-0072's first clause in the arithmetic: a panel with nothing changing on
/// it is paid for once and not again, so there is nothing to schedule and
/// nothing to be over budget with. It is the direction worth having a test
/// for — an animation is the most likely thing to take it away by accident.
#[test]
fn a_still_panel_is_zero_in_both_sums() {
    let panel = arrangement();
    let mut view = View::new(Room::Day);

    // No deck at all.
    assert!(
        declared(&view, &panel).is_empty(),
        "an empty panel declared"
    );

    // Four strips, every one of them where it was asked to be and with nothing
    // armed on either fader.
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
// What a declaration is allowed to say
// ---------------------------------------------------------------------------

/// **Every declared cost is one whole panel pass**, which is the half of
/// *keeping a declared cost honest* that can be asserted with no window, no
/// device and no clock.
///
/// `egui` is immediate mode: there is no retained tree, so *redraw the mixer
/// bay* is not an operation this console has, and what repaints is the panel
/// and not the chip (ADR-0188, ADR-0190). A region declaring a smaller,
/// per-region figure would be describing work this console cannot do — and it
/// is the kind of number that gets written because it looks more precise. The
/// day a region can be drawn once into a texture and composited after, this
/// test is what has to be changed on purpose.
///
/// The other half of honest is a measurement, it needs a window and three
/// seconds of nobody touching it, and it is in `examples/panel.rs`, which
/// holds `PANEL_PASS` against the run it has just taken.
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

/// **A declaration names a region of this arrangement**, and there is exactly
/// one of them.
///
/// P-0072's unit of deferral is a region and never a slice of time, so a
/// declaration has to be answerable by the layout — which is what makes
/// ADR-0193's *is this laid out* a question with an answer, and what a
/// scheduler would need to know which rectangle it was spending on.
///
/// The count is asserted so that the second live region is a line somebody
/// changes on purpose: it is the moment `max(cost)` stops being one number,
/// the moment `Σ` stops being one term, and the moment P-0072's deterministic
/// tie-break has anything to break.
#[test]
fn a_declaration_names_a_region_of_this_arrangement() {
    let panel = arrangement();
    let view = worst_case();
    let declared = declared(&view, &panel);

    assert_eq!(
        declared.len(),
        1,
        "the console declares {declared:?}; it had one live region when this was written, \
         and a second one wants both sums re-read"
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

    // And the one there is, is the mixer bay at the roll's rate — the tally's
    // and both faders', which share it.
    assert_eq!(
        declared[0],
        Declared {
            region: "mixer",
            cost: PANEL_PASS,
            staleness: ROLL_STALENESS,
        }
    );
}
