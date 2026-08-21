//! What a Set is resident for, per node and in total.
//!
//! **Asserted against the buffers, not against a layout recomputed here.**
//! `Set::element_storage` sums `wgpu::Buffer::size()` over the buffers each
//! node created, so a test that rebuilt the element layout to compare against
//! would pass whatever the layout did — it would assert that the engine calls a
//! function. Every number below is instead hand-walked in the comment beside it
//! from the WGSL placement rules, the same way `karakuri-ir`'s op counts are.
//!
//! That is the difference between this and the stage-4 figure it replaces. That
//! one was a second arithmetic over one procedure's `emit` list, and it was
//! wrong in three ways at once that no per-procedure test could see: an L2 is
//! sized from everything that reached it, an amplifier above a node widens
//! every element below it, and a compacted L1 pays for a destination index the
//! text of the procedure never mentions. Each of the first four claims here is
//! one of those, plus the total that only a Set can hold.
//!
//! **The last two are about the shapes a `Set` can take rather than about a
//! node**, and they are here because `Set::element_storage` walks sources and
//! their pairings itself: a Set over two geometries instantiates a chain per
//! source, and a Set whose deform declares a geometry slot holds *two*
//! simulations under one source. Neither walk is exercised by a single-source
//! fixture, and each is a place a dropped iterator would still return a
//! plausible list.

use karakuri_engine::set::{Edge, Layering};
use karakuri_engine::{Gpu, Set};
use karakuri_ir::typed::Checked;

/// **Eight elements, `position` and `tint`.** Two `vec3`s rather than a `vec3`
/// and a scalar, because a scalar after a `vec3` lands in the padding and costs
/// nothing — which would make the fixture unable to tell an L2 sized from the
/// chain apart from one sized from its own `emit` list. Two vectors is the
/// smallest emit list where dropping one changes the stride.
///
/// **No `spawn` and no `kill()`**, so its live set cannot change and the engine
/// builds no compaction scan for it. That is the baseline the fixture below
/// varies by exactly one thing.
const STILL: &str = r#"
proc still {
  kind     L1
  topology points
  capacity [8, 8] = 8

  emit position, tint

  element {
    position = vec3(0.0, 0.0, 0.0);
    tint     = vec3(1.0, 1.0, 1.0);
  }
}
"#;

/// The same procedure with one `kill()` in it.
///
/// **The emit list is identical on purpose.** Compaction is the only difference
/// between this and [`STILL`], so the difference between the two figures is the
/// scan's destination index buffer and nothing else — a subtraction rather than
/// a second hand-walked stride.
const CULLING: &str = r#"
proc culling {
  kind     L1
  topology points
  capacity [8, 8] = 8

  emit position, tint

  element {
    position = vec3(0.0, 0.0, 0.0);
    tint     = vec3(1.0, 1.0, 1.0);
    if position.y > 100.0 {
      kill();
    }
  }
}
"#;

/// An endomorphic L2 that names `position` and nothing else.
///
/// **It never mentions `tint`, and it is sized for it anyway.** What an L2
/// writes carries everything that reached it, so its element struct is
/// `upstream ∪ emit` — the exact input a figure computed from this file alone
/// cannot have, and the one the assertions below are built to catch.
const WIDEN: &str = r#"
proc widen {
  kind L2

  consumes position

  deform {
    position = position * 2.0;
  }
}
"#;

/// The same, amplifying by four.
const FAN: &str = r#"
proc fan {
  kind    L2
  amplify 4

  consumes position

  deform {
    position = position + vec3(0.0, float(copy) * 0.1, 0.0);
  }
}
"#;

/// The same procedure again under a chosen name, at sixty-four elements.
///
/// **A name per source rather than one fixture used twice**, because a Set
/// refuses two nodes called the same thing and an edge is written against the
/// name — so a pairing fixture cannot be built out of one string at all. The
/// emit list is [`STILL`]'s, so the stride below is the same 48 and the two
/// sources differ in nothing that this file measures.
fn source(name: &str) -> String {
    format!(
        r#"
proc {name} {{
  kind     L1
  topology points
  capacity [{PAIR_CAPACITY}, {PAIR_CAPACITY}] = {PAIR_CAPACITY}

  emit position, tint

  element {{
    position = vec3(0.0, 0.0, 0.0);
    tint     = vec3(1.0, 1.0, 1.0);
  }}
}}
"#
    )
}

