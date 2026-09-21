use serde_json::{json, Value};

use super::super::*;

pub(crate) const MIXING: &[Spelled] = &[
    Spelled {
        sample: || {
            (
                Operation::SetGain { deck: 0, gain: 1.0 },
                json!({ "deck": 0, "gain": 1.0 }),
            )
        },
        make: Some(|with, slots| {
            Ok(Operation::SetGain {
                deck: deck_of(with, "deck", slots)?,
                gain: f32_of(with, "gain")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({ "deck": p_deck(), "gain": p_number("the trim: the level material arrives at, colour only") }),
                &["deck", "gain"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::SetOpacity {
                    deck: 0,
                    opacity: 1.0,
                },
                json!({ "deck": 0, "opacity": 1.0 }),
            )
        },
        make: Some(|with, slots| {
            Ok(Operation::SetOpacity {
                deck: deck_of(with, "deck", slots)?,
                opacity: f32_of(with, "opacity")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({ "deck": p_deck(), "opacity": p_number("the fader across the blend") }),
                &["deck", "opacity"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::SetMute {
                    deck: 0,
                    mute: true,
                },
                json!({ "deck": 0, "mute": true }),
            )
        },
        make: Some(|with, slots| {
            Ok(Operation::SetMute {
                deck: deck_of(with, "deck", slots)?,
                mute: bool_of(with, "mute")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "deck": p_deck(),
                    "mute": p_bool("whether this deck is excluded from the composite mix"),
                }),
                &["deck", "mute"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::SetSolo {
                    deck: 0,
                    solo: true,
                },
                json!({ "deck": 0, "solo": true }),
            )
        },
        make: Some(|with, slots| {
            Ok(Operation::SetSolo {
                deck: deck_of(with, "deck", slots)?,
                solo: bool_of(with, "solo")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "deck": p_deck(),
                    "solo": p_bool("whether this deck is isolated in the composite mix"),
                }),
                &["deck", "solo"],
            )
        }),
    },
    Spelled {
        sample: || (Operation::ClearSolo, json!({})),
        make: Some(|_, _| Ok(Operation::ClearSolo)),
        shape: Some(|| shaped(json!({}), &[])),
    },
    Spelled {
        sample: || {
            (
                Operation::SetOnline {
                    deck: 0,
                    online: true,
                },
                json!({ "deck": 0, "online": true }),
            )
        },
        make: Some(|with, slots| {
            Ok(Operation::SetOnline {
                deck: deck_of(with, "deck", slots)?,
                online: bool_of(with, "online")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "deck": p_deck(),
                    "online": p_bool("whether this deck's slot is online in the composite mix"),
                }),
                &["deck", "online"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::SetBlendMode {
                    deck: 0,
                    blend: karakuri_operation::BlendMode::Over,
                },
                json!({ "deck": 0, "blend": "over" }),
            )
        },
        make: Some(|with, slots| {
            Ok(Operation::SetBlendMode {
                deck: deck_of(with, "deck", slots)?,
                blend: word_of(
                    with,
                    "blend",
                    &karakuri_operation::BlendMode::ALL,
                    karakuri_operation::BlendMode::name,
                    "a blend mode",
                )?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "deck": p_deck(),
                    "blend": p_word(
                        words(&karakuri_operation::BlendMode::ALL, karakuri_operation::BlendMode::name),
                        "how this deck meets the ones under it in the fold",
                    ),
                }),
                &["deck", "blend"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::FadeDeck { deck: 0, to: 0.0 },
                json!({ "deck": 0, "to": 0.0 }),
            )
        },
        make: Some(|with, slots| {
            Ok(Operation::FadeDeck {
                deck: deck_of(with, "deck", slots)?,
                to: f32_of(with, "to")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "deck": p_deck(),
                    "to": p_number("the opacity to arrive at; the start and the length are the operator's transition settings"),
                }),
                &["deck", "to"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::Crossfade { from: 0, to: 0 },
                json!({ "from": 0, "to": 0 }),
            )
        },
        make: Some(|with, slots| {
            Ok(Operation::Crossfade {
                from: deck_of(with, "from", slots)?,
                to: deck_of(with, "to", slots)?,
            })
        }),
        shape: Some(|| shaped(json!({ "from": p_deck(), "to": p_deck() }), &["from", "to"])),
    },
    Spelled {
        sample: || {
            (
                Operation::Wipe { from: 0, to: 0 },
                json!({ "from": 0, "to": 0 }),
            )
        },
        make: Some(|with, slots| {
            Ok(Operation::Wipe {
                from: deck_of(with, "from", slots)?,
                to: deck_of(with, "to", slots)?,
            })
        }),
        shape: Some(|| shaped(json!({ "from": p_deck(), "to": p_deck() }), &["from", "to"])),
    },
    Spelled {
        sample: || {
            (
                Operation::SetMaskShape {
                    deck: 0,
                    kind: karakuri_operation::WipeKind::Linear,
                    angle: 0.0,
                },
                json!({ "deck": 0, "kind": "linear", "angle": 0.0 }),
            )
        },
        make: Some(|with, slots| {
            Ok(Operation::SetMaskShape {
                deck: deck_of(with, "deck", slots)?,
                kind: word_of(
                    with,
                    "kind",
                    &WIPE_KINDS,
                    karakuri_operation::WipeKind::name,
                    "a mask shape",
                )?,
                angle: f32_of(with, "angle")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "deck": p_deck(),
                    "kind": p_word(words(&WIPE_KINDS, karakuri_operation::WipeKind::name), "what shape of the frame this deck's layer reaches"),
                    "angle": p_number("which way a linear front travels, in turns"),
                }),
                &["deck", "kind", "angle"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::SetMaskPosition {
                    deck: 0,
                    position: 0.5,
                },
                json!({ "deck": 0, "position": 0.5 }),
            )
        },
        make: Some(|with, slots| {
            Ok(Operation::SetMaskPosition {
                deck: deck_of(with, "deck", slots)?,
                position: f32_of(with, "position")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "deck": p_deck(),
                    "position": p_number("how far the front has travelled: 0 reveals nothing, 1 reveals everything. It is the number a wipe's scheduled move is writing, so this cancels one"),
                }),
                &["deck", "position"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::SetTransition {
                    setting: karakuri_operation::TransitionSetting::Quantum { beats: 1.0 },
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
                Operation::SelectRenderer {
                    deck: 0,
                    renderer: 0,
                },
                json!({ "deck": 0, "renderer": 0 }),
            )
        },
        make: Some(|with, slots| {
            Ok(Operation::SelectRenderer {
                deck: deck_of(with, "deck", slots)?,
                renderer: u32_of(with, "renderer")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "deck": p_deck(),
                    "renderer": p_int("which renderer of this deck is live, by its position in the deck's files"),
                }),
                &["deck", "renderer"],
            )
        }),
    },
];
