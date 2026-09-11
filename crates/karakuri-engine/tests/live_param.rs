//! A parameter written to a deck slot that is playing, from the call to the
//! texel.
//!
//! **The claim is that a knob is not a build.** `Set::write_param` writes a
//! number into a map and touches no shader and no pipeline; `Set::prepare`
//! packs that map into a uniform on every frame of every slot. So a write
//! reaches the frame after the one it was made in, with no worker, no
//! candidate and no swap — and the only thing that says so at the far end is a
//! rendered texel, because a map that is right and a slot that keeps rendering
//! the Set it was built with look identical from anywhere else.
//!
//! **And that the route in is a `Deck`.** `HotSwap::live_mut` is `pub(crate)`
//! because handing out the live `Set` would be a second way to *render* one;
//! writing a value is neither rendering nor replacement, and
//! `Deck::schedule_selection` already reaches `live_mut` on exactly those
//! terms. What is asserted here is the public road: nothing in this file names
//! a `Set` after the deck is built.
//!
//! What is *not* here is a rebuild carrying the value over. Nothing does that,
//! and `docs/adr/0280-a-parameter-written-to-a-live-set-is-a-session-record.md`
//! says why it is a decision rather than a gap.

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
mod gpu {
    use karakuri_engine::swap::HotSwap;
    use karakuri_engine::{Authority, Deck, Gpu, ParamWrite, Present, Set};
    use karakuri_ir::typed::Checked;
    use karakuri_ir::Kind;

    const W: u32 = 64;
    const H: u32 = 64;

    /// The smallest L1 that builds — the geometry half of a pair and nothing
    /// else. It declares no parameter, so a wildcard here lands on the
    /// renderer alone.
    const DOTS: &str = r#"
proc dots {
  kind     L1
  topology points
  capacity [4, 4] = 4

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#;

    /// **An L1 that declares `heat`**, so that a bare name reaches two nodes on
    /// two layers and the refusal has something to refuse.
    const HOT_DOTS: &str = r#"
proc hot_dots {
  kind     L1
  topology points
  capacity [4, 4] = 4

  param heat : float [0.0, 4.0] = 1.0

  emit position

  element {
    position = vec3(heat * 0.0, 0.0, 0.0);
  }
}
"#;

    /// **A fullscreen L4 whose colour *is* its parameters**, so the readback is
    /// the numbers themselves rather than a function of them. One scalar and
    /// one vector, because a component key is an address like any other and a
    /// write must land on one component and leave the rest.
    const GLOW: &str = r#"
proc glowing {
  kind  L4
  blend additive

  param heat : float [0.0, 4.0] = 1.0
  param glow : vec3  [0.0, 4.0] = vec3(0.4, 0.7, 1.0)

  fragment {
    color = vec4(glow * heat, 1.0);
  }
}
"#;

    fn compile(src: &str) -> Checked {
        let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("{}", render(&e, src)));
        karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{}", render(&e, src)))
    }

