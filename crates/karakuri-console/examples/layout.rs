//! The console's arrangement, on screen and under a mouse.
//!
//! `karakuri-layout` solves an arrangement and `karakuri-console` states one,
//! and until this example nothing had ever *driven* either: every claim about
//! the panel was a test's. This is the other half — a window that paints each
//! visible region as a flat rectangle, with the gaps between them left
//! unpainted so the dividers are exactly what is not drawn, and real pointer
//! and key input on top of it.
//!
//! It draws no text, on purpose: whether a divider drags is answered by
//! watching a rectangle move, and a text toolkit here would pre-empt the
//! decision this harness exists to inform. **stdout is the readout instead** —
//! a legend of every region and the colour it was given at startup, and one
//! line per operation as it happens. The line that matters most is a drag's:
//! it prints what was asked for beside what [`Layout::set_divider`] returned,
//! and the gap between "asked for 400" and "landed at 378" is a constraint
//! biting, which is the thing a person is here to see.
//!
//! ```sh
//! cargo run -p karakuri-console --example layout
//! cargo test -p karakuri-console --example layout   # the drag, without a window
//! cargo test -p karakuri-console --example layout -- --skip gpu::
//! ```
//!
//! **The crate's own suite does not run these**: `cargo test -p
//! karakuri-console` builds an example but does not run the tests inside one,
//! so the command above is how they are run until someone gives the target a
//! `test = true` of its own. One of them takes a device and lives under `mod
//! gpu` for the reason every other one in the workspace does.
//!
//! # Two rules this obeys, because they are what a real view will have to
//!
//! **Solve once per frame, after the input.** [`Layout::rect`] and
//! [`Layout::hit`] carry a `debug_assert!` that the layout is not dirty, so an
//! operation followed by a read is a panic in a debug build. Every frame here
//! solves once and then reads; an input event that has to hit-test solves
//! first, which is a flag test on a frame where nothing moved, and never once
//! per read.
//!
//! **Nothing here shadows the model.** Where the crate could not answer a
//! question, this file derives the answer from the tree it can walk rather
//! than keeping a field of its own — a folded region is `is_collapsed`, a
//! solo is `is_soloed`, and what a drag did is what `set_divider` returned.
//! The places that took deriving are listed in the report this example was
//! written for; they are the crate's missing accessors, not this harness's
//! bookkeeping. The one thing held across events is the drag itself — which
//! boundary is in hand and where on it the pointer took hold — because that
//! is the pointer's state and not the layout's.
//!
//! # The readout is driven by the layout, not by the pointer
//!
//! A line is printed when the boundary **moves**, and a stop is announced
//! once. It is tempting to print a line per pointer event — it is one
//! comparison less — and it is wrong twice over: the interesting lines are
//! buried under identical ones, and a hand held against the edge of the
//! window, which is where a drag ends up, produces hundreds of them a second
//! into a terminal that has to keep up with them while the window waits.
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
//! Every region is reached by walking from [`Layout::root`] with
//! [`Layout::children`] and asking [`Layout::name`] what it is called. So the
//! harness is whatever the arrangement currently says it is, and renaming a
//! region does not touch this file.

use std::sync::Arc;

use karakuri_engine::Gpu;
use karakuri_layout::{Axis, Hit, Layout, NodeId, Point, Rect};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// How far either side of a boundary still grabs it. Wider than any divider
/// the console draws, which is [`Layout::hit`]'s whole argument for taking a
/// grab at all: a 9px gap is not a target a hand finds.
const GRAB: f32 = 6.0;

/// The window this opens, in logical pixels. Comfortably above the smallest
/// viewport the arrangement is claimed to work at, so nothing starts clamped.
const WINDOW: (f64, f64) = (1440.0, 900.0);

/// Enough for any arrangement this crate builds; the buffer is allocated once
/// and never grows, so a frame allocates nothing.
const MAX_RECTS: usize = 64;

// ---------------------------------------------------------------------------
// The harness: the model, the input, and the readout. No window, no device.
// ---------------------------------------------------------------------------

