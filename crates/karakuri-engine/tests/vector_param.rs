//! A `vec3` parameter, from the declaration to the texel.
//!
//! **A parameter is driven one component at a time**
//! ([ADR-0268](../../../docs/adr/0268-a-vector-parameter-is-driven-one-component-at-a-time.md)):
//! the uniform stays one `vec3<f32>` field and the engine holds one `f32` per
//! component, under `glow.x`, `glow.y` and `glow.z`. Every claim here is one
//! that could only be checked at the far end of the pipe.
//!
//! - **The declared default reaches the shader as those three numbers.** It
//!   reached it as zeroes: `Param::default_scalar` folds a scalar, so a vector
//!   never entered a node's value map and `node::write_params` packed
//!   `[0.0; 3]`. Nothing short of a rendered texel says the three numbers made
//!   it — the map is checked without a device in `set.rs`'s own tests, and a
//!   map that is right and a packer that ignores it look the same there.
//! - **A write lands on one component and leaves the others.** That is the
//!   whole of what the spelling buys, and it is a claim about two numbers not
//!   moving, which needs the frame.
//! - **The published interface is the components, in `x`, `y`, `z` order.** The
//!   order is an address — a MIDI control is learned against a position in this
//!   list — so it is asserted on a built Set rather than on the key list a
//!   procedure hands over.

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
mod gpu {
    use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
    use karakuri_ir::typed::Checked;

    const W: u32 = 64;
    const H: u32 = 64;

    /// The smallest L1 that builds: a fullscreen L4 consumes nothing, so this
    /// exists only to be the geometry half of a pair.
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

    /// **A fullscreen L4 whose colour *is* the parameter**, so the readback is
    /// the three numbers and not a function of them. `blend additive` over a
    /// cleared target and one draw means the texel is what the fragment wrote.
    ///
    /// The default is `docs/ir-spec.md`'s own `param` example, which is the
    /// declaration this whole change is about: the spec advertised it while
    /// every reader discarded it.
    const GLOW: &str = r#"
proc glowing {
  kind  L4
  blend additive

  param glow : vec3 [0.0, 4.0] = vec3(0.4, 0.7, 1.0)

  fragment {
    color = vec4(glow, 1.0);
  }
}
"#;

