//! WebGL2 stereo 3D world background renderer for WebXR.
//!
//! Renders Side-by-Side (SBS) stereo visual textures from WebGPU onto the WebXR background
//! with full 6DoF parallax (Left eye gets left half U:0.0..0.5, Right eye gets right half U:0.5..1.0).

use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext, WebGlBuffer, WebGlProgram, WebGlShader,
    WebGlTexture, WebGlUniformLocation, WebGlVertexArrayObject,
};

use super::StereoEye;

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
    warp: WarpUniforms,
}

/// Where the timewarp's matrices go in the world program.
struct WarpUniforms {
    on: Option<WebGlUniformLocation>,
    cur_view: Option<WebGlUniformLocation>,
    cur_proj: Option<WebGlUniformLocation>,
    drawn_view: Option<WebGlUniformLocation>,
    drawn_proj: Option<WebGlUniformLocation>,
}

impl XrWorldRenderer {
    /// Compiles shaders and initializes fullscreen quad geometry.
    pub fn new(gl: &WebGl2RenderingContext) -> Result<Self, String> {
        // Rotational timewarp. The picture was drawn for an older head pose
        // than the one this frame is shown at; each corner's ray through the
        // current eye is turned into the drawn eye's clip space, so the
        // picture stays put in the world while the head turns. Rotation only:
        // a ray has no position, so head translation since the draw is not
        // compensated (it's small over a frame or two). Clip coordinates are
        // linear across the quad, so interpolating them and dividing per
        // fragment is exact.
        let vs_source = r#"#version 300 es
layout(location = 0) in vec2 a_pos;
uniform int u_warp; // 1 = reproject with the matrices below
uniform mat4 u_cur_view;
uniform mat4 u_cur_proj;
uniform mat4 u_drawn_view;
uniform mat4 u_drawn_proj;
out vec2 v_uv;
out vec4 v_drawn_clip;

void main() {
    v_uv = a_pos * 0.5 + 0.5;
    if (u_warp == 1) {
        vec4 q = inverse(u_cur_proj) * vec4(a_pos, 0.0, 1.0);
        vec3 world_dir = transpose(mat3(u_cur_view)) * (q.xyz / q.w);
        v_drawn_clip = u_drawn_proj * vec4(mat3(u_drawn_view) * world_dir, 0.0);
    } else {
        v_drawn_clip = vec4(a_pos, 0.0, 1.0);
    }
    // Render at farthest depth (z = 0.9999) so HUD quad draws in front
    gl_Position = vec4(a_pos, 0.9999, 1.0);
}
"#;

        let fs_source = r#"#version 300 es
precision highp float;

in vec2 v_uv;
in vec4 v_drawn_clip;
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

    // If stereo texture from the selected Set is available, composite it on
    // top where this pixel's ray falls inside the picture as drawn.
    if (u_has_texture == 1 && v_drawn_clip.w > 0.0) {
        vec2 uv = v_drawn_clip.xy / v_drawn_clip.w * 0.5 + 0.5;
        if (all(greaterThanEqual(uv, vec2(0.0))) && all(lessThanEqual(uv, vec2(1.0)))) {
            float u_min = (u_eye == 0) ? 0.0 : 0.5;
            vec2 eye_uv = vec2(u_min + uv.x * 0.5, 1.0 - uv.y);
            vec4 tex = texture(u_texture, eye_uv);
            // Additive luminance composite so Set illuminates the cosmic backdrop
            scene_color += tex.rgb;
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
        let warp = WarpUniforms {
            on: gl.get_uniform_location(&program, "u_warp"),
            cur_view: gl.get_uniform_location(&program, "u_cur_view"),
            cur_proj: gl.get_uniform_location(&program, "u_cur_proj"),
            drawn_view: gl.get_uniform_location(&program, "u_drawn_view"),
            drawn_proj: gl.get_uniform_location(&program, "u_drawn_proj"),
        };

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
            warp,
        })
    }

