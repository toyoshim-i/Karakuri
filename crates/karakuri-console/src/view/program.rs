use super::*;

// ---------------------------------------------------------------------------
// The Program bay
// ---------------------------------------------------------------------------

/// **A picture to draw in the Program bay: a texture somebody else rendered,
/// and where it goes.**
///
/// The id is `egui`'s, which means it has already been registered with an
/// [`egui_wgpu::Renderer`](crate::egui_wgpu::Renderer) — and that registration
/// needs a device, which is exactly what this crate does not have. So the
/// caller does it and hands the result over; this module draws an id and a
/// rectangle and knows nothing about either. It is the same seam the whole
/// crate is built on, one level in: `src/` reads and paints, and everything
/// that takes a device is the program's.
///
/// **The rectangle is passed rather than looked up**, and that is what makes
/// the pair checkable: whoever sized the texture and whoever placed it are the
/// same statement, so a texture sized from the window and drawn into the
/// picture's region cannot be written by accident. [`picture_rect`] is what a
/// caller derives both from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Picture {
    /// The registered texture. Whatever it holds is drawn as-is: the console
    /// tints it with nothing.
    pub id: egui::TextureId,
    /// Where to draw it, in the same logical pixels the arrangement is stated
    /// in — [`picture_rect`]'s answer for the frame this is being drawn on.
    pub rect: Rect,
}

/// **Where the picture goes**: the canvas's own shape, as large as the Program
/// bay's body leaves room for once the four deck previews have their places,
/// centred in what is left — below the bay head painted over the top of the
/// bay and inside `.program-body`'s padding.
///
/// `canvas` is what the Set renders at and what the deck is sized to: `--canvas`
/// in the product, which reaches a replay through `Record::Canvas` so a session
/// renders at the size the performance ran at. **Two dimensions rather than a
/// ratio**, and that is a conclusion rather than a habit. It is the shape
/// `karakuri_engine::present::letterbox(canvas, target)` and `Present::size`
/// already speak, so a caller hands over the number it built its `Present`
/// from rather than deriving a float on the way in — and a ratio derived at a
/// call site is exactly where `9.0 / 16.0` gets written for `16.0 / 9.0`,
/// which is a bug nothing on screen shows. It also carries its own
/// provenance: `(1280, 720)` reads as a canvas and `1.7777778` reads as a
/// number somebody typed. `src/` still takes no engine (ADR-0156) — this is a
/// pair of `u32`s, arriving the way every other value does.
///
/// `None` where there is no picture to draw, which is the manual's *"it is on
/// screen exactly when that sink is on — so there is no state where it is
/// hidden and still costing a pass"*: a caller that renders into this
/// rectangle records no pass at all when there is no rectangle. A folded
/// picture is the case that matters — see *The rectangle is the bay's now*
/// below, and [`program_bay`], which is where it is decided.
///
/// # The insets are the bay's own derivation, read backwards
///
/// The box is the **bay** less [`size::HEAD_H`] and one
/// [`size::PROGRAM_BODY_PAD`] on each of the four sides ([`bay_body`]), and
/// then less whatever arrangement the cells took out of it — a row and a
/// divider along the bottom, or a column and a divider down each side.
///
/// It reaches the mock's own numbers by the same arithmetic it always did.
/// The arrangement gives `program-view` 27 + 9 + 262 at the mock's width — bay
/// head, `.program-body`'s padding above the picture, and the picture itself —
/// and `deck-previews` 63 + 9 under it with the split's 8px divider between,
/// so the body less the row less the divider is that region less the head and
/// the top pad, to the pixel. The 9 under the picture in the CSS is the
/// divider and belongs to neither child; the 9 under the *row* is the body's
/// bottom padding, and it is the body's now rather than the row region's,
/// which is the one term that moved. `tests/program_body.rs`'s
/// `below_is_what_the_console_draws_today` is that agreement as an assertion.
///
/// At the narrowest console the mock will draw, that box is exactly
/// **466 x 262** — and 466 x 262 is 16:9 *to a quarter of a pixel* rather than
/// exactly. `.program-view` carries `aspect-ratio: 16/9`, so 466 wide is
/// **262.125** tall, and the arrangement transcribed the whole pixel the mock
/// rasterises it at. The box is therefore 1.778626 where the canvas is
/// 1.7777778, which is the whole of why the paragraph below exists.
///
/// # The rectangle is a whole number of pixels, and that is about resampling
///
/// A strict fit into that box gives 465.7778 x 262. **A caller's `physical`
/// rounds to whole texels, so the texture it then allocates is 466 wide** —
/// and the picture would be a 466-texel texture drawn into a 465.7778-wide
/// box, where every texel on screen is a fractional sample of its neighbours
/// instead of a blit. Of every region on this panel that is worst here: the
/// picture is a *preview of what is being captured*, and softening it is the
/// one thing it may not do. It also buys nothing — the texture's own ratio is
/// 466:262 either way, because `physical` rounded. **The fractional quarter
/// pixel is not more faithful to 16:9; it is the same texture, softened.**
///
/// Two smaller reasons, and they are second. Every number in this console is a
/// whole logical pixel because the mock is authored in whole ones —
/// [`preview_cells`] comes out 112 x 63 with no rounding at all because 466
/// happens to divide, not because a cell is exempt from this. And a box whose
/// extent is whole is a box the picture's edge lands on the pixel grid in,
/// which is what `.program-view`'s own hard-edged well is drawn as.
///
/// **What it costs, plainly:** the picture's ratio is then the mock's 1.778626
/// rather than exactly the canvas's. `Present::draw` is what absorbs the
/// difference and that is why it stays — see [`WHOLE_TEXTURE`]. Snapping does
/// not make the engine's letterbox redundant; it makes it sub-texel.
///
/// # The leftover is the console's ground, and a capture pays for it
///
/// Above the mock's narrowest the region is wider than the picture, and what
/// is beside the picture is the Program bay's card with nothing drawn on it.
/// [`Kind::Picture`] is where that is argued and where the cost to an operator
/// capturing a soloed window is written out, along with the `a` this console's
/// window has not got yet.
///
/// # The rectangle is the **bay's** now, and not this region's
///
/// It was this region's until the body started arranging itself. Beside the
/// picture the four cells stand in ground the `program-view` region owns — the
/// row is [`Layout::set_aside`](karakuri_layout::Layout::set_aside) there and
/// has no extent at all — so a picture inset out of this region and cells
/// taken off the bay would be two answers to *where does the picture go*, and
/// they would differ by two columns and a divider. **One derivation, and this
/// is one of its readers**: [`program_bay`] arranges the whole body once and
/// this is its picture. See [ADR-0182](../../../../docs/adr/0182-the-program-bays-body-arranges-itself-for-the-larger-picture.md).
///
/// **The `None` rule moved with it**, and that is the one line of this that is
/// not the same sentence it was. *A folded region keeps its rectangle and
/// loses its extent* was what said the picture is not on screen, and it does
/// not any more: the box is taken off the **bay**, which keeps its 378
/// whatever the picture does, so a folded picture would be handed a body and
/// fitted into it. [`program_bay`] asks the operator's fold by name and this
/// answers `None` from it — and the size test is still in there underneath,
/// on the fitted rectangle, for a bay with no room in it.
///
/// `layout` must be solved: `Layout::rect` refuses to answer from a dirty one.
pub fn picture_rect(layout: &karakuri_layout::Layout, canvas: (u32, u32)) -> Option<Rect> {
    program_bay(layout, canvas)?.picture
}

/// **The largest rectangle of `aspect` that fits inside `inside`, centred in
/// it, and a whole number of pixels in each direction.**
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
/// **`aspect` is two numbers and not a ratio**, for the reason
/// [`picture_rect`] gives at length, and it is deliberately **not** called
/// `canvas`: the picture's two numbers are the canvas's and a cell's are
/// [`PREVIEW_ASPECT`]'s, which is the mock's. One argument, two provenances,
/// each stated where it is passed.
///
/// # What is rounded and what is not
///
/// **The extent is rounded and the position is not.** The extent is what a
/// caller's `physical` turns into texels, so it is the half that decides
/// whether the picture is blitted or resampled — [`picture_rect`] is where
/// that argument is written out. The offset is left as the true centre,
/// because rounding it is a cell no longer centred in its track, and *centred*
/// is the other half of the rule ADR-0170 took. A region whose own origin is
/// fractional still samples fractionally, and that is the arrangement's
/// coordinate rather than this rule's to fix.
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

