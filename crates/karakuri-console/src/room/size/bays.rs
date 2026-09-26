//! Bay sizing constants transcribed from the stylesheet.

use super::base::*;

// -- the library's scopes -----------------------------------------------

/// `.scopes`'s `padding: 7px 9px` and its `gap: 4px`: the row of chips between
/// the bay head and the list, which says *which library is being listed* rather
/// than what is in it.
pub const SCOPES_PAD_X: f32 = 9.0;
pub const SCOPES_PAD_Y: f32 = 7.0;
pub const SCOPES_GAP: f32 = 4.0;

/// `.scope`'s `padding: 0 8px`, around one scope's word at [`BASE`]. The same
/// padding a [`PILL_PAD_X`] is and a different chip: a `.pill` is a control in
/// a bay head and this is one of a row of questions under one.
pub const SCOPE_PAD_X: f32 = 8.0;

/// One chip's box: [`BASE`] at [`LINE`], with no border to count — `.scope`
/// draws none where the `.rend` chips in the inspector each draw one, so this
/// is [`PILL_H`]'s number and not [`REND_H`]'s. 16.5.
pub const SCOPE_H: f32 = BASE * LINE;

/// Scope row box height: single chip inside [`SCOPES_PAD_Y`] plus [`HAIRLINE`] rule
/// (`SCOPES_PAD_Y * 2.0 + SCOPE_H + HAIRLINE` = 31.5px).
pub const SCOPES_H: f32 = SCOPES_PAD_Y * 2.0 + SCOPE_H + HAIRLINE;

// -- the library's path row ---------------------------------------------

/// `.path`'s `padding: 4px 10px`, around the directory this library is pointed
/// at. The row between the scope row and the filters, and the one row of this
/// bay's furniture that is drawn only sometimes — until a folder has been
/// dropped on the window there is no path and no row (ADR-0275).
pub const PATH_PAD_X: f32 = 10.0;
pub const PATH_PAD_Y: f32 = 4.0;

/// `.path`'s `font-size: 10px`: the directory, and the only type in the row.
/// [`LIB_FOOT_SIZE`]'s number at the other end of the bay and for the mock's
/// own reason — the small type is what this bay draws everything a row is not
/// named by in.
pub const PATH_SIZE: f32 = 10.0;

/// Path row box height: `PATH_PAD_Y * 2.0 + PATH_SIZE * LINE + HAIRLINE` (24px).
pub const PATH_H: f32 = PATH_PAD_Y * 2.0 + PATH_SIZE * LINE + HAIRLINE;

// -- the library's filter field and its kind chips ------------------------

/// `.lib-filters`'s `padding: 6px 9px` and its `gap: 5px`: filter row padding and gap (ADR-0338).
pub const LIB_FILTERS_PAD_X: f32 = 9.0;
pub const LIB_FILTERS_PAD_Y: f32 = 6.0;
pub const LIB_FILTERS_GAP: f32 = 5.0;

/// `.field`'s `padding: 0 9px`, around the word in it at [`BASE`]. The vertical
/// half of that declaration is zero, which is why the box below is the type's
/// own height and its border and nothing else.
pub const FIELD_PAD_X: f32 = 9.0;

/// Filter field box height: `BASE * LINE + HAIRLINE * 2.0` (18.5px, including 1px border).
pub const FIELD_H: f32 = BASE * LINE + HAIRLINE * 2.0;

/// Filter row total height: `LIB_FILTERS_PAD_Y * 2.0 + FIELD_H + HAIRLINE` (31.5px).
pub const LIB_FILTERS_H: f32 = LIB_FILTERS_PAD_Y * 2.0 + FIELD_H + HAIRLINE;

/// `.lib-kinds`'s `padding: 5px 9px` and its `gap: 4px`: kind toggle chips row (ADR-0338).
pub const LIB_KINDS_PAD_X: f32 = 9.0;
pub const LIB_KINDS_PAD_Y: f32 = 5.0;
pub const LIB_KINDS_GAP: f32 = 4.0;

/// `.kind`'s `font-size: 9px` and its `padding: 0 6px`, around the kind name.
pub const KIND_SIZE: f32 = 9.0;
pub const KIND_PAD_X: f32 = 6.0;

