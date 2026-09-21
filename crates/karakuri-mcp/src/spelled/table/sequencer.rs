use serde_json::Value;

use super::super::*;

pub(crate) const SEQUENCER: &[Spelled] = &[
    Spelled {
        sample: || {
            (
                Operation::SetStep {
                    pattern: 0,
                    lane: 0,
                    step: 0,
                    on: false,
                },
                Value::Null,
            )
        },
        make: None,
        shape: None,
    },
    Spelled {
        sample: || {
            (
                Operation::SetLaneMute {
                    pattern: 0,
                    lane: 0,
                    muted: false,
                },
                Value::Null,
            )
        },
        make: None,
        shape: None,
    },
    Spelled {
        sample: || {
            (
                Operation::PointLane {
                    pattern: 0,
                    target: karakuri_operation::LaneTarget::Fader { deck: 0 },
                },
                Value::Null,
            )
        },
        make: None,
        shape: None,
    },
    Spelled {
        sample: || {
            (
                Operation::RemoveLane {
                    pattern: 0,
                    lane: 0,
                },
                Value::Null,
            )
        },
        make: None,
        shape: None,
    },
    Spelled {
        sample: || {
            (
                Operation::SetPatternGrid {
                    pattern: 0,
                    grid: karakuri_operation::StepMode::Sixteenth,
                },
                Value::Null,
            )
        },
        make: None,
        shape: None,
    },
    Spelled {
        sample: || (Operation::SelectPattern { pattern: 0 }, Value::Null),
        make: None,
        shape: None,
    },
];
