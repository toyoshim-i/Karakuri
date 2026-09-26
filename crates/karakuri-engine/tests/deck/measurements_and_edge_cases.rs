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

        // Rescale point_rate to maintain constant pixel diameter regardless of target resolution.
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
        // Unstepped cold deck baseline measuring uninitialized origin-concentrated primitives.
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

    /// Measures preview cell presentation cost comparing downsampling against re-rendering.
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
                // Baseline floor representing empty command buffer submission and wait.
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

    /// Verifies that rendering into a target mismatched with deck dimensions panics.
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
