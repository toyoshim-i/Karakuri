use super::*;
pub(crate) use karakuri_environment::meta::put_meta;
use std::path::{Path, PathBuf};

/// Interactive live loop source at compile time, scanned by consistency tests.
pub(crate) const SOURCE: &str = concat!(
    include_str!("../../live/interactive/constants.rs"),
    include_str!("../../live/interactive/key.rs"),
    include_str!("../../live/interactive/session.rs"),
    include_str!("../../live/interactive/transport.rs"),
    include_str!("../../live/interactive/mix.rs"),
);

pub(crate) fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

pub(crate) fn example(name: &str) -> String {
    std::fs::read_to_string(workspace().join("examples").join(name)).expect("an example")
}

/// Write `src` into `dir` and hand back the path, so a test can edit a
/// procedure the way an operator does — by replacing the file.
pub(crate) fn kir(dir: &tempfile::TempDir, name: &str, src: &str) -> PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, src).expect("write");
    path
}

/// One slot's files, compiled and sorted exactly as a run compiles them — so
/// `Placed` here carries the layers and indices the real thing carries.
pub(crate) fn slot(paths: &[PathBuf]) -> (Material, Vec<Placed>) {
    let rest: Vec<Named> = paths[1..].iter().cloned().map(Named::bare).collect();
    sort_slot(0, &Named::bare(paths[0].clone()), &rest)
}

/// Builds a `Set` from the given material, capacities, salts, overrides, and camera.
pub(crate) fn set_of(
    gpu: &Gpu,
    material: &Material,
    capacities: &[u32],
    salts: &[u32],
    overrides: &[ParamWrite],
    camera: Option<karakuri_engine::camera::Orbit>,
) -> Set {
    let mut attached = vec![false; 0];
    build(
        gpu,
        &material.l1s,
        &material.l2s,
        &material.l3s,
        &material.fields,
        &material.l4s,
        karakuri_engine::set::Layering::Overdraw,
        &material.names,
        &[],
        capacities,
        &mut attached,
        overrides,
        &[],
        &[],
        salts[0],
        salts,
        camera,
        // No selection: these Sets overdraw, and a fold nobody built has
        // no renderer to be folded to. See `recorded_live`.
        None,
    )
}

/// Returns a `Running` state for a slot initialized with compiled material at launch.
pub(crate) fn launched(placed: &[Placed]) -> Running {
    Running::at_launch(&[placed.to_vec()], 1)
}

/// One node as the watcher hands it over when a build lands: the source in the
/// store, and its address beside the hash.
pub(crate) fn stored(
    store: &karakuri_store::store::Store,
    layer: &'static str,
    src: &str,
) -> Landed {
    (layer, 0, store.put_artifact(src.as_bytes()).expect("put"))
}

pub(crate) type Landed = (&'static str, u32, karakuri_store::hash::Hash);

/// Gathers, saves, and reloads a Set file via `Save::run` and `setfile::load`.
pub(crate) fn save_and_load(
    store_root: &Path,
    id: &str,
    set: &Set,
    sources: Sources,
) -> setfile::Loaded {
    Save {
        slot: 0,
        asked: Asked::Operator,
        id: id.to_string(),
        root: store_root.to_path_buf(),
        sources,
        values: playing_values(set, &[]),
    }
    .run()
    .expect("the Set file is written");
    let store = karakuri_store::store::Store::open(store_root).expect("store");
    setfile::load(&store, id).expect("and reads back")
}

/// Constructs a geometry source string declaring a param with a negative default.
pub(crate) fn signed_l1() -> String {
    let src = example("drift_shell.kir");
    let with_param = src.replace(
        "  param drift      : float [0.0, 2.0] = 0.6",
        "  param drift      : float [0.0, 2.0] = 0.6\n  \
             param signed     : float [-1.0, 1.0] = -0.35",
    );
    assert_ne!(with_param, src, "the example's params moved");
    let with_use = with_param.replace(
        "    position = p;",
        "    position = p + vec3(signed, 0.0, 0.0);",
    );
    assert_ne!(with_use, with_param, "the example's element block moved");
    with_use
}
