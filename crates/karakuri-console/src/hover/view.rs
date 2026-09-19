//! Tooltip layout, styling, and text formatting for the console hover layer.

use egui::epaint::text::{LayoutJob, TextFormat};
use egui::{Color32, FontFamily, FontId, Pos2, Rect};
use karakuri_layout::Point;

use super::probes::{descriptor_at, flat, hotkey_for_tip};
use crate::room::Palette;

// -- the box, transcribed from `[data-tip]::after` ------------------------
//
// **The mock draws the tip in CSS and this is that rule, term for term.**
// Every constant below cites `style.css`, and `tests/hover.rs` resolves each
// citation the way `tests/transcribed_constants_cite_the_mock.rs` resolves
// `room::size`'s: the stylesheet is the specification and it is the half that
// moves.

/// `[data-tip]::after`'s `min-width: 150px`: a tip is never narrower than this,
/// however few words are in it.
pub const TIP_MIN_W: f32 = 150.0;

/// `[data-tip]::after`'s `max-width: 236px`, which is what the words wrap to
/// once the padding is taken off.
pub const TIP_MAX_W: f32 = 236.0;

/// `[data-tip]::after`'s `padding: 7px 9px`, down the sides.
pub const TIP_PAD_X: f32 = 9.0;

/// `[data-tip]::after`'s `padding: 7px 9px`, top and bottom.
pub const TIP_PAD_Y: f32 = 7.0;

/// `[data-tip]::after`'s `border-radius: 9px`.
pub const TIP_RADIUS: f32 = 9.0;

/// `[data-tip]::after`'s `font-size: 10.5px`.
pub const TIP_SIZE: f32 = 10.5;

/// `[data-tip]::after`'s `line-height: 1.55`, as a multiple of the size above.
pub const TIP_LINE: f32 = 1.55;

/// `[data-tip]::after`'s `top: calc(100% + 6px)`: the gap between what is being
/// explained and the box explaining it.
pub const TIP_GAP: f32 = 6.0;

/// What the words wrap to: [`TIP_MAX_W`] less the padding either side, which is
/// the width a browser lays this text out in.
///
/// The console's own arithmetic and not a number in the stylesheet —
/// `box-sizing` is the browser's rule rather than a declaration, and this is
/// it.
pub const TIP_WRAP: f32 = TIP_MAX_W - TIP_PAD_X * 2.0;

/// `[data-tip]::after`'s `box-shadow: 0 8px 26px rgba(0,0,0,0.22)`, and it is
/// the tip's own rather than `--c-shadow`: the mock gives this one box a shadow
/// of its own, so the palette's is not the one to draw it with. 0.22 of 255 is
/// 56.
pub const TIP_SHADOW: egui::epaint::Shadow = egui::epaint::Shadow {
    offset: [0, 8],
    blur: 26,
    spread: 0,
    color: Color32::from_rgba_premultiplied(0, 0, 0, 56),
};

