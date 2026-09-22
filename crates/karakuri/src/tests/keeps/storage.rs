use super::*;
use karakuri_console::focus::Step;
use karakuri_console::view::tracker_group;
use karakuri_environment::{audio, mix, Asked};
use karakuri_operation::{BeatSource, GridScale, Operation};
use karakuri_operation_record::{Current, Written};

/// A store root of this test's own, cleared of whatever a previous run left —
/// [`a_library_is_the_store_and_a_missing_store_is_not_made`]'s arrangement, so
/// a keep is written where nothing else is writing.
fn keep_root(what: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "karakuri-keep-{what}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    root
}

/// One keep, gathered as [`Keeping::keep_procedure`] gathers it and run on this
/// thread rather than on a spawned one.
fn keep_run(root: &std::path::Path, asked: Asked, name: &str, source: &str) -> Kept {
    Kept {
        asked,
        name: name.to_owned(),
        root: root.to_path_buf(),
        source: Some(std::sync::Arc::from(source)),
        hash: karakuri_store::hash::Hash::of(source.as_bytes()),
        addr: "L3:0".to_owned(),
        outcome: Ok(std::path::PathBuf::new()),
        reply: None,
    }
    .run()
}

/// The writer puts a node's source under the name it was given, and the outcome
/// says where it went — ADR-0338 decision 4, and it is the act that makes the
/// operator's tier of the library exist at all (P-0096).
///
/// A CPU test: the bytes are the run's and the store is a directory, so nothing
/// here takes a device.
#[test]
fn a_keep_writes_the_nodes_source_into_the_operators_library() {
    let root = keep_root("library");
    let source = "proc orbit_wide {\n  kind L3\n}\n";
    let done = keep_run(&root, Asked::Operator, "orbit_wide", source);
    let path = done
        .outcome
        .as_ref()
        .unwrap_or_else(|e| panic!("the keep was refused: {e}"));
    assert_eq!(
        path,
        &root.join("procedures").join("orbit_wide.kir"),
        "an operator's own act landed outside the library"
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), source);
    let said = done.said().expect("an outcome that landed");
    assert!(
        said.contains("orbit_wide") && said.contains("L3:0"),
        "the outcome names neither what was kept nor where it came from: {said}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A name already kept is refused, and nothing claims otherwise.
///
/// The second keep leaves the first file exactly as it was — a keep is not an
/// instruction to replace, which is where it differs from a Set id and an
/// arrangement's name (`StoreError::ProcedureTaken`).
#[test]
fn a_keep_under_a_name_already_there_is_refused() {
    let root = keep_root("taken");
    let first = "proc orbit_wide {\n  kind L3\n}\n";
    keep_run(&root, Asked::Operator, "orbit_wide", first)
        .outcome
        .expect("the first keep");

    let again = keep_run(
        &root,
        Asked::Operator,
        "orbit_wide",
        "proc other {\n  kind L1\n}\n",
    );
    let said = again.said().expect_err("a taken name was accepted");
    assert!(
        said.contains("orbit_wide"),
        "the refusal does not carry the name the next attempt has to change: {said}"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("procedures").join("orbit_wide.kir")).unwrap(),
        first,
        "a refused keep wrote over what was there"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A model's keep lands in the sandbox and never in the library, which is a Set
/// save's own division one file kind along (ADR-0261, ADR-0301).
#[test]
fn a_models_keep_lands_in_the_sandbox() {
    let root = keep_root("sandbox");
    let done = keep_run(
        &root,
        Asked::Model,
        "20260910-120000",
        "proc orbit_wide {\n  kind L3\n}\n",
    );
    let path = done.outcome.as_ref().expect("the keep");
    assert_eq!(
        path,
        &root
            .join(karakuri_store::store::Store::SANDBOX)
            .join("20260910-120000.kir")
    );
    assert!(
        !root.join("procedures").join("20260910-120000.kir").exists(),
        "a model's keep turned up in the operator's library"
    );
    let said = done.said().expect("an outcome that landed");
    assert!(
        said.contains("sandbox"),
        "a model is not told where its keep went: {said}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A node a build landed has its bytes in the store rather than in hand, which
/// is `setfile::SavedNode::source` being `None`: the watcher put the source
/// there as it built it, so the keep reads it back by the address it carries
/// and never re-reads the `.kir` on disk.
#[test]
fn a_rebuilt_nodes_keep_reads_its_source_back_out_of_the_store() {
    let root = keep_root("rebuilt");
    let source = "proc orbit_wide {\n  kind L3\n}\n";
    let store = Store::open(&root).expect("a store");
    let hash = store.put_artifact(source.as_bytes()).expect("an artifact");

    let done = Kept {
        asked: Asked::Operator,
        name: "orbit_wide".to_owned(),
        root: root.clone(),
        // **`None`, which is what a rebuilt node carries.**
        source: None,
        hash,
        addr: "L3:0".to_owned(),
        outcome: Ok(std::path::PathBuf::new()),
        reply: None,
    }
    .run();
    let path = done.outcome.as_ref().expect("the keep");
    assert_eq!(std::fs::read_to_string(path).unwrap(), source);
    let _ = std::fs::remove_dir_all(&root);
}

/// The two answers that are not a record. `Silent` is named only here, because
/// nothing in the running window reaches that arm; `Owed` is reachable only by
/// a reading this window failed to take, which is the mask's accident and the
/// sync chip's missing tempo and is asserted below. Neither is a gap in the
/// vocabulary any more — the sync chip's was `Owed::NotSettled` until the
/// conversion took a session tempo, and [`unwritten`] carries what that was.
use karakuri_operation_record::{Owed, Silent};

/// The two spellings of feedback's ceiling are one number, and this is the
/// package that can see both.
///
/// `karakuri_operation::Feedback::MAX` is the reach a fader draws;
/// `karakuri_engine::master::Chain::FEEDBACK_MAX` is the wall the engine clamps
/// at, where the record is applied. Two crates state it because neither may
/// depend on the other, and both say at their own definition that it is a
/// convention held here — which is `Current::tempo`'s arrangement one control
/// along, and the shape `docs/contributing.md` §4 asks for when a guarantee
/// cannot be structural.
///
/// Both directions of the cut list too, for the same reason and by the same
/// route: `mix::cut` takes the engine's word to the vocabulary's and
/// `Cut::parse` takes the record's word back, so a cut that survived one leg
/// and not the other would be a picture a replay draws differently. The
/// crossing lives in `karakuri-environment` beside `mix::tonemap` and
/// `mix::sync`, because this window and `karakuri-cli` both make it.
#[test]
fn the_feedback_ceiling_and_the_cut_list_are_one_answer_in_two_crates() {
    assert_eq!(
        karakuri_operation::Feedback::MAX,
        karakuri_engine::Chain::FEEDBACK_MAX,
        "the reach the console draws and the wall the engine clamps at have drifted"
    );
    // **The wall itself moved into the file**, which is the one thing that
    // changed here when the chain became a list: what a slot's `amount` is
    // clamped to is the range `examples/feedback.kir` declares, and this
    // constant is now the *vocabulary's* number rather than the engine's
    // own — kept in the engine because this is the pair of spellings that
    // has to be checked against each other and there has to be somewhere to
    // check it. **The file declares a wider range than the fader draws**,
    // `[0, 1]` against 0.95, and that is a gap named rather than closed:
    // closing it is a `.kir` edit, which changes the procedure's content
    // address and therefore every stream that names it.
    for cut in Cut::ALL {
        assert_eq!(
            Cut::parse(mix::cut(cut).name()),
            Some(cut),
            "a cut did not survive the round trip through a record's word"
        );
    }
}
