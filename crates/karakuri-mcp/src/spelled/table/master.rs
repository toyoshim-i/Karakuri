use serde_json::{json, Value};

use super::super::*;

pub(crate) const MASTER: &[Spelled] = &[
    Spelled {
        sample: || (Operation::SetMasterOut { out: 1.0 }, json!({ "out": 1.0 })),
        make: Some(|with, _| {
            Ok(Operation::SetMasterOut {
                out: f32_of(with, "out")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({ "out": p_number("one level at the entry to the master chain, floored at zero and unbounded above 1.0") }),
                &["out"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::SetChainParam {
                    at: 0,
                    param: karakuri_operation::ChainParam::Declared {
                        key: "amount".to_owned(),
                        value: 0.5,
                    },
                },
                json!({ "at": 0, "key": "amount", "value": 0.5 }),
            )
        },
        // A key and a value, or a cut, and never both: a slot holds the values
        // its procedure declares and — where that procedure declares `retains`
        // — the cut it reads. A call saying both is refused with the two
        // spellings named.
        make: Some(|with, _| {
            Ok(Operation::SetChainParam {
                at: u32_of(with, "at")?,
                param: chain_param_of(with)?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "at": p_int("which slot of the master chain, by its position — 0 is the slot the mix's frame is handed to"),
                    "key": p_string("a parameter the slot's procedure declares, by the name it declares — with `value`, and never with `cut`"),
                    "value": p_number("what that parameter is set to, inside the range the procedure declares for it"),
                    "cut": p_word(
                        words(&karakuri_operation::Cut::ALL, karakuri_operation::Cut::name),
                        "which retained frame the slot reads, on a slot whose procedure declares `retains` — `mix` is one echo and `exit` is a trail",
                    ),
                }),
                &["at"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::AddChainEffect {
                    procedure:
                        "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                            .to_owned(),
                    cut: None,
                },
                json!({ "procedure": "sha256:0000000000000000000000000000000000000000000000000000000000000000" }),
            )
        },
        // A slot costs what its procedure costs from the frame it is added on,
        // however its parameters are set: a chain's price is the sum over its
        // slots and not a function of the values in them (`docs/adr/0340-…`,
        // §5). This is the one chain call that changes what a frame costs, and
        // the audit stands in front of it.
        make: Some(|with, _| {
            Ok(Operation::AddChainEffect {
                procedure: address_of(with, "procedure")?,
                cut: match with.get("cut") {
                    None | Some(Value::Null) => None,
                    Some(_) => Some(word_of(
                        with,
                        "cut",
                        &karakuri_operation::Cut::ALL,
                        karakuri_operation::Cut::name,
                        "a cut of the previous frame",
                    )?),
                },
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "procedure": p_string("the content address of a `kind L5` procedure's source, as a record spells one — `sha256:…`"),
                    "cut": p_word(
                        words(&karakuri_operation::Cut::ALL, karakuri_operation::Cut::name),
                        "which retained frame the slot reads — said exactly where the procedure declares `retains`, and left out where it does not",
                    ),
                }),
                &["procedure"],
            )
        }),
    },
    Spelled {
        sample: || (Operation::RemoveChainEffect { at: 1 }, json!({ "at": 1 })),
        make: Some(|with, _| {
            Ok(Operation::RemoveChainEffect {
                at: u32_of(with, "at")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "at": p_int("which slot of the master chain, by its position — the slots after it move up"),
                }),
                &["at"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::SetTonemap {
                    tonemap: karakuri_operation::Tonemap::Aces,
                },
                json!({ "tonemap": "aces" }),
            )
        },
        make: Some(|with, _| {
            Ok(Operation::SetTonemap {
                tonemap: word_of(
                    with,
                    "tonemap",
                    &TONEMAPS,
                    karakuri_operation::Tonemap::name,
                    "a transfer",
                )?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "tonemap": p_word(
                        words(&TONEMAPS, karakuri_operation::Tonemap::name),
                        "the transfer from unbounded linear HDR to something a display can show",
                    ),
                }),
                &["tonemap"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::SetExposure { exposure: 1.0 },
                json!({ "exposure": 1.0 }),
            )
        },
        make: Some(|with, _| {
            Ok(Operation::SetExposure {
                exposure: f32_of(with, "exposure")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({ "exposure": p_number("the level going into that transfer") }),
                &["exposure"],
            )
        }),
    },
];
