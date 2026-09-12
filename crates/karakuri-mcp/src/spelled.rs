use serde_json::{json, Value};

use super::*;

/// One operation of the vocabulary as this surface spells it.
///
/// The title is not written here. It comes back from `Operation::title` through
/// [`Spelled::sample`], so the name a client types and the heading
/// `docs/manual/operations.html` specifies the row under are one string and
/// cannot drift — which is what `karakuri-operation` exists for and what a
/// hand-written table of names beside it would give up (`docs/contributing.md`
/// §4, *Generated*). One payload, read as the operation it names — or the
/// refusal its arguments earned, in the words the seven tools refuse the same
/// mistakes in.
///
/// A name of its own because it is one shape written thirty-one times, and
/// because [`Spelled::make`] reads better for having it.
pub(crate) type Make = fn(&Value, &Slots) -> Result<Operation, String>;

pub(crate) struct Spelled {
    /// One instance of this operation, and the smallest `operate` call that names
    /// it where this surface takes one — `Value::Null` where it does not.
    ///
    /// The pair is here rather than in a test because the schema is built from it
    /// and the round trip is checked against it: `make` applied to the call has to
    /// come back equal to the operation, for every row, which is what makes this
    /// one statement rather than two.
    pub(crate) sample: fn() -> (Operation, Value),
    /// The call's `with` object as the operation it names, or `None` where
    /// [`sayable`] says this surface cannot name it.
    pub(crate) make: Option<Make>,
    /// The JSON Schema of that `with` object, for the curriculum a client is handed
    /// before it calls
    /// ([ADR-0092](../../../docs/adr/0092-a-resource-listing-is-a-curriculum.md)).
    pub(crate) shape: Option<fn() -> Value>,
}

impl Spelled {
    /// The heading this row is specified under, from the vocabulary.
    pub(crate) fn title(&self) -> &'static str {
        (self.sample)().0.title()
    }
}

/// The closed lists this surface spells on the wire, and the word for each
/// value is the vocabulary's own `name`.
///
/// Where the vocabulary publishes an `ALL`, that is what is used; where it does
/// not, the values are written out here and the words still are not. That is
/// [`layer_named`]'s arrangement one type along, and it carries
/// [`layer_named`]'s cost: a fourth `Sync` would have to be added here as well.
/// `the_wire_spells_every_value_of_every_closed_list` is what says so.
pub(crate) const SYNCS: [karakuri_operation::Sync; 3] = [
    karakuri_operation::Sync::Free,
    karakuri_operation::Sync::Tempo,
    karakuri_operation::Sync::Beat,
];
pub(crate) const GRIDS: [karakuri_operation::GridScale; 2] = [
    karakuri_operation::GridScale::Halve,
    karakuri_operation::GridScale::Double,
];
pub(crate) const WIPE_KINDS: [karakuri_operation::WipeKind; 3] = [
    karakuri_operation::WipeKind::None,
    karakuri_operation::WipeKind::Linear,
    karakuri_operation::WipeKind::Radial,
];
pub(crate) const TONEMAPS: [karakuri_operation::Tonemap; 4] = [
    karakuri_operation::Tonemap::Clamp,
    karakuri_operation::Tonemap::Reinhard,
    karakuri_operation::Tonemap::Aces,
    karakuri_operation::Tonemap::AgX,
];
pub(crate) const AUTHORITIES: [karakuri_operation::Authority; 3] = [
    karakuri_operation::Authority::Manual,
    karakuri_operation::Authority::Suggesting,
    karakuri_operation::Authority::Automatic,
];

/// The word for a grid scale, which is the one value list in this file whose
/// vocabulary type publishes no `name` of its own. Halving and doubling are the
/// two directions and the words are the manual's heading read aloud.
pub(crate) fn grid_word(scale: karakuri_operation::GridScale) -> &'static str {
    match scale {
        karakuri_operation::GridScale::Halve => "halve",
        karakuri_operation::GridScale::Double => "double",
    }
}

