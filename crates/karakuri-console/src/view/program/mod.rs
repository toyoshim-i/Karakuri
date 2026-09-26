use super::*;

// ---------------------------------------------------------------------------
// The Program bay
// ---------------------------------------------------------------------------

/// A rendered picture texture and destination rectangle in the Program bay.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Picture {
    /// The registered texture. Whatever it holds is drawn as-is: the console tints
    /// it with nothing.
    pub id: egui::TextureId,
    /// Where to draw it, in the same logical pixels the arrangement is stated in —
    /// [`picture_rect`]'s answer for the frame this is being drawn on.
    pub rect: Rect,
}

/// Returns the fitted picture rectangle for `canvas` within the Program bay,
/// or `None` if folded or if insufficient room exists (ADR-0182).
///
/// Requires `layout` to be cleanly solved before invocation.
pub fn picture_rect(layout: &karakuri_layout::Layout, canvas: (u32, u32)) -> Option<Rect> {
    program_bay(layout, canvas)?.picture
}

/// Computes the largest rectangle of `aspect` fitting within `inside`, centered per [ADR-0170](../../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md).
fn fitted(inside: Rect, aspect: (u32, u32)) -> Rect {
    let (aw, ah) = (aspect.0.max(1) as f32, aspect.1.max(1) as f32);
    let scale = (inside.width() / aw).min(inside.height() / ah);
    let w = (aw * scale).round().min(inside.width().floor());
    let h = (ah * scale).round().min(inside.height().floor());
    Rect::from_min_size(
        Pos2::new(
            inside.min.x + (inside.width() - w) * 0.5,
            inside.min.y + (inside.height() - h) * 0.5,
        ),
        egui::vec2(w, h),
    )
}

/// Aspect ratio of a deck preview cell (16:9) per CSS grid specification (ADR-0170, ADR-0182).
const PREVIEW_ASPECT: (u32, u32) = (16, 9);

/// Default aspect ratio (16:9) when no rendering canvas has been configured.
pub const MOCK_CANVAS: (u32, u32) = (16, 9);

/// Returns the four deck preview cell rectangles, or `None` if the preview row is collapsed or hidden.
pub fn preview_rects(
    layout: &karakuri_layout::Layout,
    canvas: (u32, u32),
) -> Option<[Rect; DECKS]> {
    program_bay(layout, canvas)?.cells
}

/// Calculates the four cells within a `deck-previews` region, or `None` if insufficient room.
fn preview_cells(region: Rect) -> Option<[Rect; DECKS]> {
    let pad = size::PROGRAM_BODY_PAD;
    let row = Rect::from_min_max(
        Pos2::new(region.min.x + pad, region.min.y),
        Pos2::new(region.max.x - pad, region.max.y - pad),
    );
    let cells = preview_row(row);
    // The same rule `picture_rect` states, and stated on the cell rather
    // than on the region because the cell is what gets drawn. Every cell is
    // the same size, so the first one answers for all four.
    match positive(cells[0]) {
        true => Some(cells),
        false => None,
    }
}

/// Lays out [`DECKS`] cells side by side across `row`, each maintaining [`PREVIEW_ASPECT`].
fn preview_row(row: Rect) -> [Rect; DECKS] {
    // Fits each preview cell into its track maintaining PREVIEW_ASPECT.
    std::array::from_fn(|deck| {
        fitted(
            above_caption(track(row, DECKS, deck, size::PREVIEW_GAP, Axis::Row)),
            PREVIEW_ASPECT,
        )
    })
}

/// Height of the caption area below a preview cell image.
fn caption_band() -> f32 {
    size::PREVIEW_CAPTION_GAP + size::PREVIEW_CAPTION_H
}

/// `slot` with the caption band taken off the bottom, which is the box an image
/// is fitted into.
fn above_caption(slot: Rect) -> Rect {
    Rect::from_min_max(slot.min, Pos2::new(slot.max.x, slot.max.y - caption_band()))
}

/// Derives the caption bounding box directly below preview `image`.
pub fn caption_of(image: Rect) -> Rect {
    Rect::from_min_max(
        Pos2::new(image.min.x, image.max.y + size::PREVIEW_CAPTION_GAP),
        Pos2::new(image.max.x, image.max.y + caption_band()),
    )
}