/// An L2 that reads a second geometry through a declared slot.
///
/// **It emits nothing and amplifies nothing**, so every byte it is charged
/// comes from the chain it sits in rather than from anything it says — which is
/// the point: the far geometry it reads is a *simulation* with buffers of its
/// own, and those are charged to the far simulation and not to this node. A
/// figure that counted the pairing twice, or that charged the reader for what
/// it reads, is what the test below refuses.
const MORPH: &str = r#"
proc morph {
  kind L2
  uses far : Geometry

  consumes position

  deform {
    position = mix(position, far.position, vec3(0.5, 0.5, 0.5));
  }
}
"#;

/// Draws whatever reaches it. Nothing here measures a picture, but a Set is
/// built with renderers and the L4 is what makes the fixture a real one.
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
    build_sources(gpu, &[l1], l2s, &[])
}

/// The same over several geometries, and with the edges that fill whatever
/// geometry slot a deform declares.
///
/// Each source is built at the capacity its own procedure defaults to, which is
/// the rule the single-source [`build`] already followed — a per-source
/// capacity is exactly what makes the entries below a list rather than one
/// figure times a count.
fn build_sources(gpu: &Gpu, l1s: &[&str], l2s: &[&str], edges: &[Edge]) -> Set {
    let l1: Vec<Checked> = l1s.iter().map(|s| compile(s)).collect();
    let sources: Vec<(&Checked, u32)> = l1
        .iter()
        .map(|c| {
            (
                c,
                c.capacity.expect("an L1 declares a capacity range").default,
            )
        })
        .collect();
    let l2: Vec<Checked> = l2s.iter().map(|s| compile(s)).collect();
    let l2_refs: Vec<&Checked> = l2.iter().collect();
    let l4 = compile(DOTS);
    Set::build_many(
        &gpu.device,
        &gpu.queue,
        &sources,
        &l2_refs,
        None,
        &[],
        &[&l4],
        Layering::Overdraw,
        7,
        &[],
        karakuri_engine::set::Wiring {
            edges,
            ..Default::default()
        },
    )
    .expect("a chain of some L1s, some L2s and one L4")
}

/// The stride of `emit position, tint`, walked by hand from WGSL's placement
/// rules so that nothing below is comparing the engine against itself:
///
/// ```text
/// seed        u32  @  0..4
/// birth_frac  f32  @  4..8
/// position    vec3 @ 16..28   (16-byte aligned, 12 bytes long)
/// tint        vec3 @ 32..44
/// ```
///
/// 44 bytes rounded up to the struct's own 16-byte alignment: 48.
const STRIDE: u64 = 48;

/// One `u32` per element in the alive array, and one per element in the
/// compaction scan's destination index buffer. The same four bytes for a
/// different reason each time, spelled separately because they are two
/// allocations.
const FLAG: u64 = 4;
const DEST_INDEX: u64 = 4;

const CAPACITY: u64 = 8;

/// What [`source`] declares. Not [`CAPACITY`]: the two are independent numbers
/// and a fixture that shared one would let a wrong capacity on one side of the
/// file be cancelled by the same wrong capacity on the other.
const PAIR_CAPACITY: u64 = 64;

/// **An L1 pays for two directions of everything.** It reads what it wrote last
/// step, so the element buffer and the alive array each exist twice and swap.
/// Nothing below it in a chain does, which is why this is the only figure with
/// a factor of two in it.
#[test]
fn a_static_l1_pays_for_both_directions_and_nothing_else() {
    let gpu = Gpu::headless().expect("a GPU");
    let set = build(&gpu, STILL, &[]);

    let nodes = set.element_storage();
    assert_eq!(nodes.len(), 1, "one L1, no deforms: {nodes:?}");
    assert_eq!(nodes[0].capacity, CAPACITY as u32);
    assert_eq!(nodes[0].bytes, 2 * CAPACITY * (STRIDE + FLAG));
    assert_eq!(nodes[0].per_element(), 2 * (STRIDE + FLAG), "2 * (48 + 4)");
}

/// **A compacted L1 pays for a third buffer, and its procedure does not say
/// so.** The scan writes one destination index per element, and it exists
/// because the procedure can `kill()` — nothing about the emit list, the
/// layout, or the stride changes at all. This is the term the figure that used
/// to be published from stage 4 was missing, and no reading of one `.kir`'s
/// storage declarations could have found it.
#[test]
fn a_compacted_l1_also_pays_for_the_scans_destination_index() {
    let gpu = Gpu::headless().expect("a GPU");
    let still = build(&gpu, STILL, &[]).element_storage()[0];
    let culling = build(&gpu, CULLING, &[]).element_storage()[0];

    assert_eq!(
        culling.bytes,
        2 * CAPACITY * (STRIDE + FLAG) + CAPACITY * DEST_INDEX
    );
    assert_eq!(culling.per_element(), 2 * (STRIDE + FLAG) + DEST_INDEX);
    // Stated as a difference as well, because the claim is that compaction is
    // the *only* thing that separates the two: same emit list, same stride.
    assert_eq!(
        culling.bytes - still.bytes,
        CAPACITY * DEST_INDEX,
        "one destination index per element and no other change"
    );
}