/// One kind chip's box: [`KIND_SIZE`] at [`LINE`] — 13.5.
pub const KIND_H: f32 = KIND_SIZE * LINE;

/// The kind row's box: one chip inside [`LIB_KINDS_PAD_Y`], plus the one pixel
/// of the rule under it — [`HAIRLINE`], which is `.lib-kinds`'s own
/// border-bottom and the same pixel every other rule in this bay is drawn at.
/// 24.5.
pub const LIB_KINDS_H: f32 = LIB_KINDS_PAD_Y * 2.0 + KIND_H + HAIRLINE;

// -- a row's badges -------------------------------------------------------

/// `.badge`'s `font-size: 8px` and its `padding: 0 4px`, around the layer's
/// word; `.badges`' `gap: 3px` between two of them. What a badge says is which
/// layers a row implements, and it is a readout: nothing here is pressed, and
/// what narrows the list by kind is the row of chips above (ADR-0338).
pub const BADGE_SIZE: f32 = 8.0;
pub const BADGE_PAD_X: f32 = 4.0;
pub const BADGE_GAP: f32 = 3.0;

/// One badge's box: [`BADGE_SIZE`] at [`LINE`] — 12. Its `border: 1px solid
/// var(--c-hair)` is not counted, for [`KIND_H`]'s reason read the other way:
/// every badge draws one, so counting it would move the whole row and change
/// nothing about which of them is taller.
pub const BADGE_H: f32 = BADGE_SIZE * LINE;

/// `.badge`'s `border-radius: 3px`, which is the one rounded box in this bay
/// that is not a capsule: a badge is as tall as one line of very small type,
/// and a `999px` radius on it would be a lozenge round two letters.
pub const BADGE_RADIUS: f32 = 3.0;

// -- the library's list -------------------------------------------------

/// `.lib-list`'s `padding: 3px`: card margin enclosing library listing rows.
pub const LIB_LIST_PAD: f32 = 3.0;

/// `.lib-row`'s `padding: 3px 7px`, around a Set's name at [`BASE`]. The rows
/// stack with no gap between them: `.lib-list` is a column flex with no `gap`
/// at all, which is the one list in the mock that has none.
pub const LIB_ROW_PAD_X: f32 = 7.0;
pub const LIB_ROW_PAD_Y: f32 = 3.0;

/// A row's box: [`BASE`] at [`LINE`] inside that padding — 22.5, which is the
/// 22.5 the library's minimum of 158 is written from in `lib.rs`, so a row is
/// this tall in both places or in neither.
pub const LIB_ROW_H: f32 = BASE * LINE + LIB_ROW_PAD_Y * 2.0;

/// `.lib-row`'s `border-radius: 7px`, which nothing in this bay draws until a
/// row is under the cursor: the wash behind that one row is the only fill a row
/// ever has, and a square-cornered fill inside a card rounded at [`BAY_RADIUS`]
/// is the one shape the mock never draws.
pub const LIB_ROW_RADIUS: f32 = 7.0;

/// `.lib-foot`'s `padding: 5px 10px`, around the count at [`LIB_FOOT_SIZE`].
pub const LIB_FOOT_PAD_X: f32 = 10.0;
pub const LIB_FOOT_PAD_Y: f32 = 5.0;

/// `.lib-foot`'s `font-size: 10px`: how many rows are listed of how many there
/// are, and the only type in the row.
pub const LIB_FOOT_SIZE: f32 = 10.0;

/// Library footer box height: `LIB_FOOT_PAD_Y * 2.0 + LIB_FOOT_SIZE * LINE + HAIRLINE` (26px).
pub const LIB_FOOT_H: f32 = LIB_FOOT_PAD_Y * 2.0 + LIB_FOOT_SIZE * LINE + HAIRLINE;

/// `.lib-foot`'s `gap: 8px`, between the count, the `params` chip and the
/// `load` pill. It is the row's only spacing that is not padding: `.sep` takes
/// whatever is left over, so this is what separates the two capsules at the
/// right-hand end of the row and nothing else.
pub const LIB_FOOT_GAP: f32 = 8.0;

/// The air either side of a row menu's separator — `.rowmenu .rule`'s `margin:
/// 4px 6px`, the vertical half of it. The horizontal half is the hairline's own
/// inset from the card's edge and is drawn rather than laid out, so it is not a
/// constant here.
pub const ROW_MENU_RULE_PAD: f32 = 4.0;

