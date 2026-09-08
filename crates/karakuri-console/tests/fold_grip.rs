//! **The two controls that fold the console's own shape from the panel: the
//! grip in a bay head, and a pane's own outer edge.**
//!
//! `docs/manual/operations.html` gives *Fold a bay away* the **bay head** and
//! *Fold a pane away* the **pane edge**, and until these two derivations
//! landed the console painted the first of them and hit-tested neither — so
//! the only route into either row was `f` and `g`. This file is what stands
//! under both, and it is one file rather than two because the two answer one
//! type (`view::FoldGrip`) for one operation (`Op::Fold`): what differs is
//! where the rectangle is, and that is exactly what is measured here.
//!
//! Eight things:
//!
//! 1. Where the grip's target is, as the head's own units put it — the strip
//!    `head_pills` reserves for the mark, `GRIP_W + PILL_GAP` wide and hard
//!    against `HEAD_PAD_X`, grown to `PILL_H` about the head's mid-line.
//! 2. **That exactly the heads the mock draws a grip in have one**, counted
//!    off the console's own table rather than listed here.
//! 3. That it abuts the capsule beside it in the same head and never overlaps
//!    it, so a press on `solo` is `solo`'s and a press on the grip is the
//!    fold's.
//! 4. **What it costs against a boundary's grab**, measured by asking
//!    `Layout::hit` at the target's own corners and stepping down its top edge
//!    — never by doing `view::bay_grip`'s arithmetic a second time. The answer
//!    is `program_head`'s 0.75 of a pixel, which is the same capsule box in
//!    the same head.
//! 5. Where the pane's band is, which edge of the pane it is on, and that the
//!    centre has none — the page's *"folding it is not a thing anybody wants,
//!    and solo is"*, met by a derivation rather than by a sentence.
//! 6. That the band is nobody's boundary in the middle and both boundaries' at
//!    its ends, measured the same way.
//! 7. What the band lies over, in the paddings of the bays under it.
//! 8. That a press on either performs the fold it names, read back off the
//!    layout rather than off the operation.
//!
//! None of it needs a window, a device, a disk or `egui`: neither control is as
//! wide as a word, which is the one thing that separates these two from every
//! other capsule on this console.
//!
//! **What is not here and cannot be**: that `input::claim` gives the panel a
//! press on either of them. That is a row in `input::PROBES` and it is the
//! registration half of these controls, which lives in files this test's author
//! does not own; until it lands, a press here reaches `egui` and the two panel
//! badges on `docs/manual/operations.html` are not yet earned.
//! `tests/keep_pill.rs` says the same sentence about the capsule one bay over,
//! and for the same reason.

mod common;

use common::{id_of, near, rect_of, EPS, PLAUSIBLE, SMALLEST};
use karakuri_console::panel::{Op, Outcome, Panel, GRAB};
use karakuri_console::room::size;
use karakuri_console::view::{
    bay_grip, head_of, pane_edge, region, BAY_GRIPS, FOLDING_PANES, GRIP_W, REGIONS,
};
use karakuri_layout::{Hit, Point, Rect};

/// A panel at `viewport`, solved.
fn console(viewport: Rect) -> Panel {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    panel
}

/// A `karakuri_layout` point, from `egui`'s.
fn at(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

/// Whether a boundary grabs `p` — the same question `input`'s rule 3 asks, in
/// the same terms and off the same call.
fn on_a_boundary(panel: &Panel, p: egui::Pos2) -> bool {
    matches!(panel.layout().hit(at(p), GRAB), Hit::Divider { .. })
}

/// **Every bay head the console draws a grip in**, read off the console's own
/// table rather than written out here — `head_of` is what says which, and a
/// grip drawn in a fifth head arrives in this list without anybody editing it.
fn gripped() -> Vec<&'static str> {
    REGIONS
        .iter()
        .filter(|region| head_of(region).is_some_and(|head| head.grip))
        .map(|region| region.name)
        .collect()
}

// ---------------------------------------------------------------------------
// Where the grip is
// ---------------------------------------------------------------------------