    fn render(errs: &[karakuri_ir::IrError], src: &str) -> String {
        errs.iter()
            .map(|e| e.render(src))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn set_of(gpu: &Gpu, l1: &str) -> Set {
        let mut set = Set::build(
            &gpu.device,
            &gpu.queue,
            &compile(l1),
            &compile(GLOW),
            4,
            19274,
        )
        .expect("a compatible pair");
        set.resize(&gpu.device, W, H);
        set
    }

    /// A deck of one fixed Set — **no worker**, so nothing in these tests can
    /// swap and a value that changed cannot have been carried in by a build.
    /// That is the whole control on the claim.
    fn deck_of(gpu: &Gpu, set: Set) -> Deck {
        Deck::new(&gpu.device, vec![HotSwap::fixed(set)], W, H)
    }

    /// The middle texel of one composited frame, as RGBA.
    fn texel(gpu: &Gpu, deck: &mut Deck, present: &Present) -> [f32; 4] {
        let mut frame = deck.begin_frame(&gpu.device, &gpu.queue);
        frame.render(present.hdr_view(), present.size(), 1);
        frame.finish();

        let bytes_per_row = W * 8;
        assert_eq!(bytes_per_row % 256, 0, "readback rows must stay aligned");
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(bytes_per_row * H),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            present.hdr_texture().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(H),
                },
            },
            wgpu::Extent3d {
                width: W,
                height: H,
                depth_or_array_layers: 1,
            },
        );
        gpu.queue.submit([encoder.finish()]);

        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let data = slice.get_mapped_range().expect("map");
        let at = ((H / 2 * W + W / 2) * 8) as usize;
        let h = |i: usize| f16(u16::from_le_bytes([data[at + i], data[at + i + 1]]));
        let out = [h(0), h(2), h(4), h(6)];
        drop(data);
        readback.unmap();
        out
    }

    /// `f16` bits to `f32`. Written out rather than pulled in as a dependency,
    /// the way `tests/vector_param.rs` and `tests/deck.rs` do it.
    fn f16(bits: u16) -> f32 {
        let sign = if bits & 0x8000 != 0 { -1.0 } else { 1.0 };
        let exponent = (bits >> 10) & 0x1f;
        let mantissa = bits & 0x03ff;
        let magnitude = match exponent {
            0 => f32::from(mantissa) * 2.0f32.powi(-24),
            0x1f if mantissa == 0 => f32::INFINITY,
            0x1f => f32::NAN,
            e => (1.0 + f32::from(mantissa) / 1024.0) * 2.0f32.powi(i32::from(e) - 15),
        };
        sign * magnitude
    }

    /// The target is `Rgba16Float`, so a number that survived the round trip is
    /// the one that was written to about a thousandth — far narrower than any
    /// gap this file asserts.
    fn close(got: f32, want: f32, what: &str) {
        assert!(
            (got - want).abs() < 1e-3,
            "{what}: expected {want}, read {got} back out of the frame"
        );
    }

    /// **A write through the deck is on screen at the next frame, and no build
    /// happened.**
    ///
    /// The frame before the write is asserted too, so that a test which
    /// happened to draw the new value from the start could not pass: the pair
    /// of readings is the claim, and the only thing between them is one call.
    #[test]
    fn a_write_through_the_deck_reaches_the_live_sets_uniform_without_a_rebuild() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, wgpu::TextureFormat::Rgba16Float, W, H);
        let mut deck = deck_of(&gpu, set_of(&gpu, DOTS));

        let [r, g, b, _] = texel(&gpu, &mut deck, &present);
        close(r, 0.4, "glow.x before the write");
        close(g, 0.7, "glow.y before the write");
        close(b, 1.0, "glow.z before the write");

        assert_eq!(
            deck.write_param(
                karakuri_engine::DeckSlot(0),
                &ParamWrite::everywhere("glow.y", 2.5)
            ),
            Ok(1),
            "`glow.y` reached no declaration through the deck"
        );
        let frames = deck.slot(karakuri_engine::DeckSlot(0)).frames_rendered();

        let [r, g, b, _] = texel(&gpu, &mut deck, &present);
        close(r, 0.4, "glow.x moved and nothing asked it to");
        close(g, 2.5, "glow.y");
        close(b, 1.0, "glow.z moved and nothing asked it to");

        assert_eq!(
            deck.slot(karakuri_engine::DeckSlot(0)).frames_rendered(),
            frames + 1,
            "one frame was drawn between the two readings, and the value moved in it"
        );
        assert!(
            deck.slot(karakuri_engine::DeckSlot(0))
                .measured_cost()
                .is_none(),
            "a fixed slot has no worker: nothing here can have been built"
        );
    }

    /// **An addressed write names one node**, which is what makes a component
    /// key an address rather than a suggestion — and what a published control
    /// on one renderer of several needs.
    #[test]
    fn an_addressed_write_names_one_node() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, wgpu::TextureFormat::Rgba16Float, W, H);
        let mut deck = deck_of(&gpu, set_of(&gpu, DOTS));

        assert_eq!(
            deck.write_param(
                karakuri_engine::DeckSlot(0),
                &ParamWrite::at(Kind::L4, 0, "heat", 2.0)
            ),
            Ok(1),
            "`L4:0:heat` reached no declaration"
        );
        assert_eq!(
            deck.write_param(
                karakuri_engine::DeckSlot(0),
                &ParamWrite::at(Kind::L4, 1, "heat", 3.0)
            ),
            Ok(0),
            "there is no second renderer, and a write that landed on one would \
             be writing past the layer"
        );

        let [r, g, b, _] = texel(&gpu, &mut deck, &present);
        close(r, 0.8, "glow.x at heat 2.0");
        close(g, 1.4, "glow.y at heat 2.0");
        close(b, 2.0, "glow.z at heat 2.0");
    }

    /// **The wildcard refusal survives the deck**, which is the point of
    /// reaching `Set::write_param` rather than reaching past it: a bare name
    /// over nodes that are not under one authority is one control spanning two
    /// arrangements, and it is refused whole rather than landed on the half
    /// that permits it
    /// (`docs/adr/0223-a-wildcard-write-is-refused-where-the-nodes-it-lands-on-disagree.md`).
    ///
    /// The frame is read afterwards because *refused whole* is a claim about
    /// what did **not** move: a refusal that had already written the renderer
    /// would return the same `Err`.
    #[test]
    fn a_wildcard_write_is_refused_through_the_deck_where_the_landing_disagrees() {
        let gpu = Gpu::headless().expect("no GPU available");
        let present = Present::new(&gpu.device, wgpu::TextureFormat::Rgba16Float, W, H);

        let mut set = set_of(&gpu, HOT_DOTS);
        assert!(
            set.set_authority(Kind::L4, 0, Authority::Automatic),
            "the renderer is node 0 of L4 in every Set here"
        );
        let mut deck = deck_of(&gpu, set);

        let refused = deck
            .write_param(
                karakuri_engine::DeckSlot(0),
                &ParamWrite::everywhere("heat", 3.0),
            )
            .expect_err("`heat` is declared under two authorities and a bare name reaches both");
        assert_eq!(refused.key, "heat");
        assert!(
            refused.to_string().contains("L1:0")
                && refused.to_string().contains("L4:0")
                && refused.to_string().contains("automatic"),
            "the refusal has to name the nodes it landed on and what they are under, \
             and it said: {refused}"
        );

        // The addressed form is never refused — it says which node it means —
        // so this is the way out the sentence above offers, taken.
        assert_eq!(
            deck.write_param(
                karakuri_engine::DeckSlot(0),
                &ParamWrite::at(Kind::L4, 0, "heat", 3.0)
            ),
            Ok(1),
            "an addressed write is not checked against authority and must land"
        );
        let [r, _, _, _] = texel(&gpu, &mut deck, &present);
        close(
            r,
            1.2,
            "glow.x at heat 3.0, which only the addressed write set",
        );
    }
}
