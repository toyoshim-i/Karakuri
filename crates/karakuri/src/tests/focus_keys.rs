//! **`Tab` moves focus, `esc` goes up a level, and this loop leaves the
//! event loop from the window's close and from nowhere else.**
//!
//! [ADR-0259](../../../docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
//! retires quitting to the platform's own accelerator — *"a quit ladder is
//! a sequence that ends in something irreversible, in front of an audience,
//! reached by repeating one key"*
//! ([P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md))
//! — and [ADR-0332](../../../docs/adr/0332-focus-is-a-pointer-the-console-owns-and-the-three-pointers-are-instances-of-it.md)
//! is where that lands. What the ring does is
//! `karakuri-console/tests/focus.rs`'s; what this file says is that the two
//! keys reach it and that one of them stopped doing something else.

use super::*;
use crate::keymap::{key_escape, key_tab, KeyAction, KeyCtx, KEY_BINDINGS};

/// **The run ends at the window's close and nowhere else.**
///
/// A CPU test, tested via structured dispatch rather than text scanning.
/// Asserts that `WindowEvent::CloseRequested` on the main window is the one
/// event that exits, secondary window close does not exit, and none of the
/// key bindings (specifically `esc`) trigger an application exit.
#[test]
fn the_only_way_out_of_the_run_is_the_windows_own_close() {
    // Main window close request exits the event loop
    assert_eq!(
        App::event_loop_action_for(true, &WindowEvent::CloseRequested),
        EventLoopAction::Exit,
        "WindowEvent::CloseRequested on the main window must request event loop exit"
    );

    // Projector / secondary window close request does NOT exit the event loop
    assert_eq!(
        App::event_loop_action_for(false, &WindowEvent::CloseRequested),
        EventLoopAction::Continue,
        "WindowEvent::CloseRequested on secondary/projector window must not exit the event loop"
    );

    // Other events on the main window do NOT exit
    assert_eq!(
        App::event_loop_action_for(true, &WindowEvent::RedrawRequested),
        EventLoopAction::Continue
    );

    // Check all bound keys: none of them have exit actions
    for binding in KEY_BINDINGS {
        match binding.action {
            KeyAction::Handled(_) | KeyAction::Panel(_) | KeyAction::Focus(_) => {}
        }
    }

    // Test that repeatedly calling key_escape never quits/panics (quit ladder test)
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    let mut egui_due = None;
    let mut costs = Costs::new();
    let mut recording = Sessions::new();
    let mut keeping = empty_keeping();
    let store = std::path::PathBuf::new();
    let mut ctx = KeyCtx {
        readout: &mut readout,
        egui_due: &mut egui_due,
        costs: &mut costs,
        recording: &mut recording,
        keeping: &mut keeping,
        store: &store,
        started: Instant::now(),
        shift: false,
    };

    // Repeated key_escape calls in the quit ladder:
    for _ in 0..10 {
        let _ = key_escape(&mut ctx);
    }
    // Still alive, focus remains on a valid bay
    assert!(ctx.readout.view.focused(&ctx.readout.panel).is_some());
}

/// **`Tab` moves focus and `esc` goes up a level**, and each reaches the
/// console rather than deciding anything itself.
#[test]
fn tab_moves_the_focus_and_esc_goes_up_a_level() {
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    let mut egui_due = None;
    let mut costs = Costs::new();
    let mut recording = Sessions::new();
    let mut keeping = empty_keeping();
    let store = std::path::PathBuf::new();
    let mut ctx = KeyCtx {
        readout: &mut readout,
        egui_due: &mut egui_due,
        costs: &mut costs,
        recording: &mut recording,
        keeping: &mut keeping,
        store: &store,
        started: Instant::now(),
        shift: false,
    };

    let initial_bay = ctx.readout.view.focused(&ctx.readout.panel).map(|b| b.name);
    assert!(initial_bay.is_some(), "a bay should be initially focused");

    // 1. Tab forward (shift: false) moves to the next bay in the tab ring
    let moved = key_tab(&mut ctx);
    assert!(moved, "Tab must move focus");
    let forward_bay = ctx.readout.view.focused(&ctx.readout.panel).map(|b| b.name);
    assert_ne!(
        initial_bay, forward_bay,
        "Tab must step focus to a different bay"
    );

    // 2. Tab backward (shift: true) moves back
    ctx.shift = true;
    let moved_back = key_tab(&mut ctx);
    assert!(moved_back, "shift-Tab must move focus");
    let back_bay = ctx.readout.view.focused(&ctx.readout.panel).map(|b| b.name);
    assert_eq!(
        back_bay, initial_bay,
        "shift-Tab must step focus back to previous bay"
    );

    // 3. Escape at top level
    let moved_esc = key_escape(&mut ctx);
    // At top bay level, esc cannot go up further, so it returns false and does not quit
    assert!(!moved_esc, "esc at bay level has no level above it");
    assert!(ctx.readout.view.focused(&ctx.readout.panel).is_some());
}
