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
    /// `.scopes`: the row of chips between the bay head and the list, one
    /// [`size::SCOPES_H`] tall and the full width of the bay, with its own rule
    /// along the bottom of it.
    ///
    /// `None` where the console was handed no scopes at all, which is a console
    /// nobody has told what libraries there are — every test in this crate that
    /// does not say otherwise. The list then starts directly under the head, which
    /// is where it started before this row was drawn: a band of empty card with a
    /// rule under it would be a scope row with no questions in it.
    pub scopes: Option<Rect>,
    /// `.path`: the directory this library is pointed at, between the scope row and
    /// the filters, one [`size::PATH_H`] tall and the full width of the bay, with
    /// its own rule along the bottom of it.
    ///
    /// `None` until a folder has been chosen, which is every run before a drop and
    /// every test in this crate that does not say otherwise — see this module's
    /// [`library`] and the section on this row at [`LibraryBay`] for why an absent
    /// row rather than an empty one.
    ///
    /// It is not [`scopes`](Self::scopes)' condition and not the filters'. A
    /// console handed no scopes at all can still be pointed somewhere — the row
    /// says where a send lands, and that is true of a bay with no chips drawn over
    /// it — so the two are independent and the arithmetic below takes them in the
    /// order the mock stacks them.
    pub path: Option<Rect>,
    /// `.lib-filters`: the row of two fields between the scope row and the list,
    /// one [`size::LIB_FILTERS_H`] tall and the full width of the bay, with its own
    /// rule along the bottom of it.
    ///
    /// `None` wherever [`scopes`](Self::scopes) is, and that is one condition
    /// rather than two: a filter is a question about a listing, and a console
    /// nobody has told what libraries there are has no listing to ask it of. It is
    /// also `None` in a bay too narrow to hold two fields, which is `list`'s own
    /// rule stated on a row that has a `gap` in the middle of it — see
    /// `library_box`.
    pub filters: Option<Rect>,
    /// `.lib-kinds`: the row of six kind toggles under the filter field, one
    /// [`size::LIB_KINDS_H`] tall and the full width of the bay, with its own rule
    /// along the bottom of it.
    ///
    /// `None` wherever [`filters`](Self::filters) is, and that is one condition
    /// rather than two: both are the bay's head rather than its body — one narrows
    /// the listing by what a node is called and the other by what a row *is* — so a
    /// console that has been told about no library draws neither, and a bay too
    /// narrow to hold the field is too narrow to hold six chips.
    ///
    /// Under the fields and not beside them, which is `style.css`'s own note:
    /// `.field` carries `flex: 1` and six chips sharing one row with it in a bay
    /// this narrow would leave the chips a few pixels each, and a chip nobody can
    /// hit is not a control (ADR-0338).
    pub kinds: Option<Rect>,
    /// `.lib-list`'s content box: the region under the bay head and the scope row
    /// and above the foot, inside [`size::LIB_LIST_PAD`], where the rows are laid
    /// from the top with no gap between them.
    pub list: Rect,
    /// How many rows this bay is drawing whole, which is the `n` of the `n of m` in
    /// its foot — rule 04 of [the manual](../../../../docs/manual/index.html): *"A
    /// list that showed you part of itself says so and says how much."*
    ///
    /// # It is the readout's number and not the walk's
    ///
    /// [`LibraryBay::drawn`] is what is painted and what a press is hit-tested
    /// against, and it is the wider of the two: a row cut by the top edge or the
    /// bottom one is drawn as far as the list goes and can be pressed where it is
    /// drawn. This counts the ones that are whole, so that `m of m` means *nothing
    /// is out of sight* and can never be read off a bay with a row hanging over an
    /// edge. [`InspectorPane::shown`] is the same pair one bay over, and for the
    /// same reason
    /// ([ADR-0312](../../../../docs/adr/0312-the-params-pill-is-a-toggle-and-the-library-bay-scrolls.md),
    /// [ADR-0307](../../../../docs/adr/0307-the-inspectors-pane-scrolls-and-the-position-is-the-panes-own.md)).
    ///
    /// It used to be how many were drawn, and the two were one number, because
    /// there was nowhere to scroll to: a listing longer than the list simply
    /// stopped, and the rows past the end were unreachable by any means. At rest
    /// the two are still one number, so every reading of the foot that was true
    /// before is true now.
    ///
    /// Zero is a state now, and it is the one the scope row bought. A scope that
    /// holds nothing is a question that has been asked and answered — *my sets*
    /// with nothing starred, a presets root nobody filled — so the chips are drawn,
    /// the list is empty and the foot reads `0 of 0`. That is not the row of zeroes
    /// ADR-0177 is about: that one is a reading nobody took, and this is the answer
    /// to a question the chip above it is asking. A bay with no *room* for a row is
    /// still no bay at all — see `library_box`.
    pub rows: usize,
    /// How far down the listing this bay has come, as it is drawn — the clamped
    /// position, and never the stored one.
    ///
    /// # Clamped here and stored nowhere
    ///
    /// The clamp is `0 ..= (content - list height)`, and both ends of that range
    /// move when a divider moves — so a clamp written back into [`View`] would be a
    /// resize rewriting what an operator scrolled to. That is
    /// [`InspectorPane::scroll`]'s rule one bay over and
    /// [P-0082](../../../../docs/principles/0082-looking-never-writes-back.md): a
    /// shorter bay draws less of the same position and stores nothing, so dragging
    /// it back reproduces what was on screen exactly rather than nearly.
    ///
    /// What [`View::scroll_library_by`] clamps is the *content*, which is a reading
    /// of the listing rather than of a viewport.
    pub scroll: f32,
    /// How tall the whole listing comes to, the reading block included, whether or
    /// not any of it is on screen — [`library_content_h`].
    ///
    /// It is the number [`scroll`](Self::scroll) is clamped against and the number
    /// [`View::scroll_library_by`] is clamped against, derived in one place so the
    /// two cannot disagree.
    pub content: f32,
    /// The whole bay, which is what a wheel over it is aimed at —
    /// [`crate::input::wheeled`], the region rather than the list.
    ///
    /// The head rows are part of the thing being scrolled, which is
    /// [`InspectorPane`]'s own answer: a hand resting over the scope chips or the
    /// filter fields turns the list under them, exactly as a hand over a deck head
    /// turns that pane. The foot is in it for the same reason and for one more — it
    /// is the readout the scroll is about.
    pub bay: Rect,
    /// How many Sets the selected scope holds, which is what the harness handed
    /// over. The second half of the foot's `n of m`.
    pub total: usize,
    /// `.lib-foot`, along the bottom edge of the bay, with its rule on top.
    pub foot: Rect,
    /// The reading open under the cursor, or `None` where nothing is open — which
    /// is every console until a press on the `params` chip, and every one of this
    /// crate's tests that does not say otherwise.
    ///
    /// `None` also where the row it would open under is not drawn, which is the one
    /// state the two halves can be in and disagree about: a cursor clamps against
    /// the listing ([`View::cursor_row`]) and the rows clamp against the room there
    /// is, so a bay with room for two rows and a reading of the fifth has nowhere
    /// to put it. It is answered here rather than at the paint, so what is laid out
    /// and what is painted are one statement.
    pub reading: Option<Block>,
}

