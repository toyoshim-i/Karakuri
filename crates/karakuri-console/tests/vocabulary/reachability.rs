use super::vocabulary_common::*;

/// The section this crate answers for, as text. Sliced once so that a badge
/// found past the section's end belongs to another section's row, which is the
/// same cut [`rows`] makes for the same reason.
fn arranging() -> String {
    let text = page();
    let start = text.find(SECTION).unwrap_or_else(|| {
        panic!("`{SECTION}` is gone from {PAGE} — the section this crate answers for")
    });
    let rest = &text[start + SECTION.len()..];
    let end = rest.find("<h2").unwrap_or(rest.len());
    rest[..end].to_owned()
}

/// Extracts every row with its panel badge class (`has`, `plan`, `gap`) and control home verbatim.
fn panel_badges() -> Vec<(String, String, String)> {
    let mut found = Vec::new();
    for part in arranging().split(ROW).skip(1) {
        let open = part.find("<h3>").expect("a row opens with its heading");
        let close = part[open..].find("</h3>").expect("a heading closes");
        let title = part[open + 4..open + close].to_owned();
        let body = &part[open + close..];
        let mut badge = None;
        for span in body.split(r#"<span class="rt "#).skip(1) {
            let Some(quote) = span.find('"') else {
                continue;
            };
            let class = span[..quote].to_owned();
            let Some(text) = span[quote..].strip_prefix(r#"">panel <b>"#) else {
                continue;
            };
            let Some(shut) = text.find("</b>") else {
                continue;
            };
            badge = Some((class, text[..shut].to_owned()));
            break;
        }
        let (class, home) = badge.unwrap_or_else(|| {
            panic!(
                "`{title}` under `{SECTION}` in {PAGE} has no panel badge — every row carries \
                 one, and a row that has stopped is a row this file stops measuring"
            )
        });
        found.push((title, class, home));
    }
    found
}

/// Returns visibility mask and solo status of panel nodes.
fn shape(p: &mut Panel) -> (Vec<bool>, bool) {
    p.solve();
    let ids: Vec<NodeId> = p.nodes().iter().map(|n| n.id).collect();
    let layout = p.layout();
    (
        ids.iter().map(|id| layout.visible(*id)).collect(),
        layout.is_soloed(),
    )
}

/// The point in the middle of a boundary's own gap: along the split's axis,
/// half way between the two regions it is between, and across it, half way down
/// the first of them.
fn on_the_boundary(p: &mut Panel, split: NodeId, index: usize) -> Option<(Axis, Point, NodeId)> {
    p.solve();
    let axis = p.layout().axis(split)?;
    let (a, b) = p.pair(split, index)?;
    let (ra, rb) = (p.layout().rect(a), p.layout().rect(b));
    let along = (axis.far(ra) + axis.origin(rb)) * 0.5;
    Some((
        axis,
        match axis {
            Axis::Row => Point::new(along, ra.y + ra.h * 0.5),
            Axis::Column => Point::new(ra.x + ra.w * 0.5, along),
        },
        a,
    ))
}

/// The arrangement this pass tells the pill is in use, and the one name it
/// tells it is filed. Any name a store would accept; what matters is that there
/// is one, which is what makes *save* name a file rather than ask for letters
/// and what puts a row in the list to pick.
const IN_USE: &str = "night";

