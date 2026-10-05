//! Unified, pluggable filesystem interface.
//!
//! Provides top-level std::fs-compatible functions dispatched dynamically
//! through the active `FileSystem` backend.
//!
//! On native platforms, defaults to `NativeFs` (delegating to `std::fs`).
//! On WebAssembly (`wasm32`), defaults to `MemoryFs` paired with IndexedDB persistence.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, RwLock};

#[cfg(not(target_arch = "wasm32"))]
use crate::vfs::NativeFs;
#[cfg(unix)]
pub use crate::vfs::PermissionsExt;
pub use crate::vfs::{
    DirEntry, File, FileSystem, FileType, MemoryFs, Metadata, OpenOptions, Permissions, ReadDir,
    StorageProvider,
};

#[cfg(target_arch = "wasm32")]
static DEFAULT_MEMORY_FS: LazyLock<Arc<MemoryFs>> = LazyLock::new(|| Arc::new(MemoryFs::new()));

static CURRENT_FS: LazyLock<RwLock<Arc<dyn FileSystem>>> = LazyLock::new(|| {
    #[cfg(not(target_arch = "wasm32"))]
    {
        RwLock::new(Arc::new(NativeFs::new()))
    }
    #[cfg(target_arch = "wasm32")]
    {
        RwLock::new(DEFAULT_MEMORY_FS.clone())
    }
});

#[cfg(target_arch = "wasm32")]
/// Returns the shared default MemoryFs instance on wasm32 for storage hydration.
pub fn default_memory_fs() -> Arc<MemoryFs> {
    DEFAULT_MEMORY_FS.clone()
}

/// Returns a reference clone to the currently active global filesystem instance.
pub fn current_fs() -> Arc<dyn FileSystem> {
    CURRENT_FS.read().expect("fs lock poisoned").clone()
}

/// Replaces the active global filesystem implementation.
pub fn set_filesystem(fs: Arc<dyn FileSystem>) {
    let mut lock = CURRENT_FS.write().expect("fs lock poisoned");
    *lock = fs;
}

/// Executes a closure with a temporary filesystem backend, restoring the previous backend on completion.
pub fn with_filesystem<R>(fs: Arc<dyn FileSystem>, f: impl FnOnce() -> R) -> R {
    let prev = current_fs();
    set_filesystem(fs);
    let res = f();
    set_filesystem(prev);
    res
}

/// Reads the entire contents of a file into a bytes vector.
pub fn read(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    current_fs().read(path.as_ref())
}

/// Reads the entire contents of a file into a UTF-8 string.
pub fn read_to_string(path: impl AsRef<Path>) -> io::Result<String> {
    current_fs().read_to_string(path.as_ref())
}

/// Writes a slice as the entire contents of a file.
pub fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<()> {
    current_fs().write(path.as_ref(), contents.as_ref())
}

/// Creates a new, empty directory at the provided path.
pub fn create_dir(path: impl AsRef<Path>) -> io::Result<()> {
    current_fs().create_dir(path.as_ref())
}

/// Recursively create a directory and all of its parent components if they are missing.
pub fn create_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    current_fs().create_dir_all(path.as_ref())
}

/// Removes a file from the filesystem.
pub fn remove_file(path: impl AsRef<Path>) -> io::Result<()> {
    current_fs().remove_file(path.as_ref())
}

/// Removes a directory at this path, after removing all its contents.
pub fn remove_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    current_fs().remove_dir_all(path.as_ref())
}

/// Copies the contents of one file to another.
pub fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<u64> {
    current_fs().copy(from.as_ref(), to.as_ref())
}

/// Rename a file or directory to a new name, replacing the original file if to already exists.
pub fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    current_fs().rename(from.as_ref(), to.as_ref())
}

/// Given a path, query the file system to get information about a file, directory, etc.
pub fn metadata(path: impl AsRef<Path>) -> io::Result<Metadata> {
    current_fs().metadata(path.as_ref())
}

/// Returns the canonical, absolute form of a path with all intermediate components normalized.
pub fn canonicalize(path: impl AsRef<Path>) -> io::Result<PathBuf> {
    current_fs().canonicalize(path.as_ref())
}

/// Returns an iterator over the entries within a directory.
pub fn read_dir(path: impl AsRef<Path>) -> io::Result<ReadDir> {
    current_fs().read_dir(path.as_ref())
}

pub(crate) fn open_file_with_options(path: &Path, options: &OpenOptions) -> io::Result<File> {
    current_fs().open_file(path, options)
}
