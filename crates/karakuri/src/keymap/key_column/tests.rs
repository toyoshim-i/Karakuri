use std::collections::BTreeSet;

use super::*;
use crate::keymap::KEY_BINDINGS;
use crate::KEYS;
use karakuri_console::focus::ANY;

/// The floor under both directions: a scan that matched nothing would satisfy
/// every loop below by iterating over nothing at all.
#[test]
fn the_scan_finds_the_page_and_the_keys() {
    let badges = key_badges();
    assert!(
        badges.len() >= 54,
        "only {} rows with a key badge found in {PAGE} — is a row still `{ROW}` followed by \
         an `<h3>` and its `rt` badges?",
        badges.len()
    );
    assert!(
        bound().len() >= 9,
        "only {} keys found bound — the window loop's `match` has more arms than \
         this, and a check below it is a check that has stopped matching code",
        bound().len()
    );
}

/// Verifies that KEYS entries match the window loop's bound key arms.
#[test]
fn the_keys_this_file_lists_are_the_keys_the_window_loop_binds() {
    let listed: BTreeSet<String> = KEYS.iter().map(|(k, _)| (*k).to_owned()).collect();
    assert_eq!(
        bound(),
        listed,
        "the keys the `match` in `window_event` binds are not the ones `KEYS` lists — which \
         is the list the legend prints. An arm this file does not know about reaches an \
         operation nothing checks the badge of and is told to nobody; an entry with no arm \
         is a legend naming a key an operator presses to no effect"
    );
}

/// Asserts that every key binding with a title matches a built operation row on the manual page.
#[test]
fn every_binding_the_table_names_a_title_for_reaches_a_route_marked_built() {
    let badges = key_badges();
    for binding in KEY_BINDINGS {
        let Some(title) = binding.title else {
            continue;
        };
        let found = badges
            .iter()
            .find(|(page_title, _, _)| page_title == title)
            .unwrap_or_else(|| {
                panic!(
                    "`{}` names `{title}` and {PAGE} has no row with that heading — the \
                     page is the specification, so add the row there first",
                    binding.legend
                )
            });
        assert_eq!(
            found.1,
            "has",
            "`{}`{} performs `{title}` at the panel, which {PAGE} marks `{}` in the key \
             column — an operation an operator reaches from the keyboard and a page that \
             says the instrument does not",
            binding.legend,
            said(binding.bay),
            found.1
        );
        let (keys, named) = parsed(&found.2).unwrap_or_else(|| {
            panic!(
                "{PAGE} marks `{title}` built in the key column and its badge `{}` names a \
                 bay this file has no name for",
                found.2
            )
        });
        assert_eq!(
            named,
            binding.bay,
            "`{}`{} performs `{title}` and {PAGE}'s badge for it reads `{}`{} — a press \
             goes to the bay that has focus, so a badge naming the wrong bay tells an \
             operator to address the key somewhere the press does nothing",
            binding.legend,
            said(binding.bay),
            found.2,
            said(named)
        );
        assert!(
            keys.iter().any(|k| k == binding.legend),
            "`{}`{} performs `{title}` and {PAGE} marks that row built in the key column \
             naming `{}` — a badge that says an operator reaches it by pressing something \
             else",
            binding.legend,
            said(binding.bay),
            found.2
        );
    }
}

/// Every hotkey in [`super::KEY_BINDINGS`] aligned with `ControlDescriptor` in
/// `karakuri_console::control`.
#[test]
fn every_bound_hotkey_aligns_with_control_descriptors() {
    for binding in KEY_BINDINGS {
        if let Some(desc) = karakuri_console::control::descriptor_for_hotkey(binding.legend) {
            if let (Some(title), Some(op_title)) = (binding.title, desc.operation_title) {
                assert_eq!(title, op_title);
            }
        }
    }
}

// Mixer grammar deck-routing and value-stepping assertions are verified behaviorally in GPU tests (ADR-0156).