/// **The target is the strip the head reserves for the mark, grown to a line's
/// height.**
///
/// Four numbers and every one of them is the head's own: `HEAD_PAD_X` from the
/// right of the bay, `GRIP_W + PILL_GAP` wide — what `head_pills` steps back
/// before it places a capsule — `PILL_H` tall, and centred on the mid-line of
/// the box `bay_head` paints into, which is `head_box`: the top `HEAD_H` of the
/// bay, so a bay clipped short takes its grip up with it.
#[test]
fn the_target_is_the_strip_the_head_reserves_for_the_mark() {
    for viewport in [SMALLEST, PLAUSIBLE] {
        let panel = console(viewport);
        for name in gripped() {
            let grip = bay_grip(panel.layout(), name)
                .unwrap_or_else(|| panic!("`{name}` draws a grip and has room for its target"));
            let bay = rect_of(panel.layout(), name);
            assert!(
                near(grip.grip.max.x, bay.x + bay.w - size::HEAD_PAD_X),
                "`{name}`'s target ends {} from the right of the bay and `.bay-head`'s padding \
                 is {}",
                bay.x + bay.w - grip.grip.max.x,
                size::HEAD_PAD_X
            );
            assert!(
                near(grip.grip.height(), size::PILL_H),
                "`{name}`'s target is {} tall and a line in this head is {}",
                grip.grip.height(),
                size::PILL_H
            );
            assert!(
                near(grip.grip.width(), GRIP_W + size::PILL_GAP),
                "`{name}`'s target is {} wide and the head reserves {} for the mark — \
                 `head_pills` steps `GRIP_W + PILL_GAP` back before it places a capsule, and \
                 that strip is the whole of what nothing else can be drawn in",
                grip.grip.width(),
                GRIP_W + size::PILL_GAP
            );
            assert!(
                near(grip.grip.center().y, bay.y + size::HEAD_H * 0.5),
                "`{name}`'s target is centred at {} and the head's mid-line is {}",
                grip.grip.center().y,
                bay.y + size::HEAD_H * 0.5
            );
            assert!(
                near(grip.grip.min.y - bay.y, (size::HEAD_H - size::PILL_H) * 0.5),
                "`{name}`'s target starts {} down from the bay's top edge, and a {} capsule \
                 centred in a {} head starts {} down",
                grip.grip.min.y - bay.y,
                size::PILL_H,
                size::HEAD_H,
                (size::HEAD_H - size::PILL_H) * 0.5
            );
        }
    }
}

/// **The heads with a target are the heads the mock draws a mark in**, both
/// ways round, and there are `BAY_GRIPS` of them.
///
/// The count is asserted against the console's own `const` rather than against
/// a four, because that `const` is what `input::PROBES` says this control
/// reaches: a grip drawn in a fifth head has to move both together.
#[test]
fn exactly_the_marked_heads_have_a_target() {
    let panel = console(PLAUSIBLE);
    let gripped = gripped();
    assert_eq!(
        gripped.len(),
        BAY_GRIPS,
        "the console draws a grip in {gripped:?} and `BAY_GRIPS` counts {BAY_GRIPS} — the mark \
         and the count have come apart, and the count is what says how many controls a pointer \
         reaches"
    );
    assert!(
        BAY_GRIPS >= 2,
        "only {BAY_GRIPS} bay heads carry a grip, so this file is measuring almost nothing"
    );
    for region in REGIONS {
        let has = bay_grip(panel.layout(), region.name).is_some();
        assert_eq!(
            has,
            gripped.contains(&region.name),
            "`{}` {} a target and {} the mark — the mark is the control, so a head that has one \
             and not the other is a fold an operator can see and not press, or press and not see",
            region.name,
            match has {
                true => "has",
                false => "has no",
            },
            match gripped.contains(&region.name) {
                true => "draws",
                false => "does not draw",
            }
        );
    }
}

/// **A name the arrangement draws no head for has no target**, which is the
/// heads the mock leaves unmarked said from the other side: the two headless
/// rows, the picture, the preview row and the panes.
#[test]
fn nothing_without_a_head_has_a_target() {
    let panel = console(PLAUSIBLE);
    for name in ["transport", "outputs", "program-view", "deck-previews"] {
        assert_eq!(
            bay_grip(panel.layout(), name),
            None,
            "`{name}` has no bay head (ADR-0159) and answered a fold target anyway"
        );
    }
    for name in FOLDING_PANES {
        assert_eq!(
            bay_grip(panel.layout(), name),
            None,
            "`{name}` is a split with no face of its own and answered a bay's fold target — the \
             pane folds from its edge, which is `pane_edge`"
        );
    }
}

