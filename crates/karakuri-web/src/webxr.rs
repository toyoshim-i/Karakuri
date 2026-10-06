//! WebXR Immersive Spatial Mode (Tier 1: 30-inch Deck HUD Virtual Console).
//!
//! Provides WebXR Device API integration on supported VR browsers (such as Meta Quest Browser):
//! - Detects `immersive-vr` session capability.
//! - Launches WebXR immersive sessions with a `local-floor` reference space.
//! - Renders a 30-inch virtual console Quad mesh (~70cm x 40cm) tilted 35° upwards at 70cm distance.
//! - Projects the 2D egui WebGPU console canvas onto the 3D Quad mesh in stereo views.
//! - Intersects controller target rays against the Quad plane to emulate mouse moves, clicks, and scrolls.

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext, WebGlProgram, WebGlShader, WebGlTexture, XrFrame,
    XrInputSource, XrReferenceSpace, XrRenderStateInit, XrSession, XrSessionInit, XrSessionMode,
    XrView, XrViewerPose, XrWebGlLayer, XrWebGlLayerInit,
};
use winit::event::{ElementState, MouseButton};
use winit::event_loop::EventLoopProxy;

/// Normalized pointer event generated from WebXR controller raycast on the 30" console quad.
#[derive(Debug, Clone)]
pub enum WebXrPointerAction {
    CursorMoved {
        x: f64,
        y: f64,
    },
    MouseInput {
        state: ElementState,
        button: MouseButton,
    },
    MouseWheel {
        delta_y: f32,
    },
}

/// Shared state for WebXR spatial HUD session.
pub struct WebXrState {
    pub is_supported: bool,
    pub is_active: bool,
    pub _session: Option<XrSession>,
    pub _ref_space: Option<XrReferenceSpace>,
    pub(crate) pending_pointer_events: Rc<RefCell<Vec<WebXrPointerAction>>>,
}

impl Default for WebXrState {
    fn default() -> Self {
        Self::new()
    }
}

impl WebXrState {
    pub fn new() -> Self {
        Self {
            is_supported: false,
            is_active: false,
            _session: None,
            _ref_space: None,
            pending_pointer_events: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Drains pending controller pointer events to inject into winit/egui.
    pub fn drain_pointer_events(&self) -> Vec<WebXrPointerAction> {
        self.pending_pointer_events.borrow_mut().drain(..).collect()
    }
}

/// Checks whether `immersive-vr` is supported on the current browser.
pub async fn check_webxr_support() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let promise = window
        .navigator()
        .xr()
        .is_session_supported(XrSessionMode::ImmersiveVr);
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false)
}

/// Requests and initializes a WebXR `immersive-vr` session.
pub async fn start_webxr_session(
    main_canvas: HtmlCanvasElement,
    pointer_sink: Rc<RefCell<Vec<WebXrPointerAction>>>,
    proxy: EventLoopProxy<()>,
) -> Result<XrSession, String> {
    let window = web_sys::window().ok_or("No global window")?;
    let xr = window.navigator().xr();

    let session_init = XrSessionInit::new();
    session_init.set_required_features(&[JsValue::from_str("local-floor")]);

    let session_promise =
        xr.request_session_with_options(XrSessionMode::ImmersiveVr, &session_init);
    let session_val = wasm_bindgen_futures::JsFuture::from(session_promise)
        .await
        .map_err(|e| format!("Failed to request WebXR session: {e:?}"))?;
    let session: XrSession = session_val.unchecked_into();

    // Create an offscreen WebGL2 canvas for stereo XR projection
    let document = window.document().ok_or("No document")?;
    let xr_canvas: HtmlCanvasElement = document
        .create_element("canvas")
        .map_err(|e| format!("{e:?}"))?
        .unchecked_into();
    xr_canvas.set_width(1920);
    xr_canvas.set_height(1080);

    let gl_opts = js_sys::Object::new();
    js_sys::Reflect::set(&gl_opts, &"xrCompatible".into(), &JsValue::from_bool(true)).unwrap();
    let gl: WebGl2RenderingContext = xr_canvas
        .get_context_with_context_options("webgl2", &gl_opts)
        .map_err(|e| format!("{e:?}"))?
        .ok_or("Failed to get WebGL2 context for XR")?
        .unchecked_into();

    let layer_init = XrWebGlLayerInit::new();
    layer_init.set_antialias(true);
    let xr_gl_layer =
        XrWebGlLayer::new_with_web_gl2_rendering_context_and_layer_init(&session, &gl, &layer_init)
            .map_err(|e| format!("Failed to create XrWebGlLayer: {e:?}"))?;

    let render_state = XrRenderStateInit::new();
    render_state.set_base_layer(Some(&xr_gl_layer));
    session.update_render_state_with_state(&render_state);

    // Request reference space (local-floor, with fallback to local)
    let ref_space = match wasm_bindgen_futures::JsFuture::from(
        session.request_reference_space(web_sys::XrReferenceSpaceType::LocalFloor),
    )
    .await
    {
        Ok(val) => val.unchecked_into::<XrReferenceSpace>(),
        Err(_) => {
            let fallback = wasm_bindgen_futures::JsFuture::from(
                session.request_reference_space(web_sys::XrReferenceSpaceType::Local),
            )
            .await
            .map_err(|e| format!("Failed to get reference space: {e:?}"))?;
            fallback.unchecked_into::<XrReferenceSpace>()
        }
    };

    // Compile quad rendering shader pipeline
    let renderer = Rc::new(RefCell::new(XrQuadRenderer::new(&gl)?));

    // Launch RAF render loop
    setup_xr_render_loop(
        session.clone(),
        ref_space,
        gl,
        xr_gl_layer,
        renderer,
        main_canvas,
        pointer_sink,
        proxy,
    );

    Ok(session)
}

