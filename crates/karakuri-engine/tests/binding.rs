//! Integration tests for parameter bindings, audio modulation, and GPU signals.

#[path = "common/mod.rs"]
mod engine_common;

#[path = "binding/common.rs"]
mod common;

#[path = "binding/pure.rs"]
mod pure;

#[path = "binding/gpu_params.rs"]
mod gpu_params;

#[path = "binding/gpu_session.rs"]
mod gpu_session;
