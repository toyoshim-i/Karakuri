//! Content-addressed artifact store and ndjson record formats.
//! Artifacts are immutable and metadata is reproducible from `.kir` source.

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
