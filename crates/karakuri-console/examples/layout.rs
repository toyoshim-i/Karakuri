//! The console's arrangement, on screen and under a mouse.
//!
//! `karakuri-layout` solves an arrangement and `karakuri-console` states one
//! and drives it, and until this example nothing had ever driven it *by hand*:
//! every claim about the panel was a test's. This is the other half — a window
//! that paints each visible region as a flat rectangle, with the gaps between
//! them left unpainted so the dividers are exactly what is not drawn, and real
//! pointer and key input on top of it.
//!
//! It draws no text, on purpose: whether a divider drags is answered by
//! watching a rectangle move, and a text toolkit here would pre-empt the
//! decision this harness exists to inform. **stdout is the readout instead** —
//! a legend of every region and the colour it was given at startup, and one
//! line per operation as it happens. The line that matters most is a drag's:
//! it prints what was asked for beside where [`Panel::moved`] says it landed,
//! and the gap between "asked for 400" and "landed at 378" is a constraint
//! biting, which is the thing a person is here to see.
//!
//! ```sh
//! cargo run -p karakuri-console --example layout
//! cargo test -p karakuri-console                    # the model, without a window
//! cargo test -p karakuri-console --example layout   # this file's own two tests
//! ```
//!
//! # This file owns none of the model
//!
//! The arrangement, the drag in progress and the operations that act on what
//! is under the pointer are [`karakuri_console::panel`], and they are there
//! rather than here because the egui view needs exactly them: a model grown a
//! second time is two answers to *how a pointer moves a divider*, and two
//! answers disagree quietly, each with its own passing tests. What is left in
//! this file is a window, a painter, and the formatting — a [`Dragged`] into
//! the line above, an [`Outcome`] into the line a key prints. **The model
//! returns what happened; the English is this file's.**
//!
//! So the tests that drive the model are `tests/panel.rs`, and the two that
//! remain here are the two that cannot leave: the table of what to do about a
//! frame that could not be acquired, which is a `wgpu` enum, and the painter
//! on a device, under `mod gpu` for the reason every other one in the
//! workspace is.
//!
//! # Two rules this obeys, because they are what a real view will have to
//!
//! **Solve once per frame, after the input.** `Layout::rect` and
//! `Layout::hit` carry a `debug_assert!` that the layout is not dirty, so an
//! operation followed by a read is a panic in a debug build. Every frame here
//! solves once and then reads, which the `Panel` does on its behalf; an input
//! event that has to hit-test solves first, which is a flag test on a frame
//! where nothing moved, and never once per read.
//!
//! **Nothing here shadows the model.** Not one fact about the arrangement is
//! kept in this file — a folded region is `is_collapsed`, a solo is
//! `is_soloed`, and what a drag did is what `moved` returned. The one thing
//! this file does hold is a colour per region, which is the readout's and not
//! the panel's.
//!
//! # The readout is driven by the layout, not by the pointer
//!
//! A line is printed when the boundary **moves**, and a stop is announced
//! once. That is [`Panel::moved`]'s doing rather than this file's: it returns
//! nothing at all for a move that changed nothing. It is tempting to print a
//! line per pointer event — it is one comparison less — and it is wrong twice
//! over: the interesting lines are buried under identical ones, and a hand
//! held against the edge of the window, which is where a drag ends up,
//! produces hundreds of them a second into a terminal that has to keep up with
//! them while the window waits.
//!
//! # Every frame that could not be acquired gets a decision
//!
//! This loop waits for events rather than spinning, so nothing asks for
//! another frame on its own. A `get_current_texture` that comes back
//! `Outdated` and is dropped with a bare `return` is therefore a window that
//! stops drawing and never starts again, with nothing said anywhere — see
//! `missed`, and `karakuri-cli`'s window sink, which carries the same table
//! after the same failure.
//!
//! # No region is named anywhere in this file
//!
//! Every region is reached by walking the arrangement and asking
//! `Layout::name` what it is called. So the harness is whatever the
//! arrangement currently says it is, and renaming a region does not touch this
//! file.

