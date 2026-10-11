//! Wireless presentation display streaming via Presentation API and WebRTC P2P.

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use winit::event_loop::EventLoopProxy;

#[wasm_bindgen(inline_js = r##"
export function is_presentation_available() {
    return typeof window !== 'undefined' &&
           typeof window.PresentationRequest !== 'undefined' &&
           navigator && typeof navigator.presentation !== 'undefined';
}

let activeSession = null;

export async function start_presentation_cast(on_close_cb) {
    if (!is_presentation_available()) {
        throw new Error("Presentation API is not available in this browser");
    }
    const canvas = document.getElementById('karakuri-cast-canvas');
    if (!canvas) {
        throw new Error("karakuri-cast-canvas element not found");
    }
    const stream = canvas.captureStream(60);
    const request = new PresentationRequest(['receiver.html']);

    let connection;
    try {
        connection = await request.start();
    } catch (err) {
        console.warn("Karakuri Web: Cast start dismissed or failed:", err);
        throw err;
    }

    const pc = new RTCPeerConnection({
        iceServers: [{ urls: 'stun:stun.l.google.com:19302' }]
    });

    stream.getTracks().forEach(track => pc.addTrack(track, stream));

    pc.onicecandidate = (event) => {
        if (event.candidate && connection.state === 'connected') {
            connection.send(JSON.stringify({ type: 'candidate', candidate: event.candidate }));
        }
    };

    connection.onmessage = async (event) => {
        try {
            const msg = JSON.parse(event.data);
            if (msg.type === 'answer') {
                await pc.setRemoteDescription(new RTCSessionDescription(msg.sdp));
            } else if (msg.type === 'candidate') {
                await pc.addIceCandidate(new RTCIceCandidate(msg.candidate));
            }
        } catch (e) {
            console.error("Karakuri Web: Cast sender error handling message:", e);
        }
    };

    const closeHandler = () => {
        stop_presentation_cast();
        if (typeof on_close_cb === 'function') {
            on_close_cb();
        }
    };

    connection.onclose = closeHandler;
    connection.onterminate = closeHandler;

    const sendOffer = async () => {
        const offer = await pc.createOffer();
        await pc.setLocalDescription(offer);
        connection.send(JSON.stringify({ type: 'offer', sdp: offer }));
    };

    if (connection.state === 'connected') {
        await sendOffer();
    } else {
        connection.onconnect = async () => {
            await sendOffer();
        };
    }

    activeSession = { connection, pc };
    return true;
}

export function stop_presentation_cast() {
    if (activeSession) {
        try {
            if (activeSession.pc) activeSession.pc.close();
            if (activeSession.connection) activeSession.connection.terminate();
        } catch (_) {}
        activeSession = null;
    }
}
"##)]
extern "C" {
    pub fn is_presentation_available() -> bool;
    pub fn start_presentation_cast(on_close_cb: &js_sys::Function) -> js_sys::Promise;
    pub fn stop_presentation_cast();
}

/// Cast stream actions dispatched across thread/event boundaries.
pub enum CastAction {
    Attach(wgpu::Surface<'static>),
    Detach,
}

/// Handles Cast route toggle from the Outputs row.
pub fn handle_cast_toggle(
    on: bool,
    proxy: &EventLoopProxy<()>,
    pending: &Rc<RefCell<Option<CastAction>>>,
) {
    if on {
        start_cast(proxy, pending);
    } else {
        stop_cast(proxy, pending);
    }
}

fn start_cast(proxy: &EventLoopProxy<()>, pending: &Rc<RefCell<Option<CastAction>>>) {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let doc = match window.document() {
        Some(d) => d,
        None => return,
    };
    let canvas_el = match doc.get_element_by_id("karakuri-cast-canvas") {
        Some(el) => el,
        None => {
            log::error!("Karakuri Web: #karakuri-cast-canvas element not found");
            return;
        }
    };
    let canvas: web_sys::HtmlCanvasElement = canvas_el.unchecked_into();
    canvas.set_width(1920);
    canvas.set_height(1080);

    // Create WebGPU surface targeting the cast canvas
    let instance = karakuri_engine::Gpu::instance();
    let surface = match instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas)) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Karakuri Web: Failed to create surface for cast canvas: {e:?}");
            return;
        }
    };

    *pending.borrow_mut() = Some(CastAction::Attach(surface));
    let _ = proxy.send_event(());

    // Setup on_close callback when presentation session terminates
    let proxy_for_close = proxy.clone();
    let pending_for_close = pending.clone();
    let on_close_closure = Closure::<dyn FnMut()>::wrap(Box::new(move || {
        log::info!("Karakuri Web: Presentation cast session closed");
        *pending_for_close.borrow_mut() = Some(CastAction::Detach);
        let _ = proxy_for_close.send_event(());
    }));

    let promise = start_presentation_cast(on_close_closure.as_ref().unchecked_ref());
    on_close_closure.forget();

    let proxy_for_err = proxy.clone();
    let pending_for_err = pending.clone();
    spawn_local(async move {
        match wasm_bindgen_futures::JsFuture::from(promise).await {
            Ok(_) => {
                log::info!("Karakuri Web: Presentation cast session started successfully");
            }
            Err(e) => {
                log::warn!("Karakuri Web: Presentation cast failed or cancelled: {e:?}");
                *pending_for_err.borrow_mut() = Some(CastAction::Detach);
                let _ = proxy_for_err.send_event(());
            }
        }
    });
}

fn stop_cast(proxy: &EventLoopProxy<()>, pending: &Rc<RefCell<Option<CastAction>>>) {
    stop_presentation_cast();
    *pending.borrow_mut() = Some(CastAction::Detach);
    let _ = proxy.send_event(());
}
