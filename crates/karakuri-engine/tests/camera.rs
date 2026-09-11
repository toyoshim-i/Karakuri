//! The camera edge, from the producer to the picture.
//!
//! The camera is no longer six numbers the host packs into each renderer's
//! uniform: it is a node with a state buffer, a derivation pass, and a bind
//! group every L4 reads — see `karakuri_engine::node::Camera`. Everything
//! between the `Orbit` a caller assigns and the texels that come out is GPU
//! work, so the picture is the only place to check that it arrived.
//!
//! Three claims, and the third is the one that survived a defect injection
//! before this file existed:
//!
//! - **The camera reaches the frame**, so moving it moves the material.
//! - **It is re-derived every frame**, so a camera that turns keeps turning.
//! - **The aspect ratio reaches the projection.** It belongs to the canvas
//!   rather than to the camera, which is exactly why it is the piece that can
//!   go missing without any of the above noticing: it enters at the derivation,
//!   from a different buffer, written by a different call.
//!
//! `node::camera`'s own unit tests hold the derivation against the host's copy
//! of the same arithmetic. These hold the *plumbing* against the picture, which
//! is a different question: a perfect derivation nothing binds draws nothing.

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
mod gpu {
    use karakuri_engine::camera::Orbit;
    use karakuri_engine::set::Layering;
    use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
    use karakuri_ir::typed::Checked;

    /// **One element, off the view axis and away from what the camera looks at.**
    ///
    /// The `Orbit` below is pinned at angle zero, which puts the eye on `+x` looking
    /// back at the origin — so screen-right is world `-z` and screen-up is world
    /// `+y`. `(0, 1, -1.5)` therefore lands up and to the right of centre, and every
    /// measurement here is that offset.
    ///
    /// **Away from the origin is the part that took two tries.** An orbit turns
    /// *about* the point it looks at, so material near that point stays near the
    /// centre of the frame however far the camera swings — a first fixture at
    /// `(0, 0, -1)` moved four texels over a fifth of a revolution, which reads
    /// exactly like a camera that never reached the draw.
    pub(super) const MARK: &str = r#"
proc mark {
  kind     L1
  topology points
  capacity [1, 1] = 1

  emit position

  element {
    position = vec3(0.0, 1.0, -1.5);
  }
}
"#;