use std::sync::Arc;

use karakuri_console::panel::{extent, Dragged, Op, Outcome, Panel, Pressed, Released, Visibility};
use karakuri_engine::Gpu;
use karakuri_layout::{Axis, NodeId, Point, Rect};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// The window this opens, in logical pixels. Comfortably above the smallest
/// viewport the arrangement is claimed to work at, so nothing starts clamped.
const WINDOW: (f64, f64) = (1440.0, 900.0);

/// Enough for any arrangement this crate builds; the buffer is allocated once
/// and never grows, so a frame allocates nothing.
const MAX_RECTS: usize = 64;

// ---------------------------------------------------------------------------
// The readout: a colour per region, and English for what the model returned.
// No window, no device.
// ---------------------------------------------------------------------------

/// The panel, plus the two things a readout needs that a model has no business
/// holding: a colour per region and the words for what just happened.
struct Readout {
    panel: Panel,
    /// One per [`Panel::nodes`], in the same order. A leaf gets a colour; a
    /// split gets `None`, because a split is the thing whose gaps are visible
    /// between its children rather than something painted.
    colours: Vec<Option<[f32; 3]>>,
}

impl Readout {
    fn new(width: f32, height: f32) -> Readout {
        let mut readout = Readout {
            panel: Panel::new(width, height),
            colours: Vec::new(),
        };
        readout.recolour();
        readout
    }

    /// Give every leaf a colour, in tree order, so neighbours differ and the
    /// legend reads top to bottom. Called again whenever the panel rebuilds.
    fn recolour(&mut self) {
        let leaves = self.panel.nodes().iter().filter(|n| n.leaf).count().max(1);
        let mut nth = 0;
        self.colours = self
            .panel
            .nodes()
            .iter()
            .map(|node| {
                node.leaf.then(|| {
                    let colour = hue(nth as f32 / leaves as f32);
                    nth += 1;
                    colour
                })
            })
            .collect();
    }

    // -- the words ------------------------------------------------------

    fn label(&self, id: NodeId) -> String {
        match self.panel.layout().name(id) {
            Some(name) => name.to_owned(),
            None => "(unnamed split)".to_owned(),
        }
    }

    /// The two regions a boundary is between. A split is often unnamed — the
    /// console's body row is, deliberately — so `split #0` alone does not say
    /// which boundary the pointer has hold of, and the pair does.
    fn pair(&self, split: NodeId, index: usize) -> String {
        match self.panel.pair(split, index) {
            Some((a, b)) => format!("{} | {}", self.label(a), self.label(b)),
            None => "no pair".to_owned(),
        }
    }

    // -- input ----------------------------------------------------------

    fn press(&mut self, p: Point) {
        match self.panel.press(p) {
            Pressed::Grabbed {
                split,
                index,
                axis,
                at,
                offset,
            } => println!(
                "press ({:.0}, {:.0}): the boundary {} — divider #{} of {}, {:?} — is at \
                 {:.1}, grabbed {:+.1} from it",
                p.x,
                p.y,
                self.pair(split, index),
                index,
                self.label(split),
                axis,
                at,
                offset
            ),
            Pressed::NoPair { .. } => {
                println!("press ({:.0}, {:.0}): a divider with no pair", p.x, p.y)
            }
            Pressed::Region { id, rect } => println!(
                "press ({:.0}, {:.0}): region {} at {:.0},{:.0} {:.0}x{:.0}",
                p.x,
                p.y,
                self.label(id),
                rect.x,
                rect.y,
                rect.w,
                rect.h
            ),
            Pressed::Nothing => println!("press ({:.0}, {:.0}): nothing", p.x, p.y),
        }
    }

    fn moved(&mut self, p: Point) {
        let Some(dragged) = self.panel.moved(p) else {
            return;
        };
        println!("{}", self.say_drag(dragged));
    }

