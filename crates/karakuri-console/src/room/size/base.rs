//! Base sizing constants transcribed from the stylesheet.

/// `.console`'s `font-size: 11px`.
pub const BASE: f32 = 11.0;

/// `.console`'s `line-height: 1.5`, as a multiplier.
pub const LINE: f32 = 1.5;

/// `.bay-head`'s `font-size: 10px`.
pub const HEAD_SIZE: f32 = 10.0;

/// `.bay-head`'s `letter-spacing: 0.16em` at [`HEAD_SIZE`], in pixels.
pub const HEAD_TRACKING: f32 = HEAD_SIZE * 0.16;

/// `.bay-head`'s `padding: 6px 10px`.
pub const HEAD_PAD_X: f32 = 10.0;
pub const HEAD_PAD_Y: f32 = 6.0;

/// The whole bay head: `6 + 10 * 1.5 + 6`. The arrangement's minima are written
/// from the same sum, which is why a head is 27 in both places or in neither.
pub const HEAD_H: f32 = HEAD_PAD_Y * 2.0 + HEAD_SIZE * LINE;

/// `.bay`'s `border-radius: 11px`.
pub const BAY_RADIUS: f32 = 11.0;

/// `.bay-head`'s `border-bottom: 1px solid var(--c-hair)`: 1px rule under a bay head.
pub const HAIRLINE: f32 = 1.0;

/// `.pill`'s `padding: 0 8px`, around text at [`BASE`].
pub const PILL_PAD_X: f32 = 8.0;

/// A pill's box: [`BASE`] at [`LINE`], which is what an inline span is.
pub const PILL_H: f32 = BASE * LINE;

/// `docs/manual/console.html`'s `gap: 5px`: inline gap between pills in a bay head.
pub const PILL_GAP: f32 = 5.0;

/// `.insp-split`'s `grid-template-columns: 1fr 9px 1fr`: width of `.divider-v` between panes.
pub const PANE_DIVIDER: f32 = 9.0;

/// `.program-body`'s `padding: 9px`: ground inset around Program bay content.
pub const PROGRAM_BODY_PAD: f32 = 9.0;

/// `.previews`'s `gap: 6px`: gap between deck preview cells.
pub const PREVIEW_GAP: f32 = 6.0;

/// Deck preview cell image height, derived from 16:9 aspect ratio at base 112px track width.
pub const PREVIEW_IMAGE_H: f32 = (466.0 - PREVIEW_GAP * 3.0) / 4.0 * 9.0 / 16.0;

/// `.cell`'s `gap: 4px`: between a cell's image and the caption under it, and
/// the only gap inside a cell — `.cell` is a two-child column, so there is one
/// of these and never two.
pub const PREVIEW_CAPTION_GAP: f32 = 4.0;

/// `.caption`'s `height: 13px`: strip under cell image carrying letter, state, and risk badge.
pub const PREVIEW_CAPTION_H: f32 = 13.0;

/// Total height of a deck preview row, combining image, caption gap, and caption height.
pub const PREVIEW_ROW_H: f32 = PREVIEW_IMAGE_H + PREVIEW_CAPTION_GAP + PREVIEW_CAPTION_H;

/// `.preview`'s `border-radius: 7px`, one shade tighter than the bay's
/// [`BAY_RADIUS`] because the cell is inside it.
pub const PREVIEW_RADIUS: f32 = 7.0;

/// `.caption`'s `font-size: 9px`: the deck's letter and the state word, which
/// are the only type a cell carries — and they are under the image rather than
/// on it.
pub const PREVIEW_SIZE: f32 = 9.0;

/// `.caption`'s `gap: 5px`, between the letter and the word beside it.
pub const PREVIEW_CAPTION_GAP_X: f32 = 5.0;

/// `.risk`'s `width: 6px; height: 6px`: circular risk badge dot at caption end (`border-radius: 999px`).
pub const PREVIEW_RISK: f32 = 6.0;

/// `.transport`'s `padding: 9px 12px`. The 9 is the same 9 the arrangement's 48
/// was written from (`lib.rs`: 9 + 30 + 9), so the row's height and the space
/// around what is in it are one derivation or neither.
pub const TRANSPORT_PAD_X: f32 = 12.0;
pub const TRANSPORT_PAD_Y: f32 = 9.0;

/// `.transport`'s `gap: 14px`: between the BPM and its label, the label and the
/// beat grid, and every other pair in the row.
pub const TRANSPORT_GAP: f32 = 14.0;

