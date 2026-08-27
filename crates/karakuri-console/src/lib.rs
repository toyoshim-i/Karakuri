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
//! **whether a frame is owed at all.** P-0072's first clause says a panel with
//! nothing changing on it does no per-frame work, and the way that clause is
//! got wrong is by omission — one path that changes the screen and reaches no
//! repaint, which shows as a stale control and says nothing anywhere. So the
//! decision is a value with one list of everything that can change the
//! console, and a test asks it rather than an operator noticing.
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

/// Between the Program bay's two regions: `.program-body`'s `gap: 8px`, which
/// is the gap the mock leaves between the picture and the row of deck
/// previews under it. The second divider in the console that is not 10, and
/// like the inspector's it is inside a card rather than in the ground.
const PROGRAM_DIVIDER: f32 = 8.0;

/// The console's default arrangement.
///
/// A column of three — the transport, the row of three columns, and the
/// outputs row — where each column holds its own stack of bays. That middle
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
/// `examples/panel.rs`'s fold-at-pointer turns that into `Op::Fold(split)`,
/// and `Outcome::Folded`'s `root` exists to report the root's own case. So
/// **both splits fold through the pointer today**; what they cannot be is
/// reached by anything holding only a name — a keyboard, a MIDI map or MCP.
/// **Naming them is a decision nobody has taken** (ADR-0197): it would assert
/// that folding the whole panel away, and folding the row of three panes, are
/// operations an operator asks for, and neither has a row on the manual's
/// operations page.
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
                // three columns' — the right pane's, at 530. **The model
                // does not derive this**: a split's minimum is a number it
                // is given, not a function of its children's, so the sum is
                // written here and `tests/arrangement.rs` recomputes it from
                // the tree so the two cannot drift apart.
                .min(530.0),
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
            // (7 + 16.5 + 7 = 31), and a list of three rows (3 + 3 of
            // `.lib-list` padding, plus 3 x 22.5) = 74. A library shorter
            // than one scope row and three results is not a library you can
            // ask a question of, which is what *What each region is standing
            // on* says it is for. The mock states no such minimum: this is
            // chosen.
            Spec::view("library").flex(1.0).min(132.0),
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
    // `.body-grid`'s first track: `218px`.
    .fixed(218.0)
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
        // `.body-grid`'s middle track: `minmax(340px, 1fr)` — flexible, with the
        // minimum the CSS states.
        .flex(1.0)
        .min(340.0)
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
/// # The bay is still 378, and the split is that 378 read out loud
///
/// The number is the height the mock's own program bay has at the narrowest
/// console the mock will draw: `.console`'s `min-width: 1010px` less its 10px
/// padding either side is 990, so the centre track is
/// 990 - 218 - 268 - two 10px gaps = 484. `.program-body`'s 9px padding leaves
/// 466 for the picture, which at 16:9 is 262.125 tall and is transcribed as
/// the whole pixel the mock rasterises it at, **262** — so the picture's
/// region is 16:9 to a quarter of a pixel rather than exactly, which is a
/// difference `view::picture_rect` is the one place that has to care about;
/// the four `.preview` cells
/// are (466 - three 6px gaps) / 4 = 112 wide and so 63 tall. **Bay head 27,
/// padding 9 + 9, picture 262, gap 8, previews 63 = 378** — and that sum is
/// the split, term for term, with the 8 as the divider:
///
/// - `program-view` is 27 + 9 + 262 = **298**. The bay head is inside it
///   because a bay's head is painted over the top of whatever tiles the bay —
///   which is what the inspector's two panes already do — and the top 9 is
///   `.program-body`'s padding above the picture.
/// - `deck-previews` is 63 + 9 = **72**: the row of previews and the padding
///   under it.
///
/// 298 + 8 + 72 = 378, so nothing about the bay's height changed and the
/// tests that assert 378 are asserting the same thing they were.
///
/// What "fixed" buys is the whole argument: widen the window and this stays
/// 378 instead of following the width up to the 763 the manual works out for a
/// 1900-wide window. The picture letterboxes into the width it has, which is
/// the program view's job.
///
/// # The minimum splits the same way
///
/// The bay's chrome at that width is 27 + 18 + 63 + 8 = 116, so the stated 200
/// leaves 84 for the picture — small on purpose, because a preview's size is a
/// machine's answer and a weak machine's answer is small rather than absent.
/// Term for term again: `program-view` at 27 + 9 + 84 = **120**, the divider's
/// 8, and `deck-previews` at **72**, which is its size — a row of four cells
/// at a fixed type size has nothing in it that gets smaller, exactly as the
/// mixer's strips have not. 120 + 8 + 72 = 200, so the bay's own minimum is
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
                .fixed(72.0)
                .min(72.0)
                .max(f32::INFINITY),
        ],
    )
    .named("program")
    .fixed(378.0)
    .min(200.0)
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
            // Minimum: (340 - 9) / 2, rounded down — half of what the centre
            // says it will not go below, less the divider between them.
            // Deriving it from the parent rather than from the content is
            // deliberate, because the content does not fit: `.param`'s track
            // list is `15px 88px 1fr 58px` with 8px gaps and 12 + 10 of
            // padding, which is 207 before the fader has any width at all. The
            // mock never meets that, because `.console`'s `min-width: 1010px`
            // holds the centre at 484 and each inspector pane at 237. See
            // the report on 340 against 1010 — the two numbers in the CSS do
            // not agree, and this takes the one the panel is actually held to
            // by its parent.
            Spec::view("inspector-1").flex(1.0).min(165.0),
            Spec::view("inspector-2").flex(1.0).min(165.0),
        ],
    )
    .named("inspector")
    // Flexible: the inspector is what gives height to the program and takes it
    // back, which is the other half of "sized by height".
    .flex(1.0)
    // Minimum: bay head 27, `.half-head` 27, one `.node-head` 27 and two
    // `.param` rows at 22.5. One node and two of its parameters is the least
    // that shows what the inspector is for — a group, addressed by node, with
    // rows under it.
    .min(126.0)
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
            // row, a 7px gap and an 18.5 row) = 61.
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
    // `.body-grid`'s third track: `268px`.
    .fixed(268.0)
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