/// And every key the legend prints has its rows written down, both ways round,
/// which is what keeps [`ROWS`] from being a second list of keys rather than a
/// mapping off the first.
#[test]
fn every_key_the_legend_prints_has_its_rows_written_down() {
    let printed: BTreeSet<&str> = KEYS
        .iter()
        .map(|(k, _)| *k)
        // Collapse arrow keys into a single grammar category.
        .map(|key| match SPELLED.iter().find(|(page, _)| *page == key) {
            Some((_, name)) => *name,
            None => key,
        })
        .collect();
    let mapped: BTreeSet<&str> = ROWS.iter().map(|(_, k, _)| *k).collect();
    assert_eq!(
        printed, mapped,
        "a key the legend prints has no entry in `ROWS`, or `ROWS` maps a key the legend \
         does not print. The rows a key reaches cannot be derived — a fold is a bay or a \
         pane depending on the pointer — so the mapping is written down, and this is what \
         says it is written down for exactly the keys this program binds"
    );
}

/// And the keys that reach no row are exactly [`NO_ROW`], both ways round.
#[test]
fn the_keys_that_reach_no_row_are_the_ones_written_down() {
    let silent: Vec<(Option<&str>, &str)> = ROWS
        .iter()
        .filter(|(_, _, rows)| rows.is_empty())
        .map(|(bay, key, _)| (*bay, *key))
        .collect();
    assert_eq!(
        silent, NO_ROW,
        "the keys that reach no row on {PAGE} are not the ones this file says they are — a \
         key that performs something the page never specified is a route nobody named"
    );
}

/// Asserts that every bound route reaches a manual row marked as `has` (ADR-0213, ADR-0259).
#[test]
fn every_key_the_instrument_binds_reaches_a_route_marked_built() {
    let badges = key_badges();
    for (bay, key, rows) in ROWS {
        for row in *rows {
            let found = badges
                .iter()
                .find(|(title, _, _)| title == row)
                .unwrap_or_else(|| {
                    panic!(
                        "`{key}` reaches `{row}` and {PAGE} has no row with that heading — \
                         the page is the specification, so add the row there first"
                    )
                });
            assert_eq!(
                found.1,
                "has",
                "`{key}`{} performs `{row}` at the panel, which {PAGE} marks `{}` in the key \
                 column — an operation an operator reaches from the keyboard and a page \
                 that says the instrument does not. Flip the badge, or say here why the key \
                 does not reach it",
                said(*bay),
                found.1
            );
            let (keys, named) = parsed(&found.2).unwrap_or_else(|| {
                panic!(
                    "{PAGE} marks `{row}` built in the key column and its badge `{}` names a \
                     bay this file has no name for — a badge is a key, or a key and the bay \
                     it is addressed in",
                    found.2
                )
            });
            assert_eq!(
                named,
                *bay,
                "`{key}`{} performs `{row}` and {PAGE}'s badge for it reads `{}`{} — a press \
                 goes to the bay that has focus, so a badge naming the wrong bay tells an \
                 operator to address the keys somewhere the press does nothing",
                said(*bay),
                found.2,
                said(named)
            );
            // Each manual row is reached by a single route badge.
            assert!(
                keys.iter().any(|k| k == key),
                "`{key}`{} performs `{row}` and {PAGE} marks that row built in the key column \
                 naming `{}` — a badge that says an operator reaches it by pressing \
                 something else",
                said(*bay),
                found.2
            );
        }
    }
}

