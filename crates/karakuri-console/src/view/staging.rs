use super::*;

// ---------------------------------------------------------------------------
// The Staging lane
// ---------------------------------------------------------------------------

/// The word at the head of the Staging lane, in the source's own capitalisation
/// for [`Kind::Bay`]'s reason: the mock upper-cases in CSS, and that is done at
/// paint time so the word a reader searches for is the word in the source.
pub(super) const STAGING_TITLE: &str = "Staging";

/// Candidate status indicating build, verification, and compilation state (ADR-0310, ADR-0326).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Candidate build swapped into live Set and actively rendering on screen (ADR-0313).
    Landed,
    /// Candidate exceeded frame time budget; rendering frozen on last valid frame (ADR-0316).
    Overloaded,
    /// Event::Rejected — the build failed and nothing changed: the running Set is
    /// still running, with its `t` and its live count untouched, and the disk holds
    /// material that does not assemble.
    Refused,
    /// Source verification failed during checker analysis; no Set was built (ADR-0089).
    NotCompiled,
}

impl Stage {
    /// Returns the display status label for this candidate stage.
    pub fn word(self) -> &'static str {
        match self {
            Stage::Landed => "landed",
            Stage::Overloaded => "overloaded",
            Stage::Refused => "refused",
            Stage::NotCompiled => "did not compile",
        }
    }
}

/// Represents a staged candidate modification awaiting operator verdict (ADR-0326).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// Target deck slot index (0-based) associated with the candidate verdict.
    pub deck: usize,
    /// Node address of the slot modified by the build, or `None` if unspecified.
    pub at: Option<NodeAddress>,
    /// Display address string (e.g. `L4:0`) for the modified node, or empty if none.
    pub addr: String,
    /// Display procedure name or composite build label for the candidate.
    pub name: String,
    /// Whether it is on screen — [`Stage`].
    pub stage: Stage,
    /// Diagnostic error messages reported by the checker when compilation fails (P-0083).
    pub said: Vec<String>,
}

/// Lays out candidate rows in the Staging lane.
///
/// Implements ADR-0200, ADR-0313, ADR-0316, and ADR-0326. Returns `None` if
/// there are no outstanding candidates or insufficient vertical space.
pub fn staging(layout: &karakuri_layout::Layout, candidates: &[Candidate]) -> Option<StagingBay> {
    // Returns None if no candidates are present.
    if candidates.is_empty() {
        return None;
    }
    staging_box(
        to_egui(layout.rect(layout.find("staging")?)),
        candidates.len(),
    )
}

/// The Staging lane, laid out — see [`staging`] for what is drawn in it and for
/// the six things in the mock's lane and the page's row that are not.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StagingBay {
    /// `.stage-list`'s content box: the region under the bay head, inside
    /// [`size::STAGE_LIST_PAD_TOP`], [`size::STAGE_LIST_PAD_X`] and
    /// [`size::STAGE_LIST_PAD_BOTTOM`], where the rows are laid from the top with
    /// [`size::STAGE_GAP`] between them.
    pub list: Rect,
    /// How many rows are drawn, which is how many fit in [`list`](Self::list) —
    /// never more than [`total`](Self::total), and never zero, because a lane with
    /// no room for one row draws no list at all.
    pub rows: usize,
    /// Total candidate count supplied for this frame.
    pub total: usize,
}

impl StagingBay {
    /// Returns the bounding rectangle for candidate row at `index`.
    pub fn row(&self, index: usize) -> Rect {
        Rect::from_min_size(
            Pos2::new(
                self.list.min.x,
                self.list.min.y + (size::CAND_H + size::STAGE_GAP) * index as f32,
            ),
            egui::vec2(self.list.width(), size::CAND_H),
        )
    }

    /// Returns the rectangle for row `index`'s rollback capsule, or `None` if unavailable.
    pub fn back_capsule(
        &self,
        ctx: &egui::Context,
        candidates: &[Candidate],
        index: usize,
    ) -> Option<Rect> {
        // Fonts are not valid until `egui` has run a pass, exactly as in
        // [`keep_pill`] — and on the frame before the first one there is
        // nothing drawn here to press.
        if ctx.cumulative_pass_nr() == 0 || index >= self.rows {
            return None;
        }
        let candidate = candidates.get(index)?;
        candidate.at?;
        let row = self.row(index);
        let width = |text: &str, at: f32| {
            ctx.fonts_mut(|f| {
                f.layout_job(span_at(text, at, Color32::PLACEHOLDER))
                    .size()
                    .x
            })
        };
        let verdict = width(candidate.stage.word(), size::CAND_WHO_SIZE);
        let w = pill_width(ctx, BACK_LABEL);
        let capsule = Rect::from_min_size(
            Pos2::new(
                row.max.x - size::CAND_PAD_X - verdict - size::CAND_GAP - w,
                row.center().y - size::PILL_H * 0.5,
            ),
            egui::vec2(w, size::PILL_H),
        );
        // Ensure capsule does not overlap leading deck letter and address text.
        let letter = DECK_LETTERS.get(candidate.deck).copied().unwrap_or("?");
        let least = size::CAND_PAD_X
            + width(letter, size::BASE)
            + size::CAND_GAP
            + width(&candidate.addr, size::BASE)
            + size::CAND_GAP;
        (capsule.min.x >= row.min.x + least).then_some(capsule)
    }