/// Every word of a closed list, in the order the list is written.
pub(crate) fn words<T: Copy>(values: &[T], name: fn(T) -> &'static str) -> Vec<&'static str> {
    values.iter().copied().map(name).collect()
}

/// The `with` object of one call, which is absent where the operation takes no
/// payload.
pub(crate) fn payload(args: &Value) -> Value {
    args.get("with").cloned().unwrap_or_else(|| json!({}))
}

pub(crate) fn number_of(with: &Value, key: &str) -> Result<f64, String> {
    with.get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("`with.{key}` is required and is a number"))
}

pub(crate) fn f32_of(with: &Value, key: &str) -> Result<f32, String> {
    Ok(number_of(with, key)? as f32)
}

pub(crate) fn u32_of(with: &Value, key: &str) -> Result<u32, String> {
    let n = with
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("`with.{key}` is required and is a whole number, zero or more"))?;
    u32::try_from(n).map_err(|_| format!("`with.{key}` is {n}, which is past what this addresses"))
}

pub(crate) fn bool_of(with: &Value, key: &str) -> Result<bool, String> {
    with.get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("`with.{key}` is required and is true or false"))
}

pub(crate) fn text_of<'a>(with: &'a Value, key: &str) -> Result<&'a str, String> {
    with.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("`with.{key}` is required and is a string"))
}

/// A name and never a path. Paths never cross this protocol — see [`Slots`] —
/// and every free string a payload of this table carries is a *name* something
/// in this run produced: a signal on the bus, a parameter a procedure declares,
/// a control an operator published. So a separator is refused here rather than
/// resolved anywhere, in one sentence for all of them
/// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
///
/// [`checked_id`] is the same rule for a *Set id*, and it is narrower because a
/// Set id becomes a file name. A parameter key can be `glow.x` and a signal can
/// be `control:macro`, so this refuses the two separators and the parent
/// segment and nothing else.
pub(crate) fn named_of(with: &Value, key: &str, what: &str) -> Result<String, String> {
    let said = text_of(with, key)?;
    if said.is_empty() {
        return Err(format!("`with.{key}` is empty, and {what} has a name"));
    }
    if said.contains('/') || said.contains('\\') || said.contains("..") {
        return Err(format!(
            "`with.{key}` is `{said}`, and {what} is a name rather than a path — this server \
             takes no paths at all, because a client may be on another machine where one \
             means nothing"
        ));
    }
    Ok(said.to_string())
}

/// One value of a closed list, by the word the vocabulary spells it with, and
/// the refusal lists every word there is.
pub(crate) fn word_of<T: Copy>(
    with: &Value,
    key: &str,
    values: &[T],
    name: fn(T) -> &'static str,
    what: &str,
) -> Result<T, String> {
    let said = text_of(with, key)?;
    values
        .iter()
        .copied()
        .find(|value| name(*value) == said)
        .ok_or_else(|| {
            format!(
                "`with.{key}` is `{said}`, and {what} is one of: {}",
                words(values, name).join(", ")
            )
        })
}

/// The deck a payload names, checked against what this run holds before it is
/// sent anywhere, in the sentence every other tool refuses an absent slot in
/// ([`deck_named`]).
pub(crate) fn deck_of(with: &Value, key: &str, slots: &Slots) -> Result<u8, String> {
    let slot = with
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("`with.{key}` is required and is a deck slot number"))?
        as usize;
    deck_named(slot, slots)
}

/// A node of a deck's Set, as `read_procedure` addresses one: a layer, and an
/// index into that layer that defaults to 0 for that tool's reason.
pub(crate) fn node_of(with: &Value, key: &str) -> Result<NodeAddress, String> {
    let at = with
        .get(key)
        .ok_or_else(|| format!("`with.{key}` is required and is a node: `layer`, and `index`"))?;
    let named = at
        .get("layer")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("`with.{key}.layer` is required — {}", layer_list()))?;
    let layer =
        layer_named(named).ok_or_else(|| format!("no layer `{named}` — {}", layer_list()))?;
    let index = match at.get("index") {
        None | Some(Value::Null) => 0,
        Some(_) => u32_of(at, "index")?,
    };
    Ok(NodeAddress {
        layer: layer_of(layer),
        index,
    })
}

