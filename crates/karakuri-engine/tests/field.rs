//! A `kind Field` procedure, spliced into whoever evaluates it.
//!
//! A field has no node: no buffer, no pass, no position in the chain. So there
//! is nothing to read back and nothing to address, and every claim here is
//! about the picture a *caller* draws — which is the right place for them,
//! since a field that produced the correct distances and reached nobody would
//! be a feature that does not exist.

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
mod gpu {
    use karakuri_engine::set::{Layering, SetError};
    use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
    use karakuri_ir::typed::Checked;

    const W: u32 = 64;
    const H: u32 = 64;

    /// One element, so the paired geometry costs nothing. A fullscreen renderer
    /// reads no element, but a Set always has an L1.
    pub(super) const STILL: &str = r#"
proc still {
  kind     L1
  topology points
  capacity [1, 1] = 1

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#;

    /// A sphere of a declared radius, and nothing else.
    pub(super) const BALL: &str = r#"
proc ball {
  kind Field

  param radius : float [0.1, 3.0] = 0.8

  field {
    distance = sd_sphere(point, radius);
  }
}
"#;

    /// **A second shape, offset from the first along the up axis**, so that "which
    /// field" is a question with two different answers on screen. Up rather than
    /// across, because the orbit camera looks along a horizontal axis: a shape
    /// offset that way sits behind the first and adds no silhouette to count. It declares `radius` as well,
    /// deliberately: two fields sharing a param name is the case an address has to
    /// survive, and `Field:0:radius` and `Field:1:radius` are two numbers because
    /// they are two procedures.
    const SHELL: &str = r#"
proc shell {
  kind Field

  param radius : float [0.1, 3.0] = 0.5

  field {
    distance = sd_sphere(point + vec3(0.0, 1.0, 0.0), radius);
  }
}
"#;

    /// A marcher with two slots: a shape and something cut out of it. It names
    /// neither field, which is the point — the Set says which is which.
    const CARVE: &str = r#"
proc lens {
  kind  L4
  blend additive

  uses shape  : Field
  uses cutter : Field

  fragment {
    var p = eye;
    var hit = 0.0;
    for i in 0..40 {
      let d = max(shape(p), -cutter(p));
      if d < 0.005 {
        hit = 1.0;
      }
      p = p + ray * max(d, 0.005);
    }
    color = vec4(hit, hit, hit, 1.0);
  }
}
"#;

    /// Marches whatever the Set gives it. **It contains no shape at all**, which is
    /// the whole claim: the same renderer draws any field — and it says which field
    /// by declaring a slot the Set binds, rather than by naming a word the language
    /// reserved.
    pub(super) const LENS: &str = r#"
proc lens {
  kind  L4
  blend additive

  uses shape : Field

  fragment {
    var p = eye;
    var hit = 0.0;
    for i in 0..40 {
      let d = shape(p);
      if d < 0.005 {
        hit = 1.0;
      }
      p = p + ray * max(d, 0.005);
    }
    color = vec4(hit, hit, hit, 1.0);
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

    /// One edge, pointing with the names nobody wrote: a node nothing names is
    /// called what its procedure declares, so the renderer is `lens` and the field
    /// is whatever its own `proc` line says.
    pub(super) fn edge(node: &str, slot: &str, to: &str) -> karakuri_engine::set::Edge {
        karakuri_engine::set::Edge {
            node: node.to_string(),
            slot: slot.to_string(),
            to: to.to_string(),
        }
    }

    /// The name a `.kir` gives its procedure, which is the name its node ends up
    /// with here — read off the source rather than repeated, so a test that swaps
    /// the field swaps what the edge points at.
    pub(super) fn proc_name(src: &str) -> String {
        compile(src).name
    }

    fn build(gpu: &Gpu, field: Option<&str>, l4: &str) -> Result<Set, SetError> {
        let edges: Vec<karakuri_engine::set::Edge> = field
            .map(|f| vec![edge("lens", "shape", &proc_name(f))])
            .unwrap_or_default();
        build_wired(gpu, field.as_slice(), l4, &edges)
    }

    /// **A list of fields**, because a Set holds as many as it is given — and one
    /// renderer, because every claim here is about what a caller draws.
    fn build_wired(
        gpu: &Gpu,
        fields: &[&str],
        l4: &str,
        edges: &[karakuri_engine::set::Edge],
    ) -> Result<Set, SetError> {
        build_many_wired(gpu, fields, &[l4], edges)
    }

    fn build_many_wired(
        gpu: &Gpu,
        fields: &[&str],
        l4s: &[&str],
        edges: &[karakuri_engine::set::Edge],
    ) -> Result<Set, SetError> {
        let fields: Vec<Checked> = fields.iter().map(|f| compile(f)).collect();
        let l4s: Vec<Checked> = l4s.iter().map(|l4| compile(l4)).collect();
        let mut set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&compile(STILL), 1)],
            &[],
            &[],
            &fields.iter().collect::<Vec<_>>(),
            &l4s.iter().collect::<Vec<_>>(),
            Layering::Overdraw,
            7,
            &[],
            karakuri_engine::set::Wiring {
                edges,
                ..Default::default()
            },
        )?;
        set.resize(&gpu.device, W, H);
        set.aim_camera(karakuri_engine::camera::Orbit {
            radius: 5.0,
            speed: 0.0,
            height: 0.0,
            ..Default::default()
        });
        Ok(set)
    }

