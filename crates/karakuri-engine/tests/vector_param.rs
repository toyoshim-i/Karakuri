//! Integration tests for vector parameters driven as individual scalar components or atomic values.

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
#[path = "common/mod.rs"]
mod common;

mod gpu {
    pub(super) use super::common::{compile, f16};
    use karakuri_engine::{Gpu, Layer, NodeAddress, Present, Set, Signals, Value, VideoSource};

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

    /// Fullscreen L4 whose color directly outputs the `glow` vec3 parameter.
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

    // Procedure with mixed vec2, scalar, and vec3 parameters to verify ordering.
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
        set.commit();

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

    /// Verifies that declared vector parameter defaults reach the shader uniform buffer.
    #[test]
    fn a_vector_params_declared_default_reaches_the_shader() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut set = build(&gpu, GLOW);
        let [r, g, b, _] = texel(&gpu, &mut set);
        close(r, 0.4, "glow.x");
        close(g, 0.7, "glow.y");
        close(b, 1.0, "glow.z");
    }

    /// Verifies that writing to a single component modifies only that component and leaves others intact.
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

    /// Verifies that vector parameters publish one control per component in declaration and xyz order.
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

    /// Verifies that setting a vector parameter atomically updates uniform buffers.
    #[test]
    fn setting_a_vector_param_atomically_updates_the_uniform_buffer() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut set = build(&gpu, GLOW);

        let written = set.set_param_value(None, "glow", Value::Vec3([0.25, 0.5, 0.75]));
        assert_eq!(written, 1, "setting `glow` as Vec3 should update 1 node");

        assert_eq!(
            set.param_value("glow"),
            Some(Value::Vec3([0.25, 0.5, 0.75]))
        );

        assert_eq!(set.param("glow.x"), Some(0.25));
        assert_eq!(set.param("glow.y"), Some(0.5));
        assert_eq!(set.param("glow.z"), Some(0.75));

        let [r, g, b, a] = texel(&gpu, &mut set);
        assert_eq!([r, g, b, a], [0.25, 0.5, 0.75, 1.0]);
    }

    /// **Setting an addressed vector parameter atomically updates uniform.**
    ///
    /// Addressing a specific node with `NodeAddress { layer, index }` works for
    /// vector parameters, correctly matching the layer and slot.
    #[test]
    fn setting_an_addressed_vector_param_atomically_updates_uniform() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut set = build(&gpu, GLOW);

        let address = NodeAddress {
            layer: Layer::L4,
            index: 0,
        };
        let written = set.set_param_value(Some(address), "glow", Value::Vec3([1.0, 2.0, 3.5]));
        assert_eq!(written, 1, "addressed write to L4:0 should succeed");

        assert_eq!(
            set.param_value_at_address(address, "glow"),
            Some(Value::Vec3([1.0, 2.0, 3.5]))
        );
        assert_eq!(
            set.param_value_at(karakuri_ir::Kind::L4, 0, "glow"),
            Some(Value::Vec3([1.0, 2.0, 3.5]))
        );

        let [r, g, b, a] = texel(&gpu, &mut set);
        assert_eq!([r, g, b, a], [1.0, 2.0, 3.5, 1.0]);

        // Writing to a layer without this parameter refuses the write.
        let wrong_address = NodeAddress {
            layer: Layer::L2,
            index: 0,
        };
        assert_eq!(
            set.set_param_value(Some(wrong_address), "glow", Value::Vec3([0.0, 0.0, 0.0])),
            0
        );
    }

    /// Verifies seamless interoperation between atomic vector writes and individual component writes.
    #[test]
    fn atomic_and_component_writes_interoperate_seamlessly() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut set = build(&gpu, GLOW);

        // 1. Initial atomic write
        set.set_param_value(None, "glow", Value::Vec3([0.25, 0.5, 0.75]));
        assert_eq!(texel(&gpu, &mut set), [0.25, 0.5, 0.75, 1.0]);

        // 2. Mutate single component via set_param("glow.y", 2.0)
        assert_eq!(set.set_param("glow.y", 2.0), 1);
        assert_eq!(
            set.param_value("glow"),
            Some(Value::Vec3([0.25, 2.0, 0.75])),
            "param_value must reflect component mutation"
        );
        assert_eq!(texel(&gpu, &mut set), [0.25, 2.0, 0.75, 1.0]);

        // 3. Mutate single component via set_param_value("glow.x", Value::Scalar(1.0))
        assert_eq!(set.set_param_value(None, "glow.x", Value::Scalar(1.0)), 1);
        assert_eq!(set.param_value("glow"), Some(Value::Vec3([1.0, 2.0, 0.75])));
        assert_eq!(texel(&gpu, &mut set), [1.0, 2.0, 0.75, 1.0]);

        // 4. Incompatible arity should be refused (return 0) and not corrupt existing values
        assert_eq!(
            set.set_param_value(None, "glow", Value::Vec2([0.1, 0.2])),
            0,
            "Vec2 write to Vec3 parameter must be refused"
        );
        assert_eq!(
            set.set_param_value(None, "glow", Value::Scalar(5.0)),
            0,
            "Scalar write to Vec3 parameter must be refused"
        );
        assert_eq!(set.param_value("glow"), Some(Value::Vec3([1.0, 2.0, 0.75])));
        assert_eq!(texel(&gpu, &mut set), [1.0, 2.0, 0.75, 1.0]);
    }
}
