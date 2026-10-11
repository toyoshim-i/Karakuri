//! Document Picture-in-Picture floating preview monitor for Karakuri Web.

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use winit::event_loop::EventLoopProxy;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = __karakuri)]
    pub fn is_pip_available() -> bool;

    #[wasm_bindgen(js_namespace = __karakuri)]
    fn open_pip_window(width: u32, height: u32) -> js_sys::Promise;
}

/// Document Picture-in-Picture window events and actions dispatched across thread/event boundaries.
pub enum PipAction {
    Attach {
        surface: wgpu::Surface<'static>,
        size: (u32, u32),
        window: web_sys::Window,
    },
    Detach,
    Resize((u32, u32)),
}

/// Sets up WebGPU surface and event handlers on the floating PiP window's canvas.
pub fn setup_pip_canvas(
    pip_window: &web_sys::Window,
    proxy: EventLoopProxy<()>,
    pending_action: &Rc<RefCell<Option<PipAction>>>,
) -> Result<(), String> {
    let doc = pip_window.document().ok_or("No document in PiP window")?;
    let canvas_el = doc
        .get_element_by_id("karakuri-pip-canvas")
        .ok_or("Canvas element #karakuri-pip-canvas not found in PiP window")?;
    let canvas: web_sys::HtmlCanvasElement = canvas_el.unchecked_into();

    let dpr = pip_window.device_pixel_ratio();
    let logical_w = pip_window
        .inner_width()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(640.0);
    let logical_h = pip_window
        .inner_height()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(360.0);
    let phys_w = ((logical_w * dpr) as u32).max(1);
    let phys_h = ((logical_h * dpr) as u32).max(1);
    canvas.set_width(phys_w);
    canvas.set_height(phys_h);

    // Create WebGPU surface directly on PiP canvas using shared Gpu::instance()
    let instance = karakuri_engine::Gpu::instance();
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
        .map_err(|e| format!("Failed to create surface on PiP canvas: {e:?}"))?;

    *pending_action.borrow_mut() = Some(PipAction::Attach {
        surface,
        size: (phys_w, phys_h),
        window: pip_window.clone(),
    });
    let _ = proxy.send_event(());

    // 1. Resize listener: adjust canvas buffer and sink dimensions on PiP window resize
    let pip_win_for_resize = pip_window.clone();
    let canvas_for_resize = canvas.clone();
    let pending_for_resize = pending_action.clone();
    let proxy_for_resize = proxy.clone();
    let on_resize = Closure::<dyn FnMut()>::wrap(Box::new(move || {
        let dpr = pip_win_for_resize.device_pixel_ratio();
        let lw = pip_win_for_resize
            .inner_width()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(640.0);
        let lh = pip_win_for_resize
            .inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(360.0);
        let pw = ((lw * dpr) as u32).max(1);
        let ph = ((lh * dpr) as u32).max(1);
        canvas_for_resize.set_width(pw);
        canvas_for_resize.set_height(ph);
        *pending_for_resize.borrow_mut() = Some(PipAction::Resize((pw, ph)));
        let _ = proxy_for_resize.send_event(());
    }));
    let _ =
        pip_window.add_event_listener_with_callback("resize", on_resize.as_ref().unchecked_ref());
    on_resize.forget();

    // 2. Closure listener: detect when PiP window is dismissed or closed by the user
    let pending_for_close = pending_action.clone();
    let proxy_for_close = proxy.clone();
    let on_close = Closure::<dyn FnMut()>::wrap(Box::new(move || {
        *pending_for_close.borrow_mut() = Some(PipAction::Detach);
        let _ = proxy_for_close.send_event(());
    }));
    let _ =
        pip_window.add_event_listener_with_callback("pagehide", on_close.as_ref().unchecked_ref());
    on_close.forget();

    log::info!("Karakuri Web: Document PiP floating window attached ({phys_w}x{phys_h})");
    Ok(())
}

/// Requests the Document Picture-in-Picture window and configures WebGPU rendering.
pub fn open_pip(proxy: EventLoopProxy<()>, pending_action: Rc<RefCell<Option<PipAction>>>) {
    spawn_local(async move {
        let promise = open_pip_window(640, 360);
        match wasm_bindgen_futures::JsFuture::from(promise).await {
            Ok(val) => {
                let win: web_sys::Window = val.unchecked_into();
                if let Err(e) = setup_pip_canvas(&win, proxy.clone(), &pending_action) {
                    log::warn!("Karakuri Web: Failed to setup PiP canvas: {e}");
                    *pending_action.borrow_mut() = Some(PipAction::Detach);
                    let _ = proxy.send_event(());
                }
            }
            Err(e) => {
                log::warn!(
                    "Karakuri Web: Failed to open Document Picture-in-Picture window: {e:?}"
                );
                *pending_action.borrow_mut() = Some(PipAction::Detach);
                let _ = proxy.send_event(());
            }
        }
    });
}

/// Handles PiP output toggle from the Outputs row.
pub fn handle_route_hook(
    on: bool,
    proxy: &EventLoopProxy<()>,
    pending: &Rc<RefCell<Option<PipAction>>>,
) {
    if on {
        open_pip(proxy.clone(), pending.clone());
    } else {
        *pending.borrow_mut() = Some(PipAction::Detach);
        let _ = proxy.send_event(());
    }
}

/// Applies a pending PiP action to the App state and manages the PiP window handle.
pub fn handle_pip_action(
    app: &mut karakuri::App,
    pip_window: &mut Option<web_sys::Window>,
    action: PipAction,
) {
    match action {
        PipAction::Attach {
            surface,
            size,
            window,
        } => {
            app.attach_pip_surface(surface, size);
            *pip_window = Some(window);
        }
        PipAction::Detach => {
            app.detach_pip_surface();
            if let Some(win) = pip_window.take() {
                if !win.closed().unwrap_or(true) {
                    let _ = win.close();
                }
            }
        }
        PipAction::Resize(size) => {
            app.resize_pip_surface(size);
        }
    }
}
