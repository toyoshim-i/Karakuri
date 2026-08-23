//! The arrangement survives being written down and read back.
//!
//! Saving an operator's panel is `serde` on the whole `Layout`, so the
//! question this answers is about *this* arrangement rather than about the
//! format: every number in it has to be one the wire can carry. An unbounded
//! maximum is the interesting case, and this arrangement is full of them — six
//! regions say "no maximum" on purpose, because ADR-0157 makes a maximum
//! something a solo would be held to.

mod common;

use common::{assert_sane, names, rects, solved, PLAUSIBLE};
use karakuri_layout::Layout;

#[test]
fn the_arrangement_survives_a_serde_round_trip() {
    let before = solved(PLAUSIBLE);

    let text = serde_json::to_string(&before).expect("the arrangement serialises");
    let after: Layout = serde_json::from_str(&text).expect("and reads back");

    assert_eq!(names(&before), names(&after));
    assert_eq!(rects(&before), rects(&after));
    assert_eq!(before.viewport(), after.viewport());
    assert_sane(&after);

    // Every bound survives too, unbounded maxima included — a `max` written as
    // an infinity has no spelling in JSON, and losing one is exactly the kind
    // of loss that would show up later as a soloed program with a margin.
    let mut before_bounds = Vec::new();
    let mut after_bounds = Vec::new();
    common::walk(&before, before.root(), &mut |id| {
        before_bounds.push(before.bounds(id))
    });
    common::walk(&after, after.root(), &mut |id| {
        after_bounds.push(after.bounds(id))
    });
    assert_eq!(before_bounds, after_bounds);
}

/// A panel saved while soloed comes back soloed, and one `unsolo` on the
/// loaded copy returns the arrangement that was saved under it. This is the
/// path the manual's *"how you capture this window"* actually takes — the
/// session is written out while the picture is filling the screen.
#[test]
fn a_solo_survives_the_round_trip_and_still_undoes() {
    let mut before = solved(PLAUSIBLE);
    let program = before.find("program").unwrap();
    before.solo(program);
    before.solve();

    let text = serde_json::to_string(&before).expect("the arrangement serialises");
    let mut after: Layout = serde_json::from_str(&text).expect("and reads back");

    assert!(after.is_soloed());
    assert_eq!(after.rect(after.find("program").unwrap()), PLAUSIBLE);

    after.unsolo();
    after.solve();
    assert_eq!(rects(&after), rects(&solved(PLAUSIBLE)));
}
