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

/// A Set built from that material, at the capacities, salts and camera given —
/// which is what a run hands `build`, and what a save has to read back off the
/// Set rather than off these arguments.
///
/// `camera` is an `Option` because `build`'s is: `None` is a slot no `camera`
/// record reached, which leaves the Set at the built-in orbit's defaults. See
/// `recorded_camera`.
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

/// What a run does to one slot before its first frame: the slot recorded as
/// running the material that was compiled for it.
///
/// Through [`Running::at_launch`] rather than around it, for the reason
/// [`save_and_load`] goes through [`Save::run`]: assembling the seeding by hand
/// here would be a second copy of it, and the whole of what these tests are
/// about is that there is only one.
///
/// No store root, because there is no store. A windowed run touches one when a
/// save happens and not before — see [`Running::at_launch`].
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

/// Gather, write, and read back — the whole of what pressing `k` does, minus
/// the thread and the channel.
///
/// Through [`Save::run`] rather than around it, so that what a test exercises
/// is the function the spawned thread calls. Assembling the same three steps by
/// hand here would be a second copy of the save, and a test of a copy is a test
/// of nothing.
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

/// A geometry with a negative default, which is the declaration the fold exists
/// for and the one no example in the tree carries.
///
/// `drift_shell` with one param added and read, so the procedure still
/// compiles, still draws, and now declares a default that is `Unary { Neg, Lit
/// }` rather than a literal.
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
