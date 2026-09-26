use super::*;

pub mod filters;
pub mod listing;
pub mod scopes;
pub mod state;

pub use filters::*;
pub use listing::*;
pub use scopes::*;

pub(super) use filters::{filters_into, kinds_into};
pub(super) use listing::{
    deck_list_into, foot_into, reading_into, row_menu_into, rows_into, Listed,
};
pub(super) use scopes::{path_into, scopes_into};

// ---------------------------------------------------------------------------
// The Library bay
// ---------------------------------------------------------------------------

/// The word at the head of the Library bay, in the source's own capitalisation
/// for [`Kind::Bay`]'s reason: the mock upper-cases in CSS, and that is done at
/// paint time so the word a reader searches for is the word in the source.
pub(super) const LIBRARY_TITLE: &str = "Library";

/// The Library bay, laid out: chip selectors, path breadcrumbs, rows listing, and footer count.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LibraryBay {
    /// Bounding rectangle for the scope chips row (`.scopes`), or `None` if no scopes are configured.
    pub scopes: Option<Rect>,
    /// Bounding rectangle for the path breadcrumb row (`.path`), or `None` if no folder is pointed (ADR-0275).
    pub path: Option<Rect>,
    /// Bounding rectangle for the filter fields row (`.lib-filters`), or `None` if absent or too narrow.
    pub filters: Option<Rect>,
    /// Bounding rectangle for the kind chips row (`.lib-kinds`), or `None` if absent (ADR-0338).
    pub kinds: Option<Rect>,
    /// `.lib-list`'s content box: the region under the bay head and the scope row
    /// and above the foot, inside [`size::LIB_LIST_PAD`], where the rows are laid
    /// from the top with no gap between them.
    pub list: Rect,
    /// Number of fully visible rows in the listing (`n` in the footer's `n of m` readout).
    ///
    /// Whole rows are counted so that `m of m` guarantees nothing is partially obscured.
    pub rows: usize,
    /// Clamped scroll offset for the rendered listing view (P-0082).
    pub scroll: f32,
    /// Total listing height including any expanded reading block ([`library_content_h`]).
    pub content: f32,
    /// Bounding rectangle for the entire Library bay handling wheel input.
    pub bay: Rect,
    /// How many Sets the selected scope holds, which is what the harness handed
    /// over. The second half of the foot's `n of m`.
    pub total: usize,
    /// `.lib-foot`, along the bottom edge of the bay, with its rule on top.
    pub foot: Rect,
    /// Active reading block under the cursor row, or `None` if closed or out of view.
    pub reading: Option<Block>,
}

/// Derives layout rectangles for the Library bay from the solved frame and view state (ADR-0275, ADR-0338).
pub fn library(
    layout: &karakuri_layout::Layout,
    scopes: &[Scope],
    sets: &[String],
    open: Option<Opened<'_>>,
    pointed: Option<Pointed<'_>>,
    scroll: f32,
) -> Option<LibraryBay> {
    // Omit library bay if neither scopes nor sets are configured (ADR-0177).
    if scopes.is_empty() && sets.is_empty() {
        return None;
    }
    library_box(
        to_egui(layout.rect(layout.find("library")?)),
        !scopes.is_empty(),
        pointed.is_some(),
        sets.len(),
        open.map(|open| (open.at, open.reading.rows())),
        scroll,
    )
}

