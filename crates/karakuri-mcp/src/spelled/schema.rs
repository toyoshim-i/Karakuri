use serde_json::{json, Value};

use super::*;

/// Function signature for constructing an `Operation` from JSON arguments.
pub(crate) type Make = fn(&Value, &Slots) -> Result<Operation, String>;

#[derive(Clone, Copy)]
pub(crate) struct Spelled {
    /// Instance sample and minimal `operate` call arguments.
    pub(crate) sample: fn() -> (Operation, Value),
    /// Argument parser constructing `Operation`, or `None` if unsupported.
    pub(crate) make: Option<Make>,
    /// JSON Schema generator for the tool curriculum.
    pub(crate) shape: Option<fn() -> Value>,
}

impl Spelled {
    /// The heading this row is specified under, from the vocabulary.
    pub(crate) fn title(&self) -> &'static str {
        (self.sample)().0.title()
    }
}

/// Permitted values for closed vocabulary enum options.
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

/// Extracts and validates an identifier string from JSON input, rejecting directory paths.
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

/// Parses a `ChainParam` (declared key-value pair or cut mode).
pub(crate) fn chain_param_of(with: &Value) -> Result<karakuri_operation::ChainParam, String> {
    match (with.get("key"), with.get("cut")) {
        (Some(Value::Null) | None, Some(Value::Null) | None) => Err(String::from(
            "`with` says neither `key` nor `cut`, and a slot is set to a parameter its \
             procedure declares, at a value, or to the cut it reads",
        )),
        (Some(_), Some(_)) => Err(String::from(
            "`with` says both `key` and `cut`, which are two things to set — say one",
        )),
        (Some(_), _) => Ok(karakuri_operation::ChainParam::Declared {
            key: named_of(with, "key", "a parameter")?,
            value: f32_of(with, "value")?,
        }),
        (_, Some(_)) => Ok(karakuri_operation::ChainParam::Cut(word_of(
            with,
            "cut",
            &karakuri_operation::Cut::ALL,
            karakuri_operation::Cut::name,
            "a cut of the previous frame",
        )?)),
    }
}

/// Validates and parses a SHA-256 content address format (`sha256:<64 hex digits>`).
pub(crate) fn address_of(with: &Value, key: &str) -> Result<String, String> {
    let said = text_of(with, key)?;
    let digits = said.strip_prefix("sha256:").unwrap_or("");
    if digits.len() == 64 && digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(said.to_string());
    }
    Err(format!(
        "`with.{key}` is `{said}`, and a procedure is named by the content address of its \
          source — `sha256:` and sixty-four hex digits"
    ))
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

/// Parses a revision specification: either a previous node address or a specific historical ID.
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

/// Parses a beat source argument, accepting audio input and rejecting unsupported process sources.
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

/// The layers as words, from [`LAYERS`] rather than written out beside it —
/// [`tools`]'s own arrangement, in a function because this table asks for it in
/// three places.
pub(crate) fn layer_words() -> Vec<&'static str> {
    LAYERS.iter().map(|layer| layer_name(*layer)).collect()
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
