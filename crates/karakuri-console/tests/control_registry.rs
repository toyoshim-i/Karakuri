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