/// A `karakuri_layout` point, from `egui`'s. The console's controls are laid
/// out in `egui`'s rectangles and hit-tested in the layout's points, and this
/// is the one step between them.
fn point_of(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

/// `at` moved [`DRAG`] along `axis`.
fn dragged_to(axis: Axis, at: Point) -> Point {
    match axis {
        Axis::Row => Point::new(at.x + DRAG, at.y),
        Axis::Column => Point::new(at.x, at.y + DRAG),
    }
}

/// Demonstrates every operation reachable via pointer interactions: boundary drag, grid sweep,
/// painted controls, and bay grips (ADR-0300).
fn reached_by_the_pointer() -> BTreeSet<&'static str> {
    let mut reached = BTreeSet::new();

    let mut p = panel();
    p.solve();
    let boundaries: Vec<(NodeId, usize)> = p.layout().boundaries().collect();
    assert!(
        boundaries.len() >= 5,
        "only {} boundaries in the arrangement — a sweep with nothing to drag would demonstrate \
         nothing and every assertion below would pass on an empty answer",
        boundaries.len()
    );
    for (split, index) in boundaries {
        let mut p = panel();
        let Some((axis, at, beside)) = on_the_boundary(&mut p, split, index) else {
            continue;
        };
        let was = axis.extent(p.layout().rect(beside));
        if !matches!(p.press(at), Pressed::Grabbed { .. }) {
            continue;
        }
        let dragged = matches!(
            p.moved(dragged_to(axis, at)),
            Some(Dragged::Boundary { .. })
        );
        p.released(None);
        p.solve();
        let now = axis.extent(p.layout().rect(beside));
        if dragged && (now - was).abs() >= MOVED {
            reached.insert("Move a boundary");
        }
    }

    let mut p = panel();
    let before = shape(&mut p);
    let viewport = p.layout().viewport();
    let mut grabs = 0usize;
    let mut y = viewport.y;
    while y < viewport.y + viewport.h {
        let mut x = viewport.x;
        while x < viewport.x + viewport.w {
            let at = Point::new(x, y);
            let grabbed = matches!(p.press(at), Pressed::Grabbed { .. });
            // Both ways, because a control that acts on the move rather than
            // on the press does not say which axis it is watching.
            p.moved(dragged_to(Axis::Row, at));
            p.moved(dragged_to(Axis::Column, at));
            p.released(None);
            let now = shape(&mut p);
            assert_eq!(
                now, before,
                "a press at ({x}, {y}) folded, unfolded or soloed something, and no drag this \
                 grid makes is supposed to: it drags {DRAG} pixels one way, and every boundary \
                 it can take hold of has that much room to give that way at this viewport — so \
                 none of them reaches a pane's own minimum, let alone the `GRAB` past it that \
                 closes a pane (ADR-0300). Three readings, and the header has them: the pull \
                 that closes a pane has come within this grid's reach, in which case this is \
                 *Fold a pane away* and its badge is owed; or a region that keeps its edge has \
                 been added where the grid has room; or this is a route into `{SECTION}` \
                 nobody accounted for. A control this crate *paints* cannot be reached from \
                 here at all — see `reached_through_a_painted_control`. Say which row it lands \
                 on, add it to `reached_by_the_pointer`, and flip that row's panel badge"
            );
            // A drag moved a boundary, so the arrangement the next press lands
            // on is not the one this pass started from.
            if grabbed {
                grabs += 1;
                p = panel();
            }
            x += STEP;
        }
        y += STEP;
    }
    // Verify sweep density grabs boundaries at least 5 times across the viewport.
    assert!(
        grabs >= 5,
        "the sweep pressed the whole viewport and took hold of a boundary {grabs} times — a \
         grid this coarse is not pressing the console"
    );

    reached.extend(reached_through_a_painted_control());
    reached.extend(reached_through_the_program_bays_head());
    reached.extend(reached_through_the_grip_in_a_bay_head());
    reached
}

/// Verifies fold operations reachable via bay head grips (ADR-0300).
fn reached_through_the_grip_in_a_bay_head() -> BTreeSet<&'static str> {
    let mut reached = BTreeSet::new();
    let ctx = drawn_once();
    let view = showing(&[]);

    let mut found = 0usize;
    for region in REGIONS {
        let mut p = panel();
        let default = shape(&mut p);
        let Some(grip) = bay_grip(p.layout(), region.name) else {
            continue;
        };
        found += 1;
        let at = point_of(grip.grip.center());
        assert_eq!(
            claim(&mut p, &ctx, &view, at),
            Claim::Panel,
            "`claim` gives a press on `{}`'s grip to `egui`, so no route into `{SECTION}` from \
             this control exists however it is drawn",
            region.name
        );
        let op = grip.op();
        let outcome = p.op(op);
        p.solve();
        assert!(
            matches!(outcome, Outcome::Folded { folded: true, .. }),
            "a press on `{}`'s grip asked for `{op:?}` and answered {outcome:?}, which folded \
             nothing",
            region.name
        );
        assert_ne!(
            shape(&mut p),
            default,
            "`{}`'s grip folded and left the console looking exactly as it did, so the row was \
             demonstrated and did not do what its row on {PAGE} says",
            region.name
        );
        reached.extend(rows_of(op));
    }

    // Assert all expected bay grips were discovered.
    assert_eq!(
        found, BAY_GRIPS,
        "the walk found {found} grips and `BAY_GRIPS` says {BAY_GRIPS} — this pass has stopped \
         measuring rather than found the control gone"
    );

    reached
}