    /// Declaration order and `x`, `y`, `z` order are different at every scale
    /// here: `wash` is a `vec2` before `depth`, which is a scalar, and `glow`
    /// is a `vec3` after it. Nothing about the expected list can be produced by
    /// sorting, by taking declarations in the order they appear, or by
    /// flattening one node's parameters into anything but this.
    const MIXED: &str = r#"
proc mixed {
  kind  L4
  blend additive

  param wash  : vec2  [0.0, 1.0] = vec2(0.1, 0.2)
  param depth : float [0.0, 1.0] = 0.3
  param glow  : vec3  [0.0, 4.0] = vec3(0.4, 0.7, 1.0)

  fragment {
    color = vec4(glow * depth + vec3(wash, 0.0), 1.0);
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

    fn build(gpu: &Gpu, l4: &str) -> Set {
        let mut set = Set::build(
            &gpu.device,
            &gpu.queue,
            &compile(DOTS),
            &compile(l4),
            4,
            19274,
        )
        .expect("a compatible pair");
        set.resize(&gpu.device, W, H);
        set
    }

    /// The middle texel of one frame, as RGBA.
    fn texel(gpu: &Gpu, set: &mut Set) -> [f32; 4] {
        let present = Present::new(&gpu.device, wgpu::TextureFormat::Rgba16Float, W, H);
        set.prepare(&gpu.queue, 1, &Signals::default());

        let bytes_per_row = W * 8;
        assert_eq!(bytes_per_row % 256, 0, "readback rows must stay aligned");
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(bytes_per_row * H),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        set.render(&mut encoder, present.hdr_view(), 1);
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
    /// the same way `tests/fullscreen.rs` and `tests/deck.rs` do it.
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
    /// the declared one to about a thousandth. Wide enough that half precision
    /// cannot fail it and far narrower than the gap between any of these and
    /// the zero this used to write.
    fn close(got: f32, want: f32, what: &str) {
        assert!(
            (got - want).abs() < 1e-3,
            "{what}: expected {want}, read {got} back out of the frame"
        );
    }

    /// **The declared default, in the frame.**
    ///
    /// Before ADR-0268 this read `[0.0, 0.0, 0.0]` — the packer wrote
    /// `[0.0; 3]` for every vector param, because no fold put one in the value
    /// map. The three numbers were in the `.kir` the whole time.
    #[test]
    fn a_vector_params_declared_default_reaches_the_shader() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut set = build(&gpu, GLOW);
        let [r, g, b, _] = texel(&gpu, &mut set);
        close(r, 0.4, "glow.x");
        close(g, 0.7, "glow.y");
        close(b, 1.0, "glow.z");
    }

    /// **One component moves and the other two stay**, which is what an
    /// address per component is for and is the half a wider value channel
    /// would not have bought: `--param glow.y=0.7` is one number written at one
    /// key, and the value the frame is packed from still has the other two.
    ///
    /// `set_param` is the bare-name route a `--param` and a `param` record both
    /// come through, so the key here is exactly what the command line spells.
    #[test]
    fn a_write_lands_on_one_component_and_leaves_the_others() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut set = build(&gpu, GLOW);
        assert_eq!(
            set.set_param("glow.y", 2.5),
            1,
            "`glow.y` reached no declaration"
        );
        assert_eq!(
            set.set_param("glow", 2.5),
            0,
            "the bare name must address nothing: it names three numbers and carries one"
        );
        let [r, g, b, _] = texel(&gpu, &mut set);
        close(r, 0.4, "glow.x moved and nothing asked it to");
        close(g, 2.5, "glow.y");
        close(b, 1.0, "glow.z moved and nothing asked it to");
    }

    /// **The published interface is one control per component, in declaration
    /// order and `x`, `y`, `z` within a declaration.**
    ///
    /// The order is an address: `docs/manual/console.html`, *"A MIDI control is
    /// learned against the deck and the position in its published interface"*,
    /// and `crates/karakuri/src/main.rs` turns that position into the row's
    /// ordinal. So this asserts the whole list in order rather than membership.
    ///
    /// **The built-in camera's three publish first.** Every `Set` carries one
    /// whether or not `MIXED` declares an L3 procedure of its own
    /// ([ADR-0318](../../../docs/adr/0318-the-built-in-cameras-three-placement-numbers-are-parameter-rows.md)),
    /// and `Kind::ALL` walks L3 before L4, so `radius`, `speed` and `height`
    /// sit ahead of `MIXED`'s own components rather than beside them.
    #[test]
    fn the_published_interface_is_the_components_in_order() {
        let gpu = Gpu::headless().expect("no GPU available");
        let set = build(&gpu, MIXED);
        let names: Vec<String> = set.published().into_iter().map(|p| p.name).collect();
        assert_eq!(
            names,
            vec![
                "radius", "speed", "height", "wash.x", "wash.y", "depth", "glow.x", "glow.y",
                "glow.z"
            ],
            "the published order is what a knob is learned against"
        );
        // And each component carries the declaration's own range, which is what
        // a fader's ends are — the camera's three from `Orbit::PLACEMENT`,
        // `MIXED`'s own from its declarations above.
        for control in set.published() {
            let want = match control.name.as_str() {
                "radius" => [1.0, 40.0],
                "speed" => [0.0, 2.0],
                "height" => [-40.0, 40.0],
                name if name.starts_with("glow") => [0.0, 4.0],
                _ => [0.0, 1.0],
            };
            assert_eq!(control.range, want, "{} has the wrong range", control.name);
        }
    }
}
