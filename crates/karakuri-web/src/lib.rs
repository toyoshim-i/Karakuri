//! Web entry point for Karakuri in modern WebGPU-enabled browsers.

#[cfg(target_arch = "wasm32")]
mod app;
#[cfg(target_arch = "wasm32")]
pub mod audio;
#[cfg(target_arch = "wasm32")]
pub mod cast;
#[cfg(any(target_arch = "wasm32", test))]
pub mod font_loader;
#[cfg(any(target_arch = "wasm32", test))]
pub mod harness;
#[cfg(target_arch = "wasm32")]
pub mod ime_overlay;
#[cfg(target_arch = "wasm32")]
pub mod midi;
#[cfg(target_arch = "wasm32")]
pub mod pip;
#[cfg(target_arch = "wasm32")]
pub mod projector;
#[cfg(target_arch = "wasm32")]
pub mod share;
#[cfg(any(target_arch = "wasm32", test))]
pub mod webmcp;
#[cfg(target_arch = "wasm32")]
pub mod webxr;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Web entry point invoked on WASM module startup.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn start() {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);
    log::info!("Karakuri Web starting up...");

    let mem_fs = karakuri_store::fs::default_memory_fs();
    log::info!("Karakuri Web: Hydrating VFS from IndexedDB...");
    karakuri_store::vfs::hydrate_web_fs(&mem_fs).await;
    log::info!("Karakuri Web: VFS ready");

    use app::WebApp;
    use winit::event_loop::EventLoop;
    use winit::platform::web::EventLoopExtWebSys;

    log::info!("Karakuri Web: Creating Winit event loop...");
    let event_loop: EventLoop<()> = EventLoop::new().expect("event loop");
    let proxy = event_loop.create_proxy();
    log::info!("Karakuri Web: Spawning WebApp...");
    event_loop.spawn_app(WebApp::new(proxy));
}
