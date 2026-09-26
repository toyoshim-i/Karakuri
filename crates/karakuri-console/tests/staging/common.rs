pub(crate) use super::common::{
    console_panel as console, drawn_once, rect_of, showing, PLAUSIBLE, SMALLEST,
};
pub(crate) use karakuri_console::egui;
pub(crate) use karakuri_console::input::{claim, Claim};
pub(crate) use karakuri_console::panel::Panel;
pub(crate) use karakuri_console::room::Room;
pub(crate) use karakuri_console::view::{
    staging, Candidate, Kind, Level, Pane, Stage, Strip, Tally, View, DECK_LETTERS, PANES,
};
pub(crate) use karakuri_layout::{Point, Rect};

/// `egui`'s rectangle, from `karakuri_layout`'s.
pub(crate) fn to_egui(r: Rect) -> egui::Rect {
    egui::Rect::from_min_size(egui::pos2(r.x, r.y), egui::vec2(r.w, r.h))
}

/// One mixer strip, live and settled — `mixer.rs`'s own, with everything this
/// file does not care about at rest.
pub(crate) fn strip(name: &str) -> Strip {
    Strip {
        name: name.to_owned(),
        tally: Tally::Live,
        requested: Tally::Live,
        gain: 0.72,
        gain_to: None,
        opacity: 1.0,
        opacity_to: None,
        blend: karakuri_operation::BlendMode::Add,
        mask: karakuri_console::view::Mask::None,
        mask_angle: 0.0,
        level: Some(Level {
            mean: 0.74,
            peak: 0.82,
        }),
        is_muted: false,
        is_soloed: false,
    }
}

/// Creates a candidate for a changed node on a deck slot, defaulting to `L1:0`.
pub(crate) fn candidate(deck: usize, name: &str, stage: Stage) -> Candidate {
    at_node(deck, karakuri_operation::Layer::L1, 0, name, stage)
}

/// The same, addressed.
pub(crate) fn at_node(
    deck: usize,
    layer: karakuri_operation::Layer,
    index: u32,
    name: &str,
    stage: Stage,
) -> Candidate {
    Candidate {
        deck,
        at: Some(karakuri_operation::NodeAddress { layer, index }),
        addr: format!("{}:{index}", layer_word(layer)),
        name: name.to_owned(),
        stage,
        // Empty by default; only `Stage::NotCompiled` rows carry sentences (see [`refused_candidate`]).
        said: Vec::new(),
    }
}

/// A row that names no node — a build that did not happen, or a rebuild that
/// restated the stack and changed nothing in it. It is the slot's row and
/// offers neither press.
pub(crate) fn slot_row(deck: usize, name: &str, stage: Stage) -> Candidate {
    Candidate {
        deck,
        at: None,
        addr: String::new(),
        name: name.to_owned(),
        stage,
        said: Vec::new(),
    }
}

/// Returns the layer abbreviation matching the host's `.addr` spelling in `karakuri/src/main.rs`.
pub(crate) fn layer_word(layer: karakuri_operation::Layer) -> &'static str {
    match layer {
        karakuri_operation::Layer::L1 => "L1",
        karakuri_operation::Layer::L2 => "L2",
        karakuri_operation::Layer::L3 => "L3",
        karakuri_operation::Layer::L4 => "L4",
        karakuri_operation::Layer::Field => "F",
        // Explicit arm matching host spelling; Sets currently have no L5 candidate nodes.
        karakuri_operation::Layer::L5 => "L5",
    }
}

/// Creates a rejected candidate (`Stage::NotCompiled`) with diagnostic messages.
pub(crate) fn refused_candidate(deck: usize, name: &str, said: &[&str]) -> Candidate {
    Candidate {
        deck,
        at: None,
        addr: String::new(),
        name: name.to_owned(),
        stage: Stage::NotCompiled,
        said: said.iter().map(|line| (*line).to_owned()).collect(),
    }
}

/// Counts shapes painted wholly contained inside `rect` on one frame.
pub(crate) fn shapes_inside(view: &mut View, panel: &mut Panel, rect: egui::Rect) -> usize {
    let ctx = drawn_once();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, panel));
    out.textures_delta.clear();
    out.shapes
        .into_iter()
        .filter(|clipped| {
            let bounds = clipped.shape.visual_bounding_rect();
            bounds.is_finite() && rect.contains_rect(bounds)
        })
        .count()
}
