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
//!   the one `docs/roadmap.md` calls the demand most likely to be missed by
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
//! about a `.kir`'s *bytes* needs one. `karakuri-cli` is the one crate that
//! holds both, and it is also the one holding a `Checked` at the moment an
//! artifact is put. See [`crate::Placed`].

use karakuri_ir::typed::Checked;
use karakuri_store::hash::Hash;
use karakuri_store::ndjson::Line;
use karakuri_store::record::Record;

/// The metadata file format version this build writes, on the `meta` record —
/// one number for the whole file, which is what `Record::Set`'s `v` is for a
/// Set file.
const VERSION: u32 = 1;

/// **What one artifact's metadata file says**, in the order it is written.
///
/// `hash` is passed in rather than hashed from the source here, because the
/// caller has it — it is [`crate::Placed::hash`], derived off the node the card
/// belongs to — and a second derivation would be a second answer to "which
/// artifact is this", which is the mistake `Placed::hash` being a method rather
/// than a field exists to stop being made twice. `sort_compiled` builds the
/// node before it builds the card for exactly that reason. See
/// [`crate::Placed::put`], which puts the source and then this.
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
        kind: crate::setfile::layer_of(checked.kind),
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
