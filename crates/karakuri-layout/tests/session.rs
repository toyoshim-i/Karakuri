//! One long scripted session against the console's arrangement.
//!
//! The tests beside this one each put the layout in one state and check one
//! thing about it. This drives it through several hundred states nobody chose,
//! and checks everything after every step — because the failures that survive a
//! hand-written test are the ones that need a particular viewport to arrive
//! while a particular pane happens to be folded.

mod common;

use common::{assert_invariants, console, console_ids, rects};
use karakuri_layout::{Axis, NodeId, Rect};

/// A named sequence rather than a random one: a failure here is reproduced by
/// running it again, and reduced by shortening it.
struct Script(u32);

impl Script {
    fn next(&mut self) -> u32 {
        // A small LCG. Nothing here needs statistical quality; it needs to be
        // the same sequence on every machine, which `rand` would not be.
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0 >> 8
    }

    fn pick<T: Copy>(&mut self, from: &[T]) -> T {
        from[self.next() as usize % from.len()]
    }

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (self.next() % 1000) as f32 * (hi - lo) / 1000.0
    }
}

#[test]
fn a_long_session_of_resizes_drags_and_collapses_holds_every_invariant() {
    let mut l = console();
    let ids = console_ids(&l);
    let splits = [l.root(), ids.panes, ids.left, ids.centre, ids.right];
    let folds: Vec<NodeId> = [
        "library",
        "staging",
        "program",
        "inspector",
        "mixer",
        "master",
    ]
    .iter()
    .map(|n| l.find(n).unwrap())
    .chain([ids.left, ids.centre, ids.right])
    .collect();
    let solos: Vec<NodeId> = ["program", "mixer", "library"]
        .iter()
        .map(|n| l.find(n).unwrap())
        .collect();

    let mut s = Script(0x5eed_1a70);
    // A scripted sequence is only evidence if it reaches the states it claims
    // to: 400 steps spent resizing would say nothing about a drag.
    let mut performed = [0usize; 4];
    let mut viewport = Rect::new(0.0, 0.0, 1280.0, 720.0);
    l.set_viewport(viewport);
    l.solve();

    for step in 0..400 {
        match s.next() % 10 {
            0..=3 => {
                performed[0] += 1;
                // Every size, including the ones nothing fits in.
                viewport = Rect::new(
                    s.range(-20.0, 60.0),
                    s.range(-20.0, 60.0),
                    s.range(0.0, 2600.0),
                    s.range(0.0, 1600.0),
                );
                l.set_viewport(viewport);
            }
            4..=6 => {
                performed[1] += 1;
                let split = s.pick(&splits);
                let index = (s.next() % 3) as usize;
                let axis = l.axis(split).unwrap();
                let r = l.rect(split);
                let (o, e) = match axis {
                    Axis::Row => (r.x, r.w),
                    Axis::Column => (r.y, r.h),
                };
                // Aimed well past both ends as often as inside, since a drag
                // that runs off the end is the one an operator's hand makes.
                l.set_divider(split, index, s.range(o - e, o + 2.0 * e));
            }
            7..=8 => {
                performed[2] += 1;
                l.toggle(s.pick(&folds));
            }
            _ => {
                performed[3] += 1;
                if l.is_soloed() {
                    l.unsolo();
                } else {
                    l.solo(s.pick(&solos));
                }
            }
        }

        l.solve();
        assert_invariants(&l);

        // Every so often, take the viewport away entirely and give it back.
        // Nothing about the arrangement may notice.
        if step % 37 == 0 {
            let before = rects(&l);
            for probe in [(0.0, 0.0), (3.0, 900.0), (2400.0, 2.0)] {
                l.set_viewport(Rect::new(viewport.x, viewport.y, probe.0, probe.1));
                l.solve();
                assert_invariants(&l);
            }
            l.set_viewport(viewport);
            l.solve();
            assert_eq!(rects(&l), before, "step {step} did not come back");
        }
    }

    // It ends somewhere legible rather than wedged: open everything and the
    // arrangement is still an arrangement.
    l.unsolo();
    for f in &folds {
        l.expand(*f);
    }
    l.set_viewport(Rect::new(0.0, 0.0, 1280.0, 720.0));
    l.solve();
    assert_invariants(&l);
    let program = l.find("program").unwrap();
    assert!(l.visible(program) && l.rect(program).w > 0.0 && l.rect(program).h > 0.0);

    for (kind, n) in performed.iter().enumerate() {
        assert!(
            *n > 20,
            "operation kind {kind} came up only {n} times in 400"
        );
    }
}