    /// A drag, in words. Separate from the printing so the shape of the line
    /// is one expression: what was asked, where it landed, what held it, and
    /// what the pair either side is now.
    fn say_drag(&self, d: Dragged) -> String {
        let sizes = match self.panel.pair(d.split, d.index) {
            Some((a, b)) => format!(
                "{} {:.0} | {} {:.0}",
                self.label(a),
                extent(d.axis, self.panel.layout().rect(a)),
                self.label(b),
                extent(d.axis, self.panel.layout().rect(b))
            ),
            None => "no pair".to_owned(),
        };
        let stop = match d.held {
            Some(by) => format!(" — held {by:+.1} by a stop, and it stays there until it moves"),
            None => String::new(),
        };
        format!(
            "  drag: asked {:.1}, landed {:.1}{stop} [{sizes}]",
            d.asked, d.landed
        )
    }

    fn released(&mut self) {
        match self.panel.released() {
            Some(Released::Rests { split, index, at }) => {
                println!("release: {} rests at {at:.1}", self.pair(split, index))
            }
            Some(Released::Gone { split, index }) => {
                println!("release: {} is gone", self.pair(split, index))
            }
            None => {}
        }
    }

    fn op(&mut self, op: Op) {
        let outcome = self.panel.op(op);
        if outcome == Outcome::Reset {
            self.recolour();
        }
        self.say_op(op, &outcome);
    }

    /// What an operation did, in words. The model returns the facts; which
    /// English they take is the operation that was asked for, which is why
    /// this has both.
    fn say_op(&self, op: Op, outcome: &Outcome) {
        match outcome {
            Outcome::Folded { id, folded, root } => {
                let what = match op {
                    Op::FoldEnclosing => "the split ",
                    _ => "",
                };
                println!(
                    "fold: {what}{} is now {}{}",
                    self.label(*id),
                    folding(*folded),
                    match *root && *folded {
                        true => " — that was the root, so the panel is empty; z brings it back",
                        false => "",
                    }
                );
            }
            Outcome::OnDivider { split, index } => println!(
                "fold: the pointer is on divider {}#{index} — move it into a region",
                self.label(*split)
            ),
            Outcome::Unfolded(ids) => match ids.is_empty() {
                true => println!("unfold: nothing is folded"),
                false => {
                    let names: Vec<String> = ids.iter().map(|id| self.label(*id)).collect();
                    println!("unfold: {}", names.join(", "));
                }
            },
            Outcome::Soloed(id) => println!(
                "solo: {} — everything else folded (soloed = {})",
                self.label(*id),
                self.panel.layout().is_soloed()
            ),
            Outcome::Unsoloed { was } => println!(
                "unsolo: {}",
                match was {
                    true => "the arrangement before the solo is back",
                    false => "nothing was soloed",
                }
            ),
            Outcome::Reset => println!("reset: a fresh arrangement, at the same viewport"),
            Outcome::Report(rows) => {
                println!("regions:");
                let lines: Vec<String> = rows
                    .iter()
                    .map(|row| {
                        format!(
                            "  {:width$}{:<18} {:>7.1},{:>7.1}  {:>7.1} x {:>7.1} {}",
                            "",
                            self.label(row.id),
                            row.rect.x,
                            row.rect.y,
                            row.rect.w,
                            row.rect.h,
                            match row.state {
                                Visibility::Folded => "folded",
                                Visibility::InsideAFold => "inside a fold",
                                Visibility::Visible => "",
                            },
                            width = row.depth * 2
                        )
                    })
                    .collect();
                println!("{}", lines.join("\n"));
            }
            // The three operations that act on what is under the pointer are
            // the only ones that can find nothing there.
            Outcome::Nothing => println!(
                "{}",
                match op {
                    Op::Fold => "fold: nothing under the pointer".to_owned(),
                    Op::FoldEnclosing => "fold: nothing encloses the pointer".to_owned(),
                    Op::Solo => "solo: no region under the pointer".to_owned(),
                    other => format!("{other:?}: nothing under the pointer"),
                }
            ),
        }
    }

