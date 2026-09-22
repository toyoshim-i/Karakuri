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

/// Every row of the section with its panel badge: the title, the badge's class
/// — `has`, `plan` or `gap` — and the text it names the control's home with.
///
/// Read verbatim and never decoded, which is `mcp.rs`'s rule and
/// `panel_column.rs`'s after it: a badge that names nowhere says `&mdash;`, and
/// a home that needed decoding to match would be a home nobody could find on
/// the console page.
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

/// What is folded and what is soloed, which is the whole of what an operation
/// in this section can change about an arrangement that nothing has resized. A
/// boundary drag moves rectangles and leaves this alone, which is why the drag
/// is demonstrated separately and this is what the sweep compares.
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

/// Every row of this section a hand on the panel reaches, demonstrated on a
/// running [`Panel`] rather than listed here — see the header.
///
/// Three passes. The first presses every boundary the arrangement has and drags
/// it, and reads the region beside it back either side: that is *Move a
/// boundary*, and it is in the answer only if a boundary actually moved. The
/// second presses, drags and releases at every point of a [`STEP`] grid over
/// the whole console and compares [`shape`] against what it was. The third is
/// [`reached_through_a_painted_control`], which is a different route and not a
/// finer grid.
///
/// # What the grid still asserts, now that a drag *can* fold a pane
///
/// This sentence has moved and the assertion has not, which is worth saying
/// plainly because the two used to be the same thing. It read *nothing
/// reachable through [`Panel::press`] folds, unfolds or solos*, and as a claim
/// about the code that is no longer true: a boundary beside a pane that keeps
/// its edge, dragged `panel::GRAB` past that pane's own minimum, closes it —
/// `Panel::press`, `moved`, `released`, and a fold at the end of it (ADR-0300).
/// `tests/fold_grip.rs` is where that is demonstrated.
///
/// What holds the assertion up now is the grid's own reach, and it is stated
/// here rather than left to be rediscovered. The sweep drags [`DRAG`] = 24
/// pixels in one direction only — right along a row, down a column — and every
/// boundary it can take hold of has at least that much room to give that way at
/// this viewport, so no drag it makes reaches a pane's minimum, let alone
/// `GRAB` past it. It is a narrower guarantee than it was: *the grid never asks
/// for a fold*, rather than *no press can produce one*.
///
/// So a failure here now has three readings and not two. A control nobody
/// accounted for, reached by a route nobody meant to open, which is what it
/// always was; *or* the pull that closes a pane has come within the grid's
/// reach, in which case what the sweep has found is Fold a pane away and that
/// row's panel badge is owed — it is `plan` on `docs/manual/operations.html`
/// and this file is what would have to insert the row here for it to be `has`;
/// *or* a region that keeps its edge has been added where the grid does have
/// room. It panics naming where the press was rather than guessing which, which
/// is the same reason it never guessed a row before.
///
/// The row is not inserted here today, and that is deliberate rather than an
/// oversight: the badge and the demonstration move together
/// ([`every_arrangement_operation_the_pointer_reaches_is_marked_built`] fails
/// in one direction and
/// [`every_arrangement_row_marked_built_is_reached_by_the_pointer`] in the
/// other), and flipping a badge on `docs/manual/operations.html` is not this
/// crate's.
///
/// So the two passes are not two grids of different resolution. A control this
/// crate paints is invisible to any grid driven through `Panel::press`, which
/// is exactly why the third pass drives the other route by hand.
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
    // **The floor that says the grid found the console rather than missed
    // it.** A sweep whose points all landed outside the viewport, or whose
    // step had grown past a divider's grab width, would compare `shape`
    // against itself and report every row unreached — which passes one
    // direction and fails nothing. Every boundary above is a run of points a
    // press takes hold at, so a grid that covers the panel grabs many more
    // than there are boundaries.
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

