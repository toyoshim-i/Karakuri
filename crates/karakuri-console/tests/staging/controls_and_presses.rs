use super::staging_common::*;

// ---------------------------------------------------------------------------
// What in it is a control, and what is not
// ---------------------------------------------------------------------------

/// The Staging bay's own ground takes no press, with the lane full as well as
/// empty, stated rather than inferred from the absence of a hit test.
///
/// `library.rs`'s test one bay up, and the inset is its inset for its reason: a
/// boundary is claimed for a drag from `GRAB` either side of it, and that is
/// the panel taking a *divider* rather than anything in the bay.
///
/// The rows here are `Stage::Overloaded` on purpose, and that is the half of
/// this test that is about the lane rather than about the card: an overloaded
/// row offers no keep — a slot that has stopped is not a candidate anyone is
/// choosing between (ADR-0316) — so a press on one is `egui`'s, and this is
/// where that is held. The row that *does* take a press is
/// `a_press_on_a_candidate_row_asks_to_keep_it`, and the capsule inside it is
/// `the_back_capsule_is_the_smaller_box_inside_the_row`.
#[test]
fn the_staging_bays_ground_is_not_a_control() {
    let mut panel = console(PLAUSIBLE);
    let ctx = drawn_once();
    let region = to_egui(rect_of(panel.layout(), "staging"));
    let strips = Vec::new();
    // The lane at its fullest, so the points below land on rows rather than on
    // bare card. A closure rather than one `View`, because a `View` holds a
    // frame's plan and is not `Clone`.
    let full = || {
        let mut view = showing(&strips);
        view.staging = (0..3)
            .map(|deck| candidate(deck, "drift_shell + soft_points", Stage::Overloaded))
            .collect();
        view
    };

    let inset = karakuri_console::panel::GRAB + 2.0;
    let points = [
        egui::pos2(region.min.x + inset, region.min.y + inset),
        egui::pos2(region.max.x - inset, region.min.y + inset),
        egui::pos2(region.min.x + inset, region.max.y - inset),
        egui::pos2(region.max.x - inset, region.max.y - inset),
        region.center(),
    ];
    let mut asked = 0;
    for p in points {
        for (lane, view) in [("empty", showing(&strips)), ("full", full())] {
            assert_eq!(
                claim(&mut panel, &ctx, &view, Point::new(p.x, p.y)),
                Claim::Egui,
                "the console took the pointer at {p:?}, which is inside the {lane} Staging bay"
            );
        }
        asked += 1;
    }
    // A guard, so this cannot pass by testing nothing.
    assert_eq!(asked, points.len(), "not every point was asked");
    // **And the rows themselves**, which the five points above do not cover:
    // four of them are in the corners of the bay and the fifth is its centre,
    // so a control drawn *on a row* could sit under none of them. Each row is
    // asked at both ends and in the middle — the three places the mock's
    // `.cand` puts something.
    let lane = staging(panel.layout(), &full().staging).expect("three candidates and a lane");
    let mut rows = 0;
    for index in 0..lane.rows {
        let row = lane.row(index);
        for p in [
            egui::pos2(row.min.x + 1.0, row.center().y),
            row.center(),
            egui::pos2(row.max.x - 1.0, row.center().y),
        ] {
            assert_eq!(
                claim(&mut panel, &ctx, &full(), Point::new(p.x, p.y)),
                Claim::Egui,
                "the console took the pointer at {p:?}, which is on overloaded candidate row \
                 {index} — a stopped slot offers no keep"
            );
        }
        rows += 1;
    }
    // A guard, so this cannot pass by there being no rows to ask about.
    assert_eq!(rows, 3, "the lane drew {rows} rows and was handed three");

    // **The negative control.** It was written when there was nothing in this
    // bay to break, which made the assertions above the trivially-passing kind
    // `docs/contributing.md` §3, *A test is watched to fail before it is kept*,
    // is about; a lane with rows in it has a defect to be run against, and the
    // rows above were — `claim` given a candidate row to answer `Panel` for
    // fails at the bay's centre. This stays all the same, because it is the
    // half that says `claim` is not answering `Egui` for every point it is
    // asked: the one point around here it does *not* answer `Egui` for is the
    // boundary the lane shares with the Library above it, which the panel
    // takes for a drag.
    let above = region.min.y - karakuri_console::panel::GRAB * 0.5;
    assert_eq!(
        claim(
            &mut panel,
            &ctx,
            &showing(&strips),
            Point::new(region.center().x, above)
        ),
        Claim::Panel,
        "the boundary over the Staging bay is not the panel's, so `claim` is answering \
         `Egui` for every point it is asked"
    );
}

