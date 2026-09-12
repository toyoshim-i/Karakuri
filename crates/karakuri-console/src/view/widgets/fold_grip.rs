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

/// The grip in a bay head, derived — the mark `docs/manual/console.html` draws
/// in four of the seven heads this console has, and the panel's route into
/// *Fold a bay away*.
///
/// `None` where there is nothing to press: a region this console draws no head
/// for, a head with no grip in it, a bay that is folded or off a solo somewhere
/// else, or a bay clipped shorter or narrower than the target.
///
/// # The mark is the control, and it is not given a second shape
///
/// The page's lede says what the mark means — *"The grip in a bay's head says
/// whose size you are setting: a bay with one is yours to size, and a bay
/// without one is the height of what is in it"* — and the operations page puts
/// *Fold a bay away* at the bay head. A fold is the far end of setting that
/// height, so the reading a press takes is the reading the mark already
/// carries. A *reading* which is a control does not get a second shape invented
/// for it
/// ([ADR-0291](../../../../docs/adr/0291-the-tempo-figure-is-the-track-and-the-band-is-a-guard-on-the-hand.md)),
/// so nothing new is drawn here: [`bay_head`] paints the same six dots it
/// painted before this function existed.
///
/// What that costs is the three heads the mock draws no grip in — the Mixer,
/// the Staging lane and the Sequencer, and the two headless rows beside them —
/// which have no pointer route into this row and keep `f`. Drawing a grip in
/// every head would buy them one and would be an edit to the page rather than
/// to this function: the mark would stop saying which side of a divider the
/// number belongs to, which is the whole of what the lede says it is for.
/// [ADR-0295](../../../../docs/adr/0295-the-grip-is-the-fold-and-a-panes-outer-edge-is-the-other-one.md)
/// is where that is taken.
///
/// # Where the rectangle is, and the 0.75 of a pixel it shares with `solo`
///
/// The mark is [`GRIP_W`] = 5.5 wide and 9 tall, which is not a target a hand
/// finds — the same sentence [`GRAB`] is written under, and the same one
/// [`LookRow::grip`] answers with a band. So the target is the head's own
/// reservation for the mark, grown to a line's height: [`head_pills`] steps
/// `GRIP_W + PILL_GAP` back from [`size::HEAD_PAD_X`] before it places a
/// capsule, so that 10.5 is the strip of head no other control can ever be
/// drawn in, and this is it — 10.5 x [`size::PILL_H`], hard against the head's
/// right-hand padding and centred on the head's mid-line, which is the box
/// every other capsule in this head is placed in.
///
/// The reservation rather than a pill's padding, and the capsule beside it is
/// what settles it: a `.pill` is its content inside `padding: 0 8px`, and 5.5 +
/// 16 = 21.5 would reach 6 pixels into the capsule [`head_pills`] places one
/// [`size::PILL_GAP`] to the left of the mark. The reservation ends exactly
/// where that capsule ends, so the two abut and never overlap — two rectangles
/// sharing an edge, which is what two rows of the Library bay's list already
/// are, and the capsule is asked first in both the claim and the caller.
///
/// What it takes that is not the mark is the gap, and the gap is what the head
/// keeps between the mark and the capsule so that a hand aiming at one does not
/// land on the other. Giving it to the fold is what makes the mark findable at
/// all; giving it to nobody would be a control 5.5 wide, which is the thing
/// this paragraph starts by refusing.
///
/// Sideways it clears every boundary: [`size::HEAD_PAD_X`] is 10 against a
/// [`GRAB`] of 6, and that padding is the page's. Down the head it is
/// `program_head`'s own 0.75 of a pixel, arrived at the same way and for the
/// same reason: a head is [`size::HEAD_H`] = 27 and the target is 16.5,
/// centred, so 5.25 of head sits above it against a `GRAB` of 6, and a bay
/// whose top edge is a boundary lends that boundary the top 0.75 of the target.
/// Rule 3 gives the boundary first refusal and there is no case where both
/// think they are dragging; `tests/fold_grip.rs` measures it by asking
/// [`karakuri_layout::Layout::hit`] at the target's own corners rather than by
/// doing the arithmetic again.
///
/// No `egui` and no first frame. The grip is six dots at a fixed width, so
/// nothing here is as wide as a word — this is the one control in a bay head
/// that costs no galley and exists on the frame before anything has been laid
/// out. `layout` must be solved.
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
