//! WebGL2 stereo 3D world background renderer for WebXR.
//!
//! Renders Side-by-Side (SBS) stereo visual textures from WebGPU onto the WebXR background
//! with full 6DoF parallax (Left eye gets left half U:0.0..0.5, Right eye gets right half U:0.5..1.0).

use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext, WebGlBuffer, WebGlProgram, WebGlShader,
    WebGlTexture, WebGlUniformLocation, WebGlVertexArrayObject,
};

/// WebGL2 renderer for drawing stereo 3D world background into WebXR eye viewports.
pub struct XrWorldRenderer {
    program: WebGlProgram,
    _vao: WebGlVertexArrayObject,
    _vbo: WebGlBuffer,
    texture: WebGlTexture,
    u_eye_loc: Option<WebGlUniformLocation>,
    u_has_texture_loc: Option<WebGlUniformLocation>,
    u_time_loc: Option<WebGlUniformLocation>,
    has_texture: bool,
}

impl XrWorldRenderer {
    /// Compiles shaders and initializes fullscreen quad geometry.
    pub fn new(gl: &WebGl2RenderingContext) -> Result<Self, String> {
        let vs_source = r#"#version 300 es
layout(location = 0) in vec2 a_pos;
out vec2 v_uv;

void main() {
    v_uv = a_pos * 0.5 + 0.5;
    // Render at farthest depth (z = 0.9999) so HUD quad draws in front
    gl_Position = vec4(a_pos, 0.9999, 1.0);
}
"#;

        let fs_source = r#"#version 300 es
precision highp float;

in vec2 v_uv;
uniform int u_eye; // 0 = Left eye, 1 = Right eye
uniform int u_has_texture; // 1 = Sample SBS texture, 0 = Procedural cyber stereo background
uniform float u_time;
uniform sampler2D u_texture;

out vec4 fragColor;

void main() {
    // Elegant deep cosmos void backdrop
    vec3 bg_color = vec3(0.015, 0.018, 0.026);

    // Subtle gentle cosmic vignette
    vec2 p = (v_uv - 0.5) * 2.0;
    float dist = length(p);
    vec3 scene_color = mix(bg_color, vec3(0.008, 0.010, 0.016), clamp(dist * 0.4, 0.0, 1.0));

    // If stereo texture from the selected Set is available, composite it on top
    if (u_has_texture == 1) {
        float u_min = (u_eye == 0) ? 0.0 : 0.5;
        vec2 eye_uv = vec2(u_min + v_uv.x * 0.5, 1.0 - v_uv.y);
        vec4 tex = texture(u_texture, eye_uv);
        float lum = max(tex.r, max(tex.g, tex.b));
        if (tex.a > 0.01 || lum > 0.01) {
            float alpha = max(tex.a, clamp(lum * 1.5, 0.0, 1.0));
            scene_color = mix(scene_color, tex.rgb, alpha);
        }
    }

    fragColor = vec4(scene_color, 1.0);
}
"#;

        let vs = compile_shader(gl, WebGl2RenderingContext::VERTEX_SHADER, vs_source)?;
        let fs = compile_shader(gl, WebGl2RenderingContext::FRAGMENT_SHADER, fs_source)?;
        let program = link_program(gl, &vs, &fs)?;

        let u_eye_loc = gl.get_uniform_location(&program, "u_eye");
        let u_has_texture_loc = gl.get_uniform_location(&program, "u_has_texture");
        let u_time_loc = gl.get_uniform_location(&program, "u_time");
        let u_tex_loc = gl.get_uniform_location(&program, "u_texture");

        // Fullscreen NDC quad
        let quad_vertices: [f32; 12] = [
            -1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 1.0,
        ];

        let vao = gl
            .create_vertex_array()
            .ok_or("Failed to create world VAO")?;
        gl.bind_vertex_array(Some(&vao));

        let vbo = gl.create_buffer().ok_or("Failed to create world VBO")?;
        gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&vbo));

        unsafe {
            let view = js_sys::Float32Array::view(&quad_vertices);
            gl.buffer_data_with_array_buffer_view(
                WebGl2RenderingContext::ARRAY_BUFFER,
                &view,
                WebGl2RenderingContext::STATIC_DRAW,
            );
        }

        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_with_i32(0, 2, WebGl2RenderingContext::FLOAT, false, 0, 0);

        gl.bind_vertex_array(None);
        gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, None);

        let texture = gl
            .create_texture()
            .ok_or("Failed to create world texture")?;
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

        gl.use_program(Some(&program));
        if let Some(loc) = u_tex_loc.as_ref() {
            gl.uniform1i(Some(loc), 0);
        }
        gl.use_program(None);

        Ok(Self {
            program,
            _vao: vao,
            _vbo: vbo,
            texture,
            u_eye_loc,
            u_has_texture_loc,
            u_time_loc,
            has_texture: false,
        })
    }

    /// Uploads stereo SBS image from canvas into WebGL2 texture.
    pub fn update_texture_from_canvas(
        &mut self,
        gl: &WebGl2RenderingContext,
        canvas: &HtmlCanvasElement,
    ) {
        if canvas.width() > 0 && canvas.height() > 0 {
            gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&self.texture));
            let res = gl.tex_image_2d_with_u32_and_u32_and_html_canvas_element(
                WebGl2RenderingContext::TEXTURE_2D,
                0,
                WebGl2RenderingContext::RGBA as i32,
                WebGl2RenderingContext::RGBA,
                WebGl2RenderingContext::UNSIGNED_BYTE,
                canvas,
            );
            let err = gl.get_error();
            if err != WebGl2RenderingContext::NO_ERROR {
                static ERR_WARNED: std::sync::atomic::AtomicBool =
                    std::sync::atomic::AtomicBool::new(false);
                if !ERR_WARNED.swap(true, std::sync::atomic::Ordering::Relaxed) {
                    web_sys::console::warn_1(
                        &format!("Karakuri XR world gl.get_error after tex_image_2d: 0x{err:x}")
                            .into(),
                    );
                }
            }
            match res {
                Ok(_) => {
                    static LOGGED: std::sync::atomic::AtomicBool =
                        std::sync::atomic::AtomicBool::new(false);
                    if !LOGGED.swap(true, std::sync::atomic::Ordering::Relaxed) {
                        web_sys::console::log_1(
                            &format!(
                                "Karakuri XR world initial texture upload succeeded (canvas {}x{})",
                                canvas.width(),
                                canvas.height()
                            )
                            .into(),
                        );
                    }
                    self.has_texture = true;
                }
                Err(e) => {
                    static WARNED: std::sync::atomic::AtomicBool =
                        std::sync::atomic::AtomicBool::new(false);
                    if !WARNED.swap(true, std::sync::atomic::Ordering::Relaxed) {
                        web_sys::console::warn_2(
                            &"Karakuri XR world tex_image_2d failed:".into(),
                            &e,
                        );
                    }
                }
            }
        }
    }

    /// Draws the stereo background for one eye view.
    /// `eye_index`: 0 for Left eye, 1 for Right eye.
    pub fn draw_eye(&self, gl: &WebGl2RenderingContext, eye_index: i32, time_sec: f32) {
        gl.use_program(Some(&self.program));
        gl.bind_vertex_array(Some(&self._vao));

        if let Some(loc) = self.u_eye_loc.as_ref() {
            gl.uniform1i(Some(loc), eye_index);
        }
        if let Some(loc) = self.u_has_texture_loc.as_ref() {
            gl.uniform1i(Some(loc), if self.has_texture { 1 } else { 0 });
        }
        if let Some(loc) = self.u_time_loc.as_ref() {
            gl.uniform1f(Some(loc), time_sec);
        }

        gl.active_texture(WebGl2RenderingContext::TEXTURE0);
        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&self.texture));

        // Disable depth write, depth test, and backface culling so backdrop covers the entire eye field
        gl.disable(WebGl2RenderingContext::DEPTH_TEST);
        gl.disable(WebGl2RenderingContext::CULL_FACE);
        gl.depth_mask(false);
        gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, 6);
        gl.depth_mask(true);
        gl.enable(WebGl2RenderingContext::DEPTH_TEST);

        gl.bind_vertex_array(None);
        gl.use_program(None);
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
        let log = gl
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| "Unknown shader error".into());
        gl.delete_shader(Some(&shader));
        Err(log)
    }
}

fn link_program(
    gl: &WebGl2RenderingContext,
    vs: &WebGlShader,
    fs: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let program = gl.create_program().ok_or("Failed to create program")?;
    gl.attach_shader(&program, vs);
    gl.attach_shader(&program, fs);
    gl.link_program(&program);

    if gl
        .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        let log = gl
            .get_program_info_log(&program)
            .unwrap_or_else(|| "Unknown program error".into());
        gl.delete_program(Some(&program));
        Err(log)
    }
}
