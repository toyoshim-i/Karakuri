//! Tests for Deck channel strip SOLO, MUTE, and mix contribution logic.

mod gpu {
    use karakuri_engine::deck::{Deck, DeckSlot};
    use karakuri_engine::swap::HotSwap;
    use karakuri_engine::{Gpu, Set};
    use karakuri_ir::typed::Checked;

    const WIDTH: u32 = 64;
    const HEIGHT: u32 = 64;
    const CAPACITY: u32 = 4096;

    const L1: &str = r#"
proc static_shell {
  kind     L1
  topology points
  capacity [1024, 262144] = 4096

  param radius : float [0.1, 8.0] = 2.5

  emit position, age

  element {
    let u = hash1(seed);
    let v = hash1(seed + 1000u);
    position = rot_y(sphere_point(u, v) * radius, t * 0.3);
    age      = age + dt;
  }
}
"#;

    const L4: &str = r#"
proc soft_points {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 1.0

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.015625;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    color = vec4(vec3(1.0, 1.0, 1.0) * exposure, max(0.0, 1.0 - d));
  }
}
"#;

    fn compile(src: &str) -> Checked {
        let proc = karakuri_ir::parse(src).expect("valid ir");
        let checked = karakuri_ir::check::check(&proc).expect("typechecked ir");
        karakuri_ir::cost::estimate(&checked).expect("estimated cost");
        checked
    }

    fn test_deck(gpu: &Gpu, count: usize) -> Deck {
        let l1 = compile(L1);
        let l4 = compile(L4);
        let swaps: Vec<HotSwap> = (0..count)
            .map(|seed| {
                let mut set = Set::build(&gpu.device, &gpu.queue, &l1, &l4, CAPACITY, seed as u32)
                    .expect("builds valid test Set");
                set.resize(&gpu.device, WIDTH, HEIGHT);
                HotSwap::fixed(set)
            })
            .collect();
        Deck::new(&gpu.device, swaps, WIDTH, HEIGHT)
    }

    #[test]
    fn mute_removes_slot_from_mix() {
        let gpu = Gpu::headless().expect("headless gpu");
        let mut deck = test_deck(&gpu, 2);

        let slot0 = DeckSlot(0);
        let slot1 = DeckSlot(1);

        assert!(deck.is_in_mix(slot0));
        assert!(deck.is_in_mix(slot1));

        // Muting slot 0 removes it from mix
        assert!(deck.toggle_mute(slot0));
        assert!(deck.is_muted(slot0));
        assert!(!deck.is_in_mix(slot0));
        assert!(deck.is_in_mix(slot1));

        // Unmuting slot 0 restores it
        assert!(!deck.toggle_mute(slot0));
        assert!(!deck.is_muted(slot0));
        assert!(deck.is_in_mix(slot0));
    }

    #[test]
    fn solo_isolates_slot_in_mix() {
        let gpu = Gpu::headless().expect("headless gpu");
        let mut deck = test_deck(&gpu, 3);

        let slot0 = DeckSlot(0);
        let slot1 = DeckSlot(1);
        let slot2 = DeckSlot(2);

        assert!(!deck.any_soloed());
        assert!(deck.is_in_mix(slot0));
        assert!(deck.is_in_mix(slot1));
        assert!(deck.is_in_mix(slot2));

        // Soloing slot 1 isolates slot 1
        assert!(deck.toggle_solo(slot1));
        assert!(deck.is_soloed(slot1));
        assert!(deck.any_soloed());
        assert!(!deck.is_in_mix(slot0));
        assert!(deck.is_in_mix(slot1));
        assert!(!deck.is_in_mix(slot2));

        // Also soloing slot 2 cancels slot 1 solo and isolates slot 2 in mix
        deck.set_solo(slot2, true);
        assert!(!deck.is_soloed(slot1));
        assert!(deck.is_soloed(slot2));
        assert!(!deck.is_in_mix(slot0));
        assert!(!deck.is_in_mix(slot1));
        assert!(deck.is_in_mix(slot2));

        // Turning off solo restores all unmuted slots to mix
        deck.clear_solo();
        assert!(!deck.any_soloed());
        assert!(deck.is_in_mix(slot0));
        assert!(deck.is_in_mix(slot1));
        assert!(deck.is_in_mix(slot2));
    }

    #[test]
    fn opacity_and_online_govern_in_mix() {
        let gpu = Gpu::headless().expect("headless gpu");
        let mut deck = test_deck(&gpu, 2);

        let slot0 = DeckSlot(0);

        // Zero opacity is not in mix
        deck.set_opacity(slot0, 0.0);
        assert!(!deck.is_in_mix(slot0));

        deck.set_opacity(slot0, 0.5);
        assert!(deck.is_in_mix(slot0));

        // Offline slot is not in mix
        deck.set_online(slot0, false);
        assert!(!deck.is_in_mix(slot0));

        // Setting online brings it back into mix
        deck.set_online(slot0, true);
        assert!(deck.is_in_mix(slot0));
    }
}
