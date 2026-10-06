//! Web IndexedDB persistence provider for virtual filesystem.

#[cfg(target_arch = "wasm32")]
use std::path::{Path, PathBuf};
#[cfg(target_arch = "wasm32")]
use std::sync::Arc;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;

#[cfg(target_arch = "wasm32")]
use super::memory::MemoryFs;
#[cfg(target_arch = "wasm32")]
use super::trait_def::StorageProvider;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(inline_js = r#"
const DB_NAME = "karakuri_vfs";
const STORE_NAME = "files";
let dbPromise = null;

function getDb() {
    if (!dbPromise) {
        dbPromise = new Promise((resolve, reject) => {
            const req = indexedDB.open(DB_NAME, 1);
            req.onupgradeneeded = (e) => {
                const db = e.target.result;
                if (!db.objectStoreNames.contains(STORE_NAME)) {
                    db.createObjectStore(STORE_NAME);
                }
            };
            req.onsuccess = (e) => resolve(e.target.result);
            req.onerror = (e) => reject(e.target.error);
        });
    }
    return dbPromise;
}

export async function idb_load_all() {
    const db = await getDb();
    return new Promise((resolve, reject) => {
        const tx = db.transaction(STORE_NAME, "readonly");
        const store = tx.objectStore(STORE_NAME);
        const req = store.openCursor();
        const results = [];
        req.onsuccess = (e) => {
            const cursor = e.target.result;
            if (cursor) {
                results.push({ path: cursor.key, data: cursor.value });
                cursor.continue();
            } else {
                resolve(results);
            }
        };
        req.onerror = (e) => reject(e.target.error);
    });
}

export async function idb_save(path, data) {
    const db = await getDb();
    return new Promise((resolve, reject) => {
        const tx = db.transaction(STORE_NAME, "readwrite");
        const store = tx.objectStore(STORE_NAME);
        const req = store.put(data, path);
        req.onsuccess = () => resolve();
        req.onerror = (e) => reject(e.target.error);
    });
}

export async function idb_delete(path) {
    const db = await getDb();
    return new Promise((resolve, reject) => {
        const tx = db.transaction(STORE_NAME, "readwrite");
        const store = tx.objectStore(STORE_NAME);
        const req = store.delete(path);
        req.onsuccess = () => resolve();
        req.onerror = (e) => reject(e.target.error);
    });
}
"#)]
extern "C" {
    #[wasm_bindgen(catch)]
    fn idb_load_all() -> Result<js_sys::Promise, JsValue>;

    #[wasm_bindgen(catch)]
    fn idb_save(path: &str, data: js_sys::Uint8Array) -> Result<js_sys::Promise, JsValue>;

    #[wasm_bindgen(catch)]
    fn idb_delete(path: &str) -> Result<js_sys::Promise, JsValue>;
}

/// IndexedDB persistence provider for WebAssembly runtime.
#[cfg(target_arch = "wasm32")]
pub struct IndexedDbStorageProvider;

#[cfg(target_arch = "wasm32")]
impl StorageProvider for IndexedDbStorageProvider {
    fn persist_write(&self, path: &Path, bytes: &[u8]) {
        let path_str = path.to_string_lossy().to_string();
        let uint8 = js_sys::Uint8Array::from(bytes);
        if let Ok(promise) = idb_save(&path_str, uint8) {
            wasm_bindgen_futures::spawn_local(async move {
                let _ = JsFuture::from(promise).await;
            });
        }
    }

    fn persist_remove(&self, path: &Path) {
        let path_str = path.to_string_lossy().to_string();
        if let Ok(promise) = idb_delete(&path_str) {
            wasm_bindgen_futures::spawn_local(async move {
                let _ = JsFuture::from(promise).await;
            });
        }
    }

    fn persist_rename(&self, from: &Path, _to: &Path) {
        let from_str = from.to_string_lossy().to_string();
        if let Ok(promise) = idb_delete(&from_str) {
            wasm_bindgen_futures::spawn_local(async move {
                let _ = JsFuture::from(promise).await;
            });
        }
    }
}

/// Hydrates `MemoryFs` from IndexedDB and installs `IndexedDbStorageProvider`.
#[cfg(target_arch = "wasm32")]
pub async fn hydrate_web_fs(fs: &Arc<MemoryFs>) {
    fs.set_provider(Arc::new(IndexedDbStorageProvider));

    let Ok(promise) = idb_load_all() else {
        return;
    };
    let Ok(val) = JsFuture::from(promise).await else {
        return;
    };
    let entries = js_sys::Array::from(&val);
    let mut hydrated = Vec::new();
    for item in entries.iter() {
        if let Some(obj) = js_sys::Object::try_from(&item) {
            let path_val = js_sys::Reflect::get(obj, &JsValue::from_str("path"));
            let data_val = js_sys::Reflect::get(obj, &JsValue::from_str("data"));
            if let (Ok(p), Ok(d)) = (path_val, data_val) {
                if let Some(path_str) = p.as_string() {
                    if let Ok(uint8) = d.dyn_into::<js_sys::Uint8Array>() {
                        hydrated.push((PathBuf::from(path_str), uint8.to_vec()));
                    }
                }
            }
        }
    }
    let count = hydrated.len();
    fs.hydrate(hydrated);
    log::info!("Karakuri VFS: Hydrated {count} persistent files from IndexedDB");
}
