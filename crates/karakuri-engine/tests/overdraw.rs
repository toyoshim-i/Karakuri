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

use karakuri_engine::binding::{Binding, Curve};
use karakuri_engine::{Gpu, Present, Set, SetError, Signals, VideoSource};
use karakuri_ir::typed::Checked;
use karakuri_ir::Kind;

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

/// A Set over [`PAIR_L1`] with `l4s` as its renderers, in draw order.
fn build(gpu: &Gpu, l4s: &[&str]) -> Set {
    build_over(gpu, PAIR_L1, l4s)
}

fn try_build(gpu: &Gpu, l1: &str, l4s: &[&str]) -> Result<Set, SetError> {
    let compiled: Vec<Checked> = l4s.iter().map(|s| compile(s)).collect();
    let refs: Vec<&Checked> = compiled.iter().collect();
    Set::build_many(
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
    (
        sprite("red", RED, 9.0, 1.0),
        sprite("green", GREEN, 17.0, 0.25),
    )
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

/// **A binding's blend base comes from a node that declares the name.**
///
/// `Set::bind` accepts an L4 binding if *any* renderer declares the name — one
/// binding, one value, written to every renderer that has it, which is the rule
/// a bare `--param` follows. Resolving it read the *first* renderer's map
/// unconditionally, so a name only the second declares missed, and the
/// `unwrap_or(0.0)` behind that lookup turned the declared default into zero.
///
/// Silent, and on the render path: a signal nothing provides comes back with
/// confidence 0.0 and step 4 of the binding path writes the param's own value
/// unchanged — so the failure is a renderer drawing at zero rather than an
/// error. `spread` is declared by the second renderer only, at 3.0.
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
fn weighted_sprite(name: &str, rgb: (f32, f32, f32), scale: f32, alpha: f32) -> String {
    let (r, g, b) = rgb;
    format!(
        r#"
proc {name} {{
  kind  L4
  blend weighted

  consumes position

  vertex {{
    clip       = camera * vec4(position, 1.0);
    point_size = {scale:?};
  }}

  fragment {{
    color = vec4({r:?}, {g:?}, {b:?}, {alpha:?});
  }}
}}
"#
    )
}

/// **A weighted node in a stack composites over what is under it, and order is
/// what decides which.**
///
/// Under `additive` order is invisible, which the test above asserts. It stops
/// being invisible the moment a weighted node is in the stack: its resolve is an
/// `over`, so a nearly opaque weighted sprite hides what is beneath it and does
/// not hide what is drawn after.
///
/// This is the half of `blend weighted` that a lone renderer cannot exercise at
/// all. With one node the resolve writes onto the transparent black the first
/// pass clears to, where `over` gives back exactly the source — so the `over`
/// blend state and no blend state at all are indistinguishable, and both the
/// state and the `first`/load flag went untested until a stack existed to put a
/// weighted node second in.
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

    // Drawn second, the veil covers most of the red — and **leaves the rest**.
    // `over` at alpha 0.94 keeps 6% of what is under it, and the lower bound is
    // the half that matters: a resolve that cleared instead of loading would
    // erase the red entirely and satisfy an upper bound alone. That is exactly
    // the mutant this test was written for and did not catch until the floor
    // was added.
    assert!(
        a[0] < red_alone[0] * 0.25,
        "a weighted node drawn second did not cover what was under it: {} of {}",
        a[0],
        red_alone[0]
    );
    // Measured at 1.4%, not the 6% one fragment of `alpha = 0.94` would leave:
    // the two sprites overlap, and two fragments give `1 - 0.06²`, so most of
    // the red sits under two veils rather than one. The floor is well under
    // that and well over the **zero** a clear would leave.
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

/// **A weighted node that is first still clears**, so a stack beginning with one
/// draws what that node alone would.
///
/// The other end of the same flag: `first` picks `Clear` over `Load`, and a
/// weighted resolve that always cleared would wipe whatever ran before it —
/// caught above — while one that never cleared would composite onto a stale
/// frame, which nothing else here would see.
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

/// **A stack skips the simulation only if *every* renderer is fullscreen.**
///
/// A fullscreen L4 consumes no attribute, so a Set holding only those has
/// nothing reading its element buffers and the whole simulation is work for a
/// reader that does not exist. One fullscreen node beside a per-element one does
/// not excuse it — and `any` in place of `all` there is a defect that shows up as
/// a frozen cloud rather than as an error.
#[test]
fn a_mixed_stack_still_simulates_because_one_renderer_reads_the_elements() {
    let gpu = Gpu::headless().expect("no GPU available");
    // **No `vertex` block is how an L4 says it covers the frame** — see
    // `examples/field_march.kir`. `consumes` must therefore be empty, which is
    // exactly what makes this the node that would excuse the simulation if it
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

/// **A Set with nothing to draw is refused rather than built.** A `Set` is a
/// video source, and a video source with no frame to give has no useful
/// behaviour to fall back on — and `all(is_fullscreen)` over an empty list is
/// vacuously true, which would silently stop the simulation as well.
#[test]
fn a_set_with_no_renderer_is_refused() {
    let gpu = Gpu::headless().expect("no GPU available");
    let Err(err) = try_build(&gpu, PAIR_L1, &[]) else {
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

/// **The addressed write is what a bare name cannot do: set two renderers'
/// `exposure` apart.**
///
/// A name with no address reaches every node declaring it — one knob moving both
/// renderers, which is the useful default and is why it is the default. It is
/// also, by construction, unable to give them different values. That is the gap
/// `Set::set_param_at` closes, and it is the same gap the record vocabulary has:
/// `layer` plus an `index`.
///
/// Asserted in pixels rather than in the map, because the claim is about which
/// renderer's *uniform* was written. The two own separate colour channels, so
/// one frame carries both answers.
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

/// **An address that names no node is refused rather than silently ignored**,
/// on the same terms `Set::bind` refuses a param a layer does not declare.
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

/// **An addressed binding blends from the node it names**, which is the half of
/// the address that only shows through the signal path.
///
/// Both renderers declare `exposure`, at 1.0 and 0.25. Two bindings, one per
/// renderer, both attached to a signal nothing provides — confidence 0.0, so
/// each writes its param's own value unchanged. They must resolve to the two
/// different declarations. A binding that took the first declaring node's base,
/// which is what an unaddressed one does, would give both 1.0.
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