/// **The shape of one deck preview cell**, and it is the mock's number rather
/// than the canvas's: `.preview` carries `aspect-ratio: 16/9` in `style.css`,
/// in a `.previews` grid of `repeat(4, 1fr)` with a 6px gap.
///
/// Two numbers rather than a ratio for [`picture_rect`]'s reason, and passed
/// to the same [`fitted`] the picture goes through.
///
/// # It agrees with the canvas today by coincidence, and that is a pass rather
/// than a rename
///
/// An audition is the **same canvas** the picture shows, so a canvas that is
/// not 16:9 would letterbox inside a cell — a second fit inside a rectangle
/// `Present::draw` has already fitted, which is precisely what ADR-0170
/// rejected one level up. The honest number here is therefore the canvas's.
///
/// **The reason it is not the canvas's has changed, and the number has not.**
/// It used to be structural — *"making a cell canvas-aware means putting a
/// canvas on [`View`] and writing it per frame"*, and there was no canvas on
/// [`View`] to read. There is one now ([`View::canvas`]), so that reason is
/// spent; what holds the number is
/// [ADR-0182](../../../../docs/adr/0182-the-program-bays-body-arranges-itself-for-the-larger-picture.md)'s
/// own decision instead — a cell is the mock's shape and never the canvas's,
/// asserted by `a_cell_is_the_mocks_shape_and_never_the_canvass` — and making
/// it the canvas's is still a pass of its own, with the second fit above to
/// answer for.
const PREVIEW_ASPECT: (u32, u32) = (16, 9);

/// **What the Program bay arranges its body for when nobody has said what is
/// being rendered**: the mock's own picture, which is `.program-view`'s
/// `aspect-ratio: 16/9` in `style.css`.
///
/// A console with no engine behind it has no canvas — it is `src/` taking no
/// device, one number further on (ADR-0156) — and it still has to put four
/// cells somewhere, so [`View::canvas`] starts here and whoever owns the
/// `Present` writes the real thing over it every frame. It is two numbers
/// rather than a ratio for [`picture_rect`]'s reason, and it is the mock's
/// **picture** rather than [`PREVIEW_ASPECT`]'s cell: the two are the same 16
/// and 9 read off two different rules in the same stylesheet, and a `--canvas`
/// that is not 16:9 moves one of them and not the other.
pub const MOCK_CANVAS: (u32, u32) = (16, 9);

/// **Where the four deck previews go**: a row of [`DECKS`] cells inside the
/// `deck-previews` region, under the picture.
///
/// The mirror of [`picture_rect`] and it carries the same `None` rule for the
/// same reason — a folded row has a rectangle with no extent in it, so the
/// cells come out degenerate and there is nothing to draw or to render into.
/// **A fixed-size array rather than a `Vec`**: there are four decks and there
/// is no fifth, so a caller cannot ask for one and cannot forget one either.
///
/// # The insets are `.program-body`'s, and only three of the four
///
/// The row is the region inset by [`size::PROGRAM_BODY_PAD`] left, right and
/// **bottom**, and by nothing at the top. Read the Program bay's own
/// derivation in `lib.rs`: `deck-previews` is 63 + 9, the row of cells and the
/// padding under them. The 9 *above* the cells in the CSS is not in this
/// region at all — it is the split's 8px divider plus `program-view`'s own
/// bottom, which is why [`picture_rect`] takes nothing off the bottom.
///
/// At the narrowest console the mock will draw, that leaves 484 - 9 - 9 = 466
/// for four tracks and three [`size::PREVIEW_GAP`]s: (466 - 18) / 4 = **112**
/// wide, and 112 at 16:9 is **63** tall, which is exactly the height the row
/// has. The mock's cell, arrived at from the other end.
///
/// # A cell is 16:9 and centred in its track, and the alternative is written
/// down
///
/// The arrangement pins this region at 72 tall (`lib.rs`: fixed 72, minimum
/// 72, because a row of four cells at a fixed type size has nothing in it that
/// gets smaller). So a wider window widens the track and does **not** heighten
/// the row, and past the reference width a cell cannot both fill its track and
/// stay 16:9. One of the two has to give, and it is the track:
///
/// - **Taken:** the cell is 16:9, as large as the track's width and the row's
///   height both allow, and centred in its track. At the reference width that
///   is exactly 112 x 63 and fills the track; wider, it stays 63 tall with
///   ground either side. The texture then fills the cell exactly, so nothing
///   letterboxes twice and the cell is always the shape of what it shows.
///   **[`picture_rect`] now takes that same rule for the picture**, and the
///   two share [`fitted`] rather than stating it twice.
/// - **Rejected:** fill the track and letterbox the texels inside it. That
///   keeps the row looking like a grid at every width, and pays for it by
///   stretching the cell away from the shape of its picture — a 16:9 audition
///   in a 200x63 well, with bars the console has drawn itself inside a
///   rectangle the engine already fitted. Two fits for one question, which is
///   the thing `WHOLE_TEXTURE` refuses one level up.
///
/// # The row is one of two places the cells go, and this asks which
///
/// **It insets the `deck-previews` region no longer**, and it cannot: beside
/// the picture that region has no extent at all — it is
/// [`Layout::set_aside`](karakuri_layout::Layout::set_aside), which is exactly
/// what leaves the bay's width to the picture — so a cell derived from it
/// would be `None` at every window past the crossover, and deck A's audition
/// would stop being sized, stop being drawn and stop keeping the window's loop
/// awake. The cells come through [`program_bay`] like the picture does and
/// like the drawing does: **one derivation, and every reader of it reads the
/// same frame's answer.**
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
/// **One call site now, and it is the one case where the row is the answer**:
/// [`program_bay`]'s guard branch, where the picture is folded away and the
/// row is the whole of the bay. Everywhere else the cells come off the bay's
/// body through [`program_body`], because everywhere else there is a picture
/// for them to be arranged around — and the two agree to the pixel where they
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

/// **[`DECKS`] cells side by side across `row`**, each [`PREVIEW_ASPECT`] and
/// centred in its track.
///
/// The row of the mock, with nothing said about where the row is: that is
/// [`preview_cells`]'s inset off the `deck-previews` region, and
/// [`program_body`]'s strip along the bottom of the bay's body. **Two call
/// sites for one row**, and they have to agree exactly — the second one is the
/// same row in the same place, arrived at from the bay rather than from the
/// region, and a second copy of this arithmetic is where the two would drift.
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

/// **How much of a cell the caption takes**: `.cell`'s gap and `.caption`'s
/// own height, which is the band under every image and is never inside one.
///
/// One function rather than the sum written three times — [`preview_row`],
/// [`beside`] and [`caption_of`] all need it and a second copy of it is where
/// a caption would land over the picture it labels.
fn caption_band() -> f32 {
    size::PREVIEW_CAPTION_GAP + size::PREVIEW_CAPTION_H
}

/// `slot` with the caption band taken off the bottom, which is the box an
/// image is fitted into.
fn above_caption(slot: Rect) -> Rect {
    Rect::from_min_max(slot.min, Pos2::new(slot.max.x, slot.max.y - caption_band()))
}

