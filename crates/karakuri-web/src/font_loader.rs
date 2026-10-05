//! Asynchronous CJK font loader for WebAssembly environments.
//!
//! Loads Japanese / CJK fallback fonts from the browser Cache API or CDN mirrors,
//! validating the binary payload format before caching and injecting into the
//! console font definitions.

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

const CACHE_NAME: &str = "karakuri-cjk-font-cache-v2";
const LEGACY_CACHE_NAME: &str = "karakuri-cjk-font-cache-v1";

/// Candidate endpoints for Japanese / CJK fonts, ordered by preference.
const FONT_URL_CANDIDATES: &[&str] = &[
    "https://raw.githubusercontent.com/google/fonts/main/ofl/notosansjp/NotoSansJP%5Bwght%5D.ttf",
    "https://cdn.jsdelivr.net/gh/google/fonts@main/ofl/notosansjp/NotoSansJP%5Bwght%5D.ttf",
    "https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/OTF/Japanese/NotoSansCJKjp-Regular.otf",
    "https://cdn.jsdelivr.net/gh/notofonts/noto-cjk@main/Sans/OTF/Japanese/NotoSansCJKjp-Regular.otf",
];

/// Fetches a CJK font binary asynchronously using the browser's Cache and Fetch APIs.
pub async fn fetch_cjk_font() -> Option<Vec<u8>> {
    let window = web_sys::window()?;

    // Purge legacy cache that may contain corrupt or SPA 404 HTML fallback responses
    if let Ok(caches) = window.caches() {
        let _ = JsFuture::from(caches.delete(LEGACY_CACHE_NAME)).await;
    }

    // 1. Check browser CacheStorage first for any previously cached valid font candidate
    for url in FONT_URL_CANDIDATES {
        if let Some(bytes) = try_load_from_cache(&window, CACHE_NAME, url).await {
            log::info!(
                "Karakuri Web: Loaded valid CJK font from CacheStorage ({} bytes) for: {url}",
                bytes.len()
            );
            return Some(bytes);
        }
    }

    // 2. Fetch from candidate URLs via network
    for url in FONT_URL_CANDIDATES {
        log::info!("Karakuri Web: Attempting to fetch CJK font from: {url}");
        match fetch_and_cache(&window, CACHE_NAME, url).await {
            Ok(bytes) => {
                log::info!(
                    "Karakuri Web: Successfully fetched and cached CJK font ({} bytes) from: {url}",
                    bytes.len()
                );
                return Some(bytes);
            }
            Err(err) => {
                log::warn!("Karakuri Web: Failed to fetch font from {url}: {err:?}");
            }
        }
    }

    log::error!("Karakuri Web: All CJK font fetch candidates failed");
    None
}

async fn try_load_from_cache(
    window: &web_sys::Window,
    cache_name: &str,
    url: &str,
) -> Option<Vec<u8>> {
    let caches = window.caches().ok()?;
    let cache_val = JsFuture::from(caches.open(cache_name)).await.ok()?;
    let cache: web_sys::Cache = cache_val.dyn_into().ok()?;
    let resp_val = JsFuture::from(cache.match_with_str(url)).await.ok()?;
    if resp_val.is_undefined() || resp_val.is_null() {
        return None;
    }
    let resp: web_sys::Response = resp_val.dyn_into().ok()?;
    if !resp.ok() {
        let _ = cache.delete_with_str(url);
        return None;
    }
    let buf_val = JsFuture::from(resp.array_buffer().ok()?).await.ok()?;
    let uint8 = js_sys::Uint8Array::new(&buf_val);
    let bytes = uint8.to_vec();
    if !karakuri_console::room::font::is_valid_font_bytes(&bytes) {
        log::warn!(
            "Karakuri Web: Cache for {url} contained invalid font data; purging cache entry"
        );
        let _ = cache.delete_with_str(url);
        return None;
    }
    Some(bytes)
}

async fn fetch_and_cache(
    window: &web_sys::Window,
    cache_name: &str,
    url: &str,
) -> Result<Vec<u8>, wasm_bindgen::JsValue> {
    let resp_value = JsFuture::from(window.fetch_with_str(url)).await?;
    let resp: web_sys::Response = resp_value.dyn_into()?;
    if !resp.ok() {
        return Err(wasm_bindgen::JsValue::from_str(&format!(
            "HTTP status {}",
            resp.status()
        )));
    }

    // Verify Content-Type does not indicate an HTML fallback from a dev server (e.g. Trunk SPA fallback)
    if let Ok(Some(ct)) = resp.headers().get("content-type") {
        if ct.contains("text/html") || ct.contains("text/plain") {
            return Err(wasm_bindgen::JsValue::from_str(&format!(
                "Unexpected Content-Type: {ct} (likely SPA 404 fallback)"
            )));
        }
    }

    let cloned_resp = resp.clone().ok();
    let buf_val = JsFuture::from(resp.array_buffer()?).await?;
    let uint8 = js_sys::Uint8Array::new(&buf_val);
    let bytes = uint8.to_vec();

    if !karakuri_console::room::font::is_valid_font_bytes(&bytes) {
        return Err(wasm_bindgen::JsValue::from_str(
            "Response is not a valid TTF/OTF font file",
        ));
    }

    // Save verified font response to CacheStorage
    if let Some(cloned) = cloned_resp {
        if let Ok(caches) = window.caches() {
            if let Ok(cache_val) = JsFuture::from(caches.open(cache_name)).await {
                if let Ok(cache) = cache_val.dyn_into::<web_sys::Cache>() {
                    let _ = JsFuture::from(cache.put_with_str(url, &cloned)).await;
                }
            }
        }
    }

    Ok(bytes)
}
