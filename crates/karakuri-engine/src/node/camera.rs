//! Camera edge management and GPU state derivation.
//!
//! Manages camera state buffers for either host-driven cameras (such as the built-in
//! orbit) or compute-driven procedures (L3). Produces derived uniform buffers
//! containing view-projection matrices, ray-marching bases, and depth ranges.

use karakuri_codegen::generate_l3;
use karakuri_codegen::layout::{binding, camera as wire, group, UniformLayout};
use karakuri_ir::typed::Checked;

use crate::camera::{Orbit, State};
use crate::uniforms::UniformScratch;

use super::View;

/// Returns the addressable placement parameter keys for the built-in orbit camera.
fn builtin_param_keys() -> &'static [String] {
    static KEYS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    KEYS.get_or_init(|| {
        Orbit::PLACEMENT
            .iter()
            .map(|(key, _)| key.to_string())
            .collect()
    })
}

/// The camera node, coordinating state buffers, derived views, and compute passes.
pub(crate) struct Camera {
    /// Raw camera state buffer (`CameraState`).
    state: wgpu::Buffer,
    /// Derived camera matrices and basis vectors (`Camera`).
    #[cfg_attr(not(test), allow(dead_code))]
    derived: wgpu::Buffer,
    /// Canvas parameters buffer (aspect ratio).
    canvas: wgpu::Buffer,
    derive: wgpu::ComputePipeline,
    derive_bg: wgpu::BindGroup,
    /// Bind group layout shared by downstream renderers.
    read_bgl: wgpu::BindGroupLayout,
    read_bg: wgpu::BindGroup,
    /// Compute procedure producer if this camera is driven by an L3 procedure.
    proc: Option<Producer>,
}

/// Compiled L3 procedure pass that writes [`Camera::state`].
struct Producer {
    pipeline: wgpu::ComputePipeline,
    uniforms: wgpu::Buffer,
    uniform_layout: UniformLayout,
    scratch: UniformScratch,
    uniform_bg: wgpu::BindGroup,
    state_bg: wgpu::BindGroup,
    /// Uniform field names declared by this procedure.
    param_names: Vec<String>,
    /// Parameter keys addressable by name (e.g. `eye.x`, `target.y`).
    param_keys: Vec<String>,
}

