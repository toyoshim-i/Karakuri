//! Layout geometry, hit targets, and bounding boxes for channel strips.

use egui::{Pos2, Rect};

use super::super::*;
use super::paint::reach;
use super::state::*;

/// Layout bounding boxes for a channel strip's controls and indicators.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StripBox {
    /// `.strip` itself: the 9px well everything else is inside.
    pub rect: Rect,
    /// `.strip-name`, the full width of the strip's content box because the CSS
    /// says `width: 100%`.
    pub name: Rect,
    /// `.tally`'s capsule, sized to the widest residency word to prevent resizing.
    pub tally: Rect,
    /// The SOLO toggle button rect on the left of the tally capsule.
    pub solo: Rect,
    /// The MUTE toggle button rect on the right of the tally capsule.
    pub mute: Rect,
    /// The `g` in `.trim`.
    pub trim_label: Rect,
    /// `.trim`'s `.fader`: the horizontal track, [`size::FADER_H`] tall.
    pub trim: Rect,
    /// `.vfader`: the tall track, [`size::FADER_COL_H`] high.
    pub fader: Rect,
    /// `.vmeter`, beside it.
    pub meter: Rect,
    /// `.strip-num`: the opacity as a number.
    pub num: Rect,
    /// The blend `.mini`, which is the chip a press acts on and not only the box a
    /// word is painted into — [`Mixer::blend`](crate::view::mixer::Mixer::blend) hit-tests exactly this rectangle. As
    /// wide as the word in it, inside `.mini`'s padding and border.
    pub blend: Rect,
    /// Bounding rectangle for the mask shape selection chip.
    pub mask: Rect,
}

impl StripBox {
    /// The trim at a gain, which is [`fader`] on the horizontal track.
    pub fn trim_at(&self, gain: f32) -> Fader {
        fader(
            self.trim,
            Axis::Row,
            gain,
            0.0,
            egui::vec2(size::FADER_KNOB_W, size::FADER_KNOB_H),
        )
    }

    /// The fader at an opacity, which is [`fader`] on the vertical track.
    pub fn fader_at(&self, opacity: f32) -> Fader {
        fader(
            self.fader,
            Axis::Column,
            opacity,
            size::VFADER_INSET,
            egui::vec2(size::VFADER_KNOB_W, size::VFADER_KNOB_H),
        )
    }

    /// The trim's scheduled move, from the gain it is at to the one it was asked
    /// for, `rolled` of the way — which is [`reach`] on the horizontal track.
    pub fn trim_reach(&self, gain: f32, to: f32, rolled: f32) -> Reach {
        reach(self.trim_at(gain), self.trim_at(to), rolled)
    }

    /// The fader's scheduled move, on [`StripBox::trim_reach`]'s terms and off
    /// [`StripBox::fader_at`].
    pub fn fader_reach(&self, opacity: f32, to: f32, rolled: f32) -> Reach {
        reach(self.fader_at(opacity), self.fader_at(to), rolled)
    }

    /// The meter at a reading.
    pub fn meter_at(&self, level: Level) -> Meter {
        let travel = (self.meter.height() - size::VMETER_PEAK_H).max(0.0);
        let top = self.meter.max.y - size::VMETER_PEAK_H - travel * unit(level.peak);
        Meter {
            well: self.meter,
            fill: filled(self.meter, Axis::Column, level.mean),
            peak: Rect::from_min_size(
                Pos2::new(self.meter.min.x, top),
                egui::vec2(self.meter.width(), size::VMETER_PEAK_H),
            ),
        }
    }
}

/// Where the strips go: the `.mixer-strips` grid inside a mixer region.
pub fn strips_row(region: Rect) -> Option<Rect> {
    let row = Rect::from_min_size(
        Pos2::new(
            region.min.x + size::STRIPS_PAD,
            region.min.y + size::HEAD_H + size::STRIPS_PAD,
        ),
        egui::vec2(region.width() - size::STRIPS_PAD * 2.0, size::STRIP_H),
    );
    match row.width() > 0.0 && region.contains_rect(row) {
        true => Some(row),
        false => None,
    }
}

