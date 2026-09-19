//! Handling for projector secondary window events.

use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use karakuri_console::repaint::Change;
use karakuri_operation::Output;

use super::super::*;
use crate::app::operations::routed;

impl App {
    pub(crate) fn handle_projector_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: WindowId,
        event: &WindowEvent,
    ) -> bool {
        let Some(gfx) = self.gfx.as_mut() else {
            return false;
        };
        if !gfx.projector.as_ref().is_some_and(|p| p.window.id() == id) {
            return false;
        }
        match event {
            WindowEvent::Resized(size) => {
                let at = (size.width.max(1), size.height.max(1));
                if let Some(projector) = gfx.projector.as_mut() {
                    projector.sink.resize(&gfx.gpu.device, at.0, at.1);
                    projector.size = at;
                    projector.window.request_redraw();
                }
                self.costs.owes();
                gfx.window.request_redraw();
                true
            }
            WindowEvent::CloseRequested => {
                if let Some(line) = routed(gfx, event_loop, Output::Projector(0), false) {
                    println!("{line}");
                }
                self.readout.view.projector = false;
                App::wants(
                    gfx,
                    &mut self.egui_due,
                    &mut self.costs,
                    Change::Viewport.repaint(),
                );
                true
            }
            WindowEvent::RedrawRequested => {
                // Fall through to the common frame composition.
                // When the projector is fullscreen or maximized, the console
                // window may be occluded or in a different Space, so the projector
                // window drives the redraw loop.
                false
            }
            _ => true,
        }
    }
}
