//! Virtual filesystem and storage abstractions.

pub mod memory;
#[cfg(not(target_arch = "wasm32"))]
pub mod native;
pub mod trait_def;
pub mod types;
#[cfg(target_arch = "wasm32")]
pub mod web;

pub use memory::MemoryFs;
#[cfg(not(target_arch = "wasm32"))]
pub use native::NativeFs;
pub use trait_def::{FileSystem, NoopStorageProvider, StorageProvider};
#[cfg(unix)]
pub use types::PermissionsExt;
pub use types::{DirEntry, File, FileType, Metadata, OpenOptions, Permissions, ReadDir};
#[cfg(target_arch = "wasm32")]
pub use web::{hydrate_web_fs, IndexedDbStorageProvider};