impl Camera {
    /// Builds a camera node for the optional L3 procedure or built-in orbit.
    pub(crate) fn build(
        device: &wgpu::Device,
        l3: Option<&Checked>,
        fields: karakuri_codegen::Bound<'_>,
    ) -> Camera {
        let state = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera state"),
            size: wire::STATE_SIZE,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let derived = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera"),
            // `COPY_SRC` for one test: the host keeps its own copy of this
            // arithmetic for the debug overlay, and two derivations of one
            // camera agree until one of them is edited. Reading it back is how
            // that stops being a hope.
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC,
            size: wire::SIZE,
            mapped_at_creation: false,
        });
        let canvas = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("canvas"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let source = include_str!("../shaders/camera.wgsl")
            .replace("{{STATE_STRUCT}}", wire::STATE_WGSL)
            .replace("{{CAMERA_STRUCT}}", wire::WGSL);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("camera derive"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });

        let storage = |b: u32, read_only: bool| wgpu::BindGroupLayoutEntry {
            binding: b,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let derive_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera derive"),
            entries: &[
                storage(0, true),
                storage(1, false),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let derive_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera derive"),
            layout: &derive_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: state.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: derived.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: canvas.as_entire_binding(),
                },
            ],
        });
        let derive_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("camera derive"),
            bind_group_layouts: &[Some(&derive_bgl)],
            immediate_size: 0,
        });
        let derive = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("camera derive"),
            layout: Some(&derive_layout),
            module: &module,
            entry_point: Some("derive"),
            compilation_options: Default::default(),
            cache: None,
        });

        let read_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                // A per-element L4 projects in its vertex stage and a marching
                // one builds its ray in the fragment stage, so both.
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let read_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera"),
            layout: &read_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: derived.as_entire_binding(),
            }],
        });

        let proc = l3.map(|l3| Producer::build(device, l3, &state, fields));
        Camera {
            state,
            derived,
            canvas,
            derive,
            derive_bg,
            read_bgl,
            read_bg,
            proc,
        }
    }

    /// Returns true if this camera node uses the built-in orbit rather than an L3 procedure.
    pub(crate) fn is_builtin(&self) -> bool {
        self.proc.is_none()
    }

    /// Returns the addressable parameter keys for this camera node.
    pub(crate) fn param_keys(&self) -> &[String] {
        match &self.proc {
            Some(p) => &p.param_keys,
            None => builtin_param_keys(),
        }
    }

    /// Returns the bind group layout expected by downstream renderers.
    pub(crate) fn layout(&self) -> &wgpu::BindGroupLayout {
        &self.read_bgl
    }

    /// Returns the bind group bound by downstream renderers.
    pub(crate) fn bind_group(&self) -> &wgpu::BindGroup {
        &self.read_bg
    }

    /// Prepares camera state or uniform buffers for the current frame.
    pub(crate) fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        view: &View<'_>,
        dt: f32,
        fallback: &State,
    ) {
        match &mut self.proc {
            Some(p) => p.write_uniforms(queue, view, dt),
            None => self.write_state(queue, fallback),
        }
    }

    /// Writes raw host camera state into the GPU state buffer.
    pub(crate) fn write_state(&self, queue: &wgpu::Queue, s: &State) {
        let mut bytes = [0u8; wire::STATE_SIZE as usize];
        let put = |bytes: &mut [u8], at: usize, v: f32| {
            bytes[at..at + 4].copy_from_slice(&v.to_le_bytes());
        };
        for (i, v) in s.eye.iter().enumerate() {
            put(&mut bytes, i * 4, *v);
        }
        put(&mut bytes, 12, s.fov_y);
        for (i, v) in s.target.iter().enumerate() {
            put(&mut bytes, 16 + i * 4, *v);
        }
        put(&mut bytes, 28, s.near);
        for (i, v) in s.up.iter().enumerate() {
            put(&mut bytes, 32 + i * 4, *v);
        }
        put(&mut bytes, 44, s.far);
        queue.write_buffer(&self.state, 0, &bytes);
    }

    /// Writes canvas aspect ratio into the canvas uniform buffer.
    pub(crate) fn write_canvas(&self, queue: &wgpu::Queue, aspect: f32) {
        let mut bytes = [0u8; 16];
        bytes[..4].copy_from_slice(&aspect.to_le_bytes());
        queue.write_buffer(&self.canvas, 0, &bytes);
    }

    /// Records camera compute passes (procedure pass if present, followed by derivation).
    pub(crate) fn record(&self, encoder: &mut wgpu::CommandEncoder) {
        if let Some(p) = &self.proc {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("camera"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&p.pipeline);
            pass.set_bind_group(group::UNIFORMS, &p.uniform_bg, &[]);
            pass.set_bind_group(group::STATE, &p.state_bg, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("camera derive"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.derive);
        pass.set_bind_group(0, &self.derive_bg, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
}

impl Producer {
    fn build(
        device: &wgpu::Device,
        l3: &Checked,
        state: &wgpu::Buffer,
        fields: karakuri_codegen::Bound<'_>,
    ) -> Producer {
        let shader = generate_l3(l3, fields);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{} (L3)", l3.name)),
            source: wgpu::ShaderSource::Wgsl(shader.source.as_str().into()),
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{} uniforms", l3.name)),
            size: u64::from(shader.uniform_layout.total_size),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("L3 uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: binding::UNIFORM,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let state_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("L3 state"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: binding::UNIFORM,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    // Storage buffer bound writable for L3 procedure output.
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let uniform_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("L3 uniforms"),
            layout: &uniform_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: binding::UNIFORM,
                resource: uniforms.as_entire_binding(),
            }],
        });
        let state_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("L3 state"),
            layout: &state_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: binding::UNIFORM,
                resource: state.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("L3"),
            bind_group_layouts: &[Some(&uniform_bgl), Some(&state_bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(&l3.name),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some(karakuri_codegen::l3::ENTRY),
            compilation_options: Default::default(),
            cache: None,
        });

        Producer {
            pipeline,
            uniforms,
            uniform_layout: shader.uniform_layout.clone(),
            scratch: UniformScratch::new(&shader.uniform_layout),
            uniform_bg,
            state_bg,
            param_names: l3.params.iter().map(|p| p.name.clone()).collect(),
            param_keys: crate::set::declared_keys(l3),
        }
    }

    /// Writes uniform parameters for the L3 camera procedure.
    fn write_uniforms(&mut self, queue: &wgpu::Queue, view: &View<'_>, dt: f32) {
        let mut p = self.scratch.pack(&self.uniform_layout);
        p.f32("t", view.t)
            .f32("beats", view.beats)
            .f32("dt", dt)
            .u32("seed_salt", view.seed_salt);
        super::write_params(
            &mut p,
            &self.uniform_layout,
            &self.param_names,
            view.param,
            view.param_value,
        );
        // Spliced field parameters, evaluated per procedure.
        super::write_field_params(
            &mut p,
            &self.uniform_layout,
            view.field_params,
            view.field_value,
        );
        queue.write_buffer(&self.uniforms, 0, p.finish());
    }
}

