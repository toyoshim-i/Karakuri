//! Several L4 renderers over one geometry, asserted in pixels.
//!
//! **This is the payoff the primitive-centric bet was made for.** One cloud
//! drawn as sprites *and* as streaks *and* as a solid used to cost three
//! simulations, because a Set was a pair and the only way to have two renderers
//! was to have two of everything. It is now one simulation and three draw
//! passes.
//!
//! Two claims carry the file and they pull in opposite directions:
//!
//! - **Both renderers reach the frame**, which is what fails if the second pass
//!   clears the target instead of loading it — a defect that produces a
//!   perfectly plausible picture of the last renderer alone.
//! - **The geometry is drawn twice and simulated once**, which is what fails if
//!   a stack quietly becomes two Sets.
//!
//! The material is deliberately flat and the two renderers deliberately differ
//! only in colour and size. What is under test is a compositing rule, and
//! anything that looked good would only make a failure harder to read.

use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
use karakuri_ir::typed::Checked;

const W: u32 = 64;
const H: u32 = 64;

/// Two elements at fixed positions, no spawning, no motion — the picture is a
/// pure function of `seed`, so every Set below gets literally identical
/// geometry however many renderers read it.
const PAIR_L1: &str = r#"
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

/// The same two elements, but **accumulating**: each step adds a fixed step to
/// `position`, so where an element sits is a function of how many steps ran
/// rather than of `seed` alone.
///
/// [`PAIR_L1`] cannot tell one simulation from three, because a pure function of
/// `seed` gives the same buffer however many times it is evaluated. This can.
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

