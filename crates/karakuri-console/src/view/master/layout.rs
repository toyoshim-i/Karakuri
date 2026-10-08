//! Master bay layout derivation (ADR-0224, ADR-0340).

use egui::{Color32, FontFamily, FontId, Pos2, Rect};
use karakuri_layout::Axis;

use super::super::layout::held_inside;
use super::super::to_egui;
use super::super::widgets::fader::fader;
use super::types::*;
use crate::room::size;

/// Derives layout for the Master bay body: out row, chain slots, and `+ add`.
/// Returns `None` if engine state is absent or space is insufficient.
pub fn master(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    out: Option<f32>,
    chain: Option<&Chain>,
    choices: &AddChoices,
) -> Option<MasterRow> {
    let empty_set = std::collections::BTreeSet::new();
    let default_vr = crate::view::VrProjection::default();
    master_with_state(
        ctx,
        layout,
        out,
        chain,
        choices,
        0.0,
        &empty_set,
        &empty_set,
        None,
        false,
        &default_vr,
        false,
    )
}

/// Derives layout for the Master bay body with persistent scroll, folded, muted, and soloed states.
#[allow(clippy::too_many_arguments)]
pub fn master_with_state(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    out: Option<f32>,
    chain: Option<&Chain>,
    choices: &AddChoices,
    scroll: f32,
    folded: &std::collections::BTreeSet<u32>,
    muted: &std::collections::BTreeSet<u32>,
    soloed: Option<u32>,
    vr_mode: bool,
    vr_projection: &crate::view::VrProjection,
    vr_projection_folded: bool,
) -> Option<MasterRow> {
    let out = out?;
    // Fonts are not valid until `egui` has run a pass, exactly as in `mixer`,
    // `outputs` and `transport`.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let region = to_egui(layout.rect(layout.find("master")?));
    let width = |text: &str| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size::BASE, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };
    let row = Rect::from_min_size(
        Pos2::new(
            region.min.x + size::MASTER_PAD_X,
            region.min.y + size::HEAD_H + size::MASTER_PAD_TOP,
        ),
        egui::vec2(
            region.width() - size::MASTER_PAD_X * 2.0,
            size::MASTER_ROW_H,
        ),
    );
    if row.width() <= 0.0 || !region.contains_rect(row) {
        return None;
    }

    // The widest `d.dd` there is, which is every reading a level on this bay
    // can ask for. Measured rather than assumed.
    let widest = (0..10)
        .map(|d| width(&format!("{d}.{d}{d}")))
        .fold(0.0, f32::max);

    let label = Rect::from_min_size(row.min, egui::vec2(width(MASTER_LABEL), row.height()));
    let value = Rect::from_min_size(
        Pos2::new(row.max.x - widest, row.min.y),
        egui::vec2(widest, row.height()),
    );
    let mid = row.center().y;
    let track = Rect::from_min_max(
        Pos2::new(label.max.x + size::MASTER_GAP, mid - size::FADER_H * 0.5),
        Pos2::new(value.min.x - size::MASTER_GAP, mid + size::FADER_H * 0.5),
    );
    // Suppress control when horizontal track space is depleted.
    if track.width() <= 0.0 {
        return None;
    }

    // The chain's own rows, under the out row and off the same width. A
    // console with no chain behind it draws the out row it was handed a level
    // for and nothing under it.
    let mut slots = Vec::new();
    let mut vr_stage = None;
    let mut add = None;
    let mut list: Option<Rect> = None;
    if chain.is_some() || vr_mode {
        let body_min = Pos2::new(row.min.x, row.max.y + size::MASTER_STACK_GAP);
        let body_max = Pos2::new(
            row.max.x,
            (region.max.y - size::MASTER_PAD_X).max(body_min.y),
        );
        let body_rect = Rect::from_min_max(body_min, body_max);

        // Compute total content height across vr_stage, all slots and `+ add` button.
        let mut total_content_h = 0.0;
        let vr_h = if vr_mode {
            let count = if vr_projection_folded {
                0
            } else {
                VrParamKey::ALL.len()
            };
            Some(well_height(count, vr_projection_folded))
        } else {
            None
        };
        if let Some(h) = vr_h {
            total_content_h += h;
        }

        if let Some(chain) = chain {
            for (at, slot) in chain.slots.iter().enumerate() {
                if total_content_h > 0.0 {
                    total_content_h += size::MASTER_STACK_GAP;
                }
                let is_folded = folded.contains(&(at as u32));
                total_content_h += well_height(slot.params.len(), is_folded);
            }
            if !chain.slots.is_empty() || vr_mode {
                total_content_h += size::MASTER_STACK_GAP;
            }
            total_content_h += size::FX_H;
        }

        let max_scroll = (total_content_h - body_rect.height()).max(0.0);
        let clamped_scroll = scroll.clamp(0.0, max_scroll);

        let mut top = body_min.y - clamped_scroll;

        if vr_mode {
            if let Some(h) = vr_h {
                let well =
                    Rect::from_min_size(Pos2::new(row.min.x, top), egui::vec2(row.width(), h));
                if well.max.y >= body_rect.min.y && well.min.y <= body_rect.max.y {
                    vr_stage = vr_stage_row(
                        ctx,
                        vr_projection,
                        vr_projection_folded,
                        well,
                        widest,
                        &width,
                    );
                }
                top = well.max.y + size::MASTER_STACK_GAP;
            }
        }

        if let Some(chain) = chain {
            for (at, slot) in chain.slots.iter().enumerate() {
                let at_u32 = at as u32;
                let is_folded = folded.contains(&at_u32);
                let is_soloed = soloed == Some(at_u32);
                let is_muted = muted.contains(&at_u32);
                let is_online = match soloed {
                    Some(s) => s == at_u32,
                    None => !is_muted,
                };
                let height = well_height(slot.params.len(), is_folded);
                let well =
                    Rect::from_min_size(Pos2::new(row.min.x, top), egui::vec2(row.width(), height));
                // Include slot if visible or intersecting the visible body area.
                if well.max.y >= body_rect.min.y && well.min.y <= body_rect.max.y {
                    let num_params = if is_folded { 0 } else { slot.params.len() };
                    if let Some(drawn) = slot_row(
                        ctx, at_u32, slot, well, widest, &width, num_params, is_folded, is_soloed,
                        is_muted, is_online,
                    ) {
                        slots.push(drawn);
                    }
                }
                top = well.max.y + size::MASTER_STACK_GAP;
            }

            let pill = Rect::from_min_size(
                Pos2::new(row.min.x, top),
                egui::vec2(row.width(), size::FX_H),
            );
            if pill.max.y >= body_rect.min.y && pill.min.y <= body_rect.max.y {
                add = Some(pill);
            }
        }
        list = Some(body_rect);
    }

    let card = add.and_then(|add| add_card(ctx, to_egui(layout.viewport()), add, choices));

    Some(MasterRow {
        label,
        fader: fader(
            track,
            Axis::Row,
            out,
            0.0,
            egui::vec2(size::FADER_KNOB_W, size::FADER_KNOB_H),
        ),
        value,
        out,
        slots,
        vr_stage,
        add,
        card,
        list,
    })
}