    // -- the legend -----------------------------------------------------

    fn print_legend(&mut self) {
        self.panel.solve();
        let layout = self.panel.layout();
        let viewport = layout.viewport();
        println!();
        println!(
            "the console's arrangement, in a {:.0} x {:.0} viewport. every leaf is painted in \
             its own colour and the dividers are the gaps between them.",
            viewport.w, viewport.h
        );
        println!();
        for (node, colour) in self.panel.nodes().iter().zip(&self.colours) {
            let (min, max) = layout.bounds(node.id);
            let bounds = format!(
                "min {min:.0}, max {}",
                match max.is_finite() {
                    true => format!("{max:.0}"),
                    false => "none".to_owned(),
                }
            );
            let colour = match colour {
                Some(c) => {
                    let [r, g, b] = encode(*c);
                    format!("rgb({r:>3}, {g:>3}, {b:>3})")
                }
                None => match layout.axis(node.id) {
                    Some(Axis::Row) => "split, left to right".to_owned(),
                    Some(Axis::Column) => "split, top to bottom".to_owned(),
                    None => "empty".to_owned(),
                },
            };
            println!(
                "  {:width$}{:<16} {:<22} {}",
                "",
                self.label(node.id),
                colour,
                bounds,
                width = node.depth * 2
            );
        }
        println!();
        println!("keys — the pointer's position decides what each one acts on:");
        println!("  drag     press the left button in a gap and move: the boundary follows");
        println!("  f        fold the region under the pointer");
        println!("  g        fold the split enclosing the region under the pointer");
        println!("  z        unfold everything folded (a folded region has no rectangle, so");
        println!("           the pointer cannot reach it to unfold it)");
        println!("  s        solo the region under the pointer");
        println!("  u        undo the solo");
        println!("  r        reset to a fresh arrangement");
        println!("  p        print every region's rectangle");
        println!("  esc      quit");
        println!();
    }

    /// Every rectangle to paint this frame, in tree order. Solves first, and
    /// this is the only solve a quiet frame does.
    fn painted(&mut self, out: &mut Vec<(Rect, [f32; 3])>) {
        self.panel.solve();
        out.clear();
        for (node, colour) in self.panel.nodes().iter().zip(&self.colours) {
            let Some(colour) = colour else {
                continue;
            };
            if !self.panel.layout().visible(node.id) {
                continue;
            }
            let r = self.panel.layout().rect(node.id);
            if r.w > 0.0 && r.h > 0.0 {
                out.push((r, *colour));
            }
        }
    }
}

fn folding(folded: bool) -> &'static str {
    match folded {
        true => "folded",
        false => "unfolded",
    }
}

/// A colour wheel, so neighbouring regions never share a hue.
fn hue(t: f32) -> [f32; 3] {
    let h = (t * 6.0) % 6.0;
    let (s, v) = (0.62_f32, 0.88_f32);
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    // Linear, because the surface format carries the transfer function and the
    // hardware encodes on write — see P-0064, sRGB is encoded once, at final
    // output. The legend prints what the eye sees, which is the encoded form.
    [decode(r + m), decode(g + m), decode(b + m)]
}

/// sRGB to linear.
fn decode(c: f32) -> f32 {
    match c <= 0.04045 {
        true => c / 12.92,
        false => ((c + 0.055) / 1.055).powf(2.4),
    }
}

/// Linear back to the 0-255 sRGB the legend quotes.
fn encode(c: [f32; 3]) -> [u8; 3] {
    c.map(|v| {
        let s = match v <= 0.003_130_8 {
            true => v * 12.92,
            false => 1.055 * v.powf(1.0 / 2.4) - 0.055,
        };
        (s * 255.0).round().clamp(0.0, 255.0) as u8
    })
}

// ---------------------------------------------------------------------------
// The window
// ---------------------------------------------------------------------------

