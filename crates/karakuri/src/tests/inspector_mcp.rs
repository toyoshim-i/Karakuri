//! Inspector slot MCP policy pill and Library bay MCP gate pill press handling tests.

use karakuri_operation::gate::LibraryGate;
use karakuri_operation::SlotPolicy;

use crate::readout::{Acted, Readout};

/// The press handler dispatches pointer events on Inspector slot MCP policy pill.
#[test]
fn the_press_handler_dispatches_inspector_slot_mcp_pill() {
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    assert_eq!(readout.view.slot_policies[0], SlotPolicy::On);
    assert_eq!(readout.slot_policies.policy(0), SlotPolicy::On);

    let acted = readout.cycle_slot_policy(0);
    assert_eq!(acted, Acted::Opened);
    assert_eq!(readout.view.slot_policies[0], SlotPolicy::Auto);
    assert_eq!(readout.slot_policies.policy(0), SlotPolicy::Auto);

    let acted = readout.cycle_slot_policy(0);
    assert_eq!(acted, Acted::Opened);
    assert_eq!(readout.view.slot_policies[0], SlotPolicy::Off);
    assert_eq!(readout.slot_policies.policy(0), SlotPolicy::Off);

    let acted = readout.cycle_slot_policy(0);
    assert_eq!(acted, Acted::Opened);
    assert_eq!(readout.view.slot_policies[0], SlotPolicy::On);
    assert_eq!(readout.slot_policies.policy(0), SlotPolicy::On);
}

/// The press handler dispatches pointer events on Library bay MCP gate pill.
#[test]
fn the_press_handler_dispatches_library_mcp_pill() {
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    assert_eq!(readout.view.opening.library(), LibraryGate::Preset);
    assert_eq!(readout.opening.read().library(), LibraryGate::Preset);

    let acted = readout.cycle_library_gate();
    assert_eq!(acted, Acted::Opened);
    assert_eq!(readout.view.opening.library(), LibraryGate::All);
    assert_eq!(readout.opening.read().library(), LibraryGate::All);

    let acted = readout.cycle_library_gate();
    assert_eq!(acted, Acted::Opened);
    assert_eq!(readout.view.opening.library(), LibraryGate::Off);
    assert_eq!(readout.opening.read().library(), LibraryGate::Off);

    let acted = readout.cycle_library_gate();
    assert_eq!(acted, Acted::Opened);
    assert_eq!(readout.view.opening.library(), LibraryGate::Preset);
    assert_eq!(readout.opening.read().library(), LibraryGate::Preset);
}
