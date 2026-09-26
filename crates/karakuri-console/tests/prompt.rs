//! Verification for the Prompt bay (M9 Phase 1).
//!
//! Tests CLI presets (alphabetical ordering, availability detection via PATH),
//! header pill geometry, dropdown menu layout, Rule 2 modal exclusion, and folding behavior.

mod common;

use common::{console_panel as console, drawn_once, near, PLAUSIBLE};
use karakuri_console::focus::{bay_head_at, is_bay, ring, PROMPT};
use karakuri_console::room::size;
use karakuri_console::view::prompt::{
    is_executable_on_path, prompt_ask, prompt_menu_rect, prompt_pill, CliPreset, CliSelection,
    PromptAsk, PromptState, MENU_CARD_W, MENU_ITEM_COUNT, MENU_ITEM_H, PROMPT_TITLE,
};
use karakuri_console::view::{head_of, region, ModalOverlay, View};
use karakuri_layout::Point;

// ---------------------------------------------------------------------------
// 1. CLI Presets & Selection
// ---------------------------------------------------------------------------

#[test]
fn presets_are_in_strict_alphabetical_order() {
    assert_eq!(CliPreset::ALL.len(), 15);
    let commands: Vec<&str> = CliPreset::ALL.iter().map(|p| p.command()).collect();
    assert_eq!(
        commands,
        vec![
            "agy", "aider", "claude", "codex", "copilot", "deepseek", "grok", "kimi", "mimo",
            "minimax", "mistral", "muse", "ollama", "pi", "qwen"
        ]
    );

    // Verify alphabetically sorted
    let mut sorted = commands.clone();
    sorted.sort();
    assert_eq!(commands, sorted);
}

#[test]
fn pill_label_displays_selection_and_chevron() {
    let unselected = CliSelection::Unselected;
    assert_eq!(unselected.pill_label(), "prompt ▾");

    let preset = CliSelection::Preset(CliPreset::Claude);
    assert_eq!(preset.pill_label(), "claude ▾");

    let custom_empty = CliSelection::Custom(String::new());
    assert_eq!(custom_empty.pill_label(), "custom... ▾");

    let custom_cmd = CliSelection::Custom("sh -c echo".to_owned());
    assert_eq!(custom_cmd.pill_label(), "sh -c echo ▾");
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

    // Menu rect hangs below pill
    let menu = prompt_menu_rect(pill, viewport);
    assert_eq!(menu.width(), MENU_CARD_W);
    assert!(menu.min.y >= pill.max.y);
    assert!(near(
        menu.height(),
        6.0 * 2.0 + MENU_ITEM_H * (MENU_ITEM_COUNT as f32)
    ));
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

    // Clicking custom (last item)
    let custom_y = menu.min.y + 6.0 + MENU_ITEM_H * (CliPreset::ALL.len() as f32) + 2.0;
    let custom_point = Point::new(menu.center().x, custom_y);
    assert_eq!(
        prompt_ask(&ctx, bay_rect, viewport, &state, custom_point),
        Some(PromptAsk::SelectCustom)
    );

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
