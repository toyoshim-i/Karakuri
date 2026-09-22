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

/// One candidate: a node one build changed on a deck slot, not yet ruled on.
///
/// `L1:0` on every one of them unless a test says otherwise — which node it is
/// only matters where the operation a press asks for is being read, and those
/// tests build their own.
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
        // **Empty, because every verdict but one is about a build.** The row
        // that carries a sentence is `Stage::NotCompiled`'s, and
        // [`refused_candidate`] is the one that builds it.
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

/// The mock's `.addr` spelling, which the host writes and this file restates
/// because it is building the value the host would hand in.
///
/// Kept in step with `karakuri/src/main.rs`'s `layer_word` by hand, which is
/// what a fixture that restates a host's spelling costs: the two are one
/// spelling in two places, and a row addressed by one and drawn by the other
/// would be a row an operator cannot press back.
pub(crate) fn layer_word(layer: karakuri_operation::Layer) -> &'static str {
    match layer {
        karakuri_operation::Layer::L1 => "L1",
        karakuri_operation::Layer::L2 => "L2",
        karakuri_operation::Layer::L3 => "L3",
        karakuri_operation::Layer::L4 => "L4",
        karakuri_operation::Layer::Field => "F",
        // **Answered here and reached by nothing today.** A candidate is a node
        // of a *Set* that a build produced, and a Set holds no L5 node: a frame
        // effect runs in the master chain, which is one level out from every
        // Set, and the chain is still three fixed passes. So no Staging row is
        // ever addressed `L5:0` and this arm builds a value nothing asks for.
        //
        // **Named rather than left to a wildcard anyway**, on the terms the
        // match itself exists for: what this fixture is checking is that an
        // address a row is drawn with is an address a press can type back, and
        // a catch-all is how the fifth layer went wrong one kind ago
        // (`setfile::layer_from_ordinal`). `L5` and not a bare letter, matching
        // the host's own spelling above.
        karakuri_operation::Layer::L5 => "L5",
    }
}

/// One candidate the checker turned down, with what it said.
///
/// It names no node and cannot: nothing was built, so there is no list of nodes
/// to hold against the one before it (`view::Candidate::at`).
///
/// `karakuri_engine::swap::Refusal` carries one line per diagnostic, formatted
/// on the build worker; these are the shape those lines have — where, which
/// stage, and what.
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

/// Every shape the console paints wholly inside `rect`, on one frame.
///
/// `library.rs`'s helper, and `transport.rs` is where the reasoning is written
/// out: containment rather than intersection, so the panel's ground and the
/// card's drop shadow are not counted as things drawn in the bay.
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
