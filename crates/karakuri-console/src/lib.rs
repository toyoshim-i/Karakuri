//! The console panel: its default arrangement — which regions the panel has,
//! how they nest, and the sizes and constraints each of them starts with — the
//! model a view drives that arrangement with, in [`panel`], and the view
//! itself, in [`view`].
//!
//! The arrangement is one [`Spec`] value and nothing else. `karakuri-layout`
//! knows how to solve an arrangement and knows nothing about *this* one; this
//! crate knows this one.
//!
//! **The code that reads the solved rectangles lives here too**, in [`panel`]
//! — what a pointer at a coordinate is touching, what a drag does to a
//! boundary, and what a fold or a solo did. That is on purpose: an egui view
//! that grew its own model would be a second answer to *how a pointer moves a
//! divider*, and two answers disagree quietly because each has its own passing
//! tests.
//!
//! # Which half of "no toolkit here" is still true
//!
//! This module used to say the crate pulled in no toolkit at all, and that
//! **half of it has stopped being true**: `egui`, `egui-wgpu` and `egui-winit`
//! are dependencies, because [`view`] draws the console and it lives in this
//! crate. [ADR-0155](../../../docs/adr/0155-egui-draws-the-panel-and-the-price-is-wgpu-30.md)
//! is the decision, and this is the crate it lands in.
//!
//! **The half that stays is the one ADR-0156 was about, and it is the rule
//! rather than the accident: the arrangement and [`panel`] know no toolkit,
//! and their tests still run with no device.** [`arrangement`] is built from
//! `karakuri-layout` and nothing else; [`panel`] holds a [`Layout`] and a
//! pointer, and it does not draw and cannot. Every test of either runs at full
//! speed on a machine with no adapter, and none of them is under `mod gpu`.
//! What a toolkit is allowed to do here is *read* the solved rectangles and
//! paint them, which is what [`view`] does and the whole of it.
//!
//! [`input`] is the seam between the two, and it is here rather than in the
//! window loop because it is a rule and not plumbing: a boundary drag is the
//! panel's and everything else is `egui`'s, and both would otherwise think
//! they were dragging.
//!
//! [`repaint`] is here for the same reason and it is the same kind of rule:
//! **whether a frame is owed at all.** ADR-0164's still-panel clause says a
//! panel with nothing changing on it does no per-frame work, and the way that clause is
//! got wrong is by omission — one path that changes the screen and reaches no
//! repaint, which shows as a stale control and says nothing anywhere. So the
//! decision is a value with one list of everything that can change the
//! console, and a test asks it rather than an operator noticing.
//!
//! [`budget`] is the other half of that rule and it is inputs rather than a
//! decision: ADR-0164's second clause has every live region declare **what its
//! update costs and how stale it may get**, and that module is where the two
//! numbers, the panel's budget and the frame interval are written down.
//! Nothing reads them on a frame — the two schedulability conditions are
//! arithmetic over the declarations and `tests/schedulable.rs` asserts them,
//! which is what the principle asks for in place of a stage discovering them.
//! There is still no scheduler.
//!
//! # Where the numbers come from
//!
//! `docs/manual/console.html` is the reference the panel is checked against,
//! and its mock is a real layout rather than a picture of one: the widths in
//! `docs/manual/style.css` are the arrangement's widths, and a row's height is
//! its box — padding, plus font size times the console's `line-height: 1.5`.
//! Every number below carries the rule it was read off, and **every derived
//! number is rounded to the nearest whole pixel**, said once here rather than
//! at each of them.
//!
//! The mock does not state everything. A web page's bays are content-height
//! and the page scrolls, so the CSS states **no minimum in the column axis at
//! all**, and it states no maximum anywhere. Each of those is a decision, and
//! each one says below what it was decided from.
//!
//! # Two things the manual states that the shape of the tree has to carry
//!
//! **"Every divider drags, because a preview's size is a machine's answer
//! rather than a layout's."** So no region is pinned by having its minimum
//! meet its maximum, except the transport and the outputs row, which have no
//! divider to drag in the first place — they are content-height rows with no
//! grip drawn on them. Everywhere else there is room between the minimum and
//! the viewport for the operator to make the program small on a weak machine
//! and large on a strong one.
//!
//! **"Program, sized by height."** The centre is a *column*, so the
//! boundary the operator drags there is the program's bottom edge, and the
//! program is [`Sizing::Fixed`](karakuri_layout::Sizing::Fixed) along it. That
//! is the whole of the manual's argument in the model: a 16:9 view that
//! followed the width would be 763 pixels tall in a 1900-wide window and would
//! eat the inspector, so the width does not get to decide. The program keeps
//! the height it was given when the window widens, the inspector absorbs the
//! change, and the picture letterboxes into whatever width it has — which is
//! the program view's job and not this crate's.
//!
//! **The program has no maximum, and neither has anything above it.** ADR-0157
//! is exactly why: a maximum is honoured, so a soloed region that had one
//! would be left holding its maximum with the rest of the window as trailing
//! space. `console.html` promises the opposite — *"the panel folds away and
//! only the picture is left, which is also how you capture this window"* — so
//! an unbounded maximum on `program`, `centre` and the body row is load
//! bearing rather than a default nobody got round to changing.

