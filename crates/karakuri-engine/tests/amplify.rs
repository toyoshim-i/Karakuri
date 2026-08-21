//! An L2 whose output count differs from its input's.
//!
//! `L2 : Geometry -> Geometry` is an endomorphism, which is what makes a
//! modulator freely stackable and also what makes kaleidoscopes, instancing,
//! trails and subdivision inexpressible: nothing in the layer model could
//! change the element count. `amplify <factor>` is the second kind of L2 that
//! fills that gap — `docs/ir-spec.md`, "L2 amplification".
//!
//! **Everything here is counted rather than measured.** The claim of the
//! feature is that one element becomes several, so what a test has to see is
//! *how many separate things are on screen*, and a mean position — which is
//! what `deform.rs` measures — cannot tell one element from four stacked at the
//! same point. Each fixture below therefore spreads its copies along the
//! screen's vertical by the copy index and counts the bands of lit rows.
//!
//! Four claims, and the last two are the ones that would fail silently:
//!
//! - **A copy is a real element.** It has a position of its own, is drawn on
//!   its own, and the count of them is the declared factor.
//! - **`copy` is what tells them apart**, and it composes down a chain rather
//!   than being overwritten by the next amplifier.
//! - **Liveness follows the parent.** An amplifier owns its own alive flags —
//!   its buffer is `factor` times as long and cannot share its input's — and
//!   what it writes there is each parent's flag repeated. A dead parent
//!   contributes no live copies.
//! - **The chain below an amplifier runs on the amplifier's buffers**, not on
//!   the simulation's, however many endomorphic stages sit between.

use karakuri_engine::set::Layering;
use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
use karakuri_ir::typed::Checked;

const W: u32 = 64;
const H: u32 = 64;

/// **One element, at the origin, still.** Every count below is of things the
/// amplifier made, so one parent is the whole of what they need — and it keeps
/// the arithmetic exact: a band count is the factor rather than the factor
/// times something.
const STILL: &str = r#"
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

/// **More elements than one workgroup covers**, all at the origin.
///
/// The count is the point and 64 is the number that makes it: a compute pass is
/// dispatched in workgroups of 64, so a chain of eight elements runs in one
/// workgroup whatever range it was told, and every mistake about *which* range a
/// stage below an amplifier dispatches over is invisible. At 64 parents an
/// amplifier's output needs four workgroups and a stage handed the simulation's
/// count gets one.
const CROWD: &str = r#"
proc crowd {
  kind     L1
  topology points
  capacity [64, 64] = 64

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#;

/// **Two elements, one of which dies partway through** — alive at first and
/// dead later, which is the shape the liveness claim needs.
///
/// A parent that is dead from frame zero proves nothing: an amplifier's alive
/// buffer is freshly allocated and therefore zeroed, so copies nothing ever
/// wrote read as dead by accident. Only a parent that *was* live can show
/// whether the flags are being rewritten each frame or merely left alone.
///
/// `seed` is the initial slot index for a procedure with no `spawn` block, so
/// this kills exactly one element, and `t` is the same on every run.
const ONE_DIES: &str = r#"
proc one_dies {
  kind     L1
  topology points
  capacity [2, 2] = 2

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
    if seed == 1u {
      if t > 0.05 {
        kill();
      }
    }
  }
}
"#;

/// An amplifier that fans its copies out along `y`, centred on the parent.
///
/// The offset is the whole measurement: without it every copy lands on its
/// parent and the picture is indistinguishable from no amplification at all,
/// which is a fixture that cannot fail.
fn fan(name: &str, factor: u32, spread: f32) -> String {
    let half = (factor as f32 - 1.0) / 2.0;
    format!(
        r#"
proc {name} {{
  kind    L2
  amplify {factor}

  consumes position

  deform {{
    position = position + vec3(0.0, (float(copy) - {half:?}) * {spread:?}, 0.0);
  }}
}}
"#
    )
}