    const DOT: &str = r#"
proc dot {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.03125;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;

    pub(super) fn compile(src: &str) -> Checked {
        let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("{}", render(&e, src)));
        let checked =
            karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{}", render(&e, src)));
        karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("{}", render(&e, src)));
        checked
    }

    fn render(errs: &[karakuri_ir::IrError], src: &str) -> String {
        errs.iter()
            .map(|e| e.render(src))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// A Set of one element and one renderer, at `w` by `h`, seen from `camera`.
    fn build(gpu: &Gpu, w: u32, h: u32, camera: Orbit) -> Set {
        let l4 = compile(DOT);
        let mut set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&compile(MARK), 1)],
            &[],
            &[],
            &[],
            &[&l4],
            Layering::Overdraw,
            7,
            &[],
            karakuri_engine::set::Wiring::default(),
        )
        .expect("one L1 and one L4");
        set.resize(&gpu.device, w, h);
        set.aim_camera(camera);
        set
    }

    /// The camera these tests measure against: **still**, so a frame is a frame and
    /// not a moment in a sweep.
    fn pinned() -> Orbit {
        Orbit {
            radius: 5.0,
            speed: 0.0,
            height: 0.0,
            ..Default::default()
        }
    }

    /// Brightness-weighted mean column and row of the lit texels, in texels.
    fn centroid(gpu: &Gpu, set: &mut Set, w: u32, h: u32) -> (f32, f32) {
        let px = frame(gpu, set, w, h);
        let (mut sx, mut sy, mut weight) = (0.0f64, 0.0f64, 0.0f64);
        for (i, t) in px.chunks_exact(4).enumerate() {
            if t[0] > 0.01 {
                let (x, y) = ((i as u32 % w) as f64, (i as u32 / w) as f64);
                sx += f64::from(t[0]) * x;
                sy += f64::from(t[0]) * y;
                weight += f64::from(t[0]);
            }
        }
        assert!(
            weight > 0.0,
            "nothing was drawn, so there is nowhere to measure"
        );
        ((sx / weight) as f32, (sy / weight) as f32)
    }

    /// RGBA f32 per texel, after one frame.
    fn frame(gpu: &Gpu, set: &mut Set, w: u32, h: u32) -> Vec<f32> {
        let present = Present::new(&gpu.device, wgpu::TextureFormat::Rgba16Float, w, h);
        set.prepare(&gpu.queue, 1, &Signals::default());

        let bytes_per_row = w * 8;
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(bytes_per_row * h),
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
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
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
        let out: Vec<f32> = data
            .chunks_exact(2)
            .map(|b| f16(u16::from_le_bytes([b[0], b[1]])))
            .collect();
        drop(data);
        readback.unmap();
        out
    }

    fn f16(bits: u16) -> f32 {
        let sign = f32::from_bits(u32::from(bits & 0x8000) << 16);
        let exp = (bits >> 10) & 0x1f;
        let mant = u32::from(bits & 0x3ff);
        let v = match exp {
            0 => f32::from_bits(mant << 13) * 2.0f32.powi(-112),
            0x1f => f32::from_bits(0x7f80_0000 | (mant << 13)),
            _ => f32::from_bits(((u32::from(exp) + 112) << 23) | (mant << 13)),
        };
        f32::from_bits(v.to_bits() | sign.to_bits())
    }

    // ---------------------------------------------------------------------------

    /// **The camera reaches the frame.** Nothing about it is on the host any more —
    /// the state goes into a buffer, a pass derives the matrix, and a bind group
    /// carries it to the vertex stage — so a picture that moves when the camera does
    /// is the only proof that all three happened.
    ///
    /// Dollying in rather than pitching up, because a pitch rotates about the point
    /// the camera looks at and this material is close to it: raising the eye by 1.5
    /// moves the material half a texel, which is a fact about the geometry and not
    /// about the camera. Distance scales the whole offset and cannot be cancelled by
    /// where the material happens to sit.
    #[test]
    fn moving_the_camera_moves_the_material() {
        let gpu = Gpu::headless().expect("no GPU available");
        const W: u32 = 96;
        const H: u32 = 96;
        let offset = |radius: f32| {
            let mut set = build(&gpu, W, H, Orbit { radius, ..pinned() });
            centroid(&gpu, &mut set, W, H).0 - W as f32 / 2.0
        };

        let far = offset(5.0);
        let near = offset(3.0);
        assert!(
            far.abs() > 4.0,
            "the material is on the centre column; nothing to measure"
        );
        // Two thirds the distance, so five thirds the offset — asserted as a
        // direction and a lower bound rather than a ratio, since what is under test
        // is that the camera arrives at all.
        assert!(
            near.abs() > far.abs() * 1.3,
            "closing from 5 to 3 moved the material from {far} texels off centre to \
         {near} — the camera did not reach the draw"
        );
    }

    /// **And it is derived every frame**, not once at build.
    ///
    /// An orbit that turns is the case a cached derivation gets wrong, and it gets
    /// it wrong silently: the first frame is correct, so a still fixture and a
    /// single-frame test both pass. This one lets the same Set run on.
    #[test]
    fn a_turning_camera_keeps_turning() {
        let gpu = Gpu::headless().expect("no GPU available");
        const W: u32 = 96;
        const H: u32 = 96;
        // Thirty frames at sixty steps a second is half a second; at a quarter
        // revolution a second that is an eighth of a turn, which swings the material
        // a good way across the frame without carrying it off the edge.
        let mut set = build(
            &gpu,
            W,
            H,
            Orbit {
                speed: 0.25,
                ..pinned()
            },
        );

        let first = centroid(&gpu, &mut set, W, H).0;
        let mut last = first;
        for _ in 0..30 {
            last = centroid(&gpu, &mut set, W, H).0;
        }
        assert!(
            (last - first).abs() > 6.0,
            "thirty frames of a turning camera left the material at column {last}, \
         where it started at {first} — the derivation ran once and was reused"
        );
    }

    /// **The aspect ratio reaches the projection**, and it is the piece most easily
    /// lost: it belongs to the canvas rather than to the camera, so it arrives at
    /// the derivation from its own buffer, written by its own call, and every test
    /// above passes with it stuck at 1.
    ///
    /// Same width, twice the height. The field of view is vertical, so a taller
    /// frame at a fixed width is a *narrower* one horizontally — the same world
    /// spreads over twice as many texels across, and the material's distance from
    /// the centre column doubles.
    #[test]
    fn the_canvas_shape_reaches_the_projection() {
        let gpu = Gpu::headless().expect("no GPU available");
        const W: u32 = 128;

        let wide = centroid(&gpu, &mut build(&gpu, W, 64, pinned()), W, 64).0 - W as f32 / 2.0;
        let square = centroid(&gpu, &mut build(&gpu, W, 128, pinned()), W, 128).0 - W as f32 / 2.0;

        assert!(
            wide.abs() > 4.0,
            "the material is on the centre column; nothing to measure"
        );
        assert!(
            (square - 2.0 * wide).abs() < 0.25 * wide.abs(),
            "at aspect 2 the material sits {wide} texels from the centre and at aspect 1 \
         it sits {square}, where twice {wide} was due — the canvas did not reach the \
         projection"
        );
    }

    // ---------------------------------------------------------------------------
    // L3 — a camera that is a procedure
    // ---------------------------------------------------------------------------

    /// A camera on the clock alone, parameterised so a test can move it. Writes two
    /// of the six outputs and leaves the other four to their defaults, which is what
    /// the simplest camera anyone writes looks like.
    fn sweep(dist: f32) -> String {
        format!(
            r#"
proc sweep {{
  kind L3
  param dist : float [1.0, 40.0] = {dist:?}
  camera {{
    eye    = vec3(dist, 0.0, 0.0);
    target = vec3(0.0, 0.0, 0.0);
  }}
}}
"#
        )
    }

    /// **Sized by its own param**, so a picture that changes when the param does
    /// proves three things at once: the L3's pass ran, its uniform reached it, and
    /// the state it wrote was what the derivation read.
    const GAIN_DOT: &str = r#"
