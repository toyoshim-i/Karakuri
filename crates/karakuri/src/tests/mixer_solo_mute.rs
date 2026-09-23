//! Verification of the two-layer SOLO/MUTE state architecture:
//! - Lower layer: Slot-level `online / offline` flag and mix arbitration in `Deck`.
//! - High layer (Mixer controller): SOLO / MUTE operations dispatched to slots.
//! - Unidirectional data flow: User interactions (keys, MCP, MIDI) emit operations,
//!   engine mutates state, increments revision, and marks `"mixer"` bay dirty for next frame redraw.
//! - Bay-local dirty redraw: Revision change triggers `mark_mixer_dirty()` declaring `Duration::ZERO`.

use std::time::Duration;

use super::*;
use crate::keymap::{BoundKey, KEY_BINDINGS};

mod gpu {
    use super::super::tests::shipped_slots;
    use super::*;
    use crate::bridge::{apply, mixer};
    use karakuri_console::panel::Panel;
    use karakuri_engine::{DeckSlot as EngineSlot, Gpu};
    use karakuri_store::record::{DeckSlot, Record};

    fn setup_test_engine() -> (Engine, egui_wgpu::Renderer, Gpu, Panel) {
        let gpu = Gpu::headless().expect("no GPU");
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        let mut renderer =
            egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());
        let mut panel = Panel::new(1440.0, 900.0);
        panel.solve();
        let engine = Engine::new(
            &gpu,
            &mut renderer,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            &shipped_slots(),
            panel.layout(),
            1.0,
            None,
            None,
            karakuri_mcp::Slots::unpointed(),
        );
        (engine, renderer, gpu, panel)
    }

    #[test]
    fn test_deck_solo_mute_two_layer_arbitration() {
        let (mut engine, _, _, _) = setup_test_engine();
        let deck = &mut engine.deck;
        let count = deck.slot_count();
        assert!(count >= 2, "deck should have at least 2 slots");

        // Ensure all test slots are Live with full opacity and unmuted for mixing assertions
        for i in 0..count {
            let slot = EngineSlot(i as u8);
            deck.set_residency(slot, karakuri_engine::Residency::Live);
            deck.set_mute(slot, false);
            deck.set_opacity(slot, 1.0);
        }

        // Initial state: all slots online, unmuted, no solo active
        for i in 0..count {
            let slot = EngineSlot(i as u8);
            assert!(deck.is_online(slot), "slot {i} should initially be online");
            assert!(!deck.is_muted(slot), "slot {i} should initially be unmuted");
            assert!(
                !deck.is_soloed(slot),
                "slot {i} should initially not be soloed"
            );
            assert!(deck.is_in_mix(slot), "slot {i} should initially be in mix");
        }
        assert_eq!(deck.solo(), None);
        let rev0 = deck.mixer_revision();

        // 1. Mute deck 0: sets slot 0 offline
        assert!(deck.toggle_mute(EngineSlot(0)));
        assert!(deck.is_muted(EngineSlot(0)));
        assert!(!deck.is_online(EngineSlot(0)));
        assert!(!deck.is_in_mix(EngineSlot(0)));
        assert!(deck.mixer_revision() > rev0);
        let rev1 = deck.mixer_revision();

        // Other slots remain online
        for i in 1..count {
            let slot = EngineSlot(i as u8);
            assert!(deck.is_online(slot));
            assert!(deck.is_in_mix(slot));
        }

        // 2. Solo deck 1 (exclusive solo):
        // Only deck 1 is online, all other slots are offline
        assert!(deck.toggle_solo(EngineSlot(1)));
        assert_eq!(deck.solo(), Some(1));
        assert!(deck.is_soloed(EngineSlot(1)));
        assert!(deck.mixer_revision() > rev1);
        let rev2 = deck.mixer_revision();

        assert!(deck.is_online(EngineSlot(1)));
        assert!(deck.is_in_mix(EngineSlot(1)));
        assert!(!deck.is_online(EngineSlot(0)));
        for i in 2..count {
            let slot = EngineSlot(i as u8);
            assert!(!deck.is_online(slot));
            assert!(!deck.is_in_mix(slot));
        }

        // 3. Switch exclusive solo to deck 2 (if available)
        if count > 2 {
            assert!(deck.toggle_solo(EngineSlot(2)));
            assert_eq!(deck.solo(), Some(2));
            assert!(!deck.is_soloed(EngineSlot(1)));
            assert!(deck.is_soloed(EngineSlot(2)));
            assert!(deck.mixer_revision() > rev2);

            assert!(deck.is_online(EngineSlot(2)));
            assert!(deck.is_in_mix(EngineSlot(2)));
            assert!(!deck.is_online(EngineSlot(1)));
        }

        // 4. Clear solo: restores online states based on !muted
        deck.clear_solo();
        assert_eq!(deck.solo(), None);

        // Slot 0 was muted, so it stays offline
        assert!(deck.is_muted(EngineSlot(0)));
        assert!(!deck.is_online(EngineSlot(0)));
        assert!(!deck.is_in_mix(EngineSlot(0)));

        // Slots 1..count were not muted, so they return online
        for i in 1..count {
            let slot = EngineSlot(i as u8);
            assert!(!deck.is_muted(slot));
            assert!(deck.is_online(slot));
            assert!(deck.is_in_mix(slot));
        }

        // 5. Unmute deck 0: returns online since no solo is active
        assert!(!deck.toggle_mute(EngineSlot(0)));
        assert!(!deck.is_muted(EngineSlot(0)));
        assert!(deck.is_online(EngineSlot(0)));
        assert!(deck.is_in_mix(EngineSlot(0)));
    }

    #[test]
    fn test_apply_mute_solo_online_records() {
        let (mut engine, _, _, _) = setup_test_engine();
        let deck = &mut engine.deck;
        let look = &mut engine.look;
        let mut chain = Vec::new();
        for i in 0..deck.slot_count() {
            deck.set_mute(EngineSlot(i as u8), false);
        }

        // Apply Record::Mute
        let log = apply(
            &Record::Mute {
                slot: DeckSlot(1),
                muted: true,
            },
            deck,
            look,
            &mut chain,
        );
        assert!(log.is_some());
        assert!(deck.is_muted(EngineSlot(1)));
        assert!(!deck.is_online(EngineSlot(1)));

        // Apply Record::Solo
        let log = apply(
            &Record::Solo {
                slot: DeckSlot(2),
                soloed: true,
            },
            deck,
            look,
            &mut chain,
        );
        assert!(log.is_some());
        assert_eq!(deck.solo(), Some(2));
        assert!(deck.is_online(EngineSlot(2)));
        assert!(!deck.is_online(EngineSlot(0)));
        assert!(!deck.is_online(EngineSlot(1)));

        // Apply Record::Solo (un-solo)
        let log = apply(
            &Record::Solo {
                slot: DeckSlot(2),
                soloed: false,
            },
            deck,
            look,
            &mut chain,
        );
        assert!(log.is_some());
        assert_eq!(deck.solo(), None);
        // Slot 1 is still muted, so remains offline
        assert!(!deck.is_online(EngineSlot(1)));
        // Slot 0 and 2 are unmuted, so online
        assert!(deck.is_online(EngineSlot(0)));
        assert!(deck.is_online(EngineSlot(2)));

        // Apply Record::Online
        let log = apply(
            &Record::Online {
                slot: DeckSlot(0),
                online: false,
            },
            deck,
            look,
            &mut chain,
        );
        assert!(log.is_some());
        assert!(!deck.is_online(EngineSlot(0)));
    }

    #[test]
    fn test_bridge_mixer_populates_strips_from_deck() {
        let (mut engine, _, _, _) = setup_test_engine();
        let deck = &mut engine.deck;
        for i in 0..deck.slot_count() {
            deck.set_mute(EngineSlot(i as u8), false);
        }
        deck.set_mute(EngineSlot(0), true);
        deck.set_solo(EngineSlot(2), true);

        let mut strips = Vec::new();
        let names = vec![
            "Alpha".to_string(),
            "Beta".to_string(),
            "Gamma".to_string(),
            "Delta".to_string(),
        ];
        mixer(deck, &names, &mut strips);

        assert_eq!(strips.len(), deck.slot_count());
        assert_eq!(strips[0].name, "Alpha");
        assert!(strips[0].is_muted);
        assert!(!strips[0].is_soloed);

        assert_eq!(strips[1].name, "Beta");
        assert!(!strips[1].is_muted);
        assert!(!strips[1].is_soloed);

        assert_eq!(strips[2].name, "Gamma");
        assert!(!strips[2].is_muted);
        assert!(strips[2].is_soloed);

        assert_eq!(strips[3].name, "Delta");
        assert!(!strips[3].is_muted);
        assert!(!strips[3].is_soloed);
    }

    #[test]
    fn test_initial_startup_slots_one_to_three_muted() {
        let (engine, _, _, _) = setup_test_engine();
        let deck = &engine.deck;
        let count = deck.slot_count();
        assert!(count >= 4);

        // Deck A (slot 0) starts unmuted and online in mix
        let slot0 = EngineSlot(0);
        assert!(!deck.is_muted(slot0), "slot 0 should start unmuted");
        assert!(deck.is_online(slot0), "slot 0 should start online");
        assert!(deck.is_in_mix(slot0), "slot 0 should start in mix");

        // Decks B, C, D (slots 1..3) start muted and offline
        for i in 1..count {
            let slot = EngineSlot(i as u8);
            assert!(deck.is_muted(slot), "slot {i} should start muted");
            assert!(!deck.is_online(slot), "slot {i} should start offline");
            assert!(!deck.is_in_mix(slot), "slot {i} should not start in mix");
        }
    }
}

