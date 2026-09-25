//! The two things `Set::build` does that no node does: put the two halves
//! together, and read the `.kir`'s declared defaults. The byte-level checks
//! for the L1 node's initial upload moved with it — see
//! `crate::node::simulation`'s tests.
use super::*;
use crate::binding::ParamWrite;
use crate::gpu::Gpu;
use karakuri_ir::Kind;

/// Tests that a negative default parameter value is read correctly through the IR fold.
#[test]
fn a_negative_param_default_is_read_as_its_declared_value() {
    let src = r#"
proc signed_defaults {
  kind  L4
  blend additive

  param drift  : float [-1.0, 1.0] = -0.35
  param plain  : float [ 0.0, 1.0] =  0.25

  consumes position

  vertex {
clip       = camera * vec4(position, 1.0);
point_rate = 0.004;
  }

  fragment {
color = vec4(1.0, 1.0, 1.0, drift + plain);
  }
}
"#;
    let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
    let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"));
    let of = |name: &str| {
        checked
            .params
            .iter()
            .find(|p| p.name == name)
            .expect("the param is declared")
            .default_scalar()
    };
    assert_eq!(
        of("drift"),
        Some(-0.35),
        "a negative default was read as an absence"
    );
    assert_eq!(
        of("plain"),
        Some(0.25),
        "a positive default stopped being read"
    );
}

/// Verifies that the engine's parameter map agrees with `karakuri_ir::Param::default_scalar`.
#[test]
fn the_engines_param_map_reads_a_negative_default_through_the_ir_fold() {
    let src = r#"
proc signed_defaults {
  kind  L4
  blend additive

  param drift  : float [-1.0, 1.0] = -0.35
  param plain  : float [ 0.0, 1.0] =  0.25

  consumes position

  vertex {
clip       = camera * vec4(position, 1.0);
point_rate = 0.004;
  }

  fragment {
color = vec4(1.0, 1.0, 1.0, drift + plain);
  }
}
"#;
    let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
    let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"));
    let map = declared_defaults(&checked);
    assert_eq!(
        map.get("drift").copied(),
        Some(-0.35),
        "the map a uniform is packed from has stopped agreeing with \
         `karakuri_ir::Param::default_scalar`: a fold of this file's own read `-0.35` \
         as an absence, so the run loads a default the `param_decl` beside it does not \
         state"
    );
    assert_eq!(
        map.get("plain").copied(),
        Some(0.25),
        "the map a uniform is packed from has stopped agreeing with \
         `karakuri_ir::Param::default_scalar` about an ordinary positive default"
    );
}

/// Verifies that vector parameters are expanded into per-component keys in value and range maps.
#[test]
fn a_vector_param_enters_the_value_map_one_component_at_a_time() {
    let src = r#"
proc glowing {
  kind  L4
  blend additive

  param glow   : vec3  [0.0, 4.0] = vec3(0.4, 0.7, 1.0)
  param plain  : float [ 0.0, 1.0] =  0.25

  consumes position

  vertex {
clip       = camera * vec4(position, 1.0);
point_rate = 0.004;
  }

  fragment {
color = vec4(glow * plain, 1.0);
  }
}
"#;
    let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
    let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"));

    let values = declared_defaults(&checked);
    assert_eq!(
        values.get("glow.x").copied(),
        Some(0.4),
        "a `vec3` param is still not entering the map the uniform is packed from, so it \
         reaches the shader as zeroes"
    );
    assert_eq!(values.get("glow.y").copied(), Some(0.7));
    assert_eq!(values.get("glow.z").copied(), Some(1.0));
    assert_eq!(
        values.get("glow"),
        None,
        "the bare name addresses no number and must hold none"
    );
    assert_eq!(
        values.get("plain").copied(),
        Some(0.25),
        "a scalar param keeps its own name"
    );

    let ranges = declared_ranges(&checked);
    for key in ["glow.x", "glow.y", "glow.z"] {
        assert_eq!(
            ranges.get(key).copied(),
            Some([0.0, 4.0]),
            "{key} has no declared range, so nothing can publish, bind or clamp it"
        );
    }
    assert_eq!(ranges.get("glow"), None);
    assert_eq!(ranges.get("plain").copied(), Some([0.0, 1.0]));
    assert_eq!(
        values.keys().collect::<std::collections::BTreeSet<_>>(),
        ranges.keys().collect::<std::collections::BTreeSet<_>>(),
        "the value map and the range map are keyed by one walk and have stopped agreeing"
    );
}