/// **Where a cell's caption goes**: directly under the image, the image's own
/// width, one [`size::PREVIEW_CAPTION_GAP`] below it and
/// [`size::PREVIEW_CAPTION_H`] tall.
///
/// Derived from the image rather than carried beside it in [`ProgramBay`], for
/// the reason [`Body`] gives about the picture and the cells: the two are one
/// statement. A caption that could be handed in separately is a caption that
/// could be handed in stale, and this way there is one rectangle in the world
/// and the label is a function of it. It is also what keeps
/// [`preview_rects`]'s answer the **image** — the engine sizes a texture from
/// that rectangle, and a cell rectangle that quietly included the caption
/// would put texels over the letter.
///
/// The image's width and not the track's: the image is centred in its track
/// ([ADR-0170](../../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md)),
/// and a caption starting at the track's left edge would sit off under the
/// ground beside the cell it names.
///
/// # *Cell* means two things here, and it is named rather than renamed
///
/// The mock's `.cell` is the image **and** the caption — that is what
/// [`size::PREVIEW_ROW_H`] measures — while [`ProgramBay::cells`] and
/// [`preview_rects`] answer the **images**, because a texture is sized from
/// one and a rectangle that quietly included the caption would put texels over
/// the letter. Renaming the field is a ripple through six test files and the
/// program's frame path, so the clash is written down here instead
/// ([`docs/contributing.md`](../../../../docs/contributing.md) §4
/// is the rule it is in tension with, and this is the report rather than the
/// fix).
pub fn caption_of(image: Rect) -> Rect {
    Rect::from_min_max(
        Pos2::new(image.min.x, image.max.y + size::PREVIEW_CAPTION_GAP),
        Pos2::new(image.max.x, image.max.y + caption_band()),
    )
}

/// **Which way round the Program bay's body is arranged.**
///
/// Not a state and not a setting: [`program_body`] answers it from the
/// rectangle it is given, every time it is asked, and nothing stores it. See
/// that function for the decider and for why it is a decision rather than a
/// preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// **The mock's own**: the picture across the top, the four cells in a row
    /// under it. The default, and what a tie gives.
    Below,
    /// The picture in the middle, two cells down the left and two down the
    /// right. What a bay wider than it is tall gets.
    Beside,
}

/// **Where everything in the Program bay's body goes**: the picture, and the
/// four deck preview cells.
///
/// One value rather than two calls, for [`Picture`]'s own reason: whoever
/// placed the picture and whoever placed the cells are then one statement, so
/// a picture drawn for one arrangement and cells drawn for the other cannot be
/// written by accident. That is not a hypothetical here — the two arrangements
/// put the cells in different halves of the bay, so the failure would be four
/// thumbnails over the top of the picture rather than a rectangle a few pixels
/// out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Body {
    /// Which arrangement won, carried because the caller cannot derive it from
    /// the rectangles without re-running the decider — and re-running it is
    /// the second answer this value exists to prevent.
    pub placement: Placement,
    /// The picture: the canvas's shape, as large as the arrangement leaves
    /// room for, centred in what is left ([`fitted`]).
    pub picture: Rect,
    /// The four cells, in [`DECK_LETTERS`] order — **always four, and always
    /// in that order**. See [`program_body`] for why a cell does not move when
    /// the deck behind it stops.
    pub cells: [Rect; DECKS],
}

/// **How the Program bay's body arranges itself in the rectangle it has**: the
/// picture and the four deck preview cells, either the mock's way or down the
/// sides, whichever leaves the picture larger.
///
/// `body` is the bay **less its head and less `.program-body`'s padding** —
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
/// ground. On a wide bay that leftover is **ground down each side**: at a
/// 1920-wide window the body is 1396 x 333 and the picture is 466 x 262, so
/// 930 pixels of the bay's width are empty and the four cells are 63 tall in a
/// row under it. Putting the cells in that ground is what lets the picture take
/// the height instead.
///
/// # The decider is the picture's size, and nothing else
///
/// **Whichever arrangement gives the larger picture wins, and a tie goes to
/// [`Placement::Below`]**, which is the mock's. Both are computed and their
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
/// **Three worked cases, and the arithmetic is in `tests/program_body.rs`:**
///
/// - **The mock's narrowest**, 466 x 333. Below gives 466 x 262. Beside cannot
///   be drawn at all: two columns and their gaps come to 597, which is more
///   than the body is wide, so the picture's box is negative. Below wins
///   because it is the only one there is.
/// - **A 1920 window**, 1396 x 333. Below gives 466 x 262 — 122,092 texels.
///   Beside gives **592 x 333**, which is 197,136, and the picture is **61%
///   larger**. Beside wins.
/// - **800 wide**, where below still wins: beside's picture would be 202 x 114.
///   The flip is at **1064**, one pixel of body width, and there is exactly one
///   of them — below's picture stops growing with the width at 466 and beside's
///   never shrinks, so the two curves cross once and never again.
///
/// # A side column is as wide as two stacked cells, which makes it the
/// height's answer and not the width's
///
/// A column holds [`PER_COLUMN`] cells stacked with one [`size::PREVIEW_GAP`]
/// between them, so a cell is `(H - gap) / 2` tall and the column is that at
/// [`PREVIEW_ASPECT`] — **a function of the body's height alone**. Reading it
/// off the width instead is the mistake worth naming: the column would grow
/// with the very width it is competing for, and beside would never win at any
/// width — an arrangement that exists in the source and never on the screen.
///
/// Two things had to be checked about this rule and both hold:
///
/// - **It is not so greedy that beside never wins.** The column does not follow
///   the width, so widening the bay adds the whole increment to the picture's
///   box; below's picture is capped by the height it has *after* the row and
///   the gap come off, and beside's by the whole height. Beside therefore wins
///   at every width past the crossover and the crossover exists at every
///   height — at the mock's 333 it is a body 1064 wide, which is a 1588-wide
///   window, well inside an ordinary desktop.
/// - **It is greedy in the other direction, and that is a real cost rather
///   than a caveat.** The column follows the height, so a bay dragged taller
///   widens both columns while the picture's box is what pays for them: at 1396
///   wide the picture beside peaks at a body 391 tall and shrinks after it,
///   below's grows with every pixel, and the two cross at **427** — a Program
///   bay 472 tall, which an operator can drag to. So this arrangement is the
///   answer for a bay that is **wide and short**, and the cells go back under
///   the picture when it stops being short. It is one flip in each direction
///   and not a flicker — beside's picture is single-peaked in the height and
///   below's is monotone — and `tests/program_body.rs` sweeps both axes rather
///   than taking that on trust. The corner that follows from the same rule:
///   **a bay dragged to its own minimum of 200 goes beside at every width**,
///   the mock's narrowest included, because a body 155 tall has only 84 left
///   for the picture once the row and the divider come off.
/// - **It is not so mean that the cells are unreadable.** A cell beside the
///   picture is `(333 - 6) / 2 = 163` tall against the row's **63**, so the
///   arrangement that takes the cells out of the row makes each of them larger
///   rather than smaller. At the Program bay's own minimum height the body is
///   155 and a cell is still 73, which is more than the mock's row gives at any
///   width at all.
///
/// # Which cell goes where, and why none of them moves
///
/// **A and B down the left, C and D down the right**, each column read top to
/// bottom — the row's own left-to-right order, folded in half, so an operator
/// who knows where `C` was in the row finds it at the top of the other side
/// rather than somewhere new.
///
/// **With fewer than four decks running nothing fills and nothing shifts.**
/// The mock's head reads *previews 3 of 4* and
/// [ADR-0170](../../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md)
/// is the answer: a cell is drawn whether or not a deck is behind it, because
/// *an empty cell is what off looks like, not a stand-in for a full one*. So
/// the question *does the left column fill first, or do they alternate* has a
/// third answer, and it is the one that keeps the letters meaning something:
/// **a cell's place is its deck's, not its turn's.** `C · no slot` sits at the top
/// of the right column whether or not C is running, and turning B off does not
/// slide C into B's place — the letter is the only thing naming a deck, and a
/// label that moves when a neighbour stops is a label an operator cannot point
/// at. This function is handed no liveness at all, which is that rule as a
/// signature.
///
/// # The gaps are the mock's, and there are two of them rather than three
///
/// - **Between the picture and a column**: `.program-body`'s `gap: 8px`, which
///   is `PROGRAM_DIVIDER` in `lib.rs` and the divider the arrangement already
///   leaves between the picture and the row. CSS's `gap` shorthand sets the row
///   gap and the column gap alike, so the body's own declaration states this
///   number for the across-the-bay direction too; nothing is invented for it.
/// - **Between two stacked cells**: [`size::PREVIEW_GAP`], `.previews`'s
///   `gap: 6px`, for exactly the same reading of the same shorthand — it is the
///   gap between two `.preview` cells, and the mock states one number for both
///   directions.
///
/// There is no third: the columns sit against the body's own edges, which are
/// already `.program-body`'s padding in from the card, and the cells are
pub fn program_body(body: Rect, canvas: (u32, u32)) -> Option<Body> {
    program_body_with_row_h(body, canvas, size::PREVIEW_ROW_H)
}

