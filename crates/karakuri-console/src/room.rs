//! The room the panel is in, and the colours and type it wears there.
//!
//! *"The whole panel follows the room, pastel on white by day and lit on black
//! in a dark hall."* — `docs/manual/console.html`. The mock offers three
//! settings (`day`, `night`, and follow the reader); two of them are rooms and
//! the third is a way of choosing between them, so [`Room`] has two variants
//! and choosing is the caller's.
//!
//! # Everything here is read off `docs/manual/style.css`, once
//!
//! The stylesheet states each room twice — once under
//! `@media (prefers-color-scheme: …)` for the page, and once as `.console.day`
//! and `.console.night` for the panel itself, which is what the room switch
//! toggles. **The `.console.*` pair is what this transcribes**, because those
//! are the panel's own values; the page's are the surrounding document's and
//! only happen to agree.
//!
//! The custom properties are named `--c-*` there and the fields are named
//! after them without the prefix, so a colour can be checked against the CSS
//! by searching for one word.
//!
//! # What is transcribed and what is interpreted
//!
//! Twelve of the fourteen properties are plain hex and are copied. Two are not:
//!
//! - **`--c-shadow` is two shadows by day and one by night.** `epaint` draws
//!   one, so the day pair is collapsed to its larger member — the `0 4px 14px`
//!   at 7% — and the `0 1px 0` hairline at 4% is dropped. It contributes about
//!   a pixel of edge that the bay's own boundary already gives.
//! - **`--c-glow` and `--c-glowp` are the halo on an armed control**, and no
//!   control is drawn in this pass. They are transcribed anyway, because they
//!   are two lines and the alternative is reading the stylesheet a second time
//!   for them.
//!
//! `color-mix(in srgb, X 14%, transparent)` appears throughout the mock's
//! controls and is exactly a colour at that alpha, so where it is needed it is
//! [`Color32::from_rgba_unmultiplied`] with the same percentage.
//!
//! # The type is `egui`'s default faces at the mock's sizes
//!
//! The mock sets `--f-round: "M PLUS Rounded 1c"` and
//! `--f-code: "M PLUS 1 Code"`, and neither is chased here: shipping two
//! webfonts is a decision of its own and the sizes and weights are what a
//! layout is checked against. So the sizes below are the stylesheet's and the
//! faces are whatever `egui` loaded. **Weight is the one that cannot be
//! honoured at all** — `egui`'s default proportional face has no bold — so a
//! `font-weight: 700` in the CSS is a colour and a size here and nothing else.

use egui::epaint::Shadow;
use egui::Color32;

/// Which room the panel is in.
///
/// Two, because a room is either lit or it is not. The mock's third button —
/// *follow you* — chooses between these from the reader's system theme rather
/// than being a third set of colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Room {
    #[default]
    Day,
    Night,
}

impl Room {
    /// The other one. What the room key does.
    pub fn other(self) -> Room {
        match self {
            Room::Day => Room::Night,
            Room::Night => Room::Day,
        }
    }

    /// The word the manual uses for it, which is the word the readout prints.
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

/// The console's colours in one room: `style.css`'s `--c-*`, field for
/// property.
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

/// `Color32::from_rgba_unmultiplied` is not `const`, and the four values here
/// are compile-time constants, so the premultiplication is written out. Every
/// caller of this passes a colour the CSS states as `rgba(...)`.
const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color32 {
    Color32::from_rgba_premultiplied(
        ((r as u32 * a as u32) / 255) as u8,
        ((g as u32 * a as u32) / 255) as u8,
        ((b as u32 * a as u32) / 255) as u8,
        a,
    )
}

/// The type sizes and the boxes they sit in, in the same logical pixels the
/// arrangement is stated in.
///
/// A row's height is the rule `karakuri_console`'s module documentation gives
/// for the arrangement's own numbers — padding, plus the font size times
/// `.console`'s `line-height: 1.5` — so the two are derived the same way and a
/// bay head is 27 in both places or in neither.
pub mod size {
    /// `.console`'s `font-size: 11px`.
    pub const BASE: f32 = 11.0;