/// `.octave`'s `gap: 3px`: gap between the two halves of the octave control.
pub const OCTAVE_GAP: f32 = 3.0;

/// `.octave i`'s `font-size: 9px`: `½` or `×2`, at the type size the scrub's
/// arrows and the anchor beside them are drawn at.
pub const OCTAVE_SIZE: f32 = 9.0;

/// `.octave i`'s `padding: 0 4px`, around one half of it — tighter than a
/// [`PILL_PAD_X`] for [`SCRUB_PAD_X`]'s reason, since what is inside is a mark
/// rather than a word.
pub const OCTAVE_PAD_X: f32 = 4.0;

/// Box height for one half of the octave control: `OCTAVE_SIZE * LINE + HAIRLINE * 2.0` (15.5px).
pub const OCTAVE_H: f32 = OCTAVE_SIZE * LINE + HAIRLINE * 2.0;

/// `.bpm`'s `font-size: 20px`, the one large number on the panel.
pub const BPM_SIZE: f32 = 20.0;

/// BPM readout box height: `BPM_SIZE * LINE` (30px), matching the transport row layout.
pub const BPM_H: f32 = BPM_SIZE * LINE;

/// `.bpm` letter-spacing: `-0.01em` at [`BPM_SIZE`] in pixels.
pub const BPM_TRACKING: f32 = BPM_SIZE * -0.01;

/// `.beat-grid i`'s `width: 15px; height: 6px`. Its `border-radius: 999px` on a
/// box this short is a capsule, drawn as half the height.
pub const BEAT_W: f32 = 15.0;
pub const BEAT_H: f32 = 6.0;

/// `.beat-grid`'s `gap: 4px`, and it is between the dots and nowhere else — the
/// same reading as [`PREVIEW_GAP`]: *n* dots have *n - 1* gaps, not one each.
pub const BEAT_GAP: f32 = 4.0;

/// `.beat-grid i.on`'s `box-shadow: 0 0 9px var(--c-glowp)`: the halo on the
/// lit dot, one pixel wider than the 8px the Outputs row's dot carries and in
/// the pink glow rather than the mint one.
pub const BEAT_GLOW: u8 = 9;

/// `.outputs`'s `padding: 8px 11px`. The 8 is the same 8 the arrangement's 34
/// was written from (`lib.rs`: 8 + 18.5 + 8, rounded), so the row's height and
/// the space around what is in it are one derivation or neither.
pub const OUTPUTS_PAD_X: f32 = 11.0;
pub const OUTPUTS_PAD_Y: f32 = 8.0;

/// `.outputs`'s `gap: 8px`: between the word OUTPUTS and the first sink, and
/// between two sinks when there is a second one.
pub const OUTPUTS_GAP: f32 = 8.0;

/// `.sink`'s `padding: 1px 10px`, around a dot and a name at [`BASE`].
pub const SINK_PAD_X: f32 = 10.0;
pub const SINK_PAD_Y: f32 = 1.0;

/// Sink item height: `BASE * LINE + SINK_PAD_Y * 2.0` (18.5px).
pub const SINK_H: f32 = BASE * LINE + SINK_PAD_Y * 2.0;

/// `.sink`'s own `gap: 6px`, between its dot and its name.
pub const SINK_GAP: f32 = 6.0;

/// `.dot`'s `width: 7px; height: 7px; border-radius: 50%`.
pub const SINK_DOT: f32 = 7.0;

// -- the mixer's strips -------------------------------------------------

/// `.mixer-strips`'s `padding: 6px`: outer card margin around mixer strips.
pub const STRIPS_PAD: f32 = 6.0;

/// `.mixer-strips`'s `gap: 4px`: gap between adjacent mixer strips.
pub const STRIP_GAP: f32 = 4.0;

/// `.strip`'s `border-radius: 9px`, one shade tighter than the bay's
/// [`BAY_RADIUS`] because the strip is inside it — exactly as
/// [`PREVIEW_RADIUS`] is.
pub const STRIP_RADIUS: f32 = 9.0;

/// `.strip.focus`'s `box-shadow: inset 0 0 0 2px var(--c-lav)`: selection ring width.
pub const STRIP_FOCUS_RING: f32 = 2.0;

/// `.strip.drop`'s `outline: 2px solid var(--c-text)`: drop target ring for strips and preview cells.
pub const DROP_RING: f32 = 2.0;

