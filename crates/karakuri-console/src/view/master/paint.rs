//! Master bay rendering functions (ADR-0224, ADR-0340).

use egui::{Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, Stroke, StrokeKind, Ui};

use super::super::widgets::chip::{mute_button_into, solo_button_into};
use super::super::widgets::fader::{fader_into, mini_into, tint};
use super::layout::master_text;
use super::types::*;
use crate::room::{size, Palette};

/// Paints the Master bay body (label, fader, level value, slots, and add button).
pub(crate) fn master_into(ui: &Ui, pal: &Palette, row: &MasterRow) {
    let painter = ui.painter();
    let centred = |rect: Rect, galley: std::sync::Arc<egui::Galley>, colour: Color32| {
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            colour,
        );
    };
    centred(
        row.label,
        painter.layout_no_wrap(
            MASTER_LABEL.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.faint,
        ),
        pal.faint,
    );
    fader_into(painter, pal, row.fader, false, None);
    // Right-align readout value against fader bounds.
    let galley = painter.layout_no_wrap(
        master_text(row.out),
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.text,
    );
    painter.galley(
        Pos2::new(
            row.value.max.x - galley.size().x,
            row.value.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.text,
    );

    let list_clip = row.list.unwrap_or(Rect::EVERYTHING);
    let painter = ui.painter().with_clip_rect(list_clip);
    if let Some(vr) = &row.vr_stage {
        vr_stage_into(&painter, pal, vr);
    }
    for slot in &row.slots {
        slot_into(&painter, pal, slot);
    }
    if let Some(add) = row.add {
        add_into(&painter, pal, add);
    }
}

/// Paints a single chain slot (well, mint indicator dot, name, solo, mute, cut chip, and remove button).
pub(super) fn slot_into(painter: &egui::Painter, pal: &Palette, slot: &SlotRow) {
    painter.rect_filled(slot.well, CornerRadius::same(size::FX_RADIUS), pal.well);
    let border = if slot.is_online { pal.mint } else { pal.line };
    painter.rect_stroke(
        slot.well,
        CornerRadius::same(size::FX_RADIUS),
        Stroke::new(size::HAIRLINE, border),
        StrokeKind::Inside,
    );
    let head_radius = if slot.is_folded {
        CornerRadius::same(size::FX_RADIUS)
    } else {
        CornerRadius {
            nw: size::FX_RADIUS,
            ne: size::FX_RADIUS,
            sw: 0,
            se: 0,
        }
    };
    painter.rect_filled(slot.head, head_radius, pal.tint);
    if !slot.is_folded {
        painter.line_segment(
            [
                Pos2::new(slot.head.min.x, slot.head.max.y),
                Pos2::new(slot.head.max.x, slot.head.max.y),
            ],
            Stroke::new(size::HAIRLINE, pal.line),
        );
    }
    let dot_col = if slot.is_online { pal.mint } else { pal.faint };
    painter.circle_filled(slot.dot.center(), size::FX_DOT * 0.5, dot_col);
    let word = |rect: Rect, text: &str, colour: Color32, align_right: bool| {
        let galley = painter.layout_no_wrap(
            text.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            colour,
        );
        let x = match align_right {
            true => rect.max.x - galley.size().x,
            false => rect.min.x,
        };
        painter.galley(
            Pos2::new(x, rect.center().y - galley.size().y * 0.5),
            galley,
            colour,
        );
    };
    let name_col = if slot.is_online { pal.lav } else { pal.dim };
    word(slot.name, &slot.words, name_col, false);
    solo_button_into(painter, pal, slot.solo, slot.is_soloed);
    mute_button_into(painter, pal, slot.mute, slot.is_muted);
    if let (Some(chip), Some(cut)) = (slot.cut, slot.reading) {
        mini_into(painter, pal, chip, true, |painter, colour| {
            let galley = painter.layout_no_wrap(
                cut.name().to_owned(),
                FontId::new(size::MINI_SIZE, FontFamily::Proportional),
                colour,
            );
            painter.galley(
                Pos2::new(
                    chip.center().x - galley.size().x * 0.5,
                    chip.center().y - galley.size().y * 0.5,
                ),
                galley,
                colour,
            );
        });
    }
    word(slot.remove, REMOVE_GLYPH, pal.faint, true);

    if !slot.is_folded {
        for param in &slot.params {
            let ord_galley = painter.layout_no_wrap(
                param.ord.to_string(),
                FontId::new(size::PARAM_ORD_SIZE, FontFamily::Proportional),
                pal.faint,
            );
            painter.galley(
                Pos2::new(
                    param.ord_rect.max.x - ord_galley.size().x,
                    param.ord_rect.center().y - ord_galley.size().y * 0.5,
                ),
                ord_galley,
                pal.faint,
            );
            word(param.label, &param.key, pal.faint, false);
            fader_into(painter, pal, param.fader, false, None);
            word(param.amount, &master_text(param.value), pal.text, true);
        }
    }
}