/// Layout orientation of preview cells relative to the main program picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// The mock's own: the picture across the top, the four cells in a row under
    /// it. The default, and what a tie gives.
    Below,
    /// The picture in the middle, two cells down the left and two down the right.
    /// What a bay wider than it is tall gets.
    Beside,
}

/// Solved arrangement of program bay content (picture, cells, placement orientation).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Body {
    /// Which arrangement won, carried because the caller cannot derive it from the
    /// rectangles without re-running the decider — and re-running it is the second
    /// answer this value exists to prevent.
    pub placement: Placement,
    /// The picture: the canvas's shape, as large as the arrangement leaves room
    /// for, centred in what is left ([`fitted`]).
    pub picture: Rect,
    /// The four cells, in [`DECK_LETTERS`] order — always four, and always in that
    /// order. See [`program_body`] for why a cell does not move when the deck
    /// behind it stops.
    pub cells: [Rect; DECKS],
}

/// Solves Program bay layout, choosing below or beside placement based on picture area.
///
/// Preserves canvas aspect ratio per [ADR-0181](../../../../docs/adr/0181-the-picture-is-the-canvass-shape-and-the-leftover-is-the-consoles.md) and deck slots per [ADR-0170](../../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md).
pub fn program_body(body: Rect, canvas: (u32, u32)) -> Option<Body> {
    program_body_with_row_h(body, canvas, size::PREVIEW_ROW_H)
}

/// Program bay layout state containing picture, preview cells, and active placement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgramBay {
    /// Which way round the body was arranged. [`Placement::Below`] whenever there
    /// is no picture, which is the guard rule below written into the value rather
    /// than left to the caller.
    pub placement: Placement,
    /// Where the picture goes, or `None` where the operator folded it away or the
    /// bay has no room for it — [`picture_rect`]'s answer.
    pub picture: Option<Rect>,
    /// Where the four cells go, in [`DECK_LETTERS`] order, or `None` where the
    /// operator folded the row away — [`preview_rects`]'s answer.
    pub cells: Option<[Rect; DECKS]>,
}

impl ProgramBay {
    /// Returns the deck letter index corresponding to point `p`, or `None`.
    pub fn cell(&self, p: karakuri_layout::Point) -> Option<u8> {
        let at = Pos2::new(p.x, p.y);
        self.cells?
            .iter()
            .position(|cell| cell.contains(at))
            .map(|deck| deck as u8)
    }

    /// Returns which deck index a carry dropped at `p` lands on, or `None` if unslotted per [ADR-0170](../../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md) and ADR-0265.
    pub fn dropped(&self, p: karakuri_layout::Point, slots: usize) -> Option<u8> {
        self.cell(p).filter(|deck| usize::from(*deck) < slots)
    }

    /// Returns whether `p` is on any preview cell (preview click operations retired per [ADR-0240](../../../../docs/adr/0240-the-output-shows-the-mix-and-residency-keys-belong-to-the-mixer.md) and ADR-0273).
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.cell(p).is_some()
    }
}

