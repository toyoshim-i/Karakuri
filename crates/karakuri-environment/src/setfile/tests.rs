use super::*;
use karakuri_engine::binding::{Curve, NOISE_SIGNAL};
use karakuri_engine::camera::Orbit;
use karakuri_engine::set::Layering;
use karakuri_engine::{Binding, ParamWrite};
use karakuri_ir::typed::Checked;
use karakuri_ir::Kind;
use karakuri_signal::{NoiseConfig, NoiseKind};
use karakuri_store::hash::Hash;
use karakuri_store::ndjson::Line;
use karakuri_store::record::{BindNoise, Layer, NodeAddress, Record, Value};
use karakuri_store::store::Store;

use crate::Asked;

const L1: &str = r#"
proc ring {
  kind     L1
  topology points
  capacity [1024, 262144] = 4096

  param radius : float [0.1, 8.0] = 2.5
  param spin   : float [0.0, 4.0] = 1.0

  emit position, age

  element {
    let a = t * spin + hash1(seed) * 1.2;
    position = vec3(cos(a) * radius, sin(a) * radius, 0.0);
    age      = t;
  }
}
"#;

const L4: &str = r#"
proc points {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.016;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0 - length(point_coord * 2.0 - 1.0));
  }
}
"#;

/// The rest of the chain a `--set` can spell, minimal for the reason the
/// pair above is: what is under test is the file, not the picture.
const L2: &str = r#"
proc warp {
  kind L2

  consumes position

  deform {
    position = vec3(position.x, position.y * 1.5, position.z);
  }
}
"#;

const L3: &str = r#"
proc look {
  kind L3

  camera {
    eye    = vec3(0.0, 2.0, 9.0);
    target = vec3(0.0, 0.0, 0.0);
  }
}
"#;

const FIELD: &str = r#"
proc blob {
  kind Field

  field {
    distance = sd_sphere(point, 1.0);
  }
}
"#;

/// A store with the two procedures written out beside it, and the paths.
fn fixture() -> (
    tempfile::TempDir,
    Store,
    std::path::PathBuf,
    std::path::PathBuf,
) {
    let dir = tempfile::tempdir().expect("tempdir");
    let l1 = dir.path().join("l1.kir");
    let l4 = dir.path().join("l4.kir");
    std::fs::write(&l1, L1).expect("write l1");
    std::fs::write(&l4, L4).expect("write l4");
    let store = Store::open(dir.path().join("store")).expect("store");
    (dir, store, l1, l4)
}

/// A `.kir` on disk beside the fixture's, so a test can name a node of any
/// layer it likes.
fn beside(dir: &tempfile::TempDir, file: &str, src: &str) -> std::path::PathBuf {
    let path = dir.path().join(file);
    std::fs::write(&path, src).expect("write");
    path
}

/// **A fixture's source, in the store, as [`save`] now wants it.** The
/// tests here are written against files on disk, because a file is what a
/// fixture is; the writer takes hashes. This is the one line that bridges
/// them, rather than every test growing its own `put`.
fn stored(store: &Store, path: &std::path::Path) -> Hash {
    store
        .put_artifact(&std::fs::read(path).expect("read"))
        .expect("put")
}

/// The nodes of an ordinary Set — one geometry, and the renderers over it
/// in draw order.
fn ordinary(store: &Store, l1: &std::path::Path, l4s: &[std::path::PathBuf]) -> Vec<Node> {
    std::iter::once(Node {
        hash: stored(store, l1),
        layer: Kind::L1,
        index: 0,
        name: None,
    })
    .chain(l4s.iter().enumerate().map(|(at, path)| Node {
        hash: stored(store, path),
        layer: Kind::L4,
        index: at as u32,
        name: None,
    }))
    .collect()
}

/// The whole set file as bytes, which is what a compatibility claim is
/// about. `read` keeps each line's text verbatim, so this is what is on
/// disk rather than a re-serialisation of it.
fn written(store: &Store, id: &str) -> String {
    store
        .read_set(id)
        .expect("read")
        .iter()
        .map(|line| format!("{}\n", line.as_str()))
        .collect()
}

/// A Set file's text, through the file reader — so the bytes a test writes
/// out are genuinely parsed, rather than hand-built into records that could
/// not have been written. The file is gone by the time this returns; the
/// lines are in memory.
fn parsed(text: &str) -> Vec<Line> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("hand_written.kbset");
    std::fs::write(&path, text).expect("write");
    karakuri_store::ndjson::read(&path).expect("a hand-written Set file parses")
}

/// A Set with nothing but its material and whatever bindings are given.
/// `Orbit::default()` is not `const`, so this is the one place a test names
/// its fields; `LazyLock` keeps that to one place rather than one per call.
static DEFAULT_CAMERA: std::sync::LazyLock<Orbit> = std::sync::LazyLock::new(Orbit::default);

fn plain<'a>(nodes: &'a [Node], bindings: &'a [Binding]) -> Saving<'a> {
    Saving {
        nodes,
        capacities: &[4096],
        params: &[],
        bindings,
        edges: &[],
        camera: &DEFAULT_CAMERA,
        layering: Layering::Overdraw,
        live: None,
        seeds: &[1],
    }
}

/// **The edge survives the file, and so do the names it points with.**
///
/// The two halves are one fact: an edge is between *names*, and a file that
/// carried the edge and dropped the names would come back naming nodes that
/// are no longer called that. What makes it round-trip at all is that the
/// names it points with are either written down beside the node — as `far`
/// is here — or derived from the procedure, which is a function of the
/// artifact the `slot` record already references.
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

/// **A Source-slot edge survives the file**, with the name it points with.
///
/// The record is the same `edge` a geometry slot writes — node, slot, and
/// the node it is bound to — because what an edge says is one fact whatever
/// type the slot was declared with. That is the claim: the fourth slot type
/// cost this file nothing, and a Set whose mask names a source can be saved
/// and loaded like any other.
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

/// **Everything a Set file is for, in one assertion**: what went in comes
/// back out. A format that carried the material and lost the parameters
/// would still load, still render, and still be the wrong Set.
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

/// **The material is resolved by hash out of the store**, which is what
/// makes a Set file a few dozen lines rather than a copy of the source. A
/// file whose artifacts are missing says so instead of loading something
/// else.
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

/// The bundled form: a file carrying its own source reads on a machine
/// whose store has never seen the artifact. **Inlined source wins over the
/// store**, so a bundle is self-contained rather than half-resolved.
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

/// **A file written before the address existed still means what it meant.**
///
/// `layer` on a `param` record was a placeholder: the writer put `L1` on
/// everything and said so in a comment, and the loader ignored it. So
/// honouring `layer` now would silently retarget every Set file ever
/// written — an `exposure` that reached the renderer would start reaching
/// the L1 and doing nothing.
///
/// What stops that is the address being `(layer, index)` present or absent
/// **as a unit**: no `index`, no address, whatever `layer` says. This reads
/// a hand-written old-style file to prove it, rather than one this build
/// produced — a round trip through the new writer would agree with itself
/// however wrong both halves were.
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

/// **What could not be carried is said, not dropped.** Three shapes, and
/// each is a real disagreement between what the format can address and what
/// the engine has: a salt and a capacity belong to a *geometry*, so one
/// written against a renderer names something that does not exist, and a
/// vector param has no `f32` to become.
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

/// A renderer with a vector `param`, which the pair above has none of.
/// The declaration is `docs/ir-spec.md`'s own `param` example.
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

/// A store holding the ordinary L1 and [`GLOWING`], and the nodes naming
/// them. Every vector-param test below starts here.
fn glowing_fixture() -> (tempfile::TempDir, Store, Vec<Node>) {
    let (dir, store, l1, _l4) = fixture();
    let glowing = beside(&dir, "glowing.kir", GLOWING);
    let nodes = ordinary(&store, &l1, std::slice::from_ref(&glowing));
    (dir, store, nodes)
}

/// The lines a `plain` save writes for [`glowing_fixture`], plus whatever
/// the test appends.
fn glowing_lines(store: &Store, nodes: &[Node], extra: Vec<Record>) -> Vec<Line> {
    save(store, Asked::Operator, "g1", plain(nodes, &[])).expect("save");
    let mut lines = store.read_set("g1").expect("read");
    lines.extend(extra.into_iter().map(Line::new));
    lines
}

/// **A vector `param` line becomes one write per component.**
///
/// This is where the wide `Value` earns its keep: a file — or a model
/// through one MCP call — says the vector once, and the reader expands it
/// into the three writes the engine can carry, because a parameter is
/// driven one component at a time
/// (`docs/adr/0268-a-vector-parameter-is-driven-one-component-at-a-time.md`).
/// It used to be reported and dropped, with *"the engine holds scalar
/// parameter values only"*.
///
/// **The order is the components' own**, not the file's and not a map's:
/// `glow.x` then `glow.y` then `glow.z`, carrying `0.4`, `0.7`, `1.0` in
/// the order the line wrote them. A reversal here would be a Set that
/// loads and is the wrong colour.
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

/// **One component, written and read back as itself — and the same three
/// numbers however the file spells them.**
///
/// What a `save` puts on the line is components, one `param` record each,
/// which is what lets a Set file record the single component an operator
/// moved. What a *person or a model* writes is the vector, once. The last
/// assertion is that the two spellings load to the same list: the wide
/// value earns its keep on the line and nowhere past it.
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

