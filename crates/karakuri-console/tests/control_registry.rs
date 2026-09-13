use karakuri_console::control::{
    descriptor_for, descriptor_for_hotkey, descriptor_for_probe, ControlId, DESCRIPTORS,
};
use karakuri_console::hover::TIPS;
use karakuri_console::input::PROBES;

#[test]
fn every_probe_matches_a_control_descriptor() {
    assert_eq!(PROBES.len(), 38);
    assert_eq!(TIPS.len(), 38);
    assert_eq!(ControlId::ALL.len(), 38);
    assert_eq!(DESCRIPTORS.len(), 38);

    for i in 0..38 {
        let probe = &PROBES[i];
        let descriptor = &DESCRIPTORS[i];
        let tip_probe_name = TIPS[i].0;
        let id = ControlId::ALL[i];

        assert_eq!(descriptor.id, id);
        assert_eq!(descriptor.probe_name, probe.name);
        assert_eq!(descriptor.probe_name, tip_probe_name);
        assert_eq!(descriptor.probe_name, id.probe_name());
        assert_eq!(id.probe_index(), i);
    }
}

#[test]
fn roundtrip_lookup_by_name_and_index() {
    for (i, &id) in ControlId::ALL.iter().enumerate() {
        assert_eq!(ControlId::from_probe_index(i), Some(id));
        assert_eq!(ControlId::from_probe_name(id.probe_name()), Some(id));
        assert_eq!(descriptor_for(id).id, id);
        assert_eq!(
            descriptor_for_probe(id.probe_name()).map(|d| d.id),
            Some(id)
        );
    }

    assert_eq!(ControlId::from_probe_index(38), None);
    assert_eq!(ControlId::from_probe_name("nonexistent"), None);
    assert_eq!(descriptor_for_probe("nonexistent"), None);
}

#[test]
fn hotkey_queries() {
    assert_eq!(
        descriptor_for_hotkey("r").map(|d| d.id),
        Some(ControlId::Arrangement)
    );
    assert_eq!(
        descriptor_for_hotkey("space").map(|d| d.id),
        Some(ControlId::Transition)
    );
    assert_eq!(
        descriptor_for_hotkey("g").map(|d| d.id),
        Some(ControlId::BayGrip)
    );
    assert_eq!(
        descriptor_for_hotkey("k").map(|d| d.id),
        Some(ControlId::InspectorPaneKeep)
    );
    assert_eq!(descriptor_for_hotkey("unknown_hotkey"), None);
}

#[test]
fn hover_flat_indices_map_to_control_ids_and_descriptors() {
    use karakuri_console::hover::{control_id_at, descriptor_at, flat};

    let total = flat().count();
    assert_eq!(total, 76);

    for i in 0..total {
        let id = control_id_at(i).expect("every flat tip index must map to a ControlId");
        let desc = descriptor_at(i).expect("every flat tip index must map to a ControlDescriptor");
        assert_eq!(desc.id, id);
    }
}

#[test]
fn hotkey_badge_formatting() {
    use karakuri_console::hover::format_hotkey_badge;

    assert_eq!(format_hotkey_badge("space"), "[Space]");
    assert_eq!(format_hotkey_badge("enter"), "[Enter]");
    assert_eq!(format_hotkey_badge("return"), "[Enter]");
    assert_eq!(format_hotkey_badge("tab"), "[Tab]");
    assert_eq!(format_hotkey_badge("esc"), "[Esc]");
    assert_eq!(format_hotkey_badge("k"), "[K]");
    assert_eq!(format_hotkey_badge("b"), "[B]");
    assert_eq!(format_hotkey_badge("r"), "[R]");
    assert_eq!(format_hotkey_badge("g"), "[G]");
    assert_eq!(format_hotkey_badge("z"), "[Z]");
}

#[test]
fn tooltip_with_hotkey_and_annotation() {
    use karakuri_console::hover::{annotate, with_hotkey};

    assert_eq!(with_hotkey("Keep", Some("k")), "[K] Keep");
    assert_eq!(with_hotkey("Plain", None), "Plain");

    let annotated = annotate("Wipe next deck", Some("space"), None);
    assert_eq!(annotated, "[Space] Wipe next deck");

    let mapped = "The trim. Click to drag. \u{2295} MIDI: cc → gain A, which sets it outright \
                  where a key steps it.";
    let annotated_midi = annotate(mapped, Some("b"), Some("cc 5"));
    assert!(annotated_midi.starts_with("[B] The trim. Click to drag."));
    assert!(
        annotated_midi.ends_with("\u{2295} MIDI: cc 5, which is what the map in use says today.")
    );
}

#[test]
fn hotkey_for_tip_resolves_accurately() {
    use karakuri_console::hover::{flat, hotkey_for_tip};

    for (i, tip) in flat().enumerate() {
        let hotkey = hotkey_for_tip(i);
        match tip.control {
            "the go capsule" => assert_eq!(hotkey, Some("space")),
            "the tap" => assert_eq!(hotkey, Some("b")),
            "the arrangement pill" => assert_eq!(hotkey, Some("r")),
            "the keep capsule" => assert_eq!(hotkey, Some("k")),
            "a bay grip" => assert_eq!(hotkey, Some("g")),
            "the load button" => assert_eq!(hotkey, Some("enter")),
            _ => {}
        }
    }
}
