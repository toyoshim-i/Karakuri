//! Verification for the Prompt bay (M9 Phase 1).
//!
//! Tests CLI presets (alphabetical ordering, availability detection via PATH),
//! header pill geometry, dropdown menu layout, Rule 2 modal exclusion, and folding behavior.

mod common;

use common::{console_panel as console, drawn_once, near, PLAUSIBLE};
use karakuri_console::focus::{bay_head_at, is_bay, ring, PROMPT};
use karakuri_console::room::size;
use karakuri_console::view::prompt::{
    is_executable_on_path, prompt_ask, prompt_item_rect, prompt_menu_rect, prompt_pill, CliPreset,
    CliSelection, PromptAsk, PromptState, MENU_CARD_W, MENU_ITEM_COUNT, MENU_ITEM_H,
    MENU_ROWS_PER_COL, PROMPT_TITLE,
};
use karakuri_console::view::{head_of, region, ModalOverlay, View};
use karakuri_layout::Point;

// ---------------------------------------------------------------------------
// 1. CLI Presets & Selection
// ---------------------------------------------------------------------------

#[test]
fn presets_are_in_strict_alphabetical_order() {
    assert_eq!(CliPreset::ALL.len(), 18);
    let commands: Vec<&str> = CliPreset::ALL.iter().map(|p| p.command()).collect();
    assert_eq!(
        commands,
        vec![
            "agy", "aider", "claude", "cline", "codex", "copilot", "deepseek", "grok", "hermes",
            "kimi", "mimo", "minimax", "mistral", "muse", "ollama", "opencode", "pi", "qwen"
        ]
    );

    // Verify alphabetically sorted
    let mut sorted = commands.clone();
    sorted.sort();
    assert_eq!(commands, sorted);
}

#[test]
fn pill_label_displays_selection() {
    let unselected = CliSelection::Unselected;
    assert_eq!(unselected.pill_label(), "prompt");

    let preset = CliSelection::Preset(CliPreset::Claude);
    assert_eq!(preset.pill_label(), "claude");

    let custom_empty = CliSelection::Custom(String::new());
    assert_eq!(custom_empty.pill_label(), "custom...");

    let custom_cmd = CliSelection::Custom("sh -c echo".to_owned());
    assert_eq!(custom_cmd.pill_label(), "sh -c echo");
}

#[test]
fn executable_path_resolution_checks_existence() {
    // Non-existent command
    assert!(!is_executable_on_path(
        "this_binary_definitely_does_not_exist_xyz123"
    ));
    assert!(!is_executable_on_path(""));

    // Known common system command
    #[cfg(unix)]
    assert!(is_executable_on_path("sh") || is_executable_on_path("ls"));
}

// ---------------------------------------------------------------------------
// 2. PromptState Lifecycle
// ---------------------------------------------------------------------------

#[test]
fn prompt_state_lifecycle_transitions() {
    let mut state = PromptState::new();
    assert!(state.selection.is_unselected());
    assert!(!state.menu_open);

    state.toggle_menu();
    assert!(state.menu_open);

    state.toggle_menu();
    assert!(!state.menu_open);

    state.open_menu();
    assert!(state.menu_open);

    state.select_preset(CliPreset::Agy);
    assert_eq!(state.selection, CliSelection::Preset(CliPreset::Agy));
    assert!(!state.menu_open);

    state.select_custom("bash".to_owned());
    assert_eq!(state.selection, CliSelection::Custom("bash".to_owned()));

    state.reset_selection();
    assert!(state.selection.is_unselected());
}

// ---------------------------------------------------------------------------
// 3. Header Pill & Menu Hit-Testing
// ---------------------------------------------------------------------------

#[test]
fn header_pill_and_menu_geometry() {
    let panel = console(PLAUSIBLE);
    let prompt_id = panel.layout().find("prompt").expect("prompt bay in layout");
    let bay_rect = karakuri_console::view::to_egui(panel.layout().rect(prompt_id));
    let viewport = karakuri_console::view::to_egui(panel.layout().viewport());

    let ctx = drawn_once();
    let state = PromptState::new();
    let pill = prompt_pill(&ctx, bay_rect, &state.selection);

    // Pill sits within header bar
    assert!(pill.min.x >= bay_rect.min.x);
    assert!(pill.max.x <= bay_rect.max.x);
    assert!(pill.min.y >= bay_rect.min.y);
    assert!(pill.max.y <= bay_rect.min.y + size::HEAD_H);

    // Menu rect size matches multi-column geometry
    let menu = prompt_menu_rect(pill, viewport);
    assert_eq!(menu.width(), MENU_CARD_W);
    assert!(near(
        menu.height(),
        6.0 * 2.0 + MENU_ITEM_H * (MENU_ROWS_PER_COL as f32)
    ));

    // Menu stays within viewport bounds (flips upward or clamps if needed)
    assert!(menu.min.x >= viewport.min.x);
    assert!(menu.max.x <= viewport.max.x);
    assert!(menu.min.y >= viewport.min.y);
    assert!(menu.max.y <= viewport.max.y);

    // Each item rect is valid and contained within menu
    for i in 0..MENU_ITEM_COUNT {
        let item = prompt_item_rect(menu, i).expect("valid item rect");
        assert!(menu.contains(item.min));
        assert!(menu.contains(item.max));
    }
}

