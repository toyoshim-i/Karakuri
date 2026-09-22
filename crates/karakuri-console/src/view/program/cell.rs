use super::*;

/// One deck preview cell's image: the mock's `.preview`, and whatever is
/// behind it.
///
/// Nothing is written on it. The letter and the state word are the
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
/// A cell is not dark because its deck is off air. Every slot is drawn
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
///   inside of the box in `pal.hair`, drawn last so it sits over the
///   texture exactly as an inset shadow sits over a background image.
pub fn preview(ui: &Ui, pal: &Palette, cell: Rect, picture: Option<Picture>) {
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

/// Returns the status label for a preview cell (`material`, `overloaded`, or `no slot`).
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

/// One deck preview cell's caption: the mock's `.caption`, under the
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
/// - The letter is `.caption b`'s `var(--c-dim)`, `pal.dim` — *"a label
///   beside"* a value, which is what a letter welded to a cell is. It is one
///   colour in both states, because the letter is not a state: `A` is `A`
///   whether or not anything is behind it, and a letter that dimmed when the
///   slot went away would be the cell's state said twice.
/// - The word is `.caption`'s own `var(--c-faint)`, `pal.faint`.
///
/// # The third colour, and it is the drop mark rather than a state
///
/// `marked` is `.cell.drop .caption b`'s `color: var(--c-text)`: while a
/// carried Set would land on this cell, the letter comes up out of its dim
/// with the ring round the image. It is not a fourth state of the cell —
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
/// A slot with no number draws no dot, which is the state the manual
/// already describes and is not a fifth thing this function invents: not
/// hollow, not grey, not green by default, each of which would assert a
/// reading nobody took —
/// [ADR-0200](../../../../docs/adr/0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md),
/// draw the values that exist and omit the rest. It covers three cases and
/// they are three different nothings: the governor found neither number for
/// this slot (`governor::Basis::Unbudgetable`, which does not cross the seam —
/// [`Budgeted`]), nobody has governed this deck yet, and a number that is
/// not finite, which is a failed reading rather than a small one.
///
/// And a cell with no slot behind it draws no dot either, whatever it was
/// handed. The mock's D cell is the case and its tooltip is the rule: *"No
/// slot is no cost, so there is no dot."* A dot beside [`PREVIEW_NO_SLOT`]
/// would be a cost for a thing that is not there, so the picture gates the
/// badge — which also means the two halves of a cell can never disagree about
/// whether there is a slot.
///
/// Purple is the one band that is also a behaviour, and the behaviour is not
/// this crate's. *One slot eats a whole frame and the cell stops drawing* —
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
/// the page moves first if it is to stop being. What it would have to
/// state is a second mark on the caption and what it means — a ring round the
/// dot for a measured number, say, against a filled dot for an estimated one —
/// because the operator-facing difference is that a measured band is about a
/// frame at 1280x720 and can therefore be a band too good on a larger output,
/// which is the one direction the whole estimate is built to round away from.
#[allow(clippy::too_many_arguments)]
pub fn caption_into(
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
