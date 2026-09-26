use super::super::*;
use karakuri_operation::{Operation, Tonemap};
use std::time::Duration;

fn bare_strip() -> view::Strip {
    view::Strip {
        name: String::new(),
        tally: view::Tally::Allocated,
        requested: view::Tally::Allocated,
        gain: 0.0,
        gain_to: None,
        opacity: 0.0,
        opacity_to: None,
        blend: karakuri_operation::BlendMode::Add,
        mask: view::Mask::None,
        mask_angle: 0.0,
        level: None,
        is_muted: false,
        is_soloed: false,
    }
}

/// Exercises `Readout::pointer` across baseline pointer events on empty space to verify event dispatch and claiming.
#[test]
fn the_press_handler_dispatches_pointer_events() {
    let ctx = crate::tests::drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();

    // 1. Pointer move to an arbitrary point
    let (_, acted) = readout.pointer(&ctx, Pointer::Moved(Point::new(0.0, 0.0)));
    assert_eq!(acted, Acted::Nothing);

    // 2. Pointer down on unhandled / empty region
    let (_, acted) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(acted, Acted::Nothing);

    // 3. Pointer up
    let (_, acted) = readout.pointer(&ctx, Pointer::Up);
    assert_eq!(acted, Acted::Nothing);

    // 4. Secondary click
    let (_, acted) = readout.pointer(&ctx, Pointer::Secondary);
    assert_eq!(acted, Acted::Nothing);

    // 5. Wheel event
    let (_, acted) = readout.pointer(&ctx, Pointer::Wheel(1.0));
    assert_eq!(acted, Acted::Nothing);
}

/// The press handler dispatches pointer events on the Program bay head's solo
/// pill and on folding bay grips.
#[test]
fn the_press_handler_dispatches_solo_and_bay_grips() {
    let ctx = crate::tests::drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();

    // 1. Solo pill in program head
    let head =
        program_head(&ctx, readout.panel.layout(), Open::CLOSED).expect("the bay draws its pill");
    let solo_at = Point::new(head.solo.center().x, head.solo.center().y);

    let (claim, _) = readout.pointer(&ctx, Pointer::Moved(solo_at));
    assert_eq!(claim, Claim::Panel);

    let (claim, acted) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert!(
        matches!(acted, Acted::Operated(_)),
        "press on solo pill should produce an operated action, got {acted:?}"
    );
    readout.panel.op(Op::Unsolo);
    readout.panel.solve();

    // 2. Bay grips across all regions that have a menu grip: reserved for menu, header double-click toggles fold
    for region in REGIONS {
        if let Some(grip) = bay_grip(readout.panel.layout(), region.name) {
            let at = Point::new(grip.grip.center().x, grip.grip.center().y);
            let (claim, _) = readout.pointer(&ctx, Pointer::Moved(at));
            assert_eq!(
                claim,
                Claim::Panel,
                "bay grip for {} must be claimed",
                region.name
            );

            let (claim, acted) = readout.pointer(&ctx, Pointer::Down);
            assert_eq!(claim, Claim::Panel);
            assert_eq!(
                acted,
                Acted::Nothing,
                "press on bay grip for {} is reserved for bay menu (Acted::Nothing), got {acted:?}",
                region.name
            );

            // Double click on bay head toggles fold
            readout.panel.solve();
            let bay_id = readout.panel.layout().find(region.name).unwrap();
            let bay_rect = readout.panel.layout().rect(bay_id);
            let head_at = Point::new(bay_rect.x + 20.0, bay_rect.y + 10.0);
            let _ = readout.pointer(&ctx, Pointer::Moved(head_at));
            let (claim, acted) = readout.pointer(&ctx, Pointer::DoubleDown);
            assert_eq!(claim, Claim::Panel);
            assert!(
                matches!(acted, Acted::Operated(Outcome::Folded { folded: true, .. })),
                "double click on bay head for {} should fold bay, got {acted:?}",
                region.name
            );

            // Double click again unfolds the bay
            readout.panel.solve();
            let bay_rect = readout.panel.layout().rect(bay_id);
            let head_at = Point::new(bay_rect.x + 20.0, bay_rect.y + 10.0);
            let _ = readout.pointer(&ctx, Pointer::Moved(head_at));
            let (_, acted) = readout.pointer(&ctx, Pointer::DoubleDown);
            assert!(
                matches!(
                    acted,
                    Acted::Operated(Outcome::Folded { folded: false, .. })
                ),
                "second double click on bay head for {} should unfold bay, got {acted:?}",
                region.name
            );
            readout.panel.solve();
        }
    }
}