proc gain_dot {
  kind  L4
  blend additive

  param gain : float [0.0, 4.0] = 1.0

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.03125;
  }

  fragment {
    color = vec4(gain, gain, gain, 1.0);
  }
}
"#;

    fn with_camera(gpu: &Gpu, l3: Option<&str>, l4: &str, w: u32, h: u32) -> Set {
        let l3s: Vec<Checked> = l3.map(compile).into_iter().collect();
        let l4 = compile(l4);
        let mut set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&compile(MARK), 1)],
            &[],
            &l3s.iter().collect::<Vec<_>>(),
            &[],
            &[&l4],
            Layering::Overdraw,
            7,
            &[],
            karakuri_engine::set::Wiring::default(),
        )
        .expect("one L1, an optional camera, and one L4");
        set.resize(&gpu.device, w, h);
        set.aim_camera(pinned());
        set
    }

    /// **A camera procedure produces the view**, and its params reach it. The whole
    /// path is on the GPU — a uniform write, a compute pass writing six numbers, a
    /// second deriving a matrix, and a bind group — so moving the eye and watching
    /// the material move is the only end-to-end proof there is.
    #[test]
    fn a_camera_procedure_produces_the_view() {
        let gpu = Gpu::headless().expect("no GPU available");
        const W: u32 = 96;
        const H: u32 = 96;

        let offset = |dist: f32| {
            let mut set = with_camera(&gpu, Some(&sweep(dist)), GAIN_DOT, W, H);
            centroid(&gpu, &mut set, W, H).0 - W as f32 / 2.0
        };
        let far = offset(5.0);
        let near = offset(3.0);

        assert!(
            far.abs() > 4.0,
            "the material is on the centre column; nothing to measure"
        );
        assert!(
            near.abs() > far.abs() * 1.3,
            "closing the camera's own `dist` from 5 to 3 moved the material from {far} texels off \
         centre to {near} — the procedure did not reach the frame"
        );
    }

    /// **And it can be addressed after the build**, like any other node's params.
    #[test]
    fn a_cameras_parameters_are_addressed_as_a_nodes() {
        let gpu = Gpu::headless().expect("no GPU available");
        const W: u32 = 96;
        const H: u32 = 96;
        let mut set = with_camera(&gpu, Some(&sweep(5.0)), GAIN_DOT, W, H);

        // **A procedure's map and the built-in's, side by side.** `L3:0` is the
        // `sweep` this Set names and `L3:1` is the orbit after it, which
        // declares the three placement numbers the engine states for it
        // (ADR-0318) — so this asserts two things at once: each camera's params
        // are reported at that camera's address, and the built-in's are its own
        // rather than a procedure's.
        let mut declared: Vec<(u32, &str, f32)> = set
            .params()
            .filter(|(layer, ..)| *layer == karakuri_ir::Kind::L3)
            .map(|(_, index, name, value)| (index, name, value))
            .collect();
        declared.sort_by_key(|(index, name, _)| (*index, *name));
        assert_eq!(
            declared,
            vec![
                (0, "dist", 5.0),
                (1, "height", 0.0),
                (1, "radius", 5.0),
                (1, "speed", 0.0),
            ],
            "the cameras' params are not reported as each camera's own"
        );

        let far = centroid(&gpu, &mut set, W, H).0 - W as f32 / 2.0;
        assert!(
            set.set_param_at(karakuri_ir::Kind::L3, 0, "dist", 3.0),
            "the camera declares `dist`"
        );
        let near = centroid(&gpu, &mut set, W, H).0 - W as f32 / 2.0;
        assert!(
            near.abs() > far.abs() * 1.3,
            "writing the camera's `dist` left the material at {near}, against {far}"
        );
    }

    /// **A camera between the deformations and the renderers does not shift what a
    /// renderer reads.** This is a regression: the L4 uniform pass spelled out its
    /// own slot arithmetic instead of asking [`Set::slot_of`], so inserting an L3
    /// gave every renderer the node before it — and `soft_points` drew a black
    /// frame, because its `exposure` resolved against the camera's parameter map
    /// and came back missing.
    ///
    /// The reading is a brightness rather than a position, on purpose: a shifted map
    /// leaves a declared param with no value, which the uniform path writes as
    /// `0.0`. A renderer whose colour *is* its param then goes black — which is
    /// exactly what happened, and is the one symptom a picture can show.
    #[test]
    fn a_camera_does_not_shift_the_parameters_a_renderer_reads() {
        let gpu = Gpu::headless().expect("no GPU available");
        const W: u32 = 64;
        const H: u32 = 64;

        let peak = |l3: Option<&str>| {
            let mut set = with_camera(&gpu, l3, GAIN_DOT, W, H);
            frame(&gpu, &mut set, W, H)
                .chunks_exact(4)
                .map(|t| t[0])
                .fold(0.0f32, f32::max)
        };
        // The same camera either way, so the only difference between the two Sets
        // is whether a node sits between the geometry and the renderer.
        let built_in = peak(None);
        let procedure = peak(Some(&sweep(5.0)));

        assert!(
            built_in > 0.5,
            "the renderer's own default never reached the frame: {built_in}"
        );
        assert!(
            (procedure - built_in).abs() < 0.01,
            "with a camera procedure the renderer drew at {procedure}, and without one at \
         {built_in} — a node was inserted and the renderer read the map beside its own"
        );
    }

    /// **The built-in orbit is not a second producer of a procedure's camera.** A
    /// Set whose files declare an L3 still has the `camera` field on it — a
    /// `camera` record and a Set file both set one — and it writes the orbit's own
    /// node, which is a different edge: this renderer declares no slot, so it draws
    /// from `L3:0`, which is the procedure. Two producers writing *one* edge would
    /// resolve by whichever ran last, which is what having a node apiece prevents.
    #[test]
    fn an_orbit_assigned_beside_a_camera_procedure_reaches_nothing() {
        let gpu = Gpu::headless().expect("no GPU available");
        const W: u32 = 96;
        const H: u32 = 96;

        let mut set = with_camera(&gpu, Some(&sweep(5.0)), GAIN_DOT, W, H);
        let before = centroid(&gpu, &mut set, W, H);
        // A camera nowhere near the procedure's, and pointed from above rather than
        // level, so anything of it that leaked would move the material a long way.
        set.aim_camera(Orbit {
            radius: 20.0,
            height: 18.0,
            speed: 0.0,
            ..Default::default()
        });
        let after = centroid(&gpu, &mut set, W, H);

        assert!(
            (before.0 - after.0).abs() < 0.5 && (before.1 - after.1).abs() < 0.5,
            "assigning an orbit moved the material from {before:?} to {after:?} — the built-in \
         reached a Set whose camera is a procedure"
        );
    }

    /// **An address past a layer's last node reaches nothing**, rather than the
    /// first node of the layer after it.
    ///
    /// The parameter maps are laid end to end in node order, so `slot_of(layer) +
    /// index` is a position and says nothing about whose it is. A Set with no
    /// camera makes that concrete: with nothing between the deformations and the
    /// renderers, `L3` and `L4` start at the same slot, and `--param
    /// L3:0:exposure=0.0` reached renderer 0 and blacked out the frame — silently,
    /// because the caller only reports an address that reached *zero* nodes.
    ///
    /// Two addresses, and the second is the same defect without an L3 in it:
    /// `L4:1:` on a Set of one renderer. That one is safe today only because the
    /// renderers are last and their range runs to the end of the list, which is a
    /// property of the ordering rather than of the check.
    #[test]
    fn an_address_past_a_layers_last_node_reaches_nothing() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut set = with_camera(&gpu, None, GAIN_DOT, 64, 64);

        assert_eq!(
            set.param("gain"),
            Some(1.0),
            "the renderer's declared default"
        );
        assert!(
            !set.set_param_at(karakuri_ir::Kind::L3, 0, "gain", 0.0),
            "a Set with no camera has no L3 node to address"
        );
        assert_eq!(
            set.param("gain"),
            Some(1.0),
            "the L3 address reached the renderer"
        );
        assert!(
            !set.set_param_at(karakuri_ir::Kind::L4, 1, "gain", 0.0),
            "this Set draws with one renderer, so index 1 addresses nothing"
        );
        assert_eq!(
            set.param("gain"),
            Some(1.0),
            "an out-of-range renderer address wrote anyway"
        );
        // And the address that does exist still works, so the bound is a bound and
        // not a refusal.
        assert!(set.set_param_at(karakuri_ir::Kind::L4, 0, "gain", 0.25));
        assert_eq!(set.param("gain"), Some(0.25));
    }

    /// **A vector param is driven by component, and used to panic the render
    /// thread for being declared at all.**
    ///
    /// Two defects, one after the other, and this is the test that has watched
    /// both. First: every node's uniform path wrote *every* declared name as an
    /// `f32`, and the packer panics on a field its layout says is a
    /// `vec3<f32>` — so a `.kir` that parses, checks and costs took the render
    /// thread down on the first `prepare`, and not in the swap worker, so not
    /// caught as `SetError::Panicked`. That was closed by writing the field as
    /// a vector, with zeroes, because nothing could state the value.
    ///
    /// Second: the zeroes. `Param::default_scalar` folds a scalar, so a vector
    /// never entered a node's value map and a declared `vec3(0.5, 0.5, 0.5)`
    /// reached the shader as `vec3(0.0)`. The map holds one `f32` per component
    /// now — `centre.x`, `centre.y`, `centre.z`
    /// (`docs/adr/0268-a-vector-parameter-is-driven-one-component-at-a-time.md`).
    ///
    /// The bare name still holds nothing, and that is the part that did not
    /// change: it names three numbers and `Set::param` answers with one.
    ///
    /// Building and preparing is still most of the test: the panic was
    /// unconditional.
    #[test]
    fn a_vector_param_is_driven_by_component_rather_than_packed_as_a_scalar() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mixed = r#"
