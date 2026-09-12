//! What every bay head holds, and that only one of them moved.
//!
//! The Sequencer's head gained four capsules on 2026-09-09 — `seq 1 … seq 4`,
//! the armed one drawn `.pill.armed` — and the head machinery gained the field
//! that carries them (`view::Head::banks`) and the bit that says which is lit
//! (`view::HeadWords::armed`). Both are shared: every head on the console is
//! laid out by `head_pills` off `Head::words`, so a change there is a change to
//! seven heads.
//!
//! This is the test that says the other six did not move. It states each head's
//! capsules as data — the table's own controls, then the class pill — and it
//! states the arming rule for both kinds: a class pill is lit when its class is
//! open, a bank pill when it is the armed bank, and nothing else is lit ever. A
//! head handed banks it should not have, an extra capsule, or a pill armed on
//! the wrong question fails here rather than in a screenshot.
//!
//! The pixel-level tests of the same heads are `tests/mcp_pill.rs`,
//! `tests/solo_pill.rs` and `tests/fold_grip.rs`, which were not touched by
//! that change.

use karakuri_console::view::{class_at, head_of, mcp_word, region, Head, REGIONS};
use karakuri_operation::gate::{Class, Open};
use karakuri_pattern::BANKS;

/// Only the Sequencer's head is ever handed banks, and it is handed them by a
/// caller rather than by the table — `head_of` answers `None` for every region,
/// which is what makes *the armed bank is a value* true of the head machinery
/// and not just of that bay.
#[test]
fn the_table_hands_no_head_any_banks() {
    for region in REGIONS {
        let Some(head) = head_of(region) else {
            continue;
        };
        assert_eq!(
            head.banks, None,
            "`head_of` gave {}'s head bank pills — the table carries words a region was written \
             with, and which bank is armed is a value the host writes per frame",
            region.name
        );
    }
}

/// Every head's capsules are its table entry and its class pill, in that order,
/// and nothing else — under a shut opening and under an open one.
///
/// This is the *before* of *before and after*: it is what a head held before
/// the Sequencer's four pills existed, written down so that a head that starts
/// holding something else says so.
#[test]
fn every_heads_words_are_its_own_controls_and_its_class_pill() {
    for open in [Open::CLOSED, everything_open()] {
        for region in REGIONS {
            let Some(head) = head_of(region) else {
                continue;
            };
            let mut want: Vec<&str> = head.pills.to_vec();
            if let Some(class) = head.class {
                want.push(mcp_word(open.holds(class)));
            }
            let words = head.words(open);
            assert_eq!(words.as_slice(), want.as_slice(), "{}'s head", region.name);
            for index in 0..want.len() {
                let lit = head.class.is_some()
                    && index + 1 == want.len()
                    && open.holds(head.class.unwrap());
                assert_eq!(
                    words.armed(index),
                    lit,
                    "{}'s capsule {index} — the only capsule a head lights without banks is an \
                     open class pill",
                    region.name
                );
            }
        }
    }
}

/// The class pill's arming is what it always was: the word says it and the bit
/// says it, and the two agree. `bay_head` read the word until 2026-09-09 and
/// reads the bit now, so this is the clause that says the swap changed nothing.
#[test]
fn the_class_pills_word_and_its_bit_are_one_answer() {
    for class in Class::ALL {
        // **Three of the four sit in a bay head and the fourth does not** —
        // the Outputs row is headless (ADR-0159) and its pill comes out of
        // `outputs` instead, which is `mcp_pill`'s own division.
        let Some(head) = head_of(region(karakuri_console::view::opens(*class)).expect("a region"))
        else {
            continue;
        };
        for open in [Open::CLOSED, Open::CLOSED.with(*class, true)] {
            let words = head.words(open);
            let last = words.as_slice().len() - 1;
            assert_eq!(words.as_slice()[last], mcp_word(open.holds(*class)));
            assert_eq!(
                words.armed(last),
                open.holds(*class),
                "the bit `bay_head` now paints from and the word it painted from before"
            );
        }
    }
}

/// A head handed banks draws four more capsules and lights exactly one — the
/// armed one, whichever it is, and the words count from one.
#[test]
fn a_head_with_banks_draws_four_and_lights_the_armed_one() {
    let sequencer = head_of(region("sequencer").expect("the Sequencer region")).expect("a head");
    for armed in 0..BANKS {
        let head: Head = sequencer.with_banks(armed);
        let words = head.words(Open::CLOSED);
        assert_eq!(
            words.as_slice(),
            ["seq 1", "seq 2", "seq 3", "seq 4"],
            "four fixed banks, counted from one"
        );
        for bank in 0..BANKS {
            assert_eq!(
                words.armed(bank),
                bank == armed,
                "bank {bank} with bank {armed} armed — one pill lit and never two"
            );
        }
    }
}

/// The Sequencer's head opens no class, which is what lets the bay lay its own
/// pills out without being told which classes are open: nothing in that head
/// moves with an opening.
#[test]
fn the_sequencers_head_opens_no_class() {
    assert_eq!(class_at("sequencer"), None);
    let head = head_of(region("sequencer").expect("the Sequencer region")).expect("a head");
    assert_eq!(head.class, None);
    assert_eq!(
        head.with_banks(1).words(Open::CLOSED).as_slice(),
        head.with_banks(1).words(everything_open()).as_slice(),
        "an opening moves no capsule in a head that opens no class"
    );
}

/// Every class open at once — the state no run starts in and the one this file
/// wants, because a head is only interesting under both.
fn everything_open() -> Open {
    Class::ALL
        .iter()
        .fold(Open::CLOSED, |open, class| open.with(*class, true))
}
