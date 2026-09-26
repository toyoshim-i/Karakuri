use super::super::*;

// ---------------------------------------------------------------------------
// Fold grip and pane dividers
// ---------------------------------------------------------------------------

/// The mock's `.grip`: two columns of three dots.
const GRIP_COLS: usize = 2;
const GRIP_ROWS: usize = 3;
/// One dot's radius.
const GRIP_R: f32 = 1.0;
/// Centre to centre, both ways.
const GRIP_STEP: f32 = 3.5;

/// Width of a fold grip, which [`head_pills`] steps back by before placing the first pill.
pub const GRIP_W: f32 = GRIP_STEP * (GRIP_COLS - 1) as f32 + GRIP_R * 2.0;

/// Paints the mock's `.grip` (`⋮⋮`) as six dots right-aligned to `right`.
pub(crate) fn grip_dots(ui: &Ui, pal: &Palette, right: Pos2) {
    let h = GRIP_STEP * (GRIP_ROWS - 1) as f32;
    let painter = ui.painter();
    for c in 0..GRIP_COLS {
        for r in 0..GRIP_ROWS {
            painter.circle_filled(
                Pos2::new(
                    right.x - GRIP_W + GRIP_R + c as f32 * GRIP_STEP,
                    right.y - h * 0.5 + r as f32 * GRIP_STEP,
                ),
                GRIP_R,
                pal.faint,
            );
        }
    }
}

/// The mock's `.divider-v` between a bay's subdivisions: a `--c-hair` capsule
/// in the gap, below the head.
///
/// A no-op for every bay that is a leaf, which is six of the seven.
pub(crate) fn pane_dividers(ui: &Ui, pal: &Palette, panel: &Panel, id: NodeId, rect: Rect) {
    let layout = panel.layout();
    let Some(axis) = layout.axis(id) else {
        return;
    };
    let top = (rect.min.y + size::HEAD_H).min(rect.max.y);
    for (index, _) in layout.placed_children(id).enumerate().skip(1) {
        let Some(gap) = layout.boundary(id, index - 1) else {
            continue;
        };
        let gap = to_egui(gap);
        let bar = match axis {
            Axis::Row => {
                Rect::from_min_max(Pos2::new(gap.min.x, top), Pos2::new(gap.max.x, rect.max.y))
            }
            Axis::Column => Rect::from_min_max(
                Pos2::new(rect.min.x, gap.min.y.max(top)),
                Pos2::new(rect.max.x, gap.max.y.max(top)),
            ),
        };
        if bar.width() > 0.0 && bar.height() > 0.0 {
            ui.painter().rect_filled(
                bar,
                CornerRadius::same((size::PANE_DIVIDER * 0.5) as u8),
                pal.hair,
            );
            // Draw the mock's vertical ellipsis grip decoration (`⋮`) in the center.
            let center = bar.center();
            let dot_r = 1.0;
            let dot_step = 3.5;
            for r in -1..=1 {
                ui.painter().circle_filled(
                    Pos2::new(center.x, center.y + (r as f32) * dot_step),
                    dot_r,
                    pal.faint,
                );
            }
        }
    }
}

/// A fold control: a clickable grip rectangle that folds node `id`.
///
/// Pane drag folding replaces legacy edge controls per [ADR-0295](../../../../docs/adr/0295-the-grip-is-the-fold-and-a-panes-outer-edge-is-the-other-one.md), [ADR-0300](../../../../docs/adr/0300-a-pane-folds-by-dragging-its-boundary-out-and-comes-back-by-dragging-it-in.md), and ADR-0204.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FoldGrip {
    /// The control: what a press has to land in.
    pub grip: Rect,
    /// The node a press folds.
    pub id: NodeId,
}

impl FoldGrip {
    /// Returns the fold operation for this grip's target node.
    pub fn op(&self) -> Op {
        Op::Fold(self.id)
    }

    /// Whether `p` is on the control.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.grip.contains(Pos2::new(p.x, p.y))
    }
}

/// Returns the hit-test bounding box for the fold grip in a bay's header, if present.
///
/// See ADR-0295 for details on grip hit regions and folding mechanics.
pub fn bay_grip(layout: &karakuri_layout::Layout, name: &str) -> Option<FoldGrip> {
    if !head_grip(region(name)?.kind) {
        return None;
    }
    let id = layout.find(name)?;
    // Bay must be visible.
    if !layout.visible(id) {
        return None;
    }
    let head = head_box(to_egui(layout.rect(id)));
    let mid = head.center().y;
    let right = head.max.x - size::HEAD_PAD_X;
    let grip = Rect::from_min_max(
        Pos2::new(right - GRIP_W - size::PILL_GAP, mid - size::PILL_H * 0.5),
        Pos2::new(right, mid + size::PILL_H * 0.5),
    );
    // Omit control if head cannot fully enclose the grip bounding box.
    head.contains_rect(grip).then_some(FoldGrip { grip, id })
}