/// `+ add`, painted: the word centred in a `.fx` well, faint.
pub(super) fn add_into(painter: &egui::Painter, pal: &Palette, add: Rect) {
    painter.rect_filled(add, CornerRadius::same(size::FX_RADIUS), pal.well);
    let galley = painter.layout_no_wrap(
        ADD_LABEL.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.faint,
    );
    painter.galley(
        Pos2::new(
            add.center().x - galley.size().x * 0.5,
            add.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.faint,
    );
}

/// Paints the Built-in VR Projection stage (well, indicator dot, name, Built-in badge, mode pill, and parameter faders).
pub(super) fn vr_stage_into(painter: &egui::Painter, pal: &Palette, vr: &VrProjectionRow) {
    painter.rect_filled(vr.well, CornerRadius::same(size::FX_RADIUS), pal.well);
    let is_active = vr.mode != crate::view::VrProjectionMode::Wall;
    let border = if is_active { pal.mint } else { pal.line };
    painter.rect_stroke(
        vr.well,
        CornerRadius::same(size::FX_RADIUS),
        Stroke::new(size::HAIRLINE, border),
        StrokeKind::Inside,
    );
    let head_radius = if vr.is_folded {
        CornerRadius::same(size::FX_RADIUS)
    } else {
        CornerRadius {
            nw: size::FX_RADIUS,
            ne: size::FX_RADIUS,
            sw: 0,
            se: 0,
        }
    };
    painter.rect_filled(vr.head, head_radius, pal.tint);
    if !vr.is_folded {
        painter.line_segment(
            [
                Pos2::new(vr.head.min.x, vr.head.max.y),
                Pos2::new(vr.head.max.x, vr.head.max.y),
            ],
            Stroke::new(size::HAIRLINE, pal.line),
        );
    }
    let dot_col = if is_active { pal.mint } else { pal.faint };
    painter.circle_filled(vr.dot.center(), size::FX_DOT * 0.5, dot_col);

    let word = |rect: Rect, text: &str, colour: Color32, align_right: bool| {
        let galley = painter.layout_no_wrap(
            text.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            colour,
        );
        let x = match align_right {
            true => rect.max.x - galley.size().x,
            false => rect.min.x,
        };
        painter.galley(
            Pos2::new(x, rect.center().y - galley.size().y * 0.5),
            galley,
            colour,
        );
    };

    word(vr.name, "VR Projection", pal.text, false);

    // [Built-in] badge
    let badge_radius = CornerRadius::same(size::BADGE_RADIUS as u8);
    painter.rect_filled(vr.badge, badge_radius, pal.panel);
    painter.rect_stroke(
        vr.badge,
        badge_radius,
        Stroke::new(size::HAIRLINE, pal.faint),
        StrokeKind::Inside,
    );
    let badge_galley = painter.layout_no_wrap(
        "Built-in".to_owned(),
        FontId::new(size::MINI_SIZE, FontFamily::Proportional),
        pal.faint,
    );
    painter.galley(
        Pos2::new(
            vr.badge.center().x - badge_galley.size().x * 0.5,
            vr.badge.center().y - badge_galley.size().y * 0.5,
        ),
        badge_galley,
        pal.faint,
    );

    // Mode pill
    let (pill_bg, pill_fg) = if is_active {
        (pal.mint, pal.panel)
    } else {
        (pal.tint, pal.text)
    };
    let pill_radius = CornerRadius::same((size::MINI_H * 0.5) as u8);
    painter.rect_filled(vr.mode_pill, pill_radius, pill_bg);
    painter.rect_stroke(
        vr.mode_pill,
        pill_radius,
        Stroke::new(size::HAIRLINE, if is_active { pal.mint } else { pal.line }),
        StrokeKind::Inside,
    );
    let mode_galley = painter.layout_no_wrap(
        vr.mode.label().to_owned(),
        FontId::new(size::MINI_SIZE, FontFamily::Proportional),
        pill_fg,
    );
    painter.galley(
        Pos2::new(
            vr.mode_pill.center().x - mode_galley.size().x * 0.5,
            vr.mode_pill.center().y - mode_galley.size().y * 0.5,
        ),
        mode_galley,
        pill_fg,
    );

    if !vr.is_folded {
        for param in &vr.params {
            vr_param_into(painter, pal, param);
        }
    }
}

fn vr_param_into(painter: &egui::Painter, pal: &Palette, param: &VrParamRow) {
    let is_active = param.is_active;

    let ord_col = if is_active {
        pal.faint
    } else {
        tint(pal.faint, 40)
    };
    let ord_galley = painter.layout_no_wrap(
        format!("{:02}", param.ord),
        FontId::new(size::PARAM_ORD_SIZE, FontFamily::Monospace),
        ord_col,
    );
    painter.galley(
        Pos2::new(
            param.ord_rect.min.x,
            param.ord_rect.center().y - ord_galley.size().y * 0.5,
        ),
        ord_galley,
        ord_col,
    );

    let label_col = if is_active {
        pal.text
    } else {
        tint(pal.faint, 60)
    };
    let label_galley = painter.layout_no_wrap(
        param.key.label().to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        label_col,
    );
    painter.galley(
        Pos2::new(
            param.label.min.x,
            param.label.center().y - label_galley.size().y * 0.5,
        ),
        label_galley,
        label_col,
    );

    if is_active {
        fader_into(painter, pal, param.fader, false, None);
    } else {
        // Muted gray track and knob without vibrant gradient for inactive parameters
        let fader = param.fader;
        let radius =
            CornerRadius::same((fader.track.width().min(fader.track.height()) * 0.5) as u8);
        painter.rect_filled(fader.track, radius, pal.well);
        painter.rect_stroke(
            fader.track,
            radius,
            Stroke::new(size::HAIRLINE, pal.line),
            StrokeKind::Inside,
        );
        if fader.fill.width() > 0.0 && fader.fill.height() > 0.0 {
            painter.rect_filled(fader.fill, CornerRadius::ZERO, tint(pal.line, 50));
        }
        let knob_r = CornerRadius::same((fader.knob.width().min(fader.knob.height()) * 0.5) as u8);
        painter.rect_filled(fader.knob, knob_r, pal.panel);
        painter.rect_stroke(
            fader.knob,
            knob_r,
            Stroke::new(size::HAIRLINE, pal.line),
            StrokeKind::Outside,
        );
    }

    let amount_col = if is_active {
        pal.text
    } else {
        tint(pal.faint, 60)
    };
    let amount_galley = painter.layout_no_wrap(
        format!("{:.2}", param.value),
        FontId::new(size::BASE, FontFamily::Proportional),
        amount_col,
    );
    painter.galley(
        Pos2::new(
            param.amount.max.x - amount_galley.size().x,
            param.amount.center().y - amount_galley.size().y * 0.5,
        ),
        amount_galley,
        amount_col,
    );
}