/// Dynamic row height version of program_body
pub fn program_body_with_row_h(body: Rect, canvas: (u32, u32), row_h: f32) -> Option<Body> {
    match (below(body, canvas, row_h), beside(body, canvas, row_h)) {
        (Some(below), Some(beside)) if area(beside.picture) > area(below.picture) => Some(beside),
        (Some(below), _) => Some(below),
        (None, beside) => beside,
    }
}

/// How many texels a rectangle is, which is the whole of the decider.
///
/// **Area rather than width or height**, and that is the one of the three that
/// answers the question being asked: the picture is a preview of what is being
/// captured, so what an operator gets more of is pixels. Comparing widths would
/// hand the bay to whichever arrangement is wider at a height where it is also
/// shorter.
fn area(rect: Rect) -> f32 {
    rect.width() * rect.height()
}

/// **The mock's arrangement**: the picture across the top, the four cells in a
/// row along the bottom.
///
/// The row is `row_h` tall, along `.program-body`'s bottom edge;
/// the picture takes what is left above it, less one `PROGRAM_DIVIDER`.
fn below(body: Rect, canvas: (u32, u32), row_h: f32) -> Option<Body> {
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

/// **The other arrangement**: two cells down the left, two down the right, and
/// the picture in the middle.
///
/// Preserves the preview cell size from the row arrangement (ADR-0239).
fn beside(body: Rect, canvas: (u32, u32), row_h: f32) -> Option<Body> {
    let (aw, ah) = (
        PREVIEW_ASPECT.0.max(1) as f32,
        PREVIEW_ASPECT.1.max(1) as f32,
    );
    // `row_h` is a whole cell — the image and the caption band under it — so
    // the image is what is left when the band comes off, exactly as it is in
    // the row. A cell beside the picture carries its caption too: the letter
    // is the only thing naming a deck and it does not stop naming one because
    // the bay went wide.
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

/// One arrangement, or `None` where it cannot be drawn.
///
/// [`positive`]'s rule over the whole arrangement rather than over one
/// rectangle, because the two halves are one answer: a body that holds the
/// picture and has no room for a cell is not this arrangement with a cell
/// missing, it is the other arrangement's turn.
fn drawable(placement: Placement, picture: Rect, cells: [Rect; DECKS]) -> Option<Body> {
    match positive(picture) && cells.iter().copied().all(positive) {
        true => Some(Body {
            placement,
            picture,
            cells,
        }),
        false => None,
    }
}

/// **What the Program bay holds this frame, and where it holds it**: the
/// picture, the four deck preview cells, and which way round the two were
/// arranged.
///
/// Both halves are optional because **the bay's two regions fold apart** —
/// `console.html`: *"The picture is a sink ... The deck previews under it are
/// auditions of their own, so they stay when it goes."* So a bay with a
/// picture and no cells is the operator having folded the row, a bay with
/// cells and no picture is the operator having folded the picture, and
/// [`program_bay`] answers `None` where the bay itself is not on screen. The
/// two are one value for [`Body`]'s own reason: whoever placed the picture and
/// whoever placed the cells have to be one statement, and here they are one
/// statement about one solve as well.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgramBay {
    /// Which way round the body was arranged. **[`Placement::Below`] whenever
    /// there is no picture**, which is the guard rule below written into the
    /// value rather than left to the caller.
    pub placement: Placement,
    /// Where the picture goes, or `None` where the operator folded it away or
    /// the bay has no room for it — [`picture_rect`]'s answer.
    pub picture: Option<Rect>,
    /// Where the four cells go, in [`DECK_LETTERS`] order, or `None` where the
    /// operator folded the row away — [`preview_rects`]'s answer.
    pub cells: Option<[Rect; DECKS]>,
}

impl ProgramBay {
    /// Which cell `p` is on, as a deck in [`DECK_LETTERS`] order.
    ///
    /// `None` for the ground between two cells, for anywhere else in the bay,
    /// and for a console whose preview row is folded away -- a cell that is
    /// not drawn is not one a press can be on.
    pub fn cell(&self, p: karakuri_layout::Point) -> Option<u8> {
        let at = Pos2::new(p.x, p.y);
        self.cells?
            .iter()
            .position(|cell| cell.contains(at))
            .map(|deck| deck as u8)
    }

    /// **Which deck a carry let go at `p` lands on**, or `None` where no cell
    /// of a deck that has a slot is under it.
    ///
    /// [`Mixer::dropped`]'s answer one bay over, and the same gesture:
    /// `console.html`'s *How a Set reaches a deck* names **two** sets of
    /// rectangles a release can land on, *"the four deck preview cells take a
    /// drop as well, and each names the deck its letter names"*. The Library
    /// bay is in the left pane and the mixer in the right, so a carry between
    /// them crosses the whole window; the cells are in the centre column,
    /// beside the list the Set came out of.
    ///
    /// **A cell is an operand and never a choice.** Nothing routes a cell —
    /// `A` is deck A whatever is loaded, which is ADR-0240 — so there is no
    /// second thing a release here could mean and no reading to take before
    /// it means the first.
    ///
    /// # `slots` is how many the deck has, and it is why this takes an
    /// argument where [`Mixer::dropped`] takes none
    ///
    /// The mixer walks the strips it drew and there is one per slot, so *a
    /// deck with no slot* is already a strip that is not there. **The row is
    /// [`DECKS`] cells whatever the deck holds** — a cell that vanished would
    /// move the other three, and the letter is the only thing naming a deck
    /// ([ADR-0170](../../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md))
    /// — so the fourth cell of a three-slot deck is a rectangle whose letter
    /// names no deck, and a release on it has nothing to load into. It
    /// answers `None`, which is the same refusal `3` gets from the keyboard
    /// and the same reading behind it: [`View::select`] is *"a deck the mixer
    /// has no strip for is refused"*, off [`View::mixer`]'s length, and
    /// `karakuri/src/main.rs`'s `pointed` prints it.
    ///
    /// **That is not the refusal ADR-0265 forbids**, and the two are worth
    /// keeping apart. What is not read is the deck's *residency* and the
    /// cell's *material*: a drop on a live deck asks for the load, and a cell
    /// drawing nothing because no engine has handed it a texture is a target
    /// like any other ([`View::previews`], where `None` is two states). What
    /// is read is whether the letter names a deck at all — the operand, not
    /// the answer.
    ///
    /// The count is the caller's for [`Mixer::dropped`]'s reason, one step
    /// further out: this type is the bay's *geometry*, derived from a solved
    /// layout and nothing else, and a slot count is a reading the console is
    /// handed per frame.
    pub fn dropped(&self, p: karakuri_layout::Point, slots: usize) -> Option<u8> {
        self.cell(p).filter(|deck| usize::from(*deck) < slots)
    }

    /// **A press on a cell asks for nothing, and that is the decision rather
    /// than a gap.**
    ///
    /// This used to answer `Operation::SetPreview` — the cell's deck, or the
    /// mix where the press was on the cell the output was already showing.
    /// [ADR-0240](../../../../docs/adr/0240-the-output-shows-the-mix-and-residency-keys-belong-to-the-mixer.md)
    /// retired that operation: the Program Picture always presents the master
    /// mix and the four cells always audition their own decks, drawn from
    /// `karakuri_engine::deck::Deck::slot_view` every frame, so there is
    /// nothing left for a press to swap. **The cells are still the panel's**
    /// — [`ProgramBay::owns`] and [`ProgramBay::cell`] are unchanged and
    /// `tests/preview_cells.rs` still holds the boundary arithmetic — because
    /// what a control claims is what it is drawn over, and a cell an operator
    /// can drag the row's boundary off has to be claimed whether or not a
    /// press on the middle of it asks for anything.
    ///
    /// **A release on one asks for something, and that is not this sentence
    /// weakening.** [`ProgramBay::dropped`] is where it is, and a press and a
    /// release are two moments: nothing is in hand at the press, so there is
    /// still nothing for it to ask for — the carry is what puts the second
    /// operand there ([ADR-0273](../../../../docs/adr/0273-the-carry-lands-on-two-sets-of-rectangles-and-wears-a-face.md)).
    ///
    /// Whether `p` is on any of the cells -- the union of the four, for
    /// [`crate::input`]'s rule 4.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.cell(p).is_some()
    }
}