/// Asserts that every route marked `has` in the manual is actively bound by the instrument.
#[test]
fn every_key_route_the_page_marks_built_is_bound_by_the_instrument() {
    let badges = key_badges();
    let claimed: Vec<&(String, String, String)> = badges
        .iter()
        .filter(|(_, class, _)| class == "has")
        .collect();
    assert!(
        claimed.len() >= 6,
        "only {} rows of {PAGE} mark a key route built — the scan found less than the column \
         holds, which would pass this test by finding nothing",
        claimed.len()
    );
    for (title, _, badge) in claimed {
        assert_ne!(
            badge, NOWHERE,
            "{PAGE} marks `{title}` built in the key column and names no key for it — a \
             `has` badge says an operator reaches the operation, so it has to say what to \
             press"
        );
        let (keys, bay) = parsed(badge).unwrap_or_else(|| {
            panic!(
                "{PAGE} marks `{title}` built in the key column and its badge `{badge}` names \
                 a bay this file has no name for — the nine are in `BAYS`, spelled the way \
                 the page says them"
            )
        });
        for key in keys {
            // "in any bay" requires all bays to declare the action.
            if bay == Some(ANY) {
                assert!(
                    karakuri_console::focus::BUILT.len() == BAYS.len(),
                    "{PAGE} says `{title}` is reached in any bay and \
                     `karakuri_console::focus::BUILT` declares {} of the {} the arrangement \
                     has — a press that works in some of them is not one that works in any",
                    karakuri_console::focus::BUILT.len(),
                    BAYS.len()
                );
            }
            let rows = rows_of(bay, &key).unwrap_or_else(|| {
                panic!(
                    "{PAGE} marks `{title}` built in the key column and names `{key}`{}, \
                     which this program does not bind — the page tells a player to press a \
                     key the instrument does not read. Either the key went and the badge is \
                     `plan` again, or it is another program's: the key column is the \
                     instrument's keyboard, and `karakuri-cli`'s keys are its own",
                    said(bay)
                )
            });
            assert!(
                rows.contains(&title.as_str()),
                "{PAGE} marks `{title}` built in the key column and names `{key}`{}, which \
                 this program binds to {rows:?} instead — one press, two operations",
                said(bay)
            );
        }
    }
}

/// Verifies that key grammar documented in [`ROWS`] matches console declared focus dispatch (ADR-0259, ADR-0331).
#[test]
fn the_grammar_the_page_names_is_the_grammar_the_console_declares() {
    let declared: BTreeSet<(&str, &str)> = karakuri_console::focus::reaches()
        .into_iter()
        .map(|(bay, key)| {
            (
                bay,
                match key {
                    karakuri_console::focus::Grammar::Digit => DIGIT,
                    karakuri_console::focus::Grammar::Arrows => "arrows",
                    karakuri_console::focus::Grammar::Space => "space",
                    karakuri_console::focus::Grammar::Enter => "enter",
                },
            )
        })
        .collect();
    let written: BTreeSet<(&str, &str)> = ROWS
        .iter()
        // Filter to the four console-dispatched grammar keys (excluding letter keys).
        .filter(|(_, key, _)| [DIGIT, "arrows", "space", "enter"].contains(key))
        .filter_map(|(bay, key, _)| bay.map(|bay| (bay, *key)))
        .collect();
    assert_eq!(
        written, declared,
        "the routes this file writes down and the ones `karakuri_console::focus::BUILT` \
         declares are not the same routes. A pair the console declares and this file does \
         not is a press an operator can make that no badge describes; a pair here the \
         console does not declare is a badge naming a key that reaches nothing in that bay"
    );
    assert!(
        declared.len() >= 31,
        "only {} routes are declared by the dispatch table — nine bays and the fold that is \
         addressed in all of them come to 31, so a scan finding fewer has stopped reading it",
        declared.len()
    );
}

/// Every bay a badge names is a bay the arrangement has, which is the floor
/// under [`parsed`]: a spelling nobody can resolve reads as a global key, and a
/// global key that reached a bay's row would pass both badge checks by naming
/// the wrong thing consistently.
#[test]
fn the_bay_names_the_page_uses_are_the_arrangements_own() {
    for (page, name) in BAYS {
        assert!(
            karakuri_console::view::region(name).is_some(),
            "{PAGE} names a bay `{page}` and this file resolves it to `{name}`, which is not \
             a region the console draws"
        );
    }
    assert_eq!(
        BAYS.len(),
        9,
        "the manual's *What each region is standing on* lists nine bays and this file has {}",
        BAYS.len()
    );
}
