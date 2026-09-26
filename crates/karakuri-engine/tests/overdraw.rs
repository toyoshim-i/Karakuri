//! Pixel-level integration tests for overdraw rendering (multiple L4 passes over shared geometry).
//!
//! Asserts that multiple L4 renderers composited onto a single target preserve previous pass contents,
//! maintain separate parameter scopes, and execute geometry simulation exactly once.

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
#[path = "common/mod.rs"]
mod common;

mod gpu {
    pub(super) use super::common::compile;
    use super::common::f16;
    use karakuri_engine::binding::{Binding, Curve};
    use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
    use karakuri_ir::typed::Checked;
    use karakuri_ir::Kind;

    const W: u32 = 64;
    const H: u32 = 64;

    /// Two elements at fixed positions, no spawning, no motion — the picture is a
    /// pure function of `seed`, so every Set below gets literally identical
    /// geometry however many renderers read it.
    pub(super) const PAIR_L1: &str = r#"
proc pair {
  kind     L1
  topology points
  capacity [2, 2] = 2

  emit position

  element {
    var x = 0.0;
    if seed == 1u {
      x = 3.0;
    }
    position = vec3(x, 0.0, 0.0);
  }
}
"#;

    /// Two-element geometry accumulating position each step to verify step counts.
    const CREEP_L1: &str = r#"
proc creep {
  kind     L1
  topology points
  capacity [2, 2] = 2

  emit position, velocity

  element {
    var x = 0.0;
    if seed == 1u {
      x = 0.5;
    }
    velocity = vec3(0.25 + x, 0.0, 0.0);
    position = position + velocity * dt;
  }
}
"#;

    /// Generates an L4 sprite procedure with distinct color, scale, and exposure parameters.
    fn sprite(name: &str, rgb: (f32, f32, f32), scale: f32, exposure: f32) -> String {
        let (r, g, b) = rgb;
        let scale = scale / H as f32;
        format!(
            r#"
proc {name} {{
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = {exposure:?}

  vertex {{
    clip       = camera * vec4(position, 1.0);
    point_rate = {scale:?};
  }}

  fragment {{
    let d = length(point_coord * 2.0 - 1.0);
    let a = max(0.0, 1.0 - d);
    color = vec4({r:?} * exposure, {g:?} * exposure, {b:?} * exposure, a);
  }}
}}
"#
        )
    }

    /// A Set over [`PAIR_L1`] with `l4s` as its renderers, in draw order.
    fn build(gpu: &Gpu, l4s: &[&str]) -> Set {
        build_over(gpu, PAIR_L1, l4s)
    }

