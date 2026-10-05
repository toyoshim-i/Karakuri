//! Verification of font configuration, CJK fallback glyph rendering, and prompt font sizing.

use karakuri_console::egui::{self, Color32, FontFamily, FontId, RawInput};
use karakuri_console::room::font::{configure_fonts, default_font_definitions};
use karakuri_console::room::size;
use karakuri_console::view::prompt::PROMPT_FONT_SIZE;

#[test]
fn prompt_font_size_is_compact_and_smaller_than_base() {
    assert_eq!(PROMPT_FONT_SIZE, 10.0);
    const { assert!(PROMPT_FONT_SIZE < size::BASE) };
}

#[test]
fn default_font_definitions_registers_font_families() {
    let defs = default_font_definitions();
    assert!(defs.families.contains_key(&FontFamily::Monospace));
    assert!(defs.families.contains_key(&FontFamily::Proportional));
}

#[test]
fn configure_fonts_renders_japanese_and_ascii_without_panic() {
    let ctx = egui::Context::default();
    configure_fonts(&ctx);

    let mut output = ctx.run_ui(RawInput::default(), |ctx| {
        let galley = ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                "prompt > 日本語入力テスト 123 abc こんにちは世界".to_string(),
                FontId::new(PROMPT_FONT_SIZE, FontFamily::Monospace),
                Color32::WHITE,
            )
        });
        assert!(galley.size().x > 0.0);
        assert!(galley.size().y > 0.0);
    });
    output.textures_delta.clear();
}

#[test]
fn font_definitions_with_cjk_registers_cjk_fallback_for_valid_header() {
    use karakuri_console::room::font::font_definitions_with_cjk;

    let mut valid_ttf_header = vec![0u8; 100];
    valid_ttf_header[0..4].copy_from_slice(&[0x00, 0x01, 0x00, 0x00]);
    let defs = font_definitions_with_cjk(valid_ttf_header);
    assert!(defs.font_data.contains_key("cjk_fallback"));
    assert!(defs.families[&FontFamily::Monospace].contains(&"cjk_fallback".to_string()));
    assert!(defs.families[&FontFamily::Proportional].contains(&"cjk_fallback".to_string()));
}

#[test]
fn font_definitions_with_cjk_rejects_malformed_or_html_payload() {
    use karakuri_console::room::font::font_definitions_with_cjk;

    let html_payload = b"<!doctype html><html><body>404 Not Found</body></html>".to_vec();
    let defs = font_definitions_with_cjk(html_payload);
    assert!(!defs.font_data.contains_key("cjk_fallback"));
}

#[test]
fn add_cjk_font_safely_ignores_invalid_bytes_without_panic() {
    use karakuri_console::room::font::add_cjk_font;

    let ctx = egui::Context::default();
    let html_bytes = b"<!doctype html><html><body>404 Not Found</body></html>".to_vec();
    add_cjk_font(&ctx, html_bytes);
    // Should not panic, and egui should be able to run without font parse errors
    let mut output = ctx.run_ui(RawInput::default(), |_ctx| {});
    output.textures_delta.clear();
}
