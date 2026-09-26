//! Room themes, color palettes, and typographic scales for console styling.
//!
//! Provides color constants and dimension metrics corresponding to CSS styles.

use egui::epaint::Shadow;
use egui::Color32;

/// Which room (theme) the panel is in: Day or Night.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Room {
    #[default]
    Day,
    Night,
}

impl Room {
    /// Returns the opposite room theme.
    pub fn other(self) -> Room {
        match self {
            Room::Day => Room::Night,
            Room::Night => Room::Day,
        }
    }

    /// Returns the room name string displayed in readouts.
    pub fn word(self) -> &'static str {
        match self {
            Room::Day => "day",
            Room::Night => "night",
        }
    }

    pub fn palette(self) -> Palette {
        match self {
            Room::Day => Palette::DAY,
            Room::Night => Palette::NIGHT,
        }
    }
}

/// Theme preference for the console room styling: Auto (system theme), Day, or Night.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Auto,
    Day,
    Night,
}

impl ThemeMode {
    pub const ALL: [ThemeMode; 3] = [ThemeMode::Auto, ThemeMode::Day, ThemeMode::Night];

    /// Returns the theme mode string displayed in readouts.
    pub fn word(self) -> &'static str {
        match self {
            ThemeMode::Auto => "auto",
            ThemeMode::Day => "day",
            ThemeMode::Night => "night",
        }
    }

    /// Cycles to the next theme mode (Auto -> Day -> Night -> Auto).
    pub fn next(self) -> ThemeMode {
        match self {
            ThemeMode::Auto => ThemeMode::Day,
            ThemeMode::Day => ThemeMode::Night,
            ThemeMode::Night => ThemeMode::Auto,
        }
    }

    /// Resolves the theme preference to a concrete Room theme given an optional system theme.
    pub fn resolve(self, system: Option<egui::Theme>) -> Room {
        match self {
            ThemeMode::Auto => match system {
                Some(egui::Theme::Dark) => Room::Night,
                _ => Room::Day,
            },
            ThemeMode::Day => Room::Day,
            ThemeMode::Night => Room::Night,
        }
    }
}

/// Palette color tokens corresponding to CSS variables (`--c-*`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    /// `--c-ground`: behind everything, and what shows through every divider.
    pub ground: Color32,
    /// `--c-panel`: a bay's card.
    pub panel: Color32,
    /// `--c-well`: a recess inside a bay — a fader track, a strip, a step.
    pub well: Color32,
    /// `--c-line`: a border that is meant to be seen, on a pill or a field.
    pub line: Color32,
    /// `--c-hair`: a rule between rows, one shade quieter than `line`.
    pub hair: Color32,
    /// `--c-text`: a value.
    pub text: Color32,
    /// `--c-dim`: a label beside one.
    pub dim: Color32,
    /// `--c-faint`: a heading, and anything switched off.
    pub faint: Color32,
    /// `--c-mint`: armed, bound, live in the good sense.
    pub mint: Color32,
    /// `--c-pink`: on air.
    pub pink: Color32,
    /// `--c-lav`: an address, a selection, the playhead.
    pub lav: Color32,
    /// `--c-sun`: priming, and a favourite.
    pub sun: Color32,
    /// `--c-tint`: the wash behind a node head, and the *allocated* tally.
    pub tint: Color32,
    /// `--c-glow`: the halo on a mint control.
    pub glow: Color32,
    /// `--c-glowp`: the same on a pink one.
    pub glow_pink: Color32,
    /// `--c-band-green`: risk badge base band (read through [`view::Band`](crate::view::Band)).
    /// Used by [`view::caption_into`](crate::view::caption_into) for scale-based status indication.
    pub band_green: Color32,
    /// `--c-band-blue`.
    pub band_blue: Color32,
    /// `--c-band-yellow`.
    pub band_yellow: Color32,
    /// `--c-band-red`.
    pub band_red: Color32,
    /// `--c-band-purple`.
    pub band_purple: Color32,
    /// `--c-shadow`, collapsed to one — see the module documentation.
    pub shadow: Shadow,
}