/// Solves Program bay layout for `rect`, handling arrangement and collapsed states per [ADR-0182], [ADR-0174](../../../../docs/adr/0174-a-node-claims-only-what-its-visible-content-can-use.md), and [ADR-0183](../../../../docs/adr/0183-a-node-is-out-of-the-layout-for-two-reasons-and-they-are-two-bits.md).
pub fn program_bay(layout: &karakuri_layout::Layout, canvas: (u32, u32)) -> Option<ProgramBay> {
    let bay = layout.find("program")?;
    // The bay itself, its bay folded around it, or a solo somewhere else: one
    // question for every ancestor, asked where `set_aside` is not in the way.
    if !layout.visible(bay) {
        return None;
    }
    let picture = layout.find("program-view")?;
    let row = layout.find("deck-previews")?;
    let body = bay_body(to_egui(layout.rect(bay)));
    let row_h = match layout.sizing(row) {
        karakuri_layout::Sizing::Fixed(h) => (h - size::PROGRAM_BODY_PAD).max(size::PREVIEW_ROW_H),
        _ => size::PREVIEW_ROW_H,
    };
    match (!layout.is_collapsed(picture), !layout.is_collapsed(row)) {
        // Both halves on screen, and this is the arrangement ADR-0182 decides.
        (true, true) => program_body_with_row_h(body, canvas, row_h).and_then(|arranged| {
            match arranged.placement {
                Placement::Beside => Some(ProgramBay {
                    placement: Placement::Beside,
                    picture: Some(arranged.picture),
                    cells: Some(arranged.cells),
                }),
                Placement::Below => {
                    let pic_rect = to_egui(layout.rect(picture));
                    let pic_area = Rect::from_min_max(
                        Pos2::new(
                            pic_rect.min.x + size::PROGRAM_BODY_PAD,
                            pic_rect.min.y + size::HEAD_H + size::PROGRAM_BODY_PAD,
                        ),
                        Pos2::new(pic_rect.max.x - size::PROGRAM_BODY_PAD, pic_rect.max.y),
                    );
                    let row_rect = to_egui(layout.rect(row));
                    let row_area = Rect::from_min_max(
                        Pos2::new(row_rect.min.x + size::PROGRAM_BODY_PAD, row_rect.min.y),
                        Pos2::new(
                            row_rect.max.x - size::PROGRAM_BODY_PAD,
                            row_rect.max.y - size::PROGRAM_BODY_PAD,
                        ),
                    );
                    drawable(
                        Placement::Below,
                        fitted(pic_area, canvas),
                        preview_row(row_area),
                    )
                    .map(|b| ProgramBay {
                        placement: Placement::Below,
                        picture: Some(b.picture),
                        cells: Some(b.cells),
                    })
                }
            }
        }),
        // The row is folded: nothing to arrange around, so the picture has the
        // body whole — the same `fitted` the two arrangements end in.
        (true, false) => Some(ProgramBay {
            placement: Placement::Below,
            picture: kept(fitted(body, canvas)),
            cells: None,
        }),
        // **The guard.** The picture is folded, so nothing moves: the row is
        // below at its own height, in the region the arrangement solved for
        // it, which is the rectangle it has had since ADR-0174.
        (false, true) => Some(ProgramBay {
            placement: Placement::Below,
            picture: None,
            cells: preview_cells(to_egui(layout.rect(row))),
        }),
        // Both folded. The bay has a head and no body at all, which is what it
        // had before any of this.
        (false, false) => None,
    }
}

/// Returns the inner content bounds of the Program bay, subtracting header height and padding.
fn bay_body(bay: Rect) -> Rect {
    let pad = size::PROGRAM_BODY_PAD;
    Rect::from_min_max(
        Pos2::new(bay.min.x + pad, bay.min.y + size::HEAD_H + pad),
        Pos2::new(bay.max.x - pad, bay.max.y - pad),
    )
}

/// A rectangle, where there is anything of it to draw — [`positive`] as an
/// `Option`, which is the shape all four of its call sites wanted.
fn kept(rect: Rect) -> Option<Rect> {
    match positive(rect) {
        true => Some(rect),
        false => None,
    }
}

/// Updates layout when preview row moves between below and beside placements per [ADR-0183](../../../../docs/adr/0183-a-node-is-out-of-the-layout-for-two-reasons-and-they-are-two-bits.md) and ADR-0164.
pub fn rearrange(panel: &mut Panel, canvas: (u32, u32)) -> bool {
    panel.solve();
    let beside = matches!(
        program_bay(panel.layout(), canvas).map(|bay| bay.placement),
        Some(Placement::Beside)
    );
    let Some(row) = panel.layout().find("deck-previews") else {
        return false;
    };
    let moved = panel.set_aside(row, beside);
    // The second solve, and on all but the frame the placement changed it is
    // the flag test the first one was.
    panel.solve();
    moved
}

mod badge;
mod cell;
mod head;
mod placement;

pub use badge::{
    band_of, Band, Basis, Budgeted, BAND_BLUE_MS, BAND_PURPLE_MS, BAND_RED_MS, BAND_YELLOW_MS,
};
pub use cell::{caption_into, preview, PREVIEW_MATERIAL, PREVIEW_NO_SLOT, PREVIEW_OVERLOADED};
pub use head::{program_head, ProgramHead};
pub(crate) use placement::drawable;
pub use placement::program_body_with_row_h;
