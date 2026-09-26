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