/// A row menu's separator band: the air, the hairline and the air again. It is
/// a band and not a row, because nothing in it is a control — [`LIB_ROW_H`] is
/// what an item is tall and this is what the space between the loads and the
/// send is.
pub const ROW_MENU_RULE_H: f32 = ROW_MENU_RULE_PAD * 2.0 + HAIRLINE;

/// The narrowest a row menu's card is drawn — `.rowmenu`'s `min-width: 124px`.
/// The items are as wide as the words in them and `Save as a kbset` is the
/// longest of the six, so this is only ever reached by a face narrower than the
/// mock's.
pub const ROW_MENU_MIN_W: f32 = 124.0;

/// How far in from the row's left edge the card hangs — `.rowmenu`'s `left:
/// 22px`, which puts it clear of the star and under the name the press landed
/// on rather than under the mark beside it.
pub const ROW_MENU_INSET: f32 = 22.0;

// -- the library's reading ----------------------------------------------

/// `docs/manual/console.html`'s `margin: 2px 2px 3px`: inline margins around a Set reading card.
pub const READING_MARGIN_X: f32 = 2.0;
pub const READING_MARGIN_TOP: f32 = 2.0;
pub const READING_MARGIN_BOTTOM: f32 = 3.0;

/// The same box's `docs/manual/console.html`'s `border-radius: 8px`, which is
/// [`CAND_RADIUS`]' number one bay along and not [`LIB_ROW_RADIUS`]'s row
/// radius: this is a well with rows in it, drawn at the radius the mock gives
/// every well it draws — `.cand` and `.fx` are both 8.
pub const READING_RADIUS: f32 = 8.0;

/// `docs/manual/console.html`'s `padding-left: 18px`: indent distinguishing reading rows from Sets.
pub const READING_PAD_X: f32 = 18.0;

// -- the staging lane's candidates --------------------------------------

/// `.stage-list`'s `padding: 6px 9px 8px`: asymmetric card padding enclosing candidate rows.
pub const STAGE_LIST_PAD_TOP: f32 = 6.0;
pub const STAGE_LIST_PAD_X: f32 = 9.0;
pub const STAGE_LIST_PAD_BOTTOM: f32 = 8.0;

/// `.stage-list`'s `gap: 5px`, between one candidate row and the next. The one
/// list in this console that has a gap — `.lib-list` states none and its rows
/// are flush — and it is the same 5 `lib.rs`'s 125 is written from (*"two 5px
/// gaps"*).
pub const STAGE_GAP: f32 = 5.0;

/// `.cand`'s `padding: 4px 7px`, around the row's own type at [`BASE`].
pub const CAND_PAD_X: f32 = 7.0;
pub const CAND_PAD_Y: f32 = 4.0;

/// A candidate row's box: [`BASE`] at [`LINE`] inside that padding — 24.5,
/// which is the 24.5 `lib.rs`'s 125 and its minimum of 66 are both written from
/// (*"three `.cand` rows at 4 + 16.5 + 4"*), so a row is this tall in both
/// places or in neither.
pub const CAND_H: f32 = BASE * LINE + CAND_PAD_Y * 2.0;

/// `.cand`'s `border-radius: 8px`, one shade tighter than the bay's
/// [`BAY_RADIUS`] because the row is inside it — [`PREVIEW_RADIUS`]'s relation
/// to the same bay, one column along.
pub const CAND_RADIUS: f32 = 8.0;

/// `.cand`'s `gap: 6px`, between the deck the candidate landed on and what it
/// is called.
pub const CAND_GAP: f32 = 6.0;

/// `.cand .who`'s `font-size: 10px`: the small type at the far end of a
/// candidate row. The mock puts the producer there and this console puts the
/// verdict — see [`crate::view::staging`], which is where that substitution is
/// argued.
pub const CAND_WHO_SIZE: f32 = 10.0;

// -- the inspector's panes ----------------------------------------------

/// `.half-head`'s `padding: 5px 10px` and its `gap: 6px`: the row that says
/// *which* deck a pane is showing, above the deck head that says what that deck
/// is doing.
pub const HALF_HEAD_PAD_X: f32 = 10.0;
pub const HALF_HEAD_PAD_Y: f32 = 5.0;
pub const HALF_HEAD_GAP: f32 = 6.0;