/// **A single number against a `vec3` names no component**, and the note
/// says which keys would — the refusal carries what the next attempt needs
/// (`docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md`).
/// Reported and skipped rather than landed on a component this reader
/// picked, which would be inventing an address the file did not write.
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

/// **A `bind` on a bare vector key is refused, and the sentence spells the
/// components.**
///
/// A binding resolves to one number and a `vec3` has three places to put
/// it. `Set::bind` would answer `Bound::NoSuchParam`, which says the
/// parameter does not exist — not what is wrong, and not what the next
/// attempt needs.
///
/// **Paired with the binding that must be accepted**, because a reader that
/// refused every binding would pass a test made only of refusals: one
/// component is an ordinary key and binds like any scalar.
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

/// **A binding the engine cannot honour is reported and skipped**, and the
/// load still succeeds. One unusable binding is not a reason to refuse the
/// material, and the note names the parameter that will not move.
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

/// The two diagnostics the decoder owes, asserted
/// against the decoder rather than against the flag that used to hold them.
#[test]
fn the_decoder_carries_the_diagnostics_the_flag_used_to_hold_alone() {
    let bpm = Record::Bind {
        layer: Layer::L1,
        index: None,
        key: "radius".to_string(),
        signal: "bpm".to_string(),
        curve: "lin".to_string(),
        range: [0.0, 1.0],
        noise: None,
    };
    let err = binding_from_record(&bpm).expect_err("a tempo is not a [0,1] signal");
    assert!(err.contains("bar"), "{err}");

    let octaves = Record::Bind {
        layer: Layer::L1,
        index: None,
        key: "radius".to_string(),
        signal: NOISE_SIGNAL.to_string(),
        curve: "lin".to_string(),
        range: [0.0, 1.0],
        noise: Some(BindNoise {
            kind: "white".to_string(),
            octaves: 6,
            ..BindNoise::default()
        }),
    };
    let err = binding_from_record(&octaves).expect_err("white has no octaves");
    assert!(err.contains("fbm"), "{err}");
}

/// **Two `slot L1` lines are two geometries, and one address is one node.**
///
/// The index used to be destructured and thrown away on this arm: every
/// `slot L1` landed in the same entry, so a file describing two sources
/// loaded as one. Index 1 is now the second geometry it always described,
/// and what is left to report is the collision — two lines claiming index
/// 0, which the projection folds to one whatever this loader does.
#[test]
fn a_second_geometry_is_carried_and_two_slots_at_one_address_are_reported() {
    let (_dir, store, l1, l4) = fixture();
    let put = |bytes: &[u8]| store.put_artifact(bytes).expect("put");
    let first = put(&std::fs::read(&l1).expect("read"));
    // A second geometry, distinguishable from the first by name so that
    // "the later one is used" is checked rather than asserted.
    let second = put(L1.replace("proc ring", "proc ring_two").as_bytes());
    let renderer = put(&std::fs::read(&l4).expect("read"));
    let lines = parsed(&format!(
        r#"{{"t":"set","id":"two","v":1}}
{{"t":"slot","layer":"L1","proc":"{first}"}}
{{"t":"slot","layer":"L1","proc":"{second}"}}
{{"t":"slot","layer":"L1","index":1,"proc":"{second}"}}
{{"t":"slot","layer":"L4","proc":"{renderer}"}}
"#
    ));

    let loaded = from_lines(&store, "two", &lines).expect("load");
    let notes = loaded.notes.join("\n");
    assert!(
        notes.contains("two L1 slots both claim index 0"),
        "a second geometry replaced the first in silence; notes were {notes:?}"
    );
    // Both geometries, in index order — the file named two sources and two
    // is what a Set holds.
    assert_eq!(
        loaded
            .l1s
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        ["ring_two", "ring_two"],
        "a geometry at index 1 was dropped; notes were {notes:?}"
    );
    // The later of the colliding pair is what was built, which is what the
    // note promises and the only reading under which the note is true.
    assert_eq!(loaded.l1s[0].name, "ring_two");
}

/// **A name the file recorded comes back, where it used to be reported and
/// dropped.**
///
/// "Nothing this build points at a node by name" was true until an `edge`
/// did: an edge names the node that declares a slot and the node bound to
/// it, so a load that dropped the names is a load whose edges resolve
/// against the wrong spellings — or, for a name nobody wrote, against the
/// procedure's own and by luck.
#[test]
fn a_name_the_file_recorded_comes_back_with_the_node() {
    let (_dir, store, l1, l4) = fixture();
    let put = |path: &std::path::Path| {
        store
            .put_artifact(&std::fs::read(path).expect("read"))
            .expect("put")
    };
    let (geometry, renderer) = (put(&l1), put(&l4));
    let lines = parsed(&format!(
        r#"{{"t":"set","id":"named","v":1}}
{{"t":"slot","layer":"L1","name":"veil","proc":"{geometry}"}}
{{"t":"slot","layer":"L4","proc":"{renderer}"}}
"#
    ));

    let loaded = from_lines(&store, "named", &lines).expect("a named slot still loads");
    assert_eq!(
        loaded.l1s[0].name, "ring",
        "the material is unaffected by the name"
    );
    assert_eq!(
        loaded.names.l1s,
        [Some("veil".to_string())],
        "the name the file recorded is what the node is called"
    );
    // **And an unnamed node stays unnamed** rather than being filled in
    // here: a name nobody wrote is derived where the Set is built, and
    // deriving it here as well would be the second place one fact lives.
    assert_eq!(loaded.names.l4s, [None]);
    let notes = loaded.notes.join("\n");
    assert!(
        !notes.contains("veil"),
        "a name that was honoured is not a note; notes were {notes:?}"
    );
}