    /// How many texels the marcher hit, which is the figure's area on screen.
    fn covered(gpu: &Gpu, set: &mut Set) -> usize {
        let present = Present::new(&gpu.device, wgpu::TextureFormat::Rgba16Float, W, H);
        set.prepare(&gpu.queue, 1, &Signals::default());
        let bytes_per_row = W * 8;
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
        let out = data
            .chunks_exact(8)
            .filter(|t| f16(u16::from_le_bytes([t[0], t[1]])) > 0.01)
            .count();
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

    /// **A renderer with no shape in it draws a shape**, because the Set gave it
    /// one. That is the whole feature.
    #[test]
    fn a_marcher_draws_a_field_it_does_not_contain() {
        let gpu = Gpu::headless().expect("a GPU");
        let mut set = build(&gpu, Some(BALL), LENS).expect("a Set with a field");
        let hit = covered(&gpu, &mut set);
        assert!(
            hit > 100,
            "the marcher should have found the sphere, and covered {hit} texels"
        );
        assert!(
            hit < (W * H) as usize,
            "and should not have filled the frame"
        );
    }

    /// **The field's own `param` is an operator's to ride**, addressed by its kind
    /// even though it addresses no node — every caller writes the same value into
    /// its own uniform.
    #[test]
    fn a_fields_param_reaches_every_caller() {
        let gpu = Gpu::headless().expect("a GPU");
        let mut small = build(&gpu, Some(BALL), LENS).expect("a Set with a field");
        let mut large = build(&gpu, Some(BALL), LENS).expect("a Set with a field");
        assert!(
            large.set_param_at(karakuri_ir::Kind::Field, 0, "radius", 2.0),
            "`Field:0:radius` has to reach the field's map"
        );

        let (a, b) = (covered(&gpu, &mut small), covered(&gpu, &mut large));
        assert!(
            b > a * 2,
            "a radius of 2.0 should cover much more than 0.8 does: {b} against {a}"
        );
    }

    /// **A procedure that takes a field and a Set that binds it to nothing is
    /// refused by name.** The call lowers to a function name, so a module without
    /// it is WGSL naga refuses — a panic on the thread that built it, from a `.kir`
    /// the checker accepted, which is the one shape a composition check exists to
    /// prevent.
    ///
    /// **Refused at the declaration rather than at the call**, which is the
    /// difference the slot makes: the sentence names the slot the file declared and
    /// the flag that would bind it, where the one it replaced could only say that
    /// some procedure somewhere evaluated a field.
    /// **A slot bound to something that is not a field is refused**, and the
    /// refusal says what the node it names actually is.
    ///
    /// The alternative is a Set that builds a renderer calling a function the
    /// bound node never produced — a geometry has no `_field_shape_at` to splice,
    /// and nothing below here would notice before naga did.
    /// **Two Field slots on one procedure are accepted**, which is the rule a
    /// geometry slot does not follow and the reason the two are told apart by type.
    ///
    /// A marcher wanting a shape and a cutter is the ordinary case, and a field has
    /// no node for a second one to need: each slot is another copy of a body under
    /// another name, with params of its own. Both are bound to the same field here
    /// because a Set holds one — what is being claimed is the *notation*, and that
    /// two names reach it independently.
    #[test]
    fn two_field_slots_on_one_procedure_are_accepted() {
        let gpu = Gpu::headless().expect("a GPU");
        let two = r#"
proc lens {
  kind  L4
  blend additive

  uses shape  : Field
  uses cutter : Field

  fragment {
    var p = eye;
    var hit = 0.0;
    for i in 0..40 {
      let d = max(shape(p), -cutter(p + vec3(0.4, 0.0, 0.0)));
      if d < 0.005 {
        hit = 1.0;
      }
      p = p + ray * max(d, 0.005);
    }
    color = vec4(hit, hit, hit, 1.0);
  }
}
"#;
        let edges = vec![
            edge("lens", "shape", "ball"),
            edge("lens", "cutter", "ball"),
        ];
        let mut set = build_wired(&gpu, &[BALL], two, &edges).expect("two slots, one field");
        let hit = covered(&gpu, &mut set);
        assert!(
            hit > 0 && hit < (W * H) as usize,
            "the two together carve a figure, and covered {hit} texels"
        );

        // **The two are addressed apart**, which is what the slot in the name is
        // for. Both reach one field here, so both read that field's values — one
        // procedure is one set of numbers however many names reach it — and the
        // picture is the proof that each name resolved to a body of its own.
        assert!(
            !set.node_names().is_empty(),
            "and the Set built rather than collapsing the two names into one"
        );
    }

    /// **Two fields in one Set, one per slot on one renderer**, which is the thing
    /// this commit is for: the language stopped capping a Set at one field and the
    /// plumbing went on doing it, an `Option` at a time.
    ///
    /// **Both declare `radius`**, on purpose. That is the case an address has to
    /// survive — `Field:0:radius` and `Field:1:radius` are two numbers because they
    /// are two procedures — and it is the one a param keyed by declared name alone
    /// could not tell apart.
    ///
    /// The two assertions are each other's control. Growing the shape must grow the
    /// figure and growing the cutter must swallow it, so a Set that wrote one value
    /// into both slots fails whichever way round it got them.
    #[test]
    fn two_fields_reach_one_renderer_through_two_slots() {
        let gpu = Gpu::headless().expect("a GPU");
        let edges = vec![
            edge("lens", "shape", "ball"),
            edge("lens", "cutter", "shell"),
        ];
        let carved = |gpu: &Gpu| {
            build_wired(gpu, &[BALL, SHELL], CARVE, &edges).expect("two fields, two slots")
        };

        let mut base = carved(&gpu);
        let plain = covered(&gpu, &mut base);
        assert!(
            plain > 100,
            "the shape is there and the cutter misses its silhouette: {plain} texels"
        );

        let mut grown = carved(&gpu);
        assert!(
            grown.set_param_at(karakuri_ir::Kind::Field, 0, "radius", 2.0),
            "`Field:0:radius` has to reach the first field"
        );
        let bigger = covered(&gpu, &mut grown);
        assert!(
        bigger > plain * 2,
        "the first field is the shape, so growing it grows the figure: {bigger} against {plain}"
    );

        let mut cut = carved(&gpu);
        assert!(
            cut.set_param_at(karakuri_ir::Kind::Field, 1, "radius", 2.0),
            "`Field:1:radius` has to reach the second field"
        );
        let swallowed = covered(&gpu, &mut cut);
        assert!(
            swallowed < plain / 4,
            "the second field is the cutter, and at 2.0 it contains the shape: {swallowed} \
         against {plain}"
        );
    }

    /// **Two nodes, a field each**, which is the half a single renderer cannot
    /// show: the two slots are both called `shape`, so nothing about the *key* a
    /// value is written under says which field it came from — only the node does.
    ///
    /// A Set that resolved a param by slot spelling alone would hand both renderers
    /// one field's numbers, and the picture would be right for whichever of them
    /// happened to be asked first.
    #[test]
    fn two_nodes_each_reach_the_field_their_own_edge_names() {
        let gpu = Gpu::headless().expect("a GPU");
        // `lens` and `lens-2`: two uses of one procedure are two nodes, and each
        // one's slot is bound on its own.
        let edges = vec![
            edge("lens", "shape", "ball"),
            edge("lens-2", "shape", "shell"),
        ];
        let mixed = |gpu: &Gpu| {
            build_many_wired(gpu, &[BALL, SHELL], &[LENS, LENS], &edges).expect("a field each")
        };

        let mut both = mixed(&gpu);
        let plain = covered(&gpu, &mut both);
        let mut alone = build(&gpu, Some(BALL), LENS).expect("one renderer over the first field");
        assert!(
            plain > covered(&gpu, &mut alone),
            "the second renderer draws a shape the first one does not: {plain} texels"
        );

        // **Each address moves its own node and only its own.** If both nodes read
        // one field, one of these two is a Set nothing changed.
        let mut first = mixed(&gpu);
        assert!(first.set_param_at(karakuri_ir::Kind::Field, 0, "radius", 2.0));
        assert!(
            covered(&gpu, &mut first) > plain,
            "`Field:0` has to reach `lens`, which is bound to the first field"
        );

        let mut second = mixed(&gpu);
        assert!(second.set_param_at(karakuri_ir::Kind::Field, 1, "radius", 1.5));
        assert!(
            covered(&gpu, &mut second) > plain,
            "`Field:1` has to reach `lens-2`, which is bound to the second — and a Set that \
         resolved by slot name alone would have written it into neither"
        );
    }

    /// **Two renderers in one Set read the same field value.** A field has no node,
    /// so nothing owns the value: every caller writes it into its own uniform, and
    /// "the same answer everywhere" is a property of the Set rather than of any one
    /// of them.
    ///
    /// Both renderers draw the same figure, so they agree exactly or not at all —
    /// and the assertion is against a *third* Set at a different radius, so two
    /// renderers agreeing on the wrong number cannot pass.
    #[test]
    fn two_renderers_in_one_set_agree_on_the_fields_value() {
        let gpu = Gpu::headless().expect("a GPU");
        let field = compile(BALL);
        let l4 = compile(LENS);

        let mut both = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&compile(STILL), 1)],
            &[],
            &[],
            &[&field],
            &[&l4, &l4],
            Layering::Overdraw,
            7,
            &[],
            karakuri_engine::set::Wiring {
                // **Both, and separately.** Two uses of one procedure are two nodes
                // — `lens` and `lens-2` — and a slot is bound per node, so "the same
                // field everywhere" is a thing the Set says twice rather than a
                // thing it assumes.
                edges: &[
                    edge("lens", "shape", "ball"),
                    edge("lens-2", "shape", "ball"),
                ],
                ..Default::default()
            },
        )
        .expect("two renderers over one field");
        both.resize(&gpu.device, W, H);
        both.aim_camera(karakuri_engine::camera::Orbit {
            radius: 5.0,
            speed: 0.0,
            height: 0.0,
            ..Default::default()
        });
        assert!(both.set_param_at(karakuri_ir::Kind::Field, 0, "radius", 2.0));

        let mut one = build(&gpu, Some(BALL), LENS).expect("one renderer");
        assert!(one.set_param_at(karakuri_ir::Kind::Field, 0, "radius", 2.0));

        // Additive, so two renderers drawing the same figure cover the same texels
        // — the count is the figure's area, not its brightness.
        let (a, b) = (covered(&gpu, &mut both), covered(&gpu, &mut one));
        assert_eq!(a, b, "both renderers found the same sphere");

        let mut small = build(&gpu, Some(BALL), LENS).expect("one renderer");
        assert!(
            covered(&gpu, &mut small) < b / 2,
            "and the default radius covers much less, so the agreement above is not agreement on \
         a value nothing set"
        );
    }

    /// **A field's `param` is an operator's on every surface, not just `--param`.**
    /// It has no node, and five separate loops over the layers decided which
    /// surfaces reach it — `Field` was in none of them, so a value could be
    /// overridden and then not bound, published, read back or saved.
    #[test]
    fn a_fields_param_reaches_every_operator_surface() {
        let gpu = Gpu::headless().expect("a GPU");
        let mut set = build(&gpu, Some(BALL), LENS).expect("a Set with a field");

        assert!(
            set.bind(karakuri_engine::Binding::new(
                karakuri_ir::Kind::Field,
                "radius",
                "energy",
                karakuri_engine::Curve::Lin,
                [0.3, 2.0],
            ))
            .attached(),
            "a signal has to be attachable to a field's param"
        );

        set.publish(karakuri_engine::set::Published {
            name: "size".to_string(),
            at: Some((karakuri_ir::Kind::Field, 0)),
            key: "radius".to_string(),
            range: [0.3, 2.0],
        })
        .expect("a field's param has to be publishable");

        assert!(
            set.published().iter().any(|p| p.key == "radius"),
            "and the published interface has to show it: {:?}",
            set.published()
        );
    }
}

