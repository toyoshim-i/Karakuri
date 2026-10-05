//! In-memory virtual filesystem with bundled preset fallback and pluggable persistence provider.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::{self, Error, ErrorKind};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::SystemTime;

use super::trait_def::{FileSystem, StorageProvider};
use super::types::{DirEntry, File, FileInner, MemoryFileHandle, Metadata, OpenOptions, ReadDir};

include!(concat!(env!("OUT_DIR"), "/bundled_presets.rs"));

/// Normalizes a path by removing redundant prefixes and directory navigations.
pub fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
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

/// In-memory virtual filesystem storing files in a synchronized map with optional backend persistence.
pub struct MemoryFs {
    files: Arc<RwLock<BTreeMap<PathBuf, Vec<u8>>>>,
    provider: RwLock<Option<Arc<dyn StorageProvider>>>,
}

impl Default for MemoryFs {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryFs {
    /// Creates an empty in-memory filesystem with no persistence provider.
    pub fn new() -> Self {
        Self {
            files: Arc::new(RwLock::new(BTreeMap::new())),
            provider: RwLock::new(None),
        }
    }

    /// Creates an in-memory filesystem paired with a persistence provider.
    pub fn with_provider(provider: Arc<dyn StorageProvider>) -> Self {
        Self {
            files: Arc::new(RwLock::new(BTreeMap::new())),
            provider: RwLock::new(Some(provider)),
        }
    }

    /// Sets or replaces the active persistence provider.
    pub fn set_provider(&self, provider: Arc<dyn StorageProvider>) {
        if let Ok(mut lock) = self.provider.write() {
            *lock = Some(provider);
        }
    }

    /// Hydrates the in-memory filesystem with pre-existing persistent files (e.g. from IndexedDB).
    pub fn hydrate(&self, entries: impl IntoIterator<Item = (PathBuf, Vec<u8>)>) {
        if let Ok(mut files) = self.files.write() {
            for (path, content) in entries {
                files.insert(normalize(&path), content);
            }
        }
    }

    /// Returns a snapshot of all currently stored in-memory file paths and sizes.
    pub fn dump_entries(&self) -> Vec<(PathBuf, usize)> {
        if let Ok(files) = self.files.read() {
            files.iter().map(|(p, v)| (p.clone(), v.len())).collect()
        } else {
            Vec::new()
        }
    }
}

impl FileSystem for MemoryFs {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        let norm = normalize(path);
        if let Ok(mem) = self.files.read() {
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

    fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        let norm = normalize(path);
        {
            let mut mem = self
                .files
                .write()
                .map_err(|_| Error::other("lock poisoned"))?;
            mem.insert(norm.clone(), contents.to_vec());
        }
        if let Ok(provider_lock) = self.provider.read() {
            if let Some(ref provider) = *provider_lock {
                provider.persist_write(&norm, contents);
            }
        }
        Ok(())
    }

    fn create_dir(&self, _path: &Path) -> io::Result<()> {
        Ok(())
    }

    fn create_dir_all(&self, _path: &Path) -> io::Result<()> {
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        let norm = normalize(path);
        {
            let mut mem = self
                .files
                .write()
                .map_err(|_| Error::other("lock poisoned"))?;
            mem.remove(&norm);
        }
        if let Ok(provider_lock) = self.provider.read() {
            if let Some(ref provider) = *provider_lock {
                provider.persist_remove(&norm);
            }
        }
        Ok(())
    }

    fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        let norm = normalize(path);
        let mut removed = Vec::new();
        {
            let mut mem = self
                .files
                .write()
                .map_err(|_| Error::other("lock poisoned"))?;
            let keys: Vec<PathBuf> = mem
                .keys()
                .filter(|k| k.starts_with(&norm))
                .cloned()
                .collect();
            for k in keys {
                mem.remove(&k);
                removed.push(k);
            }
        }
        if let Ok(provider_lock) = self.provider.read() {
            if let Some(ref provider) = *provider_lock {
                for k in removed {
                    provider.persist_remove(&k);
                }
            }
        }
        Ok(())
    }

    fn copy(&self, from: &Path, to: &Path) -> io::Result<u64> {
        let data = self.read(from)?;
        let len = data.len() as u64;
        self.write(to, &data)?;
        Ok(len)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        let norm_from = normalize(from);
        let norm_to = normalize(to);
        let data = self.read(&norm_from)?;
        {
            let mut mem = self
                .files
                .write()
                .map_err(|_| Error::other("lock poisoned"))?;
            mem.insert(norm_to.clone(), data);
            mem.remove(&norm_from);
        }
        if let Ok(provider_lock) = self.provider.read() {
            if let Some(ref provider) = *provider_lock {
                provider.persist_rename(&norm_from, &norm_to);
            }
        }
        Ok(())
    }

    fn metadata(&self, path: &Path) -> io::Result<Metadata> {
        let norm = normalize(path);
        if let Ok(bytes) = self.read(path) {
            return Ok(Metadata::new_file(
                bytes.len() as u64,
                SystemTime::UNIX_EPOCH,
            ));
        }
        let path_str = path.to_string_lossy();
        if path_str == "examples"
            || path_str == ".karakuri"
            || path_str == "."
            || path_str.is_empty()
            || self
                .files
                .read()
                .map(|m| m.keys().any(|k| k.starts_with(&norm)))
                .unwrap_or(false)
        {
            return Ok(Metadata::new_dir(SystemTime::UNIX_EPOCH));
        }
        Err(Error::new(
            ErrorKind::NotFound,
            format!("file not found: {}", path.display()),
        ))
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        Ok(normalize(path))
    }