/// Constructs a structured HUD card layout job for the tooltip.
pub fn build_tooltip_job(
    on: usize,
    words: &str,
    assigned: Option<&str>,
    pal: &Palette,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    let desc = descriptor_at(on);
    let tipped = flat().nth(on);
    let hotkey = hotkey_for_tip(on);

    // 1. Header: Operation Title / Control Name + Hotkey Badge
    let title = desc.and_then(|d| d.operation_title).unwrap_or_else(|| {
        desc.map(|d| d.label)
            .unwrap_or_else(|| tipped.map(|t| t.control).unwrap_or("Control"))
    });

    job.append(
        title,
        0.0,
        TextFormat {
            font_id: FontId::new(TIP_SIZE + 0.5, FontFamily::Proportional),
            color: pal.text,
            line_height: Some((TIP_SIZE + 0.5) * 1.3),
            ..Default::default()
        },
    );

    if let Some(hk) = hotkey {
        job.append(
            &format!("  {}", format_hotkey_badge(hk)),
            0.0,
            TextFormat {
                font_id: FontId::new(TIP_SIZE, FontFamily::Monospace),
                color: pal.lav,
                line_height: Some((TIP_SIZE + 0.5) * 1.3),
                ..Default::default()
            },
        );
    }

    // Spacing between Header and Body
    job.append(
        "\n\n",
        0.0,
        TextFormat {
            font_id: FontId::new(3.0, FontFamily::Proportional),
            line_height: Some(3.0),
            ..Default::default()
        },
    );

    // 2. Body: Concise factual specification
    let (body_raw, midi_raw) = match words.rsplit_once(MIDI_LINE) {
        Some((b, m)) => (b.trim(), Some(m.trim())),
        None => (words.trim(), None),
    };
    let body_clean = body_raw
        .trim_end_matches(['\u{2295}', ' ', '\t', '\n'])
        .trim();

    job.append(
        body_clean,
        0.0,
        TextFormat {
            font_id: FontId::new(TIP_SIZE, FontFamily::Proportional),
            color: pal.dim,
            line_height: Some(TIP_SIZE * TIP_LINE),
            ..Default::default()
        },
    );

    // 3. Footer: Action, MIDI mapping, MCP policy
    let action = desc.and_then(|d| d.action);
    let mcp = desc.and_then(|d| d.mcp_policy);

    let has_footer = action.is_some() || assigned.is_some() || midi_raw.is_some() || mcp.is_some();
    if has_footer {
        job.append(
            "\n\n",
            0.0,
            TextFormat {
                font_id: FontId::new(4.0, FontFamily::Proportional),
                line_height: Some(4.0),
                ..Default::default()
            },
        );

        if let Some(act) = action {
            job.append(
                &format!("Action: {act}\n"),
                0.0,
                TextFormat {
                    font_id: FontId::new(TIP_SIZE - 1.0, FontFamily::Proportional),
                    color: pal.faint,
                    line_height: Some((TIP_SIZE - 1.0) * 1.4),
                    ..Default::default()
                },
            );
        }

        if let Some(map_desc) = assigned {
            job.append(
                &format!("● MIDI: {map_desc}\n"),
                0.0,
                TextFormat {
                    font_id: FontId::new(TIP_SIZE - 1.0, FontFamily::Proportional),
                    color: pal.sun,
                    line_height: Some((TIP_SIZE - 1.0) * 1.4),
                    ..Default::default()
                },
            );
        } else if let Some(m) = midi_raw {
            let is_unassigned = m.starts_with("unassigned");
            let (bullet, color) = if is_unassigned {
                ("○", pal.faint)
            } else {
                ("●", pal.sun)
            };
            let summary = if is_unassigned {
                "MIDI: unassigned"
            } else if let Some((c, _)) = m.split_once(" — ") {
                c
            } else {
                m
            };
            job.append(
                &format!("{bullet} {summary}\n"),
                0.0,
                TextFormat {
                    font_id: FontId::new(TIP_SIZE - 1.0, FontFamily::Proportional),
                    color,
                    line_height: Some((TIP_SIZE - 1.0) * 1.4),
                    ..Default::default()
                },
            );
        }

        if let Some(policy) = mcp {
            job.append(
                &format!("MCP: {policy}"),
                0.0,
                TextFormat {
                    font_id: FontId::new(TIP_SIZE - 1.0, FontFamily::Monospace),
                    color: pal.mint,
                    line_height: Some((TIP_SIZE - 1.0) * 1.4),
                    ..Default::default()
                },
            );
        }
    }

    job.wrap.max_width = TIP_WRAP;
    job
}

/// Formats a hotkey as a badge for tooltip display (e.g. `"[Space]"`, `"[K]"`).
pub fn format_hotkey_badge(hotkey: &str) -> String {
    let key_name = match hotkey.to_lowercase().as_str() {
        "space" => "Space".to_string(),
        "enter" | "return" => "Enter".to_string(),
        "tab" => "Tab".to_string(),
        "esc" | "escape" => "Esc".to_string(),
        "backspace" => "Backspace".to_string(),
        other => other.to_uppercase(),
    };
    format!("[{key_name}]")
}

