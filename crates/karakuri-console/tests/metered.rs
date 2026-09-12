//! The mixer's level meter moves on the frames it is drawn on, so it is not in
//! what the bay declares — and the still panel survives a metered console.
//!
//!
//! [ADR-0283](../../../docs/adr/0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)
//! closed by naming the meter as an under-declaration it did not reach: *"the
//! mixer bay's level meter moves every frame and declares nothing"*.
//! [ADR-0290](../../../docs/adr/0290-the-level-meter-moves-only-when-a-frame-is-drawn-so-it-declares-nothing.md)
//! is that item worked out, and the answer is that there is no number to write.
//! `Deck::level` moves inside `Deck::begin_frame` — the only caller of
//! `Meters::collect` — and a caller calls that once per composed frame, which
//! is once per frame this panel is drawn on. So the meter's picture is a
//! function of the frames drawn rather than of wall time, and there is no
//! moment between two frames at which what is on screen is not the newest
//! reading taken.
//!
//! This file is the three claims that decision rests on, in the order they
//! would fail:
//!
//! 1. A metered console with nothing pending is still. The bay is laid out,
//! four readings are moving in it, and it asks for no frame at any phase of the
//! roll. That is ADR-0164's still-panel clause holding on a console that meters
//! — which is every console this program ships, because
//! `crates/karakuri/src/main.rs` calls `Deck::enable_meters` for the whole deck
//! at startup. 2. The reading is not in the declaration. A metered bay and an
//! unmetered one declare the same three numbers at every millisecond of a
//! period, so ADR-0283's fourteen frames are a *metered* panel's fourteen and a
//! meter declaration would be putting thirty-one back. 3. The meter is drawn
//! from the reading and not from the clock, which is the premise under the
//! other two: the roll's curve reaches the tally and both faders and does not
//! reach the meter. A meter with ballistics — a held peak, a fall time — would
//! be a function of the phase exactly as the roll is, and would have to
//! declare; this is what fails on the day one grows.
//!
//! Nothing here needs a window, a device or a clock. The phase is a value the
//! test chooses, which is what ADR-0190 made it for.

mod common;

use std::time::Duration;

use common::{drawn_once, solved, PLAUSIBLE};
use karakuri_console::budget::Declared;
use karakuri_console::panel::Panel;
use karakuri_console::repaint::{Change, Repaint};
use karakuri_console::room::Room;
use karakuri_console::view::{
    mixer, roll_at, roll_moves_in, Level, Mask, Phase, Strip, Tally, View, ROLL_PERIOD,
    ROLL_STALENESS, ROLL_TRAVEL,
};
use karakuri_operation::BlendMode;

/// The console's arrangement at a plausible window, solved — every region laid
/// out, so that nothing here is answering ADR-0193's question by accident.
fn arrangement() -> Panel {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    panel
}

/// A settled slot with its meter reading: where it was asked to be, nothing
/// armed on either fader, and the one value on the strip that moves without a
/// hand on anything.
fn metered() -> Strip {
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
            peak: 0.31,
        }),
    }
}

/// The same strip with no reading at all, which is `Deck::level`'s `None` — no
/// meter, no measurement yet, or a slot that is neither Live nor being
/// auditioned. The bay draws the well and nothing in it.
fn dark() -> Strip {
    Strip {
        level: None,
        ..metered()
    }
}

/// The parked strip, which is the one state the engine can actually produce:
/// asked to prime, held at allocated because the budget found no room. It is
/// what makes the mixer bay declare at all.
fn parked(strip: Strip) -> Strip {
    Strip {
        tally: Tally::Allocated,
        requested: Tally::Priming,
        ..strip
    }
}

/// What the console declares this frame, in `REGIONS`' order.
fn declared(view: &View, panel: &Panel) -> Vec<Declared> {
    view.declares(panel.layout()).collect()
}

// ---------------------------------------------------------------------------
// 1. A metered console with nothing pending is still
// ---------------------------------------------------------------------------