/// A flat sprite of one colour at one size. `name` keeps two of them distinct
/// as procedures; `exposure` is declared by both on purpose — every L4 in
/// `examples/` declares one, and a Set holding two of them is exactly what the
/// old `ParamCollision` refusal forbade.
///
/// **The default is an argument because two renderers must be able to declare
/// one name at two values.** With both at 1.0 a Set that handed every renderer
/// the *first* one's parameter map would draw an identical frame, and every test
/// here would pass through it — which is what happened to the first draft.
fn sprite(name: &str, rgb: (f32, f32, f32), scale: f32, exposure: f32) -> String {
    let (r, g, b) = rgb;
    format!(
        r#"
proc {name} {{
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = {exposure:?}

  vertex {{
    clip       = camera * vec4(position, 1.0);
    point_size = {scale:?};
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

fn compile(src: &str) -> Checked {
    let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    checked
}

fn render(errs: &[karakuri_ir::IrError], src: &str) -> String {
    errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n")
}

/// A Set over [`PAIR_L1`] with `l4s` as its renderers, in draw order.
fn build(gpu: &Gpu, l4s: &[&str]) -> Set {
    build_over(gpu, PAIR_L1, l4s)
}

fn build_over(gpu: &Gpu, l1: &str, l4s: &[&str]) -> Set {
    let compiled: Vec<Checked> = l4s.iter().map(|s| compile(s)).collect();
    let refs: Vec<&Checked> = compiled.iter().collect();
    let mut set = Set::build_many(&gpu.device, &gpu.queue, &compile(l1), &refs, 2, 3)
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
        wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
    );
    gpu.queue.submit([encoder.finish()]);

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    gpu.device.poll(wgpu::PollType::Wait).expect("poll");
    let data = slice.get_mapped_range();
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
/// **different defaults for the one param name they share**.
fn pair() -> (String, String) {
    (sprite("red", RED, 9.0, 1.0), sprite("green", GREEN, 17.0, 0.25))
}

// ---------------------------------------------------------------------------

/// **Both renderers reach the frame, and the arithmetic says by how much.**
///
/// The whole point, and the one defect that produces a plausible picture rather
/// than an error: if the second pass cleared the target instead of loading it,
/// the frame would be the second renderer alone — a perfectly reasonable image
/// of exactly half the work.
///
/// **Colour and alpha compose differently, and the test has to say so.** Under
/// `additive` the colour blend is `dst + src * a` — a sum, so a stack of two
/// must come out as the two drawn alone added channel for channel. Alpha is
/// `a_s + a_d * (1 - a_s)`, which is coverage rather than a fourth colour: it is
/// the union of what drew, it saturates toward 1, and it is *not* a sum. An
/// earlier draft of this test asserted a sum for all four and failed on alpha,
/// which was the blend state being right and the assertion being written from a
/// guess.
///
/// Both halves pin the *load*, since a clear on the second pass would give
/// exactly the green-only frame; and both pin that the first pass clears, since
/// a load there would accumulate whatever the target happened to hold.
///
/// The two renderers are different sizes as well as different colours, so the
/// green sprite covers texels the red one does not — a stack that drew one
/// renderer twice would still reach the right total for a single colour.
#[test]
fn a_stack_of_two_renderers_composes_what_each_of_them_draws_alone() {
    let gpu = Gpu::headless().expect("no GPU available");
    let (a, b) = pair();

    let red = draw(&gpu, &mut build(&gpu, &[&a]));
    let green = draw(&gpu, &mut build(&gpu, &[&b]));
    let stacked = draw(&gpu, &mut build(&gpu, &[&a, &b]));

    let (sr, sg) = (channel_sums(&red), channel_sums(&green));
    let total = channel_sums(&stacked);
    assert!(sr[0] > 1.0 && sg[1] > 1.0, "a lone renderer drew nothing to compare against");
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

/// **One geometry, however many read it.**
///
/// The claim the milestone is for: a second renderer costs a draw pass and not a
/// simulation. The plausible defect is a draw loop that steps the simulation
/// once per renderer, which is invisible in a picture — three sprites drawn at
/// the wrong instant still look like sprites.
///
/// **So the material has to accumulate.** [`PAIR_L1`] is a pure function of
/// `seed` and would give a byte-identical buffer however many times it was
/// evaluated, so it cannot tell one simulation from three; [`CREEP_L1`] adds a
/// step to `position` each time it runs, and a stack that stepped per renderer
/// lands three times as far along. The buffers are compared rather than the
/// counts, because `live_count` is 2 either way.
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
    let (stride, at) = (layout.stride as usize, layout.offset_of("position") as usize);
    let x_of = |bytes: &[u8], i: usize| {
        let o = i * stride + at;
        f32::from_le_bytes(bytes[o..o + 4].try_into().expect("four bytes of position.x"))
    };
    let single = one.read_elements(&gpu.device, &gpu.queue);
    assert!(x_of(&single, 0) > 0.01, "the material did not accumulate, so nothing is under test");

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

/// **Order is draw order — and under `additive` alone that is invisible, which
/// is worth asserting rather than assuming.**
///
/// Additive blending is a sum, and a sum is commutative; the coverage in alpha
/// composes as `a_s + a_d(1 - a_s)`, which is symmetric in the two. So two
/// additive renderers swapped must give the *same* frame, and a test that
/// claimed otherwise would be asserting a defect.
///
/// Naming it matters because the list is ordered and the order is real — it is
/// what decides which pass clears, and it will be what decides the composite
/// the moment a `weighted` node is in the stack, whose resolve is an `over`.
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

/// **Two renderers declaring one name hold two values, each reaches its own
/// renderer's uniform, and a bare name moves both.**
///
/// Every L4 in `examples/` declares `exposure`, which is why a Set keyed by name
/// alone had to refuse two renderers outright. Both fixtures here declare one,
/// at **different defaults**, which is what makes this a claim about two values
/// rather than about one written twice.
///
/// The pixels are what make it a claim about the *uniforms* rather than about a
/// map. `exposure` scales the colour each renderer writes, and the two
/// renderers own separate colour channels, so the two contributions can be read
/// off one frame independently: setting a bare `exposure` to 2.0 has to take red
/// from 1.0 to 2.0 — twice — and green from 0.25 to 2.0 — eight times. A Set
/// that handed both renderers the first one's map would have drawn green at 1.0
/// to begin with and would move it by two.
#[test]
fn one_name_declared_by_two_renderers_is_two_values_each_reaching_its_own() {
    let gpu = Gpu::headless().expect("no GPU available");
    let (a, b) = pair();

    let mut plain = build(&gpu, &[&a, &b]);
    let mut brighter = build(&gpu, &[&a, &b]);

    let mut declared: Vec<f32> = plain
        .params()
        .filter(|(_, name, _)| *name == "exposure")
        .map(|(_, _, value)| value)
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
        assert!(base[i] > 1.0, "channel {name} drew nothing at its default exposure");
        assert!(
            (raised[i] - factor * base[i]).abs() <= 0.02 * factor * base[i],
            "channel {name}: `exposure = 2.0` gave {} where {factor} times {} was due — \
             that renderer read another node's value",
            raised[i],
            base[i]
        );
    }
}