/// **The Program bay, arranged for the rectangle it solved to** — the one
/// derivation of where the picture and the four cells go, and the only thing
/// in this crate that reads the bay's geometry to decide it.
///
/// [`picture_rect`], [`preview_rects`], [`rearrange`] and [`View::draw`] are
/// its four readers and none of them derives a rectangle of its own. That is
/// the rule [`fitted`] and [`preview_row`] already live by, one level up:
/// **the picture and the cells move together or they overlap.**
///
/// # The body is the bay's, and the bay is what does not move
///
/// The rectangle handed to [`program_body`] is the **bay** less the head
/// painted over it and less `.program-body`'s padding — not the `program-view`
/// region, which is what [`picture_rect`] used to inset. It has to be the bay,
/// for the reason ADR-0182 gives (the arrangement below spans both regions and
/// the divider between them) and for one more that only matters here: **the
/// bay's rectangle does not depend on the bit this decision writes.** The bay
/// is `Fixed(378)` over a flexible `program-view`, so what it can use is
/// unbounded whether or not the row is set aside
/// ([ADR-0174](../../../../docs/adr/0174-a-node-claims-only-what-its-visible-content-can-use.md)),
/// and the placement is therefore a fixed point after one write rather than
/// something that could chase itself around the solve. [`rearrange`] is where
/// that argument is finished.
///
/// # The guard rule, and it is the console's because the crate may not hold it
///
/// **The body only arranges itself while the picture is there.** A split can
/// use nothing when none of its children is laid out, so setting the row aside
/// while `program-view` is folded would leave the whole bay claiming zero and
/// the Program bay would vanish from the panel — and the manual promises the
/// opposite: the deck previews *"are auditions of their own, so they stay when
/// it goes"*.
/// [ADR-0183](../../../../docs/adr/0183-a-node-is-out-of-the-layout-for-two-reasons-and-they-are-two-bits.md)
/// says outright that this rule is the caller's, *"and a crate that does not
/// know what a picture is may not hold it"*. So it is here, and it is
/// structural rather than a check: with the picture folded there is nothing to
/// arrange around, the row is [`Placement::Below`] at its own height in the
/// region the arrangement gave it, and ADR-0174's round trip is exactly what
/// it was before this pass.
///
/// The other fold is the mirror of it and was already true: with the row
/// folded there are no cells to place, so the picture takes the body whole.
///
/// # Which bit is asked, and it is the operator's
///
/// [`is_collapsed`](karakuri_layout::Layout::is_collapsed) for each half and
/// [`visible`](karakuri_layout::Layout::visible) for the bay, which is
/// ADR-0183's two bits read apart on purpose. **Asking `visible` of the row
/// here would be this function reading its own answer back**: the row is set
/// aside exactly when this said *beside*, so the cells would vanish on the
/// frame after they moved. The ancestors are asked once, of the bay, where the
/// second bit cannot be.
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

/// **The `solo` pill in the Program bay's head, derived** -- the one control
/// this console has in a bay head, and the panel's route into *Solo a region*.
///
/// `docs/manual/console.html` draws it and says what it does in as many
/// words: *"Solo the program view: the panel folds away and only the picture
/// is left, which is also how you capture this window."* So the region it
/// names is `program-view` and not the bay around it, and that is read off the
/// page rather than chosen here.
///
/// # It is two operations and no toggle, which is [`Outputs::op`]'s rule
///
/// A solo has an undo and the vocabulary spells the two apart --
/// `karakuri_operation::Operation::Solo`'s `region` is `None` for *undo the
/// solo*, *"explicit rather than a toggle: the caller says which way"* -- so
/// [`Op::Solo`] and [`Op::Unsolo`] are what a press asks for and the choosing
/// between them is the affordance. [`ProgramHead::soloed`] is what it is
/// chosen from, and it is read back out of the layout rather than remembered.
///
/// **It undoes a solo it did not make.** `Layout::solo` collapses everything
/// off the soloed node's path, so the only solo this pill is still drawn under
/// is one on `program-view` itself or on something enclosing it -- every other
/// solo takes the Program bay off the screen and there is no pill to press.
/// Which is the same sentence `Op::Unsolo` already carries: what an unsolo
/// undoes is *the* solo, because there is only ever one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgramHead {
    /// **The control**: the `solo` capsule, which is what a press has to land
    /// in. [`head_pills`]'s answer for it, so it is the rectangle
    /// [`bay_head`] painted.
    pub solo: Rect,
    /// The node it solos: the picture, `program-view`.
    pub id: NodeId,
    /// Whether anything is soloed -- `layout.is_soloed()`, read here.
    pub soloed: bool,
}

impl ProgramHead {
    /// **What a press on the pill asks for.** See the type's own
    /// documentation for why it is two operations rather than one that
    /// toggles.
    pub fn op(&self) -> Op {
        match self.soloed {
            true => Op::Unsolo,
            false => Op::Solo(self.id),
        }
    }

    /// Whether `p` is on the control.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.solo.contains(Pos2::new(p.x, p.y))
    }
}

/// **The Program bay's head, derived**: the one pill in it, and the node it
/// acts on.
///
/// `None` where there is no pill to press -- before the first frame, with the
/// bay folded or off a solo somewhere else, or in a bay too short to hold its
/// own head. That is [`outputs`]'s rule stated on a capsule instead of on a
/// chip: a rectangle with nothing in it is not something to paint or to click.
///
/// **The pill is found by name rather than by position.** [`REGIONS`] is where
/// a bay's controls are listed, and this reads [`SOLO_PILL`] out of the
/// Program bay's entry -- so a second pill added to that head moves this one
/// along and nothing here has to be told.
///
/// **A second pill was duly added and this capsule duly moved.** The class pill
/// ([`mcp_pill`]) sits to `solo`'s right and is not the same width in its two
/// states, which is why this now takes an opening: [`head_capsule`] lays the
/// whole head out under that opening and hands back the one capsule asked for,
/// so the two pills cannot be laid out against two different states.
///
/// `layout` must be solved: [`karakuri_layout::Layout::rect`] refuses to
/// answer from a dirty one. `ctx` is asked for the type, because a `.pill` is
/// as wide as the word in it.
///
/// # What it costs the operator, and it is 0.75 of a pixel
///
/// **This is one of the first two controls on the console that do not clear
/// every boundary's grab** — the four deck preview cells under it are the
/// other, and [`ProgramBay::preview`] carries theirs. The number is worth
/// having in front of you rather than in a test alone. A bay head is [`size::HEAD_H`] = 27 and a `.pill` is
/// [`size::PILL_H`] = 16.5, centred, so there is (27 - 16.5) / 2 = **5.25** of
/// head above the capsule -- against a [`crate::panel::GRAB`] of **6**. The
/// Program bay is the first child of the centre column, so its top edge is the
/// body row's, and the boundary between the transport row and the body grabs
/// six pixels past it.
///
/// So the top **0.75** of the capsule is the boundary's and the other 15.75 is
/// the panel's. [`crate::input`]'s rule 3 is what decides that and it decides
/// it the same way every time: the boundary gets first refusal, there is no
/// case where both think they are dragging, and the hazard that rule was
/// written for does not arise. What is lost is the sliver, and
/// `tests/solo_pill.rs` is what states the number and fails if it grows.
///
/// **The three things that could change it are all somebody else's**: the
/// head's height and the pill's box are `docs/manual/console.html`'s, and
/// `GRAB` is the rule's. This function draws the control where the page puts
/// it.
pub fn program_head(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    open: Open,
) -> Option<ProgramHead> {
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // [`outputs`]: a press before the first frame is a press on a control that
    // has never been drawn.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let bay = layout.find("program")?;
    // The bay itself, its column folded around it, or a solo somewhere else:
    // one question for every ancestor, which is [`program_bay`]'s own guard.
    if !layout.visible(bay) {
        return None;
    }
    let id = layout.find("program-view")?;
    let head = head_of(region("program")?)?;
    // A head clipped to a bay shorter than 27 has nowhere to put a 16.5
    // capsule, and a capsule half out of the head is not one to press --
    // [`head_capsule`]'s own guard, which is why it is not repeated here.
    let solo = head_capsule(ctx, to_egui(layout.rect(bay)), &head, open, SOLO_PILL)?;
    Some(ProgramHead {
        solo,
        id,
        soloed: layout.is_soloed(),
    })
}

