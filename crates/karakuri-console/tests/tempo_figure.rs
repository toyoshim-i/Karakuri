//! The tempo figure: the track is the reading, and the band is a guard on the
//! hand.
//!
//! `docs/manual/console.html` gives `.bpm` a `data-tip` and it is this
//! control's specification: *"Click along the figure to name a tempo … A press
//! names a value outright rather than stepping, so the figure is the track and
//! the number under your finger is the one you get. The band is ±15% of what
//! the grid is running at, and a press outside it is ignored rather than
//! clamped — that is a guard against a mis-click and not a statement about what
//! a tempo may be."*
//!
//! Four things, and the second and the third are why this file exists rather
//! than a few more assertions in `tests/transport.rs`:
//!
//! 1. That the figure is a track and its middle is the number it draws — the
//! one property that makes *the number under your finger* true rather than a
//! phrase, and the reason no track is painted under it. 2. That a press names
//! the tempo it landed on, outright and not by a step, which is what
//! `Operation::SetFreeRunTempo` carries and what separates this control from
//! the octave beside it. 3. That a press outside the band is ignored — no
//! operation, nothing clamped to the edge of the band, and the point left to
//! `egui`. It is asserted at a point *on the figure*, because a guard that
//! could only be missed by missing the number is not a guard at all. 4. That
//! the band is the tempo at the press and moves with the grid, so the figure is
//! a percentage of whatever is running rather than a range written down
//! anywhere — and that it bounds a press and not a tempo: 240 is out of reach
//! in one press from the mock's 128 and reached by walking the band five times.
//!
//! None of it needs a window or a device. It does need `egui`'s fonts, because
//! the figure's width is the width of the number in it — see
//! `common::drawn_once`.
//!
//! What is not here is the claim, and it is not here because it is not this
//! crate's to make alone: `input::PROBES` is what puts a control in front of
//! `egui` and `karakuri/src/main.rs` is what acts on it. What this file can say
//! about the pointer is the half that holds either way — a press on the guard
//! is `egui`'s — and it says it below.

mod common;

use common::{drawn_once, near, rect_of, showing, PLAUSIBLE};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Panel, GRAB};
use karakuri_console::view::{transport, Transport, TransportRow, TEMPO_BAND, TEMPO_SPAN};
use karakuri_layout::{Point, Rect};
use karakuri_operation::Operation;

/// The mock's own transport, which is where `128.0` comes from.
fn mock() -> Transport {
    common::mock_transport()
}

/// A panel at a viewport, solved, with a context that has drawn once.
fn console(viewport: Rect) -> (Panel, egui::Context) {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    (panel, drawn_once())
}

/// The row, drawn at `bpm`.
fn row_at(panel: &Panel, ctx: &egui::Context, bpm: f32) -> TransportRow {
    transport(ctx, panel.layout(), Some(Transport { bpm, ..mock() })).expect("the transport row")
}

/// A point `unit` of the way along the figure, on its centre line.
fn along(row: &TransportRow, unit: f32) -> Point {
    Point::new(row.bpm.min.x + row.bpm.width() * unit, row.bpm.center().y)
}

/// What a press there asks for, in beats a minute — and nothing at all where
/// the press was not this control's.
fn asked(row: &TransportRow, unit: f32) -> Option<f32> {
    match row.tempo(along(row, unit)) {
        Some(Operation::SetFreeRunTempo { bpm }) => Some(bpm),
        Some(other) => panic!("the tempo figure emitted {other:?}"),
        None => None,
    }
}

// ---------------------------------------------------------------------------
// The figure is the track
// ---------------------------------------------------------------------------