/// The pane head's box: [`BASE`] at [`LINE`] inside that padding, plus the one
/// pixel of its own `border-bottom: 1px solid var(--c-hair)` — 27.5, which is
/// the 27.5 the inspector's minimum of 151.5 is written from in `lib.rs`, so
/// the row is this tall in both places or in neither.
pub const HALF_HEAD_H: f32 = HALF_HEAD_PAD_Y * 2.0 + BASE * LINE + HAIRLINE;

/// `.deck-head`'s `padding: 5px 10px` and its `gap: 6px`: the strip of chips
/// that heads a *deck* rather than a bay — its clock and its fold.
pub const DECK_HEAD_PAD_X: f32 = 10.0;
pub const DECK_HEAD_PAD_Y: f32 = 5.0;
pub const DECK_HEAD_GAP: f32 = 6.0;

/// The deck head's box: a [`MINI_H`] chip inside that padding — 25.5, which is
/// the stylesheet's own arithmetic for the row (*"the tallest thing in it is a
/// `.mini` at 15.5, so the row is 5 + 15.5 + 5"*) and the 25.5 the inspector's
/// minimum of 151.5 is written from in `lib.rs`.
pub const DECK_HEAD_H: f32 = DECK_HEAD_PAD_Y * 2.0 + MINI_H;

/// `.anchor`'s `font-size: 9px`: the tempo a deck was engaged at, beside the
/// chip that says what its clock is locked to.
pub const ANCHOR_SIZE: f32 = 9.0;

/// `.scrub`'s `gap: 3px`, between the two arrows a quarter beat a press goes
/// through.
pub const SCRUB_GAP: f32 = 3.0;

/// `.scrub i`'s `font-size: 9px`: one of the two arrows a quarter beat a press
/// goes through, at the same type size as the anchor beside it.
pub const SCRUB_SIZE: f32 = 9.0;

/// `.scrub i`'s `padding: 0 4px`, around one arrow — tighter than a
/// [`MINI_PAD_X`] because what is inside it is a mark and not a word.
pub const SCRUB_PAD_X: f32 = 4.0;

/// One arrow's box: [`SCRUB_SIZE`] at [`LINE`] inside its `border: 1px solid
/// var(--c-line)` — 15.5, which is [`MINI_H`]'s own number at the same type
/// size, and is why the two arrows sit in the deck head without making
/// [`DECK_HEAD_H`] any taller than the chip beside them.
pub const SCRUB_H: f32 = SCRUB_SIZE * LINE + HAIRLINE * 2.0;

/// `.node-head`'s `padding: 5px 10px` and its `gap: 7px`: a node's address, its
/// name and who is allowed to move it.
pub const NODE_HEAD_PAD_X: f32 = 10.0;
pub const NODE_HEAD_PAD_Y: f32 = 5.0;
pub const NODE_HEAD_GAP: f32 = 7.0;

/// A node head's box: [`BASE`] at [`LINE`] inside that padding — 26.5, which is
/// the 26.5 the inspector's minimum of 151.5 is written from in `lib.rs`. The
/// authority chips beside the name are shorter than the name is ([`AUTH_H`]
/// against 16.5), so the name is what sets the height.
pub const NODE_HEAD_H: f32 = NODE_HEAD_PAD_Y * 2.0 + BASE * LINE;

/// `.auth`'s `gap: 3px`, between the three words on a node head.
pub const AUTH_GAP: f32 = 3.0;

/// `.auth span`'s `padding: 0 5px` and its `font-size: 9px`: one of `man`,
/// `sug` and `auto`.
pub const AUTH_PAD_X: f32 = 5.0;
pub const AUTH_SIZE: f32 = 9.0;

/// An authority chip's box: [`AUTH_SIZE`] at [`LINE`] — 13.5, with no border to
/// count, which is what makes it shorter than the node name beside it. Its
/// `border-radius: 999px` on a box this short is a capsule.
pub const AUTH_H: f32 = AUTH_SIZE * LINE;