/// How tall a slot's well is: the head line, then one line per declared
/// parameter, inside the well's own padding and [`size::FX_PAD_Y`] between the
/// lines.
pub(super) fn well_height(params: usize, is_folded: bool) -> f32 {
    if is_folded {
        return size::NODE_HEAD_H;
    }
    size::NODE_HEAD_H
        + size::FX_PAD_Y * 2.0
        + size::MASTER_ROW_H * params as f32
        + size::FX_PAD_Y * params.saturating_sub(1) as f32
}

/// Lays out a single effects chain slot within `well`.
#[allow(clippy::too_many_arguments)]
fn slot_row(
    ctx: &egui::Context,
    at: u32,
    slot: &ChainSlot,
    well: Rect,
    widest: f32,
    width: &dyn Fn(&str) -> f32,
    num_params: usize,
    is_folded: bool,
    is_soloed: bool,
    is_muted: bool,
    is_online: bool,
) -> Option<SlotRow> {
    let width_at = |text: &str, size: f32| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };
    let head = Rect::from_min_size(well.min, egui::vec2(well.width(), size::NODE_HEAD_H));
    if head.width() <= 0.0 {
        return None;
    }
    let mid = head.center().y;
    let dot = Rect::from_center_size(
        Pos2::new(head.min.x + size::FX_PAD_X + size::FX_DOT * 0.5, mid),
        egui::vec2(size::FX_DOT, size::FX_DOT),
    );
    let name = Rect::from_min_size(
        Pos2::new(dot.max.x + size::FX_GAP, head.min.y),
        egui::vec2(width(&slot.name), head.height()),
    );
    let minus = width(REMOVE_GLYPH);
    let remove = Rect::from_min_size(
        Pos2::new(head.max.x - size::FX_PAD_X - minus, head.min.y),
        egui::vec2(minus, head.height()),
    );
    let mut right_cursor = remove.min.x - size::FX_GAP;
    let cut = slot.cut.map(|_| {
        let word = karakuri_operation::Cut::ALL
            .iter()
            .map(|c| width_at(c.name(), size::MINI_SIZE))
            .fold(0.0, f32::max);
        let chip = word + size::MINI_PAD_X * 2.0 + size::HAIRLINE * 2.0;
        let rect = Rect::from_center_size(
            Pos2::new(right_cursor - chip * 0.5, mid),
            egui::vec2(chip, size::MINI_H),
        );
        right_cursor = rect.min.x - size::FX_GAP;
        rect
    });
    let btn_w = 18.0;
    let btn_h = size::AUTH_H;
    let mute = Rect::from_center_size(
        Pos2::new(right_cursor - btn_w * 0.5, mid),
        egui::vec2(btn_w, btn_h),
    );
    right_cursor = mute.min.x - size::FX_GAP;
    let solo = Rect::from_center_size(
        Pos2::new(right_cursor - btn_w * 0.5, mid),
        egui::vec2(btn_w, btn_h),
    );

    let mut params = Vec::with_capacity(num_params);
    if !is_folded {
        let mut top = head.max.y + size::FX_PAD_Y;
        for (i, param) in slot.params.iter().take(num_params).enumerate() {
            let ord = i + 1;
            let line = Rect::from_min_size(
                Pos2::new(well.min.x + size::FX_PAD_X, top),
                egui::vec2(well.width() - size::FX_PAD_X * 2.0, size::MASTER_ROW_H),
            );
            let ord_rect =
                Rect::from_min_size(line.min, egui::vec2(size::PARAM_ORD_W, line.height()));
            let label = Rect::from_min_size(
                Pos2::new(ord_rect.max.x + size::PARAM_GAP, line.min.y),
                egui::vec2(width(&param.key), line.height()),
            );
            let amount = Rect::from_min_size(
                Pos2::new(line.max.x - widest, line.min.y),
                egui::vec2(widest, line.height()),
            );
            let centre = line.center().y;
            let track = Rect::from_min_max(
                Pos2::new(label.max.x + size::FX_GAP, centre - size::FADER_H * 0.5),
                Pos2::new(amount.min.x - size::FX_GAP, centre + size::FADER_H * 0.5),
            );
            if track.width() <= 0.0 {
                return None;
            }
            let row = ParamRow {
                at,
                ord,
                key: param.key.clone(),
                range: param.range,
                value: param.value,
                ord_rect,
                label,
                fader: fader(
                    track,
                    Axis::Row,
                    0.0,
                    0.0,
                    egui::vec2(size::FADER_KNOB_W, size::FADER_KNOB_H),
                ),
                amount,
            };
            let placed = ParamRow {
                fader: fader(
                    track,
                    Axis::Row,
                    row.along(),
                    0.0,
                    egui::vec2(size::FADER_KNOB_W, size::FADER_KNOB_H),
                ),
                ..row
            };
            params.push(placed);
            top = line.max.y + size::FX_PAD_Y;
        }
    }

    Some(SlotRow {
        at,
        well,
        head,
        dot,
        name,
        words: slot.name.clone(),
        solo,
        is_soloed,
        mute,
        is_muted,
        is_online,
        is_folded,
        cut,
        reading: slot.cut,
        remove,
        params,
    })
}