/// The middle of the number is the number, and the ends are [`TEMPO_SPAN`]
/// either way.
///
/// This is the whole of why nothing is painted for this control: the figure is
/// drawn at the tempo it names, so the point that asks for what is already
/// running is the point the ink is on. A mapping with the tempo anywhere else
/// along the figure would need a mark to say where it was.
#[test]
fn the_middle_of_the_figure_is_the_number_it_draws() {
    let (panel, ctx) = console(PLAUSIBLE);
    let row = row_at(&panel, &ctx, 128.0);

    assert!(
        near(row.tempo_at(0.5), 128.0),
        "the middle of the figure asks for {} and the figure reads 128.0",
        row.tempo_at(0.5)
    );
    assert!(near(row.tempo_at(0.0), 128.0 * (1.0 - TEMPO_SPAN)));
    assert!(near(row.tempo_at(1.0), 128.0 * (1.0 + TEMPO_SPAN)));

    // The band is the middle half of it, which is [`TEMPO_SPAN`] being two
    // bands: the quarters are exactly the ±15% and everything between them is
    // a tempo a press may ask for.
    assert!(near(row.tempo_at(0.25), 128.0 * (1.0 - TEMPO_BAND)));
    assert!(near(row.tempo_at(0.75), 128.0 * (1.0 + TEMPO_BAND)));
    assert!(row.in_band(row.tempo_at(0.25)));
    assert!(row.in_band(row.tempo_at(0.75)));
    assert!(
        !row.in_band(row.tempo_at(0.0)),
        "the left-hand end is inside the band, so nothing on this figure can be ignored"
    );
    assert!(
        !row.in_band(row.tempo_at(1.0)),
        "the right-hand end is inside the band, so nothing on this figure can be ignored"
    );

    // And the figure is wide enough to press: the guard at either end is a
    // target of its own, not a rounding.
    let guard = row.bpm.width() * 0.25;
    assert!(
        guard >= 8.0,
        "the guard either side of the band is {guard} pixels wide, which is not something a \
         hand can be said to have missed by"
    );
}

/// A press names the tempo it landed on, and two presses a few pixels apart
/// name two different tempi — which is what *outright rather than stepping*
/// means and is the difference between this control and the `×2` beside it.
#[test]
fn a_press_names_the_tempo_it_landed_on() {
    let (panel, ctx) = console(PLAUSIBLE);
    let row = row_at(&panel, &ctx, 128.0);

    assert_eq!(asked(&row, 0.5), Some(128.0));
    assert!(near(
        asked(&row, 0.75).expect("a press on the band's right edge"),
        147.2
    ));
    assert!(near(
        asked(&row, 0.25).expect("a press on the band's left edge"),
        108.8
    ));

    // Monotone across the band, and no two of these are the same answer: a
    // control that stepped would give the same tempo for every press on one
    // side of it.
    let mut last = f32::NEG_INFINITY;
    for step in 0..=10 {
        let unit = 0.25 + 0.05 * step as f32;
        let bpm = asked(&row, unit)
            .unwrap_or_else(|| panic!("a press {unit} along the figure was refused"));
        assert!(
            bpm > last,
            "a press at {unit} asks for {bpm} and one to the left of it asked for {last}"
        );
        assert!(row.in_band(bpm));
        last = bpm;
    }
}

// ---------------------------------------------------------------------------
// The guard
// ---------------------------------------------------------------------------

/// A press outside the band is ignored: no operation, nothing clamped, and the
/// point stays `egui`'s.
///
/// The press is *on the number* — a fifth of the figure's width past the band's
/// edge — which is what makes this the guard rather than a miss. And the clamp
/// is what is being refused: a press the operator did not mean, turned into the
/// largest move this row can make, is the failure the band exists for.
#[test]
fn a_press_outside_the_band_is_ignored_rather_than_clamped() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let row = row_at(&panel, &ctx, 128.0);
    let mut view = showing(&[]);
    view.transport = Some(mock());

    for unit in [0.0, 0.05, 0.1, 0.2, 0.8, 0.9, 0.95, 1.0] {
        let would = row.tempo_at(unit);
        assert!(
            !row.in_band(would),
            "a press {unit} along the figure asks for {would}, which is inside the band — this \
             probe is no longer outside it"
        );
        assert!(
            row.bpm
                .contains(egui::pos2(along(&row, unit).x, along(&row, unit).y)),
            "the probe at {unit} is not on the figure at all, so it is a miss and not the guard"
        );
        assert_eq!(
            asked(&row, unit),
            None,
            "a press {unit} along the figure was taken — clamped to the band's edge, or taken \
             outright"
        );
        assert!(!row.on_tempo(along(&row, unit)));
        // **And the panel does not swallow it.** A control that claimed the
        // press and then did nothing is the one outcome an operator cannot
        // tell from a panel that has stopped.
        assert_eq!(
            claim(&mut panel, &ctx, &view, along(&row, unit)),
            Claim::Egui,
            "a press {unit} along the figure is claimed by the panel and asks for nothing"
        );
    }

    // Past the number either way is not this control's either, and it is a
    // different answer from the guard's: there is no figure under it.
    for x in [row.bpm.min.x - 1.0, row.bpm.max.x + 1.0] {
        assert_eq!(row.tempo(Point::new(x, row.bpm.center().y)), None);
    }
    // Nor above or below it, which is the row's own padding: `.bpm` is 30 of
    // the row's 48 and the rest is the boundary's and `egui`'s.
    for y in [row.bpm.min.y - 1.0, row.bpm.max.y + 1.0] {
        assert_eq!(row.tempo(Point::new(row.bpm.center().x, y)), None);
    }
}