struct XrQuadRenderer {
    program: WebGlProgram,
    texture: WebGlTexture,
    u_mvp: web_sys::WebGlUniformLocation,
    u_cursor: web_sys::WebGlUniformLocation,
    u_cursor_active: web_sys::WebGlUniformLocation,
    _vertex_buffer: web_sys::WebGlBuffer,
    _index_buffer: web_sys::WebGlBuffer,
    vao: web_sys::WebGlVertexArrayObject,
}

impl XrQuadRenderer {
    fn new(gl: &WebGl2RenderingContext) -> Result<Self, String> {
        let vs_source = r#"#version 300 es
        in vec3 a_position;
        in vec2 a_uv;
        uniform mat4 u_mvp;
        out vec2 v_uv;
        void main() {
            v_uv = a_uv;
            gl_Position = u_mvp * vec4(a_position, 1.0);
        }
        "#;

        let fs_source = r#"#version 300 es
        precision highp float;
        in vec2 v_uv;
        uniform sampler2D u_texture;
        uniform vec2 u_cursor;
        uniform int u_cursor_active;
        out vec4 frag_color;

        void main() {
            // Bezel frame styling
            vec2 b_dist = min(v_uv, 1.0 - v_uv);
            float border_w = 0.008;
            float is_border = 1.0 - step(border_w, min(b_dist.x, b_dist.y));

            // Sample console texture (flip Y so canvas top matches quad top)
            vec2 tex_uv = vec2(v_uv.x, 1.0 - v_uv.y);
            vec4 tex_sample = texture(u_texture, tex_uv);

            // Dark slate console base background
            vec3 panel_base = vec3(0.06, 0.07, 0.10);

            // Robust content calculation: show texel colors even if alpha channel is zero
            vec3 content_rgb = panel_base;
            float lum = dot(tex_sample.rgb, vec3(0.299, 0.587, 0.114));
            if (tex_sample.a > 0.05) {
                content_rgb = mix(panel_base, tex_sample.rgb, tex_sample.a);
            } else if (lum > 0.01) {
                content_rgb = tex_sample.rgb;
            }

            // Glowing cyan/mint cyberpunk bezel
            vec3 bezel_color = vec3(0.0, 0.94, 0.82);
            vec3 final_rgb = mix(content_rgb, bezel_color, is_border);

            // Controller laser hit reticle
            if (u_cursor_active == 1) {
                vec2 diff = v_uv - u_cursor;
                diff.x *= (0.72 / 0.42); // Aspect correction
                float dist = length(diff);
                float ring = smoothstep(0.015, 0.012, dist) * smoothstep(0.008, 0.011, dist);
                float dot = smoothstep(0.005, 0.003, dist);
                vec3 reticle_color = vec3(1.0, 0.2, 0.55);
                final_rgb = mix(final_rgb, reticle_color, max(ring, dot));
            }

            // WebXR MUST have alpha = 1.0 to prevent compositor transparency
            frag_color = vec4(final_rgb, 1.0);
        }
        "#;

        let vs = compile_shader(gl, WebGl2RenderingContext::VERTEX_SHADER, vs_source)?;
        let fs = compile_shader(gl, WebGl2RenderingContext::FRAGMENT_SHADER, fs_source)?;
        let program = gl.create_program().ok_or("Failed to create program")?;
        gl.attach_shader(&program, &vs);
        gl.attach_shader(&program, &fs);
        gl.link_program(&program);

        if !gl
            .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            return Err("Shader link failed".to_string());
        }

