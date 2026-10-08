pub mod hud;
pub mod types;
pub mod world;

pub use hud::*;
pub use types::*;
pub use world::*;

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext, XrFrame, XrHandedness, XrInputSource,
    XrReferenceSpace, XrRenderStateInit, XrSession, XrSessionInit, XrSessionMode, XrView,
    XrViewerPose, XrWebGlLayer, XrWebGlLayerInit,
};
use winit::event::{ElementState, MouseButton};
use winit::event_loop::EventLoopProxy;

/// Checks whether `immersive-vr` and `immersive-ar` are supported on the current browser.
pub async fn check_webxr_support() -> (bool, bool) {
    let Some(window) = web_sys::window() else {
        return (false, false);
    };
    let xr = window.navigator().xr();
    let vr_promise = xr.is_session_supported(XrSessionMode::ImmersiveVr);
    let ar_promise = xr.is_session_supported(XrSessionMode::ImmersiveAr);
    let vr_supported = wasm_bindgen_futures::JsFuture::from(vr_promise)
        .await
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false);
    let ar_supported = wasm_bindgen_futures::JsFuture::from(ar_promise)
        .await
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false);
    (vr_supported, ar_supported)
}

/// Synchronously initiates an `immersive-vr` or `immersive-ar` session request within the user gesture event context.
pub fn request_webxr_session(
    mode: WebXrSessionMode,
    ar_supported: bool,
) -> Result<js_sys::Promise, String> {
    let window = web_sys::window().ok_or("No global window")?;
    let xr = window.navigator().xr();

    let session_init = XrSessionInit::new();
    session_init.set_required_features(&[JsValue::from_str("local-floor")]);

    let has_xrgpu = js_sys::Reflect::has(&window, &"XRGPUBinding".into()).unwrap_or(false);
    web_sys::console::log_1(&format!("WebXR diagnostic: window.XRGPUBinding = {has_xrgpu}").into());

    let session_mode = match mode {
        WebXrSessionMode::Ar if ar_supported => XrSessionMode::ImmersiveAr,
        _ => XrSessionMode::ImmersiveVr,
    };

    Ok(xr
        .request_session_with_options(session_mode, &session_init)
        .unchecked_into())
}

