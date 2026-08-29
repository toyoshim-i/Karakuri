//! The arrangement survives being written down and read back.
//!
//! Saving an operator's panel is `serde` on the whole `Layout`, so the
//! question this answers is about *this* arrangement rather than about the
//! format: every number in it has to be one the wire can carry. An unbounded
//! maximum is the interesting case, and this arrangement is full of them — six
//! regions say "no maximum" on purpose, because ADR-0157 makes a maximum
//! something a solo would be held to.

mod common;

use common::{assert_sane, names, rects, solved, EPS, PLAUSIBLE, SMALLEST};
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

/// **A saved arrangement comes back into the window you are looking at, not
/// the one it was saved in.**
///
/// This is the panel's half of putting a saved arrangement back, and the whole
/// of what it decides: `Panel::restore` is `Op::Reset`'s arm with the
/// arrangement handed in, so the folds and the solo are the file's and the
/// viewport is the running window's. A restore that took the file's viewport
/// too would open a console arranged on a 1920x1080 desktop at that size
/// inside a smaller window — every rectangle laid out past the edge, and the
/// solve never asked to fit them.
///
/// Read as a pair with `crates/karakuri/src/main.rs`, which is where the name
/// becomes these bytes: this crate has no store and never sees the name
/// (ADR-0156, ADR-0221).
#[test]
fn a_restored_arrangement_keeps_its_folds_and_takes_the_window_it_arrives_in() {
    use karakuri_console::panel::{Op, Outcome, Panel};

    // Saved on the desktop, with something folded and something soloed — the
    // two things an arrangement is for and the two a reset forgets.
    let mut saved = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    let staging = saved.layout().find("staging").expect("a staging bay");
    saved.op(Op::Fold(staging));
    let mixer = saved.layout().find("mixer").expect("a mixer bay");
    saved.op(Op::Solo(mixer));
    saved.solve();
    let text = serde_json::to_string(saved.layout()).expect("the arrangement serialises");

    // Put back into the smallest window the arrangement is claimed to work at,
    // which is a different one in both axes.
    let mut window = Panel::new(SMALLEST.w, SMALLEST.h);
    let layout: Layout = serde_json::from_str(&text).expect("and reads back");
    assert_eq!(window.restore(layout), Outcome::Restored);

    let now = window.layout();
    assert_eq!(
        now.viewport(),
        SMALLEST,
        "a restored arrangement brought the viewport it was saved at with it — the window is \
         the operator's and never the file's, so a console arranged on a desktop would open \
         past the edges of the laptop it was put back on"
    );
    assert!(
        now.is_soloed(),
        "the solo did not survive being put back, and a solo is one of the two things an \
         arrangement is kept for"
    );
    assert!(
        now.is_collapsed(now.find("staging").expect("staging is still a region")),
        "the fold did not survive being put back"
    );
    // And it is a solved arrangement in the new window rather than the old
    // one's rectangles carried over: everything is inside the viewport it
    // arrived in.
    assert_sane(now);
    assert!(
        rects(now)
            .iter()
            .all(|r| r.x >= -EPS && r.y >= -EPS && r.w <= SMALLEST.w + EPS),
        "a region was laid out past the edge of the window it was restored into"
    );
}