/// The arithmetic of one strip, away from the type it measures and the layout
/// it reads.
pub(crate) fn strip_box(track: Rect, label_w: f32, tally_w: f32, blend_w: f32) -> Option<StripBox> {
    let inner = Rect::from_min_max(
        Pos2::new(
            track.min.x + size::STRIP_PAD_X,
            track.min.y + size::STRIP_PAD_Y,
        ),
        Pos2::new(
            track.max.x - size::STRIP_PAD_X,
            track.max.y - size::STRIP_PAD_Y,
        ),
    );
    let column_w = size::VFADER_W + size::FADER_COL_GAP + size::VMETER_W;
    if inner.width() < column_w {
        return None;
    }
    let mut y = inner.min.y;
    let mut row = |h: f32| {
        let at = Rect::from_min_size(Pos2::new(inner.min.x, y), egui::vec2(inner.width(), h));
        y = at.max.y + size::STRIP_GAP_Y;
        at
    };
    let name = row(size::STRIP_NAME_SIZE * size::LINE);
    let tally = centred_in(row(size::TALLY_H), tally_w + size::TALLY_PAD_X * 2.0);
    let gap = 4.0;
    let btn_w = (tally.width() - gap) * 0.5;
    let solo = Rect::from_min_size(tally.min, egui::vec2(btn_w, tally.height()));
    let mute = Rect::from_min_size(
        Pos2::new(tally.max.x - btn_w, tally.min.y),
        egui::vec2(btn_w, tally.height()),
    );
    let trim = row(size::TRIM_H);
    let column = centred_in(row(size::FADER_COL_H), column_w);
    let num = row(size::STRIP_NUM_SIZE * size::LINE);
    let blend_w = blend_w + size::MINI_PAD_X * 2.0 + size::HAIRLINE * 2.0;
    let mask_w = size::MINI_SIZE + size::MINI_PAD_X * 2.0 + size::HAIRLINE * 2.0;
    let mode = centred_in(row(size::MINI_H), blend_w + size::MODE_GAP + mask_w);

    let trim_label = Rect::from_min_size(
        Pos2::new(trim.min.x + size::TRIM_PAD_X, trim.min.y),
        egui::vec2(label_w, trim.height()),
    );
    let trim_track = Rect::from_min_size(
        Pos2::new(
            trim_label.max.x + size::TRIM_GAP,
            trim.center().y - size::FADER_H * 0.5,
        ),
        egui::vec2(
            trim.max.x - size::TRIM_PAD_X - trim_label.max.x - size::TRIM_GAP,
            size::FADER_H,
        ),
    );
    match trim_track.width() > 0.0 {
        true => Some(StripBox {
            rect: track,
            name,
            tally,
            solo,
            mute,
            trim_label,
            trim: trim_track,
            fader: Rect::from_min_size(column.min, egui::vec2(size::VFADER_W, column.height())),
            meter: Rect::from_min_size(
                Pos2::new(column.max.x - size::VMETER_W, column.min.y),
                egui::vec2(size::VMETER_W, column.height()),
            ),
            num,
            blend: Rect::from_min_size(mode.min, egui::vec2(blend_w, mode.height())),
            mask: Rect::from_min_size(
                Pos2::new(mode.max.x - mask_w, mode.min.y),
                egui::vec2(mask_w, mode.height()),
            ),
        }),
        false => None,
    }
}

/// A box `w` wide centred across `row`, which is `align-items: center` on one
/// child of a flex column.
pub(crate) fn centred_in(row: Rect, w: f32) -> Rect {
    Rect::from_min_size(
        Pos2::new(row.center().x - w * 0.5, row.min.y),
        egui::vec2(w, row.height()),
    )
}
