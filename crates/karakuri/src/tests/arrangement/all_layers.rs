use super::*;
use karakuri_environment::setfile;
use karakuri_store::Hash;

const L1: &str = r#"
proc test_l1 {
  kind L1
  topology points
  capacity [128, 4096] = 512
  emit position
  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#;

const L2: &str = r#"
proc test_l2 {
  kind L2
  consumes position
  deform {
    position = position + vec3(1.0, 0.0, 0.0);
  }
}
"#;

const L3: &str = r#"
proc test_l3 {
  kind L3
  camera {
    eye = vec3(0.0, 2.0, 9.0);
    target = vec3(0.0, 0.0, 0.0);
  }
}
"#;

const FIELD: &str = r#"
proc test_field {
  kind Field
  field {
    distance = sd_sphere(point, 1.0);
  }
}
"#;

const L4: &str = r#"
proc test_l4 {
  kind L4
  blend additive
  consumes position
  vertex {
    clip = camera * vec4(position, 1.0);
    point_rate = 0.016;
  }
  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;

const L5: &str = r#"
proc test_l5 {
  kind L5
  param scanline : float [0.0, 1.0] = 0.5
  frame {
    let s = tap(src, point_coord);
    color = vec4(s.xyz * scanline, s.w);
  }
}
"#;

#[test]
fn a_load_with_all_six_layers_includes_l5_and_its_parameters() {
    let root = std::env::temp_dir().join(format!(
        "karakuri-all-layers-load-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let store = Store::open(&root).expect("store");

    let sources = [
        (karakuri_ir::Kind::L1, L1, "l1"),
        (karakuri_ir::Kind::L2, L2, "l2"),
        (karakuri_ir::Kind::L3, L3, "l3"),
        (karakuri_ir::Kind::Field, FIELD, "field"),
        (karakuri_ir::Kind::L4, L4, "l4"),
        (karakuri_ir::Kind::L5, L5, "l5"),
    ];

    let nodes: Vec<setfile::Node> = sources
        .iter()
        .map(|(kind, src, name)| {
            let hash = Hash::of(src.as_bytes());
            store.put_artifact(src.as_bytes()).expect("store artifact");
            setfile::Node {
                hash,
                layer: *kind,
                index: 0,
                name: Some((*name).to_string()),
            }
        })
        .collect();

    let params = vec![karakuri_engine::ParamWrite::at(
        karakuri_ir::Kind::L5,
        0,
        "scanline",
        0.8,
    )];

    setfile::save(
        &store,
        Asked::Operator,
        "all_six",
        setfile::Saving {
            nodes: &nodes,
            capacities: &[128],
            params: &params,
            bindings: &[],
            edges: &[],
            camera: &karakuri_engine::camera::Orbit::default(),
            layering: Layering::Overdraw,
            live: None,
            seeds: &[1],
        },
    )
    .expect("save all_six set");

    let (tx, rx) = std::sync::mpsc::channel();
    let mut aiming = Aiming::new(
        tx,
        watch::Aim {
            head: karakuri_environment::compile::Named::bare("dummy.kir"),
            rest: Vec::new(),
            layering: Layering::Overdraw,
            live: None,
            capacity: None,
            seed_salt: 0,
            salts: Vec::new(),
            camera: karakuri_engine::camera::Orbit::default(),
            overrides: Vec::new(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges: Vec::new(),
            authorities: Vec::new(),
            set: None,
        },
        karakuri_mcp::Slots::unpointed(),
        0,
    );

    let _line = loading(&root, 0, 1, &mut aiming, "all_six").expect("loading succeeds");
    let aim = rx.try_recv().expect("aim received");

    assert_eq!(aim.head.name.as_deref(), Some("l1"));
    assert_eq!(
        aim.rest.len(),
        5,
        "aim.rest must have exactly 5 remaining layers (L2, L3, L4, Field, L5)"
    );
    let rest_names: Vec<Option<&str>> = aim.rest.iter().map(|n| n.name.as_deref()).collect();
    assert!(
        rest_names.contains(&Some("l5")),
        "aim.rest must contain l5, but has: {rest_names:?}"
    );
    assert!(
        aim.overrides
            .iter()
            .any(|p| p.key == "scanline" && p.value == 0.8),
        "aim.overrides must contain scanline parameter for L5"
    );

    let _ = std::fs::remove_dir_all(&root);
}
