#![allow(unused_imports)]

use super::common::*;

/// Verifies that edges and their associated node names round-trip through file serialization.
#[test]
fn an_edge_and_the_names_it_points_with_survive_the_file() {
    let (dir, store, l1, l4) = fixture();
    let l1b = beside(&dir, "l1b.kir", &L1.replace("proc ring", "proc ring_two"));
    let l2 = beside(&dir, "l2.kir", L2);
    let nodes = vec![
        Node {
            hash: stored(&store, &l1),
            layer: Kind::L1,
            index: 0,
            name: None,
        },
        Node {
            hash: stored(&store, &l1b),
            layer: Kind::L1,
            index: 1,
            // Written, because it is what the edge points with.
            name: Some("far".to_string()),
        },
        Node {
            hash: stored(&store, &l2),
            layer: Kind::L2,
            index: 0,
            name: None,
        },
        Node {
            hash: stored(&store, &l4),
            layer: Kind::L4,
            index: 0,
            name: None,
        },
    ];
    let edges = vec![karakuri_engine::set::Edge {
        node: "warp".to_string(),
        slot: "far".into(),
        to: "far".to_string(),
    }];
    save(
        &store,
        Asked::Operator,
        "wired",
        Saving {
            nodes: &nodes,
            capacities: &[4096, 4096],
            params: &[],
            bindings: &[],
            edges: &edges,
            camera: &DEFAULT_CAMERA,
            layering: Layering::Overdraw,
            live: None,
            seeds: &[1, 2],
        },
    )
    .expect("save");

    let text = written(&store, "wired");
    assert!(
        text.contains(r#"{"t":"edge","node":"warp","slot":"far","to":"far"}"#),
        "the edge is one line naming both ends: {text}"
    );
    // Edges serialise after slot declarations.
    assert!(
        text.find(r#""t":"edge""#) > text.rfind(r#""t":"slot""#),
        "an edge is written after the slots it names: {text}"
    );

    let loaded = load(&store, "wired").expect("load");
    assert_eq!(loaded.edges, edges, "the edge came back as it went in");
    assert_eq!(loaded.names.l1s, [None, Some("far".to_string())]);
    assert!(
        loaded.notes.is_empty(),
        "nothing here is unhonourable: {:?}",
        loaded.notes
    );
}

/// A deformation that names one of the Set's sources through a slot.
const DISSOLVE: &str = r#"
proc dissolve {
  kind L2

  uses only : Source

  consumes position

  mask {
    strength = 0.0;
    if source == only { strength = 1.0; }
  }

  deform {
    position = position * 0.2;
  }
}
"#;

/// Verifies round-trip persistence of source-slot edges and target node names.
#[test]
fn a_source_slot_edge_survives_the_file() {
    let (dir, store, l1, l4) = fixture();
    let l1b = beside(&dir, "l1b.kir", &L1.replace("proc ring", "proc ring_two"));
    let l2 = beside(&dir, "l2.kir", DISSOLVE);
    let nodes = vec![
        Node {
            hash: stored(&store, &l1),
            layer: Kind::L1,
            index: 0,
            name: None,
        },
        Node {
            hash: stored(&store, &l1b),
            layer: Kind::L1,
            index: 1,
            // Written, because it is what the edge points with — and here
            // it is what the mask *means*, rather than a second buffer.
            name: Some("victim".to_string()),
        },
        Node {
            hash: stored(&store, &l2),
            layer: Kind::L2,
            index: 0,
            name: None,
        },
        Node {
            hash: stored(&store, &l4),
            layer: Kind::L4,
            index: 0,
            name: None,
        },
    ];
    let edges = vec![karakuri_engine::set::Edge {
        node: "dissolve".to_string(),
        slot: "only".into(),
        to: "victim".to_string(),
    }];
    save(
        &store,
        Asked::Operator,
        "masked",
        Saving {
            nodes: &nodes,
            capacities: &[4096, 4096],
            params: &[],
            bindings: &[],
            edges: &edges,
            camera: &DEFAULT_CAMERA,
            // Preserves geometry salt identities across reload.
            layering: Layering::Overdraw,
            live: None,
            seeds: &[11, 22],
        },
    )
    .expect("save");

    let text = written(&store, "masked");
    assert!(
        text.contains(r#"{"t":"edge","node":"dissolve","slot":"only","to":"victim"}"#),
        "one line, both ends, and nothing about the type: {text}"
    );

    let loaded = load(&store, "masked").expect("load");
    assert_eq!(loaded.edges, edges, "the edge came back as it went in");
    assert_eq!(loaded.names.l1s, [None, Some("victim".to_string())]);
    assert_eq!(
        loaded.salts,
        vec![Some(11), Some(22)],
        "and so did the salts the mask compares against"
    );
    assert!(
        loaded.notes.is_empty(),
        "nothing here is unhonourable: {:?}",
        loaded.notes
    );
}

fn a_binding() -> Binding {
    Binding::new(Kind::L1, "spin", "beat", Curve::Pow2, [0.5, 3.0])
}

/// Everything a Set file is for, in one assertion: what went in comes back out.
/// A format that carried the material and lost the parameters would still load,
/// still render, and still be the wrong Set.
#[test]
fn a_saved_set_loads_back_as_what_was_saved() {
    let (_dir, store, l1, l4) = fixture();
    let params = vec![ParamWrite::everywhere("radius", 3.25)];
    let camera = Orbit {
        radius: 11.5,
        speed: 0.42,
        height: -3.75,
        ..Orbit::default()
    };
    save(
        &store,
        Asked::Operator,
        "s1",
        Saving {
            nodes: &ordinary(&store, &l1, std::slice::from_ref(&l4)),
            capacities: &[65_536],
            params: &params,
            bindings: &[a_binding()],
            edges: &[],
            camera: &camera,
            layering: Layering::Overdraw,
            live: None,
            seeds: &[4242],
        },
    )
    .expect("save");

    let loaded = load(&store, "s1").expect("load");
    assert_eq!(loaded.id, "s1");
    assert_eq!(loaded.capacities, vec![Some(65_536)]);
    assert_eq!(loaded.params, params);
    assert_eq!(loaded.salts, vec![Some(4242)]);
    assert_eq!(
        loaded.camera.map(|c| (c.radius, c.speed, c.height)),
        Some((11.5, 0.42, -3.75)),
        "the three placement numbers are what a Set file carries about the built-in \
             camera, and the height was the one it dropped until ADR-0318"
    );
    assert_eq!(loaded.bindings.len(), 1);
    let back = &loaded.bindings[0];
    assert_eq!(back.layer, Kind::L1);
    assert_eq!(back.key, "spin");
    assert_eq!(back.signal, "beat");
    assert_eq!(back.curve, Curve::Pow2);
    assert_eq!(back.range, [0.5, 3.0]);
    // And the procedures themselves came back through the store, compiled.
    assert!(
        loaded.notes.is_empty(),
        "unexpected notes: {:?}",
        loaded.notes
    );
}

/// The material is resolved by hash out of the store, which is what makes a Set
/// file a few dozen lines rather than a copy of the source. A file whose
/// artifacts are missing says so instead of loading something else.
#[test]
fn a_set_whose_artifacts_are_missing_says_which_and_why() {
    let (_dir, store, l1, l4) = fixture();
    save(
        &store,
        Asked::Operator,
        "s1",
        plain(&ordinary(&store, &l1, std::slice::from_ref(&l4)), &[]),
    )
    .expect("save");
    let lines = store.read_set("s1").expect("read");

    // A second store that has the file but not the artifacts — a Set file
    // carried to a machine that has never seen the procedures.
    let elsewhere = tempfile::tempdir().expect("tempdir");
    let bare = Store::open(elsewhere.path()).expect("store");
    let err = from_lines(&bare, "s1", &lines).expect_err("the artifacts are not there");
    assert!(err.contains("the store does not have it"), "{err}");
    assert!(err.contains("inlined"), "{err}");
}

/// The bundled form: a file carrying its own source reads on a machine whose
/// store has never seen the artifact. Inlined source wins over the store, so a
/// bundle is self-contained rather than half-resolved.
#[test]
fn inlined_source_loads_without_a_store_that_knows_the_artifact() {
    let (_dir, store, l1, l4) = fixture();
    save(
        &store,
        Asked::Operator,
        "s1",
        plain(&ordinary(&store, &l1, std::slice::from_ref(&l4)), &[]),
    )
    .expect("save");
    let mut lines = store.read_set("s1").expect("read");
    // Bundle it: every slot's source inlined, line by line, as `src`.
    let mut bundled = Vec::new();
    for line in &lines {
        if let Record::Slot { proc_hash, at, .. } = line.record() {
            let src = match at.layer {
                Layer::L1 => L1,
                _ => L4,
            };
            for (n, text) in src.lines().enumerate() {
                bundled.push(Line::new(Record::Src {
                    hash: *proc_hash,
                    line: n as u32,
                    s: text.to_string(),
                }));
            }
        }
    }
    lines.append(&mut bundled);

    let elsewhere = tempfile::tempdir().expect("tempdir");
    let bare = Store::open(elsewhere.path()).expect("store");
    let loaded = from_lines(&bare, "s1", &lines).expect("the file carries its own source");
    assert_eq!(loaded.l1s[0].name, "ring");
    assert_eq!(loaded.l4s[0].name, "points");
}

/// Verifies legacy param records without an index are treated as wildcard writes across layers.
#[test]
fn a_param_record_without_an_index_is_a_wildcard_whatever_its_layer_says() {
    let (_dir, store, l1, l4) = fixture();
    let hashes: Vec<String> = [&l1, &l4]
        .iter()
        .map(|p| {
            let bytes = std::fs::read(p).expect("read");
            store.put_artifact(&bytes).expect("put").to_string()
        })
        .collect();
    let text = format!(
        r#"{{"t":"set","id":"old","v":1}}
{{"t":"slot","layer":"L1","proc":"{}"}}
{{"t":"slot","layer":"L4","proc":"{}"}}
{{"t":"param","layer":"L1","key":"exposure","value":0.4}}
{{"t":"param","layer":"L4","index":1,"key":"exposure","value":0.9}}
"#,
        hashes[0], hashes[1]
    );
    let lines = parsed(&text);

    let loaded = from_lines(&store, "old", &lines).expect("an old-style file still loads");
    assert_eq!(
        loaded.params,
        vec![
            // No index: a wildcard, even though the record says `L1`.
            ParamWrite::everywhere("exposure", 0.4),
            // An index: an address, and `layer` is load-bearing beside it.
            ParamWrite::at(Kind::L4, 1, "exposure", 0.9),
        ]
    );
}

/// Ensures unsupported parameter or seed records produce diagnostics rather than silent omissions.
#[test]
fn what_the_engine_cannot_carry_is_reported_rather_than_dropped() {
    let (_dir, store, l1, l4) = fixture();
    save(
        &store,
        Asked::Operator,
        "s1",
        plain(&ordinary(&store, &l1, std::slice::from_ref(&l4)), &[]),
    )
    .expect("save");
    let mut lines = store.read_set("s1").expect("read");
    lines.push(Line::new(Record::Seed {
        stream: Layer::L4,
        index: 0,
        value: 7,
    }));
    lines.push(Line::new(Record::Capacity {
        at: NodeAddress {
            layer: Layer::L4,
            index: 0,
        },
        value: 128,
    }));
    lines.push(Line::new(Record::Param {
        at: None,
        key: "tint".to_string(),
        value: Value::Vec3([1.0, 0.0, 0.0]),
    }));

    let loaded = from_lines(&store, "s1", &lines).expect("load");
    let notes = loaded.notes.join("\n");
    assert!(notes.contains("seed on L4"), "{notes}");
    assert!(notes.contains("capacity on L4"), "{notes}");
    assert!(notes.contains("`tint`"), "{notes}");
    // The L1 values are still the ones applied: a note is not a refusal.
    assert_eq!(loaded.salts, vec![Some(1)]);
    assert_eq!(loaded.capacities, vec![Some(4096)]);
}

/// A renderer with a vector `param`, which the pair above has none of. The
/// declaration is `docs/ir-spec.md`'s own `param` example.
const GLOWING: &str = r#"
proc glowing {
  kind  L4
  blend additive

  param glow : vec3 [0.0, 4.0] = vec3(0.4, 0.7, 1.0)

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.016;
  }

  fragment {
    color = vec4(glow, 1.0);
  }
}
"#;

/// A store holding the ordinary L1 and [`GLOWING`], and the nodes naming them.
/// Every vector-param test below starts here.
fn glowing_fixture() -> (tempfile::TempDir, Store, Vec<Node>) {
    let (dir, store, l1, _l4) = fixture();
    let glowing = beside(&dir, "glowing.kir", GLOWING);
    let nodes = ordinary(&store, &l1, std::slice::from_ref(&glowing));
    (dir, store, nodes)
}

/// The lines a `plain` save writes for [`glowing_fixture`], plus whatever the
/// test appends.
fn glowing_lines(store: &Store, nodes: &[Node], extra: Vec<Record>) -> Vec<Line> {
    save(store, Asked::Operator, "g1", plain(nodes, &[])).expect("save");
    let mut lines = store.read_set("g1").expect("read");
    lines.extend(extra.into_iter().map(Line::new));
    lines
}

/// Verifies vector param records expand into individual component writes (x, y, z) per ADR-0268.
#[test]
fn a_vector_param_record_is_expanded_into_its_components() {
    let (_dir, store, nodes) = glowing_fixture();
    let lines = glowing_lines(
        &store,
        &nodes,
        vec![Record::Param {
            at: Some(NodeAddress {
                layer: Layer::L4,
                index: 0,
            }),
            key: "glow".to_string(),
            value: Value::Vec3([0.4, 0.7, 1.0]),
        }],
    );

    let loaded = from_lines(&store, "g1", &lines).expect("load");
    assert_eq!(
        loaded.params,
        vec![
            ParamWrite::at(Kind::L4, 0, "glow.x", 0.4),
            ParamWrite::at(Kind::L4, 0, "glow.y", 0.7),
            ParamWrite::at(Kind::L4, 0, "glow.z", 1.0),
        ]
    );
    assert!(
        !loaded.notes.iter().any(|n| n.contains("glow")),
        "a vector param is carried now, not reported: {:?}",
        loaded.notes
    );
}

/// Verifies single-component writes round-trip and produce equivalent results to full vector writes.
#[test]
fn a_component_write_round_trips_through_the_file() {
    let (_dir, store, nodes) = glowing_fixture();
    let moved = [
        ParamWrite::at(Kind::L4, 0, "glow.y", 0.7),
        ParamWrite::everywhere("glow.z", 2.0),
    ];
    save(
        &store,
        Asked::Operator,
        "g1",
        Saving {
            params: &moved,
            ..plain(&nodes, &[])
        },
    )
    .expect("save");
    let text = written(&store, "g1");
    assert!(
        text.contains(r#""key":"glow.y","value":0.7"#),
        "a component is written as a scalar under its own key: {text}"
    );

    let loaded = load(&store, "g1").expect("load");
    assert_eq!(
        loaded.params,
        vec![
            // `None` sorts first: the Set-wide write above the addressed
            // one, which is the order `save` puts them in.
            ParamWrite::everywhere("glow.z", 2.0),
            ParamWrite::at(Kind::L4, 0, "glow.y", 0.7),
        ]
    );
    assert!(
        loaded.notes.is_empty(),
        "a component key is an ordinary scalar write: {:?}",
        loaded.notes
    );

    // Single vector parameter line encodes equivalent values to component lines.
    let spelled_out = Saving {
        params: &[
            ParamWrite::at(Kind::L4, 0, "glow.x", 0.4),
            ParamWrite::at(Kind::L4, 0, "glow.y", 0.7),
            ParamWrite::at(Kind::L4, 0, "glow.z", 1.0),
        ],
        ..plain(&nodes, &[])
    };
    save(&store, Asked::Operator, "g2", spelled_out).expect("save");
    let components = load(&store, "g2").expect("load").params;
    let mut as_a_vector = store.read_set("g2").expect("read");
    as_a_vector.retain(|line| !matches!(line.record(), Record::Param { .. }));
    as_a_vector.push(Line::new(Record::Param {
        at: Some(NodeAddress {
            layer: Layer::L4,
            index: 0,
        }),
        key: "glow".to_string(),
        value: Value::Vec3([0.4, 0.7, 1.0]),
    }));
    assert_eq!(
        from_lines(&store, "g2", &as_a_vector).expect("load").params,
        components,
        "one vector line and three component lines are the same Set"
    );
}

/// Ensures assigning a scalar to a vector parameter reports available component keys per P-0083.
#[test]
fn a_scalar_against_a_vector_declaration_is_reported_with_its_components() {
    let (_dir, store, nodes) = glowing_fixture();
    let lines = glowing_lines(
        &store,
        &nodes,
        vec![Record::Param {
            at: Some(NodeAddress {
                layer: Layer::L4,
                index: 0,
            }),
            key: "glow".to_string(),
            value: Value::Scalar(0.5),
        }],
    );

    let loaded = from_lines(&store, "g1", &lines).expect("load");
    assert!(loaded.params.is_empty(), "{:?}", loaded.params);
    let notes = loaded.notes.join("\n");
    for want in ["`glow`", "`vec3`", "`glow.x`", "`glow.y`", "`glow.z`"] {
        assert!(notes.contains(want), "{want} is not in the note: {notes}");
    }
}

/// Ensures bindings targeting a bare vector key are rejected with suggested component targets.
#[test]
fn a_binding_on_a_bare_vector_key_is_refused_with_the_component_spelling() {
    let (_dir, store, nodes) = glowing_fixture();
    let lines = glowing_lines(
        &store,
        &nodes,
        vec![
            Record::Bind {
                layer: Layer::L4,
                index: None,
                key: "glow".to_string(),
                signal: "energy".to_string(),
                curve: "lin".to_string(),
                range: [0.0, 1.0],
                noise: None,
            },
            Record::Bind {
                layer: Layer::L4,
                index: None,
                key: "glow.y".to_string(),
                signal: "energy".to_string(),
                curve: "lin".to_string(),
                range: [0.0, 1.0],
                noise: None,
            },
        ],
    );

    let loaded = from_lines(&store, "g1", &lines).expect("load");
    assert_eq!(
        loaded.bindings.len(),
        1,
        "one component binds; the bare name does not"
    );
    assert_eq!(loaded.bindings[0].key, "glow.y");
    let notes = loaded.notes.join("\n");
    for want in ["`glow.x`", "`glow.y`", "`glow.z`", "skipped"] {
        assert!(notes.contains(want), "{want} is not in the note: {notes}");
    }
}

/// A binding the engine cannot honour is reported and skipped, and the load
/// still succeeds. One unusable binding is not a reason to refuse the material,
/// and the note names the parameter that will not move.
#[test]
fn an_unusable_binding_is_named_and_the_rest_of_the_set_still_loads() {
    let (_dir, store, l1, l4) = fixture();
    save(
        &store,
        Asked::Operator,
        "s1",
        plain(
            &ordinary(&store, &l1, std::slice::from_ref(&l4)),
            &[a_binding()],
        ),
    )
    .expect("save");
    let mut lines = store.read_set("s1").expect("read");
    lines.push(Line::new(Record::Bind {
        layer: Layer::L1,
        index: None,
        key: "radius".to_string(),
        signal: "bpm".to_string(),
        curve: "lin".to_string(),
        range: [0.0, 1.0],
        noise: None,
    }));

    let loaded = from_lines(&store, "s1", &lines).expect("load");
    assert_eq!(loaded.bindings.len(), 1, "the good binding survived");
    let notes = loaded.notes.join("\n");
    assert!(notes.contains("bpm"), "{notes}");
    assert!(notes.contains("skipped"), "{notes}");
}

/// Bindings to `audio.energy`, `sub`, `audio.bass`, `mid`, and `air` round-trip through Set files.
#[test]
fn audio_and_named_spectral_band_bindings_survive_set_file() {
    let (_dir, store, l1, l4) = fixture();
    let audio_bindings = vec![
        Binding::new(Kind::L1, "radius", "audio.energy", Curve::Lin, [0.1, 5.0]),
        Binding::new(Kind::L1, "spin", "sub", Curve::Pow2, [0.0, 2.0]),
        Binding::new(Kind::L4, "hue", "audio.bass", Curve::Smooth, [0.2, 0.8]),
        Binding::new(Kind::L4, "point_scale", "mid", Curve::Sqrt, [0.01, 0.2]),
        Binding::new(Kind::L4, "falloff", "air", Curve::Lin, [0.5, 4.0]),
    ];

    save(
        &store,
        Asked::Operator,
        "audio_set",
        plain(
            &ordinary(&store, &l1, std::slice::from_ref(&l4)),
            &audio_bindings,
        ),
    )
    .expect("save");

    let loaded = load(&store, "audio_set").expect("load");
    assert_eq!(loaded.bindings.len(), 5);
    assert_eq!(loaded.bindings[0].signal, "audio.energy");
    assert_eq!(
        loaded.bindings[0].signal_id,
        karakuri_signal::SignalId::Energy
    );
    assert_eq!(loaded.bindings[1].signal, "sub");
    assert_eq!(
        loaded.bindings[1].signal_id,
        karakuri_signal::SignalId::Band(0)
    );
    assert_eq!(loaded.bindings[2].signal, "audio.bass");
    assert_eq!(
        loaded.bindings[2].signal_id,
        karakuri_signal::SignalId::Band(1)
    );
    assert_eq!(loaded.bindings[3].signal, "mid");
    assert_eq!(
        loaded.bindings[3].signal_id,
        karakuri_signal::SignalId::Band(3)
    );
    assert_eq!(loaded.bindings[4].signal, "air");
    assert_eq!(
        loaded.bindings[4].signal_id,
        karakuri_signal::SignalId::Band(7)
    );
    assert!(
        loaded.notes.is_empty(),
        "all audio bindings should be honoured: {:?}",
        loaded.notes
    );
}
