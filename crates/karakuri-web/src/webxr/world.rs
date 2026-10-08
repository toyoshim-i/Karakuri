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
    u_dome_mode_loc: Option<WebGlUniformLocation>,
    u_time_loc: Option<WebGlUniformLocation>,
    has_texture: bool,
    dome_mode: i32,
    /// The size `texture` was last allocated at.
    texture_size: (u32, u32),
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
        // Rotational timewarp. The picture was drawn for an older head pose
        // than the one this frame is shown at; each corner's ray through the
        // current eye is turned into the drawn eye's clip space, so the
        // picture stays put in the world while the head turns.
        // For Celestial Dome (Tier 3), we also pass the world-space ray
        // direction to the fragment shader for exact dome-master projection.
        let vs_source = r#"#version 300 es
layout(location = 0) in vec2 a_pos;
uniform int u_warp; // 1 = reproject with the matrices below
uniform mat4 u_cur_view;
uniform mat4 u_cur_proj;
uniform mat4 u_drawn_view;
uniform mat4 u_drawn_proj;
out vec2 v_uv;
out vec3 v_world_ray;
out vec4 v_drawn_clip;

void main() {
    v_uv = a_pos * 0.5 + 0.5;
    if (u_warp == 1) {
        vec4 q = inverse(u_cur_proj) * vec4(a_pos, 0.0, 1.0);
        vec3 ray = q.xyz / q.w;
        v_world_ray = transpose(mat3(u_cur_view)) * ray;
        v_drawn_clip = u_drawn_proj * vec4(mat3(u_drawn_view) * v_world_ray, 0.0);
    } else {
        v_world_ray = vec3(a_pos.x * 1.2, a_pos.y * 1.2, -1.0);
        v_drawn_clip = vec4(a_pos, 0.0, 1.0);
    }
    // Render at farthest depth (z = 0.9999) so HUD quad draws in front
    gl_Position = vec4(a_pos, 0.9999, 1.0);
}
"#;

        let fs_source = r#"#version 300 es
precision highp float;

in vec2 v_uv;
in vec3 v_world_ray;
in vec4 v_drawn_clip;
uniform int u_warp;
uniform mat4 u_drawn_view;
uniform int u_eye; // 0 = Left eye, 1 = Right eye
uniform int u_has_texture; // 1 = Sample SBS texture, 0 = Procedural cyber stereo background
uniform int u_dome_mode; // 1 = 180 deg Celestial Dome, 2 = 220 deg Wide Dome, 0 = Planar Screen
uniform float u_time;
uniform sampler2D u_texture;

out vec4 fragColor;

const float PI = 3.141592653589793;

