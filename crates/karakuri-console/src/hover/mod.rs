//! The console paints its own hover layer, and the words in it are the
//! manual's own.
//!
//! # Who owns the pointer
//!
//! `docs/roadmap.md`'s M5.11 waited on one decision — *whether the panel gains
//! `egui` widgets, or paints its own hover layer* — and
//! [`crate::input`] named it as its own and deliberately did not take it.
//! [ADR-0330](../../../docs/adr/0330-the-console-paints-its-own-hover-layer-and-the-tips-are-the-manuals-own-words.md)
//! takes it: the console paints its own layer, because the mechanism is
//! already here and the alternative buys nothing this crate does not have.
//!
//! - [`crate::input::PROBES`] already answers *what is under the pointer*, one
//!   derivation per row, for every control the panel draws. A tooltip is that
//!   question asked on a rest instead of on a press
//!   ([P-0085](../../../docs/principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md)).
//! - [ADR-0156](../../../docs/adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)'s
//!   seam is that the console paints and takes no device, and that `egui`
//!   receives a rectangle rather than the arrangement. An `egui` widget under
//!   every control would hand hit testing and layout to a second system for
//!   this one feature, and `claim`'s rule 4 — *a press routed to `egui` there
//!   reaches nothing at all* — would have to become two rules.
//!
//! # The words are the page's, and only the key is written down here
//!
//! `docs/manual/console.html` is the only copy of a tip. The page is
//! [`PAGE`], embedded at compile time, and [`Tips::read`] parses the
//! `data-tip` attributes out of it once, at start-up, off the frame path
//! (P-0091). Nothing here restates a word of one, so there is no second copy
//! to drift ([`docs/contributing.md` §4](../../../docs/contributing.md), which
//! is *generated* rather than *tested*).
//!
//! What cannot be generated is the key: which drawn control a tip belongs
//! to. The mock carries no identity for a control — no `id`, no
//! `data-control`, and its classes repeat (twenty tipped elements are a bare
//! `.pill`) — so *which element is the tone map's capsule* is not derivable
//! from the page by any rule that survives an edit to it. That much is
//! transcribed, and it is transcribed as a citation and not as prose: a
//! [`Cite`] is the element's class and the words inside it, which is the
//! smallest thing that names one element of the mock. [`TIPS`] is that
//! transcription and `tests/hover.rs` is what holds it to the page, in
//! `tests/transcribed_constants_cite_the_mock.rs`'s pattern — a cite that
//! resolves to no element, or to more than the one it says, fails there.
//!
//! # The table is the console's own rows
//!
//! [`TIPS`] is `[(&str, &[Tipped]); PROBES.len()]`, one entry per row of
//! [`crate::input::PROBES`] and in that crate's own order, which is
//! `karakuri/src/main.rs`'s `ASKED` shape one crate over and for its reason:
//! a control added to that table arrives here as a compile error. A row
//! whose controls the mock draws with no tip carries an empty slice, which is
//! the roadmap's own reading rule — *the mock is not exhaustive … there will
//! be gaps in the functions too* — said as a value rather than as a silence.
//!
//! Where a row claims several controls the slice has one entry each, asked
//! in the caller's order: the controls inside a container first and the
//! container last, which is `claim`'s rule 4 (*a control claims what it acts
//! on and no more*) and `main.rs`'s press order. [`resolve`] takes the first
//! that answers, so the order is what makes a tip on a strip's fader the
//! fader's rather than the strip's.
//!
//! # What it costs
//!
//! - 340 KB of page in the binary, and one parse of it at start-up. The
//!   alternative was ~60 long strings transcribed by hand and held equal by a
//!   test, which is a second copy of the manual's prose kept in a source file:
//!   cheaper to run and dearer to keep true, and the page is the half that
//!   moves.
//! - One walk of [`TIPS`] per pointer move that [`crate::input::claim`]
//!   answered `Panel` to, which is the walk that rule 4 already makes,
//!   asked a second time to say *which*. A move `claim` gave to `egui` is on
//!   no control at all and costs one comparison.
//! - Nothing on the frame after the first. The galley is laid out on the
//!   frame the tip appears and kept while the pointer stays on that control;
//!   every frame after it is one cached galley and four shapes.
//! - No frame at rest. [`Hover::owed`] answers a deadline while a dwell is
//!   running and nothing at all once the tip is up — the tip does not move
//!   while it is shown, so its picture is not different from the one on
//!   screen. That is
//!   [ADR-0283](../../../docs/adr/0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)'s
//!   `moves_in` on a region whose motion is a hand holding still.
//!
//! # Modal overlay suppression
//!
//! While a modal overlay card, menu, or chooser is down
//! ([`View::has_modal_overlay`]), this layer rests on nothing: [`resolve`] answers
//! `None` for a pointer move made under one, and [`Hover::paint`] forgets the rest
//! it was holding — exactly as [`Hover::left`] does — so a tip that is up goes, a
//! dwell that is running is abandoned, nothing is owed, and the words return only
//! once the card is gone *and* the pointer rests on the control again.
//!
//! It is asked in both places because a card comes down two ways. A press that
//! opens one is a move, and [`resolve`] is asked on moves; a key, or a press that
//! moved the pointer nowhere, is no move at all, and the frame the card is drawn on
//! is where the layer hears about it. [`View::has_modal_overlay`] is the only
//! derivation of *is a card down* on either path.

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

