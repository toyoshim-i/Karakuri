use super::*;

// ---------------------------------------------------------------------------
// The Program bay
// ---------------------------------------------------------------------------

/// A picture to draw in the Program bay: a texture somebody else rendered, and
/// where it goes.
///
/// The id is `egui`'s, which means it has already been registered with an
/// [`egui_wgpu::Renderer`](crate::egui_wgpu::Renderer) — and that registration
/// needs a device, which is exactly what this crate does not have. So the
/// caller does it and hands the result over; this module draws an id and a
/// rectangle and knows nothing about either. It is the same seam the whole
/// crate is built on, one level in: `src/` reads and paints, and everything
/// that takes a device is the program's.
///
/// The rectangle is passed rather than looked up, and that is what makes the
/// pair checkable: whoever sized the texture and whoever placed it are the same
/// statement, so a texture sized from the window and drawn into the picture's
/// region cannot be written by accident. [`picture_rect`] is what a caller
/// derives both from.
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

/// The largest rectangle of `aspect` that fits inside `inside`, centred in it,
/// and a whole number of pixels in each direction.
///
/// # One derivation with two call sites, and they were one rule before they
/// were one function
///
/// [`picture_rect`] fits the canvas into what the bay's body leaves it and
/// [`preview_cells`] fits a cell into its track, and
/// [ADR-0170](../../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md)
/// states the second in words the first now takes unchanged: *"as large as the
/// track's width and the row's height both allow, centred"*. Two copies of it
/// is two chances for a picture and the thumbnails under it to disagree about
/// what *centred* means.
///
/// `aspect` is two numbers and not a ratio, for the reason [`picture_rect`]
/// gives at length, and it is deliberately not called `canvas`: the picture's
/// two numbers are the canvas's and a cell's are [`PREVIEW_ASPECT`]'s, which is
/// the mock's. One argument, two provenances, each stated where it is passed.
///
/// # What is rounded and what is not
///
/// The extent is rounded and the position is not. The extent is what a caller's
/// `physical` turns into texels, so it is the half that decides whether the
/// picture is blitted or resampled — [`picture_rect`] is where that argument is
/// written out. The offset is left as the true centre, because rounding it is a
/// cell no longer centred in its track, and *centred* is the other half of the
/// rule ADR-0170 took. A region whose own origin is fractional still samples
/// fractionally, and that is the arrangement's coordinate rather than this
/// rule's to fix.
///
/// The clamp is `floor` rather than the box's own extent so that the answer
/// stays whole: rounding up can exceed the box by up to half a pixel, and
/// clamping to a fractional edge would hand back the fractional extent this
/// exists to avoid.
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

/// What the Program bay arranges its body for when nobody has said what is
/// being rendered: the mock's own picture, which is `.program-view`'s
/// `aspect-ratio: 16/9` in `style.css`.
///
/// A console with no engine behind it has no canvas — it is `src/` taking no
/// device, one number further on (ADR-0156) — and it still has to put four
/// cells somewhere, so [`View::canvas`] starts here and whoever owns the
/// `Present` writes the real thing over it every frame. It is two numbers
/// rather than a ratio for [`picture_rect`]'s reason, and it is the mock's
/// picture rather than [`PREVIEW_ASPECT`]'s cell: the two are the same 16 and 9
/// read off two different rules in the same stylesheet, and a `--canvas` that
/// is not 16:9 moves one of them and not the other.
pub const MOCK_CANVAS: (u32, u32) = (16, 9);

