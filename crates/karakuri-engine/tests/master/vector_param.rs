use super::common::*;

/// Writes its vector and scalar params straight to the frame: `rgb = glaze * gain`,
/// `a = tilt.y`. Every param type an L5 may declare (`float`, `vec2`, `vec3`).
pub const GLAZE: &str = r#"
proc glaze_probe {
  kind L5

  param glaze : vec3 [0.0, 1.0] = vec3(0.0, 0.25, 1.0)
  param tilt  : vec2 [0.0, 1.0] = vec2(0.5, 0.75)
  param gain  : float [0.0, 2.0] = 1.0

  frame {
    color = vec4(glaze * gain, tilt.y);
  }
}
"#;

mod gpu {
    use super::*;

    fn texel(pixels: &[u8]) -> [f32; 4] {
        let c = channels(&pixels[..8]);
        [c[0], c[1], c[2], c[3]]
    }

    fn assert_near(got: [f32; 4], want: [f32; 4], what: &str) {
        for (c, (g, w)) in got.iter().zip(want).enumerate() {
            assert!(
                (g - w).abs() < 0.01,
                "{what}: channel {c} is {g}, wanted {w} (texel {got:?})"
            );
        }
    }

    /// A chain slot's vector params pack into their `vecN` uniform fields: the
    /// declared defaults reach the frame, a write on one component key moves only
    /// that component, and the reading names every component with its default.
    #[test]
    fn a_chain_slot_packs_its_vector_params_per_component() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut present = present(&gpu);
        let mut points = hot_points(&gpu);
        let slot = Slot::build(
            &gpu.device,
            present.chain_layout(),
            address(GLAZE),
            &checked(GLAZE),
            None,
            BTreeMap::new(),
        )
        .expect("a vector-param L5 is a legal chain slot");
        drop(present.set_chain(&gpu.device, &gpu.queue, Chain::new(vec![slot])));

        assert_near(
            texel(&frame(&gpu, &present, &mut points)),
            [0.0, 0.25, 1.0, 0.75],
            "declared defaults",
        );

        let reading = present.chain_reading();
        let rows: Vec<(&str, f32, f32)> = reading[0]
            .params
            .iter()
            .map(|p| (p.key.as_str(), p.value, p.default))
            .collect();
        assert_eq!(
            rows,
            [
                ("glaze.x", 0.0, 0.0),
                ("glaze.y", 0.25, 0.25),
                ("glaze.z", 1.0, 1.0),
                ("tilt.x", 0.5, 0.5),
                ("tilt.y", 0.75, 0.75),
                ("gain", 1.0, 1.0),
            ],
            "a chain slot's surface names each vector component with its declared default"
        );

        let shape = present.chain_shape();
        let moved: BTreeMap<String, f32> = [("glaze.x".to_string(), 1.0)].into_iter().collect();
        assert!(
            present.set_chain_params(&gpu.queue, &shape, &[moved]),
            "a parameter move on the running shape was refused"
        );
        assert_near(
            texel(&frame(&gpu, &present, &mut points)),
            [1.0, 0.25, 1.0, 0.75],
            "glaze.x written to 1.0",
        );
        assert_eq!(present.chain_spec()[0].params.get("glaze.x"), Some(&1.0));
    }
}