/// **The Program bay's body**: its rectangle less the head painted over the
/// top of it and less `.program-body`'s padding on all four sides.
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

/// **Arrange the Program bay for the rectangle it has, and tell the layout
/// what that means for the row**: `deck-previews` is set aside where the cells
/// went beside the picture, and put back where they are under it.
///
/// Returns **whether anything moved**, which is a
/// [`Change::Rearranged`](crate::repaint::Change::Rearranged) and is the whole
/// of what a caller does with it.
///
/// # Where in the frame this goes, and why it is here rather than in the solve
///
/// The bit *has to be computed from a solved layout* — it is a function of the
/// bay's rectangle — and *writing it dirties the layout when it changes*. So
/// the order is forced: **solve, decide, write, solve.**
///
/// - **A frame on which nothing moved does no work.** Both solves are the flag
///   test `Layout::solve` opens with, and the write is
///   [`set_aside`](karakuri_layout::Layout::set_aside) with the value the node
///   already carries, which marks nothing dirty by construction (ADR-0183).
///   What is left is one [`program_bay`] — a dozen divisions and two fits, no
///   allocation — and no repaint is asked for, which is ADR-0164's
///   still-panel clause.
/// - **The frame it does change solves to the new arrangement and not to the
///   previous one**, because the second solve is after the write. That frame
///   costs **two solves**, and it is worth saying plainly rather than hiding:
///   a placement only changes when the bay's rectangle does, which is a window
///   resize or a drag on a boundary, and ADR-0210 does not budget what the
///   operator does.
/// - **Nothing re-enters the solve.** The value written is derived from the
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

/// **One deck preview cell's image**: the mock's `.preview`, and whatever is
/// behind it.
///
/// **Nothing is written on it.** The letter and the state word are the
/// caption's, under the image — [`caption_into`] — and the reason is the one
/// the mock states beside the rule: a letter laid over the material is
/// unreadable exactly when the deck is live and the material is bright, which
/// is the one moment the row is read fastest. The image carries material or it
/// carries the bare well, and either way it carries nothing this crate wrote.
///
/// The cell is painted whether or not a texture was handed in, because *no
/// slot behind it* is a state rather than a thing not built yet — the module
/// documentation is where that argument is written out.
///
/// **A cell is not dark because its deck is off air.** Every slot is drawn
/// into its own target on every frame, so a parked or warming deck has a
/// picture here exactly as a Live one does; what the residency decides is
/// whether that slot reaches the *mix*, which is the picture above and the
/// mixer strip beside.
///
/// Term for term from `.preview` in `style.css`:
///
/// - `background: var(--c-well)` — `pal.well`, and it is the ground the
///   texture is drawn over rather than a fallback for one, so a texture with
///   any transparency in it reads as a recess and not as a hole.
/// - `border-radius: 7px` — [`size::PREVIEW_RADIUS`].
/// - `box-shadow: inset 0 0 0 1px var(--c-hair)` — a 1px stroke on the
///   **inside** of the box in `pal.hair`, drawn last so it sits over the
///   texture exactly as an inset shadow sits over a background image.
pub(super) fn preview(ui: &Ui, pal: &Palette, cell: Rect, picture: Option<Picture>) {
    let radius = CornerRadius::same(size::PREVIEW_RADIUS as u8);
    // Clipped to the cell for the reason the picture is clipped to its region:
    // the rectangle in `picture` came from outside, and a stale one is a
    // thumbnail painted across the bay rather than a wrong thumbnail.
    let painter = ui.painter().with_clip_rect(cell);
    painter.rect_filled(cell, radius, pal.well);

    if let Some(picture) = picture {
        painter.image(picture.id, picture.rect, WHOLE_TEXTURE, Color32::WHITE);
    }
    // **One stroke, `pal.hair`, on every cell.** A cell used to wear a
    // `--c-mint` ring when the output was auditioning the deck it is drawn
    // for, and the ring went with the operation: ADR-0240 makes the picture
    // the master mix always and every cell its own deck's monitor always, so
    // there is no longer a fifth fact for a mark to carry and no cell that is
    // more *on* than the other three. That is the mock's own state restored —
    // `style.css` gives `.preview` one `inset 0 0 0 1px var(--c-hair)` and no
    // second colour.
    painter.rect_stroke(
        cell,
        radius,
        Stroke::new(size::HAIRLINE, pal.hair),
        StrokeKind::Inside,
    );
}

/// **The word a cell's caption gives for what the cell is showing.**
///
/// Three, because three is what the pair of values this reads distinguishes
/// and drawing a fourth would be drawing a state the program cannot be in.
///
/// - [`PREVIEW_MATERIAL`] — there is a deck slot behind this cell and the
///   image is that slot's own target. It says nothing about residency: a
///   parked deck's still and a live deck's frame are the same word, which is
///   [ADR-0258](../../../../docs/adr/0258-the-look-comes-before-the-fader-so-a-cell-draws-every-slot-and-says-which-nothing-it-is.md).
/// - [`PREVIEW_OVERLOADED`] — there is a slot behind this cell and it has
///   **stopped updating**: the version in it costs more than one frame may, so
///   the engine skips its step and its draw and the image is the last frame it
///   made (ADR-0316). **It outranks `material` and does not replace what the
///   cell shows**, which is the whole shape of the decision — the image stays,
///   because blanking it would be indistinguishable from an empty slot, and
///   the word is what separates a still from a preview
///   ([ADR-0269](../../../../docs/adr/0269-a-slot-that-is-drawn-is-stepped-and-a-preview-runs-at-the-rooms-tempo.md)).
///   It is not residency either: this cell reads the same word on air and off.
/// - [`PREVIEW_NO_SLOT`] — there is no slot behind this cell at all: a deck of
///   fewer slots than there are cells, or a console with no engine behind it.
///   **It wins over the mark**, because a mark about a slot that is not there
///   is about nothing: the flag crosses the seam per cell and a caller writing
///   one beside no picture is saying two things at once, of which this draws
///   the one that is about the cell.
///
/// **Not `empty` and not `off`, and both of those are worth naming.** The
/// mock's D cell said `D · off` when
/// [ADR-0170](../../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md)
/// landed and this function's ancestor copied the word; the page has since
/// moved and neither word is what the cell distinguishes. *Off* was residency,
/// and residency has not gated a cell since ADR-0240. *Empty* is a slot that
/// exists with nothing loaded into it — the manual names it and the engine
/// cannot be in it, because `Deck::new` builds a slot per `HotSwap` and
/// `slot_view` is `None` only past `slot_count`. Writing either here would
/// assert a reading nobody took, which is ADR-0200's rule and ADR-0191's
/// before it.
fn state_word(picture: Option<Picture>, overloaded: bool) -> &'static str {
    match (picture, overloaded) {
        (None, _) => PREVIEW_NO_SLOT,
        (Some(_), true) => PREVIEW_OVERLOADED,
        (Some(_), false) => PREVIEW_MATERIAL,
    }
}

