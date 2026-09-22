pub(crate) use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

pub(crate) use super::common::{
    self as common, console, drawn_once, near, point, rect_of, showing, PLAUSIBLE, SMALLEST,
};
pub(crate) use karakuri_console::egui;
pub(crate) use karakuri_console::input::{claim, Claim};
pub(crate) use karakuri_console::panel::{Panel, GRAB};
pub(crate) use karakuri_console::room::{size, Room};
pub(crate) use karakuri_console::view::{
    mixer, transition, Go, Level, Mask, Strip, Tally, TransitionRow, TransitionSettings, View,
};
pub(crate) use karakuri_layout::{Hit, Point};
pub(crate) use karakuri_operation::{BlendMode, Operation, TransitionSetting, WipeKind};

pub(crate) const SHAPES: [(WipeKind, f32); 6] = [
    (WipeKind::None, 0.0),
    (WipeKind::Linear, 0.0),
    (WipeKind::Linear, FRAC_PI_2),
    (WipeKind::Linear, FRAC_PI_4),
    (WipeKind::Linear, -FRAC_PI_4),
    (WipeKind::Radial, 0.0),
];

/// The three grids the second pill cycles — `karakuri-cli`'s `QUANTA`, in its
/// order: the next bar, the next beat, now.
pub(crate) const QUANTA: [f64; 3] = [4.0, 1.0, 0.0];

/// The four lengths the third pill cycles — `karakuri-cli`'s `FADE_BEATS`: a
/// bar, half a bar, two bars, and a cut.
pub(crate) const LENGTHS: [f64; 4] = [4.0, 2.0, 8.0, 0.0];

/// The words the six shapes read, in the same order — what the pill is as wide
/// as and what a reader sees. `no shape` and never `off`: that word is a
/// residency on this console and `preview_caption.rs` asserts it is painted
/// nowhere.
pub(crate) const SHAPE_WORDS: [&str; 6] = [
    "no shape",
    "left",
    "up",
    "diagonal",
    "back diagonal",
    "iris",
];
pub(crate) const QUANTUM_WORDS: [&str; 3] = ["next bar", "next beat", "now"];
pub(crate) const LENGTH_WORDS: [&str; 4] = ["4 beats", "2 beats", "8 beats", "cut"];

/// Three strips, so the bay above the row is drawn and a press on it can be
/// asked about. The values are `mixer.rs`'s mock read loosely; nothing here
/// reads any of them.
pub(crate) fn strips() -> Vec<Strip> {
    ["drift_night", "lattice_veil", "glass_shell"]
        .into_iter()
        .enumerate()
        .map(|(slot, name)| Strip {
            name: name.to_owned(),
            tally: Tally::Live,
            requested: Tally::Live,
            gain: 0.2 + 0.15 * slot as f32,
            gain_to: None,
            opacity: 0.8 - 0.15 * slot as f32,
            opacity_to: None,
            blend: BlendMode::ALL[slot % BlendMode::ALL.len()],
            mask: Mask::None,
            mask_angle: 0.0,
            level: Some(Level {
                mean: 0.5,
                peak: 0.6,
            }),
            is_muted: false,
            is_soloed: false,
        })
        .collect()
}

/// A console showing `strips` with the transition row at `settings`.
pub(crate) fn showing_at(strips: &[Strip], settings: TransitionSettings) -> View {
    let mut view = showing(strips);
    apply(&mut view, settings);
    view
}

/// Walk `view`'s row to `settings` through the only door there is, and panic
/// where a setting is refused — a test that silently kept the old value would
/// be asserting about a row it did not set.
pub(crate) fn apply(view: &mut View, settings: TransitionSettings) {
    for setting in [
        TransitionSetting::WipeShape {
            kind: settings.kind,
            angle: settings.angle,
        },
        TransitionSetting::Quantum {
            beats: settings.quantum,
        },
        TransitionSetting::Length {
            beats: settings.length,
        },
    ] {
        view.set_transition(setting);
    }
    assert_eq!(
        view.transition(),
        settings,
        "the console refused a setting this test needs it to be on"
    );
}

/// The row, laid out. Panics where it is not drawn, which is the failure worth
/// reading.
pub(crate) fn row(
    panel: &Panel,
    ctx: &egui::Context,
    settings: TransitionSettings,
) -> TransitionRow {
    transition(ctx, panel.layout(), settings).expect("the mixer bay draws its transition row")
}

/// Settings at a named place in each of the three cycles.
pub(crate) fn place(shape: usize, quantum: usize, length: usize) -> TransitionSettings {
    TransitionSettings {
        kind: SHAPES[shape].0,
        angle: SHAPES[shape].1,
        quantum: QUANTA[quantum],
        length: LENGTHS[length],
    }
}

/// Every text the console paints on one frame, with where it was painted —
/// `preview_caption.rs`'s helper, which is how *what is drawn* is asked
/// anywhere in this crate.
pub(crate) fn texts(view: &mut View, panel: &mut Panel) -> Vec<(egui::Pos2, String)> {
    let ctx = drawn_once();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, panel));
    out.textures_delta.clear();
    out.shapes
        .into_iter()
        .filter_map(|c| match c.shape {
            egui::Shape::Text(at) => Some((at.pos, at.galley.text().to_owned())),
            _ => None,
        })
        .collect()
}