#[test]
fn test_mixer_key_bindings_and_routing() {
    // Verify KEY_BINDINGS routing table carries bay: Some("mixer") for m, s, u
    let binding_m = KEY_BINDINGS
        .iter()
        .find(|b| matches!(b.key, BoundKey::Character("m")))
        .expect("m binding must exist");
    assert_eq!(binding_m.bay, Some("mixer"));

    let binding_s = KEY_BINDINGS
        .iter()
        .find(|b| matches!(b.key, BoundKey::Character("s")))
        .expect("s binding must exist");
    assert_eq!(binding_s.bay, Some("mixer"));

    let binding_u = KEY_BINDINGS
        .iter()
        .find(|b| matches!(b.key, BoundKey::Character("u")))
        .expect("u binding must exist");
    assert_eq!(binding_u.bay, Some("mixer"));
}

#[test]
fn test_dirty_notification_marks_mixer_dirty() {
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();

    // Mark mixer dirty
    readout.view.mark_mixer_dirty();

    // Mixer bay declares immediate repaint moves_in: Duration::ZERO
    let mixer_decl = readout
        .view
        .declares(readout.panel.layout())
        .find(|d| d.region == "mixer");
    assert!(
        mixer_decl.is_some(),
        "mixer declaration must exist when mixer bay is laid out"
    );
    assert_eq!(
        mixer_decl.unwrap().moves_in,
        Duration::ZERO,
        "dirty mixer bay must request immediate repaint with Duration::ZERO"
    );
}