    fn read_dir(&self, path: &Path) -> io::Result<ReadDir> {
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
                        is_dir: false,
                        len: content.len() as u64,
                        modified: SystemTime::UNIX_EPOCH,
                    },
                );
            }
        }

        if let Ok(mem) = self.files.read() {
            for (k, v) in mem.iter() {
                if let Some(parent) = k.parent() {
                    if parent == norm
                        || (norm.as_os_str().is_empty() && parent.as_os_str().is_empty())
                    {
                        if let Some(file_name) = k.file_name() {
                            let name = file_name.to_string_lossy().to_string();
                            map.insert(
                                name,
                                DirEntry {
                                    path: k.clone(),
                                    file_name: file_name.to_os_string(),
                                    is_dir: false,
                                    len: v.len() as u64,
                                    modified: SystemTime::UNIX_EPOCH,
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

    fn open_file(&self, path: &Path, options: &OpenOptions) -> io::Result<File> {
        let norm = normalize(path);
        let mut initial_data = Vec::new();
        if !options.truncate {
            if let Ok(data) = self.read(&norm) {
                initial_data = data;
            }
        }
        let cursor = if options.append {
            initial_data.len()
        } else {
            0
        };
        let buffer = Arc::new(Mutex::new(initial_data));

        let files_clone = Arc::clone(&self.files);
        let provider_clone = self.provider.read().ok().and_then(|p| p.clone());
        let on_flush = Arc::new(move |flush_path: &Path, bytes: &[u8]| {
            let norm = normalize(flush_path);
            if let Ok(mut mem) = files_clone.write() {
                mem.insert(norm.clone(), bytes.to_vec());
            }
            if let Some(ref provider) = provider_clone {
                provider.persist_write(&norm, bytes);
            }
            Ok(())
        });

        Ok(File {
            inner: FileInner::Memory(MemoryFileHandle {
                path: norm,
                cursor,
                on_flush: Some(on_flush),
                buffer,
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestProvider {
        writes: AtomicUsize,
        removes: AtomicUsize,
    }

    impl StorageProvider for TestProvider {
        fn persist_write(&self, _path: &Path, _bytes: &[u8]) {
            self.writes.fetch_add(1, Ordering::SeqCst);
        }
        fn persist_remove(&self, _path: &Path) {
            self.removes.fetch_add(1, Ordering::SeqCst);
        }
        fn persist_rename(&self, _from: &Path, _to: &Path) {
            self.removes.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn memory_fs_crud_and_provider_persistence() {
        let provider = Arc::new(TestProvider {
            writes: AtomicUsize::new(0),
            removes: AtomicUsize::new(0),
        });
        let fs = MemoryFs::with_provider(provider.clone());

        let test_path = Path::new(".karakuri/sets/test.kbset");
        assert!(!fs.exists(test_path));

        // Write
        fs.write(test_path, b"hello karakuri")
            .expect("write succeeds");
        assert!(fs.exists(test_path));
        assert_eq!(fs.read_to_string(test_path).unwrap(), "hello karakuri");
        assert_eq!(provider.writes.load(Ordering::SeqCst), 1);

        // Copy
        let dest_path = Path::new(".karakuri/sets/copy.kbset");
        fs.copy(test_path, dest_path).expect("copy succeeds");
        assert_eq!(fs.read_to_string(dest_path).unwrap(), "hello karakuri");
        assert_eq!(provider.writes.load(Ordering::SeqCst), 2);

        // Rename
        let moved_path = Path::new(".karakuri/sets/moved.kbset");
        fs.rename(dest_path, moved_path).expect("rename succeeds");
        assert!(!fs.exists(dest_path));
        assert_eq!(fs.read_to_string(moved_path).unwrap(), "hello karakuri");

        // ReadDir
        let dir = Path::new(".karakuri/sets");
        let entries: Vec<_> = fs
            .read_dir(dir)
            .expect("read_dir")
            .map(|r| r.unwrap().file_name())
            .collect();
        assert!(entries.contains(&OsString::from("test.kbset")));
        assert!(entries.contains(&OsString::from("moved.kbset")));

        // Remove
        fs.remove_file(test_path).expect("remove succeeds");
        assert!(!fs.exists(test_path));
        assert_eq!(provider.removes.load(Ordering::SeqCst), 2); // 1 from rename + 1 from remove_file
    }

    #[test]
    fn memory_fs_hydration() {
        let fs = MemoryFs::new();
        let seeded = vec![
            (
                PathBuf::from(".karakuri/policies.json"),
                b"[\"Auto\"]".to_vec(),
            ),
            (PathBuf::from("maps/test.map"), b"cc 1 -> gain 0".to_vec()),
        ];
        fs.hydrate(seeded);

        assert_eq!(
            fs.read_to_string(Path::new(".karakuri/policies.json"))
                .unwrap(),
            "[\"Auto\"]"
        );
        assert_eq!(
            fs.read_to_string(Path::new("maps/test.map")).unwrap(),
            "cc 1 -> gain 0"
        );
    }
}