/// **The target abuts the capsule beside it and never overlaps it.**
///
/// The Program bay is the case: `solo` and the class pill are laid out right to
/// left from the same padding this target is measured off, and a target padded
/// like a pill — the mark inside `PILL_PAD_X` either side — would reach six
/// pixels back into that capsule. What is asserted is that the target's left
/// edge is exactly where `head_pills` stops: the head's padding, less the mark
/// and one `PILL_GAP`.
#[test]
fn the_target_abuts_the_capsule_beside_it() {
    for viewport in [SMALLEST, PLAUSIBLE] {
        let panel = console(viewport);
        for name in gripped() {
            let head = head_of(region(name).expect("a region")).expect("a head");
            if head.pills.is_empty() && head.class.is_none() {
                continue;
            }
            let grip = bay_grip(panel.layout(), name).expect("a head with room for its target");
            let bay = rect_of(panel.layout(), name);
            // Where `head_pills` puts the right-hand edge of the rightmost
            // capsule: the head's own padding, less the mark and one gap.
            let capsule_ends = bay.x + bay.w - size::HEAD_PAD_X - GRIP_W - size::PILL_GAP;
            assert!(
                near(grip.grip.min.x, capsule_ends),
                "`{name}`'s target starts at {} and the capsule beside it ends at \
                 {capsule_ends} — less than that and the fold takes a press meant for the \
                 capsule; more and the head has a strip of ground in it that neither control \
                 answers for",
                grip.grip.min.x
            );
        }
    }
}

// ---------------------------------------------------------------------------
// What the grip costs against a boundary's grab
// ---------------------------------------------------------------------------

/// **The target clears every boundary but the one above the bay, and that one
/// takes 0.75 of a pixel** — `program_head`'s number, arrived at again because
/// it is the same capsule box in the same head.
///
/// Measured by asking `Layout::hit` at the target's own corners and stepping
/// down its top edge until the boundary lets go, rather than by doing
/// `bay_grip`'s arithmetic a second time. A bay whose top edge is not a
/// boundary loses nothing at all, and nothing here assumes which is which.
#[test]
fn the_target_gives_a_boundary_the_top_three_quarters_of_a_pixel_and_no_more() {
    /// The most of the target's own height any boundary is allowed to reach,
    /// and it is `program_head`'s: a `HEAD_H` of 27 holding a `PILL_H` of 16.5
    /// leaves 5.25 above it, against a `GRAB` of 6.
    const SLIVER: f32 = 0.75;
    for viewport in [SMALLEST, PLAUSIBLE] {
        let panel = console(viewport);
        for name in gripped() {
            let grip = bay_grip(panel.layout(), name).expect("a head with room for its target");
            let rect = grip.grip;
            // The corners and the middle no boundary may reach at all: the
            // target is `HEAD_PAD_X` in from the bay's right edge against a
            // `GRAB` of 6, and the whole of the bay is below it.
            for probe in [
                rect.left_bottom(),
                rect.right_bottom(),
                rect.center(),
                egui::pos2(rect.min.x, rect.center().y),
                egui::pos2(rect.max.x, rect.center().y),
            ] {
                assert!(
                    !on_a_boundary(&panel, probe),
                    "a boundary grabs {probe:?}, which is on `{name}`'s fold target — the \
                     control is inside a boundary's grab, and `input`'s rule 3 is what would \
                     have to change"
                );
                assert!(
                    grip.hit(at(probe)),
                    "{probe:?} is a corner of `{name}`'s target and the target says it is not \
                     on it"
                );
            }
            // How far down the target the boundary above the bay reaches,
            // asked of the layout rather than computed.
            let mut reached = 0.0f32;
            let mut down = 0.0f32;
            while down <= GRAB {
                if on_a_boundary(&panel, egui::pos2(rect.center().x, rect.min.y + down)) {
                    reached = down;
                }
                down += 0.05;
            }
            // The floor under the number above: every one of these bays has a
            // boundary over it today, so a sliver of nothing at all would mean
            // the walk is measuring somewhere the boundary is not rather than a
            // control that has moved out of its way.
            assert!(
                reached > 0.0,
                "no boundary reaches into `{name}`'s fold target at all, and every gripped bay \
                 on this console has one above it — this walk has stopped measuring"
            );
            assert!(
                reached <= SLIVER + EPS,
                "a boundary reaches {reached} into `{name}`'s fold target, and the head's own \
                 arithmetic allows {SLIVER}: `HEAD_H` {} less `PILL_H` {}, halved, against a \
                 `GRAB` of {GRAB}. The control has moved, the head has got shorter, or the grab \
                 has widened",
                size::HEAD_H,
                size::PILL_H
            );
        }
    }
}

// ---------------------------------------------------------------------------
// What a press on the grip does
// ---------------------------------------------------------------------------

