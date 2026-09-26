//! Check-pass tests: verifies example IR files check clean and produce expected
//! resolved shapes, alongside failure mode tests for check-pass invariants.

#[path = "check/common.rs"]
mod common;

#[path = "check/canonical.rs"]
mod canonical;

#[path = "check/failures.rs"]
mod failures;

#[path = "check/accumulation.rs"]
mod accumulation;

#[path = "check/layers.rs"]
mod layers;

#[path = "check/camera.rs"]
mod camera;

#[path = "check/mask.rs"]
mod mask;

#[path = "check/slots.rs"]
mod slots;

#[path = "check/fields.rs"]
mod fields;

#[path = "check/source_and_rate.rs"]
mod source_and_rate;

#[path = "check/effects.rs"]
mod effects;
