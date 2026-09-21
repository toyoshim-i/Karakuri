use serde_json::{json, Value};

use super::super::*;

pub(crate) const PROCEDURES: &[Spelled] = &[
    Spelled {
        sample: || {
            (
                Operation::ReadProcedure {
                    deck: 0,
                    node: NodeAddress {
                        layer: karakuri_operation::Layer::L4,
                        index: 0,
                    },
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
                Operation::WriteProcedure {
                    deck: 0,
                    node: NodeAddress {
                        layer: karakuri_operation::Layer::L4,
                        index: 0,
                    },
                    source: String::new(),
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
                Operation::WatchFiles {
                    watching: karakuri_operation::Undecided,
                },
                Value::Null,
            )
        },
        make: None,
        shape: None,
    },
    Spelled {
        sample: || (Operation::SwapOutcome, Value::Null),
        make: None,
        shape: None,
    },
    Spelled {
        sample: || {
            (
                Operation::KeepCandidate {
                    deck: 0,
                    node: NodeAddress {
                        layer: karakuri_operation::Layer::L4,
                        index: 0,
                    },
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
                Operation::RestoreProcedure {
                    deck: 0,
                    revision: karakuri_operation::Revision::Previous(NodeAddress {
                        layer: karakuri_operation::Layer::L4,
                        index: 0,
                    }),
                },
                json!({ "deck": 0, "revision": { "previous": { "layer": "L4" } } }),
            )
        },
        make: Some(|with, slots| {
            Ok(Operation::RestoreProcedure {
                deck: deck_of(with, "deck", slots)?,
                revision: revision_of(with, "revision")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "deck": p_deck(),
                    "revision": {
                        "type": "object",
                        "description": "either `previous`, naming the node whose present source is to be replaced by the one it replaced, or `picked`, a version by the name the store filed it under — which is the name `write_procedure` hands back",
                        "properties": {
                            "previous": p_node("the node to step back one version on"),
                            "picked": json!({ "type": "string", "pattern": ID_PATTERN, "description": "a filed version's name" }),
                        },
                        "additionalProperties": false,
                    },
                }),
                &["deck", "revision"],
            )
        }),
    },
];
