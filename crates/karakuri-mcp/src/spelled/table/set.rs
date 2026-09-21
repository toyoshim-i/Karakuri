use serde_json::{json, Value};

use super::super::*;

pub(crate) const SET: &[Spelled] = &[
    Spelled {
        sample: || {
            (
                Operation::WriteParam {
                    deck: 0,
                    param: karakuri_operation::ParamAt {
                        node: None,
                        key: "radius".into(),
                    },
                    value: karakuri_operation::ParamValue::Scalar(1.0),
                },
                json!({ "deck": 0, "param": { "key": "radius" }, "value": 1.0 }),
            )
        },
        make: Some(|with, slots| {
            Ok(Operation::WriteParam {
                deck: deck_of(with, "deck", slots)?,
                param: param_of(with, "param")?,
                value: value_of(with, "value")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "deck": p_deck(),
                    "param": {
                        "type": "object",
                        "description": "which parameter. A `node` left out is every node of the Set that declares this key, which is what `--param exposure=2.0` means",
                        "properties": { "key": p_string("the parameter's own name inside the node"), "node": p_node("one node of the Set") },
                        "required": ["key"],
                        "additionalProperties": false,
                    },
                    "value": json!({
                        "description": "a number, or an array of two or three — the three widths a `.kir` can declare. Never a range: a range is the procedure's declaration",
                    }),
                }),
                &["deck", "param", "value"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::AttachSignal {
                    deck: 0,
                    param: karakuri_operation::BindAt {
                        layer: karakuri_operation::Layer::L4,
                        index: None,
                        key: "radius".into(),
                    },
                    signal: "energy".into(),
                    curve: karakuri_operation::Curve::Lin,
                    range: [0.0, 1.0],
                },
                json!({
                    "deck": 0,
                    "param": { "layer": "L4", "key": "radius" },
                    "signal": "energy",
                    "curve": "lin",
                    "range": [0.0, 1.0],
                }),
            )
        },
        make: Some(|with, slots| {
            Ok(Operation::AttachSignal {
                deck: deck_of(with, "deck", slots)?,
                param: bind_of(with, "param")?,
                signal: named_of(with, "signal", "a signal")?,
                curve: word_of(
                    with,
                    "curve",
                    &karakuri_operation::Curve::ALL,
                    karakuri_operation::Curve::name,
                    "a curve",
                )?,
                range: range_of(with, "range")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "deck": p_deck(),
                    "param": {
                        "type": "object",
                        "description": "which parameter, as an attachment addresses one: the layer is always said, and an `index` left out is every node of that layer declaring the key",
                        "properties": {
                            "layer": { "type": "string", "enum": layer_words() },
                            "index": p_int("which node of that layer"),
                            "key": p_string("the parameter's name, and a component key such as `glow.x` where it is a vector"),
                        },
                        "required": ["layer", "key"],
                        "additionalProperties": false,
                    },
                    "signal": p_string("what drives it — a signal on the bus, or `control:NAME` for a published control"),
                    "curve": p_word(
                        words(&karakuri_operation::Curve::ALL, karakuri_operation::Curve::name),
                        "how the signal is shaped on its way to the parameter",
                    ),
                    "range": json!({
                        "type": "array",
                        "description": "`[low, high]`: what the signal is mapped between",
                    }),
                }),
                &["deck", "param", "signal", "curve", "range"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::TakeParamBack {
                    deck: 0,
                    param: karakuri_operation::BindAt {
                        layer: karakuri_operation::Layer::L4,
                        index: None,
                        key: "radius".into(),
                    },
                },
                json!({ "deck": 0, "param": { "layer": "L4", "key": "radius" } }),
            )
        },
        make: Some(|with, slots| {
            Ok(Operation::TakeParamBack {
                deck: deck_of(with, "deck", slots)?,
                param: bind_of(with, "param")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "deck": p_deck(),
                    "param": {
                        "type": "object",
                        "description": "the attachment to remove, addressed as `Attach a signal to a parameter` addresses one",
                        "properties": {
                            "layer": { "type": "string", "enum": layer_words() },
                            "index": p_int("which node of that layer"),
                            "key": p_string("the parameter's name"),
                        },
                        "required": ["layer", "key"],
                        "additionalProperties": false,
                    },
                }),
                &["deck", "param"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::WireInput {
                    deck: 0,
                    node: String::new(),
                    slot: InputPort(String::new()),
                    to: String::new(),
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
                Operation::Publish {
                    deck: 0,
                    controls: vec![karakuri_operation::Control {
                        name: "glow".to_string(),
                        node: Some(NodeAddress {
                            layer: karakuri_operation::Layer::L4,
                            index: 0,
                        }),
                        key: "glow".to_string(),
                        range: [0.0, 1.0],
                    }],
                },
                json!({
                    "deck": 0,
                    "controls": [
                        { "name": "glow", "node": { "layer": "L4" }, "key": "glow", "range": [0.0, 1.0] },
                    ],
                }),
            )
        },
        // Replaces the published interface atomically; an empty array publishes all declared controls.
        make: Some(|with, slots| {
            let deck = deck_of(with, "deck", slots)?;
            let listed = with.get("controls").and_then(Value::as_array).ok_or(
                "`with.controls` is required and is the whole published interface as a list, \
                 in the order it is drawn — `[]` publishes every control the deck declares, \
                 which is what an unnarrowed deck is. `read_set` names what one declares",
            )?;
            let mut controls = Vec::with_capacity(listed.len());
            for control in listed {
                controls.push(karakuri_operation::Control {
                    name: named_of(control, "name", "a published control")?,
                    // Absent is the wildcard — the control matches by key
                    // wherever it is declared, which is what the default
                    // interface is made of.
                    node: match control.get("node") {
                        Some(Value::Null) | None => None,
                        Some(_) => Some(node_of(control, "node")?),
                    },
                    key: named_of(control, "key", "a parameter key")?,
                    range: range_of(control, "range")?,
                });
            }
            Ok(Operation::Publish { deck, controls })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "deck": p_deck(),
                    "controls": {
                        "type": "array",
                        "description": "the whole published interface, in the order it is drawn — `[]` publishes every control the deck declares. A knob is learned against a position in this list, so reordering it moves what a mapped control reaches",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": p_string("what the console shows"),
                                "node": p_node("which node declares it; left out for a control that matches by key wherever it is declared, which is what the default interface is made of"),
                                "key": p_string("the parameter key it writes"),
                                "range": {
                                    "type": "array",
                                    "items": { "type": "number" },
                                    "minItems": 2,
                                    "maxItems": 2,
                                    "description": "low and high, which narrow the declared range and never redefine it",
                                },
                            },
                            "required": ["name", "key", "range"],
                            "additionalProperties": false,
                        },
                    },
                }),
                &["deck", "controls"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::SetProperty {
                    deck: 0,
                    property: karakuri_operation::Property::Capacity { elements: 65536 },
                },
                json!({ "deck": 0, "property": { "capacity": 65536 } }),
            )
        },
        make: Some(|with, slots| {
            let deck = deck_of(with, "deck", slots)?;
            let at = with.get("property").ok_or(
                "`with.property` is required and is either `{\"capacity\": N}` — how many \
                 elements this slot's geometries run at — or `{\"seed\": N}`, the salt its \
                 randomness comes from",
            )?;
            let property = match (at.get("capacity"), at.get("seed")) {
                (Some(Value::Null) | None, Some(Value::Null) | None) => {
                    return Err("`with.property` says neither `capacity` nor `seed`".into())
                }
                (Some(_), Some(_)) => {
                    return Err(
                        "`with.property` says both `capacity` and `seed`, which are two \
                         answers to which property — say one"
                            .into(),
                    )
                }
                (Some(_), _) => karakuri_operation::Property::Capacity {
                    elements: u32_of(at, "capacity")?,
                },
                (_, Some(_)) => karakuri_operation::Property::Seed {
                    salt: u32_of(at, "seed")?,
                },
            };
            Ok(Operation::SetProperty { deck, property })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "deck": p_deck(),
                    "property": {
                        "type": "object",
                        "description": "either `capacity`, how many elements this slot's geometries run at — a slot that already runs at it is refused, because it would buy a recompile and land on the same picture — or `seed`, the salt its randomness comes from",
                        "properties": {
                            "capacity": p_int("the element count"),
                            "seed": p_int("the salt"),
                        },
                        "additionalProperties": false,
                    },
                }),
                &["deck", "property"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::SetAuthority {
                    deck: 0,
                    node: NodeAddress {
                        layer: karakuri_operation::Layer::L4,
                        index: 0,
                    },
                    authority: karakuri_operation::Authority::Manual,
                },
                json!({ "deck": 0, "node": { "layer": "L4" }, "authority": "manual" }),
            )
        },
        make: Some(|with, slots| {
            Ok(Operation::SetAuthority {
                deck: deck_of(with, "deck", slots)?,
                node: node_of(with, "node")?,
                authority: word_of(
                    with,
                    "authority",
                    &AUTHORITIES,
                    karakuri_operation::Authority::name,
                    "an authority",
                )?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "deck": p_deck(),
                    "node": p_node("which node the authority is over"),
                    "authority": p_word(words(&AUTHORITIES, karakuri_operation::Authority::name), "who may move this node's parameters"),
                }),
                &["deck", "node", "authority"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::KeepProcedure {
                    deck: 0,
                    node: NodeAddress {
                        layer: karakuri_operation::Layer::L4,
                        index: 0,
                    },
                    id: None,
                },
                json!({ "deck": 0, "node": { "layer": "L4" } }),
            )
        },
        make: Some(|with, slots| {
            Ok(Operation::KeepProcedure {
                deck: deck_of(with, "deck", slots)?,
                node: node_of(with, "node")?,
                // Validates user-supplied procedure ID; omits explicit timestamp to use server-generated stamps.
                id: match with.get("id") {
                    None | Some(Value::Null) => None,
                    Some(_) => Some(checked_id(text_of(with, "id")?)?),
                },
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "deck": p_deck(),
                    "node": p_node("which node's source to keep — a head standing over several nodes has none, and the built-in camera is a node with no procedure behind it"),
                    "id": p_string("what to file it under, or leave it out for the time you kept it. A keep asked for here lands in `<store>/sandbox/` rather than in the operator's library, and never overwrites a name already there"),
                }),
                &["deck", "node"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::PointPane {
                    pane: "inspector-1".to_string(),
                    deck: 0,
                },
                Value::Null,
            )
        },
        make: None,
        shape: None,
    },
];
