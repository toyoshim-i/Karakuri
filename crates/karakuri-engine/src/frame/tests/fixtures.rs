use super::*;
use crate::set::Set;
use crate::swap::HotSwap;
use karakuri_ir::typed::Checked;
use std::cell::Cell;
use std::rc::Rc;

/// A sink that draws nowhere and can be told to refuse.
///
/// This is the third implementation, and the reason the other two were worth
/// putting behind a trait. `karakuri-cli`'s `Live::frame` had never been
/// reached by a test: it needs a window, and `Outdated` — the case that
/// produced a real defect — cannot be synthesised at all. (It was
/// `SurfaceError::Outdated` until wgpu 30 replaced the `Result` with
/// `CurrentSurfaceTexture`; the point survives the rename.) Two of the three
/// record-ordering bugs found in this codebase lived in that function. A sink
/// that refuses on demand is what makes the case reachable.
pub(crate) struct TestSink {
    pub(crate) target: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    /// So a test can look at what was drawn. Every width used here times four is a
    /// multiple of 256, so the rows need no padding — see `karakuri-cli`'s
    /// `render::unpad_rows` for when they do.
    pub(crate) readback: wgpu::Buffer,
    pub(crate) width: u32,
    pub(crate) height: u32,
    /// What `acquire` answers, popped front to back. Empty means accept.
    pub(crate) answers: Vec<Result<(), Skip>>,
    /// What `present` answers, every time. `None` accepts.
    pub(crate) present_answer: Option<String>,
    pub(crate) acquired: usize,
    /// Every call the contract names, counted separately, because the claim a
    /// refusing sink makes is about what was *not* called on it and a single "was
    /// it drawn" flag cannot tell "never asked" from "asked and did nothing".
    /// `view` takes `&self`, hence the `Cell`.
    pub(crate) viewed: Cell<usize>,
    pub(crate) after_drawn: usize,
    pub(crate) presented: usize,
    /// A tick shared with the other sinks and with the caller's `finally`, so the
    /// order of the calls across all of them is read off afterwards rather than
    /// inferred from a per-sink counter. A count says how many times; only a shared
    /// tick says *when*.
    pub(crate) order: Rc<Cell<usize>>,
    /// The tick this sink's `after_draw` took, and the address of the encoder it
    /// was handed. The second is what says `finally` was given this encoder rather
    /// than one `compose` made for it — an encoder of its own would be a second
    /// command buffer, which is exactly the race ADR-0166 is about.
    pub(crate) drawn_at: usize,
    pub(crate) encoder_at: usize,
    /// Whether the last presented frame had any light in it at all.
    pub(crate) lit: Option<bool>,
    /// And the frame itself, so a test can ask *where* the light is. A `Vec` on a
    /// test double, which is the one place in this file where that is nobody's
    /// business.
    pub(crate) pixels: Vec<u8>,
}

impl TestSink {
    pub(crate) fn new(gpu: &Gpu, answers: Vec<Result<(), Skip>>) -> TestSink {
        TestSink::sized(gpu, SIZE, SIZE, answers)
    }

    pub(crate) fn sized(
        gpu: &Gpu,
        width: u32,
        height: u32,
        answers: Vec<Result<(), Skip>>,
    ) -> TestSink {
        assert_eq!(width * 4 % 256, 0, "a padded readback would need unpadding");
        let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("test sink"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("test sink readback"),
            size: u64::from(width * height * 4),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        TestSink {
            target,
            view,
            readback,
            width,
            height,
            answers,
            present_answer: None,
            acquired: 0,
            viewed: Cell::new(0),
            after_drawn: 0,
            presented: 0,
            order: Rc::new(Cell::new(0)),
            drawn_at: 0,
            encoder_at: 0,
            lit: None,
            pixels: Vec::new(),
        }
    }

    /// Take the ticks from the caller's clock rather than from one of this sink's
    /// own, which is what makes two sinks and a `finally` comparable.
    pub(crate) fn ordered_by(&mut self, order: &Rc<Cell<usize>>) {
        self.order = Rc::clone(order);
    }

    /// Whether any texel in a column band of the last presented frame has light in
    /// it. The band is what says `Present::draw` fitted the canvas into *this*
    /// sink's size rather than into somebody else's.
    pub(crate) fn lit_between(&self, from: u32, to: u32) -> bool {
        (0..self.height).any(|y| {
            (from..to).any(|x| {
                let at = ((y * self.width + x) * 4) as usize;
                self.pixels[at..at + 3].iter().any(|&b| b > 0)
            })
        })
    }

    /// Nothing at all was called on this sink but `acquire`, which is the whole of
    /// what a refusal promises.
    pub(crate) fn untouched_after_refusing(&self) -> bool {
        self.viewed.get() == 0 && self.after_drawn == 0 && self.presented == 0
    }
}

impl Sink for TestSink {
    fn acquire(&mut self, _gpu: &Gpu) -> Result<(), Skip> {
        let answer = if self.acquired < self.answers.len() {
            self.answers[self.acquired].clone()
        } else {
            Ok(())
        };
        self.acquired += 1;
        answer
    }
    fn view(&self) -> &wgpu::TextureView {
        self.viewed.set(self.viewed.get() + 1);
        &self.view
    }
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    fn after_draw(&mut self, encoder: &mut wgpu::CommandEncoder) {
        self.after_drawn += 1;
        self.order.set(self.order.get() + 1);
        self.drawn_at = self.order.get();
        self.encoder_at = std::ptr::from_ref(&*encoder) as usize;
        encoder.copy_texture_to_buffer(
            self.target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.width * 4),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
    }