/// An amplifier that moves nothing. Used to make a *stacked* pair's arithmetic
/// unambiguous: with the first stage silent, the positions on screen are
/// decided entirely by whether the second stage composed the index or replaced
/// it, and the two answers differ in the band count rather than only in where
/// the bands are.
fn silent(name: &str, factor: u32) -> String {
    format!(
        r#"
proc {name} {{
  kind    L2
  amplify {factor}

  consumes position

  deform {{
    position = position;
  }}
}}
"#
    )
}

/// An ordinary endomorphic L2, for the chain that has to keep running on the
/// amplifier's buffers after one of these sits below it.
///
/// **A scale rather than a shift**, and the difference is the whole of what the
/// fixture below can see. An element a short dispatch never wrote holds nothing,
/// which is the origin — and the origin is where the parents were, so a *shift*
/// moves the copies that were written to within a texel of the ones that were
/// not, and the two merge into one band. Scaling moves every copy *away* from
/// the origin, which leaves the elements nothing wrote sitting alone in the
/// middle of an otherwise empty centre.
fn grow(name: &str, by: f32) -> String {
    format!(
        r#"
proc {name} {{
  kind L2

  consumes position

  deform {{
    position = position * {by:?};
  }}
}}
"#
    )
}

/// Draws whatever reaches it, as one small sprite per element. Small so that
/// neighbouring copies do not merge into one band.
const DOTS: &str = r#"
proc dots {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 2.0;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;

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

fn build(gpu: &Gpu, l1: &str, l2s: &[&str]) -> Set {
    // The Set's capacity is the L1's own declared default; what an amplifier
    // allocates is that times its factor, and nothing here has to say so.
    let capacity = compile(l1)
        .capacity
        .expect("an L1 declares a capacity range")
        .default;
    let l2: Vec<Checked> = l2s.iter().map(|s| compile(s)).collect();
    let l2_refs: Vec<&Checked> = l2.iter().collect();
    let l4 = compile(DOTS);
    let mut set = Set::build_many(
        &gpu.device,
        &gpu.queue,
        &[(&compile(l1), capacity)],
        &l2_refs,
        &[],
        &[],
        &[&l4],
        Layering::Overdraw,
        7,
        &[],
        karakuri_engine::set::Wiring::default(),
    )
    .expect("a chain of one L1, some L2s and one L4");
    set.resize(&gpu.device, W, H);
    // Pinned, for the reason `deform.rs` gives at length: an orbiting camera
    // foreshortens by a different amount every frame, and these fixtures lay
    // their material along an axis that has to stay in the picture plane.
    set.camera = karakuri_engine::camera::Orbit {
        radius: 5.0,
        speed: 0.0,
        height: 0.0,
        ..Default::default()
    };
    set
}

/// How many separate horizontal bands of lit rows the frame holds.
///
/// **A count, not a position.** The claim under test is that one element became
/// several, and several elements at one point look exactly like one — so the
/// measurement has to be of separateness. A band is a maximal run of rows with
/// any light in them, which is what a row of sprites is and what the gap
/// between two rows of sprites is not.
fn bands(gpu: &Gpu, set: &mut Set) -> usize {
    let px = frame(gpu, set);
    let mut lit = vec![false; H as usize];
    for (i, t) in px.chunks_exact(4).enumerate() {
        if t[0] > 0.01 {
            lit[i / W as usize] = true;
        }
    }
    assert!(
        lit.iter().any(|&b| b),
        "nothing was drawn, so there are no bands to count"
    );
    lit.iter()
        .enumerate()
        .filter(|&(y, &on)| on && (y == 0 || !lit[y - 1]))
        .count()
}

/// Total brightness in the frame, after one frame.
///
/// **The measurement for "how many elements were drawn", and the one thing a
/// band count cannot be.** Under `blend additive` every sprite contributes the
/// same energy wherever it lands, so the sum over the frame is proportional to
/// the element count *even when the sprites sit on top of each other* — which
/// is precisely the case a liveness question puts them in, since a parent and
/// its copies are placed by the fixture rather than by the thing under test.
fn total_light(gpu: &Gpu, set: &mut Set) -> f64 {
    frame(gpu, set)
        .chunks_exact(4)
        .map(|t| f64::from(t[0]))
        .sum()
}

/// Total brightness after a frame that **draws without stepping** — the audition
/// path, where a deck shows an `Allocated` slot the still it stopped at.
fn draw_only(gpu: &Gpu, set: &mut Set) -> f64 {
    render_frame(gpu, set, false)
        .chunks_exact(4)
        .map(|t| f64::from(t[0]))
        .sum()
}

/// RGBA f32 per texel, after one frame.
fn frame(gpu: &Gpu, set: &mut Set) -> Vec<f32> {
    render_frame(gpu, set, true)
}

fn render_frame(gpu: &Gpu, set: &mut Set, step: bool) -> Vec<f32> {
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
    if step {
        set.render(&mut encoder, present.hdr_view(), 1);
    } else {
        set.draw(&mut encoder, present.hdr_view());
    }
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

/// **One element in, four elements out**, and the four are separately visible.
///
/// The baseline is the same geometry with no amplifier, which is what makes the
/// assertion about the *feature* rather than about the fixture: one parent is
/// one band, and it is four only because something made it four.
#[test]
fn an_amplifying_l2_draws_one_element_as_many() {
    let gpu = Gpu::headless().expect("a GPU");

    let mut plain = build(&gpu, STILL, &[]);
    assert_eq!(bands(&gpu, &mut plain), 1, "one element is one band");

    let fan4 = fan("fan4", 4, 0.9);
    let mut amplified = build(&gpu, STILL, &[&fan4]);
    assert_eq!(
        bands(&gpu, &mut amplified),
        4,
        "`amplify 4` makes four elements of one, at four positions"
    );
}

/// **The factor is what decides the count**, so a different one gives a
/// different picture. Without this a hardcoded four anywhere in the lowering
/// would pass the test above.
#[test]
fn the_declared_factor_is_the_number_of_copies() {
    let gpu = Gpu::headless().expect("a GPU");
    for factor in [2u32, 3, 5] {
        let f = fan("fanned", factor, 0.8);
        let mut set = build(&gpu, STILL, &[&f]);
        assert_eq!(
            bands(&gpu, &mut set),
            factor as usize,
            "`amplify {factor}` should make {factor} copies"
        );
    }
}

/// **Stacked amplifiers multiply, and the index composes.**
///
/// The first stage moves nothing, so every position on screen comes from the
/// second stage's `copy`. Composed, that index runs `0..6` and the six copies
/// land on six rows. Overwritten — which is what a second amplifier assigning
/// its loop variable straight into the slot would do — it runs `0..3`, the two
/// halves land on top of each other, and there are three bands for six
/// elements. That is the difference this fixture exists to see, and it is
/// invisible to any measurement of where the material is.
#[test]
fn stacked_amplifiers_multiply_and_compose_the_index() {
    let gpu = Gpu::headless().expect("a GPU");
    let first = silent("first", 2);
    let second = fan("second", 6, 0.55);
    // The second stage's factor is 3, but it spreads over the *composed* range,
    // so it is written as a fan of six over a chain whose product is six.
    let second = second.replace("amplify 6", "amplify 3");
    let mut set = build(&gpu, STILL, &[&first, &second]);
    assert_eq!(
        bands(&gpu, &mut set),
        6,
        "two by three is six distinct elements, not three pairs at three places"
    );
}

/// **A dead parent contributes no live copies.**
///
/// An amplifier cannot share its input's alive buffer — its own is `factor`
/// times as long — so it writes one, and what it writes is each parent's flag
/// repeated. Writing it *before* the early return is the whole of why this
/// holds: a `return` above the write would leave whatever the buffer held from
/// the frame before, which on the first frame is uninitialised memory and
/// afterwards is a copy of a parent that has since died.
#[test]
fn a_dead_parent_leaves_no_live_copies() {
    let gpu = Gpu::headless().expect("a GPU");

    // **Two Sets over the same geometry, stepped in lockstep**, one amplified
    // and one not. The invariant is that the amplified frame holds exactly
    // three times the light of the plain one, *on every frame* — which is the
    // whole claim, stated in a form that does not depend on knowing which frame
    // the kill lands on.
    //
    // The frame it lands on is the one that matters and is easy to miss. A
    // killed element stays in the live range until the *next* step's scan
    // compacts it, so for exactly one frame it is a dead slot inside the range,
    // and its copies' flags have to say so. Miss that frame and every measurable
    // difference is gone: after compaction the range shrinks, the stale entries
    // fall outside it, and nothing reads them again.
    let f = fan("fan3", 3, 0.9);
    let mut plain = build(&gpu, ONE_DIES, &[]);
    let mut amplified = build(&gpu, ONE_DIES, &[&f]);

    let mut saw_the_death = false;
    let mut first = None;
    for frame in 0..10 {
        let one = total_light(&gpu, &mut plain);
        let many = total_light(&gpu, &mut amplified);
        let base = *first.get_or_insert(one);
        assert!(
            (many - one * 3.0).abs() < one * 0.05,
            "frame {frame}: {many} lit against {one} for the same geometry unamplified — \
             three copies of every live parent and no copies of a dead one"
        );
        if one < base * 0.75 {
            saw_the_death = true;
        }
    }
    assert!(
        saw_the_death,
        "one of the two parents has to actually die inside the window, or the \
         invariant above was never put under any strain"
    );
}

/// **The chain below an amplifier runs on the amplifier's buffers.**
///
/// An endomorphic stage below one owns no liveness and no counts of its own, so
/// it has to be handed the amplifier's — at build time, for the range its
/// `deform` bounds itself by, and at record time, for the number of workgroups
/// it is dispatched in. Getting either from the simulation instead deforms the
/// first `range` of `range * factor` elements and leaves the rest holding
/// whatever their buffer had, which on the first frame is nothing at all.
///
/// **Two chains, and the second is the one that finds things.** One stage below
/// the amplifier is a case where the obvious spelling happens to be right — the
/// node above *is* the amplifier, so asking it gives the right answer. Two
/// stages is where "ask the previous node" and "ask the last node that
/// amplified" stop agreeing, and 64 parents is where a wrong workgroup count
/// stops being covered by the one workgroup a small fixture needs anyway.
#[test]
fn every_stage_below_an_amplifier_still_sees_every_copy() {
    let gpu = Gpu::headless().expect("a GPU");

    let f = fan("fan4", 4, 0.8);
    let after = grow("after", 1.4);
    let again = grow("again", 1.15);

    let mut one = build(&gpu, CROWD, &[&f, &after]);
    assert_eq!(
        bands(&gpu, &mut one),
        4,
        "a stage below the amplifier deforms every copy, not the one parent that reached it"
    );

    let mut two = build(&gpu, CROWD, &[&f, &after, &again]);
    assert_eq!(
        bands(&gpu, &mut two),
        4,
        "and so does the stage below that one — a fifth band is the elements a \
         short dispatch never wrote, sitting at the origin"
    );
}

/// **Four copies were drawn, and the count does not depend on telling them
/// apart.**
///
/// Every other fixture here spreads its copies out and counts bands, which
/// answers "how many *places*". This one puts them exactly on top of each other
/// and measures light, which answers "how many *elements*" — and the two come
/// apart whenever a copy lands where another already is, which is what a
/// kaleidoscope at low spread does all the time.
///
/// Named for the corner count because that was the hypothesis, and it was
/// wrong: multiplying `vertex_count` by the factor does *not* change this
/// figure. The extra corners run past what `corner_of` defines and collapse, so
/// the sprite is drawn once however many vertices are asked for. That defect is
/// caught by reading the buffer, in `set.rs`'s own tests — there is no picture
/// it changes.
#[test]
fn stacked_copies_are_counted_by_the_light_they_add() {
    let gpu = Gpu::headless().expect("a GPU");

    // The copies land on their parent, deliberately: with nothing separating
    // them the only thing that can differ between the two frames is how much
    // light each element contributed, which is what the claim is about.
    let still = fan("still4", 4, 0.0);
    let mut plain = build(&gpu, STILL, &[]);
    let mut amplified = build(&gpu, STILL, &[&still]);

    let one = total_light(&gpu, &mut plain);
    let four = total_light(&gpu, &mut amplified);
    assert!(
        (four - one * 4.0).abs() < one * 0.05,
        "four copies of one element is four times the light, not sixteen: {four} against {one}"
    );
}

/// **A Set that has never been stepped still draws.**
///
/// A deck draws an `Allocated` slot without stepping it — that is what an
/// audition is, and both `Set::draw` and `Deck` say in as many words that it
/// shows the still the Set stopped at. An amplifier's counts are written by a
/// pass of its own, and left to the step alone that pass had never run: the
/// buffer was freshly allocated, therefore zeroed, therefore no vertices and no
/// instances. A working Set auditioned as black.
#[test]
fn an_amplified_set_draws_without_having_been_stepped() {
    let gpu = Gpu::headless().expect("a GPU");

    let f = fan("fan4", 4, 0.0);
    let mut plain = build(&gpu, STILL, &[]);
    let mut amplified = build(&gpu, STILL, &[&f]);

    let one = draw_only(&gpu, &mut plain);
    let four = draw_only(&gpu, &mut amplified);
    assert!(
        one > 0.0,
        "the unamplified Set draws its initial state without a step"
    );
    assert!(
        (four - one * 4.0).abs() < one * 0.05,
        "and the amplified one draws four copies of it: {four} against {one}"
    );
}

/// **A chain too large for the device is refused by name, not fatally.**
///
/// wgpu answers an over-limit binding by panicking the thread that made it,
/// which at startup takes the process down and on the swap worker is a
/// `SetError::Panicked` with no sentence in it. The checker's ceiling on
/// `amplify` cannot stand in for this: it sees one declaration, and what is too
/// large is the Set's `capacity` times every factor above the node times the
/// element stride — none of which a `.kir` file knows.
///
/// The numbers here are past any device rather than tuned to this one, so the
/// test asserts the shape of the answer rather than a threshold.
#[test]
fn an_amplified_chain_too_large_for_the_device_is_refused_rather_than_fatal() {
    let gpu = Gpu::headless().expect("a GPU");
    let huge = r#"
proc huge {
  kind     L1
  topology points
  capacity [1, 1048576] = 1048576

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#;
    // **A body that does nothing**, because the cost ceiling is a separate
    // guard and would otherwise refuse this first: a factor of 1024 multiplies
    // the block cost by 1024, so anything but the smallest `deform` is over
    // 4096 ops/element before the buffer is ever sized. That is the estimator
    // doing its job; it is not this one, and the two limits are independent.
    let l2 = silent("enormous", 1024);
    let l2 = compile(&l2);
    let l4 = compile(DOTS);
    let built = Set::build_many(
        &gpu.device,
        &gpu.queue,
        &[(&compile(huge), 1_048_576)],
        &[&l2],
        &[],
        &[],
        &[&l4],
        Layering::Overdraw,
        7,
        &[],
        karakuri_engine::set::Wiring::default(),
    );
    let Err(err) = built else {
        panic!("a billion elements is past every device, and building it should have said so");
    };
    let text = err.to_string();
    assert!(
        text.contains("enormous") && text.contains("amplifies to"),
        "the refusal has to name the node and the size: {text}"
    );
}