    fn build_over(gpu: &Gpu, l1: &str, l4s: &[&str]) -> Set {
        let compiled: Vec<Checked> = l4s.iter().map(|s| compile(s)).collect();
        let refs: Vec<&Checked> = compiled.iter().collect();
        let mut set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&compile(l1), 2)],
            &[],
            &[],
            &[],
            &refs,
            karakuri_engine::set::Layering::Overdraw,
            3,
            &[],
            karakuri_engine::set::Wiring::default(),
        )
        .expect("one L1 and however many renderers over it");
        set.resize(&gpu.device, W, H);
        set
    }

    /// RGBA f32 per texel, after one step.
    fn draw(gpu: &Gpu, set: &mut Set) -> Vec<[f32; 4]> {
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
        set.commit();

        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let data = slice.get_mapped_range().expect("map");
        let out: Vec<[f32; 4]> = data
            .chunks_exact(8)
            .map(|t| {
                let h = |i: usize| f16(u16::from_le_bytes([t[i], t[i + 1]]));
                [h(0), h(2), h(4), h(6)]
            })
            .collect();
        drop(data);
        readback.unmap();
        out
    }

    /// `f16` bits to `f32`, written out rather than pulled in as a dependency —
    /// the same way `tests/weighted.rs` and `tests/lines.rs` do it.
    fn channel_sums(px: &[[f32; 4]]) -> [f32; 4] {
        px.iter().fold([0.0; 4], |mut acc, t| {
            for i in 0..4 {
                acc[i] += t[i];
            }
            acc
        })
    }

    const RED: (f32, f32, f32) = (1.0, 0.0, 0.0);
    const GREEN: (f32, f32, f32) = (0.0, 1.0, 0.0);

    /// The two renderers used throughout: different colours, different sizes, and
    /// Distinct default values for shared parameter names.
    fn pair() -> (String, String) {
        (
            sprite("red", RED, 9.0, 1.0),
            sprite("green", GREEN, 17.0, 0.25),
        )
    }

    // ---------------------------------------------------------------------------

    /// Verifies that multiple L4 renderers correctly composite both color (additive sum) and alpha (coverage union).
    #[test]
    fn a_stack_of_two_renderers_composes_what_each_of_them_draws_alone() {
        let gpu = Gpu::headless().expect("no GPU available");
        let (a, b) = pair();

        let red = draw(&gpu, &mut build(&gpu, &[&a]));
        let green = draw(&gpu, &mut build(&gpu, &[&b]));
        let stacked = draw(&gpu, &mut build(&gpu, &[&a, &b]));

        let (sr, sg) = (channel_sums(&red), channel_sums(&green));
        let total = channel_sums(&stacked);
        assert!(
            sr[0] > 1.0 && sg[1] > 1.0,
            "a lone renderer drew nothing to compare against"
        );
        // Measured in coverage, not in colour: the two declare `exposure` at
        // different defaults, so their colour sums say nothing about their areas.
        assert!(
            sg[3] > sr[3] * 1.5,
            "the two renderers must cover different areas for this to say anything: \
         coverage {} against {}",
            sg[3],
            sr[3]
        );

        // Colour: a sum.
        for (i, name) in ["r", "g", "b"].iter().enumerate() {
            let want = sr[i] + sg[i];
            let slack = 0.02 * want.max(1.0);
            assert!(
                (total[i] - want).abs() <= slack,
                "channel {name}: the stack summed to {} where the two alone sum to {want} — \
             a second pass that cleared would give {}",
                total[i],
                sg[i]
            );
        }

        // Coverage: the union, per texel, exactly.
        let mut overlapping = 0;
        for (i, ((r, g), s)) in red.iter().zip(&green).zip(&stacked).enumerate() {
            let want = r[3] + g[3] * (1.0 - r[3]);
            if r[3] > 0.05 && g[3] > 0.05 {
                overlapping += 1;
            }
            assert!(
                (s[3] - want).abs() <= 1e-2,
                "texel {i}: coverage came out {} where the union of {} and {} is {want}",
                s[3],
                r[3],
                g[3]
            );
        }
        assert!(
            overlapping > 8,
            "the two sprites barely overlap ({overlapping} texels), so the union says little"
        );
    }

    /// Asserts that adding multiple renderers adds draw passes without redundant
    /// simulation steps on shared geometry.
    #[test]
    fn a_second_renderer_costs_a_pass_and_not_a_simulation() {
        let gpu = Gpu::headless().expect("no GPU available");
        let (a, b) = pair();

        let mut one = build_over(&gpu, CREEP_L1, &[&a]);
        let mut three = build_over(&gpu, CREEP_L1, &[&a, &b, &a]);
        for _ in 0..5 {
            draw(&gpu, &mut one);
            draw(&gpu, &mut three);
        }

        // The material moved at all, or the comparison is between two zeroes.
        let layout = one.element_layout();
        let (stride, at) = (
            layout.stride as usize,
            layout.offset_of("position") as usize,
        );
        let x_of = |bytes: &[u8], i: usize| {
            let o = i * stride + at;
            f32::from_le_bytes(
                bytes[o..o + 4]
                    .try_into()
                    .expect("four bytes of position.x"),
            )
        };
        let single = one.read_elements(&gpu.device, &gpu.queue);
        assert!(
            x_of(&single, 0) > 0.01,
            "the material did not accumulate, so nothing is under test"
        );

        assert_eq!(
            single,
            three.read_elements(&gpu.device, &gpu.queue),
            "three renderers over one geometry simulated it a different number of times: \
         element 0 reached {} against {}",
            x_of(&single, 0),
            x_of(&three.read_elements(&gpu.device, &gpu.queue), 0)
        );
        assert_eq!(
            one.live_count(&gpu.device, &gpu.queue),
            three.live_count(&gpu.device, &gpu.queue)
        );
    }

    /// Verifies that draw order commutativity holds for purely additive blend modes without visual change.
    #[test]
    fn swapping_two_additive_renderers_does_not_change_the_frame() {
        let gpu = Gpu::headless().expect("no GPU available");
        let (a, b) = pair();

        let forwards = draw(&gpu, &mut build(&gpu, &[&a, &b]));
        let backwards = draw(&gpu, &mut build(&gpu, &[&b, &a]));

        assert!(
            channel_sums(&forwards)[1] > 1.0,
            "the material drew nothing, so the comparison is vacuous"
        );
        for (i, (f, r)) in forwards.iter().zip(&backwards).enumerate() {
            for c in 0..4 {
                assert!(
                    (f[c] - r[c]).abs() <= 1e-3,
                    "texel {i} channel {c} moved when two additive renderers were swapped: \
                 {} against {}",
                    f[c],
                    r[c]
                );
            }
        }
    }

    /// Verifies that identically-named parameters across renderers maintain separate scopes and defaults.
    #[test]
    fn one_name_declared_by_two_renderers_is_two_values_each_reaching_its_own() {
        let gpu = Gpu::headless().expect("no GPU available");
        let (a, b) = pair();

        let mut plain = build(&gpu, &[&a, &b]);
        let mut brighter = build(&gpu, &[&a, &b]);

        let mut declared: Vec<f32> = plain
            .params()
            .filter(|(_, _, name, _)| *name == "exposure")
            .map(|(_, _, _, value)| value)
            .collect();
        declared.sort_by(|x, y| x.partial_cmp(y).expect("no NaN"));
        assert_eq!(
            declared,
            vec![0.25, 1.0],
            "the two declarations of `exposure` did not survive as two values"
        );

        assert_eq!(
            brighter.set_param("exposure", 2.0),
            2,
            "a bare name must reach both renderers that declare it"
        );

        let base = channel_sums(&draw(&gpu, &mut plain));
        let raised = channel_sums(&draw(&gpu, &mut brighter));
        for (i, (name, factor)) in [("r", 2.0 / 1.0), ("g", 2.0 / 0.25)].iter().enumerate() {
            assert!(
                base[i] > 1.0,
                "channel {name} drew nothing at its default exposure"
            );
            assert!(
                (raised[i] - factor * base[i]).abs() <= 0.02 * factor * base[i],
                "channel {name}: `exposure = 2.0` gave {} where {factor} times {} was due — \
             that renderer read another node's value",
                raised[i],
                base[i]
            );
        }
    }

    /// Verifies that bindings resolve their base value from the renderer declaring that parameter.
    #[test]
    fn a_binding_blends_from_a_node_that_declares_the_name_not_the_first_one() {
        let gpu = Gpu::headless().expect("no GPU available");
        let plain = sprite("red", RED, 9.0, 1.0);
        // The same fixture with one extra param, declared here and nowhere else.
        let extra = sprite("green", GREEN, 17.0, 0.25).replace(
            "  param exposure",
            "  param spread : float [0.0, 8.0] = 3.0\n  param exposure",
        );

        let mut set = build(&gpu, &[&plain, &extra]);
        assert!(
            set.bind(Binding::new(
                Kind::L4,
                "spread",
                "nothing_measures_this",
                Curve::Lin,
                [0.0, 100.0]
            ))
            .attached(),
            "`spread` is declared by one of this Set's renderers"
        );
        set.prepare(&gpu.queue, 1, &Signals::default());

        let resolved = set
            .bindings()
            .iter()
            .find(|b| b.key == "spread")
            .expect("the binding is attached")
            .value();
        assert_eq!(
            resolved, 3.0,
            "the binding blended from a map that has no `spread`, so the renderer's \
         declared default became {resolved}"
        );
    }

    /// An opaque, flat sprite under `blend weighted`. Its resolve composites `over`
    /// what is under it rather than replacing a clear, which is the whole of what a
    /// weighted node in a stack has to get right.
    /// `scale` converts the same way [`sprite`]'s does.
    fn weighted_sprite(name: &str, rgb: (f32, f32, f32), scale: f32, alpha: f32) -> String {
        let (r, g, b) = rgb;
        let scale = scale / H as f32;
        format!(
            r#"
proc {name} {{
  kind  L4
  blend weighted

  consumes position

  vertex {{
    clip       = camera * vec4(position, 1.0);
    point_rate = {scale:?};
  }}

  fragment {{
    color = vec4({r:?}, {g:?}, {b:?}, {alpha:?});
  }}
}}
"#
        )
    }

    /// Verifies that weighted renderers composite over previous passes according to draw order.
    #[test]
    fn a_weighted_node_in_a_stack_composites_over_what_is_under_it() {
        let gpu = Gpu::headless().expect("no GPU available");
        let under = sprite("red", RED, 25.0, 1.0);
        let over = weighted_sprite("veil", GREEN, 25.0, 0.94);

        let red_alone = channel_sums(&draw(&gpu, &mut build(&gpu, &[&under])));
        let veil_over_red = draw(&gpu, &mut build(&gpu, &[&under, &over]));
        let red_over_veil = draw(&gpu, &mut build(&gpu, &[&over, &under]));

        let (a, b) = (channel_sums(&veil_over_red), channel_sums(&red_over_veil));
        assert!(
            red_alone[0] > 1.0,
            "the material under the veil drew nothing"
        );

        // The veil at alpha 0.94 preserves non-zero underlying content;
        // verifies that resolve loads rather than clears the target.
        assert!(
            a[0] < red_alone[0] * 0.25,
            "a weighted node drawn second did not cover what was under it: {} of {}",
            a[0],
            red_alone[0]
        );
        // Measured at 1.4%, not the 6% one fragment of `alpha = 0.94` would leave:
        // the two sprites overlap, and two fragments give `1 - 0.06²`, so most of
        // the red sits under two veils rather than one. The floor is well under
        // that and well above the zero value left by a clear pass.
        assert!(
            a[0] > red_alone[0] * 0.005,
            "a weighted node drawn second erased what was under it rather than \
         compositing over it: {} of {} left",
            a[0],
            red_alone[0]
        );
        // Drawn first, it is under the red and hides nothing.
        assert!(
            b[0] > red_alone[0] * 0.9,
            "a weighted node drawn first swallowed the renderer above it: {} of {}",
            b[0],
            red_alone[0]
        );
        assert!(
            b[0] > a[0] * 2.0,
            "swapping a weighted node with an additive one changed nothing, so the \
         resolve is replacing rather than compositing"
        );
    }

    /// Verifies that a weighted node drawn as the first pass clears the target buffer.
    #[test]
    fn a_weighted_node_drawn_first_clears_what_was_in_the_target() {
        let gpu = Gpu::headless().expect("no GPU available");
        let veil = weighted_sprite("veil", GREEN, 25.0, 0.94);

        let alone = channel_sums(&draw(&gpu, &mut build(&gpu, &[&veil])));
        // Two frames from one Set: the second must not accumulate onto the first.
        let mut twice = build(&gpu, &[&veil]);
        draw(&gpu, &mut twice);
        let second = channel_sums(&draw(&gpu, &mut twice));

        assert!(alone[1] > 1.0, "the veil drew nothing");
        assert!(
            (second[1] - alone[1]).abs() <= 0.02 * alone[1],
            "a second frame came out at {} where the first was {} — the target was not cleared",
            second[1],
            alone[1]
        );
    }

    /// Verifies that simulation runs if any renderer consumes element buffers in a mixed stack.
    #[test]
    fn a_mixed_stack_still_simulates_because_one_renderer_reads_the_elements() {
        let gpu = Gpu::headless().expect("no GPU available");
        // L4 fullscreen passes omit vertex blocks and consumes declarations.
        // were the only one.
        let marcher = r#"
proc wash {
  kind  L4
  blend additive

  fragment {
    color = vec4(0.02, 0.0, 0.04, 1.0);
  }
}
"#;
        let sprites = sprite("red", RED, 9.0, 1.0);

        let mut per_element = build_over(&gpu, CREEP_L1, &[&sprites]);
        let mut mixed = build_over(&gpu, CREEP_L1, &[marcher, &sprites]);
        for _ in 0..4 {
            draw(&gpu, &mut per_element);
            draw(&gpu, &mut mixed);
        }

        assert_eq!(
            per_element.read_elements(&gpu.device, &gpu.queue),
            mixed.read_elements(&gpu.device, &gpu.queue),
            "a stack with one fullscreen renderer in it stopped simulating for the other"
        );
    }

    /// Verifies that index-addressed parameter writes target specific renderers in the stack.
    #[test]
    fn an_addressed_write_reaches_one_renderer_and_leaves_the_other() {
        let gpu = Gpu::headless().expect("no GPU available");
        let (a, b) = pair();

        let base = channel_sums(&draw(&gpu, &mut build(&gpu, &[&a, &b])));
        let mut split = build(&gpu, &[&a, &b]);
        assert!(
            split.set_param_at(Kind::L4, 1, "exposure", 1.0),
            "renderer 1 declares `exposure`"
        );

        let after = channel_sums(&draw(&gpu, &mut split));
        // Renderer 1 went from 0.25 to 1.0 — four times. Renderer 0 was not
        // addressed and must not have moved at all.
        assert!(
            (after[1] - 4.0 * base[1]).abs() <= 0.02 * 4.0 * base[1],
            "the addressed renderer went to {} where four times {} was due",
            after[1],
            base[1]
        );
        assert!(
            (after[0] - base[0]).abs() <= 0.02 * base[0],
            "an addressed write reached a renderer it did not name: {} was {}",
            after[0],
            base[0]
        );
    }

    /// Unbound node address targets are rejected with explicit refusals.
    #[test]
    fn an_address_past_the_end_of_the_stack_is_refused() {
        let gpu = Gpu::headless().expect("no GPU available");
        let (a, b) = pair();
        let mut set = build(&gpu, &[&a, &b]);

        assert!(set.set_param_at(Kind::L4, 1, "exposure", 2.0));
        assert!(
            !set.set_param_at(Kind::L4, 2, "exposure", 2.0),
            "there is no third renderer"
        );
        assert!(!set.set_param_at(Kind::L4, 0, "nothing_declares_this", 2.0));
        // The L1 is one node, so only 0 addresses it — and `pair`'s L1 declares no
        // `exposure`, which is what makes this a claim about the address rather
        // than about the name.
        assert!(!set.set_param_at(Kind::L1, 0, "exposure", 2.0));

        // A binding is addressed on the same terms.
        let bind_at = |i: u32| {
            Binding::new(
                Kind::L4,
                "exposure",
                "nothing_measures_this",
                Curve::Lin,
                [0.0, 8.0],
            )
            .at(i)
        };
        assert!(
            set.bind(bind_at(1)).attached(),
            "renderer 1 declares `exposure`"
        );
        assert!(
            !set.bind(bind_at(9)).attached(),
            "there is no tenth renderer to bind into"
        );
    }

    /// Asserts that addressed bindings blend from their respective targeted nodes
    /// when default parameter declarations differ.
    #[test]
    fn two_addressed_bindings_on_one_name_blend_from_their_own_nodes() {
        let gpu = Gpu::headless().expect("no GPU available");
        let (a, b) = pair();
        let mut set = build(&gpu, &[&a, &b]);

        for i in 0..2 {
            assert!(
                set.bind(
                    Binding::new(
                        Kind::L4,
                        "exposure",
                        "nothing_measures_this",
                        Curve::Lin,
                        [0.0, 8.0]
                    )
                    .at(i)
                )
                .attached(),
                "renderer {i} declares `exposure`"
            );
        }
        set.prepare(&gpu.queue, 1, &Signals::default());

        let resolved = |i: u32| -> f32 {
            set.bindings()
                .iter()
                .find(|b| b.index == Some(i))
                .expect("both renderers are bound")
                .value()
        };
        assert_eq!(
            (resolved(0), resolved(1)),
            (1.0, 0.25),
            "the two bindings blended from one node's declaration rather than their own"
        );
    }
}