/// Prepends a hotkey badge to the tooltip prose if a hotkey is assigned.
pub fn with_hotkey(words: &str, hotkey: Option<&str>) -> String {
    match hotkey {
        Some(key) => format!("{} {words}", format_hotkey_badge(key)),
        None => words.to_owned(),
    }
}

/// Dynamic tooltip annotation combining optional hotkey badge and live MIDI assignment.
pub fn annotate(words: &str, hotkey: Option<&str>, on: Option<&str>) -> String {
    let with_key = with_hotkey(words, hotkey);
    assigned(&with_key, on)
}

/// The tip, with its MIDI line read off the live map where there is one to
/// read.
///
/// The page's `⊕ MIDI:` clause is the last thing every tip says, so what this
/// does is cut there and write the fact instead of the picture — see
/// [`crate::hover::Hover::assign`] for why, and
/// `docs/adr/0336-a-learn-is-a-map-edit-and-the-tips-midi-line-is-the-live-map.md`
/// for the whole argument.
///
/// Three cases and only one of them rewrites anything.
///
/// - A control the map reaches: the clause becomes what the map says.
/// - A control nothing is mapped to (`on` is `None`): the page's own
///   sentence stays, because it carries the reason — *a map line names a
///   slot, a range or a word from a closed list* — which is worth more than
///   the word *unassigned* this could put there instead.
/// - A tip with no `⊕ MIDI:` clause at all: untouched. The mock is not
///   exhaustive and a clause invented for a control the page is silent about
///   would be this console writing the manual.
///
/// It is a `Cow` in effect and an allocation only where it rewrites: a
/// `String` is built on the frame a tip appears and on no other, which is the
/// same frame the galley is laid out on.
pub fn assigned(words: &str, on: Option<&str>) -> String {
    let Some(on) = on else {
        return words.to_owned();
    };
    let Some((head, _)) = words.rsplit_once(MIDI_LINE) else {
        return words.to_owned();
    };
    format!("{head}{MIDI_LINE} {on}, which is what the map in use says today.")
}

/// The last clause of every tip on the page, as `console.html` spells it once
/// the entities are resolved — the `⊕` is `&#8853;`.
///
/// Written here rather than derived because it is the page's own punctuation
/// and there is nothing to derive it from: what makes it safe is that
/// `tests/hover.rs` fails if the page stops ending its tips this way.
pub const MIDI_LINE: &str = "\u{2295} MIDI:";

/// Where the box goes: under the pointer, and inside the window.
///
/// The mock hangs a tip off the element it explains — `top: calc(100% + 6px)`,
/// `left: 0` — and flips it at two edges: `.tip-right` and the last column open
/// leftward, and the Outputs row opens upward, *so that hovering cannot summon
/// a scrollbar*. This console has no scrollbar to summon and the rule is the
/// same one: a tip never leaves the window, so it opens down and to the right
/// of the pointer and flips at whichever edge it would cross.
///
/// It is anchored to the pointer rather than to the control, which is the one
/// place this departs from the page. A [`crate::hover::Tipped::at`] answers *is the pointer
/// on this control* and not *where is it*, so a box hung off the control's own
/// box would need every derivation to hand back a rectangle. The pointer is
/// where the operator is looking; the day a rectangle is wanted for something
/// else, this is what would change.
///
/// A box wider or taller than the window is clamped to the near edge rather
/// than flipped, because a flip would only move which half is cut off.
pub fn placed(viewport: Rect, at: Point, size: egui::Vec2) -> Rect {
    let x = match at.x + size.x > viewport.max.x {
        true => at.x - size.x,
        false => at.x,
    };
    let y = match at.y + TIP_GAP + size.y > viewport.max.y {
        true => at.y - TIP_GAP - size.y,
        false => at.y + TIP_GAP,
    };
    let min = Pos2::new(
        x.clamp(
            viewport.min.x,
            (viewport.max.x - size.x).max(viewport.min.x),
        ),
        y.clamp(
            viewport.min.y,
            (viewport.max.y - size.y).max(viewport.min.y),
        ),
    );
    Rect::from_min_size(min, size)
}
