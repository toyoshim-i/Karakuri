pub use std::sync::mpsc::{self, Sender};
pub use std::time::{Duration, Instant};

pub use karakuri_engine::governor::Estimated;
pub use karakuri_engine::swap::{Event, HotSwap, Polled, Refusal, Request, Source};
pub use karakuri_engine::{Basis, Binding, Curve, Gpu, Present, Set, Signals, Unfit, VideoSource};
pub use karakuri_ir::typed::Checked;

/// Deliberately small for the structural tests: what they check does not
/// depend on the workload, and a dozen of them at a realistic size would make
/// `cargo test` a coffee break. The measurement at the bottom of this file
/// uses [`REAL`] instead, because its numbers do depend on it.
pub const WIDTH: u32 = 256;
pub const HEIGHT: u32 = 256;

/// The two capacities the structural tests build at. They differ so that which
/// Set is live is observable from outside: `capacity` is a Set-level dial, so
/// the same pair of procedures at two capacities is two Sets and one artifact.
pub const FIRST: u32 = 4096;
pub const SECOND: u32 = 8192;

/// The workload the reported numbers are taken at — the CLI's own defaults,
/// so that they are comparable with the other host-clock figures in this
/// repository rather than being a measurement of a toy. That one workload
/// is the rule and not a coincidence: `docs/contributing.md` §1.
pub const REAL: (u32, (u32, u32)) = (262_144, (1280, 720));

/// A budget no frame in this harness will come near, for the tests that want a
/// candidate accepted rather than rolled back.
pub const GENEROUS_MS: f32 = 10_000.0;

/// How long a test will spin waiting for the worker before giving up. Generous:
/// it covers WGSL generation, two `create_shader_module` calls, pipeline
/// creation, and a whole-capacity buffer upload, on whatever machine CI turns
/// out to be.
pub const PATIENCE: Duration = Duration::from_secs(30);

pub const L1: &str = r#"
proc static_shell {
  kind     L1
  topology points
  capacity [1024, 262144] = 4096

  param radius : float [0.1, 8.0] = 2.5

  emit position, age

  element {
    let u = hash1(seed);
    let v = hash1(seed + 1000u);
    position = sphere_point(u, v) * radius;
    age      = age + dt;
  }
}
"#;

pub const L4: &str = r#"
proc soft_points {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 1.0

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.015625;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    color = vec4(vec3(1.0, 1.0, 1.0) * exposure, max(0.0, 1.0 - d));
  }
}
"#;

/// A viewport with no room under it for two rungs, which is the estimate's
/// deterministic refusal — see
/// [`a_candidate_the_estimator_refuses_is_judged_on_its_measurement`].
pub const TINY: (u32, u32) = (2, 2);