pub mod budget;
pub mod input;
pub mod panel;
pub mod repaint;
pub mod room;
pub mod view;

/// The toolkit, re-exported, and **the version pairing is this crate's to
/// state**.
///
/// `egui`, `egui-wgpu` and `egui-winit` have to move together, and
/// `egui-wgpu` is what pins `wgpu`: 0.32 wants 25, 0.33 wants 27, 0.34 and
/// 0.35 want 29, and 0.36 wants 30 (ADR-0155). A consumer that depended on
/// `egui-wgpu` itself could pick a version that wants a different `wgpu`, and
/// then the engine and the panel cannot share a `Device` — which is the whole
/// of what ADR-0155 paid a major version for. So a window loop written against
/// this crate takes all three from here.
pub use {egui, egui_wgpu, egui_winit};

use karakuri_layout::{Layout, Spec};

// For the inspector's divider, which is `.divider-v`'s width and is stated
// once, in `room::size`. See [`room::size::PANE_DIVIDER`].
use crate::room::size;

/// Between the transport, the body row and the outputs row: `.console`'s
/// `gap: 10px`.
const ROOT_DIVIDER: f32 = 10.0;

/// Between the three columns: `.body-grid`'s `gap: 10px`.
const COLUMN_DIVIDER: f32 = 10.0;

/// Between the bays stacked inside one column: `.col`'s `gap: 10px`.
const BAY_DIVIDER: f32 = 10.0;

/// Between the Program bay's two regions: the console's own 4px divider between
/// the picture and the row of deck previews under it.
const PROGRAM_DIVIDER: f32 = 4.0;

/// **The narrowest an inspector pane may be**, and the one number the centre's
/// declared width is built from.
///
/// A pane is exactly as wide as the parameter rows drawn in it — `node_into`
/// in [`view`] gives a row the pane's full width and nothing insets it — so
/// this is one `.param` read across: [`size::PARAM_PAD_L`] 12, the ordinal's
/// [`size::PARAM_ORD_W`] 15, the name's [`size::PARAM_NAME_W`] 88, the
/// value's [`size::PARAM_VAL_W`] 58, the three [`size::PARAM_GAP`]s between
/// the four tracks, and [`size::PARAM_PAD_R`] 10 — **207**, which is what the
/// row needs before the fader has any width at all.
///
/// **Plus one pixel, because the fader is the track that is left over.** The
/// mock's fourth track is `1fr`, so what a pane gives the fader is whatever it
/// has over 207, and `param_into` in [`view`] draws the fader only where that
/// leftover is positive. At exactly 207 the leftover is zero and the pane
/// holds parameter rows with no fader in them — the control the bay exists
/// for, missing, on a pane at its own declared minimum. One logical pixel is
/// the least leftover that is still a whole pixel, which is the rounding rule
/// stated once in the module documentation.
///
/// **So a pane is 208 and the fader at that pane is one pixel wide.** This is
/// a floor under the solve rather than a comfortable width, exactly as the
/// right pane's 172 is the Mixer's last strip fitting with nothing to spare;
/// what a pane is at a window anyone works at is far wider — 335.5 at 1920,
/// where the fader is 128.5.
///
/// **The one pixel is where this differs from the mixer's threshold**, and it
/// is the reason ADR-0272's 2 x 207 + 9 is not the answer: `strip_box`'s
/// condition is *not narrower than* the fader column, so 172 is a width at
/// which the strips are drawn, while a fader's is *wider than* its fixed
/// tracks, so 207 is a width at which the fader is not.
const INSPECTOR_PANE_MIN: f32 = size::PARAM_PAD_L
    + size::PARAM_ORD_W
    + size::PARAM_GAP * 3.0
    + size::PARAM_NAME_W
    + size::PARAM_VAL_W
    + size::PARAM_PAD_R
    + 1.0;

