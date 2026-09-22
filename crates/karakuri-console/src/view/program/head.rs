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

/// The Program bay's head, derived: the one pill in it, and the node it acts
/// on.
///
/// `None` where there is no pill to press -- before the first frame, with the
/// bay folded or off a solo somewhere else, or in a bay too short to hold its
/// own head. That is [`outputs`]'s rule stated on a capsule instead of on a
/// chip: a rectangle with nothing in it is not something to paint or to click.
///
/// The pill is found by name rather than by position. [`REGIONS`] is where a
/// bay's controls are listed, and this reads [`SOLO_PILL`] out of the Program
/// bay's entry -- so a second pill added to that head moves this one along and
/// nothing here has to be told.
///
/// A second pill was duly added and this capsule duly moved. The class pill
/// ([`mcp_pill`]) sits to `solo`'s right and is not the same width in its two
/// states, which is why this now takes an opening: [`head_capsule`] lays the
/// whole head out under that opening and hands back the one capsule asked for,
/// so the two pills cannot be laid out against two different states.
///
/// `layout` must be solved: [`karakuri_layout::Layout::rect`] refuses to answer
/// from a dirty one. `ctx` is asked for the type, because a `.pill` is as wide
/// as the word in it.
///
/// # What it costs the operator, and it is 0.75 of a pixel
///
/// This is one of the first two controls on the console that do not clear every
/// boundary's grab — the four deck preview cells under it are the other, and
/// [`ProgramBay::preview`] carries theirs. The number is worth having in front
/// of you rather than in a test alone. A bay head is [`size::HEAD_H`] = 27 and
/// a `.pill` is [`size::PILL_H`] = 16.5, centred, so there is (27 - 16.5) / 2 =
/// 5.25 of head above the capsule -- against a [`crate::panel::GRAB`] of 6. The
/// Program bay is the first child of the centre column, so its top edge is the
/// body row's, and the boundary between the transport row and the body grabs
/// six pixels past it.
///
/// So the top 0.75 of the capsule is the boundary's and the other 15.75 is the
/// panel's. [`crate::input`]'s rule 3 is what decides that and it decides it
/// the same way every time: the boundary gets first refusal, there is no case
/// where both think they are dragging, and the hazard that rule was written for
/// does not arise. What is lost is the sliver, and `tests/solo_pill.rs` is what
/// states the number and fails if it grows.
///
/// The three things that could change it are all somebody else's: the head's
/// height and the pill's box are `docs/manual/console.html`'s, and `GRAB` is
/// the rule's. This function draws the control where the page puts it.
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