proc mixed {
  kind  L4
  blend additive

  param centre : vec3  [0.0, 1.0] = vec3(0.5, 0.5, 0.5)
  param gain   : float [0.0, 4.0] = 1.0

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.03125;
  }

  fragment {
    color = vec4(gain, gain, gain, 1.0);
  }
}
"#;
        let mut set = with_camera(&gpu, Some(&sweep(5.0)), mixed, 64, 64);
        let peak = frame(&gpu, &mut set, 64, 64)
            .chunks_exact(4)
            .map(|t| t[0])
            .fold(0.0f32, f32::max);

        // And the scalar beside it still arrives, so the filter is a filter rather
        // than a node that gave up on its params.
        assert!(
            peak > 0.5,
            "the scalar param never reached the frame: {peak}"
        );
        assert_eq!(set.param("gain"), Some(1.0));
        assert_eq!(
            set.param("centre"),
            None,
            "the bare name of a vector param names three numbers and holds none"
        );
        for key in ["centre.x", "centre.y", "centre.z"] {
            assert_eq!(
                set.param(key),
                Some(0.5),
                "{key} is what the uniform is packed from, and the declaration states it"
            );
        }
    }

    // ---------------------------------------------------------------------------
    // Several cameras, and which one each renderer reads
    // ---------------------------------------------------------------------------

    /// A camera on the `+x` axis or the `-x` axis, looking at the origin. **The two
    /// are mirror images**, so material off the view axis lands on opposite sides
    /// of the frame — which is a reading that cannot be produced by a Set that drew
    /// both renderers from one camera, whichever one it picked.
    fn from_x(name: &str, x: f32) -> String {
        format!(
            r#"
proc {name} {{
  kind L3
  camera {{
    eye    = vec3({x:?}, 0.0, 0.0);
    target = vec3(0.0, 0.0, 0.0);
  }}
}}
"#
        )
    }

    /// A renderer that says which camera it draws from, in one colour channel so
    /// that two of them in one frame can be measured apart.
    pub(super) fn through(name: &str, colour: [f32; 3]) -> String {
        let (r, g, b) = (colour[0], colour[1], colour[2]);
        format!(
            r#"
proc {name} {{
  kind  L4
  blend additive

  uses view : Camera

  consumes position

  vertex {{
    clip       = view.clip * vec4(position, 1.0);
    point_rate = 0.03125;
  }}

  fragment {{
    color = vec4({r:?}, {g:?}, {b:?}, 1.0);
  }}
}}
"#
        )
    }

    /// A renderer that names no camera and reads the Set's, which is what every
    /// renderer written before the slot existed does.
    const PLAIN: &str = r#"
