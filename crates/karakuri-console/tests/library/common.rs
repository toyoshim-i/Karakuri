pub use super::common::{drawn_once, id_of, near, rect_of, solved, PLAUSIBLE, SMALLEST};
pub use karakuri_console::egui;
pub use karakuri_console::input::{claim, wheeled, Claim, Turned};
pub use karakuri_console::panel::{Panel, GRAB};
pub use karakuri_console::room::{size, Room};
pub use karakuri_console::view::{
    library, mcp_pill, Aim, Field, Filters, KindChip, LibraryBay, Opened, Picked, Published, Read,
    Reading, RowItem, RowKind, Rows, Scope, Target, View, DECK_LETTERS, HOLDS_UNSET, LAYERS,
};
pub use karakuri_layout::{Point, Rect};
pub use karakuri_operation::gate::{Class, Open};
pub use karakuri_operation::{Operation, SetTransfer};

pub fn listed(names: &[String]) -> Rows<'_> {
    Rows { names, kinds: &[] }
}

pub fn mock() -> Vec<String> {
    [
        "drift_night",
        "lattice_veil",
        "glass_shell",
        "night01",
        "strand_bloom",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect()
}

pub const SCOPES: &[Scope] = &Scope::ALL;

pub fn console(viewport: Rect) -> Panel {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    panel
}

pub const AIMED: Target = Target {
    deck: 0,
    decks: 0,
    open: false,
};

pub fn bay(panel: &Panel) -> LibraryBay {
    library(panel.layout(), SCOPES, &mock(), None, None, 0.0)
        .expect("the library bay lists its rows")
}

pub fn to_egui(r: Rect) -> egui::Rect {
    egui::Rect::from_min_size(egui::pos2(r.x, r.y), egui::vec2(r.w, r.h))
}

pub fn viewport(panel: &Panel) -> egui::Rect {
    to_egui(panel.layout().viewport())
}

pub fn showing_mock() -> (View, Panel) {
    let mut view = View::new(Room::Day);
    view.library = mock();
    view.scopes = Scope::ALL.to_vec();
    (view, console(PLAUSIBLE))
}

pub fn strip() -> karakuri_console::view::Strip {
    karakuri_console::view::Strip {
        name: String::new(),
        tally: karakuri_console::view::Tally::Allocated,
        requested: karakuri_console::view::Tally::Allocated,
        gain: 0.0,
        gain_to: None,
        opacity: 0.0,
        opacity_to: None,
        blend: karakuri_operation::BlendMode::Over,
        mask: karakuri_console::view::Mask::None,
        mask_angle: 0.0,
        level: None,
        is_muted: false,
        is_soloed: false,
    }
}

pub fn reading() -> Reading {
    Reading {
        id: "drift_night".to_owned(),
        knobs: [
            ("radius", "0 – 8 · 2"),
            ("turbulence", "0 – 3 · 0.4"),
            ("amount", "0 – 1 · 0.5"),
            ("twist", "−2 – 2 · 0"),
            ("hue", "0 – 1 · 0.5"),
            ("exposure", "0 – 1 · 0.4"),
        ]
        .iter()
        .map(|(key, range)| Published {
            key: (*key).to_owned(),
            range: (*range).to_owned(),
        })
        .collect(),
        capacity: Some("16384 – 1048576 · 262144".to_owned()),
        emits: Some("position, size, life".to_owned()),
        nodes: 5,
        described: 5,
    }
}

/// Every shape the console paints wholly inside `rect`, on one frame.
pub fn shapes_inside(view: &mut View, panel: &mut Panel, rect: egui::Rect) -> Vec<egui::Shape> {
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

/// Every shape the scope row painted, which is not `shapes_inside`.
pub fn shapes_across(view: &mut View, panel: &mut Panel, row: egui::Rect) -> Vec<egui::Shape> {
    shapes_inside(view, panel, egui::Rect::EVERYTHING)
        .into_iter()
        .filter(|shape| {
            let bounds = shape.visual_bounding_rect();
            bounds.is_finite()
                && bounds.min.y >= row.min.y - 1.0
                && bounds.max.y <= row.max.y + 1.0
                && bounds.intersects(row)
        })
        .collect()
}

/// Every word the scope row painted, left to right.
pub fn chips(view: &mut View, panel: &mut Panel, bay: &LibraryBay) -> Vec<String> {
    let row = bay.scopes.expect("the bay was handed scopes");
    let mut found: Vec<(f32, String)> = shapes_across(view, panel, row)
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Text(at) => Some((at.pos.x, at.galley.text().to_owned())),
            _ => None,
        })
        .collect();
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    found.into_iter().map(|(_, text)| text).collect()
}

/// The word inside the one washed chip, or `None` where nothing is washed — and
/// it panics where more than one is, which is the half of the claim a `Vec` of
/// them would let pass.
pub fn marked(view: &mut View, panel: &mut Panel, bay: &LibraryBay) -> Option<String> {
    let row = bay.scopes.expect("the bay was handed scopes");
    let shapes = shapes_across(view, panel, row);
    let washes: Vec<egui::Rect> = shapes
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Rect(at) => Some(at.rect),
            _ => None,
        })
        .collect();
    assert!(
        washes.len() <= 1,
        "{} chips are washed, and `.scope.sel` is one of them",
        washes.len()
    );
    let wash = washes.first()?;
    shapes.iter().find_map(|shape| match shape {
        egui::Shape::Text(at) if wash.contains(at.pos) => Some(at.galley.text().to_owned()),
        _ => None,
    })
}
