//! Integration tests for GPU element storage allocation and memory sizing accounting.
//!
//! Asserts exact buffer allocations per node, compaction overhead, amplifier capacity expansion,
//! and multi-source chain topologies directly against wgpu buffer sizes.

// Every test here takes a device, so the whole file is one `mod gpu` — the
// prefix `cargo test -- --skip gpu::` filters on. The convention, and the test
// that enforces it, are in `tests/gpu_tests_are_under_mod_gpu.rs`.
#[path = "common/mod.rs"]
mod common;

mod gpu {
    pub(super) use super::common::compile;
    use karakuri_engine::set::{Edge, Layering, PlannedStorage};
    use karakuri_engine::{Gpu, Set};
    use karakuri_ir::typed::Checked;

    /// Eight-element fixture without spawning or compaction, establishing baseline storage sizing.
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

    /// Eight-element fixture with compaction, isolating destination index buffer overhead.
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

    /// Endomorphic L2 deformer verifying upstream attribute preservation in element layouts.
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

    /// Returns an L1 source procedure with specified name and default capacity.
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

    /// An L2 deformer reading a second geometry via an input slot without introducing
    /// its own emission or amplification storage.
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
    point_rate = 0.015625;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;

    fn build(gpu: &Gpu, l1: &str, l2s: &[&str]) -> Set {
        build_sources(gpu, &[l1], l2s, &[])
    }

    /// Builds a multi-source Set with specified deformation chain and graph edges.
    fn build_sources(gpu: &Gpu, l1s: &[&str], l2s: &[&str], edges: &[Edge]) -> Set {
        both(gpu, l1s, l2s, edges).1
    }

    /// Compiles and constructs both planned storage accounting and active GPU Set.
    fn both(gpu: &Gpu, l1s: &[&str], l2s: &[&str], edges: &[Edge]) -> (Vec<PlannedStorage>, Set) {
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
        let wiring = || karakuri_engine::set::Wiring {
            edges,
            ..Default::default()
        };
        let planned = Set::validate(
            &sources,
            &l2_refs,
            &[],
            &[],
            &[&l4],
            Layering::Overdraw,
            7,
            &[],
            wiring(),
        )
        .expect("the same arguments the build below is given")
        .element_storage();
        let set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &sources,
            &l2_refs,
            &[],
            &[],
            &[&l4],
            Layering::Overdraw,
            7,
            &[],
            wiring(),
        )
        .expect("a chain of some L1s, some L2s and one L4");
        (planned, set)
    }

    /// Expected stride of emit position, tint struct (44 bytes padded to 48 alignment).
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

    /// L1 simulation buffers allocate double-buffered ping-pong storage.
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

    /// Verifies that compacted L1 procedures allocate an additional scan destination index buffer.
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

    /// Verifies that non-amplifying L2 deformers allocate a single buffer matching upstream element stride.
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

    /// Verifies that amplifiers allocate buffers for their expanded capacity and own distinct alive arrays.
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

    /// Verifies that total Set storage sums every node at its specific amplified capacity.
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

    /// Verifies that multi-source Sets account for separate storage allocations per source chain.
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

    /// Verifies that paired L2 deformers charge the paired geometry once without duplicating input buffers.
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
                slot: "far".into(),
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

    /// Verifies that offline PlannedStorage accounting matches active GPU Set buffer allocations exactly.
    #[test]
    fn the_plan_reports_what_the_built_set_allocates() {
        let gpu = Gpu::headless().expect("a GPU");
        let same = |l1s: &[&str],
                    l2s: &[&str],
                    edges: &[Edge],
                    term: &str|
         -> (Vec<PlannedStorage>, Set) {
            let (planned, set) = both(&gpu, l1s, l2s, edges);
            assert_eq!(
                planned.iter().map(|p| p.storage).collect::<Vec<_>>(),
                set.element_storage(),
                "the plan and the buffers disagree about {term}"
            );
            // The total as well as the entries: a walk that dropped a node
            // would fail the first assertion, and one that counted a node twice
            // at zero bytes would pass it.
            assert_eq!(
                planned.iter().map(|p| p.storage.bytes).sum::<u64>(),
                set.element_storage_bytes(),
                "the plan and the buffers disagree about the total for {term}"
            );
            (planned, set)
        };

        same(&[STILL], &[], &[], "an L1's two directions");
        same(
            &[CULLING],
            &[],
            &[],
            "the compaction scan's destination index",
        );
        same(
            &[STILL],
            &[WIDEN],
            &[],
            "an L2 sized for what reached it rather than for its own emit list",
        );
        same(
            &[STILL],
            &[FAN, WIDEN],
            &[],
            "an amplifier's factor reaching the node below it",
        );

        let left = source("left");
        let right = source("right");
        same(
            &[&left, &right],
            &[WIDEN],
            &[],
            "a chain instantiated once per geometry",
        );

        // Storage reports verify node naming across multi-instance sources.
        // that would only repeat the equality.
        let near = source("near");
        let far = source("far");
        let (planned, set) = same(
            &[&near, &far],
            &[MORPH],
            &[Edge {
                node: "morph".to_string(),
                slot: "far".into(),
                to: "far".to_string(),
            }],
            "the far geometry a pairing Set holds",
        );
        assert_eq!(
            planned
                .iter()
                .map(|p| set.node_names()[p.node].as_str())
                .collect::<Vec<_>>(),
            vec!["near", "far", "morph"],
            "an entry names a node other than the one it is an instance of"
        );
    }
}
