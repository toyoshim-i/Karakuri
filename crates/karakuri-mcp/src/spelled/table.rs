use serde_json::{json, Value};

use super::*;

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
                Operation::RemoveLane {
                    pattern: 0,
                    lane: 0,
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