/// The press handler dispatches pointer events on MCP class pills.
#[test]
fn the_press_handler_dispatches_mcp_class_pills() {
    let ctx = crate::tests::drawn_once();

    for class in Class::ALL {
        let mut readout = Readout::new(1440.0, 900.0);
        readout.panel.solve();

        let pill = mcp_pill(&ctx, readout.panel.layout(), *class, readout.view.opening)
            .unwrap_or_else(|| panic!("{class:?} draws no pill"));
        let at = Point::new(pill.pill.center().x, pill.pill.center().y);

        let (claim, _) = readout.pointer(&ctx, Pointer::Moved(at));
        assert_eq!(claim, Claim::Panel);

        let (claim, acted) = readout.pointer(&ctx, Pointer::Down);
        assert_eq!(claim, Claim::Panel);
        assert_eq!(
            acted,
            Acted::Opened,
            "press on {class:?} pill should yield Acted::Opened"
        );
        assert!(
            readout.opening.read().holds(*class),
            "press on {class:?} pill did not open its class in Opening"
        );
    }
}

/// The press handler dispatches pointer events on transport and transition
/// controls.
#[test]
fn the_press_handler_dispatches_transport_and_transition_controls() {
    let ctx = crate::tests::drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();

    // 1. Transport controls: record pill and tempo figure
    readout.view.transport = Some(view::Transport {
        bpm: 128.0,
        beats: 144.0,
        beats_per_bar: 4,
        fps: Some(60.0),
        frame_ms: 16.6,
        budget_ms: Some(16.6),
        chain_ms: None,
        health: Some(view::Stage::Landed),
        rec: Some(view::Rec::Idle),
    });

    let row =
        transport_row(&ctx, readout.panel.layout(), readout.view.transport).expect("transport row");

    // Rec pill
    let rec_rect = row.rec.expect("rec pill drawn");
    let rec_at = Point::new(rec_rect.center().x, rec_rect.center().y);
    let (claim, _) = readout.pointer(&ctx, Pointer::Moved(rec_at));
    assert_eq!(claim, Claim::Panel);
    let (claim, acted) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert!(
        matches!(acted, Acted::Emitted(Some(Operation::RecordSession { .. }))),
        "expected RecordSession on rec pill press, got {acted:?}"
    );

    // Tempo figure
    let tempo_at = Point::new(row.bpm.center().x, row.bpm.center().y);
    let (claim, _) = readout.pointer(&ctx, Pointer::Moved(tempo_at));
    assert_eq!(claim, Claim::Panel);
    let (claim, acted) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert!(
        matches!(
            acted,
            Acted::Emitted(Some(Operation::SetFreeRunTempo { .. }))
        ),
        "expected SetFreeRunTempo on tempo figure press, got {acted:?}"
    );

    // 2. Transition row controls: shape, quantum, length
    readout.view.mixer = vec![bare_strip(), bare_strip(), bare_strip(), bare_strip()];
    let trow = transition_row(&ctx, readout.panel.layout(), readout.view.transition())
        .expect("transition row");

    let shape_at = Point::new(trow.shape.center().x, trow.shape.center().y);
    let (claim, _) = readout.pointer(&ctx, Pointer::Moved(shape_at));
    assert_eq!(claim, Claim::Panel);
    let (claim, acted) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert!(
        matches!(acted, Acted::Emitted(Some(Operation::SetTransition { .. }))),
        "expected SetTransition on shape pill press, got {acted:?}"
    );

    let quantum_at = Point::new(trow.quantum.center().x, trow.quantum.center().y);
    let (claim, _) = readout.pointer(&ctx, Pointer::Moved(quantum_at));
    assert_eq!(claim, Claim::Panel);
    let (claim, acted) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert!(
        matches!(acted, Acted::Emitted(Some(Operation::SetTransition { .. }))),
        "expected SetTransition on quantum pill press, got {acted:?}"
    );

    let length_at = Point::new(trow.length.center().x, trow.length.center().y);
    let (claim, _) = readout.pointer(&ctx, Pointer::Moved(length_at));
    assert_eq!(claim, Claim::Panel);
    let (claim, acted) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert!(
        matches!(acted, Acted::Emitted(Some(Operation::SetTransition { .. }))),
        "expected SetTransition on length pill press, got {acted:?}"
    );
}

