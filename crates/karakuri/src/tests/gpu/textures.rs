use super::common::*;

mod gpu {
    use super::*;

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
