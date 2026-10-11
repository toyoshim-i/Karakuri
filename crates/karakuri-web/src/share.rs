//! URL Hash permalink sharing and Web Share integration.

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = __karakuri, catch)]
    fn get_url_hash_set() -> Result<js_sys::Promise, JsValue>;

    #[wasm_bindgen(js_namespace = __karakuri, catch)]
    fn share_set_with_video(id: &str, ndjson: &str) -> Result<js_sys::Promise, JsValue>;
}

/// Retrieves and decompresses any `.kbset` NDJSON bundle passed in the URL hash (`#set=...`).
pub async fn load_url_hash_set() -> Result<Option<String>, String> {
    let promise = match get_url_hash_set() {
        Ok(p) => p,
        Err(e) => return Err(format!("failed to call get_url_hash_set: {e:?}")),
    };
    let val = JsFuture::from(promise)
        .await
        .map_err(|e| format!("get_url_hash_set promise rejected: {e:?}"))?;
    if val.is_null() || val.is_undefined() {
        Ok(None)
    } else {
        Ok(val.as_string())
    }
}

/// Triggers video capture, URL hash generation, and social sharing.
pub async fn execute_web_share(id: &str, ndjson: &str) -> Result<(), String> {
    let promise = match share_set_with_video(id, ndjson) {
        Ok(p) => p,
        Err(e) => return Err(format!("failed to call share_set_with_video: {e:?}")),
    };
    JsFuture::from(promise)
        .await
        .map_err(|e| format!("share_set_with_video promise rejected: {e:?}"))?;
    Ok(())
}
