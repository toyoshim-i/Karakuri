use serde_json::Value;

use super::super::*;

pub(crate) const ARRANGEMENT: &[Spelled] = &[
    Spelled {
        sample: || {
            (
                Operation::MoveBoundary {
                    boundary: karakuri_operation::Undecided,
                },
                Value::Null,
            )
        },
        make: None,
        shape: None,
    },
    Spelled {
        sample: || (Operation::FoldBay { bay: String::new() }, Value::Null),
        make: None,
        shape: None,
    },
    Spelled {
        sample: || {
            (
                Operation::FoldPane {
                    pane: String::new(),
                },
                Value::Null,
            )
        },
        make: None,
        shape: None,
    },
    Spelled {
        sample: || (Operation::Unfold { region: None }, Value::Null),
        make: None,
        shape: None,
    },
    Spelled {
        sample: || (Operation::Solo { region: None }, Value::Null),
        make: None,
        shape: None,
    },
    Spelled {
        sample: || (Operation::ResetArrangement, Value::Null),
        make: None,
        shape: None,
    },
    Spelled {
        sample: || {
            (
                Operation::SaveArrangement {
                    name: String::new(),
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
                Operation::RestoreArrangement {
                    name: String::new(),
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
                Operation::SizeWindow {
                    width: 0,
                    height: 0,
                },
                Value::Null,
            )
        },
        make: None,
        shape: None,
    },
];
