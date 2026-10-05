//! Model Context Protocol (MCP) server for Karakuri.
//! Maps incoming tool invocations over HTTP loopback to engine operations and inspection queries.

pub mod in_process;
pub mod protocol;
pub mod resources;
pub mod server;
pub mod spelled;
pub mod state;
pub mod tools;

#[cfg(test)]
mod tests;

pub use in_process::{in_process, InProcessMcp};
pub use karakuri_ir::{Diagnostic, DiagnosticReport};
pub use protocol::PROTOCOL;
pub use server::{serve, serve_at, Pending};
pub use state::{Event, News, Pointed, Reply, Reporter, SaveRequest, Slots, WireRequest};
pub use tools::{check_procedure, check_set_configuration, checked_id, OperateRequest, LISTED};

pub(crate) use protocol::*;
pub(crate) use resources::*;
pub(crate) use spelled::*;
pub(crate) use state::*;
pub(crate) use tools::*;
