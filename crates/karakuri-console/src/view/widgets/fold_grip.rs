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

/// How wide a grip is, which [`head_pills`] steps back by before it places the
/// first pill. A constant rather than a return value, because the pills are now
/// laid out where nothing is painting.
///
/// Public since the mark became a control ([`bay_grip`]): 5.5 is what says a
/// target grown the way a `.pill` grows would reach into the capsule beside it,
/// and that measurement is a test's rather than this module's.
pub const GRIP_W: f32 = GRIP_STEP * (GRIP_COLS - 1) as f32 + GRIP_R * 2.0;

/// The mock's `.grip`, `⋮⋮` — drawn rather than typed, because whether a
/// vertical ellipsis is in `egui`'s default face is a question with no good
/// answer and six dots is the same mark either way. Right-aligned to `right`;
/// its width is [`GRIP_W`], which is what the pills beside it step back by.
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

/// A fold, as a rectangle to press and the node it folds — the console's own
/// shape, reached from the panel instead of from `f`.
///
/// # It was two controls and it is one, because a pane stopped needing a shape
///
///
/// [ADR-0295](../../../../docs/adr/0295-the-grip-is-the-fold-and-a-panes-outer-edge-is-the-other-one.md)
/// gave *Fold a pane away* a second derivation here — `pane_edge`, a band
/// `GRAB` deep on the pane's outer edge with nothing drawn in it — and it was
/// never registered, because that band lay over the outer three pixels of every
/// row of the Library bay's list.
/// [ADR-0300](../../../../docs/adr/0300-a-pane-folds-by-dragging-its-boundary-out-and-comes-back-by-dragging-it-in.md)
/// replaced it with a drag: a pane's boundary pulled out past the pane's own
/// minimum closes it, and the divider it leaves behind at the window's edge is
/// what pulls it back. There is no band while the pane is open, nothing
/// overlaps a library row, and the control needs no derivation at all — a
/// boundary is `karakuri_console::input`'s rule 3, claimed before any control
/// is asked. So this type is one control now: the grip, which never had the
/// conflict.
///
/// The two are still one row's worth of operation apiece and one variant here —
/// [`Op::Fold`] of a node — and `tests/vocabulary.rs`'s `rows_of` gives it both
/// of *Fold a bay away* and *Fold a pane away*.
///
/// # There is no unfold on it, and that is the arrangement rather than a gap
///
/// [`Outputs::op`] and [`ProgramHead::op`] each choose between two operations,
/// because the thing they act on is still on screen when it is off. A folded
/// node has no rectangle, so this control is not drawn once its press has
/// landed: the grip goes with the head it is in. That is `Op`'s own sentence
/// about the pointer — *"a folded region has no rectangle, so the pointer could
/// never be over one, so `f` on the keyboard only ever folded"* — met by a
/// control instead of by a key, and the way back is `z` for the same reason it
/// is for `f`.
///
/// # A fold writes no session record
///
/// `karakuri-operation-record` answers `Silent::Surface` for all four fold
/// operations — *a surface's own state* — so nothing here routes through the
/// window's `written`, and there is no `Operation` for it to route as: this is
/// [`Op`] and the two unnamed splits are why (ADR-0204).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FoldGrip {
    /// The control: what a press has to land in.
    pub grip: Rect,
    /// The node a press folds.
    pub id: NodeId,
}

impl FoldGrip {
    /// What a press asks for, and there is one answer — see the type's own
    /// documentation for why this is not the two [`ProgramHead::op`] chooses
    /// between.
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
    // The bay itself, a column folded around it, or a solo somewhere else: one
    // question for every ancestor, which is [`program_head`]'s own guard.
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
    // A head clipped to a bay too short or too narrow to hold the target draws
    // no control rather than half of one — [`head_capsule`]'s own guard, one
    // capsule along.
    head.contains_rect(grip).then_some(FoldGrip { grip, id })
}
