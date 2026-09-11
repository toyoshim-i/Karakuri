//! The content-addressed artifact store and the ndjson record formats.
//!
//! Artifacts are immutable and identified by the hash of their `.kir` source.
//! Metadata is regenerated from the source plus a compile pass, so a
//! `.meta.ndjson` is a reproducible artifact rather than a hand-authored one.

pub mod hash;
pub mod ndjson;
pub mod project;
pub mod record;
pub mod schema;
pub mod store;
pub mod stream;

pub use hash::{Hash, HashParseError};
pub use ndjson::Line;
pub use project::project;
pub use record::{BindNoise, Layer, NodeAddress, Record, Value, CURRENT_SCHEMA_VERSION, MAX_STEPS};
pub use schema::{detect_version, migrate_to_current};
pub use store::{ArrangementEntry, ArtifactEntry, ProcedureEntry, SetEntry, Store, StoreError};
pub use stream::{MmapStreamReader, RawLines, TickIndexEntry};
