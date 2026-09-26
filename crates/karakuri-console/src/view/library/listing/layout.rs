use egui::{Color32, FontFamily, FontId, Pos2, Rect};
use karakuri_operation::Operation;

use super::*;

// ---------------------------------------------------------------------------
// LibraryBay row layout and hit-testing
// ---------------------------------------------------------------------------

impl LibraryBay {
    /// Returns the bounding rectangle for row `index`, adjusted for current scroll offset (ADR-0307, ADR-0312).
    pub fn row(&self, index: usize) -> Rect {
        Rect::from_min_size(
            Pos2::new(
                self.list.min.x,
                self.list.min.y - self.scroll + size::LIB_ROW_H * index as f32 + self.pushed(index),
            ),
            egui::vec2(self.list.width(), size::LIB_ROW_H),
        )
    }

    /// Returns the range of row indices that overlap the visible listing area.
    pub fn drawn(&self) -> std::ops::Range<usize> {
        let touching = |index: usize| {
            let row = self.row(index);
            row.max.y > self.list.min.y && row.min.y < self.list.max.y
        };
        let first = (0..self.total).find(|index| touching(*index));
        match first {
            None => 0..0,
            // **From the first one that touches, and it is a run**: every row
            // after it either touches or is below the list, so the end is the
            // first that does not.
            Some(first) => {
                let end = (first..self.total)
                    .find(|index| !touching(*index))
                    .unwrap_or(self.total);
                first..end
            }
        }
    }

    /// Vertical offset added to rows below an open reading block (ADR-0312).
    fn pushed(&self, index: usize) -> f32 {
        match self.reading {
            Some(block) if index >= block.under => {
                size::READING_MARGIN_TOP + block.well.height() + size::READING_MARGIN_BOTTOM
            }
            _ => 0.0,
        }
    }

    /// Returns the bounding rectangle for the star icon in row `index`.
    pub fn star(&self, index: usize) -> Rect {
        let row = self.row(index);
        Rect::from_min_size(
            Pos2::new(
                row.min.x + size::LIB_ROW_PAD_X,
                row.center().y - STAR_SIZE * 0.5,
            ),
            egui::vec2(STAR_SIZE, STAR_SIZE),
        )
    }

    /// X-coordinate where item name text begins for row `index`.
    pub(crate) fn named(&self, index: usize) -> f32 {
        self.star(index).max.x + STAR_GAP
    }

    /// Returns an operation to toggle the star at `p`, or `None` if unhit (ADR-0156, ADR-0299, P-0090).
    pub fn starred(
        &self,
        rows: Rows<'_>,
        marks: &std::collections::BTreeSet<String>,
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        // **The star's own box, inside the list** — `at_row`'s bound stated on
        // the smaller rectangle: a star belongs to a row, and a row off the
        // top of the list has one.
        let row = self
            .drawn()
            .find(|index| self.list.contains(p) && self.star(*index).contains(p))?;
        let id = rows.set(row)?;
        Some(Operation::SetFavourite {
            id: id.to_owned(),
            favourite: !marks.contains(id),
        })
    }

    /// Returns the index of the drawn row containing `p`, or `None` if outside bounds (ADR-0307, ADR-0312).
    fn at_row(&self, p: Pos2) -> Option<usize> {
        if !self.list.contains(p) {
            return None;
        }
        self.drawn().find(|index| self.row(*index).contains(p))
    }

    /// What the foot reads: `n of m`, the mock's own `5 of 27`.
    pub fn count(&self) -> String {
        format!("{} of {}", self.rows, self.total)
    }

    /// Returns bounding box for a capsule of `width` anchored at the right side of the foot.
    pub fn pill(&self, width: f32) -> Rect {
        Rect::from_min_size(
            Pos2::new(
                self.foot.max.x - size::LIB_FOOT_PAD_X - width,
                self.foot.center().y - size::PILL_H * 0.5,
            ),
            egui::vec2(width, size::PILL_H),
        )
    }