/// Shader fixture that consumes an unavailable attribute (`normal`) to trigger worker build failure.
pub const L4_INCOMPATIBLE: &str = r#"
proc wants_normal {
  kind  L4
  blend additive

  consumes position, normal

  vertex {
    clip       = camera * vec4(position + normal, 1.0);
    point_rate = 0.015625;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;

pub use crate::engine_common::{compile, render};

/// A second renderer over the same geometry: bigger sprites, its own
/// `exposure`. Declaring that name twice in one Set is the thing a flat
/// parameter map could not hold.
pub const L4_WIDE: &str = r#"
proc wide_points {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 0.5

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.04296875;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    color = vec4(vec3(0.2, 0.9, 1.0) * exposure, max(0.0, 1.0 - d));
  }
}
"#;

pub fn request(l4_src: &str, capacity: u32, label: &str) -> Request {
    request_many(&[l4_src], capacity, label)
}

pub fn request_many(l4_srcs: &[&str], capacity: u32, label: &str) -> Request {
    Request {
        names: karakuri_engine::swap::RequestNames::default(),
        edges: Vec::new(),
        id: 1,
        l1s: vec![(compile(L1), capacity)],
        l2s: Vec::new(),
        l3s: Vec::new(),
        fields: Vec::new(),
        layering: karakuri_engine::set::Layering::Overdraw,
        live: None,
        published: Vec::new(),
        l4s: l4_srcs.iter().map(|s| compile(s)).collect(),
        seed_salt: 19274,
        camera: karakuri_engine::camera::Orbit::default(),
        salts: Vec::new(),
        params: Vec::new(),
        bindings: Vec::new(),
        authorities: Vec::new(),
        label: label.to_string(),
    }
}

/// A source that never has anything to build — a watcher over a file
/// nobody is editing. It still has to sleep, or it spins the worker.
pub struct Silent;

impl Source for Silent {
    fn poll(&mut self) -> Option<Polled> {
        std::thread::sleep(karakuri_engine::swap::POLL_INTERVAL);
        None
    }
}

pub struct Harness {
    pub gpu: Gpu,
    pub present: Present,
    pub size: (u32, u32),
    pub swap: HotSwap,
    /// Every frame interval this harness has measured for itself, in
    /// milliseconds. Independent of the one `HotSwap` keeps, so the printed
    /// numbers are not the watchdog reporting on its own homework.
    pub intervals: Vec<f32>,
    pub last: Option<Instant>,
    /// `capacity` observed at the top and at the bottom of each frame body. If
    /// these ever differ, a swap landed in the middle of a frame.
    pub frame_capacities: Vec<(u32, u32)>,
}

impl Harness {
    pub fn new(
        budget_ms: f32,
        capacity: u32,
        size: (u32, u32),
        source: Box<dyn Source>,
    ) -> Harness {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(
            &gpu.device,
            wgpu::TextureFormat::Rgba16Float,
            size.0,
            size.1,
        );
        let set = Harness::build(&gpu, L4, capacity);
        let mut swap = HotSwap::new(&gpu.device, &gpu.queue, set, budget_ms, source);
        swap.resize(&gpu.device, size.0, size.1);
        Harness {
            gpu,
            present,
            size,
            swap,
            intervals: Vec::new(),
            last: None,
            frame_capacities: Vec::new(),
        }
    }

    /// A harness plus the `Sender` that drives it, for the tests that decide
    /// when a rebuild happens rather than watching a file for it.
    pub fn channel_driven(budget_ms: f32) -> (Harness, Sender<Request>) {
        Harness::channel_driven_at(budget_ms, FIRST, (WIDTH, HEIGHT))
    }

    pub fn channel_driven_at(
        budget_ms: f32,
        capacity: u32,
        size: (u32, u32),
    ) -> (Harness, Sender<Request>) {
        let (tx, rx) = mpsc::channel();
        (Harness::new(budget_ms, capacity, size, Box::new(rx)), tx)
    }

    pub fn build(gpu: &Gpu, l4_src: &str, capacity: u32) -> Set {
        Set::build(
            &gpu.device,
            &gpu.queue,
            &compile(L1),
            &compile(l4_src),
            capacity,
            19274,
        )
        .expect("the pair is compatible and the capacity is in range")
    }

    /// One frame, shaped exactly like the CLI's: `begin_frame`, then the whole
    /// frame recorded through the borrow it returns.
    pub fn frame(&mut self) {
        let now = Instant::now();
        if let Some(last) = self.last.replace(now) {
            self.intervals
                .push(now.duration_since(last).as_secs_f32() * 1_000.0);
        }

        let hdr = self.present.hdr_view();
        let device = &self.gpu.device;
        let queue = &self.gpu.queue;

        let set = self.swap.begin_frame(device);
        let at_top = set.capacity();
        set.prepare(queue, 1, &Signals::default());
        let mut encoder = device.create_command_encoder(&Default::default());
        set.render(&mut encoder, hdr, 1);
        let at_bottom = set.capacity();
        queue.submit([encoder.finish()]);
        set.commit();

        self.frame_capacities.push((at_top, at_bottom));
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
    }

    /// **Turn a knob on the Set that is playing**, which is what
    /// `Deck::write_param` does for a surface: `begin_frame` is the public
    /// road to a live `&mut Set` and the write is a uniform value, on screen
    /// at the next frame with no build.
    pub fn ride(&mut self, key: &str, value: f32) {
        let device = &self.gpu.device;
        let live = self.swap.begin_frame(device);
        let landed = live
            .write_param(&karakuri_engine::ParamWrite::everywhere(key, value))
            .expect("nothing here grants a node away, so no write crosses an authority");
        assert!(landed > 0, "nothing in the live Set declares `{key}`");
    }

    /// Render frames until `wanted` matches an event, and return how many
    /// frames that took.
    pub fn frames_until(
        &mut self,
        wanted: impl Fn(&Event) -> bool,
        what: &str,
    ) -> (u64, Vec<String>) {
        let started = Instant::now();
        let from = self.swap.frames_rendered();
        let mut seen = Vec::new();
        loop {
            self.frame();
            let mut found = false;
            for event in self.swap.events() {
                found |= wanted(&event);
                seen.push(event.to_string());
            }
            if found {
                return (self.swap.frames_rendered() - from, seen);
            }
            assert!(
                started.elapsed() < PATIENCE,
                "waited {PATIENCE:?} for {what} and it never happened; saw {seen:?}"
            );
        }
    }
}

pub fn is_swapped(event: &Event) -> bool {
    matches!(event, Event::Swapped { .. })
}

pub fn steps_taken(set: &Set) -> u64 {
    (set.time() * 60.0).round() as u64
}

pub fn budgeted_by_the_one_rule(swap: &HotSwap) -> (Basis, Option<f32>) {
    karakuri_engine::governor::budgeted(
        swap.measured_cost(),
        swap.estimated_cost().map(Estimated::from),
    )
}

pub const L1_STATEFUL: &str = r#"
proc fountain {
  kind     L1
  topology points
  capacity [1024, 262144] = 4096

  param spawn_rate : float [0.0, 40000.0] = 700.0
  param speed      : float [0.0, 16.0]    = 4.0

  emit position, velocity, age

  spawn {
    position = vec3(0.0, 0.0, 0.0);
    velocity = sphere_point(hash1(seed), hash1(seed + 7u)) * speed;
    age      = 0.0;
  }

  element {
    position = position + velocity * dt;
    velocity = velocity + vec3(0.0, -2.0, 0.0) * dt;
    age      = age + dt;
    if age > 0.35 {
      kill();
    }
  }
}
"#;

pub fn stateful(gpu: &Gpu) -> Set {
    let mut set = Set::build(
        &gpu.device,
        &gpu.queue,
        &compile(L1_STATEFUL),
        &compile(L4),
        FIRST,
        19274,
    )
    .expect("the pair is compatible and the capacity is in range");
    set.resize(&gpu.device, WIDTH, HEIGHT);
    set
}

pub fn drive(gpu: &Gpu, set: &mut Set, view: &wgpu::TextureView, steps: u8) {
    set.prepare(&gpu.queue, steps, &Signals::default());
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    set.render(&mut encoder, view, steps);
    gpu.queue.submit([encoder.finish()]);
    set.commit();
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
}

pub fn pixels(gpu: &Gpu, texture: &wgpu::Texture) -> Vec<u16> {
    let (width, height) = (texture.width(), texture.height());
    let bytes_per_row = width * 8;
    assert_eq!(bytes_per_row % 256, 0, "row pitch must be 256-byte aligned");
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("rewind readback"),
        size: u64::from(bytes_per_row * height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit([encoder.finish()]);
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let data = slice.get_mapped_range().expect("map");
    let out = data
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    drop(data);
    buffer.unmap();
    out
}

pub const L1_EDITED_DEFAULT: &str = r#"
proc static_shell {
  kind     L1
  topology points
  capacity [1024, 262144] = 4096

  param radius : float [0.1, 8.0] = 5.0

  emit position, age

  element {
    let u = hash1(seed);
    let v = hash1(seed + 1000u);
    position = sphere_point(u, v) * radius;
    age      = age + dt;
  }
}
"#;

pub const L4_NO_EXPOSURE: &str = r#"
proc plain_points {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.015625;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    color = vec4(1.0, 1.0, 1.0, max(0.0, 1.0 - d));
  }
}
"#;

pub fn request_from(l1_src: &str, l4_srcs: &[&str], label: &str) -> Request {
    let mut r = request_many(l4_srcs, FIRST, label);
    r.l1s = vec![(compile(l1_src), FIRST)];
    r
}

pub fn value_at(set: &Set, layer: karakuri_ir::Kind, index: u32, key: &str) -> Option<f32> {
    set.params()
        .find(|(l, i, k, _)| *l == layer && *i == index && *k == key)
        .map(|(_, _, _, v)| v)
}
