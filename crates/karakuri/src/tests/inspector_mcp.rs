//! Inspector slot MCP policy pill press handling and policy cycling tests.

use karakuri_operation::SlotPolicy;

use super::*;
use crate::readout::{Acted, Readout};

/// The press handler dispatches pointer events on Inspector slot MCP policy pill.
#[test]
fn the_press_handler_dispatches_inspector_slot_mcp_pill() {
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();
    assert_eq!(readout.view.slot_policies[0], SlotPolicy::Auto);
    assert_eq!(readout.slot_policies.policy(0), SlotPolicy::Auto);

    let acted = readout.cycle_slot_policy(0);
    assert_eq!(acted, Acted::Opened);
    assert_eq!(readout.view.slot_policies[0], SlotPolicy::On);
    assert_eq!(readout.slot_policies.policy(0), SlotPolicy::On);

    let acted = readout.cycle_slot_policy(0);
    assert_eq!(acted, Acted::Opened);
    assert_eq!(readout.view.slot_policies[0], SlotPolicy::Off);
    assert_eq!(readout.slot_policies.policy(0), SlotPolicy::Off);

    let acted = readout.cycle_slot_policy(0);
    assert_eq!(acted, Acted::Opened);
    assert_eq!(readout.view.slot_policies[0], SlotPolicy::Auto);
    assert_eq!(readout.slot_policies.policy(0), SlotPolicy::Auto);
}
