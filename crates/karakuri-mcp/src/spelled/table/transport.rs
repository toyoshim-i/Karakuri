use serde_json::json;

use super::super::*;

pub(crate) const TRANSPORT: &[Spelled] = &[
    Spelled {
        sample: || (Operation::TapBeat, json!({})),
        make: Some(|_, _| Ok(Operation::TapBeat)),
        shape: Some(|| shaped(json!({}), &[])),
    },
    Spelled {
        sample: || {
            (
                Operation::ScaleGrid {
                    by: karakuri_operation::GridScale::Halve,
                },
                json!({ "by": "halve" }),
            )
        },
        make: Some(|with, _| {
            Ok(Operation::ScaleGrid {
                by: word_of(with, "by", &GRIDS, grid_word, "a direction")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({ "by": p_word(words(&GRIDS, grid_word), "which way the grid moves") }),
                &["by"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::SetLatencyOffset { ms: 5.0 },
                json!({ "ms": 5.0 }),
            )
        },
        make: Some(|with, _| {
            Ok(Operation::SetLatencyOffset {
                ms: f32_of(with, "ms")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({ "ms": p_number("the delay between what a room hears and what it sees, signed and in milliseconds") }),
                &["ms"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::SetSync {
                    deck: 0,
                    sync: karakuri_operation::Sync::Beat,
                },
                json!({ "deck": 0, "sync": "beat" }),
            )
        },
        make: Some(|with, slots| {
            Ok(Operation::SetSync {
                deck: deck_of(with, "deck", slots)?,
                sync: word_of(
                    with,
                    "sync",
                    &SYNCS,
                    karakuri_operation::Sync::name,
                    "a sync mode",
                )?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "deck": p_deck(),
                    "sync": p_word(words(&SYNCS, karakuri_operation::Sync::name), "what this deck's clock is locked to"),
                }),
                &["deck", "sync"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::ScrubDeck {
                    deck: 0,
                    beats: 0.25,
                },
                json!({ "deck": 0, "beats": 0.25 }),
            )
        },
        make: Some(|with, slots| {
            Ok(Operation::ScrubDeck {
                deck: deck_of(with, "deck", slots)?,
                beats: number_of(with, "beats")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "deck": p_deck(),
                    "beats": p_number("how far to move, in beats, and relative — the record carries where it lands"),
                }),
                &["deck", "beats"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::SetFreeRunTempo { bpm: 120.0 },
                json!({ "bpm": 120.0 }),
            )
        },
        make: Some(|with, _| {
            Ok(Operation::SetFreeRunTempo {
                bpm: f32_of(with, "bpm")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({ "bpm": p_number("what the grid runs at with nothing driving it") }),
                &["bpm"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::AttachBeatSource {
                    source: karakuri_operation::BeatSource::AudioInput("an input".into()),
                },
                json!({ "source": { "audio_input": "an input" } }),
            )
        },
        make: Some(|with, _| {
            Ok(Operation::AttachBeatSource {
                source: source_of(with, "source")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "source": {
                        "type": "object",
                        "description": "an audio input to track, by the name the host offers it under. A process source is a command line and this server takes none",
                        "properties": { "audio_input": p_string("the device's name") },
                        "required": ["audio_input"],
                        "additionalProperties": false,
                    },
                }),
                &["source"],
            )
        }),
    },
];
