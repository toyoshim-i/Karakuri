//! The regions the console has, and the names they answer to.
//!
//! A name is how the keyboard, a MIDI map and MCP each reach a region, so this
//! is checked against `docs/manual/console.html` rather than against the
//! arrangement: the list below is read off the manual, and the arrangement has
//! to match it in both directions.

mod common;

use common::names;

/// Every heading in *What each region is standing on* that names a region of
/// the panel, plus the two bays the mock draws that the headings fold together
/// — "Library, and staging under it" is one heading and two bays, and the mock
/// heads them `Library` and `Staging`.
///
/// The headings that are not regions are not here: *Who is holding a control*,
/// *A knob is bound to a deck*, *Two focuses*, *Authority is per node* and
/// *Every icon explains itself* are all properties of what a region contains.
/// *Health, in the transport* names the transport, which is also the mock's
/// first bay.
const MANUAL: &[&str] = &[
    "transport",
    "library",
    "staging",
    "program",
    "inspector",
    "mixer",
    "master",
    "sequencer",
    "outputs",
];

/// Names that are not headings of *What each region is standing on*, each with
/// what it is and where the word came from.
///
/// The three columns are the manual's lede: a left pane and a right pane "which
/// fold away to give room, and the centre, which is what they give it to".
/// `left-pane` is `karakuri-layout`'s own word for the first of them, the split
/// that "fold the left pane away" reaches by name, and `right-pane` is that
/// operation on the other side. `centre` is deliberately not a third pane
/// (ADR-0159) — folding it is not an operation anybody wants — and it is named
/// because the drag on the program's height addresses it.
///
/// The inspector's panes are the *n* the manual describes ("n panes, each
/// showing whatever you point it at"), numbered because the mock's control for
/// them counts — `2 up` — and because a view's name is required.
///
/// The Program bay's two regions are the last two, and they are the one pair
/// here the manual names outright without heading them. *"The bay is two
/// regions and they fold apart. The picture is a sink, listed in Outputs as
/// program view ... The deck previews under it are auditions of their own, so
/// they stay when it goes."* Both words are the manual's, and both are
/// addressed: turning the sink off is a fold on `program-view`, and the row
/// staying when it goes is `deck-previews` not folding with it.
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

/// The body row — the one holding the two panes and the centre — has no name,
/// and *nothing reaches it* is not the reason. `Layout::hit` hands the row out
/// as `Hit::Divider { split, .. }` and `crates/karakuri/src/main.rs`'s
/// fold-at-pointer turns that into `Op::Fold(split)`, so `g` over the gap
/// between two panes folds this row today. What it cannot be is reached by
/// anything holding only a name — a keyboard, a MIDI map or MCP — and giving it
/// one would assert that folding the row of three panes is an operation an
/// operator asks for, which is a decision nobody has taken (ADR-0197).
///
/// So this is not a test that the row is unreachable. It pins the two things
/// that decision would change: the row is still the three-way split holding the
/// panes and the centre, and it still answers `None` when asked for a name.
/// Naming it is then a line somebody writes here on purpose, rather than one
/// that arrives with an edit to the arrangement.
#[test]
fn the_row_holding_the_panes_and_the_centre_is_unnamed() {
    let layout = karakuri_console::layout();
    let body = layout.children(layout.root())[1];
    assert_eq!(layout.name(body), None);
    assert_eq!(layout.children(body).len(), 3);
}
