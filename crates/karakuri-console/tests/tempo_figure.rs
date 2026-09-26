//! The tempo figure interaction: track reading, absolute tempo naming, and ±15% guard bands.
//!
//! Validates centering on current BPM, absolute value dispatch, ignoring out-of-band clicks,
//! and iterative band traversal to target tempos.

mod common;

use common::{console, near, rect_of, showing, PLAUSIBLE};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Panel, GRAB};
use karakuri_console::view::{transport, Transport, TransportRow, TEMPO_BAND, TEMPO_SPAN};
use karakuri_layout::Point;
use karakuri_operation::Operation;

/// The mock's own transport, which is where `128.0` comes from.
fn mock() -> Transport {
    common::mock_transport()
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

/// The center of the tempo number corresponds to current BPM, spanning [`TEMPO_SPAN`] in each direction.
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

/// Clicks outside the ±15% band are ignored and left to egui without clamping.
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

/// The ±15% valid band scales dynamically with current tempo, requiring multiple steps to reach distant values.
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