/// Where the four deck previews go: a row of [`DECKS`] cells inside the
/// `deck-previews` region, under the picture.
///
/// The mirror of [`picture_rect`] and it carries the same `None` rule for the
/// same reason — a folded row has a rectangle with no extent in it, so the
/// cells come out degenerate and there is nothing to draw or to render into.
/// A fixed-size array rather than a `Vec`: there are four decks and there
/// is no fifth, so a caller cannot ask for one and cannot forget one either.
///
/// # The insets are `.program-body`'s, and only three of the four
///
/// The row is the region inset by [`size::PROGRAM_BODY_PAD`] left, right and
/// bottom, and by nothing at the top. Read the Program bay's own
/// derivation in `lib.rs`: `deck-previews` is 63 + 9, the row of cells and the
/// padding under them. The 9 *above* the cells in the CSS is not in this
/// region at all — it is the split's 8px divider plus `program-view`'s own
/// bottom, which is why [`picture_rect`] takes nothing off the bottom.
///
/// At the narrowest console the mock will draw, that leaves 484 - 9 - 9 = 466
/// for four tracks and three [`size::PREVIEW_GAP`]s: (466 - 18) / 4 = 112
/// wide, and 112 at 16:9 is 63 tall, which is exactly the height the row
/// has. The mock's cell, arrived at from the other end.
///
/// # A cell is 16:9 and centred in its track, and the alternative is written
/// down
///
/// The arrangement pins this region at 72 tall (`lib.rs`: fixed 72, minimum
/// 72, because a row of four cells at a fixed type size has nothing in it that
/// gets smaller). So a wider window widens the track and does not heighten
/// the row, and past the reference width a cell cannot both fill its track and
/// stay 16:9. One of the two has to give, and it is the track:
///
/// - Taken: the cell is 16:9, as large as the track's width and the row's
///   height both allow, and centred in its track. At the reference width that
///   is exactly 112 x 63 and fills the track; wider, it stays 63 tall with
///   ground either side. The texture then fills the cell exactly, so nothing
///   letterboxes twice and the cell is always the shape of what it shows.
///   [`picture_rect`] now takes that same rule for the picture, and the
///   two share [`fitted`] rather than stating it twice.
/// - Rejected: fill the track and letterbox the texels inside it. That
///   keeps the row looking like a grid at every width, and pays for it by
///   stretching the cell away from the shape of its picture — a 16:9 audition
///   in a 200x63 well, with bars the console has drawn itself inside a
///   rectangle the engine already fitted. Two fits for one question, which is
///   the thing `WHOLE_TEXTURE` refuses one level up.
///
/// # The row is one of two places the cells go, and this asks which
///
/// It insets the `deck-previews` region no longer, and it cannot: beside
/// the picture that region has no extent at all — it is
/// [`Layout::set_aside`](karakuri_layout::Layout::set_aside), which is exactly
/// what leaves the bay's width to the picture — so a cell derived from it
/// would be `None` at every window past the crossover, and deck A's audition
/// would stop being sized, stop being drawn and stop keeping the window's loop
/// awake. The cells come through [`program_bay`] like the picture does and
/// like the drawing does: one derivation, and every reader of it reads the
/// same frame's answer.
///
/// `canvas` is therefore an argument that was not here before. A cell is the
/// mock's shape and never the canvas's ([`PREVIEW_ASPECT`]), and this still
/// holds — what the canvas decides is *which arrangement*, because the thing
/// being compared is the picture, and the picture is the canvas's shape.
///
/// `layout` must be solved: `Layout::rect` refuses to answer from a dirty one,
/// and it wants [`rearrange`] to have been run on it — see [`program_bay`].
pub fn preview_rects(
    layout: &karakuri_layout::Layout,
    canvas: (u32, u32),
) -> Option<[Rect; DECKS]> {
    program_bay(layout, canvas)?.cells
}

/// The four cells inside a `deck-previews` region, or `None` where there is no
/// room for them.
///
/// One call site now, and it is the one case where the row is the answer:
/// [`program_bay`]'s guard branch, where the picture is folded away and the row
/// is the whole of the bay. Everywhere else the cells come off the bay's body
/// through [`program_body`], because everywhere else there is a picture for
/// them to be arranged around — and the two agree to the pixel where they
/// overlap, which is `tests/program_body.rs`'s
/// `below_is_what_the_console_draws_today`.
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

/// [`DECKS`] cells side by side across `row`, each [`PREVIEW_ASPECT`] and
/// centred in its track.
///
/// The row of the mock, with nothing said about where the row is: that is
/// [`preview_cells`]'s inset off the `deck-previews` region, and
/// [`program_body`]'s strip along the bottom of the bay's body. Two call sites
/// for one row, and they have to agree exactly — the second one is the same row
/// in the same place, arrived at from the bay rather than from the region, and
/// a second copy of this arithmetic is where the two would drift.
fn preview_row(row: Rect) -> [Rect; DECKS] {
    // [`PREVIEW_ASPECT`], as large as the track and the row both allow, and
    // centred — which is `fitted`, the same call `picture_rect` makes. The
    // aspect passed is the mock's rather than the canvas's, and the constant
    // is where that difference is argued.
    //
    // **The caption band comes off the track before the image is fitted**, so
    // what this answers is the image and never the image plus its label — see
    // [`caption_band`] and [`caption_of`].
    std::array::from_fn(|deck| {
        fitted(
            above_caption(track(row, DECKS, deck, size::PREVIEW_GAP, Axis::Row)),
            PREVIEW_ASPECT,
        )
    })
}

