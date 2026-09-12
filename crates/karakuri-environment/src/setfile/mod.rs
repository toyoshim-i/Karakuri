//! Set files: the material, through the record stream at last.
//!
//! Everything else an operator moves — audio, tempo, the mix, the transport —
//! reaches the engine as a record. The *material* did not: two `.kir` paths and
//! a handful of flags went straight into `Set::build`, so the invariant had to
//! say "the performance is on the record path and the material is not". This is
//! the other half.
//!
//! Two directions, and both are needed or neither is worth anything:
//!
//! - [`save`] writes a Set file that references every node's source by hash,
//!   with the capacities, parameters, bindings, camera and seed the run was
//!   using. Getting the bytes into the store is the caller's — see [`Node`].
//! - [`load`] reads one back and returns everything `Set::build_many` and the
//!   flags used to supply.
//!
//! ## The whole chain, not an L1 and its renderers
//!
//! Both halves handled a pair and then a stack: an L1, and the L4s drawn over
//! it. Everything else a `--set` can spell — the L2s that deform, an L3 that
//! looks, a `kind Field` that shapes — was refused by [`save`] and skipped with
//! a note by [`load`], so a cube morphing into a sphere was a Set that could be
//! played and could not be kept, and `--record-session` refused it for the same
//! reason, since a session opens with a Set file.
//!
//! **Nothing in the format had to change to close that.** A `slot` record has
//! carried a layer, an index and a name since the address existed; what was
//! missing was a writer that put a node's own `kind` into it and a reader that
//! honoured the index on every layer rather than on one.
//!
//! ## Where the flag went
//!
//! `--bind`'s fields are `Record::Bind`'s fields, and a debt came with that:
//! **two diagnostics guarding the flag — a `bpm`
//! binding, and `noise.octaves` on a kind that has no octaves — lived only in
//! the flag, and the decoder owed them too.** Paying that by writing them a
//! second time would be two copies of a rule that must not differ.
//!
//! So the flag is now what its documentation always claimed: **a way to write
//! the record**. `parse_bind` turns a `--bind` string into a [`Record::Bind`]
//! and hands it to [`binding_from_record`], which is where every semantic check
//! lives. One rule, one place, and a Set file and a command line cannot disagree
//! about what a binding means.
//!
//! ## Where the format was finer than the engine, and how each was closed
//!
//! A `param` may be a vector where the engine's map holds `f32`, and that used
//! to be reported and dropped. It is **expanded** now: a parameter is driven
//! one component at a time
//! ([ADR-0268](../../../docs/adr/0268-a-vector-parameter-is-driven-one-component-at-a-time.md)),
//! so a `{"t":"param","key":"glow","value":[0.4,0.7,1.0]}` against a `vec3
//! glow` becomes three writes — `glow.x`, `glow.y`, `glow.z`. The wide `Value`
//! earns its keep on the line rather than in the engine: a file, or a model,
//! says the vector once and this expands it.
//!
//! Capacity was a second and `seed` a third, and both were closed the other way
//! round: each is keyed by node, the engine caught up and now holds one per
//! geometry, and so each is carried rather than reported. That is the shape of
//! objection the vector case had to answer — *the engine catches up and the
//! value channel widens* — and it does not fit, because a capacity and a salt
//! are one number per node where a vector is three the pipeline touches one at
//! a time. What a recorded seed buys is more than the symmetry — a salt derived
//! from a source's position in `--set` moves when the list is reordered, and one
//! read back from a file does not.
//!
//! **What is still reported rather than carried is the disagreement, not the
//! width**: a scalar written against a vector declaration names no component, a
//! `vec2` written against a `vec3` is not that parameter, and a binding on a
//! bare vector key resolves to one number with three places to put it. Each is
//! said with the component keys in the sentence. Loading reports what it could
//! not carry — see [`Loaded::notes`] — because a Set file that half-applies is
//! the failure mode this repository keeps refusing: checking clean and coming
//! up short later. `camera` carried two of the six fields the engine's orbit
//! has until 2026-09-09 and was the one gap left in that list; it carries the
//! three an operator can move now, which is what closed it. The lens three —
//! `fov_y`, `near` and `far` — come back as declared, and that is not the same
//! gap under a smaller number: nothing anywhere moves them, so a save has
//! nothing about them to lose (ADR-0318).

pub mod binding;
pub mod bundle;
pub mod codec;
pub mod summary;
pub mod types;

#[cfg(test)]
mod tests;

pub use binding::*;
pub use bundle::*;
pub use codec::*;
pub use summary::*;
pub use types::*;

pub use crate::compile::Names;
pub use crate::meta::{kind_name, kind_of, layer_name, layer_named, layer_of};
