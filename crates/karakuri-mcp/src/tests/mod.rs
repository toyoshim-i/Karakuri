use std::sync::mpsc;

use karakuri_ir::Kind;
use serde_json::{json, Value};

use super::server::*;
use super::*;

/// A state with the loop's half of both channels missing, for the tests that
/// are about what one method answers rather than about a render loop.
///
/// The save channel's receiver is dropped on the way out, which is exactly the
/// "the loop is gone" case: anything that tried to ask for a save here would be
/// told so rather than wait.
pub(crate) fn state(events: mpsc::Receiver<Event>) -> State {
    State {
        slots: slots(),
        // **A root, and nothing here opens it.** Only `read_set` does, on
        // the call, which is what lets every test in this module build a
        // state without a directory — see `serve`.
        store: "a/store".into(),
        watching: true,
        // **Closed, all four classes**, which is the state a run starts in
        // and the state every test in this module reasons under.
        opening: karakuri_environment::Opening::closed(),
        slot_policies: karakuri_environment::SlotPolicies::default(),
        events,
        asked: mpsc::sync_channel(ASKED).0,
        wiring: mpsc::sync_channel(ASKED).0,
        operating: mpsc::sync_channel(ASKED).0,
        recent: Vec::new(),
        dropped: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
    }
}

/// Two slots of a head and one more file, and none of these paths exists.
///
/// That is deliberate rather than lazy. A layer is read off a file's own `kind`
/// line, and a file that cannot be read counts as a renderer — the fallback
/// [`Slots::nodes`] shares with `history::seed`, asserted in
/// [`an_unreadable_file_is_counted_as_a_renderer`] and relied on here, so these
/// two slots are the L1-and-one-renderer pair they read as.
pub(crate) fn slots() -> Slots {
    Slots::of(vec![
        ("a/l1.kir".into(), vec!["a/l4.kir".into()]),
        ("b/l1.kir".into(), vec!["b/l4.kir".into()]),
    ])
}

/// Writes `name` declaring `kind`, and nothing that would compile.
///
/// A layer is scanned out of the text rather than parsed, so that this surface
/// works on a file the checker would refuse — which is the file a model most
/// needs to be able to read. A fixture that compiled would not say so.
pub(crate) fn declaring(dir: &std::path::Path, name: &str, kind: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, format!("kind {kind}\nnot a procedure at all\n")).expect("fixture");
    path
}

/// The text under one `# ` heading of the rendered vocabulary.
///
/// The two halves of the test below ask opposite questions of one section each,
/// and mixing sections silently weakens both — the "nothing invented" half went
/// looking for `clip` in `Builtin::from_name` the moment stage outputs were
/// added to the page, which is the failure working rather than a nuisance.
fn section<'a>(rendered: &'a str, heading: &str) -> &'a str {
    let start = rendered
        .find(heading)
        .unwrap_or_else(|| panic!("the vocabulary has no `{heading}` section"));
    let rest = &rendered[start + heading.len()..];
    match rest.find("\n# ") {
        Some(end) => &rest[..end],
        None => rest,
    }
}

mod operations;
mod routes;
mod save;
mod tools;
mod vocabulary;