/// How much of a cell the caption takes: `.cell`'s gap and `.caption`'s own
/// height, which is the band under every image and is never inside one.
///
/// One function rather than the sum written three times — [`preview_row`],
/// [`beside`] and [`caption_of`] all need it and a second copy of it is where a
/// caption would land over the picture it labels.
fn caption_band() -> f32 {
    size::PREVIEW_CAPTION_GAP + size::PREVIEW_CAPTION_H
}

/// `slot` with the caption band taken off the bottom, which is the box an image
/// is fitted into.
fn above_caption(slot: Rect) -> Rect {
    Rect::from_min_max(slot.min, Pos2::new(slot.max.x, slot.max.y - caption_band()))
}

/// Where a cell's caption goes: directly under the image, the image's own
/// width, one [`size::PREVIEW_CAPTION_GAP`] below it and
/// [`size::PREVIEW_CAPTION_H`] tall.
///
/// Derived from the image rather than carried beside it in [`ProgramBay`], for
/// the reason [`Body`] gives about the picture and the cells: the two are one
/// statement. A caption that could be handed in separately is a caption that
/// could be handed in stale, and this way there is one rectangle in the world
/// and the label is a function of it. It is also what keeps [`preview_rects`]'s
/// answer the image — the engine sizes a texture from that rectangle, and a
/// cell rectangle that quietly included the caption would put texels over the
/// letter.
///
/// The image's width and not the track's: the image is centred in its track
/// ([ADR-0170](../../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md)),
/// and a caption starting at the track's left edge would sit off under the
/// ground beside the cell it names.
///
/// # *Cell* means two things here, and it is named rather than renamed
///
/// The mock's `.cell` is the image and the caption — that is what
/// [`size::PREVIEW_ROW_H`] measures — while [`ProgramBay::cells`] and
/// [`preview_rects`] answer the images, because a texture is sized from one and
/// a rectangle that quietly included the caption would put texels over the
/// letter. Renaming the field is a ripple through six test files and the
/// program's frame path, so the clash is written down here instead
/// ([`docs/contributing.md`](../../../../docs/contributing.md) §4 is the rule
/// it is in tension with, and this is the report rather than the fix).
pub fn caption_of(image: Rect) -> Rect {
    Rect::from_min_max(
        Pos2::new(image.min.x, image.max.y + size::PREVIEW_CAPTION_GAP),
        Pos2::new(image.max.x, image.max.y + caption_band()),
    )
}

/// Which way round the Program bay's body is arranged.
///
/// Not a state and not a setting: [`program_body`] answers it from the
/// rectangle it is given, every time it is asked, and nothing stores it. See
/// that function for the decider and for why it is a decision rather than a
/// preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// The mock's own: the picture across the top, the four cells in a row under
    /// it. The default, and what a tie gives.
    Below,
    /// The picture in the middle, two cells down the left and two down the right.
    /// What a bay wider than it is tall gets.
    Beside,
}

/// Where everything in the Program bay's body goes: the picture, and the four
/// deck preview cells.
///
/// One value rather than two calls, for [`Picture`]'s own reason: whoever
/// placed the picture and whoever placed the cells are then one statement, so a
/// picture drawn for one arrangement and cells drawn for the other cannot be
/// written by accident. That is not a hypothetical here — the two arrangements
/// put the cells in different halves of the bay, so the failure would be four
/// thumbnails over the top of the picture rather than a rectangle a few pixels
/// out.
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