/// Which parameter a value lands on, on `Record::Param`'s terms: a key, and a
/// node it is addressed to or every node of the Set that declares it.
pub(crate) fn param_of(with: &Value, key: &str) -> Result<karakuri_operation::ParamAt, String> {
    let at = with
        .get(key)
        .ok_or_else(|| format!("`with.{key}` is required and is a parameter: `key`, and `node`"))?;
    Ok(karakuri_operation::ParamAt {
        node: match at.get("node") {
            None | Some(Value::Null) => None,
            Some(_) => Some(node_of(at, "node")?),
        },
        key: named_of(at, "key", "a parameter")?,
    })
}

/// Which parameter an attachment lands on, which is not [`param_of`]'s address
/// and the difference is a fact about a binding: a binding is resolved through
/// the nodes of one layer, so the layer is always said and the index is what
/// may be left out.
pub(crate) fn bind_of(with: &Value, key: &str) -> Result<karakuri_operation::BindAt, String> {
    let at = with.get(key).ok_or_else(|| {
        format!("`with.{key}` is required and is a binding address: `layer`, `key`, and `index`")
    })?;
    let named = at
        .get("layer")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("`with.{key}.layer` is required — {}", layer_list()))?;
    let layer =
        layer_named(named).ok_or_else(|| format!("no layer `{named}` — {}", layer_list()))?;
    Ok(karakuri_operation::BindAt {
        layer: layer_of(layer),
        index: match at.get("index") {
            None | Some(Value::Null) => None,
            Some(_) => Some(u32_of(at, "index")?),
        },
        key: named_of(at, "key", "a parameter")?,
    })
}

/// A parameter's value at one of the three widths a `.kir` can declare — a
/// number, or two or three of them. Never a range: a range is the procedure's
/// declaration and not an operator's to write.
pub(crate) fn value_of(with: &Value, key: &str) -> Result<karakuri_operation::ParamValue, String> {
    let at = with.get(key).ok_or_else(|| {
        format!("`with.{key}` is required and is a number, or two or three of them")
    })?;
    if let Some(number) = at.as_f64() {
        return Ok(karakuri_operation::ParamValue::Scalar(number as f32));
    }
    let list = at
        .as_array()
        .ok_or_else(|| format!("`with.{key}` is a number, or an array of two or three of them"))?;
    let mut numbers = Vec::with_capacity(list.len());
    for (at, one) in list.iter().enumerate() {
        numbers.push(one.as_f64().ok_or_else(|| {
            format!("`with.{key}[{at}]` is not a number, and every component of a value is")
        })? as f32);
    }
    match numbers[..] {
        [x] => Ok(karakuri_operation::ParamValue::Scalar(x)),
        [x, y] => Ok(karakuri_operation::ParamValue::Vec2([x, y])),
        [x, y, z] => Ok(karakuri_operation::ParamValue::Vec3([x, y, z])),
        _ => Err(format!(
            "`with.{key}` has {} components, and a declared parameter is one, two or three \
             wide",
            numbers.len()
        )),
    }
}

/// The `[low, high]` an attachment maps a signal into.
pub(crate) fn range_of(with: &Value, key: &str) -> Result<[f32; 2], String> {
    let list = with
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("`with.{key}` is required and is `[low, high]`"))?;
    let ends: Vec<f64> = list.iter().filter_map(Value::as_f64).collect();
    match ends[..] {
        [low, high] if ends.len() == list.len() => Ok([low as f32, high as f32]),
        _ => Err(format!(
            "`with.{key}` is `[low, high]` — two numbers, and the attachment maps the signal \
             between them"
        )),
    }
}