proc plain {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.03125;
  }

  fragment {
    color = vec4(0.0, 0.0, 1.0, 1.0);
  }
}
"#;

    /// A Set of the one mark, `l3s` cameras and `l4s` renderers, wired by `edges`.
    fn wired(
        gpu: &Gpu,
        l3s: &[String],
        l4s: &[&str],
        edges: &[(&str, &str, &str)],
        w: u32,
        h: u32,
    ) -> Result<Set, karakuri_engine::set::SetError> {
        let l3s: Vec<Checked> = l3s.iter().map(|s| compile(s)).collect();
        let l4s: Vec<Checked> = l4s.iter().map(|s| compile(s)).collect();
        let edges: Vec<karakuri_engine::set::Edge> = edges
            .iter()
            .map(|(node, slot, to)| karakuri_engine::set::Edge {
                node: node.to_string(),
                slot: (*slot).into(),
                to: to.to_string(),
            })
            .collect();
        let set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&compile(MARK), 1)],
            &[],
            &l3s.iter().collect::<Vec<_>>(),
            &[],
            &l4s.iter().collect::<Vec<_>>(),
            Layering::Overdraw,
            7,
            &[],
            karakuri_engine::set::Wiring {
                edges: &edges,
                ..Default::default()
            },
        );
        set.map(|mut set| {
            set.resize(&gpu.device, w, h);
            set.aim_camera(pinned());
            set
        })
    }

    /// Brightness-weighted mean column of one colour channel's lit texels.
    fn column(px: &[f32], w: u32, channel: usize) -> f32 {
        let (mut sx, mut weight) = (0.0f64, 0.0f64);
        for (i, t) in px.chunks_exact(4).enumerate() {
            if t[channel] > 0.01 {
                sx += f64::from(t[channel]) * f64::from(i as u32 % w);
                weight += f64::from(t[channel]);
            }
        }
        assert!(weight > 0.0, "channel {channel} drew nothing to measure");
        (sx / weight) as f32
    }

    /// **Two cameras in one Set, and two renderers drawing from different ones.**
    ///
    /// The whole point of the slot: a Set could hold one viewpoint because a
    /// renderer had no way to say which of two it meant, and `camera` meant "the
    /// Set's" because there was only ever one. Two mirror-image cameras put the
    /// same element on opposite sides of the frame, so the two channels of one
    /// picture are the proof — a Set that drew both from one camera puts them in
    /// the same place whichever one it picked.
    ///
    /// **And swapping the edges swaps the picture**, which is the half a static
    /// frame cannot show: without it, "each renderer read a different camera" and
    /// "each renderer read the camera at its own index" are the same measurement.
    #[test]
    fn two_renderers_draw_from_the_cameras_their_edges_name() {
        let gpu = Gpu::headless().expect("no GPU available");
        const W: u32 = 96;
        const H: u32 = 96;
        let cameras = [from_x("east", 5.0), from_x("west", -5.0)];
        let red = through("red", [1.0, 0.0, 0.0]);
        let green = through("green", [0.0, 1.0, 0.0]);

        let measure = |edges: &[(&str, &str, &str)]| {
            let mut set = wired(&gpu, &cameras, &[&red, &green], edges, W, H).expect("two cameras");
            let px = frame(&gpu, &mut set, W, H);
            (
                column(&px, W, 0) - W as f32 / 2.0,
                column(&px, W, 1) - W as f32 / 2.0,
            )
        };

        let (r, g) = measure(&[("red", "view", "east"), ("green", "view", "west")]);
        assert!(
            r.abs() > 4.0 && g.abs() > 4.0,
            "the material is on the centre column in one of the two; nothing to measure: {r}, {g}"
        );
        assert!(
            r.signum() != g.signum(),
            "two mirror-image cameras left both renderers on the same side of the frame — \
         {r} and {g} — so both drew from one camera"
        );

        // The same two renderers and the same two cameras, wired the other way
        // round.
        let (r2, g2) = measure(&[("red", "view", "west"), ("green", "view", "east")]);
        assert!(
            (r2 - g).abs() < 1.0 && (g2 - r).abs() < 1.0,
            "swapping the edges left the picture at {r2}, {g2} where {g}, {r} was due — \
         a renderer is reading the camera at its own index rather than the one it names"
        );
    }

    /// **A renderer that declares no slot reads the Set's camera**, which is the
    /// first one — and that is what `camera`, `eye` and `ray` have always meant.
    ///
    /// Every renderer in the library is this one, so it is the case that must not
    /// have moved: the slot is how a renderer says *which*, and saying nothing has
    /// to keep meaning what it meant.
    #[test]
    fn a_renderer_with_no_slot_reads_the_sets_camera() {
        let gpu = Gpu::headless().expect("no GPU available");
        const W: u32 = 96;
        const H: u32 = 96;
        let cameras = [from_x("east", 5.0), from_x("west", -5.0)];
        let bound = through("bound", [1.0, 0.0, 0.0]);

        // The unbound renderer draws blue and the bound one draws red, in one
        // frame, so the two readings come from one build and one camera pass.
        let mut set = wired(
            &gpu,
            &cameras,
            &[&bound, PLAIN],
            &[("bound", "view", "east")],
            W,
            H,
        )
        .expect("a slot and a renderer that declares none");
        let px = frame(&gpu, &mut set, W, H);
        let named = column(&px, W, 0) - W as f32 / 2.0;
        let silent = column(&px, W, 2) - W as f32 / 2.0;

        assert!(
            named.abs() > 4.0,
            "the material is on the centre column; nothing to measure"
        );
        assert!(
            (named - silent).abs() < 1.0,
            "the renderer that named `east` drew at {named} and the one that named nothing at \
         {silent} — a renderer with no slot has to read camera 0, which is `east`"
        );
    }

    /// **The built-in camera is a node, and an edge can name it.**
    ///
    /// It was a field on the `Set` and reachable from nowhere: a Set with no L3 had
    /// no L3 node at all, so a renderer could draw from the orbit only by saying
    /// nothing. Now it is `orbit` — a name like any other — and the proof that the
    /// edge reached the *producer* is that moving the orbit moves the material.
    #[test]
    fn the_built_in_camera_is_a_node_an_edge_can_name() {
        let gpu = Gpu::headless().expect("no GPU available");
        const W: u32 = 96;
        const H: u32 = 96;
        let named = through("named", [1.0, 0.0, 0.0]);

        let offset = |radius: f32| {
            let mut set = wired(
                &gpu,
                &[],
                &[&named],
                &[("named", "view", karakuri_engine::set::BUILTIN_CAMERA)],
                W,
                H,
            )
            .expect("a renderer bound to the built-in camera");
            set.aim_camera(Orbit { radius, ..pinned() });
            column(&frame(&gpu, &mut set, W, H), W, 0) - W as f32 / 2.0
        };

        let far = offset(5.0);
        let near = offset(3.0);
        assert!(
            far.abs() > 4.0,
            "the material is on the centre column; nothing to measure"
        );
        assert!(
            near.abs() > far.abs() * 1.3,
            "closing the orbit from 5 to 3 moved the material from {far} texels off centre to \
         {near} — the edge did not reach the built-in producer"
        );
    }

    /// **The built-in camera is addressable as `L3:0`, and `L4:0` still reaches the
    /// first renderer.**
    ///
    /// This is the off-by-one this commit could have introduced. `slot_of` computes
    /// a layer's origin by summing the layers before it, so giving the camera layer
    /// a node in a Set that had none shifts every renderer's parameter map by one —
    /// unless [`Set::params`] grows an entry at the same position, which is a
    /// different file's job. Get it wrong and `--param L4:0:gain` writes the
    /// camera's map and the renderer keeps its default, silently.
    #[test]
    fn the_built_in_camera_takes_a_slot_without_moving_the_renderers() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut set = with_camera(&gpu, None, GAIN_DOT, 64, 64);

        assert_eq!(
            set.node_named(karakuri_engine::set::BUILTIN_CAMERA),
            Some((karakuri_ir::Kind::L3, 0)),
            "a Set with no camera procedure holds the built-in as its L3 node"
        );
        // The renderer is still the first node of L4, and its params are still
        // reported as its own — beside the camera's three, which are reported
        // at `L3:0` and not at the renderer's address (ADR-0318). **That is
        // what makes this test sharper rather than weaker**: the camera's map
        // is no longer empty, so an origin off by one now lands the orbit's
        // `radius` on the renderer instead of landing nothing there.
        let mut declared: Vec<(karakuri_ir::Kind, u32, &str)> = set
            .params()
            .map(|(layer, index, name, _)| (layer, index, name))
            .collect();
        declared.sort_by_key(|(layer, index, name)| (format!("{layer:?}"), *index, *name));
        assert_eq!(
            declared,
            vec![
                (karakuri_ir::Kind::L3, 0, "height"),
                (karakuri_ir::Kind::L3, 0, "radius"),
                (karakuri_ir::Kind::L3, 0, "speed"),
                (karakuri_ir::Kind::L4, 0, "gain"),
            ],
            "the renderer's params are reported at the renderer's address"
        );

        // And a write at that address reaches it: the value moves and the picture
        // moves with it.
        let peak = |set: &mut Set| {
            frame(&gpu, set, 64, 64)
                .chunks_exact(4)
                .map(|t| t[0])
                .fold(0.0f32, f32::max)
        };
        assert!(peak(&mut set) > 0.5, "the renderer's default never drew");
        assert!(
            set.set_param_at(karakuri_ir::Kind::L4, 0, "gain", 0.0),
            "`L4:0` addresses the first renderer"
        );
        assert!(
            peak(&mut set) < 0.01,
            "writing `L4:0:gain` did not reach the renderer — every L4 address is off by one"
        );
        // The camera's own address reaches the camera, rather than reaching the
        // renderer's map. It declares three parameters of its own since
        // ADR-0318 and `gain` is not one of them, which is what makes this an
        // off-by-one test rather than an empty-map one.
        assert!(
            !set.set_param_at(karakuri_ir::Kind::L3, 0, "gain", 1.0),
            "`L3:0` is the built-in camera, which has no `gain` — an address that writes one \
             has walked into the renderer's map"
        );
    }

    // ----- The built-in camera's three placement numbers ------------------
    //
    // `docs/adr/0318-the-built-in-cameras-three-placement-numbers-are-parameter-rows.md`:
    // the orbit's `radius`, `speed` and `height` are parameters of the camera
    // node, so every route a parameter has reaches them and no route was
    // invented for them. These four are the four claims that decision makes,
    // and each was watched to fail against the tree that did not carry it.

    /// **The built-in camera declares three parameters**, at the values the
    /// Set was aimed with and over the ranges the engine states.
    ///
    /// The Set here holds no camera procedure, so `L3:0` is the built-in — and
    /// the values are `pinned()`'s rather than `Orbit::default()`'s, which is
    /// the second claim in one: `Set::aim_camera` states the three into the
    /// node's map and not only into the field beside it.
    #[test]
    fn the_built_in_camera_declares_its_three_placement_numbers() {
        let gpu = Gpu::headless().expect("no GPU available");
        let set = with_camera(&gpu, None, GAIN_DOT, 32, 32);

        let mut declared: Vec<(u32, &str, f32)> = set
            .params()
            .filter(|(layer, ..)| *layer == karakuri_ir::Kind::L3)
            .map(|(_, index, key, value)| (index, key, value))
            .collect();
        declared.sort_by_key(|(_, key, _)| *key);
        assert_eq!(
            declared,
            vec![(0, "height", 0.0), (0, "radius", 5.0), (0, "speed", 0.0),],
            "the built-in camera's parameter map is not the orbit it was aimed with"
        );
    }

    /// **They are published, addressed, in the order the orbit states them**,
    /// and over the declared ranges — which is what a fader draws and what a
    /// MIDI control is learned against.
    ///
    /// **Addressed and not bare**, which is the part with a picture behind it:
    /// seven of this repository's example procedures declare a `radius`, so a
    /// bare control would weld the camera to a geometry.
    #[test]
    fn the_cameras_three_publish_addressed_and_in_order() {
        let gpu = Gpu::headless().expect("no GPU available");
        let set = with_camera(&gpu, None, GAIN_DOT, 32, 32);

        let mine: Vec<(String, [f32; 2])> = set
            .published()
            .into_iter()
            .filter(|p| Orbit::PLACEMENT.iter().any(|(key, _)| *key == p.key))
            .inspect(|p| {
                assert_eq!(
                    p.at,
                    Some((karakuri_ir::Kind::L3, 0)),
                    "`{}` was published bare, and a bare name is a control over every node \
                     that declares it",
                    p.key
                );
            })
            .map(|p| (p.key, p.range))
            .collect();
        assert_eq!(
            mine,
            vec![
                ("radius".to_owned(), [1.0, 40.0]),
                ("speed".to_owned(), [0.0, 2.0]),
                ("height".to_owned(), [-40.0, 40.0]),
            ],
            "the camera's rows are not the three the engine declares, in order"
        );
    }

    /// **A write to `L3:0:radius` reaches the frame**, which is the whole
    /// claim: a row that emits a write nothing draws is a row that does
    /// nothing.
    ///
    /// The same measurement `a_camera_procedure_produces_the_view` makes, with
    /// the built-in as the producer instead of an L3 — so it is the plumbing
    /// from the parameter map to the state buffer that is under test and
    /// nothing else.
    #[test]
    fn a_write_to_the_cameras_radius_reaches_the_frame() {
        let gpu = Gpu::headless().expect("no GPU available");
        const W: u32 = 96;
        const H: u32 = 96;

        let offset = |radius: f32| {
            let mut set = with_camera(&gpu, None, GAIN_DOT, W, H);
            assert!(
                set.write_param(&karakuri_engine::ParamWrite::at(
                    karakuri_ir::Kind::L3,
                    0,
                    "radius",
                    radius,
                ))
                .expect("one node, so no authority to cross")
                    == 1,
                "the write did not land on the camera node"
            );
            centroid(&gpu, &mut set, W, H).0 - W as f32 / 2.0
        };
        let far = offset(5.0);
        let near = offset(3.0);

        assert!(
            far.abs() > 4.0,
            "the material is on the centre column; nothing to measure"
        );
        assert!(
            near.abs() > far.abs() * 1.3,
            "closing the camera's radius from 5 to 3 moved the material from {far} texels off \
             centre to {near} — the parameter did not reach the frame"
        );
    }

    /// **A bare name does not reach it**, which is the other half of the row
    /// above and the one with a defect behind it: `drift_shell` declares a
    /// `radius` of its own, so a `--param radius=…` that also swung the camera
    /// would be a control doing something it does not draw.
    #[test]
    fn a_bare_name_does_not_reach_the_cameras_radius() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut set = with_camera(&gpu, None, GAIN_DOT, 32, 32);

        let landed = set
            .write_param(&karakuri_engine::ParamWrite::everywhere("radius", 2.0))
            .expect("nothing to cross");
        assert_eq!(
            landed, 0,
            "a bare `radius` landed somewhere, and the only node declaring one here is the camera"
        );
        assert_eq!(set.orbit().radius, 5.0, "the camera moved on a bare name");
    }

    /// **A rebuild keeps a radius somebody rode**, which is ADR-0132 met by
    /// ADR-0282's rule rather than by anything written for the camera: the
    /// ridden value is marked, the rebuild restates the aim, and the mark is
    /// carried back over the top of it.
    ///
    /// **The lens three come from the restatement and not from the ride**,
    /// which is the half that says the two are different kinds of fact.
    #[test]
    fn a_rebuild_keeps_a_ridden_camera_and_restates_the_rest() {
        let gpu = Gpu::headless().expect("no GPU available");
        let mut outgoing = with_camera(&gpu, None, GAIN_DOT, 32, 32);
        outgoing
            .write_param(&karakuri_engine::ParamWrite::at(
                karakuri_ir::Kind::L3,
                0,
                "radius",
                12.0,
            ))
            .expect("nothing to cross");

        // What the request would state: the aim the slot is pointed at, which
        // is `pinned()` with a wider lens than the ride ever touches.
        let restated = Orbit {
            fov_y: 1.0,
            ..pinned()
        };
        let mut incoming = with_camera(&gpu, None, GAIN_DOT, 32, 32);
        incoming.aim_camera(restated);
        assert_eq!(
            incoming.orbit().radius,
            5.0,
            "the restatement is what a fresh build holds before anything is carried"
        );

        assert_eq!(
            incoming.carry_moved_from(&outgoing),
            1,
            "one value was moved, so one is carried"
        );
        assert_eq!(
            incoming.orbit().radius,
            12.0,
            "the ridden radius did not survive the rebuild"
        );
        assert_eq!(
            incoming.orbit().fov_y,
            1.0,
            "the lens came from the outgoing Set rather than from what was restated"
        );
        assert_eq!(
            incoming.orbit().speed,
            pinned().speed,
            "a number nobody moved came from somewhere other than the restatement"
        );
    }
}

