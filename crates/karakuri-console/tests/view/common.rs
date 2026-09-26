pub(crate) use super::common::{
    arranged, drawn_once, id_of, near, rect_of, showing, solved, PLAUSIBLE, SMALLEST,
};
pub(crate) use karakuri_console::egui;
pub(crate) use karakuri_console::input::{claim, Claim};
pub(crate) use karakuri_console::panel::Panel;
pub(crate) use karakuri_console::room::{Palette, Room};
pub(crate) use karakuri_console::view::{
    caption_of, picture_rect, plan_into, preview_rects, region, Kind, Placed, DECKS, REGIONS,
};
pub(crate) use karakuri_layout::{NodeId, Point};

pub(crate) const BAYS: &[&str] = &[
    "library",
    "staging",
    "prompt",
    "program",
    "inspector",
    "mixer",
    "master",
    "sequencer",
];

pub(crate) const CANVAS: (u32, u32) = (1280, 720);
pub(crate) const SQUARISH: (u32, u32) = (1024, 768);
pub(crate) const ROWS: &[&str] = &["transport", "outputs"];

pub(crate) fn planned(panel: &mut Panel) -> Vec<Placed> {
    let mut out = Vec::new();
    plan_into(panel, CANVAS, &mut out);
    out
}
