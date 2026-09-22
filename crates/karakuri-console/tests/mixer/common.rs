#![allow(unused_imports, dead_code)]

pub use super::common::{drawn_once, id_of, near, rect_of, showing, solved, PLAUSIBLE, SMALLEST};
pub use karakuri_console::input::{claim, Claim};
pub use karakuri_console::panel::Panel;
pub use karakuri_console::room::{size, Room};
pub use karakuri_console::view::{mixer, Level, Mask, Mixer, Strip, StripBox, Tally, View, DECKS};
pub use karakuri_layout::{Point, Rect};
pub use karakuri_operation::{BlendMode, Operation};

/// The mock's own first strip, as values: `drift_night` live, a trim at 0.72,
/// the fader at 1.00, `add` over no mask, and a meter reading.
///
/// The numbers are the mock's percentages read as the values behind them —
/// `.trim`'s `width: 72%`, `.strip-num`'s `1.00`, `.vmeter b`'s `74%` and
/// `.vmeter u`'s `82%`.
pub fn mock() -> Strip {
    Strip {
        name: "drift_night".to_owned(),
        tally: Tally::Live,
        // Settled: the request and the effective residency agree, so nothing
        // in these strips is pending and nothing rolls. What a strip whose
        // two halves disagree draws is `parked.rs`.
        requested: Tally::Live,
        gain: 0.72,
        gain_to: None,
        opacity: 1.0,
        opacity_to: None,
        blend: BlendMode::Add,
        mask: Mask::None,
        mask_angle: 0.0,
        level: Some(Level {
            mean: 0.74,
            peak: 0.82,
        }),
        is_muted: false,
        is_soloed: false,
    }
}

/// The mock's second and third, which are the other two tallies, the other
/// mask, a different blend and — on the third — no reading at all.
pub fn mock_strips() -> Vec<Strip> {
    vec![
        mock(),
        Strip {
            name: "lattice_veil".to_owned(),
            tally: Tally::Priming,
            requested: Tally::Priming,
            gain: 0.44,
            gain_to: None,
            opacity: 0.3,
            opacity_to: None,
            blend: BlendMode::Over,
            mask: Mask::Linear,
            mask_angle: 0.0,
            level: Some(Level {
                mean: 0.12,
                peak: 0.12,
            }),
            is_muted: false,
            is_soloed: false,
        },
        Strip {
            name: "glass_shell".to_owned(),
            tally: Tally::Allocated,
            requested: Tally::Allocated,
            gain: 0.0,
            gain_to: None,
            opacity: 0.0,
            opacity_to: None,
            blend: BlendMode::Add,
            mask: Mask::Radial,
            mask_angle: 0.0,
            level: None,
            is_muted: false,
            is_soloed: false,
        },
    ]
}

/// A panel at a viewport, solved, with a context that has drawn once — the pair
/// every test here starts from, and `transport.rs`'s own opening.
pub fn console(viewport: Rect) -> (Panel, egui::Context) {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    (panel, drawn_once())
}

/// The bay, laid out with `strips`.
pub fn bay<'a>(panel: &Panel, ctx: &egui::Context, strips: &'a [Strip]) -> Mixer<'a> {
    mixer(ctx, panel.layout(), strips).expect("the mixer bay draws its strips")
}

/// The `.mixer-strips` row inside the mixer's region, worked out here from the
/// mock's boxes rather than asked of the crate — so that a row derived wrong is
/// a row in the wrong place rather than two functions agreeing.
pub fn strips_row(region: Rect) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(
            region.x + size::STRIPS_PAD,
            region.y + size::HEAD_H + size::STRIPS_PAD,
        ),
        egui::vec2(region.w - size::STRIPS_PAD * 2.0, size::STRIP_H),
    )
}

/// A `karakuri_layout` point, from `egui`'s. Named for what it makes rather
/// than for where it is, because `at` is a strip's box everywhere below.
pub fn point(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

/// Collects all visual shapes rendered inside a specified rectangle.
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

/// How many `.strip` wells were painted inside `rect`: a filled rectangle a
/// strip's own size.
///
/// Filled and unstroked, which the prose above always said and the filter did
/// not. The deck selection's ring is `.strip.focus`'s `box-shadow: inset 0 0 0
/// 2px` and is drawn at exactly the strip's rectangle, so a scan that took any
/// rect of that size counted the selected strip twice and read a one-slot deck
/// as two. This is the reach corrected and not the claim: one well per strip is
/// still the whole assertion.
pub fn wells(shapes: &[egui::Shape], width: f32) -> usize {
    shapes
        .iter()
        .filter(|shape| match shape {
            egui::Shape::Rect(at) => {
                near(at.rect.height(), size::STRIP_H)
                    && near(at.rect.width(), width)
                    && at.fill != egui::Color32::TRANSPARENT
                    && at.stroke.width == 0.0
            }
            _ => false,
        })
        .count()
}

/// Renders and collects the drawn shapes and bounding box for a given strip.
pub fn strip_shapes(strips: Vec<Strip>, index: usize) -> (StripBox, Vec<egui::Shape>) {
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let ctx = drawn_once();
    let at = mixer(&ctx, panel.layout(), &strips)
        .expect("the mixer bay draws its strips")
        .strip(index);
    let mut view = View::new(Room::Day);
    view.mixer = strips;
    // The whole strip and half the gap around it, which is what `strip_into`
    // clips to: a knob is meant to stand a little proud of its track.
    let shapes = shapes_inside(
        &mut view,
        &mut panel,
        at.rect.expand(size::STRIP_GAP * 0.5 + 1.0),
    );
    (at, shapes)
}

/// The galleys painted inside `rect`.
pub fn galleys(shapes: &[egui::Shape], rect: egui::Rect) -> Vec<std::sync::Arc<egui::Galley>> {
    shapes
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Text(text) if rect.contains(text.pos) => Some(text.galley.clone()),
            _ => None,
        })
        .collect()
}

/// The words painted inside `rect`.
pub fn words(shapes: &[egui::Shape], rect: egui::Rect) -> Vec<String> {
    galleys(shapes, rect)
        .iter()
        .map(|galley| galley.text().to_owned())
        .collect()
}

/// The filled circles painted inside `rect`, as their radii.
pub fn discs(shapes: &[egui::Shape], rect: egui::Rect) -> Vec<f32> {
    shapes
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Circle(circle) if rect.contains(circle.center) && circle.fill.a() > 0 => {
                Some(circle.radius)
            }
            _ => None,
        })
        .collect()
}

/// Finds indices of strips that have a selection focus ring drawn around them.
pub fn ringed(shapes: &[egui::Shape], at: &Mixer) -> Vec<usize> {
    (0..at.count())
        .filter(|index| {
            let rect = at.strip(*index).rect;
            shapes.iter().any(|shape| match shape {
                egui::Shape::Rect(r) => {
                    near(r.stroke.width, size::STRIP_FOCUS_RING)
                        && near(r.rect.width(), rect.width())
                        && near(r.rect.min.x, rect.min.x)
                }
                _ => false,
            })
        })
        .collect()
}