/// Which version a put-back puts back, and the two arms are the two things a
/// surface can say rather than two features
/// ([ADR-0192](../../../docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)).
///
/// Neither arm is a path. `previous` names a node, and `picked` names a version
/// by the name the store filed it under — which is what `write_procedure`
/// already hands back about the version it replaced, so a model spells one it
/// was given rather than one it built.
pub(crate) fn revision_of(with: &Value, key: &str) -> Result<karakuri_operation::Revision, String> {
    let at = with.get(key).ok_or_else(|| {
        format!(
            "`with.{key}` is required and is either `{{\"previous\": {{\"layer\": …}}}}` — the \
             version this node's present source replaced — or `{{\"picked\": \"…\"}}`, a \
             version by the name the store filed it under"
        )
    })?;
    match (at.get("previous"), at.get("picked")) {
        (Some(Value::Null) | None, Some(Value::Null) | None) => Err(format!(
            "`with.{key}` says neither `previous` nor `picked`, and a put-back is one or the \
             other"
        )),
        (Some(_), Some(_)) => Err(format!(
            "`with.{key}` says both `previous` and `picked`, which are two answers to which \
             version — say one"
        )),
        (Some(_), _) => Ok(karakuri_operation::Revision::Previous(node_of(
            at, "previous",
        )?)),
        (_, Some(_)) => Ok(karakuri_operation::Revision::Picked(checked_id(text_of(
            at, "picked",
        )?)?)),
    }
}

/// A beat source, and one of its two arms does not cross this protocol.
///
/// `AudioInput` names a device the host is offering and is a name like any
/// other. `Process` is a command line for this machine to run, which is a path
/// with arguments after it and is the sharpest thing on this page a client
/// could be handed — so it is refused here, saying what it is and where a
/// process is still started from.
pub(crate) fn source_of(with: &Value, key: &str) -> Result<karakuri_operation::BeatSource, String> {
    let at = with.get(key).ok_or_else(|| {
        format!("`with.{key}` is required and is `{{\"audio_input\": \"DEVICE\"}}`")
    })?;
    if at.get("process").is_some() {
        return Err(format!(
            "`with.{key}` names a process, and this server takes none: a process source is a \
             command line for the render machine to run, which is a path with arguments after \
             it. It is still a flag before the run — `--tempo-source` — and nothing during one \
             starts it"
        ));
    }
    Ok(karakuri_operation::BeatSource::AudioInput(named_of(
        at,
        "audio_input",
        "an audio input",
    )?))
}

/// One payload's schema. Closed: a key this table does not name is a mistake a
/// caller had no way to see, and saying so is cheaper than performing half of
/// what was asked.
pub(crate) fn shaped(properties: Value, required: &[&str]) -> Value {
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    })
}

pub(crate) fn p_number(about: &str) -> Value {
    json!({ "type": "number", "description": about })
}

pub(crate) fn p_int(about: &str) -> Value {
    json!({ "type": "integer", "minimum": 0, "description": about })
}

pub(crate) fn p_bool(about: &str) -> Value {
    json!({ "type": "boolean", "description": about })
}

pub(crate) fn p_string(about: &str) -> Value {
    json!({ "type": "string", "description": about })
}

pub(crate) fn p_word(list: Vec<&'static str>, about: &str) -> Value {
    json!({ "type": "string", "enum": list, "description": about })
}

/// The deck slot, in `read_procedure`'s own words and with its own bound: how
/// many slots this run holds is not knowable when a schema is answered.
pub(crate) fn p_deck() -> Value {
    json!({
        "type": "integer",
        "minimum": 0,
        "description": "which deck slot, numbered as `read_procedure`'s `slot` is",
    })
}

pub(crate) fn p_node(about: &str) -> Value {
    json!({
        "type": "object",
        "description": about,
        "properties": {
            "layer": { "type": "string", "enum": layer_words(), "description": "which layer" },
            "index": p_int("which node of that layer, in the order the deck's files were named; 0 where it is left out"),
        },
        "required": ["layer"],
        "additionalProperties": false,
    })
}

