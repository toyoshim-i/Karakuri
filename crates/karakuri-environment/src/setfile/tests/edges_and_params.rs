#![allow(unused_imports)]

use super::common::*;

/// The edge survives the file, and so do the names it points with.
///
/// The two halves are one fact: an edge is between *names*, and a file that
/// carried the edge and dropped the names would come back naming nodes that are
/// no longer called that. What makes it round-trip at all is that the names it
/// points with are either written down beside the node — as `far` is here — or
/// derived from the procedure, which is a function of the artifact the `slot`
/// record already references.
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
    // **After the slots**, so a reader has every name in hand by the time
    // it meets the record that uses them.
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

/// A Source-slot edge survives the file, with the name it points with.
///
/// The record is the same `edge` a geometry slot writes — node, slot, and the
/// node it is bound to — because what an edge says is one fact whatever type
/// the slot was declared with. That is the claim: the fourth slot type cost
/// this file nothing, and a Set whose mask names a source can be saved and
/// loaded like any other.
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
            // **The salts are the identities the mask compares**, so a
            // saved Set that gave them back differently would be a mask
            // pointing at a different geometry after a reload.
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

/// A file written before the address existed still means what it meant.
///
/// `layer` on a `param` record was a placeholder: the writer put `L1` on
/// everything and said so in a comment, and the loader ignored it. So honouring
/// `layer` now would silently retarget every Set file ever written — an
/// `exposure` that reached the renderer would start reaching the L1 and doing
/// nothing.
///
/// What stops that is the address being `(layer, index)` present or absent as a
/// unit: no `index`, no address, whatever `layer` says. This reads a
/// hand-written old-style file to prove it, rather than one this build produced
/// — a round trip through the new writer would agree with itself however wrong
/// both halves were.
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

/// What could not be carried is said, not dropped. Three shapes, and each is a
/// real disagreement between what the format can address and what the engine
/// has: a salt and a capacity belong to a *geometry*, so one written against a
/// renderer names something that does not exist, and a vector param has no
/// `f32` to become.
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

/// A vector `param` line becomes one write per component.
///
/// This is where the wide `Value` earns its keep: a file — or a model through
/// one MCP call — says the vector once, and the reader expands it into the
/// three writes the engine can carry, because a parameter is driven one
/// component at a time
/// (`docs/adr/0268-a-vector-parameter-is-driven-one-component-at-a-time.md`).
/// It used to be reported and dropped, with *"the engine holds scalar parameter
/// values only"*.
///
/// The order is the components' own, not the file's and not a map's: `glow.x`
/// then `glow.y` then `glow.z`, carrying `0.4`, `0.7`, `1.0` in the order the
/// line wrote them. A reversal here would be a Set that loads and is the wrong
/// colour.
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

/// One component, written and read back as itself — and the same three numbers
/// however the file spells them.
///
/// What a `save` puts on the line is components, one `param` record each, which
/// is what lets a Set file record the single component an operator moved. What
/// a *person or a model* writes is the vector, once. The last assertion is that
/// the two spellings load to the same list: the wide value earns its keep on
/// the line and nowhere past it.
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

    // **The same Set, spelled as one vector line.** Three `param` records
    // under the component keys and one under the declared name are two
    // spellings of one thing, and a file that meant different things by
    // them would be a format with two answers.
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

/// A single number against a `vec3` names no component, and the note says which
/// keys would — the refusal carries what the next attempt needs
/// (`docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md`).
/// Reported and skipped rather than landed on a component this reader picked,
/// which would be inventing an address the file did not write.
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

/// A `bind` on a bare vector key is refused, and the sentence spells the
/// components.
///
/// A binding resolves to one number and a `vec3` has three places to put it.
/// `Set::bind` would answer `Bound::NoSuchParam`, which says the parameter does
/// not exist — not what is wrong, and not what the next attempt needs.
///
/// Paired with the binding that must be accepted, because a reader that refused
/// every binding would pass a test made only of refusals: one component is an
/// ordinary key and binds like any scalar.
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
