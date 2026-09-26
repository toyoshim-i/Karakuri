use super::common::*;

mod gpu {
    use super::*;

    /// ADR-0155: Verifies rendered engine texels reach the presentation panel in correct order,
    /// asserting lit texel counts within the picture bounds.
    #[test]
    fn the_engines_frame_reaches_the_picture_in_the_program_bay() {
        // Gamma space, and 1408 rather than 1440 because
        // `copy_texture_to_buffer` wants `bytes_per_row` a multiple of 256.
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1408;
        const H: u32 = 900;
        const ROOM: Room = Room::Night;

        let gpu = Gpu::headless().expect("no GPU");
        let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("program probe"),
            size: wgpu::Extent3d {
                width: W,
                height: H,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());

        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        let rect = picture_rect(panel.layout(), CANVAS).expect("the picture is on screen");
        let cells = preview_rects(panel.layout(), CANVAS).expect("the preview row is on screen");
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            HEADLESS_PICTURE_FORMAT,
            &shipped_slots(),
            panel.layout(),
            1.0,
            None,
            None,
            mcp::Slots::unpointed(),
        );
        // **Built at what the file declares**, which is the other half of
        // `the_capacity_is_the_l1s_own_declaration_and_the_l4_declares_none`:
        // that one says what the `.kir` says, and this one says the deck was
        // built with it rather than with a number written here.
        assert_eq!(
            engine.capacity,
            checked(&shipped().l1)
                .capacity
                .expect("the L1 declares a capacity")
                .default,
            "the deck was not built at the capacity its L1 declares"
        );