/// The Library bay's rows, derived — see [`LibraryBay`] for what is drawn here
/// and for the six things in the mock's bay that are not.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one. Unlike [`outputs`], [`transport`] and [`mixer`] this asks `egui` for
/// nothing: every box in the bay is the full width of the list, so no rectangle
/// here is the width of the type in it.
///
/// `None` where there is nothing to draw at all, and `None` where there is no
/// room to draw it: a console with no scopes and no rows has had nothing said
/// to it about any library, which is every test in this crate that does not say
/// otherwise and the whole of `cargo test -p karakuri-console`, and what the
/// bay draws then is the card and its head and nothing else — this is
/// [`mixer`]'s rule, one column along, and [`View::picture`]'s before that.
///
/// A scope with nothing in it is not that, and the difference is the whole of
/// what the chips bought: handed scopes and no rows, the bay draws the row of
/// questions and answers the marked one with `0 of 0`, because a question that
/// has been asked is owed an answer even where the answer is *nothing*.
/// `console.html`: *"An empty tier is a library nobody has filled rather than
/// something gone wrong."*
///
/// `scopes` is the row of chips, in the order they are drawn; `sets` is the
/// listing of whichever of them is marked. Which one that is does not reach
/// here, because no rectangle in this bay depends on it: it is a pointer, and a
/// pointer goes to the paint beside the library cursor — see `library_into` and
/// [`View::scope`].
///
/// `open` is the exception, and it is the one pointer a rectangle in this bay
/// does depend on. A reading is drawn *under a row*, so where every row below
/// it goes and how many of them there is room for both follow the cursor — see
/// [`Opened`] and [`Block`]. `None` is a bay with nothing open, which is every
/// console until a press on the `params` chip.
///
/// `pointed` is a row of furniture rather than a pointer, and only whether
/// there is one reaches the arithmetic: the `.path` row is drawn once a folder
/// has been chosen and not at all before, so everything under it — the fields,
/// the list, and how many rows there is room for — is measured from where it
/// ends. What it *reads* goes to `path_into` and nowhere else, exactly as the
/// marked chip does. `None` is a bay pointed nowhere with nothing over the
/// window, which is every console until a folder is dropped on one (ADR-0275) —
/// see [`Pointed`] and [`View::pointed`].
pub fn library(
    layout: &karakuri_layout::Layout,
    scopes: &[Scope],
    sets: &[String],
    open: Option<Opened<'_>>,
    pointed: Option<Pointed<'_>>,
    scroll: f32,
) -> Option<LibraryBay> {
    // **Nothing said about any library, so there is nothing to draw.** Not the
    // same as a scope that holds nothing — see this function's own doc, and
    // ADR-0177 for the row of zeroes this is still refusing.
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

/// The arithmetic of the bay, away from the layout it reads.
///
/// Term for term from `style.css`:
///
/// - `.lib-foot { padding: 5px 10px; font-size: 10px; border-top: 1px solid
///   var(--c-hair) }` — a [`size::LIB_FOOT_H`] row along the bottom of the
///   bay, its rule the top pixel of it.
/// - `.scopes { display: flex; gap: 4px; padding: 7px 9px }` — a
///   [`size::SCOPES_H`] row under the bay head, its own rule the bottom pixel
///   of it, and drawn only where there are scopes to put in it.
/// - `.path { padding: 4px 10px; font-size: 10px; border-bottom: 1px solid
///   var(--c-hair) }` — a [`size::PATH_H`] row under the scopes, its own rule
///   the bottom pixel of it, and drawn only once a folder has been dropped on
///   the window.
/// - `.lib-list { display: flex; flex-direction: column; padding: 3px }` —
///   what is left between the scope row and the foot, inset by
///   [`size::LIB_LIST_PAD`] on all four sides.
/// - `.lib-row { padding: 3px 7px }` — [`size::LIB_ROW_H`] each, stacked from
///   the top of the list with no gap, because `.lib-list` states none.
///
/// # The foot is at the bottom and the leftover is the list's
///
/// The mock's bay is a flow: its children stack from the top and whatever is
/// left over is under the last of them. Here the leftover goes to the list
/// instead, and the mock says which: the Library is the bay that carries
/// `style="flex:1"` in its column and a `.grip` in its head, which is *"the
/// bay that absorbs its column's height"* — and what absorbs it is the list,
/// since a foot of one line at a fixed type size has nothing in it that gets
/// bigger. A foot left floating under the last row would also put its
/// `border-top` between the list and bare card, which is a rule separating
/// something from nothing.
///
/// `None` where the region cannot hold the foot and one row, which is
/// [`picture_rect`]'s rule stated on a listing. It is room and not content:
/// a scope that lists nothing still wants room for a row, because the bay it is
/// drawn in is the bay the next scope's rows land in and a question drawn over
/// somewhere there is no room to answer it is worse than no question.
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
    // **The scope row is the head's business and not the list's**, which is
    // why it is taken off the top before the list is measured: the mock draws
    // it between the bay head's rule and `.lib-list`, and `.lib-list`'s own
    // padding is inside whatever is left.
    let scopes = chips.then(|| {
        Rect::from_min_max(
            Pos2::new(region.min.x, under_head),
            Pos2::new(region.max.x, under_head + size::SCOPES_H),
        )
    });
    // **The path row is under the chips and above the fields**, which is where
    // the mock puts it, and it is nobody's condition: a bay is pointed at a
    // directory or it is not, and that is true whether or not it was handed
    // scopes to draw over it. **Its own condition is that there is a path at
    // all** — until a folder has been dropped the row is not drawn and this
    // bay is a line shorter (ADR-0275), which is why every rectangle under it
    // is measured from where it ends rather than from the scope row.
    let under_scopes = scopes.map_or(under_head, |scopes| scopes.max.y);
    let path = pointed.then(|| {
        Rect::from_min_max(
            Pos2::new(region.min.x, under_scopes),
            Pos2::new(region.max.x, under_scopes + size::PATH_H),
        )
    });
    let under_path = path.map_or(under_scopes, |path| path.max.y);
    // **The filter row is the scope row's condition and not a second one.**
    // Both are the bay's head rather than its body — one says which library is
    // being read and the other narrows what that library answers — so a
    // console that has been told about no library draws neither. The width
    // check is `list`'s below, stated on a row that has a box and a padding in
    // it: a row too narrow for the field would draw a sliver and a rule.
    let filters = (chips && region.width() - size::LIB_FILTERS_PAD_X * 2.0 > 0.0).then(|| {
        Rect::from_min_max(
            Pos2::new(region.min.x, under_path),
            Pos2::new(region.max.x, under_path + size::LIB_FILTERS_H),
        )
    });
    // **The kind row is the filter row's condition and not a third one**, which
    // is the sentence above read once more: both are the bay's head, one asks
    // what a Set is made of and the other what a row *is*, and a bay with room
    // for neither draws neither. **The chips are not measured against the
    // width**, which is the scope row's own answer one band up: a chip is as
    // wide as the word in it, the row clips, and a press is held to the part of
    // a chip that is drawn (ADR-0338, `LibraryBay::kind`).
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
    // **Narrower than its own padding is no list**, which is
    // [`picture_rect`]'s rule stated across the axis. There is no matching
    // check down it: a bay too short for the foot is already a bay too short
    // for a row, and `fits` below is what answers that.
    if list.width() <= 0.0 {
        return None;
    }
    let fits = (list.height() / size::LIB_ROW_H).floor().max(0.0) as usize;
    // **A reading opens under the row it is of, whether or not that row is on
    // screen.** It used to open only under a row this bay was *drawing*,
    // because a reading scrolled past the bottom had nowhere to be; a bay with
    // a position has somewhere, and the block scrolls with the rows because it
    // is between two of them rather than over them. What still holds the two
    // together is [`View::opened`], which answers `None` the moment the row
    // under the cursor stops being the Set the reading is of.
    let block = open
        .filter(|(at, _)| *at < total)
        // **The block is between the cursor's row and the next**, so what is
        // above it is the cursor's row and everything before it.
        .map(|(at, rows)| (at + 1, rows));
    let content = library_content_h(total, block.map(|(_, rows)| rows));
    // **The clamp that is drawn and never stored** — see [`LibraryBay::scroll`]
    // and [P-0082]. Zero-width where the listing is shorter than the list,
    // which is a bay that cannot be scrolled at all.
    //
    // [P-0082]: ../../../docs/principles/0082-looking-never-writes-back.md
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
    // **Built once with the count unanswered and then answered off itself**,
    // because how many rows are whole is a question about the rectangles this
    // bay hands out — [`LibraryBay::row`] and [`LibraryBay::drawn`] — and a
    // second arithmetic here would be a second answer to where a row is.
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
    // **Down the column and not across it**: a row is exactly as wide as the
    // list and starts where it starts, so the only edge a row can be cut by is
    // the top one or the bottom one.
    let whole = bay
        .drawn()
        .filter(|index| {
            let row = bay.row(*index);
            row.min.y >= list.min.y && row.max.y <= list.max.y
        })
        .count();
    (fits > 0).then_some(LibraryBay { rows: whole, ..bay })
}