/// `.param`'s `padding: 3px 10px 3px 12px` — the one row in the mock whose two
/// side paddings differ, twelve in from the left of the pane and ten from the
/// right, which is what indents a parameter under the node head above it — and
/// its `gap: 8px`, between the four tracks.
pub const PARAM_PAD_L: f32 = 12.0;
pub const PARAM_PAD_R: f32 = 10.0;
pub const PARAM_PAD_Y: f32 = 3.0;
pub const PARAM_GAP: f32 = 8.0;

/// `.param`'s `grid-template-columns: 15px 88px 1fr 58px`: the ordinal a MIDI
/// control is learned against, the name, the fader — which takes whatever the
/// other three leave — and the value.
pub const PARAM_ORD_W: f32 = 15.0;
pub const PARAM_NAME_W: f32 = 88.0;
pub const PARAM_VAL_W: f32 = 58.0;

/// `.param .ord`'s `font-size: 9.5px`: the position in the deck's published
/// interface, and the only type in the row that is not the pane's own size.
pub const PARAM_ORD_SIZE: f32 = 9.5;

/// Parameter row box height: `PARAM_PAD_Y * 2.0 + BASE * LINE` (22.5px).
pub const PARAM_H: f32 = PARAM_PAD_Y * 2.0 + BASE * LINE;

/// Scroll step distance in pixels for one notch of mouse wheel, set to three parameter rows.
///
/// See ADR-0307.
pub const WHEEL_STEP: f32 = PARAM_H * 3.0;

/// `.rend-row`'s `padding: 3px 10px 6px 12px` — the renderer chips stand on the
/// same 12 and 10 a parameter row does, with more room under them than over —
/// and its `gap: 5px`, between two chips.
pub const REND_ROW_PAD_L: f32 = 12.0;
pub const REND_ROW_PAD_R: f32 = 10.0;
pub const REND_ROW_PAD_T: f32 = 3.0;
pub const REND_ROW_PAD_B: f32 = 6.0;
pub const REND_GAP: f32 = 5.0;

/// `.rend`'s `padding: 0 7px`, around a renderer's name at [`BASE`].
pub const REND_PAD_X: f32 = 7.0;

/// A renderer chip's box: [`BASE`] at [`LINE`] inside its `border: 1px solid
/// var(--c-line)` — the same box [`MINI_H`] is, at the pane's own type size
/// rather than a mini's, so 18.5.
pub const REND_H: f32 = BASE * LINE + HAIRLINE * 2.0;

/// Renderer row box height: `REND_ROW_PAD_T + REND_H + REND_ROW_PAD_B` (27.5px), for single-row chips.
pub const REND_ROW_H: f32 = REND_ROW_PAD_T + REND_H + REND_ROW_PAD_B;

/// `.sens`'s `padding: 2px 10px 6px 12px` — the sensitivity row stands on the
/// same 12 and 10 the parameter row above it does, tight under it and with the
/// group's own breathing room below.
pub const SENS_PAD_L: f32 = 12.0;
pub const SENS_PAD_R: f32 = 10.0;
pub const SENS_PAD_T: f32 = 2.0;
pub const SENS_PAD_B: f32 = 6.0;

/// `.sens`'s `grid-template-columns: 111px 1fr` and its `gap: 8px`: the word
/// *sensitivity*, then the chips.
pub const SENS_LABEL_W: f32 = 111.0;
pub const SENS_GAP: f32 = 8.0;

/// `.sens .chips`'s `gap: 5px`, between two chips.
pub const SENS_CHIP_GAP: f32 = 5.0;

/// `.sens`'s `font-size: 10px`, which the pills inside it inherit — so a chip
/// here is smaller than a renderer chip and larger than an authority one, and
/// none of the three is [`BASE`].
pub const SENS_SIZE: f32 = 10.0;

/// A sensitivity chip's box: [`SENS_SIZE`] at [`LINE`] inside a pill's `border:
/// 1px solid var(--c-line)` — 17. [`REND_H`]'s shape written at this row's own
/// type size, and the border counts for [`REND_H`]'s reason: a pill's is a real
/// border where a mini's is a wash.
pub const SENS_CHIP_H: f32 = SENS_SIZE * LINE + HAIRLINE * 2.0;