        let u_mvp = gl
            .get_uniform_location(&program, "u_mvp")
            .ok_or("No u_mvp uniform")?;
        let u_cursor = gl
            .get_uniform_location(&program, "u_cursor")
            .ok_or("No u_cursor uniform")?;
        let u_cursor_active = gl
            .get_uniform_location(&program, "u_cursor_active")
            .ok_or("No u_cursor_active uniform")?;

        let texture = gl.create_texture().ok_or("Failed to create texture")?;
        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&texture));
        gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_MIN_FILTER,
            WebGl2RenderingContext::LINEAR as i32,
        );
        gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_MAG_FILTER,
            WebGl2RenderingContext::LINEAR as i32,
        );
        gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_WRAP_S,
            WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_WRAP_T,
            WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
        );

        // Seed initial texture with high-contrast test pattern
        let test_pattern: [u8; 16] = [
            255, 30, 140, 255, // neon magenta
            0, 240, 200, 255, // cyan
            0, 240, 200, 255, // cyan
            255, 30, 140, 255, // neon magenta
        ];
        let _ = gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            WebGl2RenderingContext::TEXTURE_2D,
            0,
            WebGl2RenderingContext::RGBA as i32,
            2,
            2,
            0,
            WebGl2RenderingContext::RGBA,
            WebGl2RenderingContext::UNSIGNED_BYTE,
            Some(&test_pattern),
        );

        // Preallocate static geometry VAO, VBO, IBO once
        let hw = QUAD_WIDTH * 0.5;
        let hh = QUAD_HEIGHT * 0.5;
        let vertices: [f32; 20] = [
            -hw, -hh, 0.0, 0.0, 0.0, hw, -hh, 0.0, 1.0, 0.0, hw, hh, 0.0, 1.0, 1.0, -hw, hh, 0.0,
            0.0, 1.0,
        ];
        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

        let vao = gl
            .create_vertex_array()
            .ok_or("Failed to create vertex array")?;
        gl.bind_vertex_array(Some(&vao));

        let vertex_buffer = gl.create_buffer().ok_or("Failed to create vertex buffer")?;
        gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&vertex_buffer));
        unsafe {
            let view = js_sys::Float32Array::view(&vertices);
            gl.buffer_data_with_array_buffer_view(
                WebGl2RenderingContext::ARRAY_BUFFER,
                &view,
                WebGl2RenderingContext::STATIC_DRAW,
            );
        }

        let index_buffer = gl.create_buffer().ok_or("Failed to create index buffer")?;
        gl.bind_buffer(
            WebGl2RenderingContext::ELEMENT_ARRAY_BUFFER,
            Some(&index_buffer),
        );
        unsafe {
            let view = js_sys::Uint16Array::view(&indices);
            gl.buffer_data_with_array_buffer_view(
                WebGl2RenderingContext::ELEMENT_ARRAY_BUFFER,
                &view,
                WebGl2RenderingContext::STATIC_DRAW,
            );
        }

        let a_pos = gl.get_attrib_location(&program, "a_position") as u32;
        let a_uv = gl.get_attrib_location(&program, "a_uv") as u32;

        gl.enable_vertex_attrib_array(a_pos);
        gl.vertex_attrib_pointer_with_i32(a_pos, 3, WebGl2RenderingContext::FLOAT, false, 20, 0);

        gl.enable_vertex_attrib_array(a_uv);
        gl.vertex_attrib_pointer_with_i32(a_uv, 2, WebGl2RenderingContext::FLOAT, false, 20, 12);

        gl.bind_vertex_array(None);

        Ok(Self {
            program,
            texture,
            u_mvp,
            u_cursor,
            u_cursor_active,
            _vertex_buffer: vertex_buffer,
            _index_buffer: index_buffer,
            vao,
        })
    }

    fn update_texture(&self, gl: &WebGl2RenderingContext, canvas: &HtmlCanvasElement) {
        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&self.texture));
        if let Err(e) = gl.tex_image_2d_with_u32_and_u32_and_html_canvas_element(
            WebGl2RenderingContext::TEXTURE_2D,
            0,
            WebGl2RenderingContext::RGBA as i32,
            WebGl2RenderingContext::RGBA,
            WebGl2RenderingContext::UNSIGNED_BYTE,
            canvas,
        ) {
            web_sys::console::warn_2(&"Karakuri WebXR tex_image_2d failed:".into(), &e);
        }
    }
}

