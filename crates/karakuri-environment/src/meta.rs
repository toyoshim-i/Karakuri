//! Generates metadata cards for compiled artifacts according to `docs/ir-spec.md`.

use karakuri_ir::typed::Checked;
use karakuri_ir::Kind;
use karakuri_operation::Layer as OpLayer;
use karakuri_store::hash::Hash;
use karakuri_store::ndjson::Line;
use karakuri_store::record::{Layer, Record};

/// The engine [`Kind`] a record `Layer` names. The inverse of [`layer_of`], and
/// total because every layer a record can name is a layer this engine has.
pub fn kind_of(layer: Layer) -> Kind {
    match layer {
        Layer::L1 => Kind::L1,
        Layer::L2 => Kind::L2,
        Layer::L3 => Kind::L3,
        Layer::L4 => Kind::L4,
        Layer::Field => Kind::Field,
        Layer::L5 => Kind::L5,
    }
}

/// The record `Layer` an engine [`Kind`] names. The inverse of [`kind_of`], and
/// total for the same reason: every layer a Set can hold is a layer a record
/// can address.
pub fn layer_of(kind: Kind) -> Layer {
    match kind {
        Kind::L1 => Layer::L1,
        Kind::L2 => Layer::L2,
        Kind::L3 => Layer::L3,
        Kind::L4 => Layer::L4,
        Kind::Field => Layer::Field,
        Kind::L5 => Layer::L5,
    }
}

/// The operation [`OpLayer`] an engine [`Kind`] names.
pub fn op_layer_of(kind: Kind) -> OpLayer {
    match kind {
        Kind::L1 => OpLayer::L1,
        Kind::L2 => OpLayer::L2,
        Kind::L3 => OpLayer::L3,
        Kind::L4 => OpLayer::L4,
        Kind::Field => OpLayer::Field,
        Kind::L5 => OpLayer::L5,
    }
}

/// The engine [`Kind`] an operation [`OpLayer`] names.
pub fn kind_of_op(layer: OpLayer) -> Kind {
    match layer {
        OpLayer::L1 => Kind::L1,
        OpLayer::L2 => Kind::L2,
        OpLayer::L3 => Kind::L3,
        OpLayer::L4 => Kind::L4,
        OpLayer::Field => Kind::Field,
        OpLayer::L5 => Kind::L5,
    }
}

/// Convert a store record [`Layer`] to an operation [`OpLayer`].
pub fn record_to_op(layer: Layer) -> OpLayer {
    op_layer_of(kind_of(layer))
}

/// Convert an operation [`OpLayer`] to a store record [`Layer`].
pub fn op_to_record(layer: OpLayer) -> Layer {
    layer_of(kind_of_op(layer))
}

/// The layer name as an operator writes it.
pub fn kind_name(kind: Kind) -> &'static str {
    kind.name()
}

/// The record layer name as serialized in store records.
pub fn layer_name(layer: Layer) -> &'static str {
    layer.name()
}

/// Look up a layer by name as written in user input or Set files.
pub fn layer_named(name: &str) -> Option<Kind> {
    name.parse().ok()
}

/// The metadata file format version this build writes, on the `meta` record —
/// one number for the whole file, which is what `Record::Set`'s `v` is for a
/// Set file.
const VERSION: u32 = 1;

/// Generates metadata record lines describing the compiled procedure and its interface declarations.
pub fn card(hash: &Hash, checked: &Checked) -> Vec<Line> {
    let mut lines = vec![Line::new(Record::Meta {
        hash: *hash,
        name: checked.name.clone(),
        kind: layer_of(checked.kind),
        v: VERSION,
    })];
    for p in &checked.params {
        lines.push(Line::new(Record::ParamDecl {
            key: p.name.clone(),
            ty: p.ty.name().to_string(),
            min: p.min,
            max: p.max,
            default: p.default_scalar(),
        }));
    }
    if let Some(c) = checked.capacity {
        lines.push(Line::new(Record::CapacityDecl {
            min: c.min,
            max: c.max,
            default: c.default,
        }));
    }
    if !checked.emit.is_empty() {
        lines.push(Line::new(Record::Emit {
            attrs: checked.emit.iter().map(|a| a.name().to_string()).collect(),
        }));
    }
    lines
}

/// Writes artifact metadata card to store, returning warning message if write fails.
pub fn put_meta(
    store: &karakuri_store::store::Store,
    hash: &karakuri_store::hash::Hash,
    card: &[karakuri_store::ndjson::Line],
) -> Option<String> {
    let e = store.write_meta(hash, card).err()?;
    let said = format!(
        "  the metadata for {}: {e} — the artifact is stored and the library will \
         regenerate its card from the source on the next compile",
        hash.short(12)
    );
    eprintln!("{said}");
    Some(said)
}

#[cfg(test)]
mod tests {
    use super::*;
    use karakuri_ir::Kind;
    use karakuri_operation::Layer as OpLayer;
    use karakuri_store::record::Layer as RecordLayer;

    #[test]
    fn layer_conversions_and_contracts_are_coherent() {
        for kind in Kind::ALL {
            let record = layer_of(kind);
            let op = op_layer_of(kind);

            // Inverses
            assert_eq!(kind_of(record), kind);
            assert_eq!(kind_of_op(op), kind);
            assert_eq!(record_to_op(record), op);
            assert_eq!(op_to_record(op), record);

            // Display and names match
            assert_eq!(kind.name(), kind_name(kind));
            assert_eq!(kind.to_string(), kind.name());
            assert_eq!(op.to_string(), kind.name());
            assert_eq!(record.to_string(), kind.name());

            // Parsing roundtrip
            assert_eq!(kind.name().parse::<Kind>().unwrap(), kind);
            assert_eq!(kind.name().parse::<OpLayer>().unwrap(), op);
            assert_eq!(kind.name().parse::<RecordLayer>().unwrap(), record);
            assert_eq!(layer_named(kind.name()), Some(kind));

            // Case-insensitivity
            assert_eq!(kind.name().to_lowercase().parse::<Kind>().unwrap(), kind);
            assert_eq!(kind.name().to_lowercase().parse::<OpLayer>().unwrap(), op);
            assert_eq!(
                kind.name().to_lowercase().parse::<RecordLayer>().unwrap(),
                record
            );
            assert_eq!(layer_named(&kind.name().to_lowercase()), Some(kind));

            // Frozen serde serialization contract: JSON string must match Display / name
            let serialized = serde_json::to_string(&record).expect("record::Layer serialization");
            assert_eq!(serialized, format!("\"{}\"", kind.name()));
            let deserialized: RecordLayer =
                serde_json::from_str(&serialized).expect("record::Layer deserialization");
            assert_eq!(deserialized, record);
        }
    }

    #[test]
    fn invalid_layer_strings_fail_parsing() {
        assert!("L0".parse::<Kind>().is_err());
        assert!("L6".parse::<OpLayer>().is_err());
        assert!("invalid".parse::<RecordLayer>().is_err());
        assert_eq!(layer_named("unknown"), None);
    }
}