/// Four slots metering, the bay on screen, and not one frame asked for.
///
/// The state is the still panel's: the picture and the preview row folded away,
/// no transport row — a console with no engine behind it is every test in this
/// crate — and every strip settled. The meters are reading, which is the whole
/// point of asking here: with meters enabled the strip's one measurement
/// changes on every frame the engine renders, and the reason that buys no frame
/// is that the engine renders on the frames this panel is drawn on and on no
/// others (ADR-0290).
///
/// The bay is laid out, asserted rather than assumed, so that this is not
/// ADR-0193's answer arriving by accident: a region nobody can see declares
/// nothing for a different reason, and that reason would hide this one.
///
/// Run against its defect: `mixer_declares` widened to *pending or metering* —
/// the cheapest version of giving the meter the bay's rate, and the one that
/// reads as free — fails with *"a metered console with nothing pending declared
/// [Declared { region: "mixer", cost: 1.26ms, staleness: 33.333ms, moves_in:
/// 33.333ms }] at 0 ms"*.
#[test]
fn a_metered_console_with_nothing_pending_asks_for_no_frame() {
    let panel = arrangement();
    let mut view = View::new(Room::Day);
    view.mixer = vec![metered(), metered(), metered(), metered()];

    let bay = panel
        .layout()
        .find("mixer")
        .expect("the arrangement names `mixer`");
    assert!(
        panel.layout().visible(bay),
        "the mixer bay is folded away, so this is ADR-0193 answering rather than the meter"
    );
    assert!(
        view.mixer.iter().all(|strip| strip.level.is_some()),
        "every strip in this console is supposed to be metering"
    );

    // A whole period, so that no phase of the roll is where this holds.
    for millis in (0..ROLL_PERIOD.as_millis() as u64).step_by(10) {
        view.phase = Phase::since(Duration::from_millis(millis));
        assert_eq!(
            declared(&view, &panel).len(),
            0,
            "a metered console with nothing pending declared {:?} at {millis} ms",
            declared(&view, &panel)
        );
        assert_eq!(
            view.animating(panel.layout()),
            None,
            "a metered console asked for {:?} at {millis} ms with nothing pending in the bay",
            view.animating(panel.layout())
        );
        assert_eq!(
            Change::Animating(view.animating(panel.layout())).repaint(),
            Repaint::Never,
            "the window was given a reason to draw at {millis} ms and nothing on the panel \
             is moving between frames"
        );
    }
}

// ---------------------------------------------------------------------------
// 2. The reading is not in the declaration
// ---------------------------------------------------------------------------

/// A metered bay and an unmetered one declare the same three numbers, at every
/// millisecond of a period.
///
/// This is what makes ADR-0283's count a metered panel's count. Its own
/// `settled()` strip carries a reading and `crates/karakuri/src/main.rs` meters
/// every slot in the deck, so *the* configuration this program ships is the
/// metered one: the fourteen frames a parked panel asks for are fourteen with
/// the meters on, and a declaration for the meter would be putting the
/// thirty-one back rather than adding a line.
///
/// The rest is still one sleep, asserted at the foot of it on the metered
/// console, because that is the frame ADR-0283 bought and the one a meter would
/// take back first.
///
/// Run against its defect: `moves_in` reduced to `ROLL_STALENESS` wherever a
/// strip is metering fails with *"the metered bay asked for 33.333ms at 400 ms
/// and the unmetered one asked for 600.000024ms"*.
#[test]
fn the_reading_is_not_in_what_the_mixer_bay_declares() {
    let panel = arrangement();
    let mut lit = View::new(Room::Day);
    lit.mixer = vec![metered(), parked(metered())];
    let mut unlit = View::new(Room::Day);
    unlit.mixer = vec![dark(), parked(dark())];

    for millis in 0..ROLL_PERIOD.as_millis() as u64 {
        let phase = Phase::since(Duration::from_millis(millis));
        lit.phase = phase;
        unlit.phase = phase;
        let (metered, unmetered) = (declared(&lit, &panel), declared(&unlit, &panel));
        assert_eq!(
            metered.len(),
            1,
            "a parked bay declared {metered:?} at {millis} ms, and the mixer is one region"
        );
        assert_eq!(
            metered[0].moves_in, unmetered[0].moves_in,
            "the metered bay asked for {:?} at {millis} ms and the unmetered one asked for \
             {:?}",
            metered[0].moves_in, unmetered[0].moves_in
        );
        assert_eq!(
            metered, unmetered,
            "a reading moved what the bay declares at {millis} ms"
        );
        assert_eq!(
            metered[0].moves_in,
            roll_moves_in(phase),
            "the bay asked for something other than where its roll has got to at {millis} ms"
        );
    }

    // And the rest is one sleep on the metered console, which is the frame
    // ADR-0283 bought: the roll leaves zero at `ROLL_TRAVEL` and nothing in
    // this bay moves again until the period wraps.
    lit.phase = Phase::since(ROLL_TRAVEL);
    let asked = lit.animating(panel.layout());
    assert_eq!(
        asked,
        Some(roll_moves_in(Phase::since(ROLL_TRAVEL))),
        "the metered console asked for {asked:?} at the foot of the rest"
    );
    assert!(
        asked.is_some_and(|asked| ROLL_TRAVEL + asked >= ROLL_PERIOD),
        "the metered console asked to be woken inside its own rest: {asked:?}"
    );
    assert!(
        asked.is_some_and(|asked| asked > ROLL_STALENESS),
        "the rest of a metered bay is being serviced at the travel's rate after all"
    );
}

