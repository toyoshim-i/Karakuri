use super::common::*;

mod gpu {
    use super::*;

    // ---------------------------------------------------------------------------
    // Measured, reported.
    // ---------------------------------------------------------------------------

    /// Measures and prints execution costs of single-slot vs multi-slot decks,
    /// isolating composite pass overhead and off-air slot costs (ADR-0258, ADR-0269).
    #[test]
    #[ignore = "a measurement, not a check; run with --ignored --nocapture"]
    fn the_cost_of_a_slot_and_of_the_composite_are_measured_and_reported() {
        const CAP: u32 = 262_144;
        const W: u32 = 1280;
        const H: u32 = 720;
        const WARMUP: usize = 60;
        const MEASURED: usize = 120;

        // **The sprite is expressed against the height it is rendered at**,
        // which is the whole of what a reader has to hold: a rate is a size
        // only once a height is named. `point_rate` is a fraction of the
        // target's height, so [`L4`]'s 0.015625 is four texels at [`HEIGHT`]
        // and 11.25 here, at 720 — 7.9 times the area, on an additive blend
        // that is paid by area. That is what this test measured from
        // 2026-09-02 to 2026-09-07, and a quarter to two fifths of every
        // figure it printed was the fixture rather than the deck: on a quiet
        // machine `four, one Live` reads 21.3 ms at 11.25 texels against 16.1
        // at four, and with the machine loaded the same pair read 62.7 and
        // 67.3 against 33.6 and 32.7, because contention lands on the
        // four-slot lines. The rate is rescaled here rather than changed in
        // [`L4`], because [`L4`]'s number is the right one for the 256-high
        // target the assertions render into.
        const SPRITE_PX: f32 = 4.0;
        let l4 = L4.replace(
            "point_rate = 0.015625;",
            &format!("point_rate = {};", SPRITE_PX / H as f32),
        );
        // A silent no-op here would put the 11.25-texel sprite back and read
        // as a measurement.
        assert_ne!(
            l4, L4,
            "the sprite is no longer spelled the way this rescaling looks for"
        );

        let gpu = Gpu::headless().expect("no GPU available");

        let summarize = |label: &str, mut xs: Vec<f32>| {
            xs.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
            eprintln!(
                "  {label:<24} n={:<4} median {:.3} ms   worst {:.3} ms",
                xs.len(),
                xs[xs.len() / 2],
                xs[xs.len() - 1]
            );
        };

        let present = Present::new(&gpu.device, Present::HDR_FORMAT, W, H);

        // Bare Set, no deck: the path every earlier test measures.
        let mut bare = Set::build(
            &gpu.device,
            &gpu.queue,
            &compile(L1),
            &compile(&l4),
            CAP,
            SEED_A,
        )
        .expect("the pair is compatible");
        bare.resize(&gpu.device, W, H);
        let mut bare_ms = Vec::new();
        for i in 0..WARMUP + MEASURED {
            let at = Instant::now();
            bare_frame(&gpu, &mut bare, &present, 1);
            if i >= WARMUP {
                bare_ms.push(at.elapsed().as_secs_f32() * 1_000.0);
            }
        }

        let deck_ms = |seeds: &[u32]| -> Vec<f32> {
            let swaps = seeds
                .iter()
                .map(|&seed| {
                    let mut set = Set::build(
                        &gpu.device,
                        &gpu.queue,
                        &compile(L1),
                        &compile(&l4),
                        CAP,
                        seed,
                    )
                    .expect("the pair is compatible");
                    set.resize(&gpu.device, W, H);
                    HotSwap::fixed(set)
                })
                .collect();
            let mut deck = Deck::new(&gpu.device, swaps, W, H);
            let mut out = Vec::new();
            for i in 0..WARMUP + MEASURED {
                let at = Instant::now();
                frame(&gpu, &mut deck, &present, 1);
                if i >= WARMUP {
                    out.push(at.elapsed().as_secs_f32() * 1_000.0);
                }
            }
            out
        };
        let one = deck_ms(&[SEED_A]);
        let four = deck_ms(&[SEED_A, SEED_B, SEED_A + 1, SEED_B + 1]);
        let one_live = {
            let seeds = [SEED_A, SEED_B, SEED_A + 1, SEED_B + 1];
            let swaps = seeds
                .iter()
                .map(|&seed| {
                    let mut set = Set::build(
                        &gpu.device,
                        &gpu.queue,
                        &compile(L1),
                        &compile(&l4),
                        CAP,
                        seed,
                    )
                    .expect("the pair is compatible");
                    set.resize(&gpu.device, W, H);
                    HotSwap::fixed(set)
                })
                .collect();
            let mut deck = Deck::new(&gpu.device, swaps, W, H);
            // Warmed on air first, then parked: three slots with material in
            // them, drawn and not stepped, which is the panel's own state.
            for _ in 0..8 {
                frame(&gpu, &mut deck, &present, 1);
            }
            for slot in 1..4 {
                deck.set_residency(karakuri_engine::DeckSlot(slot), Residency::Allocated);
            }
            let mut out = Vec::new();
            for i in 0..WARMUP + MEASURED {
                let at = Instant::now();
                frame(&gpu, &mut deck, &present, 1);
                if i >= WARMUP {
                    out.push(at.elapsed().as_secs_f32() * 1_000.0);
                }
            }
            out
        };
        // **The same deck, never warmed**, which is the panel's own state
        // rather than this test's: three slots that have been off air since
        // the window opened, holding element state `Simulation::initialize`
        // filled with zeros. Every element is at the origin, so the whole
        // capacity is drawn at one clip position and the raster back end
        // serialises order-dependent blending at that one address. The gap
        // between this line and `four, one Live` is what an unstepped draw
        // costs, and it is the reason ADR-0269 steps every drawn slot.
        let one_live_cold = {
            let seeds = [SEED_A, SEED_B, SEED_A + 1, SEED_B + 1];
            let swaps = seeds
                .iter()
                .map(|&seed| {
                    let mut set = Set::build(
                        &gpu.device,
                        &gpu.queue,
                        &compile(L1),
                        &compile(&l4),
                        CAP,
                        seed,
                    )
                    .expect("the pair is compatible");
                    set.resize(&gpu.device, W, H);
                    HotSwap::fixed(set)
                })
                .collect();
            let mut deck = Deck::new(&gpu.device, swaps, W, H);
            // Off air before a frame has ever run, so nothing warmed them.
            for slot in 1..4 {
                deck.set_residency(karakuri_engine::DeckSlot(slot), Residency::Allocated);
            }
            let mut out = Vec::new();
            for i in 0..WARMUP + MEASURED {
                let at = Instant::now();
                frame(&gpu, &mut deck, &present, 1);
                if i >= WARMUP {
                    out.push(at.elapsed().as_secs_f32() * 1_000.0);
                }
            }
            out
        };

        let deck_mb = 4.0 * f64::from(W) * f64::from(H) * 8.0 / 1_048_576.0;
        eprintln!(
            "\nframe times at capacity {CAP}, {W}x{H}, host clock around submit-and-wait \
     (four slot targets at 8 bytes a texel is {deck_mb:.1} MB):"
        );
        summarize("bare Set, no deck", bare_ms);
        summarize("deck of one", one);
        summarize("deck of four", four);
        summarize("four, one Live", one_live);
        summarize("four, one Live, cold", one_live_cold);
        eprintln!(
        "  `deck of one` is what `four, one Live` cost before every slot was drawn: \n               the three off-air slots did nothing at all on the frame path. The gap between \n               those two lines is what ADR-0258 costs on this machine."
    );
        eprintln!();
    }

