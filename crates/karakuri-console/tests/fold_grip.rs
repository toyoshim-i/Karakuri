//! The two ways the console's own shape folds from the panel: the grip in a bay
//! head, and a pane's own boundary dragged out through its edge.
//!
//! `docs/manual/operations.html` gives *Fold a bay away* the bay head and *Fold
//! a pane away* the pane edge, and until these landed the console painted the
//! first of them and hit-tested neither — so the only route into either row was
//! `f` and `g`.
//!
//! The two halves are no longer the same shape, and that is
//! [ADR-0300](../../../docs/adr/0300-a-pane-folds-by-dragging-its-boundary-out-and-comes-back-by-dragging-it-in.md).
//! The bay's fold is a rectangle: `view::bay_grip` answers a `view::FoldGrip`
//! over the mark the head already draws, and a press on it asks for `Op::Fold`.
//! The pane's is a gesture: its boundary dragged out past the pane's own
//! minimum closes it, and the divider the closed pane keeps at the window's
//! edge is what brings it back. ADR-0295 gave the pane a rectangle too — a band
//! on its outer edge — and that band lay over the outer three pixels of every
//! Library row, which is why it was never registered. There is no band now:
//! while the pane is open there is nothing at the window's edge at all.
//!
//! Eight things:
//!
//! 1. Where the grip's target is, as the head's own units put it — the strip
//! `head_pills` reserves for the mark, `GRIP_W + PILL_GAP` wide and hard
//! against `HEAD_PAD_X`, grown to `PILL_H` about the head's mid-line. 2. That
//! exactly the heads the mock draws a grip in have one, counted off the
//! console's own table rather than listed here. 3. That it abuts the capsule
//! beside it in the same head and never overlaps it, so a press on `solo` is
//! `solo`'s and a press on the grip is the fold's. 4. What it costs against a
//! boundary's grab, measured by asking `Layout::hit` at the target's own
//! corners and stepping down its top edge — never by doing `view::bay_grip`'s
//! arithmetic a second time. The answer is `program_head`'s 0.75 of a pixel,
//! which is the same capsule box in the same head. 5. Which regions fold to
//! their edge, read off the arrangement — the two panes the page names and
//! nothing else, the centre included: *"folding it is not a thing anybody
//! wants, and solo is."* 6. That a closed pane keeps a boundary at the window's
//! edge, and that while it is open there is none — which is the whole of why
//! this control can exist where ADR-0295's band could not. 7. That a drag
//! closes the pane and a drag brings it back, through `Panel::press`, `moved`
//! and `released`, with the reopened pane at its declared minimum and `z` still
//! holding the width it had. 8. That a press on the grip performs the fold it
//! names, read back off the layout rather than off the operation.
//!
//! None of it needs a window, a device, a disk or `egui`: neither route is as
//! wide as a word, which is the one thing that separates these from every other
//! capsule on this console.
//!
//! What is not here and cannot be: that `input::claim` gives the panel a press
//! on the grip. That is a row in `input::PROBES` and it is the registration
//! half of that control, which lives in files this test's author does not own;
//! until it lands, a press there reaches `egui` and *Fold a bay away*'s panel
//! badge is not yet earned. `tests/keep_pill.rs` says the same sentence about
//! the capsule one bay over, and for the same reason. The pane's half owes
//! nothing: a boundary is claimed by `input`'s rule 3 before any control is
//! asked, and the window loop already routes a boundary drag into `Panel`.

mod common;

use common::{id_of, near, rect_of, EPS, PLAUSIBLE, SMALLEST};
use karakuri_console::panel::{Dragged, Op, Outcome, Panel, Pressed, GRAB};
use karakuri_console::room::size;
use karakuri_console::view::{bay_grip, head_of, region, BAY_GRIPS, GRIP_W, REGIONS};
use karakuri_layout::{Axis, Hit, NodeId, Point, Rect};

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

/// Every region whose fold leaves its edge behind, walked off the arrangement
/// rather than listed here — `karakuri_console::arrangement` is the one
/// statement of which, and a third pane declaring it arrives in this list
/// without anybody editing it.
fn folds_to_its_edge(panel: &Panel) -> Vec<String> {
    let layout = panel.layout();
    panel
        .nodes()
        .iter()
        .filter(|node| layout.keeps_its_edge(node.id))
        .map(|node| {
            layout
                .name(node.id)
                .expect("a region that folds to its edge is one an operator names")
                .to_owned()
        })
        .collect()
}

/// Every bay head the console draws a grip in, read off the console's own table
/// rather than written out here — `head_of` is what says which, and a grip
/// drawn in a fifth head arrives in this list without anybody editing it.
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

/// The target is the strip the head reserves for the mark, grown to a line's
/// height.
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

