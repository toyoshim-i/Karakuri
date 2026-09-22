use super::common::*;

mod gpu {
    use super::*;

    /// The whole loop, closed on the look: a press on the tone map capsule chooses
    /// the next operator and keeps the level, and a press on the exposure track
    /// sets the level and keeps the operator.
    ///
    /// `tests/look.rs` asserts everything up to the operation with no engine
    /// anywhere, which is the point of that file. This is the other end, and it
    /// needs a device because [`Engine`] does — and because the value being moved
    /// is [`Engine::look`], which is what every sink is drawn under.
    ///
    /// What separates this from the plausible wrong answer is the third of the
    /// record neither press names. `Record::Look` is an operator, a level and a
    /// white point; each control asks for one of the first two and [`reading`]
    /// supplies the rest (ADR-0192). A build that filled the missing thirds from a
    /// default would cycle the tone map and silently reset the exposure — and would
    /// rewrite `white_point`, which is on no surface at all and would therefore
    /// change with nothing saying so. So the look this starts from has none of the
    /// three at its default.
    ///
    /// The middle step is the one worth the device, as in the mask's test: between
    /// the press and the record the look must not have moved, or the console would
    /// be applying what it is only supposed to ask for.
    #[test]
    fn a_press_on_the_look_controls_moves_the_look_every_sink_is_drawn_under() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;
        /// Not `LOOK`'s three, so a press that dropped a third of the record and filled
        /// it from a default is visible in every one of them.
        const STARTS_AT: Look = Look {
            op: TonemapOp::Reinhard,
            exposure: 0.5,
            white_point: 3.5,
        };

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());
        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
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
        engine.look = STARTS_AT;

        // What the console reads this frame, off the look the engine holds.
        let ctx = super::tests::drawn_once();
        let mut view = View::new(karakuri_console::room::Room::Day);
        // The mock's own transport, which is what `tests/transport.rs` and
        // the console's own tests read: the group is measured from the
        // arrangement pill and the pill from the bar, so a row is needed to
        // have either.
        view.transport = Some(view::Transport {
            bpm: 128.0,
            beats: 144.0,
            beats_per_bar: karakuri_signal::oscillator::BEATS_PER_BAR,
            fps: Some(58.0),
            frame_ms: 12.4,
            budget_ms: Some(16.6),
            chain_ms: None,
            health: Some(view::Stage::Landed),
            rec: Some(view::Rec::Idle),
        });
        view.look = Some(look(&engine.look));
        assert_eq!(
            view.look,
            Some(view::Look {
                tonemap: karakuri_operation::Tonemap::Reinhard,
                exposure: 0.5,
            }),
            "the console is not reading the look the engine is drawing under"
        );

        let row = look_row(
            &ctx,
            panel.layout(),
            view.transport,
            view.audio.as_ref(),
            view.tracker,
            None,
            &view.arrangement,
            view.look,
        )
        .expect("the transport row draws the look controls");

        // ---- the capsule: the next operator, at the level that is running --
        let capsule = row.tone.center();
        let operation = row
            .tonemap(Point::new(capsule.x, capsule.y))
            .expect("a press on the tone map capsule");
        assert_eq!(
            operation,
            Operation::SetTonemap {
                tonemap: karakuri_operation::Tonemap::Aces,
            },
            "the press did not ask for the operator after `reinhard`"
        );
        // **Nothing has been told anything yet.**
        assert_eq!(
            engine.look, STARTS_AT,
            "the look moved before the record did"
        );

        // **The transition settings are where a run begins and this press
        // does not read them**: `Current::transition` is a wipe's, a fade's, a
        // crossfade's and a selection's, and none of the three conversions
        // below is one of those. Handed in because `reading` takes them, and
        // `START` rather than a chosen value so that nothing here can look
        // like a setting the test needed.
        let chosen = written(
            &operation,
            &reading(
                &operation,
                &engine.deck,
                &engine.look,
                &engine.chain,
                TransitionSettings::START,
                &karakuri_pattern::Banks::default(),
            ),
        );
        let Written::Records(records) = &chosen else {
            panic!("a press on the tone map capsule wrote no record: {chosen:?}")
        };
        assert_eq!(
            records.as_slice(),
            [Record::Look {
                op: "aces".to_owned(),
                exposure: 0.5,
                white_point: 3.5,
            }],
            "the record is not the whole look with only the operator changed"
        );
        assert!(apply(
            &records[0],
            &mut engine.deck,
            &mut engine.look,
            &mut engine.chain
        )
        .is_some());
        assert_eq!(
            engine.look,
            Look {
                op: TonemapOp::Aces,
                ..STARTS_AT
            },
            "cycling the tone map did not leave the level and the white point alone"
        );

        // ---- the track: the level under the press, at the operator running -
        let row = look_row(
            &ctx,
            panel.layout(),
            view.transport,
            view.audio.as_ref(),
            view.tracker,
            None,
            &view.arrangement,
            Some(look(&engine.look)),
        )
        .expect("the group is still drawn");
        let middle = row.grip.center();
        let operation = row
            .exposure(Point::new(middle.x, middle.y))
            .expect("a press on the exposure track");
        assert_eq!(
            operation,
            Operation::SetExposure { exposure: 1.0 },
            "a press at the middle of the track did not ask for unity"
        );

        // **The transition settings are where a run begins and this press
        // does not read them**: `Current::transition` is a wipe's, a fade's, a
        // crossfade's and a selection's, and none of the three conversions
        // below is one of those. Handed in because `reading` takes them, and
        // `START` rather than a chosen value so that nothing here can look
        // like a setting the test needed.
        let levelled = written(
            &operation,
            &reading(
                &operation,
                &engine.deck,
                &engine.look,
                &engine.chain,
                TransitionSettings::START,
                &karakuri_pattern::Banks::default(),
            ),
        );
        let Written::Records(records) = &levelled else {
            panic!("a press on the exposure track wrote no record: {levelled:?}")
        };
        assert_eq!(
            records.as_slice(),
            [Record::Look {
                op: "aces".to_owned(),
                exposure: 1.0,
                white_point: 3.5,
            }],
            "the record is not the whole look with only the level changed — the operator the \
         press cannot name, or the white point no surface can, was rewritten"
        );
        assert!(apply(
            &records[0],
            &mut engine.deck,
            &mut engine.look,
            &mut engine.chain
        )
        .is_some());
        assert_eq!(
            engine.look,
            Look {
                op: TonemapOp::Aces,
                exposure: 1.0,
                white_point: 3.5,
            },
            "the level did not land, or it took the operator or the white point with it"
        );

        // And the console follows, because it is read off the engine rather
        // than remembered.
        assert_eq!(
            look(&engine.look),
            view::Look {
                tonemap: karakuri_operation::Tonemap::Aces,
                exposure: 1.0,
            }
        );
    }

    /// Which rectangle each sink's texture is sized from, and where the console
    /// then draws it — asked of the call the frame actually makes.
    ///
    /// This is the hole `docs/roadmap.md` recorded, closed. The decision used to be
    /// two `match`es inside `App::window_event`, and `winit` will not hand a test
    /// an `ActiveEventLoop`, so nothing could call it: `mod gpu` asserted what
    /// `Engine::new` did and not what the frame chose. Sizing deck A's texture from
    /// the picture's rectangle was injected there and every test still passed. It
    /// is [`aims`] and [`Engine::aim`] now, which take a solved layout and a scale
    /// factor and touch no window, and this asks them at a viewport and a scale
    /// neither of which `Engine::new` was given — so what is asserted is what `aim`
    /// decided rather than what construction left behind.
    ///
    /// Every half of it fails silently. A texture sized from the wrong rectangle
    /// looks perfectly correct — the cell is drawn at whatever size it is and the
    /// texture fills it — and is four to twenty times the texels the cell needs,
    /// per frame, for as long as the deck runs. A `Picture` carrying an id from
    /// before a resize is a freed registration, which `egui` draws as nothing at
    /// all. And a folded region whose sink still acquires is the manual's *"no
    /// state where it is hidden and still costing a pass"* quietly stopping being
    /// true.
    #[test]
    fn the_frame_aims_each_sink_at_its_own_rectangle() {
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const W: u32 = 1440;
        const H: u32 = 900;
        /// A display of a different scale, because the size is the rectangle and the
        /// scale and a test at 1.0 cannot tell them apart.
        const SCALE: f32 = 2.0;

        let gpu = Gpu::headless().expect("no GPU");
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());

        let mut panel = Panel::new(W as f32, H as f32);
        panel.solve();
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

        // A different window on a different display, so nothing asserted below
        // can be what construction happened to leave in place — and at 1760
        // wide it is a window past the crossover, so what is asserted below is
        // the arrangement with the four cells **beside** the picture. The
        // rearrangement is the frame's own first act and this test makes it in
        // the same order.
        panel.set_viewport(W as f32 + 320.0, H as f32 - 120.0);
        view::rearrange(&mut panel, CANVAS);
        assert!(
            panel
                .layout()
                .is_set_aside(panel.layout().find("deck-previews").expect("the row")),
            "1760 is not past the crossover, so the cells are still in the row"
        );
        let rect = picture_rect(panel.layout(), CANVAS).expect("the picture is on screen");
        let cell = preview_rects(panel.layout(), CANVAS).expect("the preview row is on screen")[0];
        let (picture, previews) = engine.aim(&gpu, &mut renderer, panel.layout(), SCALE, None);

        // **Each texture is the size of its own rectangle, at this scale.**
        assert_eq!(
            engine.picture.size,
            physical(rect, SCALE),
            "the picture's texture is not the size of the picture's region"
        );
        assert_eq!(
            engine.previews[0].size,
            physical(cell, SCALE),
            "deck A's texture is not the size of deck A's cell — it was sized from some \
         other rectangle, and nothing on screen would say so"
        );
        assert_ne!(
            engine.previews[0].size, engine.picture.size,
            "deck A's texture is the picture's size"
        );
        // **Half the picture in each direction, and it is half rather than the
        // quarter this used to ask for.** A quarter of the width was the row's
        // arithmetic — four tracks across the bay — and beside the picture a
        // cell is half a column, so it is about half the picture each way and
        // a quarter of its texels. The claim being made is the one that
        // catches the defect either way: a cell sized from the picture's
        // rectangle, or from the window, is *larger* than this and not
        // smaller.
        assert!(
            engine.previews[0].size.0 * 2 <= engine.picture.size.0
                && engine.previews[0].size.1 * 2 <= engine.picture.size.1,
            "a preview cell is not much smaller than the picture: {:?} against {:?}",
            engine.previews[0].size,
            engine.picture.size
        );

        // **And where the console draws it is the same statement**: the
        // rectangle the texture was just sized from, and the id the sizing may
        // have just replaced.
        let drawn = picture.expect("the picture is on screen and the frame aimed nothing at it");
        assert_eq!(
            drawn.rect, rect,
            "the picture is drawn somewhere other than the region \
         its texture was sized from"
        );
        assert_eq!(
            drawn.id, engine.picture.id,
            "the view carries the id from before the resize, which is a freed registration"
        );
        assert!(renderer.texture(&drawn.id).is_some());
        let monitored = previews[0].expect("deck A has a slot and the frame aimed nothing at it");
        assert_eq!(monitored.rect, cell);
        assert_eq!(monitored.id, engine.previews[0].id);
        assert!(renderer.texture(&monitored.id).is_some());
        // **All four, and residency has nothing to do with it.** Deck A is the
        // only Live slot on this engine and the other three rest at
        // `Allocated`; every one of them is drawn into its own target and
        // every one of them is aimed at a cell, which is ADR-0258 —
        // `every_cell_with_a_slot_behind_it_is_aimed_whatever_its_residency`
        // is where that is asserted across all four residency arrangements.
        assert!(
            previews.iter().all(Option::is_some),
            "a cell with a deck slot behind it was not aimed: an operator watches a \
         candidate's cell to decide whether to put it on air, so a cell that waits \
         for Live is dark at the one moment it is wanted"
        );

        // **Aimed is what `Sink::acquire` answers from**, and that is the
        // whole of what `compose` asks either of them.
        assert_eq!(engine.picture.acquire(&gpu), Ok(()));
        assert_eq!(engine.previews[0].acquire(&gpu), Ok(()));

        // **Fold the picture away and its sink has no target** — so `compose`
        // records no present pass into it, the deck still advances, and the
        // four cells go on monitoring underneath. Both halves matter: a fold
        // that took the cells with it is the console going dark from one
        // keystroke.
        let picture_node = panel.layout().find("program-view").expect("program-view");
        assert!(
            matches!(
                panel.op(Op::Fold(picture_node)),
                Outcome::Folded { folded: true, .. }
            ),
            "the picture did not fold"
        );
        // **The bay rearranges around the fold, and this is the guard rule
        // reached through the frame's own call.** The cells were beside the
        // picture; with the picture gone the row comes back under it, because
        // a bay whose only laid-out child is set aside can use nothing at all
        // and would claim no height. Reading a rectangle without this is
        // reading one from before the fold — the same contract `Layout::rect`
        // has about a stale solve.
        view::rearrange(&mut panel, CANVAS);
        assert!(picture_rect(panel.layout(), CANVAS).is_none());
        assert!(
            !panel
                .layout()
                .is_set_aside(panel.layout().find("deck-previews").expect("the row")),
            "the picture is folded and the row is still set aside, so the Program bay \
         claims nothing and has gone from the panel"
        );
        let (picture, previews) = engine.aim(&gpu, &mut renderer, panel.layout(), SCALE, None);
        assert!(
            picture.is_none(),
            "the picture is folded away and the frame still gave the console one to draw"
        );
        assert_eq!(
            engine.picture.acquire(&gpu),
            Err(Skip::Transient),
            "the picture is folded away and its sink still took the frame, so a present \
         pass is recorded into a texture nothing shows"
        );
        assert!(
            previews.iter().all(Option::is_some),
            "folding the picture away stopped the cells monitoring under it"
        );
        assert_eq!(
            engine.previews[0].acquire(&gpu),
            Ok(()),
            "folding the picture away stopped deck A's cell taking the frame"
        );
    }

    /// ADR-0155's bet, as an assertion.
    ///
    /// The record chose `egui` and paid a `wgpu` major version for it on the
    /// grounds that the panel and the engine share one `Device`. This builds the
    /// console's frame with an `egui` context, tessellates it, renders it through
    /// `egui-wgpu` into a texture on a device `karakuri-engine` created, and reads
    /// the texels back. If the two ever resolve different `wgpu`s it does not
    /// compile; if the render path breaks, `poll` gives a device-side complaint
    /// somewhere to surface; and if what lands is not the console, the two pixels
    /// below say so.
    ///
    /// The pixels are the point. A frame that renders without complaining and is
    /// the wrong colour is the failure that is easy to ship: `egui`'s shader writes
    /// gamma-encoded texels because it is told the target is gamma space, so an
    /// sRGB target encodes a second time and the whole panel washes out — with no
    /// error anywhere. So a bay's body is asserted to be exactly `--c-panel` and a
    /// divider is asserted not to be.
    #[test]
    fn egui_paints_the_console_onto_a_device() {
        // Gamma space, not sRGB: see above, and the window's own choice of
        // surface format, which is made for this reason.
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        // 1408 rather than 1440 for one reason: `copy_texture_to_buffer` wants
        // `bytes_per_row` a multiple of 256, and 1408 * 4 is 5632.
        const W: u32 = 1408;
        const H: u32 = 900;
        const ROOM: Room = Room::Night;

        let gpu = Gpu::headless().expect("no GPU");
        let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("console probe"),
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

        let ctx = egui::Context::default();
        let mut panel = Panel::new(W as f32, H as f32);
        let mut view = View::new(ROOM);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(W as f32, H as f32),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| view.draw(ui, &mut panel));
        let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
        // The console has thirteen regions and seven headings, so a frame that
        // tessellated to nothing is a frame that drew nothing.
        assert!(
            !primitives.is_empty(),
            "the console tessellated to no primitives at all"
        );

        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [W, H],
            pixels_per_point: 1.0,
        };
        for (id, deltas) in &output.textures_delta.set {
            for delta in deltas {
                renderer.update_texture(&gpu.device, &gpu.queue, *id, delta);
            }
        }
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("console probe"),
            });
        let user =
            renderer.update_buffers(&gpu.device, &gpu.queue, &mut encoder, &primitives, &screen);
        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("console probe"),
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
        for id in &output.textures_delta.free {
            renderer.free_texture(id);
        }
        output.textures_delta.clear();

        let row = W * 4;
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("console probe"),
            size: (row * H) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
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
        gpu.queue.submit(user.into_iter().chain([encoder.finish()]));
        readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the device stopped");
        let texels = readback.slice(..).get_mapped_range().expect("readback");
        let at = |p: karakuri_layout::Point| {
            let i = (p.y as u32 * row + p.x as u32 * 4) as usize;
            [texels[i], texels[i + 1], texels[i + 2]]
        };

        let pal = ROOM.palette();
        let panel_rgb = [pal.panel.r(), pal.panel.g(), pal.panel.b()];

        // A bay's body, well clear of its head and its edges.
        panel.solve();
        let library = panel
            .layout()
            .rect(panel.layout().find("library").expect("library"));
        let inside =
            karakuri_layout::Point::new(library.x + library.w * 0.5, library.y + library.h * 0.5);
        assert_eq!(
            at(inside),
            panel_rgb,
            "an empty bay's body is not --c-panel; the colour space or the palette is wrong"
        );

        // A divider: the ground shows through. Not `--c-ground` exactly, and
        // that is right rather than a tolerance — the bays either side cast
        // their shadow into the gap, as they do in the mock. So the assertion
        // is which of the two colours it is nearer, which is the question
        // "does the ground show through" and is not a threshold anybody has to
        // tune.
        let ground_rgb = [pal.ground.r(), pal.ground.g(), pal.ground.b()];
        let (split, index) = panel.layout().boundaries().next().expect("no boundary");
        let gap = panel.layout().boundary(split, index).expect("no pair");
        let in_gap = karakuri_layout::Point::new(gap.x + gap.w * 0.5, gap.y + gap.h * 0.5);
        let found = at(in_gap);
        let away = |from: [u8; 3]| -> i32 {
            (0..3)
                .map(|i| (found[i] as i32 - from[i] as i32).abs())
                .sum()
        };
        assert!(
            away(ground_rgb) < away(panel_rgb),
            "a divider at {found:?} is nearer the bay {panel_rgb:?} than the ground \
         {ground_rgb:?}, so the ground is not showing through"
        );
    }
}
