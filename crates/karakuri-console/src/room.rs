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

    /// One pixel wherever the mock draws a rule, and the rule the console
    /// actually draws is the one under a bay head:
    /// `.bay-head`'s `border-bottom: 1px solid var(--c-hair)`.
    ///
    /// **The other rules in the mock are all one pixel too** — `.scopes`,
    /// `.path`, `.node-group` and the rest — so the citation is one of many
    /// and not an arbitrary pick: it is the one this constant is used for.
    /// Cited rather than called the console's own, because a number nothing
    /// checks is the thing the guard beside this module exists to end, and
    /// this was the last of them.
    pub const HAIRLINE: f32 = 1.0;

    /// `.pill`'s `padding: 0 8px`, around text at [`BASE`].
    pub const PILL_PAD_X: f32 = 8.0;

    /// A pill's box: [`BASE`] at [`LINE`], which is what an inline span is.
    pub const PILL_H: f32 = BASE * LINE;

    /// The gap between the pills in a bay head, and the one number here the
    /// stylesheet does not carry: `.bay-head`'s own `gap` is `0.8rem`, and
    /// the two heads that hold pills — the program's and the inspector's —
    /// set the pills' gap on the span that holds them. So this is
    /// `docs/manual/console.html`'s `gap: 5px`, inline on that span.
    pub const PILL_GAP: f32 = 5.0;

    /// `.divider-v`, the bar between the inspector's panes. The stylesheet
    /// gives it no width of its own, so its width is the middle track of
    /// `.insp-split`'s `grid-template-columns: 1fr 9px 1fr`.
    ///
    /// **And it is the divider the arrangement leaves for that bar**, which
    /// `lib.rs` declared separately as `INSPECTOR_DIVIDER` until the two were
    /// found to be the same 9 read off the same track. It is the one divider
    /// in the console that is not 10, because the mock draws this one as
    /// something a hand takes hold of rather than leaving it as ground —
    /// `.divider-v`'s `cursor: col-resize`. So the bar `view.rs` paints and
    /// the gap `arrangement()` opens for it are one number or neither.
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

    /// **How tall the row of deck preview cells is**, which is the 63 in the
    /// arrangement's 72 for `deck-previews` — the row of cells, and
    /// [`PROGRAM_BODY_PAD`] under them.
    ///
    /// **Derived rather than transcribed, because the mock states no height
    /// for the row at all.** `.previews` is
    /// `grid-template-columns: repeat(4, 1fr)` with [`PREVIEW_GAP`] between
    /// the tracks, and a cell carries `.preview`'s `aspect-ratio: 16/9`, so
    /// the row's height falls out of the width it has at the narrowest console
    /// the mock will draw. That width is 466 — `.console`'s
    /// `min-width: 1010px` less its own `padding: 10px` either side, less
    /// `.body-grid`'s two fixed tracks and the two gaps between the three,
    /// less `.program-body`'s padding either side — and `lib.rs` writes that
    /// derivation out where the Program bay's 378 is built from it. A track is
    /// (466 - three gaps) / 4 = 112, and 112 at that aspect is **63**.
    pub const PREVIEW_ROW_H: f32 = (466.0 - PREVIEW_GAP * 3.0) / 4.0 * 9.0 / 16.0;

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

    /// `.transport`'s `padding: 9px 12px`. The 9 is the same 9 the
    /// arrangement's 48 was written from (`lib.rs`: 9 + 30 + 9), so the row's
    /// height and the space around what is in it are one derivation or
    /// neither.
    pub const TRANSPORT_PAD_X: f32 = 12.0;
    pub const TRANSPORT_PAD_Y: f32 = 9.0;

    /// `.transport`'s `gap: 14px`: between the BPM and its label, the label
    /// and the beat grid, and every other pair in the row.
    pub const TRANSPORT_GAP: f32 = 14.0;

    /// `.bpm`'s `font-size: 20px`, the one large number on the panel.
    pub const BPM_SIZE: f32 = 20.0;

    /// The BPM's box: [`BPM_SIZE`] at [`LINE`] — **30**, which is the 30 in
    /// the arrangement's `9 + 30 + 9` for the transport row. The row is 48
    /// because this number is this tall, so the two are read off each other
    /// rather than measured twice.
    pub const BPM_H: f32 = BPM_SIZE * LINE;

    /// `.bpm`'s `letter-spacing: -0.01em` at [`BPM_SIZE`], in pixels.
    /// **Negative**: the mock tightens this one number rather than spacing it,
    /// which is the opposite of what [`HEAD_TRACKING`] does to a heading. Like
    /// that one it applies after the last glyph as well, so the box is a fifth
    /// of a pixel narrower than the glyphs need; left alone for the same
    /// reason.
    pub const BPM_TRACKING: f32 = BPM_SIZE * -0.01;

    /// `.beat-grid i`'s `width: 15px; height: 6px`. Its
    /// `border-radius: 999px` on a box this short is a capsule, drawn as half
    /// the height.
    pub const BEAT_W: f32 = 15.0;
    pub const BEAT_H: f32 = 6.0;

    /// `.beat-grid`'s `gap: 4px`, and it is **between** the dots and nowhere
    /// else — the same reading as [`PREVIEW_GAP`]: *n* dots have *n - 1* gaps,
    /// not one each.
    pub const BEAT_GAP: f32 = 4.0;

    /// `.beat-grid i.on`'s `box-shadow: 0 0 9px var(--c-glowp)`: the halo on
    /// the lit dot, one pixel wider than the 8px the Outputs row's dot carries
    /// and in the pink glow rather than the mint one.
    pub const BEAT_GLOW: u8 = 9;

    /// `.outputs`'s `padding: 8px 11px`. The 8 is the same 8 the arrangement's
    /// 34 was written from (`lib.rs`: 8 + 18.5 + 8, rounded), so the row's
    /// height and the space around what is in it are one derivation or
    /// neither.
    pub const OUTPUTS_PAD_X: f32 = 11.0;
    pub const OUTPUTS_PAD_Y: f32 = 8.0;

    /// `.outputs`'s `gap: 8px`: between the word OUTPUTS and the first sink,
    /// and between two sinks when there is a second one.
    pub const OUTPUTS_GAP: f32 = 8.0;

    /// `.sink`'s `padding: 1px 10px`, around a dot and a name at [`BASE`].
    pub const SINK_PAD_X: f32 = 10.0;
    pub const SINK_PAD_Y: f32 = 1.0;

    /// A sink's box: [`BASE`] at [`LINE`] inside that padding — **18.5**,
    /// which is the 18.5 in the arrangement's `8 + 18.5 + 8` for the outputs
    /// row. The row is 34 because a sink is this tall, so the two are read
    /// off each other rather than measured twice.
    pub const SINK_H: f32 = BASE * LINE + SINK_PAD_Y * 2.0;

    /// `.sink`'s own `gap: 6px`, between its dot and its name.
    pub const SINK_GAP: f32 = 6.0;

    /// `.dot`'s `width: 7px; height: 7px; border-radius: 50%`.
    pub const SINK_DOT: f32 = 7.0;

    // -- the mixer's strips -------------------------------------------------

    /// `.mixer-strips`'s `padding: 6px`: the ring of card the strips sit
    /// inside, under the bay head. The mixer's 316 is written from the same 6
    /// — see the right pane's derivation in `lib.rs`, where the strips are
    /// `6 + 215.5 + 6` — so the padding a rectangle is inset by here and the
    /// padding the bay's height was derived from are one number or neither.
    pub const STRIPS_PAD: f32 = 6.0;

    /// `.mixer-strips`'s `gap: 4px`: between two strips, and it is **between**
    /// them and nowhere else — the same reading as [`PREVIEW_GAP`], because it
    /// is the same shape of CSS: `repeat(4, 1fr)` with this between the tracks
    /// and nothing outside them, where the outside is [`STRIPS_PAD`]. The
    /// right pane's minimum of 172 is written from the same 4 (four 37-wide
    /// strips and three of these, inside 6 + 6).
    pub const STRIP_GAP: f32 = 4.0;

    /// `.strip`'s `border-radius: 9px`, one shade tighter than the bay's
    /// [`BAY_RADIUS`] because the strip is inside it — exactly as
    /// [`PREVIEW_RADIUS`] is.
    pub const STRIP_RADIUS: f32 = 9.0;

    /// `.strip.focus`'s `box-shadow: inset 0 0 0 2px var(--c-lav)`: the ring
    /// round the selected deck's strip. **Inset**, so it is drawn inside the
    /// strip's own box and takes no width from the gap beside it — which is
    /// what lets a selection move between two strips 4 apart without either of
    /// them appearing to grow. Twice [`HAIRLINE`] and deliberately so: the
    /// mock's other focus is a *dashed* outline at the same 2, and the two
    /// have to be told apart by their line and not by their weight.
    pub const STRIP_FOCUS_RING: f32 = 2.0;

    /// `.strip`'s `padding: 7px 4px`.
    pub const STRIP_PAD_X: f32 = 4.0;
    pub const STRIP_PAD_Y: f32 = 7.0;

    /// `.strip`'s `gap: 5px`: between the six things stacked in a strip, so
    /// there are **five** of these and not six — the same reading as
    /// [`STRIP_GAP`], one axis along.
    pub const STRIP_GAP_Y: f32 = 5.0;

    /// `.strip-name`'s `font-size: 10px`: what the deck is playing.
    pub const STRIP_NAME_SIZE: f32 = 10.0;

    /// `.tally`'s `font-size: 9px`.
    pub const TALLY_SIZE: f32 = 9.0;

    /// `.tally`'s `letter-spacing: 0.1em` at [`TALLY_SIZE`], in pixels — the
    /// same treatment [`HEAD_TRACKING`] gives a heading, and applied after the
    /// last glyph for the same reason and left alone for it.
    pub const TALLY_TRACKING: f32 = TALLY_SIZE * 0.1;

    /// `.tally`'s `padding: 0 7px`, around text at [`TALLY_SIZE`].
    pub const TALLY_PAD_X: f32 = 7.0;

    /// A tally's box: [`TALLY_SIZE`] at [`LINE`] — **13.5**, which is the 13.5
    /// in the mixer's own 215.5. Its `border-radius: 999px` on a box this
    /// short is a capsule.
    pub const TALLY_H: f32 = TALLY_SIZE * LINE;

    /// `.tally.live`'s `box-shadow: 0 0 10px var(--c-glowp)`: the halo on a
    /// slot that is on air, in the pink glow and one pixel wider than the 9px
    /// the transport's lit beat carries.
    pub const TALLY_GLOW: u8 = 10;

    /// `.trim`'s `gap: 5px`, between the `g` and the mini fader beside it.
    pub const TRIM_GAP: f32 = 5.0;

    /// `.trim`'s `padding: 0 3px`, inside the strip's own.
    pub const TRIM_PAD_X: f32 = 3.0;

    /// `.trim .lbl`'s `font-size: 9px`: the `g`, and the only type in the row.
    pub const TRIM_LABEL_SIZE: f32 = 9.0;

    /// The trim row's box: the label is the tallest thing in it —
    /// [`TRIM_LABEL_SIZE`] at [`LINE`] is 13.5 against the track's
    /// [`FADER_H`], which is the 13.5 in the mixer's own 215.5.
    pub const TRIM_H: f32 = TRIM_LABEL_SIZE * LINE;

    /// `.fader`'s `height: 5px`. Its `border-radius: 999px` on a box this
    /// short is a capsule, drawn as half the height.
    pub const FADER_H: f32 = 5.0;

    /// `.fader s`'s `width: 9px; height: 11px`: the trim's knob, taller than
    /// the track it rides so that a 5px control has a mark a hand can see.
    pub const FADER_KNOB_W: f32 = 9.0;
    pub const FADER_KNOB_H: f32 = 11.0;

    /// `.fader-col`'s `height: 104px` — **stated in the CSS rather than
    /// derived from anything in it**, and the one number in a strip that is
    /// not type. It is the 104 in the mixer's own 215.5, which is why the
    /// manual can say four strips never scroll: nothing in the bay gets
    /// smaller.
    pub const FADER_COL_H: f32 = 104.0;

    /// `.fader-col`'s `gap: 6px`, between the vertical fader and the meter.
    pub const FADER_COL_GAP: f32 = 6.0;

    /// `.vfader`'s `width: 17px`.
    pub const VFADER_W: f32 = 17.0;

    /// `.vfader b`'s `left: 3px; right: 3px; bottom: 3px`: the fill sits
    /// inside the track rather than filling it edge to edge, which is what
    /// makes the track read as a well with something in it.
    pub const VFADER_INSET: f32 = 3.0;

    /// `.vfader s`'s `height: 9px`, and its `left: -2px; right: -2px` — a knob
    /// two pixels proud of the track either side, so [`VFADER_KNOB_W`] wide.
    pub const VFADER_KNOB_H: f32 = 9.0;
    pub const VFADER_KNOB_OUT: f32 = 2.0;
    pub const VFADER_KNOB_W: f32 = VFADER_W + VFADER_KNOB_OUT * 2.0;

    /// `.strip.live .vfader s`'s `box-shadow: 0 0 0 1px var(--c-pink),
    /// 0 0 9px var(--c-glowp)`: the knob of a slot that is on air carries a
    /// pink rim and a pink halo, and no other knob does.
    pub const VFADER_KNOB_GLOW: u8 = 9;

    /// `.vmeter`'s `width: 6px` — a third of the fader beside it, which is how
    /// a reading is told from a control at a glance.
    pub const VMETER_W: f32 = 6.0;

    /// `.vmeter u`'s `height: 2px`: the peak mark.
    pub const VMETER_PEAK_H: f32 = 2.0;

    /// `.strip-num`'s `font-size: 10px`: the opacity, as a number.
    pub const STRIP_NUM_SIZE: f32 = 10.0;

    /// `.strip-mode`'s `gap: 3px`, between the blend mini and the mask mini.
    pub const MODE_GAP: f32 = 3.0;

    /// `.mini`'s `font-size: 9px`.
    pub const MINI_SIZE: f32 = 9.0;

    /// `.mini`'s `padding: 0 6px`, around text at [`MINI_SIZE`].
    pub const MINI_PAD_X: f32 = 6.0;

    /// A mini's box: [`MINI_SIZE`] at [`LINE`] inside its
    /// `border: 1px solid var(--c-line)` — **15.5**, which is the 15.5 in the
    /// mixer's own 215.5. The border is counted because the mock's box model
    /// is the browser default (`content-box`) for everything except where a
    /// `box-shadow: inset` draws it, and this one is a real border.
    pub const MINI_H: f32 = MINI_SIZE * LINE + HAIRLINE * 2.0;

    /// **The whole of a strip**, and it is the sum every other constant in
    /// this section feeds: `.strip`'s [`STRIP_PAD_Y`] either side of six
    /// children — the name at 15, the tally at 13.5, the trim at 13.5, the
    /// fader column's 104, the number at 15 and the modes at 15.5 — with five
    /// [`STRIP_GAP_Y`]s between them. **215.5**, which is the 215.5 the
    /// mixer's 316 is written from in `lib.rs`, so a strip is this tall in
    /// both places or in neither.
    pub const STRIP_H: f32 = STRIP_PAD_Y * 2.0
        + STRIP_NAME_SIZE * LINE
        + TALLY_H
        + TRIM_H
        + FADER_COL_H
        + STRIP_NUM_SIZE * LINE
        + MINI_H
        + STRIP_GAP_Y * 5.0;

    // -- the master bay's out row -------------------------------------------

    /// `.master-body`'s `padding: 8px 10px 10px`: the ring of card the bay's
    /// rows sit inside, under the bay head. Two constants because the top and
    /// the sides differ and the bottom is the sides' number again — the one
    /// row this bay draws is measured off the first two, and the third is
    /// under the chain nothing draws yet.
    pub const MASTER_PAD_X: f32 = 10.0;
    pub const MASTER_PAD_TOP: f32 = 8.0;

    /// `.master-row`'s `gap: 8px`, between the `out` label, the track and the
    /// figure. Not [`TRIM_GAP`]'s 5: the mixer's trim is a label against a
    /// track inside a 53-wide strip and this row is the width of a bay.
    pub const MASTER_GAP: f32 = 8.0;

    /// **The out row**, which is one line of type: the tallest thing in it is
    /// the label and the figure, both at [`BASE`], and the track is 5.
    /// `lib.rs`'s minimum for this bay is written from the same 16.5 — *"the
    /// out row 16.5"* — so the row a rectangle is given here and the height
    /// the arrangement reserves are one number or neither.
    pub const MASTER_ROW_H: f32 = BASE * LINE;

    // -- the library's scopes -----------------------------------------------

    /// `.scopes`'s `padding: 7px 9px` and its `gap: 4px`: the row of chips
    /// between the bay head and the list, which says *which library is being
    /// listed* rather than what is in it.
    pub const SCOPES_PAD_X: f32 = 9.0;
    pub const SCOPES_PAD_Y: f32 = 7.0;
    pub const SCOPES_GAP: f32 = 4.0;

    /// `.scope`'s `padding: 0 8px`, around one scope's word at [`BASE`]. The
    /// same padding a [`PILL_PAD_X`] is and a different chip: a `.pill` is a
    /// control in a bay head and this is one of a row of questions under one.
    pub const SCOPE_PAD_X: f32 = 8.0;

    /// One chip's box: [`BASE`] at [`LINE`], with **no border to count** —
    /// `.scope` draws none where the `.rend` chips in the inspector each draw
    /// one, so this is [`PILL_H`]'s number and not [`REND_H`]'s. **16.5**.
    pub const SCOPE_H: f32 = BASE * LINE;

    /// The scope row's box: one chip inside [`SCOPES_PAD_Y`], plus the one
    /// pixel of the rule under it — [`HAIRLINE`], which is `.scopes`'s own
    /// border-bottom and the same pixel every other rule in the mock is drawn
    /// at. **31.5**, which is what `lib.rs`'s minimum for this bay counts now:
    /// that sum said 31 while this row was undrawn — the rule left out and
    /// what was left rounded up — and it is written from this constant's own
    /// terms now, as [`LIB_FOOT_H`] is.
    ///
    /// **One row and not a wrap**, which is [`REND_ROW_H`]'s rule one bay
    /// along: `.scopes` carries a wrapping flex and this console draws the
    /// chips that fit in one row of it.
    pub const SCOPES_H: f32 = SCOPES_PAD_Y * 2.0 + SCOPE_H + HAIRLINE;

    // -- the library's list -------------------------------------------------

    /// `.lib-list`'s `padding: 3px`: the ring of card the rows sit inside,
    /// under the bay head and the scope row. The library's minimum of 158 in
    /// `lib.rs` is written from the same 3 — *"a list of three rows (3 + 3 of
    /// `.lib-list` padding, plus 3 x 22.5)"* — so the padding a rectangle is
    /// inset by here and the minimum the bay declares are one number or
    /// neither.
    pub const LIB_LIST_PAD: f32 = 3.0;

    /// `.lib-row`'s `padding: 3px 7px`, around a Set's name at [`BASE`]. The
    /// rows stack with no gap between them: `.lib-list` is a column flex with
    /// no `gap` at all, which is the one list in the mock that has none.
    pub const LIB_ROW_PAD_X: f32 = 7.0;
    pub const LIB_ROW_PAD_Y: f32 = 3.0;

    /// A row's box: [`BASE`] at [`LINE`] inside that padding — **22.5**, which
    /// is the 22.5 the library's minimum of 158 is written from in `lib.rs`,
    /// so a row is this tall in both places or in neither.
    pub const LIB_ROW_H: f32 = BASE * LINE + LIB_ROW_PAD_Y * 2.0;

    /// `.lib-row`'s `border-radius: 7px`, which nothing in this bay draws
    /// until a row is under the cursor: the wash behind that one row is the
    /// only fill a row ever has, and a square-cornered fill inside a card
    /// rounded at [`BAY_RADIUS`] is the one shape the mock never draws.
    pub const LIB_ROW_RADIUS: f32 = 7.0;

    /// `.lib-foot`'s `padding: 5px 10px`, around the count at
    /// [`LIB_FOOT_SIZE`].
    pub const LIB_FOOT_PAD_X: f32 = 10.0;
    pub const LIB_FOOT_PAD_Y: f32 = 5.0;

    /// `.lib-foot`'s `font-size: 10px`: how many rows are listed of how many
    /// there are, and the only type in the row.
    pub const LIB_FOOT_SIZE: f32 = 10.0;

    /// The foot's box: [`LIB_FOOT_SIZE`] at [`LINE`] inside that padding, plus
    /// the one pixel of the rule above it — [`HAIRLINE`], which is
    /// `.lib-foot`'s own border-top and the same pixel every other rule in the
    /// mock is drawn at. **26**, and the library's minimum of 158 in `lib.rs`
    /// is written from it — as it is from [`SCOPES_H`], and as it was from
    /// neither while this row and that one were undrawn.
    pub const LIB_FOOT_H: f32 = LIB_FOOT_PAD_Y * 2.0 + LIB_FOOT_SIZE * LINE + HAIRLINE;

    // -- the staging lane's candidates --------------------------------------

    /// `.stage-list`'s `padding: 6px 9px 8px`: the ring of card the candidate
    /// rows sit inside, under the bay head. Three numbers and not two — the
    /// lane is the one list in the mock whose bottom padding is not its top —
    /// so the list's box is asymmetric down the column and even across it.
    /// `lib.rs`'s 125 for this bay is written from the same 6 and 8 — *"6 + 8
    /// of `.stage-list` padding"* — so the padding a rectangle is inset by
    /// here and the height the arrangement reserves are one derivation or
    /// neither.
    pub const STAGE_LIST_PAD_TOP: f32 = 6.0;
    pub const STAGE_LIST_PAD_X: f32 = 9.0;
    pub const STAGE_LIST_PAD_BOTTOM: f32 = 8.0;

    /// `.stage-list`'s `gap: 5px`, between one candidate row and the next.
    /// **The one list in this console that has a gap** — `.lib-list` states
    /// none and its rows are flush — and it is the same 5 `lib.rs`'s 125 is
    /// written from (*"two 5px gaps"*).
    pub const STAGE_GAP: f32 = 5.0;

    /// `.cand`'s `padding: 4px 7px`, around the row's own type at [`BASE`].
    pub const CAND_PAD_X: f32 = 7.0;
    pub const CAND_PAD_Y: f32 = 4.0;

    /// A candidate row's box: [`BASE`] at [`LINE`] inside that padding —
    /// **24.5**, which is the 24.5 `lib.rs`'s 125 and its minimum of 66 are
    /// both written from (*"three `.cand` rows at 4 + 16.5 + 4"*), so a row is
    /// this tall in both places or in neither.
    pub const CAND_H: f32 = BASE * LINE + CAND_PAD_Y * 2.0;

    /// `.cand`'s `border-radius: 8px`, one shade tighter than the bay's
    /// [`BAY_RADIUS`] because the row is inside it — [`PREVIEW_RADIUS`]'s
    /// relation to the same bay, one column along.
    pub const CAND_RADIUS: f32 = 8.0;

    /// `.cand`'s `gap: 6px`, between the deck the candidate landed on and what
    /// it is called.
    pub const CAND_GAP: f32 = 6.0;

    /// `.cand .who`'s `font-size: 10px`: the small type at the far end of a
    /// candidate row. The mock puts the producer there and this console puts
    /// the verdict — see [`crate::view::staging`], which is where that
    /// substitution is argued.
    pub const CAND_WHO_SIZE: f32 = 10.0;

    // -- the inspector's panes ----------------------------------------------

    /// `.half-head`'s `padding: 5px 10px` and its `gap: 6px`: the row that
    /// says *which* deck a pane is showing, above the deck head that says what
    /// that deck is doing.
    pub const HALF_HEAD_PAD_X: f32 = 10.0;
    pub const HALF_HEAD_PAD_Y: f32 = 5.0;
    pub const HALF_HEAD_GAP: f32 = 6.0;

    /// The pane head's box: [`BASE`] at [`LINE`] inside that padding, plus the
    /// one pixel of its own `border-bottom: 1px solid var(--c-hair)` —
    /// **27.5**, which is the 27.5 the inspector's minimum of 151.5 is written
    /// from in `lib.rs`, so the row is this tall in both places or in neither.
    pub const HALF_HEAD_H: f32 = HALF_HEAD_PAD_Y * 2.0 + BASE * LINE + HAIRLINE;

    /// `.deck-head`'s `padding: 5px 10px` and its `gap: 6px`: the strip of
    /// chips that heads a *deck* rather than a bay — its clock and its fold.
    pub const DECK_HEAD_PAD_X: f32 = 10.0;
    pub const DECK_HEAD_PAD_Y: f32 = 5.0;
    pub const DECK_HEAD_GAP: f32 = 6.0;

    /// The deck head's box: a [`MINI_H`] chip inside that padding — **25.5**,
    /// which is the stylesheet's own arithmetic for the row (*"the tallest
    /// thing in it is a `.mini` at 15.5, so the row is 5 + 15.5 + 5"*) and the
    /// 25.5 the inspector's minimum of 151.5 is written from in `lib.rs`.
    pub const DECK_HEAD_H: f32 = DECK_HEAD_PAD_Y * 2.0 + MINI_H;

    /// `.anchor`'s `font-size: 9px`: the tempo a deck was engaged at, beside
    /// the chip that says what its clock is locked to.
    pub const ANCHOR_SIZE: f32 = 9.0;

    /// `.scrub`'s `gap: 3px`, between the two arrows a quarter beat a press
    /// goes through.
    pub const SCRUB_GAP: f32 = 3.0;

    /// `.scrub i`'s `font-size: 9px`: one of the two arrows a quarter beat a
    /// press goes through, at the same type size as the anchor beside it.
    pub const SCRUB_SIZE: f32 = 9.0;

    /// `.scrub i`'s `padding: 0 4px`, around one arrow — tighter than a
    /// [`MINI_PAD_X`] because what is inside it is a mark and not a word.
    pub const SCRUB_PAD_X: f32 = 4.0;

    /// One arrow's box: [`SCRUB_SIZE`] at [`LINE`] inside its
    /// `border: 1px solid var(--c-line)` — **15.5**, which is [`MINI_H`]'s own
    /// number at the same type size, and is why the two arrows sit in the deck
    /// head without making [`DECK_HEAD_H`] any taller than the chip beside
    /// them.
    pub const SCRUB_H: f32 = SCRUB_SIZE * LINE + HAIRLINE * 2.0;

    /// `.node-head`'s `padding: 5px 10px` and its `gap: 7px`: a node's
    /// address, its name and who is allowed to move it.
    pub const NODE_HEAD_PAD_X: f32 = 10.0;
    pub const NODE_HEAD_PAD_Y: f32 = 5.0;
    pub const NODE_HEAD_GAP: f32 = 7.0;

    /// A node head's box: [`BASE`] at [`LINE`] inside that padding — **26.5**,
    /// which is the 26.5 the inspector's minimum of 151.5 is written from in
    /// `lib.rs`. The authority chips beside the name are shorter than the name
    /// is ([`AUTH_H`] against 16.5), so the name is what sets the height.
    pub const NODE_HEAD_H: f32 = NODE_HEAD_PAD_Y * 2.0 + BASE * LINE;

    /// `.auth`'s `gap: 3px`, between the three words on a node head.
    pub const AUTH_GAP: f32 = 3.0;

    /// `.auth span`'s `padding: 0 5px` and its `font-size: 9px`: one of
    /// `man`, `sug` and `auto`.
    pub const AUTH_PAD_X: f32 = 5.0;
    pub const AUTH_SIZE: f32 = 9.0;

    /// An authority chip's box: [`AUTH_SIZE`] at [`LINE`] — **13.5**, with no
    /// border to count, which is what makes it shorter than the node name
    /// beside it. Its `border-radius: 999px` on a box this short is a capsule.
    pub const AUTH_H: f32 = AUTH_SIZE * LINE;

    /// `.param`'s `padding: 3px 10px 3px 12px` — **the one row in the mock
    /// whose two side paddings differ**, twelve in from the left of the pane
    /// and ten from the right, which is what indents a parameter under the
    /// node head above it — and its `gap: 8px`, between the four tracks.
    pub const PARAM_PAD_L: f32 = 12.0;
    pub const PARAM_PAD_R: f32 = 10.0;
    pub const PARAM_PAD_Y: f32 = 3.0;
    pub const PARAM_GAP: f32 = 8.0;

    /// `.param`'s `grid-template-columns: 15px 88px 1fr 58px`: the ordinal a
    /// MIDI control is learned against, the name, the fader — which takes
    /// whatever the other three leave — and the value.
    pub const PARAM_ORD_W: f32 = 15.0;
    pub const PARAM_NAME_W: f32 = 88.0;
    pub const PARAM_VAL_W: f32 = 58.0;

    /// `.param .ord`'s `font-size: 9.5px`: the position in the deck's
    /// published interface, and the only type in the row that is not the
    /// pane's own size.
    pub const PARAM_ORD_SIZE: f32 = 9.5;

    /// A parameter row's box: [`BASE`] at [`LINE`] inside that padding —
    /// **22.5**, which is the 22.5 the inspector's minimum of 151.5 is written
    /// from in `lib.rs`, so a row is this tall in both places or in neither.
    /// The fader in the middle of it is [`FADER_H`] and its knob
    /// [`FADER_KNOB_H`], both shorter than the type either side.
    pub const PARAM_H: f32 = PARAM_PAD_Y * 2.0 + BASE * LINE;

    /// `.rend-row`'s `padding: 3px 10px 6px 12px` — the renderer chips stand
    /// on the same 12 and 10 a parameter row does, with more room under them
    /// than over — and its `gap: 5px`, between two chips.
    pub const REND_ROW_PAD_L: f32 = 12.0;
    pub const REND_ROW_PAD_R: f32 = 10.0;
    pub const REND_ROW_PAD_T: f32 = 3.0;
    pub const REND_ROW_PAD_B: f32 = 6.0;
    pub const REND_GAP: f32 = 5.0;

    /// `.rend`'s `padding: 0 7px`, around a renderer's name at [`BASE`].
    pub const REND_PAD_X: f32 = 7.0;

    /// A renderer chip's box: [`BASE`] at [`LINE`] inside its
    /// `border: 1px solid var(--c-line)` — the same box [`MINI_H`] is, at the
    /// pane's own type size rather than a mini's, so **18.5**.
    pub const REND_H: f32 = BASE * LINE + HAIRLINE * 2.0;

    /// The renderer row's box: one chip inside [`REND_ROW_PAD_T`] and
    /// [`REND_ROW_PAD_B`] — **27.5**. One row and not a wrap: `.rend-row`
    /// carries `flex-wrap: wrap` and the console draws the chips that fit,
    /// which is [`crate::view::inspector`]'s rule about a pane that overflows
    /// stated one row along.
    pub const REND_ROW_H: f32 = REND_ROW_PAD_T + REND_H + REND_ROW_PAD_B;
}
