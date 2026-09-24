use std::sync::mpsc;

use karakuri_ir::Kind;
use serde_json::{json, Value};

use super::server::*;
use super::*;

/// Creates a mock test state with channels disconnected from a real render loop.
pub(crate) fn state(events: mpsc::Receiver<Event>) -> State {
    State {
        slots: slots(),
        store: "a/store".into(),
        watching: true,
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

/// Returns test slots configuration with mock file paths.
pub(crate) fn slots() -> Slots {
    Slots::of(vec![
        ("a/l1.kir".into(), vec!["a/l4.kir".into()]),
        ("b/l1.kir".into(), vec!["b/l4.kir".into()]),
    ])
}

/// Creates a test fixture declaring a layer `kind` with invalid procedure content.
pub(crate) fn declaring(dir: &std::path::Path, name: &str, kind: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, format!("kind {kind}\nnot a procedure at all\n")).expect("fixture");
    path
}

/// Extracts the text under a specific `# ` heading from rendered vocabulary documentation.
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