    /// Returns the operation to restore a node's previous version from a back capsule press.
    pub fn back(
        &self,
        ctx: &egui::Context,
        candidates: &[Candidate],
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        let at = egui::pos2(p.x, p.y);
        (0..self.rows).find_map(|index| {
            let capsule = self.back_capsule(ctx, candidates, index)?;
            let node = candidates.get(index)?.at?;
            capsule.contains(at).then(|| Operation::RestoreProcedure {
                deck: candidates[index].deck as u8,
                revision: Revision::Previous(node),
            })
        })
    }

    /// Returns the operation to accept and keep the candidate under point `p`.
    pub fn keep(
        &self,
        ctx: &egui::Context,
        candidates: &[Candidate],
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        let at = egui::pos2(p.x, p.y);
        (0..self.rows).find_map(|index| {
            let candidate = candidates.get(index)?;
            let node = candidate.at?;
            if candidate.stage == Stage::Overloaded || !self.row(index).contains(at) {
                return None;
            }
            if self
                .back_capsule(ctx, candidates, index)
                .is_some_and(|capsule| capsule.contains(at))
            {
                return None;
            }
            Some(Operation::KeepCandidate {
                deck: candidate.deck as u8,
                node,
            })
        })
    }
}

/// Label for the candidate rollback capsule button (`back`).
const BACK_LABEL: &str = "back";

/// Computes visible capacity and row count within the available layout rectangle.
fn staging_box(region: Rect, total: usize) -> Option<StagingBay> {
    let list = Rect::from_min_max(
        Pos2::new(
            region.min.x + size::STAGE_LIST_PAD_X,
            region.min.y + size::HEAD_H + size::STAGE_LIST_PAD_TOP,
        ),
        Pos2::new(
            region.max.x - size::STAGE_LIST_PAD_X,
            region.max.y - size::STAGE_LIST_PAD_BOTTOM,
        ),
    );
    // Suppress layout if list width is smaller than horizontal padding.
    if list.width() <= 0.0 {
        return None;
    }
    let fits = ((list.height() + size::STAGE_GAP) / (size::CAND_H + size::STAGE_GAP))
        .floor()
        .max(0.0) as usize;
    let rows = fits.min(total);
    (rows > 0).then_some(StagingBay { list, rows, total })
}

/// Renders candidate rows into the Staging bay.
pub(super) fn staging_into(ui: &Ui, pal: &Palette, bay: &StagingBay, candidates: &[Candidate]) {
    let painter = ui.painter().with_clip_rect(bay.list);
    for (index, candidate) in candidates.iter().take(bay.rows).enumerate() {
        let row = bay.row(index);
        painter.rect_filled(row, size::CAND_RADIUS, pal.well);

        let letter = DECK_LETTERS.get(candidate.deck).copied().unwrap_or("?");
        let deck = painter.layout_job(span_at(letter, size::BASE, pal.faint));
        let after = deck.size().x;
        painter.galley(
            Pos2::new(
                row.min.x + size::CAND_PAD_X,
                row.center().y - deck.size().y * 0.5,
            ),
            deck,
            pal.faint,
        );

        // Node address in accent lavender (`.addr`), matching Inspector head.
        let addr = painter.layout_job(span_at(&candidate.addr, size::BASE, pal.lav));
        let addressed = addr.size().x;
        painter.galley(
            Pos2::new(
                row.min.x + size::CAND_PAD_X + after + size::CAND_GAP,
                row.center().y - addr.size().y * 0.5,
            ),
            addr,
            pal.lav,
        );
        let after = match candidate.addr.is_empty() {
            true => after,
            false => after + size::CAND_GAP + addressed,
        };

        let name = painter.layout_job(span_at(&candidate.name, size::BASE, pal.dim));
        let named = name.size().x;
        painter.galley(
            Pos2::new(
                row.min.x + size::CAND_PAD_X + after + size::CAND_GAP,
                row.center().y - name.size().y * 0.5,
            ),
            name,
            pal.dim,
        );

        // Formatted checker diagnostics for uncompiled candidates (Candidate::said).
        if let Some(first) = candidate.said.first() {
            let rest = candidate.said.len() - 1;
            let text = match rest {
                0 => first.clone(),
                1 => format!("{first} · 1 more"),
                more => format!("{first} · {more} more"),
            };
            let said = painter.layout_job(span_at(&text, size::CAND_WHO_SIZE, pal.faint));
            painter.galley(
                Pos2::new(
                    row.min.x + size::CAND_PAD_X + after + size::CAND_GAP + named + size::CAND_GAP,
                    row.center().y - said.size().y * 0.5,
                ),
                said,
                pal.faint,
            );
        }

        let word = painter.layout_job(span_at(
            candidate.stage.word(),
            size::CAND_WHO_SIZE,
            pal.faint,
        ));
        painter.galley(
            Pos2::new(
                row.max.x - size::CAND_PAD_X - word.size().x,
                row.center().y - word.size().y * 0.5,
            ),
            word,
            pal.faint,
        );

        // Rollback capsule rendered using back_capsule layout.
        if let Some(capsule) = bay.back_capsule(ui.ctx(), candidates, index) {
            pill_at(ui, pal, capsule, BACK_LABEL);
        }
    }
}
