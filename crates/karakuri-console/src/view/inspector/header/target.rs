use super::super::*;
use super::deck_name::*;

/// The pulldown on a pane head for aiming the pane at another deck (P-0090).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaneTarget {
    /// The `▾` after the run — [`DeckName::chevron`], made live. A press on it puts
    /// the card down; a press on it while the card is down is the host's to read as
    /// *shut it*, which is [`Load`]'s arrangement.
    pub chevron: Rect,
    /// Which pane this head belongs to, as an index into [`PANE_NAMES`] — what
    /// [`Operation::PointPane`]'s `pane` is spelled from, and what says which of
    /// [`View::pane_deck`]'s entries a pick moves.
    pub pane: usize,
    /// How many decks the card offers, which is how many strips the mixer is
    /// drawing while it is down and zero while it is shut — [`Load::rows`]' shape
    /// and its reason: [`PaneTarget::row`] cannot hand out a rectangle for a card
    /// nobody opened.
    pub rows: usize,
}

impl PaneTarget {
    /// Whether `p` is on the mark, which is the whole of what the shut control
    /// owns: the run to its left is [`DeckName`]'s and the count to its right is a
    /// readout.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.chevron.contains(Pos2::new(p.x, p.y))
    }

    /// The card under the mark, or `None` while it is shut — and `None` for a
    /// console with no strip to offer, which is every test in this crate that hands
    /// no mixer in.
    pub fn list(&self, viewport: Rect) -> Option<Rect> {
        if self.rows == 0 {
            return None;
        }
        let height = size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H * self.rows as f32;
        let width = size::LIB_ROW_H + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0;
        Some(held_inside(
            &viewport,
            self.chevron.min.x,
            self.chevron.max.y + size::PILL_GAP,
            width,
            height,
        ))
    }

    /// Returns the bounding rectangle for row `index` in the deck target card.
    pub fn row(&self, card: Rect, index: usize) -> Rect {
        assert!(index < self.rows, "deck {index} of a list of {}", self.rows);
        Rect::from_min_size(
            Pos2::new(
                card.min.x + size::LIB_LIST_PAD,
                card.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * index as f32,
            ),
            egui::vec2(card.width() - size::LIB_LIST_PAD * 2.0, size::LIB_ROW_H),
        )
    }

    /// Returns which deck is selected by a press at `p` on the card, or `None`.
    pub fn picked(&self, viewport: Rect, p: karakuri_layout::Point) -> Option<Operation> {
        let card = self.list(viewport)?;
        let at = Pos2::new(p.x, p.y);
        let deck = (0..self.rows).find(|index| self.row(card, *index).contains(at))?;
        Some(Operation::PointPane {
            pane: PANE_NAMES.get(self.pane)?.to_string(),
            deck: deck as u8,
        })
    }
}

/// Derives the pane target dropdown chevron and optional selection card.
#[derive(Clone, Copy)]
pub struct PaneTargetCtx<'a> {
    pub at: &'a InspectorPane,
    pub pane: &'a Pane,
    pub index: usize,
    pub naming: Option<&'a str>,
    pub decks: usize,
    pub open: bool,
    pub mcp: Option<Rect>,
}

/// refusal: the mark sits one gap after the run, so a head too narrow to paint
/// any of the name has nowhere to put it. The run is clipped short of this mark
/// rather than over it — see [`deck_name`], where that is one line.
pub fn pane_target(ctx: &egui::Context, target_ctx: PaneTargetCtx<'_>) -> Option<PaneTarget> {
    let PaneTargetCtx {
        at,
        pane,
        index,
        naming,
        decks,
        open,
        mcp,
    } = target_ctx;
    let named = deck_name(ctx, at, pane, naming, mcp)?;
    Some(PaneTarget {
        chevron: named.chevron,
        pane: index,
        // Rows count is zero while dropdown card is closed.
        rows: match open {
            true => decks.min(DECK_LETTERS.len()),
            false => 0,
        },
    })
}

/// Paints the pane target deck selection card, highlighting the currently displayed deck.
pub fn pane_list_into(ui: &Ui, pal: &Palette, target: &PaneTarget, showing: usize, card: Rect) {
    let painter = ui.painter();
    popup_card(painter, pal, card);
    // `take` rather than a range, because the rows are the letters — see
    // [`deck_list_into`], and `View::point_pane` is what stops a deck this
    // crate has no letter for being asked for.
    for (index, letter) in DECK_LETTERS.iter().enumerate().take(target.rows) {
        let row = target.row(card, index);
        let ink = match index == showing {
            true => pal.text,
            false => pal.dim,
        };
        let galley = painter.layout_no_wrap(
            (*letter).to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            ink,
        );
        painter.galley(
            Pos2::new(
                row.min.x + size::LIB_ROW_PAD_X,
                row.center().y - galley.size().y * 0.5,
            ),
            galley,
            ink,
        );
    }
}
