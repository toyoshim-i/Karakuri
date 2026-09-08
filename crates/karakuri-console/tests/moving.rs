//! **A region asks for frames while it is moving, and not while it is merely
//! pending.**
//!
//! [ADR-0164](../../../docs/adr/0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md)
//! gives a live region two numbers — what its update costs and how stale it
//! may get — and neither of them says whether the region has changed. So the
//! window served the mixer bay's declared thirty a second for as long as
//! anything in it was outstanding, and
//! [`roll_at`](karakuri_console::view::roll_at) is **exactly zero** for the
//! 600 ms of every [`ROLL_PERIOD`] that is not [`ROLL_TRAVEL`]: seventeen of
//! those thirty-one frames redrew the panel exactly as it already was.
//!
//! [ADR-0283](../../../docs/adr/0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)
//! is the third number, `Declared::moves_in`, and this file is what it bought
//! and what it may not take away:
//!
//! 1. **The count.** What a parked panel asks for over one period, before and
//!    after, counted rather than described.
//! 2. **What was dropped.** Every frame that is no longer asked for would have
//!    drawn the chip in the position it was already in — asserted off the
//!    curve, so it is a claim about the picture and not about the schedule.
//! 3. **What may not be dropped.** The beat keeps its rate at every phase of
//!    the roll, which is
//!    [P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md):
//!    a panel that stopped moving because nothing had *changed* is exactly the
//!    console that has gone quiet.
//!
//! Nothing here needs a window, a device or a clock. The phase is a value the
//! test chooses, which is what ADR-0190 made it for.

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

/// A strip where it was asked to be, with nothing armed on either fader.
///
/// **With a reading in its meter**, which is what every strip in
/// `crates/karakuri/src/main.rs` has — `Deck::enable_meters` is called for the
/// whole deck at startup — so the counts below are a *metered* panel's counts.
/// The meter moves on the frames this panel is drawn on and on no others, so
/// it is in neither of the two numbers a deadline here is made of
/// ([ADR-0290](../../../docs/adr/0290-the-level-meter-moves-only-when-a-frame-is-drawn-so-it-declares-nothing.md),
/// `tests/metered.rs`).
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

/// **The parked strip**, which is the one state the engine can actually
/// produce: asked to prime, held at allocated because the budget found no
/// room.
fn parked() -> Strip {
    Strip {
        tally: Tally::Allocated,
        requested: Tally::Priming,
        ..settled()
    }
}

/// **The mock's own transport**, which is what makes a console live: without
/// it there is no beat grid, and the beat is the one thing on this panel that
/// never rests.
fn running() -> Transport {
    Transport {
        bpm: 128.0,
        beats: 144.0,
        beats_per_bar: 4,
        fps: Some(58.0),
        frame_ms: 12.4,
        budget_ms: Some(16.6),
        health: Some(karakuri_console::view::Stage::Landed),
        // **Nobody has said whether a recording is running**, so no `rec`
        // pill is drawn — the console's own answer for a program that never
        // told it, and what every test in this crate that does not say
        // otherwise draws.
        rec: None,
    }
}

/// **The window loop, run on a phase the test chooses**: the frames a panel
/// would be drawn on over `over`, starting with one at the origin.
///
/// It is `crates/karakuri/src/main.rs`'s loop and nothing else — ask the view
/// what it wants, sleep exactly that long, draw, ask again — with the clock
/// replaced by arithmetic. That is the whole reason
/// [`View::animating`](karakuri_console::view::View::animating) hands back a
/// number instead of touching one: a schedule is countable without a window.
///
/// A view that answers `None` has stopped asking, and the walk ends there.
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