/// The press handler dispatches pointer events on mixer strip controls and
/// outputs row.
#[test]
fn the_press_handler_dispatches_mixer_and_output_controls() {
    let ctx = crate::tests::drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.mixer = vec![bare_strip(), bare_strip(), bare_strip(), bare_strip()];

    let mbay = mixer_bay(&ctx, readout.panel.layout(), &readout.view.mixer).expect("mixer bay");
    let strip = mbay.strip(0);

    // Blend chip
    let blend_at = Point::new(strip.blend.center().x, strip.blend.center().y);
    let (claim, _) = readout.pointer(&ctx, Pointer::Moved(blend_at));
    assert_eq!(claim, Claim::Panel);
    let (claim, acted) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert!(
        matches!(acted, Acted::Emitted(Some(Operation::SetBlendMode { .. }))),
        "expected SetBlendMode on blend chip, got {acted:?}"
    );

    // Tally chip
    let tally_at = Point::new(strip.tally.center().x, strip.tally.center().y);
    let (claim, _) = readout.pointer(&ctx, Pointer::Moved(tally_at));
    assert_eq!(claim, Claim::Panel);
    let (claim, acted) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert!(
        matches!(acted, Acted::Emitted(Some(Operation::SetResidency { .. }))),
        "expected SetResidency on tally chip, got {acted:?}"
    );

    // Mask mini
    let mask_at = Point::new(strip.mask.center().x, strip.mask.center().y);
    let (claim, _) = readout.pointer(&ctx, Pointer::Moved(mask_at));
    assert_eq!(claim, Claim::Panel);
    let (claim, acted) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert!(
        matches!(acted, Acted::Emitted(Some(Operation::SetMaskShape { .. }))),
        "expected SetMaskShape on mask mini, got {acted:?}"
    );

    // Select deck via strip body
    let select_at = Point::new(strip.name.center().x, strip.name.center().y);
    let (claim, _) = readout.pointer(&ctx, Pointer::Moved(select_at));
    assert_eq!(claim, Claim::Panel);
    let (claim, acted) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert!(
        matches!(acted, Acted::Emitted(Some(Operation::SelectDeck { .. }))),
        "expected SelectDeck on strip press, got {acted:?}"
    );

    // Outputs row sink chip
    let outs = outputs(&ctx, readout.panel.layout(), readout.view.opening).expect("outputs row");
    let sink_at = Point::new(outs.sink.center().x, outs.sink.center().y);
    let (claim, _) = readout.pointer(&ctx, Pointer::Moved(sink_at));
    assert_eq!(claim, Claim::Panel);
    let (claim, acted) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert!(
        matches!(acted, Acted::Operated(_)),
        "expected Operated on outputs sink chip, got {acted:?}"
    );
}

