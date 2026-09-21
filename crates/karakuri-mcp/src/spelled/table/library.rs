use serde_json::{json, Value};

use super::super::*;

pub(crate) const LIBRARY: &[Spelled] = &[
    Spelled {
        sample: || (Operation::SaveSet { deck: 0, id: None }, Value::Null),
        make: None,
        shape: None,
    },
    Spelled {
        sample: || {
            (
                Operation::ListSets {
                    holds: None,
                    layer: None,
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
                Operation::FilterLibrary {
                    kinds: karakuri_operation::LibraryKinds::EVERYTHING,
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
                Operation::SelectScope {
                    scope: karakuri_operation::Undecided,
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
                Operation::SetFavourite {
                    id: "a_set".to_string(),
                    favourite: true,
                },
                json!({ "set": "a_set", "favourite": true }),
            )
        },
        // **Taken, and answered with a refusal.** `Standing::Open` and the
        // gate are untouched (ADR-0301): a model's star is refused by the
        // *performer*, in a sentence that names the id and says where the Set
        // is, so what a model gets is an answer it can hand to the person
        // sitting there rather than a name this tool does not know.
        make: Some(|with, _| {
            Ok(Operation::SetFavourite {
                id: checked_id(text_of(with, "set")?)?,
                favourite: bool_of(with, "favourite")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "set": {
                        "type": "string",
                        "pattern": ID_PATTERN,
                        "description": "a Set id this store holds, as `list_sets` names them",
                    },
                    "favourite": p_bool("whether this Set is under `my sets`. It names the state rather than flipping one. A model's star is refused: `my sets` is the list of Sets the operator chose, and the refusal says where the Set is"),
                }),
                &["set", "favourite"],
            )
        }),
    },
    Spelled {
        sample: || (Operation::ReadSet { id: String::new() }, Value::Null),
        make: None,
        shape: None,
    },
    Spelled {
        sample: || {
            (
                Operation::TransferSet {
                    transfer: karakuri_operation::SetTransfer::Send { id: String::new() },
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
                // **A tool of its own, so `operate` does not spell it** — the
                // sample is here to name the row and the schema is
                // `walk_history`'s own, beside `read_set`'s and `list_sets`'.
                Operation::WalkHistory {
                    set: Some("night01".to_string()),
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
                Operation::LoadProcedure {
                    deck: 0,
                    procedure: "orbit_wide".to_string(),
                },
                json!({ "deck": 0, "procedure": "orbit_wide" }),
            )
        },
        // **`LoadSet`'s spelling with the name in the other tier**, which is
        // what this operation is: a deck, and one procedure of the library
        // written over the layer it declares (ADR-0338). `checked_id` for the
        // same reason that row takes it — a procedure is filed under its name,
        // so the name becomes a file name and a separator is refused here
        // rather than resolved anywhere.
        //
        // **Which node it lands on is not in the payload and is not missing
        // from it**: a procedure declares one kind and the load takes the
        // first node of that kind, which is the limit the page writes down.
        make: Some(|with, slots| {
            Ok(Operation::LoadProcedure {
                deck: deck_of(with, "deck", slots)?,
                procedure: checked_id(text_of(with, "procedure")?)?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "deck": p_deck(),
                    "procedure": json!({
                        "type": "string",
                        "pattern": ID_PATTERN,
                        "description": "a procedure of the library, by the name it is filed under — the layer it lands on is the kind it declares, and it takes the first node of that kind",
                    }),
                }),
                &["deck", "procedure"],
            )
        }),
    },
];