/// The heads with a target are the heads the mock draws a mark in, both ways
/// round, and there are `BAY_GRIPS` of them.
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
    // **The guard is on what the walk found rather than on the constant it was
    // just matched against.** The two are one number by the assertion above, so
    // nothing is given up by asking the walk — and asserting the `const` was
    // asserting a literal: `BAY_GRIPS >= 2` is decided where it is written and
    // says nothing about this run, which is what clippy's
    // `assertions_on_constants` is for. `gripped` is read off `REGIONS` and
    // `head_of` every time, so this is the premise the loop below actually
    // needs: that there are marked heads to walk at all.
    assert!(
        gripped.len() >= 2,
        "the walk found a grip in {gripped:?} and `BAY_GRIPS` counts {BAY_GRIPS} — with fewer \
         than two marked heads this file is measuring almost nothing"
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

/// A name the arrangement draws no head for has no target, which is the heads
/// the mock leaves unmarked said from the other side: the two headless rows,
/// the picture, the preview row and the panes.
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
    for name in folds_to_its_edge(&panel) {
        assert_eq!(
            bay_grip(panel.layout(), &name),
            None,
            "`{name}` is a split with no face of its own and answered a bay's fold target — a \
             pane folds by its own boundary and not by a rectangle"
        );
    }
}

/// The target abuts the capsule beside it and never overlaps it.
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

