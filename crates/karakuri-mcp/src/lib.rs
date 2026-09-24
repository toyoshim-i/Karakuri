//! Model Context Protocol (MCP) server for Karakuri.
//!
//! Provides an MCP interface over HTTP loopback to allow tools and models to query
//! slot procedures, trigger hot-swaps, wire inputs, and interact with the engine.
//!
//! Requests and modifications are mapped into standard engine operations and
//! evaluated against validation and budget checks before application.

pub mod protocol;
pub mod resources;
pub mod server;
pub mod spelled;
pub mod state;
pub mod tools;

#[cfg(test)]
mod tests;

pub use karakuri_ir::{Diagnostic, DiagnosticReport};
pub use protocol::PROTOCOL;
pub use server::serve;
pub use state::{Event, Pointed, Reply, Reporter, SaveRequest, Slots, WireRequest};
pub use tools::{check_procedure, check_set_configuration, checked_id, OperateRequest, LISTED};

pub(crate) use protocol::*;
pub(crate) use resources::*;
pub(crate) use spelled::*;
pub(crate) use state::*;
pub(crate) use tools::*;