    /// Computes layout rectangles for the footer load control and deck pulldown menu.
    pub fn load(&self, ctx: &egui::Context, at: Target) -> Load {
        let run = |text: &str| {
            if ctx.cumulative_pass_nr() == 0 {
                return egui::Vec2::ZERO;
            }
            ctx.fonts_mut(|f| {
                f.layout_no_wrap(
                    text.to_owned(),
                    FontId::new(size::BASE, FontFamily::Proportional),
                    Color32::PLACEHOLDER,
                )
                .size()
            })
        };
        let (word, mark) = (run(LOAD_PILL), run(at.letter()));
        let mid = self.foot.center().y;
        // **The pulldown**, and the gap inside it between the letter and the
        // chevron is the one gap the mock states inside a capsule — the same
        // `.sink` gap `arr · night ▾` puts between its words and its mark.
        let deck = self.pill(size::PILL_PAD_X * 2.0 + mark.x + size::SINK_GAP + CHEVRON_W);
        let letter = Rect::from_min_size(
            Pos2::new(deck.min.x + size::PILL_PAD_X, mid - mark.y * 0.5),
            mark,
        );
        let chevron = Rect::from_center_size(
            Pos2::new(deck.max.x - size::PILL_PAD_X - CHEVRON_W * 0.5, mid),
            egui::vec2(CHEVRON_W, CHEVRON_H),
        );
        // **The label, on the foot's own ground and on neither capsule.**
        let arrow = Rect::from_center_size(
            Pos2::new(deck.min.x - size::LIB_FOOT_GAP - LOAD_ARROW * 0.5, mid),
            egui::vec2(LOAD_ARROW, LOAD_ARROW),
        );
        let button = Rect::from_min_size(
            Pos2::new(
                arrow.min.x - size::LIB_FOOT_GAP - (word.x + size::PILL_PAD_X * 2.0),
                mid - size::PILL_H * 0.5,
            ),
            egui::vec2(word.x + size::PILL_PAD_X * 2.0, size::PILL_H),
        );
        Load {
            button,
            text: Rect::from_min_size(
                Pos2::new(button.min.x + size::PILL_PAD_X, mid - word.y * 0.5),
                word,
            ),
            arrow,
            deck,
            letter,
            chevron,
            // **Zero while it is shut**, which is what stops [`Load::row`]
            // handing out a rectangle for a list nobody opened.
            rows: match at.open {
                true => at.decks,
                false => 0,
            },
        }
    }

    /// Computes bounding box for the footer `params` chip.
    pub fn params_chip(&self, ctx: &egui::Context, at: Target) -> Rect {
        let load = self.load(ctx, at).button;
        let width = pill_width(ctx, PARAMS_PILL);
        Rect::from_min_size(
            Pos2::new(load.min.x - size::LIB_FOOT_GAP - width, load.min.y),
            egui::vec2(width, size::PILL_H),
        )
    }

    /// What a press at `p` on the `params` chip asks for, or `None` where there is
    /// no chip under it.
    ///
    /// Evaluates open/closed state against [`LibraryBay::reading`] to toggle reading display (ADR-0156).
    pub fn read(
        &self,
        ctx: &egui::Context,
        at: Target,
        set: Option<&str>,
        p: karakuri_layout::Point,
    ) -> Option<Read> {
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        if !self.params_chip(ctx, at).contains(Pos2::new(p.x, p.y)) {
            return None;
        }
        match self.reading {
            Some(_) => Some(Read::Shut),
            None => Some(Read::Open(Operation::ReadSet {
                id: set?.to_owned(),
            })),
        }
    }

    /// Returns the operation or action for a click at `p` in the footer load area (ADR-0338).
    pub fn aim(
        &self,
        ctx: &egui::Context,
        viewport: Rect,
        at: Target,
        rows: Rows<'_>,
        cursor: usize,
        p: karakuri_layout::Point,
    ) -> Option<Aim> {
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let load = self.load(ctx, at);
        if load.hit_deck(p) {
            return Some(match at.open {
                true => Aim::Shut,
                false => Aim::Open,
            });
        }
        if at.open {
            return Some(match load.picked(viewport, p) {
                Some(deck) => Aim::Deck(deck),
                None => Aim::Shut,
            });
        }
        if !load.hit_button(p) {
            return None;
        }
        // Dispatches LoadProcedure or LoadSet based on row type (ADR-0338).
        Some(match (rows.name(cursor), rows.procedure(cursor)) {
            (Some(name), true) => Aim::Load(Operation::LoadProcedure {
                deck: at.deck,
                procedure: name.to_owned(),
            }),
            (Some(id), false) => Aim::Load(Operation::LoadSet {
                deck: at.deck,
                set: id.to_owned(),
            }),
            (None, _) => Aim::NoSet,
        })
    }

    /// Computes layout for the row context menu popup, hanging downward from row bounds.
    pub fn menu(&self, ctx: &egui::Context, viewport: Rect, at: Menued) -> Option<RowMenu> {
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let row = self.row(at.row?);
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
        let loads = at.decks.min(DECKS);
        // **The send is measured only where it is drawn**, which is
        // [`Menued::sends`]: a card as wide as `Save as a kbset` with no such
        // item in it would be a menu whose width said what it holds and was
        // wrong.
        let widest = (0..loads)
            .map(|deck| width(&load_item(deck as u8)))
            .chain(at.sends.then(|| width(MENU_SAVE)))
            .fold(size::ROW_MENU_MIN_W, f32::max);
        let sent = match at.sends {
            true => size::LIB_ROW_H + size::ROW_MENU_RULE_H,
            false => 0.0,
        };
        let height = size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H * loads as f32 + sent;
        let card = held_inside(
            &viewport,
            row.min.x + size::ROW_MENU_INSET,
            row.max.y,
            widest + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0,
            height,
        );
        let band = card.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * loads as f32;
        Some(RowMenu {
            card,
            loads,
            rule: at.sends.then(|| {
                Rect::from_min_size(
                    Pos2::new(card.min.x + size::LIB_LIST_PAD, band),
                    egui::vec2(
                        card.width() - size::LIB_LIST_PAD * 2.0,
                        size::ROW_MENU_RULE_H,
                    ),
                )
            }),
            save: at.sends.then(|| {
                Rect::from_min_size(
                    Pos2::new(
                        card.min.x + size::LIB_LIST_PAD,
                        band + size::ROW_MENU_RULE_H,
                    ),
                    egui::vec2(card.width() - size::LIB_LIST_PAD * 2.0, size::LIB_ROW_H),
                )
            }),
        })
    }

