use super::common::*;

mod gpu {
    use super::*;

    /// ADR-0155's other half, as an assertion: the engine's texels reach the panel.
    ///
    /// The first half — `egui` and `karakuri-engine` resolving one `wgpu` and
    /// sharing one `Device` — is what `egui_paints_the_console_onto_a_device` below
    /// settles. This is the question that was left: a Set is built, a deck frame is
    /// rendered, the present pass letterboxes the canvas into the picture's
    /// rectangle, and the panel's own pass samples that texture — all into one
    /// command encoder and one submission, engine first — and what lands in the
    /// window is read back.
    ///
    /// # Why the assertion is "lit" and not "not the bay's colour"
    ///
    /// A picture that never had anything drawn into it is black, and black is
    /// already not `--c-panel`. So "the picture is not the card" passes for a
    /// texture that was registered, sampled and never rendered — which is exactly
    /// what the ordering defect produces: record the panel's pass before the
    /// engine's and the frame samples an empty texture, with no complaint from
    /// anywhere. The particles are the evidence, so the count of lit texels inside
    /// the picture is what is asserted, and the two controls beside it — the bay's
    /// own body, and the deck preview row — say the picture stayed in its region.
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

        // **Aimed by the call the window makes, and the view is what that
        // answered** rather than three lines this test writes by hand: an id
        // or a rectangle assembled here is a test agreeing with itself about
        // the one thing `Engine::aim` exists to decide. All four cells are
        // aimed — every slot has a Set and every slot is drawn — and this test
        // drives one of them, because what it is about is the ordering of the
        // engine's pass against the panel's rather than the row.
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

        // **The frame, through the call the window loop makes** — not a
        // hand-rolled copy of it beside it, which is what this used to be and
        // is the drift `karakuri_engine::frame` exists to end. `compose` asks
        // both sinks, advances the deck, presents the canvas into each of
        // them, and hands the frame's own encoder to the closure: the panel,
        // over the top of all of it, in one submission.
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

        // **The picture, and it is lit.** Every texel of the region the
        // rectangle names, counted rather than sampled: the material is
        // additive points on a black clear, so a handful of rows through the
        // middle could miss and a count cannot.
        // Two counts over every texel of the region, rather than a handful of
        // samples through the middle: the material is additive points on a
        // black clear, so a row that missed would say nothing and a count
        // cannot.
        //
        // **Dark** is the present pass's own clear — the letterbox bars, and
        // the empty sky between the particles — and it is what the bay's card
        // is not: `--c-panel` at night is `#17142a`, whose brightest channel
        // is 42. It is the half the ordering defect fails, and it fails it
        // completely rather than by a margin: an unwritten texture is
        // `rgba(0, 0, 0, 0)` and `egui` blends premultiplied, so a picture
        // sampled before it was drawn is not black — it is *transparent*, and
        // the card shows through every texel of it. Recording the panel's pass
        // before the engine's, and leaving the present pass out altogether,
        // both read here as **zero** dark texels.
        //
        // **Bright** is the particles, well past anything the panel draws. It
        // is the control on the fixture, in the sense `karakuri-cli`'s frame
        // tests use: a picture that is opaque and empty — a deck compositing
        // nothing — is all dark and no bright, and would satisfy the first
        // count while showing an operator a black rectangle.
        // Two counts over every texel of a rectangle: dark, and lit. A helper
        // because the picture and deck A's cell are the same question asked of
        // two rectangles, and a second copy of the loop is a second threshold
        // to keep in step.
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

        // **Deck A's cell, and the same two counts.** It is the second present
        // pass arriving, and it fails the same two ways: a cell the pass never
        // wrote is an unrendered texture, which is transparent rather than
        // black, so the well shows through every texel of it and *nothing* is
        // dark. The lit count is the control on that — a cell that is opaque
        // and empty would satisfy the first and show an operator a black
        // thumbnail.
        //
        // The cell is a fifth the picture's width, so this is also the claim
        // that one `Present` fits its canvas into two targets of different
        // sizes rather than drawing the picture's rectangle twice.
        //
        // **The same two thresholds as the picture**, on the same helper, and
        // they are not tuned to this rectangle: measured here the cell comes
        // out 76% dark and 16% lit, against the 50% and 1% asked for. A cell
        // that reads anything like a lit picture passes; one the pass missed
        // reads zero dark, which is a factor away rather than a margin.
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

