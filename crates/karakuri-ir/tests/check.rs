//! Check-pass tests: the three complete `.kir` examples from `docs/ir-spec.md`
//! must check clean and produce the resolved shape we expect, and a battery of
//! invalid fixtures exercises every failure mode the check pass is
//! responsible for.

#[path = "check/common.rs"]
mod common;

#[path = "check/canonical.rs"]
mod canonical;

#[path = "check/layers.rs"]
mod layers;

#[path = "check/slots.rs"]
mod slots;

#[path = "check/effects.rs"]
mod effects;