/// `.wfocus`'s `outline: 2px dashed var(--c-sun)`: outline width for the window/bay focus indicator. See ADR-0259.
pub const WFOCUS_RING: f32 = 2.0;

/// `.wfocus`'s `outline-offset: 2px`: proud offset for window/bay focus outline.
pub const WFOCUS_OFFSET: f32 = 2.0;

/// Dash length for window/bay focus outline — the console's own, paired with [`WFOCUS_GAP`].
pub const WFOCUS_DASH: f32 = 4.0;

/// The gap between two dashes of [`WFOCUS_DASH`] — the console's own, for that
/// constant's reason and stated beside it.
pub const WFOCUS_GAP: f32 = 3.0;

/// `.strip`'s `padding: 7px 4px`.
pub const STRIP_PAD_X: f32 = 4.0;
pub const STRIP_PAD_Y: f32 = 7.0;

/// `.strip`'s `gap: 5px`: between the six things stacked in a strip, so there
/// are five of these and not six — the same reading as [`STRIP_GAP`], one axis
/// along.
pub const STRIP_GAP_Y: f32 = 5.0;

/// `.strip-name`'s `font-size: 10px`: what the deck is playing.
pub const STRIP_NAME_SIZE: f32 = 10.0;

/// `.tally`'s `font-size: 9px`.
pub const TALLY_SIZE: f32 = 9.0;

/// `.tally`'s `letter-spacing: 0.1em` at [`TALLY_SIZE`], in pixels — the same
/// treatment [`HEAD_TRACKING`] gives a heading, and applied after the last
/// glyph for the same reason and left alone for it.
pub const TALLY_TRACKING: f32 = TALLY_SIZE * 0.1;

/// `.tally`'s `padding: 0 7px`, around text at [`TALLY_SIZE`].
pub const TALLY_PAD_X: f32 = 7.0;

/// A tally's box: [`TALLY_SIZE`] at [`LINE`] — 13.5, which is the 13.5 in the
/// mixer's own 215.5. Its `border-radius: 999px` on a box this short is a
/// capsule.
pub const TALLY_H: f32 = TALLY_SIZE * LINE;

/// `.tally.live`'s `box-shadow: 0 0 10px var(--c-glowp)`: the halo on a slot
/// that is on air, in the pink glow and one pixel wider than the 9px the
/// transport's lit beat carries.
pub const TALLY_GLOW: u8 = 10;

/// `.trim`'s `gap: 5px`, between the `g` and the mini fader beside it.
pub const TRIM_GAP: f32 = 5.0;

/// `.trim`'s `padding: 0 3px`, inside the strip's own.
pub const TRIM_PAD_X: f32 = 3.0;

/// `.trim .lbl`'s `font-size: 9px`: the `g`, and the only type in the row.
pub const TRIM_LABEL_SIZE: f32 = 9.0;

/// The trim row's box: the label is the tallest thing in it —
/// [`TRIM_LABEL_SIZE`] at [`LINE`] is 13.5 against the track's [`FADER_H`],
/// which is the 13.5 in the mixer's own 215.5.
pub const TRIM_H: f32 = TRIM_LABEL_SIZE * LINE;

/// `.fader`'s `height: 5px`. Its `border-radius: 999px` on a box this short is
/// a capsule, drawn as half the height.
pub const FADER_H: f32 = 5.0;

/// `.fader s`'s `width: 9px; height: 11px`: the trim's knob, taller than the
/// track it rides so that a 5px control has a mark a hand can see.
pub const FADER_KNOB_W: f32 = 9.0;
pub const FADER_KNOB_H: f32 = 11.0;

/// `.fader-col`'s `height: 104px`: fixed vertical fader column height in a mixer strip.
pub const FADER_COL_H: f32 = 104.0;

/// `.fader-col`'s `gap: 6px`, between the vertical fader and the meter.
pub const FADER_COL_GAP: f32 = 6.0;

/// `.vfader`'s `width: 17px`.
pub const VFADER_W: f32 = 17.0;

/// `.vfader b`'s `left: 3px; right: 3px; bottom: 3px`: the fill sits inside the
/// track rather than filling it edge to edge, which is what makes the track
/// read as a well with something in it.
pub const VFADER_INSET: f32 = 3.0;

