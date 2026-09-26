//! Pointer event translation and bay event dispatch methods.

use karakuri_console::egui;
pub(crate) use karakuri_operation::gate::{Class, Open};
use karakuri_operation::Output;

use super::*;

pub(crate) mod actions;
pub(crate) mod press;
pub(crate) mod types;
pub(crate) use types::*;

impl Readout {
    /// Routes a pointer event between the console panel and egui, returning claim and resulting action.
    pub(crate) fn pointer(&mut self, ctx: &egui::Context, event: Pointer) -> (Claim, Acted) {
        let at = match event {
            Pointer::Moved(p) => p,
            _ => self.panel.cursor(),
        };
        // Evaluate claim before acting to preserve drag state consistency across events.
        let mut claim = claim(&mut self.panel, ctx, &self.view, at);
        let mut did = Acted::Nothing;
        if matches!(event, Pointer::Down | Pointer::DoubleDown)
            && !self.panel.dragging()
            && !self.view.has_modal_overlay()
        {
            if let Some(bay) = karakuri_console::focus::bay_at(self.panel.layout(), at) {
                self.view.focus_bay(&self.panel, bay.name);
                if bay.name == "prompt" {
                    self.view.prompt.set_captured(true);
                } else {
                    self.view.prompt.set_captured(false);
                }
            } else {
                self.view.prompt.set_captured(false);
            }
        }
        if matches!(event, Pointer::DoubleDown)
            && !self.panel.dragging()
            && !self.view.has_modal_overlay()
        {
            self.panel.solve();
            if let Some(bay) = karakuri_console::focus::bay_head_at(self.panel.layout(), at) {
                let is_pill = program_head(ctx, self.panel.layout(), self.view.opening)
                    .is_some_and(|head| head.hit(at))
                    || Class::ALL.iter().any(|class| {
                        mcp_pill(ctx, self.panel.layout(), *class, self.view.opening)
                            .is_some_and(|pill| pill.hit(at))
                    })
                    || bay_grip(self.panel.layout(), bay.name).is_some_and(|grip| grip.hit(at));
                if !is_pill {
                    if let karakuri_console::focus::Asked::Panel(op) =
                        karakuri_console::focus::fold(&self.panel, bay)
                    {
                        return (Claim::Panel, Acted::Operated(self.folded(op)));
                    }
                }
            }
            return (Claim::Panel, Acted::Nothing);
        }
        match (event, claim) {
            // Update pointer coordinates and emit operation if dragging an active fader.
            (Pointer::Moved(p), _) => {
                self.panel.set_cursor(p);
                let fading = matches!(self.panel.in_hand(), Some(InHand::Fader));
                let operation = self.moved(p);
                if fading {
                    did = Acted::Emitted(operation);
                }
            }
            // Dispatches panel-claimed button-down to controls and focused bay handlers.
            (Pointer::Down, Claim::Panel) => {
                self.panel.solve();
                // Decomposed pointer press dispatch across modal overlays and individual bays.
                // Each bay handler returns `Some(Acted)` if it claims the press, or `None` to pass through.
                let did = self
                    .dispatch_modal_press(ctx, at)
                    .or_else(|| self.dispatch_transport_press(ctx, at))
                    .or_else(|| self.dispatch_inspector_press(ctx, at))
                    .or_else(|| self.dispatch_head_press(ctx, at))
                    .or_else(|| self.dispatch_library_press(ctx, at))
                    .or_else(|| self.dispatch_staging_press(ctx, at))
                    .or_else(|| self.dispatch_transition_press(ctx, at))
                    .or_else(|| self.dispatch_sequencer_press(ctx, at))
                    .unwrap_or_else(|| self.dispatch_mixer_and_master_press(ctx, at));
                return (claim, did);
            }
            // Resolve drop target for carried items across mixer, program bay, and master chain (ADR-0273).
            (Pointer::Up, Claim::Panel) => {
                let onto = match self.panel.in_hand() {
                    Some(InHand::Carrying) => mixer_bay(ctx, self.panel.layout(), &self.view.mixer)
                        .as_ref()
                        .and_then(|bay| bay.dropped(at))
                        .or_else(|| {
                            program_bay(self.panel.layout(), self.view.canvas)
                                .as_ref()
                                .and_then(|cells| cells.dropped(at, self.view.mixer.len()))
                        })
                        .map(Landing::Deck)
                        // Drops onto master chain if carrying a valid L5 procedure (ADR-0273).
                        .or_else(|| {
                            let adding = self.view.chain_choices();
                            master_row(
                                ctx,
                                self.panel.layout(),
                                self.view.master_out,
                                self.view.master_chain.as_ref(),
                                &adding,
                            )
                            .as_ref()
                            .and_then(|row| row.dropped(at))?;
                            let carried = self.panel.carried()?;
                            Some(Landing::Chain(self.view.chain_landing(carried)))
                        }),
                    _ => None,
                };
                did = self.released(onto);
            }
            // Scroll inspector panes or library list based on target region (ADR-0312).
            (Pointer::Wheel(by), _) => {
                if let Some(turned) = wheeled(&mut self.panel, &self.view, at) {
                    let moved = match turned {
                        Turned::Pane(pane) => self.view.scroll_by(pane, by),
                        Turned::Library => self.view.scroll_library_by(by),
                    };
                    if moved {
                        did = Acted::Pointed;
                    }
                    claim = Claim::Panel;
                }
            }
            // Secondary button opens context menu on library rows or dismisses open menus.
            (Pointer::Secondary, Claim::Panel) => {
                self.panel.solve();
                let picked = library_bay(
                    self.panel.layout(),
                    &self.view.scopes,
                    &self.view.library,
                    self.view.opened(),
                    self.view.pointed(),
                    self.view.library_scroll(),
                )
                .and_then(|bay| {
                    bay.menu_ask(
                        ctx,
                        view::to_egui(self.panel.layout().viewport()),
                        self.view.menued(),
                        self.view.rows(),
                        at,
                    )
                });
                if let Some(ask) = picked {
                    did = self.menued(ask);
                }
            }
            (Pointer::Down | Pointer::DoubleDown | Pointer::Up | Pointer::Secondary, _) => {}
        }
        (claim, did)
    }
}
