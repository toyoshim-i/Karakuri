use serde_json::{json, Value};

use super::super::*;

pub(crate) const RUN: &[Spelled] = &[
    Spelled {
        sample: || {
            (
                Operation::RouteFrame {
                    output: karakuri_operation::Output::Projector(0),
                    on: true,
                },
                json!({ "output": "projector", "index": 0, "on": true }),
            )
        },
        make: Some(|with, _| {
            let named = text_of(with, "output")?;
            let numbered = with.get("index").is_some_and(|at| !at.is_null());
            // Read only where it means something, so a bad `index` beside
            // `program` is refused for being there rather than for its value.
            let index = |with: &Value| -> Result<u8, String> {
                if !numbered {
                    return Ok(0);
                }
                u8::try_from(u32_of(with, "index")?).map_err(|_| {
                    "`with.index` is past what an output list holds — outputs are numbered \
                     from zero in the order the list draws them"
                        .to_string()
                })
            };
            let output = match named {
                "program" if numbered => {
                    return Err(
                        "`with.index` is given with `program`, and the picture in the Program \
                         bay is one output with no number — say `program` on its own"
                            .to_string(),
                    )
                }
                "program" => karakuri_operation::Output::Program,
                "projector" => karakuri_operation::Output::Projector(index(with)?),
                "plugin" => karakuri_operation::Output::Plugin(index(with)?),
                said => {
                    return Err(format!(
                        "`with.output` is `{said}`, and an output is one of: program, \
                         projector, plugin"
                    ))
                }
            };
            Ok(Operation::RouteFrame {
                output,
                on: bool_of(with, "on")?,
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "output": p_word(
                        vec!["program", "projector", "plugin"],
                        "which destination: the picture in the Program bay, a window this program opens, or a sink a plugin brings. Nothing loads a plugin today and asking for one is refused saying so",
                    ),
                    "index": p_int("which projector or plugin, by its place in the list; 0 where it is left out, and refused with `program`"),
                    "on": p_bool("whether it is publishing. All of them may be off — the deck previews are monitors rather than outputs and keep running"),
                }),
                &["output", "on"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::RecordSession {
                    recording: karakuri_operation::Recording::Start { id: None },
                },
                json!({ "recording": "start" }),
            )
        },
        make: Some(|with, _| {
            let recording = match text_of(with, "recording")? {
                "start" => karakuri_operation::Recording::Start { id: None },
                "stop" => karakuri_operation::Recording::Stop,
                said => {
                    return Err(format!(
                        "`with.recording` is `{said}`, and a recording is one of: start, stop"
                    ))
                }
            };
            Ok(Operation::RecordSession { recording })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "recording": p_word(
                        vec!["start", "stop"],
                        "start one or end the one running. A start is filed under a stamp and cannot be named here, because a second head under one id is read back as edits. Its head is the material deck A is playing as it stands, so a replay of it begins that material from the top",
                    ),
                }),
                &["recording"],
            )
        }),
    },
    Spelled {
        sample: || (Operation::Quit, json!({})),
        make: Some(|_, _| Ok(Operation::Quit)),
        shape: Some(|| shaped(json!({}), &[])),
    },
];