/// How the Program bay's body arranges itself in the rectangle it has: the
/// picture and the four deck preview cells, either the mock's way or down the
/// sides, whichever leaves the picture larger.
///
/// `body` is the bay less its head and less `.program-body`'s padding —
/// [`bay_body`], taken off the bay as a whole because the arrangement below
/// spans both of the bay's regions and the divider between them. `canvas` is the picture's
/// aspect and arrives as two numbers for the reason [`picture_rect`] gives at
/// length.
///
/// `None` where neither arrangement can be drawn — [`positive`]'s rule, asked
/// of the picture and of every cell.
///
/// # Why there is a second arrangement at all
///
/// [ADR-0181](../../../../docs/adr/0181-the-picture-is-the-canvass-shape-and-the-leftover-is-the-consoles.md)
/// gave the picture the canvas's shape, and the leftover became the console's
/// ground. On a wide bay that leftover is ground down each side: at a
/// 1920-wide window the body is 1396 x 333 and the picture is 466 x 262, so
/// 930 pixels of the bay's width are empty and the four cells are 63 tall in a
/// row under it. Putting the cells in that ground is what lets the picture take
/// the height instead.
///
/// # The decider is the picture's size, and nothing else
///
/// Whichever arrangement gives the larger picture wins, and a tie goes to
/// [`Placement::Below`], which is the mock's. Both are computed and their
/// pictures compared; nothing is stored, nothing is remembered between frames,
/// and the same rectangle always gives the same answer — so
/// [P-0082](../../../../docs/principles/0082-looking-never-writes-back.md)
/// is untouched, and so is the property it buys: a window dragged wide and
/// back again comes back to exactly the arrangement it left.
///
/// The alternative is a stored mode — a preference, or a hysteresis band
/// around the crossover — and each of those is a second piece of state about
/// the same question, which is what this repository stops on. What a stateless
/// decider costs is a flip at one width, and the numbers below are what say
/// whether that width is anywhere an operator lives.
///
/// Three worked cases, and the arithmetic is in `tests/program_body.rs`:
///
/// - The mock's narrowest, 466 x 333. Below gives 466 x 262. Beside cannot
///   be drawn at all: two columns and their gaps come to 597, which is more
///   than the body is wide, so the picture's box is negative. Below wins
///   because it is the only one there is.
/// - A 1920 window, 1396 x 333. Below gives 466 x 262 — 122,092 texels.
///   Beside gives 592 x 333, which is 197,136, and the picture is 61%
///   larger. Beside wins.
/// - 800 wide, where below still wins: beside's picture would be 202 x 114.
///   The flip is at 1064, one pixel of body width, and there is exactly one
///   of them — below's picture stops growing with the width at 466 and beside's
///   never shrinks, so the two curves cross once and never again.
///
/// # A side column is as wide as two stacked cells, which makes it the
/// height's answer and not the width's
///
/// A column holds [`PER_COLUMN`] cells stacked with one [`size::PREVIEW_GAP`]
/// between them, so a cell is `(H - gap) / 2` tall and the column is that at
/// [`PREVIEW_ASPECT`] — a function of the body's height alone. Reading it
/// off the width instead is the mistake worth naming: the column would grow
/// with the very width it is competing for, and beside would never win at any
/// width — an arrangement that exists in the source and never on the screen.
///
/// Two things had to be checked about this rule and both hold:
///
/// - It is not so greedy that beside never wins. The column does not follow
///   the width, so widening the bay adds the whole increment to the picture's
///   box; below's picture is capped by the height it has *after* the row and
///   the gap come off, and beside's by the whole height. Beside therefore wins
///   at every width past the crossover and the crossover exists at every
///   height — at the mock's 333 it is a body 1064 wide, which is a 1588-wide
///   window, well inside an ordinary desktop.
/// - It is greedy in the other direction, and that is a real cost rather
///   than a caveat. The column follows the height, so a bay dragged taller
///   widens both columns while the picture's box is what pays for them: at 1396
///   wide the picture beside peaks at a body 391 tall and shrinks after it,
///   below's grows with every pixel, and the two cross at 427 — a Program
///   bay 472 tall, which an operator can drag to. So this arrangement is the
///   answer for a bay that is wide and short, and the cells go back under
///   the picture when it stops being short. It is one flip in each direction
///   and not a flicker — beside's picture is single-peaked in the height and
///   below's is monotone — and `tests/program_body.rs` sweeps both axes rather
///   than taking that on trust. The corner that follows from the same rule:
///   a bay dragged to its own minimum of 200 goes beside at every width,
///   the mock's narrowest included, because a body 155 tall has only 84 left
///   for the picture once the row and the divider come off.
/// - It is not so mean that the cells are unreadable. A cell beside the
///   picture is `(333 - 6) / 2 = 163` tall against the row's 63, so the
///   arrangement that takes the cells out of the row makes each of them larger
///   rather than smaller. At the Program bay's own minimum height the body is
///   155 and a cell is still 73, which is more than the mock's row gives at any
///   width at all.
///
/// # Which cell goes where, and why none of them moves
///
/// A and B down the left, C and D down the right, each column read top to
/// bottom — the row's own left-to-right order, folded in half, so an operator
/// who knows where `C` was in the row finds it at the top of the other side
/// rather than somewhere new.
///
/// With fewer than four decks running nothing fills and nothing shifts.
/// The mock's head reads *previews 3 of 4* and
/// [ADR-0170](../../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md)
/// is the answer: a cell is drawn whether or not a deck is behind it, because
/// *an empty cell is what off looks like, not a stand-in for a full one*. So
/// the question *does the left column fill first, or do they alternate* has a
/// third answer, and it is the one that keeps the letters meaning something:
/// a cell's place is its deck's, not its turn's. `C · no slot` sits at the top
/// of the right column whether or not C is running, and turning B off does not
/// slide C into B's place — the letter is the only thing naming a deck, and a
/// label that moves when a neighbour stops is a label an operator cannot point
/// at. This function is handed no liveness at all, which is that rule as a
/// signature.
///
/// # The gaps are the mock's, and there are two of them rather than three
///
/// - Between the picture and a column: `.program-body`'s `gap: 8px`, which
///   is `PROGRAM_DIVIDER` in `lib.rs` and the divider the arrangement already
///   leaves between the picture and the row. CSS's `gap` shorthand sets the row
///   gap and the column gap alike, so the body's own declaration states this
///   number for the across-the-bay direction too; nothing is invented for it.
/// - Between two stacked cells: [`size::PREVIEW_GAP`], `.previews`'s
///   `gap: 6px`, for exactly the same reading of the same shorthand — it is the
///   gap between two `.preview` cells, and the mock states one number for both
///   directions.
///
/// There is no third: the columns sit against the body's own edges, which are
/// already `.program-body`'s padding in from the card, and the cells are
pub fn program_body(body: Rect, canvas: (u32, u32)) -> Option<Body> {
    program_body_with_row_h(body, canvas, size::PREVIEW_ROW_H)
}

