//! The content-addressed artifact store and the ndjson record formats.
//!
//! Artifacts are immutable and identified by the hash of their `.kir` source.
//! Metadata is regenerated from the source plus a compile pass, so a
//! `.meta.ndjson` is a reproducible artifact rather than a hand-authored one.

pub mod hash;
pub mod record;
pub mod store;

pub use hash::{Hash, HashParseError};
pub use record::{Layer, Record, Value, MAX_STEPS};
pub use store::{Store, StoreError};