/// The console's hover layer: which control the pointer is resting on, since
/// when, and the one galley the tip is drawn from.
///
/// It holds no clock. Every time in it arrives as a [`Duration`] from the
/// caller, which is this crate's rule and `crate::view::Transport`'s argument
/// for it: *"an `Instant` here would put a clock in it"*.
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

    /// Where the pointer is resting and on which control, or `None` where it is on
    /// none — the point, and an index into [`flat`].
    ///
    /// This is the learn seam. Learn is *point at a control and move a knob*, so
    /// what it needs is exactly what this layer already worked out for the tip:
    /// which control the pointer is on. A second derivation in the host would be a
    /// second answer that could disagree with the tip the operator is reading while
    /// they do it
    /// (`docs/adr/0336-a-learn-is-a-map-edit-and-the-tips-midi-line-is-the-live-map.md`).
    ///
    /// It answers before the dwell. A tip waits half a second because reading one
    /// is a decision; pointing at a control and reaching for a knob is not, and a
    /// learn that only worked once the box was up would be a gesture with a hidden
    /// timer in it.
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

    /// Which knob the control under the pointer is on, for the tip's last line —
    /// `Some("cc 5")`, or `None` for a control nothing is mapped to.
    ///
    /// # Why this crosses the seam instead of being derived here
    ///
    /// A control's assignment is a fact the map holds, and this crate cannot read
    /// one: `karakuri-midi` depends on `midir`, and
    /// [ADR-0156](../../../docs/adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)
    /// is that this crate takes no device. So it arrives the way every other value
    /// does — derived by the host and handed in, beside the [`View`].
    ///
    /// # And why it is derived at all
    ///
    /// The page's own `⊕ MIDI:` line is the mock's assignment and no operator's. A
    /// run whose map puts `cc 5` on gain A read `cc → gain A` in the tip because
    /// the page said so, and the tip was then confidently wrong about the one thing
    /// somebody would hover to check
    /// ([P-0087](../../../docs/principles/0087-name-the-property-never-the-shape.md)).
    /// Where nothing is mapped the page's sentence stays, because it says *why* —
    /// *a map line names a slot, a range or a word from a closed list, and a Set id
    /// is none of the three* — and no reverse lookup can produce that.
    ///
    /// Set once per frame by the host; `None` on a run with no surface at all,
    /// which leaves every tip exactly as the page wrote it.
    pub fn assign(&mut self, on: Option<String>) {
        self.assigned = on;
    }

    /// How long a pointer holds still before a tip appears, and it is `egui`'s own
    /// `Interaction::tooltip_delay` rather than a number written here.
    ///
    /// Read rather than transcribed, which is [`docs/contributing.md`
    /// §4](../../../docs/contributing.md)'s first tier over a number this program
    /// would otherwise have invented: the toolkit this console is drawn with
    /// already carries the interval a tooltip waits for, so the console paints its
    /// own layer and still waits as long as everything else the operator's machine
    /// draws. It is 0.5 s in `egui` 0.36's default style, and a caller that changes
    /// the style moves this with it.
    pub fn dwell(ctx: &egui::Context) -> Duration {
        let style = ctx.style_of(ctx.theme());
        Duration::from_secs_f32(style.interaction.tooltip_delay.max(0.0))
    }

    /// The pointer moved. Returns what is owed for it — [`Tip::Gone`] where a tip
    /// is on screen and the pointer has left the control it belongs to, and
    /// [`Tip::Still`] otherwise, because a move that starts a dwell has already
    /// earned a frame from [`crate::repaint::Change::Pointer`] and [`Hover::owed`]
    /// is what that frame asks for the deadline.
    ///
    /// The claim goes in with the point. `crate::input::claim` has already walked
    /// the console's controls to answer it, and `Claim::Egui` is the panel saying
    /// the pointer is on none of them — so the common move, which is over nothing,
    /// costs one comparison here and the walk below happens only where a control
    /// really is under the pointer.
    ///
    /// A drag is not a hover. A gesture in hand is the pointer being used rather
    /// than pointed, so it takes any tip down and starts none: that is `claim`'s
    /// rule 1 read on this layer rather than a second copy of it.
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

    /// When this layer's picture is next different from the one on screen, asked on
    /// every frame the way `crate::view::View::animating` is.
    ///
    /// [`Tip::Dwelling`] while a dwell is running and [`Tip::Still`] everywhere
    /// else — including once the tip is up, because it does not move while it is
    /// shown. A panel nobody is pointing at asks for nothing.
    ///
    /// A dwell with nothing left of it is `Still` and not `Dwelling(0)`, which is
    /// what makes this *when is the picture next different from the one this frame
    /// is about to draw* rather than *from the one already on screen*: the caller
    /// asks this inside the pass that is about to paint, so the frame the deadline
    /// was for is the frame it is being asked on, and a zero here would buy one
    /// more frame drawing what this one drew.
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

    /// Paint the tip where one is up, and paint nothing otherwise.
    ///
    /// The panel paints it last, after every other bay, which is why the tip's
    /// box goes over every card and every bay — which is the mock's `z-index: 30`
    /// and the order the four cards are already painted in.
    ///
    /// Nothing is allocated after the first frame it is up: the galley is laid out
    /// once for the control the pointer is on and kept until the pointer leaves it.
    ///
    /// **A frame with a modal overlay down rests on nothing.** The layer forgets
    /// where the pointer was resting, exactly as [`Hover::left`] does, so a tip
    /// that is up goes on the frame the card is drawn on, a dwell that is running
    /// is abandoned, and the words come back only once the overlay is gone *and*
    /// the pointer rests on the control again — a fresh dwell and not the old one.
    /// This is the frame's own reading of [`View::has_modal_overlay`], which is the
    /// only derivation of *is a card down*: [`resolve`] answers `None` for a move
    /// made under one, and a card put down by a key or by a press that moved the
    /// pointer nowhere reaches this layer through no move at all.
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