/// What the Program bay holds this frame, and where it holds it: the picture,
/// the four deck preview cells, and which way round the two were arranged.
///
/// Both halves are optional because the bay's two regions fold apart —
/// `console.html`: *"The picture is a sink ... The deck previews under it are
/// auditions of their own, so they stay when it goes."* So a bay with a picture
/// and no cells is the operator having folded the row, a bay with cells and no
/// picture is the operator having folded the picture, and [`program_bay`]
/// answers `None` where the bay itself is not on screen. The two are one value
/// for [`Body`]'s own reason: whoever placed the picture and whoever placed the
/// cells have to be one statement, and here they are one statement about one
/// solve as well.
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
    /// Which cell `p` is on, as a deck in [`DECK_LETTERS`] order.
    ///
    /// `None` for the ground between two cells, for anywhere else in the bay, and
    /// for a console whose preview row is folded away -- a cell that is not drawn
    /// is not one a press can be on.
    pub fn cell(&self, p: karakuri_layout::Point) -> Option<u8> {
        let at = Pos2::new(p.x, p.y);
        self.cells?
            .iter()
            .position(|cell| cell.contains(at))
            .map(|deck| deck as u8)
    }

    /// Which deck a carry let go at `p` lands on, or `None` where no cell of a deck
    /// that has a slot is under it.
    ///
    /// [`Mixer::dropped`]'s answer one bay over, and the same gesture:
    /// `console.html`'s *How a Set reaches a deck* names two sets of rectangles a
    /// release can land on, *"the four deck preview cells take a drop as well, and
    /// each names the deck its letter names"*. The Library bay is in the left pane
    /// and the mixer in the right, so a carry between them crosses the whole
    /// window; the cells are in the centre column, beside the list the Set came out
    /// of.
    ///
    /// A cell is an operand and never a choice. Nothing routes a cell — `A` is deck
    /// A whatever is loaded, which is ADR-0240 — so there is no second thing a
    /// release here could mean and no reading to take before it means the first.
    ///
    /// # `slots` is how many the deck has, and it is why this takes an argument
    /// where [`Mixer::dropped`] takes none
    ///
    /// The mixer walks the strips it drew and there is one per slot, so *a deck
    /// with no slot* is already a strip that is not there. The row is [`DECKS`]
    /// cells whatever the deck holds — a cell that vanished would move the other
    /// three, and the letter is the only thing naming a deck
    /// ([ADR-0170](../../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md))
    /// — so the fourth cell of a three-slot deck is a rectangle whose letter names
    /// no deck, and a release on it has nothing to load into. It answers `None`,
    /// which is the same refusal `3` gets from the keyboard and the same reading
    /// behind it: [`View::select`] is *"a deck the mixer has no strip for is
    /// refused"*, off [`View::mixer`]'s length, and `karakuri/src/main.rs`'s
    /// `pointed` prints it.
    ///
    /// That is not the refusal ADR-0265 forbids, and the two are worth keeping
    /// apart. What is not read is the deck's *residency* and the cell's *material*:
    /// a drop on a live deck asks for the load, and a cell drawing nothing because
    /// no engine has handed it a texture is a target like any other
    /// ([`View::previews`], where `None` is two states). What is read is whether
    /// the letter names a deck at all — the operand, not the answer.
    ///
    /// The count is the caller's for [`Mixer::dropped`]'s reason, one step further
    /// out: this type is the bay's *geometry*, derived from a solved layout and
    /// nothing else, and a slot count is a reading the console is handed per frame.
    pub fn dropped(&self, p: karakuri_layout::Point, slots: usize) -> Option<u8> {
        self.cell(p).filter(|deck| usize::from(*deck) < slots)
    }

    /// A press on a cell asks for nothing, and that is the decision rather than a
    /// gap.
    ///
    /// This used to answer `Operation::SetPreview` — the cell's deck, or the mix
    /// where the press was on the cell the output was already showing.
    /// [ADR-0240](../../../../docs/adr/0240-the-output-shows-the-mix-and-residency-keys-belong-to-the-mixer.md)
    /// retired that operation: the Program Picture always presents the master mix
    /// and the four cells always audition their own decks, drawn from
    /// `karakuri_engine::deck::Deck::slot_view` every frame, so there is nothing
    /// left for a press to swap. The cells are still the panel's —
    /// [`ProgramBay::owns`] and [`ProgramBay::cell`] are unchanged and
    /// `tests/preview_cells.rs` still holds the boundary arithmetic — because what
    /// a control claims is what it is drawn over, and a cell an operator can drag
    /// the row's boundary off has to be claimed whether or not a press on the
    /// middle of it asks for anything.
    ///
    /// A release on one asks for something, and that is not this sentence
    /// weakening. [`ProgramBay::dropped`] is where it is, and a press and a release
    /// are two moments: nothing is in hand at the press, so there is still nothing
    /// for it to ask for — the carry is what puts the second operand there
    /// ([ADR-0273](../../../../docs/adr/0273-the-carry-lands-on-two-sets-of-rectangles-and-wears-a-face.md)).
    ///
    /// Whether `p` is on any of the cells -- the union of the four, for
    /// [`crate::input`]'s rule 4.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.cell(p).is_some()
    }
}