/// A cell showing its slot's own material. See [`state_word`].
pub const PREVIEW_MATERIAL: &str = "material";

/// A cell showing the last frame a stopped slot drew — the word
/// `swap::Event::Overloaded` and [`Stage::Overloaded`] carry, said here because
/// this is where the still is. See [`state_word`] and [`View::overloaded`].
pub const PREVIEW_OVERLOADED: &str = "overloaded";

/// A cell with no deck slot behind it. See [`state_word`].
pub const PREVIEW_NO_SLOT: &str = "no slot";

// ---------------------------------------------------------------------------
// The risk badge: one number, five bands.
// ---------------------------------------------------------------------------

/// **What the governor budgeted one slot at, and which of its two numbers that
/// is** — the seam the risk badge is drawn from.
///
/// `karakuri_engine::governor::Decision` carries `budgeted_ms` and a `Basis`
/// saying whether it is the two-draw estimate at the output's size or the
/// single-draw measurement at the reference resolution
/// ([ADR-0296](../../../../docs/adr/0296-the-governor-budgets-on-the-estimate-where-it-answers-and-on-the-measurement-where-it-does-not.md)).
/// This is that pair, arriving the way every other value does: `src/` takes no
/// engine (ADR-0156), so whoever holds the deck reads the report and writes
/// [`View::costs`].
///
/// **The engine's third basis is this type's [`None`].**
/// `governor::Basis::Unbudgetable` is a slot nothing measured and nothing
/// estimated, and `Decision::budgeted_ms` is `None` there. It is not a number
/// and it is emphatically not a zero, so it does not cross this seam as one:
/// no entry, no dot. See [`View::costs`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Budgeted {
    /// The number the governor spent on this slot, in milliseconds —
    /// `Decision::budgeted_ms`. What [`band_of`] reads.
    pub ms: f32,
    /// **How it was taken.** See [`Basis`], and the argument on [`band_of`]
    /// for why the dot does not draw it.
    pub basis: Basis,
}

/// **Which of the governor's two numbers [`Budgeted::ms`] is** —
/// `karakuri_engine::governor::Basis`, less the variant this crate spells
/// [`None`].
///
/// Carried across the seam and **not drawn**, which is a decision rather than
/// an omission: see [`band_of`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basis {
    /// The single draw at the engine's reference resolution, because nothing
    /// estimated this slot or the estimate refused. It is a real reading of
    /// this Set and it is a reading at a size that is the operator's only by
    /// coincidence.
    Measured,
    /// The two-draw fit, evaluated at the size this deck is actually drawing
    /// into.
    Estimated,
}

/// **The five bands the risk badge is drawn in**, and the words the mock's
/// `.risk.green`, `.risk.blue`, `.risk.yellow`, `.risk.red` and `.risk.purple`
/// name them by.
///
/// **The table is the console's and not the engine's.**
/// `karakuri_engine::estimate`'s own documentation says so, and
/// `crates/karakuri-engine/tests/governor.rs` quotes the boundaries rather
/// than sharing them for the same reason: the engine produces a number of
/// milliseconds and has no opinion about how many of a thing an operator can
/// mix. The scale is a reading of one frame's worth of budget shared four
/// ways, which is a fact about this panel's four cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    /// Up to [`BAND_BLUE_MS`]: four of these at 60 Hz.
    Green,
    /// Over [`BAND_BLUE_MS`]: not four at 60, and two at 60 is fine.
    Blue,
    /// [`BAND_YELLOW_MS`]: two at 60 Hz, four at 30 Hz.
    Yellow,
    /// [`BAND_RED_MS`]: one at 60 Hz, two at 30 Hz.
    Red,
    /// [`BAND_PURPLE_MS`] and over: one slot eats a whole frame.
    Purple,
}

impl Band {
    /// The word the mock's class names this band by — `.risk.green` and its
    /// four siblings. What `tests/transcribed_constants_cite_the_mock.rs`
    /// looks the boundary up in `docs/manual/console.html` by, and what
    /// `karakuri-engine`'s own band prediction spells.
    pub fn word(self) -> &'static str {
        match self {
            Band::Green => "green",
            Band::Blue => "blue",
            Band::Yellow => "yellow",
            Band::Red => "red",
            Band::Purple => "purple",
        }
    }

    /// The colour it is drawn in, from the room's own five — `--c-band-*`.
    /// See [`Palette`], where the argument for five properties of their own
    /// rather than `--c-mint` and `--c-pink` is written.
    pub fn colour(self, pal: &Palette) -> Color32 {
        match self {
            Band::Green => pal.band_green,
            Band::Blue => pal.band_blue,
            Band::Yellow => pal.band_yellow,
            Band::Red => pal.band_red,
            Band::Purple => pal.band_purple,
        }
    }

    /// The five, worst last. The order the scale is written in.
    pub const ALL: [Band; 5] = [
        Band::Green,
        Band::Blue,
        Band::Yellow,
        Band::Red,
        Band::Purple,
    ];
}

/// **Where green ends and blue begins**: `docs/manual/console.html`, *What a
/// deck preview cell shows, and when* — *"Green, up to 4 ms: four of these at
/// 60 Hz."*
///
/// Four slots share one frame and 16.7 ms at 60 Hz is about 4 ms each, which
/// is where the whole scale comes from: it answers *how many of these, and at
/// what rate* rather than handing an operator a number to divide in a dark
/// room.
///
/// **Each of these four is named for the band it lets you into rather than the
/// one it leaves**, because that is the rounding rule written into the name: a
/// value *on* a boundary rounds to the worse band, so 4.0 is blue and not
/// green. See [`band_of`].
pub const BAND_BLUE_MS: f32 = 4.0;

/// *"Yellow, about 8 ms: the boundary for two at 60 Hz, and four at 30 Hz."*
pub const BAND_YELLOW_MS: f32 = 8.0;

/// *"Red, about 12 ms: one at 60 Hz, two at 30 Hz."*
pub const BAND_RED_MS: f32 = 12.0;

/// *"Purple, over 16 ms: one slot eats a whole frame and the cell stops
/// drawing."*
///
/// **The last band is the one that is also a behaviour**, and the behaviour is
/// not this crate's: a slot over budget is stopped by the governor rather than
/// shown harder, and a stopped slot reaches the console as a cell with no
/// picture in it. So the console never has to decide to stop drawing — what it
/// draws is the band, and the stopping has already happened upstream.
pub const BAND_PURPLE_MS: f32 = 16.0;

