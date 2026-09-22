pub(crate) use super::common::{
    at, console, drawn_once, id_of, near, rect_of, showing, solved, PLAUSIBLE, SMALLEST,
};
pub(crate) use karakuri_console::egui;
pub(crate) use karakuri_console::input::{claim, Claim};
pub(crate) use karakuri_console::panel::Panel;
pub(crate) use karakuri_console::room::{size, Room};
pub(crate) use karakuri_console::view::{beat_at, transport, Stage, Transport, TransportRow, View};
pub(crate) use karakuri_layout::{Point, Rect};

pub(crate) fn mock() -> Transport {
    super::common::mock_transport()
}

pub(crate) fn row(panel: &Panel, ctx: &egui::Context) -> TransportRow {
    transport(ctx, panel.layout(), Some(mock())).expect("the transport row draws its readouts")
}

/// Every shape the console paints wholly inside `rect`, on one frame.
pub(crate) fn shapes_inside(
    view: &mut View,
    panel: &mut Panel,
    rect: egui::Rect,
) -> Vec<egui::Shape> {
    let ctx = drawn_once();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, panel));
    out.textures_delta.clear();
    out.shapes
        .into_iter()
        .filter(|clipped| {
            let bounds = clipped.shape.visual_bounding_rect();
            bounds.is_finite() && rect.contains_rect(bounds)
        })
        .map(|clipped| clipped.shape)
        .collect()
}