/// **The refusals, which reach no device.**
///
/// A slot nothing binds, a slot bound to the wrong kind of node, and a caller
/// too expensive with its field inlined are all decided before a pipeline
/// exists — and were only reachable through a constructor that took one.
/// `Set::validate` is that check pass on its own, and `Set::build_many` reaches
/// these rules by calling it rather than by holding a copy.
mod refused {
    use super::gpu::{compile, edge, proc_name, BALL, LENS, STILL};
    use karakuri_engine::set::{Edge, Layering, SetError, Wiring};
    use karakuri_engine::Set;
    use karakuri_ir::typed::Checked;

    /// The Set `build_many_wired` above describes — one `STILL` at capacity 1,
    /// these fields, these renderers — minus the device and everything
    /// downstream of it.
    fn validate_wired(fields: &[&str], l4s: &[&str], edges: &[Edge]) -> Result<(), SetError> {
        let fields: Vec<Checked> = fields.iter().map(|f| compile(f)).collect();
        let l4s: Vec<Checked> = l4s.iter().map(|l4| compile(l4)).collect();
        Set::validate(
            &[(&compile(STILL), 1)],
            &[],
            &[],
            &fields.iter().collect::<Vec<_>>(),
            &l4s.iter().collect::<Vec<_>>(),
            Layering::Overdraw,
            7,
            &[],
            Wiring {
                edges,
                ..Default::default()
            },
        )
        .map(|_| ())
    }