// ---------------------------------------------------------------------------
// The two presses a row offers
// ---------------------------------------------------------------------------

/// A press on a candidate row asks to keep that candidate, addressed by the
/// node the row is for.
///
/// The row *is* the control — `console.html`'s *The control is the row itself*
/// — which is the Library bay's list one bay up read the other way round: there
/// a row press takes a Set in hand and names no operation, and here it names
/// one outright. What decided that this act is the large target is what it
/// costs: keeping moves nothing, writes nothing, and takes a line off a list
/// (ADR-0326).
///
/// Both halves are asserted, because they fail differently: `claim` says the
/// console took the press at all, and `StagingBay::keep` says what it asked
/// for. A row that was claimed and handed back the wrong node would pass the
/// first alone.
#[test]
fn a_press_on_a_candidate_row_asks_to_keep_it() {
    let mut panel = console(PLAUSIBLE);
    let ctx = drawn_once();
    let strips = Vec::new();
    let rows = || {
        vec![
            at_node(
                0,
                karakuri_operation::Layer::L1,
                0,
                "drift_shell",
                Stage::Landed,
            ),
            at_node(
                0,
                karakuri_operation::Layer::L4,
                1,
                "soft_points",
                Stage::Landed,
            ),
        ]
    };
    let mut view = showing(&strips);
    view.staging = rows();
    let lane = staging(panel.layout(), &view.staging).expect("two candidates and a lane");
    assert_eq!(lane.rows, 2, "two candidates and {} rows", lane.rows);

    // The left-hand end of each row, which is over the address and never over
    // the capsule at the other end.
    let mut asked = 0;
    for (index, want) in [
        (0, karakuri_operation::Layer::L1, 0u32),
        (1, karakuri_operation::Layer::L4, 1u32),
    ]
    .map(|(index, layer, at)| (index, karakuri_operation::NodeAddress { layer, index: at }))
    {
        let row = lane.row(index);
        let p = Point::new(row.min.x + 2.0, row.center().y);
        assert_eq!(
            claim(&mut panel, &ctx, &view, p),
            Claim::Panel,
            "the console handed row {index} to `egui`, and the row is the keep"
        );
        assert_eq!(
            lane.keep(&ctx, &view.staging, p),
            Some(karakuri_operation::Operation::KeepCandidate {
                deck: 0,
                node: want
            }),
            "row {index} asked for the wrong candidate"
        );
        asked += 1;
    }
    assert_eq!(asked, 2, "not every row was asked");

    // **The negative control, and it is the division the page draws.** A row
    // on `overloaded` is a slot that has stopped rather than a candidate
    // anyone is choosing between, and a row that names no node has nothing to
    // settle — neither takes this press, and a `keep` that answered off the
    // rectangle alone would hand back an operation for both.
    let mut view = showing(&strips);
    view.staging = vec![
        at_node(
            0,
            karakuri_operation::Layer::L1,
            0,
            "drift_shell",
            Stage::Overloaded,
        ),
        slot_row(1, "drift_shell + soft_points", Stage::Refused),
    ];
    let lane = staging(panel.layout(), &view.staging).expect("two candidates and a lane");
    for (index, why) in [(0, "an overloaded row"), (1, "a row that names no node")] {
        let row = lane.row(index);
        let p = Point::new(row.min.x + 2.0, row.center().y);
        assert_eq!(
            lane.keep(&ctx, &view.staging, p),
            None,
            "{why} answered a keep"
        );
        assert_eq!(
            claim(&mut panel, &ctx, &view, p),
            Claim::Egui,
            "the console took a press on {why}"
        );
    }
}