/// The fifth pass: the grip in a bay head, driven the way the fourth pass
/// drives the `solo` pill beside it.
///
/// A grip is a painted control in the third pass's sense — [`claim`] says the
/// press is the panel's, the derivation that drew it says what the press asks
/// for, and the caller performs it — so the sweep above is blind to it by
/// construction, exactly as it is to `solo`.
///
/// A pane is not here and needs no pass. It folds by its own boundary being
/// pulled past the narrowest it goes
/// ([ADR-0300](../../../docs/adr/0300-a-pane-folds-by-dragging-its-boundary-out-and-comes-back-by-dragging-it-in.md)),
/// which is `Panel::press` and `Panel::moved` — the first pass's route. That
/// pass does not reach it either, and its own doc says why: the sweep drags
/// `DRAG` pixels in one direction, and no boundary it can grab has a pane's
/// minimum within that reach. So both fold rows arrive here through the grip,
/// which is what `rows_of(Op::Fold)` answering both of them means.
///
/// A fresh panel per bay, because a folded bay has no rectangle: a second
/// derivation would be asked about a console the first press changed.
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

    // **The floor that says the walk found the grips rather than missed
    // them.** `bay_grip` answers `None` for a head that declares no grip, so a
    // rename that made it answer `None` for every head would insert nothing
    // and report both fold rows unreached — which fails the assertion above
    // with a message about the page rather than about this walk.
    assert_eq!(
        found, BAY_GRIPS,
        "the walk found {found} grips and `BAY_GRIPS` says {BAY_GRIPS} — this pass has stopped \
         measuring rather than found the control gone"
    );

    reached
}