/// The Program bay, arranged for the rectangle it solved to — the one
/// derivation of where the picture and the four cells go, and the only thing in
/// this crate that reads the bay's geometry to decide it.
///
/// [`picture_rect`], [`preview_rects`], [`rearrange`] and [`View::draw`] are
/// its four readers and none of them derives a rectangle of its own. That is
/// the rule [`fitted`] and [`preview_row`] already live by, one level up: the
/// picture and the cells move together or they overlap.
///
/// # The body is the bay's, and the bay is what does not move
///
/// The rectangle handed to [`program_body`] is the bay less the head painted
/// over it and less `.program-body`'s padding — not the `program-view` region,
/// which is what [`picture_rect`] used to inset. It has to be the bay, for the
/// reason ADR-0182 gives (the arrangement below spans both regions and the
/// divider between them) and for one more that only matters here: the bay's
/// rectangle does not depend on the bit this decision writes. The bay is
/// `Fixed(378)` over a flexible `program-view`, so what it can use is unbounded
/// whether or not the row is set aside
/// ([ADR-0174](../../../../docs/adr/0174-a-node-claims-only-what-its-visible-content-can-use.md)),
/// and the placement is therefore a fixed point after one write rather than
/// something that could chase itself around the solve. [`rearrange`] is where
/// that argument is finished.
///
/// # The guard rule, and it is the console's because the crate may not hold it
///
/// The body only arranges itself while the picture is there. A split can use
/// nothing when none of its children is laid out, so setting the row aside
/// while `program-view` is folded would leave the whole bay claiming zero and
/// the Program bay would vanish from the panel — and the manual promises the
/// opposite: the deck previews *"are auditions of their own, so they stay when
/// it goes"*.
/// [ADR-0183](../../../../docs/adr/0183-a-node-is-out-of-the-layout-for-two-reasons-and-they-are-two-bits.md)
/// says outright that this rule is the caller's, *"and a crate that does not
/// know what a picture is may not hold it"*. So it is here, and it is
/// structural rather than a check: with the picture folded there is nothing to
/// arrange around, the row is [`Placement::Below`] at its own height in the
/// region the arrangement gave it, and ADR-0174's round trip is exactly what it
/// was before this pass.
///
/// The other fold is the mirror of it and was already true: with the row folded
/// there are no cells to place, so the picture takes the body whole.
///
/// # Which bit is asked, and it is the operator's
///
/// [`is_collapsed`](karakuri_layout::Layout::is_collapsed) for each half and
/// [`visible`](karakuri_layout::Layout::visible) for the bay, which is
/// ADR-0183's two bits read apart on purpose. Asking `visible` of the row here
/// would be this function reading its own answer back: the row is set aside
/// exactly when this said *beside*, so the cells would vanish on the frame
/// after they moved. The ancestors are asked once, of the bay, where the second
/// bit cannot be.
///
/// `layout` must be solved, and it wants [`rearrange`] to have been run on it
/// this frame — with a stale bit and a folded picture the row's region is the
/// one place here that reads a rectangle the bit decides.
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

