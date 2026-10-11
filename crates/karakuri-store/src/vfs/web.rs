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
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = __karakuri, catch)]
    fn idb_load_all() -> Result<js_sys::Promise, JsValue>;

    #[wasm_bindgen(js_namespace = __karakuri, catch)]
    fn idb_save(path: &str, data: js_sys::Uint8Array) -> Result<js_sys::Promise, JsValue>;

    #[wasm_bindgen(js_namespace = __karakuri, catch)]
    fn idb_delete(path: &str) -> Result<js_sys::Promise, JsValue>;

    #[wasm_bindgen(js_namespace = __karakuri, catch)]
    fn idb_rename(
        from: &str,
        to: &str,
        data: js_sys::Uint8Array,
    ) -> Result<js_sys::Promise, JsValue>;
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

    fn persist_rename(&self, from: &Path, to: &Path, bytes: &[u8]) {
        let from_str = from.to_string_lossy().to_string();
        let to_str = to.to_string_lossy().to_string();
        let uint8 = js_sys::Uint8Array::from(bytes);
        if let Ok(promise) = idb_rename(&from_str, &to_str, uint8) {
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