/// One node of the arrangement as this file needs it: what the crate could not
/// be asked for.
struct Entry {
    id: NodeId,
    /// **Derived, because [`Layout`] has no `parent`.** Folding the split that
    /// encloses the region under the pointer needs it, and a hit only ever
    /// resolves to a leaf.
    parent: Option<NodeId>,
    depth: usize,
    /// A leaf paints; a split is the thing whose gaps are visible between its
    /// children.
    colour: Option<[f32; 3]>,
}

/// A boundary in hand: which one, and where along it the pointer took hold.
struct Drag {
    split: NodeId,
    index: usize,
    axis: Axis,
    /// Pointer coordinate minus the boundary's, at the moment of the press.
    /// Subtracted from every later coordinate so the boundary does not jump to
    /// the pointer on the first move.
    offset: f32,
    /// Where the boundary was when this drag last said anything, and whether
    /// what it said was that a stop was holding it.
    ///
    /// **The readout is driven by what the layout did, not by what the pointer
    /// did.** A pointer dragged on past a stop asks for a new position sixty
    /// times a second and the boundary does not move for any of them; a line
    /// per ask is a flood that says the same thing every time, and it is worst
    /// exactly where a person is looking hardest. So a stop is announced once,
    /// and the next line is the one where the boundary moves again.
    said: Option<f32>,
    held: bool,
}

/// What a key does. Named as operations rather than as keys so the tests can
/// ask for one without a keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Fold,
    FoldEnclosing,
    UnfoldAll,
    Solo,
    Unsolo,
    Reset,
    Report,
}

struct Harness {
    layout: Layout,
    /// The tree, flattened, rebuilt whenever the layout is.
    entries: Vec<Entry>,
    cursor: Point,
    drag: Option<Drag>,
}

impl Harness {
    fn new(width: f32, height: f32) -> Harness {
        let mut harness = Harness {
            layout: karakuri_console::layout(),
            entries: Vec::new(),
            cursor: Point::new(-1.0, -1.0),
            drag: None,
        };
        harness.rebuild();
        harness.set_viewport(width, height);
        harness
    }

    /// Walk the arrangement and give every leaf a colour. Order is the tree's,
    /// so neighbours differ and the legend reads top to bottom.
    fn rebuild(&mut self) {
        let mut entries = Vec::new();
        walk(&self.layout, self.layout.root(), None, 0, &mut entries);
        let leaves = entries.iter().filter(|e| e.colour.is_some()).count().max(1);
        let mut nth = 0;
        for entry in &mut entries {
            if entry.colour.is_some() {
                entry.colour = Some(hue(nth as f32 / leaves as f32));
                nth += 1;
            }
        }
        self.entries = entries;
    }

    fn set_viewport(&mut self, width: f32, height: f32) {
        self.layout.set_viewport(Rect::new(0.0, 0.0, width, height));
    }

    /// The one solve. Everything that reads calls this first; on a frame where
    /// nothing changed it is a flag test.
    fn solve(&mut self) {
        self.layout.solve();
    }

    fn label(&self, id: NodeId) -> String {
        match self.layout.name(id) {
            Some(name) => name.to_owned(),
            None => "(unnamed split)".to_owned(),
        }
    }

    /// The two regions a boundary is between. A split is often unnamed — the
    /// console's body row is, deliberately — so `split #0` alone does not say
    /// which boundary the pointer has hold of, and the pair does.
    fn pair(&self, split: NodeId, index: usize) -> String {
        let children = self.visible_children(split);
        match (children.get(index), children.get(index + 1)) {
            (Some(&a), Some(&b)) => format!("{} | {}", self.label(a), self.label(b)),
            _ => "no pair".to_owned(),
        }
    }

    fn parent_of(&self, id: NodeId) -> Option<NodeId> {
        self.entries.iter().find(|e| e.id == id)?.parent
    }

    /// The children a divider index counts, which is the visible ones.
    fn visible_children(&self, split: NodeId) -> Vec<NodeId> {
        self.layout
            .children(split)
            .iter()
            .copied()
            .filter(|c| !self.layout.is_collapsed(*c))
            .collect()
    }

    /// Where boundary `index` of `split` currently is, along the split's axis:
    /// the far edge of the visible child before it.
    ///
    /// Reads solved rectangles, so the caller has solved.
    fn boundary(&self, split: NodeId, index: usize) -> Option<f32> {
        let axis = self.layout.axis(split)?;
        let before = *self.visible_children(split).get(index)?;
        Some(far(axis, self.layout.rect(before)))
    }