    /// The picture's texture is the size of the picture, the picture is the
    /// canvas's shape, and the texture therefore carries no bars.
    ///
    /// Three claims and every one of them fails without a mark on the screen. A
    /// texture sized from the window looks perfectly correct — the picture fills
    /// whatever rectangle it is given — and is wrong by however much the panel is
    /// not the picture, which here is most of it. A texture sized from the whole
    /// region looks perfectly correct too, and that is the one this change is
    /// about: it is the shape of the region rather than of the canvas, so
    /// `Present::draw` fills the middle of it and clears the rest, and the bars are
    /// allocated, cleared and sampled sixty times a second for nobody. At this
    /// window that is 225 texels down each side of a 916-wide texture.
    ///
    /// The bars are asked of the engine's own `letterbox` rather than re-derived
    /// here, because that is the function that draws them: it answers where the
    /// canvas sits inside the texture, so a bar is what it leaves over. What is
    /// asserted is that the bar is under one texel — not zero, and the difference
    /// is the whole of why `Present::draw` stays. `picture_rect` rounds to whole
    /// pixels, so the picture is the mock's 466 x 262 rather than exactly 16:9, and
    /// the fit still has a quarter of a pixel to absorb. Sub-texel is what this
    /// change makes it; redundant is what it does not.
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

    /// The frame is composited at the picture's rectangle, and a projector raises
    /// it — ADR-0325, on a real device rather than on `render_size`'s arithmetic.
    ///
    /// # Why this is not `render_size`'s test said twice
    ///
    /// `render_size` is a maximum over sizes and is tested on the CPU. What this
    /// asserts is the *wiring*: that the picture's size is what reaches that
    /// maximum, that both the deck and the present pass followed the answer, and
    /// that they followed it together — which is the one thing a caller can get
    /// wrong here and be told about a frame later, by `Frame::render`'s size check
    /// panicking at the call site. Injecting a resize of one and not the other
    /// passes every CPU test in this file.
    ///
    /// The projector's size is handed in rather than a window opened, because no
    /// test in this workspace can open one: `ActiveEventLoop::create_window` needs
    /// a live event loop and `mod gpu` has none. That is the seam `Engine::aim`'s
    /// `projector` argument is on, and it is exactly why the argument is a size
    /// rather than a `&Projector`.
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

