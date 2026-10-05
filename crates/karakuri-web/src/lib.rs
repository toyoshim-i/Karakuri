//! Web entry point for Karakuri in modern WebGPU-enabled browsers.

#[cfg(target_arch = "wasm32")]
mod app;
#[cfg(target_arch = "wasm32")]
pub mod audio;
#[cfg(any(target_arch = "wasm32", test))]
pub mod font_loader;
#[cfg(any(target_arch = "wasm32", test))]
pub mod harness;
#[cfg(target_arch = "wasm32")]
pub mod ime_overlay;
#[cfg(target_arch = "wasm32")]
pub mod midi;
#[cfg(any(target_arch = "wasm32", test))]
pub mod webmcp;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Web entry point invoked on WASM module startup.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);
    log::info!("Karakuri Web starting up...");

    use app::WebApp;
    use winit::event_loop::EventLoop;
    use winit::platform::web::EventLoopExtWebSys;

    let event_loop: EventLoop<()> = EventLoop::new().expect("event loop");
    let proxy = event_loop.create_proxy();
    event_loop.spawn_app(WebApp::new(proxy));
}
