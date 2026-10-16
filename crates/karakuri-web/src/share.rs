//! URL Hash permalink sharing and Web Share integration.

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = __karakuri, catch)]
    fn get_url_hash_set() -> Result<js_sys::Promise, JsValue>;

    #[wasm_bindgen(js_namespace = __karakuri, catch)]
    fn share_set_with_video(id: &str, ndjson: &str) -> Result<js_sys::Promise, JsValue>;
}

/// Retrieves and decompresses any `.kbset` NDJSON bundle passed in the URL hash (`#set=...`).
pub async fn load_url_hash_set() -> Result<Option<String>, String> {
    let promise = match get_url_hash_set() {
        Ok(p) => p,
        Err(e) => return Err(format!("failed to call get_url_hash_set: {e:?}")),
    };
    let val = JsFuture::from(promise)
        .await
        .map_err(|e| format!("get_url_hash_set promise rejected: {e:?}"))?;
    if val.is_null() || val.is_undefined() {
        Ok(None)
    } else {
        Ok(val.as_string())
    }
}

pub enum RecorderAction {
    Attach((u32, u32)),
    Detach,
}

pub fn is_recorder_ready() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let Ok(func_val) =
        js_sys::Reflect::get(&window, &JsValue::from_str("__karakuri_is_recorder_ready"))
    else {
        return true;
    };
    if let Some(func) = func_val.dyn_ref::<js_sys::Function>() {
        func.call0(&window)
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    } else {
        true
    }
}

pub fn notify_recorder_frame() {
    let Some(window) = web_sys::window() else {
        return;
    };
    if let Ok(func_val) =
        js_sys::Reflect::get(&window, &JsValue::from_str("__karakuri_on_recorder_frame"))
    {
        if let Some(func) = func_val.dyn_ref::<js_sys::Function>() {
            let _ = func.call0(&window);
        }
    }
}

pub fn start_recorder(
    proxy: &winit::event_loop::EventLoopProxy<()>,
    pending: &std::rc::Rc<std::cell::RefCell<Option<RecorderAction>>>,
) {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let doc = match window.document() {
        Some(d) => d,
        None => return,
    };
    let canvas_el = match doc.get_element_by_id("karakuri-recorder-canvas") {
        Some(el) => el,
        None => {
            log::error!("Karakuri Web: #karakuri-recorder-canvas element not found");
            return;
        }
    };
    let attr_w = canvas_el
        .get_attribute("data-target-width")
        .and_then(|s| s.parse::<u32>().ok());
    let attr_h = canvas_el
        .get_attribute("data-target-height")
        .and_then(|s| s.parse::<u32>().ok());
    let canvas: web_sys::HtmlCanvasElement = wasm_bindgen::JsCast::unchecked_into(canvas_el);
    let (width, height) = match (attr_w, attr_h) {
        (Some(w), Some(h)) if w >= 1280 && h >= 720 => (w, h),
        _ if canvas.width() >= 1280 && canvas.height() >= 720 => (canvas.width(), canvas.height()),
        _ => (1920, 1080),
    };

    *pending.borrow_mut() = Some(RecorderAction::Attach((width, height)));
    let _ = proxy.send_event(());
}

pub fn handle_recorder_action(app: &mut karakuri::App, action: RecorderAction) {
    match action {
        RecorderAction::Attach(size) => attach_recorder_canvas(app, size),
        RecorderAction::Detach => app.detach_recorder_surface(),
    }
}

pub fn attach_recorder_canvas(app: &mut karakuri::App, (width, height): (u32, u32)) {
    // Always detach any previous recorder sink first so its wgpu::Surface is dropped
    // before resizing the DOM canvas and creating a fresh surface.
    app.detach_recorder_surface();

    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(doc) = window.document() else {
        return;
    };
    let Some(canvas_el) = doc.get_element_by_id("karakuri-recorder-canvas") else {
        log::error!("Karakuri Web: #karakuri-recorder-canvas element not found");
        return;
    };
    let canvas: web_sys::HtmlCanvasElement = wasm_bindgen::JsCast::unchecked_into(canvas_el);
    canvas.set_width(width);
    canvas.set_height(height);
    let _ = canvas.style().set_property("width", &format!("{width}px"));
    let _ = canvas
        .style()
        .set_property("height", &format!("{height}px"));

    let instance = karakuri_engine::Gpu::instance();
    let surface = match instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas)) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Karakuri Web: Failed to create surface for recorder canvas: {e:?}");
            return;
        }
    };

    app.set_recorder_ready(is_recorder_ready());
    app.attach_recorder_surface(surface, (width, height));
}

pub fn stop_recorder(
    proxy: &winit::event_loop::EventLoopProxy<()>,
    pending: &std::rc::Rc<std::cell::RefCell<Option<RecorderAction>>>,
) {
    *pending.borrow_mut() = Some(RecorderAction::Detach);
    let _ = proxy.send_event(());
}

/// Triggers video capture, URL hash generation, and social sharing.
pub async fn execute_web_share(id: &str, ndjson: &str) -> Result<(), String> {
    let promise = match share_set_with_video(id, ndjson) {
        Ok(p) => p,
        Err(e) => return Err(format!("failed to call share_set_with_video: {e:?}")),
    };
    JsFuture::from(promise)
        .await
        .map_err(|e| format!("share_set_with_video promise rejected: {e:?}"))?;
    Ok(())
}