/// Demonstrates operations reachable via painted controls (arrangement pill and menu; ADR-0156).
fn reached_through_a_painted_control() -> BTreeSet<&'static str> {
    let mut reached = BTreeSet::new();

    // Fold bays to ensure non-default state so reset operation is measurable.
    let mut p = panel();
    let default = shape(&mut p);
    for bay in ["staging", "sequencer"] {
        let id = p
            .layout()
            .find(bay)
            .unwrap_or_else(|| panic!("a {bay} bay"));
        p.op(Op::Fold(id));
    }
    p.solve();
    assert_ne!(
        shape(&mut p),
        default,
        "the folds this pass makes to have something to forget changed nothing, so a reset \
         performed below would be indistinguishable from a press that did nothing"
    );

    // Render arrangement pill with simulated store state.
    let ctx = drawn_once();
    let mut view = running();
    view.arrangement.name = Some(IN_USE.to_owned());
    view.arrangement.filed = vec![IN_USE.to_owned()];
    let pill = arrangement(
        &ctx,
        p.layout(),
        view.transport,
        None,
        None,
        None,
        &view.arrangement,
    )
    .unwrap_or_else(|| {
        panic!(
            "the transport row draws no arrangement pill on a solved console with an \
                 engine behind it, so nothing here can demonstrate a row of `{SECTION}` from a \
                 pointer — this pass has stopped measuring rather than found the control gone"
        )
    });

    // Step 1: `claim` intercepts pointer input before UI dispatch.
    let on_the_pill = point_of(pill.pill.center());
    assert_eq!(
        claim(&mut p, &ctx, &view, on_the_pill),
        Claim::Panel,
        "`claim` gives a press on the arrangement pill to `egui`, so no route into `{SECTION}` \
         from this control exists however it is drawn"
    );

    // Step 2: Query control intent; opening a menu is a local affordance.
    match pill.ask(&view.arrangement, on_the_pill) {
        Some(Ask::Open) => view.arrangement.opened(),
        other => panic!(
            "a press on the arrangement pill asks for {other:?} rather than opening its menu, \
             so the rows this pass reads `{SECTION}` off are unreachable"
        ),
    }

    let open = arrangement(
        &ctx,
        p.layout(),
        view.transport,
        None,
        None,
        None,
        &view.arrangement,
    )
    .expect("the pill was drawn a moment ago and the panel has not moved");
    assert!(
        open.rows > 0,
        "the arrangement pill's menu opened with no rows in it, so every ask below is asked of \
         nothing and this pass would report every row unreached whatever the control does"
    );

    // Step 3: Dispatch operation; controls yield intent without direct execution authority (P-0090).
    for row in 0..open.rows {
        match open.ask(&view.arrangement, point_of(open.row(row).center())) {
            Some(Ask::Panel(op)) => {
                let outcome = p.op(op);
                p.solve();
                // Verify that the outcome explicitly reports Reset.
                assert_eq!(
                    outcome,
                    Outcome::Reset,
                    "menu row {row} asked for `{op:?}`, and this pass takes an `Op` from a menu \
                     row as `Reset the arrangement` being reached — so an `Op` that is not the \
                     reset is a row asking for something nobody accounted for"
                );
                assert_eq!(
                    shape(&mut p),
                    default,
                    "menu row {row} asked for `{op:?}` and performing it left the console \
                     folded — the row was demonstrated and it did not do what its row on \
                     {PAGE} says, which is a worse answer than not reaching it at all"
                );
                reached.extend(rows_of(op));
            }
            // Named, not performed: the payload is a file and this crate has
            // no store. See this function's header, and `NO_OP`.
            Some(Ask::Operation(operation)) => {
                reached.insert(operation.title());
            }
            // *Save* with nothing in use, and a press on the card that is on
            // no row. Neither names an operation, and neither is a failure —
            // the first is the one flow that asks for letters, and this pass
            // hands in a name in use so that it does not arrive.
            Some(Ask::Name) | Some(Ask::Open) | Some(Ask::Shut) | None => {}
        }
    }
    reached
}