/// The Program bay's body: its rectangle less the head painted over the top of
/// it and less `.program-body`'s padding on all four sides.
///
/// The inset [`picture_rect`] used to take off the `program-view` region, off
/// the bay instead — and with the bottom padding taken off, which that one
/// could not: the 9 under the picture belonged to `deck-previews` when the
/// region was the box, and belongs to the body now that the bay is.
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

/// Arrange the Program bay for the rectangle it has, and tell the layout
/// what that means for the row: `deck-previews` is set aside where the cells
/// went beside the picture, and put back where they are under it.
///
/// Returns whether anything moved, which is a
/// [`Change::Rearranged`](crate::repaint::Change::Rearranged) and is the whole
/// of what a caller does with it.
///
/// # Where in the frame this goes, and why it is here rather than in the solve
///
/// The bit *has to be computed from a solved layout* — it is a function of the
/// bay's rectangle — and *writing it dirties the layout when it changes*. So
/// the order is forced: solve, decide, write, solve.
///
/// - A frame on which nothing moved does no work. Both solves are the flag
///   test `Layout::solve` opens with, and the write is
///   [`set_aside`](karakuri_layout::Layout::set_aside) with the value the node
///   already carries, which marks nothing dirty by construction (ADR-0183).
///   What is left is one [`program_bay`] — a dozen divisions and two fits, no
///   allocation — and no repaint is asked for, which is ADR-0164's
///   still-panel clause.
/// - The frame it does change solves to the new arrangement and not to the
///   previous one, because the second solve is after the write. That frame
///   costs two solves, and it is worth saying plainly rather than hiding:
///   a placement only changes when the bay's rectangle does, which is a window
///   resize or a drag on a boundary, and ADR-0210 does not budget what the
///   operator does.
/// - Nothing re-enters the solve. The value written is derived from the
///   bay's rectangle, and the bay is `Fixed(378)` over a flexible
///   `program-view`: what it can use is unbounded whether or not the row is
///   set aside, so the second solve gives the bay the rectangle the first one
///   did and asking again would write the same bit. One step, and it is a
///   fixed point rather than a loop — `tests/rearrange.rs` asserts that by
///   running this twice and watching the second one say nothing moved.
///
/// The one case where the bay's rectangle *does* depend on the bit is the one
/// the guard rule covers: with the picture folded the bay can use only the
/// row, and a row set aside would leave it able to use nothing. [`program_bay`]
/// never answers *beside* there, so that fixed point is the row's 72 and not
/// zero.
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
