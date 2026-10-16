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
    Attach(wgpu::Surface<'static>, (u32, u32)),
    Detach,
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
    let canvas: web_sys::HtmlCanvasElement = wasm_bindgen::JsCast::unchecked_into(canvas_el);
    let (width, height) = if canvas.width() >= 1280 && canvas.height() >= 720 {
        (canvas.width(), canvas.height())
    } else {
        let (w, h) = (1920, 1080);
        canvas.set_width(w);
        canvas.set_height(h);
        (w, h)
    };

    let instance = karakuri_engine::Gpu::instance();
    let surface = match instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas)) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Karakuri Web: Failed to create surface for recorder canvas: {e:?}");
            return;
        }
    };

    *pending.borrow_mut() = Some(RecorderAction::Attach(surface, (width, height)));
    let _ = proxy.send_event(());
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