/// The `back` capsule is the smaller box inside the row, and it asks for the
/// node's previous version.
///
/// It is the Library bay's star and its row: the capsule is asked before the
/// row it sits in, so a press on it reaches the capsule and not the keep
/// underneath — `input::claim`'s rule 4, *a control claims what it acts on and
/// no more*. The act that writes a file is the small target and the act that
/// writes nothing is the large one, which is the whole of why they are this way
/// round (ADR-0326).
///
/// The overloaded row is the one that matters, and it is why the capsule is
/// offered on more rows than the keep is: landing an earlier version is one of
/// the three ways out of a stopped slot, and it is the only one of the three
/// this bay can offer.
#[test]
fn the_back_capsule_is_the_smaller_box_inside_the_row() {
    let mut panel = console(PLAUSIBLE);
    let ctx = drawn_once();
    let strips = Vec::new();
    let mut view = showing(&strips);
    view.staging = vec![
        at_node(
            2,
            karakuri_operation::Layer::L4,
            1,
            "soft_points",
            Stage::Overloaded,
        ),
        at_node(
            3,
            karakuri_operation::Layer::L2,
            0,
            "curl_drift",
            Stage::Landed,
        ),
    ];
    let lane = staging(panel.layout(), &view.staging).expect("two candidates and a lane");

    let mut asked = 0;
    for (index, deck, want) in [
        (
            0,
            2u8,
            karakuri_operation::NodeAddress {
                layer: karakuri_operation::Layer::L4,
                index: 1,
            },
        ),
        (
            1,
            3u8,
            karakuri_operation::NodeAddress {
                layer: karakuri_operation::Layer::L2,
                index: 0,
            },
        ),
    ] {
        let capsule = lane
            .back_capsule(&ctx, &view.staging, index)
            .unwrap_or_else(|| panic!("row {index} draws no `back` capsule"));
        assert!(
            lane.row(index).contains(capsule.center()),
            "row {index}'s capsule is not inside the row"
        );
        let p = Point::new(capsule.center().x, capsule.center().y);
        assert_eq!(
            claim(&mut panel, &ctx, &view, p),
            Claim::Panel,
            "the console handed row {index}'s capsule to `egui`"
        );
        assert_eq!(
            lane.back(&ctx, &view.staging, p),
            Some(karakuri_operation::Operation::RestoreProcedure {
                deck,
                revision: karakuri_operation::Revision::Previous(want),
            }),
            "row {index}'s capsule asked for the wrong version"
        );
        // **And the keep does not answer for the same point**, which is
        // structural rather than an ordering: `StagingBay::keep` refuses a
        // point its own capsule claims, so a press on the capsule cannot also
        // be a keep whatever order a caller asks the two in.
        assert_eq!(
            lane.keep(&ctx, &view.staging, p),
            None,
            "row {index}'s capsule is also a keep, so a press on it does two things"
        );
        asked += 1;
    }
    assert_eq!(asked, 2, "not every capsule was asked");

    // **The negative control**: a row that names no node draws no capsule and
    // answers no press, because both operations are spelled with a node.
    let mut view = showing(&strips);
    view.staging = vec![refused_candidate(
        0,
        "drift_shell.kir",
        &["3:5: parse: expected `}`"],
    )];
    let lane = staging(panel.layout(), &view.staging).expect("one candidate and a lane");
    assert_eq!(
        lane.back_capsule(&ctx, &view.staging, 0),
        None,
        "a row that names no node drew a `back` capsule"
    );
    let row = lane.row(0);
    assert_eq!(
        lane.back(
            &ctx,
            &view.staging,
            Point::new(row.max.x - 60.0, row.center().y)
        ),
        None,
        "a row that names no node answered a press where a capsule would have been"
    );
}