/// Computes layout rectangles for the Library bay components from the available region.
fn library_box(
    region: Rect,
    chips: bool,
    pointed: bool,
    total: usize,
    open: Option<(usize, usize)>,
    scroll: f32,
) -> Option<LibraryBay> {
    let foot = Rect::from_min_max(
        Pos2::new(region.min.x, region.max.y - size::LIB_FOOT_H),
        region.max,
    );
    let under_head = region.min.y + size::HEAD_H;
    // Scopes row is positioned directly beneath the bay header.
    let scopes = chips.then(|| {
        Rect::from_min_max(
            Pos2::new(region.min.x, under_head),
            Pos2::new(region.max.x, under_head + size::SCOPES_H),
        )
    });
    // The path row sits below scopes and above filters if a folder is dropped (ADR-0275).
    let under_scopes = scopes.map_or(under_head, |scopes| scopes.max.y);
    let path = pointed.then(|| {
        Rect::from_min_max(
            Pos2::new(region.min.x, under_scopes),
            Pos2::new(region.max.x, under_scopes + size::PATH_H),
        )
    });
    let under_path = path.map_or(under_scopes, |path| path.max.y);
    // Filters require configured scopes and sufficient width for input fields.
    let filters = (chips && region.width() - size::LIB_FILTERS_PAD_X * 2.0 > 0.0).then(|| {
        Rect::from_min_max(
            Pos2::new(region.min.x, under_path),
            Pos2::new(region.max.x, under_path + size::LIB_FILTERS_H),
        )
    });
    // Kinds row sits below filters and clips to available width (ADR-0338).
    let kinds = filters.map(|filters| {
        Rect::from_min_max(
            Pos2::new(region.min.x, filters.max.y),
            Pos2::new(region.max.x, filters.max.y + size::LIB_KINDS_H),
        )
    });
    let top = match (kinds, filters) {
        (Some(kinds), _) => kinds.max.y,
        (None, Some(filters)) => filters.max.y,
        (None, None) => under_path,
    };
    let list = Rect::from_min_max(
        Pos2::new(region.min.x + size::LIB_LIST_PAD, top + size::LIB_LIST_PAD),
        Pos2::new(
            region.max.x - size::LIB_LIST_PAD,
            foot.min.y - size::LIB_LIST_PAD,
        ),
    );
    // List must have positive width after subtracting padding.
    if list.width() <= 0.0 {
        return None;
    }
    let fits = (list.height() / size::LIB_ROW_H).floor().max(0.0) as usize;
    // The reading block expands between rows under the selected cursor row.
    let block = open
        .filter(|(at, _)| *at < total)
        // Expanded reading block inserts immediately after the selected item.
        .map(|(at, rows)| (at + 1, rows));
    let content = library_content_h(total, block.map(|(_, rows)| rows));
    // Scroll offset clamped to content bounds without writing back (P-0082).
    let scroll = scroll.clamp(0.0, (content - list.height()).max(0.0));
    let reading = block.map(|(under, rows)| {
        let top = list.min.y - scroll + size::LIB_ROW_H * under as f32 + size::READING_MARGIN_TOP;
        Block {
            well: Rect::from_min_max(
                Pos2::new(list.min.x + size::READING_MARGIN_X, top),
                Pos2::new(
                    list.max.x - size::READING_MARGIN_X,
                    top + size::LIB_ROW_H * rows as f32,
                ),
            ),
            rows,
            under,
        }
    });
    // Compute whole row count from the initial layout geometry.
    let bay = LibraryBay {
        scopes,
        path,
        filters,
        kinds,
        list,
        rows: 0,
        total,
        foot,
        reading,
        scroll,
        content,
        bay: region,
    };
    // Count rows fully contained vertically within the list bounds.
    let whole = bay
        .drawn()
        .filter(|index| {
            let row = bay.row(*index);
            row.min.y >= list.min.y && row.max.y <= list.max.y
        })
        .count();
    (fits > 0).then_some(LibraryBay { rows: whole, ..bay })
}

/// Calculates total height of a library listing including any expanded reading block.
pub(super) fn library_content_h(total: usize, block: Option<usize>) -> f32 {
    size::LIB_ROW_H * total as f32
        + block.map_or(0.0, |rows| {
            size::READING_MARGIN_TOP + size::LIB_ROW_H * rows as f32 + size::READING_MARGIN_BOTTOM
        })
}

/// Paints the Library bay rows, star indicators, foot count, and any open reading block.
pub(super) fn library_into(
    ui: &Ui,
    pal: &Palette,
    bay: &LibraryBay,
    listed: Listed<'_>,
    cursor: usize,
    at: Target,
    open: Option<Opened<'_>>,
) {
    rows_into(ui, pal, bay, listed, cursor);

    // Reading block clipped within the list region to prevent overlapping the foot.
    if let (Some(block), Some(open)) = (bay.reading, open) {
        let painter = ui.painter().with_clip_rect(bay.list);
        reading_into(&painter, pal, &block, open.reading);
    }

    foot_into(ui, pal, bay, at);
}