    /// Every boundary in the arrangement. The window has a pointer to find
    /// them with; the tests do not, so this is theirs.
    #[cfg(test)]
    fn dividers(&mut self) -> Vec<(NodeId, usize)> {
        self.solve();
        let splits: Vec<NodeId> = self
            .entries
            .iter()
            .filter(|e| e.colour.is_none())
            .map(|e| e.id)
            .collect();
        let mut out = Vec::new();
        for split in splits {
            let visible = self.visible_children(split).len();
            for index in 0..visible.saturating_sub(1) {
                out.push((split, index));
            }
        }
        out
    }

    /// A point in the middle of a boundary's gap — what a hand aims at, and
    /// what a test presses instead of having one.
    #[cfg(test)]
    fn grab_point(&self, split: NodeId, index: usize) -> Option<Point> {
        let axis = self.layout.axis(split)?;
        let children = self.visible_children(split);
        let (a, b) = (*children.get(index)?, *children.get(index + 1)?);
        let (ra, rb) = (self.layout.rect(a), self.layout.rect(b));
        let along = (far(axis, ra) + near(axis, rb)) / 2.0;
        let across = match axis {
            Axis::Row => ra.y + ra.h / 2.0,
            Axis::Column => ra.x + ra.w / 2.0,
        };
        Some(match axis {
            Axis::Row => Point::new(along, across),
            Axis::Column => Point::new(across, along),
        })
    }

    // -- input ----------------------------------------------------------

    fn press(&mut self, p: Point) {
        self.solve();
        self.cursor = p;
        match self.layout.hit(p, GRAB) {
            Hit::Divider { split, index } => {
                let axis = self.layout.axis(split).expect("a divider is on a split");
                let Some(boundary) = self.boundary(split, index) else {
                    println!("press ({:.0}, {:.0}): a divider with no pair", p.x, p.y);
                    return;
                };
                let offset = along(axis, p) - boundary;
                println!(
                    "press ({:.0}, {:.0}): the boundary {} — divider #{} of {}, {:?} — is at \
                     {:.1}, grabbed {:+.1} from it",
                    p.x,
                    p.y,
                    self.pair(split, index),
                    index,
                    self.label(split),
                    axis,
                    boundary,
                    offset
                );
                self.drag = Some(Drag {
                    split,
                    index,
                    axis,
                    offset,
                    said: None,
                    held: false,
                });
            }
            Hit::View(id) => {
                let r = self.layout.rect(id);
                println!(
                    "press ({:.0}, {:.0}): region {} at {:.0},{:.0} {:.0}x{:.0}",
                    p.x,
                    p.y,
                    self.label(id),
                    r.x,
                    r.y,
                    r.w,
                    r.h
                );
            }
            Hit::Nothing => println!("press ({:.0}, {:.0}): nothing", p.x, p.y),
        }
    }

    /// A move with a boundary in hand. **Absolute**: the pointer's coordinate
    /// along the split's axis, less the offset it grabbed at, straight into
    /// `set_divider`. Nothing accumulates, which is what a drag past a stop and
    /// back is here to demonstrate.
    ///
    /// Returns the line the readout should carry, if this move changed
    /// anything worth a line. The caller prints it — which is what lets a test
    /// count the lines a drag produces without capturing stdout.
    fn moved(&mut self, p: Point) -> Option<String> {
        self.cursor = p;
        let drag = self.drag.as_ref()?;
        let (split, index, axis, offset) = (drag.split, drag.index, drag.axis, drag.offset);
        let asked = along(axis, p) - offset;
        let landed = self.layout.set_divider(split, index, asked);
        // `set_divider` solves before it returns, so the reads below are of a
        // clean layout.
        let by = landed - asked;
        let held = by.abs() >= 0.05;

        let drag = self.drag.as_mut()?;
        let say = match drag.said {
            None => true,
            Some(said) => (landed - said).abs() >= 0.5 || drag.held != held,
        };
        if !say {
            return None;
        }
        drag.said = Some(landed);
        drag.held = held;

        let children = self.visible_children(split);
        let sizes = match (children.get(index), children.get(index + 1)) {
            (Some(&a), Some(&b)) => format!(
                "{} {:.0} | {} {:.0}",
                self.label(a),
                extent(axis, self.layout.rect(a)),
                self.label(b),
                extent(axis, self.layout.rect(b))
            ),
            _ => "no pair".to_owned(),
        };
        let stop = match held {
            true => format!(" — held {by:+.1} by a stop, and it stays there until it moves"),
            false => String::new(),
        };
        Some(format!(
            "  drag: asked {asked:.1}, landed {landed:.1}{stop} [{sizes}]"
        ))
    }