#[cfg(test)]
mod tests {
    //! Tests comparing GPU camera derivations against CPU reference implementations.

    use super::*;
    use crate::gpu::Gpu;
    use crate::node::simulation::read_buffer;

    fn f32_at(bytes: &[u8], at: usize) -> f32 {
        f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
    }

    fn vec3_at(bytes: &[u8], at: usize) -> [f32; 3] {
        [
            f32_at(bytes, at),
            f32_at(bytes, at + 4),
            f32_at(bytes, at + 8),
        ]
    }

    fn close(a: f32, b: f32, what: &str) {
        // Relative, because `view_proj`'s entries span the frustum's whole
        // scale: `far / (near - far)` is about -1 and `near * far / (near -
        // far)` is about -0.1, while a pre-scaled basis vector is a fraction.
        let tol = 1e-5 * a.abs().max(b.abs()).max(1.0);
        assert!((a - b).abs() <= tol, "{what}: host {a}, gpu {b}");
    }

    // All four compare a derived block the GPU wrote against one the host computed,
    // so all four take a device. Wrapped rather than left alone anyway: the rule is
    // the same everywhere, and a target nobody wrapped is indistinguishable from one
    // nobody checked. See `../../tests/gpu_tests_are_under_mod_gpu.rs`.
    mod gpu {
        use super::*;