/// Every operation of the vocabulary, and how this surface spells the ones it
/// takes.
///
/// Sixty-four rows, one per `<h3>` of `docs/manual/operations.html`, in that
/// page's order. `every_operation_of_the_vocabulary_is_spelled_here` walks
/// `Operation::TITLES` against this, so a row cannot be missing and a
/// sixty-fifth operation arrives here as a failing test as well as a failing
/// build ([`sayable`]).
pub(crate) const SPELLED: &[Spelled] = &[
    // ----- Transport and tempo ---------------------------------------------
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
    // ----- Decks -----------------------------------------------------------
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
    // ----- Mixing ----------------------------------------------------------
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
    // ----- The master chain ------------------------------------------------
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
                Operation::SetFeedback {
                    params: karakuri_operation::Feedback {
                        amount: 0.5,
                        cut: karakuri_operation::Cut::Mix,
                    },
                },
                json!({ "amount": 0.5, "cut": "mix" }),
            )
        },
        make: Some(|with, _| {
            Ok(Operation::SetFeedback {
                params: karakuri_operation::Feedback {
                    amount: f32_of(with, "amount")?,
                    cut: word_of(
                        with,
                        "cut",
                        &karakuri_operation::Cut::ALL,
                        karakuri_operation::Cut::name,
                        "a cut of the previous frame",
                    )?,
                },
            })
        }),
        shape: Some(|| {
            shaped(
                json!({
                    "amount": p_number("how much of the retained frame comes back, up to 0.95"),
                    "cut": p_word(
                        words(&karakuri_operation::Cut::ALL, karakuri_operation::Cut::name),
                        "which frame the amount is of — the two are one operation because the same amount means two different pictures",
                    ),
                }),
                &["amount", "cut"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::SetBloom {
                    params: karakuri_operation::Bloom { amount: 0.25 },
                },
                json!({ "amount": 0.25 }),
            )
        },
        make: Some(|with, _| {
            Ok(Operation::SetBloom {
                params: karakuri_operation::Bloom {
                    amount: f32_of(with, "amount")?,
                },
            })
        }),
        shape: Some(|| {
            shaped(
                json!({ "amount": p_number("how much of the blurred bright part is added back, `[0, 1]`") }),
                &["amount"],
            )
        }),
    },
    Spelled {
        sample: || {
            (
                Operation::SetRgbShift {
                    params: karakuri_operation::RgbShift { amount: 0.25 },
                },
                json!({ "amount": 0.25 }),
            )
        },
        make: Some(|with, _| {
            Ok(Operation::SetRgbShift {
                params: karakuri_operation::RgbShift {
                    amount: f32_of(with, "amount")?,
                },
            })
        }),
        shape: Some(|| {
            shaped(
                json!({ "amount": p_number("how far the three channels are pulled apart, `[0, 1]` of the pass's own maximum") }),
                &["amount"],
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
    // ----- The sequencer ---------------------------------------------------
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
    // ----- Inside a Set ----------------------------------------------------
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
        // **The whole list, never one entry**, which is what the operation
        // carries and why: adding or removing one at a time is a statement
        // about an entry, and it is what lets two hands on one deck disagree
        // about what is published. An empty list asks for every declared
        // control published, which is what an unnarrowed deck is.
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
                // **Left out is a stamp**, which is `save_set`'s own
                // convention and is what the panel's capsule sends: a caller
                // that can type a name is not made to take a timestamp, and
                // one that says nothing gets the time it kept it. Checked
                // here, because what a model names becomes one path component
                // (`checked_id`).
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
    // ----- The library -----------------------------------------------------
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
    // ----- Procedures ------------------------------------------------------
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
    // ----- Arranging the console -------------------------------------------
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
    // ----- The run ---------------------------------------------------------
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
        // **A word from a closed list and never the label on a chip**, which
        // is the operation's own rule: a projector chip carries the display it
        // is on and that changes when the cable does. The index is the
        // projector's or the plugin's place in the list, and the picture has
        // none — one output, no number.
        //
        // **`plugin` is taken and refused by the performer**, which is a built
        // route rather than a missing one: there is no manifest to read a sink
        // out of, and the refusal is where that is said.
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
        // **No id on the wire, and the vocabulary's `Option` is always
        // `None`.** Each start takes a fresh stamp (ADR-0289) — a second head
        // written into a stream that already exists is read back as edits —
        // and `crates/karakuri`'s performer refuses a named one in as many
        // words. A key this table accepted and the frame then refused would be
        // exactly the `ok` for work that did not happen this surface is
        // arranged against, so the key is not offered.
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

/// The layers as words, from [`LAYERS`] rather than written out beside it —
/// [`tools`]'s own arrangement, in a function because this table asks for it in
/// three places.
pub(crate) fn layer_words() -> Vec<&'static str> {
    LAYERS.iter().map(|layer| layer_name(*layer)).collect()
}

/// Every operation `operate` takes, in the manual's order.
pub(crate) fn operable() -> Vec<&'static Spelled> {
    SPELLED.iter().filter(|row| row.make.is_some()).collect()
}

/// The row for one heading, or `None` where the vocabulary does not carry it.
pub(crate) fn spelled_named(title: &str) -> Option<&'static Spelled> {
    SPELLED.iter().find(|row| row.title() == title)
}

/// The heading nearest to a name this vocabulary does not carry.
///
///
/// [P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md):
/// a model that misremembers a heading by one word gets the heading back rather
/// than a list of sixty-four to search. The measure is how many words the name
/// and the heading share, with the closest length for a tie, and it is
/// deliberately not an edit distance: the mistakes here are whole words rather
/// than letters.
///
/// The joining words are dropped, and matching whole words is the point.
/// Matched as substrings, *set the gain* comes back as *Reset the arrangement*
/// — `the` is in both and `set` is inside `Reset` — which is a confident wrong
/// answer of exactly the kind
/// [P-0084](../../../docs/principles/0084-a-confident-wrong-automatic-judgement-is-worse-than-not-judging.md)
/// is about.
pub(crate) fn nearest(said: &str) -> &'static str {
    /// The words that say nothing about which operation is meant.
    const COMMON: [&str; 16] = [
        "the", "a", "an", "of", "to", "in", "on", "is", "it", "and", "or", "what", "which", "one",
        "at", "for",
    ];
    fn parts(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|word| !word.is_empty() && !COMMON.contains(word))
            .map(str::to_string)
            .collect()
    }
    let asked = parts(said);
    let mut best = ("", 0usize, usize::MAX);
    for title in Operation::TITLES {
        let held = parts(title);
        let shared = asked.iter().filter(|word| held.contains(word)).count();
        let apart = title.len().abs_diff(said.len());
        if shared > best.1 || (shared == best.1 && apart < best.2) {
            best = (title, shared, apart);
        }
    }
    best.0
}

