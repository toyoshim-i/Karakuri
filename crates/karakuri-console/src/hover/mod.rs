//! Console hover layer and tooltip rendering using manual citations (ADR-0330, ADR-0156).
//!
//! Maps pointer hit tests to manual tip citations and renders tooltips with modal suppression
//! and frame-deadline tracking (ADR-0283, P-0085, P-0091).

pub mod citation;
pub mod probes;
pub mod view;

pub use citation::*;
pub use probes::*;
pub use view::*;

use std::sync::Arc;
use std::time::Duration;

use egui::{CornerRadius, Galley, Stroke, StrokeKind, Ui};
use karakuri_layout::Point;

use crate::input::Claim;
use crate::panel::Panel;
use crate::room::size::HAIRLINE;
use crate::room::Room;
use crate::view::{to_egui, View};

/// What the hover layer is owed, and what [`crate::repaint::Change::Tip`] turns
/// into a decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tip {
    /// Nothing. The pointer is on no tipped control, or the tip it is on is already
    /// drawn and does not move while it is up. This is the layer declaring nothing
    /// at rest.
    Still,
    /// A dwell is running, and this is what is left of it: the picture is different
    /// from the one on screen in exactly this long, which is
    /// [`crate::budget::Declared::moves_in`]'s question asked of a hand holding
    /// still (ADR-0283).
    Dwelling(Duration),
    /// A tip is on screen that must not be: the pointer has left the control it
    /// belongs to. A frame is owed now, to take it down.
    Gone,
}

/// The console's hover layer tracking pointer rest, tip display, and cached layout galley.
///
/// Timestamps arrive as external [`Duration`] values rather than maintaining an internal clock.
pub struct Hover {
    tips: Tips,
    resting: Option<Rest>,
    /// Whether the tip has been painted. Written by [`Hover::paint`], because what
    /// is on screen is a fact about the frame that drew it.
    up: bool,
    /// The words laid out, kept while the pointer stays on the control they belong
    /// to and its assignment has not moved. A learn changes the last line of the
    /// tip that is on screen, and a cache keyed on the control alone would go on
    /// drawing the old one.
    galley: Option<CachedTipGalley>,
    /// Which knob the control under the pointer is on, as the host derived it from
    /// the live map — see [`Hover::assign`].
    assigned: Option<String>,
    tip_box: Option<egui::Rect>,
    key_badge_rect: Option<egui::Rect>,
    key_global_rect: Option<egui::Rect>,
    learning_key: Option<usize>,
    key_globalize: bool,
    custom_hotkey: Option<String>,
}

type CachedTipGalley = (
    usize,
    Option<String>,
    Option<String>,
    bool,
    Room,
    Arc<Galley>,
);

/// Where the pointer came to rest, when, and on what.
#[derive(Debug, Clone, Copy)]
struct Rest {
    at: Point,
    since: Duration,
    on: usize,
}

impl Default for Hover {
    fn default() -> Hover {
        Hover::new()
    }
}

impl Hover {
    /// Read the page and take no other reading. At start-up: the parse is
    /// [`Tips::read`]'s and is not on any frame path.
    pub fn new() -> Hover {
        Hover {
            tips: Tips::read(),
            resting: None,
            up: false,
            galley: None,
            assigned: None,
            tip_box: None,
            key_badge_rect: None,
            key_global_rect: None,
            learning_key: None,
            key_globalize: false,
            custom_hotkey: None,
        }
    }

    /// Pointer rest position and control index in [`flat`] (ADR-0336 learn seam).
    ///
    /// Returns immediately without waiting for tooltip dwell completion.
    pub fn resting(&self) -> Option<(Point, usize)> {
        self.resting.map(|rest| (rest.at, rest.on))
    }

    /// Whether the hover layer is currently in Key Learn mode for a control.
    pub fn learning_key(&self) -> Option<usize> {
        self.learning_key
    }

    /// Sets or clears the active Key Learn mode control.
    pub fn set_learning_key(&mut self, on: Option<usize>) {
        self.learning_key = on;
        self.galley = None;
    }

    /// Toggles the globalize flag for key learn.
    pub fn toggle_globalize(&mut self) {
        self.key_globalize = !self.key_globalize;
        self.galley = None;
    }

    /// Whether key learn will promote binding to global.
    pub fn key_globalize(&self) -> bool {
        self.key_globalize
    }

    /// Explicitly sets the globalize flag for key learn.
    pub fn set_key_globalize(&mut self, glob: bool) {
        self.key_globalize = glob;
        self.galley = None;
    }

    /// Sets the custom hotkey for the active control under hover.
    pub fn set_custom_hotkey(&mut self, hk: Option<String>) {
        if self.custom_hotkey != hk {
            self.custom_hotkey = hk;
            self.galley = None;
        }
    }

    /// Whether `p` hits the active tooltip card bounding box.
    pub fn hit_tip_box(&self, p: Point) -> bool {
        self.up
            && self
                .tip_box
                .is_some_and(|b| b.contains(egui::pos2(p.x, p.y)))
    }

