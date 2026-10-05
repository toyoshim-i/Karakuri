//! Abstraction traits for pluggable filesystems and persistent storage providers.

use std::io;
use std::path::{Path, PathBuf};

use super::types::{File, Metadata, OpenOptions, ReadDir};

/// Core filesystem interface abstracting standard file and directory operations.
pub trait FileSystem: Send + Sync {
    /// Reads entire file contents into a byte buffer.
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;

    /// Reads entire file contents into a UTF-8 string.
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        let bytes = self.read(path)?;
        String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Writes raw byte contents to a file, replacing existing content.
    fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()>;

    /// Creates a directory if it does not exist.
    fn create_dir(&self, path: &Path) -> io::Result<()>;

    /// Recursively creates a directory and all parent directories.
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;

    /// Removes an existing file.
    fn remove_file(&self, path: &Path) -> io::Result<()>;

    /// Recursively removes a directory and all its contents.
    fn remove_dir_all(&self, path: &Path) -> io::Result<()>;

    /// Copies a file from `from` to `to`, returning total bytes copied.
    fn copy(&self, from: &Path, to: &Path) -> io::Result<u64>;

    /// Renames a file or directory from `from` to `to`.
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;

    /// Queries file or directory metadata.
    fn metadata(&self, path: &Path) -> io::Result<Metadata>;

    /// Canonicalizes a path into its normalized form.
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;

    /// Reads directory entries.
    fn read_dir(&self, path: &Path) -> io::Result<ReadDir>;

    /// Opens a file with given open options.
    fn open_file(&self, path: &Path, options: &OpenOptions) -> io::Result<File>;

    /// Returns true if the path exists.
    fn exists(&self, path: &Path) -> bool {
        self.metadata(path).is_ok()
    }
}

/// Asynchronous or synchronous backend persistence interface for virtual filesystems.
pub trait StorageProvider: Send + Sync {
    /// Persists written file contents for `path`.
    fn persist_write(&self, path: &Path, bytes: &[u8]);

    /// Persists deletion of file at `path`.
    fn persist_remove(&self, path: &Path);

    /// Persists file rename or move from `from` to `to`.
    fn persist_rename(&self, from: &Path, to: &Path);
}

/// Default no-op storage provider (in-memory only).
pub struct NoopStorageProvider;

impl StorageProvider for NoopStorageProvider {
    fn persist_write(&self, _path: &Path, _bytes: &[u8]) {}
    fn persist_remove(&self, _path: &Path) {}
    fn persist_rename(&self, _from: &Path, _to: &Path) {}
}