    /// One renderer over the field its own edge names, which is how every
    /// accepting test in this file is wired.
    fn validate(field: Option<&str>, l4: &str) -> Result<(), SetError> {
        let edges: Vec<Edge> = field
            .map(|f| vec![edge("lens", "shape", &proc_name(f))])
            .unwrap_or_default();
        validate_wired(field.as_slice(), &[l4], &edges)
    }

    /// **A `.kir` that checks clean and a Set that cannot be built.** A marcher
    /// evaluating a field nothing bound would compile a call to a function
    /// nothing spliced in: it is WGSL naga refuses — a panic on the thread that
    /// built it, from a `.kir` the checker accepted, which is the one shape a
    /// composition check exists to prevent.
    ///
    /// **Refused at the declaration rather than at the call**, which is the
    /// difference the slot makes: the sentence names the slot the file declared
    /// and the flag that would bind it, where the one it replaced could only say
    /// that some procedure somewhere evaluated a field.
    #[test]
    fn an_unbound_field_slot_is_refused_rather_than_fatal() {
        let err = validate_wired(&[], &[LENS], &[]).expect_err("nothing fills `lens.shape`");
        let text = err.to_string();
        assert!(
            text.contains("lens") && text.contains("shape") && text.contains("Field"),
            "{text}"
        );

        // And a Set that *holds* a field still refuses one that says nothing about
        // which: "there is exactly one, so use it" is the rule the slot removes.
        let err = validate_wired(&[BALL], &[LENS], &[])
            .expect_err("an unbound slot is not filled in from what is lying around");
        assert!(err.to_string().contains("shape"), "{err}");
    }