#[test]
fn prompt_hit_test_routes_clicks() {
    let panel = console(PLAUSIBLE);
    let prompt_id = panel.layout().find("prompt").expect("prompt bay in layout");
    let bay_rect = karakuri_console::view::to_egui(panel.layout().rect(prompt_id));
    let viewport = karakuri_console::view::to_egui(panel.layout().viewport());

    let ctx = drawn_once();
    let mut state = PromptState::new();
    let pill = prompt_pill(&ctx, bay_rect, &state.selection);

    // Click on closed pill triggers menu toggle
    let pill_center = Point::new(pill.center().x, pill.center().y);
    assert_eq!(
        prompt_ask(&ctx, bay_rect, viewport, &state, pill_center),
        Some(PromptAsk::ToggleMenu)
    );

    // When menu is open:
    state.open_menu();
    let menu = prompt_menu_rect(pill, viewport);

    // Clicking custom (last item, in column 1)
    let custom_rect = prompt_item_rect(menu, CliPreset::ALL.len()).expect("custom item rect");
    let custom_point = Point::new(custom_rect.center().x, custom_rect.center().y);
    assert_eq!(
        prompt_ask(&ctx, bay_rect, viewport, &state, custom_point),
        Some(PromptAsk::SelectCustom)
    );

    // Clicking item in column 0 (index 0: Agy)
    let agy_rect = prompt_item_rect(menu, 0).expect("agy item rect");
    let agy_point = Point::new(agy_rect.center().x, agy_rect.center().y);
    let agy_ask = prompt_ask(&ctx, bay_rect, viewport, &state, agy_point);
    if CliPreset::Agy.is_available() {
        assert_eq!(agy_ask, Some(PromptAsk::SelectPreset(CliPreset::Agy)));
    } else {
        assert_eq!(agy_ask, None);
    }

    // Clicking outside open menu returns Shut (Rule 2)
    let outside_point = Point::new(bay_rect.min.x + 10.0, bay_rect.max.y - 10.0);
    assert_eq!(
        prompt_ask(&ctx, bay_rect, viewport, &state, outside_point),
        Some(PromptAsk::Shut)
    );
}

// ---------------------------------------------------------------------------
// 4. Modal Overlay (Rule 2)
// ---------------------------------------------------------------------------

#[test]
fn prompt_menu_is_rule_2_modal_overlay() {
    let mut view = View::new(karakuri_console::room::Room::Day);
    assert_eq!(view.active_overlay(), None);
    assert!(!view.has_modal_overlay());

    view.prompt.open_menu();
    assert_eq!(view.active_overlay(), Some(ModalOverlay::PromptCliMenu));
    assert!(view.has_modal_overlay());
    assert_eq!(ModalOverlay::PromptCliMenu.name(), "PromptCliMenu");

    // Dismissal shuts menu
    assert!(view.dismiss_modal_overlay());
    assert!(!view.prompt.menu_open);
    assert_eq!(view.active_overlay(), None);
}

// ---------------------------------------------------------------------------
// 5. Layout, Region Registry & Folding
// ---------------------------------------------------------------------------

#[test]
fn prompt_bay_is_registered_in_regions() {
    let r = region("prompt").expect("prompt region is in REGIONS");
    assert_eq!(r.name, "prompt");
    assert_eq!(r.kind, karakuri_console::view::Kind::Prompt);

    // Has a bay head with title "Prompt" and a fold grip
    let head = head_of(r).expect("prompt bay has a head descriptor");
    assert_eq!(head.title, PROMPT_TITLE);
    assert!(head.grip);
}

#[test]
fn prompt_bay_in_focus_ring() {
    let panel = console(PLAUSIBLE);
    let bays = ring(panel.layout());
    assert!(bays.iter().any(|b| b.name == PROMPT));
    assert!(is_bay(karakuri_console::view::Kind::Prompt));
}