/// The press handler dispatches pointer events on look row and learn pill.
#[test]
fn the_press_handler_dispatches_look_and_learn_controls() {
    let ctx = crate::tests::drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();

    // 1. Look row: tonemap and exposure (requires transport to be populated)
    readout.view.transport = Some(view::Transport {
        bpm: 128.0,
        beats: 144.0,
        beats_per_bar: 4,
        fps: Some(60.0),
        frame_ms: 16.6,
        budget_ms: Some(16.6),
        chain_ms: None,
        health: Some(view::Stage::Landed),
        rec: Some(view::Rec::Idle),
    });
    readout.view.look = Some(view::Look {
        tonemap: Tonemap::Aces,
        exposure: 0.0,
    });
    let lrow = look_row(
        &ctx,
        readout.panel.layout(),
        readout.view.transport,
        readout.view.audio.as_ref(),
        readout.view.tracker,
        readout.view.map.as_ref(),
        &readout.view.arrangement,
        readout.view.look,
    )
    .expect("look row");

    let tone_at = Point::new(lrow.tone.center().x, lrow.tone.center().y);
    let (claim, _) = readout.pointer(&ctx, Pointer::Moved(tone_at));
    assert_eq!(claim, Claim::Panel);
    let (claim, acted) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert!(
        matches!(acted, Acted::Emitted(Some(Operation::SetTonemap { .. }))),
        "expected SetTonemap on tone capsule, got {acted:?}"
    );

    let exp_at = Point::new(lrow.grip.center().x, lrow.grip.center().y);
    let (claim, _) = readout.pointer(&ctx, Pointer::Moved(exp_at));
    assert_eq!(claim, Claim::Panel);
    let (claim, acted) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert!(
        matches!(acted, Acted::Emitted(Some(Operation::SetExposure { .. }))),
        "expected SetExposure on exposure track, got {acted:?}"
    );

    // 2. Learn pill (requires a controller map pill to be present)
    readout.view.map = Some(view::MapPill {
        name: Some("default".to_owned()),
    });
    let lpill = view::learn_pill(
        &ctx,
        readout.panel.layout(),
        readout.view.transport,
        readout.view.audio.as_ref(),
        readout.view.tracker,
        readout.view.map.as_ref(),
        false,
    )
    .expect("learn pill");
    let learn_at = Point::new(lpill.pill.center().x, lpill.pill.center().y);
    let (claim, _) = readout.pointer(&ctx, Pointer::Moved(learn_at));
    assert_eq!(claim, Claim::Panel);
    let (claim, acted) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(acted, Acted::Opened);
    assert!(readout.view.learn, "learn state should now be armed");
}

/// The press handler dispatches pointer events on Library bay controls.
#[test]
fn the_press_handler_dispatches_library_controls() {
    let ctx = crate::tests::drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.scopes = Scope::ALL.to_vec();
    readout.view.library = vec!["orbit_wide".to_owned(), "lattice_veil".to_owned()];
    readout.view.select_scope(Scope::MySets);

    let lib = library_bay(
        readout.panel.layout(),
        &readout.view.scopes,
        &readout.view.library,
        readout.view.opened(),
        readout.view.pointed(),
        readout.view.library_scroll(),
    )
    .expect("library bay");

    // 1. Scope chip
    let (_, scope_rect) = lib
        .chips(&ctx, &readout.view.scopes)
        .next()
        .expect("scope chip");
    let scope_at = Point::new(scope_rect.center().x, scope_rect.center().y);
    let (claim, _) = readout.pointer(&ctx, Pointer::Moved(scope_at));
    assert_eq!(claim, Claim::Panel);
    let (claim, acted) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert!(
        matches!(acted, Acted::Emitted(Some(Operation::SelectScope { .. }))),
        "expected SelectScope on scope chip, got {acted:?}"
    );

    // 2. Params chip
    let params_rect = lib.params_chip(&ctx, readout.view.target());
    let params_at = Point::new(params_rect.min.x + 2.0, params_rect.center().y);
    let (claim, _) = readout.pointer(&ctx, Pointer::Moved(params_at));
    assert_eq!(claim, Claim::Panel);
    let (claim, acted) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert!(
        matches!(acted, Acted::Emitted(Some(Operation::ReadSet { .. }))),
        "expected ReadSet on params chip, got {acted:?}"
    );

    // 3. Star button
    let star_rect = lib.star(0);
    let star_at = Point::new(star_rect.center().x, star_rect.center().y);
    let (claim, _) = readout.pointer(&ctx, Pointer::Moved(star_at));
    assert_eq!(claim, Claim::Panel);
    let (claim, acted) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert!(
        matches!(acted, Acted::Emitted(Some(Operation::SetFavourite { .. }))),
        "expected SetFavourite on star button, got {acted:?}"
    );

    // 4. Library row primary press takes Set in hand (carrying)
    let row_rect = lib.row(0);
    let row_at = Point::new(row_rect.center().x, row_rect.center().y);
    let (claim, _) = readout.pointer(&ctx, Pointer::Moved(row_at));
    assert_eq!(claim, Claim::Panel);
    let (claim, _) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert!(
        matches!(readout.panel.in_hand(), Some(InHand::Carrying)),
        "primary press on library row should take set in hand"
    );
    readout.pointer(&ctx, Pointer::Up);

    // 5. Library row secondary press opens row menu
    let (claim, acted) = readout.pointer(&ctx, Pointer::Secondary);
    assert_eq!(claim, Claim::Panel);
    assert_eq!(acted, Acted::Nothing);
    assert!(
        readout.view.menu_open(),
        "secondary press on library row should open context menu"
    );
}