void main() {
    // Mode 3: Passthrough MR - emit transparent black so real-world video shines through
    if (u_dome_mode == 3) {
        fragColor = vec4(0.0, 0.0, 0.0, 0.0);
        return;
    }

    vec3 world_dir = normalize(v_world_ray);

    // 1. Procedural 3D Celestial Cosmos (The Sky Dome / 天球)
    // Deep cosmic space atmosphere oriented in 3D world space
    float elev = world_dir.y;
    vec3 zenith_col = vec3(0.010, 0.014, 0.026);
    vec3 horizon_col = vec3(0.016, 0.020, 0.035);
    vec3 nadir_col = vec3(0.005, 0.006, 0.010);

    vec3 scene_color = mix(horizon_col, zenith_col, clamp(elev, 0.0, 1.0));
    scene_color = mix(scene_color, nadir_col, clamp(-elev, 0.0, 1.0));

    // Subtle celestial grid: Horizon (Equator) and elevation parallels
    float horizon_line = 1.0 - smoothstep(0.0, 0.006, abs(elev));
    scene_color += vec3(0.025, 0.060, 0.100) * horizon_line * 0.45;

    float ring30 = 1.0 - smoothstep(0.0, 0.005, abs(abs(elev) - 0.5));
    float ring60 = 1.0 - smoothstep(0.0, 0.005, abs(abs(elev) - 0.866));
    scene_color += vec3(0.015, 0.030, 0.055) * (ring30 + ring60) * 0.25;

    // Celestial meridians along azimuth (every 30 degrees = PI/6)
    float azimuth = atan(world_dir.x, -world_dir.z);
    float meridian_phase = fract(azimuth * (6.0 / PI) + 0.5) - 0.5;
    float meridian_dist = abs(meridian_phase) * (PI / 6.0) * max(0.1, sqrt(max(0.0, 1.0 - elev * elev)));
    float meridian_line = 1.0 - smoothstep(0.0, 0.004, meridian_dist);
    scene_color += vec3(0.012, 0.025, 0.045) * meridian_line * 0.2;

    // 3D Procedural Starfield fixed to celestial sphere
    vec3 star_p = world_dir * 110.0;
    vec3 star_id = floor(star_p);
    vec3 star_f = fract(star_p) - 0.5;
    float h = fract(sin(dot(star_id, vec3(12.9898, 78.233, 45.164))) * 43758.5453);
    if (h > 0.93) {
        vec3 jitter = vec3(
            fract(h * 13.3) - 0.5,
            fract(h * 27.7) - 0.5,
            fract(h * 41.9) - 0.5
        ) * 0.65;
        float d = length(star_f - jitter);
        float twinkle = 0.75 + 0.25 * sin(u_time * 2.5 + h * 6.283);
        float star = smoothstep(0.08, 0.01, d) * twinkle;
        vec3 star_color = mix(vec3(0.7, 0.85, 1.0), vec3(1.0, 0.85, 0.7), fract(h * 5.0));
        scene_color += star_color * star * (0.35 + (h - 0.93) * 12.0);
    }

    vec3 micro_p = world_dir * 240.0;
    vec3 micro_id = floor(micro_p);
    float h2 = fract(sin(dot(micro_id, vec3(93.123, 34.567, 67.891))) * 28461.23);
    if (h2 > 0.975) {
        vec3 micro_f = fract(micro_p) - 0.5;
        float d2 = length(micro_f);
        float micro_star = smoothstep(0.12, 0.02, d2);
        scene_color += vec3(0.5, 0.65, 0.9) * micro_star * 0.25;
    }

    // 2. Visual Performance Projection (Tier 3 Celestial Dome vs Planar)
    if (u_has_texture == 1) {
        float u_min = (u_eye == 0) ? 0.0 : 0.5;

        if (u_dome_mode == 0) {
            // Planar Screen Mode (Reprojected virtual rectangular cinema screen)
            if (v_drawn_clip.w > 0.0) {
                vec2 uv = v_drawn_clip.xy / v_drawn_clip.w * 0.5 + 0.5;
                if (all(greaterThanEqual(uv, vec2(0.0))) && all(lessThanEqual(uv, vec2(1.0)))) {
                    vec2 eye_uv = vec2(u_min + uv.x * 0.5, 1.0 - uv.y);
                    vec4 tex = texture(u_texture, eye_uv);
                    scene_color += tex.rgb;
                }
            }
        } else {
            // Celestial Dome Mode (1 = 180 deg Dome-Master Hemisphere, 2 = 220 deg Wide Horizon Dome)
            // Compute ray direction in drawn camera space
            // In drawn camera space: forward is -Z, up is +Y, right is +X.
            vec3 local_dir = (u_warp == 1) ? normalize(mat3(u_drawn_view) * world_dir) : world_dir;

            // Polar angle theta from the forward sightline (-Z)
            float theta = acos(clamp(-local_dir.z, -1.0, 1.0));

            // Dome angular diameter Theta_max
            float theta_max = (u_dome_mode == 2) ? (220.0 * PI / 180.0) : PI;

            // Radius in normalized dome-master space [0, 0.5]
            float r = theta / theta_max;

            if (r <= 0.5) {
                // Azimuth direction in the XY plane
                float rho = length(local_dir.xy);
                vec2 dir_xy = (rho > 1e-6) ? (local_dir.xy / rho) : vec2(0.0);

                // Polar fisheye dome mapping:
                // Forward (theta=0) -> (0.5, 0.5)
                // Looking UP (local_dir.y > 0) -> uv.y moves towards top (smaller Y in WebGL texture coords)
                // Looking RIGHT (local_dir.x > 0) -> uv.x moves towards right (> 0.5)
                vec2 dome_uv = vec2(0.5 + r * dir_xy.x, 0.5 - r * dir_xy.y);

                // Smooth aesthetic edge falloff at the dome boundary
                float dome_mask = 1.0 - smoothstep(0.46, 0.50, r);

                vec2 eye_uv = vec2(u_min + clamp(dome_uv.x, 0.0, 1.0) * 0.5, clamp(dome_uv.y, 0.0, 1.0));
                vec4 tex = texture(u_texture, eye_uv);

                // Additive luminance composite: Set illuminates the celestial dome
                scene_color += tex.rgb * dome_mask;
            }
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
        let u_dome_mode_loc = gl.get_uniform_location(&program, "u_dome_mode");
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
            u_dome_mode_loc,
            u_time_loc,
            has_texture: false,
            dome_mode: 1, // Default: Celestial Dome (180° Planetarium)
            texture_size: (0, 0),
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

            // The canvas follows the headset's eye size, so it can change
            // size; texSubImage cannot grow a texture, so reallocate then.
            let size = (canvas.width(), canvas.height());
            let res = if !self.has_texture || self.texture_size != size {
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
                    self.texture_size = size;
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
        if let Some(loc) = self.u_dome_mode_loc.as_ref() {
            gl.uniform1i(Some(loc), self.dome_mode);
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

    /// Returns the active celestial dome visual projection mode:
    /// 1 = 180° Celestial Dome (Planetarium), 2 = 220° Wide Horizon Dome, 0 = Planar Screen, 3 = MR Passthrough.
    pub fn dome_mode(&self) -> i32 {
        self.dome_mode
    }

    /// Sets the active celestial dome visual projection mode.
    pub fn set_dome_mode(&mut self, mode: i32) {
        self.dome_mode = mode;
    }

    /// Cycles to the next projection mode:
    /// 1 (Celestial Dome) -> 2 (Wide Dome) -> 0 (Planar Screen) -> 3 (MR Passthrough) -> 1.
    pub fn cycle_dome_mode(&mut self) -> i32 {
        self.dome_mode = match self.dome_mode {
            1 => 2,
            2 => 0,
            0 => 3,
            _ => 1,
        };
        self.dome_mode
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
