//! Default arrangement, layout constraints, view model, and rendering for the console panel.
//!
//! Coordinates layout hierarchy and sizing constraints (ADR-0155, ADR-0156, ADR-0157, ADR-0164)
//! matching `docs/manual/console.html`.

pub mod budget;
pub mod control;
pub mod focus;
pub mod hover;
pub mod input;
pub mod panel;
pub mod repaint;
pub mod room;
pub mod view;

pub use control::*;

/// Re-exported GUI toolkit crates pinned to ensure compatibility with `wgpu` (ADR-0155).
pub use {egui, egui_wgpu, egui_winit};

use karakuri_layout::{Layout, Spec};

// For the inspector's divider, which is `.divider-v`'s width and is stated
// once, in `room::size`. See [`room::size::PANE_DIVIDER`].
use crate::room::size;

/// Between the transport, the body row and the outputs row: `.console`'s `gap:
/// 10px`.
const ROOT_DIVIDER: f32 = 10.0;

/// Between the three columns: `.body-grid`'s `gap: 10px`.
const COLUMN_DIVIDER: f32 = 10.0;

/// Between the bays stacked inside one column: `.col`'s `gap: 10px`.
const BAY_DIVIDER: f32 = 10.0;

/// Between the Program bay's two regions: the console's own 4px divider between
/// the picture and the row of deck previews under it.
const PROGRAM_DIVIDER: f32 = 4.0;

/// The narrowest an inspector pane may be (208px), based on parameter row elements
/// (padding, ordinal, gaps, name, value) plus 1px minimum fader width (ADR-0272).
const INSPECTOR_PANE_MIN: f32 = size::PARAM_PAD_L
    + size::PARAM_ORD_W
    + size::PARAM_GAP * 3.0
    + size::PARAM_NAME_W
    + size::PARAM_VAL_W
    + size::PARAM_PAD_R
    + 1.0;

/// The console's default layout specification.
///
/// Configures columns and bay regions per ADR-0159 and ADR-0204.
pub fn arrangement() -> Spec {
    Spec::column(
        ROOT_DIVIDER,
        vec![
            // Transport row: fixed height 48px (9 + 30 + 9) matching fixed readout contents.
            Spec::view("transport").fixed(48.0).min(48.0).max(48.0),
            Spec::row(COLUMN_DIVIDER, vec![left_pane(), centre(), right_pane()])
                .flex(1.0)
                // Minimum height matches the tallest column (centre: program 395 + divider 10 + inspector 151.5).
                .min(556.5),
            // Outputs row: fixed height 34px (8 + 18.5 + 8 rounded) for fixed chips.
            Spec::view("outputs").fixed(34.0).min(34.0).max(34.0),
        ],
    )
}

/// Returns the constructed console layout.
pub fn layout() -> Layout {
    Layout::new(arrangement())
}

/// Minimum viewport (777.0 x 658.5) where every region satisfies its declared minimum (the console's own).
///
/// Sums declared tracks and dividers (ADR-0250, ADR-0272, ADR-0279). Recomputed in `tests/arrangement.rs`.
pub const MINIMUM_VIEWPORT: (f32, f32) = (777.0, 658.5);

/// Left column specification: flexible library bay over fixed-height staging lane.
fn left_pane() -> Spec {
    Spec::column(
        BAY_DIVIDER,
        vec![
            // Library minimum: head 27 + scope 31.5 + 3 result rows 73.5 + foot 26 = 158px.
            Spec::view("library")
                .flex(1.0)
                .min(158.0)
                .collapsed_size(size::HEAD_H),
            // Staging bay: natural height 125px (head 27 + padding 14 + 3 rows + gaps); min 66px (one candidate).
            Spec::view("staging")
                .fixed(125.0)
                .min(66.0)
                .max(125.0)
                .collapsed_size(size::HEAD_H),
        ],
    )
    .named("left-pane")
    // Preserves outer edge at zero width when collapsed (ADR-0300).
    .keeps_its_edge()
    // `.body-grid`'s first track: `340px` (ADR-0239).
    .fixed(340.0)
    // Minimum: 160.0 allows legible text and timestamp without truncation.
    .min(160.0)
    .max(f32::INFINITY)
}