    fn released(&mut self) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        self.solve();
        match self.boundary(drag.split, drag.index) {
            Some(at) => println!(
                "release: {} rests at {at:.1}",
                self.pair(drag.split, drag.index)
            ),
            None => println!("release: {} is gone", self.pair(drag.split, drag.index)),
        }
    }

    fn op(&mut self, op: Op) {
        self.solve();
        match op {
            Op::Fold => match self.layout.hit(self.cursor, 0.0) {
                Hit::View(id) => {
                    let folded = self.layout.toggle(id);
                    println!("fold: {} is now {}", self.label(id), folding(folded));
                }
                Hit::Divider { split, index } => println!(
                    "fold: the pointer is on divider {}#{} — move it into a region",
                    self.label(split),
                    index
                ),
                Hit::Nothing => println!("fold: nothing under the pointer"),
            },
            Op::FoldEnclosing => {
                let target = match self.layout.hit(self.cursor, 0.0) {
                    // A divider already names its split; a region's enclosing
                    // split is its parent, which `Layout` does not answer for
                    // — see `Entry::parent`.
                    Hit::Divider { split, .. } => Some(split),
                    Hit::View(id) => self.parent_of(id),
                    Hit::Nothing => None,
                };
                match target {
                    Some(split) => {
                        let root = split == self.layout.root();
                        let folded = self.layout.toggle(split);
                        println!(
                            "fold: the split {} is now {}{}",
                            self.label(split),
                            folding(folded),
                            match root && folded {
                                true =>
                                    " — that was the root, so the panel is empty; z brings it back",
                                false => "",
                            }
                        );
                    }
                    None => println!("fold: nothing encloses the pointer"),
                }
            }
            Op::UnfoldAll => {
                let folded: Vec<NodeId> = self
                    .entries
                    .iter()
                    .map(|e| e.id)
                    .filter(|id| self.layout.is_collapsed(*id))
                    .collect();
                match folded.is_empty() {
                    true => println!("unfold: nothing is folded"),
                    false => {
                        let names: Vec<String> = folded.iter().map(|id| self.label(*id)).collect();
                        for id in folded {
                            self.layout.expand(id);
                        }
                        println!("unfold: {}", names.join(", "));
                    }
                }
            }
            Op::Solo => match self.layout.hit(self.cursor, 0.0) {
                Hit::View(id) => {
                    self.layout.solo(id);
                    println!(
                        "solo: {} — everything else folded (soloed = {})",
                        self.label(id),
                        self.layout.is_soloed()
                    );
                }
                _ => println!("solo: no region under the pointer"),
            },
            Op::Unsolo => {
                let was = self.layout.is_soloed();
                self.layout.unsolo();
                println!(
                    "unsolo: {}",
                    match was {
                        true => "the arrangement before the solo is back",
                        false => "nothing was soloed",
                    }
                );
            }
            Op::Reset => {
                let viewport = self.layout.viewport();
                self.layout = karakuri_console::layout();
                self.layout.set_viewport(viewport);
                self.rebuild();
                self.drag = None;
                println!("reset: a fresh arrangement, at the same viewport");
            }
            Op::Report => {
                // No solve of its own: the one at the top of this method is
                // the frame's, and a second here would hide an operation that
                // left one owed rather than catch it. See the test.
                println!("regions:");
                let rows: Vec<String> = self
                    .entries
                    .iter()
                    .map(|e| {
                        let r = self.layout.rect(e.id);
                        let state =
                            match (self.layout.is_collapsed(e.id), self.layout.visible(e.id)) {
                                (true, _) => "folded",
                                (false, false) => "inside a fold",
                                (false, true) => "",
                            };
                        format!(
                            "  {:width$}{:<18} {:>7.1},{:>7.1}  {:>7.1} x {:>7.1} {}",
                            "",
                            self.label(e.id),
                            r.x,
                            r.y,
                            r.w,
                            r.h,
                            state,
                            width = e.depth * 2
                        )
                    })
                    .collect();
                println!("{}", rows.join("\n"));
            }
        }
        self.solve();
    }

    // -- the readout ----------------------------------------------------

    fn print_legend(&mut self) {
        self.solve();
        let viewport = self.layout.viewport();
        println!();
        println!(
            "the console's arrangement, in a {:.0} x {:.0} viewport. every leaf is painted in \
             its own colour and the dividers are the gaps between them.",
            viewport.w, viewport.h
        );
        println!();
        for entry in &self.entries {
            let (min, max) = self.layout.bounds(entry.id);
            let bounds = format!(
                "min {min:.0}, max {}",
                match max.is_finite() {
                    true => format!("{max:.0}"),
                    false => "none".to_owned(),
                }
            );
            let colour = match entry.colour {
                Some(c) => {
                    let [r, g, b] = encode(c);
                    format!("rgb({r:>3}, {g:>3}, {b:>3})")
                }
                None => match self.layout.axis(entry.id) {
                    Some(Axis::Row) => "split, left to right".to_owned(),
                    Some(Axis::Column) => "split, top to bottom".to_owned(),
                    None => "empty".to_owned(),
                },
            };
            println!(
                "  {:width$}{:<16} {:<22} {}",
                "",
                self.label(entry.id),
                colour,
                bounds,
                width = entry.depth * 2
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
        self.solve();
        out.clear();
        for entry in &self.entries {
            let Some(colour) = entry.colour else {
                continue;
            };
            if !self.layout.visible(entry.id) {
                continue;
            }
            let r = self.layout.rect(entry.id);
            if r.w > 0.0 && r.h > 0.0 {
                out.push((r, colour));
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

fn walk(layout: &Layout, id: NodeId, parent: Option<NodeId>, depth: usize, out: &mut Vec<Entry>) {
    let leaf = layout.axis(id).is_none();
    out.push(Entry {
        id,
        parent,
        depth,
        // A placeholder the second pass replaces; a split keeps `None`.
        colour: leaf.then_some([0.0; 3]),
    });
    for child in layout.children(id) {
        walk(layout, *child, Some(id), depth + 1, out);
    }
}

/// `Axis`'s own `coord`, `origin` and `extent` are `pub(crate)`, so a caller
/// outside the crate writes them again. These three are that.
fn along(axis: Axis, p: Point) -> f32 {
    match axis {
        Axis::Row => p.x,
        Axis::Column => p.y,
    }
}

#[cfg(test)]
fn near(axis: Axis, r: Rect) -> f32 {
    match axis {
        Axis::Row => r.x,
        Axis::Column => r.y,
    }
}

fn far(axis: Axis, r: Rect) -> f32 {
    match axis {
        Axis::Row => r.x + r.w,
        Axis::Column => r.y + r.h,
    }
}

fn extent(axis: Axis, r: Rect) -> f32 {
    match axis {
        Axis::Row => r.w,
        Axis::Column => r.h,
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
    harness: Harness,
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
            harness: Harness::new(WINDOW.0 as f32, WINDOW.1 as f32),
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

        self.harness.set_viewport(
            size.width as f32 / self.scale as f32,
            size.height as f32 / self.scale as f32,
        );
        self.harness.print_legend();

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
                self.harness.set_viewport(w, h);
                println!("viewport: {w:.0} x {h:.0}");
                gfx.window.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let p = Point::new(
                    (position.x / self.scale) as f32,
                    (position.y / self.scale) as f32,
                );
                if let Some(line) = self.harness.moved(p) {
                    println!("{line}");
                }
                gfx.window.request_redraw();
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                match state {
                    ElementState::Pressed => {
                        let cursor = self.harness.cursor;
                        self.harness.press(cursor);
                    }
                    ElementState::Released => self.harness.released(),
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
                self.harness.op(op);
                gfx.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                self.harness.painted(&mut self.rects);
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
                let viewport = self.harness.layout.viewport();
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
// The drag, without a window
// ---------------------------------------------------------------------------

/// The input handling above is a plain `Harness` method, so a drag is a test:
/// press at a point, move to another, release, and read the rectangles. No
/// event loop and no device.
#[cfg(test)]
mod tests {
    use super::*;

    /// Pixel coordinates in the hundreds; the same tolerance the crate's own
    /// tests use.
    const EPS: f32 = 1e-3;

    fn rects(h: &mut Harness) -> Vec<Rect> {
        h.solve();
        h.entries.iter().map(|e| h.layout.rect(e.id)).collect()
    }

    fn offset(axis: Axis, p: Point, by: f32) -> Point {
        match axis {
            Axis::Row => Point::new(p.x + by, p.y),
            Axis::Column => Point::new(p.x, p.y + by),
        }
    }

    fn same(a: &[Rect], b: &[Rect]) -> bool {
        a.len() == b.len()
            && a.iter().zip(b).all(|(x, y)| {
                (x.x - y.x).abs() <= EPS
                    && (x.y - y.y).abs() <= EPS
                    && (x.w - y.w).abs() <= EPS
                    && (x.h - y.h).abs() <= EPS
            })
    }

    /// Every divider in the arrangement, dragged and dragged back — including
    /// far past whatever stops it — leaves the arrangement exactly as it was,
    /// and at least one of them moves on the way.
    ///
    /// The second half is the point: `set_divider` takes an absolute
    /// coordinate, so the frames spent past a stop contribute nothing to
    /// accumulate. A harness that fed it deltas would come back short.
    #[test]
    fn a_drag_out_and_back_leaves_the_arrangement_where_it_was() {
        let mut probe = Harness::new(1600.0, 1000.0);
        let dividers = probe.dividers();
        assert!(!dividers.is_empty(), "the arrangement has no dividers");
        let total = dividers.len();

        let mut moved = 0;
        let mut grabbed = 0;
        for (split, index) in dividers {
            let mut h = Harness::new(1600.0, 1000.0);
            h.solve();
            let before = rects(&mut h);
            let axis = h.layout.axis(split).expect("a divider is on a split");
            let start = h.boundary(split, index).expect("a boundary");
            let point = h.grab_point(split, index).expect("a gap to aim at");

            h.press(point);
            let Some(drag) = &h.drag else {
                // Reported rather than asserted: a divider a pointer cannot
                // reach is a finding about `hit`, not about the drag.
                continue;
            };
            assert_eq!((drag.split, drag.index), (split, index));
            grabbed += 1;

            let _ = h.moved(offset(axis, point, 40.0));
            if (h.boundary(split, index).expect("a boundary") - start).abs() > EPS {
                moved += 1;
                assert!(
                    !same(&before, &rects(&mut h)),
                    "the boundary moved and no rectangle changed"
                );
            }

            // Past every stop there is, in both directions, and back to the
            // exact pointer position the drag began at.
            let _ = h.moved(offset(axis, point, 9000.0));
            let _ = h.moved(offset(axis, point, -9000.0));
            let _ = h.moved(point);
            h.released();

            let after = rects(&mut h);
            assert!(
                same(&before, &after),
                "divider {}#{index} did not come back: {:?} against {:?}",
                h.label(split),
                before,
                after
            );
        }
        assert_eq!(
            grabbed, total,
            "a boundary the arrangement has that a pointer cannot grab"
        );
        // Two of the console's boundaries cannot move at all — a region whose
        // minimum meets its maximum is pinned — so this is "at least one", not
        // "all of them", and the restore above is what holds for every one.
        assert!(moved > 0, "no divider moved under a 40px drag");
    }

    /// A drag that runs off the left edge of the window. winit reports
    /// negative pointer coordinates there, and the operator dragged into them.
    #[test]
    fn a_drag_past_the_left_edge_of_the_window() {
        let mut probe = Harness::new(1920.0, 1080.0);
        let dividers = probe.dividers();
        for (split, index) in dividers {
            let mut h = Harness::new(1920.0, 1080.0);
            h.solve();
            let axis = h.layout.axis(split).expect("a divider is on a split");
            let point = h.grab_point(split, index).expect("a gap to aim at");
            h.press(point);
            for to in [-40.0, -200.0, -1000.0, -5000.0] {
                let p = match axis {
                    Axis::Row => Point::new(to, point.y),
                    Axis::Column => Point::new(point.x, to),
                };
                let _ = h.moved(p);
            }
            h.released();
        }
    }

    /// A pointer dragged on past a stop says so once.
    ///
    /// This is the readout's own bug, and it is worst exactly where a person
    /// is looking hardest: at a stop the boundary does not move, and a line
    /// per pointer event is hundreds of identical lines a second into a
    /// terminal that has to keep up with them.
    #[test]
    fn a_drag_held_at_a_stop_says_so_once() {
        let mut probe = Harness::new(1600.0, 1000.0);
        let mut stops = 0;
        for (split, index) in probe.dividers() {
            let mut h = Harness::new(1600.0, 1000.0);
            h.solve();
            let axis = h.layout.axis(split).expect("a divider is on a split");
            let point = h.grab_point(split, index).expect("a gap to aim at");
            h.press(point);

            // Well past whatever stops it, and then on, a pixel at a time,
            // the way a hand held against the edge of the window does.
            let _ = h.moved(offset(axis, point, -9000.0));
            let at = h.boundary(split, index).expect("a boundary");
            let mut said = 0;
            for step in 1..=200 {
                if h.moved(offset(axis, point, -9000.0 - step as f32))
                    .is_some()
                {
                    said += 1;
                }
            }
            let still = h.boundary(split, index).expect("a boundary");
            assert!(
                (still - at).abs() <= EPS,
                "the boundary was supposed to be against a stop and moved"
            );
            assert_eq!(
                said, 0,
                "a boundary that did not move said something 200 times over"
            );
            stops += 1;
        }
        assert!(stops > 0, "no divider was driven against a stop");
    }

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

    /// A sweep of everything the window can do to the harness, at viewports
    /// from comfortable down to below the arrangement's own minima, with the
    /// pointer walked well outside each of them.
    #[test]
    fn a_sweep_of_hostile_input() {
        for (vw, vh) in [
            (1920.0_f32, 1080.0_f32),
            (1440.0, 900.0),
            (990.0, 632.0),
            (400.0, 300.0),
            (1.0, 1.0),
        ] {
            let mut probe = Harness::new(vw, vh);
            for (split, index) in probe.dividers() {
                let mut h = Harness::new(vw, vh);
                h.solve();
                let Some(point) = h.grab_point(split, index) else {
                    continue;
                };
                h.press(point);
                for to in [
                    Point::new(-1.0, -1.0),
                    Point::new(-4000.0, point.y),
                    Point::new(point.x, -4000.0),
                    Point::new(vw + 4000.0, vh + 4000.0),
                    Point::new(0.0, 0.0),
                    Point::new(f32::MAX, f32::MAX),
                    point,
                ] {
                    let _ = h.moved(to);
                    for op in [
                        Op::Fold,
                        Op::FoldEnclosing,
                        Op::Report,
                        Op::Solo,
                        Op::Report,
                        Op::Unsolo,
                        Op::UnfoldAll,
                    ] {
                        h.op(op);
                    }
                    // A resize in the middle of a drag, which a window can do.
                    h.set_viewport(vw / 2.0, vh / 2.0);
                    let _ = h.moved(to);
                    h.set_viewport(0.0, 0.0);
                    let _ = h.moved(to);
                    h.set_viewport(vw, vh);
                }
                h.released();
                // A release with nothing in hand, and a press outside.
                h.released();
                h.press(Point::new(-10.0, -10.0));
                let _ = h.moved(Point::new(-10.0, -10.0));
                h.op(Op::Reset);
            }
        }
    }

    /// Every operation, in a debug build, with a read after each one.
    ///
    /// `rect()` and `hit()` both `debug_assert!` that the layout is not dirty,
    /// so this fails if any operation here leaves a solve owed — which is the
    /// mistake a real view will make first, and the reason the harness solves
    /// at the top of everything that reads.
    ///
    /// It also asserts what each operation is *for*: a fold folds, an unfold
    /// puts the arrangement back exactly, a solo leaves one region visible,
    /// and an unsolo restores what it replaced.
    #[test]
    fn every_operation_leaves_the_layout_readable_and_undoes_exactly() {
        let mut h = Harness::new(1600.0, 1000.0);
        h.solve();
        let before = rects(&mut h);

        // Name-free: the pointer goes to the middle of the first leaf big
        // enough to aim at.
        let leaf = h
            .entries
            .iter()
            .filter(|e| e.colour.is_some())
            .map(|e| (e.id, h.layout.rect(e.id)))
            .find(|(_, r)| r.w > 20.0 && r.h > 20.0)
            .expect("a leaf to point at");
        h.cursor = Point::new(leaf.1.x + leaf.1.w / 2.0, leaf.1.y + leaf.1.h / 2.0);

        let folded = |h: &Harness| {
            h.entries
                .iter()
                .filter(|e| h.layout.is_collapsed(e.id))
                .count()
        };

        h.op(Op::Fold);
        assert_eq!(folded(&h), 1, "f folded something other than one region");
        h.op(Op::UnfoldAll);
        assert_eq!(folded(&h), 0);
        assert!(same(&before, &rects(&mut h)), "an unfold did not restore");

        h.op(Op::FoldEnclosing);
        assert!(folded(&h) > 0, "g folded nothing");
        h.op(Op::UnfoldAll);
        assert!(same(&before, &rects(&mut h)), "an unfold did not restore");

        h.op(Op::Solo);
        assert!(h.layout.is_soloed());
        assert!(
            h.layout.visible(leaf.0),
            "a solo folded the region it was aimed at"
        );
        // Straight into an operation that reads every rectangle, with nothing
        // solving in between: this is the pair that catches a stale solve, and
        // it fails on `rect() read a stale solve` if `op` stops solving.
        h.op(Op::Report);
        h.op(Op::Unsolo);
        assert!(!h.layout.is_soloed());
        assert!(same(&before, &rects(&mut h)), "an unsolo did not restore");

        // A drag, then a reset: the arrangement is fresh, at the same viewport.
        let (split, index) = h.dividers()[0];
        let point = h.grab_point(split, index).expect("a gap to aim at");
        let axis = h.layout.axis(split).expect("a divider is on a split");
        h.press(point);
        let _ = h.moved(offset(axis, point, 60.0));
        h.released();
        h.op(Op::Reset);
        assert!(same(&before, &rects(&mut h)), "a reset did not restore");
    }

    /// A drag lands where it is asked unless something stops it, and what
    /// `set_divider` returns is what actually happened — the pair either side
    /// keeps its combined extent whatever was asked for.
    #[test]
    fn a_drag_returns_where_it_landed_and_moves_only_the_pair() {
        let mut h = Harness::new(1600.0, 1000.0);
        let dividers = h.dividers();
        let mut checked = 0;
        for (split, index) in dividers {
            let mut h = Harness::new(1600.0, 1000.0);
            h.solve();
            let axis = h.layout.axis(split).expect("a divider is on a split");
            let children = h.visible_children(split);
            let (a, b) = (children[index], children[index + 1]);
            let span = extent(axis, h.layout.rect(a)) + extent(axis, h.layout.rect(b));
            let others: Vec<Rect> = children
                .iter()
                .filter(|c| **c != a && **c != b)
                .map(|c| h.layout.rect(*c))
                .collect();

            let point = h.grab_point(split, index).expect("a gap to aim at");
            h.press(point);
            if h.drag.is_none() {
                continue;
            }
            checked += 1;
            let _ = h.moved(offset(axis, point, 9000.0));

            let landed = h.boundary(split, index).expect("a boundary");
            let after: Vec<Rect> = children
                .iter()
                .filter(|c| **c != a && **c != b)
                .map(|c| h.layout.rect(*c))
                .collect();
            assert!(
                same(&others, &after),
                "a drag moved a sibling that is not either side of it"
            );
            let now = extent(axis, h.layout.rect(a)) + extent(axis, h.layout.rect(b));
            assert!(
                (now - span).abs() <= EPS,
                "the pair's combined extent changed: {span} to {now}"
            );
            let (min, max) = h.layout.bounds(a);
            assert!(
                landed - near(axis, h.layout.rect(a)) <= max + EPS
                    && landed - near(axis, h.layout.rect(a)) >= min - EPS,
                "a drag landed outside the bounds it was stopped by"
            );
        }
        assert!(checked > 0, "no divider could be grabbed at all");
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
