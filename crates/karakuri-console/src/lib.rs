//! The console panel: its default arrangement — which regions the panel has,
//! how they nest, and the sizes and constraints each of them starts with — and
//! in [`panel`], the model a view drives that arrangement with.
//!
//! The arrangement is one [`Spec`] value and nothing else. `karakuri-layout`
//! knows how to solve an arrangement and knows nothing about *this* one; this
//! crate knows this one and nothing about drawing.
//!
//! **The code that reads the solved rectangles now lives here too**, in
//! [`panel`] — what a pointer at a coordinate is touching, what a drag does to
//! a boundary, and what a fold or a solo did. That half of the sentence this
//! module used to carry has stopped being true, and it stopped on purpose: an
//! egui view that grew its own model would be a second answer to *how a
//! pointer moves a divider*, and two answers disagree quietly because each has
//! its own passing tests.
//!
//! **The other half stays true, and is the rule rather than the accident:
//! nothing here pulls in a toolkit, a device or a window**
//! ([ADR-0156](../../../docs/adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)).
//! [`panel`] holds a [`Layout`] and a pointer; it does not draw and it cannot.
//! `wgpu`, `winit`, `karakuri-engine` and `pollster` are dev-dependencies for
//! `examples/layout.rs` alone, and the crate's own dependency is
//! `karakuri-layout`. That is what lets the panel's behaviour be a test on a
//! machine with no adapter, and it is what a view is expected to be written
//! *against* rather than inside.
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

pub mod panel;

use karakuri_layout::{Layout, Spec};

/// Between the transport, the body row and the outputs row: `.console`'s
/// `gap: 10px`.
const ROOT_DIVIDER: f32 = 10.0;

/// Between the three columns: `.body-grid`'s `gap: 10px`.
const COLUMN_DIVIDER: f32 = 10.0;

/// Between the bays stacked inside one column: `.col`'s `gap: 10px`.
const BAY_DIVIDER: f32 = 10.0;

/// Between the inspector's two panes: `.insp-split`'s middle track,
/// `grid-template-columns: 1fr 9px 1fr`. The one divider in the console that
/// is not 10, and the mock draws it as a grabbable bar (`cursor: col-resize`).
const INSPECTOR_DIVIDER: f32 = 9.0;

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
/// The body row holding all three is left **unnamed**: nothing folds, solos
/// or drags it as a whole, and a split nobody addresses does not get a name.
///
/// `inspector-1` and `inspector-2` are the inspector's two panes. The
/// inspector itself is the split, so the divider between them is reached as
/// `inspector` plus an index and the panes need no name for *that*; they have
/// one because a view's name is required, and because a pane is the unit the
/// mock's `2 up` control counts.
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
    Spec::column(
        BAY_DIVIDER,
        vec![
            // **Fixed, and this is the manual's "sized by height".** The
            // number is the height the mock's own program bay has at the
            // narrowest console the mock will draw: `.console`'s
            // `min-width: 1010px` less its 10px padding either side is 990, so
            // the centre track is 990 - 218 - 268 - two 10px gaps = 484.
            // `.program-body`'s 9px padding leaves 466 for the picture, which
            // at 16:9 is 262 tall; the four `.preview` cells are
            // (466 - three 6px gaps) / 4 = 112 wide and so 63 tall. Bay head
            // 27, padding 9 + 9, picture 262, gap 8, previews 63 = 378.
            //
            // What "fixed" buys is the whole argument: widen the window and
            // this stays 378 instead of following the width up to the 763 the
            // manual works out for a 1900-wide window. The picture letterboxes
            // into the width it has, which is the program view's job.
            //
            // Minimum: the bay's chrome at that width is 27 + 18 + 63 + 8 =
            // 116, so 200 leaves 84 for the picture. Small on purpose — a
            // preview's size is a machine's answer, and a weak machine's
            // answer is small rather than absent. The mock states no minimum.
            //
            // No maximum, and it is load bearing: `solo` on the program has to
            // leave the program holding the window (ADR-0157).
            Spec::view("program")
                .fixed(378.0)
                .min(200.0)
                .max(f32::INFINITY),
            inspector(),
        ],
    )
    .named("centre")
    // `.body-grid`'s middle track: `minmax(340px, 1fr)` — flexible, with the
    // minimum the CSS states.
    .flex(1.0)
    .min(340.0)
    // No maximum: the centre is what a fold gives its width to, and what a
    // solo on the program has to be able to fill.
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
        INSPECTOR_DIVIDER,
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