/// Flat rectangles and nothing else: one pipeline, one vertex buffer written
/// per frame. Both are built once, at startup — see P-0001.
struct Painter {
    pipeline: wgpu::RenderPipeline,
    vertices: wgpu::Buffer,
    /// Reused, so a frame allocates nothing.
    bytes: Vec<u8>,
}

/// Position and colour, interleaved.
const STRIDE: u64 = 20;

impl Painter {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Painter {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("layout regions"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) colour: vec3<f32>,
};

@vertex
fn vs(@location(0) xy: vec2<f32>, @location(1) colour: vec3<f32>) -> VsOut {
    var out: VsOut;
    out.position = vec4<f32>(xy, 0.0, 1.0);
    out.colour = colour;
    return out;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.colour, 1.0);
}
"#
                .into(),
            ),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("layout regions"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });

        let attributes = [
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 8,
                shader_location: 1,
            },
        ];

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("layout regions"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: STRIDE,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &attributes,
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("layout regions"),
            size: STRIDE * 6 * MAX_RECTS as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Painter {
            pipeline,
            vertices,
            bytes: Vec::with_capacity((STRIDE * 6 * MAX_RECTS as u64) as usize),
        }
    }

    fn draw(
        &mut self,
        gpu: &Gpu,
        view: &wgpu::TextureView,
        rects: &[(Rect, [f32; 3])],
        viewport: (f32, f32),
    ) {
        self.bytes.clear();
        for (r, colour) in rects.iter().take(MAX_RECTS) {
            let x0 = 2.0 * r.x / viewport.0 - 1.0;
            let x1 = 2.0 * (r.x + r.w) / viewport.0 - 1.0;
            let y0 = 1.0 - 2.0 * r.y / viewport.1;
            let y1 = 1.0 - 2.0 * (r.y + r.h) / viewport.1;
            for (x, y) in [(x0, y0), (x1, y0), (x1, y1), (x0, y0), (x1, y1), (x0, y1)] {
                for v in [x, y, colour[0], colour[1], colour[2]] {
                    self.bytes.extend_from_slice(&v.to_ne_bytes());
                }
            }
        }
        gpu.queue.write_buffer(&self.vertices, 0, &self.bytes);

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("layout"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("layout"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // What shows through every gap: the dividers are this.
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.008,
                            g: 0.008,
                            b: 0.010,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let count = (self.bytes.len() as u64 / STRIDE) as u32;
            if count > 0 {
                pass.set_pipeline(&self.pipeline);
                pass.set_vertex_buffer(0, self.vertices.slice(..));
                pass.draw(0..count, 0..1);
            }
        }
        gpu.queue.submit([encoder.finish()]);
    }
}

/// What to do about a frame that could not be acquired.
///
/// **Every outcome gets a decision, because ignoring one is invisible.** This
/// loop waits for events rather than spinning, so a `return` that neither
/// reconfigures nor asks for another frame is a window that stops drawing and
/// never starts again — and there is nothing on screen or on stdout to say
/// why. `karakuri-cli`'s window sink carries the same table with the same
/// argument, and says that returning silently is what left it with a frozen
/// window once already.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Missed {
    /// The swapchain needs remaking: reconfigure, then ask for another frame.
    Remake,
    /// Ordinary jitter. Ask again.
    Again,
    /// Nobody can see the window. Doing nothing is right — it is the OS that
    /// says when it is back, and asking for frames meanwhile is a spin.
    Idle,
    /// Not self-correcting, and not something a retry mends. Say so.
    Fault,
}

/// `None` where a texture was handed over; a decision for every other case.
fn missed(outcome: &wgpu::CurrentSurfaceTexture) -> Option<Missed> {
    match outcome {
        // `Suboptimal` draws correctly and asks to be reconfigured for
        // performance, which the next resize does anyway.
        wgpu::CurrentSurfaceTexture::Success(_) | wgpu::CurrentSurfaceTexture::Suboptimal(_) => {
            None
        }
        wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
            Some(Missed::Remake)
        }
        wgpu::CurrentSurfaceTexture::Timeout => Some(Missed::Again),
        wgpu::CurrentSurfaceTexture::Occluded => Some(Missed::Idle),
        wgpu::CurrentSurfaceTexture::Validation => Some(Missed::Fault),
    }
}

