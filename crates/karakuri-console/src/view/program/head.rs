use super::*;

/// Layout state for the Program bay header `solo` pill control.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgramHead {
    /// The control: the `solo` capsule, which is what a press has to land in.
    /// [`head_pills`]'s answer for it, so it is the rectangle [`bay_head`] painted.
    pub solo: Rect,
    /// The node it solos: the picture, `program-view`.
    pub id: NodeId,
    /// Whether anything is soloed -- `layout.is_soloed()`, read here.
    pub soloed: bool,
}

impl ProgramHead {
    /// What a press on the pill asks for. See the type's own documentation for why
    /// it is two operations rather than one that toggles.
    pub fn op(&self) -> Op {
        match self.soloed {
            true => Op::Unsolo,
            false => Op::Solo(self.id),
        }
    }

    /// Whether `p` is on the control.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.solo.contains(Pos2::new(p.x, p.y))
    }
}

/// Derives the Program bay's head controls (`solo` pill and target node ID).
///
/// Returns `None` if the bay is folded, absent, or too short to contain the head.
pub fn program_head(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    open: Open,
) -> Option<ProgramHead> {
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // [`outputs`]: a press before the first frame is a press on a control that
    // has never been drawn.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let bay = layout.find("program")?;
    // The bay itself, its column folded around it, or a solo somewhere else:
    // one question for every ancestor, which is [`program_bay`]'s own guard.
    if !layout.visible(bay) {
        return None;
    }
    let id = layout.find("program-view")?;
    let head = head_of(region("program")?)?;
    // A head clipped to a bay shorter than 27 has nowhere to put a 16.5
    // capsule, and a capsule half out of the head is not one to press --
    // [`head_capsule`]'s own guard, which is why it is not repeated here.
    let solo = head_capsule(ctx, to_egui(layout.rect(bay)), &head, open, SOLO_PILL)?;
    Some(ProgramHead {
        solo,
        id,
        soloed: layout.is_soloed(),
    })
}
