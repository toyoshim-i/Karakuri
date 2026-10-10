//! Auxiliary popout window management for real-time projector display output (ADR-0358).

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use winit::event_loop::EventLoopProxy;

/// Projector window events and actions dispatched across thread/event boundaries.
pub enum ProjectorAction {
    Attach {
        surface: wgpu::Surface<'static>,
        size: (u32, u32),
        window: web_sys::Window,
    },
    Detach,
    Resize((u32, u32)),
}

/// Sets up WebGPU surface and event handlers on the popout window's projector canvas.
pub fn setup_projector_canvas(
    popout: &web_sys::Window,
    proxy: EventLoopProxy<()>,
    pending_action: &std::rc::Rc<std::cell::RefCell<Option<ProjectorAction>>>,
) -> Result<(), String> {
    let doc = popout.document().ok_or("No document in projector window")?;
    let canvas_el = doc
        .get_element_by_id("karakuri-projector-canvas")
        .ok_or("Canvas element #karakuri-projector-canvas not found in projector window")?;
    let canvas: web_sys::HtmlCanvasElement = canvas_el.unchecked_into();

    let dpr = popout.device_pixel_ratio();
    let logical_w = popout
        .inner_width()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(1920.0);
    let logical_h = popout
        .inner_height()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(1080.0);
    let phys_w = ((logical_w * dpr) as u32).max(1);
    let phys_h = ((logical_h * dpr) as u32).max(1);
    canvas.set_width(phys_w);
    canvas.set_height(phys_h);

    // Create WebGPU surface directly on child canvas using parent Gpu::instance()
    let instance = karakuri_engine::Gpu::instance();
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
        .map_err(|e| format!("Failed to create surface on projector canvas: {e:?}"))?;

    *pending_action.borrow_mut() = Some(ProjectorAction::Attach {
        surface,
        size: (phys_w, phys_h),
        window: popout.clone(),
    });
    let _ = proxy.send_event(());

    // 1. Fullscreen handlers: Double-click or 'F' key toggles fullscreen (ADR-0358)
    let popout_doc_clone = doc.clone();
    let on_dblclick = Closure::<dyn FnMut()>::wrap(Box::new(move || {
        if let Some(el) = popout_doc_clone.document_element() {
            if popout_doc_clone.fullscreen_element().is_some() {
                popout_doc_clone.exit_fullscreen();
            } else {
                let _ = el.request_fullscreen();
            }
        }
    }));
    let _ =
        canvas.add_event_listener_with_callback("dblclick", on_dblclick.as_ref().unchecked_ref());
    on_dblclick.forget();

    let popout_doc_clone2 = doc.clone();
    let on_keydown = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::wrap(Box::new(
        move |e: web_sys::KeyboardEvent| {
            if e.key().eq_ignore_ascii_case("f") {
                if let Some(el) = popout_doc_clone2.document_element() {
                    if popout_doc_clone2.fullscreen_element().is_some() {
                        popout_doc_clone2.exit_fullscreen();
                    } else {
                        let _ = el.request_fullscreen();
                    }
                }
            }
        },
    ));
    let _ = popout.add_event_listener_with_callback("keydown", on_keydown.as_ref().unchecked_ref());
    on_keydown.forget();

    // 2. Auto-hide mouse cursor after 2s of inactivity
    if let Some(body) = doc.body() {
        let body_clone = body.clone();
        let popout_clone_timer = popout.clone();
        let timer_id = std::rc::Rc::new(std::cell::Cell::new(None::<i32>));
        let timer_id_for_move = timer_id.clone();
        let on_mousemove = Closure::<dyn FnMut()>::wrap(Box::new(move || {
            let _ = body_clone.style().set_property("cursor", "default");
            if let Some(id) = timer_id_for_move.get() {
                popout_clone_timer.clear_timeout_with_handle(id);
            }
            let body_for_hide = body_clone.clone();
            let hide_cb = Closure::<dyn FnMut()>::wrap(Box::new(move || {
                let _ = body_for_hide.style().set_property("cursor", "none");
            }));
            if let Ok(id) = popout_clone_timer
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    hide_cb.as_ref().unchecked_ref(),
                    2000,
                )
            {
                timer_id_for_move.set(Some(id));
            }
            hide_cb.forget();
        }));
        let _ = popout
            .add_event_listener_with_callback("mousemove", on_mousemove.as_ref().unchecked_ref());
        on_mousemove.forget();
    }

    // 3. Window resize listener
    let canvas_for_resize = canvas.clone();
    let popout_for_resize = popout.clone();
    let pending_for_resize = pending_action.clone();
    let proxy_for_resize = proxy.clone();
    let on_resize = Closure::<dyn FnMut()>::wrap(Box::new(move || {
        let dpr = popout_for_resize.device_pixel_ratio();
        let logical_w = popout_for_resize
            .inner_width()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(1920.0);
        let logical_h = popout_for_resize
            .inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(1080.0);
        let phys_w = ((logical_w * dpr) as u32).max(1);
        let phys_h = ((logical_h * dpr) as u32).max(1);
        canvas_for_resize.set_width(phys_w);
        canvas_for_resize.set_height(phys_h);
        *pending_for_resize.borrow_mut() = Some(ProjectorAction::Resize((phys_w, phys_h)));
        let _ = proxy_for_resize.send_event(());
    }));
    let _ = popout.add_event_listener_with_callback("resize", on_resize.as_ref().unchecked_ref());
    on_resize.forget();

    // 4. Window close listener (beforeunload)
    let pending_for_close = pending_action.clone();
    let proxy_for_close = proxy.clone();
    let on_close = Closure::<dyn FnMut()>::wrap(Box::new(move || {
        *pending_for_close.borrow_mut() = Some(ProjectorAction::Detach);
        let _ = proxy_for_close.send_event(());
    }));
    let _ =
        popout.add_event_listener_with_callback("beforeunload", on_close.as_ref().unchecked_ref());
    on_close.forget();

    log::info!("Karakuri Web: Projector popout window attached ({phys_w}x{phys_h})");
    Ok(())
}