/// `.vfader s`'s `height: 9px`, and its `left: -2px; right: -2px` — a knob two
/// pixels proud of the track either side, so [`VFADER_KNOB_W`] wide.
pub const VFADER_KNOB_H: f32 = 9.0;
pub const VFADER_KNOB_OUT: f32 = 2.0;
pub const VFADER_KNOB_W: f32 = VFADER_W + VFADER_KNOB_OUT * 2.0;

/// `.strip.live .vfader s`'s `box-shadow: 0 0 0 1px var(--c-pink), 0 0 9px
/// var(--c-glowp)`: the knob of a slot that is on air carries a pink rim and a
/// pink halo, and no other knob does.
pub const VFADER_KNOB_GLOW: u8 = 9;

/// `.vmeter`'s `width: 6px` — a third of the fader beside it, which is how a
/// reading is told from a control at a glance.
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

/// Mini button height: `MINI_SIZE * LINE + HAIRLINE * 2.0` (15.5px).
pub const MINI_H: f32 = MINI_SIZE * LINE + HAIRLINE * 2.0;

/// Total height of a mixer strip (215.5px), summing padding, child rows, and gaps.
pub const STRIP_H: f32 = STRIP_PAD_Y * 2.0
    + STRIP_NAME_SIZE * LINE
    + TALLY_H
    + TRIM_H
    + FADER_COL_H
    + STRIP_NUM_SIZE * LINE
    + MINI_H
    + STRIP_GAP_Y * 5.0;

// -- the mixer's transition row -----------------------------------------

/// `.xfade`'s `padding: 8px 10px 10px`: asymmetric vertical padding around the transition row.
pub const XFADE_PAD_TOP: f32 = 8.0;
pub const XFADE_PAD_X: f32 = 10.0;
pub const XFADE_PAD_BOTTOM: f32 = 10.0;

/// `.xfade`'s `gap: 7px`: gap between rows in transition block, preserved for tests (`tests/mixer.rs`).
pub const XFADE_GAP: f32 = 7.0;

/// `.xrow`'s `gap: 8px`: between two pills of the transition row, and it is
/// between them and nowhere else — [`STRIP_GAP`]'s reading, one row down. What
/// is outside them is [`XFADE_PAD_X`].
pub const XROW_GAP: f32 = 8.0;

/// Transition row pill height: `BASE * LINE + HAIRLINE * 2.0` (18.5px, including 1px border).
pub const XPILL_H: f32 = BASE * LINE + HAIRLINE * 2.0;

/// Transition row total height: `HAIRLINE + XFADE_PAD_TOP + XPILL_H + XFADE_PAD_BOTTOM` (37.5px).
pub const XFADE_H: f32 = HAIRLINE + XFADE_PAD_TOP + XPILL_H + XFADE_PAD_BOTTOM;

// -- the master bay's out row -------------------------------------------

/// `.master-body`'s `padding: 8px 10px 10px`: margins for rows under master bay head.
pub const MASTER_PAD_X: f32 = 10.0;
pub const MASTER_PAD_TOP: f32 = 8.0;

/// `.master-row`'s `gap: 8px`, between the `out` label, the track and the
/// figure. Not [`TRIM_GAP`]'s 5: the mixer's trim is a label against a track
/// inside a 53-wide strip and this row is the width of a bay.
pub const MASTER_GAP: f32 = 8.0;

/// Master out row height: `BASE * LINE` (16.5px).
pub const MASTER_ROW_H: f32 = BASE * LINE;

// -- the master bay's three effect rows ---------------------------------

/// `.master-body`'s `gap: 8px`: vertical gap between rows under master bay head.
pub const MASTER_STACK_GAP: f32 = 8.0;

/// `.fx`'s `padding: 4px 8px`, `gap: 7px` and `border-radius: 8px`: one effect
/// of the master chain, drawn as a well with a dot, a name, a track and a
/// figure in it.
pub const FX_PAD_X: f32 = 8.0;
pub const FX_PAD_Y: f32 = 4.0;
pub const FX_GAP: f32 = 7.0;
pub const FX_RADIUS: u8 = 8;

/// Master effect row height: `FX_PAD_Y * 2.0 + BASE * LINE` (24.5px).
pub const FX_H: f32 = FX_PAD_Y * 2.0 + BASE * LINE;

/// `.fx .dot`'s `width: 6px` — square, and drawn as a circle by its own
/// `border-radius: 50%`. The mark that says this pass is in the frame.
pub const FX_DOT: f32 = 6.0;
