//! Bay head pill contents and arming state consistency checks across all bay heads.

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

/// Verifies each head contains only its designated table entry and class pill in order.
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

/// Verifies that the class pill's display word and armed state bit remain consistent.
#[test]
fn the_class_pills_word_and_its_bit_are_one_answer() {
    for class in Class::ALL {
        // Three of the four classes reside in a bay head; Outputs is headless (ADR-0159).
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

/// A head with `building = true` draws a "building" pill, which is armed.
#[test]
fn a_head_with_building_draws_building_badge() {
    let master = head_of(region("master").expect("the Master region")).expect("a head");
    let head = master.with_building(true);
    let words = head.words(Open::CLOSED);
    assert_eq!(
        words.as_slice(),
        ["building", "mcp · off"],
        "building badge appears in master head to the left of the class pill"
    );
    assert!(words.armed(0), "building badge is lit/armed");
    assert!(!words.armed(1), "closed class is not lit");

    let words_open = head.words(everything_open());
    assert_eq!(
        words_open.as_slice(),
        ["building", "mcp · on"],
        "building badge preserved with class open"
    );
    assert!(words_open.armed(0), "building badge is armed");
    assert!(words_open.armed(1), "open class is armed");
}

/// Every class open at once — the state no run starts in and the one this file
/// wants, because a head is only interesting under both.
fn everything_open() -> Open {
    Class::ALL
        .iter()
        .fold(Open::CLOSED, |open, class| open.with(*class, true))
}
