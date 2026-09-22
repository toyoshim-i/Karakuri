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

/// A floor, not a count: the point is that the list cannot come back empty. The
/// exact number is the manual's to state and is asserted against the page
/// itself in `tests/`. It read 46 when this landed, 45 once two residency rows
/// became one (ADR-0186), 46 again since the look split into a tone map and an
/// exposure (ADR-0192), and 48 since the mask took a row for its shape and a
/// row for its position (ADR-0201); it moves with the page and is never lowered
/// to make a shorter list pass. It was 49 once the arrangement gained a reset
/// (ADR-0208), 50 since a node gained an authority (ADR-0211), 52 since the
/// staging lane gained a keep and a put-back, 54 since the arrangement's reset
/// stopped being the only member of its family (ADR-0221), and is 55 since the
/// master out became a level something can name (ADR-0224).
#[test]
fn the_vocabulary_is_not_empty() {
    assert!(
        Operation::TITLES.len() >= 55,
        "only {} operations named — the vocabulary has shrunk below what the manual \
         specifies",
        Operation::TITLES.len()
    );
}
