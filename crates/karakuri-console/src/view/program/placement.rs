use super::*;

/// Dynamic row height version of program_body
pub fn program_body_with_row_h(body: Rect, canvas: (u32, u32), row_h: f32) -> Option<Body> {
    match (below(body, canvas, row_h), beside(body, canvas, row_h)) {
        (Some(below), Some(beside)) if area(beside.picture) > area(below.picture) => Some(beside),
        (Some(below), _) => Some(below),
        (None, beside) => beside,
    }
}

/// Returns the area in texels of `rect`, used to pick the larger preview arrangement.
fn area(rect: Rect) -> f32 {
    rect.width() * rect.height()
}

/// Layout with picture on top and deck previews across the bottom.
pub fn below(body: Rect, canvas: (u32, u32), row_h: f32) -> Option<Body> {
    if body.height() <= row_h + crate::PROGRAM_DIVIDER {
        return None;
    }
    let row = Rect::from_min_max(Pos2::new(body.min.x, body.max.y - row_h), body.max);
    let picture = fitted(
        Rect::from_min_max(
            body.min,
            Pos2::new(body.max.x, row.min.y - crate::PROGRAM_DIVIDER),
        ),
        canvas,
    );
    drawable(Placement::Below, picture, preview_row(row))
}

/// The other arrangement: two cells down the left, two down the right, and the
/// picture in the middle.
///
/// Preserves the preview cell size from the row arrangement (ADR-0239).
pub fn beside(body: Rect, canvas: (u32, u32), row_h: f32) -> Option<Body> {
    let (aw, ah) = (
        PREVIEW_ASPECT.0.max(1) as f32,
        PREVIEW_ASPECT.1.max(1) as f32,
    );
    // Cell height includes both preview image and caption band.
    let cell_h = (row_h - caption_band()).min(body.height());
    let cell_w = (cell_h * aw / ah).round();
    let column = cell_w;

    let min_w = column * 2.0 + crate::PROGRAM_DIVIDER * 2.0;
    let total_cells_h =
        (cell_h + caption_band()) * PER_COLUMN as f32 + size::PREVIEW_GAP * (PER_COLUMN - 1) as f32;
    if body.width() <= min_w || body.height() < total_cells_h {
        return None;
    }

    let picture = fitted(
        Rect::from_min_max(
            Pos2::new(body.min.x + column + crate::PROGRAM_DIVIDER, body.min.y),
            Pos2::new(body.max.x - column - crate::PROGRAM_DIVIDER, body.max.y),
        ),
        canvas,
    );

    let top_offset = ((body.height() - total_cells_h) / 2.0).max(0.0).round();
    let cells = std::array::from_fn(|deck| {
        let col_idx = deck / PER_COLUMN; // 0 for left (A, B), 1 for right (C, D)
        let row_idx = deck % PER_COLUMN; // 0 for top (A, C), 1 for bottom (B, D)
        let x = match col_idx {
            0 => body.min.x,
            _ => body.max.x - column,
        };
        let y = body.min.y
            + top_offset
            + row_idx as f32 * (cell_h + caption_band() + size::PREVIEW_GAP);
        Rect::from_min_size(Pos2::new(x, y), egui::vec2(cell_w, cell_h))
    });
    drawable(Placement::Beside, picture, cells)
}

/// Returns the placement if both picture and cell bounds have positive area.
pub fn drawable(placement: Placement, picture: Rect, cells: [Rect; DECKS]) -> Option<Body> {
    match positive(picture) && cells.iter().copied().all(positive) {
        true => Some(Body {
            placement,
            picture,
            cells,
        }),
        false => None,
    }
}
