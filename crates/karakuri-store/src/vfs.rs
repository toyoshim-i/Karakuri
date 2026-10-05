//! In-memory virtual filesystem for WebAssembly execution.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::{self, Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, RwLock};
use std::time::SystemTime;

include!(concat!(env!("OUT_DIR"), "/bundled_presets.rs"));

static IN_MEMORY: LazyLock<RwLock<BTreeMap<PathBuf, Vec<u8>>>> =
    LazyLock::new(|| RwLock::new(BTreeMap::new()));

fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        use std::path::Component;
        match component {
            Component::Prefix(_) | Component::RootDir => {}
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(c) => {
                normalized.push(c);
            }
        }
    }
    normalized
}

fn get_bundled(path: &Path) -> Option<&'static [u8]> {
    let file_name = path.file_name()?.to_str()?;
    for (name, content) in BUNDLED_PRESETS {
        if *name == file_name {
            return Some(content.as_bytes());
        }
    }
    None
}

pub fn read(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    let path = path.as_ref();
    let norm = normalize(path);
    if let Ok(mem) = IN_MEMORY.read() {
        if let Some(data) = mem.get(&norm) {
            return Ok(data.clone());
        }
    }
    if let Some(bundled) = get_bundled(path) {
        return Ok(bundled.to_vec());
    }
    Err(Error::new(
        ErrorKind::NotFound,
        format!("file not found: {}", path.display()),
    ))
}

pub fn read_to_string(path: impl AsRef<Path>) -> io::Result<String> {
    let bytes = read(path)?;
    String::from_utf8(bytes).map_err(|e| Error::new(ErrorKind::InvalidData, e))
}

pub fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<()> {
    let norm = normalize(path.as_ref());
    let mut mem = IN_MEMORY
        .write()
        .map_err(|_| Error::other("lock poisoned"))?;
    mem.insert(norm, contents.as_ref().to_vec());
    Ok(())
}

pub fn create_dir_all(_path: impl AsRef<Path>) -> io::Result<()> {
    Ok(())
}

pub fn create_dir(_path: impl AsRef<Path>) -> io::Result<()> {
    Ok(())
}

pub fn remove_file(path: impl AsRef<Path>) -> io::Result<()> {
    let norm = normalize(path.as_ref());
    let mut mem = IN_MEMORY
        .write()
        .map_err(|_| Error::other("lock poisoned"))?;
    mem.remove(&norm);
    Ok(())
}

pub fn remove_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    let norm = normalize(path.as_ref());
    let mut mem = IN_MEMORY
        .write()
        .map_err(|_| Error::other("lock poisoned"))?;
    mem.retain(|k, _| !k.starts_with(&norm));
    Ok(())
}

pub fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<u64> {
    let data = read(from)?;
    let len = data.len() as u64;
    write(to, data)?;
    Ok(len)
}

pub fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    let data = read(from.as_ref())?;
    write(to, data)?;
    let _ = remove_file(from);
    Ok(())
}

#[derive(Debug)]
pub struct File {
    path: PathBuf,
}

impl io::Write for File {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut mem = IN_MEMORY
            .write()
            .map_err(|_| Error::other("lock poisoned"))?;
        let entry = mem.entry(normalize(&self.path)).or_default();
        entry.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct OpenOptions {
    create: bool,
    append: bool,
    write: bool,
}

impl OpenOptions {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn create(&mut self, val: bool) -> &mut Self {
        self.create = val;
        self
    }
    pub fn append(&mut self, val: bool) -> &mut Self {
        self.append = val;
        self
    }
    pub fn write(&mut self, val: bool) -> &mut Self {
        self.write = val;
        self
    }
    pub fn open(&self, path: impl AsRef<Path>) -> io::Result<File> {
        let path = path.as_ref().to_path_buf();
        Ok(File { path })
    }
}

#[derive(Debug, Clone)]
pub struct FileType {
    is_dir: bool,
}

impl FileType {
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }
    pub fn is_file(&self) -> bool {
        !self.is_dir
    }
}

#[derive(Debug, Clone)]
pub struct Metadata {
    len: u64,
    modified: SystemTime,
}

impl Metadata {
    pub fn len(&self) -> u64 {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn modified(&self) -> io::Result<SystemTime> {
        Ok(self.modified)
    }
}

pub fn metadata(path: impl AsRef<Path>) -> io::Result<Metadata> {
    let path = path.as_ref();
    let norm = normalize(path);
    if let Ok(bytes) = read(path) {
        return Ok(Metadata {
            len: bytes.len() as u64,
            modified: SystemTime::UNIX_EPOCH,
        });
    }
    let path_str = path.to_string_lossy();
    if path_str == "examples"
        || path_str == ".karakuri"
        || path_str == "."
        || path_str.is_empty()
        || IN_MEMORY
            .read()
            .map(|m| m.keys().any(|k| k.starts_with(&norm)))
            .unwrap_or(false)
    {
        return Ok(Metadata {
            len: 0,
            modified: SystemTime::UNIX_EPOCH,
        });
    }
    Err(Error::new(
        ErrorKind::NotFound,
        format!("file not found: {}", path.display()),
    ))
}

pub fn canonicalize(path: impl AsRef<Path>) -> io::Result<PathBuf> {
    let path = path.as_ref();
    let norm = normalize(path);
    Ok(norm)
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    path: PathBuf,
    file_name: OsString,
    len: u64,
}

impl DirEntry {
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }
    pub fn file_name(&self) -> OsString {
        self.file_name.clone()
    }
    pub fn file_type(&self) -> io::Result<FileType> {
        Ok(FileType { is_dir: false })
    }
    pub fn metadata(&self) -> io::Result<Metadata> {
        Ok(Metadata {
            len: self.len,
            modified: SystemTime::UNIX_EPOCH,
        })
    }
}

pub struct ReadDir {
    entries: std::vec::IntoIter<DirEntry>,
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;
    fn next(&mut self) -> Option<Self::Item> {
        self.entries.next().map(Ok)
    }
}

pub fn read_dir(path: impl AsRef<Path>) -> io::Result<ReadDir> {
    let path = path.as_ref();
    let norm = normalize(path);
    let mut map: BTreeMap<String, DirEntry> = BTreeMap::new();

    let path_str = path.to_string_lossy();
    let is_examples = path_str.contains("examples") || path_str.is_empty() || path_str == ".";
    if is_examples {
        for (name, content) in BUNDLED_PRESETS {
            map.insert(
                name.to_string(),
                DirEntry {
                    path: path.join(name),
                    file_name: OsString::from(name),
                    len: content.len() as u64,
                },
            );
        }
    }

    if let Ok(mem) = IN_MEMORY.read() {
        for (k, v) in mem.iter() {
            if let Some(parent) = k.parent() {
                if parent == norm || (norm.as_os_str().is_empty() && parent.as_os_str().is_empty())
                {
                    if let Some(file_name) = k.file_name() {
                        let name = file_name.to_string_lossy().to_string();
                        map.insert(
                            name,
                            DirEntry {
                                path: k.clone(),
                                file_name: file_name.to_os_string(),
                                len: v.len() as u64,
                            },
                        );
                    }
                }
            }
        }
    }

    Ok(ReadDir {
        entries: map.into_values().collect::<Vec<_>>().into_iter(),
    })
}