/// The console's default arrangement.
/// child is `.body-grid` in the mock, and *the body row* wherever the
/// comments below have to refer to it.
///
/// # The names
///
/// Every name here is the manual's word for the region, because a name is
/// addressable from four surfaces and so is chosen once: `transport`,
/// `library`, `staging`, `program`, `inspector`, `mixer`, `master`,
/// `sequencer`, `outputs` are the headings of *What each region is standing
/// on* and the bay heads of the mock.
///
/// Three more names are the columns those bays are stacked in, which the
/// manual names in its lede rather than as regions: `left-pane`, `centre` and
/// `right-pane`. `left-pane` is `karakuri-layout`'s own word — *"the
/// console's left pane is a split, holding the library and the staging lane
/// stacked, and 'fold the left pane away' is an operation the keyboard, a
/// MIDI map and MCP each reach by that name"* — and `right-pane` is that same
/// operation on the other side.
///
/// **`centre` is deliberately not a third pane** (ADR-0159): the side panes
/// are what an operator folds away to give room, and the centre is what they
/// fold them away *for*, so one noun over all three would assert a symmetry
/// the console does not have. It is named all the same, because it is
/// addressed — the program's bottom edge is a divider of *this* split, and a
/// drag on the program's height reaches it by name.
///
/// The body row holding all three has **no name**, and neither has the root
/// column it sits in — and *nobody addresses them* is not the reason.
/// `Layout::hit` hands a split out as `Hit::Divider { split, .. }`,
/// `crates/karakuri/src/main.rs`'s fold-at-pointer turns that into `Op::Fold(split)`,
/// and `Outcome::Folded`'s `root` exists to report the root's own case. So
/// **both splits fold through the pointer today**; what they cannot be is
/// reached by anything holding only a name — a keyboard, a MIDI map or MCP.
/// **Naming them was the decision ADR-0197 left open, and it has been taken:
/// they stay unnamed** (ADR-0204). Naming them would assert that folding the
/// whole panel away, and folding the row of three panes, are operations an
/// operator asks for. Neither has a row on the manual's operations page — the
/// blank window a folded root leaves is not what anybody is reaching for, and
/// *Solo a region* is how the page reaches the outcome that is (*"the panel
/// folding away and only the picture left"*), while the body row has no word
/// on that page at all. **What it costs is that `panel::Op` cannot become
/// `Operation`**, so the console's arrangement operations stay the one surface
/// that does not route into the vocabulary, and *"arranging the console is
/// reachable from a pointer and nothing else"* stays true for these two.
///
/// **The pointer no longer reaches what they hide**: a folded root is not a
/// hit target, because a node that is not laid out is not hit-testable
/// (ADR-0204, with
/// [P-0073](../../../docs/principles/0073-a-node-claims-only-what-its-visible-content-can-use.md)
/// and ADR-0193 as the same rule on the other two axes). `Op::UnfoldAll` and
/// `Op::Reset` name no target, so the way back never went through the pointer.
///
/// `inspector-1` and `inspector-2` are the inspector's two panes. The
/// inspector itself is the split, so the divider between them is reached as
/// `inspector` plus an index and the panes need no name for *that*; they have
/// one because a view's name is required, and because a pane is the unit the
/// mock's `2 up` control counts.
///
/// `program-view` and `deck-previews` are the Program bay's two regions, and
/// **both are addressed rather than merely named**: the manual lists the
/// picture in Outputs as *program view* and says it is on screen exactly when
/// that sink is on, which is a fold by name, and it says *"the deck previews
/// under it are auditions of their own, so they stay when it goes"*, which is
/// the other one not folding with it. See [`program`].
pub fn arrangement() -> Spec {
    Spec::column(
        ROOT_DIVIDER,
        vec![
            // `.transport`: 9px of padding above and below the `.bpm`, which
            // is 20px at the console's line-height 1.5 — 9 + 30 + 9.
            //
            // Minimum meets maximum, and that is the point: the transport is a
            // row of readouts at a fixed type size, it has no grip in the
            // mock, and there is no answer to "how tall should it be" other
            // than the height of its contents.
            Spec::view("transport").fixed(48.0).min(48.0).max(48.0),
            Spec::row(COLUMN_DIVIDER, vec![left_pane(), centre(), right_pane()])
                .flex(1.0)
                // The body row's minimum height is the tallest of the
                // three columns'. **It changed hands on 2026-08-29**: it was
                // the right pane's 530 until the manual gave the inspector a
                // deck head, and the centre is now the taller at 556.5 —
                // `program` 395, the divider 10, and `inspector` 151.5. The
                // program's term is its *fixed* height and not its own
                // minimum, because a window that cannot show the Program bay
                // at the size the mock draws it is not a window this console
                // is claimed to work at; it grew by 17 when the preview cells
                // got their captions.
                // **The model does not derive this**: a split's minimum is a
                // number it is given, not a function of its children's, so
                // the sum is written here and `tests/arrangement.rs`
                // recomputes it from the tree so the two cannot drift apart.
                // That guard is what caught the 9.5 px, and the console's
                // claimed smallest window was short by exactly that from the
                // moment the row was specified.
                .min(556.5),
            // `.outputs`: 8px of padding above and below a `.sink` row, which
            // is 11px at line-height 1.5 plus its 1px border top and bottom —
            // 8 + 18.5 + 8, rounded. Fixed for the same reason the transport
            // is: a row of chips at a fixed type size, and no grip.
            Spec::view("outputs").fixed(34.0).min(34.0).max(34.0),
        ],
    )
}