/// **The refusals, which reach no device.**
///
/// A Camera slot nothing binds, and one bound to a node that is not a camera:
/// both are decided by the edge walk in `Set::validate`, before any pipeline
/// exists. They used to take an adapter apiece because `Set::build_many` was
/// the only door to the rule; it reaches the same rule by calling `validate`.
mod refused {
    use super::gpu::{compile, through, MARK};
    use karakuri_engine::set::{Edge, Layering, SetError, Wiring};
    use karakuri_engine::Set;
    use karakuri_ir::typed::Checked;

    /// The Set `wired` above describes — one `MARK` at capacity 1, these
    /// cameras and these renderers — minus the device and everything
    /// downstream of it.
    fn validate_wired(
        l3s: &[String],
        l4s: &[&str],
        edges: &[(&str, &str, &str)],
    ) -> Result<(), SetError> {
        let l3s: Vec<Checked> = l3s.iter().map(|s| compile(s)).collect();
        let l4s: Vec<Checked> = l4s.iter().map(|s| compile(s)).collect();
        let edges: Vec<Edge> = edges
            .iter()
            .map(|(node, slot, to)| Edge {
                node: node.to_string(),
                slot: (*slot).into(),
                to: to.to_string(),
            })
            .collect();
        Set::validate(
            &[(&compile(MARK), 1)],
            &[],
            &l3s.iter().collect::<Vec<_>>(),
            &[],
            &l4s.iter().collect::<Vec<_>>(),
            Layering::Overdraw,
            7,
            &[],
            Wiring {
                edges: &edges,
                ..Default::default()
            },
        )
        .map(|_| ())
    }