    /// **A slot bound to something that is not a field is refused**, and the
    /// refusal says what the node it names actually is.
    ///
    /// The alternative is a Set that builds a renderer calling a function the
    /// bound node never produced — a geometry has no `_field_shape_at` to splice,
    /// and nothing below here would notice before naga did.
    #[test]
    fn a_field_slot_bound_to_something_that_is_not_a_field_is_refused() {
        let to_the_l1 = vec![edge("lens", "shape", "still")];
        let err = validate_wired(&[BALL], &[LENS], &to_the_l1).expect_err("an L1 is not a field");
        let text = err.to_string();
        assert!(
            text.contains("still") && text.contains("an L1") && text.contains("Field"),
            "{text}"
        );
    }

    /// **Neither file is over budget and the pair is**, which is what a Set-level
    /// check exists for: a `field(p)` weighs nothing where a single procedure is
    /// estimated, so the ceiling each of them passed was applied to a figure
    /// missing the other.
    #[test]
    fn a_caller_and_its_field_are_costed_together() {
        // Expensive on its own terms and nowhere near any ceiling: a field has no
        // ceiling of its own, precisely because what it costs is the caller's.
        let heavy = r#"
proc heavy {
  kind Field

  field {
    var d = 0.0;
    for i in 0..64 {
      d = d + fbm(point, 4) * 0.01;
    }
    distance = length(point) - 1.0 + d;
  }
}
"#;
        let cost = karakuri_ir::cost::estimate(&compile(heavy)).expect("a field has no ceiling");
        assert!(
            cost.ops_per_evaluation > 0,
            "the field costs something per evaluation"
        );

        let err = validate(Some(heavy), LENS)
            .expect_err("forty evaluations of an expensive field is over the fragment ceiling");
        let text = err.to_string();
        assert!(
            text.contains("heavy") && text.contains("lens") && text.contains("evaluations"),
            "the refusal names both and says how many evaluations: {text}"
        );
    }
}