        // **The projector off again, and the frame comes back down.** The
        // picture is still on, so the maximum is over one output and it is the
        // picture's — which is the half of the rule that costs: turning a
        // larger sink on raises what every frame costs and turning it off
        // lowers it again, visibly and by the operator's own act. The other
        // half, where *every* output is off and the size stands, is
        // `render_size`'s `None` and is asserted on the CPU.
        engine.aim(&gpu, &mut renderer, panel.layout(), 1.0, None);
        assert_eq!(
            engine.present.size(),
            picture,
            "with the projector gone the picture is the only output left and the frame is \
         its size again"
        );
    }

    /// A wider window remakes no texture at all, and a drag on the program's height
    /// remakes one and frees the registration it replaces.
    ///
    /// The first half is new and is the saving this change is for. The picture's
    /// rectangle used to follow the window's width, so every frame of a horizontal
    /// drag was a texture destroyed and rebuilt and a registration freed and
    /// re-registered — on the render thread. It is the canvas's shape now and the
    /// arrangement pins its height, so a widening moves nothing and there is
    /// nothing to remake. That is asserted rather than described, because a rule
    /// that quietly went back to remaking costs exactly what it used to and says
    /// nothing.
    ///
    /// The second half is the claim the first one must not be allowed to weaken: a
    /// `register_native_texture` with no `free_texture` beside it leaks a bind
    /// group and a sampler per remade frame, and the height is still something an
    /// operator drags. So the free is asserted on the resize that still happens.
    #[test]
    fn a_wider_window_remakes_nothing_and_a_taller_picture_frees_the_old_texture() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());

        let mut panel = Panel::new(W as f32, H as f32);
        view::rearrange(&mut panel, CANVAS);
        let want = physical(
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
        let first = engine.picture.id;
        assert_eq!(engine.picture.size, want);

        // **A window 30 wider, and nothing moves.** The region widens and the
        // picture does not, so `aim` — the call the frame makes — finds the
        // size it already had and remakes nothing.
        panel.set_viewport(W as f32 + 30.0, H as f32);
        view::rearrange(&mut panel, CANVAS);
        assert!(
            !panel
                .layout()
                .is_set_aside(panel.layout().find("deck-previews").expect("the row")),
            "1470 is past the crossover, so this is two arrangements and not one width"
        );
        let wider = physical(
            picture_rect(panel.layout(), CANVAS).expect("on screen"),
            1.0,
        );
        assert_eq!(
            wider, want,
            "a wider window changed the picture's texture, so the picture is still the \
         width of its region and every frame of a horizontal drag reallocates"
        );
        engine.aim(&gpu, &mut renderer, panel.layout(), 1.0, None);
        assert_eq!(
            engine.freed, 0,
            "a wider window freed a registration, so it remade the texture"
        );
        assert_eq!(engine.picture.id, first);
        assert_eq!(engine.picture.size, want);

        // **A drag on the program's bottom edge is what does change it** —
        // through the panel's own pointer, which is the gesture the leak is
        // about rather than a size written by hand.
        let program = panel
            .layout()
            .rect(panel.layout().find("program").expect("program"));
        let edge = Point {
            x: program.x + program.w * 0.5,
            y: program.y + program.h + 2.0,
        };
        assert!(
            matches!(panel.press(edge), Pressed::Grabbed { .. }),
            "the boundary under the program is not where the drag starts"
        );
        panel.moved(Point {
            x: edge.x,
            y: edge.y + 300.0,
        });
        // A boundary, so there is no destination to hand in — see
        // `Panel::released`, which takes one for the carry's sake alone.
        panel.released(None);
        view::rearrange(&mut panel, CANVAS);
        let taller = physical(
            picture_rect(panel.layout(), CANVAS).expect("on screen"),
            1.0,
        );
        assert!(
            taller.1 > want.1 && taller.0 > want.0,
            "dragging the program taller did not grow the picture: {taller:?} against \
         {want:?}"
        );

        engine.aim(&gpu, &mut renderer, panel.layout(), 1.0, None);
        assert_eq!(engine.picture.size, taller);
        assert_eq!(
            (
                engine.picture.texture.width(),
                engine.picture.texture.height()
            ),
            taller
        );
        assert_ne!(engine.picture.id, first);
        assert!(renderer.texture(&engine.picture.id).is_some());
        assert!(
            renderer.texture(&first).is_none(),
            "the registration the resize replaced is still in the atlas, so the atlas \
         grows once per dragged frame"
        );
        assert_eq!(engine.freed, 1);

        // And a second aim with nothing moved remakes nothing, which is what
        // keeps all of the above on the resize path instead of on every frame.
        engine.aim(&gpu, &mut renderer, panel.layout(), 1.0, None);
        assert_eq!(engine.freed, 1);
        assert!(renderer.texture(&engine.picture.id).is_some());
    }

    /// Every cell with a deck slot behind it is aimed, whatever that slot's
    /// residency — and a cell with no slot behind it is off.
    ///
    /// This is
    /// [ADR-0258](../../../docs/adr/0258-the-look-comes-before-the-fader-so-a-cell-draws-every-slot-and-says-which-nothing-it-is.md)
    /// on this surface. The operator decides whether to raise a fader by watching
    /// the cell, so the cell has to be running *before* the fader goes up; a cell
    /// gated on `Residency::Live` answers the question only after it has stopped
    /// being asked, and that gate was here.
    ///
    /// All four residency arrangements, and the one that fails a constant. A cell
    /// aimed because a constant said four would pass this while being the older
    /// defect in the other direction — so the deck is put through every level,
    /// including all four Allocated, where a live-gated `aim` reports nothing at
    /// all and a correct one reports four.
    ///
    /// The empty case is the fourth cell of a deck that does not have one.
    /// `Engine::new` says a slot cannot hold nothing — `HotSwap::new` takes a live
    /// `Set` — so *empty* is not a slot with no material, it is a cell with no
    /// slot: `Deck::slot_view` is `None` past `slot_count`, the bind group is
    /// `None`, no pass is recorded, the view's entry stays `None` and
    /// `karakuri_console::view` draws `D · no slot` in `pal.faint`. This deck is
    /// full, so that is asserted where it can be — the view past the last slot —
    /// rather than by building a short deck this program cannot have.
    #[test]
    fn every_cell_with_a_slot_behind_it_is_aimed_whatever_its_residency() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());
        let mut panel = Panel::new(W as f32, H as f32);
        view::rearrange(&mut panel, CANVAS);
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

        // **A cell has a slot behind it or it has nothing**, and that is the
        // whole of the gate. Past the last slot there is no view to sample.
        assert_eq!(
            engine.deck.slot_count(),
            DECKS,
            "this program's deck is full, so every cell has a slot and the empty case is \
         the assertion below rather than one of them"
        );
        assert!(
            engine.deck.slot_view(EngineSlot(DECKS as u8)).is_none(),
            "the deck answered with a view for a slot it does not have, so a cell past \
         the last slot would sample somebody else's texture"
        );

        // **Every slot is warmed first**, because a Set that has never stepped
        // draws its zeroed element state and that is black — the honest face
        // of a cold candidate, and indistinguishable at a cell's size from a
        // cell nothing drew into. What is asserted below is that a slot with
        // material in it reaches its cell whatever its residency, so the
        // material has to be there first. Live is how a slot gets it here;
        // priming is how an operator gets it without the room seeing.
        for slot in 0..DECKS {
            engine
                .deck
                .set_residency(EngineSlot(slot as u8), Residency::Live);
        }
        engine.aim(&gpu, &mut renderer, panel.layout(), 1.0, None);
        for _ in 0..4 {
            let Engine {
                deck,
                present,
                previews,
                slot_bind_groups,
                look,
                ..
            } = &mut engine;
            compose(
                &gpu,
                deck,
                present,
                &mut [],
                &mut |_, _| {},
                |_| Committed {
                    steps: STEPS_A_FRAME,
                    look: *look,
                },
                |encoder| monitor(present, previews, slot_bind_groups, encoder),
            )
            .expect("the frame composed");
        }
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");

        for residencies in [
            [
                Residency::Live,
                Residency::Priming,
                Residency::Allocated,
                Residency::Allocated,
            ],
            [Residency::Allocated; DECKS],
            [Residency::Live; DECKS],
            [Residency::Priming; DECKS],
        ] {
            for (slot, residency) in residencies.into_iter().enumerate() {
                engine.deck.set_residency(EngineSlot(slot as u8), residency);
            }
            let (_, previews) = engine.aim(&gpu, &mut renderer, panel.layout(), 1.0, None);
            for (slot, aimed) in previews.into_iter().enumerate() {
                let aimed = aimed.unwrap_or_else(|| {
                    panic!(
                        "cell {} was not aimed with the deck at {residencies:?} — a slot's \
                     own material is what an operator watches to decide whether to put \
                     it on air, so a cell that waits for its deck to be Live is dark at \
                     the one moment it is wanted (ADR-0258)",
                        deck_letter(slot as u8)
                    )
                });
                assert_eq!(
                    aimed.rect,
                    cells[slot],
                    "cell {} was aimed at a rectangle that is not its own",
                    deck_letter(slot as u8)
                );
            }
            let mut view = View::new(Room::Night);
            view.previews = previews;
            assert!(
                live(&view),
                "four running cells did not keep the loop awake at {residencies:?}"
            );

            // **And the pass is recorded, through the frame the window
            // makes.** Aimed and not drawn is the other half of the defect —
            // a texture from an earlier frame held under a live letter — so
            // the cells are cleared, one frame is composed with `monitor` in
            // the same encoder, and every cell has to come back with texels
            // in it. `compose` with no sinks still renders the deck, which is
            // `frame.rs`'s *every output off is a frame*.
            for pres in &mut engine.previews {
                clear(&gpu, &pres.target);
            }
            for slot in 0..DECKS {
                assert_eq!(
                    texels(&gpu, &engine.previews[slot].texture),
                    0,
                    "cell {} did not clear, so nothing below can tell a fresh pass from \
                 a stale one",
                    deck_letter(slot as u8)
                );
            }
            let Engine {
                deck,
                present,
                previews,
                slot_bind_groups,
                look,
                ..
            } = &mut engine;
            compose(
                &gpu,
                deck,
                present,
                &mut [],
                &mut |_, _| {},
                |_| Committed {
                    steps: STEPS_A_FRAME,
                    look: *look,
                },
                |encoder| monitor(present, previews, slot_bind_groups, encoder),
            )
            .expect("the frame composed");
            gpu.device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("poll");
            for slot in 0..DECKS {
                assert!(
                    texels(&gpu, &engine.previews[slot].texture) > 0,
                    "cell {} was aimed at {residencies:?} and nothing was drawn into it — \
                 an aimed cell that takes no pass holds whatever was in it last, \
                 under a letter that says it is live",
                    deck_letter(slot as u8)
                );
            }
        }
    }

    /// Deck A's texture is the size of its cell, a resize frees the
    /// registration it replaces, and the tally counts both textures.
    ///
    /// The picture's own test above, one cell down, and it fails the same
    /// silent ways. A preview sized from anything but its cell — the row, the
    /// region, the picture, the window — looks perfectly correct on screen,
    /// because the cell is drawn at whatever size it is and the texture fills
    /// it; it is simply four to twenty times more texels than the audition
    /// needs, per frame, for as long as the deck runs. And a
    /// `register_native_texture` with no `free_texture` beside it leaks a bind
    /// group and a sampler per remade frame.
    ///
    /// # What this test is for now, and the sentence it used to carry
    ///
    /// It used to say: *"the size a cell is remade at is the scale rather than
    /// the window ... `deck-previews` is pinned at 72 tall, so a cell is 16:9
    /// inside a fixed height and stays exactly as big at any wider window"* —
    /// and it widened the window by 400 to prove it. That is false since the
    /// bay started arranging itself, and it was false at exactly the two
    /// widths this test already used: 1440 is below the crossover and 1840 is
    /// past it, so the 400 the test widens by is the one resize that makes a
    /// cell eleven times the texels it was.
    ///
    /// So the widths stay and the claim is the other one, which is the claim
    /// worth having: a cell's texture is the size of a cell in whichever
    /// arrangement the bay is in, and a resize that changes that frees the
    /// registration it replaces. Three sizes, and each is a different way of
    /// getting it wrong:
    ///
    /// - 112 x 63 in the row, which is the cell and not the row, the
    ///   region, the picture or the window.
    /// - 252 x 142 beside the picture, which is the same rule read off a
    ///   column instead of a track — 35,784 texels against 7,056, which is
    ///   5.1x, remade once at the crossover and not once per frame of
    ///   the drag that crossed it.
    /// - and the scale, which is the display it is dragged onto rather
    ///   than the window it is in: `ScaleFactorChanged`, and
    ///   `physical(cell, scale)`.
    ///
    /// Between them they hold the cell's size against every one of the four
    /// things that can change it, and the freed tally counts every remake.
    #[test]
    fn deck_a_preview_texture_is_its_cells_size_and_a_resize_frees_the_old_one() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());

        let mut panel = Panel::new(W as f32, H as f32);
        view::rearrange(&mut panel, CANVAS);
        let row = panel.layout().find("deck-previews").expect("the row");
        assert!(
            !panel.layout().is_set_aside(row),
            "1440 is past the crossover, so the cells start beside the picture and the \
         row's own arithmetic is not what is asserted below"
        );
        let picture = physical(
            picture_rect(panel.layout(), CANVAS).expect("on screen"),
            1.0,
        );
        let cells = preview_rects(panel.layout(), CANVAS).expect("the preview row is on screen");
        let want = physical(cells[0], 1.0);
        assert_eq!(
            want,
            (112, 63),
            "the mock's own cell, at the mock's own width"
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

        // **The cell's, in both axes** — not the row's, not the picture's and
        // not the window's. The row holds four of these side by side with
        // ground between them, so a texture sized from the row is out by a
        // factor of four in one axis alone.
        assert_eq!(
            (
                engine.previews[0].texture.width(),
                engine.previews[0].texture.height()
            ),
            want
        );
        assert_eq!(engine.previews[0].size, want);
        assert_ne!(engine.previews[0].size, (W, H));
        assert_ne!(
            engine.previews[0].size, engine.picture.size,
            "deck A's texture is the picture's size, so it was sized from the wrong \
         rectangle and nothing on screen would say so"
        );
        assert!(
            engine.previews[0].size.0 * 4 < engine.picture.size.0
                && engine.previews[0].size.1 * 2 < engine.picture.size.1,
            "a preview cell is not much smaller than the picture: {:?} against {:?}",
            engine.previews[0].size,
            engine.picture.size
        );
        assert!(renderer.texture(&engine.previews[0].id).is_some());

        // **A wider window goes beside**, preserving (112, 63) at default row height.
        // **Dragging the preview row's boundary 172 up from the bay's bottom**
        // leaves the row 168 tall — the 4 of `PROGRAM_DIVIDER` is above the
        // boundary — and that is 159 of cell once `.program-body`'s 9 comes
        // off. A cell is its image and the caption band under it now, so the
        // image is 159 - 17 = **142**, and 142 at 16:9 is **252** (ADR-0239 for
        // the preserved size, and `room::size::PREVIEW_CAPTION_H` for the band
        // that was not there when this read 283 x 159). The registration it
        // replaces is freed.
        panel.set_viewport(W as f32 + 400.0, H as f32);
        panel.solve();
        let program_id = panel.layout().find("program").expect("program");
        let prog_rect = panel.layout().rect(program_id);
        panel
            .layout_mut()
            .set_divider(program_id, 0, prog_rect.y + prog_rect.h - 172.0);
        panel.solve();
        view::rearrange(&mut panel, CANVAS);
        assert!(
            panel.layout().is_set_aside(row),
            "1840 is not past the crossover, so this resize is not the one being asserted"
        );
        let beside = physical(
            preview_rects(panel.layout(), CANVAS).expect("on screen")[0],
            1.0,
        );
        assert_eq!(
            beside,
            (252, 142),
            "a cell beside the picture is not the row's image height"
        );
        let was = engine.previews[0].id;
        assert!(
            engine.previews[0].fit(&gpu, &mut renderer, beside, &mut engine.freed),
            "the cells moved beside the picture and deck A's texture was not remade, so \
         the audition is 112 x 63 texels stretched over a 252 x 142 cell"
        );
        assert_eq!(engine.previews[0].size, beside);
        assert_eq!(engine.freed, 1);
        assert!(
            renderer.texture(&was).is_none(),
            "the registration the crossover replaced is still in the atlas"
        );
        assert!(renderer.texture(&engine.previews[0].id).is_some());

        // **And a wider window inside *that* arrangement remakes nothing
        // either**, which is the sentence this test used to make about the row
        // and is true of a column for a better reason: a column is
        // `(H - 6) / 2` at 16:9, a function of the bay's **height** alone, so
        // every pixel of width past the crossover goes to the picture. A frame
        // where nothing moved remakes nothing, which is what keeps the free on
        // the resize path instead of on every frame.
        panel.set_viewport(W as f32 + 800.0, H as f32);
        view::rearrange(&mut panel, CANVAS);
        let wider = physical(
            preview_rects(panel.layout(), CANVAS).expect("on screen")[0],
            1.0,
        );
        assert_eq!(wider, beside, "a wider window changed the size of a cell");
        assert!(!engine.previews[0].fit(&gpu, &mut renderer, wider, &mut engine.freed));
        assert_eq!(engine.freed, 1);

        // A display of a different scale is what changes it next.
        let was = engine.previews[0].id;
        let retina = physical(
            preview_rects(panel.layout(), CANVAS).expect("on screen")[0],
            2.0,
        );
        assert_eq!(retina, (beside.0 * 2, beside.1 * 2));
        assert!(
            engine.previews[0].fit(&gpu, &mut renderer, retina, &mut engine.freed),
            "a cell that changed size did not remake the texture"
        );
        assert_eq!(engine.previews[0].size, retina);
        assert_eq!(
            (
                engine.previews[0].texture.width(),
                engine.previews[0].texture.height()
            ),
            retina
        );
        assert_ne!(engine.previews[0].id, was);
        assert!(renderer.texture(&engine.previews[0].id).is_some());
        assert!(
            renderer.texture(&was).is_none(),
            "the registration the resize replaced is still in the atlas, so the atlas \
         grows once per remade frame"
        );
        assert_eq!(engine.freed, 2);

        // **The tally is the whole engine's, over both textures.** Fitting the
        // picture as well takes it to three: a count kept per texture would
        // read two here, and `mod gpu` would be asserting on half the leak.
        assert!(engine.picture.fit(
            &gpu,
            &mut renderer,
            (picture.0 + 40, picture.1),
            &mut engine.freed
        ));
        assert_eq!(
            engine.freed, 3,
            "the freed tally did not count both textures"
        );
    }
}