/// Verifies that a vector default that cannot be evaluated leaves all component keys out.
#[test]
fn a_vector_default_that_cannot_be_stated_leaves_no_component_behind() {
    let src = r#"
proc partial {
  kind  L4
  blend additive

  param glow : vec3 [0.0, 4.0] = vec3(0.4, 0.7, 0.5 + 0.5)

  consumes position

  vertex {
clip       = camera * vec4(position, 1.0);
point_rate = 0.004;
  }

  fragment {
color = vec4(glow, 1.0);
  }
}
"#;
    let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
    let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"));
    assert!(
        declared_defaults(&checked).is_empty(),
        "a default the fold cannot state must leave the whole declaration out"
    );
    assert_eq!(
        declared_ranges(&checked).len(),
        3,
        "the range is declared whatever the default says"
    );
}

/// Tests that unassigned nodes default to `Authority::Manual`.
#[test]
fn a_node_nobody_has_spoken_for_is_manual() {
    assert_eq!(Authority::default(), Authority::Manual);
    assert_eq!(
        Authority::ALL[0],
        Authority::default(),
        "the list should start where a node starts"
    );
}

/// Tests string representation round-trips for each authority level.
#[test]
fn every_authority_has_a_name_and_answers_to_it() {
    assert_eq!(Authority::Manual.name(), "manual");
    assert_eq!(Authority::Suggesting.name(), "suggesting");
    assert_eq!(Authority::Automatic.name(), "automatic");
    for level in Authority::ALL {
        assert_eq!(
            Authority::from_name(level.name()),
            Some(level),
            "`{}` does not read back as the level that wrote it",
            level.name()
        );
    }
    assert_eq!(
        Authority::from_name("man"),
        None,
        "the console's abbreviation is a surface's vocabulary, not a record's"
    );
}

/// Tests that wildcard parameter writes are refused when matched nodes have differing authorities.
#[test]
fn a_bare_name_is_refused_only_where_the_nodes_it_lands_on_disagree() {
    let uniform = [
        (Kind::L1, 0, Authority::Manual),
        (Kind::L4, 0, Authority::Manual),
        (Kind::L4, 1, Authority::Manual),
    ];
    assert_eq!(
        CrossesAuthority::over("radius", &uniform),
        None,
        "three nodes the operator kept are one arrangement, not a disagreement"
    );
    assert_eq!(
        CrossesAuthority::over("radius", &[]),
        None,
        "a key this Set declares nowhere lands on nothing, which is `no parameter \
         named` and not a refusal"
    );
    assert_eq!(
        CrossesAuthority::over("radius", &[(Kind::L4, 0, Authority::Automatic)]),
        None,
        "one node cannot disagree with itself, whatever it was granted to"
    );

    let refused = CrossesAuthority::over(
        "radius",
        &[
            (Kind::L1, 0, Authority::Manual),
            (Kind::L4, 0, Authority::Automatic),
        ],
    )
    .expect("a node the operator kept and a node an agent acts on are two arrangements");
    assert_eq!(refused.key, "radius");
    assert_eq!(
        refused.landing, "L1:0 manual, L4:0 automatic",
        "the refusal names every node the write lands on, with what each is under"
    );
    let sentence = refused.to_string();
    assert!(
        sentence.contains("L1:0 manual, L4:0 automatic") && sentence.contains("`radius`"),
        "the one sentence has to carry the control and the nodes that disagreed: \
         {sentence}"
    );
}

// The three that build a Set for real. The ones above check the layout arithmetic
// `Set::build` would go on to use, and take no device — which is most of what
// `cargo test -p karakuri-engine --lib -- --skip gpu::` is for.
// See `../tests/gpu_tests_are_under_mod_gpu.rs`.
mod gpu {
    use super::*;

