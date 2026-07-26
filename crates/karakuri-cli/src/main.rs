//! The V1 entry point.
//!
//! Two `.kir` files go in, through parse, type and contract checking, cost
//! estimation, WGSL generation, and pipeline creation, and come out as either a
//! window or a PNG. Nothing here is hand-written shader code.
//!
//! This is also the only place a clock is read. The engine advances by `steps`
//! from a `tick` record and never measures anything; deriving that count from
//! elapsed real time is the job of whoever drives the engine live, and on
//! replay it is read back from the stream instead. Keeping the measurement out
//! here is what lets the same engine code be deterministic.

mod compile;
mod render;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use karakuri_engine::{Gpu, Present, Set, VideoSource};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

/// The fixed simulation step. Not the frame delta.
const DT: f32 = 1.0 / 60.0;

/// Past this the simulation falls behind rather than catching up. Unbounded
/// catch-up turns a load spike into a death spiral.
const MAX_STEPS: u8 = 4;

const SEED: u32 = 19_274;

struct Args {
    /// `--param name=value`, applied after the Set is built. A parameter
    /// change is a uniform write, not a structural change, which is why it
    /// needs no fork and no recompilation.
    overrides: Vec<(String, f32)>,
    l1: PathBuf,
    l4: PathBuf,
    capacity: u32,
    render_to: Option<PathBuf>,
    seq_to: Option<PathBuf>,
    frames: u32,
    size: (u32, u32),
}

fn parse_args() -> Args {
    let mut args = Args {
        overrides: Vec::new(),
        l1: "examples/drift_shell.kir".into(),
        l4: "examples/soft_points.kir".into(),
        capacity: 262_144,
        render_to: None,
        seq_to: None,
        frames: 240,
        size: (1280, 720),
    };
    let mut positional = Vec::new();
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--render" => args.render_to = it.next().map(PathBuf::from),
            "--seq" => args.seq_to = it.next().map(PathBuf::from),
            "--param" => {
                if let Some((k, v)) = it.next().and_then(|s| {
                    s.split_once('=')
                        .and_then(|(k, v)| v.parse().ok().map(|v| (k.to_string(), v)))
                }) {
                    args.overrides.push((k, v));
                }
            }
            "--frames" => args.frames = it.next().and_then(|v| v.parse().ok()).unwrap_or(240),
            "--capacity" => {
                args.capacity = it.next().and_then(|v| v.parse().ok()).unwrap_or(262_144)
            }
            "--size" => {
                if let Some(v) = it.next() {
                    if let Some((w, h)) = v.split_once('x') {
                        if let (Ok(w), Ok(h)) = (w.parse(), h.parse()) {
                            args.size = (w, h);
                        }
                    }
                }
            }
            _ => positional.push(PathBuf::from(arg)),
        }
    }
    if positional.len() == 2 {
        args.l1 = positional[0].clone();
        args.l4 = positional[1].clone();
    }
    args
}

