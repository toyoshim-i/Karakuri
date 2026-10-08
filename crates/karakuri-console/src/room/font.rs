//! Font configuration and system CJK fallback discovery.

use std::sync::Arc;

/// Candidate system font paths to probe for Japanese / CJK fallback support.
#[cfg(not(target_arch = "wasm32"))]
const CJK_FONT_CANDIDATES: &[&str] = &[
    // macOS
    "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc",
    "/System/Library/Fonts/Hiragino Sans GB.ttc",
    "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
    "/System/Library/Fonts/AppleSDGothicNeo.ttc",
    "/Library/Fonts/Arial Unicode.ttf",
    // Linux
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
    "/usr/share/fonts/truetype/fonts-japanese-gothic.ttf",
    "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/opentype/ipafont-gothic/ipag.ttf",
    "/usr/share/fonts/truetype/vlgothic/VL-Gothic-Regular.ttf",
    // Windows
    "C:\\Windows\\Fonts\\msgothic.ttc",
    "C:\\Windows\\Fonts\\meiryo.ttc",
    "C:\\Windows\\Fonts\\YuGothM.ttc",
];

/// Validates whether the given byte slice represents a recognizable TTF, OTF, or TTC font file.
pub fn is_valid_font_bytes(bytes: &[u8]) -> bool {
    if bytes.len() < 12 {
        return false;
    }
    matches!(
        &bytes[0..4],
        &[0x00, 0x01, 0x00, 0x00] | b"OTTO" | b"ttcf" | b"true" | b"typ1"
    )
}

/// Builds font definitions augmented with the given CJK font byte slice.
/// Returns default font definitions if the byte slice is not a valid font.
pub fn font_definitions_with_cjk(bytes: Vec<u8>) -> egui::FontDefinitions {
    if !is_valid_font_bytes(&bytes) {
        return egui::FontDefinitions::default();
    }
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "cjk_fallback".to_string(),
        Arc::new(egui::FontData::from_owned(bytes)),
    );
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("cjk_fallback".to_string());
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .push("cjk_fallback".to_string());
    fonts
}

/// Builds default font definitions augmented with system Japanese / CJK fallback fonts.
pub fn default_font_definitions() -> egui::FontDefinitions {
    if let Some(bytes) = load_system_cjk_font() {
        font_definitions_with_cjk(bytes)
    } else {
        egui::FontDefinitions::default()
    }
}

/// Applies default font definitions with CJK fallback to the given context.
pub fn configure_fonts(ctx: &egui::Context) {
    ctx.set_fonts(default_font_definitions());
}

/// Applies font definitions with the given CJK font bytes to the given context.
/// If the byte slice is not a valid font file, the call is safely ignored without panicking.
pub fn add_cjk_font(ctx: &egui::Context, bytes: Vec<u8>) {
    if !is_valid_font_bytes(&bytes) {
        return;
    }
    ctx.set_fonts(font_definitions_with_cjk(bytes));
}

fn load_system_cjk_font() -> Option<Vec<u8>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(override_path) = std::env::var("KARAKURI_FONT_PATH") {
            if let Ok(bytes) = std::fs::read(&override_path) {
                return Some(bytes);
            }
        }
        for path in CJK_FONT_CANDIDATES {
            if let Ok(bytes) = std::fs::read(path) {
                return Some(bytes);
            }
        }
    }
    None
}