impl Palette {
    /// `.console.day` — pastel on a cool white.
    pub const DAY: Palette = Palette {
        ground: rgb(0xe6, 0xef, 0xf7),
        panel: rgb(0xff, 0xff, 0xff),
        well: rgb(0xee, 0xf5, 0xfa),
        line: rgb(0xd5, 0xe3, 0xee),
        hair: rgb(0xe6, 0xef, 0xf5),
        text: rgb(0x2b, 0x30, 0x50),
        dim: rgb(0x65, 0x6e, 0x91),
        faint: rgb(0xa3, 0xad, 0xc7),
        mint: rgb(0x17, 0xb4, 0xab),
        pink: rgb(0xee, 0x5b, 0x9c),
        lav: rgb(0x7d, 0x73, 0xe8),
        sun: rgb(0xed, 0xa3, 0x2f),
        // rgba(43,48,80,0.05)
        tint: rgba(0x2b, 0x30, 0x50, 13),
        // rgba(23,180,171,0.30)
        glow: rgba(0x17, 0xb4, 0xab, 77),
        // rgba(238,91,156,0.30)
        glow_pink: rgba(0xee, 0x5b, 0x9c, 77),
        band_green: rgb(0x2c, 0x9e, 0x63),
        band_blue: rgb(0x3a, 0x7b, 0xd5),
        band_yellow: rgb(0xd3, 0x9a, 0x1c),
        band_red: rgb(0xd2, 0x4b, 0x46),
        band_purple: rgb(0x7d, 0x5b, 0xd6),
        // 0 4px 14px rgba(43,48,80,0.07)
        shadow: Shadow {
            offset: [0, 4],
            blur: 14,
            spread: 0,
            color: rgba(0x2b, 0x30, 0x50, 18),
        },
    };

    /// `.console.night` — the same three colours, lit.
    pub const NIGHT: Palette = Palette {
        ground: rgb(0x0b, 0x09, 0x14),
        panel: rgb(0x17, 0x14, 0x2a),
        well: rgb(0x10, 0x0d, 0x1f),
        line: rgb(0x2b, 0x25, 0x47),
        hair: rgb(0x24, 0x1f, 0x3b),
        text: rgb(0xe7, 0xe3, 0xf7),
        dim: rgb(0xa0, 0x98, 0xc6),
        faint: rgb(0x6a, 0x62, 0x8f),
        mint: rgb(0x5f, 0xef, 0xe4),
        pink: rgb(0xff, 0x8f, 0xc6),
        lav: rgb(0xa7, 0x9b, 0xff),
        sun: rgb(0xff, 0xcf, 0x6b),
        // rgba(231,227,247,0.06)
        tint: rgba(0xe7, 0xe3, 0xf7, 15),
        // rgba(95,239,228,0.42)
        glow: rgba(0x5f, 0xef, 0xe4, 107),
        // rgba(255,143,198,0.42)
        glow_pink: rgba(0xff, 0x8f, 0xc6, 107),
        band_green: rgb(0x4f, 0xd6, 0x8f),
        band_blue: rgb(0x6f, 0xa8, 0xff),
        band_yellow: rgb(0xff, 0xcf, 0x6b),
        band_red: rgb(0xff, 0x7a, 0x72),
        band_purple: rgb(0xb4, 0x92, 0xff),
        // 0 2px 18px rgba(0,0,0,0.5)
        shadow: Shadow {
            offset: [0, 2],
            blur: 18,
            spread: 0,
            color: rgba(0, 0, 0, 128),
        },
    };
}

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// Premultiplies RGB by alpha in const evaluation for CSS `rgba(...)` values.
const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color32 {
    Color32::from_rgba_premultiplied(
        ((r as u32 * a as u32) / 255) as u8,
        ((g as u32 * a as u32) / 255) as u8,
        ((b as u32 * a as u32) / 255) as u8,
        a,
    )
}

/// Font configuration and system CJK fallback discovery.
pub mod font;

pub use font::{configure_fonts, default_font_definitions};

/// Type sizes and box dimensions in logical pixels, derived matching arrangement rules
/// (`padding + font_size * line_height`).
pub mod size;
