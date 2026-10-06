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
    let window = match web_sys::window() {
        Some(w) => w,
        None => return false,
    };
    let navigator = window.navigator();
    let xr = navigator.xr();
    let promise = xr.is_session_supported(XrSessionMode::ImmersiveVr);
    match wasm_bindgen_futures::JsFuture::from(promise).await {
        Ok(val) => val.as_bool().unwrap_or(false),
        Err(_) => false,
    }
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
    let local_floor = JsValue::from_str("local-floor");
    let features = [local_floor];
    session_init.set_required_features(&features);

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
    let session_clone = session.clone();
    let ref_space_clone = ref_space;
    let main_canvas_clone = main_canvas;
    let gl_clone = gl;
    let layer_clone = xr_gl_layer;

    setup_xr_render_loop(
        session_clone,
        ref_space_clone,
        gl_clone,
        layer_clone,
        renderer,
        main_canvas_clone,
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
            vec3 screen_rgb = mix(panel_base, tex_sample.rgb, tex_sample.a);

            // Glowing cyan/mint cyberpunk bezel
            vec3 bezel_color = vec3(0.0, 0.94, 0.82);
            vec3 final_rgb = mix(screen_rgb, bezel_color, is_border);

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

        // Preallocate static geometry VAO, VBO, IBO once
        let hw = QUAD_WIDTH * 0.5;
        let hh = QUAD_HEIGHT * 0.5;
        let vertices: [f32; 20] = [
            -hw, -hh, 0.0, 0.0, 0.0, // bottom-left
            hw, -hh, 0.0, 1.0, 0.0, // bottom-right
            hw, hh, 0.0, 1.0, 1.0, // top-right
            -hw, hh, 0.0, 0.0, 1.0, // top-left
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
        let _ = gl.tex_image_2d_with_u32_and_u32_and_html_canvas_element(
            WebGl2RenderingContext::TEXTURE_2D,
            0,
            WebGl2RenderingContext::RGBA as i32,
            WebGl2RenderingContext::RGBA,
            WebGl2RenderingContext::UNSIGNED_BYTE,
            canvas,
        );
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
    if gl
        .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(gl
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| "Unknown error".to_string()))
    }
}

/// 30-inch Deck HUD model parameters in meters:
/// - Width: 0.72m, Height: 0.42m (~30" 16:9 widescreen)
/// - Position: Center at (0.0, 1.15, -0.75) in local-floor coordinates
/// - Tilt: Tilted up towards user by 28 degrees (-28 deg pitch around X axis)
const QUAD_WIDTH: f32 = 0.72;
const QUAD_HEIGHT: f32 = 0.42;
const QUAD_POS_Y: f32 = 1.15;
const QUAD_POS_Z: f32 = -0.75;
const QUAD_TILT_RAD: f32 = -28.0 * std::f32::consts::PI / 180.0;

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
    let session_loop = session.clone();

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move |_time: f64, frame: XrFrame| {
        let pose: Option<XrViewerPose> = frame.get_viewer_pose(&ref_space);

        // Upload latest egui console canvas frame to WebGL texture
        renderer.borrow().update_texture(&gl, &canvas);

        // Keep driving WebGPU console redraws even when window RAF is backgrounded
        let _ = proxy.send_event(());

        // Process controller ray intersections for Tier 1 HUD interaction
        let input_sources = session_loop.input_sources();
        let num_sources = input_sources.length();
        let canvas_w = canvas.width() as f64;
        let canvas_h = canvas.height() as f64;
        let mut hit_cursor: Option<(f32, f32)> = None;

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

                    // Plane equation for tilted quad:
                    let cos_t = QUAD_TILT_RAD.cos();
                    let sin_t = QUAD_TILT_RAD.sin();
                    let nx = 0.0f32;
                    let ny = cos_t;
                    let nz = -sin_t;

                    let denom = nx * dx + ny * dy + nz * dz;
                    if denom.abs() > 1e-4 {
                        let t = ((0.0 - ox) * nx + (QUAD_POS_Y - oy) * ny + (QUAD_POS_Z - oz) * nz)
                            / denom;
                        if t > 0.05 && t < 3.5 {
                            // Hit point in world coords
                            let hx = ox + t * dx;
                            let hy = oy + t * dy;
                            let hz = oz + t * dz;

                            // Transform hit point to quad local UV space
                            let rel_x = hx;
                            let rel_y = hy - QUAD_POS_Y;
                            let rel_z = hz - QUAD_POS_Z;

                            let local_x = rel_x;
                            let local_y = rel_y * cos_t - rel_z * (-sin_t);

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

                                // Check trigger button from gamepad
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
                }
            }
        }

        // Render stereo eye views
        if let Some(pose) = pose {
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
                draw_hud_quad(&gl, &renderer.borrow(), &view, hit_cursor);
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
    hit_cursor: Option<(f32, f32)>,
) {
    gl.use_program(Some(&renderer.program));

    // Construct Model-View-Projection matrix
    let proj = view.projection_matrix();
    let view_inv = view.transform().inverse();
    let view_matrix = view_inv.matrix();

    let mvp = compute_quad_mvp(&proj, &view_matrix);
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

fn compute_quad_mvp(proj: &[f32], view: &[f32]) -> [f32; 16] {
    // Model matrix: Translation(0, QUAD_POS_Y, QUAD_POS_Z) * RotationX(QUAD_TILT_RAD)
    let c = QUAD_TILT_RAD.cos();
    let s = QUAD_TILT_RAD.sin();
    let model: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, 0.0, c, s, 0.0, 0.0, -s, c, 0.0, 0.0, QUAD_POS_Y, QUAD_POS_Z, 1.0,
    ];

    // vm = view * model
    let mut vm = [0.0f32; 16];
    for col in 0..4 {
        for row in 0..4 {
            let mut sum = 0.0;
            for k in 0..4 {
                sum += view[k * 4 + row] * model[col * 4 + k];
            }
            vm[col * 4 + row] = sum;
        }
    }

    // mvp = proj * vm
    let mut mvp = [0.0f32; 16];
    for col in 0..4 {
        for row in 0..4 {
            let mut sum = 0.0;
            for k in 0..4 {
                sum += proj[k * 4 + row] * vm[col * 4 + k];
            }
            mvp[col * 4 + row] = sum;
        }
    }

    mvp
}