        // Aims view cells via `Engine::aim` to test render pass sequencing against the panel.
        let mut view = View::new(ROOM);
        (view.picture, view.previews) = engine.aim(&gpu, &mut renderer, panel.layout(), 1.0, None);
        assert_eq!(
            view.picture.expect("the picture was not aimed").rect,
            rect,
            "the picture is drawn somewhere other than the region it was sized from"
        );
        assert_eq!(
            view.previews[0].expect("deck A was not aimed").rect,
            cells[0]
        );

        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(W as f32, H as f32),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| view.draw(ui, &mut panel));
        let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [W, H],
            pixels_per_point: 1.0,
        };
        for (id, deltas) in &output.textures_delta.set {
            for delta in deltas {
                renderer.update_texture(&gpu.device, &gpu.queue, *id, delta);
            }
        }

        let row = W * 4;
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("program probe"),
            size: (row * H) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Composes frame through engine sinks and hands encoder to panel closure in a single submission.
        let mut user_empty = false;
        let mut refusals: Vec<(usize, Skip)> = Vec::new();
        let outcome = {
            let textures_delta = &mut output.textures_delta;
            let Engine {
                deck,
                present,
                picture,
                previews,
                ..
            } = &mut engine;
            let mut sinks: [&mut dyn Sink; 2] = [picture, &mut previews[0]];
            compose(
                &gpu,
                deck,
                present,
                &mut sinks,
                &mut |at, skip| refusals.push((at, skip)),
                |_| Committed {
                    steps: STEPS_A_FRAME,
                    look: LOOK,
                },
                |encoder| {
                    let user = renderer.update_buffers(
                        &gpu.device,
                        &gpu.queue,
                        encoder,
                        &primitives,
                        &screen,
                    );
                    {
                        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("program probe"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &target_view,
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
                        });
                        renderer.render(&mut pass.forget_lifetime(), &primitives, &screen);
                    }
                    for id in &textures_delta.free {
                        renderer.free_texture(id);
                    }
                    textures_delta.clear();
                    encoder.copy_texture_to_buffer(
                        wgpu::TexelCopyTextureInfo {
                            texture: &target,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::TexelCopyBufferInfo {
                            buffer: &readback,
                            layout: wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(row),
                                rows_per_image: Some(H),
                            },
                        },
                        wgpu::Extent3d {
                            width: W,
                            height: H,
                            depth_or_array_layers: 1,
                        },
                    );
                    user_empty = user.is_empty();
                },
            )
            .expect("neither of the console's sinks presents anything")
        };
        assert!(
            user_empty,
            "a paint callback appeared: it has to be submitted ahead of the pass"
        );
        assert!(refusals.is_empty(), "a sink refused: {refusals:?}");
        // **Both sinks took the frame, and nothing else was in the slice.**
        // The panel is not one of them — it is drawn in `finally`, and a
        // `reached` of 3 here would be the console counting a consumer as an
        // output. See `frame::compose`.
        assert_eq!(
            outcome,
            karakuri_engine::Outcome {
                reached: 2,
                missed: 0
            }
        );

        readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the device stopped");
        let texels = readback.slice(..).get_mapped_range().expect("readback");
        let at = |x: u32, y: u32| {
            let i = (y * row + x * 4) as usize;
            [texels[i], texels[i + 1], texels[i + 2]]
        };
        let brightest = |rgb: [u8; 3]| rgb.into_iter().max().unwrap_or(0);

        // Counts dark and bright texels in a rect to verify opaque background and rendered particles.
        let counted = |r: egui::Rect| {
            let mut dark = 0usize;
            let mut bright = 0usize;
            let mut inside = 0usize;
            for y in r.min.y as u32..r.max.y as u32 {
                for x in r.min.x as u32..r.max.x as u32 {
                    inside += 1;
                    match brightest(at(x, y)) {
                        b if b <= 8 => dark += 1,
                        b if b >= 192 => bright += 1,
                        _ => {}
                    }
                }
            }
            (dark, bright, inside)
        };

        let (dark, bright, inside) = counted(rect);
        assert!(
            dark * 2 > inside,
            "only {dark} of {inside} texels in the picture are darker than anything the \
         panel draws — the picture is the bay's card, so the engine's texture never \
         reached it"
        );
        assert!(
            bright * 100 > inside,
            "{bright} of {inside} texels in the picture are lit — the picture reached the \
         panel and there is nothing in it, so the deck composited nothing and this \
         would pass over a black rectangle"
        );

        // Asserts deck preview cell presentation matches expected dark/lit threshold distributions across different target dimensions.
        let (dark, bright, inside) = counted(cells[0]);
        assert!(
            dark * 2 > inside,
            "only {dark} of {inside} texels in deck A's cell are darker than anything the \
         panel draws — the cell is still the mock's well, so the second present pass \
         never reached it"
        );
        assert!(
            bright * 100 > inside,
            "{bright} of {inside} texels in deck A's cell are lit — the audition reached \
         the panel and there is nothing in it"
        );

        // **And each stayed where it was put.** Three controls, because a
        // picture drawn over the whole window would satisfy every count above:
        // deck D's cell is off, so it is the mock's well and nothing else;
        // and a bay the Program is nowhere near is still the bay's card.
        let pal = ROOM.palette();
        let panel_rgb = [pal.panel.r(), pal.panel.g(), pal.panel.b()];
        let well_rgb = [pal.well.r(), pal.well.g(), pal.well.b()];
        let d = cells[DECKS - 1].center();
        assert_eq!(
            at(d.x as u32, d.y as u32),
            well_rgb,
            "deck D's cell is off and is not the mock's well — either the picture painted \
         over the preview row, or an audition was drawn outside its own cell"
        );
        let library = panel
            .layout()
            .rect(panel.layout().find("library").expect("library"));
        assert_eq!(
            at(
                (library.x + library.w * 0.5) as u32,
                (library.y + library.h * 0.5) as u32
            ),
            panel_rgb,
            "the picture reached the Library bay"
        );
    }

    /// Verifies picture texture dimensions match canvas aspect ratio without redundant padding or oversized margins.
    #[test]
    fn the_picture_is_the_canvass_shape_and_carries_no_bars() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());

        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
        let rect = picture_rect(panel.layout(), CANVAS).expect("on screen");
        let want = physical(rect, 1.0);
        let engine = Engine::new(
            &gpu,
            &mut renderer,
            HEADLESS_PICTURE_FORMAT,
            &shipped_slots(),
            panel.layout(),
            1.0,
            None,
            None,
            mcp::Slots::unpointed(),
        );

        // The picture's, in both axes, and **neither of them is the window's**
        // — the picture is narrower than the window by both panes and taller
        // by nothing like the window's height.
        assert_eq!(
            (
                engine.picture.texture.width(),
                engine.picture.texture.height()
            ),
            want
        );
        assert_ne!(engine.picture.size, (W, H));
        assert!(engine.picture.size.0 < W && engine.picture.size.1 < H / 2);
        assert!(renderer.texture(&engine.picture.id).is_some());

        // **And it is not the region's either**, which is the texture this
        // change removes: the region is the same height and hundreds of pixels
        // wider, all of it bars.
        let region = panel
            .layout()
            .rect(panel.layout().find("program-view").expect("program-view"));
        assert!(
            (region.w - rect.width()) > 180.0,
            "the picture is the width of its region, so it is the region that was sized \
         from and the bars are still inside the texture: {} against {}",
            rect.width(),
            region.w
        );

        // **No bars, asked of the pass that would draw them.** `letterbox` is
        // what `Present::draw` sets its viewport from, so what it leaves over
        // at the edges is exactly what gets cleared to black.
        let (x, y, w, h) = letterbox(CANVAS, engine.picture.size);
        let (tw, th) = (engine.picture.size.0 as f32, engine.picture.size.1 as f32);
        assert!(
            x < 1.0 && y < 1.0,
            "the canvas sits {x} x {y} into its own texture, which is {} and {} texels of \
         bar down each side — the picture is not the canvas's shape",
            x.round(),
            y.round()
        );
        assert!(
            w > tw - 2.0 && h > th - 2.0,
            "the canvas covers {w} x {h} of a {tw} x {th} texture, so the rest is cleared \
         to black every frame"
        );

        // The control on all of it: a region-sized texture is what the
        // assertions above would pass over, and it does not — this is the
        // number in the doc, computed rather than quoted.
        let (bar, _, _, _) = letterbox(CANVAS, (region.w.round() as u32, want.1));
        assert!(
            bar > 80.0,
            "a texture sized from the region would carry {bar} texels of bar, and the \
         thresholds above are not measuring anything"
        );
    }

    /// Verifies composited frame size coordinates across picture bounds and projector target sizes (ADR-0325).
    #[test]
    fn the_frame_is_composited_at_the_largest_enabled_output() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());

        let mut panel = Panel::new(W as f32, H as f32);
        view::rearrange(&mut panel, CANVAS);
        let picture = physical(
            picture_rect(panel.layout(), CANVAS).expect("on screen"),
            1.0,
        );
        let mut engine = Engine::new(
            &gpu,
            &mut renderer,
            HEADLESS_PICTURE_FORMAT,
            &shipped_slots(),
            panel.layout(),
            1.0,
            None,
            None,
            mcp::Slots::unpointed(),
        );

        // **The picture alone**, which is every run this program opens on.
        engine.aim(&gpu, &mut renderer, panel.layout(), 1.0, None);
        assert_eq!(
            engine.present.size(),
            picture,
            "the frame is composited at {:?} while the only output on is the picture at \
         {picture:?} — every output is a downscale of the one render, so a frame larger \
         than its only destination is a render nobody asked for and a frame smaller is \
         an upscale",
            engine.present.size()
        );
        assert_ne!(
            picture, CANVAS,
            "this window's picture happens to be exactly the session canvas, so the \
         assertion above cannot tell the derivation from the constant it replaced"
        );

        // **A projector on, and it is larger**, so the frame follows it and
        // the picture becomes a downscale of one render.
        let projector = (3840, 2160);
        engine.aim(&gpu, &mut renderer, panel.layout(), 1.0, Some(projector));
        assert_eq!(
            engine.present.size(),
            projector,
            "a projector larger than the picture did not raise the frame, so it would be \
         shown an upscale of the picture's size"
        );

        // **And the deck followed the present pass.** Composing is what says
        // so: `Frame::render` checks the size it is handed against the deck's
        // and panics at the call site, so a deck left at the old size is a
        // panic here rather than a black frame later.
        let Engine {
            deck,
            present,
            picture: into,
            look,
            ..
        } = &mut engine;
        let mut sinks: [&mut dyn Sink; 1] = [into];
        compose(
            &gpu,
            deck,
            present,
            &mut sinks,
            &mut |_, _| {},
            |_| Committed {
                steps: STEPS_A_FRAME,
                look: *look,
            },
            |_| {},
        )
        .expect("the frame composes at the derived size");

        // Disabling the projector drops presentation size back to picture bounds.
        engine.aim(&gpu, &mut renderer, panel.layout(), 1.0, None);
        assert_eq!(
            engine.present.size(),
            picture,
            "with the projector gone the picture is the only output left and the frame is \
         its size again"
        );
    }
}