/// The arrangement, built. Every caller wants the [`Layout`], not the
/// [`Spec`]; the `Spec` is public because it is the thing that is *read*.
pub fn layout() -> Layout {
    Layout::new(arrangement())
}

/// **The smallest viewport at which every region still holds the minimum it
/// declares**, in the logical pixels [`arrangement`] is written in — and what
/// a host passes to its window as a minimum inner size.
///
/// # What it is the minimum *of*
///
/// Below this the solve stops honouring the minima and scales the whole
/// arrangement down together
/// ([ADR-0250](../../../docs/adr/0250-below-the-minima-the-arrangement-scales-rather-than-being-rewritten.md)),
/// which keeps every rectangle sane and takes the panel's controls with it: a
/// mixer strip narrower than its fader column draws nothing, and the four
/// preview cells beside it need only positive area, so the pointer loses the
/// deck selection while the keys keep it. **A window that cannot be dragged
/// below this cannot reach that state**
/// ([ADR-0272](../../../docs/adr/0272-the-window-has-a-minimum-and-only-one-of-adr-0250s-three-cases-is-real.md)).
///
/// # Both figures, term for term
///
/// **Both are the console's own**, and neither is in the mock: the stylesheet
/// states no minimum for the two outer tracks and no minimum in the column
/// axis at all, so what is summed here is the minima this file chooses, each
/// argued where it is declared.
///
/// **777 wide** is the body row's three tracks at the minima each declares,
/// plus the two [`COLUMN_DIVIDER`]s between them: `left-pane` 160, `centre`
/// 425, `right-pane` 172, `+ 10 + 10`. Two of the three are thresholds in
/// [`view`] met to the pixel. The right pane's 172 is where the Mixer bay's
/// last strip fits — a track is 37 and the fader column is 29 inside 4 + 4 of
/// padding, which is `strip_box`'s own condition — so one logical pixel
/// narrower and the bay draws no strips at all. The centre's 425 is
/// [`INSPECTOR_PANE_MIN`] twice over one [`size::PANE_DIVIDER`], which is the
/// narrowest centre an inspector pane can draw a parameter fader in.
///
/// **It was 692, and the term that moved is the centre's**: 340 was
/// `.body-grid`'s CSS track and not a reading of the console's own content,
/// which is what [ADR-0279](../../../docs/adr/0279-the-centre-is-two-parameter-rows-wide-because-a-pane-that-cannot-draw-a-fader-is-not-a-minimum.md)
/// changed and what the paragraph below used to have to say instead.
///
/// **658.5 high** is the root column's three rows at theirs, plus the two
/// [`ROOT_DIVIDER`]s: `transport` 48, the body row 556.5, `outputs` 34,
/// `+ 10 + 10`. The body row's 556.5 is the centre's, and where it comes from
/// is written at the row itself.
///
/// **The declared minima and not the capped ones.** ADR-0174 caps a claim by
/// what a node's visible content can use, so an arrangement with its bays
/// folded away needs less than this; a window minimum that followed that
/// would move as an operator folds, and would leave a window that cannot be
/// grown back to what unfolding needs. These numbers are a property of the
/// tree, fixed when the [`Spec`] is built, and no drag, fold, session or
/// screen changes them.
///
/// **It is a floor under the solve rather than a promise about every
/// control**, and every width term in it is now a reading of what the console
/// draws. The centre's was not: at the 340 the CSS track states, a pane is
/// 165.5 where a parameter row's fixed tracks want 207 before the fader has
/// any width, so the faders were not drawn at the centre's own declared
/// minimum — which a divider drag reaches at any window width, so no window
/// minimum could close it. It was closed on the axis it was on, by moving the
/// declared minimum; this constant moved with it.
///
/// **What it still does not promise is comfort.** At 777 a parameter fader is
/// one pixel wide and a mixer strip clears its own threshold by nothing at
/// all: a floor is where the panel stops being able to draw what it describes,
/// and not where it is pleasant to work.
///
/// `tests/arrangement.rs` recomputes both figures from the tree, so this is a
/// claim about the arrangement rather than a copy of one.
pub const MINIMUM_VIEWPORT: (f32, f32) = (777.0, 658.5);

