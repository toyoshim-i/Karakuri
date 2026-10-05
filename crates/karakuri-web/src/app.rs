//! Application controller orchestrating DOM window setup, WebGPU initialization,
//! and event dispatching to the core Karakuri App.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use karakuri::{mcp, seeded, working_copies, App, Launch, Sources, SLOTS};
use karakuri_engine::Gpu;
use karakuri_environment::{Opening, SlotPolicies};
use karakuri_store::store::Store;
use wasm_bindgen::JsCast;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::platform::web::WindowAttributesExtWebSys;
use winit::window::{Window, WindowId};

type PendingGpu = Option<(Arc<Window>, wgpu::Surface<'static>, Gpu)>;

fn spawn_web_midi_scan(
    pending_midi: Rc<RefCell<Option<karakuri_environment::midi::Surface>>>,
    midi_session: Rc<RefCell<Option<crate::midi::WebMidiSession>>>,
    midi_scanning: Rc<RefCell<bool>>,
    proxy: EventLoopProxy<()>,
) {
    if *midi_scanning.borrow() {
        return;
    }
    *midi_scanning.borrow_mut() = true;

    wasm_bindgen_futures::spawn_local(async move {
        match crate::midi::start_web_midi(proxy.clone()).await {
            Ok((surface, session)) => {
                *midi_session.borrow_mut() = Some(session);
                *pending_midi.borrow_mut() = Some(surface);
                let _ = proxy.send_event(());
            }
            Err(e) => {
                log::info!("Karakuri Web: Web MIDI scan: {e}");
            }
        }
        *midi_scanning.borrow_mut() = false;
    });
}

pub struct WebApp {
    app: App,
    proxy: EventLoopProxy<()>,
    window: Option<Arc<Window>>,
    pending_gpu: Rc<RefCell<PendingGpu>>,
    pending_font: Rc<RefCell<Option<Vec<u8>>>>,
    pending_audio: Rc<RefCell<Option<karakuri_environment::audio::Audio>>>,
    _audio_session: Rc<RefCell<Option<crate::audio::WebAudioSession>>>,
    pending_midi: Rc<RefCell<Option<karakuri_environment::midi::Surface>>>,
    _midi_session: Rc<RefCell<Option<crate::midi::WebMidiSession>>>,
    initialized: bool,
    ime_overlay: Option<crate::ime_overlay::ImeOverlay>,
}

impl WebApp {
    pub fn new(proxy: EventLoopProxy<()>) -> Self {
        let presets_dir = std::path::PathBuf::from("examples");
        let store_dir = std::path::PathBuf::from(".karakuri");
        let sources = Sources::under(&presets_dir);

        let launch = Launch {
            sources: sources.clone(),
            store: store_dir.clone(),
            presets: Some(karakuri_environment::places::Presets {
                dir: presets_dir,
                found: karakuri_environment::places::Found::Beside,
            }),
            plugins: None,
            mcp: None,
        };

        let (_scratch, running) = working_copies(&launch.store, &launch.sources, SLOTS)
            .expect("working copies in virtual store");
        let held = Arc::new(Store::open(&launch.store).expect("store open"));
        let snapshots = seeded(&launch.store, &running);
        let opening = Opening::closed();
        let slot_policies = SlotPolicies::new();
        let pointing = mcp::Slots::of(
            running
                .iter()
                .map(|pair| (pair.l1.clone(), vec![pair.l4.clone()]))
                .collect(),
        );

        let (reporter, in_process_mcp) = karakuri_mcp::in_process(
            pointing.clone(),
            launch.store.clone(),
            true,
            opening.clone(),
            slot_policies.clone(),
        );

        let mut app = App::new(
            launch,
            running,
            held,
            snapshots,
            Some(reporter),
            opening,
            pointing,
            proxy.clone(),
            slot_policies,
        );

        let mcp_for_harness = in_process_mcp.clone();
        let proxy_for_harness = proxy.clone();
        app.set_prompt_spawner(move |id, _args| {
            let (session, _harness) = crate::harness::create_harness_session(
                id.to_string(),
                mcp_for_harness.clone(),
                proxy_for_harness.clone(),
            );
            session
        });

        // Initialize Prompt bay default selection to in-process agent harness
        let prompt_state = app.prompt_state();
        prompt_state.select(karakuri_console::view::prompt::CliSelection::Custom(
            "in-process".to_string(),
        ));

        if let Err(e) = crate::webmcp::setup_webmcp(in_process_mcp, proxy.clone()) {
            log::warn!("Karakuri Web: Failed to initialize WebMCP: {e:?}");
        }

        let pending_font = Rc::new(RefCell::new(None));
        let font_proxy = proxy.clone();
        let pending_font_clone = Rc::clone(&pending_font);
        wasm_bindgen_futures::spawn_local(async move {
            if let Some(bytes) = crate::font_loader::fetch_cjk_font().await {
                *pending_font_clone.borrow_mut() = Some(bytes);
                let _ = font_proxy.send_event(());
            }
        });

        let pending_audio = Rc::new(RefCell::new(None));
        let audio_session = Rc::new(RefCell::new(None));
        let audio_requested = Rc::new(RefCell::new(false));

        let audio_proxy = proxy.clone();
        let pending_audio_clone = Rc::clone(&pending_audio);
        let audio_session_clone = Rc::clone(&audio_session);
        let audio_requested_clone = Rc::clone(&audio_requested);

        app.set_audio_request_hook(move || {
            if *audio_requested_clone.borrow() {
                return;
            }
            *audio_requested_clone.borrow_mut() = true;

            let pending = Rc::clone(&pending_audio_clone);
            let session_slot = Rc::clone(&audio_session_clone);
            let proxy = audio_proxy.clone();
            let req = Rc::clone(&audio_requested_clone);

            wasm_bindgen_futures::spawn_local(async move {
                match crate::audio::start_web_audio(120.0).await {
                    Ok((audio, session)) => {
                        *session_slot.borrow_mut() = Some(session);
                        *pending.borrow_mut() = Some(audio);
                        let _ = proxy.send_event(());
                    }
                    Err(e) => {
                        log::warn!("Karakuri Web: Failed to activate Web Audio: {e:?}");
                        *req.borrow_mut() = false;
                    }
                }
            });
        });

        let pending_midi = Rc::new(RefCell::new(None));
        let midi_session = Rc::new(RefCell::new(None));
        let midi_scanning = Rc::new(RefCell::new(false));

        // Auto-detect connected Web MIDI controllers on startup
        spawn_web_midi_scan(
            Rc::clone(&pending_midi),
            Rc::clone(&midi_session),
            Rc::clone(&midi_scanning),
            proxy.clone(),
        );

        let midi_proxy = proxy.clone();
        let pending_midi_hook = Rc::clone(&pending_midi);
        let midi_session_hook = Rc::clone(&midi_session);
        let midi_scanning_hook = Rc::clone(&midi_scanning);
        app.set_midi_request_hook(move || {
            spawn_web_midi_scan(
                Rc::clone(&pending_midi_hook),
                Rc::clone(&midi_session_hook),
                Rc::clone(&midi_scanning_hook),
                midi_proxy.clone(),
            );
        });

        Self {
            app,
            proxy,
            window: None,
            pending_gpu: Rc::new(RefCell::new(None)),
            pending_font,
            pending_audio,
            _audio_session: audio_session,
            pending_midi,
            _midi_session: midi_session,
            initialized: false,
            ime_overlay: None,
        }
    }
}

impl ApplicationHandler<()> for WebApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.initialized {
            return;
        }
        self.initialized = true;

        let dom_window = web_sys::window().expect("global window");
        let document = dom_window.document().expect("document");
        let canvas = document
            .get_element_by_id("karakuri-canvas")
            .expect("canvas with id karakuri-canvas")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("HtmlCanvasElement");

        let dpr = dom_window.device_pixel_ratio();
        let logical_w = dom_window
            .inner_width()
            .ok()
            .and_then(|w| w.as_f64())
            .unwrap_or(1440.0);
        let logical_h = dom_window
            .inner_height()
            .ok()
            .and_then(|h| h.as_f64())
            .unwrap_or(900.0);

        let physical_w = (logical_w * dpr).max(1.0) as u32;
        let physical_h = (logical_h * dpr).max(1.0) as u32;
        canvas.set_width(physical_w);
        canvas.set_height(physical_h);

        let attrs = Window::default_attributes()
            .with_title("The Karakuri console")
            .with_canvas(Some(canvas.clone()));
        let window = Arc::new(event_loop.create_window(attrs).expect("window"));
        window.set_ime_allowed(true);
        self.window = Some(window.clone());

        let canvas_clone = canvas;
        let win_clone = window.clone();
        let resize_cb = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
            if let Some(win) = web_sys::window() {
                let dpr = win.device_pixel_ratio();
                let lw = win
                    .inner_width()
                    .ok()
                    .and_then(|w| w.as_f64())
                    .unwrap_or(1440.0);
                let lh = win
                    .inner_height()
                    .ok()
                    .and_then(|h| h.as_f64())
                    .unwrap_or(900.0);
                let pw = (lw * dpr).max(1.0) as u32;
                let ph = (lh * dpr).max(1.0) as u32;
                canvas_clone.set_width(pw);
                canvas_clone.set_height(ph);
                let _ = win_clone.request_inner_size(winit::dpi::PhysicalSize::new(pw, ph));
                win_clone.request_redraw();
            }
        });
        dom_window
            .add_event_listener_with_callback("resize", resize_cb.as_ref().unchecked_ref())
            .expect("register resize event listener");
        resize_cb.forget();

        let instance = Gpu::instance();
        let surface = instance
            .create_surface(window.clone())
            .expect("create WebGPU surface");

        let pending = Rc::clone(&self.pending_gpu);
        let proxy = self.proxy.clone();
        let win = window.clone();

        wasm_bindgen_futures::spawn_local(async move {
            match Gpu::from_instance(instance, Some(&surface)).await {
                Ok(gpu) => {
                    log::info!("Karakuri Web: WebGPU initialized successfully");
                    *pending.borrow_mut() = Some((win, surface, gpu));
                    let _ = proxy.send_event(());
                }
                Err(err) => {
                    log::error!("Karakuri Web: Failed to initialize WebGPU: {err}");
                }
            }
        });

        match crate::ime_overlay::ImeOverlay::new(
            self.app.prompt_state().clone(),
            self.proxy.clone(),
        ) {
            Ok(overlay) => {
                self.ime_overlay = Some(overlay);
            }
            Err(e) => {
                log::warn!("Karakuri Web: Failed to initialize IME overlay: {e:?}");
            }
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, (): ()) {
        if let Some((window, surface, gpu)) = self.pending_gpu.borrow_mut().take() {
            self.app.attach_gfx(window.clone(), surface, gpu);
            window.request_redraw();
        }
        if let Some(font_bytes) = self.pending_font.borrow_mut().take() {
            self.app.add_cjk_font(font_bytes);
            if let Some(ref window) = self.window {
                window.request_redraw();
            }
        }
        if let Some(audio) = self.pending_audio.borrow_mut().take() {
            self.app.attach_audio(audio);
            if let Some(ref window) = self.window {
                window.request_redraw();
            }
        }
        if let Some(surface) = self.pending_midi.borrow_mut().take() {
            self.app.attach_midi(surface);
            if let Some(ref window) = self.window {
                window.request_redraw();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        self.app.on_window_event(event_loop, id, event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(ref overlay) = self.ime_overlay {
            let rect = self.app.prompt_rect();
            let scale = self
                .window
                .as_ref()
                .map(|w| w.scale_factor())
                .unwrap_or(1.0);
            overlay.sync(rect, scale);
        }
        self.app.on_about_to_wait(event_loop);
        if matches!(
            event_loop.control_flow(),
            winit::event_loop::ControlFlow::Wait
        ) {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(
                web_time::Instant::now() + std::time::Duration::from_millis(32),
            ));
        }
    }
}