/// **The band one number falls in**, and the whole of the console's half of
/// the risk badge.
///
/// The four boundaries are [`BAND_BLUE_MS`], [`BAND_YELLOW_MS`],
/// [`BAND_RED_MS`] and [`BAND_PURPLE_MS`], and **a value on a boundary rounds
/// to the worse band** — the mock says so in those words, and it is the reason
/// every comparison here is `>=` and the fall-through is green. The direction
/// is the same one the estimate rounds in: `karakuri_engine::estimate` rounds
/// toward refusing at every step, so a badge that rounded a boundary the
/// generous way would be the one place in the chain that reads a number
/// kindly.
///
/// # It does not read [`Basis`], and that is the decision rather than the
/// default
///
/// A dot drawn from an estimate at this deck's own size and a dot drawn from a
/// measurement at 1280x720 are **not the same statement** — P-0095 is exactly
/// that, and ADR-0296 §3 carries `FloorRead` and `Floored` on the decision so
/// the difference cannot be lost. The console keeps it ([`Budgeted::basis`])
/// and does not draw it, for two reasons.
///
/// - **The number is the one the governor spent.** A badge that drew a dot
///   only where the basis is [`Basis::Estimated`] would be drawing a different
///   quantity from the one the deck is being governed on: every Set swapped in
///   on a live run arrives unestimated (ADR-0296, *Consequences*), so the dot
///   would vanish at the moment an operator loaded something — and a slot the
///   governor parks on a measured number would be parked with no visible
///   cause, which is
///   [ADR-0191](../../../../docs/adr/0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md)'s
///   complaint. **A refusal is not a green dot; a number that was spent is not
///   a refusal.**
/// - **The mock does not distinguish them**, and the page moves first. `.risk`
///   has five classes and there is no sixth mark, no hollow ring and no second
///   word in the caption for *how this was taken*. Inventing one here would be
///   the console specifying itself. What the page would have to say is
///   reported rather than drawn — see the module documentation on
///   [`caption_into`].
///
/// **What P-0095 does get** is the one thing the page already specifies: no
/// number, no dot. `governor::Basis::Unbudgetable` does not cross the seam
/// (see [`Budgeted`]), and a cell handed nothing draws nothing — not hollow,
/// not grey, not green by default.
pub fn band_of(ms: f32) -> Band {
    match ms {
        _ if ms >= BAND_PURPLE_MS => Band::Purple,
        _ if ms >= BAND_RED_MS => Band::Red,
        _ if ms >= BAND_YELLOW_MS => Band::Yellow,
        _ if ms >= BAND_BLUE_MS => Band::Blue,
        _ => Band::Green,
    }
}

/// **One deck preview cell's caption**: the mock's `.caption`, under the
/// image and never on it.
///
/// Term for term from `style.css`:
///
/// - `.cell`'s `gap: 4px` and `.caption`'s `height: 13px` — [`caption_of`],
///   which is where the rectangle comes from.
/// - `.caption`'s `font-size: 9px` — [`size::PREVIEW_SIZE`], for both the
///   letter and the word.
/// - `.caption`'s `gap: 5px` — [`size::PREVIEW_CAPTION_GAP_X`], between the
///   two.
/// - `align-items: center` — both galleys centred on the caption's own middle,
///   which is what puts a 9px letter and a 9px word on one baseline in a 13px
///   strip.
///
/// # The two colours, and what each is standing for
///
/// - **The letter** is `.caption b`'s `var(--c-dim)`, `pal.dim` — *"a label
///   beside"* a value, which is what a letter welded to a cell is. It is one
///   colour in both states, because the letter is not a state: `A` is `A`
///   whether or not anything is behind it, and a letter that dimmed when the
///   slot went away would be the cell's state said twice.
/// - **The word** is `.caption`'s own `var(--c-faint)`, `pal.faint`.
///
/// # The third colour, and it is the drop mark rather than a state
///
/// `marked` is `.cell.drop .caption b`'s `color: var(--c-text)`: while a
/// carried Set would land on this cell, the letter comes up out of its dim
/// with the ring round the image. **It is not a fourth state of the cell** —
/// the two above are still the only two [`View::previews`] distinguishes, and
/// nothing here reads the picture for it. What the ring says is *where the
/// release lands* and what the letter says is *which deck that is*, which is
/// the one thing on a cell that already names the operand
/// ([`drop_ring`], and `console.html`'s *How a Set reaches a deck*).
///
/// # The badge, and the two ways it is not drawn
///
/// `.risk` is `width: 6px; height: 6px; border-radius: 999px; margin-left:
/// auto` — a 6px dot in the band's colour, pushed to the far end of the
/// caption so the four line up down the row and can be read as a column
/// without reading a word ([`size::PREVIEW_RISK`], and the stylesheet's own
/// comment says the reason). [`band_of`] is the table; [`Band::colour`] is the
/// five properties the room states for it.
///
/// **A slot with no number draws no dot**, which is the state the manual
/// already describes and is not a fifth thing this function invents: not
/// hollow, not grey, not green by default, each of which would assert a
/// reading nobody took —
/// [ADR-0200](../../../../docs/adr/0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md),
/// draw the values that exist and omit the rest. It covers three cases and
/// they are three different nothings: the governor found neither number for
/// this slot (`governor::Basis::Unbudgetable`, which does not cross the seam —
/// [`Budgeted`]), nobody has governed this deck yet, and **a number that is
/// not finite**, which is a failed reading rather than a small one.
///
/// **And a cell with no slot behind it draws no dot either, whatever it was
/// handed.** The mock's D cell is the case and its tooltip is the rule: *"No
/// slot is no cost, so there is no dot."* A dot beside [`PREVIEW_NO_SLOT`]
/// would be a cost for a thing that is not there, so the picture gates the
/// badge — which also means the two halves of a cell can never disagree about
/// whether there is a slot.
///
/// **Purple is the one band that is also a behaviour, and the behaviour is not
/// this crate's.** *One slot eats a whole frame and the cell stops drawing* —
/// stopping it is the governor's, and a stopped slot arrives here as a cell
/// with no picture. So this function never decides to stop drawing; it draws
/// the band it was handed and the stopping has already happened upstream.
///
/// # What the page would have to say before the badge could say more
///
/// The dot is one mark for two kinds of number. A band read from an estimate
/// at this deck's own size and a band read from a measurement at the reference
/// resolution are not the same statement (P-0095, and ADR-0296 §3), and the
/// mock has five classes on `.risk` and nothing for *how this was taken*.
/// [`Budgeted::basis`] carries the difference across the seam and this draws
/// it nowhere; [`band_of`] argues why that is the honest reading today, and
/// **the page moves first** if it is to stop being. What it would have to
/// state is a second mark on the caption and what it means — a ring round the
/// dot for a measured number, say, against a filled dot for an estimated one —
/// because the operator-facing difference is that a measured band is about a
/// frame at 1280x720 and can therefore be a band too good on a larger output,
/// which is the one direction the whole estimate is built to round away from.
#[allow(clippy::too_many_arguments)]
pub(super) fn caption_into(
    ui: &Ui,
    pal: &Palette,
    image: Rect,
    deck: usize,
    picture: Option<Picture>,
    // **Whether this slot has stopped updating** — [`View::overloaded`]. It
    // changes the word and nothing else: not the letter, not the badge, and
    // not the image above, which is the still it is about.
    overloaded: bool,
    cost: Option<Budgeted>,
    marked: bool,
) {
    let at = caption_of(image);
    let painter = ui.painter().with_clip_rect(at);
    let font = FontId::new(size::PREVIEW_SIZE, FontFamily::Proportional);

    let ink = match marked {
        true => pal.text,
        false => pal.dim,
    };
    let letter = painter.layout_no_wrap(DECK_LETTERS[deck].to_owned(), font.clone(), ink);
    let width = letter.size().x;
    painter.galley(
        Pos2::new(at.min.x, at.center().y - letter.size().y * 0.5),
        letter,
        ink,
    );

    let word = painter.layout_no_wrap(state_word(picture, overloaded).to_owned(), font, pal.faint);
    painter.galley(
        Pos2::new(
            at.min.x + width + size::PREVIEW_CAPTION_GAP_X,
            at.center().y - word.size().y * 0.5,
        ),
        word,
        pal.faint,
    );

    // **The badge, where there is a slot and a number for it.** Both halves
    // are gates rather than one: `picture` is whether there is a slot at all
    // and `cost` is whether anything budgeted it, and the mock's D cell is the
    // first of the two — *"No slot is no cost, so there is no dot."* A number
    // that is not finite is a failed reading and takes the same path as no
    // number; see this function's documentation for all three nothings.
    let dot = picture
        .and(cost)
        .filter(|budgeted| budgeted.ms.is_finite())
        .map(|budgeted| band_of(budgeted.ms));
    if let Some(band) = dot {
        // `margin-left: auto` — the far end of the caption, centred in its
        // height, and `border-radius: 999px` on a 6px box is a circle of half
        // that across.
        let radius = size::PREVIEW_RISK * 0.5;
        painter.circle_filled(
            Pos2::new(at.max.x - radius, at.center().y),
            radius,
            band.colour(pal),
        );
    }
}