/// Opens the secondary popout window loading `projector.html`.
pub fn open_projector_window(
    proxy: EventLoopProxy<()>,
    pending_action: std::rc::Rc<std::cell::RefCell<Option<ProjectorAction>>>,
) -> Result<(), String> {
    let window = web_sys::window().ok_or("No global window found")?;

    let setup_done = std::rc::Rc::new(std::cell::Cell::new(false));
    let setup_done_ready = setup_done.clone();
    let setup_done_load = setup_done.clone();

    // Expose callback on parent window so projector.html can signal readiness on DOMContentLoaded
    let proxy_cb = proxy.clone();
    let pending_cb = pending_action.clone();
    let on_ready =
        Closure::<dyn FnMut(web_sys::Window)>::wrap(Box::new(move |child_win: web_sys::Window| {
            if !setup_done_ready.replace(true) {
                if let Err(e) = setup_projector_canvas(&child_win, proxy_cb.clone(), &pending_cb) {
                    log::warn!("Karakuri Web: Failed to setup projector canvas on ready: {e}");
                }
            }
        }));
    let _ = js_sys::Reflect::set(
        &window,
        &JsValue::from_str("onKarakuriProjectorReady"),
        on_ready.as_ref(),
    );
    on_ready.forget();

    // Open clean, minimal popup window pointing to same-origin projector.html
    let popout = window
        .open_with_url_and_target_and_features(
            "projector.html",
            "karakuri-projector",
            "width=1920,height=1080,menubar=no,toolbar=no,location=no,status=no",
        )
        .map_err(|e| format!("Failed to open projector window: {e:?}"))?
        .ok_or("Projector window popup was blocked by the browser")?;

    // If window was already open and loaded, setup canvas immediately
    if let Some(doc) = popout.document() {
        if doc.get_element_by_id("karakuri-projector-canvas").is_some() {
            if !setup_done.replace(true) {
                let _ = setup_projector_canvas(&popout, proxy.clone(), &pending_action);
                return Ok(());
            }
        }
    }

    // Also attach load listener on popout as fallback
    let popout_load = popout.clone();
    let proxy_load = proxy.clone();
    let pending_load = pending_action.clone();
    let on_load = Closure::<dyn FnMut()>::wrap(Box::new(move || {
        if !setup_done_load.replace(true) {
            if let Err(e) = setup_projector_canvas(&popout_load, proxy_load.clone(), &pending_load)
            {
                log::warn!("Karakuri Web: Failed to setup projector canvas on load: {e}");
            }
        }
    }));
    let _ = popout.add_event_listener_with_callback("load", on_load.as_ref().unchecked_ref());
    on_load.forget();

    Ok(())
}

/// Handles Projector output toggle from the Outputs row.
pub fn handle_route_hook(
    on: bool,
    proxy: &EventLoopProxy<()>,
    pending: &std::rc::Rc<std::cell::RefCell<Option<ProjectorAction>>>,
) {
    if on {
        if let Err(e) = open_projector_window(proxy.clone(), pending.clone()) {
            log::warn!("Karakuri Web: Failed to open projector window: {e}");
            *pending.borrow_mut() = Some(ProjectorAction::Detach);
            let _ = proxy.send_event(());
        }
    } else {
        *pending.borrow_mut() = Some(ProjectorAction::Detach);
        let _ = proxy.send_event(());
    }
}

/// Applies a pending Projector action to the App state and manages the popout window handle.
pub fn handle_projector_action(
    app: &mut karakuri::App,
    projector_window: &mut Option<web_sys::Window>,
    action: ProjectorAction,
) {
    match action {
        ProjectorAction::Attach {
            surface,
            size,
            window,
        } => {
            app.attach_projector(surface, size);
            *projector_window = Some(window);
        }
        ProjectorAction::Detach => {
            app.detach_projector();
            if let Some(win) = projector_window.take() {
                if !win.closed().unwrap_or(true) {
                    let _ = win.close();
                }
            }
        }
        ProjectorAction::Resize(size) => {
            app.resize_projector(size);
        }
    }
}
