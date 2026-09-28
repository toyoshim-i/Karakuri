//! Pass execution, parameter binding, and runtime frame processing for Set instances.

mod common;
mod execution;
mod params;
mod uniforms;

pub(crate) use execution::allocate_hdr_target;