/// The program over the inspector.
fn centre() -> Spec {
    Spec::column(BAY_DIVIDER, vec![program(), inspector()])
        .named("centre")
        .flex(1.0)
        .min(INSPECTOR_PANE_MIN * 2.0 + size::PANE_DIVIDER)
        .max(f32::INFINITY)
}

/// The Program bay split (program view picture and deck previews row).
///
/// Configures picture sink (`program-view`) and preview row (`deck-previews`)
/// as distinct folding nodes (ADR-0170, ADR-0174).
fn program() -> Spec {
    Spec::column(
        PROGRAM_DIVIDER,
        vec![
            // Picture view absorbs bay height adjustments; preview row remains fixed underneath.
            Spec::view("program-view")
                .flex(1.0)
                .min(120.0)
                .max(f32::INFINITY),
            // Deck preview row: fixed 89px height for four preview cells.
            Spec::view("deck-previews")
                .fixed(89.0)
                .min(89.0)
                .max(f32::INFINITY),
        ],
    )
    .named("program")
    .fixed(395.0)
    .min(217.0)
    .max(f32::INFINITY)
    .collapsed_size(size::HEAD_H)
}

/// Centre inspector split configuring two side-by-side panes (`inspector-1`, `inspector-2`).
fn inspector() -> Spec {
    Spec::row(
        size::PANE_DIVIDER,
        vec![
            // Each pane requires at least INSPECTOR_PANE_MIN (208px) to draw a parameter row with fader (ADR-0279).
            Spec::view("inspector-1").flex(1.0).min(INSPECTOR_PANE_MIN),
            Spec::view("inspector-2").flex(1.0).min(INSPECTOR_PANE_MIN),
        ],
    )
    .named("inspector")
    // Flexible inspector pane absorbing vertical space under the program bay.
    .flex(1.0)
    // Inspector min height (151.5px): head, half-head, deck-head, node-head, and 2 parameter rows.
    .min(
        size::HEAD_H
            + size::HALF_HEAD_H
            + size::DECK_HEAD_H
            + size::NODE_HEAD_H
            + size::PARAM_H * 2.0,
    )
    .collapsed_size(size::HEAD_H)
}

/// Right column specification: fixed mixer, flexible master chain, and fixed sequencer.
fn right_pane() -> Spec {
    Spec::column(
        BAY_DIVIDER,
        vec![
            // Mixer bay: fixed height 292px ensuring four channel strips and transition row fit without dead space (ADR-0157).
            Spec::view("mixer")
                .fixed(292.0)
                .min(292.0)
                .max(292.0)
                .collapsed_size(size::HEAD_H),
            // Master minimum height (94px): bay head 27, padding 18, out row 16.5, gap 8, and one fx slot 24.5.
            Spec::view("master")
                .flex(1.0)
                .min(94.0)
                .max(f32::INFINITY)
                .collapsed_size(size::HEAD_H),
            // Sequencer bay: natural height 178px; minimum 119.5px holding ruler, one lane, and footer.
            Spec::view("sequencer")
                .fixed(178.0)
                .min(119.5)
                .max(f32::INFINITY)
                .collapsed_size(size::HEAD_H),
        ],
    )
    .named("right-pane")
    // Preserves outer edge at zero width when collapsed (ADR-0300).
    .keeps_its_edge()
    // `.body-grid`'s third track: `400px` (ADR-0239).
    .fixed(400.0)
    // Minimum width 172px: fits four mixer strips side-by-side with padding and gaps.
    .min(172.0)
    .max(f32::INFINITY)
}
