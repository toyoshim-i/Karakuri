//! The camera edge, as a GPU buffer: `() -> Camera`, or `Geometry -> Camera`.
//!
//! **One consumer, two kinds of producer.** A camera that reads only the clock —
//! the built-in orbit, and every L3 that jumps on a beat or sweeps on `t` — has
//! its state written from the host with a `queue.write_buffer`. A camera that
//! follows an element has it written by a compute pass, because the element's
//! position is in a buffer and reading it back would stall the frame. Both write
//! the same 48 bytes, and everything downstream of them is identical.
//!
//! That is why the state lives here rather than in a renderer's uniform. Until
//! this node existed the host derived a `view_proj` from an `Orbit` it owned and
//! packed it into every L4's uniform block, which works exactly as long as the
//! host knows where the camera is — and `docs/ir-spec.md` settles that it will
//! not: *「1つ目の頂点を追いかける」* takes geometry as an input.
//!
//! # Two buffers, because state and its derivations are different edges
//!
//! `CameraState` is what a producer writes and what a *blend* of two producers
//! would be meaningful on: six numbers, no aspect ratio, no matrix. `Camera` is
//! what a renderer reads: `view_proj` for the raster path, the ray basis for the
//! marched one, `depth_range` for `blend weighted`. Three derivations of one
//! camera that must agree, produced in one pass from one state — which is the
//! same argument [`crate::camera`] makes for deriving rather than passing.
//!
//! The derivation is where the aspect ratio enters, since it belongs to the
//! canvas rather than to the camera.

use karakuri_codegen::layout::camera as wire;

use crate::camera::State;

/// The camera node: a state buffer, its derived form, and the pass between.
pub(crate) struct Camera {
    /// `CameraState`. Storage rather than uniform because the other producer is
    /// a compute pass, and `COPY_DST` because this one is the host.
    state: wgpu::Buffer,
    /// `Camera`. Bound as a uniform by every renderer and written as storage by
    /// the pass below — one buffer with both usages rather than a copy between
    /// two.
    ///
    /// **Held only so a test can read it back.** The bind groups keep it alive
    /// on their own, and nothing on the frame path names it again once they are
    /// built.
    #[cfg_attr(not(test), allow(dead_code))]
    derived: wgpu::Buffer,
    canvas: wgpu::Buffer,
    derive: wgpu::ComputePipeline,
    derive_bg: wgpu::BindGroup,
    /// What a renderer's pipeline layout names. Handed out rather than rebuilt
    /// per renderer so every L4 in a Set is layout-compatible with the one bind
    /// group below.
    read_bgl: wgpu::BindGroupLayout,
    read_bg: wgpu::BindGroup,
}

impl Camera {
    pub(crate) fn new(device: &wgpu::Device) -> Camera {
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
                wgpu::BindGroupEntry { binding: 0, resource: state.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: derived.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: canvas.as_entire_binding() },
            ],
        });
        let derive_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("camera derive"),
            bind_group_layouts: &[&derive_bgl],
            push_constant_ranges: &[],
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
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: derived.as_entire_binding() }],
        });

        Camera { state, derived, canvas, derive, derive_bg, read_bgl, read_bg }
    }

    /// What a renderer's pipeline layout names.
    pub(crate) fn layout(&self) -> &wgpu::BindGroupLayout {
        &self.read_bgl
    }

    /// What a renderer's draw sets. One bind group for every L4 in the Set:
    /// **one camera serves every node**, which is what stops two renderers in a
    /// Set disagreeing about where the frame is being watched from.
    pub(crate) fn bind_group(&self) -> &wgpu::BindGroup {
        &self.read_bg
    }

    /// The host producer: six numbers straight into the edge.
    ///
    /// **The scalars ride in the padding after the vectors**, which is what
    /// makes `CameraState` 48 bytes rather than 64 and is the layout the two
    /// shaders that name it agree on.
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

    /// The canvas the derivation projects onto.
    ///
    /// Separate from the state on purpose: an aspect ratio is a property of what
    /// is being drawn into, so a resize changes this and leaves the camera
    /// alone.
    pub(crate) fn write_canvas(&self, queue: &wgpu::Queue, aspect: f32) {
        let mut bytes = [0u8; 16];
        bytes[..4].copy_from_slice(&aspect.to_le_bytes());
        queue.write_buffer(&self.canvas, 0, &bytes);
    }

    /// Derive, ahead of anything that reads a camera this frame.
    ///
    /// **One dispatch of one invocation.** Recorded per frame rather than
    /// cached, because the state moves every frame and there is no cheaper test
    /// for "did it" than doing it.
    pub(crate) fn record(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("camera derive"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.derive);
        pass.set_bind_group(0, &self.derive_bg, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
}

#[cfg(test)]
mod tests {
    //! **Two derivations of one camera, held against each other.**
    //!
    //! `camera::State` still derives on the host, for the debug overlay in
    //! [`crate::points`], and `shaders/camera.wgsl` derives on the GPU for
    //! everything a `.kir` draws. That is exactly the hazard the host module
    //! warns about — two cameras that agree until one of them is edited — and
    //! the only thing that turns it into a checked property is reading the
    //! buffer back and comparing.
    //!
    //! **A readback, in a test, on purpose.** The frame path never does this;
    //! that is the whole reason the derivation moved to the GPU. Here it costs
    //! one stall and buys the guarantee that moving it did not change the
    //! picture.

    use super::*;
    use crate::gpu::Gpu;
    use crate::node::simulation::read_buffer;

    fn f32_at(bytes: &[u8], at: usize) -> f32 {
        f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
    }

    fn vec3_at(bytes: &[u8], at: usize) -> [f32; 3] {
        [f32_at(bytes, at), f32_at(bytes, at + 4), f32_at(bytes, at + 8)]
    }

    fn close(a: f32, b: f32, what: &str) {
        // Relative, because `view_proj`'s entries span the frustum's whole
        // scale: `far / (near - far)` is about -1 and `near * far / (near -
        // far)` is about -0.1, while a pre-scaled basis vector is a fraction.
        let tol = 1e-5 * a.abs().max(b.abs()).max(1.0);
        assert!((a - b).abs() <= tol, "{what}: host {a}, gpu {b}");
    }

    #[test]
    fn the_pass_derives_what_the_host_would_have() {
        let Ok(gpu) = Gpu::headless() else {
            eprintln!("no adapter; skipping");
            return;
        };
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

        let cam = Camera::new(&gpu.device);
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
                close(*want, f32_at(&bytes, (c * 4 + r) * 4), &format!("view_proj[{c}][{r}]"));
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

    /// **The aspect ratio is the canvas's and not the camera's**, which is only
    /// visible as the two places it reaches: the horizontal scale of the
    /// projection, and the pre-scaled `right` a marching ray is built from.
    /// Nothing else in the derived block may move with it.
    #[test]
    fn the_canvas_widens_the_projection_and_leaves_the_camera_where_it_is() {
        let Ok(gpu) = Gpu::headless() else {
            eprintln!("no adapter; skipping");
            return;
        };
        let state = State {
            eye: [0.0, 1.0, 5.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            fov_y: 1.0,
            near: 0.1,
            far: 50.0,
        };
        let cam = Camera::new(&gpu.device);
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
        close(f32_at(&square, 0) * 0.5, f32_at(&wide, 0), "view_proj[0][0]");
        for i in 0..3 {
            close(vec3_at(&square, 96)[i] * 2.0, vec3_at(&wide, 96)[i], "right");
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