    fn present(&mut self, gpu: &Gpu) -> Result<(), String> {
        self.presented += 1;
        let slice = self.readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        self.pixels = slice.get_mapped_range().expect("map").to_vec();
        self.readback.unmap();
        // **Colour only.** The present pass returns `vec4(rgb, 1.0)`, so
        // every texel's alpha is 255 and a scan over all four channels
        // answers "lit" for a frame that is entirely black.
        self.lit = Some(
            self.pixels
                .chunks(4)
                .any(|texel| texel[..3].iter().any(|&b| b > 0)),
        );
        match &self.present_answer {
            Some(e) => Err(e.clone()),
            None => Ok(()),
        }
    }
}

pub(crate) const SIZE: u32 = 64;
/// A second sink twice as wide as the canvas and the same height, so the canvas
/// letterboxes into a band with black either side of it. `128 * 4` is 512,
/// which is aligned, and the band is exactly `[32, 96)`.
pub(crate) const WIDE: u32 = 128;
pub(crate) const BAND: (u32, u32) = ((WIDE - SIZE) / 2, (WIDE + SIZE) / 2);
pub(crate) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// One sink, no refusals worth reporting: what most of these tests want.
pub(crate) fn one(sink: &mut TestSink) -> [&mut dyn Sink; 1] {
    [sink]
}

pub(crate) fn look() -> Look {
    Look {
        op: TonemapOp::Clamp,
        exposure: 1.0,
        white_point: 1.0,
    }
}

/// The three stages a `.kir` goes through before [`Set::build`] will take it,
/// the way every other test in this crate spells them — except that these tests
/// read the shipped `examples/` pair rather than an inline fixture, because
/// what they want is material that draws something at [`SIZE`] and the
/// workspace already has some.
pub(crate) fn compile(path: &std::path::Path) -> Checked {
    let src = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let render = |errs: &[karakuri_ir::IrError]| {
        errs.iter()
            .map(|e| e.render(&src))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let proc = karakuri_ir::parse(&src).unwrap_or_else(|e| panic!("{}", render(&e)));
    let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{}", render(&e)));
    karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("{}", render(&e)));
    checked
}

/// A one-slot deck built from the example pair — the same pair
/// `karakuri-engine`'s own examples build from, and the one the CLI's window
/// opens on by default.
pub(crate) fn one_slot_deck(gpu: &Gpu) -> Deck {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let l1 = compile(&root.join("examples/drift_shell.kir"));
    let l4 = compile(&root.join("examples/soft_points.kir"));
    let set = Set::build(&gpu.device, &gpu.queue, &l1, &l4, 4096, 7).expect("set");
    Deck::new(&gpu.device, vec![HotSwap::fixed(set)], SIZE, SIZE)
}

/// A chain of one L5, built against the `Present` that will run it.
///
/// The procedure reads the clock and nothing else. What it draws is not
/// read here; `tests/master.rs` holds a chain slot's picture to its
/// uniform.
pub(crate) fn clock_chain(gpu: &Gpu, present: &Present) -> crate::master::Chain {
    const CLOCK: &str = r#"
proc clock_probe {
  kind L5

  frame {
    color = vec4(t, beats, dt, 1.0);
  }
}
"#;
    let proc = karakuri_ir::parse(CLOCK).expect("the probe parses");
    let checked = karakuri_ir::check::check(&proc).expect("the probe checks");
    let slot = crate::master::Slot::build(
        &gpu.device,
        present.chain_layout(),
        "test:clock_probe",
        &checked,
        None,
        Default::default(),
    )
    .expect("an L5 with no `retains` is a legal chain slot");
    crate::master::Chain::new(vec![slot])
}

// All seven drive a real `Present`, so all seven are here.
