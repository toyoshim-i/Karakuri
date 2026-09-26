//! Console regions and their corresponding lookup names.
//!
//! Reconciled against `docs/manual/console.html` so names reachable by
//! keyboard, MIDI, and MCP match the manual in both directions.

mod common;

use common::names;

/// Panel regions named in manual headings, including bays drawn by the mock.
const MANUAL: &[&str] = &[
    "transport",
    "library",
    "staging",
    "prompt",
    "program",
    "inspector",
    "mixer",
    "master",
    "sequencer",
    "outputs",
];

/// Structural region names and layout identifiers (ADR-0159).
const STRUCTURAL: &[&str] = &[
    "left-pane",
    "centre",
    "right-pane",
    "inspector-1",
    "inspector-2",
    "program-view",
    "deck-previews",
];

#[test]
fn every_region_the_manual_names_resolves() {
    let layout = karakuri_console::layout();
    for name in MANUAL {
        assert!(
            layout.find(name).is_some(),
            "the manual names {name} and the arrangement does not"
        );
    }
}

#[test]
fn nothing_resolves_that_is_not_a_named_region() {
    let layout = karakuri_console::layout();
    let mut found = names(&layout);
    found.sort();

    let mut expected: Vec<String> = MANUAL
        .iter()
        .chain(STRUCTURAL)
        .map(|s| (*s).to_owned())
        .collect();
    expected.sort();

    assert_eq!(
        found, expected,
        "the arrangement's names and the manual's regions have diverged"
    );
}

/// The body row is unnamed to prevent direct naming operations (ADR-0197)
/// while still supporting pointer-based divider folding.
#[test]
fn the_row_holding_the_panes_and_the_centre_is_unnamed() {
    let layout = karakuri_console::layout();
    let body = layout.children(layout.root())[1];
    assert_eq!(layout.name(body), None);
    assert_eq!(layout.children(body).len(), 3);
}