/// One `operate` call as the operation it names, or the refusal it earned.
///
/// The two halves are a name and a payload, and they are refused in that order
/// for [`asked`]'s reason: a call with two mistakes in it is told about the one
/// a reader would fix first.
pub(crate) fn operated(args: &Value, slots: &Slots) -> Result<Operation, String> {
    let named = args.get("operation").and_then(Value::as_str).ok_or(
        "`operation` is required and is an operation's own heading, spelled exactly as \
             `docs/manual/operations.html` writes it — this tool's `operation` list is every \
             one it takes",
    )?;
    let Some(row) = spelled_named(named) else {
        return Err(format!(
            "no operation `{named}` — the nearest heading this vocabulary carries is \
             `{}`. Every name this tool takes is in its own `operation` list, and \
             `karakuri://operations` is that list with each payload's shape beside it",
            nearest(named)
        ));
    };
    let Some(make) = row.make else {
        let (operation, _) = (row.sample)();
        return Err(match sayable(&operation) {
            // Cannot happen: `make` is `Some` for exactly the operable rows, and
            // `the_table_and_the_classification_agree` is what says so. Written
            // out rather than left to a wildcard for [`absent`]'s reason.
            Sayable::Operable => format!(
                "`{named}` is an operation this tool takes and this server has no spelling \
                 for, which is a fault in the server rather than in the call"
            ),
            Sayable::Tool(tool) => format!(
                "`{named}` is reached over MCP by `{tool}` rather than by `operate`: it does \
                 something only this server can do — a file, the store, a listing — and a \
                 second spelling of a tool is a second spelling. Call `{tool}`"
            ),
            Sayable::Window => format!(
                "`{named}` is a surface's own state, and a route into a surface's own state \
                 is a route into a window a model is not looking at. Nothing here can ask \
                 for it, and the person at the panel is who it belongs to"
            ),
            Sayable::Never(why) => format!(
                "`{named}` is named by this vocabulary and has no route on this surface: \
                 {why}"
            ),
        });
    };
    make(&payload(args), slots)
}

