use egui::Rect;

use super::*;
use crate::focus::{self, Focus};
use crate::panel::Panel;
use crate::view::regions::Region;
use crate::view::widgets::head_of;

// ---------------------------------------------------------------------------
// View navigation, selection, and focus
// ---------------------------------------------------------------------------

impl View {
    /// Returns the currently addressed deck index (0-based) from Mixer focus memory.
    ///
    /// Defaults to deck 0 (deck A) if unaddressed (ADR-0219, ADR-0259).
    pub fn selection(&self) -> u8 {
        self.focus
            .address(focus::MIXER)
            .and_then(|address| address.remembered(&[]))
            .map_or(0, |nth| nth.saturating_sub(1) as u8)
    }

    /// Addresses keyboard controls to `deck`.
    ///
    /// Refuses out-of-range decks without clamping to avoid unintended moves.
    /// Returns `true` if the selection changed, avoiding no-op repaints (P-0091).
    pub fn select(&mut self, deck: u8) -> bool {
        if usize::from(deck) >= self.mixer.len() {
            return false;
        }
        let moved = self.selection() != deck;
        // Store one-based selection index in focus address.
        self.focus
            .address_mut(focus::MIXER)
            .remember(&[], usize::from(deck) + 1);
        moved
    }

    /// Mark the mixer bay dirty, notifying the console that mixer state has been arbitrated
    /// and the mixer bay requires a redraw on the next frame.
    pub fn mark_mixer_dirty(&mut self) {
        self.mixer_dirty = true;
    }

    /// Returns an immutable reference to the focus and bay navigation state.
    pub fn focus(&self) -> &Focus {
        &self.focus
    }

    /// Returns a mutable reference to focus state for internal console navigation.
    pub(crate) fn focus_mut(&mut self) -> &mut Focus {
        &mut self.focus
    }

    /// Resolves the currently focused bay region in the arrangement layout.
    pub fn focused(&self, panel: &Panel) -> Option<&'static Region> {
        self.focus.bay(panel.layout())
    }

    /// Moves focus by `step` (+1 forward, -1 backward).
    ///
    /// Preserves per-bay remembered addresses while dismissing cards on the
    /// focus path (ADR-0350, ADR-0351, ADR-0352). Returns `true` if focus moved.
    pub fn tab(&mut self, panel: &Panel, step: i32) -> bool {
        let moved = self.focus.tab(panel.layout(), step);
        if moved {
            focus::shut_cards(self);
            self.prompt.set_captured(false);
        }
        moved
    }

    /// Focuses `bay`, closing any open cards on movement. Returns `true` if moved.
    pub fn focus_bay(&mut self, panel: &Panel, bay: &str) -> bool {
        let moved = self.focus.put(panel.layout(), bay);
        if moved {
            focus::shut_cards(self);
            if bay != "prompt" {
                self.prompt.set_captured(false);
            }
        }
        moved
    }

    /// Navigates up one focus level (`esc`), dismissing active path cards (ADR-0332).
    ///
    /// Returns `false` if already at bay level without unfocusing (ADR-0259, P-0083).
    pub fn focus_up(&mut self, panel: &Panel) -> bool {
        if let Some((control, inside)) = focus::card_on_path(self, panel) {
            focus::shut_card(self, control);
            if inside {
                self.focus.up(panel.layout());
            }
            return true;
        }
        self.focus.up(panel.layout())
    }

    /// Returns the focus ring rectangle for the focused bay, or `None` if absent.
    /// Requires a solved layout.
    pub fn focus_mark(&self, panel: &Panel) -> Option<Rect> {
        focus::mark(panel.layout(), self.focus.bay(panel.layout())?)
    }

    /// Returns the mark rectangle and title when the focused bay is folded (ADR-0159, ADR-0259).
    /// Requires a solved layout.
    pub fn folded_mark(&self, panel: &Panel) -> Option<(Rect, &'static str)> {
        let bay = self.focus.bay(panel.layout())?;
        let mark = focus::folded_head(panel.layout(), bay)?;
        // A headless row has no title to draw, and the mark is what says a bay
        // is there — so the word alone stands for it, exactly as the row
        // stands in for the head that is not drawn (ADR-0159, ADR-0259).
        Some((mark, head_of(bay).map_or("", |head| head.title)))
    }
}