/// The library over the staging lane.
///
/// **Which of the two absorbs the change is stated by the mock**: the library
/// bay carries `style="flex:1"` and the staging bay carries nothing, so
/// staging is content-height and the library takes what is left. The same
/// reading decides the other two columns, and the mock confirms it a second way
/// — a `.grip` is drawn in the bay head of exactly the bays that absorb, and
/// on none of the others.
fn left_pane() -> Spec {
    Spec::column(
        BAY_DIVIDER,
        vec![
            // Minimum: the bay head (6 + 15 + 6 = 27), the scope row
            // (7 + 16.5 + 7 + 1 = 31.5), a list of three rows (3 + 3 of
            // `.lib-list` padding, plus 3 x 22.5 = 73.5) and the foot
            // (5 + 15 + 5 + 1 = 26) = 158. A library shorter than one scope
            // row and three results is not a library you can ask a question
            // of, which is what *What each region is standing on* says it is
            // for. The mock states no such minimum: this is chosen.
            //
            // **It was 132, and the two terms it was short are the two the
            // bay grew.** The sum was written when neither the scope row nor
            // the foot was drawn: it counted the row it could not yet draw,
            // a pixel light and with its rule left out, and did not count the
            // foot at all. At 132 the bay this file describes has room for
            // one result rather than three, which is the sentence above
            // quietly ceasing to be true — so the terms are the boxes
            // `room::size` states and the number is their sum.
            Spec::view("library").flex(1.0).min(158.0),
            // 27 of bay head, 6 + 8 of `.stage-list` padding, three `.cand`
            // rows at 4 + 16.5 + 4, and two 5px gaps — the mock's own staging
            // lane, two candidates and the note under them, at its natural
            // height.
            //
            // Minimum: the same without the second candidate and the note —
            // 27 + 14 + 24.5. Below one waiting candidate the lane says
            // nothing that its header's "2 waiting" does not.
            Spec::view("staging").fixed(125.0).min(66.0),
        ],
    )
    .named("left-pane")
    // `.body-grid`'s first track: `340px` (ADR-0239).
    .fixed(340.0)
    // Minimum: a library row at its narrowest useful — `.lib-list` padding
    // 3 + 3, `.lib-row` padding 7 + 7 and two 7px gaps, a star, a Set name of
    // a dozen characters and a duration, at the code face's ~6.6px per
    // character. The mock states no minimum for this track; 160 is chosen, and
    // it is the width at which a row still reads as a name and a time rather
    // than as an ellipsis.
    .min(160.0)
    // No maximum. See the module documentation: a maximum is honoured, so one
    // here would put trailing space beside a soloed left pane. Nothing else
    // needs it — the centre is the only flexible child of the body row, so a
    // wide window widens the centre and this track stays where it is put.
    .max(f32::INFINITY)
}

/// The program over the inspector.
fn centre() -> Spec {
    Spec::column(BAY_DIVIDER, vec![program(), inspector()])
        .named("centre")
        // `.body-grid`'s middle track is `minmax(340px, 1fr)`: flexible, which
        // this takes, and a minimum of 340, which it no longer does.
        .flex(1.0)
        // Minimum: the inspector's two panes at [`INSPECTOR_PANE_MIN`] with one
        // [`size::PANE_DIVIDER`] between them — 208 + 9 + 208 = **425**. The
        // panes are the narrowest thing in this column: the Program bay above
        // them declares no width minimum at all — its picture is a rectangle
        // fitted into whatever it is given — so the centre is as narrow as an
        // inspector pane lets it be, exactly as the right pane is as narrow as
        // four mixer strips side by side let it be.
        //
        // **It was the CSS track's 340**, which is a number about a web page's
        // grid rather than a reading of anything this console draws: at 340 a
        // pane is 165.5 against the 207 a parameter row's fixed tracks want, so
        // the bay drew rows with no fader in them at the centre's own declared
        // minimum — and a divider drag reaches that minimum at any window width
        // at all, which is why no window minimum could close it (ADR-0272,
        // ADR-0279).
        .min(INSPECTOR_PANE_MIN * 2.0 + size::PANE_DIVIDER)
        // No maximum: the centre is what a fold gives its width to, and what a
        // solo on the program has to be able to fill.
        .max(f32::INFINITY)
}

