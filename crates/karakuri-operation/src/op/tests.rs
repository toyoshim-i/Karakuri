use super::*;

/// The macro is what keeps [`Operation::TITLES`] and the variants in step, and
/// this is the one property of it worth asserting on its own: a title reached
/// through a value is the same string the list holds.
#[test]
fn a_title_is_the_one_in_the_list() {
    let op = Operation::SetGain { deck: 0, gain: 1.0 };
    assert_eq!(op.title(), "Gain");
    assert!(Operation::TITLES.contains(&op.title()));
    assert_eq!(Operation::Quit.title(), "Quit");
}

/// Titles are the key the manual is matched on, so two variants sharing one
/// would let a missing operation pass the cross-check: the duplicate would
/// answer for the row and nothing would say the second variant was never
/// specified.
#[test]
fn no_two_operations_share_a_title() {
    let mut seen: Vec<&str> = Vec::new();
    for title in Operation::TITLES {
        assert!(
            !seen.contains(title),
            "`{title}` is the title of two variants of `Operation` — the manual is \
             matched by title, so the second one would never be checked against a row"
        );
        seen.push(title);
    }
}

/// Verifies that the vocabulary count meets or exceeds the minimum floor.
#[test]
fn the_vocabulary_is_not_empty() {
    assert!(
        Operation::TITLES.len() >= 55,
        "only {} operations named — the vocabulary has shrunk below what the manual \
         specifies",
        Operation::TITLES.len()
    );
}