// ---------------------------------------------------------------------------
// 3. The meter is drawn from the reading and not from the clock
// ---------------------------------------------------------------------------

/// The roll's curve reaches a fader's band and does not reach the meter, which
/// is the premise the two claims above rest on.
///
/// The two presentations are asked in the same strip, at two phases where
/// [`roll_at`] is a different number: the reach moves, because it is measured
/// from the curve, and the meter cannot, because
/// [`StripBox::meter_at`](karakuri_console::view::StripBox::meter_at) is a
/// function of the reading and there is nowhere for a phase to enter it. What
/// moves the meter is a new reading, and only a new reading — asserted in both
/// directions so that neither half is vacuous.
///
/// What the meter's rectangles are worth against the mock's own percentages is
/// `tests/mixer.rs`; this is about what does and does not move them.
///
/// This is the test that fails when a meter grows ballistics. A held peak or a
/// fall time is a function of the clock exactly as the roll is: it would move
/// between two frames, it would go stale through the roll's 566 ms rest, and it
/// would have to declare — which is ADR-0290's decision turned over, and it
/// should not be turned over quietly. It fails at the compiler rather than in
/// an assertion, because a meter on the clock takes the phase the way
/// `StripBox::trim_reach` takes the curve, and this file calls `meter_at` with
/// a reading and nothing else.
///
/// Run against its defect: `meter_at` painting a fill that does not follow the
/// reading — `filled(self.meter, Axis::Column, 0.5)` — fails with *"the fill
/// did not follow the mean, so this strip is not drawing what it was given"*,
/// which is what keeps the two assertions below from being vacuous.
#[test]
fn the_meter_is_drawn_from_the_reading_and_not_from_the_clock() {
    let ctx = drawn_once();
    let layout = solved(PLAUSIBLE);
    let strips = vec![metered()];
    let bay = mixer(&ctx, &layout, &strips).expect("the mixer bay lays out at a plausible window");
    let strip = bay.strip(0);
    let reading = strips[0].level.expect("this strip is metering");

    // Two phases of the one clock, on either side of the travel's peak.
    let (rest, travelling) = (Phase::ZERO, Phase::since(ROLL_TRAVEL / 2));
    assert_ne!(
        roll_at(rest),
        roll_at(travelling),
        "the two phases this test contrasts are the same point on the curve"
    );

    // The fader's reach is measured from the curve, so it moves with it.
    let at_rest = strip.fader_reach(0.3, 0.9, roll_at(rest));
    let under_way = strip.fader_reach(0.3, 0.9, roll_at(travelling));
    assert_ne!(
        at_rest.band, under_way.band,
        "the fader's reach did not move between two phases, so this strip is not on the \
         panel's clock at all and the contrast below says nothing"
    );

    // The meter is measured from the reading, and the same reading is the same
    // picture whatever the panel's clock is doing.
    assert_eq!(
        strip.meter_at(reading),
        strip.meter_at(reading),
        "one reading drew two meters"
    );

    // And a reading that moved is the one thing that moves it.
    let louder = Level {
        mean: reading.mean + 0.2,
        peak: reading.peak + 0.2,
    };
    let (was, now) = (strip.meter_at(reading), strip.meter_at(louder));
    assert_eq!(was.well, now.well, "the well moved with the reading in it");
    assert_ne!(
        was.fill, now.fill,
        "the fill did not follow the mean, so this strip is not drawing what it was given"
    );
    assert_ne!(
        was.peak, now.peak,
        "the peak mark did not follow the peak, so this strip is not drawing what it was given"
    );
}