/// The target clears every boundary but the one above the bay, and that one
/// takes 0.75 of a pixel — `program_head`'s number, arrived at again because it
/// is the same capsule box in the same head.
///
/// Measured by asking `Layout::hit` at the target's own corners and stepping
/// down its top edge until the boundary lets go, rather than by doing
/// `bay_grip`'s arithmetic a second time. A bay whose top edge is not a
/// boundary loses nothing at all, and nothing here assumes which is which.
#[test]
fn the_target_gives_a_boundary_the_top_three_quarters_of_a_pixel_and_no_more() {
    /// The most of the target's own height any boundary is allowed to reach, and it
    /// is `program_head`'s: a `HEAD_H` of 27 holding a `PILL_H` of 16.5 leaves 5.25
    /// above it, against a `GRAB` of 6.
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

/// A press folds the bay the grip is in, performed and read back off the layout
/// — and the control goes with it, which is why there is no unfold on it.
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

/// A head with no room for the target draws none rather than half of one, which
/// is `deck_head`'s rule and `head_capsule`'s guard, stated on a control that
/// is one rectangle.
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
// The pane's own edge, which is a boundary and a gesture rather than a shape
// ---------------------------------------------------------------------------

/// The body row, the index of the boundary that has `name` on one side of it,
/// and whether the pane is the near side — so a test knows which way *out* is.
fn edge_of(panel: &Panel, name: &str) -> (NodeId, usize, bool) {
    let layout = panel.layout();
    let id = id_of(layout, name);
    let row = layout.parent(id).expect("a pane is in a row");
    for (split, index) in layout.boundaries() {
        if split != row {
            continue;
        }
        let (a, b) = panel.pair(split, index).expect("a boundary has a pair");
        if a == id {
            return (row, index, true);
        }
        if b == id {
            return (row, index, false);
        }
    }
    panic!("`{name}` has no boundary beside it, and every pane in this row does")
}

/// Take hold of a boundary in the middle of its own gap, half way down the pane
/// beside it.
fn take_hold(panel: &mut Panel, split: NodeId, index: usize) -> Point {
    panel.solve();
    let gap = panel
        .layout()
        .boundary(split, index)
        .expect("a boundary with a pair has a gap");
    let at = Point::new(gap.x + gap.w * 0.5, gap.y + gap.h * 0.5);
    assert!(
        matches!(panel.press(at), Pressed::Grabbed { .. }),
        "a press at {at:?} — the middle of boundary {index}'s own gap — did not take it in hand"
    );
    at
}

/// Exactly the two panes the page names fold to their edge, read off the
/// arrangement.
///
/// `docs/manual/console.html`: *"a left pane and a right pane, which fold away
/// to give room, and the centre, which is what they give it to … The middle one
/// is not a third pane on purpose: folding it is not a thing anybody wants, and
/// solo is."* The centre is the case that matters: fold the left pane and the
/// centre inherits the window's edge, so a rule reading the edge alone would
/// hand an operator the fold the page has just refused.
#[test]
fn only_the_two_panes_the_page_names_fold_to_their_edge() {
    let panel = console(PLAUSIBLE);
    assert_eq!(
        folds_to_its_edge(&panel),
        vec!["left-pane".to_owned(), "right-pane".to_owned()],
        "the regions that keep an edge when they fold are not the two panes the page names"
    );
}

/// An open pane has no boundary at the window's edge, and a closed one does —
/// which is the whole of why this control can exist where ADR-0295's band could
/// not.
///
/// The band that record chose was `GRAB` deep on the pane's outer edge *while
/// the pane was open*, so it lay over the outer three pixels of every Library
/// row (`.lib-list`'s padding is 3). Here there is nothing at the window's edge
/// until the pane is folded, and by then the pane draws nothing at all.
#[test]
fn the_window_edge_is_a_boundary_only_while_the_pane_is_closed() {
    for name in ["left-pane", "right-pane"] {
        let mut panel = console(PLAUSIBLE);
        let viewport = panel.layout().viewport();
        let pane = rect_of(panel.layout(), name);
        // The window's own edge on the pane's outer side, and one `GRAB` in
        // from it — two of the coordinates ADR-0295's band covered.
        let outer = match name {
            "left-pane" => viewport.x,
            _ => viewport.x + viewport.w - 1.0,
        };
        let inward = match name {
            "left-pane" => outer + GRAB,
            _ => outer - GRAB,
        };
        let mid = pane.y + pane.h * 0.5;
        for x in [outer, inward] {
            assert!(
                !on_a_boundary(&panel, egui::pos2(x, mid)),
                "with `{name}` open, a boundary grabs ({x}, {mid}) — the window's own edge — and \
                 the pane's outer edge is supposed to be nobody's while the pane is on screen"
            );
        }

        let id = id_of(panel.layout(), name);
        panel.op(Op::Fold(id));
        panel.solve();
        assert!(
            !panel.layout().visible(id),
            "`{name}` is still on screen after the fold"
        );
        assert!(
            panel.layout().is_closed(id),
            "`{name}` folded away rather than closing to its edge, so there is nothing at the \
             window's edge to pull back in and `z` is the only way back"
        );
        for x in [outer, inward] {
            assert!(
                on_a_boundary(&panel, egui::pos2(x, mid)),
                "with `{name}` closed, no boundary grabs ({x}, {mid}) — the pane keeps its edge \
                 exactly so that this coordinate is one, and without it the fold has no way back \
                 but `z`"
            );
        }
    }
}

/// A closed pane gives its width to the centre and keeps the divider it costs,
/// and the width it was storing survives the fold.
#[test]
fn a_closed_pane_gives_its_width_to_the_centre_and_keeps_its_divider() {
    for name in ["left-pane", "right-pane"] {
        let mut panel = console(PLAUSIBLE);
        let was = rect_of(panel.layout(), "centre").w;
        let pane = rect_of(panel.layout(), name).w;
        let (row, _, _) = edge_of(&panel, name);
        let divider = panel
            .layout()
            .divider(row)
            .expect("the body row is a split");

        let id = id_of(panel.layout(), name);
        panel.op(Op::Fold(id));
        panel.solve();
        let now = rect_of(panel.layout(), "centre").w;
        assert!(
            near(now, was + pane),
            "`{name}` closed and the centre went from {was} to {now} — it takes the pane's {pane} \
             and not the {divider} the pane keeps beside it"
        );
        assert!(
            near(rect_of(panel.layout(), name).w, 0.0),
            "a closed `{name}` is {} wide",
            rect_of(panel.layout(), name).w
        );

        // And nothing was destroyed: `z` still brings back the width it had.
        panel.op(Op::UnfoldAll);
        panel.solve();
        assert!(
            near(rect_of(panel.layout(), name).w, pane),
            "`z` brought `{name}` back at {} rather than the {pane} it was storing all along",
            rect_of(panel.layout(), name).w
        );
    }
}

/// A drag out through the pane's own edge closes it, and a drag back in opens
/// it at the pane's declared minimum — through `Panel::press`, `moved` and
/// `released`, which is the window loop's own route.
#[test]
fn a_drag_out_closes_the_pane_and_a_drag_in_brings_it_back_at_its_minimum() {
    for name in ["left-pane", "right-pane"] {
        let mut panel = console(PLAUSIBLE);
        let id = id_of(panel.layout(), name);
        let (row, index, leading) = edge_of(&panel, name);
        let (min, _) = panel.layout().bounds(id);
        let was = rect_of(panel.layout(), name).w;
        assert!(
            was > min,
            "`{name}` starts at its own minimum, so this test cannot tell a pane driven to its \
             stop from one that was already there"
        );

        // **Out, in two moves, and the first of them is exactly as far as the
        // pane goes.** A drag to the pane's own minimum and no further is an
        // ordinary drag and folds nothing — that is the stop an operator meets
        // every time they make a pane as narrow as it goes. What folds it is
        // the `GRAB` after that.
        let at = take_hold(&mut panel, row, index);
        let out = match leading {
            true => -1.0,
            false => 1.0,
        };
        let stopped = panel.moved(Point::new(at.x + out * (was - min), at.y));
        assert!(
            matches!(stopped, Some(Dragged::Boundary { .. })),
            "a drag to `{name}`'s own minimum answered {stopped:?} rather than moving the boundary"
        );
        assert!(
            near(rect_of(panel.layout(), name).w, min),
            "`{name}` is {} wide after a drag that asked for exactly its minimum of {min}",
            rect_of(panel.layout(), name).w
        );
        let closed = panel.moved(Point::new(at.x + out * (was - min + GRAB + 1.0), at.y));
        assert_eq!(
            closed,
            Some(Dragged::Pane(Op::Fold(id))),
            "a drag {} past `{name}`'s own minimum answered {closed:?}, and `GRAB` is {GRAB}",
            GRAB + 1.0
        );
        assert!(
            panel.layout().is_closed(id),
            "`{name}` reported a fold and is not closed"
        );
        panel.released(None);
        panel.solve();

        // In: the boundary the closed pane kept, pulled back the other way.
        let at = take_hold(&mut panel, row, index);
        let opened = panel.moved(Point::new(at.x - out * (GRAB + 1.0), at.y));
        assert_eq!(
            opened,
            Some(Dragged::Pane(Op::Unfold(id))),
            "a drag in from `{name}`'s closed edge answered {opened:?}"
        );
        panel.released(None);
        panel.solve();
        assert!(
            near(rect_of(panel.layout(), name).w, min),
            "`{name}` came back {} wide and its declared minimum is {min} — a pane opened by a \
             hand at the window's edge comes back at its minimum, not at the {was} it was",
            rect_of(panel.layout(), name).w
        );
    }
}

/// One gesture asks for one of them, and this is the flicker that would be
/// there without it: a pane closed by a drag puts its own edge under a pointer
/// that is already further than the distance which opens it.
#[test]
fn one_drag_closes_or_opens_once_and_then_does_neither() {
    for name in ["left-pane", "right-pane"] {
        let mut panel = console(PLAUSIBLE);
        let id = id_of(panel.layout(), name);
        let (row, index, leading) = edge_of(&panel, name);
        let at = take_hold(&mut panel, row, index);
        let out = match leading {
            true => -1.0,
            false => 1.0,
        };

        let mut acted = 0usize;
        let mut step = 20.0f32;
        while step <= 400.0 {
            if let Some(Dragged::Pane(_)) = panel.moved(Point::new(at.x + out * step, at.y)) {
                acted += 1;
            }
            step += 20.0;
        }
        assert_eq!(
            acted, 1,
            "`{name}` was folded or unfolded {acted} times by one drag — a gesture asks for one \
             of them, and the way back is another gesture"
        );
        assert!(
            panel.layout().is_closed(id),
            "`{name}` ended a drag out through its own edge not closed"
        );
    }
}

/// A drag past a stop folds nothing that does not keep its edge, however far
/// past it goes: every other boundary on the console is dragged to both ends
/// and the arrangement comes back folded exactly as much as it was, which is
/// not at all.
#[test]
fn a_drag_past_a_stop_folds_nothing_that_does_not_keep_its_edge() {
    let probe = console(PLAUSIBLE);
    let panes: Vec<String> = folds_to_its_edge(&probe);
    let boundaries: Vec<(NodeId, usize)> = probe.layout().boundaries().collect();
    assert!(
        boundaries.len() >= 5,
        "only {} boundaries to drag",
        boundaries.len()
    );
    let mut dragged = 0usize;
    for (split, index) in boundaries {
        for out in [-4000.0f32, 4000.0] {
            let mut panel = console(PLAUSIBLE);
            let (a, b) = panel.pair(split, index).expect("a boundary has a pair");
            let names: Vec<String> = [a, b]
                .iter()
                .filter_map(|id| panel.layout().name(*id).map(str::to_owned))
                .collect();
            if names.iter().any(|n| panes.contains(n)) {
                continue;
            }
            let axis = panel.layout().axis(split).expect("a divider is on a split");
            let at = take_hold(&mut panel, split, index);
            let to = match axis {
                Axis::Row => Point::new(at.x + out, at.y),
                Axis::Column => Point::new(at.x, at.y + out),
            };
            let said = panel.moved(to);
            dragged += 1;
            panel.released(None);
            panel.solve();
            assert!(
                !matches!(said, Some(Dragged::Pane(_))),
                "dragging boundary {index} of {split:?} — between {names:?} — {out} away answered \
                 {said:?}, and neither side of it is a region whose fold leaves an edge behind"
            );
            for node in panel.nodes() {
                assert!(
                    !panel.layout().is_collapsed(node.id),
                    "a drag on the boundary between {names:?} folded {:?}",
                    panel.layout().name(node.id)
                );
            }
        }
    }
    assert!(
        dragged >= 6,
        "only {dragged} drags reached a boundary with no pane beside it, so this is measuring \
         almost nothing"
    );
}
