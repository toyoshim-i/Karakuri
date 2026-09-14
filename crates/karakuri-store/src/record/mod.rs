//! ndjson records.
//!
//! One record per line, and concatenation is composition. The same record types
//! serve three files:
//!
//! - a Set file (`.kbset`) is a state projection — what is loaded and what
//! every value currently is. It carries no time, so it never contains
//! [`Record::Tick`]. - a session stream is a timeline — a Set file followed by
//! ticks and the edits between them. Every edit lands at an exact frame
//! position because it sits between two known ticks. - an artifact's metadata
//! (`<hash>.meta.ndjson`) is what one procedure *declares*, regenerated from
//! its `.kir` plus a compile pass. It is neither of the other two and is read
//! by a decoder of its own — see [`Record::is_metadata`].
//!
//! A Set file is the session stream with the ticks dropped and the state folded
//! down. See the Set file, session stream and metadata file sections of
//! `docs/ir-spec.md`.
//!
//! One type for all three, and the names disjoint across them. The
//! specification's rule is that one `t` means one shape in every file, which is
//! a rule about *names* — `param_decl` beside `param` — and sharing the Rust
//! type is what lets a decoder tell a record in the wrong file from a record it
//! has never heard of. Three enums could not: a `param_decl` read by the Set
//! decoder would come back [`Record::Unknown`], and the format promises to pass
//! over exactly that.
//!
//! `slot` is three things in this vocabulary, and all three now have their own
//! type. [`Record::Slot`] is a node of a Set — a procedure at a `(layer,
//! index)` address, optionally with a name. The `slot` field, [`DeckSlot`], on
//! [`Record::Gain`], [`Record::Opacity`], [`Record::Blend`],
//! [`Record::Residency`], [`Record::Procedure`], [`Record::Authority`],
//! [`Record::Ride`], [`Record::Source`], [`Record::Mask`],
//! [`Record::Transition`], [`Record::Select`], [`Record::Transport`] and
//! [`Record::Save`] is a member of the deck — an index into the mixer, and
//! nothing about the Set in it. [`Record::Edge`]'s `slot` is [`InputPort`], the
//! third: an input a node declares, which is what `uses far : Geometry` names.
//!
//! None of the three is renamed, at the Rust identifier or on the wire.
//! `docs/adr/0049-slot-means-two-things-and-the-clash-is-recorded.md` recorded
//! the ambiguity rather than resolving it;
//! `docs/adr/0344-slot-is-disambiguated-into-three-types-and-adr-0049s-wait-is-over.md`
//! supersedes it and gives each sense its own type instead — [`InputPort`]
//! first, because nothing about that sense was ever waiting on a GUI or a
//! settled record format the way the other two were, and [`DeckSlot`] second,
//! on the deck sense alone: a member of the deck across thirteen record types.
//! The field name stays `slot` everywhere: `docs/contributing.md` §4's *"they
//! have to be disjoint by name"* was already met by the sentence, not the
//! identifier, so a type change pays for the disambiguation without also paying
//! for a thirteen-record rename.
//!
//! A wire field can be renamed, and one has been. [`Record::Transport`]'s scrub
//! was spelled `offset_beats` and collided with the operator's latency offset;
//! it is `scrub_beats` now, on disk as well as in Rust, because before v1 the
//! bill is a bill and not an argument — see that variant's own documentation
//! and
//! `docs/principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md`.
//! What made that affordable and `slot` not is the size of the bill, not a rule
//! against the move.
//!
//! The one collision there was is settled, and it is the metadata name that
//! moved. `docs/ir-spec.md` listed a metadata `preview` carrying a `path`
//! beside a deck record `preview` carrying an `Option<u8>` slot — which slot
//! was being auditioned. Two shapes under one `t`, and silently so: the deck
//! record's `slot` was an `Option`, so the specified line decoded as the deck
//! record with its `path` dropped and nothing said — the one record for which
//! "an unknown `t` is ignored" protected nothing, because the `t` was not
//! unknown. The library asset is spelled `thumbnail` now, which is the word the
//! library already used for it — see
//! `docs/adr/0134-the-metadata-preview-becomes-thumbnail.md`. The deck record
//! is gone:
//! `docs/adr/0240-the-output-shows-the-mix-and-residency-keys-belong-to-the-mixer.md`
//! retired the operation behind it, and switching a preview turned out to be a
//! bay-internal move rather than an engine one, so there is nothing to record.
//! The name is free and is not being reused: `thumbnail` is the word.

pub mod helpers;
pub mod types;
pub mod variants;

#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub use helpers::*;
pub use types::*;
pub use variants::*;