#[test]
fn prompt_bay_retains_27px_header_when_folded() {
    let mut panel = console(PLAUSIBLE);
    let prompt_id = panel.layout().find("prompt").expect("prompt region exists");

    // Unfolded: height >= 96.0
    let rect_open = panel.layout().rect(prompt_id);
    assert!(rect_open.h >= 96.0);

    // Fold Prompt bay
    panel.layout_mut().collapse(prompt_id);
    panel.solve();

    // Folded: height is exactly size::HEAD_H (27px, ADR-0364)
    let rect_folded = panel.layout().rect(prompt_id);
    assert!(near(rect_folded.h, size::HEAD_H));

    // Verify header hit test identifies the prompt bay
    let header_point = Point::new(rect_folded.x + 50.0, rect_folded.y + 10.0);
    let head_bay = bay_head_at(panel.layout(), header_point);
    assert_eq!(head_bay.map(|b| b.name), Some("prompt"));

    // Unfold Prompt bay
    panel.layout_mut().expand(prompt_id);
    panel.solve();

    // Now unfolded again
    let rect_reopened = panel.layout().rect(prompt_id);
    assert!(rect_reopened.h >= 96.0);
}

#[test]
fn prompt_menu_flips_upward_when_near_viewport_bottom() {
    let viewport = egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), egui::vec2(800.0, 600.0));

    // Case 1: Pill near top of viewport (e.g. y = 100) -> opens downward
    let pill_top =
        egui::Rect::from_min_size(egui::Pos2::new(100.0, 100.0), egui::vec2(100.0, 20.0));
    let menu_down = prompt_menu_rect(pill_top, viewport);
    assert!(menu_down.min.y >= pill_top.max.y);

    // Case 2: Pill near bottom of viewport (e.g. y = 500 in 600px viewport) -> flips upward
    let pill_bottom =
        egui::Rect::from_min_size(egui::Pos2::new(100.0, 500.0), egui::vec2(100.0, 20.0));
    let menu_up = prompt_menu_rect(pill_bottom, viewport);
    assert!(menu_up.max.y <= pill_bottom.min.y);
    assert!(menu_up.min.y >= viewport.min.y);
}

// ---------------------------------------------------------------------------
// 6. Interactive Terminal Session & PTY Execution (M9 Phase 2)
// ---------------------------------------------------------------------------

#[test]
fn terminal_session_spawns_and_captures_pty_output() {
    #[cfg(unix)]
    {
        use karakuri_console::view::prompt::TerminalSession;
        let session =
            TerminalSession::spawn("test-sh".to_string(), "sh", &["-c", "echo hello from pty"]);

        let mut found = false;
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(20));
            let lines = session.lines();
            if lines.iter().any(|l| l.contains("hello from pty")) {
                found = true;
                break;
            }
        }
        assert!(
            found,
            "expected PTY output to contain 'hello from pty', got: {:?}",
            session.lines()
        );
    }
}

#[test]
fn session_manager_manages_presets_and_custom() {
    let mut state = PromptState::new();
    assert!(state.active_session().is_none());

    state.select_custom("echo karakuri".to_string());
    assert!(state.active_session().is_some());

    state.set_input("test input");
    assert_eq!(state.input(), "test input");
}

#[test]
fn terminal_session_stdin_writing_and_reading() {
    #[cfg(unix)]
    {
        use karakuri_console::view::prompt::TerminalSession;
        let session = TerminalSession::spawn("test-cat".to_string(), "cat", &[]);

        // Wait briefly for cat to start
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(session.is_running());

        // Send a line through PTY stdin
        session.send_line("hello from stdin").expect("send line");

        let mut found = false;
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(20));
            let lines = session.lines();
            if lines.iter().any(|l| l.contains("hello from stdin")) {
                found = true;
                break;
            }
        }
        assert!(
            found,
            "expected PTY echo to contain 'hello from stdin', got: {:?}",
            session.lines()
        );
    }
}