/// **A non-amplifying L2 pays once, and for the whole chain's element.**
///
/// Once, because its output is rebuilt from its input every frame and nothing
/// reads back what it wrote — there is no second direction. And for the whole
/// element, because what it writes carries everything that reached it: `widen`
/// names `position` alone and is sized for `tint` as well. A figure built from
/// this procedure's own `emit` list would report a stride of 8 — `seed` and
/// `birth_frac` and nothing else — against the 48 the engine allocates.
///
/// It pays nothing for flags: the elements it emits are the elements that
/// reached it, under the flags they arrived with, so it shares the L1's array.
#[test]
fn a_non_amplifying_l2_pays_once_and_for_what_reached_it() {
    let gpu = Gpu::headless().expect("a GPU");
    let set = build(&gpu, STILL, &[WIDEN]);

    let nodes = set.element_storage();
    assert_eq!(nodes.len(), 2, "the L1 and the deform: {nodes:?}");
    let l2 = nodes[1];
    assert_eq!(l2.capacity, CAPACITY as u32, "an endomorphism keeps count");
    assert_eq!(l2.bytes, CAPACITY * STRIDE);
    assert_eq!(
        l2.per_element(),
        STRIDE,
        "one buffer at the chain's stride, and no flags of its own"
    );
}

/// **An amplifier is sized against the elements it makes, at the chain's
/// stride, and it owns flags for them.**
///
/// Its output count is the Set's capacity times its factor; its element carries
/// everything upstream emitted plus the `copy` index that starts existing here;
/// and its outputs are new elements nothing upstream holds a flag for, so it
/// allocates an alive array of its own rather than sharing one.
///
/// `copy` is free: `seed` and `birth_frac` leave eight bytes of the first
/// 16-byte block unused and a `u32` lands in four of them, so the stride is the
/// same 48 the L1 has. That is deliberate in the fixture — it means the number
/// below cannot be reached by accidentally counting `copy` twice.
#[test]
fn an_amplifying_l2_is_sized_for_the_attributes_that_reached_it() {
    let gpu = Gpu::headless().expect("a GPU");
    let set = build(&gpu, STILL, &[FAN]);

    let nodes = set.element_storage();
    assert_eq!(nodes.len(), 2, "the L1 and the amplifier: {nodes:?}");
    let l2 = nodes[1];
    assert_eq!(l2.capacity, 4 * CAPACITY as u32, "`amplify 4` over eight");
    assert_eq!(l2.bytes, 4 * CAPACITY * (STRIDE + FLAG));
    assert_eq!(
        l2.per_element(),
        STRIDE + FLAG,
        "one element buffer and one flag per element it makes"
    );
}

/// **The Set is the first thing that can be asked what a chain holds.**
///
/// `capacity` differs per node and an amplifier multiplies it for everything
/// below, so the total is not any node's figure times any one number: the L1
/// runs at eight elements, the amplifier and everything after it at
/// thirty-two. A chain of `[amplify 4, endomorphism]` is the shortest fixture
/// where a total computed from the Set's capacity alone would be wrong by a
/// factor of four on two of its three terms.
#[test]
fn a_sets_total_is_every_node_at_its_own_capacity() {
    let gpu = Gpu::headless().expect("a GPU");
    let set = build(&gpu, STILL, &[FAN, WIDEN]);

    let nodes = set.element_storage();
    assert_eq!(nodes.len(), 3, "the L1 and two deforms: {nodes:?}");
    assert_eq!(
        nodes.iter().map(|n| n.capacity).collect::<Vec<_>>(),
        vec![8, 32, 32],
        "the amplifier's factor reaches the node below it"
    );

    // L1:  2 * 8  * (48 + 4)  =  832
    // fan:      32 * (48 + 4) = 1664
    // widen:    32 *  48      = 1536   (`copy` is carried, and still free)
    let l1 = 2 * CAPACITY * (STRIDE + FLAG);
    let fan = 4 * CAPACITY * (STRIDE + FLAG);
    let widen = 4 * CAPACITY * STRIDE;
    assert_eq!(set.element_storage_bytes(), l1 + fan + widen);
    assert_eq!(set.element_storage_bytes(), 4032, "832 + 1664 + 1536");

    // And the renderer contributes nothing at all rather than a row of zeroes:
    // it draws from the last deform's buffer, which is already counted.
    assert_eq!(nodes.iter().map(|n| n.bytes).sum::<u64>(), 4032);
}

