//! **An artifact's metadata card**, produced from the compile that already
//! happened.
//!
//! `docs/ir-spec.md`, "Metadata file format", specifies `<hash>.meta.ndjson` as
//! nine record types and says the store regenerates it *"from the `.kir` plus a
//! compile pass"*. This is that sentence, for the four of the nine a compile
//! pass can answer: `meta`, `param_decl`, `capacity_decl` and `emit`. Every one
//! of them is read straight off a [`Checked`] and off nothing else.
//!
//! **The five that are not here have no producer, and an empty one would be
//! worse than none.**
//!
//! - `perf` carries `ns_per_element`, which is a *measurement*. What a compile
//!   pass can answer is `ops_per_element`, from `karakuri_ir::cost::estimate`,
//!   which is a different quantity in different units; writing one under the
//!   other's name is exactly the shape of number the specification withdrew a
//!   `bytes_per_element` for — a figure published under a name that reads as a
//!   measurement, with no way for the record to say which of the two it is. It
//!   wants the stage-7 probe. **Not `Checked::cost`**, which is the field named
//!   for that number and reads as the place to find it: the check pass sets it
//!   `None` unconditionally and nothing else fills it, so a writer reaching for
//!   it here would publish an absence rather than the wrong number — see its
//!   own doc, which says to ask `cost::estimate` instead.
//! - `origin`, `parent` and `tag` describe where an artifact *came from*, and
//!   nothing generates procedures yet, so there is nothing to say. `parent` is
//!   the demand most likely to be missed by
//!   arriving late: leaving room for it is the point, and writing a `parent`
//!   with nothing in it would fill the hole with a lie rather than leave it
//!   visible.
//! - `thumbnail` is a stored asset — a short loop and a still, rendered at
//!   promotion time — and nothing renders one. It is `thumbnail` and not
//!   `preview` because the deck's `preview` is a different thing under a name
//!   that was taken; see `Record::is_metadata`.
//!
//! **This crate rather than either of the two it joins.** The records are
//! `karakuri-store`'s and the declarations are `karakuri-ir`'s, and the store
//! does not depend on the IR — a content-addressed blob store that had to link
//! a type checker to write a file name would be the wrong shape, and nothing
//! about a `.kir`'s *bytes* needs one. This package holds both, and it is also
//! the one holding a `Checked` at the moment an artifact is put — see
//! [`crate::compile::Placed`], which is the caller and is in this package now.

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

/// **What one artifact's metadata file says**, in the order it is written.
///
/// `hash` is passed in rather than hashed from the source here, because the
/// caller has it — it is [`crate::compile::Placed::hash`], derived off the node
/// the card belongs to — and a second derivation would be a second answer to
/// "which artifact is this", which is the mistake `Placed::hash` being a method
/// rather than a field exists to stop being made twice. `sort_compiled` builds
/// the node before it builds the card for exactly that reason. See
/// `Placed::put`, which puts the source and then this.
///
/// **A `Vec<Line>` and not a written file**: what a card says is decided here,
/// where a `Checked` is, and where it lands is `karakuri-store`'s. That split
/// is what lets the whole of this be checked without a store, a disk or a GPU.
pub fn card(hash: &Hash, checked: &Checked) -> Vec<Line> {
    let mut lines = vec![Line::new(Record::Meta {
        hash: *hash,
        // **The procedure's own declared name**, from `proc <name>`, and never
        // the name a Set gave the node. That one belongs to the *use* — see
        // `Record::Slot`'s `name` — and one artifact loaded twice under two
        // names is still one artifact with one card.
        name: checked.name.clone(),
        kind: layer_of(checked.kind),
        v: VERSION,
    })];
    // **Declaration order, which is the file's.** `Checked::params` is built by
    // walking the header, so the card reads down the `.kir` — a surface
    // rendering knobs from this gets the order the author chose rather than
    // whatever a sort would have imposed on it.
    for p in &checked.params {
        lines.push(Line::new(Record::ParamDecl {
            key: p.name.clone(),
            ty: p.ty.name().to_string(),
            min: p.min,
            max: p.max,
            // **One fold, `karakuri-ir`'s, and the same call the engine makes
            // when it builds the uniform.** A reader of its own here would
            // agree with the shader by coincidence, and the coincidence breaks
            // on `= -0.35`: a card claiming `0.0` where the run loaded `-0.35`
            // describes a procedure nobody ran, and nothing downstream could
            // tell which of the two was wrong. See
            // `karakuri_ir::Param::default_scalar` and `Record::ParamDecl`'s
            // `default`, which is why `None` is written as an absent key rather
            // than as an absent record.
            default: p.default_scalar(),
        }));
    }
    // Only an L1 declares one, so most cards have no `capacity_decl` — and an
    // absent record is the honest form of "this procedure declares no
    // capacity", where zeroes would be a range nobody wrote.
    if let Some(c) = checked.capacity {
        lines.push(Line::new(Record::CapacityDecl {
            min: c.min,
            max: c.max,
            default: c.default,
        }));
    }
    // The same rule: a renderer emits nothing and gets no record, rather than
    // an `emit` with an empty list. Both read as "nothing declared" and only
    // one of them is a line somebody has to explain.
    if !checked.emit.is_empty() {
        lines.push(Line::new(Record::Emit {
            attrs: checked.emit.iter().map(|a| a.name().to_string()).collect(),
        }));
    }
    lines
}

/// **Write an artifact's metadata card, and never let it stop a save.**
///
/// The one place either put path says this, so that the judgement — the card is
/// derived and the artifact is not — is made once and the sentence is one
/// sentence. See [`crate::compile::Placed::put`], which is the other caller's
/// other half.
///
/// The hash is passed rather than recomputed: the caller has just put the bytes
/// under it, and deriving it again here would be a second answer to which
/// artifact this card is for.
///
/// **It hands back what it said, and both callers drop it.** The policy — a
/// card that will not write is reported and does not fail the save — was
/// asserted in prose and nowhere else, because a sentence that is only printed
/// is a sentence no test can hold. Returning it costs one `Option` and buys
/// `karakuri-cli`'s
/// `live_save_tests::a_card_that_will_not_write_is_said_and_does_not_fail_the_save`,
/// which reads it back. `None` is a card on disk.
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