/// The figure is clear of the boundary under the row, which is the clearance
/// `tests/arrangement_pill.rs` keeps for the one control in this row that had
/// it first: a control inside a boundary's [`GRAB`] is a control the pointer
/// rule never reaches, because rule 3 answers before rule 4.
#[test]
fn the_figure_is_clear_of_the_boundary_bands() {
    let (panel, ctx) = console(PLAUSIBLE);
    let strip = rect_of(panel.layout(), "transport");
    let row = row_at(&panel, &ctx, 128.0);

    assert!(
        row.bpm.min.y - strip.y >= GRAB,
        "the figure's top is {} from the row's, and the boundary above takes {GRAB}",
        row.bpm.min.y - strip.y
    );
    assert!(
        (strip.y + strip.h) - row.bpm.max.y >= GRAB,
        "the figure's bottom is {} from the row's, and the boundary below takes {GRAB}",
        (strip.y + strip.h) - row.bpm.max.y
    );
}

// ---------------------------------------------------------------------------
// The band is the tempo at the press
// ---------------------------------------------------------------------------

/// ±15% of what the grid is running at, and not of a range written down
/// anywhere.
///
/// The same point on the figure asks for a different tempo at every tempo, and
/// nothing here holds a pair of ends: `karakuri_audio::tempo::BPM_RANGE` says
/// of itself that it is not the range of answers, and the band does not say it
/// either.
///
/// 240 is the case the manual names, and the band is what says how long it
/// takes rather than whether it happens: one press from the mock's 128 cannot
/// reach it, and walking the right-hand edge of the band five times does — with
/// the figure naming 240 outright on the last of them, because by then it is
/// inside the band.
///
/// The manual's tip says two presses and that is not arithmetic that works from
/// 128. ±15% a press is 147.2, and the `×2` beside the figure is drawn inert at
/// this tempo — `karakuri_audio`'s `BPM_RANGE` does not contain 256 — so there
/// is no pair of presses that reaches 240 from where the mock stands. The walk
/// is asserted here at the length it actually is; the tip is reported rather
/// than met by widening a band somebody decided.
#[test]
fn the_band_is_the_tempo_at_the_press_and_moves_with_the_grid() {
    let (panel, ctx) = console(PLAUSIBLE);
    let slow = row_at(&panel, &ctx, 128.0);
    let fast = row_at(&panel, &ctx, 256.0);

    // One press from 128 does not reach 240, and the figure does not even name
    // it: the guard is not the only thing in the way.
    assert!(!slow.in_band(240.0));
    assert!(slow.tempo_at(1.0) < 240.0);

    // The walk: press the right-hand edge of the band until 240 is inside it,
    // and then press 240 itself.
    let mut bpm = 128.0;
    let mut presses = 0;
    while !near(bpm, 240.0) {
        let row = row_at(&panel, &ctx, bpm);
        bpm = match row.in_band(240.0) {
            true => {
                let unit = 0.5 + (240.0 / bpm - 1.0) / (2.0 * TEMPO_SPAN);
                asked(&row, unit).expect("a press naming 240 from inside the band")
            }
            // Just inside the band's right edge: the edge itself is the
            // one point whose round trip through a pixel can land a hair
            // outside a bound written with `<=`.
            false => asked(&row, 0.745).expect("a press inside the band's right edge"),
        };
        presses += 1;
        assert!(presses <= 12, "240 was not reached by walking the band");
    }
    assert_eq!(
        presses, 5,
        "the walk from the mock's tempo to 240 is {presses} presses, and the manual's tip says          two — if this number moved, the band moved with it"
    );

    // The last of those lands because 240 is inside the band by then, which is
    // the whole reason the walk arrives rather than approaching: the press
    // before it left the grid within 15% of where it was going.
    assert!(row_at(&panel, &ctx, 240.0 / 1.15).in_band(240.0));

    // And the same pixel is a different tempo on the two rows, because the
    // whole control is a percentage of what is running.
    assert!(near(fast.tempo_at(0.75), 2.0 * slow.tempo_at(0.75)));
    assert!(
        !fast.in_band(slow.tempo_at(1.0)),
        "the fastest tempo a press can name at 128 is inside the band at 256, so the band is \
         not moving with the grid"
    );
    // The figure is the same width at both, because both numbers are five
    // glyphs — so this is the *scale* moving and not the box.
    assert!(near(slow.bpm.width(), fast.bpm.width()));
}
