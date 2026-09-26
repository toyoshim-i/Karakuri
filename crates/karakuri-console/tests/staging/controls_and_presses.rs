use super::staging_common::*;

// ---------------------------------------------------------------------------
// What in it is a control, and what is not
// ---------------------------------------------------------------------------

/// Verifies that empty ground and unpressable rows in the Staging bay yield pointer claims to `egui`.
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
    // Tests row bounds (both ends and center) to ensure non-interactive rows don't claim input.
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

    // Boundary grab test confirming `claim` detects panel divider hits rather than always returning `Egui`.
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

/// Verifies that pressing a candidate row claims input and emits a `KeepCandidate` operation (ADR-0326).
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

    // Overloaded rows and nodeless rows do not offer keep actions or claim clicks.
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

/// Verifies that the `back` capsule claims input over the parent row and emits `RestoreProcedure` (ADR-0326).
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
        // `StagingBay::keep` refuses points claimed by capsules, ensuring a single action per press.
        assert_eq!(
            lane.keep(&ctx, &view.staging, p),
            None,
            "row {index}'s capsule is also a keep, so a press on it does two things"
        );
        asked += 1;
    }
    assert_eq!(asked, 2, "not every capsule was asked");

    // Negative control: rows naming no node draw no capsule and answer no press.
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
