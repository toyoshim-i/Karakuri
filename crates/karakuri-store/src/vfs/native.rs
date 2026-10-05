//! Native host filesystem implementation delegating directly to `std::fs`.

#[cfg(not(target_arch = "wasm32"))]
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::trait_def::FileSystem;
use super::types::{DirEntry, File, FileInner, Metadata, OpenOptions, ReadDir};

/// Native OS filesystem backend delegating to `std::fs`.
#[derive(Debug, Default, Clone, Copy)]
pub struct NativeFs;

impl NativeFs {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl FileSystem for NativeFs {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        fs::read_to_string(path)
    }

    fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        fs::write(path, contents)
    }

    fn create_dir(&self, path: &Path) -> io::Result<()> {
        fs::create_dir(path)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        fs::remove_dir_all(path)
    }

    fn copy(&self, from: &Path, to: &Path) -> io::Result<u64> {
        fs::copy(from, to)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(from, to)
    }

    fn metadata(&self, path: &Path) -> io::Result<Metadata> {
        let meta = fs::metadata(path)?;
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::PermissionsExt;
            meta.permissions().mode()
        };
        Ok(Metadata {
            len: meta.len(),
            modified: meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH),
            is_dir: meta.is_dir(),
            permissions: super::types::Permissions {
                readonly: meta.permissions().readonly(),
                #[cfg(unix)]
                mode,
            },
        })
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        fs::canonicalize(path)
    }

    fn read_dir(&self, path: &Path) -> io::Result<ReadDir> {
        let entries = fs::read_dir(path)?;
        let mut list = Vec::new();
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name();
            let meta = entry.metadata()?;
            list.push(DirEntry {
                path,
                file_name,
                is_dir: meta.is_dir(),
                len: meta.len(),
                modified: meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH),
            });
        }
        Ok(ReadDir {
            entries: list.into_iter(),
        })
    }

    fn open_file(&self, path: &Path, options: &OpenOptions) -> io::Result<File> {
        let mut opts = fs::OpenOptions::new();
        opts.read(options.read)
            .write(options.write)
            .append(options.append)
            .truncate(options.truncate)
            .create(options.create)
            .create_new(options.create_new);
        let f = opts.open(path)?;
        Ok(File {
            inner: FileInner::Native(f),
        })
    }
}