/// The sensitivity row's box: one chip inside [`SENS_PAD_T`] and [`SENS_PAD_B`]
/// — 25. One row and not a wrap, which is [`REND_ROW_H`]'s rule: `.sens .chips`
/// carries `flex-wrap: wrap` and the console draws the chips that fit.
pub const SENS_H: f32 = SENS_PAD_T + SENS_CHIP_H + SENS_PAD_B;

/// `.pill`'s `padding: 0 8px` is [`PILL_PAD_X`]; this is the same number named
/// for this row, so a change to one is not silently a change to the other. The
/// mock gives `.sens`'s pills no padding of their own.
pub const SENS_CHIP_PAD_X: f32 = PILL_PAD_X;

// -- a node's declared inputs -------------------------------------------

/// `.uses`'s `font-size: 10px`, which the capsule in it inherits — the
/// sensitivity row's size, and for the same reason: both are a line of prose
/// with a control on the end of it rather than a row of the pane's own type.
pub const USES_SIZE: f32 = 10.0;

/// `.uses`'s `padding: 2px 10px 4px 12px`, top and bottom. The left is
/// [`PARAM_PAD_L`] and the right [`PARAM_PAD_R`] — the same indent a parameter
/// row stands on, because the line belongs to the rows under it.
pub const USES_PAD_T: f32 = 2.0;
/// The bottom of `.uses`'s `padding: 2px 10px 4px 12px`, which is a little more
/// than the top for [`SENS_PAD_B`]'s reason: the line sits tight under the head
/// it belongs to and clear of the rows it stands over.
pub const USES_PAD_B: f32 = 4.0;

/// `.uses`'s `gap: 6px`, between the slot's word and the capsule.
pub const USES_GAP: f32 = 6.0;

/// A `.uses` capsule's box: [`USES_SIZE`] at [`LINE`] inside a pill's border —
/// [`SENS_CHIP_H`]'s arithmetic at this row's own type size, and the same
/// number, because the two rows are the same size.
pub const USES_CHIP_H: f32 = USES_SIZE * LINE + HAIRLINE * 2.0;

/// One `.uses` line's box: a capsule inside that padding — 23.
pub const USES_H: f32 = USES_PAD_T + USES_CHIP_H + USES_PAD_B;

// -- the sequencer bay --------------------------------------------------

/// `.seq`'s `padding: 7px 9px 9px`: the ring of card the bay's rows sit inside,
/// under the bay head. Two constants because the top differs from the sides and
/// the bottom is the sides' number again.
pub const SEQ_PAD_X: f32 = 9.0;
pub const SEQ_PAD_TOP: f32 = 7.0;

/// `.seq`'s `gap: 5px`, between the head, the ruler and the body.
pub const SEQ_STACK_GAP: f32 = 5.0;

/// `.seq-head`'s `gap: 6px`, between the mode pill and the step readout.
pub const SEQ_HEAD_GAP: f32 = 6.0;

/// `.seq-body`'s `gap: 3px`, between two lanes. Tighter than the stack's 5: the
/// rows are one pattern read against itself, which is why they share a ruler.
pub const SEQ_BODY_GAP: f32 = 3.0;

/// `.seq-row`'s `grid-template-columns: 30px 1fr auto` and its `gap: 5px`: label column and cell gap.
pub const SEQ_LABEL_W: f32 = 30.0;
pub const SEQ_ROW_GAP: f32 = 5.0;

/// `.seq-label`'s `font-size: 10px`: a lane's name, smaller than the bay's own
/// type because it is a label on a row rather than a reading.
pub const SEQ_LABEL_SIZE: f32 = 10.0;

/// `.seq-lane`'s `gap: 2px`, between two cells.
pub const SEQ_CELL_GAP: f32 = 2.0;

/// `.seq-lane i`'s `height: 15px` and its `border-radius: 3px`: step cell in a lane (ADR-0306).
pub const SEQ_CELL_H: f32 = 15.0;
pub const SEQ_CELL_RADIUS: u8 = 3;

/// `.seq-ruler`'s `font-size: 9px` and its `margin-left: 35px`: ruler count over cells.
pub const SEQ_RULER_SIZE: f32 = 9.0;
pub const SEQ_RULER_INSET: f32 = 35.0;

/// `.seq-play .lane i`'s `border-radius: 5px`: the playhead, the column
/// standing over the rows at the step the poll last answered.
pub const SEQ_PLAY_RADIUS: u8 = 5;
