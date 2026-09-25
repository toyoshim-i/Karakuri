use karakuri_engine::binding::{Curve, NOISE_SIGNAL};
use karakuri_engine::Binding;
use karakuri_ir::Kind;
use karakuri_signal::{NoiseConfig, NoiseKind};
use karakuri_store::record::{BindNoise, Layer, Record, Value};

use crate::meta::{kind_of, layer_name, layer_of};

use super::types::DEFAULT_OCTAVES;

/// Converts [`Record::Bind`] into an engine binding, validating BPM scaling,
/// generator octave requirements, and noise configuration applicability.
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
    // An absent noise record defaults to the standard generator configuration.
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

/// Converts a [`Record::Source`] into an engine binding via [`binding_from_record`].
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

/// Raw parameter record: target address, parameter key, and value.
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