    /// Uploads stereo SBS image from offscreen canvas into WebGL2 texture.
    pub fn update_texture_from_canvas(
        &mut self,
        gl: &WebGl2RenderingContext,
        canvas: &HtmlCanvasElement,
    ) {
        if canvas.width() > 0 && canvas.height() > 0 {
            let t2d = WebGl2RenderingContext::TEXTURE_2D;
            gl.bind_texture(t2d, Some(&self.texture));

            let res = if !self.has_texture {
                let r = gl.tex_image_2d_with_u32_and_u32_and_html_canvas_element(
                    t2d,
                    0,
                    WebGl2RenderingContext::RGBA as i32,
                    WebGl2RenderingContext::RGBA,
                    WebGl2RenderingContext::UNSIGNED_BYTE,
                    canvas,
                );
                if r.is_ok() {
                    self.has_texture = true;
                    web_sys::console::log_1(
                        &format!(
                            "Karakuri XR world texture allocated: {}x{}",
                            canvas.width(),
                            canvas.height()
                        )
                        .into(),
                    );
                }
                r
            } else {
                gl.tex_sub_image_2d_with_u32_and_u32_and_html_canvas_element(
                    t2d,
                    0,
                    0,
                    0,
                    WebGl2RenderingContext::RGBA,
                    WebGl2RenderingContext::UNSIGNED_BYTE,
                    canvas,
                )
            };

            static UPLOAD_COUNT: std::sync::atomic::AtomicU64 =
                std::sync::atomic::AtomicU64::new(0);
            let u = UPLOAD_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if u % 144 == 1 {
                web_sys::console::log_1(
                    &format!(
                        "Karakuri XR world texture updated frame {u}: {}x{}, has_texture={}",
                        canvas.width(),
                        canvas.height(),
                        self.has_texture
                    )
                    .into(),
                );
            }

            let err = gl.get_error();
            if err != WebGl2RenderingContext::NO_ERROR {
                static WARNED: std::sync::atomic::AtomicBool =
                    std::sync::atomic::AtomicBool::new(false);
                if !WARNED.swap(true, std::sync::atomic::Ordering::Relaxed) {
                    web_sys::console::warn_1(
                        &format!("Karakuri XR world gl.get_error after update_texture: 0x{err:x}")
                            .into(),
                    );
                }
            }
            if let Err(e) = res {
                static WARNED: std::sync::atomic::AtomicBool =
                    std::sync::atomic::AtomicBool::new(false);
                if !WARNED.swap(true, std::sync::atomic::Ordering::Relaxed) {
                    web_sys::console::warn_2(
                        &"Karakuri XR world update_texture failed:".into(),
                        &e,
                    );
                }
            }
        }
    }

    /// Draws the stereo background for one eye view.
    /// `eye_index`: 0 for Left eye, 1 for Right eye. `eyes` is this eye as
    /// it is now and as it was when the picture was drawn, for reprojection;
    /// without it the picture is shown as is.
    pub fn draw_eye(
        &self,
        gl: &WebGl2RenderingContext,
        eye_index: i32,
        time_sec: f32,
        eyes: Option<(StereoEye, StereoEye)>,
    ) {
        gl.use_program(Some(&self.program));
        gl.bind_vertex_array(Some(&self._vao));

        let w = &self.warp;
        gl.uniform1i(w.on.as_ref(), eyes.is_some() as i32);
        if let Some((shown, drawn)) = eyes {
            gl.uniform_matrix4fv_with_f32_array(w.cur_view.as_ref(), false, &shown.view);
            gl.uniform_matrix4fv_with_f32_array(w.cur_proj.as_ref(), false, &shown.proj);
            gl.uniform_matrix4fv_with_f32_array(w.drawn_view.as_ref(), false, &drawn.view);
            gl.uniform_matrix4fv_with_f32_array(w.drawn_proj.as_ref(), false, &drawn.proj);
        }

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