    /// Verifies that a procedure with a spawn block builds a `Set` starting with zero live count.
    #[test]
    fn a_procedure_with_a_spawn_block_builds_and_starts_empty() {
        let gpu = Gpu::headless().expect("no GPU available");
        let l1_src = r#"
proc probe_spawn_l1 {
  kind     L1
  topology points
  capacity [4, 64] = 8

  param spawn_rate : float [0.0, 40000.0] = 1000.0

  emit position, age

  spawn {
position = vec3(0.0, 0.0, 0.0);
age      = 0.0;
  }

  element {
position = position;
age      = age;
  }
}
"#;
        let l4_src = r#"
proc probe_l4 {
  kind  L4
  blend additive

  consumes position

  vertex {
clip       = camera * vec4(position, 1.0);
point_rate = 0.004;
  }

  fragment {
color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
        let compile = |src: &str| -> Checked {
            let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
            let checked =
                karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"));
            karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("cost: {e:?}"));
            checked
        };
        let l1 = compile(l1_src);
        let l4 = compile(l4_src);
        let set = Set::build(&gpu.device, &gpu.queue, &l1, &l4, 8, 1).expect("compatible pair");

        assert_eq!(
            set.live_count(&gpu.device, &gpu.queue),
            0,
            "a spawn-block procedure starts empty"
        );
    }
    /// Verifies that wildcard parameter writes across nodes with conflicting authorities
    /// are refused without mutating parameter values, while addressed writes succeed.
    #[test]
    fn a_wildcard_write_is_refused_where_the_nodes_it_lands_on_disagree() {
        let gpu = Gpu::headless().expect("no GPU available");
        let l1_src = r#"
proc probe_shared_l1 {
  kind     L1
  topology points
  capacity [4, 64] = 8

  param radius : float [0.0, 8.0] = 1.0

  emit position

  element {
position = vec3(radius, 0.0, 0.0);
  }
}
"#;
        let l4_src = r#"
proc probe_shared_l4 {
  kind  L4
  blend additive

  param radius : float [0.0, 8.0] = 1.0

  consumes position

  vertex {
clip       = camera * vec4(position, 1.0);
point_rate = radius;
  }

  fragment {
color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
        let compile = |src: &str| -> Checked {
            let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
            let checked =
                karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"));
            karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("cost: {e:?}"));
            checked
        };
        let l1 = compile(l1_src);
        let l4 = compile(l4_src);
        let mut set = Set::build(&gpu.device, &gpu.queue, &l1, &l4, 8, 1).expect("compatible pair");
        let radius = |set: &Set| -> Vec<(Kind, u32, f32)> {
            let mut found: Vec<(Kind, u32, f32)> = set
                .params()
                .filter(|(_, _, key, _)| *key == "radius")
                .map(|(layer, index, _, value)| (layer, index, value))
                .collect();
            found.sort_by_key(|(layer, index, _)| (format!("{layer:?}"), *index));
            found
        };

        // Nobody has spoken for either node, so the bare name is one
        // control over one arrangement and reaches both.
        assert_eq!(
            set.write_param(&ParamWrite::everywhere("radius", 2.0)),
            Ok(2),
            "a Set nobody has spoken for is uniform, and a bare name reaches every \
             declaration of the key"
        );
        assert_eq!(
            radius(&set),
            vec![
                (Kind::L1, 0, 2.0),
                // The built-in camera declares `radius` but is addressed-only
                // ([`Set::addressed_only`]), so wildcard writes do not reach it.
                (Kind::L3, 0, 8.0),
                (Kind::L4, 0, 2.0),
            ],
            "both declarations should hold what the wildcard wrote, and the camera's own \
             `radius` should be untouched by a bare name"
        );

        // One node handed to an agent, and the same control now spans two
        // arrangements.
        assert!(
            set.set_authority(Kind::L4, 0, Authority::Automatic),
            "the renderer is a node of this Set"
        );
        let refused = set
            .write_param(&ParamWrite::everywhere("radius", 7.0))
            .expect_err("a bare name over a kept node and a granted one is refused");
        assert_eq!(refused.key, "radius");
        assert_eq!(
            refused.landing, "L1:0 manual, L4:0 automatic",
            "the refusal names the nodes it would have landed on and what each is under"
        );
        assert_eq!(
            radius(&set),
            vec![(Kind::L1, 0, 2.0), (Kind::L3, 0, 8.0), (Kind::L4, 0, 2.0)],
            "a refused write moves nothing — landing on the permitted node is the \
             silently partial control P-0094 rules out"
        );

        // And the reach is not what was taken away.
        assert_eq!(
            set.write_param(&ParamWrite::at(Kind::L4, 0, "radius", 7.0)),
            Ok(1),
            "an addressed write says which node it means, so it crosses nothing"
        );
        assert_eq!(
            radius(&set),
            vec![(Kind::L1, 0, 2.0), (Kind::L3, 0, 8.0), (Kind::L4, 0, 7.0)],
            "the addressed write lands on the node it names and on no other"
        );
    }

    /// Verifies that derived `Counts` in amplifier stages correctly scales all element counts
    /// including `survivors`, preserving count field invariants across pipeline stages.
    #[test]
    fn an_amplifiers_derived_counts_multiply_every_element_count_and_no_other_field() {
        // Was a silent `return` — the one test in the workspace that
        // reported success for having done nothing at all. Now the same
        // panic as the rest; a machine without an adapter uses
        // `--skip gpu::` rather than a test that lies to it.
        let gpu = Gpu::headless().expect("no GPU available");
        const FACTOR: u32 = 4;
        const CAPACITY: u32 = 64;

        let compile = |src: &str| -> Checked {
            let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
            let checked =
                karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"));
            karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("cost: {e:?}"));
            checked
        };
        let l1 = compile(
            r#"
proc still {
  kind     L1
  topology points
  capacity [64, 64] = 64

  emit position

  element {
position = vec3(0.0, 0.0, 0.0);
  }
}
"#,
        );
        let l2 = compile(
            r#"
proc mirror {
  kind    L2
  amplify 4

  consumes position

  deform {
position = position + vec3(0.0, float(copy), 0.0);
  }
}
"#,
        );
        let l4 = compile(
            r#"
proc dots {
  kind  L4
  blend additive

  consumes position

  vertex {
clip       = camera * vec4(position, 1.0);
point_rate = 0.004;
  }

  fragment {
color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#,
        );
        let set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&l1, CAPACITY)],
            &[&l2],
            &[],
            &[],
            &[&l4],
            Layering::Overdraw,
            1,
            &[],
            Wiring::default(),
        )
        .expect("a chain of one L1, one amplifying L2 and one L4");

        // `build_many` primes the chain, so the derived counts are current
        // without a step — which is the other thing this asserts.
        let read = |buf: &wgpu::Buffer| -> [u32; 12] {
            let size = karakuri_codegen::layout::counts::SIZE;
            let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("counts readback"),
                size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            encoder.copy_buffer_to_buffer(buf, 0, &readback, 0, size);
            gpu.queue.submit([encoder.finish()]);
            let slice = readback.slice(..);
            slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
            gpu.device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("poll");
            let data = slice.get_mapped_range().expect("map");
            let mut out = [0u32; 12];
            for (i, w) in data.chunks_exact(4).take(12).enumerate() {
                out[i] = u32::from_le_bytes([w[0], w[1], w[2], w[3]]);
            }
            drop(data);
            readback.unmap();
            out
        };

        let source = &set.sources[0];
        let from = read(source.sim.counts());
        let derived = read(source.deforms[0].counts().expect("the node amplifies"));

        // Field order is `counts::WGSL`'s: elem_xyz, range, vertex_count,
        // instance_count, first_vertex, first_instance, survivors.
        assert_eq!(derived[3], from[3] * FACTOR, "range");
        assert_eq!(derived[5], from[5] * FACTOR, "instance_count");
        assert_eq!(derived[8], from[8] * FACTOR, "survivors");
        assert_eq!(
            derived[0],
            from[3] * FACTOR / karakuri_codegen::layout::WORKGROUP_SIZE,
            "workgroups, over the amplified range"
        );
        assert_eq!((derived[1], derived[2]), (1, 1), "the other two dimensions");

        // Verify non-element count fields (`vertex_count`, `first_vertex`, `first_instance`)
        // are unaffected by the amplifier factor.
        assert_eq!(derived[4], from[4], "vertex_count");
        assert_eq!(derived[6], from[6], "first_vertex");
        assert_eq!(derived[7], from[7], "first_instance");
    }
}