    /// What filling a deck slot's **preview cell** costs, two ways. **Printed,
    /// not asserted.**
    ///
    /// A cell in the program bay is **112 x 63** — `karakuri-console`'s
    /// `view::preview_cells` derives the width from `(466 - three 6px gaps) / 4
    /// = 112`, and 112 at 16:9 is 63 — while a slot renders at the deck's
    /// canvas, 1280x720. Two ways to get one into the other, and this measures
    /// both rather than arguing them:
    ///
    /// - **downsample**: present the 1280x720 target into the cell. No extra
    ///   draw. The horizontal stride is `1280 / 112` = 11.4 source texels at 8
    ///   bytes each, so consecutive output texels fall in different cache lines.
    /// - **re-render**: draw the Set again into a 112x63 target. Better
    ///   locality, and **not fewer primitives**: the point count is the
    ///   capacity either way, 262144.
    ///
    /// **The filter decides how many taps the downsample takes, and this one
    /// takes four.** `Present`'s sampler is `Linear`/`Linear` over a texture
    /// with `mip_level_count: 1`, so there is no mip chain to fall back on and
    /// a fragment reads a 2x2 neighbourhood, not an 11x11 box. The downsample
    /// therefore *undersamples* — it is bilinear point-picking with aliasing,
    /// not a box filter — and it does not read the 1.8 MB the source occupies:
    /// 7056 output texels at four taps is 28k taps, scattered.
    ///
    /// **`point_rate` is a fraction of the target's height, so a small target
    /// has fewer fragments per sprite — until a sprite reaches a pixel.**
    /// `karakuri-codegen`'s L4 expansion scales the quad by the rate rather than
    /// by a pixel count, so a sprite is the same share of the frame at every
    /// size. This paragraph read that a 112x63 render therefore rasterises
    /// roughly `(63/720)²` of the fragments a 1280x720 one does, and that is no
    /// longer true at the bottom of the sweep: a quad below a pixel is floored
    /// at one pixel and dimmed rather than dropped (ADR-0245), so a `Points`
    /// procedure's fragment count bottoms out at one per element
    /// instead of falling with the area. At 112x63 this material's sprites are
    /// about a third of a pixel across, so the cell render sits entirely in that
    /// floored regime.
    ///
    /// **The sweep's lit-texel column is not the check for that**, and reading
    /// it as one would be a mistake. At capacity 262144 the coverage saturates —
    /// 231 points land on each lit texel at 112x63 — so the column reports the
    /// material's silhouette rather than any sprite's extent, and it comes out
    /// near 16% at every size for that reason. What a sprite's extent does at
    /// two target sizes is asserted to the texel in `tests/lines.rs`, on one
    /// element, where nothing saturates.
    ///
    /// Same method as its neighbour
    /// [`the_cost_of_a_slot_and_of_the_composite_are_measured_and_reported`]:
    /// host clock around submit-and-wait, the same 60-frame warm-up and
    /// 120-frame window, medians and worst rather than means. Two departures,
    /// both because this compares configurations against each other rather
    /// than reporting them one at a time:
    ///
    /// - **the configurations are interleaved, one frame each per round, and
    ///   the order rotates.** Run as blocks, the first block measured a cold
    ///   GPU and the last a hot one: 1280x720 came out 9.3 ms as the first
    ///   block and 7.3 ms as the last, which is a quarter of the number and
    ///   none of it the configuration. Those two figures are from the run that
    ///   found the hazard, under the pixel-size semantics; the hazard is the
    ///   point and it is not sensitive to either.
    /// - **each present writes into its own cell texture**, so "did this pass
    ///   write anything" can still be asked of each of them at the end. A
    ///   target that persists between frames is the hazard the pixel tests
    ///   above defeat by resizing; here every destination is blackened before
    ///   the loop and counted after it.
    ///
    /// One configuration is **nothing at all** — an empty command buffer,
    /// submitted and waited on. A host clock around submit-and-wait pays for a
    /// round trip whether or not there is work in it, and the present passes
    /// here are small enough that the round trip is most of what is timed:
    /// 0.13 ms of the 0.49 ms. Every present figure worth quoting is net of
    /// that line.
    ///
    /// **Run it alone.** `cargo test` runs the two benchmarks in this file on
    /// two threads and one GPU, and every number in both comes out about 40%
    /// high; `--test-threads=1`, or a name filter, is part of the method.
    ///
    /// **What it found, so that the next reader need not run it.** The
    /// downsample is under a millisecond net of the floor and the 11.4-texel
    /// stride costs almost nothing — 0.883 ms against 0.805 ms for the same
    /// present with no scaling at all.
    ///
    /// **The cost order has reversed twice, and the second time it reversed
    /// back.** Under the old pixel-size semantics, re-rendering into the cell
    /// was 17.5 ms against 9.3 ms for the whole 1280x720 frame. `point_rate`
    /// turned that around — 7.6 ms against 10.5 ms, the sweep climbing with the
    /// target — and the one-pixel floor turned it back. Measured here as a
    /// **pair**, one machine and one sitting, with the floor removed and
    /// restored, because the older figures are another machine's and a
    /// difference between them would be unreadable:
    ///
    /// | target | no floor | floored |
    /// |---|---|---|
    /// | 112x63 | 7.338 ms | 10.573 ms |
    /// | 224x126 | 8.209 ms | 10.700 ms |
    /// | 448x252 | 10.239 ms | 10.036 ms |
    /// | 640x360 | 10.693 ms | 9.860 ms |
    /// | 1280x720 | 11.687 ms | 10.014 ms |
    ///
    /// The draw-only halves say it without the compute in them: 4.797 ms against
    /// 8.103 ms at 112x63, and 7.48 ms either way at 1280x720. **Rendering at
    /// cell size went from 0.6x the whole 720p frame to 1.1x of it**, which is
    /// what a floor of one fragment per element does to a target of 7056 texels
    /// holding 262144 elements.
    ///
    /// **The 1280x720 row is the control, and it moved 1.7 ms.** The floor is
    /// inert there — this material is 1.4 to 4 pixels across at that size — so
    /// that gap is this machine's run-to-run agreement rather than an effect of
    /// anything. Read the shape of the sweep and not the rows.
    ///
    /// **It is still not the way to fill a cell, and the reason has changed.**
    /// The reason used to be that most of this material's sprites cover no pixel
    /// centre at 112x63 and vanish — which was a defect in the renderer rather
    /// than a fact about cells, and ADR-0245 is where it went. What stands in its
    /// place is the plain cost: the full-size target is rendered anyway, so
    /// downsampling costs line 2 alone, 0.489 ms net of the floor, against a
    /// second 10.573 ms pass. **What fills a cell is open again** in the sense
    /// that the correctness argument is spent; the cost argument now points the
    /// same way, and the panel downsamples today.
    ///
    /// All of that is a claim about point sprites and not about every L4 — a
    /// fullscreen node such as `examples/field_march.kir` has a fragment count
    /// that *is* the pixel count, and nothing here measures one.
    ///
    /// `#[ignore]`d for the same reason its neighbour is: capacity 262144,
    /// eleven configurations.
    #[test]
    #[ignore = "a measurement, not a check; run with --ignored --nocapture"]
    fn the_cost_of_filling_a_preview_cell_is_measured_and_reported() {
        /// The panel's own material, rather than this file's fixtures: the
        /// question is about what a slot on the console costs.
        const PANEL_L1: &str = include_str!("../../../../examples/drift_shell.kir");
        const PANEL_L4: &str = include_str!("../../../../examples/soft_points.kir");

        const CAP: u32 = 262_144;
        const W: u32 = 1280;
        const H: u32 = 720;
        const CELL_W: u32 = 112;
        const CELL_H: u32 = 63;
        const WARMUP: usize = 60;
        const MEASURED: usize = 120;

        /// The sizes between the cell and the canvas, for the sweep. The two
        /// ends of it are configurations 0 and 2 and are not repeated here.
        const BETWEEN: [(u32, u32); 3] = [(224, 126), (448, 252), (640, 360)];

        const LABELS: [&str; 11] = [
            "1  render 1280x720",
            "2  present 1280x720 -> cell",
            "3  render 112x63",
            "4  present cell -> cell 1:1",
            "1b draw only, 1280x720",
            "1c step only, no draw",
            "3b draw only, 112x63",
            "   render 224x126",
            "   render 448x252",
            "   render 640x360",
            "0  empty submit + poll",
        ];

        let gpu = Gpu::headless().expect("no GPU available");

        let sorted = |xs: &[f32]| {
            let mut xs = xs.to_vec();
            xs.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
            xs
        };
        let summarize = |label: &str, xs: &[f32]| {
            let xs = sorted(xs);
            eprintln!(
                "  {label:<30} n={:<4} median {:.3} ms   worst {:.3} ms",
                xs.len(),
                xs[xs.len() / 2],
                xs[xs.len() - 1]
            );
        };
        let median = |xs: &[f32]| sorted(xs)[xs.len() / 2];

        let make_set = |width: u32, height: u32| -> Set {
            let mut set = Set::build(
                &gpu.device,
                &gpu.queue,
                &compile(PANEL_L1),
                &compile(PANEL_L4),
                CAP,
                SEED_A,
            )
            .expect("the panel's own pair is compatible");
            set.resize(&gpu.device, width, height);
            set
        };

        let make_cell = |label: &'static str| -> (wgpu::Texture, wgpu::TextureView) {
            let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: CELL_W,
                    height: CELL_H,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: Present::HDR_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            (texture, view)
        };