/// Initializes an active WebXR session from the requested promise.
pub async fn start_webxr_session(
    session_promise: js_sys::Promise,
    main_canvas: HtmlCanvasElement,
    mode: WebXrSessionMode,
    xr_state: Rc<RefCell<WebXrState>>,
    pending_active: Rc<RefCell<Option<bool>>>,
    proxy: EventLoopProxy<()>,
) -> Result<XrSession, String> {
    let window = web_sys::window().ok_or("No global window")?;
    let session_val = wasm_bindgen_futures::JsFuture::from(session_promise)
        .await
        .map_err(|e| format!("Failed to request WebXR session: {e:?}"))?;
    let session: XrSession = session_val.unchecked_into();

    // Create an offscreen WebGL2 canvas with alpha enabled for stereo XR projection / MR passthrough
    let document = window.document().ok_or("No document")?;
    let xr_canvas: HtmlCanvasElement = document
        .create_element("canvas")
        .map_err(|e| format!("{e:?}"))?
        .unchecked_into();
    xr_canvas.set_width(1920);
    xr_canvas.set_height(1080);

    let gl_opts = js_sys::Object::new();
    js_sys::Reflect::set(&gl_opts, &"xrCompatible".into(), &JsValue::from_bool(true)).unwrap();
    js_sys::Reflect::set(&gl_opts, &"alpha".into(), &JsValue::from_bool(true)).unwrap();
    let gl: WebGl2RenderingContext = xr_canvas
        .get_context_with_context_options("webgl2", &gl_opts)
        .map_err(|e| format!("{e:?}"))?
        .ok_or("Failed to get WebGL2 context for XR")?
        .unchecked_into();

    let layer_init = XrWebGlLayerInit::new();
    layer_init.set_antialias(true);
    layer_init.set_alpha(true);
    let xr_gl_layer =
        XrWebGlLayer::new_with_web_gl2_rendering_context_and_layer_init(&session, &gl, &layer_init)
            .map_err(|e| format!("Failed to create XrWebGlLayer: {e:?}"))?;

    let render_state = XrRenderStateInit::new();
    render_state.set_base_layer(Some(&xr_gl_layer));
    session.update_render_state_with_state(&render_state);

    let ref_space_promise =
        session.request_reference_space(web_sys::XrReferenceSpaceType::LocalFloor);
    let ref_space: XrReferenceSpace =
        match wasm_bindgen_futures::JsFuture::from(ref_space_promise).await {
            Ok(val) => val.unchecked_into(),
            Err(_) => {
                let fallback = wasm_bindgen_futures::JsFuture::from(
                    session.request_reference_space(web_sys::XrReferenceSpaceType::Local),
                )
                .await
                .map_err(|e| format!("Failed to get reference space: {e:?}"))?;
                fallback.unchecked_into::<XrReferenceSpace>()
            }
        };

    let pointer_sink = xr_state.borrow().pending_pointer_events.clone();
    let current_stereo_pose = xr_state.borrow().current_stereo_pose.clone();
    let rendered_stereo_pose = xr_state.borrow().rendered_stereo_pose.clone();
    let is_fullscreen = xr_state.borrow().is_fullscreen.clone();
    let vr_projection = xr_state.borrow().vr_projection.clone();

    // Register 'end' event listener on session so if the user exits via Quest system menu or session.end(),
    // the application cleans up gracefully.
    let end_proxy = proxy.clone();
    let end_state = xr_state.clone();
    let end_pending = pending_active.clone();
    let on_end = Closure::wrap(Box::new(move |_ev: web_sys::Event| {
        log::info!("Karakuri Web: WebXR session ended callback fired");
        end_state.borrow_mut().is_active = false;
        end_state.borrow_mut().session = None;
        // A stale pose would keep the stereo world drawing after the session
        // and carry its rig origin into the next one.
        *end_state.borrow().current_stereo_pose.borrow_mut() = None;
        end_state.borrow().set_rendered_stereo_pose(None);
        *end_pending.borrow_mut() = Some(false);
        let _ = end_proxy.send_event(());
    }) as Box<dyn FnMut(web_sys::Event)>);
    let _ = session.add_event_listener_with_callback("end", on_end.as_ref().unchecked_ref());
    on_end.forget();

    let renderer = Rc::new(RefCell::new(XrQuadRenderer::new(&gl)?));
    let mut wr = XrWorldRenderer::new(&gl)?;
    if mode == WebXrSessionMode::Ar {
        wr.set_dome_mode(3);
    }
    let world_renderer = Rc::new(RefCell::new(wr));
    let world_canvas: Option<HtmlCanvasElement> = document
        .get_element_by_id("karakuri-xr-canvas")
        .and_then(|el| el.dyn_into::<HtmlCanvasElement>().ok());
    web_sys::console::log_1(
        &format!(
            "WebXR session init: karakuri-xr-canvas found = {}, main_canvas = {}x{}",
            world_canvas.is_some(),
            main_canvas.width(),
            main_canvas.height()
        )
        .into(),
    );

    let wr_for_window = world_renderer.clone();
    let toggle_bg_cb = Closure::wrap(Box::new(move || {
        let on = wr_for_window.borrow_mut().toggle_background();
        web_sys::console::log_1(&format!("WebXR cosmic background: {on}").into());
        on
    }) as Box<dyn FnMut() -> bool>);
    let _ = js_sys::Reflect::set(
        &window,
        &"__karakuri_toggle_background".into(),
        toggle_bg_cb.as_ref().unchecked_ref(),
    );
    toggle_bg_cb.forget();

    setup_xr_render_loop(
        session.clone(),
        ref_space,
        gl,
        xr_gl_layer,
        renderer,
        world_renderer,
        main_canvas,
        world_canvas,
        pointer_sink,
        current_stereo_pose,
        rendered_stereo_pose,
        is_fullscreen,
        vr_projection,
        mode,
        proxy,
    );

    Ok(session)
}

type XrFrameClosure = Rc<RefCell<Option<Closure<dyn FnMut(f64, XrFrame)>>>>;