    /// **A declared Camera slot must be bound**, exactly as a geometry slot and a
    /// Field slot must be.
    ///
    /// Filling it in from the Set's only camera would be right every time today and
    /// is the rule this notation exists to remove: "if there is exactly one, use
    /// it" is what capped a Set at one viewpoint, and a renderer that means the
    /// Set's camera says so by declaring no slot.
    #[test]
    fn an_unbound_camera_slot_is_refused() {
        let named = through("named", [1.0, 0.0, 0.0]);
        let err = validate_wired(&[], &[&named], &[])
            .expect_err("an unbound slot is not filled in from the Set's only camera");
        let text = format!("{err}");
        assert!(
            text.contains("`named` declares `view : Camera`"),
            "the refusal has to name the slot and the type it takes: {text}"
        );
    }

    /// **And bound to a camera**, rather than to whatever node the edge happened to
    /// name. The sentence says what the node it found actually is, because that is
    /// the half an operator cannot see from the edge.
    #[test]
    fn a_camera_slot_bound_to_something_that_is_not_a_camera_is_refused() {
        let named = through("named", [1.0, 0.0, 0.0]);
        let err = validate_wired(&[], &[&named], &[("named", "view", "mark")])
            .expect_err("a geometry is not a camera");
        let text = format!("{err}");
        assert!(
            text.contains("bound to `mark`") && text.contains("an L1"),
            "the refusal has to say what was bound and what it is: {text}"
        );
        assert!(
            text.contains(karakuri_engine::set::BUILTIN_CAMERA),
            "and which cameras this Set holds: {text}"
        );
    }
}
