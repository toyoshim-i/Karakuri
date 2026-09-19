//! Operation spelling, parameter decoding, and MCP vocabulary curriculum.
//!
//! Decomposed into:
//! - [`schema`]: `Spelled` and `Make` definitions, constant value lists, decoders, and JSON Schema builders.
//! - [`table`]: `SPELLED` table declaring wire spellings for all 64 operations in vocabulary order.
//! - [`dispatch`]: Vocabulary lookup, typo matching, `operate` tool generation, and operations curriculum rendering.

use super::*;

pub(crate) mod dispatch;
pub(crate) mod schema;
pub(crate) mod table;

pub(crate) use dispatch::*;
pub(crate) use schema::*;
pub(crate) use table::*;