/// Derives the `+ add` chooser card, clamped to the viewport, or `None` if shut/empty.
fn add_card(
    ctx: &egui::Context,
    viewport: Rect,
    add: Rect,
    choices: &AddChoices,
) -> Option<AddCard> {
    if !choices.open || choices.items.is_empty() {
        return None;
    }
    let width = |text: &str| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size::BASE, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };
    let widest = choices
        .items
        .iter()
        .map(|item| width(&item.words))
        .fold(size::ROW_MENU_MIN_W, f32::max);
    let card_w = widest + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0;
    let height = size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H * choices.items.len() as f32;
    // If opening downward would spill past the bottom of the viewport, open upward above `+ add`.
    let y = if add.max.y + size::PILL_GAP + height > viewport.max.y {
        add.min.y - size::PILL_GAP - height
    } else {
        add.max.y + size::PILL_GAP
    };
    let card = held_inside(
        &viewport,
        add.max.x - card_w,
        y.max(viewport.min.y),
        card_w,
        height,
    );
    Some(AddCard {
        card,
        items: choices.items.len(),
    })
}

/// Lays out the Built-in VR Projection stage well within `well`.
fn vr_stage_row(
    ctx: &egui::Context,
    proj: &crate::view::VrProjection,
    is_folded: bool,
    well: Rect,
    widest: f32,
    width: &dyn Fn(&str) -> f32,
) -> Option<VrProjectionRow> {
    let width_at = |text: &str, size: f32| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };
    let head = Rect::from_min_size(well.min, egui::vec2(well.width(), size::NODE_HEAD_H));
    if head.width() <= 0.0 {
        return None;
    }
    let mid = head.center().y;
    let dot = Rect::from_center_size(
        Pos2::new(head.min.x + size::FX_PAD_X + size::FX_DOT * 0.5, mid),
        egui::vec2(size::FX_DOT, size::FX_DOT),
    );
    let title_text = "VR Projection";
    let name = Rect::from_min_size(
        Pos2::new(dot.max.x + size::FX_GAP, head.min.y),
        egui::vec2(width(title_text), head.height()),
    );

    let badge_word = "Built-in";
    let badge_w = width_at(badge_word, size::MINI_SIZE) + size::MINI_PAD_X * 2.0;
    let badge = Rect::from_center_size(
        Pos2::new(name.max.x + size::FX_GAP + badge_w * 0.5, mid),
        egui::vec2(badge_w, size::MINI_H),
    );

    let mode_label = proj.mode.label();
    let mode_text_w = width_at(mode_label, size::MINI_SIZE);
    let mode_pill_w = mode_text_w + size::MINI_PAD_X * 2.0 + size::HAIRLINE * 2.0;
    let mode_pill = Rect::from_center_size(
        Pos2::new(head.max.x - size::FX_PAD_X - mode_pill_w * 0.5, mid),
        egui::vec2(mode_pill_w, size::MINI_H),
    );

    let mut params = Vec::new();
    if !is_folded {
        let mut top = head.max.y + size::FX_PAD_Y;
        for (i, &key) in VrParamKey::ALL.iter().enumerate() {
            let ord = i + 1;
            let line = Rect::from_min_size(
                Pos2::new(well.min.x + size::FX_PAD_X, top),
                egui::vec2(well.width() - size::FX_PAD_X * 2.0, size::MASTER_ROW_H),
            );
            let ord_rect =
                Rect::from_min_size(line.min, egui::vec2(size::PARAM_ORD_W, line.height()));
            let label = Rect::from_min_size(
                Pos2::new(ord_rect.max.x + size::PARAM_GAP, line.min.y),
                egui::vec2(width(key.label()), line.height()),
            );
            let amount = Rect::from_min_size(
                Pos2::new(line.max.x - widest, line.min.y),
                egui::vec2(widest, line.height()),
            );
            let centre = line.center().y;
            let track = Rect::from_min_max(
                Pos2::new(label.max.x + size::FX_GAP, centre - size::FADER_H * 0.5),
                Pos2::new(amount.min.x - size::FX_GAP, centre + size::FADER_H * 0.5),
            );
            if track.width() <= 0.0 {
                return None;
            }
            let val = key.value(proj);
            let row = VrParamRow {
                key,
                ord,
                ord_rect,
                label,
                fader: fader(
                    track,
                    Axis::Row,
                    0.0,
                    0.0,
                    egui::vec2(size::FADER_KNOB_W, size::FADER_KNOB_H),
                ),
                amount,
                value: val,
                range: key.range(),
                is_active: key.is_active_for(proj.mode),
            };
            let placed = VrParamRow {
                fader: fader(
                    track,
                    Axis::Row,
                    row.along(),
                    0.0,
                    egui::vec2(size::FADER_KNOB_W, size::FADER_KNOB_H),
                ),
                ..row
            };
            params.push(placed);
            top = line.max.y + size::FX_PAD_Y;
        }
    }

    Some(VrProjectionRow {
        well,
        head,
        dot,
        name,
        badge,
        mode_pill,
        mode: proj.mode,
        is_folded,
        params,
    })
}

/// The level, as the mock's `.val` writes it — `1.00`, two places, and the same
/// string the transport row's exposure is written with.
pub(super) fn master_text(out: f32) -> String {
    format!("{out:.2}")
}