        #[test]
        fn the_pass_derives_what_the_host_would_have() {
            // Panics rather than skipping when there is no adapter, as every
            // other GPU test in this workspace does. These four used to print
            // and return, so a machine with no device ran them green — and the
            // reason that was tolerable (there was no other way to get a run
            // out of such a machine) stopped being true when `--skip gpu::`
            // arrived. See `../../tests/gpu_tests_are_under_mod_gpu.rs`.
            let gpu = Gpu::headless().expect("no GPU available");
            // Nothing axis-aligned and nothing at the origin: a camera looking down
            // an axis at (0,0,0) with `up` exactly +Y makes several columns of the
            // view matrix zero, and a transcription error in the ones that stayed
            // zero would not show.
            let state = State {
                eye: [3.0, -1.5, 4.25],
                target: [-0.5, 0.75, 1.0],
                up: [0.1, 0.9, -0.2],
                fov_y: 0.9,
                near: 0.25,
                far: 60.0,
            };
            let aspect = 16.0 / 9.0;

            let cam = Camera::build(&gpu.device, None, &[]);
            cam.write_state(&gpu.queue, &state);
            cam.write_canvas(&gpu.queue, aspect);
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            cam.record(&mut encoder);
            gpu.queue.submit([encoder.finish()]);
            let bytes = read_buffer(&gpu.device, &gpu.queue, &cam.derived, wire::SIZE);

            // Column-major, so `(c * 4 + r) * 4` is the byte offset of the entry in
            // row `r` of column `c` — the order both `Mat4` and WGSL store one in.
            for (c, col) in state.view_proj(aspect).iter().enumerate() {
                for (r, want) in col.iter().enumerate() {
                    close(
                        *want,
                        f32_at(&bytes, (c * 4 + r) * 4),
                        &format!("view_proj[{c}][{r}]"),
                    );
                }
            }

            let basis = state.basis(aspect);
            for (at, (name, want)) in [
                (64, ("eye", basis.eye)),
                (80, ("fwd", basis.forward)),
                (96, ("right", basis.right)),
                (112, ("up", basis.up)),
            ] {
                let got = vec3_at(&bytes, at);
                for i in 0..3 {
                    close(want[i], got[i], &format!("{name}[{i}]"));
                }
            }

            let range = state.depth_range();
            close(range[0], f32_at(&bytes, 128), "depth_range.near");
            close(range[1], f32_at(&bytes, 132), "depth_range.span");
        }
        /// Tests that a camera procedure's defaulted fields match the built-in camera defaults.
        #[test]
        fn a_camera_procedure_that_writes_two_outputs_matches_the_built_in_in_the_other_four() {
            let gpu = Gpu::headless().expect("no GPU available");
            let src = r#"
proc two {
  kind L3
  camera {
    eye    = vec3(3.0, 2.0, 7.0);
    target = vec3(-1.0, 0.5, 0.0);
  }
}
"#;
            let parsed = karakuri_ir::parse(src).expect("parses");
            let l3 = karakuri_ir::check::check(&parsed).expect("checks");
            let aspect = 16.0 / 9.0;

            let mut cam = Camera::build(&gpu.device, Some(&l3), &[]);
            cam.write_canvas(&gpu.queue, aspect);
            cam.prepare(
                &gpu.queue,
                &View {
                    t: 0.0,
                    beats: 0.0,
                    seed_salt: 0,
                    viewport: [16.0, 9.0],
                    param: &|_| None,
                    param_value: None,
                    field_params: &[],
                    field_value: &|_| None,
                    // A camera declares no Source slot — `uses … : Source` is
                    // refused on an L3 — so nothing here can ask for one.
                    source_value: &|_| None,
                },
                crate::set::DT,
                // Unread: this node has a procedure, so the built-in is not its
                // producer. Passed as what it would have been.
                &crate::camera::Orbit::default().state(0.0),
            );
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            cam.record(&mut encoder);
            gpu.queue.submit([encoder.finish()]);
            let bytes = read_buffer(&gpu.device, &gpu.queue, &cam.derived, wire::SIZE);

            // The same eye and target the procedure writes, and every other field
            // from the struct that documents the built-in's answers.
            let orbit = crate::camera::Orbit::default();
            let want = State {
                eye: [3.0, 2.0, 7.0],
                target: [-1.0, 0.5, 0.0],
                up: [0.0, 1.0, 0.0],
                fov_y: orbit.fov_y,
                near: orbit.near,
                far: orbit.far,
            };
            for (c, col) in want.view_proj(aspect).iter().enumerate() {
                for (r, v) in col.iter().enumerate() {
                    close(
                        *v,
                        f32_at(&bytes, (c * 4 + r) * 4),
                        &format!("view_proj[{c}][{r}]"),
                    );
                }
            }
            let range = want.depth_range();
            close(range[0], f32_at(&bytes, 128), "depth_range.near");
            close(range[1], f32_at(&bytes, 132), "depth_range.span");
        }

