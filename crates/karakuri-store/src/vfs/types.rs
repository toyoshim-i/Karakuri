//! Filesystem metadata and directory entry types matching `std::fs` representations.

use std::ffi::OsString;
use std::io::{self, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct FileType {
    pub(crate) is_dir: bool,
}

impl FileType {
    pub fn new_file() -> Self {
        Self { is_dir: false }
    }
    pub fn new_dir() -> Self {
        Self { is_dir: true }
    }
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }
    pub fn is_file(&self) -> bool {
        !self.is_dir
    }
    pub fn is_symlink(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone)]
pub struct Permissions {
    pub(crate) readonly: bool,
    #[cfg(unix)]
    pub(crate) mode: u32,
}

impl Permissions {
    pub fn readonly(&self) -> bool {
        self.readonly
    }
}

#[cfg(unix)]
pub trait PermissionsExt {
    fn mode(&self) -> u32;
}

#[cfg(unix)]
impl PermissionsExt for Permissions {
    fn mode(&self) -> u32 {
        self.mode
    }
}

#[derive(Debug, Clone)]
pub struct Metadata {
    pub(crate) len: u64,
    pub(crate) modified: SystemTime,
    pub(crate) is_dir: bool,
    pub(crate) permissions: Permissions,
}

impl Metadata {
    pub fn new_file(len: u64, modified: SystemTime) -> Self {
        Self {
            len,
            modified,
            is_dir: false,
            permissions: Permissions {
                readonly: false,
                #[cfg(unix)]
                mode: 0o755,
            },
        }
    }
    pub fn new_dir(modified: SystemTime) -> Self {
        Self {
            len: 0,
            modified,
            is_dir: true,
            permissions: Permissions {
                readonly: false,
                #[cfg(unix)]
                mode: 0o755,
            },
        }
    }
    pub fn len(&self) -> u64 {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }
    pub fn is_file(&self) -> bool {
        !self.is_dir
    }
    pub fn modified(&self) -> io::Result<SystemTime> {
        Ok(self.modified)
    }
    pub fn permissions(&self) -> Permissions {
        self.permissions.clone()
    }
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub(crate) path: PathBuf,
    pub(crate) file_name: OsString,
    pub(crate) is_dir: bool,
    pub(crate) len: u64,
    pub(crate) modified: SystemTime,
}

impl DirEntry {
    pub fn new(
        path: PathBuf,
        file_name: OsString,
        is_dir: bool,
        len: u64,
        modified: SystemTime,
    ) -> Self {
        Self {
            path,
            file_name,
            is_dir,
            len,
            modified,
        }
    }
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }
    pub fn file_name(&self) -> OsString {
        self.file_name.clone()
    }
    pub fn file_type(&self) -> io::Result<FileType> {
        Ok(FileType {
            is_dir: self.is_dir,
        })
    }
    pub fn metadata(&self) -> io::Result<Metadata> {
        if self.is_dir {
            Ok(Metadata::new_dir(self.modified))
        } else {
            Ok(Metadata::new_file(self.len, self.modified))
        }
    }
}

pub struct ReadDir {
    pub(crate) entries: std::vec::IntoIter<DirEntry>,
}

impl ReadDir {
    pub fn from_entries(entries: Vec<DirEntry>) -> Self {
        Self {
            entries: entries.into_iter(),
        }
    }
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;
    fn next(&mut self) -> Option<Self::Item> {
        self.entries.next().map(Ok)
    }
}

/// Callback hook invoked when an in-memory file handle flushes writes.
pub type FlushHook = Arc<dyn Fn(&Path, &[u8]) -> io::Result<()> + Send + Sync>;

/// Abstract in-memory file handle supporting Write and Seek.
#[derive(Clone)]
pub struct MemoryFileHandle {
    pub path: PathBuf,
    pub cursor: usize,
    pub on_flush: Option<FlushHook>,
    pub buffer: Arc<Mutex<Vec<u8>>>,
}

pub struct File {
    pub(crate) inner: FileInner,
}

impl File {
    /// Opens a file in write-only mode, creating it if it doesn't exist and truncating it if it does.
    pub fn create(path: impl AsRef<Path>) -> io::Result<Self> {
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
    }

    /// Opens a file in read-only mode.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        OpenOptions::new().read(true).open(path)
    }
}

pub(crate) enum FileInner {
    #[cfg(not(target_arch = "wasm32"))]
    Native(std::fs::File),
    Memory(MemoryFileHandle),
}

impl io::Write for File {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match &mut self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            FileInner::Native(f) => f.write(buf),
            FileInner::Memory(mem) => {
                let mut guard = mem
                    .buffer
                    .lock()
                    .map_err(|_| io::Error::other("lock error"))?;
                if mem.cursor > guard.len() {
                    guard.resize(mem.cursor, 0);
                }
                let end = mem.cursor + buf.len();
                if end > guard.len() {
                    guard.resize(end, 0);
                }
                guard[mem.cursor..end].copy_from_slice(buf);
                mem.cursor = end;
                Ok(buf.len())
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match &mut self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            FileInner::Native(f) => f.flush(),
            FileInner::Memory(mem) => {
                if let Some(ref on_flush) = mem.on_flush {
                    let guard = mem
                        .buffer
                        .lock()
                        .map_err(|_| io::Error::other("lock error"))?;
                    on_flush(&mem.path, &guard)?;
                }
                Ok(())
            }
        }
    }
}

impl io::Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match &mut self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            FileInner::Native(f) => f.read(buf),
            FileInner::Memory(mem) => {
                let guard = mem
                    .buffer
                    .lock()
                    .map_err(|_| io::Error::other("lock error"))?;
                if mem.cursor >= guard.len() {
                    return Ok(0);
                }
                let available = guard.len() - mem.cursor;
                let to_read = available.min(buf.len());
                buf[..to_read].copy_from_slice(&guard[mem.cursor..mem.cursor + to_read]);
                mem.cursor += to_read;
                Ok(to_read)
            }
        }
    }
}

impl Seek for File {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match &mut self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            FileInner::Native(f) => f.seek(pos),
            FileInner::Memory(mem) => {
                let guard = mem
                    .buffer
                    .lock()
                    .map_err(|_| io::Error::other("lock error"))?;
                let new_pos = match pos {
                    SeekFrom::Start(off) => off as i64,
                    SeekFrom::Current(off) => mem.cursor as i64 + off,
                    SeekFrom::End(off) => guard.len() as i64 + off,
                };
                if new_pos < 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "negative seek offset",
                    ));
                }
                mem.cursor = new_pos as usize;
                Ok(mem.cursor as u64)
            }
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct OpenOptions {
    pub(crate) read: bool,
    pub(crate) write: bool,
    pub(crate) append: bool,
    pub(crate) truncate: bool,
    pub(crate) create: bool,
    pub(crate) create_new: bool,
}

impl OpenOptions {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn read(&mut self, val: bool) -> &mut Self {
        self.read = val;
        self
    }
    pub fn write(&mut self, val: bool) -> &mut Self {
        self.write = val;
        self
    }
    pub fn append(&mut self, val: bool) -> &mut Self {
        self.append = val;
        self
    }
    pub fn truncate(&mut self, val: bool) -> &mut Self {
        self.truncate = val;
        self
    }
    pub fn create(&mut self, val: bool) -> &mut Self {
        self.create = val;
        self
    }
    pub fn create_new(&mut self, val: bool) -> &mut Self {
        self.create_new = val;
        self
    }
    pub fn open(&self, path: impl AsRef<Path>) -> io::Result<File> {
        crate::fs::open_file_with_options(path.as_ref(), self)
    }
}