/// The `operate` tool, generated from [`SPELLED`].
///
/// The `operation` list is every heading this surface takes, in the manual's
/// order, and the payloads are described under it rather than as one `oneOf`:
/// what a client needs before it calls is *which names there are* and *what
/// each one takes*, and a schema that expressed the second as a union of thirty
/// objects would be read by nothing and understood by no one. The shapes go to
/// `karakuri://operations`, which is the same table rendered
/// ([ADR-0092](../../../docs/adr/0092-a-resource-listing-is-a-curriculum.md)).
pub(crate) fn operate_tool() -> Value {
    let names: Vec<&'static str> = operable().iter().map(|row| row.title()).collect();
    json!({
        "name": "operate",
        "description":
            "Ask for one operation of this instrument by its own name. The names are the \
             headings of `docs/manual/operations.html`, which is the one vocabulary every \
             surface routes into — the panel, the keyboard, a MIDI map and this server all \
             name the same things, so an operation asked for here is performed where a hand \
             on the panel would have performed it, on the next frame. Read \
             `karakuri://operations` for the payload each name takes.\n\n\
             Operations that could stop a performance are refused until the operator opens \
             their class at the panel. The refusal says which class it is in and where the \
             operator opens it, so it can be handed to the person sitting there. The list \
             above never shortens: an operation is connected whether or not its class is \
             open, and the answer is a refusal rather than a missing tool.\n\n\
             The seven tools beside this one are not spelled here. Each of them does \
             something only this server can do, and `operate` names the tool instead.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": names,
                    "description": "the operation's own heading, verbatim",
                },
                "with": {
                    "type": "object",
                    "description":
                        "the payload, whose shape follows the operation — \
                         `karakuri://operations` has one schema per name. Absent where the \
                         operation takes none",
                },
            },
            "required": ["operation"],
        },
    })
}

/// Every operation this surface takes, with its payload's shape — the
/// curriculum a client reads before it calls, generated from [`SPELLED`] rather
/// than written down beside it.
pub(crate) fn operations() -> String {
    let mut out = String::from(
        "# Operations `operate` takes\n\n\
         Generated from this server's own table, so this is exactly what will be \
         accepted. Each heading is the name to put in `operation`, and the schema under \
         it is the `with` object.\n\n\
         An operation whose class the operator has not opened is **refused**, and the \
         refusal says which class and where it opens. That is not a reason to avoid \
         calling it: the refusal is what tells the person at the panel what to open.\n\n",
    );
    for row in operable() {
        let (_, call) = (row.sample)();
        let shape = (row.shape).expect("an operable row has a shape")();
        out.push_str(&format!(
            "## {}\n\n```json\n{}\n```\n\nOne call:\n\n```json\n{}\n```\n\n",
            row.title(),
            serde_json::to_string_pretty(&shape).unwrap_or_else(|_| shape.to_string()),
            serde_json::to_string_pretty(&json!({ "operation": row.title(), "with": call }))
                .unwrap_or_default(),
        ));
    }
    out
}