        /// Tests that every declared camera procedure output correctly updates the camera state.
        #[test]
        fn every_camera_output_reaches_the_state_it_names() {
            let gpu = Gpu::headless().expect("no GPU available");
            let src = r#"
proc six {
  kind L3
  camera {
    eye    = vec3(2.0, -1.0, 6.0);
    target = vec3(0.5, 1.5, -0.5);
    up     = vec3(0.2, 0.3, 0.9);
    fov_y  = 0.62;
    near   = 0.4;
    far    = 37.0;
  }
}
"#;
            let parsed = karakuri_ir::parse(src).expect("parses");
            let l3 = karakuri_ir::check::check(&parsed).expect("checks");
            let aspect = 4.0 / 3.0;

            let mut cam = Camera::build(&gpu.device, Some(&l3), &[]);
            cam.write_canvas(&gpu.queue, aspect);
            cam.prepare(
                &gpu.queue,
                &View {
                    t: 0.0,
                    beats: 0.0,
                    seed_salt: 0,
                    viewport: [4.0, 3.0],
                    param: &|_| None,
                    param_value: None,
                    field_params: &[],
                    field_value: &|_| None,
                    // A camera declares no Source slot — `uses … : Source` is
                    // refused on an L3 — so nothing here can ask for one.
                    source_value: &|_| None,
                },
                crate::set::DT,
                &crate::camera::Orbit::default().state(0.0),
            );
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            cam.record(&mut encoder);
            gpu.queue.submit([encoder.finish()]);
            let bytes = read_buffer(&gpu.device, &gpu.queue, &cam.derived, wire::SIZE);

            let want = State {
                eye: [2.0, -1.0, 6.0],
                target: [0.5, 1.5, -0.5],
                up: [0.2, 0.3, 0.9],
                fov_y: 0.62,
                near: 0.4,
                far: 37.0,
            };
            for (c, col) in want.view_proj(aspect).iter().enumerate() {
                for (r, v) in col.iter().enumerate() {
                    close(
                        *v,
                        f32_at(&bytes, (c * 4 + r) * 4),
                        &format!("view_proj[{c}][{r}]"),
                    );
                }
            }
            // The projection carries `fov_y` and the planes; the basis is what
            // carries `up`, and an `up` that never arrived would show here and not
            // above — `view_proj` and the ray basis are two derivations of it.
            let basis = want.basis(aspect);
            for (at, (name, want)) in [
                (64, ("eye", basis.eye)),
                (80, ("fwd", basis.forward)),
                (96, ("right", basis.right)),
                (112, ("up", basis.up)),
            ] {
                let got = vec3_at(&bytes, at);
                for i in 0..3 {
                    close(want[i], got[i], &format!("{name}[{i}]"));
                }
            }
            let range = want.depth_range();
            close(range[0], f32_at(&bytes, 128), "depth_range.near");
            close(range[1], f32_at(&bytes, 132), "depth_range.span");
        }

        /// Tests that canvas aspect ratio changes affect only the projection width and ray basis.
        #[test]
        fn the_canvas_widens_the_projection_and_leaves_the_camera_where_it_is() {
            let gpu = Gpu::headless().expect("no GPU available");
            let state = State {
                eye: [0.0, 1.0, 5.0],
                target: [0.0, 0.0, 0.0],
                up: [0.0, 1.0, 0.0],
                fov_y: 1.0,
                near: 0.1,
                far: 50.0,
            };
            let cam = Camera::build(&gpu.device, None, &[]);
            let derive = |aspect: f32| {
                cam.write_state(&gpu.queue, &state);
                cam.write_canvas(&gpu.queue, aspect);
                let mut encoder = gpu.device.create_command_encoder(&Default::default());
                cam.record(&mut encoder);
                gpu.queue.submit([encoder.finish()]);
                read_buffer(&gpu.device, &gpu.queue, &cam.derived, wire::SIZE)
            };
            let square = derive(1.0);
            let wide = derive(2.0);

            // Twice as wide a canvas halves the horizontal scale of the projection
            // and doubles the ray basis's `right` — the same field of view spread
            // over more pixels either way.
            close(
                f32_at(&square, 0) * 0.5,
                f32_at(&wide, 0),
                "view_proj[0][0]",
            );
            for i in 0..3 {
                close(
                    vec3_at(&square, 96)[i] * 2.0,
                    vec3_at(&wide, 96)[i],
                    "right",
                );
            }
            // And nothing else: the eye, the direction, the vertical half-angle and
            // the depth range are the camera's own.
            for at in [64, 80, 112, 128] {
                for i in 0..3 {
                    close(vec3_at(&square, at)[i], vec3_at(&wide, at)[i], "unmoved");
                }
            }
            close(f32_at(&square, 20), f32_at(&wide, 20), "view_proj[1][1]");
        }
    }
}