fn compile_shader(
    gl: &WebGl2RenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, String> {
    let shader = gl
        .create_shader(shader_type)
        .ok_or("Failed to create shader")?;
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);
    let ok = gl
        .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false);
    if ok {
        Ok(shader)
    } else {
        Err(gl
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| "Unknown error".to_string()))
    }
}

/// 30-inch Deck HUD model dimensions in meters (~30" 16:9 widescreen)
const QUAD_WIDTH: f32 = 0.72;
const QUAD_HEIGHT: f32 = 0.42;

#[derive(Clone, Copy, Debug)]
struct HudAnchor {
    center: [f32; 3],
    normal: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    model: [f32; 16],
}

impl HudAnchor {
    /// Constructs a spatial HUD anchor positioned relative to the user's initial viewer pose:
    /// - Front distance: ~0.65m in the horizontal gaze direction
    /// - Height offset: -0.38m below the eyes (natural lap/desk/hand height whether sitting or standing)
    /// - Tilt: 30° upwards facing the user's eyes
    fn from_viewer_pose(pose: &XrViewerPose) -> Self {
        let transform = pose.transform();
        let pos = transform.position();
        let orient = transform.orientation();

        let (hx, hy, hz) = (pos.x() as f32, pos.y() as f32, pos.z() as f32);
        let (qx, qy, qz, qw) = (
            orient.x() as f32,
            orient.y() as f32,
            orient.z() as f32,
            orient.w() as f32,
        );

        // Forward vector in world coordinates (ignoring head pitch/roll)
        let fwd_x = 2.0 * (qx * qz + qw * qy);
        let fwd_z = -(1.0 - 2.0 * (qx * qx + qy * qy));
        let mut fwd_len = (fwd_x * fwd_x + fwd_z * fwd_z).sqrt();
        if fwd_len < 1e-4 {
            fwd_len = 1.0;
        }
        let fwd = (fwd_x / fwd_len, 0.0, fwd_z / fwd_len);

        // Horizontal right vector: perpendicular to horizontal forward
        let right = [-fwd.2, 0.0, fwd.0];

        // Hand-level console center
        let center = [hx + fwd.0 * 0.65, hy - 0.38, hz + fwd.2 * 0.65];

        // 30 degrees tilt up towards user's eyes
        let tilt: f32 = 30.0 * std::f32::consts::PI / 180.0;
        let cos_t = tilt.cos();
        let sin_t = tilt.sin();

        // Up vector: tilted backwards (+fwd) into the scene
        let up = [fwd.0 * sin_t, cos_t, fwd.2 * sin_t];

        // Normal vector: points upwards (+Y) and towards user (-fwd)
        let normal = [-fwd.0 * cos_t, sin_t, -fwd.2 * cos_t];

        // Column-major affine matrix [right, up, normal, center]
        let model = [
            right[0], right[1], right[2], 0.0, up[0], up[1], up[2], 0.0, normal[0], normal[1],
            normal[2], 0.0, center[0], center[1], center[2], 1.0,
        ];

        Self {
            center,
            normal,
            right,
            up,
            model,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct GrabState {
    source_index: u32,
    initial_ctrl_pos: [f32; 3],
    initial_anchor_center: [f32; 3],
}

type XrFrameClosure = Rc<RefCell<Option<Closure<dyn FnMut(f64, XrFrame)>>>>;

#[allow(clippy::too_many_arguments)]
fn setup_xr_render_loop(
    session: XrSession,
    ref_space: XrReferenceSpace,
    gl: WebGl2RenderingContext,
    layer: XrWebGlLayer,
    renderer: Rc<RefCell<XrQuadRenderer>>,
    canvas: HtmlCanvasElement,
    pointer_sink: Rc<RefCell<Vec<WebXrPointerAction>>>,
    proxy: EventLoopProxy<()>,
) {
    let f: XrFrameClosure = Rc::new(RefCell::new(None));
    let g = f.clone();

    let mut last_trigger_pressed = false;
    let mut hud_anchor: Option<HudAnchor> = None;
    let mut grab_state: Option<GrabState> = None;
    let mut frame_count: u64 = 0;
    let session_loop = session.clone();

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move |_time: f64, frame: XrFrame| {
        frame_count = frame_count.wrapping_add(1);
        let pose: Option<XrViewerPose> = frame.get_viewer_pose(&ref_space);

        // Lazily anchor the HUD to the user's initial head pose
        if hud_anchor.is_none() {
            if let Some(ref p) = pose {
                hud_anchor = Some(HudAnchor::from_viewer_pose(p));
            }
        }

        // Upload latest egui console canvas frame to WebGL texture
        renderer.borrow().update_texture(&gl, &canvas);

        // Keep driving WebGPU console redraws even when window RAF is backgrounded
        let _ = proxy.send_event(());

        // Heartbeat log every ~2 seconds (144 frames) to confirm texture pipeline on remote relay
        if frame_count % 144 == 1 {
            let (ax, ay, az) = hud_anchor
                .map(|a| (a.center[0], a.center[1], a.center[2]))
                .unwrap_or((0.0, 0.0, 0.0));
            web_sys::console::log_1(&format!(
                "WebXR HUD frame {frame_count}: canvas {}x{}, anchor at ({ax:.2}, {ay:.2}, {az:.2})",
                canvas.width(), canvas.height()
            ).into());
        }

        // Process controller ray intersections for Tier 1 HUD interaction
        let input_sources = session_loop.input_sources();
        let num_sources = input_sources.length();
        let canvas_w = canvas.width() as f64;
        let canvas_h = canvas.height() as f64;
        let mut hit_cursor: Option<(f32, f32)> = None;

        if let Some(anchor) = hud_anchor {
            for i in 0..num_sources {
                if let Some(source) = input_sources.get(i) {
                    let source: XrInputSource = source.unchecked_into();
                    let target_ray_space = source.target_ray_space();
                    if let Some(ray_pose) = frame.get_pose(&target_ray_space, &ref_space) {
                        let transform = ray_pose.transform();
                        let pos = transform.position();
                        let orient = transform.orientation();

                        // Compute ray origin and direction vector from orientation quaternion
                        let (ox, oy, oz) = (pos.x() as f32, pos.y() as f32, pos.z() as f32);
                        let (qx, qy, qz, qw) = (
                            orient.x() as f32,
                            orient.y() as f32,
                            orient.z() as f32,
                            orient.w() as f32,
                        );
                        // Forward direction vector (0, 0, -1) rotated by quaternion
                        let dx = 2.0 * (qx * qz - qw * qy);
                        let dy = 2.0 * (qy * qz + qw * qx);
                        let dz = -(1.0 - 2.0 * (qx * qx + qy * qy));

                        // Plane equation: (P - Center) . Normal = 0
                        let nx = anchor.normal[0];
                        let ny = anchor.normal[1];
                        let nz = anchor.normal[2];

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

                                let u = (local_x / QUAD_WIDTH) + 0.5;
                                let v = (local_y / QUAD_HEIGHT) + 0.5;

                                if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v) {
                                    hit_cursor = Some((u, v));
                                    let screen_x = u as f64 * canvas_w;
                                    let screen_y = (1.0 - v as f64) * canvas_h;

                                    let mut sink = pointer_sink.borrow_mut();
                                    sink.push(WebXrPointerAction::CursorMoved {
                                        x: screen_x,
                                        y: screen_y,
                                    });

                                    // Check buttons from gamepad
                                    if let Some(gamepad) = source.gamepad() {
                                        let buttons = gamepad.buttons();
                                        if buttons.length() > 0 {
                                            // Button 0: Trigger (Primary click)
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

                                        // Thumbstick scroll (axes 2 or 3)
                                        let axes = gamepad.axes();
                                        if axes.length() >= 4 {
                                            if let Ok(stick_y_val) =
                                                js_sys::Reflect::get(&axes, &3.into())
                                            {
                                                if let Some(stick_y) = stick_y_val.as_f64() {
                                                    if stick_y.abs() > 0.15 {
                                                        sink.push(WebXrPointerAction::MouseWheel {
                                                            delta_y: (-stick_y * 15.0) as f32,
                                                        });
                                                    }
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
                                if let Ok(grip_val) = js_sys::Reflect::get(&buttons, &1.into()) {
                                    let grip_btn: web_sys::GamepadButton =
                                        grip_val.unchecked_into();
                                    let grip_pressed = grip_btn.pressed();

                                    if grip_pressed {
                                        if let Some(grab) = grab_state {
                                            if grab.source_index == i {
                                                // Currently dragging quad: update center in real-time
                                                let shift_x = ox - grab.initial_ctrl_pos[0];
                                                let shift_y = oy - grab.initial_ctrl_pos[1];
                                                let shift_z = oz - grab.initial_ctrl_pos[2];

                                                if let Some(ref mut a) = hud_anchor {
                                                    a.center = [
                                                        grab.initial_anchor_center[0] + shift_x,
                                                        grab.initial_anchor_center[1] + shift_y,
                                                        grab.initial_anchor_center[2] + shift_z,
                                                    ];
                                                    a.model[12] = a.center[0];
                                                    a.model[13] = a.center[1];
                                                    a.model[14] = a.center[2];
                                                }
                                            }
                                        } else if hit_cursor.is_some() {
                                            // Grip just pressed while targeting quad: initiate grab!
                                            grab_state = Some(GrabState {
                                                source_index: i,
                                                initial_ctrl_pos: [ox, oy, oz],
                                                initial_anchor_center: anchor.center,
                                            });
                                        }
                                    } else if let Some(grab) = grab_state {
                                        if grab.source_index == i {
                                            // Grip released: release quad and lock in place!
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

        // Render stereo eye views
        if let (Some(pose), Some(anchor)) = (pose, hud_anchor) {
            let views = pose.views();
            let num_views = views.length();

            gl.bind_framebuffer(
                WebGl2RenderingContext::FRAMEBUFFER,
                layer.framebuffer().as_ref(),
            );
            gl.enable(WebGl2RenderingContext::DEPTH_TEST);

            // Clear color to deep cosmic navy with alpha = 1.0 to avoid compositor blacking
            gl.clear_color(0.02, 0.02, 0.05, 1.0);
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

                // Quad mesh rendering for Tier 1 HUD Console
                draw_hud_quad(&gl, &renderer.borrow(), &view, &anchor.model, hit_cursor);
            }
        }

        // Schedule next XR frame
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

fn draw_hud_quad(
    gl: &WebGl2RenderingContext,
    renderer: &XrQuadRenderer,
    view: &XrView,
    model: &[f32; 16],
    hit_cursor: Option<(f32, f32)>,
) {
    gl.use_program(Some(&renderer.program));

    // Construct Model-View-Projection matrix
    let proj = view.projection_matrix();
    let view_inv = view.transform().inverse();
    let view_matrix = view_inv.matrix();

    let mvp = compute_quad_mvp(&proj, &view_matrix, model);
    gl.uniform_matrix4fv_with_f32_array(Some(&renderer.u_mvp), false, &mvp);

    // Update cursor reticle uniforms
    if let Some((u, v)) = hit_cursor {
        gl.uniform2f(Some(&renderer.u_cursor), u, v);
        gl.uniform1i(Some(&renderer.u_cursor_active), 1);
    } else {
        gl.uniform1i(Some(&renderer.u_cursor_active), 0);
    }

    // Bind static VAO preallocated during renderer initialization
    gl.bind_vertex_array(Some(&renderer.vao));
    gl.draw_elements_with_i32(
        WebGl2RenderingContext::TRIANGLES,
        6,
        WebGl2RenderingContext::UNSIGNED_SHORT,
        0,
    );
    gl.bind_vertex_array(None);
}

fn compute_quad_mvp(proj: &[f32], view: &[f32], model: &[f32]) -> [f32; 16] {
    let mut vm = [0.0f32; 16];
    for c in 0..4 {
        for r in 0..4 {
            vm[c * 4 + r] = (0..4).map(|k| view[k * 4 + r] * model[c * 4 + k]).sum();
        }
    }
    let mut mvp = [0.0f32; 16];
    for c in 0..4 {
        for r in 0..4 {
            mvp[c * 4 + r] = (0..4).map(|k| proj[k * 4 + r] * vm[c * 4 + k]).sum();
        }
    }
    mvp
}