/// Validation tests for empty renderer stack refusals without device allocation.
mod refused {
    use super::gpu::{compile, PAIR_L1};
    use karakuri_engine::set::{Layering, SetError, Wiring};
    use karakuri_engine::Set;
    use karakuri_ir::typed::Checked;

    /// One L1 at capacity 2 and these renderers over it, which is the Set
    /// every overdraw test in this file is built from — minus the device.
    fn validate_over(l1: &str, l4s: &[&str]) -> Result<(), SetError> {
        let compiled: Vec<Checked> = l4s.iter().map(|s| compile(s)).collect();
        let refs: Vec<&Checked> = compiled.iter().collect();
        Set::validate(
            &[(&compile(l1), 2)],
            &[],
            &[],
            &[],
            &refs,
            Layering::Overdraw,
            3,
            &[],
            Wiring::default(),
        )
        .map(|_| ())
    }

    /// Empty Sets with no renderable geometry are rejected at build time.
    /// vacuously true, which would silently stop the simulation as well.
    #[test]
    fn a_set_with_no_renderer_is_refused() {
        let Err(err) = validate_over(PAIR_L1, &[]) else {
            panic!("a Set with no renderer was built");
        };
        assert!(
            matches!(err, SetError::NoRenderer { .. }),
            "the wrong diagnostic for an empty stack: {err}"
        );
        assert!(
            err.to_string().contains("pair"),
            "the message does not name the L1: {err}"
        );
    }
}