/// **A press folds the bay the grip is in**, performed and read back off the
/// layout — and the control goes with it, which is why there is no unfold on
/// it.
#[test]
fn a_press_on_the_grip_folds_that_bay_and_takes_the_control_with_it() {
    for name in gripped() {
        let mut panel = console(PLAUSIBLE);
        let grip = bay_grip(panel.layout(), name).expect("a head with room for its target");
        let op = grip.op();
        let outcome = panel.op(op);
        panel.solve();
        assert_eq!(
            outcome,
            Outcome::Folded {
                id: grip.id,
                folded: true,
                root: false,
            },
            "a press on `{name}`'s grip asked for `{op:?}` and the panel answered {outcome:?}"
        );
        assert!(
            !panel.layout().visible(grip.id),
            "`{name}` is still on screen after the press its own grip asked for"
        );
        assert_eq!(
            bay_grip(panel.layout(), name),
            None,
            "`{name}` is folded and still answers a fold target — a folded node has no \
             rectangle, and the way back is `z`"
        );
    }
}

/// **A head with no room for the target draws none rather than half of one**,
/// which is `deck_head`'s rule and `head_capsule`'s guard, stated on a control
/// that is one rectangle.
///
/// The viewport is driven under the console's own minimum on purpose: below it
/// the solve stops honouring minima and scales everything down together
/// (ADR-0250), which is the one way to get a bay shorter than its own head
/// without building a rectangle by hand. What is asserted at every size is the
/// invariant — a target that exists is inside the head it is drawn in — and at
/// the smallest of them, that the guard fires rather than clipping a control in
/// half.
#[test]
fn a_head_with_no_room_for_the_target_draws_none() {
    let mut refused = 0usize;
    for (w, h) in [(400.0, 90.0), (200.0, 200.0), (60.0, 60.0)] {
        let panel = console(Rect {
            x: 0.0,
            y: 0.0,
            w,
            h,
        });
        for name in gripped() {
            let bay = rect_of(panel.layout(), name);
            let head = egui::Rect::from_min_max(
                egui::pos2(bay.x, bay.y),
                egui::pos2(bay.x + bay.w, (bay.y + size::HEAD_H).min(bay.y + bay.h)),
            );
            match bay_grip(panel.layout(), name) {
                Some(grip) => assert!(
                    head.contains_rect(grip.grip),
                    "at {w}x{h} `{name}`'s target is {:?} and its head is {head:?} — a target \
                     hanging out of the head is a control the paint clips in half",
                    grip.grip
                ),
                None => refused += 1,
            }
        }
    }
    assert!(
        refused > 0,
        "no bay refused a target at any of these viewports, so the guard that keeps a clipped \
         control off the console is not being exercised here"
    );
}

// ---------------------------------------------------------------------------
// Where the pane's band is
// ---------------------------------------------------------------------------

/// **The band is `GRAB` deep on the pane's outer edge and the whole height of
/// the pane**, and which edge that is is read off the row rather than listed.
#[test]
fn the_band_is_on_the_panes_own_outer_edge() {
    for viewport in [SMALLEST, PLAUSIBLE] {
        let panel = console(viewport);
        for name in FOLDING_PANES {
            let band = pane_edge(panel.layout(), name)
                .unwrap_or_else(|| panic!("`{name}` is laid out and folds from its own edge"));
            let pane = rect_of(panel.layout(), name);
            assert!(
                pane.w > GRAB * 4.0,
                "`{name}` is {} wide, which is too narrow for this to be measuring an edge",
                pane.w
            );
            assert!(
                near(band.grip.width(), GRAB),
                "`{name}`'s band is {} deep and a hand finds an edge {GRAB} from it",
                band.grip.width()
            );
            assert!(
                near(band.grip.height(), pane.h) && near(band.grip.min.y, pane.y),
                "`{name}`'s band is {} tall over a pane of {} — the edge is the whole of the \
                 pane's side",
                band.grip.height(),
                pane.h
            );
            let outer = match name {
                "left-pane" => near(band.grip.min.x, pane.x),
                _ => near(band.grip.max.x, pane.x + pane.w),
            };
            assert!(
                outer,
                "`{name}`'s band is at {:?} and the pane is at {pane:?} — the band is on the \
                 edge the centre is behind rather than on the pane's own outer edge",
                band.grip
            );
            assert_eq!(band.id, id_of(panel.layout(), name));
        }
    }
}

/// **The centre has no band, and nothing else does either.**
///
/// `docs/manual/console.html`: *"The middle one is not a third pane on purpose:
/// folding it is not a thing anybody wants, and solo is."* The derivation reads
/// a list of two rather than *whatever is at the end of the row* for exactly
/// this reason — fold the left pane and the centre inherits its outer edge.
#[test]
fn only_the_two_panes_the_page_names_have_a_band() {
    let mut panel = console(PLAUSIBLE);
    for name in ["centre", "library", "mixer", "program", "transport"] {
        assert_eq!(
            pane_edge(panel.layout(), name),
            None,
            "`{name}` answered a pane's fold band, and the page names two panes"
        );
    }
    let left = id_of(panel.layout(), "left-pane");
    panel.op(Op::Fold(left));
    panel.solve();
    assert_eq!(
        pane_edge(panel.layout(), "centre"),
        None,
        "with the left pane folded the centre is the first thing in the body row and answered a \
         fold band — an edge is not what makes a pane, and the centre does not fold"
    );
    assert_eq!(
        pane_edge(panel.layout(), "left-pane"),
        None,
        "a folded left pane still answers a band, and a folded node has no rectangle"
    );
}

