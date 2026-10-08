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
use winit::dpi::PhysicalPosition;
use winit::event::{MouseScrollDelta, WindowEvent};
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
    pending_xr_surface: Rc<RefCell<Option<wgpu::Surface<'static>>>>,
    pending_font: Rc<RefCell<Option<Vec<u8>>>>,
    pending_audio: Rc<RefCell<Option<karakuri_environment::audio::Audio>>>,
    _audio_session: Rc<RefCell<Option<crate::audio::WebAudioSession>>>,
    pending_midi: Rc<RefCell<Option<karakuri_environment::midi::Surface>>>,
    _midi_session: Rc<RefCell<Option<crate::midi::WebMidiSession>>>,
    webxr_state: Rc<RefCell<crate::webxr::WebXrState>>,
    pending_webxr_support: Rc<RefCell<Option<bool>>>,
    pending_webxr_active: Rc<RefCell<Option<bool>>>,
    initialized: bool,
    ime_overlay: Option<crate::ime_overlay::ImeOverlay>,
    /// Where the head was when the WebXR session began, in the headset's
    /// reference space; the stereo eyes ride the Set's camera from here.
    xr_rig_origin: Option<[f32; 3]>,
    /// The headset pose last handed to the App as stereo matrices.
    xr_synced_pose: Option<crate::webxr::StereoPose>,
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

        let webxr_state = Rc::new(RefCell::new(crate::webxr::WebXrState::new()));
        let pending_webxr_support = Rc::new(RefCell::new(None));
        let pending_webxr_active = Rc::new(RefCell::new(None));

        // Auto-detect WebXR immersive-vr and immersive-ar capability on startup
        let xr_proxy = proxy.clone();
        let pending_support_clone = Rc::clone(&pending_webxr_support);
        let xr_state_clone = Rc::clone(&webxr_state);
        wasm_bindgen_futures::spawn_local(async move {
            let (vr_supported, ar_supported) = crate::webxr::check_webxr_support().await;
            let supported = vr_supported || ar_supported;
            xr_state_clone.borrow_mut().is_supported = supported;
            xr_state_clone.borrow_mut().is_ar_supported = ar_supported;
            *pending_support_clone.borrow_mut() = Some(supported);
            let _ = xr_proxy.send_event(());
        });

        // Shared helper for initiating or ending WebXR immersive session
        let start_vr_flow = {
            let xr_state = Rc::clone(&webxr_state);
            let pending_active = Rc::clone(&pending_webxr_active);
            let proxy = proxy.clone();
            Rc::new(
                move |xr_session_mode: karakuri_console::view::XrSessionMode| {
                    if xr_state.borrow().is_active {
                        log::info!("Karakuri Web: User requested exit XR - ending session");
                        xr_state.borrow_mut().end_session();
                        *pending_active.borrow_mut() = Some(false);
                        let _ = proxy.send_event(());
                        return;
                    }

                    let mode = match xr_session_mode {
                        karakuri_console::view::XrSessionMode::Vr => {
                            crate::webxr::WebXrSessionMode::Vr
                        }
                        karakuri_console::view::XrSessionMode::Mr => {
                            crate::webxr::WebXrSessionMode::Ar
                        }
                    };
                    let ar_supported = xr_state.borrow().is_ar_supported;

                    let session_promise =
                        match crate::webxr::request_webxr_session(mode, ar_supported) {
                            Ok(p) => p,
                            Err(e) => {
                                log::warn!("Karakuri Web: Failed to request WebXR session: {e}");
                                *pending_active.borrow_mut() = Some(false);
                                let _ = proxy.send_event(());
                                return;
                            }
                        };

                    let dom_window = web_sys::window().expect("window");
                    let document = dom_window.document().expect("document");
                    let canvas = document
                        .get_element_by_id("karakuri-canvas")
                        .expect("canvas with id karakuri-canvas")
                        .dyn_into::<web_sys::HtmlCanvasElement>()
                        .expect("HtmlCanvasElement");

                    let xr_state_for_start = xr_state.clone();
                    let pending_active_for_start = pending_active.clone();
                    let proxy_for_start = proxy.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        match crate::webxr::start_webxr_session(
                            session_promise,
                            canvas.clone(),
                            mode,
                            xr_state_for_start.clone(),
                            pending_active_for_start.clone(),
                            proxy_for_start.clone(),
                        )
                        .await
                        {
                            Ok(session) => {
                                log::info!("Karakuri Web: WebXR session started (mode: {mode:?})");
                                canvas.set_width(1920);
                                canvas.set_height(1080);
                                let _ = canvas.style().set_property("width", "1920px");
                                let _ = canvas.style().set_property("height", "1080px");
                                xr_state_for_start.borrow_mut().session = Some(session);
                                xr_state_for_start.borrow_mut().session_mode = mode;
                                xr_state_for_start.borrow_mut().is_active = true;
                                *pending_active_for_start.borrow_mut() = Some(true);
                                let _ = proxy_for_start.send_event(());
                            }
                            Err(e) => {
                                log::warn!("Karakuri Web: Failed to start WebXR session: {e}");
                                xr_state_for_start.borrow_mut().is_active = false;
                                xr_state_for_start.borrow_mut().session = None;
                                *pending_active_for_start.borrow_mut() = Some(false);
                                let _ = proxy_for_start.send_event(());
                            }
                        }
                    });
                },
            )
        };

        // Set up WebXR session request hook when operator clicks WebXR plugin chip
        let vr_flow_for_hook = Rc::clone(&start_vr_flow);
        app.set_plugin_route_hook(move |_n, _on, xr_mode| {
            vr_flow_for_hook(xr_mode);
        });

        Self {
            app,
            proxy,
            window: None,
            pending_gpu: Rc::new(RefCell::new(None)),
            pending_xr_surface: Rc::new(RefCell::new(None)),
            pending_font,
            pending_audio,
            _audio_session: audio_session,
            pending_midi,
            _midi_session: midi_session,
            webxr_state,
            pending_webxr_support,
            pending_webxr_active,
            initialized: false,
            ime_overlay: None,
            xr_rig_origin: None,
            xr_synced_pose: None,
        }
    }

    /// Hands the headset's latest eye poses to the App for the stereo world.
    ///
    /// Each eye rides the Set's camera: rig space is the headset's reference
    /// space re-centred on where the head was when the session began, so
    /// standing still puts the eyes exactly on the camera and any head motion
    /// is a look-around from it (one world unit = one metre for now).
    fn sync_stereo_matrices(&mut self) {
        let pose = self.webxr_state.borrow().stereo_pose();
        let Some(pose) = pose else {
            self.xr_rig_origin = None;
            self.xr_synced_pose = None;
            self.app.set_stereo_matrices(None);
            self.app.set_stereo_eye_size(None);
            if self.app.view().vr_mode {
                self.app.view_mut().configure_normal_resolutions();
            }
            return;
        };
        if !self.app.view().vr_mode {
            self.app.view_mut().configure_vr_resolutions(pose.eye_size);
        }
        let eye_size = self
            .app
            .view()
            .selected_resolution()
            .unwrap_or((pose.eye_size.0 / 2, pose.eye_size.1 / 2));
        self.app.set_stereo_eye_size(Some(eye_size));
        let origin = *self.xr_rig_origin.get_or_insert_with(|| {
            let (l, r) = (pose.left.eye, pose.right.eye);
            [
                (l[0] + r[0]) * 0.5,
                (l[1] + r[1]) * 0.5,
                (l[2] + r[2]) * 0.5,
            ]
        });
        // head = V_xr * T(origin): a rig-space point p sits at p + origin in
        // the headset's reference space.
        let eye = |e: &crate::webxr::StereoEye| {
            let mut v = e.view;
            let [ox, oy, oz] = origin;
            for row in 0..3 {
                v[12 + row] += ox * v[row] + oy * v[4 + row] + oz * v[8 + row];
            }
            karakuri_engine::StereoMatrices::from_slices(&v, &e.proj)
        };
        self.app
            .set_stereo_matrices(Some((eye(&pose.left), eye(&pose.right))));
        self.xr_synced_pose = Some(pose);
    }

    /// After a redraw: if it drew the stereo world, the XR canvas now shows
    /// the pose last handed over, which the XR layer reprojects from.
    fn note_stereo_drawn(&mut self) {
        if self.app.take_stereo_drawn() {
            self.webxr_state
                .borrow()
                .set_rendered_stereo_pose(self.xr_synced_pose);
            self.webxr_state
                .borrow()
                .set_is_fullscreen(self.app.is_fullscreen_stereo());
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

        let resize_proxy = self.proxy.clone();
        let resize_cb = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
            let _ = resize_proxy.send_event(());
        });
        dom_window
            .add_event_listener_with_callback("resize", resize_cb.as_ref().unchecked_ref())
            .expect("register resize event listener");
        resize_cb.forget();

        let instance = Gpu::instance();
        let surface = instance
            .create_surface(window.clone())
            .expect("create WebGPU surface");
        // Initialize secondary WebGPU surface on offscreen Canvas for WebXR stereo world rendering
        // Note: Using wgpu::SurfaceTarget::Canvas directly without creating a winit Window,
        // which completely eliminates any ResizeObserver or event loop conflicts.
        let xr_canvas_elem: Option<web_sys::HtmlCanvasElement> = document
            .get_element_by_id("karakuri-xr-canvas")
            .and_then(|el| el.dyn_into::<web_sys::HtmlCanvasElement>().ok());
        if let Some(xr_c) = xr_canvas_elem {
            xr_c.set_width(1920);
            xr_c.set_height(1080);
            if let Ok(xr_surf) = instance.create_surface(wgpu::SurfaceTarget::Canvas(xr_c)) {
                *self.pending_xr_surface.borrow_mut() = Some(xr_surf);
            }
        }

        let pending = Rc::clone(&self.pending_gpu);
        let proxy = self.proxy.clone();
        let win = window.clone();

        wasm_bindgen_futures::spawn_local(async move {
            match Gpu::from_instance(instance, Some(&surface)).await {
                Ok(gpu) => {
                    log::info!("Karakuri Web: WebGPU initialized successfully");
                    // Validation errors otherwise go only to devtools, never
                    // through console.log, so the remote log relay would not
                    // see a frame the GPU refused.
                    gpu.device
                        .on_uncaptured_error(std::sync::Arc::new(|error: wgpu::Error| {
                            use std::sync::atomic::{AtomicU32, Ordering};
                            // A per-frame error would otherwise flood the relay.
                            static SAID: AtomicU32 = AtomicU32::new(0);
                            if SAID.fetch_add(1, Ordering::Relaxed) < 20 {
                                log::error!("Karakuri Web: WebGPU error: {error}");
                            }
                        }));
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
            if let Some(xr_surf) = self.pending_xr_surface.borrow_mut().take() {
                self.app.attach_xr_surface(xr_surf);
                log::info!("Karakuri Web: Attached secondary WebXR surface successfully");
            }
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
        if let Some(supported) = self.pending_webxr_support.borrow_mut().take() {
            if supported {
                self.app.set_plugin_override(true, Some("WebXR"), false);
            }
            if let Some(ref window) = self.window {
                window.request_redraw();
            }
        }
        if let Some(active) = self.pending_webxr_active.borrow_mut().take() {
            self.app.set_plugin_override(true, Some("WebXR"), active);
            if let Some(ref window) = self.window {
                let dom_window = web_sys::window().expect("window");
                let target_size = if active {
                    winit::dpi::PhysicalSize::new(1920, 1080)
                } else {
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
                    let document = dom_window.document().expect("document");
                    if let Some(canvas) = document
                        .get_element_by_id("karakuri-canvas")
                        .and_then(|el| el.dyn_into::<web_sys::HtmlCanvasElement>().ok())
                    {
                        let _ = canvas.style().remove_property("width");
                        let _ = canvas.style().remove_property("height");
                        let pw = (logical_w * dpr).max(1.0) as u32;
                        let ph = (logical_h * dpr).max(1.0) as u32;
                        canvas.set_width(pw);
                        canvas.set_height(ph);
                    }
                    // Schedule delayed resize syncs to catch browser window restore animations
                    let delayed_proxy = self.proxy.clone();
                    if let Ok(cb) = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
                        let _ = delayed_proxy.send_event(());
                    })
                    .into_js_value()
                    .dyn_into::<js_sys::Function>()
                    {
                        let _ = dom_window
                            .set_timeout_with_callback_and_timeout_and_arguments_0(&cb, 120);
                        let _ = dom_window
                            .set_timeout_with_callback_and_timeout_and_arguments_0(&cb, 320);
                    }

                    winit::dpi::PhysicalSize::new(
                        (logical_w * dpr).max(1.0) as u32,
                        (logical_h * dpr).max(1.0) as u32,
                    )
                };
                let _ = window.request_inner_size(target_size);
                self.app.on_window_event(
                    _event_loop,
                    window.id(),
                    WindowEvent::Resized(target_size),
                );
                window.request_redraw();
            }
        }

        // On window resize or delayed sync while XR is not active, ensure canvas and viewport match DOM window
        if !self.webxr_state.borrow().is_active {
            if let Some(ref window) = self.window {
                if let Some(dom_window) = web_sys::window() {
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
                    let pw = (logical_w * dpr).max(1.0) as u32;
                    let ph = (logical_h * dpr).max(1.0) as u32;

                    let document = dom_window.document().expect("document");
                    if let Some(canvas) = document
                        .get_element_by_id("karakuri-canvas")
                        .and_then(|el| el.dyn_into::<web_sys::HtmlCanvasElement>().ok())
                    {
                        if canvas.width() != pw || canvas.height() != ph {
                            let _ = canvas.style().remove_property("width");
                            let _ = canvas.style().remove_property("height");
                            canvas.set_width(pw);
                            canvas.set_height(ph);
                            let target_size = winit::dpi::PhysicalSize::new(pw, ph);
                            let _ = window.request_inner_size(target_size);
                            self.app.on_window_event(
                                _event_loop,
                                window.id(),
                                WindowEvent::Resized(target_size),
                            );
                            window.request_redraw();
                        }
                    }
                }
            }
        }

        // Keep driving WebGPU rendering while WebXR session is active
        if self.webxr_state.borrow().is_active {
            if let Some(id) = self.window.as_ref().map(|w| w.id()) {
                // The XR frame that woke us has just stored the newest head
                // pose; draw with it rather than the one about_to_wait last
                // handed over, which is a frame older.
                self.sync_stereo_matrices();
                self.app
                    .on_window_event(_event_loop, id, WindowEvent::RedrawRequested);
                self.note_stereo_drawn();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        self.app.on_window_event(event_loop, id, event);
        self.note_stereo_drawn();
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

        // Drain controller pointer actions from WebXR spatial HUD session and inject into App
        let pointer_events = self.webxr_state.borrow().drain_pointer_events();
        if !pointer_events.is_empty() {
            if let Some(ref window) = self.window {
                let win_id = window.id();
                for action in pointer_events {
                    match action {
                        crate::webxr::WebXrPointerAction::CursorMoved { x, y } => {
                            self.app.on_window_event(
                                event_loop,
                                win_id,
                                WindowEvent::CursorMoved {
                                    device_id: winit::event::DeviceId::dummy(),
                                    position: PhysicalPosition::new(x, y),
                                },
                            );
                        }
                        crate::webxr::WebXrPointerAction::MouseInput { state, button } => {
                            self.app.on_window_event(
                                event_loop,
                                win_id,
                                WindowEvent::MouseInput {
                                    device_id: winit::event::DeviceId::dummy(),
                                    state,
                                    button,
                                },
                            );
                        }
                        crate::webxr::WebXrPointerAction::MouseWheel { delta_y } => {
                            self.app.on_window_event(
                                event_loop,
                                win_id,
                                WindowEvent::MouseWheel {
                                    device_id: winit::event::DeviceId::dummy(),
                                    delta: MouseScrollDelta::LineDelta(0.0, delta_y),
                                    phase: winit::event::TouchPhase::Moved,
                                },
                            );
                        }
                    }
                }
                window.request_redraw();
            }
        }

        self.note_stereo_drawn();
        self.sync_stereo_matrices();

        self.app.on_about_to_wait(event_loop);
        self.note_stereo_drawn();
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