/// **A parked slot asks for the roll's rate through the travel and for the
/// rest of the rest through the rest** — fourteen frames a second where it
/// asked for thirty-one.
///
/// The panel here is the still one: a deck with a slot the governor has
/// refused, the mixer bay on screen, and **no transport row**, because the
/// beat is a second and sooner declaration and would decide every deadline on
/// its own. That is the state ADR-0193 measured the old defect in — the
/// picture and the preview row folded away, nothing making texels, and the
/// window woken thirty times a second to redraw a chip nobody could see
/// moving. This is the same window with the chip on screen and the chip not
/// moving.
///
/// **Fourteen and not twelve**, and the two extra are worth naming rather than
/// rounding away. `ROLL_STALENESS` is 33.333 ms and the travel is 400, so
/// thirteen steps from the origin land at 399.996 ms — inside the travel by
/// four microseconds — and the fourteenth is the first frame of the rest,
/// which is the frame that discovers there is one. The rest itself is then a
/// single sleep of 566.671 ms, and it is the whole of what this record buys.
///
/// **Run against its defect**: `View::animating` reading `staleness` instead
/// of `moves_in` — which is what it did before ADR-0283 — fails with *"a
/// parked panel drew 31 frames in a second and 17 of them were drawn while the
/// roll was at rest"*.
#[test]
fn a_parked_panel_asks_for_frames_only_while_the_word_is_travelling() {
    let panel = arrangement();
    let mut view = View::new(Room::Day);
    view.mixer = vec![settled(), parked()];

    let drawn = frames_over(&mut view, &panel, ROLL_PERIOD);
    // **A frame drawn while the roll is at rest**, which is the region's own
    // answer and not a threshold this file chose: `roll_moves_in` is the
    // declared rate through the travel and something longer through the rest.
    // It is asked rather than `roll_at`, because the curve passes through zero
    // at both ends of the travel as well and those two frames are motion.
    let resting: Vec<&Duration> = drawn
        .iter()
        .filter(|at| roll_moves_in(Phase::since(**at)) > ROLL_STALENESS)
        .collect();
    // **What the declared rate alone asks for over the same second**, which is
    // what this window did before ADR-0283 and is the number being beaten.
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

    // **One of the fourteen is drawn at rest**, and it is the frame that
    // discovers the rest rather than a frame spent in it: the thirteen before
    // it are the travel, and the next one is a whole period later.
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

/// **Every frame that is no longer asked for would have drawn the chip exactly
/// where it already was.**
///
/// This is the claim the whole record rests on, and it is a claim about the
/// *picture* rather than about the schedule — so it is asserted off
/// [`roll_at`], which is what the tally and both faders are drawn from, and
/// not off the deadline. The curve is flat at zero for the whole of the rest,
/// so the displacement at the moment the rest begins and the displacement at
/// every 33.333 ms step the old schedule would have taken through it are the
/// same number.
///
/// **Run against its defect**: `roll_moves_in` reduced to `ROLL_STALENESS` at
/// every phase — a region that goes on declaring a rate it is not using —
/// fails with *"the rest began at 400ms and the region asked to be woken in
/// 33.333ms, which is inside its own rest"*.
///
/// **A drawn frame that changes nothing is not free and it is not harmless.**
/// ADR-0164 measured a panel pass at 184 allocations and 226.2 kB; the panel
/// as it now stands reads 525 and 694.3 kB. Seventeen of those a second, for
/// as long as a slot is parked, is the cost that was being paid to redraw a
/// still chip.
///
/// **Eighteen steps and seventeen frames, and the difference is one frame that
/// is still drawn.** Eighteen steps of the declared staleness fall inside the
/// rest; the schedule above keeps the first of them, because that is the frame
/// on which the panel discovers there is a rest to sleep through.
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

/// **The beat keeps its rate at every phase of the roll**, which is P-0094 and
/// is the thing this record is most able to break.
///
/// The failure it guards against is precise: the beat's declaration is written
/// as *the row is drawn and there is an engine behind it*, which is the shape
/// of an **I am drawn** claim rather than an **I move at this rate** one, so it
/// is the declaration a reader would reach for first when looking for
/// something else to gate. It is honest because the light travels on every
/// frame the session advances — but nothing in this crate advances a session,
/// so the honesty is the harness's and the rule has to be held here.
///
/// A console that stopped moving because nothing had *changed* is a console
/// that has gone quiet, and P-0094's forced clause is that something is moving
/// continuously while it is live. So the beat is asked at a hundred phases
/// spread across the roll's period — including every phase at which the mixer
/// bay has just been given permission to sleep — and it answers the same
/// number at all of them.
///
/// **Run against its defect**: `transport_declares` handing back
/// `roll_moves_in(self.phase)` instead of its own rate — the beat gated the
/// way the mixer now is — fails with *"a live console asked for
/// Some(33.333ms) at 0 ms into the roll, and the beat declares 24.671ms
/// whatever else the panel is doing"*.
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

    // **And it is the beat that is holding it there**: folding the transport
    // row away leaves the mixer's answer alone, which is where the rest shows
    // up again.
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