struct Gfx {
    window: Arc<Window>,
    gpu: Gpu,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    painter: Painter,
}

struct App {
    gfx: Option<Gfx>,
    /// A validation fault is said once rather than sixty times a second.
    faulted: bool,
    readout: Readout,
    /// Reused by every frame.
    rects: Vec<(Rect, [f32; 3])>,
    /// Logical size, so the numbers printed are the arrangement's own units
    /// rather than the display's.
    scale: f64,
}

impl App {
    fn new() -> App {
        App {
            gfx: None,
            faulted: false,
            readout: Readout::new(WINDOW.0 as f32, WINDOW.1 as f32),
            rects: Vec::with_capacity(MAX_RECTS),
            scale: 1.0,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Karakuri console layout")
            .with_inner_size(winit::dpi::LogicalSize::new(WINDOW.0, WINDOW.1));
        let window = Arc::new(event_loop.create_window(attrs).expect("window"));
        let size = window.inner_size();
        self.scale = window.scale_factor();

        let instance = Gpu::instance();
        let surface = instance.create_surface(window.clone()).expect("surface");
        let gpu = pollster::block_on(Gpu::from_instance(instance, Some(&surface))).expect("gpu");

        let caps = surface.get_capabilities(&gpu.adapter);
        // An sRGB format, so the hardware encodes on write and the colours
        // above stay linear all the way here. P-0064.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&gpu.device, &config);
        let painter = Painter::new(&gpu.device, format);

        self.readout.panel.set_viewport(
            size.width as f32 / self.scale as f32,
            size.height as f32 / self.scale as f32,
        );
        self.readout.print_legend();

        window.request_redraw();
        self.gfx = Some(Gfx {
            window,
            gpu,
            surface,
            config,
            painter,
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(gfx) = self.gfx.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => self.scale = scale_factor,
            WindowEvent::Resized(size) => {
                gfx.config.width = size.width.max(1);
                gfx.config.height = size.height.max(1);
                gfx.surface.configure(&gfx.gpu.device, &gfx.config);
                let (w, h) = (
                    size.width as f32 / self.scale as f32,
                    size.height as f32 / self.scale as f32,
                );
                self.readout.panel.set_viewport(w, h);
                println!("viewport: {w:.0} x {h:.0}");
                gfx.window.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let p = Point::new(
                    (position.x / self.scale) as f32,
                    (position.y / self.scale) as f32,
                );
                self.readout.moved(p);
                gfx.window.request_redraw();
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                match state {
                    ElementState::Pressed => {
                        let cursor = self.readout.panel.cursor();
                        self.readout.press(cursor);
                    }
                    ElementState::Released => self.readout.released(),
                }
                gfx.window.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                let op = match event.logical_key.as_ref() {
                    Key::Named(NamedKey::Escape) => {
                        event_loop.exit();
                        return;
                    }
                    Key::Character("f") => Op::Fold,
                    Key::Character("g") => Op::FoldEnclosing,
                    Key::Character("z") => Op::UnfoldAll,
                    Key::Character("s") => Op::Solo,
                    Key::Character("u") => Op::Unsolo,
                    Key::Character("r") => Op::Reset,
                    Key::Character("p") => Op::Report,
                    _ => return,
                };
                self.readout.op(op);
                gfx.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                self.readout.painted(&mut self.rects);
                let acquired = gfx.surface.get_current_texture();
                if let Some(missed) = missed(&acquired) {
                    match missed {
                        Missed::Remake => {
                            gfx.surface.configure(&gfx.gpu.device, &gfx.config);
                            gfx.window.request_redraw();
                        }
                        Missed::Again => gfx.window.request_redraw(),
                        Missed::Idle => {}
                        Missed::Fault => {
                            if !self.faulted {
                                self.faulted = true;
                                println!(
                                    "the surface raised a validation error acquiring a frame — \
                                     the window has stopped drawing"
                                );
                            }
                        }
                    }
                    return;
                }
                self.faulted = false;
                let frame = match acquired {
                    wgpu::CurrentSurfaceTexture::Success(frame)
                    | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
                    // `missed` returned `None`, so there is a texture here.
                    _ => return,
                };
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let viewport = self.readout.panel.layout().viewport();
                gfx.painter
                    .draw(&gfx.gpu, &view, &self.rects, (viewport.w, viewport.h));
                gfx.gpu.queue.present(frame);
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("event loop");
    // Nothing animates: a frame is drawn when something happened.
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut App::new()).expect("run");
}

// ---------------------------------------------------------------------------
// The two tests that cannot leave this file
// ---------------------------------------------------------------------------

/// Everything that drove the panel's model moved with it and is
/// `tests/panel.rs`, which `cargo test -p karakuri-console` runs. What is left
/// here is what is about a `wgpu` type: the table below, and the painter on a
/// device under `mod gpu`.
#[cfg(test)]
mod tests {
    use super::*;

    /// Nothing that comes back from `get_current_texture` is dropped without a
    /// decision. The loop waits for events, so an outcome that neither
    /// reconfigures nor asks for another frame is a window that never draws
    /// again and says nothing about it.
    #[test]
    fn every_frame_that_could_not_be_acquired_is_acted_on() {
        use wgpu::CurrentSurfaceTexture as Acquired;
        // The two that mean the swapchain is stale: reconfigure, and ask again.
        assert_eq!(missed(&Acquired::Outdated), Some(Missed::Remake));
        assert_eq!(missed(&Acquired::Lost), Some(Missed::Remake));
        // Jitter: ask again, without reconfiguring.
        assert_eq!(missed(&Acquired::Timeout), Some(Missed::Again));
        // A window nobody can see: asking again is a spin, and the OS says
        // when it is back.
        assert_eq!(missed(&Acquired::Occluded), Some(Missed::Idle));
        // Not self-correcting, so it is said rather than retried.
        assert_eq!(missed(&Acquired::Validation), Some(Missed::Fault));
    }
}

#[cfg(test)]
mod gpu {
    //! The painter, on a real device, with the frames a hostile drag produces.

    use super::*;

    fn target(gpu: &Gpu) -> (wgpu::Texture, wgpu::TextureView) {
        let t = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("layout probe"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let v = t.create_view(&wgpu::TextureViewDescriptor::default());
        (t, v)
    }

    /// Nothing to paint, a viewport with no extent, a rectangle of no size and
    /// one far outside the window: every one of these is reachable by dragging
    /// a divider against a stop or resizing the window to nothing, and a wgpu
    /// validation error is a panic in this process rather than a return value.
    /// So the assertion is that the frame completes — `poll` waits for the
    /// submission, so a device-side complaint has somewhere to surface.
    #[test]
    fn the_painter_survives_degenerate_frames() {
        let gpu = Gpu::headless().expect("no GPU");
        let (_target, view) = target(&gpu);
        let mut painter = Painter::new(&gpu.device, wgpu::TextureFormat::Rgba8UnormSrgb);

        painter.draw(&gpu, &view, &[], (1440.0, 900.0));
        painter.draw(
            &gpu,
            &view,
            &[(Rect::new(0.0, 0.0, 10.0, 10.0), [1.0, 0.0, 0.0])],
            (0.0, 0.0),
        );
        painter.draw(
            &gpu,
            &view,
            &[
                (Rect::new(0.0, 0.0, 0.0, 0.0), [1.0, 0.0, 0.0]),
                (Rect::new(-500.0, -500.0, 10.0, 10.0), [0.0, 1.0, 0.0]),
                (Rect::new(0.0, 0.0, 100_000.0, 100_000.0), [0.0, 0.0, 1.0]),
            ],
            (1440.0, 900.0),
        );
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the device stopped");
    }
}
