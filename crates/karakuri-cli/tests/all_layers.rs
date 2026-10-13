use std::path::{Path, PathBuf};
use std::process::Command;

use karakuri_environment::setfile;
use karakuri_store::hash::Hash;
use karakuri_store::store::Store;

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

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn run(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_karakuri-cli"))
        .current_dir(workspace())
        .args(args)
        .output()
        .expect("run karakuri-cli");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    (out.status.success(), stderr)
}

#[test]
fn cli_load_set_with_and_without_watch_includes_all_six_layers_and_l5_params() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store_path = dir.path().join("store");
    let store = Store::open(&store_path).expect("store");

    let sources = [
        (karakuri_ir::Kind::L1, L1, "test_l1"),
        (karakuri_ir::Kind::L2, L2, "test_l2"),
        (karakuri_ir::Kind::L3, L3, "test_l3"),
        (karakuri_ir::Kind::Field, FIELD, "test_field"),
        (karakuri_ir::Kind::L4, L4, "test_l4"),
        (karakuri_ir::Kind::L5, L5, "test_l5"),
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
        karakuri_environment::Asked::Operator,
        "all_six",
        setfile::Saving {
            nodes: &nodes,
            capacities: &[512],
            params: &params,
            bindings: &[],
            edges: &[],
            camera: &karakuri_engine::camera::Orbit::default(),
            layering: karakuri_engine::set::Layering::Overdraw,
            live: None,
            seeds: &[1],
        },
    )
    .expect("save all_six set");

    let store_str = store_path.to_str().unwrap();
    let out_no_watch = dir.path().join("out_no_watch.png");
    let out_no_watch_str = out_no_watch.to_str().unwrap();

    let (ok, stderr_no_watch) = run(&[
        "--store",
        store_str,
        "--load-set",
        "all_six",
        "--render",
        out_no_watch_str,
        "--frames",
        "2",
    ]);
    assert!(ok, "run without --watch failed:\n{stderr_no_watch}");
    assert!(
        !stderr_no_watch.contains("no parameter named `scanline`"),
        "stderr without --watch rejected scanline parameter:\n{stderr_no_watch}"
    );

    let out_watch = dir.path().join("out_watch.png");
    let out_watch_str = out_watch.to_str().unwrap();

    let (ok, stderr_watch) = run(&[
        "--store",
        store_str,
        "--load-set",
        "all_six",
        "--watch",
        "--render",
        out_watch_str,
        "--frames",
        "2",
    ]);
    assert!(ok, "run with --watch failed:\n{stderr_watch}");
    assert!(
        !stderr_watch.contains("no parameter named `scanline`"),
        "stderr with --watch rejected scanline parameter:\n{stderr_watch}"
    );
    assert!(
        stderr_watch.contains("test_l5"),
        "stderr with --watch must include test_l5 in nodes/slot:\n{stderr_watch}"
    );
}