#[test]
fn pointer_move_and_hover_across_all_points() {
    let ctx = egui::Context::default();
    assert_eq!(ctx.cumulative_pass_nr(), 0);
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    readout.view.mixer = vec![bare_strip(), bare_strip(), bare_strip(), bare_strip()];
    let mut hover = karakuri_console::hover::Hover::new();

    for x in (0..1440).step_by(10) {
        for y in (0..900).step_by(10) {
            let p = Point::new(x as f32, y as f32);
            let (claim, acted) = readout.pointer(&ctx, Pointer::Moved(p));
            assert_eq!(acted, Acted::Nothing);
            let _tip = hover.moved(
                claim,
                &readout.panel,
                &ctx,
                &readout.view,
                p,
                Duration::from_millis(500),
            );
        }
    }
}

/// Verifies that initial pointer movements, clicks, and hover events across
/// the full window layout do not panic even when received immediately at startup
/// before the first frame has rendered (cumulative_pass_nr == 0).
#[test]
fn startup_pointer_and_hover_events_do_not_panic_before_first_frame() {
    let ctx = egui::Context::default();
    assert_eq!(ctx.cumulative_pass_nr(), 0);

    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();

    // Populate realistic startup state matching App::resumed
    readout.view.scopes = karakuri_console::view::Scope::ALL.to_vec();
    readout
        .view
        .select_scope(karakuri_console::view::Scope::AllSets);
    readout.view.mixer = vec![bare_strip(), bare_strip(), bare_strip(), bare_strip()];
    readout.view.master_out = Some(1.0);
    readout.view.audio = Some(karakuri_console::view::AudioIn::NONE);
    readout.view.map = Some(karakuri_console::view::MapPill { name: None });

    let mut hover = karakuri_console::hover::Hover::new();

    // Sweep across the entire console layout
    for x in (0..1440).step_by(25) {
        for y in (0..900).step_by(25) {
            let p = Point::new(x as f32, y as f32);

            // 1. Pointer move event
            let (claim, acted) = readout.pointer(&ctx, Pointer::Moved(p));
            assert_eq!(acted, Acted::Nothing);

            // 2. Hover move event
            let tip = hover.moved(
                claim,
                &readout.panel,
                &ctx,
                &readout.view,
                p,
                Duration::from_millis(100),
            );
            // Tip must be Still before first frame
            assert_eq!(tip, karakuri_console::hover::Tip::Still);

            // 3. Pointer down and up events
            let (_down_claim, _down_acted) = readout.pointer(&ctx, Pointer::Down);
            let (_up_claim, _up_acted) = readout.pointer(&ctx, Pointer::Up);
        }
    }

    let tip = hover.left();
    assert_eq!(tip, karakuri_console::hover::Tip::Still);
}

