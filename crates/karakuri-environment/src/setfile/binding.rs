use karakuri_engine::binding::{Curve, NOISE_SIGNAL};
use karakuri_engine::Binding;
use karakuri_ir::Kind;
use karakuri_signal::{NoiseConfig, NoiseKind};
use karakuri_store::record::{BindNoise, Layer, Record, Value};

use crate::meta::{kind_of, layer_name, layer_of};

use super::types::DEFAULT_OCTAVES;

/// Convert one [`Record::Bind`] into the binding the engine applies.
///
/// **The one place a binding's semantics live.** `--bind` reaches here too, so
/// the flag and a Set file cannot mean different things by the same fields.
///
/// The two diagnostics the decoder owes are here and
/// nowhere else:
///
/// - **`signal=bpm` is refused.** A tempo is not a `[0, 1]` signal, so the
///   curve clamps it and the binding sits pinned at the top of its range for
///   the whole run. From the outside a pinned binding and a working one are the
///   same number on a status line, which is exactly why this cannot be a
///   silent clamp. `beat` and `bar` carry the same tempo in the range a binding
///   is defined over.
/// - **`noise.octaves` needs `kind=fbm`.** The other three generators have no
///   layers, so an octave count on one of them is asking for a generator nobody
///   named. Refused rather than ignored, for the same reason.
///
/// And one more that is the same shape: `noise` on a binding whose signal is
/// not `noise` is refused, because accepting it leaves an operator re-reading
/// the noise fields to find out why the parameter does not move.
///
/// **A fourth refusal is deliberately not here: a binding on a bare vector
/// key.** A binding resolves to one number and a `vec3` has three places to
/// put it, so `bind key=glow` names no component — but whether `glow` is a
/// `vec3` is a fact about the *procedures*, which this function is not handed
/// and a `--bind` string does not carry. It is refused in [`from_lines`](crate::setfile::from_lines),
/// where the checked procedures are, with the component keys in the sentence
/// (ADR-0268).
pub fn binding_from_record(record: &Record) -> Result<Binding, String> {
    let Record::Bind {
        layer,
        index,
        key,
        signal,
        curve,
        range,
        noise,
    } = record
    else {
        return Err("not a `bind` record".to_string());
    };

    let bad = |what: String| format!("bind {}={key}: {what}", layer_name(*layer));

    let kind = kind_of(*layer);
    let curve = Curve::parse(curve).ok_or_else(|| {
        bad(format!(
            "curve `{curve}` — expected lin, pow2, sqrt or smooth"
        ))
    })?;

    if signal == "bpm" {
        return Err(bad(
            "signal `bpm` — a tempo is not a [0, 1] signal, so the curve clamps it and this \
             binding would sit at the top of its range for the whole run; bind `beat` or \
             `bar` instead"
                .to_string(),
        ));
    }

    let mut binding = Binding::new(kind, key.clone(), signal.clone(), curve, *range);
    // Absent stays absent: a binding with no `index` is the layer's, every node
    // declaring the key — see `Binding::index`.
    if let Some(at) = index {
        binding = binding.at(*at);
    }
    if signal != NOISE_SIGNAL {
        if noise.is_some() {
            return Err(bad(format!(
                "a generator needs `signal={NOISE_SIGNAL}`, and this binds `{signal}`"
            )));
        }
        return Ok(binding);
    }
    // **Absent means the default generator, not the absence of one**, which is
    // what `Record::Bind::noise` says and the only thing the name can mean: a
    // binding to `noise` with nothing else said is a binding to the default
    // generator. Materialised here rather than left as `None` so that what the
    // binding carries is what it will use.
    let noise = noise.clone().unwrap_or_default();
    let noise = &noise;
    // Read before the kind is folded, because `NoiseKind` carries the octave
    // count inside the `fbm` variant: once folded there is nothing left to
    // check against, and an octave count on a `white` would have turned it into
    // an `fbm` on the way past.
    let octaves_named = noise.octaves != DEFAULT_OCTAVES;
    if octaves_named && noise.kind != "fbm" {
        return Err(bad(format!(
            "`octaves` needs kind `fbm`, and this asks for `{}`",
            noise.kind
        )));
    }
    let kind = match noise.kind.as_str() {
        "white" => NoiseKind::White,
        "value" => NoiseKind::Value,
        "perlin" => NoiseKind::Perlin,
        "fbm" => NoiseKind::Fbm {
            octaves: noise.octaves,
        },
        other => {
            return Err(bad(format!(
                "noise kind `{other}` — expected white, value, perlin or fbm"
            )))
        }
    };
    Ok(binding.with_noise(NoiseConfig {
        kind,
        rate: noise.rate,
        stream: noise.stream,
    }))
}

