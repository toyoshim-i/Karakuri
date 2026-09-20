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
    /// Which deck the keys are addressed to — the Mixer bay's remembered address,
    /// read as a deck.
    ///
    /// [`Operation::SelectDeck`] *"writes no record, and is the reason every other
    /// variant names its deck instead of meaning the selected one"* — so nothing
    /// downstream can be the model of record for it, and a host that kept a copy
    /// would be keeping the console's state on its behalf.
    /// [ADR-0219](../../../../docs/adr/0219-the-crossfader-spans-the-selection-and-the-one-after-it.md)
    /// recorded it as living *"in the specification and not in `karakuri-console`'s
    /// code"*; [`View::focus`] the field is where that stopped being true, and
    /// ADR-0259 is why it is the same field the library cursor and the marked scope
    /// are in.
    ///
    /// Deck A for a bay nobody has addressed, which is the strip the mock rings.
    /// The digit that names the first strip is `1` — digits count what the bay drew
    /// — and this is the one place the step down to a deck index is written.
    ///
    /// [`View::focus`]: Self::focus
    pub fn selection(&self) -> u8 {
        self.focus
            .address(focus::MIXER)
            .and_then(|address| address.remembered(&[]))
            .map_or(0, |nth| nth.saturating_sub(1) as u8)
    }

    /// Address the keys to `deck`, and answer whether that moved anything.
    ///
    /// A deck the mixer has no strip for is refused, and that is the whole of the
    /// rule: the selection is drawn as a ring round a strip and is what every
    /// deck-addressed key names its deck by, so a selection past the deck's slots
    /// would be a ring nowhere and a key aimed at a deck the press would be turned
    /// down on. [`View::aim_at`] refuses on the same count one mark along.
    /// [`View::mixer`] is *"one per slot the deck has"*, so its length is the
    /// deck's own count arriving the way every other reading does — and a console
    /// with no deck behind it has no strip to select, which is every test in this
    /// crate.
    ///
    /// It refuses rather than clamping. A press on `3` at a two-slot deck means
    /// *deck D* and there is no deck D; clamping would answer *deck B*, which is a
    /// different deck than the one asked for and would move the mix under a hand
    /// that asked for nothing of the sort.
    ///
    /// The `bool` is [`Arrangement::typed`]'s: a caller repaints on a move and not
    /// on a press, so a press that changed nothing costs no frame (P-0091).
    pub fn select(&mut self, deck: u8) -> bool {
        if usize::from(deck) >= self.mixer.len() {
            return false;
        }
        let moved = self.selection() != deck;
        // **The Mixer bay's remembered item, one-based**, which is the digit
        // that names the strip: `1` is deck A. See [`crate::focus::Address`].
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

    /// Where focus is and what every bay remembers, to read — [`View::focus`] the
    /// field, which is where the argument is.
    ///
    /// A reading and not a door. It is `&`, so the three pointers still have
    /// exactly the writers they had: `select`, `walk`, `point_at`, `select_scope`
    /// and `step_scope` are what refuse a deck the mixer draws no strip for, a row
    /// past the listing and a scope with no chip, and a `&mut` here would be a way
    /// past all five. What it is for is asking one question — *are these the same
    /// field* — which `tests/focus.rs` asks and nothing else can.
    ///
    /// [`View::focus`]: Self::focus
    pub fn focus(&self) -> &Focus {
        &self.focus
    }

    /// Where focus is and what every bay remembers, to write — and it is
    /// `pub(crate)` where [`View::focus`] above is `pub`.
    ///
    /// What it is for is the address path and not the memory.
    /// [`crate::focus::press`] moves `Address::at` as a digit descends and `esc`
    /// climbs, and every write to the *remembered* item still goes through
    /// [`View::select`], [`View::walk`], [`View::point_at`], [`View::select_scope`]
    /// and [`View::step_scope`] — the five methods that refuse a deck the mixer
    /// draws no strip for, a row past the listing and a scope with no chip. The
    /// grammar adds a route and no exception, which is the whole of why this door
    /// is the crate's and not the world's.
    pub(crate) fn focus_mut(&mut self) -> &mut Focus {
        &mut self.focus
    }

    /// Which bay a key press is addressed to, resolved against the arrangement —
    /// [`Focus::bay`], which is where the argument is.
    ///
    /// `None` only for an arrangement with no bay in it, which no arrangement this
    /// crate builds is.
    pub fn focused(&self, panel: &Panel) -> Option<&'static Region> {
        self.focus.bay(panel.layout())
    }

    /// `Tab`, and `shift-Tab` at `step` of `-1` — [`Focus::tab`], and the `bool` is
    /// [`View::select`]'s: a caller repaints on a move.
    ///
    /// The bay it leaves keeps where it was. `Tab` never descends and never pops,
    /// so the address the Mixer had reached is the address it has when focus comes
    /// back to it — which is the deck selection persisting *"while your hands are
    /// in the library"*, seen as the general rule rather than as a habit of one
    /// bay.
    ///
    /// It takes every card the address descends into away, wherever the address
    /// is: the Transport's two
    /// ([ADR-0350](../../../../docs/adr/0350-the-transports-two-cards-are-walked-and-the-tempo-figure-steps-by-a-beat-a-minute.md)),
    /// the Sequencer's `+ lane`
    /// ([ADR-0351](../../../../docs/adr/0351-the-lane-chooser-is-a-rung-of-the-address.md))
    /// and the Master's `+ add`
    /// ([ADR-0352](../../../../docs/adr/0352-the-chains-list-is-the-master-bays-items-and-a-slot-is-taken-out-by-a-glyph-on-its-row.md)).
    /// The cards the address does not walk are left alone.
    pub fn tab(&mut self, panel: &Panel, step: i32) -> bool {
        let moved = self.focus.tab(panel.layout(), step);
        if moved {
            focus::shut_cards(self);
        }
        moved
    }

    /// `esc`: up one level of the focused bay's address, and `false` where there
    /// was no level to leave — [`Focus::up`].
    ///
    /// At bay level it acts on nothing and the caller says so, which is ADR-0259's
    /// own clause and
    /// [P-0083](../../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md):
    /// there is no unfocused state to fall out into, and a key that declines
    /// silently is indistinguishable from one that is not bound. It does not quit —
    /// the window's own close is what does
    /// ([ADR-0315](../../../../docs/adr/0315-a-model-has-no-window-so-the-twelve-surface-rows-mcp-badges-are-gap.md)).
    ///
    /// A card on the address's path is a level of the address, so `esc` takes it
    /// away and the address ends on the control it hangs from — one level up where
    /// the address had descended into the card, and where it already was
    /// otherwise. A card that is down anywhere else is left alone and `esc` is the
    /// ordinary climb. [`focus::card_on_path`] is that reading, and it is one
    /// reading for all four cards
    /// ([ADR-0332](../../../../docs/adr/0332-focus-is-a-pointer-the-console-owns-and-the-three-pointers-are-instances-of-it.md)).
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
    /// Where the dashed focus ring goes, or `None` where the arrangement gives the
    /// focused bay no rectangle to put one on — [`focus::mark`], with the bay this
    /// console has focus on.
    ///
    /// A derivation rather than a paint, which is this crate's arrangement
    /// everywhere a mark is drawn: [`View::draw`] paints from this and
    /// `tests/focus.rs` asks it the same question, so the ring an operator sees and
    /// the ring a test reads cannot come apart.
    ///
    /// `panel` must be solved: [`Layout::rect`] refuses to answer from a dirty one.
    pub fn focus_mark(&self, panel: &Panel) -> Option<Rect> {
        focus::mark(panel.layout(), self.focus.bay(panel.layout())?)
    }

    /// The mark a *folded* bay holding focus wears, or `None` where the focused bay
    /// is not folded — [`focus::folded_head`], with the bay this console has focus
    /// on, and the title to paint in it.
    ///
    /// A folded region has no rectangle, so the ring alone would land on nothing an
    /// operator could read; `docs/manual/console.html` specifies the head alone,
    /// and this is where the panel draws it. It is [`View::draw`]'s one mark
    /// painted over the bays rather than inside one, for the reason the five cards
    /// are: the edge a fold leaves belongs to whatever is drawn next to it, and
    /// this is only ever drawn while an operator has deliberately tabbed onto the
    /// bay that is not there.
    ///
    /// `panel` must be solved: [`Layout::rect`] refuses to answer from a dirty one.
    pub fn folded_mark(&self, panel: &Panel) -> Option<(Rect, &'static str)> {
        let bay = self.focus.bay(panel.layout())?;
        let mark = focus::folded_head(panel.layout(), bay)?;
        // A headless row has no title to draw, and the mark is what says a bay
        // is there — so the word alone stands for it, exactly as the row
        // stands in for the head that is not drawn (ADR-0159, ADR-0259).
        Some((mark, head_of(bay).map_or("", |head| head.title)))
    }
}