/// The press handler switches active bay focus when the pointer clicks inside a bay.
#[test]
fn the_press_handler_selects_active_bay_on_mouse_click() {
    let ctx = crate::tests::drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();

    assert_eq!(
        readout.view.focused(&readout.panel).map(|b| b.name),
        Some("transport")
    );

    // Move to mixer bay and click
    let mixer_id = readout
        .panel
        .layout()
        .find("mixer")
        .expect("mixer bay exists");
    let mixer_rect = readout.panel.layout().rect(mixer_id);
    let mixer_pt = Point::new(mixer_rect.x + 20.0, mixer_rect.y + 10.0);

    readout.pointer(&ctx, Pointer::Moved(mixer_pt));
    readout.pointer(&ctx, Pointer::Down);
    assert_eq!(
        readout.view.focused(&readout.panel).map(|b| b.name),
        Some("mixer")
    );
    readout.pointer(&ctx, Pointer::Up);

    // Move to library bay and click
    let lib_id = readout
        .panel
        .layout()
        .find("library")
        .expect("library bay exists");
    let lib_rect = readout.panel.layout().rect(lib_id);
    let lib_pt = Point::new(lib_rect.x + 20.0, lib_rect.y + 10.0);

    readout.pointer(&ctx, Pointer::Moved(lib_pt));
    readout.pointer(&ctx, Pointer::Down);
    assert_eq!(
        readout.view.focused(&readout.panel).map(|b| b.name),
        Some("library")
    );
    readout.pointer(&ctx, Pointer::Up);

    // Move to inspector bay and click
    let insp_id = readout
        .panel
        .layout()
        .find("inspector")
        .expect("inspector bay exists");
    let insp_rect = readout.panel.layout().rect(insp_id);
    let insp_pt = Point::new(insp_rect.x + 20.0, insp_rect.y + 10.0);

    readout.pointer(&ctx, Pointer::Moved(insp_pt));
    readout.pointer(&ctx, Pointer::Down);
    assert_eq!(
        readout.view.focused(&readout.panel).map(|b| b.name),
        Some("inspector")
    );
    readout.pointer(&ctx, Pointer::Up);
}

/// Verifies Prompt bay header pill clicks toggle dropdown menu and menu clicks select presets/custom.
#[test]
fn the_press_handler_dispatches_prompt_bay_controls() {
    let ctx = crate::tests::drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();

    let prompt_id = readout
        .panel
        .layout()
        .find("prompt")
        .expect("prompt bay exists");
    let bay_rect = view::to_egui(readout.panel.layout().rect(prompt_id));
    let viewport = view::to_egui(readout.panel.layout().viewport());

    let pill = view::prompt_pill(&ctx, bay_rect, &readout.view.prompt.selection);
    let pill_center = Point::new(pill.center().x, pill.center().y);

    // Initial state: menu closed, unselected
    assert!(!readout.view.prompt.menu_open);
    assert!(readout.view.prompt.selection.is_unselected());

    // 1. Click on header selector pill opens dropdown menu
    readout.pointer(&ctx, Pointer::Moved(pill_center));
    readout.pointer(&ctx, Pointer::Down);
    assert!(readout.view.prompt.menu_open);
    assert_eq!(
        readout.view.active_overlay(),
        Some(view::ModalOverlay::PromptCliMenu)
    );

    // 2. Click on custom option inside menu
    let menu = view::prompt_menu_rect(pill, viewport);
    let custom_rect =
        view::prompt_item_rect(menu, view::CliPreset::ALL.len()).expect("custom item rect");
    let custom_pt = Point::new(custom_rect.center().x, custom_rect.center().y);

    readout.pointer(&ctx, Pointer::Moved(custom_pt));
    readout.pointer(&ctx, Pointer::Down);
    assert!(!readout.view.prompt.menu_open);
    assert_eq!(
        readout.view.prompt.selection,
        view::CliSelection::Custom(
            karakuri_console::view::prompt::default_custom_command().to_string()
        )
    );

    // 3. Open menu again and click outside to dismiss (Rule 2)
    let new_pill = view::prompt_pill(&ctx, bay_rect, &readout.view.prompt.selection);
    let new_pill_center = Point::new(new_pill.center().x, new_pill.center().y);

    readout.pointer(&ctx, Pointer::Moved(new_pill_center));
    readout.pointer(&ctx, Pointer::Down);
    assert!(readout.view.prompt.menu_open);

    let outside_pt = Point::new(bay_rect.min.x + 10.0, bay_rect.max.y - 10.0);
    readout.pointer(&ctx, Pointer::Moved(outside_pt));
    readout.pointer(&ctx, Pointer::Down);
    assert!(!readout.view.prompt.menu_open);
}