fn main() {
    let args = parse_args();

    eprintln!("compiling:");
    let l1 = match compile::load(&args.l1) {
        Ok(c) => c,
        Err(report) => {
            eprintln!("{report}");
            std::process::exit(1);
        }
    };
    let l4 = match compile::load(&args.l4) {
        Ok(c) => c,
        Err(report) => {
            eprintln!("{report}");
            std::process::exit(1);
        }
    };

    match args.render_to.clone().or(args.seq_to.clone()) {
        Some(path) => {
            let gpu = Gpu::headless().expect("no GPU");
            let mut set = build(&gpu, &l1, &l4, args.capacity, &args.overrides);
            let (w, h) = args.size;
            eprintln!(
                "rendering {w}x{h}, {} elements, {} frames -> {}",
                args.capacity,
                args.frames,
                path.display()
            );
            let result = if args.seq_to.is_some() {
                render::to_sequence(&gpu, &mut set, w, h, args.frames, &path)
            } else {
                render::to_png(&gpu, &mut set, w, h, args.frames, &path)
            };
            if let Err(e) = result {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        None => {
            let event_loop = EventLoop::new().expect("event loop");
            event_loop.set_control_flow(ControlFlow::Poll);
            event_loop
                .run_app(&mut App {
                    args,
                    procs: Some((l1, l4)),
                    live: None,
                })
                .expect("run");
        }
    }
}

fn build(
    gpu: &Gpu,
    l1: &karakuri_ir::typed::Checked,
    l4: &karakuri_ir::typed::Checked,
    capacity: u32,
    overrides: &[(String, f32)],
) -> Set {
    match Set::build(&gpu.device, &gpu.queue, l1, l4, capacity, SEED) {
        Ok(mut set) => {
            for (name, value) in overrides {
                match set.params.get_mut(name) {
                    Some(slot) => *slot = *value,
                    None => eprintln!("  no parameter named `{name}`, ignoring"),
                }
            }
            set
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

struct App {
    args: Args,
    procs: Option<(karakuri_ir::typed::Checked, karakuri_ir::typed::Checked)>,
    live: Option<Live>,
}

struct Live {
    window: Arc<Window>,
    gpu: Gpu,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    present: Present,
    set: Set,
    /// Fractional steps carried between frames, so a frame rate that does not
    /// divide the step rate still advances at the right average rate. The same
    /// accumulator shape as spawn quantisation, for the same reason.
    carry: f32,
    last: Instant,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let Some((l1, l4)) = self.procs.take() else {
            return;
        };

        let attrs = Window::default_attributes()
            .with_title("Karakuri")
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.args.size.0,
                self.args.size.1,
            ));
        let window = Arc::new(event_loop.create_window(attrs).expect("window"));
        let size = window.inner_size();

        let instance = Gpu::instance();
        let surface = instance.create_surface(window.clone()).expect("surface");
        let gpu = pollster::block_on(Gpu::from_instance(instance, Some(&surface))).expect("gpu");

        let caps = surface.get_capabilities(&gpu.adapter);
        // sRGB encoding happens once, at final output: pick a surface format
        // that carries the transfer function so the hardware does it on write.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: caps.present_modes[0],
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&gpu.device, &config);

        let present = Present::new(&gpu.device, format, config.width, config.height);
        let mut set = build(&gpu, &l1, &l4, self.args.capacity, &self.args.overrides);
        set.resize(config.width, config.height);

        eprintln!(
            "running: {} elements on {}",
            set.capacity(),
            gpu.adapter.get_info().name
        );

        self.live = Some(Live {
            window,
            gpu,
            surface,
            config,
            present,
            set,
            carry: 0.0,
            last: Instant::now(),
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(live) = self.live.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => live.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                live.frame();
                live.window.request_redraw();
            }
            _ => {}
        }
    }
}

impl Live {
    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.gpu.device, &self.config);
        self.present.resize(&self.gpu.device, width, height);
        self.set.resize(width, height);
    }

    /// The one measurement in the program: elapsed real time becomes a step
    /// count, which is exactly what a `tick` record carries.
    fn steps(&mut self) -> u8 {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last).as_secs_f32();
        self.last = now;

        self.carry += elapsed / DT;
        let whole = self.carry.floor();
        self.carry -= whole;
        (whole as u32).min(u32::from(MAX_STEPS)) as u8
    }

    fn frame(&mut self) {
        let steps = self.steps();

        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.gpu.device, &self.config);
                return;
            }
            Err(_) => return,
        };
        let view = frame.texture.create_view(&Default::default());

        self.set.prepare(&self.gpu.queue, steps);

        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
        self.set
            .render(&mut encoder, self.present.hdr_view(), steps);
        self.present.draw(&mut encoder, &view);
        self.gpu.queue.submit([encoder.finish()]);
        frame.present();
    }
}
