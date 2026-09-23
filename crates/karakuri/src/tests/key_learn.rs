//! Tests for tooltip interactive Key Learn mode and keyboard customization.

use super::*;
use crate::keymap::{ActionId, Keymap};
use karakuri_console::hover::flat;
use winit::keyboard::Key;

#[test]
fn tooltip_key_learn_badge_and_toggle_hit_test() {
    let mut hover = karakuri_console::hover::Hover::new();
    assert_eq!(hover.learning_key(), None);
    assert!(!hover.key_globalize());

    hover.set_learning_key(Some(2));
    assert_eq!(hover.learning_key(), Some(2));

    hover.toggle_globalize();
    assert!(hover.key_globalize());

    hover.set_key_globalize(false);
    assert!(!hover.key_globalize());

    hover.set_learning_key(None);
    assert_eq!(hover.learning_key(), None);
}

#[test]
fn key_learn_binding_workflow_with_globalization() {
    let temp_dir =
        std::env::temp_dir().join(format!("karakuri_test_key_learn_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);

    let mut keymap = Keymap::default_keymap();

    // Verify initial binding for TapBeat ('b' in transport, not global)
    let key_b = Key::Character("b");
    assert!(keymap.find_binding(&key_b, Some("transport")).is_some());
    assert!(keymap.find_binding(&key_b, Some("mixer")).is_none());

    // Bind TapBeat to 'x' with globalize = true
    let notes = keymap
        .bind_action(&temp_dir, ActionId::TapBeat, Some("transport"), "x", true)
        .expect("bind_action should succeed");
    assert!(notes.is_empty() || notes.iter().all(|n| n.starts_with("warning:")));

    // Verify 'x' is now active in transport AND globally in mixer
    let key_x = Key::Character("x");
    let b_transport = keymap.find_binding(&key_x, Some("transport"));
    assert!(b_transport.is_some());
    assert_eq!(b_transport.unwrap().legend, "x");
    assert!(b_transport.unwrap().globalize);

    let b_mixer = keymap.find_binding(&key_x, Some("mixer"));
    assert!(b_mixer.is_some());
    assert!(b_mixer.unwrap().globalize);

    // Verify keymap file was written and can be reloaded
    let reloaded = Keymap::load_or_default(&temp_dir);
    let b_reloaded = reloaded.find_binding(&key_x, Some("mixer"));
    assert!(b_reloaded.is_some());
    assert!(b_reloaded.unwrap().globalize);

    // Clean up
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn action_id_mapping_for_descriptors() {
    use karakuri_console::control::ControlId;

    assert_eq!(
        ActionId::for_control(ControlId::Tracker, Some("Tap the beat")),
        Some(ActionId::TapBeat)
    );
    assert_eq!(
        ActionId::for_control(ControlId::Arrangement, Some("Reset the arrangement")),
        Some(ActionId::Reset)
    );
    assert_eq!(
        ActionId::for_control(
            ControlId::InspectorPaneKeep,
            Some("Keep what a deck is playing")
        ),
        Some(ActionId::Save)
    );
    assert_eq!(
        ActionId::for_control(ControlId::MixerStrip, Some("Mute a deck")),
        Some(ActionId::ToggleMute)
    );
    assert_eq!(
        ActionId::for_control(ControlId::MixerStrip, Some("Solo a deck")),
        Some(ActionId::ToggleSolo)
    );
}
