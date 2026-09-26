#![allow(unused_imports)]

use super::common::*;

/// Verifies that invalid Set structures (missing layers, duplicate addresses, or gaps) are rejected on save.
#[test]
fn what_the_file_cannot_hold_is_refused_rather_than_written() {
    let (_dir, store, l1, l4) = fixture();
    // Fixtures stored once and referenced by content address across test cases.
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

/// Verifies that built-in camera parameters are serialized in the `camera` record rather than redundant `param` lines.
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
    // Unaddressed param line remains untouched.
    assert!(
        text.contains(r#"{"t":"param","layer":"L1","key":"radius","value":3.25}"#),
        "a bare write was dropped with the camera's:\n{text}"
    );
}

/// Verifies that legacy camera records omitting `height` receive the engine's default camera height.
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

/// Verifies that multiple camera nodes in layer L3 are preserved during load.
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

/// Verifies that an unindexed `camera` record defaults to describing node index 0.
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

/// A `bind` record and a `Binding` are the same thing in two shapes, and `save`
/// writes one from the other. Every generator kind survives, so a saved `fbm`
/// does not come back as the perlin the default would give.
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