/// The third pass: the rows a *painted control* reaches, driven the way the
/// window loop drives one.
///
/// The two passes above go through [`Panel::press`], and that is the whole of
/// what they can see. A control this crate paints is reached by a different
/// route with three steps in it — [`claim`] says the press is the panel's, the
/// derivation that drew the control says what the press asks for, and the
/// caller performs whatever that turns out to be — and not one of the
/// three is `Panel::press`. So the sweep is blind to a painted control whether
/// or not it exists, which is not a gap in the sweep: it is what
/// `crates/karakuri/src/main.rs` does with an event, written down in the one
/// place this crate can run it.
///
/// This demonstrates rather than declares, exactly as the sweep does. It
/// lays the arrangement pill out at the size the panel is really solved at,
/// asks [`claim`] who a press on it belongs to, asks the pill what that press
/// asks for, opens the menu with the answer, and then asks every row of the
/// open menu the same question. A list here saying *the pill reaches the
/// reset* would be the second copy of something nothing states
/// (`docs/contributing.md` §4), and
/// it would go on being true after the arm was deleted.
///
/// # Three rows come out of one menu, and they are not all reached the same way
///
/// - *Start a new one* is an [`Op`], so it is performed here and read back:
///   the console is folded about first, the `Op` the row asks for is put
///   through [`Panel::op`], and [`shape`] has to come back to what a fresh
///   panel's was. A reset that reset nothing would be a row demonstrated and
///   not reached.
/// - *Save* and a filed name are `karakuri_operation::Operation`s, and
///   [`NO_OP`] is where the reason is: their payload is a file under a store
///   this crate cannot reach (ADR-0156). So what is demonstrated for those
///   two is the whole of what this crate has — that a press on a row of a
///   drawn menu comes back as that operation, named — and who performs it is
///   `crates/karakuri`'s, which is the same split the header states for *Move
///   a boundary*: this file checks the necessary half and never the
///   sufficient one.
///
/// The row's title comes from the operation rather than from this file —
/// [`rows_of`] for an `Op` and `Operation::title` for an `Operation`, which is
/// the same call `panel_column.rs` makes. A menu that reordered itself is
/// still measured, and a row whose ask changed is not silently read as the one
/// it used to be.
///
/// # The store's answer is handed in, and that is the seam rather than a prop
///
/// The pill is asked with an arrangement in use and one name filed,
/// because that is what decides two of the three asks: with nothing in use
/// *save* asks for letters instead of naming a file, and with nothing filed
/// there is no name to pick. Neither of those is the console being coy — a
/// name and a listing are a file and a directory, and `src/` has no disk, so
/// they arrive per frame from whoever does, exactly as
/// [`karakuri_console::view::View::transport`] does. Handing them in here is
/// this test standing where `crates/karakuri` stands.
///
/// What that leaves uncovered is named rather than implied: on a console
/// with *no* arrangement in use, *save* answers `Ask::Name` and the operation
/// arrives only after a name is typed and committed — which is a keyboard, and
/// this crate has none. That half is `crates/karakuri`'s
/// `a_finished_name_is_the_save_the_menu_would_have_asked_for`.
///
/// # Which failure is loud and which one is quiet, and why they differ
///
/// Everything that would mean the demonstration is not running panics, and
/// the message names what stopped being checked: no pill laid out, a press
/// [`claim`] does not give the panel, a pill that will not open, an open menu
/// with no rows in it, folds that changed nothing to forget, or a picked `Op`
/// that said it reset and did not.
///
/// The one thing that does not panic is the answer itself. If a row stops
/// asking for what it asked for, nothing is inserted for it and
/// [`every_arrangement_row_marked_built_is_reached_by_the_pointer`] is what
/// fails — naming the row, and offering the two readings it always offers:
/// *either the control went and the badge is `plan` again, or it was never on
/// the panel*. That is the direction this pass exists to serve, so it is left
/// to the assertion written for it rather than pre-empted here.
fn reached_through_a_painted_control() -> BTreeSet<&'static str> {
    let mut reached = BTreeSet::new();

    // **Something to forget.** A reset on a console that is already the
    // default changes nothing, so a pass that started there could not tell a
    // reset from a press that did nothing at all.
    //
    // **Two folds and no solo, and that is a fact about the control rather
    // than a convenience.** `Layout::solo` collapses everything off the solo's
    // path, so any solo but one on the transport row itself takes that row off
    // the screen — and a row that is not drawn has no pill in it. The reset
    // forgets a solo as well as a fold, but a hand cannot ask *this* control
    // for it while one is in force; that is the Outputs row's situation one
    // row up (`tests/outputs.rs`), and it belongs to the arrangement rather
    // than to this pass.
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

    // The pill is as wide as the name in it and sits one gap after the bar, so
    // it needs `egui`'s fonts and it needs a console with an engine behind it
    // — see `common::running`, and `view::arrangement` for why a row that is
    // not drawn has no control in it. The name and the listing are the store's
    // answer, handed in.
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

    // **Step one of the real route.** `claim` is what the window loop asks
    // before anything acts, and a press it hands to `egui` never reaches the
    // control at all.
    let on_the_pill = point_of(pill.pill.center());
    assert_eq!(
        claim(&mut p, &ctx, &view, on_the_pill),
        Claim::Panel,
        "`claim` gives a press on the arrangement pill to `egui`, so no route into `{SECTION}` \
         from this control exists however it is drawn"
    );

    // **Step two**: what the press asks for, off the same derivation `claim`
    // hit-tested. Opening a menu is not an operation and never will be — no
    // MIDI map and no MCP call could want to say it — so this arm is the
    // affordance, and the rows below are where the vocabulary starts.
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

    // **Step three**: the caller performs it, which is what an `Op` arriving
    // from a control means — the pill holds no authority and applies nothing
    // (P-0090).
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

    // **Step two**: what the press asks for, off the same derivation `claim`
    // hit-tested. **Step three**: the caller performs it — the pill holds no
    // authority and applies nothing (P-0090).
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

/// A control reaching past the page.
///
/// The other direction, and it fails apart from the test above because it is
/// the other failure: that one says the specification promises a player a
/// control nothing draws, and this one says a hand on the panel already
/// performs something the page still calls designed.
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
