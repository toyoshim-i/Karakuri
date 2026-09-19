#![allow(unused_imports, dead_code)]

pub(super) use super::super::*;
pub(super) use karakuri_engine::binding::{Curve, NOISE_SIGNAL};
pub(super) use karakuri_engine::camera::Orbit;
pub(super) use karakuri_engine::set::Layering;
pub(super) use karakuri_engine::{Binding, ParamWrite};
pub(super) use karakuri_ir::typed::Checked;
pub(super) use karakuri_ir::Kind;
pub(super) use karakuri_signal::{NoiseConfig, NoiseKind};
pub(super) use karakuri_store::hash::Hash;
pub(super) use karakuri_store::ndjson::Line;
pub(super) use karakuri_store::record::{BindNoise, Layer, NodeAddress, Record, Value};
pub(super) use karakuri_store::store::Store;

pub(super) use crate::Asked;

pub(super) const L1: &str = r#"
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

pub(super) const L4: &str = r#"
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

/// The rest of the chain a `--set` can spell, minimal for the reason the pair
/// above is: what is under test is the file, not the picture.
pub(super) const L2: &str = r#"
proc warp {
  kind L2

  consumes position

  deform {
    position = vec3(position.x, position.y * 1.5, position.z);
  }
}
"#;

pub(super) const L3: &str = r#"
proc look {
  kind L3

  camera {
    eye    = vec3(0.0, 2.0, 9.0);
    target = vec3(0.0, 0.0, 0.0);
  }
}
"#;

pub(super) const FIELD: &str = r#"
proc blob {
  kind Field

  field {
    distance = sd_sphere(point, 1.0);
  }
}
"#;

/// A store with the two procedures written out beside it, and the paths.
pub(super) fn fixture() -> (
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
pub(super) fn beside(dir: &tempfile::TempDir, file: &str, src: &str) -> std::path::PathBuf {
    let path = dir.path().join(file);
    std::fs::write(&path, src).expect("write");
    path
}

/// A fixture's source, in the store, as [`save`] now wants it. The tests here
/// are written against files on disk, because a file is what a fixture is; the
/// writer takes hashes. This is the one line that bridges them, rather than
/// every test growing its own `put`.
pub(super) fn stored(store: &Store, path: &std::path::Path) -> Hash {
    store
        .put_artifact(&std::fs::read(path).expect("read"))
        .expect("put")
}

/// The nodes of an ordinary Set — one geometry, and the renderers over it in
/// draw order.
pub(super) fn ordinary(
    store: &Store,
    l1: &std::path::Path,
    l4s: &[std::path::PathBuf],
) -> Vec<Node> {
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

/// The whole set file as bytes, which is what a compatibility claim is about.
/// `read` keeps each line's text verbatim, so this is what is on disk rather
/// than a re-serialisation of it.
pub(super) fn written(store: &Store, id: &str) -> String {
    store
        .read_set(id)
        .expect("read")
        .iter()
        .map(|line| format!("{}\n", line.as_str()))
        .collect()
}

/// A Set file's text, through the file reader — so the bytes a test writes out
/// are genuinely parsed, rather than hand-built into records that could not
/// have been written. The file is gone by the time this returns; the lines are
/// in memory.
pub(super) fn parsed(text: &str) -> Vec<Line> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("hand_written.kbset");
    std::fs::write(&path, text).expect("write");
    karakuri_store::ndjson::read(&path).expect("a hand-written Set file parses")
}

/// A Set with nothing but its material and whatever bindings are given.
/// `Orbit::default()` is not `const`, so this is the one place a test names its
/// fields; `LazyLock` keeps that to one place rather than one per call.
pub(super) static DEFAULT_CAMERA: std::sync::LazyLock<Orbit> =
    std::sync::LazyLock::new(Orbit::default);

pub(super) fn plain<'a>(nodes: &'a [Node], bindings: &'a [Binding]) -> Saving<'a> {
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