    /// `.console`'s `line-height: 1.5`, as a multiplier.
    pub const LINE: f32 = 1.5;

    /// `.bay-head`'s `font-size: 10px`.
    pub const HEAD_SIZE: f32 = 10.0;

    /// `.bay-head`'s `letter-spacing: 0.16em` at [`HEAD_SIZE`], in pixels. `egui`
    /// spaces after every glyph including the last, which the CSS also does,
    /// so the two agree and a right-aligned head is a pixel and a half wider
    /// than its glyphs. Left alone: it is the trailing space of an
    /// upper-cased, letter-spaced heading and it reads as intended.
    pub const HEAD_TRACKING: f32 = HEAD_SIZE * 0.16;

    /// `.bay-head`'s `padding: 6px 10px`.
    pub const HEAD_PAD_X: f32 = 10.0;
    pub const HEAD_PAD_Y: f32 = 6.0;

    /// The whole bay head: `6 + 10 * 1.5 + 6`. The arrangement's minima are
    /// written from the same sum, which is why a head is 27 in both places or
    /// in neither.
    pub const HEAD_H: f32 = HEAD_PAD_Y * 2.0 + HEAD_SIZE * LINE;

    /// `.bay`'s `border-radius: 11px`.
    pub const BAY_RADIUS: f32 = 11.0;

    /// `border-bottom: 1px solid var(--c-hair)` under a bay head, and every
    /// other rule in the mock.
    pub const HAIRLINE: f32 = 1.0;

    /// `.pill`'s `padding: 0 8px`, around text at [`BASE`].
    pub const PILL_PAD_X: f32 = 8.0;

    /// A pill's box: [`BASE`] at [`LINE`], which is what an inline span is.
    pub const PILL_H: f32 = BASE * LINE;

    /// The gap between the pills in a bay head: `.bay-head`'s inner
    /// `gap: 5px`, as the program's and the inspector's heads set it.
    pub const PILL_GAP: f32 = 5.0;

    /// `.divider-v`, the bar between the inspector's panes:
    /// `.insp-split`'s middle track.
    pub const PANE_DIVIDER: f32 = 9.0;

    /// `.program-body`'s `padding: 9px`: the ring of ground the Program bay
    /// leaves between its head, its edges and the picture inside it. The
    /// arrangement's 378 is written from the same 9 — see the Program bay's
    /// own derivation in `lib.rs` — so the padding a rectangle is inset by
    /// here and the padding the bay's height was derived from are one number
    /// or neither.
    pub const PROGRAM_BODY_PAD: f32 = 9.0;

    /// `.previews`'s `gap: 6px`: between two deck preview cells, and the only
    /// gap in that row — the CSS grid is `repeat(4, 1fr)` with this between
    /// the tracks and nothing outside them, so the padding either end is
    /// [`PROGRAM_BODY_PAD`] and not this. The Program bay's 378 is written
    /// from the same 6 (`(466 - three 6px gaps) / 4 = 112`), so a cell is 112
    /// wide in both places or in neither.
    pub const PREVIEW_GAP: f32 = 6.0;

    /// `.preview`'s `border-radius: 7px`, one shade tighter than the bay's
    /// [`BAY_RADIUS`] because the cell is inside it.
    pub const PREVIEW_RADIUS: f32 = 7.0;

    /// `.preview`'s `font-size: 9px`: the deck's letter, which is the only
    /// type in the cell.
    pub const PREVIEW_SIZE: f32 = 9.0;

    /// `.preview`'s `padding: 3px 5px`, the box the letter sits in at the
    /// bottom-left corner of a cell (`align-items: flex-end`).
    pub const PREVIEW_PAD_X: f32 = 5.0;
    pub const PREVIEW_PAD_Y: f32 = 3.0;
}