/// **The band is nobody's boundary in the middle and both boundaries' at its
/// ends**, measured by asking the layout.
///
/// The pane's outer edge is the window's own, so nothing is drawn there and no
/// divider is either — which is the whole of why the band can exist at all. Up
/// and down it runs into the two boundaries of the row it is in and gives each
/// of them its `GRAB`, which is rule 3's ordinary price.
#[test]
fn the_band_is_not_a_boundary_except_where_the_row_ends() {
    let panel = console(PLAUSIBLE);
    for name in FOLDING_PANES {
        let band = pane_edge(panel.layout(), name)
            .expect("a laid-out pane")
            .grip;
        assert!(
            !on_a_boundary(&panel, band.center()),
            "a boundary grabs the middle of `{name}`'s band, so rule 3 takes the press and the \
             pane cannot be folded from its edge at all"
        );
        for probe in [
            egui::pos2(band.center().x, band.min.y + GRAB + 1.0),
            egui::pos2(band.center().x, band.max.y - GRAB - 1.0),
        ] {
            assert!(
                !on_a_boundary(&panel, probe),
                "a boundary grabs {probe:?}, which is more than {GRAB} inside `{name}`'s band"
            );
        }
        for probe in [
            egui::pos2(band.center().x, band.min.y),
            egui::pos2(band.center().x, band.max.y),
        ] {
            assert!(
                on_a_boundary(&panel, probe),
                "{probe:?} is the end of `{name}`'s band and no boundary grabs it — the row's \
                 own boundary has moved, and the price this band pays is not what it was"
            );
        }
    }
}

/// **What the band lies over, in the paddings of the bays under it.**
///
/// It is not a capsule in a row of its own: it lies over whatever the pane's
/// bays draw at their outer edge. Everything in both panes clears it except one
/// thing, and the one thing is written down here rather than discovered —
/// `.lib-list`'s padding is 3, so the outer 3 pixels of a library row are under
/// the band, and the row wins because this control is asked last.
#[test]
fn everything_in_the_panes_clears_the_band_except_a_library_row() {
    for (what, pad) in [
        ("the Library bay's scope chips", size::SCOPES_PAD_X),
        ("the Library bay's filter fields", size::LIB_FILTERS_PAD_X),
        ("a mixer strip", size::STRIPS_PAD),
        ("the transition row", size::XFADE_PAD_X),
        ("the Master bay's out", size::MASTER_PAD_X),
    ] {
        assert!(
            pad >= GRAB,
            "{what} is {pad} in from its bay's edge against a band of {GRAB} — a control inside \
             the band is a control the fold takes a press from, and the ordering that saves the \
             library's rows would have to save this one too"
        );
    }
    assert!(
        size::LIB_LIST_PAD < GRAB,
        "`.lib-list`'s padding is {} and the band is {GRAB} — a library row now clears the band, \
         so the ordering `view::pane_edge` documents is buying nothing and should go",
        size::LIB_LIST_PAD
    );
}

/// **A press folds the pane the band is on**, performed and read back — and the
/// centre takes the width, which is what the row on the page promises.
#[test]
fn a_press_on_the_band_folds_that_pane_and_the_centre_takes_the_width() {
    for name in FOLDING_PANES {
        let mut panel = console(PLAUSIBLE);
        let was = rect_of(panel.layout(), "centre").w;
        let band = pane_edge(panel.layout(), name).expect("a laid-out pane");
        let op = band.op();
        let outcome = panel.op(op);
        panel.solve();
        assert_eq!(
            outcome,
            Outcome::Folded {
                id: band.id,
                folded: true,
                root: false,
            },
            "a press on `{name}`'s edge asked for `{op:?}` and the panel answered {outcome:?}"
        );
        assert!(
            !panel.layout().visible(band.id),
            "`{name}` is still on screen after the press its own edge asked for"
        );
        let now = rect_of(panel.layout(), "centre").w;
        assert!(
            now > was,
            "`{name}` folded and the centre went from {was} to {now} — the row says the centre \
             takes the width"
        );
    }
}