/// Convert one [`Record::Source`]'s attachment into the binding the engine
/// applies — the session record's road into [`binding_from_record`].
///
/// **One decoder and not two.** A `source` carries a `bind`'s four payload
/// fields beside a `bind`'s address, so a second reader for it would be a
/// second answer to *what does `signal=bpm` mean*, *what does `octaves` need*
/// and *what does an absent `noise` mean* — the three diagnostics
/// [`binding_from_record`] says it owns and nowhere else. This builds the
/// `bind` those fields spell and hands it over, so a live attachment and a Set
/// file's cannot come to mean different things.
///
/// The take-back carries no attachment at all and never reaches here: a
/// [`Record::Source`] with no `source` is [`crate::mix::Change::Source`] with
/// no binding.
pub fn binding_from_source(
    layer: Layer,
    index: Option<u32>,
    key: &str,
    source: &karakuri_store::record::Source,
) -> Result<Binding, String> {
    binding_from_record(&Record::Bind {
        layer,
        index,
        key: key.to_string(),
        signal: source.signal.clone(),
        curve: source.curve.clone(),
        range: source.range,
        noise: source.noise.clone(),
    })
}

/// **One `param` record as the file wrote it**: where it lands, the key it
/// names, and the value.
///
/// Held rather than turned into a [`ParamWrite`] on sight, because what a
/// vector value becomes depends on what the procedures declare and they are not
/// checked until every `slot` record has been met — see [`from_lines`](crate::setfile::from_lines).
pub type ParamRecord = (Option<(Kind, u32)>, String, Value);

/// A written param's fold key: its address, then its name. `None` sorts first,
/// which puts the Set-wide value above the narrower ones that override it.
pub type ParamKey<'a> = (Option<(u8, u32)>, &'a str);

/// `Layer` as a number, so an address can sort. In node order, which is the
/// order a Set holds them in.
pub fn layer_ordinal(layer: Kind) -> u8 {
    match layer {
        Kind::L1 => 0,
        Kind::L2 => 1,
        Kind::L3 => 2,
        Kind::L4 => 3,
        // Last, matching `Set::slot_of`.
        Kind::Field => 4,
        // **After the fields, matching `Set::slot_of`** — placed at the end so
        // that every address a Set file already carries keeps its number.
        Kind::L5 => 5,
    }
}

pub fn layer_from_ordinal(n: u8) -> Layer {
    match n {
        0 => Layer::L1,
        1 => Layer::L2,
        2 => Layer::L3,
        3 => Layer::L4,
        // **Not the `_` arm.** `Field` used to fall into `L4`'s catch-all, so a
        // `--param Field:0:x` was saved as `L4:0:x` and reloaded onto renderer
        // zero — silently dropped if that renderer had no such name, and
        // silently wrong if it did.
        4 => Layer::Field,
        // And the sixth is named for the same reason, one kind later: a
        // catch-all here is how the fifth went wrong.
        _ => Layer::L5,
    }
}

/// The record a binding is. The inverse of [`binding_from_record`], and what
/// [`save`](crate::setfile::save) writes.
pub fn record_from_binding(binding: &Binding) -> Record {
    Record::Bind {
        layer: layer_of(binding.layer),
        index: binding.index,
        key: binding.key.clone(),
        signal: binding.signal.clone(),
        curve: binding.curve.name().to_string(),
        range: binding.range,
        noise: binding.noise.map(|n| BindNoise {
            kind: match n.kind {
                NoiseKind::White => "white".to_string(),
                NoiseKind::Value => "value".to_string(),
                NoiseKind::Perlin => "perlin".to_string(),
                NoiseKind::Fbm { .. } => "fbm".to_string(),
            },
            rate: n.rate,
            stream: n.stream,
            octaves: match n.kind {
                NoiseKind::Fbm { octaves } => octaves,
                _ => DEFAULT_OCTAVES,
            },
        }),
    }
}