        // Black, so that "this pass wrote something" can be told from "the
        // texture still holds what something else put there".
        let blacken = |view: &wgpu::TextureView| {
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("blacken"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            }));
            gpu.queue.submit([encoder.finish()]);
            gpu.device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("poll");
        };

        // [`readback`] above requires a 256-aligned row and a 112-wide
        // `Rgba16Float` row is 896 bytes, so this pads the pitch and walks the
        // padding back off. Counting lit texels is all it is for.
        let lit_texels = |texture: &wgpu::Texture| -> u32 {
            let (width, height) = (texture.width(), texture.height());
            let pitch = (width * 8).div_ceil(256) * 256;
            let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("cell readback"),
                size: u64::from(pitch * height),
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
                        bytes_per_row: Some(pitch),
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
            let mut lit = 0;
            for y in 0..height as usize {
                for x in 0..width as usize {
                    let at = y * pitch as usize + x * 8;
                    // The three colour channels only: the present pass writes
                    // alpha 1.0 everywhere and would count every texel.
                    if data[at..at + 6].iter().any(|&b| b != 0) {
                        lit += 1;
                    }
                }
            }
            drop(data);
            buffer.unmap();
            lit
        };

        let canvas = Present::new(&gpu.device, Present::HDR_FORMAT, W, H);
        let cell_canvas = Present::new(&gpu.device, Present::HDR_FORMAT, CELL_W, CELL_H);
        let (down_cell, down_view) = make_cell("downsampled cell");
        let (flat_cell, flat_view) = make_cell("1:1 cell");

        let mut big = make_set(W, H);
        let mut small = make_set(CELL_W, CELL_H);
        let mut between: Vec<(Present, Set)> = BETWEEN
            .iter()
            .map(|&(w, h)| {
                (
                    Present::new(&gpu.device, Present::HDR_FORMAT, w, h),
                    make_set(w, h),
                )
            })
            .collect();

        for view in [
            canvas.hdr_view(),
            cell_canvas.hdr_view(),
            &down_view,
            &flat_view,
        ] {
            blacken(view);
        }
        for (target, _) in &between {
            blacken(target.hdr_view());
        }

        let submit = |encoder: wgpu::CommandEncoder| {
            gpu.queue.submit([encoder.finish()]);
            gpu.device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("poll");
        };

        // One frame of one configuration, submitted and waited on. All ten in
        // one closure because several of them share a Set and cannot each hold
        // their own mutable borrow of it.
        let mut run = |cfg: usize| {
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            match cfg {
                // The slot at the deck's canvas: the baseline, and the same
                // path `bare_frame` takes.
                0 => {
                    big.prepare(&gpu.queue, 1, &Signals::default());
                    big.render(&mut encoder, canvas.hdr_view(), 1);
                }
                // The downsample. The canvas holds a real picture from
                // configuration 0, so this is not sampling a texture that has
                // only ever been fast-cleared.
                1 => canvas.draw(&mut encoder, &down_view, (CELL_W, CELL_H)),
                // The same Set rendered straight into a cell-sized target.
                2 => {
                    small.prepare(&gpu.queue, 1, &Signals::default());
                    small.render(&mut encoder, cell_canvas.hdr_view(), 1);
                }
                // A present that is not a downsample: 112x63 into 112x63,
                // viewport 1:1, so the difference from configuration 1 is what
                // the 11.4-texel stride costs and nothing else.
                3 => cell_canvas.draw(&mut encoder, &flat_view, (CELL_W, CELL_H)),
                // The raster half alone — an off-air slot's own path, drawn
                // every frame and never stepped.
                4 => big.draw(&mut encoder, canvas.hdr_view()),
                // The compute half alone: no target, no draw.
                5 => {
                    big.prepare(&gpu.queue, 1, &Signals::default());
                    big.step(&mut encoder, 1);
                }
                6 => small.draw(&mut encoder, cell_canvas.hdr_view()),
                // The sweep: everything held fixed but the target size.
                7..=9 => {
                    let (target, set) = &mut between[cfg - 7];
                    set.prepare(&gpu.queue, 1, &Signals::default());
                    set.render(&mut encoder, target.hdr_view(), 1);
                }
                // Nothing at all, submitted and waited on: the floor this
                // harness can measure. A present pass costing "tens of
                // microseconds" is unreadable here unless it is read against
                // this line, because a host clock around submit-and-wait is
                // paying for a round trip either way.
                _ => {}
            }
            submit(encoder);
        };

        let mut times: Vec<Vec<f32>> = vec![Vec::with_capacity(MEASURED); LABELS.len()];
        for i in 0..WARMUP + MEASURED {
            // Rotated, so no configuration is permanently the one that follows
            // the heaviest.
            for j in 0..LABELS.len() {
                let cfg = (j + i) % LABELS.len();
                let at = Instant::now();
                run(cfg);
                if i >= WARMUP {
                    times[cfg].push(at.elapsed().as_secs_f32() * 1_000.0);
                }
            }
        }

        eprintln!(
            "\nfilling one {CELL_W}x{CELL_H} preview cell from a {W}x{H} slot, \
         capacity {CAP}, `examples/drift_shell.kir` + `examples/soft_points.kir`,\n\
         host clock around submit-and-wait, interleaved, \
         {WARMUP} rounds of warm-up discarded:"
        );
        summarize(LABELS[10], &times[10]);
        for cfg in 0..4 {
            summarize(LABELS[cfg], &times[cfg]);
        }
        eprintln!("  and, to split line 1:");
        for cfg in 4..7 {
            summarize(LABELS[cfg], &times[cfg]);
        }
        eprintln!(
            "  the compute half is {:.3} ms of line 1's {:.3} ms and the raster half \
         {:.3} ms;\n               rendering at cell size leaves {:.3} ms, which is \
         {:.1}x the whole 720p frame.",
            median(&times[5]),
            median(&times[0]),
            median(&times[4]),
            median(&times[2]),
            median(&times[2]) / median(&times[0]),
        );

        eprintln!(
            "  the same Set and the same {CAP} points, target size swept — \
         the cost follows the target, and the lit share does not:"
        );
        let sweep = [
            (
                CELL_W,
                CELL_H,
                median(&times[2]),
                lit_texels(cell_canvas.hdr_texture()),
            ),
            (
                BETWEEN[0].0,
                BETWEEN[0].1,
                median(&times[7]),
                lit_texels(between[0].0.hdr_texture()),
            ),
            (
                BETWEEN[1].0,
                BETWEEN[1].1,
                median(&times[8]),
                lit_texels(between[1].0.hdr_texture()),
            ),
            (
                BETWEEN[2].0,
                BETWEEN[2].1,
                median(&times[9]),
                lit_texels(between[2].0.hdr_texture()),
            ),
            (W, H, median(&times[0]), lit_texels(canvas.hdr_texture())),
        ];
        for (w, h, ms, lit) in sweep {
            eprintln!(
                "    {w:>4}x{h:<4} {ms:>7.3} ms   {lit} lit of {} ({:.1}% of the target) \
             — {:.0} points per lit texel",
                w * h,
                100.0 * f64::from(lit) / f64::from(w * h),
                f64::from(CAP) / f64::from(lit.max(1)),
            );
        }

        eprintln!(
            "  net of the floor, the downsample is {:.3} ms and the 1:1 present {:.3} ms.",
            median(&times[1]) - median(&times[10]),
            median(&times[3]) - median(&times[10]),
        );

        let (down_lit, flat_lit) = (lit_texels(&down_cell), lit_texels(&flat_cell));
        eprintln!(
            "  the two cells came back {down_lit} and {flat_lit} lit of {}; \
         zero would mean a present pass wrote nothing and was timed anyway.",
            CELL_W * CELL_H
        );
        assert!(
            down_lit > 0 && flat_lit > 0,
            "a present pass wrote nothing, so its timing is not a timing of the present"
        );
        eprintln!();
    }

    /// A mix target that is not the deck's size is refused rather than mixed.
    ///
    /// The composite reads its sources with `textureLoad`, and an out-of-range
    /// `textureLoad` is *defined* to return zero — so resizing `Present` and
    /// forgetting the deck would produce a black frame, every frame, with nothing
    /// logged. A panic at the call is louder than a picture that is quietly wrong,
    /// and this is a programming error rather than an input error: no `.kir` and
    /// no record stream can reach it.
    #[test]
    #[should_panic(expected = "resize both")]
    fn a_mix_target_of_the_wrong_size_is_refused() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut deck = deck_of(&gpu, &[1]);
        let present = Present::new(
            &gpu.device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            WIDTH,
            HEIGHT,
        );

        // The deck moves, the target does not — the direction a window resize
        // takes if only one of the two handlers is wired.
        deck.resize(&gpu.device, WIDTH * 2, HEIGHT * 2);

        let mut frame = deck.begin_frame(&gpu.device, &gpu.queue);
        frame.render(present.hdr_view(), present.size(), 1);
    }

    /// A frame that is forgotten (e.g. dropped without submitting via `std::mem::forget`)
    /// leaves host simulation clocks, signals, and buffer ping-pong parity uncommitted and
    /// completely uncorrupted. The subsequent frame can begin, render, and submit cleanly
    /// without state desynchronization.
    #[test]
    fn forgetting_a_frame_does_not_corrupt_set_or_desync_parity_and_clock() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut deck = deck_of(&gpu, &[SEED_A]);
        let present = Present::new(
            &gpu.device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            WIDTH,
            HEIGHT,
        );

        let slot0 = karakuri_engine::DeckSlot(0);

        // Verify initial state: 0 steps, initial oscillator phase
        assert_eq!(deck.signals().oscillator().steps_taken(), 0);
        assert_eq!(deck.slot(slot0).live().steps_taken(), 0);
        assert_eq!(deck.slot(slot0).live().staged_delta(), 0);
        assert_eq!(deck.slot(slot0).live().committed_parity(), 0);

        let start = deck.signals().oscillator().beats();
        deck.schedule(Transition::new(
            0,
            Control::Opacity,
            1.0,
            0.0,
            start,
            4.0,
            Curve::Lin,
        ));
        deck.schedule_selection(Selection::new(0, 0, start));
        assert_eq!(deck.transitions_on(slot0).count(), 1);
        assert_eq!(deck.selections_on(slot0).count(), 1);

        // Phase 1: Begin a frame, render with 3 steps, but forget it before submitting.
        let mut frame = deck.begin_frame(&gpu.device, &gpu.queue);
        frame.render(present.hdr_view(), present.size(), 3);
        std::mem::forget(frame);

        // Host committed state must remain untouched: 0 steps taken, parity uncommitted,
        // and transitions/selections not yet committed.
        assert_eq!(deck.signals().oscillator().steps_taken(), 0);
        assert_eq!(deck.slot(slot0).live().steps_taken(), 0);
        assert_eq!(deck.slot(slot0).live().staged_delta(), 3);
        assert_eq!(deck.slot(slot0).live().committed_parity(), 0);
        assert_eq!(deck.transitions_on(slot0).count(), 1);
        assert_eq!(deck.selections_on(slot0).count(), 1);

        // Phase 2: Discard test - begin another frame (which discards the uncommitted staged delta),
        // render with 4 steps, then call frame.discard().
        let mut frame = deck.begin_frame(&gpu.device, &gpu.queue);
        frame.render(present.hdr_view(), present.size(), 4);
        frame.discard();

        // Host state must still remain untouched and discard cleared staged state.
        assert_eq!(deck.signals().oscillator().steps_taken(), 0);
        assert_eq!(deck.slot(slot0).live().steps_taken(), 0);
        assert_eq!(deck.slot(slot0).live().staged_delta(), 0);
        assert_eq!(deck.slot(slot0).live().committed_parity(), 0);
        assert_eq!(deck.transitions_on(slot0).count(), 1);
        assert_eq!(deck.selections_on(slot0).count(), 1);

        // Phase 3: Now render a real frame with 1 step and submit it.
        let mut frame = deck.begin_frame(&gpu.device, &gpu.queue);
        frame.render(present.hdr_view(), present.size(), 1);
        frame.finish();

        // Host state must now reflect exactly 1 step committed.
        assert_eq!(deck.signals().oscillator().steps_taken(), 1);
        assert_eq!(deck.slot(slot0).live().steps_taken(), 1);
        assert_eq!(deck.slot(slot0).live().staged_delta(), 0);
        assert_eq!(deck.slot(slot0).live().committed_parity(), 1);
        assert_eq!(deck.selections_on(slot0).count(), 0);
    }
}
