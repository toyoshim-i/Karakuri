//! What may be asked, and by what. The audit
//! [ADR-0235](../../../docs/adr/0235-mcp-reaches-every-operation-and-what-could-stop-the-show-is-refused-until-the-operator-opens-it.md)
//! decided: every operation is connected, and the ones that could stop a
//! performance are refused until the operator opens the class they are in.
//!
//! # It is one classification and one sentence, for every route
//!
//!
//! [ADR-0236](../../../docs/adr/0236-a-map-is-the-layer-between-a-surface-and-the-vocabulary-and-the-audit-is-one-of-the-things-it-does.md)
//! is why it is not in the MCP server: *"a rule held by one surface binds one
//! surface"*, and a sequencer lane is already decided as a fifth route
//! (ADR-0222) which would arrive with a second copy of this table. The audit
//! belongs to the map — the layer between a surface and the vocabulary — and
//! that layer does not exist as a crate: `karakuri-midi` is its only built
//! instance and ADR-0236 deliberately builds none of the rest of it.
//!
//! So this sits in the leaf every surface already depends on, and that is a
//! co-location rather than a claim that the audit is vocabulary. ADR-0236 is
//! explicit that *the map is not the vocabulary*, and this module is not an
//! [`Operation`]: it names none, it adds none, and `Operation::TITLES` is
//! untouched by it. What it buys by being here is the thing ADR-0236 asks for —
//! `karakuri-console`, `karakuri-midi`, `karakuri-cli` and
//! `karakuri-environment` all depend on this crate and on nothing in common
//! besides, so the check and the sentence are written once and no future route
//! needs a new dependency to reach them. The day the map layer is a crate this
//! module moves into it whole. (`docs/contributing.md` §4: *every surface
//! reaches the vocabulary through a map* is the shape being named, not the
//! shape that exists.)
//!
//! # The check is a type, not a call at the top of a function
//!
//! *"An audit skipped on one path is the whole mechanism gone."* So [`audit`]
//! is the only constructor of [`Allowed`], [`Allowed`]'s field is private, and
//! a performer takes an [`Allowed`] rather than an [`Operation`]. A caller in
//! another crate cannot reach the performer without having been through here;
//! forgetting the check is a compile error rather than a review comment.
//! (`docs/contributing.md` §4.)
//!
//! # Closed by default is the type's own default
//!
//! [`Open`]'s fields are private and every one of them is `false` to begin, so
//! there is no literal anywhere that starts a class open: the only way to an
//! open class is [`Open::with`], which names it. A caller that says nothing has
//! closed all four.
//!
//! # The classification is exhaustive over the vocabulary
//!
//! [`standing`] is a `match` with no wildcard arm, which is
//! `karakuri_operation_record::written`'s discipline and its reason: *"an
//! operation added to the vocabulary stops the build here until somebody says
//! what it writes, so the classification cannot drift the way a wildcard arm
//! would let it."* A sixty-fifth operation does not compile until somebody says
//! which class it is in.

pub mod rules;
pub mod types;

#[cfg(test)]
mod tests;

pub use rules::*;
pub use types::*;