/// The Program bay, which is a split: the picture, and the row of deck
/// previews under it.
///
/// **Two regions that fold apart, because `console.html` says so**: *"The bay
/// is two regions and they fold apart. The picture is a sink, listed in
/// Outputs as program view, and it is on screen exactly when that sink is on
/// — so there is no state where it is hidden and still costing a pass. The
/// deck previews under it are auditions of their own, so they stay when it
/// goes."* A fold is the operation that turns a sink off here, and a fold acts
/// on a node — so the picture has to *be* a node, and so does the row that
/// outlives it.
///
/// # The bay is 395, and the split is that 395 read out loud
///
/// The number is the height the mock's own program bay has at the narrowest
/// console the mock will draw: `.console`'s `min-width: 1010px` less its 10px
/// padding either side is 990, so the centre track is
/// 990 - 218 - 268 - two 10px gaps = 484. `.program-body`'s 9px padding leaves
/// 466 for the picture, which at 16:9 is 262.125 tall and is transcribed as
/// the whole pixel the mock rasterises it at, **262** — so the picture's
/// region is 16:9 to a quarter of a pixel rather than exactly, which is a
/// difference `view::picture_rect` is the one place that has to care about;
/// the four `.preview` images
/// are (466 - three 6px gaps) / 4 = 112 wide and so 63 tall, and each carries
/// a `.cell`'s 4px gap and a `.caption`'s 13px under it, which is **80** for
/// the row. **Bay head 27, padding 9 + 9, picture 262, gap 8, previews 80 =
/// 395** — and that sum is the split, term for term, with the 8 as the
/// divider:
///
/// - `program-view` is 27 + 9 + 262 = **298**. The bay head is inside it
///   because a bay's head is painted over the top of whatever tiles the bay —
///   which is what the inspector's two panes already do — and the top 9 is
///   `.program-body`'s padding above the picture.
/// - `deck-previews` is 80 + 9 = **89**: the row of previews and the padding
///   under it.
///
/// 298 + 8 + 89 = 395.
///
/// **The 8 in that sum is the mock's `.program-body` gap and
/// [`PROGRAM_DIVIDER`] is 4**, which is a four-pixel difference this
/// derivation has carried since the divider was halved and the prose was not.
/// It is not a gap in the bay: `program-view` is the flexible child, so the
/// four pixels the divider gives back land in the picture's region and the
/// bay's total is the number it declares either way. Named here rather than
/// quietly reconciled, because the two are one derivation or neither and
/// whoever wants them to agree has to decide which of the two moves.
///
/// # The row grew rather than the cells shrinking, and that is a decision
///
/// The caption under a cell is 17 pixels the row did not have — a
/// `.cell` gap and a `.caption` — and there were two places to take them
/// from. **The cells could have kept the row at 72 and got shorter**, which
/// costs nothing anywhere else in the arrangement and is wrong twice over. An
/// image is 16:9 and centred in its track
/// ([ADR-0170](../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md)),
/// so 17 off its height takes 30 off its width as well: a cell 112 x 63 would
/// become 82 x 46, and the row would lose a quarter of the thing it exists to
/// show in order to make room for a label about it. And the 63 is not this
/// crate's to spend — it is a **reading of the mock**, `.preview`'s
/// `aspect-ratio` at the width the mock's own narrowest console gives the row,
/// so shrinking it would be inventing a number the stylesheet does not carry
/// and the transcription guard would have nothing to hold it against.
///
/// The mock's bay is a flow and its height is the sum of its parts, so the
/// mock's bay grew by the same 17 the moment the caption was added to it.
/// **395 is that sum read out loud, exactly as 378 was.** The row is also the
/// one the operator sizes — the divider above it is what
/// `view::program_bay` reads back through `Layout::sizing` — so what grew is
/// the row's *default*, and a row's default is what its content needs
/// ([ADR-0174](../../../docs/adr/0174-a-node-claims-only-what-its-visible-content-can-use.md)).
/// The content grew.
///
/// What "fixed" buys is the whole argument: widen the window and this stays
/// 395 instead of following the width up to the 763 the manual works out for a
/// 1900-wide window. The picture letterboxes into the width it has, which is
/// the program view's job.
///
/// # The minimum splits the same way
///
/// The bay's chrome at that width is 27 + 18 + 80 + 8 = 133, so the stated 217
/// leaves 84 for the picture — small on purpose, because a preview's size is a
/// machine's answer and a weak machine's answer is small rather than absent.
/// Term for term again: `program-view` at 27 + 9 + 84 = **120**, the divider's
/// 8, and `deck-previews` at **89**, which is its size — a row of four cells
/// at a fixed type size has nothing in it that gets smaller, exactly as the
/// mixer's strips have not. 120 + 8 + 89 = 217, so the bay's own minimum is
/// still the one it declares rather than a number its children now imply.
///
/// # No maximum anywhere in here
///
/// Load bearing on the bay for ADR-0157's reason — `solo` on the program has
/// to leave the program holding the window — and load bearing on both children
/// for the same reason one level down: *"Solo the program view: the panel
/// folds away and only the picture is left"* is a solo on `program-view`, and
/// a maximum on it would leave a margin in a window somebody is capturing.
fn program() -> Spec {
    Spec::column(
        PROGRAM_DIVIDER,
        vec![
            // The picture, and what absorbs the bay's height: dragging the
            // program's bottom edge is the operator answering *how big should
            // the picture be*, and the preview row underneath is not part of
            // that answer.
            Spec::view("program-view")
                .flex(1.0)
                .min(120.0)
                .max(f32::INFINITY),
            // The previews, at the height four `.preview` cells are — the same
            // reading of the mock that makes `staging` content-height and the
            // library flexible, and the same reason the mixer is fixed: there
            // is nothing in the row that gets smaller.
            Spec::view("deck-previews")
                .fixed(89.0)
                .min(89.0)
                .max(f32::INFINITY),
        ],
    )
    .named("program")
    .fixed(395.0)
    .min(217.0)
    .max(f32::INFINITY)
}

