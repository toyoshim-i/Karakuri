use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext, WebGlProgram, WebGlShader, WebGlTexture, XrView,
    XrViewerPose,
};

/// 30-inch Deck HUD model dimensions in meters (~30" 16:9 widescreen FHD)
pub const QUAD_WIDTH: f32 = 0.72;
pub const QUAD_HEIGHT: f32 = 0.405;

pub struct XrQuadRenderer {
    pub(crate) program: WebGlProgram,
    pub(crate) texture: WebGlTexture,
    pub(crate) u_mvp: web_sys::WebGlUniformLocation,
    pub(crate) u_texture: web_sys::WebGlUniformLocation,
    pub(crate) u_cursor: web_sys::WebGlUniformLocation,
    pub(crate) u_cursor_active: web_sys::WebGlUniformLocation,
    pub(crate) _vertex_buffer: web_sys::WebGlBuffer,
    pub(crate) _index_buffer: web_sys::WebGlBuffer,
    pub(crate) vao: web_sys::WebGlVertexArrayObject,
}

impl XrQuadRenderer {
    pub fn new(gl: &WebGl2RenderingContext) -> Result<Self, String> {
        let vs_source = r#"#version 300 es
        precision highp float;
        precision highp int;
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
        precision highp int;
        in vec2 v_uv;
        uniform sampler2D u_texture;
        uniform vec2 u_cursor;
        uniform int u_cursor_active;
        out vec4 frag_color;

        void main() {
            // Sample console texture (flip Y so canvas top matches quad top)
            vec2 tex_uv = vec2(v_uv.x, 1.0 - v_uv.y);
            vec4 tex_sample = texture(u_texture, tex_uv);

            // Clean direct console presentation: full fidelity UI without obstructing inner borders
            vec3 final_rgb = tex_sample.rgb;
            if (tex_sample.a < 0.05) {
                // Subtle dark background fallback only if console pixel is transparent
                final_rgb = vec3(0.04, 0.05, 0.07);
            }

            // Controller laser hit reticle
            if (u_cursor_active == 1) {
                vec2 diff = v_uv - u_cursor;
                diff.x *= (0.72 / 0.405); // Aspect correction matching QUAD_WIDTH / QUAD_HEIGHT
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
        let u_texture = gl
            .get_uniform_location(&program, "u_texture")
            .ok_or("No u_texture uniform")?;
        let u_cursor = gl
            .get_uniform_location(&program, "u_cursor")
            .ok_or("No u_cursor uniform")?;
        let u_cursor_active = gl
            .get_uniform_location(&program, "u_cursor_active")
            .ok_or("No u_cursor_active uniform")?;

        let texture = gl.create_texture().ok_or("Failed to create texture")?;
        let t2d = WebGl2RenderingContext::TEXTURE_2D;
        gl.bind_texture(t2d, Some(&texture));
        let clamp = WebGl2RenderingContext::CLAMP_TO_EDGE as i32;
        let linear = WebGl2RenderingContext::LINEAR as i32;
        gl.tex_parameteri(t2d, WebGl2RenderingContext::TEXTURE_MIN_FILTER, linear);
        gl.tex_parameteri(t2d, WebGl2RenderingContext::TEXTURE_MAG_FILTER, linear);
        gl.tex_parameteri(t2d, WebGl2RenderingContext::TEXTURE_WRAP_S, clamp);
        gl.tex_parameteri(t2d, WebGl2RenderingContext::TEXTURE_WRAP_T, clamp);

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
            u_texture,
            u_cursor,
            u_cursor_active,
            _vertex_buffer: vertex_buffer,
            _index_buffer: index_buffer,
            vao,
        })
    }

    pub fn update_texture(&self, gl: &WebGl2RenderingContext, canvas: &HtmlCanvasElement) {
        if canvas.width() == 0 || canvas.height() == 0 {
            return;
        }

        let t2d = WebGl2RenderingContext::TEXTURE_2D;
        gl.bind_texture(t2d, Some(&self.texture));
        let res = gl.tex_image_2d_with_u32_and_u32_and_html_canvas_element(
            t2d,
            0,
            WebGl2RenderingContext::RGBA as i32,
            WebGl2RenderingContext::RGBA,
            WebGl2RenderingContext::UNSIGNED_BYTE,
            canvas,
        );

        let err = gl.get_error();
        if err != WebGl2RenderingContext::NO_ERROR {
            static WARNED: std::sync::atomic::AtomicBool =
                std::sync::atomic::AtomicBool::new(false);
            if !WARNED.swap(true, std::sync::atomic::Ordering::Relaxed) {
                web_sys::console::warn_1(
                    &format!("WebXR gl.get_error after update_texture: 0x{err:x}").into(),
                );
            }
        }
        if let Err(e) = res {
            web_sys::console::warn_2(&"Karakuri WebXR update_texture failed:".into(), &e);
        }
    }
}

pub fn compile_shader(
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
        let log = gl
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| "Unknown shader compile error".into());
        Err(log)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HudAnchor {
    pub center: [f32; 3],
    pub normal: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub model: [f32; 16],
}

impl HudAnchor {
    /// Constructs a spatial HUD anchor positioned relative to the user's initial viewer pose:
    /// - Front distance: ~0.65m in the horizontal gaze direction
    /// - Height offset: -0.38m below the eyes (natural lap/desk/hand height whether sitting or standing)
    /// - Tilt: 30° upwards facing the user's eyes
    pub fn from_viewer_pose(pose: &XrViewerPose) -> Self {
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
        let len = (fwd_x * fwd_x + fwd_z * fwd_z).sqrt().max(1e-4);
        let fwd = (fwd_x / len, 0.0, fwd_z / len);

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
pub struct GrabState {
    pub source_index: u32,
    pub initial_ctrl_pos: [f32; 3],
    pub initial_anchor_center: [f32; 3],
}

pub fn draw_hud_quad(
    gl: &WebGl2RenderingContext,
    renderer: &XrQuadRenderer,
    view: &XrView,
    model: &[f32; 16],
    hit_cursor: Option<(f32, f32)>,
) {
    gl.use_program(Some(&renderer.program));

    // Bind texture to unit 0
    gl.active_texture(WebGl2RenderingContext::TEXTURE0);
    gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&renderer.texture));
    gl.uniform1i(Some(&renderer.u_texture), 0);

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

pub fn compute_quad_mvp(proj: &[f32], view: &[f32], model: &[f32]) -> [f32; 16] {
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