/// How tall a library listing comes to, the reading block included —
/// [`LibraryBay::content`], and what both clamps are taken against.
///
/// One function because the two clamps are one number read twice: `library_box`
/// clamps the position it *draws* against it and [`View::scroll_library_by`]
/// clamps the position it *stores* against it, and a second arithmetic in
/// either would be a bay that could be scrolled to a place it will not draw.
/// [`content_h`] is the same shape one bay over.
///
/// `block` is how many rows the reading under the cursor is, or `None` where
/// none is open — the same run [`LibraryBay::pushed`] adds to every row below
/// it, so the two cannot disagree about what a reading costs.
pub(super) fn library_content_h(total: usize, block: Option<usize>) -> f32 {
    size::LIB_ROW_H * total as f32
        + block.map_or(0.0, |rows| {
            size::READING_MARGIN_TOP + size::LIB_ROW_H * rows as f32 + size::READING_MARGIN_BOTTOM
        })
}

/// The Library bay's rows, painted.
///
/// Where everything goes is [`library`]'s, so this paints and derives nothing.
///
/// Term for term from `style.css`:
///
/// - `.lib-row` — `color: var(--c-dim)`, a star and then a name at
///   [`size::BASE`], one [`size::LIB_ROW_PAD_X`] in from the left of the list
///   and centred across the row's own height, with `.lib-row`'s own
///   [`STAR_GAP`] between the two.
/// - `.lib-row .star` — `color: var(--c-sun)` where the store has starred that
///   Set, and `.star.off`'s `var(--c-faint)` where it has not. Drawn rather
///   than typed ([`star_mark`]), so the two states are a filled mark and an
///   outline of the same mark rather than two characters that may or may not
///   be in the face.
/// - `.lib-foot` — `color: var(--c-faint)` at [`size::LIB_FOOT_SIZE`], one
///   [`size::LIB_FOOT_PAD_X`] in and centred, over a
///   `border-top: 1px solid var(--c-hair)`.
///
/// A name too long for the track is clipped rather than elided, which is
/// the mock's own answer: `.lib-row` sets no `text-overflow` where `.path` and
/// `.strip-name` both do, so there is no ellipsis to draw. The clip is
/// `.lib-list`'s box, which is the same `with_clip_rect` the picture, a
/// preview cell and a tally are each drawn inside.
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

    // **The reading, under the row it is a reading of.** Drawn inside the same
    // clip as the rows, which is what makes a reading taller than the bay a
    // clipped box rather than a box drawn over the foot — `.lib-list`'s own
    // answer to a name too long for the track, one axis round.
    if let (Some(block), Some(open)) = (bay.reading, open) {
        let painter = ui.painter().with_clip_rect(bay.list);
        reading_into(&painter, pal, &block, open.reading);
    }

    foot_into(ui, pal, bay, at);
}