#[test]
fn prompt_boundary_drag_cascades_through_staging_into_library() {
    use karakuri_layout::Rect;

    let mut layout = karakuri_console::layout();
    layout.set_viewport(Rect::new(0.0, 0.0, 1280.0, 800.0));
    layout.solve();

    let left_pane = layout.find("left-pane").unwrap();
    let library = layout.find("library").unwrap();
    let staging = layout.find("staging").unwrap();
    let prompt = layout.find("prompt").unwrap();

    let initial_lib_h = layout.rect(library).h;
    let initial_staging_h = layout.rect(staging).h;
    let initial_prompt_h = layout.rect(prompt).h;

    assert_eq!(initial_staging_h, 125.0);
    assert_eq!(initial_prompt_h, 220.0);

    // Drag divider 1 (between staging and prompt) upward by 100px.
    // Staging shrinks by 59px (from 125 to min 66), and the remaining 41px
    // cascades into Library, shrinking Library by 41px while Staging stays at min 66.
    let curr_div1 = layout.rect(staging).y + layout.rect(staging).h;
    let landed = layout.set_divider(left_pane, 1, curr_div1 - 100.0);

    assert_eq!(landed, curr_div1 - 100.0);
    assert_eq!(layout.rect(staging).h, 66.0, "staging capped at min 66");
    assert_eq!(layout.rect(prompt).h, 320.0, "prompt grew by 100");
    assert_eq!(
        layout.rect(library).h,
        initial_lib_h - 41.0,
        "library absorbed overflow"
    );

    // Now drag divider 1 back downward by 100px to shrink Prompt.
    // Staging expands by 59px (from 66 to max 125), and the remaining 41px
    // cascades into Library, expanding Library by 41px back to initial.
    let new_div1 = layout.rect(staging).y + layout.rect(staging).h;
    let landed_back = layout.set_divider(left_pane, 1, new_div1 + 100.0);

    assert_eq!(landed_back, new_div1 + 100.0);
    assert_eq!(layout.rect(staging).h, 125.0, "staging capped at max 125");
    assert_eq!(layout.rect(prompt).h, 220.0, "prompt returned to 220");
    assert_eq!(layout.rect(library).h, initial_lib_h, "library restored");
}

#[test]
fn staging_top_boundary_drag_downward_cascades_through_staging_into_prompt() {
    use karakuri_layout::Rect;

    let mut layout = karakuri_console::layout();
    layout.set_viewport(Rect::new(0.0, 0.0, 1280.0, 800.0));
    layout.solve();

    let left_pane = layout.find("left-pane").unwrap();
    let library = layout.find("library").unwrap();
    let staging = layout.find("staging").unwrap();
    let prompt = layout.find("prompt").unwrap();

    let initial_lib_h = layout.rect(library).h;
    let initial_staging_h = layout.rect(staging).h;
    let initial_prompt_h = layout.rect(prompt).h;

    assert_eq!(initial_staging_h, 125.0);
    assert_eq!(initial_prompt_h, 220.0);

    // Drag divider 0 (top of Staging, between library and staging) downward by 100px.
    // Staging shrinks by 59px (from 125 to min 66), and the remaining 41px
    // cascades into Prompt, shrinking Prompt from 220 to 179 while Staging stays at min 66.
    let curr_div0 = layout.rect(library).y + layout.rect(library).h;
    let landed = layout.set_divider(left_pane, 0, curr_div0 + 100.0);

    assert_eq!(landed, curr_div0 + 100.0);
    assert_eq!(
        layout.rect(library).h,
        initial_lib_h + 100.0,
        "library expanded by 100"
    );
    assert_eq!(layout.rect(staging).h, 66.0, "staging capped at min 66");
    assert_eq!(
        layout.rect(prompt).h,
        initial_prompt_h - 41.0,
        "prompt compressed by 41"
    );
}

#[test]
fn terminal_session_detects_child_process_termination() {
    #[cfg(unix)]
    {
        use karakuri_console::view::prompt::{SessionStatus, TerminalSession};

        let session = TerminalSession::spawn("test-exit".to_string(), "true", &[]);

        let mut exited = false;
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(20));
            if matches!(session.status(), SessionStatus::Exited(Some(0))) {
                exited = true;
                break;
            }
        }
        assert!(
            exited,
            "expected session to terminate with exit code 0, status is: {:?}",
            session.status()
        );
        assert!(
            !session.is_running(),
            "exited session should not report is_running"
        );
    }
}

#[test]
fn session_manager_restarts_exited_session() {
    #[cfg(unix)]
    {
        use karakuri_console::view::prompt::{CliSelection, SessionManager, SessionStatus};
        use std::sync::Arc;

        let manager = SessionManager::new();
        let selection = CliSelection::Custom("true".to_string());

        let s1 = manager
            .get_or_spawn(&selection)
            .expect("spawn initial session");
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(20));
            if matches!(s1.status(), SessionStatus::Exited(_)) {
                break;
            }
        }
        assert!(!s1.is_running());

        let s2 = manager.restart(&selection).expect("restart session");
        assert!(
            !Arc::ptr_eq(&s1, &s2),
            "restart should create a new session instance"
        );
    }
}
