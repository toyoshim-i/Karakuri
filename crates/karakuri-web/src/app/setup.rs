use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use karakuri::{mcp, seeded, working_copies, App, Launch, Sources, SLOTS};
use karakuri_environment::{Opening, SlotPolicies};
use karakuri_store::store::Store;
use wasm_bindgen::JsCast;
use winit::event_loop::EventLoopProxy;

use super::{spawn_web_midi_scan, WebApp};

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

        // Set up WebXR and Cast session request hook when operator clicks plugin chips
        let pending_cast: Rc<RefCell<Option<crate::cast::CastAction>>> =
            Rc::new(RefCell::new(None));
        let pending_cast_hook = pending_cast.clone();
        let proxy_cast = proxy.clone();
        let vr_flow_for_hook = Rc::clone(&start_vr_flow);
        app.set_plugin_route_hook(move |n, on, xr_mode| {
            if n == 0 {
                vr_flow_for_hook(xr_mode);
            } else if n == 1 {
                crate::cast::handle_cast_toggle(on, &proxy_cast, &pending_cast_hook);
            }
        });

        let cast_available = crate::cast::is_presentation_available();
        app.set_cast_override(cast_available, false);

        // Set up Projector popout window hook when operator toggles Projector chip
        let pending_projector: Rc<RefCell<Option<crate::projector::ProjectorAction>>> =
            Rc::new(RefCell::new(None));
        let pending_proj_hook = pending_projector.clone();
        let proxy_proj = proxy.clone();
        app.set_projector_route_hook(move |on| {
            crate::projector::handle_route_hook(on, &proxy_proj, &pending_proj_hook);
        });

        // Set up Document Picture-in-Picture floating window hook when operator toggles PiP chip
        let pending_pip: Rc<RefCell<Option<crate::pip::PipAction>>> = Rc::new(RefCell::new(None));
        let pending_pip_hook = pending_pip.clone();
        let proxy_pip = proxy.clone();
        app.set_pip_route_hook(move |on| {
            crate::pip::handle_route_hook(on, &proxy_pip, &pending_pip_hook);
        });

        let pip_available = crate::pip::is_pip_available();
        app.set_pip_override(pip_available, false);

        let pending_share = Rc::new(RefCell::new(false));
        let share_trigger = pending_share.clone();
        let share_proxy = proxy.clone();
        app.set_share_hook(move || {
            *share_trigger.borrow_mut() = true;
            let _ = share_proxy.send_event(());
        });

        app.set_log_toggle_hook(move || {
            use wasm_bindgen::JsCast;
            if let Some(w) = web_sys::window() {
                if let Some(doc) = w.document() {
                    if let Some(el) = doc.get_element_by_id("karakuri-log-drawer") {
                        if let Some(html_el) = el.dyn_ref::<web_sys::HtmlElement>() {
                            let cur = html_el
                                .style()
                                .get_property_value("display")
                                .unwrap_or_default();
                            let next = if cur == "flex" { "none" } else { "flex" };
                            let _ = html_el.style().set_property("display", next);
                        }
                    }
                }
            }
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
            pending_projector,
            projector_window: None,
            pending_pip,
            pip_window: None,
            pending_cast,
            pending_recorder: Rc::new(RefCell::new(None)),
            pending_share,
            pending_import: Rc::new(RefCell::new(None)),
            initialized: false,
            ime_overlay: None,
            xr_rig_origin: None,
            xr_synced_pose: None,
        }
    }
}
