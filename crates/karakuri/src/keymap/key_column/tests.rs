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

/// [`super::KEY_BINDINGS`] checked directly against the page, which is what a
/// table buys that a text scan never could: a key that names a row can be held
/// against that row, rather than merely counted. [`ROWS`] checks the same page
/// for the same ten keys already, by hand, in
/// [`every_key_the_instrument_binds_reaches_a_route_marked_built`] below — this
/// is the table checking itself, off `KeyBinding::title` rather than off a
/// second, hand-written list, and it is what makes `title` a fact this file
/// relies on rather than a field nothing reads.
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

// **The grammar's mix answers act on the deck the address is on, and read
// the value they step from off that deck** — no longer checked here.
//
// Until 2026-09-11 this was a `#[test]`,
// `the_grammars_mix_answers_act_on_the_addressed_deck_and_read_it_off_the_deck`,
// that read this file as text and looked for the statements below as
// literal substrings. It caught the same three claims a run of this
// program actually makes, at the cost every scan in this module pays:
// adding a comment that happened to contain one of these strings, or
// reformatting `holding` so a blank line no longer flattened the way the
// cut expected, failed the test for a reason that had nothing to do with
// any of the three claims.
//
// - **The deck is the one the address named**, and **the reading is the
//   deck's, not a strip's** — both of [`holding`]'s claims — are now
//   `gpu::holding_reads_the_addressed_decks_own_state_and_never_a_strip_that_predates_it`,
//   which presses [`holding`] itself against a deck moved after a frame
//   had already copied its old state, and checks the *values* it hands
//   back rather than the syntax it is spelled with.
//   `gpu::a_mix_key_moves_the_deck_operator_selected_and_leaves_the_others_alone`
//   presses the same two claims for [`gain_key`] and [`opacity_key`], the
//   pair [`answered`] steps for the trim and the fader, the same way.
// - **Nothing here reaches for a strip because nothing here has one to
//   reach for**, which used to be the scan's fourth assertion and is now
//   a fact about the crate graph rather than about this file's text:
//   [`holding`]'s only parameters are `&Deck` and a slot, and
//   `karakuri-engine` does not depend on `karakuri-console` (ADR-0156),
//   so there is no `view::Strip` a function with that signature could
//   name even by mistake. A scan cannot make that claim stronger than the
//   crate graph already does, and does not need to try.
// - **[`held`] guards every read**, and **`answered` builds
//   `Operation::SetGain`/`SetOpacity` naming the addressed deck**, are
//   the two claims this file cannot re-derive behaviourally: [`answered`]
//   takes `&mut Gfx`, which bundles a live `winit::window::Window` and a
//   `wgpu::Surface`, and nothing in this workspace builds one off-screen
//   for a test the way [`Engine`] is built for [`Deck`]-only checks.
//   `gpu::a_mix_key_moves_the_deck_operator_selected_and_leaves_the_others_alone`
//   presses [`held`], [`gain_key`] and [`opacity_key`] by hand, in the
//   same order [`answered`]'s `Trim`/`Fader` arm calls them, and is the
//   nearest a test in this crate gets to entering [`answered`] itself —
//   its own doc says so. The master out, the exposure and the latency
//   offset arms, and the two arms that are not operations at all
//   (`focus::Asked::Panel` and `Routed`, which leave through
//   `Readout::op` and `Readout::sink`), are checked only by their own
//   pure functions' tests (`the_master_out_steps_…`,
//   `the_exposure_steps_…`, `the_offset_steps_…`) and by
//   `karakuri-console`'s own tests of what `Readout::op` and
//   `Readout::sink` do once called — not by anything that presses
//   [`answered`] and watches those five arms run. That gap predates this
//   change: the retired scan read the same five arms' text and could only
//   ever say they were *spelled*, never that they ran, so nothing here
//   is weaker for their sake than it was.

/// And every key the legend prints has its rows written down, both ways round,
/// which is what keeps [`ROWS`] from being a second list of keys rather than a
/// mapping off the first.
#[test]
fn every_key_the_legend_prints_has_its_rows_written_down() {
    let printed: BTreeSet<&str> = KEYS
        .iter()
        .map(|(k, _)| *k)
        // **The four arrow keys are one route**, which is the one place
        // the legend and the routes count differently and it is the
        // grammar's own shape: `up`, `down`, `left` and `right` are four
        // keys an operator presses and *the arrows* is one rule about
        // kinds of thing. The legend prints four sentences; [`ROWS`] holds
        // one entry per bay.
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

/// A route reaching past the page.
///
/// A route this program binds whose row is not marked built in the key column —
/// ADR-0213's failure mode from the side where the code moved first, which is
/// how this whole column came to be wrong: the panel binary was given six
/// arrangement keys and six rows went on reading `gap`.
///
/// The badge is parsed rather than word-matched since 2026-09-10, which is the
/// rewrite ADR-0259 scheduled: a key alone no longer determines a row, so the
/// badge has to be read as *these keys, in that bay* and resolved against the
/// pair.
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
            // **A row is reached by one route on this page**, which is
            // what one badge per row comes to: a row reached both by a
            // letter and by the grammar would need two badges, and the
            // column has one. That is why `f`, `s` and `u` are unbound —
            // see [`crate::KEYS`], where each is named.
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

/// The page claiming a route nothing binds.
///
/// It fails apart from the test above because it is the other failure: that one
/// says the program reached past the specification, this one says the
/// specification tells a player to press a key the instrument does not read. It
/// is the likelier of the two here, because twenty rows carried a built badge
/// for `karakuri-cli`'s keyboard before the column said whose it was.
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
            // **`in any bay` is read as all nine**, which is the stronger
            // claim and the honest one: a badge that says a press works
            // wherever focus is has to be true wherever focus is. The
            // route is written down once, under `ANY`, and this is what
            // holds the page's *any* to the console's.
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
        // **The four keys of the grammar and not every route with a bay in
        // it.** `g` is addressed to the focused bay too — its operand is
        // the bay that has focus — but it is a letter this file binds and
        // not one of the four the console dispatches, so the console
        // declares nothing about it and holding it against that table
        // would be asking the wrong half.
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
