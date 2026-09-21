use serde_json::{json, Value};

use super::super::*;

pub(crate) const DECKS: &[Spelled] = &[
    Spelled {
        sample: || (Operation::SelectDeck { deck: 0 }, Value::Null),
        make: None,
        shape: None,
    },
    Spelled {
        sample: || {
            (
                Operation::SetResidency {
                    deck: 0,
                    residency: karakuri_operation::Residency::Live,
                },
                json!({ "deck": 0, "residency": "live" }),
            )
        },
        make: Some(|with, slots| {
            Ok(Operation::SetResidency {
                deck: deck_of(with, "deck", slots)?,
                residency: word_of(
                    with,
                    "residency",
                    &karakuri_operation::Residency::ALL,
                    karakuri_operation::Residency::name,
                    "a residency",
                )?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "deck": p_deck(),
                    "residency": p_word(
                        words(&karakuri_operation::Residency::ALL, karakuri_operation::Residency::name),
                        "on air, primed, or holding its allocation and drawing nothing",
                    ),
                }),
                &["deck", "residency"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::LoadSet {
                    deck: 0,
                    set: "a_set".into(),
                },
                json!({ "deck": 0, "set": "a_set" }),
            )
        },
        make: Some(|with, slots| {
            Ok(Operation::LoadSet {
                deck: deck_of(with, "deck", slots)?,
                set: checked_id(text_of(with, "set")?)?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "deck": p_deck(),
                    "set": json!({
                        "type": "string",
                        "pattern": ID_PATTERN,
                        "description": "a Set id this store holds, as `list_sets` names them",
                    }),
                }),
                &["deck", "set"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::SetCompositing {
                    deck: 0,
                    compositing: true,
                },
                json!({ "deck": 0, "compositing": true }),
            )
        },
        make: Some(|with, slots| {
            Ok(Operation::SetCompositing {
                deck: deck_of(with, "deck", slots)?,
                compositing: bool_of(with, "compositing")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "deck": p_deck(),
                    "compositing": p_bool("whether this deck's renderers are composited into one picture rather than overdrawn"),
                }),
                &["deck", "compositing"],
            )
        }),
    },
];