/// **The whole chain, through the file and back.**
///
/// A Set file recorded an L1 and its renderers: [`save`] refused an L2, an
/// L3 or a `kind Field` outright, so a cube morphing into a sphere was a
/// Set that could be played and not kept — and `--record-session` refused
/// it with the same message, because a session opens with a Set file. Every
/// layer is asserted separately, because writing them all as `L4` slots is
/// exactly what used to happen and a count would not have noticed.
#[test]
fn the_whole_chain_survives_the_file_it_is_written_to() {
    let (dir, store, l1, l4) = fixture();
    let l1b = beside(&dir, "l1b.kir", &L1.replace("proc ring", "proc ring_two"));
    let l2 = beside(&dir, "l2.kir", L2);
    let l3 = beside(&dir, "l3.kir", L3);
    let field = beside(&dir, "field.kir", FIELD);
    let l4b = beside(&dir, "l4b.kir", &L4.replace("proc points", "proc streaks"));

    let nodes = vec![
        Node {
            hash: stored(&store, &l1),
            layer: Kind::L1,
            index: 0,
            // A name the operator wrote, which is what `--set veil=l1.kir`
            // spells and what nothing here could record before.
            name: Some("veil".to_string()),
        },
        Node {
            hash: stored(&store, &l1b),
            layer: Kind::L1,
            index: 1,
            name: None,
        },
        Node {
            hash: stored(&store, &l2),
            layer: Kind::L2,
            index: 0,
            name: None,
        },
        Node {
            hash: stored(&store, &l3),
            layer: Kind::L3,
            index: 0,
            name: None,
        },
        Node {
            hash: stored(&store, &field),
            layer: Kind::Field,
            index: 0,
            name: None,
        },
        Node {
            hash: stored(&store, &l4),
            layer: Kind::L4,
            index: 0,
            name: None,
        },
        Node {
            hash: stored(&store, &l4b),
            layer: Kind::L4,
            index: 1,
            name: None,
        },
    ];
    save(
        &store,
        Asked::Operator,
        "chain",
        Saving {
            nodes: &nodes,
            capacities: &[4096, 8192],
            params: &[],
            bindings: &[],
            edges: &[],
            camera: &DEFAULT_CAMERA,
            layering: Layering::Overdraw,
            live: None,
            seeds: &[1, 2],
        },
    )
    .expect("save");
    assert!(
        written(&store, "chain").contains(r#""name":"veil""#),
        "a name the operator wrote went nowhere: {}",
        written(&store, "chain")
    );

    let loaded = load(&store, "chain").expect("load");
    let named = |procs: &[Checked]| procs.iter().map(|c| c.name.clone()).collect::<Vec<_>>();
    assert_eq!(named(&loaded.l1s), ["ring", "ring_two"]);
    assert_eq!(named(&loaded.l2s), ["warp"]);
    assert_eq!(named(&loaded.l3s), ["look"]);
    assert_eq!(named(&loaded.fields), ["blob"]);
    assert_eq!(named(&loaded.l4s), ["points", "streaks"]);
    // Node order, which is what the scratch places them in — see
    // [`Loaded::nodes`].
    assert_eq!(
        loaded
            .nodes()
            .map(|(checked, _)| checked.name.clone())
            .collect::<Vec<_>>(),
        ["ring", "ring_two", "warp", "look", "points", "streaks", "blob"]
    );
    assert!(
        loaded.srcs[2].contains("proc warp"),
        "a node was paired with another node's source: {}",
        loaded.srcs[2]
    );
    // And the name it was written with, placed on the node it belongs to
    // rather than on whichever node the walk happened to reach.
    assert_eq!(loaded.names.l1s, [Some("veil".to_string()), None]);
    assert_eq!(loaded.names.l2s, [None]);
}

/// **Two fields, each at its own index, through the file and back.**
///
/// The format could always say it — a `slot` record carries a layer and an
/// index, and Field is a layer like any other — and the loader would not:
/// it took `field_srcs.first()` and filed the rest under a note. So a Set
/// whose marcher took a shape and a cutter saved as a Set that came back
/// with one of them, and the edge naming the missing one no longer
/// resolved.
///
/// **Index as well as count**, because a pair that came back in the other
/// order is a Set whose `--param Field:1:…` moves the wrong shape, and a
/// count would not have noticed.
#[test]
fn two_fields_come_back_at_their_own_indices() {
    let (dir, store, l1, l4) = fixture();
    let shape = beside(&dir, "shape.kir", FIELD);
    let cutter = beside(&dir, "cutter.kir", &FIELD.replace("proc blob", "proc bite"));
    let nodes = vec![
        Node {
            hash: stored(&store, &l1),
            layer: Kind::L1,
            index: 0,
            name: None,
        },
        Node {
            hash: stored(&store, &l4),
            layer: Kind::L4,
            index: 0,
            name: None,
        },
        Node {
            hash: stored(&store, &shape),
            layer: Kind::Field,
            index: 0,
            name: None,
        },
        // Named, because an edge points with names and a Set holding two
        // fields is the first one that has to tell them apart.
        Node {
            hash: stored(&store, &cutter),
            layer: Kind::Field,
            index: 1,
            name: Some("knife".to_string()),
        },
    ];
    save(
        &store,
        Asked::Operator,
        "two_fields",
        Saving {
            nodes: &nodes,
            capacities: &[4096],
            params: &[],
            bindings: &[],
            edges: &[karakuri_engine::set::Edge {
                node: "points".to_string(),
                slot: "cutter".into(),
                to: "knife".to_string(),
            }],
            camera: &DEFAULT_CAMERA,
            layering: Layering::Overdraw,
            live: None,
            seeds: &[1],
        },
    )
    .expect("save");

    let loaded = load(&store, "two_fields").expect("load");
    let named = |procs: &[Checked]| procs.iter().map(|c| c.name.clone()).collect::<Vec<_>>();
    assert_eq!(named(&loaded.fields), ["blob", "bite"]);
    assert!(
        loaded.notes.is_empty(),
        "nothing was skipped: {:?}",
        loaded.notes
    );
    // The names, on the nodes they belong to — `None` for the one written
    // bare, which is what an edge pointing at `knife` needs to resolve.
    assert_eq!(loaded.names.fields, [None, Some("knife".to_string())]);
    // Node order, which is what the scratch places sources in: the fields
    // are last and they are in index order.
    assert_eq!(
        loaded
            .nodes()
            .map(|(checked, _)| checked.name.clone())
            .collect::<Vec<_>>(),
        ["ring", "points", "blob", "bite"]
    );
    assert_eq!(
        loaded.node_names().collect::<Vec<_>>(),
        [None, None, None, Some("knife".to_string())]
    );
    assert_eq!(loaded.edges.len(), 1, "the edge that binds the second");
}

/// **Each geometry runs at the number written against it.**
///
/// The capacity was keyed by node in the format and by Set in this loader:
/// a `capacity` on L1 index 1 was reported and dropped, so a Set whose two
/// sources were sized differently came back with the second at whatever its
/// `.kir` declared. The engine takes one per source and now so does this.
#[test]
fn each_geometry_keeps_the_capacity_it_was_saved_with() {
    let (dir, store, l1, l4) = fixture();
    let l1b = beside(&dir, "l1b.kir", &L1.replace("proc ring", "proc ring_two"));
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
            name: None,
        },
        Node {
            hash: stored(&store, &l4),
            layer: Kind::L4,
            index: 0,
            name: None,
        },
    ];
    save(
        &store,
        Asked::Operator,
        "two",
        Saving {
            nodes: &nodes,
            capacities: &[4096, 65_536],
            params: &[],
            bindings: &[],
            edges: &[],
            camera: &DEFAULT_CAMERA,
            layering: Layering::Overdraw,
            live: None,
            seeds: &[1, 2],
        },
    )
    .expect("save");

    let loaded = load(&store, "two").expect("load");
    assert_eq!(loaded.capacities, vec![Some(4096), Some(65_536)]);
    assert!(loaded.notes.is_empty(), "{:?}", loaded.notes);
}

/// **A salt is recorded per geometry and comes back per geometry**, which
/// is what makes a saved Set reproduce its colours whatever order its
/// records are in — `docs/ir-spec.md`, "A `source` value is assigned and
/// recorded, never derived". This wrote one `seed` for the whole Set and
/// read node 0's, so the second geometry's randomness was a function of
/// where its path sat on the command line and of nothing in the file.
///
/// **The bytes are asserted, not just the round trip.** Index 0 is absent
/// and index 1 is written, which is the whole of what keeps the file a Set
/// of one geometry has always written unchanged.
#[test]
fn each_geometry_keeps_the_salt_it_was_saved_with() {
    let (dir, store, l1, l4) = fixture();
    let l1b = beside(&dir, "l1b.kir", &L1.replace("proc ring", "proc ring_two"));
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
            name: None,
        },
        Node {
            hash: stored(&store, &l4),
            layer: Kind::L4,
            index: 0,
            name: None,
        },
    ];
    save(
        &store,
        Asked::Operator,
        "two",
        Saving {
            nodes: &nodes,
            capacities: &[4096, 4096],
            params: &[],
            bindings: &[],
            edges: &[],
            camera: &DEFAULT_CAMERA,
            layering: Layering::Overdraw,
            live: None,
            seeds: &[7, 9],
        },
    )
    .expect("save");

    let text = written(&store, "two");
    assert!(
        text.contains("{\"t\":\"seed\",\"stream\":\"L1\",\"value\":7}\n")
            && text.contains("{\"t\":\"seed\",\"stream\":\"L1\",\"index\":1,\"value\":9}\n"),
        "{text}"
    );

    let loaded = load(&store, "two").expect("load");
    assert_eq!(loaded.salts, vec![Some(7), Some(9)]);
    assert!(loaded.notes.is_empty(), "{:?}", loaded.notes);
}

/// **A Set file written when a seed salted the whole Set still loads**, and
/// says so by carrying one salt for the geometry it was written against.
///
/// That is the older file's shape: one `seed` record, no index on it, and
/// however many geometries. The geometry it names keeps the colours it was
/// saved with; the ones it does not are salted the way an unsaved run is,
/// which is what `None` in [`Loaded::salts`] asks the engine for. Refusing
/// or defaulting either half would be a file that loads and draws something
/// nobody saved.
#[test]
fn a_file_that_salted_the_whole_set_still_loads() {
    let (dir, store, l1, l4) = fixture();
    let l1b = beside(&dir, "l1b.kir", &L1.replace("proc ring", "proc ring_two"));
    let put = |path: &std::path::Path| {
        store
            .put_artifact(&std::fs::read(path).expect("read"))
            .expect("put")
    };
    let lines = parsed(&format!(
        r#"{{"t":"set","id":"old","v":1}}
{{"t":"slot","layer":"L1","proc":"{}"}}
{{"t":"slot","layer":"L1","index":1,"proc":"{}"}}
{{"t":"slot","layer":"L4","proc":"{}"}}
{{"t":"seed","stream":"L1","value":4242}}
"#,
        put(&l1),
        put(&l1b),
        put(&l4)
    ));

    let loaded = from_lines(&store, "old", &lines).expect("load");
    assert_eq!(loaded.l1s.len(), 2);
    assert_eq!(
        loaded.salts,
        vec![Some(4242)],
        "the one seed the file carries salts the geometry it names, and the other \
             geometry is left to be derived"
    );
    assert!(loaded.notes.is_empty(), "{:?}", loaded.notes);
}

/// **A one-geometry Set is byte for byte the file it has always been**,
/// except for the one field the format has grown since.
///
/// Every Set file ever written is one L1 and its renderers, and the fields
/// that carry a chain — `index` on every layer, `name` on a slot — are
/// absent rather than defaulted for exactly this reason. A literal, not a
/// re-save compared against itself: a round trip through one writer agrees
/// with itself however far both halves have drifted.
///
/// **`height` on the `camera` line is the exception, and it is a decision
/// rather than drift** (ADR-0318, 2026-09-09): the record carried two of
/// the orbit's three placement numbers, so a camera saved looking down came
/// back looking along the equator. This literal grew the field; a file
/// written without it still reads as the 2.0 it meant. What this test is
/// for is that nothing grows one *silently*, and it did its job.
#[test]
fn a_one_geometry_set_is_byte_for_byte_the_file_it_always_was() {
    let (_dir, store, l1, l4) = fixture();
    save(
        &store,
        Asked::Operator,
        "s1",
        plain(&ordinary(&store, &l1, std::slice::from_ref(&l4)), &[]),
    )
    .expect("save");
    let hash = |path: &std::path::Path| {
        store
            .put_artifact(&std::fs::read(path).expect("read"))
            .expect("put")
    };
    assert_eq!(
        written(&store, "s1"),
        format!(
            r#"{{"t":"set","id":"s1","v":1}}
{{"t":"slot","layer":"L1","proc":"{}"}}
{{"t":"slot","layer":"L4","proc":"{}"}}
{{"t":"capacity","layer":"L1","value":4096}}
{{"t":"camera","kind":"orbit","radius":8.0,"speed":0.15,"height":2.0}}
{{"t":"seed","stream":"L1","value":1}}
"#,
            hash(&l1),
            hash(&l4)
        )
    );
}