/// Evaluates operations reached through the program bay solo pill.
fn reached_through_the_program_bays_head() -> BTreeSet<&'static str> {
    let mut reached = BTreeSet::new();

    let ctx = drawn_once();
    let view = showing(&[]);
    let mut p = panel();
    let default = shape(&mut p);

    // Verify that pointer claim routes to panel and hits the solo pill.
    let head = program_head(&ctx, p.layout(), Open::CLOSED).unwrap_or_else(|| {
        panic!(
            "the Program bay draws no `solo` pill on a solved console, so nothing here can \
             demonstrate a row of `{SECTION}` from a pointer — this pass has stopped measuring \
             rather than found the control gone"
        )
    });
    let on_the_pill = point_of(head.solo.center());
    assert_eq!(
        claim(&mut p, &ctx, &view, on_the_pill),
        Claim::Panel,
        "`claim` gives a press on the Program bay's `solo` pill to `egui`, so no route into \
         `{SECTION}` from this control exists however it is drawn"
    );

    // Step 2 & 3: Query and dispatch operation without local authority (P-0090).
    let op = head.op();
    let outcome = p.op(op);
    p.solve();
    assert_eq!(
        outcome,
        Outcome::Soloed(head.id),
        "a press on the `solo` pill asked for `{op:?}`, which did not solo the picture"
    );
    assert_ne!(
        shape(&mut p),
        default,
        "the `solo` pill soloed the picture and left the console looking exactly as it did, \
         so the row was demonstrated and did not do what its row on {PAGE} says"
    );
    reached.extend(rows_of(op));

    // And the other answer, off a head derived again on the console the first
    // press left.
    let head = program_head(&ctx, p.layout(), Open::CLOSED).unwrap_or_else(|| {
        panic!(
            "the Program bay draws no `solo` pill with the picture soloed, so the undo this \
             row promises is unreachable — a solo takes every other control off the screen, \
             and this is the one left"
        )
    });
    let on_the_pill = point_of(head.solo.center());
    assert_eq!(
        claim(&mut p, &ctx, &view, on_the_pill),
        Claim::Panel,
        "`claim` gives a press on the `solo` pill to `egui` once the picture is soloed"
    );
    let op = head.op();
    let outcome = p.op(op);
    p.solve();
    assert_eq!(
        outcome,
        Outcome::Unsoloed { was: true },
        "a second press on the `solo` pill asked for `{op:?}`, which did not undo the solo"
    );
    assert_eq!(
        shape(&mut p),
        default,
        "undoing the solo from the pill left the console somewhere else — the row says it \
         restores what was folded before, including whatever was already folded"
    );
    reached.extend(rows_of(op));

    reached
}

/// The floor under both directions below, and the same one `panel_column.rs`
/// carries: a scan that matched nothing satisfies every loop by iterating over
/// nothing at all.
#[test]
fn the_sweep_finds_the_section_and_the_panel() {
    let badges = panel_badges();
    assert_eq!(
        badges.len(),
        rows().len(),
        "{PAGE} has {} rows under `{SECTION}` and {} panel badges — is a badge still an `rt` \
         span reading `panel <b>…</b>`?",
        rows().len(),
        badges.len()
    );
    assert!(
        badges.len() >= 8,
        "only {} rows with a panel badge under `{SECTION}` in {PAGE}",
        badges.len()
    );
    assert!(
        !reached_by_the_pointer().is_empty(),
        "the sweep reached nothing at all — every divider on this panel drags, so a sweep that \
         demonstrates none of them has stopped pressing the panel rather than found it inert"
    );
}

/// Verifies that every arrangement operation marked built on the manual page is reachable by pointers (ADR-0213, ADR-0225).
#[test]
fn every_arrangement_row_marked_built_is_reached_by_the_pointer() {
    let reached = reached_by_the_pointer();
    for (title, class, home) in panel_badges() {
        if class != "has" {
            continue;
        }
        assert!(
            reached.contains(title.as_str()),
            "{PAGE} marks `{title}` built in the panel column and no gesture on a running \
             `Panel` performs it — the page claims a control an operator cannot find. Either \
             the control went and the badge is `plan` again, or it was never on the panel"
        );
        assert_ne!(
            home, NOWHERE,
            "{PAGE} marks `{title}` built in the panel column and names no home for it — a \
             `has` badge says an operator reaches the operation, so it has to say where the \
             control is"
        );
    }
}

/// Asserts that any operation reachable via pointer gestures is marked as built (`has`) in the manual.
#[test]
fn every_arrangement_operation_the_pointer_reaches_is_marked_built() {
    let badges = panel_badges();
    for row in reached_by_the_pointer() {
        let (_, class, _) = badges
            .iter()
            .find(|(title, _, _)| title == row)
            .unwrap_or_else(|| {
                panic!(
                    "a gesture on a running `Panel` performs `{row}` and {PAGE} has no such row \
                     under `{SECTION}` — the page is the specification, so add the row there \
                     first"
                )
            });
        assert_eq!(
            class, "has",
            "a hand on the panel performs `{row}`, which {PAGE} marks `{class}` in the panel \
             column — an operation an operator reaches and a page that says no program a player \
             runs does (ADR-0213). Flip the badge, or say here why the gesture is not reachable"
        );
    }
}