#[allow(clippy::too_many_arguments)]
fn setup_xr_render_loop(
    session: XrSession,
    ref_space: XrReferenceSpace,
    gl: WebGl2RenderingContext,
    layer: XrWebGlLayer,
    renderer: Rc<RefCell<XrQuadRenderer>>,
    world_renderer: Rc<RefCell<XrWorldRenderer>>,
    canvas: HtmlCanvasElement,
    world_canvas: Option<HtmlCanvasElement>,
    pointer_sink: Rc<RefCell<Vec<WebXrPointerAction>>>,
    current_stereo_pose: Rc<RefCell<Option<StereoPose>>>,
    rendered_stereo_pose: Rc<RefCell<Option<StereoPose>>>,
    _is_fullscreen: Rc<RefCell<bool>>,
    vr_projection: Rc<RefCell<Option<karakuri_console::view::VrProjection>>>,
    session_mode: WebXrSessionMode,
    proxy: EventLoopProxy<()>,
) {
    let f: XrFrameClosure = Rc::new(RefCell::new(None));
    let g = f.clone();

    let mut last_trigger_pressed = false;
    let mut hud_anchor: Option<HudAnchor> = None;
    let mut grab_state: Option<GrabState> = None;
    let mut frame_count: u64 = 0;
    let mut last_frame_time: Option<f64> = None;
    let mut locomotion_pos = [0.0f32; 3];
    let mut locomotion_yaw = 0.0f32;
    let session_loop = session.clone();

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move |_time: f64, frame: XrFrame| {
        frame_count = frame_count.wrapping_add(1);
        let dt = if let Some(last_t) = last_frame_time {
            ((_time - last_t) * 0.001).clamp(0.0005, 0.05) as f32
        } else {
            1.0 / 72.0
        };
        last_frame_time = Some(_time);

        let pose: Option<XrViewerPose> = frame.get_viewer_pose(&ref_space);

        // Lazily anchor the HUD to the user's initial head pose
        if hud_anchor.is_none() {
            if let Some(ref p) = pose {
                hud_anchor = Some(HudAnchor::from_viewer_pose(p));
            }
        }

        let to_mat4 = |v: Vec<f32>| -> [f32; 16] { v.try_into().unwrap_or([0.0; 16]) };

        // Determine player head horizontal facing and right vectors for locomotion
        let mut move_fwd = [0.0f32, 0.0, -1.0];
        let mut move_right = [1.0f32, 0.0, 0.0];
        if let Some(ref p) = pose {
            let views = p.views();
            if views.length() > 0 {
                let v0: XrView = views.get(0).unchecked_into();
                let raw_v0 = to_mat4(v0.transform().inverse().matrix());

                let fwd_h = [-raw_v0[2], 0.0, -raw_v0[10]];
                let len_fwd = (fwd_h[0] * fwd_h[0] + fwd_h[2] * fwd_h[2]).sqrt();
                let (uf_x, uf_z) = if len_fwd > 1e-4 {
                    (fwd_h[0] / len_fwd, fwd_h[2] / len_fwd)
                } else {
                    (0.0, -1.0)
                };
                let (ur_x, ur_z) = (-uf_z, uf_x);
                let (cos_y, sin_y) = (locomotion_yaw.cos(), locomotion_yaw.sin());
                move_fwd = [
                    uf_x * cos_y - uf_z * sin_y,
                    0.0,
                    uf_x * sin_y + uf_z * cos_y,
                ];
                move_right = [
                    ur_x * cos_y - ur_z * sin_y,
                    0.0,
                    ur_x * sin_y + ur_z * cos_y,
                ];
            }
        }

        // Process controller inputs (Left stick = Move, Right stick = Turn & HUD scroll, Trigger = Click, Grip = HUD drag)
        let input_sources = session_loop.input_sources();
        let num_sources = input_sources.length();
        let canvas_w = canvas.width() as f64;
        let canvas_h = canvas.height() as f64;
        let mut hit_cursor: Option<(f32, f32)> = None;

        for i in 0..num_sources {
            if let Some(source) = input_sources.get(i) {
                let source: XrInputSource = source.unchecked_into();
                let handedness = source.handedness();
                let is_left = handedness == XrHandedness::Left
                    || (handedness == XrHandedness::None && i == 1);
                let is_right = handedness == XrHandedness::Right
                    || (handedness == XrHandedness::None && i == 0);

                let mut stick_x = 0.0f32;
                let mut stick_y = 0.0f32;

                if let Some(gamepad) = source.gamepad() {
                    let axes = gamepad.axes();
                    if axes.length() >= 4 {
                        stick_x = js_sys::Reflect::get(&axes, &2.into())
                            .ok()
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0) as f32;
                        stick_y = js_sys::Reflect::get(&axes, &3.into())
                            .ok()
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0) as f32;
                    } else if axes.length() >= 2 {
                        stick_x = js_sys::Reflect::get(&axes, &0.into())
                            .ok()
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0) as f32;
                        stick_y = js_sys::Reflect::get(&axes, &1.into())
                            .ok()
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0) as f32;
                    }

                    if grab_state.is_none() {
                        if is_left {
                            // Left stick: Translation (forward/backward, strafe left/right)
                            let mag = (stick_x * stick_x + stick_y * stick_y).sqrt();
                            let deadzone = 0.15f32;
                            if mag > deadzone {
                                let norm_mag =
                                    ((mag - deadzone) / (1.0 - deadzone)).clamp(0.0, 1.0);
                                let curved_speed = norm_mag * norm_mag * 5.0; // 5.0 m/s

                                let forward_input = -stick_y; // Stick forward is -Y in WebXR
                                let strafe_input = stick_x; // Stick right is +X in WebXR

                                let move_dir_x = (move_fwd[0] * forward_input
                                    + move_right[0] * strafe_input)
                                    / mag;
                                let move_dir_z = (move_fwd[2] * forward_input
                                    + move_right[2] * strafe_input)
                                    / mag;

                                locomotion_pos[0] += move_dir_x * curved_speed * dt;
                                locomotion_pos[2] += move_dir_z * curved_speed * dt;
                            }
                        } else if is_right {
                            // Right stick horizontal: Rotate locomotion yaw / facing direction
                            let deadzone_x = 0.15f32;
                            let abs_x = stick_x.abs();
                            if abs_x > deadzone_x {
                                let norm_x =
                                    ((abs_x - deadzone_x) / (1.0 - deadzone_x)).clamp(0.0, 1.0);
                                let turn_speed = 2.0f32; // 2.0 rad/s
                                let turn_rate = stick_x.signum() * (norm_x * norm_x) * turn_speed;
                                locomotion_yaw += turn_rate * dt;
                            }

                            // Right stick vertical: HUD scrolling as before
                            let deadzone_y = 0.18f32;
                            let abs_y = stick_y.abs();
                            if abs_y > deadzone_y {
                                let sign = -stick_y.signum();
                                let mag = (abs_y - deadzone_y) / (1.0 - deadzone_y);
                                let curved = mag * mag;
                                let delta_y = (sign * curved * 0.45) as f32;
                                let mut sink = pointer_sink.borrow_mut();
                                sink.push(WebXrPointerAction::MouseWheel { delta_y });
                            }
                        }
                    }
                }

                if let Some(anchor) = hud_anchor {
                    let target_ray_space = source.target_ray_space();
                    if let Some(ray_pose) = frame.get_pose(&target_ray_space, &ref_space) {
                        let transform = ray_pose.transform();
                        let pos = transform.position();
                        let orient = transform.orientation();

                        let (ox, oy, oz) = (pos.x() as f32, pos.y() as f32, pos.z() as f32);
                        let (qx, qy, qz, qw) = (
                            orient.x() as f32,
                            orient.y() as f32,
                            orient.z() as f32,
                            orient.w() as f32,
                        );
                        let dx = 2.0 * (qx * qz - qw * qy);
                        let dy = 2.0 * (qy * qz + qw * qx);
                        let dz = -(1.0 - 2.0 * (qx * qx + qy * qy));

                        let nx = anchor.normal[0];
                        let ny = anchor.normal[1];
                        let nz = anchor.normal[2];

                        let mut is_targeting_quad = false;
                        let denom = nx * dx + ny * dy + nz * dz;
                        if denom.abs() > 1e-4 {
                            let t = ((anchor.center[0] - ox) * nx
                                + (anchor.center[1] - oy) * ny
                                + (anchor.center[2] - oz) * nz)
                                / denom;
                            if t > 0.05 && t < 3.5 {
                                let hx = ox + t * dx;
                                let hy = oy + t * dy;
                                let hz = oz + t * dz;

                                let rel_x = hx - anchor.center[0];
                                let rel_y = hy - anchor.center[1];
                                let rel_z = hz - anchor.center[2];

                                let local_x = rel_x * anchor.right[0]
                                    + rel_y * anchor.right[1]
                                    + rel_z * anchor.right[2];
                                let local_y = rel_x * anchor.up[0]
                                    + rel_y * anchor.up[1]
                                    + rel_z * anchor.up[2];

                                let quad_w = QUAD_WIDTH * anchor.scale;
                                let quad_h = QUAD_HEIGHT * anchor.scale;

                                let u = (local_x / quad_w) + 0.5;
                                let v = (local_y / quad_h) + 0.5;

                                is_targeting_quad =
                                    (-0.1..=1.1).contains(&u) && (-0.1..=1.1).contains(&v);

                                if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v) {
                                    hit_cursor = Some((u, v));
                                    let screen_x = u as f64 * canvas_w;
                                    let screen_y = (1.0 - v as f64) * canvas_h;

                                    let mut sink = pointer_sink.borrow_mut();
                                    sink.push(WebXrPointerAction::CursorMoved {
                                        x: screen_x,
                                        y: screen_y,
                                    });

                                    if let Some(gamepad) = source.gamepad() {
                                        let buttons = gamepad.buttons();
                                        if buttons.length() > 0 {
                                            if let Ok(btn_val) =
                                                js_sys::Reflect::get(&buttons, &0.into())
                                            {
                                                let btn: web_sys::GamepadButton =
                                                    btn_val.unchecked_into();
                                                let pressed = btn.pressed();
                                                if pressed != last_trigger_pressed {
                                                    last_trigger_pressed = pressed;
                                                    sink.push(WebXrPointerAction::MouseInput {
                                                        state: if pressed {
                                                            ElementState::Pressed
                                                        } else {
                                                            ElementState::Released
                                                        },
                                                        button: MouseButton::Left,
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Direct hand grabbing & spatial move with Grip button (Button 1 / Squeeze)
                        if let Some(gamepad) = source.gamepad() {
                            let buttons = gamepad.buttons();
                            if buttons.length() > 1 {
                                if let Ok(btn_val) = js_sys::Reflect::get(&buttons, &1.into()) {
                                    let btn: web_sys::GamepadButton = btn_val.unchecked_into();
                                    let grip_pressed = btn.pressed();

                                    if grip_pressed {
                                        match grab_state {
                                            None => {
                                                let dist_to_center = ((ox - anchor.center[0])
                                                    .powi(2)
                                                    + (oy - anchor.center[1]).powi(2)
                                                    + (oz - anchor.center[2]).powi(2))
                                                .sqrt();
                                                let can_grab =
                                                    is_targeting_quad || dist_to_center < 0.6;
                                                if can_grab {
                                                    let offset_0 = [
                                                        anchor.center[0] - ox,
                                                        anchor.center[1] - oy,
                                                        anchor.center[2] - oz,
                                                    ];
                                                    let init_d = (offset_0[0] * offset_0[0]
                                                        + offset_0[1] * offset_0[1]
                                                        + offset_0[2] * offset_0[2])
                                                        .sqrt()
                                                        .max(0.1);

                                                    grab_state = Some(GrabState {
                                                        source_index: i,
                                                        initial_ctrl_pos: [ox, oy, oz],
                                                        initial_ctrl_orient: [qx, qy, qz, qw],
                                                        initial_anchor_center: anchor.center,
                                                        initial_anchor_right: anchor.right,
                                                        initial_anchor_up: anchor.up,
                                                        initial_anchor_normal: anchor.normal,
                                                        current_distance: init_d,
                                                        current_scale: anchor.scale,
                                                    });
                                                }
                                            }
                                            Some(mut gs) if gs.source_index == i => {
                                                // 1. Thumbstick Y: distance along beam
                                                let deadzone = 0.15;
                                                let abs_y = stick_y.abs();
                                                if abs_y > deadzone {
                                                    let sign = -stick_y.signum();
                                                    let mag = (abs_y - deadzone) / (1.0 - deadzone);
                                                    let delta = (sign * mag * mag * 0.025) as f32;
                                                    gs.current_distance = (gs.current_distance
                                                        + delta)
                                                        .clamp(0.20, 8.0);
                                                }

                                                // 2. Thumbstick X: scale HUD
                                                let abs_x = stick_x.abs();
                                                if abs_x > deadzone {
                                                    let sign = stick_x.signum();
                                                    let mag = (abs_x - deadzone) / (1.0 - deadzone);
                                                    let factor = 1.0 + (sign * mag * 0.015) as f32;
                                                    gs.current_scale = (gs.current_scale * factor)
                                                        .clamp(0.25, 4.0);
                                                }

                                                let q_curr = [qx, qy, qz, qw];
                                                let q_inv = [
                                                    -gs.initial_ctrl_orient[0],
                                                    -gs.initial_ctrl_orient[1],
                                                    -gs.initial_ctrl_orient[2],
                                                    gs.initial_ctrl_orient[3],
                                                ];
                                                let delta_q = quat_mul(q_curr, q_inv);

                                                let offset_0 = [
                                                    gs.initial_anchor_center[0]
                                                        - gs.initial_ctrl_pos[0],
                                                    gs.initial_anchor_center[1]
                                                        - gs.initial_ctrl_pos[1],
                                                    gs.initial_anchor_center[2]
                                                        - gs.initial_ctrl_pos[2],
                                                ];
                                                let init_d = (offset_0[0] * offset_0[0]
                                                    + offset_0[1] * offset_0[1]
                                                    + offset_0[2] * offset_0[2])
                                                    .sqrt()
                                                    .max(1e-4);
                                                let unit_dir_0 = [
                                                    offset_0[0] / init_d,
                                                    offset_0[1] / init_d,
                                                    offset_0[2] / init_d,
                                                ];
                                                let unit_dir_rot =
                                                    quat_rotate_vec(delta_q, unit_dir_0);
                                                let new_center = [
                                                    ox + unit_dir_rot[0] * gs.current_distance,
                                                    oy + unit_dir_rot[1] * gs.current_distance,
                                                    oz + unit_dir_rot[2] * gs.current_distance,
                                                ];

                                                let right = quat_rotate_vec(
                                                    delta_q,
                                                    gs.initial_anchor_right,
                                                );
                                                let up =
                                                    quat_rotate_vec(delta_q, gs.initial_anchor_up);
                                                let normal = quat_rotate_vec(
                                                    delta_q,
                                                    gs.initial_anchor_normal,
                                                );

                                                let s = gs.current_scale;
                                                let model = [
                                                    right[0] * s,
                                                    right[1] * s,
                                                    right[2] * s,
                                                    0.0,
                                                    up[0] * s,
                                                    up[1] * s,
                                                    up[2] * s,
                                                    0.0,
                                                    normal[0],
                                                    normal[1],
                                                    normal[2],
                                                    0.0,
                                                    new_center[0],
                                                    new_center[1],
                                                    new_center[2],
                                                    1.0,
                                                ];

                                                hud_anchor = Some(HudAnchor {
                                                    center: new_center,
                                                    normal,
                                                    right,
                                                    up,
                                                    model,
                                                    scale: gs.current_scale,
                                                });
                                                grab_state = Some(gs);
                                            }
                                            _ => {}
                                        }
                                    } else if let Some(gs) = grab_state {
                                        if gs.source_index == i {
                                            grab_state = None;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Update stereo poses for Karakuri stereo camera feed, applying locomotion
        if let Some(ref p) = pose {
            let views = p.views();
            if views.length() >= 2 {
                let v0: XrView = views.get(0).unchecked_into();
                let v1: XrView = views.get(1).unchecked_into();

                let p0 = v0.transform().position();
                let p1 = v1.transform().position();

                let p0_arr = [p0.x() as f32, p0.y() as f32, p0.z() as f32];
                let p1_arr = [p1.x() as f32, p1.y() as f32, p1.z() as f32];
                let head_center = [
                    (p0_arr[0] + p1_arr[0]) * 0.5,
                    (p0_arr[1] + p1_arr[1]) * 0.5,
                    (p0_arr[2] + p1_arr[2]) * 0.5,
                ];

                let raw_v0 = to_mat4(v0.transform().inverse().matrix());
                let raw_v1 = to_mat4(v1.transform().inverse().matrix());

                let (left_view, left_eye) =
                    apply_locomotion(raw_v0, p0_arr, head_center, locomotion_pos, locomotion_yaw);
                let (right_view, right_eye) =
                    apply_locomotion(raw_v1, p1_arr, head_center, locomotion_pos, locomotion_yaw);

                let eye_size = layer
                    .get_viewport(&v0)
                    .map(|vp| (vp.width().max(1) as u32, vp.height().max(1) as u32))
                    .unwrap_or((960, 1080));
                let stereo = StereoPose {
                    left: StereoEye {
                        view: left_view,
                        proj: to_mat4(v0.projection_matrix()),
                        eye: left_eye,
                    },
                    right: StereoEye {
                        view: right_view,
                        proj: to_mat4(v1.projection_matrix()),
                        eye: right_eye,
                    },
                    eye_size,
                };
                *current_stereo_pose.borrow_mut() = Some(stereo);
            }
        }

        // Upload latest egui console canvas frame to WebGL texture
        renderer.borrow().update_texture(&gl, &canvas);

        // Keep driving WebGPU console redraws even when window RAF is backgrounded
        let _ = proxy.send_event(());

        if frame_count % 144 == 1 {
            let (ax, ay, az) = hud_anchor
                .map(|a| (a.center[0], a.center[1], a.center[2]))
                .unwrap_or((0.0, 0.0, 0.0));
            web_sys::console::log_1(&format!(
                "WebXR HUD frame {frame_count}: canvas {}x{}, anchor at ({ax:.2}, {ay:.2}, {az:.2}), loco at ({:.2}, {:.2}, {:.2}), yaw {:.1} deg",
                canvas.width(), canvas.height(),
                locomotion_pos[0], locomotion_pos[1], locomotion_pos[2],
                locomotion_yaw.to_degrees()
            ).into());
        }

        // Visual projection mode: Mode 0 (Natural Stereo Perspective Reprojection) for VR, Mode 3 for AR
        let target_mode = match session_mode {
            WebXrSessionMode::Ar => 3, // MR Passthrough
            WebXrSessionMode::Vr => 0, // Natural Stereo Perspective (preserves WebGPU 6DoF stereo camera and Quest off-centre frustum)
        };
        world_renderer.borrow_mut().set_dome_mode(target_mode);

        if let Some(proj) = *vr_projection.borrow() {
            world_renderer.borrow_mut().set_environment_params(
                proj.stars,
                proj.density,
                proj.grid,
                proj.lines,
            );
        }

        // Render stereo eye views
        if let (Some(pose), Some(anchor)) = (pose, hud_anchor) {
            let views = pose.views();
            let num_views = views.length();

            if let Some(ref xr_c) = world_canvas {
                world_renderer
                    .borrow_mut()
                    .update_texture_from_canvas(&gl, xr_c);
            }
            // The picture just uploaded was drawn for an older head pose than
            // this frame's; each eye reprojects from that pose to its own.
            let shown = *current_stereo_pose.borrow();
            let drawn_for = (*rendered_stereo_pose.borrow()).or(shown);

            gl.bind_framebuffer(
                WebGl2RenderingContext::FRAMEBUFFER,
                layer.framebuffer().as_ref(),
            );
            gl.enable(WebGl2RenderingContext::DEPTH_TEST);
            let is_passthrough = world_renderer.borrow().dome_mode() == 3;
            if is_passthrough {
                gl.clear_color(0.0, 0.0, 0.0, 0.0);
            } else {
                gl.clear_color(0.01, 0.01, 0.02, 1.0);
            }
            gl.clear(
                WebGl2RenderingContext::COLOR_BUFFER_BIT | WebGl2RenderingContext::DEPTH_BUFFER_BIT,
            );

            for v in 0..num_views {
                let view: XrView = views.get(v).unchecked_into();
                let viewport = layer.get_viewport(&view).unwrap();

                gl.viewport(
                    viewport.x(),
                    viewport.y(),
                    viewport.width(),
                    viewport.height(),
                );

                let pick = |p: Option<StereoPose>| p.map(|p| if v == 0 { p.left } else { p.right });
                world_renderer.borrow().draw_eye(
                    &gl,
                    v as i32,
                    (_time * 0.001) as f32,
                    pick(shown).zip(pick(drawn_for)),
                );
                draw_hud_quad(&gl, &renderer.borrow(), &view, &anchor.model, hit_cursor);
            }
        }

        if let Some(ref cb) = *f.borrow() {
            let _ = session_loop.request_animation_frame(cb.as_ref().unchecked_ref());
        }
    }) as Box<dyn FnMut(f64, XrFrame)>));

    {
        let cb_ref = g.borrow();
        if let Some(ref cb) = *cb_ref {
            let _ = session.request_animation_frame(cb.as_ref().unchecked_ref());
        }
    }
}

/// Applies artificial locomotion (translation and yaw rotation around player's head center)
/// to an eye's raw view matrix and world-space eye position.
pub fn apply_locomotion(
    eye_view: [f32; 16],
    eye_pos: [f32; 3],
    head_center: [f32; 3],
    locomotion_pos: [f32; 3],
    locomotion_yaw: f32,
) -> ([f32; 16], [f32; 3]) {
    let c = head_center;
    let d = [
        c[0] + locomotion_pos[0],
        c[1] + locomotion_pos[1],
        c[2] + locomotion_pos[2],
    ];
    let (cos_y, sin_y) = (locomotion_yaw.cos(), locomotion_yaw.sin());

    // M_loco_inv transforms world points back into raw reference space:
    // M_loco_inv = T(c) * Ry(-locomotion_yaw) * T(-d)
    // Column-major order:
    let m_inv = [
        // Col 0
        cos_y,
        0.0,
        -sin_y,
        0.0,
        // Col 1
        0.0,
        1.0,
        0.0,
        0.0,
        // Col 2
        sin_y,
        0.0,
        cos_y,
        0.0,
        // Col 3
        c[0] - d[0] * cos_y - d[2] * sin_y,
        c[1] - d[1],
        c[2] + d[0] * sin_y - d[2] * cos_y,
        1.0,
    ];

    // V_new = eye_view * M_loco_inv
    let mut v_new = [0.0f32; 16];
    for col in 0..4 {
        for row in 0..4 {
            let mut sum = 0.0f32;
            for k in 0..4 {
                sum += eye_view[k * 4 + row] * m_inv[col * 4 + k];
            }
            v_new[col * 4 + row] = sum;
        }
    }

    // eye_new = M_loco * eye_pos
    // M_loco = T(d) * Ry(locomotion_yaw) * T(-c)
    let delta = [eye_pos[0] - c[0], eye_pos[1] - c[1], eye_pos[2] - c[2]];
    let rot_delta = [
        delta[0] * cos_y - delta[2] * sin_y,
        delta[1],
        delta[0] * sin_y + delta[2] * cos_y,
    ];
    let eye_new = [
        rot_delta[0] + d[0],
        rot_delta[1] + d[1],
        rot_delta[2] + d[2],
    ];

    (v_new, eye_new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_locomotion_identity() {
        let view_identity = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let eye_pos = [0.03, 1.7, 0.0];
        let head_center = [0.0, 1.7, 0.0];
        let (view_out, eye_out) =
            apply_locomotion(view_identity, eye_pos, head_center, [0.0, 0.0, 0.0], 0.0);

        for (a, b) in view_out.iter().zip(view_identity.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
        for (a, b) in eye_out.iter().zip(eye_pos.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn test_apply_locomotion_translation() {
        let view_identity = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let eye_pos = [0.03, 1.7, 0.0];
        let head_center = [0.0, 1.7, 0.0];
        let (view_out, eye_out) =
            apply_locomotion(view_identity, eye_pos, head_center, [2.0, 0.0, -5.0], 0.0);

        assert!((eye_out[0] - 2.03).abs() < 1e-5);
        assert!((eye_out[1] - 1.7).abs() < 1e-5);
        assert!((eye_out[2] - (-5.0)).abs() < 1e-5);

        // A point in front of the moved eye at [2.03, 1.7, -6.0] should map to [0.0, 0.0, -1.0]
        let pt = [2.03f32, 1.7, -6.0, 1.0];
        let mut eye_space_z = 0.0f32;
        for k in 0..4 {
            eye_space_z += view_out[k * 4 + 2] * pt[k];
        }
        assert!((eye_space_z - (-1.0)).abs() < 1e-5);
    }

    #[test]
    fn test_apply_locomotion_rotation() {
        let view_identity = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let eye_pos = [0.03, 1.7, 0.0];
        let head_center = [0.0, 1.7, 0.0];
        let yaw = std::f32::consts::FRAC_PI_2; // 90 degrees clockwise (turn right)
        let (view_out, eye_out) =
            apply_locomotion(view_identity, eye_pos, head_center, [0.0, 0.0, 0.0], yaw);

        // Head center remains at [0.0, 1.7, 0.0], while right offset [0.03, 0, 0] rotates clockwise to back [0, 0, 0.03]
        assert!((eye_out[0] - 0.0).abs() < 1e-5);
        assert!((eye_out[1] - 1.7).abs() < 1e-5);
        assert!((eye_out[2] - 0.03).abs() < 1e-5);

        // Looking right: a point at [10.0, 1.7, 0.0] in world space should be directly in front of camera (-Z in eye space)
        let pt = [10.0f32, 1.7, 0.0, 1.0];
        let mut eye_space_z = 0.0f32;
        for k in 0..4 {
            eye_space_z += view_out[k * 4 + 2] * pt[k];
        }
        assert!((eye_space_z - (-10.0)).abs() < 1e-4);
    }
}
