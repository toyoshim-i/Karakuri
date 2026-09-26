use super::common::*;

mod gpu {
    use super::*;

    /// Verifies horizontal window resizing retains texture allocation, while vertical resizing
    /// adjusts texture size and frees previous GPU native texture registrations.
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

        // Horizontal window expansion without picture resizing avoids recreating textures.
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

        // Dragging the program bottom edge resizes the picture texture.
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

    /// Verifies every valid slot aims its preview cell regardless of residency, while unpopulated
    /// slots remain blank (ADR-0258).
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

        // Out-of-range deck slots yield no texture view.
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

        // Warms slot state to ensure active render passes produce non-empty texels.
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

            // Records and submits composed frames to ensure all aimed preview cells receive rendered texels.
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

    /// Verifies preview cell texture sizing accurately tracks bay layout arrangements and scale changes,
    /// releasing replaced registrations upon resize.
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

        // Preview texture dimensions match the individual cell bounds in both axes.
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

        // Boundary drag adjusts preview row height and recalculates 16:9 cell image dimensions (ADR-0239).
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

        // Width expansion beyond column height constraint does not reallocate preview textures.
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

        // Freed texture tally aggregates across all engine sinks.
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