/// The inspector, which is a split: *n* panes side by side.
///
/// **Two of them, because the model has no other number to offer.** The mock
/// draws two (`.insp-split`'s `1fr 9px 1fr`) and its header says `2 up`, with
/// a tooltip saying a wide window fits three or four and that the operator
/// chooses how many *while running*. A [`Spec`] builds a [`Layout`] once and
/// the arena has no insert or remove, so the count is settled here and cannot
/// be changed afterwards without building a new arrangement — which would
/// throw away every drag and fold in it. The default is the mock's two.
fn inspector() -> Spec {
    Spec::row(
        size::PANE_DIVIDER,
        vec![
            // Minimum: [`INSPECTOR_PANE_MIN`] — one parameter row with a fader
            // in it, 208, and the derivation is written at the constant.
            //
            // **It was 165, and it was read off the parent**: (340 - 9) / 2
            // rounded down, half of what the centre said it would not go below.
            // Deriving it that way was deliberate and it was the wrong way
            // round, because the content did not fit in what came back — a pane
            // whose minimum its own rows overflow is not a minimum. The pane now
            // reads its own content and the centre sums the panes (ADR-0279).
            //
            // **The mock's own two numbers still do not agree, and 208 is
            // neither of them**: `.console`'s `min-width: 1010px` holds the
            // centre at 484 and a pane at 237, while `.body-grid`'s middle track
            // says 340 and a pane of 165.5. This is a reading of `.param`, which
            // is what makes it a minimum rather than a transcription.
            Spec::view("inspector-1").flex(1.0).min(INSPECTOR_PANE_MIN),
            Spec::view("inspector-2").flex(1.0).min(INSPECTOR_PANE_MIN),
        ],
    )
    .named("inspector")
    // Flexible: the inspector is what gives height to the program and takes it
    // back, which is the other half of "sized by height".
    .flex(1.0)
    // Minimum: bay head 27, `.half-head` 27.5, the `.deck-head` under it at
    // 25.5, one `.node-head` 26.5 and two `.param` rows at 22.5 — **151.5**.
    // A deck head, one node and two of its parameters is the least that shows
    // what the inspector is for: a deck's clock and its fold, over a group
    // addressed by node with rows under it.
    //
    // **Every term is `room::size`'s**, so the minimum and the boxes the panes
    // are drawn in are one derivation: [`size::HEAD_H`] 6 + 10 x 1.5 + 6,
    // [`size::HALF_HEAD_H`] 5 + 16.5 + 5 + 1, [`size::DECK_HEAD_H`] 5 + 15.5 +
    // 5, [`size::NODE_HEAD_H`] 5 + 16.5 + 5 and [`size::PARAM_H`] 3 + 16.5 + 3.
    //
    // **The deck head is the term that was missing**, and the 126 this
    // replaces was not a rounding of 151.5 but the same sum without it: 27 +
    // 27.5 + 26.5 + 45 is exactly 126, so the four terms it did have were
    // right to the half pixel and the comment's `27, 27, 27` was the rounding.
    // The row landed in `docs/manual/console.html` on 2026-08-29 — *"It sits
    // under the pane's head rather than in it"* — and a minimum that does not
    // count it is a minimum at which the console's own first pass overflows.
    .min(
        size::HEAD_H
            + size::HALF_HEAD_H
            + size::DECK_HEAD_H
            + size::NODE_HEAD_H
            + size::PARAM_H * 2.0,
    )
}

