//! Serialization and deserialization of Set files, providing [`save`] and [`load`] routines
//! for persisting and reconstructing node topologies, parameters, bindings, and camera states.

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