/// **A composited Set is saved as one and comes back as one**, folded to
/// the renderer it was folded to.
///
/// This is the round trip the `merge` record exists for. A Set file could
/// not say that a slot composites, so a variant pool written out came back
/// overdrawing: no L5, no edges into one, and `Set::select_renderer` with
/// nothing to select between. Both halves are asserted here because either
/// one alone is useless — a layering that survived without its selection
/// comes up folding every alternative at once, which is a different picture
/// from the one that was saved.
#[test]
fn a_composited_set_comes_back_composited_and_still_folded_where_it_was() {
    let (dir, store, l1, l4) = fixture();
    let l4b = beside(
        &dir,
        "l4b.kir",
        &L4.replace("proc points", "proc points_two"),
    );
    let nodes = ordinary(&store, &l1, &[l4, l4b]);
    save(
        &store,
        Asked::Operator,
        "pool",
        Saving {
            layering: Layering::Composite,
            live: Some(1),
            ..plain(&nodes, &[])
        },
    )
    .expect("save");

    // The line itself, because the record's *presence* is the statement
    // and a test that only read the decoder back could pass on a writer
    // that wrote nothing and a reader that assumed everything.
    assert!(
        written(&store, "pool").contains(r#"{"t":"merge","live":1}"#),
        "the file does not carry the merge the Set was saved with:\n{}",
        written(&store, "pool")
    );

    let loaded = load(&store, "pool").expect("load");
    assert_eq!(
        loaded.layering,
        Layering::Composite,
        "a composited Set loaded back overdrawing"
    );
    assert_eq!(
        loaded.live,
        Some(1),
        "the fold came back with a different renderer live"
    );
    assert!(
        loaded.notes.is_empty(),
        "unexpected notes: {:?}",
        loaded.notes
    );
}

/// **A Set that overdraws writes no `merge` line at all**, and loads back
/// overdrawing.
///
/// The record's absence is how overdraw has always been spelled — there is
/// no boolean field, because a record that could say `false` would be a
/// second spelling of not writing one. So this asserts the *bytes*: a Set
/// that overdraws is byte for byte the file it was before the record
/// existed, and every file written by an older build reads as what it was.
#[test]
fn an_overdrawing_set_writes_no_merge_line_and_loads_back_overdrawing() {
    let (dir, store, l1, l4) = fixture();
    let l4b = beside(
        &dir,
        "l4b.kir",
        &L4.replace("proc points", "proc points_two"),
    );
    let nodes = ordinary(&store, &l1, &[l4, l4b]);
    save(&store, Asked::Operator, "stack", plain(&nodes, &[])).expect("save");

    let text = written(&store, "stack");
    assert!(
        !text.contains("merge"),
        "an overdrawing Set wrote a merge record:\n{text}"
    );

    let loaded = load(&store, "stack").expect("load");
    assert_eq!(
        loaded.layering,
        Layering::Overdraw,
        "a Set that recorded no merge loaded back compositing"
    );
    assert_eq!(loaded.live, None, "a Set with no merge recorded a fold");
}

/// **A save a model asked for lands in the sandbox and never in the
/// operator's library.**
///
/// This is
/// `docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md`
/// at the one line that decides it — [`save`]'s match on [`Asked`] — and
/// the property it holds is a *negative* one: the file that must not be
/// there. Delete the `Asked::Model` arm and the sandbox assertion still
/// passes on nothing, so the assertion that matters is the second: `sets/`
/// is where the operator's presets are, and a model writing an id one of
/// them already has is the loss P-0096 exists against.
///
/// **The control is the same call with the other actor**, which is what
/// stops this passing against a `save` that had stopped writing anywhere
/// the library can see.
#[test]
fn a_save_a_model_asked_for_lands_in_the_sandbox_and_never_in_the_library() {
    let (dir, store, l1, l4) = fixture();
    let nodes = ordinary(&store, &l1, &[l4]);
    let root = dir.path().join("store");

    save(&store, Asked::Model, "night01", plain(&nodes, &[])).expect("save");

    assert!(
        root.join("sandbox").join("night01.kbset").exists(),
        "a save asked for over MCP did not reach the sandbox"
    );
    assert!(
        !root.join("sets").join("night01.kbset").exists(),
        "a save asked for over MCP wrote the operator's library"
    );
    assert!(
        load(&store, "night01").is_err(),
        "the library loaded a Set only the sandbox holds"
    );

    // The control. Without it this would pass against a `save` that wrote
    // no library file for anybody.
    save(&store, Asked::Operator, "night01", plain(&nodes, &[])).expect("save");
    assert!(
        root.join("sets").join("night01.kbset").exists(),
        "the operator's own save did not reach the library"
    );
    load(&store, "night01").expect("and the library loads it back");
}

/// **Two saves a model asked for under one name are two files.**
///
/// `filed_as` is where this is decided and the reason is written there: the
/// sandbox holds snapshots, and a snapshot a later snapshot can replace is
/// not one. Asserted here rather than beside that function because the
/// property is about what is on the disk afterwards — a rule about an id
/// that never reached a store would be a rule about a string.
///
/// **The control is the operator's own name, which overwrites**, and that
/// is ADR-0128 unchanged: an id an operator types is an instruction.
#[test]
fn two_saves_a_model_asked_for_under_one_name_are_two_files() {
    let (dir, store, l1, l4) = fixture();
    let nodes = ordinary(&store, &l1, &[l4]);
    let root = dir.path().join("store");

    let first = crate::accepted_save(
        0,
        Asked::Model,
        Some("take".into()),
        &Sources(vec![]),
        &root,
        None,
    );
    let second = crate::accepted_save(
        0,
        Asked::Model,
        Some("take".into()),
        &Sources(vec![]),
        &root,
        None,
    );
    assert_ne!(
        first, second,
        "a model's second save was handed the first one's id, so it would \
             have written over a snapshot it had been told was kept"
    );
    save(&store, Asked::Model, &first, plain(&nodes, &[])).expect("save");
    save(&store, Asked::Model, &second, plain(&nodes, &[])).expect("save");
    let kept = std::fs::read_dir(root.join("sandbox"))
        .expect("the sandbox")
        .count();
    assert_eq!(kept, 2, "two snapshots did not leave two files");

    // The control: an operator's own name is an instruction and is reused.
    assert_eq!(
        crate::accepted_save(
            0,
            Asked::Operator,
            Some("take".into()),
            &Sources(vec![]),
            &root,
            None
        ),
        "take",
        "an operator's own id was not the id it asked for"
    );
}

/// **A composited Set nobody selected in writes no `live`**, and comes back
/// with every renderer live.
///
/// Absent is *not* renderer 0. Read that way it would silence every
/// renderer but the first in every composited Set ever saved without a
/// selection — a picture nobody asked for, from a file that said nothing
/// had changed.
#[test]
fn a_composited_set_nobody_selected_in_writes_no_live() {
    let (dir, store, l1, l4) = fixture();
    let l4b = beside(
        &dir,
        "l4b.kir",
        &L4.replace("proc points", "proc points_two"),
    );
    let nodes = ordinary(&store, &l1, &[l4, l4b]);
    save(
        &store,
        Asked::Operator,
        "unselected",
        Saving {
            layering: Layering::Composite,
            live: None,
            ..plain(&nodes, &[])
        },
    )
    .expect("save");

    let text = written(&store, "unselected");
    assert!(
        text.contains(r#"{"t":"merge"}"#),
        "the bare merge record is not in the file:\n{text}"
    );

    let loaded = load(&store, "unselected").expect("load");
    assert_eq!(loaded.layering, Layering::Composite);
    assert_eq!(
        loaded.live, None,
        "a merge with no `live` came back naming a renderer"
    );
}

/// **A fold naming a renderer the file does not have is said and dropped.**
///
/// A hand-written or hand-edited file can name one; the Set is whole either
/// way, so the honest answer is to load it with every renderer live — the
/// state it would have come up in — and say which line was not honoured.
/// Checked where the `camera` index is checked and for its reason: how many
/// renderers a file names is only known once every `slot` record has been
/// met.
#[test]
fn a_fold_naming_a_renderer_that_is_not_there_is_reported_and_dropped() {
    let (_dir, store, l1, l4) = fixture();
    let nodes = ordinary(&store, &l1, std::slice::from_ref(&l4));
    save(
        &store,
        Asked::Operator,
        "wrong",
        Saving {
            layering: Layering::Composite,
            live: Some(3),
            ..plain(&nodes, &[])
        },
    )
    .expect("save");

    let loaded = load(&store, "wrong").expect("load");
    assert_eq!(loaded.layering, Layering::Composite, "the Set still folds");
    assert_eq!(loaded.live, None, "a fold nobody can honour was applied");
    assert!(
        loaded.notes.iter().any(|n| n.contains("renderer 3")),
        "nothing said which renderer was not there: {:?}",
        loaded.notes
    );
}

/// **Refused rather than written into a file that cannot be read back.**
///
/// What [`save`] refuses is no longer a layer — it has a slot for every one
/// — but the two shapes that are not a Set, and the two a projection keyed
/// by address cannot fold: a repeat, and a gap.
#[test]
fn what_the_file_cannot_hold_is_refused_rather_than_written() {
    let (_dir, store, l1, l4) = fixture();
    // **Stored once and addressed many times**, which is exactly what a
    // content address buys: the two fixtures go in here, and every case
    // below is a different arrangement of the same two hashes. This used to
    // leak both paths to `'static` so that a borrowing `Node` could outlive
    // them; a `Hash` is `Copy` and there is nothing left to outlive.
    let (l1, l4) = (stored(&store, &l1), stored(&store, &l4));
    let node = |hash: Hash, layer, index| Node {
        hash,
        layer,
        index,
        name: None,
    };

    let cases: Vec<(Vec<Node>, &[u32], &str)> = vec![
        (
            vec![node(l4, Kind::L4, 0)],
            &[],
            "none of these files declares `kind L1`",
        ),
        (
            vec![node(l1, Kind::L1, 0)],
            &[4096],
            "none of these files declares `kind L4`",
        ),
        (
            vec![
                node(l1, Kind::L1, 0),
                node(l4, Kind::L4, 0),
                node(l4, Kind::L4, 0),
            ],
            &[4096],
            "second node at that address",
        ),
        (
            vec![
                node(l1, Kind::L1, 0),
                node(l4, Kind::L4, 0),
                node(l4, Kind::L4, 2),
            ],
            &[4096],
            "far side of a gap",
        ),
        (
            vec![node(l1, Kind::L1, 0), node(l4, Kind::L4, 0)],
            &[4096, 4096],
            "1 geometry and 2 capacities",
        ),
    ];
    for (nodes, capacities, expected) in cases {
        let err = save(
            &store,
            Asked::Operator,
            "bad",
            Saving {
                nodes: &nodes,
                capacities,
                params: &[],
                bindings: &[],
                edges: &[],
                camera: &DEFAULT_CAMERA,
                layering: Layering::Overdraw,
                live: None,
                seeds: &[1],
            },
        )
        .expect_err("a file nothing could read back was written");
        assert!(err.contains(expected), "{err}");
    }
}

/// **A gap in a layer is refused on the way in too**, and on every layer:
/// index 2 with no index 1 says a chain with a hole in it, and closing it up
/// would silently change what deforms what — or, on L4, draw order.
/// **The built-in camera's three are written once, and the `camera` record
/// is where.**
///
/// They are that node's parameters since ADR-0318, so `Set::params` reports
/// them and a writer that took the list whole would put `radius` in the
/// file twice — once as a `param` at `L3:0` and once on the `camera` line.
/// Two spellings of one fact leave a reader asking which a writer meant by
/// choosing the other, so the `param` run leaves that node to the record
/// that describes it, exactly as the `slot` run already does.
#[test]
fn the_built_in_cameras_three_are_written_as_the_camera_record_and_not_as_params() {
    let (_dir, store, l1, l4) = fixture();
    // A Set with no camera procedure, so the built-in is `L3:0`.
    let params = vec![
        ParamWrite::at(Kind::L3, 0, "radius", 12.0),
        ParamWrite::at(Kind::L3, 0, "height", -4.0),
        ParamWrite::everywhere("radius", 3.25),
    ];
    let camera = Orbit {
        radius: 12.0,
        height: -4.0,
        ..Orbit::default()
    };
    save(
        &store,
        Asked::Operator,
        "s1",
        Saving {
            nodes: &ordinary(&store, &l1, std::slice::from_ref(&l4)),
            capacities: &[4096],
            params: &params,
            bindings: &[],
            edges: &[],
            camera: &camera,
            layering: Layering::Overdraw,
            live: None,
            seeds: &[1],
        },
    )
    .expect("save");

    let text = written(&store, "s1");
    assert!(
        !text.contains(r#""layer":"L3""#),
        "the built-in camera's parameters were written as `param` records as well as on \
             the `camera` line:\n{text}"
    );
    assert!(
        text.contains(r#"{"t":"camera","kind":"orbit","radius":12.0,"speed":0.15,"height":-4.0}"#),
        "the `camera` record does not carry the three:\n{text}"
    );
    // **And the bare write is untouched**, which is the half that says this
    // is about one node rather than about the layer or the key: `radius`
    // written everywhere is the geometry's and still a `param` line.
    assert!(
        text.contains(r#"{"t":"param","layer":"L1","key":"radius","value":3.25}"#),
        "a bare write was dropped with the camera's:\n{text}"
    );
}

/// **A file written before `height` reads as the default it meant**, and
/// that default is the engine's own.
///
/// `karakuri-store` restates `Orbit::default().height` because it depends on
/// nothing and cannot ask for it; this is the test that holds the two
/// together, here because this crate is where a Set file meets an `Orbit`.
#[test]
fn a_camera_line_without_a_height_reads_as_the_engines_default() {
    let (_dir, store, l1, l4) = fixture();
    let put = |path: &std::path::Path| {
        store
            .put_artifact(&std::fs::read(path).expect("read"))
            .expect("put")
    };
    let lines = parsed(&format!(
        r#"{{"t":"set","id":"old","v":1}}
{{"t":"slot","layer":"L1","proc":"{}"}}
{{"t":"slot","layer":"L4","proc":"{}"}}
{{"t":"camera","kind":"orbit","radius":8.0,"speed":0.15}}
"#,
        put(&l1),
        put(&l4)
    ));

    let loaded = from_lines(&store, "old", &lines).expect("load");
    assert_eq!(
        loaded.camera.map(|c| c.height),
        Some(Orbit::default().height),
        "a file with no `height` has to read as what it always meant, which is the \
             engine's default and not a number the record crate picked separately"
    );
    assert!(loaded.notes.is_empty(), "{:?}", loaded.notes);
}

#[test]
fn a_gap_in_a_chain_is_refused_rather_than_closed_up() {
    let (dir, store, l1, l4) = fixture();
    let l2 = beside(&dir, "l2.kir", L2);
    let put = |path: &std::path::Path| {
        store
            .put_artifact(&std::fs::read(path).expect("read"))
            .expect("put")
    };
    let lines = parsed(&format!(
        r#"{{"t":"set","id":"holed","v":1}}
{{"t":"slot","layer":"L1","proc":"{}"}}
{{"t":"slot","layer":"L2","index":1,"proc":"{}"}}
{{"t":"slot","layer":"L4","proc":"{}"}}
"#,
        put(&l1),
        put(&l2),
        put(&l4)
    ));

    let err = from_lines(&store, "holed", &lines).expect_err("index 1 with no index 0");
    assert!(
        err.contains("names a L2 at index 0 but none before it"),
        "{err}"
    );
}

/// **A second camera is a second camera.** The format could address one
/// all along and this loader used to read the first and report the rest as
/// skipped — a refusal about the plumbing, which took a `Vec` here and in
/// the engine to lift. Which renderer draws from which is an `edge`, so
/// there is nothing here to arbitrate.
#[test]
fn a_second_camera_loads_beside_the_first() {
    let (dir, store, l1, l4) = fixture();
    let l3 = beside(&dir, "l3.kir", L3);
    let l3b = beside(&dir, "l3b.kir", &L3.replace("proc look", "proc look_two"));
    let put = |path: &std::path::Path| {
        store
            .put_artifact(&std::fs::read(path).expect("read"))
            .expect("put")
    };
    let lines = parsed(&format!(
        r#"{{"t":"set","id":"two_eyes","v":1}}
{{"t":"slot","layer":"L1","proc":"{}"}}
{{"t":"slot","layer":"L3","proc":"{}"}}
{{"t":"slot","layer":"L3","index":1,"proc":"{}"}}
{{"t":"slot","layer":"L4","proc":"{}"}}
"#,
        put(&l1),
        put(&l3),
        put(&l3b),
        put(&l4)
    ));

    let loaded = from_lines(&store, "two_eyes", &lines).expect("two cameras load");
    assert_eq!(
        loaded
            .l3s
            .iter()
            .map(|c| c.name.clone())
            .collect::<Vec<_>>(),
        ["look", "look_two"],
        "the second camera was dropped"
    );
    assert!(
        loaded.notes.is_empty(),
        "two cameras are ordinary material now; notes were {:?}",
        loaded.notes
    );
}

/// **A `camera` record with no index is node 0's**, which is what every
/// file ever written means by it: a Set held one camera, so there was one
/// node for the record to describe, and a file that names no camera
/// procedure still has the built-in at `L3:0`.
///
/// The index exists because the L3 layer holds several now. The built-in
/// orbit is the node after the procedures, and the only one a `camera`
/// record can be about — a camera that is a procedure writes its own six
/// numbers every frame — so an index naming one of those is said rather
/// than applied to it.
#[test]
fn a_camera_record_with_no_index_is_node_zeros() {
    let (_dir, store, l1, l4) = fixture();
    let put = |path: &std::path::Path| {
        store
            .put_artifact(&std::fs::read(path).expect("read"))
            .expect("put")
    };
    let file = |camera: &str| {
        parsed(&format!(
            r#"{{"t":"set","id":"seen","v":1}}
{{"t":"slot","layer":"L1","proc":"{}"}}
{{"t":"slot","layer":"L4","proc":"{}"}}
{camera}
"#,
            put(&l1),
            put(&l4)
        ))
    };

    // As a file written before the index existed spells it.
    let loaded = from_lines(
        &store,
        "seen",
        &file(r#"{"t":"camera","kind":"orbit","radius":7.5,"speed":0.5}"#),
    )
    .expect("load");
    assert_eq!(
        loaded.camera.map(|c| (c.radius, c.speed)),
        Some((7.5, 0.5)),
        "a `camera` record with no index has to reach the built-in camera"
    );
    assert!(loaded.notes.is_empty(), "{:?}", loaded.notes);

    // Written out, the index is absent again: 0 is not serialised, so a
    // file this build saves is the line every earlier build wrote.
    save(
        &store,
        Asked::Operator,
        "written_back",
        Saving {
            nodes: &ordinary(&store, &l1, std::slice::from_ref(&l4)),
            capacities: &[65_536],
            params: &[],
            bindings: &[],
            edges: &[],
            camera: &DEFAULT_CAMERA,
            layering: Layering::Overdraw,
            live: None,
            seeds: &[1],
        },
    )
    .expect("save");
    let text = written(&store, "written_back");
    let line = text
        .lines()
        .find(|l| l.contains(r#""t":"camera""#))
        .expect("a camera record is written");
    assert!(!line.contains("index"), "index 0 is not written: {line}");

    // An index that is not the built-in's names a camera that is a
    // procedure, which produces its own state — said rather than applied.
    let loaded = from_lines(
        &store,
        "seen",
        &file(r#"{"t":"camera","kind":"orbit","index":1,"radius":7.5,"speed":0.5}"#),
    )
    .expect("load");
    assert!(
        loaded.camera.is_none(),
        "an orbit was applied to a camera node that produces its own state"
    );
    assert!(
        loaded.notes.iter().any(|n| n.contains("camera at L3:1")),
        "and it has to be said: {:?}",
        loaded.notes
    );
}

/// A `bind` record and a `Binding` are the same thing in two shapes, and
/// `save` writes one from the other. **Every generator kind survives**, so
/// a saved `fbm` does not come back as the perlin the default would give.
#[test]
fn every_noise_generator_survives_the_record_it_is_written_as() {
    for kind in [
        NoiseKind::White,
        NoiseKind::Value,
        NoiseKind::Perlin,
        NoiseKind::Fbm { octaves: 6 },
    ] {
        let binding = Binding::new(Kind::L1, "radius", NOISE_SIGNAL, Curve::Lin, [0.0, 1.0])
            .with_noise(NoiseConfig {
                kind,
                rate: 2.5,
                stream: 3,
            });
        let back = binding_from_record(&record_from_binding(&binding))
            .unwrap_or_else(|e| panic!("{kind:?}: {e}"));
        assert_eq!(back.noise.map(|n| n.kind), Some(kind), "{kind:?}");
        assert_eq!(back.noise.map(|n| (n.rate, n.stream)), Some((2.5, 3)));
    }
}
// -- Bundling --------------------------------------------------------

/// The text a bundle is written out as, back through the reader — so a
/// test round-trips through the *file*, which is what `--package >` writes
/// and what `--take-in` reads, rather than through records held in memory
/// that could not have survived a serialisation.
fn as_a_file(lines: &[Line]) -> Vec<Line> {
    parsed(
        &lines
            .iter()
            .map(|line| format!("{}\n", line.as_str()))
            .collect::<String>(),
    )
}

/// **The round trip both flags exist for**: a bundle written out of one
/// store loads in a store that has never held its artifacts.
///
/// `inlined_source_loads_without_a_store_that_knows_the_artifact` above
/// proves the *reader* does that, from `src` records a test hand-built.
/// This is the writing half beside it: nothing here spells a record out —
/// [`bundle`] produces the file and [`unbundle`] takes it in, and the
/// material arrives on the far side as procedures with their own names.
///
/// **And the cards come with it.** An artifact whose card is missing is an
/// ordinary store rather than a damaged one, so this is not the difference
/// between a bundle that works and one that does not — but a bundle that
/// dropped them would leave every library taken in thinner than the one it
/// came from, silently.
#[test]
fn a_bundle_loads_in_a_store_that_has_never_seen_the_artifacts() {
    let (_dir, store, l1, l4) = fixture();
    save(
        &store,
        Asked::Operator,
        "s1",
        plain(&ordinary(&store, &l1, std::slice::from_ref(&l4)), &[]),
    )
    .expect("save");
    let sent = as_a_file(&bundle(&store, "s1").expect("bundle"));

    let elsewhere = tempfile::tempdir().expect("tempdir");
    let bare = Store::open(elsewhere.path()).expect("store");
    let said = unbundle(&bare, &sent).expect("the file carries its own source");
    assert!(said.contains("`s1`"), "{said}");

    let loaded = load(&bare, "s1").expect("the set is filed and its artifacts are here");
    assert_eq!(loaded.l1s[0].name, "ring");
    assert_eq!(loaded.l4s[0].name, "points");
    for hash in [
        Hash::of(&std::fs::read(&l1).expect("read")),
        Hash::of(&std::fs::read(&l4).expect("read")),
    ] {
        bare.read_meta(&hash)
            .unwrap_or_else(|e| panic!("{}: {e}", hash.short(12)));
    }
}

/// **A bundle missing one procedure is refused whole, naming it.**
///
/// The alternative is a file that looks self-contained and is not, whose
/// failure surfaces on somebody else's machine — where the artifact it
/// wants is not, and never was.
#[test]
fn a_bundle_is_refused_when_the_store_lacks_a_source() {
    let (_dir, store, l1, _l4) = fixture();
    let here = stored(&store, &l1);
    let missing = Hash::of(b"a renderer that was never put in this store");
    let text = format!(
        r#"{{"t":"set","id":"gone","v":1}}
{{"t":"slot","layer":"L1","proc":"{here}"}}
{{"t":"slot","layer":"L4","name":"veil","proc":"{missing}"}}
"#
    );
    store.write_set("gone", &parsed(&text)).expect("write");

    let e = bundle(&store, "gone").expect_err("a bundle cannot carry what is not there");
    assert!(e.contains("veil"), "the node is not named: {e}");
    assert!(e.contains(&missing.short(12)), "{e}");
}

/// **Two nodes over one artifact inline it once.** The reader keys `src` by
/// hash, so a second run would be a second copy of the same bytes that
/// nothing ever reads — and this Set is one geometry drawn twice by the
/// same renderer, which is the ordinary way that happens.
#[test]
fn one_artifact_referenced_twice_is_inlined_once() {
    let (_dir, store, l1, l4) = fixture();
    let twice = vec![l4.clone(), l4.clone()];
    save(
        &store,
        Asked::Operator,
        "s1",
        plain(&ordinary(&store, &l1, &twice), &[]),
    )
    .expect("save");
    let bundled = bundle(&store, "s1").expect("bundle");

    let renderer = Hash::of(&std::fs::read(&l4).expect("read"));
    let slots = bundled
            .iter()
            .filter(|line| matches!(line.record(), Record::Slot { proc_hash, .. } if *proc_hash == renderer))
            .count();
    assert_eq!(slots, 2, "the fixture is meant to name the renderer twice");
    // One run, so line 0 appears once.
    let heads = bundled
        .iter()
        .filter(
            |line| matches!(line.record(), Record::Src { hash, line: 0, .. } if *hash == renderer),
        )
        .count();
    assert_eq!(heads, 1, "the renderer's source was inlined {heads} times");
    // And the run is whole: as many `src` records as the source has lines.
    let run = bundled
        .iter()
        .filter(|line| matches!(line.record(), Record::Src { hash, .. } if *hash == renderer))
        .count();
    assert_eq!(run, L4.split('\n').count());
}

/// **A source that does not hash to the address its `slot` names is
/// refused, and nothing is stored.**
///
/// This is the check that makes a bundle worth trusting at all: without it
/// a `src` run is a way to file arbitrary text under an address the
/// operator on the far side recognises, and every guarantee content
/// addressing makes is gone. Refusing *after* storing some of it would be
/// nearly as bad — the store would hold half a stranger's file.
#[test]
fn an_unbundle_refuses_a_source_that_does_not_hash_to_its_address() {
    let (_dir, store, l1, l4) = fixture();
    save(
        &store,
        Asked::Operator,
        "s1",
        plain(&ordinary(&store, &l1, std::slice::from_ref(&l4)), &[]),
    )
    .expect("save");
    let renderer = Hash::of(&std::fs::read(&l4).expect("read"));
    // One line of the renderer's inlined source rewritten, everything else
    // — the `slot` record's hash included — left exactly as written.
    let tampered: Vec<Line> = bundle(&store, "s1")
        .expect("bundle")
        .into_iter()
        .map(|line| match line.record() {
            Record::Src { hash, line: at, .. } if *hash == renderer && *at == 1 => {
                Line::new(Record::Src {
                    hash: renderer,
                    line: 1,
                    s: "proc points_but_not_really {".to_string(),
                })
            }
            _ => line,
        })
        .collect();

    let elsewhere = tempfile::tempdir().expect("tempdir");
    let bare = Store::open(elsewhere.path()).expect("store");
    let e = unbundle(&bare, &as_a_file(&tampered))
        .expect_err("text that is not the bytes its address names");
    // The node, by the address the file gives it and by what it is called
    // — which in a store with no card for it is its short hash.
    assert!(e.contains("L4:0"), "the node is not named: {e}");
    assert!(e.contains(&renderer.short(12)), "{e}");
    assert!(
        bare.list_artifacts().expect("list").is_empty(),
        "a refused bundle left an artifact behind"
    );
    assert!(
        bare.list_sets().expect("list").is_empty(),
        "a refused bundle left a Set file behind"
    );
}

/// **An id already taken is refused, and the Set that was there is left
/// exactly as it was.**
///
/// Deliberately not `--save-set`'s rule, which overwrites: an id you type
/// is an instruction, and an id that arrived inside somebody else's file is
/// not. The bytes are compared before and after, because "it refused" and
/// "it refused without having written" are two different claims.
#[test]
fn an_unbundle_refuses_an_id_already_taken_and_leaves_the_set_alone() {
    let (dir, store, l1, l4) = fixture();
    save(
        &store,
        Asked::Operator,
        "s1",
        plain(&ordinary(&store, &l1, std::slice::from_ref(&l4)), &[]),
    )
    .expect("save");
    let sent = as_a_file(&bundle(&store, "s1").expect("bundle"));

    // Somebody else's store, with a Set of their own under that word: one
    // geometry drawn by two renderers, where the bundle names one.
    let elsewhere = tempfile::tempdir().expect("tempdir");
    let theirs = Store::open(elsewhere.path()).expect("store");
    let l2 = beside(&dir, "theirs.kir", L2);
    let mine = vec![
        Node {
            hash: stored(&theirs, &l1),
            layer: Kind::L1,
            index: 0,
            name: None,
        },
        Node {
            hash: stored(&theirs, &l2),
            layer: Kind::L2,
            index: 0,
            name: Some("preset".to_string()),
        },
        Node {
            hash: stored(&theirs, &l4),
            layer: Kind::L4,
            index: 0,
            name: None,
        },
    ];
    save(&theirs, Asked::Operator, "s1", plain(&mine, &[])).expect("save");
    let before = written(&theirs, "s1");

    let e = unbundle(&theirs, &sent).expect_err("an id that arrived in a file is not typed");
    assert!(e.contains("`s1`"), "the id is not named: {e}");
    assert_eq!(before, written(&theirs, "s1"), "the preset was overwritten");
}

/// **A source this build cannot compile is stored, keeps its slot, and is
/// reported.**
///
/// Refusing the whole file would tell an operator that *something* is
/// wrong. Storing it means `--load-set` fails against the source itself,
/// with the checker's span and hint on the line that is wrong — which is a
/// thing they can fix. So the note says which node and what the checker
/// said, and the artifact is on disk to be read and edited.
#[test]
fn an_unbundle_stores_a_source_that_does_not_compile_and_says_so() {
    let broken = "proc veil {\n  kind L4\n  this is not a renderer\n}\n";
    let renderer = Hash::of(broken.as_bytes());
    let geometry = Hash::of(L1.as_bytes());
    let mut lines = vec![
        Line::new(Record::Set {
            id: "sent".to_string(),
            v: VERSION,
        }),
        Line::new(Record::Slot {
            at: NodeAddress {
                layer: Layer::L1,
                index: 0,
            },
            name: None,
            proc_hash: geometry,
        }),
        Line::new(Record::Slot {
            at: NodeAddress {
                layer: Layer::L4,
                index: 0,
            },
            name: Some("veil".to_string()),
            proc_hash: renderer,
        }),
    ];
    for (hash, src) in [(geometry, L1), (renderer, broken)] {
        for (n, text) in src.split('\n').enumerate() {
            lines.push(Line::new(Record::Src {
                hash,
                line: n as u32,
                s: text.to_string(),
            }));
        }
    }

    let elsewhere = tempfile::tempdir().expect("tempdir");
    let bare = Store::open(elsewhere.path()).expect("store");
    let said = unbundle(&bare, &as_a_file(&lines)).expect("one bad source is not a refusal");

    let report = crate::compile::check(broken).expect_err("the fixture must not compile");
    let first = report.lines().next().expect("a diagnostic").trim();
    assert!(said.contains("veil"), "the node is not named: {said}");
    assert!(
        said.contains(first),
        "the checker's own words are not in the note: {said}"
    );
    bare.get_artifact(&renderer)
        .expect("a source that will not compile is still stored");
    assert!(
        bare.read_meta(&renderer).is_err(),
        "a card was written for a source that never compiled"
    );
    bare.read_meta(&geometry).expect("the good one is carded");
    let filed = bare.read_set("sent").expect("the set is filed");
    assert_eq!(
        filed
            .iter()
            .filter(|line| matches!(line.record(), Record::Slot { .. }))
            .count(),
        2,
        "the node that will not compile lost its slot"
    );
}

// -- The authoring form: resolution, and the wall around it -----------

/// An authoring Set file beside the two `.kir` the fixture wrote, naming
/// them by the relative paths they actually have.
///
/// Written by hand rather than by a writer, because there is no writer:
/// a `.kset` is a file a person authors, and what these tests are about is
/// reading one somebody else wrote.
fn authored(dir: &tempfile::TempDir, name: &str, parts: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    std::fs::write(
        &path,
        format!("{{\"t\":\"set\",\"id\":\"authored\",\"v\":1}}\n{parts}"),
    )
    .expect("write");
    path
}

/// **A `.kset` resolves to the `.kbset` it names, with its parts in the
/// store as artifacts.**
///
/// The whole of what resolution is, checked as three separate facts because
/// two of them can hold while the third does not: every `part` has become a
/// `slot`, each `slot` names the content address of the bytes on disk, and
/// the store can hand those bytes back. A resolver that emitted the right
/// records and stored nothing would pass the first two and produce a file
/// nobody can load.
#[test]
fn a_kset_resolves_to_the_kbset_it_names_with_its_parts_in_the_store() {
    let (dir, store, l1, l4) = fixture();
    let kset = authored(
        &dir,
        "night.kset",
        "{\"t\":\"part\",\"layer\":\"L1\",\"path\":\"l1.kir\"}\n\
             {\"t\":\"part\",\"layer\":\"L4\",\"name\":\"veil\",\"path\":\"l4.kir\"}\n\
             {\"t\":\"capacity\",\"layer\":\"L1\",\"value\":8192}\n",
    );

    let resolved = resolve(&store, &kset).expect("every part is beside the file");

    let l1_hash = Hash::of(&std::fs::read(&l1).expect("read"));
    let l4_hash = Hash::of(&std::fs::read(&l4).expect("read"));
    let records: Vec<&Record> = resolved.iter().map(Line::record).collect();
    assert!(
        matches!(records[1], Record::Slot { at: NodeAddress { layer: Layer::L1, index: 0 }, name: None, proc_hash } if *proc_hash == l1_hash),
        "the L1 part became a slot naming its source's address: {:?}",
        records[1]
    );
    assert!(
        matches!(records[2], Record::Slot { at: NodeAddress { layer: Layer::L4, .. }, name: Some(name), proc_hash, .. } if name == "veil" && *proc_hash == l4_hash),
        "the L4 part kept the name this Set gave it: {:?}",
        records[2]
    );
    // **And everything else is passed through unchanged**, which is half of
    // what makes the two forms one format.
    assert!(
        matches!(records[0], Record::Set { id, v: 1 } if id == "authored"),
        "{:?}",
        records[0]
    );
    assert!(
        matches!(
            records[3],
            Record::Capacity {
                at: NodeAddress {
                    layer: Layer::L1,
                    index: 0
                },
                value: 8192
            }
        ),
        "{:?}",
        records[3]
    );
    assert_eq!(records.len(), 4, "no record was added or dropped");

    for hash in [l1_hash, l4_hash] {
        assert!(
            store.get_artifact(&hash).is_ok(),
            "{}: resolution puts the bytes in the store, not only their address",
            hash.short(12)
        );
    }
}

/// **A `.kbset` made from a `.kset` loads with the authoring file deleted,
/// and with the parts it named deleted too** — which is the whole point of
/// the form.
///
/// An authoring file is only readable beside its neighbours; the resolved
/// one is readable anywhere its material is, and a bundle carries the
/// material with it. So this deletes the entire directory the `.kset` and
/// its `.kir` files lived in, takes it into a store that has never held
/// any of it, and loads. Nothing that resolves a path could survive that,
/// which is what makes it the test of the difference rather than of the
/// pipeline.
#[test]
fn a_kbset_made_from_a_kset_loads_with_the_authoring_file_deleted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let beside_it = dir.path().join("parts");
    std::fs::create_dir(&beside_it).expect("mkdir");
    std::fs::write(beside_it.join("l1.kir"), L1).expect("write l1");
    std::fs::write(beside_it.join("l4.kir"), L4).expect("write l4");
    std::fs::write(
        beside_it.join("night.kset"),
        "{\"t\":\"set\",\"id\":\"night\",\"v\":1}\n\
             {\"t\":\"part\",\"layer\":\"L1\",\"path\":\"l1.kir\"}\n\
             {\"t\":\"part\",\"layer\":\"L4\",\"path\":\"l4.kir\"}\n",
    )
    .expect("write kset");

    let author = Store::open(dir.path().join("store")).expect("store");
    let sent = as_a_file(
        &bundle_authored(&author, &beside_it.join("night.kset"))
            .expect("bundle the authoring file"),
    );

    // The authoring file, the parts, and the store that resolved them: all
    // gone. What is left is the text in `sent`.
    std::fs::remove_dir_all(dir.path()).expect("remove the whole directory");

    let elsewhere = tempfile::tempdir().expect("tempdir");
    let bare = Store::open(elsewhere.path()).expect("store");
    unbundle(&bare, &sent).expect("the bundle carries its own sources");
    let loaded = load(&bare, "night").expect("the set is filed and its artifacts are here");
    assert_eq!(loaded.l1s[0].name, "ring");
    assert_eq!(loaded.l4s[0].name, "points");
}

/// **A `part` in a `.kbset` refuses the load**, because it is a file
/// disagreeing with its own extension.
///
/// Not skipped with a note, which is what this reader does with every other
/// line it cannot honour: a `part` is a *node*, and skipping one hands back
/// a Set that is a geometry short. The refusal names the node and the path
/// it wanted.
#[test]
fn a_part_in_a_resolved_set_file_refuses_the_load() {
    let (_dir, store, l1, _l4) = fixture();
    let here = stored(&store, &l1);
    let text = format!(
        "{{\"t\":\"set\",\"id\":\"mixed\",\"v\":1}}\n\
             {{\"t\":\"slot\",\"layer\":\"L1\",\"proc\":\"{here}\"}}\n\
             {{\"t\":\"part\",\"layer\":\"L4\",\"path\":\"l4.kir\"}}\n"
    );
    let refused = from_lines(&store, "mixed", &parsed(&text)).expect_err("a part is refused");
    assert!(refused.contains("l4.kir"), "{refused}");
    assert!(refused.contains("content address"), "{refused}");
}

/// **A part naming an absolute path is refused**, and the refusal names the
/// path.
///
/// The first of the three spellings of one escape. It is refused without
/// the filesystem being asked anything, which is why the path here need not
/// exist — and why a machine where it *does* exist gets the same answer.
#[test]
fn a_part_naming_an_absolute_path_is_refused() {
    let (dir, store, _l1, _l4) = fixture();
    let kset = authored(
        &dir,
        "night.kset",
        "{\"t\":\"part\",\"layer\":\"L1\",\"path\":\"/etc/passwd\"}\n",
    );
    let refused = resolve(&store, &kset).expect_err("an absolute path is not relative");
    assert!(refused.contains("/etc/passwd"), "{refused}");
    assert!(refused.contains("absolute path"), "{refused}");
    assert!(
        refused.contains("Refused rather than repaired"),
        "{refused}"
    );
}

/// **A part that climbs out of the Set file's own directory is refused**,
/// naming what it climbed out of.
///
/// The second spelling. The `.kset` is one level down so that `..` has
/// somewhere to go, and the file it reaches for genuinely exists — a wall
/// that only refuses paths that were not there anyway is not a wall.
#[test]
fn a_part_that_climbs_out_of_the_set_files_directory_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("l1.kir"), L1).expect("write the neighbour above");
    let inside = dir.path().join("inside");
    std::fs::create_dir(&inside).expect("mkdir");
    let kset = inside.join("night.kset");
    std::fs::write(
        &kset,
        "{\"t\":\"set\",\"id\":\"authored\",\"v\":1}\n\
             {\"t\":\"part\",\"layer\":\"L1\",\"path\":\"../l1.kir\"}\n",
    )
    .expect("write");
    let store = Store::open(dir.path().join("store")).expect("store");

    let refused = resolve(&store, &kset).expect_err("`..` climbs out");
    assert!(refused.contains("../l1.kir"), "{refused}");
    assert!(refused.contains("climbs out of"), "{refused}");
    assert!(
        refused.contains(&inside.display().to_string())
            || refused.contains(
                &std::fs::canonicalize(&inside)
                    .expect("canonicalize")
                    .display()
                    .to_string()
            ),
        "the refusal names the directory that was escaped: {refused}"
    );
}

/// **A `..` that lands back inside is an ordinary path and is allowed.**
///
/// What the wall refuses is *leaving*, not the spelling — a rule that
/// refused every `..` would refuse `parts/../l1.kir`, which names a file in
/// the directory the Set file is in, and an operator would learn that by
/// experiment. This is the test that keeps the check on containment rather
/// than on characters.
#[test]
fn a_dotdot_that_lands_back_inside_is_a_path_and_is_allowed() {
    let (dir, store, l1, _l4) = fixture();
    std::fs::create_dir(dir.path().join("parts")).expect("mkdir");
    let kset = authored(
        &dir,
        "night.kset",
        "{\"t\":\"part\",\"layer\":\"L1\",\"path\":\"parts/../l1.kir\"}\n",
    );
    let resolved = resolve(&store, &kset).expect("this path never leaves the directory");
    let l1_hash = Hash::of(&std::fs::read(&l1).expect("read"));
    assert!(
        matches!(resolved[1].record(), Record::Slot { proc_hash, .. } if *proc_hash == l1_hash),
        "{:?}",
        resolved[1].record()
    );
}

/// **A part that is a symlink out of the directory is refused**, which is
/// the spelling that gets missed.
///
/// Lexically this include is one plain component with no `..` and no
/// leading `/`; every character in it is one the other two rules allow. It
/// is only an escape once the link is followed, which is why the comparison
/// is between canonical paths — and why the fixture's own directory is
/// canonicalised too, since on macOS a temporary directory is itself
/// reached through a symlink and a naive comparison would refuse
/// everything.
#[test]
fn a_part_that_is_a_symlink_out_of_the_directory_is_refused() {
    let elsewhere = tempfile::tempdir().expect("tempdir");
    let secret = elsewhere.path().join("secret.kir");
    std::fs::write(&secret, L1).expect("write the file outside");

    let (dir, store, _l1, _l4) = fixture();
    std::os::unix::fs::symlink(&secret, dir.path().join("innocent.kir")).expect("symlink");
    let kset = authored(
        &dir,
        "night.kset",
        "{\"t\":\"part\",\"layer\":\"L1\",\"path\":\"innocent.kir\"}\n",
    );

    let refused = resolve(&store, &kset).expect_err("the link points out of the directory");
    assert!(refused.contains("innocent.kir"), "{refused}");
    assert!(refused.contains("symlink out"), "{refused}");
    assert!(
        refused.contains(
            &std::fs::canonicalize(&secret)
                .expect("canonicalize")
                .display()
                .to_string()
        ),
        "the refusal names where the link actually went: {refused}"
    );
    assert!(
        store
            .get_artifact(&Hash::of(&std::fs::read(&secret).expect("read")))
            .is_err(),
        "a refused part is not in the store: the wall runs before anything is read"
    );
}

/// **A directory reached through a symlink still contains its own parts.**
///
/// The other half of the sentence above, and the failure the first
/// implementation of a containment check makes: canonicalise the target and
/// not the root, and every part of every Set authored under `/var/folders`
/// on macOS — or under any linked path anywhere — is refused as an escape.
/// A wall that refuses everything is a wall somebody switches off.
#[test]
fn a_directory_reached_through_a_symlink_still_contains_its_own_parts() {
    let dir = tempfile::tempdir().expect("tempdir");
    let real = dir.path().join("real");
    std::fs::create_dir(&real).expect("mkdir");
    std::fs::write(real.join("l1.kir"), L1).expect("write");
    std::fs::write(
        real.join("night.kset"),
        "{\"t\":\"set\",\"id\":\"authored\",\"v\":1}\n\
             {\"t\":\"part\",\"layer\":\"L1\",\"path\":\"l1.kir\"}\n",
    )
    .expect("write");
    let linked = dir.path().join("linked");
    std::os::unix::fs::symlink(&real, &linked).expect("symlink");
    let store = Store::open(dir.path().join("store")).expect("store");

    resolve(&store, &linked.join("night.kset"))
        .expect("the file's own directory contains the file's own parts, link or no link");
}

/// **A file that is not a `.kset` is not resolved**, because the extension
/// is the whole of what says which of a Set's two forms a file is.
#[test]
fn a_file_that_is_not_a_kset_is_not_resolved() {
    let (dir, store, _l1, _l4) = fixture();
    let kbset = authored(
        &dir,
        "night.kbset",
        "{\"t\":\"part\",\"layer\":\"L1\",\"path\":\"l1.kir\"}\n",
    );
    let refused = resolve(&store, &kbset).expect_err("a `.kbset` is read, not resolved");
    assert!(refused.contains(".kset"), "{refused}");
}

/// **A part naming a file that is not there says so**, rather than saying
/// it escaped.
///
/// The two are different mistakes and an operator fixes them differently:
/// one is a typo or a part left behind, the other is a file that was trying
/// to leave. A wall that answered "refused" to both would send whoever
/// mistyped `l1.kir` looking for a security problem.
#[test]
fn a_part_naming_a_file_that_is_not_there_says_so() {
    let (dir, store, _l1, _l4) = fixture();
    let kset = authored(
        &dir,
        "night.kset",
        "{\"t\":\"part\",\"layer\":\"L1\",\"path\":\"l9.kir\"}\n",
    );
    let refused = resolve(&store, &kset).expect_err("there is no `l9.kir`");
    assert!(refused.contains("l9.kir"), "{refused}");
    assert!(
        refused.contains("no such file beside the Set file"),
        "{refused}"
    );
}