/// The mixer over the master chain over the sequencer.
///
/// The mock gives a `.grip` to the master bay alone of the three, so the
/// master chain is what absorbs the pane's height and the other two keep
/// theirs. That is the same rule the left pane's `flex:1` states outright.
fn right_pane() -> Spec {
    Spec::column(
        BAY_DIVIDER,
        vec![
            // Bay head 27, `.mixer-strips` (6 + 6 of padding, and a `.strip`
            // of 7 + 7 padding, five 5px gaps and its contents: name 15, tally
            // 13.5, trim 13.5, `.fader-col`'s stated 104, number 15, mode
            // 15.5) = 227.5, and `.xfade` (1px rule, 8 + 10 padding, a 16.5
            // row, a 7px gap and an 18.5 row) = 61. **The 16.5 row and the gap
            // were the crossfader's**, and the mock stopped drawing them the
            // day the mixer was decided to have no crossfader — so `.xfade` is
            // 37.5 there and this bay reserves 23.5 for a row nothing draws.
            // The height has not been re-derived, which is a decision about
            // the bay rather than a transcription, and it is written down
            // rather than silently carried.
            //
            // **Minimum equals the size**, and that is the manual rather than
            // laziness: *"Four channel strips, all visible, nothing that
            // scrolls out of reach mid transition."* The strips' height is
            // `.fader-col`'s fixed 104 plus fixed type; there is nothing in
            // the bay that gets smaller, so a mixer shorter than 316 is a
            // mixer with a strip cut off. It has no maximum, so it may still
            // grow — it is the only visible child of a soloed right pane, and
            // a fixed child with room to grow takes it (ADR-0157).
            Spec::view("mixer")
                .fixed(316.0)
                .min(316.0)
                .max(f32::INFINITY),
            // Minimum: bay head 27, `.master-body` padding 8 + 10, the out row
            // 16.5, an 8px gap and one `.fx` at 4 + 16.5 + 4. The out fader
            // and one effect is the least that is still a chain.
            Spec::view("master").flex(1.0).min(94.0),
            // Bay head 27, `.seq` (7 + 9 padding, four 5px gaps, head 18.5,
            // ruler 13.5, four 15px lanes with three 3px gaps = 69, foot 18.5).
            //
            // Minimum: the same with one lane and no foot — 27 + 16 + 18.5 +
            // 5 + 13.5 + 5 + 15. A sequencer showing the ruler and one lane is
            // still one; showing the ruler alone is not.
            Spec::view("sequencer").fixed(178.0).min(100.0),
        ],
    )
    .named("right-pane")
    // `.body-grid`'s third track: `400px` (ADR-0239).
    .fixed(400.0)
    // Minimum: four mixer strips still side by side, which is the narrowest
    // thing in the pane and the one the manual pins. A `.strip` is 4 + 4 of
    // padding around a `.fader-col` of `.vfader` 17, a 6px gap and `.vmeter`
    // 6 — 37 — and four of them with three 4px gaps inside `.mixer-strips`'
    // 6 + 6 padding is 172. The mock states no minimum for this track; this
    // one is read off the strip it has to hold.
    .min(172.0)
    // No maximum, for the reason the left pane and the centre have none.
    .max(f32::INFINITY)
}