    /// Whether `p` hits the key learning badge in the active tooltip, returning the control index if so.
    pub fn hit_key_badge(&self, p: Point) -> Option<usize> {
        if self.up
            && self
                .key_badge_rect
                .is_some_and(|b| b.contains(egui::pos2(p.x, p.y)))
        {
            self.resting.map(|r| r.on).or(self.learning_key)
        } else {
            None
        }
    }

    /// Whether `p` hits the globalize checkbox toggle in the active tooltip.
    pub fn hit_global_toggle(&self, p: Point) -> bool {
        self.up
            && self
                .key_global_rect
                .is_some_and(|b| b.contains(egui::pos2(p.x, p.y)))
    }

    /// Sets the active MIDI assignment for the hovered control (ADR-0156, P-0087).
    ///
    /// Host-derived once per frame to overlay actual mappings over static manual text.
    pub fn assign(&mut self, on: Option<String>) {
        self.assigned = on;
    }

    /// Dwell duration before a tooltip appears, read from `ctx.style().interaction.tooltip_delay`.
    pub fn dwell(ctx: &egui::Context) -> Duration {
        let style = ctx.style_of(ctx.theme());
        Duration::from_secs_f32(style.interaction.tooltip_delay.max(0.0))
    }

    /// Updates hover state on pointer movement, returning owed [`Tip`] status.
    ///
    /// Fast-paths unhovered controls via [`Claim::Egui`] and clears tips on active drags.
    pub fn moved(
        &mut self,
        claim: Claim,
        panel: &Panel,
        ctx: &egui::Context,
        view: &View,
        p: Point,
        now: Duration,
    ) -> Tip {
        if ctx.cumulative_pass_nr() == 0 {
            return Tip::Still;
        }
        let pos = egui::pos2(p.x, p.y);
        if self.up && self.tip_box.is_some_and(|b| b.contains(pos)) {
            return Tip::Still;
        }
        if self.learning_key.is_some() {
            return Tip::Still;
        }
        let on = match claim == Claim::Panel && !panel.dragging() {
            true => resolve(panel, ctx, view, p),
            false => None,
        };
        match (self.resting, on) {
            // The same control: a tip that is up stays exactly where it is,
            // and one that is not yet up starts its dwell again from here.
            (Some(rest), Some(on)) if rest.on == on => {
                if !self.up {
                    self.resting = Some(Rest {
                        at: p,
                        since: now,
                        on,
                    });
                }
                Tip::Still
            }
            // Another control, or none at all.
            (_, on) => {
                let owed = match self.up {
                    true => Tip::Gone,
                    false => Tip::Still,
                };
                self.up = false;
                self.tip_box = None;
                self.key_badge_rect = None;
                self.key_global_rect = None;
                self.resting = on.map(|on| Rest {
                    at: p,
                    since: now,
                    on,
                });
                owed
            }
        }
    }

    /// The pointer left the window, which is not a move to anywhere: any tip goes
    /// and no dwell is running.
    pub fn left(&mut self) -> Tip {
        if self.learning_key.is_some() {
            return Tip::Still;
        }
        let owed = match self.up {
            true => Tip::Gone,
            false => Tip::Still,
        };
        self.up = false;
        self.tip_box = None;
        self.key_badge_rect = None;
        self.key_global_rect = None;
        self.resting = None;
        owed
    }

    /// Returns when the hover layer next requires a repaint (e.g. dwell deadline).
    ///
    /// Yields [`Tip::Dwelling`] while dwelling, or [`Tip::Still`] if static or already shown.
    pub fn owed(&self, ctx: &egui::Context, now: Duration) -> Tip {
        match (self.resting, self.up) {
            (Some(rest), false) => {
                match Hover::dwell(ctx).saturating_sub(now.saturating_sub(rest.since)) {
                    left if left.is_zero() => Tip::Still,
                    left => Tip::Dwelling(left),
                }
            }
            _ => Tip::Still,
        }
    }

    /// The words on screen, or `None` where no tip is up. What a test reads, and
    /// the only way anything outside this module can tell.
    pub fn showing(&self) -> Option<&str> {
        match self.up {
            true => self.tips.get(self.resting?.on),
            false => None,
        }
    }

    /// The annotated words on screen (including hotkey badge and live MIDI assignment),
    /// or `None` if no tip is up.
    pub fn showing_annotated(&self) -> Option<String> {
        if !self.up {
            return None;
        }
        let on = self.resting?.on;
        let words = self.tips.get(on)?;
        let hotkey = hotkey_for_tip(on);
        Some(annotate(words, hotkey, self.assigned.as_deref()))
    }