    /// Handles clicks on the row menu or context triggers at `p`.
    pub fn menu_ask(
        &self,
        ctx: &egui::Context,
        viewport: Rect,
        at: Menued,
        rows: Rows<'_>,
        p: karakuri_layout::Point,
    ) -> Option<Picked> {
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let Some(row) = at.row else {
            let row = self.at_row(Pos2::new(p.x, p.y))?;
            rows.name(row)?;
            return Some(Picked::Open(row));
        };
        let Some(menu) = self.menu(ctx, viewport, at) else {
            return Some(Picked::Shut);
        };
        let Some(name) = rows.name(row) else {
            return Some(Picked::Shut);
        };
        Some(match menu.picked(p) {
            // Dispatches LoadProcedure or LoadSet to target deck (ADR-0338).
            Some(RowItem::Load(deck)) => Picked::Load(match rows.procedure(row) {
                true => Operation::LoadProcedure {
                    deck,
                    procedure: name.to_owned(),
                },
                false => Operation::LoadSet {
                    deck,
                    set: name.to_owned(),
                },
            }),
            // Procedure rows cannot be saved directly; dismisses on click (ADR-0338).
            Some(RowItem::Save) => match rows.set(row) {
                Some(id) => Picked::Send(Operation::TransferSet {
                    transfer: karakuri_operation::SetTransfer::Send { id: id.to_owned() },
                }),
                None => Picked::Shut,
            },
            None => Picked::Shut,
        })
    }

    /// What a press at `p` on the list takes in hand, or `None` if outside drawn rows.
    ///
    /// Identifies the selected row Set or procedure for drag-and-drop operations (ADR-0338).
    pub fn take(&self, rows: Rows<'_>, p: karakuri_layout::Point) -> Option<Taken> {
        self.at_row(Pos2::new(p.x, p.y)).and_then(|row| {
            Some(Taken {
                row,
                set: rows.name(row)?.to_owned(),
                procedure: rows.procedure(row),
            })
        })
    }

    /// Resolves a click on a history listing row to an [`Operation::RestoreProcedure`].
    pub fn land(
        &self,
        versions: &[String],
        at: Target,
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        let row = self.at_row(Pos2::new(p.x, p.y))?;
        Some(Operation::RestoreProcedure {
            deck: at.deck,
            revision: karakuri_operation::Revision::Picked(versions.get(row)?.clone()),
        })
    }

    /// Returns an iterator over badges and their bounding boxes for row `index`, aligned right to left.
    pub fn badges<'a>(
        &self,
        ctx: &'a egui::Context,
        index: usize,
        words: &'a [&'static str],
    ) -> impl Iterator<Item = (&'static str, Rect)> + 'a {
        let row = self.row(index);
        let widths: Vec<f32> = if ctx.cumulative_pass_nr() == 0 {
            words.iter().map(|_| size::BADGE_PAD_X * 2.0).collect()
        } else {
            words
                .iter()
                .map(|word| {
                    ctx.fonts_mut(|f| {
                        f.layout_no_wrap(
                            (*word).to_owned(),
                            FontId::new(size::BADGE_SIZE, FontFamily::Proportional),
                            Color32::PLACEHOLDER,
                        )
                        .size()
                        .x
                    }) + size::BADGE_PAD_X * 2.0
                })
                .collect()
        };
        let whole: f32 =
            widths.iter().sum::<f32>() + size::BADGE_GAP * widths.len().saturating_sub(1) as f32;
        let mut x = row.max.x - size::LIB_ROW_PAD_X - whole;
        let top = row.center().y - size::BADGE_H * 0.5;
        words
            .iter()
            .zip(widths)
            .map(move |(word, width)| {
                let box_ = Rect::from_min_size(Pos2::new(x, top), egui::vec2(width, size::BADGE_H));
                x += width + size::BADGE_GAP;
                (*word, box_)
            })
            .collect::<Vec<_>>()
            .into_iter()
    }
}