/// **A Set over two sources instantiates the whole chain per source**, and the
/// entries are per instance.
///
/// Two geometries emitting the same two attributes at the same capacity: the
/// figures are equal on purpose, because the claim is about the *walk* and not
/// about arithmetic that already has four tests above it. What a per-source
/// walk gets wrong is the count of entries and the total, and both are stated
/// here.
///
/// Each source is a static L1 with no `spawn` and no `kill()`, so each pays for
/// two directions of the element buffer and the alive array and for nothing
/// else: `2 * 64 * (48 + 4)` is 6656, twice.
#[test]
fn every_source_in_a_set_is_charged_for_its_own_chain() {
    let gpu = Gpu::headless().expect("a GPU");
    let left = source("left");
    let right = source("right");
    let set = build_sources(&gpu, &[&left, &right], &[], &[]);

    let nodes = set.element_storage();
    assert_eq!(nodes.len(), 2, "one L1 per source, no deforms: {nodes:?}");
    for node in &nodes {
        assert_eq!(node.capacity, PAIR_CAPACITY as u32);
        assert_eq!(node.bytes, 2 * PAIR_CAPACITY * (STRIDE + FLAG));
        assert_eq!(node.per_element(), 2 * (STRIDE + FLAG), "2 * (48 + 4)");
    }

    // 6656 + 6656. The second source is the whole of the difference from a
    // one-source Set, which is what a walk that stopped at the first would
    // silently halve.
    assert_eq!(
        set.element_storage_bytes(),
        2 * (2 * PAIR_CAPACITY * (STRIDE + FLAG))
    );
    assert_eq!(set.element_storage_bytes(), 13312, "6656 + 6656");
}

/// **A paired Set holds two simulations under one source, and the far one is
/// charged once.**
///
/// `uses far : Geometry` collapses the source list: the two geometries become
/// one `Source` with a near simulation, a far simulation and one chain over
/// them — so the entries are three rather than the two a reader counting
/// *sources* would expect, and the far side appears exactly once despite being
/// read every frame by the node below it.
///
/// The far simulation is charged as a simulation, not as an input: `2 * 64 *
/// (48 + 4)` is 6656, the same as the near one, because it is a full L1 with
/// its own two directions. The deform pays `64 * 48` = 3072 — one buffer at the
/// chain's stride, with no flags of its own, since it emits the elements that
/// reached it under the flags they arrived with — and it pays nothing at all
/// for reading `far.position`, which is somebody else's buffer.
#[test]
fn a_pairing_l2_charges_the_far_geometry_once_and_reads_it_free() {
    let gpu = Gpu::headless().expect("a GPU");
    let near = source("near");
    let far = source("far");
    let set = build_sources(
        &gpu,
        &[&near, &far],
        &[MORPH],
        &[Edge {
            node: "morph".to_string(),
            slot: "far".to_string(),
            to: "far".to_string(),
        }],
    );

    let nodes = set.element_storage();
    assert_eq!(
        nodes.len(),
        3,
        "the near simulation, the far one it pairs with, and the deform: {nodes:?}"
    );
    assert_eq!(
        nodes.iter().map(|n| n.capacity).collect::<Vec<_>>(),
        vec![64, 64, 64],
        "nothing amplifies, so every node runs at the geometry's own count"
    );

    let sim = 2 * PAIR_CAPACITY * (STRIDE + FLAG);
    assert_eq!(nodes[0].bytes, sim, "the near geometry");
    assert_eq!(nodes[1].bytes, sim, "the far geometry, counted once");
    assert_eq!(nodes[2].bytes, PAIR_CAPACITY * STRIDE, "the deform");
    assert_eq!(
        nodes[2].per_element(),
        STRIDE,
        "one buffer at the chain's stride, and nothing for the far side it reads"
    );

    // 6656 + 6656 + 3072.
    assert_eq!(
        set.element_storage_bytes(),
        2 * sim + PAIR_CAPACITY * STRIDE
    );
    assert_eq!(set.element_storage_bytes(), 16384, "6656 + 6656 + 3072");
}
