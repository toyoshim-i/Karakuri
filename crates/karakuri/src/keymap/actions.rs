use super::*;

pub(crate) fn key_tab(ctx: &mut KeyCtx) -> bool {
    let step = match ctx.shift {
        true => -1,
        false => 1,
    };
    ctx.readout.view.tab(&ctx.readout.panel, step)
}

pub(crate) fn key_escape(ctx: &mut KeyCtx) -> bool {
    let moved = ctx.readout.view.focus_up(&ctx.readout.panel);
    if !moved {
        println!(
            "  esc: the address is already at the {} bay and there is no \
             level above it — press tab to move focus to another bay, or \
             close the window to quit",
            ctx.readout
                .view
                .focused(&ctx.readout.panel)
                .map_or("focused", |bay| bay.name)
        );
    }
    moved
}

pub(super) fn key_fold_enclosing(ctx: &mut KeyCtx) -> Option<Op> {
    ctx.readout
        .view
        .focused(&ctx.readout.panel)
        .and_then(|bay| ctx.readout.panel.layout().find(bay.name))
        .map(Op::FoldEnclosing)
}

pub(super) fn key_unfold_all(_ctx: &mut KeyCtx) -> Option<Op> {
    Some(Op::UnfoldAll)
}

pub(super) fn key_reset(_ctx: &mut KeyCtx) -> Option<Op> {
    Some(Op::Reset)
}

pub(super) fn key_save(ctx: &mut KeyCtx, gfx: &mut Gfx) {
    let deck = ctx.readout.view.selection();
    // Save current deck set; passing None generates a default timestamped ID.
    let acted = Acted::Emitted(Some(Operation::SaveSet { deck, id: None }));
    // Records the action emission before executing the set save operation.
    let repaint = App::performed(
        gfx,
        ctx.started,
        ctx.readout,
        ctx.recording.recorder(),
        &acted,
        Repaint::Never,
    )
    .repaint;
    ctx.keeping.save_set(
        &gfx.engine,
        ctx.store,
        Asked::Operator,
        usize::from(deck),
        None,
        None,
    );
    App::wants(gfx, ctx.egui_due, ctx.costs, repaint);
}

pub(super) fn key_tap_beat(ctx: &mut KeyCtx, gfx: &mut Gfx) {
    let acted = Acted::Emitted(Some(Operation::TapBeat));
    let repaint = App::performed(
        gfx,
        ctx.started,
        ctx.readout,
        ctx.recording.recorder(),
        &acted,
        Repaint::Never,
    )
    .repaint;
    App::wants(gfx, ctx.egui_due, ctx.costs, repaint);
}

/// The shared half of [`key_scale_grid_halve`] and [`key_scale_grid_double`] —
/// the two keys' one difference is `by`.
pub(super) fn key_scale_grid(ctx: &mut KeyCtx, gfx: &mut Gfx, by: GridScale) {
    let acted = Acted::Emitted(Some(Operation::ScaleGrid { by }));
    let repaint = App::performed(
        gfx,
        ctx.started,
        ctx.readout,
        ctx.recording.recorder(),
        &acted,
        Repaint::Never,
    )
    .repaint;
    App::wants(gfx, ctx.egui_due, ctx.costs, repaint);
}

pub(super) fn key_scale_grid_halve(ctx: &mut KeyCtx, gfx: &mut Gfx) {
    key_scale_grid(ctx, gfx, GridScale::Halve);
}

pub(super) fn key_scale_grid_double(ctx: &mut KeyCtx, gfx: &mut Gfx) {
    key_scale_grid(ctx, gfx, GridScale::Double);
}

pub(super) fn key_room(ctx: &mut KeyCtx, gfx: &mut Gfx) {
    ctx.readout.room();
    App::wants(gfx, ctx.egui_due, ctx.costs, Change::Room.repaint());
}

pub(super) fn key_toggle_mute(ctx: &mut KeyCtx, gfx: &mut Gfx) {
    let deck = ctx.readout.view.selection();
    let slot = karakuri_engine::DeckSlot(deck);
    let mute = !gfx.engine.deck.is_muted(slot);
    let acted = Acted::Emitted(Some(Operation::SetMute { deck, mute }));
    let repaint = App::performed(
        gfx,
        ctx.started,
        ctx.readout,
        ctx.recording.recorder(),
        &acted,
        Repaint::Never,
    )
    .repaint;
    App::wants(gfx, ctx.egui_due, ctx.costs, repaint);
}

pub(super) fn key_toggle_solo(ctx: &mut KeyCtx, gfx: &mut Gfx) {
    let deck = ctx.readout.view.selection();
    let slot = karakuri_engine::DeckSlot(deck);
    let solo = !gfx.engine.deck.is_soloed(slot);
    let acted = Acted::Emitted(Some(Operation::SetSolo { deck, solo }));
    let repaint = App::performed(
        gfx,
        ctx.started,
        ctx.readout,
        ctx.recording.recorder(),
        &acted,
        Repaint::Never,
    )
    .repaint;
    App::wants(gfx, ctx.egui_due, ctx.costs, repaint);
}

pub(super) fn key_clear_solo(ctx: &mut KeyCtx, gfx: &mut Gfx) {
    let acted = Acted::Emitted(Some(Operation::ClearSolo));
    let repaint = App::performed(
        gfx,
        ctx.started,
        ctx.readout,
        ctx.recording.recorder(),
        &acted,
        Repaint::Never,
    )
    .repaint;
    App::wants(gfx, ctx.egui_due, ctx.costs, repaint);
}