    /// Which control the pointer is resting on, whether or not its dwell has run
    /// out.
    pub fn resting_on(&self) -> Option<&'static Tipped> {
        flat().nth(self.resting?.on)
    }

    /// The tips this layer read out of the page.
    pub fn tips(&self) -> &Tips {
        &self.tips
    }

    /// Paints the active hover tooltip over other panel elements.
    ///
    /// Gathers cached layout galley and suppresses rendering when modal overlays are active.
    pub fn paint(&mut self, ui: &Ui, panel: &Panel, view: &View, now: Duration) {
        if view.has_modal_overlay() {
            self.left();
            return;
        }
        let Some(rest) = self.resting else {
            return;
        };
        if now.saturating_sub(rest.since) < Hover::dwell(ui.ctx()) {
            return;
        }
        let Some(words) = self.tips.get(rest.on) else {
            return;
        };
        let pal = view.room.palette();
        let is_learning = self.learning_key == Some(rest.on);
        let custom_hk = self.custom_hotkey.as_deref();
        let galley = match &self.galley {
            Some((on, was, was_hk, was_learn, room, galley))
                if *on == rest.on
                    && was.as_deref() == self.assigned.as_deref()
                    && was_hk.as_deref() == custom_hk
                    && *was_learn == is_learning
                    && *room == view.room =>
            {
                galley.clone()
            }
            _ => {
                let job = build_tooltip_card_job(
                    rest.on,
                    words,
                    self.assigned.as_deref(),
                    custom_hk,
                    is_learning,
                    &pal,
                );
                let galley = ui.painter().layout_job(job);
                self.galley = Some((
                    rest.on,
                    self.assigned.clone(),
                    self.custom_hotkey.clone(),
                    is_learning,
                    view.room,
                    galley.clone(),
                ));
                galley
            }
        };
        self.up = true;

        let key_pill_h = 20.0;
        let size = egui::vec2(
            galley.size().x.max(TIP_MIN_W - TIP_PAD_X * 2.0) + TIP_PAD_X * 2.0,
            galley.size().y + TIP_PAD_Y * 2.0 + key_pill_h + 6.0,
        );
        let box_ = placed(to_egui(panel.layout().viewport()), rest.at, size);
        self.tip_box = Some(box_);

        let painter = ui.painter();
        let radius = CornerRadius::same(TIP_RADIUS as u8);
        painter.add(TIP_SHADOW.as_shape(box_, radius));
        painter.rect_filled(box_, radius, pal.panel);
        painter.rect_stroke(
            box_,
            radius,
            Stroke::new(HAIRLINE, pal.line),
            StrokeKind::Inside,
        );
        painter.galley(
            box_.min + egui::vec2(TIP_PAD_X, TIP_PAD_Y),
            galley,
            pal.text,
        );

        // Draw interactive key learning / globalize bar at bottom
        let pill_y = box_.max.y - TIP_PAD_Y - key_pill_h;
        let pill_rect = egui::Rect::from_min_max(
            egui::pos2(box_.min.x + TIP_PAD_X, pill_y),
            egui::pos2(box_.max.x - TIP_PAD_X, pill_y + key_pill_h),
        );
        let global_w = 68.0;
        let badge_rect = egui::Rect::from_min_max(
            pill_rect.min,
            egui::pos2(pill_rect.max.x - global_w, pill_rect.max.y),
        );
        let global_rect = egui::Rect::from_min_max(
            egui::pos2(pill_rect.max.x - global_w + 4.0, pill_rect.min.y),
            pill_rect.max,
        );
        self.key_badge_rect = Some(badge_rect);
        self.key_global_rect = Some(global_rect);

        // Render badge button
        let pill_radius = CornerRadius::same(4);
        let badge_bg = if is_learning { pal.well } else { pal.ground };
        painter.rect_filled(badge_rect, pill_radius, badge_bg);
        painter.rect_stroke(
            badge_rect,
            pill_radius,
            Stroke::new(HAIRLINE, if is_learning { pal.sun } else { pal.line }),
            StrokeKind::Inside,
        );

        let badge_text = if is_learning {
            "● Press key (Esc)".to_string()
        } else if let Some(hk) = custom_hk.or_else(|| hotkey_for_tip(rest.on)) {
            format!("Key: [{hk}] ✎")
        } else {
            "Key: none +".to_string()
        };
        painter.text(
            badge_rect.center(),
            egui::Align2::CENTER_CENTER,
            badge_text,
            egui::FontId::new(TIP_SIZE - 1.5, egui::FontFamily::Proportional),
            if is_learning { pal.sun } else { pal.text },
        );

        // Render globalize toggle button
        painter.rect_filled(global_rect, pill_radius, pal.ground);
        painter.rect_stroke(
            global_rect,
            pill_radius,
            Stroke::new(
                HAIRLINE,
                if self.key_globalize {
                    pal.mint
                } else {
                    pal.line
                },
            ),
            StrokeKind::Inside,
        );
        let glob_text = if self.key_globalize {
            "[✓] Global"
        } else {
            "[ ] Global"
        };
        painter.text(
            global_rect.center(),
            egui::Align2::CENTER_CENTER,
            glob_text,
            egui::FontId::new(TIP_SIZE - 2.0, egui::FontFamily::Proportional),
            if self.key_globalize {
                pal.mint
            } else {
                pal.dim
            },
        );
    }
}
