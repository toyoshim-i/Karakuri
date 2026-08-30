//! The console, drawn: every region in its place, with its heading, and
//! nothing inside.
//!
//! This is the half of the crate that knows a toolkit. It reads the solved
//! rectangles out of [`Panel`] and paints them with `egui`; it holds no fact
//! about the arrangement and answers no question [`karakuri_layout::Layout`]
//! can answer.
//!
//! # What is here, and what is deliberately not
//!
//! **Every leaf of the arrangement gets its region, and seven of them get a
//! bay head.** Nothing else. A bay's body is empty and looks it — no
//! placeholder rows, no sample values, no greyed-out control that hints at
//! what will be there. Scaffolding that looks finished does not get replaced,
//! and the next person to open the panel should be in no doubt about what
//! exists.
//!
//! **One body is not empty, and it is not an exception to that.** The Program
//! bay's picture is a [`Kind::Picture`], and what it draws is a texture handed
//! in from outside — real texels off a real device, not a mock-up of some. It
//! draws nothing at all when there is no texture, exactly as every other empty
//! body does. See [`Picture`] and [`picture_rect`].
//!
//! # The deck previews are the one thing drawn with nothing behind it
//!
//! [`View::draw`] paints four cells whether or not a deck is running in any of
//! them, and that is **not** the scaffolding the paragraph above forbids. A greyed-out control is a placeholder for a control that does not
//! exist yet; a preview cell is not a placeholder for anything, it is the
//! region's own face. The mock's `.preview` is a well with a letter in it
//! before it is a picture — three of its four cells hold no texels at all —
//! and the Program bay's head reads *previews 2 of 4*, which is an operator's
//! ordinary choice and not a state waiting to be finished. **An empty cell is
//! what off looks like, not a stand-in for a full one**, and the label says
//! so: `A` where there is a picture and `C · off` where there is not.
//!
//! So the two rules are one rule. Nothing is drawn that claims something
//! exists which does not; a cell that is off exists and says it is off.
//! See [`DECKS`], [`preview_rects`] and [`View::previews`], and
//! [ADR-0170](../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md)
//! for the alternative that lost and for what would reopen it.
//!
//! # The Staging lane is the one bay whose rows have no producer, so it draws none
//!
//! [ADR-0200](../../../docs/adr/0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md)
//! draws every part of the mock that has a value behind it and omits the
//! rest. Applied to the Staging bay it omits the entire body, and the reason
//! is neither a missing spelling nor a missing control — it is that **nothing
//! in this workspace can put a row there.**
//!
//! **What a row would be.** `console.html` specifies four things: the node
//! (`L4:0`), what the procedure calls itself, whether it is *on screen* —
//! landed, rolled back for costing too much, or refused by the checker — and
//! when it arrived. So the lane is a list of **nodes whose newest version has
//! not been settled**: `karakuri_operation::Operation`'s `KeepCandidate` and
//! `RestoreProcedure` both address a node and not a version, because a node
//! has at most one unsettled version.
//!
//! **That set is empty here structurally, and not merely for now.** The
//! program this panel is drawn by builds both its deck slots with
//! `karakuri_engine::HotSwap::fixed` — no watcher, no MCP, no build `Source`
//! of any kind — and `fixed` keeps a `Receiver` whose `Sender` was dropped at
//! construction. So `install_if_ready` finds a disconnected channel and
//! installs nothing, `trial` is never `Some` and the watchdog returns on its
//! first line, and **not one `swap::Event` of any variant is emitted in any
//! run of this program.** A candidate row would be
//! [ADR-0191](../../../docs/adr/0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md)'s
//! parked chip: a drawing of a state the engine never entered.
//!
//! **And the row's address is not on the wire even once a producer is.** A
//! `swap::Event` carries an `id` and a `label` and no node, because a
//! `karakuri_engine::Request` restates *every* node of the slot — so a
//! verdict is over a build. Which node of that build changed is derivable:
//! `karakuri_environment::watch::Built` carries `(layer, index, hash)` for the
//! whole stack on every build, and consecutive builds differ where the hashes
//! do. Nothing derives it. `karakuri_environment::history::Snapshots::record`
//! computes exactly that discrimination on the worker thread — it returns the
//! path it wrote, or `None` for a source that did not change — and the
//! watcher drops the answer. The per-node `proc` name never crosses the
//! channel at all; only the `label`, which is every node's name joined.
//!
//! **The rest of the mock's lane, each omission with what it waits on.**
//!
//! - **The coloured dot, and the `you` in `you, 14:41`** — who wrote it.
//!   `origin` — the prompt, the model, the seed — is specified in
//!   `docs/ir-spec.md` and produced by nothing, so a hand in an editor and a
//!   model over MCP are the same save down the same path. The dot goes with
//!   it, because the colour *is* the producer.
//! - **`14:41`** — when it arrived. It waits on what the Library bay's `.dim`
//!   column waits on and is refused for its reason: the value would exist and
//!   a *spelling* does not. `karakuri_environment::history`'s is
//!   `%H%M%S-%3f`, which is half a filename;
//!   `karakuri_environment::setfile::written_at` is local to the second; the
//!   mock's `14:41` is a third. **The reason this column waited has changed**:
//!   it was that the one spelling sat in a package with no library target, and
//!   ADR-0214 moved it, so what is left is three spellings and no decision
//!   about which is the one.
//! - **The head's `2 waiting`** — a count. It is not a queue depth: a
//!   finished build is installed at a frame boundary rather than held for a
//!   verdict, so the number would be *unsettled nodes* and waits on the rows.
//!   One thing does wait and it is not this one: `install_if_ready` declines
//!   to install while a candidate is on trial, so a build that finishes
//!   inside a judging window sits in the channel until the verdict is in, and
//!   a build superseded there is retired without ever having been drawn.
//!   Nothing exposes either number, and no row is counted by either.
//! - **The mock's third `.cand`,** *a rejected candidate costs nothing*. The
//!   page says what that is: a note to whoever is reading the mock, and not a
//!   thing the lane draws. Empty, this lane draws *"no row, no placeholder,
//!   and no standing sentence"*.
//!
//! So the bay is its card and its head, which is the shapes the Sequencer bay
//! draws — the lane's twin in this console's furniture, a title with no pill
//! and no grip over nothing; `tests/staging.rs` counts the two against each
//! other. **What the lane is for is still the row the mock does not draw** —
//! after
//! `swap::Event::RolledBack` the watchdog puts the previous *Set* back on
//! screen and does not put the previous *file* back, while the watcher
//! re-reads every file of the slot on every rebuild, so the picture is the
//! old version and the disk is the over-budget one — and that row waits on
//! the same producer every other row here does.
//!
//! **The height is what this leaves wrong, and it is not this module's to
//! fix.** `lib.rs` pins the lane at `fixed(125.0)` with a minimum of `66.0` —
//! the mock's three `.cand` rows, and one — and empty is not only this lane's
//! ordinary state but the only state this program can reach, so those pixels
//! are held open over nothing at the expense of the Library, which is the bay
//! in that column that absorbs. A content-height lane needs `arrangement()`
//! to take an argument, or `karakuri-layout` to grow a setter for a view's
//! size, and it has neither. The numbers stay where they are.
//!
//! # The Program bay's body arranges itself, and that is one derivation
//!
//! The picture and the four cells are **not** where the two regions that hold
//! them are: on a wide, short bay the cells go down the sides of the picture
//! and `deck-previews` is taken out of the layout altogether, which is what
//! leaves the bay's width to the picture. [`program_bay`] is the one place
//! that decides it and [`rearrange`] is what writes the one bit that follows;
//! [`picture_rect`], [`preview_rects`] and [`View::draw`] are readers of that
//! one answer and derive no rectangle of their own.
//!
//! It is why [`Kind::Previews`] draws nothing: the row's region is not where
//! the cells are, and past the crossover it is not even in the plan.
//! [ADR-0182](../../../docs/adr/0182-the-program-bays-body-arranges-itself-for-the-larger-picture.md)
//! is the arrangement and
//! [ADR-0183](../../../docs/adr/0183-a-node-is-out-of-the-layout-for-two-reasons-and-they-are-two-bits.md)
//! is the bit.
//!
//! The transport and the outputs are **rows, not bays**: they carry no
//! heading, because they have none in the mock — both carry `class="bay"`,
//! which is the card styling, and neither carries a `.bay-head`
//! ([ADR-0159](../../../docs/adr/0159-the-consoles-words-are-the-manuals-and-the-middle-one-is-not-a-pane.md)).
//!
//! # The transport row is four readouts and one control
//!
//! [`Kind::Transport`] draws the tempo, the beat grid, the bar and the frame
//! readout — **the four things in the mock's row that are a value somebody
//! measured rather than a control over something that does not exist.** The
//! ones the console cannot know are named in [`transport`], one by one, with
//! what is missing behind each; the paragraph above is the whole of the
//! argument and this row is where it costs the most, because **most of what the
//! mock draws there goes.**
//!
//! **The one that stayed is the arrangement pill, and it stayed for the rule
//! rather than despite it.** Every other control in this row is a control over
//! machinery that is in neither this crate nor the program — a tap nothing
//! times, a learn mode with no map, a recorder that writes no stream. The
//! arrangement is not one of those: it is the tree this crate owns, it is
//! already saved and put back by name over a real file, and `r` already
//! resets it. So `arr · night ▾` is a control over something that exists,
//! which is the only test this module's rule has ever applied — see
//! [`arrangement`], [`Arrangement`] and
//! [ADR-0221](../../../docs/adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md).
//! It is the mock's own `map · nanoKONTROL2 ▾` in shape and in position,
//! because a named file with *save*, *load* and *start a new one* under one
//! control is exactly the family, and the map pill is the same three over a
//! file this program has not got.
//!
//! **No count is written here.** This comment said *six of the mock's ten* and
//! the mock has gained an offset pill and an octave pair since, which is the
//! third time a transcribed figure about that page has gone stale in this
//! repository. What the mock holds is
//! `docs/manual/console.html`'s `.transport` block, and counting it is a
//! command rather than a memory.
//!
//! What it draws with no engine behind it is **nothing at all** — not a row of
//! zeroes and not a row of dashes, either of which is a reading invented for a
//! console that has none. That is [`View::picture`]'s rule, and the values
//! arrive the same way it does: a plain [`Transport`] written per frame by
//! whoever owns the engine, because `src/` takes no device, no window and no
//! clock (ADR-0156) and this row is made of a clock and an engine.
//!
//! # The Outputs row has one control in it, and it is the console's first
//!
//! [`Kind::Outputs`] draws the word OUTPUTS and **one** `.sink` — the picture,
//! which the manual lists as *program view*. Everything else in the mock's row
//! is a control over something that does not exist, and [`outputs`] names each
//! of them and says why it is not drawn. The dot is lit from
//! [`Layout::visible`] on the picture's node and a press on it asks for
//! [`Op::Fold`] or [`Op::Unfold`] by name, so the control and the keyboard
//! reach one operation between them rather than two states that agree until
//! they do not.
//!
//! It is also the first thing on the panel a press has to *reach*, which is
//! the rule [`crate::input`] was written for and the clearance
//! `tests/outputs.rs` holds.
//!
//! # The Mixer bay is the first bay with something in its body
//!
//! [`Kind::Mixer`] draws the card and the head every other bay gets, and then
//! **as many strips as the deck has** — not four. `Deck::slot_count` is what
//! there is, and a page of this bay has [`DECKS`] tracks whatever that number
//! is, so a track with no strip in it draws nothing at all rather than an
//! empty strip. **That is not the deck previews' rule read backwards.** A
//! preview cell is the region's own face and says `off`; a strip is six
//! readings, and an empty one is six readings nobody took — the row of zeroes
//! ADR-0177 refuses, with a different glyph. See [`mixer`] and [`Strip`], and
//! [`Meter`] for why a meter is not a [`Fader`].
//!
//! **Five things in that bay answer a pointer and the rest are readouts**,
//! and the source says which rather than leaving the next reader to discover
//! it. The two fader knobs are played, and the blend mini, the tally chip and
//! the mask mini each cycle
//! ([ADR-0185](../../../docs/adr/0185-a-fader-translates-a-drag-into-an-operation-and-applies-nothing.md),
//! [ADR-0187](../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md),
//! [ADR-0195](../../../docs/adr/0195-the-tally-chip-cycles-from-the-request-so-the-parked-case-needs-no-case.md),
//! [ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)):
//! each emits an [`Operation`] and applies nothing, because the value belongs
//! to the engine rather than to the arrangement. The meter and the number are
//! drawn from what the deck says and a press on either reaches nothing — and
//! so does a press on a fader's *track*, off the knob.
//!
//! **The mask mini is the one control that carries a value it never draws.**
//! The chip says *which shape*, and [`Operation::SetMaskShape`] carries a
//! shape **and an angle**; so [`Strip::mask_angle`] is read to build the
//! operation and painted nowhere, which is what stops choosing a shape from
//! straightening a diagonal front ([`Mixer::mask`]).
//!
//! **The tally is the one control that reads two values.** `Deck::residency`
//! is what a slot is doing and `Deck::requested_residency` is what it was asked
//! to do, and while they disagree the chip's word rolls part of the way toward
//! the request and falls back, once a second, and never lands — see
//! [`Strip::pending`], [`roll_at`] and [`tally_into`], and
//! [P-0075](../../../docs/principles/0075-a-pending-transition-shows-where-it-is-where-it-is-going-and-that-it-has-not-arrived.md)
//! for what that has to say. **A press on it cycles from the residency that
//! was *requested*** ([`Mixer::tally`]), which is what makes a parked chip's
//! press the withdrawal of its own prime request with no case in the code for
//! it. It is the panel's first live region and still its only
//! one ([`View::declares`], which answers with the bay's name, what one update
//! of it costs and how stale it may get) — and it declares only while the bay
//! it rolls in is laid out, because a fold takes the chip off the screen and a
//! price paid for what nobody can see is P-0072 broken rather than served.
//!
//! # The bay head is one component with seven call sites
//!
//! [`bay_head`] is written once and seven bays call it: six through
//! [`Kind::Bay`] in [`REGIONS`] and the mixer through [`Kind::Mixer`], which
//! is a bay whose body is not empty. That is this repository's rule about an
//! abstraction needing two call sites, satisfied on the day it is written
//! rather than promised for later — and so is [`fader`], which is written once
//! and drawn twice in every strip.
//!
//! # `.console`'s own 10px of padding is not drawn
//!
//! The mock's console is a card with `padding: 10px`, so the ground shows in a
//! ring around the outermost bays as well as in every divider. The
//! arrangement's root fills the viewport it is given and
//! [`Panel::set_viewport`] puts that viewport at the origin, so the ring would
//! have to be an offset applied to every rectangle on the way out and undone
//! on every pointer coordinate on the way in — a second coordinate space, for
//! ten pixels of margin. The window's edge is the panel's edge here, and the
//! ground shows in the dividers alone.

use std::time::Duration;

use egui::epaint::text::{LayoutJob, TextFormat};
use egui::{Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, Stroke, StrokeKind, Ui};
use karakuri_layout::{Axis, Hit, NodeId};
use karakuri_operation::{Authority, BlendMode, Operation, Residency, Sync, Tonemap, WipeKind};

use crate::budget::{Declared, PANEL_PASS};
use crate::panel::{unit, Grab, InHand, Knob, Op, Panel, GRAB};
use crate::room::{size, Palette, Room};

/// The whole of a texture, in `egui`'s texture coordinates. The picture fills
/// its rectangle, and that is now one answer rather than two.
///
/// # What changed, and why the argument is not deleted
///
/// This used to read *"the fitting happened when the engine drew into it, and
/// doing it again here would be two answers to how does a 16:9 canvas sit in
/// this box"*. That was true while [`picture_rect`] handed the engine the
/// whole `program-view` region: the region was not the canvas's shape, so
/// `Present::draw` had to choose where the canvas sat inside it, and a second
/// fit here would have chosen differently.
///
/// **The console decides the box now.** [`picture_rect`] gives the picture the
/// canvas's own aspect, so the engine is handed a target it already agrees
/// with and its fit has nothing left to do. The two answers did not become
/// one by one of them being dropped — one of them became the identity.
///
/// # The engine's letterbox stays, and it is not redundant
///
/// [`fitted`] rounds to whole pixels and the caller's own `physical` rounds
/// again to whole texels, so the target is **never exactly the canvas's
/// ratio**: the mock's 466 x 262 is 16:9 to a quarter of a pixel and no
/// closer. `letterbox` is what absorbs that, and after this rule the bars it
/// draws are sub-texel where they used to be the black half of a region.
const WHOLE_TEXTURE: Rect = Rect {
    min: Pos2::new(0.0, 0.0),
    max: Pos2::new(1.0, 1.0),
};

/// **How many deck preview cells there are**, and it is derived rather than
/// picked off the mock.
///
/// `karakuri_engine`'s `deck::MAX_SLOTS` is 4 — *"a deck holds 1 to 4
/// slots"*, asserted in `Deck::new` — so four is the most auditions there can
/// ever be at once, and a fifth cell would be a cell no deck can ever fill.
/// The mock agrees from the other end: `.previews` is
/// `grid-template-columns: repeat(4, 1fr)` with four `.preview` cells in it,
/// and the Program bay's head reads *previews 2 of 4*. Two readings, one
/// number.
///
/// `karakuri-engine` is not a dependency of this crate and is not becoming one
/// for a `usize` — `src/` takes no device, which is the seam ADR-0156 left
/// standing. The number is transcribed with its derivation, the way every
/// number read off the mock is.
pub const DECKS: usize = 4;

/// The letters the mock puts in the four cells, which is how an operator says
/// *which* deck. `A` through `D`, in slot order.
///
/// Public because a harness saying *which deck a fader moved* has to say it in
/// the letters the cells are drawn with, and a second list written out there
/// would be a copy that goes on saying `A B C D` the day this one does not —
/// [ADR-0179](../../../docs/adr/0179-a-transcribed-number-cites-the-rule-it-was-copied-from.md)
/// on a word instead of on a number.
pub const DECK_LETTERS: [&str; DECKS] = ["A", "B", "C", "D"];

/// **How many cells go down one side when they are beside the picture**: half
/// of [`DECKS`], which is two — A and B down the left, C and D down the right.
///
/// **Two columns of two rather than one column of four**, and the arithmetic
/// is worth writing down because it does not point where a reader expects. A
/// column of *n* cells stacked down a body *H* tall is about `H / n` per cell,
/// so it is `(H / n) * 16 / 9` wide, and all four of them cost
/// `(4 / n) * (H / n) * 16 / 9` — **one column of four is the cheapest
/// arrangement and four columns of one is the dearest.** So this is not the
/// choice that leaves the picture the most room; one side of four would leave
/// more.
///
/// It is chosen for the two things that are not width. The picture stays
/// **centred in the bay** with equal ground either side, which is what
/// [`fitted`] gives every other box on this panel; and a cell stays
/// `H / 2` rather than `H / 4` tall, which at the mock's own body is 163
/// against 79 — a preview an operator can read a cut in against one that is a
/// smaller thumbnail than the row it replaced.
const PER_COLUMN: usize = DECKS / 2;

/// Two columns divide [`DECKS`] exactly, and a `DECKS` that stopped being even
/// would put a cell in neither column rather than fail. It is 4 and the
/// engine's `deck::MAX_SLOTS` is why, so this is a compile-time reading of that
/// number and not a runtime check.
const _: () = assert!(DECKS % 2 == 0 && DECKS == PER_COLUMN * 2);

/// What the console draws in a region — the third thing about a region, after
/// its name and its rectangle, and the only one this module owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A titled box. The manual's word, sixteen times over, and the mock's
    /// `.bay-head` is the title bar this draws.
    Bay {
        /// The mock's own capitalisation. `.bay-head` upper-cases in CSS, and
        /// that is done at paint time here rather than in the string, so the
        /// word a reader searches for is the word in the source.
        title: &'static str,
        /// The controls the mock draws in this head, and **only the ones that
        /// are controls**. Every other pill in the mock's bay heads states a
        /// value the console does not have yet — `1920x1080`, `previews 2 of
        /// 4`, `4 of 4 - page 1`, `2 waiting` — and a pill reading `previews 2
        /// of 4` over an empty bay is exactly the scaffolding that looks
        /// finished. They arrive with the bay that knows the number.
        pills: &'static [&'static str],
        /// Whether the mock draws a `.grip` in this head. Four of the eight
        /// bays carry one, and it marks the bay that absorbs its column's
        /// height. **Stated rather than derived**: `program` carries a grip
        /// and is [`karakuri_layout::Sizing::Fixed`], so the two do not agree
        /// and the mock is the reference.
        grip: bool,
    },
    /// **The transport row**: a strip with no heading (ADR-0159), holding the
    /// tempo, the beat grid, the bar and the frame readout.
    ///
    /// A kind of its own for the reason [`Kind::Outputs`] is one, and it was
    /// [`Row`](Kind::Outputs) — the abstract *headless strip* — while its body
    /// was empty. Now that both rows have something in them, a `Row` would
    /// mean *the transport* to [`View::draw`] and nothing in the table would
    /// say so; the alternative is comparing a name on the frame path, which
    /// puts a string where the table already says what a region is.
    ///
    /// See [`transport`] for what is drawn here, for where the values come
    /// from, and for the six things in the mock's row that are **not** drawn.
    Transport,
    /// **The Outputs row**, which is a headless strip like
    /// [`Kind::Transport`] with the console's one control in it.
    ///
    /// A row in every other respect — it carries `class="bay"` for the card
    /// and no `.bay-head`, which is ADR-0159 — and it is a kind of its own for
    /// the reason [`Kind::Picture`] is one: [`View::draw`] has to know *which*
    /// row the sinks go in, and the alternative is comparing a name on the
    /// frame path, which puts a string where the table already says what a
    /// region is.
    ///
    /// See [`outputs`] for what is drawn here, for the state the dot is read
    /// from, and for the four things in the mock's row that are **not** drawn.
    Outputs,
    /// **The Mixer bay**, which is a bay in every other respect: the same card
    /// and the same [`bay_head`], carrying [`MIXER_TITLE`], no pill and no
    /// grip — the mock gives the mixer none of the three.
    ///
    /// A kind of its own for the reason [`Kind::Picture`] is one, and it is
    /// the first *bay* to need it: [`View::draw`] has to know **which** bay
    /// the strips go in, and the alternative is comparing a name on the frame
    /// path, which puts a string where the table already says what a region
    /// is.
    ///
    /// See [`mixer`] for what is drawn here, for where the values come from,
    /// and for the four things in the mock's bay that are **not** drawn.
    Mixer,
    /// **The Library bay**, which is a bay in every other respect: the same
    /// card and the same [`bay_head`], carrying [`LIBRARY_TITLE`], no pill and
    /// the grip the mock draws in this head.
    ///
    /// A kind of its own for [`Kind::Mixer`]'s reason: [`View::draw`] has to
    /// know **which** bay the listing goes in, and the alternative is
    /// comparing a name on the frame path, which puts a string where the table
    /// already says what a region is.
    ///
    /// See [`library`] for what is drawn here, for where the values come from,
    /// and for the six things in the mock's bay that are **not** drawn.
    Library,
    /// One subdivision of a bay, which has no head of its own because the bay
    /// around it has one. The inspector's two panes.
    Pane,
    /// **The one region a texture is drawn into**: the picture in the Program
    /// bay, which is a sink and whose texels somebody else rendered.
    ///
    /// A pane in every other respect — it is inside the Program bay's card and
    /// has no head of its own — and it is a kind of its own for one reason:
    /// [`View::draw`] has to know *which* pane the picture goes in, and the
    /// alternative is comparing a name in the frame path, which puts a string
    /// where the table already says what a region is.
    ///
    /// **It carries no label**, and the manual says why: *"The picture carries
    /// no label of its own. The bay head already says Program, and this is a
    /// region somebody may be capturing: a capture that is neither the canvas
    /// nor a clean crop of it is worse than useless, and a word burnt into the
    /// corner is exactly that."*
    ///
    /// **A clean crop is what the picture now is.** [`picture_rect`] gives it
    /// the canvas's own aspect rather than the whole region, so the rectangle
    /// somebody captures is the canvas's shape and the sentence above is
    /// satisfied rather than merely quoted — before that rule the region was
    /// neither the canvas nor a crop of it, and the manual asked for one of
    /// the two.
    ///
    /// # What is beside it is the bay's card, and the cells may be in it
    ///
    /// The region is wider than the picture at every window above the mock's
    /// narrowest, and the leftover is the console's ground rather than the
    /// engine's black inside the texture. The bay's card shows through exactly
    /// as it does in every other empty body, which is the rule this module
    /// opens with and is also the mock's own answer — `.program-view` *is* the
    /// picture, and what surrounds it is `.program-body`, which sets no
    /// background of its own.
    ///
    /// **Far enough above it, the four deck previews are what is in that
    /// ground** — [`program_bay`], and ADR-0182 for why it is the ground down
    /// each side that they take. What is drawn beside the picture is therefore
    /// either nothing or the cells, and never a placeholder; the paragraph
    /// below is about the case where it is nothing, which is every window a
    /// capture is likely to be taken at.
    ///
    /// **What that costs a capture, said here rather than found later.** The
    /// `solo` pill's tooltip is *"Solo the program view: the panel folds away
    /// and only the picture is left, which is also how you capture this
    /// window"* — so after a solo the window **is** this region, and an
    /// operator whose window is not the canvas's shape captures `--c-panel`
    /// bars where black would read as an ordinary letterbox. That is a real
    /// cost and it is new: before this rule the bars were the engine's clear
    /// inside the texture, and they were black.
    ///
    /// `karakuri-cli`'s `a` — `Live::snap_to_canvas` — is the answer to
    /// exactly this one window along, and its own documentation says why:
    /// *"an OBS window capture of it would otherwise pick up the bars and a
    /// scale."* **The console's window has no `a` yet, and this rule is what
    /// creates that gap.**
    Picture,
    /// **The region the deck previews are in when they are in a region**: the
    /// row under the picture, [`DECKS`] cells side by side.
    ///
    /// A pane in every other respect, and **it draws nothing**, which is the
    /// one thing about it that is worth reading twice. The four cells are the
    /// *bay's* body rather than this region's — beside the picture they are
    /// down the sides and this region is set aside, with no extent and no
    /// entry in the plan — so they are drawn once from [`program_bay`] and
    /// never from here. The kind stays because the arrangement still has the
    /// region and the table still has to say what it is: a row that folds
    /// apart from the picture, which is what the manual promises and what the
    /// operator's `f` still acts on.
    ///
    /// **Each cell carries a label and the picture does not**, and the manual
    /// states both in one breath: *"The picture carries no label of its own …
    /// The A–D under it keep their letters, which are outside anything you
    /// would capture and are the only thing naming a deck."* So the two are
    /// consistent rather than at odds. See [`preview_rects`] and
    /// [`View::previews`].
    Previews,
}

/// One region of the console: the name the arrangement knows it by, and what
/// the panel draws there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    /// The arrangement's name, which is the manual's word for the region and
    /// the name all four surfaces address it by (ADR-0156, ADR-0159).
    pub name: &'static str,
    pub kind: Kind,
}

/// Every region the console draws, and what it draws there.
///
/// **The table is the whole of what is region-specific.** A node the
/// arrangement names and this does not list is structure — `left-pane`,
/// `centre`, `right-pane` are splits an operator folds, not things with a face
/// — and is not drawn. `tests/view.rs` asserts in both directions: every
/// visible leaf of the arrangement is in the plan, and nothing is in the plan
/// that is not a region.
///
/// The order is the arrangement's, top to bottom and left to right, so this
/// reads like the panel.
pub const REGIONS: &[Region] = &[
    Region {
        name: "transport",
        kind: Kind::Transport,
    },
    Region {
        name: "library",
        kind: Kind::Library,
    },
    Region {
        name: "staging",
        // **No pill, and the mock's head has one**: `2 waiting` is a count of
        // rows, and this lane has no row and no producer for one. The module
        // documentation is where that is argued, omission by omission, and
        // `tests/staging.rs` is what holds the bay to a card and a head.
        kind: Kind::Bay {
            title: "Staging",
            pills: &[],
            grip: false,
        },
    },
    Region {
        name: "program",
        kind: Kind::Bay {
            title: "Program",
            // `solo` is an operation the panel already has — `Op::Solo` over
            // whatever the pointer is on — so it is a control and not a
            // readout. It is drawn, and it is the only pill in the mock's
            // heads that is.
            pills: &["solo"],
            grip: true,
        },
    },
    Region {
        name: "program-view",
        kind: Kind::Picture,
    },
    Region {
        name: "deck-previews",
        kind: Kind::Previews,
    },
    Region {
        name: "inspector",
        kind: Kind::Bay {
            title: "Inspector",
            pills: &[],
            grip: true,
        },
    },
    Region {
        name: "inspector-1",
        kind: Kind::Pane,
    },
    Region {
        name: "inspector-2",
        kind: Kind::Pane,
    },
    Region {
        name: "mixer",
        kind: Kind::Mixer,
    },
    Region {
        name: "master",
        kind: Kind::Bay {
            title: "Master",
            pills: &[],
            grip: true,
        },
    },
    Region {
        name: "sequencer",
        kind: Kind::Bay {
            title: "Sequencer",
            pills: &[],
            grip: false,
        },
    },
    Region {
        name: "outputs",
        kind: Kind::Outputs,
    },
];

/// The region a name is, or `None` where the arrangement names something this
/// panel does not draw a face for.
pub fn region(name: &str) -> Option<&'static Region> {
    REGIONS.iter().find(|r| r.name == name)
}

/// One region, where it solved to this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    pub id: NodeId,
    pub region: &'static Region,
    pub rect: karakuri_layout::Rect,
}

/// Every region to draw this frame, in tree order, appended to `out` after
/// clearing it.
///
/// **Tree order is load bearing and not a convenience.** The inspector is a
/// split: its bay card and head cover the same rectangle its two panes tile,
/// so the card has to be painted before them. Tree order gives that for
/// nothing, and any other order would need the rule written out.
///
/// **Arranges the Program bay first**, which is [`rearrange`] and is where the
/// solve happens: `Layout::rect` refuses to answer from a dirty layout, and a
/// plan taken before the bay had arranged itself would list `deck-previews` as
/// a region to draw on the very frame its cells went somewhere else. On a
/// frame where nothing moved both solves are flag tests and the bit is written
/// with the value it already had, which marks nothing dirty (ADR-0183).
///
/// `canvas` is the picture's shape, which is what decides that arrangement —
/// [`program_bay`]. It is [`View::canvas`] at the one call site that draws.
pub fn plan_into(panel: &mut Panel, canvas: (u32, u32), out: &mut Vec<Placed>) {
    rearrange(panel, canvas);
    out.clear();
    let layout = panel.layout();
    for node in panel.nodes() {
        if !layout.visible(node.id) {
            continue;
        }
        let Some(region) = layout.name(node.id).and_then(region) else {
            continue;
        };
        out.push(Placed {
            id: node.id,
            region,
            rect: layout.rect(node.id),
        });
    }
}

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
/// this is its picture. See [ADR-0182](../../../docs/adr/0182-the-program-bays-body-arranges-itself-for-the-larger-picture.md).
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

/// **Whether there is anything of this rectangle to draw**, which is the one
/// rule [`picture_rect`], [`preview_cells`] and [`program_body`] each answer
/// `None` from.
///
/// It is asked of the *fitted* rectangle rather than of the box it was fitted
/// into, always: a box under half a pixel rounds to nothing, which is nothing
/// to draw and nothing to render into, and a box the inset turned inside out
/// gives a negative extent `egui` draws back-to-front rather than refuses. The
/// three call sites had a copy of this comparison each before they had a
/// function; the sentence is the same one in all three, and now so is the
/// answer. [`kept`] is the same rule as an `Option`, for the callers that hand
/// the rectangle straight back.
fn positive(rect: Rect) -> bool {
    rect.width() > 0.0 && rect.height() > 0.0
}

/// **The largest rectangle of `aspect` that fits inside `inside`, centred in
/// it, and a whole number of pixels in each direction.**
///
/// # One derivation with two call sites, and they were one rule before they
/// were one function
///
/// [`picture_rect`] fits the canvas into what the bay's body leaves it and
/// [`preview_cells`] fits a cell into its track, and
/// [ADR-0170](../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md)
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
/// [ADR-0182](../../../docs/adr/0182-the-program-bays-body-arranges-itself-for-the-larger-picture.md)'s
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
    std::array::from_fn(|deck| {
        fitted(
            track(row, DECKS, deck, size::PREVIEW_GAP, Axis::Row),
            PREVIEW_ASPECT,
        )
    })
}

/// **One of `count` equal tracks laid along `axis` inside `strip`**, with `gap`
/// between them and nowhere else.
///
/// That last clause is the whole of it, and it is the reading the mock's grids
/// and flex rows all take: `repeat(4, 1fr)` with a `gap` is four tracks and
/// **three** gaps, not four tracks each carrying one. The same sentence is
/// written on [`size::PREVIEW_GAP`], on [`size::BEAT_GAP`] and on
/// [`size::STRIP_GAP`], and this is the arithmetic all three describe —
/// [`TransportRow::dot`] is the fourth, and it steps a fixed dot width rather
/// than dividing a strip, so it states the rule and does not call this.
///
/// **Three call sites, and the third is what made it worth a function.** The
/// row of previews and the mixer's page of strips were the same six lines
/// written twice with a different gap in them; a column beside the picture is
/// the third, and it is those six lines read one axis along. A track spans
/// `strip` across the axis, exactly as a node of the arrangement spans its
/// parent across its own — which is why the axis is
/// [`karakuri_layout::Axis`] rather than a `bool`.
fn track(strip: Rect, count: usize, index: usize, gap: f32, axis: Axis) -> Rect {
    let gaps = gap * (count.max(1) - 1) as f32;
    let (along, across) = match axis {
        Axis::Row => (strip.width(), strip.height()),
        Axis::Column => (strip.height(), strip.width()),
    };
    let size = (along - gaps) / count.max(1) as f32;
    let at = (size + gap) * index as f32;
    match axis {
        Axis::Row => Rect::from_min_size(
            Pos2::new(strip.min.x + at, strip.min.y),
            egui::vec2(size, across),
        ),
        Axis::Column => Rect::from_min_size(
            Pos2::new(strip.min.x, strip.min.y + at),
            egui::vec2(across, size),
        ),
    }
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
/// [ADR-0181](../../../docs/adr/0181-the-picture-is-the-canvass-shape-and-the-leftover-is-the-consoles.md)
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
/// [P-0071](../../../docs/principles/0071-solving-a-layout-never-mutates-it.md)
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
/// The mock's head reads *previews 2 of 4* and
/// [ADR-0170](../../../docs/adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md)
/// is the answer: a cell is drawn whether or not a deck is behind it, because
/// *an empty cell is what off looks like, not a stand-in for a full one*. So
/// the question *does the left column fill first, or do they alternate* has a
/// third answer, and it is the one that keeps the letters meaning something:
/// **a cell's place is its deck's, not its turn's.** `C · off` sits at the top
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
/// centred in their tracks by [`fitted`] rather than pushed to an edge.
pub fn program_body(body: Rect, canvas: (u32, u32)) -> Option<Body> {
    // Both, every time, and then one comparison. Two fits and six divisions is
    // not a cost worth a cached answer — and a cached answer is the state the
    // paragraph above refuses.
    match (below(body, canvas), beside(body, canvas)) {
        (Some(below), Some(beside)) if area(beside.picture) > area(below.picture) => Some(beside),
        // The tie, and every case where beside cannot be drawn.
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
/// The row is [`size::PREVIEW_ROW_H`] tall — the 63 the arrangement pins
/// `deck-previews` at, less the padding under it — and it keeps that height at
/// every width, exactly as the arrangement does. The picture takes what is left
/// above it, less one `PROGRAM_DIVIDER`.
fn below(body: Rect, canvas: (u32, u32)) -> Option<Body> {
    let row = Rect::from_min_max(
        Pos2::new(body.min.x, body.max.y - size::PREVIEW_ROW_H),
        body.max,
    );
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
/// The column's width is the whole of the rule — see [`program_body`], where it
/// is argued and checked — and everything else here is [`track`] and
/// [`fitted`], the same two calls the row goes through.
fn beside(body: Rect, canvas: (u32, u32)) -> Option<Body> {
    let (aw, ah) = (
        PREVIEW_ASPECT.0.max(1) as f32,
        PREVIEW_ASPECT.1.max(1) as f32,
    );
    // **The body's height, and never its width.** Two cells stacked with one
    // gap between them, and the column is one of them lying at its aspect.
    let cell = (body.height() - size::PREVIEW_GAP * (PER_COLUMN - 1) as f32) / PER_COLUMN as f32;
    let column = cell * aw / ah;
    let picture = fitted(
        Rect::from_min_max(
            Pos2::new(body.min.x + column + crate::PROGRAM_DIVIDER, body.min.y),
            Pos2::new(body.max.x - column - crate::PROGRAM_DIVIDER, body.max.y),
        ),
        canvas,
    );
    let cells = std::array::from_fn(|deck| {
        // A and B down the left, C and D down the right: the deck's index
        // decides which side and which of the two tracks, and nothing else
        // does.
        let side = match deck < PER_COLUMN {
            true => Rect::from_min_max(body.min, Pos2::new(body.min.x + column, body.max.y)),
            false => Rect::from_min_max(Pos2::new(body.max.x - column, body.min.y), body.max),
        };
        fitted(
            track(
                side,
                PER_COLUMN,
                deck % PER_COLUMN,
                size::PREVIEW_GAP,
                Axis::Column,
            ),
            PREVIEW_ASPECT,
        )
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
/// ([ADR-0174](../../../docs/adr/0174-a-node-claims-only-what-its-visible-content-can-use.md)),
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
/// [ADR-0183](../../../docs/adr/0183-a-node-is-out-of-the-layout-for-two-reasons-and-they-are-two-bits.md)
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
    match (!layout.is_collapsed(picture), !layout.is_collapsed(row)) {
        // Both halves on screen, and this is the arrangement ADR-0182 decides.
        (true, true) => program_body(body, canvas).map(|arranged| ProgramBay {
            placement: arranged.placement,
            picture: Some(arranged.picture),
            cells: Some(arranged.cells),
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
///   allocation — and no repaint is asked for, which is P-0072's first clause.
/// - **The frame it does change solves to the new arrangement and not to the
///   previous one**, because the second solve is after the write. That frame
///   costs **two solves**, and it is worth saying plainly rather than hiding:
///   a placement only changes when the bay's rectangle does, which is a window
///   resize or a drag on a boundary, and P-0072 does not budget what the
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

/// **What the transport row reads this frame**: the session's tempo and
/// position, and what the frame before this one cost.
///
/// # The same seam as [`View::picture`], and it is not crossed
///
/// Every field is a number and none of them is a `Deck`, an `Instant` or a
/// `Duration` that means *now*. This crate takes no device, no window and no
/// clock (ADR-0156), and this row is made of a clock and an engine — so
/// whoever owns those reads them and writes this per frame, exactly as
/// whoever owns the device registers a texture and writes [`View::picture`].
/// A `Deck` here would put the engine in this crate's dependencies; an
/// `Instant` here would put a clock in it, and then the row would be reading
/// wall time in a repository whose first principle is that nothing does
/// ([P-0002](../../../docs/principles/0002-simulation-time-comes-from-a-record-never-from-a-clock.md)).
///
/// **It carries what cannot be derived and nothing that can.** The beat within
/// the bar and the bar number are arithmetic on [`Transport::beats`] and are
/// [`Transport::beat`] and [`Transport::bar`] rather than two more fields:
/// three numbers for one position is three chances for the lit dot and the bar
/// beside it to come from different arithmetic and disagree.
///
/// **No repaint arm.** [`crate::repaint::Change`] is one list of everything
/// that can change what the console shows, and this is not on it: the values
/// move when the engine draws a frame, and a caller that is drawing engine
/// frames is already asking for frames for the picture beside this row. A
/// panel with nothing live on it is a panel where the oscillator is not
/// advancing either, so the row is right to be still — see
/// [`Transport::fps`], which is the one field that would go stale there and
/// is the one the program leaves `None`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transport {
    /// The tempo, drawn in `.bpm`'s treatment. `karakuri_signal`'s
    /// `Oscillator::bpm` is an `f32` and so is this.
    ///
    /// **Not derivable from [`Transport::beats`]**, which is why it is its
    /// own field: the oscillator's position is an accumulator precisely so
    /// that a tempo correction changes the rate from now on without moving a
    /// beat that has already happened, so the tempo is not the slope of the
    /// position and cannot be read off it.
    pub bpm: f32,
    /// **Musical position, unbounded and monotone** — `Oscillator::beats`,
    /// which counts from the start of the session and never wraps.
    ///
    /// `f64` because that is the type the oscillator accumulates it in, and
    /// narrowing a number this crate does not own is a rounding taken for
    /// nothing. What it would eventually cost is real but distant — an `f32`
    /// stops separating consecutive beats somewhere past eight million of
    /// them — so the reason to keep it is the first one and not the second.
    ///
    /// Which dot is lit and which bar it is are both read off this one number
    /// — see [`Transport::beat`] and [`Transport::bar`].
    pub beats: f64,
    /// **How many beats there are in a bar**, which is how many dots the grid
    /// has.
    ///
    /// **A field rather than a constant, and that is a conclusion rather than
    /// caution.** The mock draws four and `karakuri_signal`'s
    /// `oscillator::BEATS_PER_BAR` is 4, so the number is not in doubt
    /// today — but that constant says of
    /// itself that *"v0.2 of the IR spec has no time-signature concept
    /// anywhere … a fixed assumption of common time rather than something
    /// derived from the record stream. Treat it as provisional until a real
    /// time signature shows up in the format."* A constant here would be this
    /// crate transcribing a number the crate that owns it has written down as
    /// provisional, and the day a time signature lands the console would draw
    /// four dots against a grid of three with nothing saying so. So it is
    /// asked of whoever owns the grid, once a frame, and the grid is that many
    /// dots wide.
    ///
    /// Read through [`Transport::dots`], which is where a zero is dealt with.
    pub beats_per_bar: u32,
    /// **Frames a second**, or `None` where nobody can say yet.
    ///
    /// A rate is measured over a stretch, so a window that has not been left
    /// alone for one has no rate — and `None` draws no `fps` at all rather
    /// than a `0` or a stale number. **Not derived from
    /// [`Transport::frame_ms`]**: `1000 / frame_ms` is the rate the loop
    /// *could* manage, and what it *does* draw is decided by the display and
    /// by whether anything asked for a frame. On a `Fifo` surface those two
    /// differ by the whole vsync wait, which is most of the frame.
    pub fps: Option<f32>,
    /// **What one frame cost on the CPU**, in milliseconds — the `12.4` in the
    /// mock's `12.4/16.6 ms`.
    ///
    /// It is a fact about a frame that has already been drawn, so it does not
    /// go stale on a still window the way a rate does: the last frame did cost
    /// this.
    pub frame_ms: f32,
    /// **What a frame has to fit in**, in milliseconds — the `16.6`, and
    /// `None` where nothing can say what it is.
    ///
    /// Then the row draws the frame time with no budget beside it, which is
    /// the rule the whole of this value follows: a reading nobody has is not
    /// drawn as a plausible one.
    pub budget_ms: Option<f32>,
}

impl Transport {
    /// **How many dots the beat grid has**: [`Transport::beats_per_bar`], and
    /// at least one.
    ///
    /// The clamp is here and in one place because a bar of no beats is two
    /// failures at once — a grid with nothing in it, and a division by zero in
    /// [`Transport::beat`] that comes out `NaN` and lights no dot in a row
    /// that has none. One beat to the bar is the nearest thing to a grid that
    /// can be drawn, and every reader of the field goes through here.
    pub fn dots(&self) -> u32 {
        self.beats_per_bar.max(1)
    }

    /// **Where the beat is inside the current bar**, from zero, and
    /// continuous: `0.0` is on the first dot, `1.5` is halfway between the
    /// second and the third, and `3.75` is three quarters of the way from the
    /// last dot back round to the first.
    ///
    /// **A position and not an index**, which is the whole of what the grid
    /// draws now
    /// ([ADR-0212](../../../docs/adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md)):
    /// the light is somewhere on the grid at every instant rather than on one
    /// dot at a time, so what the row reads is the fractional beat the
    /// oscillator already accumulates rather than a truncation of it. The
    /// fraction was always in [`Transport::beats`] and was thrown away here.
    ///
    /// **It is the whole of the beat's motion, and it is not a clock.** No
    /// second value is written for the grid and none is derived from
    /// [`Phase`]: the beat moves because the session moves, which is what a
    /// beat grid is for, and a wall-clock sweep would go on sweeping at its
    /// own rate over a session running at another one — see [`beat_at`].
    ///
    /// `rem_euclid` rather than `%` because [`Transport::beats`] can be
    /// negative — `Oscillator::behind` reads the same grid at an earlier time,
    /// and a slot warming behind the session is exactly what that is for — and
    /// `%` on a negative is negative, which is a position no grid has.
    ///
    /// **The clamp this used to need went with the index.** `(-1e-18_f64)
    /// .rem_euclid(4.0)` is `4.0` exactly: the true remainder is a hair under
    /// the divisor and rounds up to it. As a dot *index* that was one past the
    /// end of the grid — a rectangle drawn beside it, or a panic — and it was
    /// clamped. As a *position* it is the downbeat: `4.0` and `0.0` are the
    /// same point on a cycle of four, [`beat_at`] measures round the cycle,
    /// and the value arrives where it belongs with nothing written for it.
    pub fn position(&self) -> f32 {
        self.beats.rem_euclid(self.dots() as f64) as f32
    }

    /// **Which bar it is, counting from one** — the `37` in the mock's
    /// `bar 37`.
    ///
    /// One-based because that is how a bar is counted out loud, and there is
    /// no bar 0 in anything an operator says. Signed for the reason
    /// [`Transport::beat`] takes `rem_euclid`: a position before the session's
    /// zero is a bar before the first one, and `as u32` on it would saturate
    /// to zero and draw `bar 1` for every one of them.
    pub fn bar(&self) -> i64 {
        self.beats.div_euclid(self.dots() as f64) as i64 + 1
    }
}

/// **The transport row, laid out**: where each of the four readouts goes, and
/// which beat is lit.
///
/// # One derivation, for the reason [`Outputs`] is one
///
/// [`View::draw`] paints exactly these rectangles. Nothing hit-tests *these*
/// four, because none of them is a control — see below — but this now has
/// three call sites rather than the one it was written with: the frame paints
/// it, and [`arrangement`] asks it where the bar ended and where the frame
/// readout begins, from both the frame and [`crate::input::claim`]. A value
/// rather than a paint-as-you-go pass is what made that possible, and it was
/// already the right shape for the reason it was written: it makes the
/// arithmetic something `tests/transport.rs` can ask about without a device,
/// which is the whole of how this crate is checked.
///
/// # None of these four is a control, and that is the answer rather than an
/// omission
///
/// A point in this row that is not inside a boundary's [`GRAB`] and not on the
/// arrangement pill is `egui`'s. Every one of the mock's *other* controls in
/// this row is in the list below of things that are not drawn, so what is left
/// beside the pill — a tempo, a beat, a bar and a frame time — is four
/// readouts, and a readout is not something a press acts on.
/// `tests/transport.rs` asserts it over the row rather than leaving it to be
/// inferred from the absence of a hit test, and it asks with the pill drawn so
/// that the assertion cannot pass on the control having gone.
///
/// **This paragraph said *nothing here is a control* and the change is
/// deliberate.** [`arrangement`] is laid out from [`TransportRow::bar`] and
/// held clear of [`TransportRow::frame`], so the pill's place is this
/// derivation's answer rather than a second one; and it is a separate function
/// because the *row* is four readouts and a tempo, where the pill is neither.
///
/// # What is in the mock's row and is deliberately not here
///
/// The mock draws ten things and **four of them exist**. Each of the other six
/// is a control over machinery that is in neither this crate nor the program,
/// and drawing one is the scaffolding this module's documentation refuses — a
/// pill that looks like a control and does nothing does not get replaced:
///
/// - `audio-in`, drawn `.armed`, is the session listening to the room. What
///   would be behind it is a measured `AudioFrame` on the session's signals,
///   and nothing here opens an input or installs one — a `Deck` with nobody
///   calling `set_audio` runs on a synthesized bus, and an armed pill would
///   say the room is being listened to while it is not.
/// - `tap` is tap tempo: a performer's taps timed and turned into a tempo
///   correction. Nothing times a tap here, and the pill would correct nothing.
/// - `learn` is MIDI learn, and its own tooltip says what it would do —
///   *"Click, then point at any control and move a knob on your surface, and
///   the two are mapped."* There is no map, no learn mode, and exactly one
///   control on the panel to point at.
/// - `map · nanoKONTROL2 ▾` is the map in use, *and it is a file*. None is
///   loaded, saved or recalled anywhere here, and a menu naming a device
///   nobody has plugged in is worse than no menu. **It is the one item in this
///   list that has a twin that is drawn**: [`arrangement`] is this pill's
///   shape over a file that does exist, which is the manual's own argument for
///   putting the arrangement family in this row.
/// - `landed` is what the last write did — *landed, rolled back for cost, or
///   failed to build*. That is a build watcher's verdict over a hot swap, and
///   nothing writes a procedure while this panel runs, so the pill would be
///   reporting on a write that never happens.
/// - `● rec` is recording the session to the store as it happens. There is no
///   session recorder behind this panel and no record stream is written from
///   it.
///
/// The `.sep` between the bar and the frame readout **is** drawn, in the only
/// way a `flex: 1` spacer can be: it is space, so what it does is push the
/// frame readout to the right edge, and that is where [`transport_row`] puts
/// it.
///
/// # No tooltips, for the reason the Outputs row has none
///
/// Most of the mock's items carry a `data-tip` and so do most of the drawn
/// ones — the beat grid's is what the travelling light means, and it landed
/// with the light
/// ([ADR-0212](../../../docs/adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md)). A tooltip needs `egui` to own a widget, this console paints, and
/// giving one readout a widget is a decision about who owns the pointer — see
/// [`outputs`], where the same sentence is written about a control.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one. `ctx` is asked for the type, because where each readout ends is where
/// the next one starts.
pub fn transport(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
) -> Option<TransportRow> {
    // **No engine behind the console, so there is no row.** Every test in this
    // crate is here, and so is the whole of `cargo test -p karakuri-console`.
    // Drawing a row of zeroes would be inventing a tempo nothing is running
    // at; this is `View::picture`'s rule, one row along.
    let values = values?;
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // `outputs`. Nothing routes a pointer here, so this is only the frame
    // before the first one — and there is nothing to draw on it either.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let row = to_egui(layout.rect(layout.find("transport")?));
    let width = |job: LayoutJob| ctx.fonts_mut(|f| f.layout_job(job).size().x);
    let bpm = width(bpm_job(&values, Color32::PLACEHOLDER, Color32::PLACEHOLDER));
    let label = width(span(BPM_LABEL, Color32::PLACEHOLDER));
    let bar = width(span(&bar_text(&values), Color32::PLACEHOLDER));
    let frame = width(frame_job(
        &values,
        Color32::PLACEHOLDER,
        Color32::PLACEHOLDER,
    ));
    transport_row(row, &values, bpm, label, bar, frame)
}

/// **The faint word beside the number**, and the mock's own capitalisation
/// this time: `.transport`'s `BPM` is upper-case in the markup rather than in
/// CSS, so it is upper-case here.
const BPM_LABEL: &str = "BPM";

/// The transport row's furniture: a rectangle for each readout, and which beat
/// is lit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransportRow {
    /// The tempo, in `.bpm`'s 20px box.
    pub bpm: Rect,
    /// The faint `BPM` beside it.
    pub label: Rect,
    /// The whole `.beat-grid`: [`TransportRow::dots`] dots and the gaps
    /// between them. One dot is [`TransportRow::dot`].
    pub grid: Rect,
    /// How many dots are in that grid — [`Transport::dots`], carried so the
    /// painter and a test read the same number the width was built from.
    pub dots: u32,
    /// **Where the light is**, in beats from the first dot's centre —
    /// [`Transport::position`], carried for the reason [`TransportRow::dots`]
    /// is carried: the painter and a test read the one number the row was
    /// measured from. How much of it lands on any given dot is
    /// [`TransportRow::lit`].
    pub at: f32,
    /// `bar 37`.
    pub bar: Rect,
    /// The frame readout, pushed to the right edge by the `.sep`.
    pub frame: Rect,
    /// **The values these rectangles were measured from.**
    ///
    /// Carried rather than passed to the painter beside this, for
    /// [`Picture`]'s own reason: whoever measured the type and whoever paints
    /// it are then one statement, so a row laid out for one tempo and painted
    /// with another cannot be written by accident. It is also what leaves
    /// **one** place where *no engine means nothing at all* is decided — this
    /// answering `None` — rather than that rule being asked once here and
    /// again in [`View::draw`], which is two answers that agree until they do
    /// not.
    pub values: Transport,
}

impl TransportRow {
    /// One dot of the beat grid, from the left. **The gaps are between the
    /// dots and nowhere else**, which is what a flex row with a `gap` is —
    /// the same reading [`preview_cells`] takes of a grid.
    ///
    /// Panics on a dot this grid has not got, which is a caller having
    /// invented a beat: [`TransportRow::on`] is [`Transport::beat`] and that
    /// is inside the grid by construction.
    pub fn dot(&self, index: u32) -> Rect {
        assert!(index < self.dots, "dot {index} of a grid of {}", self.dots);
        Rect::from_min_size(
            Pos2::new(self.grid.min.x + BEAT_PITCH * index as f32, self.grid.min.y),
            egui::vec2(size::BEAT_W, size::BEAT_H),
        )
    }

    /// **How much of the light is on the dot at `index` this frame**, from
    /// `0.0` to `1.0` — [`beat_at`] at this row's position, so the painter and
    /// a test ask one question and get one answer.
    ///
    /// Panics on a dot this grid has not got, for [`TransportRow::dot`]'s
    /// reason.
    pub fn lit(&self, index: u32) -> f32 {
        assert!(index < self.dots, "dot {index} of a grid of {}", self.dots);
        beat_at(self.at, index, self.dots)
    }
}

/// **From one dot to the next**: [`size::BEAT_W`] and the [`size::BEAT_GAP`]
/// after it, which is what one beat of travel measures on this grid.
///
/// Derived rather than transcribed: the mock declares an item width and a flex
/// `gap`, and the pitch of a flex row is the one plus the other. It is the
/// unit [`beat_at`] measures in and the unit [`TransportRow::dot`] steps by,
/// which is why it is a constant rather than the same sum written twice.
pub const BEAT_PITCH: f32 = size::BEAT_W + size::BEAT_GAP;

/// **The tempo this row is drawn at in the mock** — `.bpm`'s `128.0` — in
/// thousandths of a beat a minute, so [`BEAT_STALENESS`]'s arithmetic stays in
/// integers.
const MOCK_BPM_MILLI: u64 = 128_000;

/// **One beat at that tempo**, in microseconds: 468 750, which is 468.75 ms.
const BEAT_MICROS: u64 = 60 * 1_000_000 * 1_000 / MOCK_BPM_MILLI;

/// **How many steps one beat of travel is drawn in**: the pixels in one
/// [`BEAT_PITCH`], so the light moves by at most one of them between updates.
///
/// [`ROLL_STEPS`]'s shape and a different answer, because the two motions are
/// different sizes: the roll travels a few pixels of a word's pitch and twelve
/// steps is smooth over it, where the light crosses a whole dot and a gap
/// every beat.
const BEAT_STEPS: u64 = BEAT_PITCH as u64;

/// **How stale the beat grid may get**, which is what the transport row
/// declares under
/// [P-0072](../../../docs/principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md)
/// and what the harness turns into a deadline.
///
/// [`BEAT_MICROS`] in [`BEAT_STEPS`] steps — **24.67 ms, about forty a
/// second**. One beat of travel is one [`BEAT_PITCH`], so this is the light
/// moving by one pixel and no more, which is the coarsest step that reads as a
/// movement rather than as a sequence of positions
/// ([P-0077](../../../docs/principles/0077-continuous-motion-is-how-a-stopped-panel-announces-itself.md)).
///
/// **Stated at the mock's tempo, and it is the one number here that the music
/// moves.** A beat is 468.75 ms at 128.0 BPM and 375 ms at 160, so the same
/// declaration is a pixel and a quarter a step up there. The alternative —
/// derive it per frame from [`Transport::bpm`], which the row is handed — is
/// [ADR-0212](../../../docs/adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md),
/// and it lost on what it does to the arithmetic rather than on the drawing: a
/// staleness that falls with the tempo makes `Σ (cost / staleness)` a function
/// of how fast the music is, so the two schedulability conditions could only
/// be asserted against a fastest tempo nobody has written down — and inventing
/// one inside the test that noticed it was missing is exactly what
/// [ADR-0210](../../../docs/adr/0210-a-declared-cost-is-one-panel-pass-written-down-and-held-against-the-run.md)
/// refused for the panel's share of the budget.
///
/// **Finer than [`ROLL_STALENESS`]'s 33.33 ms and coarser than a frame**, and
/// both are the presentation rather than a preference for frames: the roll
/// travels a few pixels and this crosses the grid, so it wants more steps; and
/// a 60 Hz frame is 16.6 ms, so a beat grid that asked for every frame would
/// be asking for more than its own drawing can use.
pub const BEAT_STALENESS: Duration = Duration::from_micros(BEAT_MICROS / BEAT_STEPS);

/// **How much of the light is on the dot at `index`**, with the light `at`
/// beats into a bar of `dots`: `1.0` under its centre, `0.0` a whole
/// [`BEAT_PITCH`] away, and a raised cosine between the two.
///
/// # The same curve as [`roll_at`], and two of the reasons are the same
///
/// It leaves and arrives at zero **with zero velocity**, so a dot does not
/// snap into being dark as the light leaves it. And it is a pure function of a
/// value the harness handed in, so a test asserts it at a position it chose
/// and nothing samples a clock to find out what the panel is doing.
///
/// # Two dots at once, and the row's total light is constant
///
/// The falloff is exactly one pitch wide, so at most two dots are lit and
/// `f(d) + f(1 - d) = 1` for every `d` — the raised cosine's own identity, and
/// therefore the grid's dots always sum to exactly one dot's worth of light
/// (every grid but the degenerate one below, which is one dot and holds all of
/// it). The
/// light **moves along the grid** rather than the grid brightening and dimming
/// as it goes, which is what makes a stop visible: a still grid at half
/// brightness would be indistinguishable from a light sat between two dots.
///
/// # Measured round the cycle, not along the row
///
/// The bar wraps, so the distance from the last dot to the first is one pitch
/// and not three: the light leaves the right-hand end of the grid and arrives
/// at the left-hand end in the same instant, each dot half lit, and there is
/// no frame on which it jumps. That is also why [`Transport::position`] needs
/// no clamp — a position of exactly `dots` is a distance of zero from the
/// first dot.
///
/// # What it draws at the instant of a beat is the mock
///
/// At a whole `at` the dot under the light is `1.0` and every other is exactly
/// `0.0`: one dot in `--c-pink` with its halo and the rest in `--c-line`,
/// which is `.beat-grid i.on` and the four dots the mock's markup draws. **The
/// mock is a frame of this** rather than a picture this contradicts, and
/// `docs/manual/style.css` carries the travel between those frames.
pub fn beat_at(at: f32, index: u32, dots: u32) -> f32 {
    let dots = dots.max(1) as f32;
    // **A grid of one dot has nowhere for the light to go**, so it is on that
    // dot at every position. Measuring round a cycle one pitch long would say
    // *half lit* halfway through the beat and leave the row dimming with
    // nothing to dim toward — the identity above wants two dots to share the
    // light between. This is [`Transport::dots`]' own clamp seen from here: a
    // bar of no beats is drawn as the nearest thing to a grid there is.
    if dots < 2.0 {
        return 1.0;
    }
    let round = (index as f32 - at).rem_euclid(dots);
    let away = round.min(dots - round);
    match away < 1.0 {
        true => 0.5 * (1.0 + (away * std::f32::consts::PI).cos()),
        false => 0.0,
    }
}

/// The arithmetic of the row, away from the type it measures and the layout it
/// reads.
///
/// Term for term from `.transport` and what is in it, in `style.css`:
///
/// - `.transport { display: flex; align-items: center; gap: 14px;
///   padding: 9px 12px }` — the readouts laid left to right from
///   [`size::TRANSPORT_PAD_X`], one [`size::TRANSPORT_GAP`] between each pair,
///   and every one of them **centred in the row** rather than sat on its
///   padding. The four boxes are four different heights — 30 for the number,
///   16.5 for a line of type, 6 for the beat grid — and `align-items: center`
///   is what puts them on one line through the middle.
/// - `.sep { flex: 1 }` — a spacer that takes everything left over, so what
///   comes after it is against the row's right padding. The frame readout is
///   laid out from the right edge backwards for that reason, and it is the one
///   thing in the row whose position is not the position of the thing before
///   it.
///
/// # The 48 comes back here
///
/// The row is 48 and the number in it is 30 (`.bpm`'s 20px at
/// `line-height: 1.5`), so there is (48 - 30) / 2 = **9** of row above and
/// below it — which is `.transport`'s own `padding: 9px`, arrived at from the
/// other end. The arrangement's `9 + 30 + 9` and the centring agree exactly
/// here, where the Outputs row's 8 + 18.5 + 8 had to give up a quarter pixel.
///
/// `None` where the row cannot hold what goes in it: folded away, soloed away,
/// or too narrow to keep the frame readout clear of the bar. **The mock wraps
/// and this does not**, which is the argument [`outputs_row`] makes about the
/// second sink, in the one case where the mock's `flex-wrap: wrap` has
/// something to wrap: a row too narrow draws nothing rather than a second line
/// of transport in a bay 48 tall that has no room for one.
fn transport_row(
    row: Rect,
    t: &Transport,
    bpm_w: f32,
    label_w: f32,
    bar_w: f32,
    frame_w: f32,
) -> Option<TransportRow> {
    let mid = row.center().y;
    // An inline span at the console's own type: `font-size: 11px` at
    // `line-height: 1.5`, which is the box every line of ordinary text in the
    // mock sits in.
    let span_h = size::BASE * size::LINE;
    let bpm = Rect::from_min_size(
        Pos2::new(row.min.x + size::TRANSPORT_PAD_X, mid - size::BPM_H * 0.5),
        egui::vec2(bpm_w, size::BPM_H),
    );
    let label = Rect::from_min_size(
        Pos2::new(bpm.max.x + size::TRANSPORT_GAP, mid - span_h * 0.5),
        egui::vec2(label_w, span_h),
    );
    let dots = t.dots();
    let grid = Rect::from_min_size(
        Pos2::new(label.max.x + size::TRANSPORT_GAP, mid - size::BEAT_H * 0.5),
        egui::vec2(
            size::BEAT_W * dots as f32 + size::BEAT_GAP * (dots - 1) as f32,
            size::BEAT_H,
        ),
    );
    let bar = Rect::from_min_size(
        Pos2::new(grid.max.x + size::TRANSPORT_GAP, mid - span_h * 0.5),
        egui::vec2(bar_w, span_h),
    );
    // The `.sep`: everything after it is against the right padding.
    let frame = Rect::from_min_size(
        Pos2::new(
            row.max.x - size::TRANSPORT_PAD_X - frame_w,
            mid - span_h * 0.5,
        ),
        egui::vec2(frame_w, span_h),
    );
    // The same rule `picture_rect` states, on the two ends of the row: the
    // number is the tallest thing in it and the frame readout is the furthest
    // right, so a row that holds both holds everything between them — and the
    // last clause is the wrap the mock does and this does not.
    match row.contains_rect(bpm)
        && row.contains_rect(frame)
        && frame.min.x >= bar.max.x + size::TRANSPORT_GAP
    {
        true => Some(TransportRow {
            bpm,
            label,
            grid,
            dots,
            at: t.position(),
            bar,
            frame,
            values: *t,
        }),
        false => None,
    }
}

/// **The tempo as one laid-out run**, so that measuring it and painting it
/// cannot be two different runs of type.
///
/// `.bpm`'s `128.0` is one decimal place, which is also as fine as a tempo is
/// ever named — and the mock's own number, so a transcription that started
/// printing `128` would be visible against it.
///
/// # The gradient is drawn per glyph, which is what the CSS comes to here
///
/// `.bpm` is `background: linear-gradient(94deg, var(--c-mint), var(--c-lav))`
/// with `background-clip: text`: the two colours run left to right across the
/// number, near enough — 94deg is four degrees off horizontal. `epaint` fills
/// a galley with one colour, so the run is split into one format run per glyph
/// and each takes its own point along the ramp. Five glyphs is a coarse ramp
/// and it is the mock's two colours rather than one of them; the alternative
/// is picking an end and losing the other, which is a transcription that drops
/// half of what it read.
fn bpm_job(t: &Transport, from: Color32, to: Color32) -> LayoutJob {
    let text = bpm_text(t);
    let mut job = LayoutJob::default();
    let last = text.chars().count().saturating_sub(1).max(1) as f32;
    for (n, (at, ch)) in text.char_indices().enumerate() {
        job.append(
            &text[at..at + ch.len_utf8()],
            0.0,
            TextFormat {
                font_id: FontId::new(size::BPM_SIZE, FontFamily::Proportional),
                extra_letter_spacing: size::BPM_TRACKING,
                color: mix(from, to, n as f32 / last),
                ..Default::default()
            },
        );
    }
    job
}

/// The tempo, as the mock writes it.
fn bpm_text(t: &Transport) -> String {
    format!("{:.1}", t.bpm)
}

/// The bar, as the mock writes it: `bar 37`.
fn bar_text(t: &Transport) -> String {
    format!("bar {}", t.bar())
}

/// **The frame readout as one laid-out run**: `58 fps · 12.4/16.6 ms`, with
/// the numbers in `.val` and everything else faint.
///
/// One [`LayoutJob`] rather than four galleys laid end to end, because the
/// mock is one run of text with two colours in it — `.val { color:
/// var(--c-text); font-weight: 500 }` inside a span that is `--c-faint` — and
/// laying it out as one is what keeps the spaces between the parts the type's
/// own rather than a gap this file invented. The weight is not honoured:
/// `egui`'s default proportional face has no bold, which `room` says once for
/// the whole crate.
///
/// **Both absences drop their own words and nothing else.** No rate drops
/// `58 fps · ` and leaves `12.4 ms`; no budget drops `/16.6` and leaves
/// `12.4 ms`. Neither draws a `0`, a `—` or a plausible 16.6 that nothing
/// measured.
fn frame_job(t: &Transport, val: Color32, faint: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut push = |text: String, colour: Color32| {
        job.append(
            &text,
            0.0,
            TextFormat {
                font_id: FontId::new(size::BASE, FontFamily::Proportional),
                color: colour,
                ..Default::default()
            },
        );
    };
    if let Some(fps) = t.fps {
        push(format!("{fps:.0}"), val);
        push(" fps · ".to_owned(), faint);
    }
    push(format!("{:.1}", t.frame_ms), val);
    match t.budget_ms {
        Some(budget) => push(format!("/{budget:.1} ms"), faint),
        None => push(" ms".to_owned(), faint),
    }
    job
}

/// A plain span of the console's own type: `font-size: 11px`, one colour.
fn span(text: &str, colour: Color32) -> LayoutJob {
    LayoutJob::simple_singleline(
        text.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        colour,
    )
}

/// Two colours mixed, `t` of the way from the first to the second.
///
/// In gamma space, component by component, because that is where a CSS
/// `linear-gradient` in `srgb` interpolates and this is transcribing one.
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let at = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    Color32::from_rgb(at(a.r(), b.r()), at(a.g(), b.g()), at(a.b(), b.b()))
}

/// **The transport row's contents**: the tempo, the beat grid, the bar and the
/// frame readout.
///
/// Where everything goes is [`transport_row`]'s, so this paints and derives
/// nothing. Term for term from `style.css`:
///
/// - `.bpm` — the gradient, which is [`bpm_job`]'s.
/// - the label beside it — `style="color:var(--c-faint)"` in the markup, which
///   is `pal.faint`.
/// - `.beat-grid i` — `background: var(--c-line)`, and `.on` is
///   `var(--c-pink)` with `box-shadow: 0 0 9px var(--c-glowp)`. The halo is an
///   [`egui::epaint::Shadow`] at [`size::BEAT_GLOW`] with a corner radius of
///   half the dot's height, which is the same mechanism the Outputs row's dot
///   uses for its own glow. **Those two are the ends of a ramp rather than two
///   states**: [`beat_at`] says how much of the light is on each dot, the fill
///   is mixed between the two colours by it and the halo is scaled by it, and
///   the mock's `.on` is what a dot with all of the light on it looks like.
///   The mock's own travel between those frames is `@keyframes beat-sweep`.
/// - `bar 37` — `style="color:var(--c-dim)"`, `pal.dim`, *a label beside a
///   value*.
/// - the frame readout — `.val` over `--c-faint`, which is [`frame_job`]'s.
///
/// Every galley is centred in the box [`transport_row`] gave it, which is
/// `align-items: center` done where the type's real height is known: a
/// 20px face is not 30 tall and an 11px one is not 16.5, so the CSS box and
/// the galley are different heights and the box is what is centred on.
fn transport_into(ui: &Ui, pal: &Palette, row: &TransportRow) {
    let t = &row.values;
    let painter = ui.painter();
    let centred = |rect: Rect, galley: std::sync::Arc<egui::Galley>| {
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            // Every run in these jobs carries its own colour, so the fallback
            // `epaint` substitutes for a `Color32::PLACEHOLDER` is never
            // reached — and where it were, ink is a better answer than none.
            pal.text,
        );
    };

    centred(row.bpm, painter.layout_job(bpm_job(t, pal.mint, pal.lav)));
    centred(row.label, painter.layout_job(span(BPM_LABEL, pal.faint)));

    let radius = CornerRadius::same((size::BEAT_H * 0.5) as u8);
    for index in 0..row.dots {
        let dot = row.dot(index);
        let lit = row.lit(index);
        // **The halo belongs to the light rather than to the dot**, so it
        // fades in and out with it instead of switching. Nothing at all is
        // added for a dot the light has left: `beat_at` is exactly zero a
        // pitch away, so a grid of four is one halo at the instant of a beat
        // and two between two of them, and never four.
        if lit > 0.0 {
            painter.add(
                egui::epaint::Shadow {
                    offset: [0, 0],
                    blur: size::BEAT_GLOW,
                    spread: 0,
                    color: pal.glow_pink.gamma_multiply(lit),
                }
                .as_shape(dot, radius),
            );
        }
        painter.rect_filled(dot, radius, mix(pal.line, pal.pink, lit));
    }

    centred(row.bar, painter.layout_job(span(&bar_text(t), pal.dim)));
    centred(
        row.frame,
        painter.layout_job(frame_job(t, pal.text, pal.faint)),
    );
}

// ---------------------------------------------------------------------------
// The arrangement pill
// ---------------------------------------------------------------------------

/// **The pill's first word**, the mock's own abbreviation one pill along from
/// `map · nanoKONTROL2 ▾`. The map pill names a file with *save*, *load* and
/// *start a new one* under it, and this is the same three over a different
/// file, so it is the same two-part label.
const ARRANGEMENT_LABEL: &str = "arr";

/// **What the pill says where no arrangement has been named, and it is
/// deliberately not a name.**
///
/// The default arrangement *has* no name: it reaches
/// [`crate::layout`] rather than a file, so it is the one arrangement nobody
/// could have saved and nothing filed can shadow it
/// ([ADR-0221](../../../docs/adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md)
/// §2). Writing `default` here would be this console inventing one — and a
/// worse invention than most, because an operator *may* save an arrangement
/// called `default` and it shadows nothing, so the pill would read the same
/// for two different states.
///
/// A space is what makes it safe as well as honest: a name is one path
/// component of letters, digits, `-` and `_`, so `the default` is not a name
/// anything can be filed under and no save can make this line ambiguous. It is
/// the preview cell's `C · off` one row up — a word for a state, where a name
/// would be a reading invented for a console that has none.
const NO_ARRANGEMENT: &str = "the default";

/// **What *save* is called in the menu.** The ellipsis is the one thing on
/// this panel that says *this item asks for something before it does
/// anything*, and it is only there while it is true: with a name in use,
/// saving again means that name and asks for nothing, so the word loses it.
const SAVE_ITEM: &str = "save";
const SAVE_ITEM_ASKING: &str = "save as…";

/// **What the reset is called in the menu**, in the manual's own words rather
/// than in the vocabulary's. *Reset the arrangement* is the row and
/// [`Op::Reset`] is the operation; *start a new one* is what the family reads
/// as from inside this control, where the default is *"one arrangement among
/// the ones you could name"* and not a fourth thing beside the three.
const NEW_ITEM: &str = "start a new one";

/// **The `▾` at the end of the pill, drawn rather than typed** — [`grip_dots`]'
/// reason one control along. Whether a black down-pointing small triangle is
/// in `egui`'s default face is a question with no good answer, and a triangle
/// is the same mark either way.
///
/// Half the type it sits beside wide and half of that tall, which is about
/// what the glyph's ink measures at [`size::BASE`].
const CHEVRON_W: f32 = size::BASE * 0.5;
const CHEVRON_H: f32 = CHEVRON_W * 0.5;

/// **What the arrangement pill reads this frame, and what its menu is doing.**
///
/// # The name and the list are handed in, for [`View::picture`]'s reason
///
/// Which arrangement is in use is *which file was last written or read*, and
/// which names exist is a directory. `src/` takes no device, no window and no
/// clock (ADR-0156) and it takes no disk either — `karakuri-console`'s
/// manifest has no entry that could reach one — so whoever owns the store
/// reads both and writes them here, exactly as whoever owns the engine writes
/// [`View::transport`]. What crosses the seam is a name and a list of names.
///
/// # The menu is not handed in, and that is the other half of the same seam
///
/// [`Menu`] is this crate's: it is what the *control* is doing, not what the
/// instrument is doing, and it moves only through the methods below. A menu
/// open in the program's memory would be the arrangement living in the
/// toolkit's memory one level along (ADR-0156's own argument), and the console
/// would then be drawing a state it could not answer questions about.
///
/// It is held here rather than in [`Panel`] because it is not part of the
/// arrangement: nothing about an open menu is saved, restored or reset, and a
/// [`Panel::restore`] that put somebody else's open menu back would be
/// restoring a gesture.
#[derive(Debug, Clone, PartialEq)]
pub struct Arrangement {
    /// **The arrangement in use**, or `None` for the default — which is not a
    /// name and is drawn as [`NO_ARRANGEMENT`].
    ///
    /// A save and a restore both put a name here, because both leave that
    /// arrangement the one in use; a reset takes it away, because the default
    /// is what is now on screen and it has no name.
    pub name: Option<String>,
    /// **Every name already filed**, in the order whoever read the store
    /// listed them — `Store::list_arrangements` sorts by name, so this is
    /// alphabetical and the menu does not sort it again.
    ///
    /// **Read when it changes rather than per frame**, which is
    /// [`View::library`]'s rule for its reason: a listing is a directory read
    /// and that is not a thing to do on a frame path (P-0072). It changes
    /// exactly when a save lands, and whoever performed the save is who
    /// re-reads it.
    ///
    /// Empty is a console with no store behind it — every test in this crate —
    /// and the menu then offers *save* and *start a new one* and lists
    /// nothing, which is honest: there is nothing to put back.
    pub filed: Vec<String>,
    /// What the control is doing. See [`Menu`].
    pub menu: Menu,
}

/// **What the pill's menu is doing**, and the console's only state that is
/// neither the arrangement nor a value handed in.
///
/// Three states rather than a `bool` and a buffer beside it: *shut*, *open*,
/// and *open with a name being typed into it*. The third is a state of the
/// menu and not a fourth thing, which is what stops a buffer being read while
/// nothing is asking for one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Menu {
    /// The pill alone.
    #[default]
    Shut,
    /// The menu is down: *save*, *start a new one*, and the names filed.
    Open,
    /// **The one flow on this panel that asks for letters**, with what has
    /// been typed so far.
    ///
    /// The buffer is a `String` this crate owns and whoever holds the keyboard
    /// fills, one character at a time, through [`Arrangement::typed`] and
    /// [`Arrangement::rubbed_out`] — the same split as everything else here,
    /// since `src/` has no key events to read (ADR-0156).
    ///
    /// **Nothing in it is checked.** A name that is not one path component is
    /// refused where the record is applied, in one sentence, by whoever writes
    /// the file — the surface owns the affordance and never the authority
    /// ([P-0076](../../../docs/principles/0076-a-surface-owns-the-affordance-never-the-authority.md),
    /// [P-0061](../../../docs/principles/0061-a-refusal-a-person-can-reach-from-two-surfaces-is-one-sentence.md)).
    /// A pill that quietly dropped the characters it did not like would be a
    /// rule an operator could only find by experiment.
    Naming(String),
}

impl Arrangement {
    /// **A console with no store behind it**: the default arrangement, nothing
    /// filed, and the menu shut.
    ///
    /// A `const` rather than a `Default` impl alone so that a test — and
    /// [`crate::input::claim`]'s own documentation — can name the state
    /// without building one. Every test in this crate is this.
    pub const NONE: Arrangement = Arrangement {
        name: None,
        filed: Vec::new(),
        menu: Menu::Shut,
    };

    /// **What the pill says after `arr ·`**: the name in use, or the word for
    /// the arrangement that has none. See [`NO_ARRANGEMENT`].
    pub fn word(&self) -> &str {
        self.name.as_deref().unwrap_or(NO_ARRANGEMENT)
    }

    /// The menu is down, whether or not a name is being typed into it.
    pub fn open(&self) -> bool {
        !matches!(self.menu, Menu::Shut)
    }

    /// **What has been typed so far**, or `None` when nothing is asking for a
    /// name. The caret is drawn after it and there is no selection: this is a
    /// name, not a document.
    pub fn naming(&self) -> Option<&str> {
        match &self.menu {
            Menu::Naming(typed) => Some(typed),
            _ => None,
        }
    }

    /// Put the menu down.
    pub fn opened(&mut self) {
        self.menu = Menu::Open;
    }

    /// Take it away, typed name and all. A name abandoned half-typed is not
    /// kept for the next time the menu opens: the buffer is the gesture, and
    /// the gesture ended.
    pub fn shut(&mut self) {
        self.menu = Menu::Shut;
    }

    /// **Ask for a name**, starting from empty. Reached only where there is no
    /// name in use — with one in use, *save* means that name and asks nothing
    /// ([`ArrangementPill::ask`]).
    pub fn asks_a_name(&mut self) {
        self.menu = Menu::Naming(String::new());
    }

    /// **One character into the name being typed**, and `false` where nothing
    /// was asking for one.
    ///
    /// Control characters are not a name and never reach the buffer — a
    /// newline is Return arriving as text, which is the commit and not a
    /// letter. Everything else does, unchecked, for the reason
    /// [`Menu::Naming`] gives.
    pub fn typed(&mut self, c: char) -> bool {
        match (&mut self.menu, c.is_control()) {
            (Menu::Naming(name), false) => {
                name.push(c);
                true
            }
            _ => false,
        }
    }

    /// **The last character back out again**, and `false` where there was
    /// nothing to take — no name being typed, or an empty one.
    pub fn rubbed_out(&mut self) -> bool {
        match &mut self.menu {
            Menu::Naming(name) => name.pop().is_some(),
            _ => false,
        }
    }

    /// **How many rows the open menu has**: *save*, *start a new one*, and one
    /// per name filed. Zero while the menu is shut or asking for a name, which
    /// is a menu with a field in it rather than a list.
    fn rows(&self) -> usize {
        match self.menu {
            Menu::Open => VERBS + self.filed.len(),
            _ => 0,
        }
    }
}

impl Default for Arrangement {
    fn default() -> Arrangement {
        Arrangement::NONE
    }
}

/// **The two items above the list**: *save* and *start a new one*. The names
/// filed follow them, and *load* is that list rather than an item of its own —
/// which is the manual's own sentence: *"load is a list — the names already
/// filed, which a hand can pick without typing anything."*
const VERBS: usize = 2;

/// **What a press on one of the menu's rows lands on.**
///
/// [`ArrangementPill::ask`] turns one of these into what the press *asks for*;
/// this is only which row it was. Split in two so that the hit test and the
/// operation are one derivation asked twice rather than one function that does
/// both — [`Outputs::op`]'s arrangement, over a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    /// *save*. With no name in use it asks for one; with a name in use it
    /// means that name.
    Save,
    /// *start a new one*, which is the reset.
    New,
    /// The name at this index of [`Arrangement::filed`] — put that
    /// arrangement back.
    Filed(usize),
}

/// **What a press on the pill or on one of its rows asks for.**
///
/// Every arm is either a move of this control's own state or one named
/// operation, and never a change to the arrangement made here: the pill asks,
/// and whoever applies the record decides
/// ([P-0076](../../../docs/principles/0076-a-surface-owns-the-affordance-never-the-authority.md)).
/// **Nothing is refused in this list.** A name nothing is filed under and a
/// file that disagrees with itself are both refused where the bytes are, in
/// one sentence each, and this control cannot see either.
#[derive(Debug, Clone, PartialEq)]
pub enum Ask {
    /// Put the menu down — a press on the pill with the menu shut.
    Open,
    /// Take it away — a press on the pill again, or anywhere off the menu
    /// while it is down.
    Shut,
    /// **Ask for a name**: *save* with no arrangement in use. The one item on
    /// this panel that asks for letters.
    Name,
    /// **The reset**, which reaches code and not a file, and is the same
    /// [`Op`] the `r` key performs — one operation, two surfaces
    /// ([ADR-0208](../../../docs/adr/0208-resetting-is-the-default-case-of-restoring-an-arrangement.md)).
    Panel(Op),
    /// **One operation of the vocabulary, named**: a save under a name, or a
    /// restore of one. Both reach a file and only whoever holds the store can
    /// perform either.
    Operation(Operation),
}

/// **The arrangement pill, laid out**: the capsule, what is written in it, and
/// the menu under it while it is down.
///
/// # One derivation, for [`Outputs`]' reason
///
/// [`View::draw`] paints exactly these rectangles and [`crate::input::claim`]
/// hit-tests exactly these rectangles. Two copies of the arithmetic is a menu
/// row that lights under a pointer that cannot pick it, with nothing on screen
/// saying so.
///
/// # It carries no name and no list
///
/// The rectangles are here and the words are [`Arrangement`]'s, which is why
/// [`ArrangementPill::ask`] takes one: a laid-out pill that had *copied* the
/// name it was measured from is a second copy to drift, and the caller has the
/// first one in its hand already. It is [`Mixer`]'s split with the borrow
/// turned round — the mixer keeps the strips because a knob's position *is* a
/// value, and nothing here moves with the name except the width.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrangementPill {
    /// **The capsule**, which is what a press has to land in to open the menu.
    /// The mock gives the whole pill the click and so does this.
    pub pill: Rect,
    /// Where `arr · night` is painted, inside the capsule's padding.
    pub text: Rect,
    /// The `▾` after it. Drawn rather than typed — see [`CHEVRON_W`].
    pub chevron: Rect,
    /// **The menu, or `None` while it is shut** — the whole card, which is
    /// what a press has to land in to be a pick rather than a dismissal.
    pub menu: Option<Rect>,
    /// **How many of [`Arrangement::rows`] the menu has room for**, between
    /// the pill and the bottom of the console.
    ///
    /// Fewer than there are is a store with more arrangements than the window
    /// is tall, and the foot says so in the Library bay's own words — `n of m`
    /// — rather than the list quietly ending. Zero while the menu is shut, and
    /// while it is asking for a name: that menu is a field, not a list.
    pub rows: usize,
    /// **What the menu would list if the window were tall enough**, carried so
    /// that the foot and the rows are one number rather than two.
    pub of: usize,
}

impl ArrangementPill {
    /// Whether `p` is on the pill itself.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.pill.contains(Pos2::new(p.x, p.y))
    }

    /// **Whether `p` is anywhere this control owns** — the pill, or the menu
    /// while it is down. This is what [`crate::input::claim`] asks, and it is
    /// wider than [`ArrangementPill::hit`] by exactly the open menu.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        let at = Pos2::new(p.x, p.y);
        self.pill.contains(at) || self.menu.is_some_and(|menu| menu.contains(at))
    }

    /// **Where one row of the menu is**, from the top. The rows stack with no
    /// gap between them, which is `.lib-list`'s own reading — the one list in
    /// the mock that has none.
    ///
    /// Panics on a row this menu has not got, which is [`TransportRow::dot`]'s
    /// rule: a caller has invented an item.
    pub fn row(&self, index: usize) -> Rect {
        assert!(index < self.rows, "row {index} of a menu of {}", self.rows);
        let menu = self.menu.expect("a menu with rows in it");
        let top = menu.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * index as f32
            // The rule under the two verbs, which the names sit below.
            + match index >= VERBS {
                true => size::HAIRLINE,
                false => 0.0,
            };
        Rect::from_min_size(
            Pos2::new(menu.min.x + size::LIB_LIST_PAD, top),
            egui::vec2(menu.width() - size::LIB_LIST_PAD * 2.0, size::LIB_ROW_H),
        )
    }

    /// **Which item `p` is on**, or `None` for a point on no row — the menu's
    /// padding, its foot, or anywhere off it.
    pub fn item(&self, p: karakuri_layout::Point) -> Option<Item> {
        let at = Pos2::new(p.x, p.y);
        (0..self.rows)
            .find(|index| self.row(*index).contains(at))
            .map(|index| match index {
                0 => Item::Save,
                1 => Item::New,
                other => Item::Filed(other - VERBS),
            })
    }

    /// **What a press at `p` asks for**, or `None` where the press was on
    /// nothing this control owns.
    ///
    /// The same derivation [`crate::input::claim`] hit-tests, asked a second
    /// time rather than copied — [`Outputs::op`]'s arrangement, and the reason
    /// is the same: the pill that claims a press and the pill that acts on it
    /// cannot come apart.
    ///
    /// # The three answers a press on a row can give
    ///
    /// - **Save** with a name in use is [`Operation::SaveArrangement`] naming
    ///   it. *"Once a name is in use, saving again means that name: saving
    ///   over it is what saving it again is."* With no name in use there is
    ///   nothing to save over, so it asks for one instead.
    /// - **Start a new one** is [`Op::Reset`] — the same operation `r`
    ///   performs, reached from the other end of the panel exactly as the
    ///   Outputs row's dot reaches `f`'s fold.
    /// - **A name** is [`Operation::RestoreArrangement`] naming it, which is
    ///   the reset's own sentence with a name in it.
    ///
    /// A press on the pill toggles the menu; a press anywhere else on the menu
    /// — its padding, its foot — shuts it, because a press that did nothing at
    /// all is the one thing worse than a press that declines.
    pub fn ask(&self, arr: &Arrangement, p: karakuri_layout::Point) -> Option<Ask> {
        if self.hit(p) {
            return Some(match arr.open() {
                true => Ask::Shut,
                false => Ask::Open,
            });
        }
        let menu = self.menu?;
        if !menu.contains(Pos2::new(p.x, p.y)) {
            return None;
        }
        Some(match self.item(p) {
            Some(Item::Save) => match &arr.name {
                Some(name) => Ask::Operation(Operation::SaveArrangement { name: name.clone() }),
                None => Ask::Name,
            },
            Some(Item::New) => Ask::Panel(Op::Reset),
            Some(Item::Filed(index)) => match arr.filed.get(index) {
                Some(name) => Ask::Operation(Operation::RestoreArrangement { name: name.clone() }),
                // A row past the end of the list, which `row` has already
                // refused to produce a rectangle for. Unreachable rather than
                // guessed at.
                None => Ask::Shut,
            },
            None => Ask::Shut,
        })
    }
}

/// **The arrangement pill's furniture, derived**: the capsule, the words in
/// it, and the menu under it.
///
/// # Where it sits, and why it is after the bar
///
/// `docs/manual/console.html`'s `.transport` is a flex row and this pill is
/// the last item in it before the `.sep`, immediately after
/// `map · nanoKONTROL2 ▾`. Everything the mock draws between the bar and that
/// pill is one of the six controls [`transport`] names and does not draw, so a
/// flex row closes up and this lands one [`size::TRANSPORT_GAP`] after
/// `bar 37`. That is the mock's own layout with the undrawn items taken out,
/// and not a position chosen here.
///
/// **It moves with the tempo, by a glyph or two.** `92.5` is narrower than
/// `128.0` and everything after it slides, which is what a flex row is and
/// what the beat grid and the bar already do. The alternative — pinning it to
/// the right edge, where the mock's `landed` and `rec` sit — buys a control
/// that never moves and puts it in the group the manual does not put it in.
///
/// # No row means no pill, and that is the row's answer rather than a second
/// one
///
/// `None` wherever [`transport`] answers `None`: the row folded away, soloed
/// away, too narrow, or a console with no engine behind it. The last is the
/// one worth stating, because the arrangement exists whether or not a tempo
/// does — but the *row* does not, and a pill floating in a bay that is drawing
/// nothing at all would be a control in a row that is not there. Where the row
/// is, this asks it for the bar's right edge and lays out from there, so the
/// pill's place and the readouts' places are one derivation.
///
/// # What it costs to ask
///
/// One galley lookup for the pill's own words, always. **While the menu is
/// down it is one more per row** — the two verbs and every name filed — since
/// the card is as wide as the widest thing in it, and a name this crate never
/// measured would be a name drawn outside its own card. Paid on a pointer
/// event and on a frame, and only while the menu is open, which is a menu an
/// operator is looking at.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one.
pub fn arrangement(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    arr: &Arrangement,
) -> Option<ArrangementPill> {
    // The row, asked once and for everything: whether there is one at all,
    // and where the bar ended. `transport` answers the first for the whole
    // module, which is what keeps *no engine means nothing at all* in one
    // place.
    let row = transport(ctx, layout, values)?;
    let strip = to_egui(layout.rect(layout.find("transport")?));
    let width = |text: &str| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size::BASE, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };

    let words = pill_text(arr);
    let text_w = width(&words);
    // `.pill`'s `padding: 0 8px` around the words, one `.sink` gap, and the
    // chevron — the one gap the mock states inside a capsule.
    let pill_w = size::PILL_PAD_X * 2.0 + text_w + size::SINK_GAP + CHEVRON_W;
    let mid = strip.center().y;
    let pill = Rect::from_min_size(
        Pos2::new(
            row.bar.max.x + size::TRANSPORT_GAP,
            mid - size::PILL_H * 0.5,
        ),
        egui::vec2(pill_w, size::PILL_H),
    );
    // The same rule [`outputs_row`] states: a capsule that does not fit in the
    // row it is drawn in is no control at all, rather than half of one over
    // the frame readout.
    if !strip.contains_rect(pill) || pill.max.x + size::TRANSPORT_GAP > row.frame.min.x {
        return None;
    }
    let text = Rect::from_min_size(
        Pos2::new(pill.min.x + size::PILL_PAD_X, mid - size::PILL_H * 0.5),
        egui::vec2(text_w, size::PILL_H),
    );
    let chevron = Rect::from_center_size(
        Pos2::new(pill.max.x - size::PILL_PAD_X - CHEVRON_W * 0.5, mid),
        egui::vec2(CHEVRON_W, CHEVRON_H),
    );

    let (menu, rows, of) = menu_card(&pill, layout, arr, &width);
    Some(ArrangementPill {
        pill,
        text,
        chevron,
        menu,
        rows,
        of,
    })
}

/// **The pill's words**: `arr · night`, or `arr · the default`.
///
/// One string rather than three galleys laid end to end, for [`frame_job`]'s
/// reason: the mock writes one run of text and laying it out as one keeps the
/// spaces round the `·` the type's own rather than a gap this file invented.
fn pill_text(arr: &Arrangement) -> String {
    format!("{ARRANGEMENT_LABEL} · {}", arr.word())
}

/// **The menu card under the pill**: where it is, how many rows fit in it, and
/// how many there are.
///
/// `(None, 0, 0)` while the menu is shut, which is the ordinary state and
/// costs one branch.
///
/// # It hangs from the pill and is held inside the console
///
/// Down from the pill's bottom edge by one [`size::PILL_GAP`], left-aligned
/// with it, and pushed back inside the viewport's right edge where a long name
/// would take it past — a card half outside the window is a list with items
/// nobody can read.
///
/// **The bottom is a count and not a clip.** The transport row is at the top
/// of the console, so a menu hanging down has the whole window; where a store
/// holds more arrangements than that window is tall, the card lists as many as
/// fit and says `n of m` in the Library bay's own foot. Truncating in silence
/// is the failure P-0027 is about, and this is that bay's answer to the same
/// question rather than a second one.
fn menu_card(
    pill: &Rect,
    layout: &karakuri_layout::Layout,
    arr: &Arrangement,
    width: &dyn Fn(&str) -> f32,
) -> (Option<Rect>, usize, usize) {
    if !arr.open() {
        return (None, 0, 0);
    }
    let viewport = to_egui(layout.viewport());
    let top = pill.max.y + size::PILL_GAP;

    // **Asking for a name is a field and not a list**, so the card is one row
    // wide enough to type into and there is nothing to pick.
    if let Some(typed) = arr.naming() {
        let field = width(&naming_text(typed)).max(pill.width());
        let card = held_inside(
            &viewport,
            pill.min.x,
            top,
            field + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0,
            size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H,
        );
        return (Some(card), 0, 0);
    }

    let of = arr.rows();
    let widest = std::iter::once(save_word(arr))
        .chain(std::iter::once(NEW_ITEM))
        .chain(arr.filed.iter().map(String::as_str))
        .map(width)
        .fold(pill.width(), f32::max);
    // How many rows there is room for between the card's top and the bottom of
    // the console, once the padding and the rule under the verbs are paid for.
    // The foot is only owed where something is left out, so it is asked for
    // twice: once assuming it is not there and once assuming it is.
    let furniture = size::LIB_LIST_PAD * 2.0 + size::HAIRLINE;
    let room = |foot: f32| {
        (((viewport.max.y - top - furniture - foot) / size::LIB_ROW_H).floor()).max(0.0) as usize
    };
    let rows = match room(0.0) >= of {
        true => of,
        false => room(size::LIB_FOOT_H).min(of),
    };
    let foot = match rows < of {
        true => size::LIB_FOOT_H,
        false => 0.0,
    };
    let card = held_inside(
        &viewport,
        pill.min.x,
        top,
        widest + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0,
        furniture + size::LIB_ROW_H * rows as f32 + foot,
    );
    (Some(card), rows, of)
}

/// A card of that size at that corner, pushed back inside the viewport's right
/// edge if it would hang over it — and never past its left edge, which is what
/// the `max` is for on a window narrower than the card.
fn held_inside(viewport: &Rect, x: f32, y: f32, w: f32, h: f32) -> Rect {
    let x = x.min(viewport.max.x - w).max(viewport.min.x);
    Rect::from_min_size(Pos2::new(x, y), egui::vec2(w, h))
}

/// **What *save* is called with this arrangement in use.** The ellipsis is
/// there exactly while the item asks for something — see [`SAVE_ITEM`].
fn save_word(arr: &Arrangement) -> &'static str {
    match arr.name {
        Some(_) => SAVE_ITEM,
        None => SAVE_ITEM_ASKING,
    }
}

/// **The name being typed, with the caret after it** — `night▏`.
///
/// One run of text rather than a galley and a drawn bar, so that measuring the
/// field and painting it cannot be two different runs: the caret is the width
/// of the caret whatever the face is, and a field measured without it would
/// put the caret outside its own card at the moment the name filled it.
fn naming_text(typed: &str) -> String {
    format!("{typed}{CARET}")
}

/// **The caret**, a light vertical bar. Typed rather than drawn, unlike the
/// chevron: `U+258F LEFT ONE EIGHTH BLOCK` is a box-drawing character, and
/// `egui`'s default face carries the block elements. Where it did not, the
/// fallback is a visible box in the one place an operator is looking, which is
/// louder than a caret that has quietly gone.
const CARET: char = '▏';

/// **The arrangement pill, painted**, and the menu under it.
///
/// Where everything goes is [`arrangement`]'s, so this paints and derives
/// nothing. Term for term from `.pill` in `style.css`:
///
/// - `border: 1px solid var(--c-line); border-radius: 999px; padding: 0 8px;
///   color: var(--c-dim)` — a capsule with a hairline round it, which is
///   [`pill_at`]'s treatment and this is the same pill in another row.
/// - the `▾` — `--c-dim` with the words, since it is part of the same run in
///   the mock's markup.
///
/// The menu has no term in the stylesheet, because the mock draws no menu: it
/// is the Library bay's list, which is the one list this console already
/// draws, at the same `.lib-row` box and inside the same `.lib-list` padding
/// (P-0019). It sits on a card with the panel's own shadow under it, which is
/// what says it is above the bays rather than inside one.
fn arrangement_into(ui: &Ui, pal: &Palette, pill: &ArrangementPill, arr: &Arrangement) {
    let painter = ui.painter();
    painter.rect_stroke(
        pill.pill,
        // `border-radius: 999px` on a box this short is a capsule.
        CornerRadius::same((size::PILL_H * 0.5) as u8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    let galley = painter.layout_no_wrap(
        pill_text(arr),
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.dim,
    );
    painter.galley(
        Pos2::new(
            pill.text.min.x,
            pill.text.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.dim,
    );
    // The `▾`: a triangle with its point down, in the box `arrangement`
    // measured for it.
    painter.add(egui::Shape::convex_polygon(
        vec![
            pill.chevron.left_top(),
            pill.chevron.right_top(),
            Pos2::new(pill.chevron.center().x, pill.chevron.max.y),
        ],
        pal.dim,
        Stroke::NONE,
    ));

    let Some(card) = pill.menu else {
        return;
    };
    painter.add(pal.shadow.as_shape(card, CornerRadius::same(8)));
    painter.rect_filled(card, CornerRadius::same(8), pal.panel);
    painter.rect_stroke(
        card,
        CornerRadius::same(8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );

    let row_text = |painter: &egui::Painter, rect: Rect, text: String, colour: Color32| {
        let galley = painter.layout_no_wrap(
            text,
            FontId::new(size::BASE, FontFamily::Proportional),
            colour,
        );
        painter.galley(
            Pos2::new(
                rect.min.x + size::LIB_ROW_PAD_X,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            colour,
        );
    };

    // **The field, where a name is being asked for.** The card has one row in
    // it and `pill.rows` is zero, which is what stops `row` handing out a
    // rectangle for something that is not a list.
    if let Some(typed) = arr.naming() {
        let field = Rect::from_min_size(
            Pos2::new(
                card.min.x + size::LIB_LIST_PAD,
                card.min.y + size::LIB_LIST_PAD,
            ),
            egui::vec2(card.width() - size::LIB_LIST_PAD * 2.0, size::LIB_ROW_H),
        );
        row_text(painter, field, naming_text(typed), pal.text);
        return;
    }

    for index in 0..pill.rows {
        let rect = pill.row(index);
        let (text, colour) = match index {
            0 => (save_word(arr).to_owned(), pal.text),
            1 => (NEW_ITEM.to_owned(), pal.text),
            other => (arr.filed[other - VERBS].clone(), pal.dim),
        };
        row_text(painter, rect, text, colour);
    }
    // The rule under the two verbs, which is what makes the names below it a
    // list rather than two more items.
    if pill.rows > VERBS {
        let y = pill.row(VERBS).min.y - size::HAIRLINE * 0.5;
        painter.line_segment(
            [
                Pos2::new(card.min.x + size::LIB_LIST_PAD, y),
                Pos2::new(card.max.x - size::LIB_LIST_PAD, y),
            ],
            Stroke::new(size::HAIRLINE, pal.hair),
        );
    }
    // **`n of m`, in the Library bay's own words**, and only where the list
    // could not be shown whole.
    if pill.rows < pill.of {
        let foot = Rect::from_min_max(
            Pos2::new(card.min.x, card.max.y - size::LIB_FOOT_H),
            card.max,
        );
        let galley = painter.layout_no_wrap(
            format!("{} of {}", pill.rows.saturating_sub(VERBS), pill.of - VERBS),
            FontId::new(size::LIB_FOOT_SIZE, FontFamily::Proportional),
            pal.faint,
        );
        painter.line_segment(
            [
                Pos2::new(foot.min.x, foot.min.y),
                Pos2::new(foot.max.x, foot.min.y),
            ],
            Stroke::new(size::HAIRLINE, pal.hair),
        );
        painter.galley(
            Pos2::new(
                foot.min.x + size::LIB_FOOT_PAD_X,
                foot.center().y - galley.size().y * 0.5,
            ),
            galley,
            pal.faint,
        );
    }
}

// ---------------------------------------------------------------------------
// The look: the tone map, and the level going into it
// ---------------------------------------------------------------------------

/// **The tone map pill's first word**, the mock's own two-part label one pill
/// along from `arr · night ▾` — and **no chevron**, which is what says this one
/// acts on the press instead of putting a card down. The two pills that open a
/// menu carry the mark and the four that do not (`learn`, `tap`,
/// `offset −15 ms`, this) do not.
const TONEMAP_LABEL: &str = "tone";

/// **The word before the exposure track** — `.trim`'s `g` one row up, and the
/// same arrangement: a faint label, [`size::TRIM_GAP`], a horizontal
/// [`fader`].
const EXPOSURE_LABEL: &str = "exp";

/// **Every tone map operator, in the order this control walks them.**
///
/// `karakuri_operation::Tonemap` has no `ALL` — nothing had read one until now
/// — so unlike the blend chip's cycle this is **not** a second copy of a list
/// the vocabulary keeps ([`after`] says what that costs). It is the only
/// statement of the order in this workspace apart from `karakuri-cli`'s own
/// `next_tonemap`, which walks the engine's `TonemapOp` in exactly this order,
/// and it is what [`next_tonemap`] is checked against.
///
/// The order is the engine's own declaration order, which is the order
/// [ADR-0037](../../../docs/adr/0037-tone-mapping-is-a-uniform-and-the-default-is-chosen-by-looking.md)
/// compared them in: `Clamp` first, because it is *"not a tone mapper — the
/// control that shows what the other three are fixing"* and is where a
/// `Present` that nobody has written to starts. A cycle should start where the
/// thing it cycles starts, which is [`next_shape`]'s own reason.
const TONEMAPS: [Tonemap; 4] = [
    Tonemap::Clamp,
    Tonemap::Reinhard,
    Tonemap::Aces,
    Tonemap::AgX,
];

/// **How many stops of exposure the track spans**, and the whole of what
/// decides its two ends: 1.0 sits at the middle, so the track runs from
/// `2^-6` to `2^6` — a sixty-fourth to sixty-four.
///
/// It is `karakuri-cli`'s own interactive range, restated rather than shared
/// because `EXPOSURE_MIN` and `EXPOSURE_MAX` are private to that binary and
/// this crate depends on nothing that could reach them. What is stated there
/// is *"the interactive bounds, and only the interactive ones"* — the bounds a
/// key nudges inside, where `--exposure` is deliberately not held to them
/// because *"a batch render asks for something extreme on purpose"*. A control
/// under a hand is exactly the interactive case, so this is the same span for
/// the same reason.
const EXPOSURE_STOPS: f32 = 12.0;

/// The two ends of that span, which is [`exposure_at`] at each end of the
/// track. Written out so a reader meets the numbers rather than an exponent,
/// and asserted against the derivation in `tests/look.rs`.
pub const EXPOSURE_MIN: f32 = 1.0 / 64.0;
pub const EXPOSURE_MAX: f32 = 64.0;

/// **How many presses of an exposure key cross the whole span.**
///
/// `karakuri-cli`'s `EXPOSURE_STEP` is `1.189_207`, which is `2^(1/4)`, and it
/// says why it is a ratio rather than an addition: *"a stop is a ratio, and an
/// additive step would be enormous at 0.1 and invisible at 8.0. This is a
/// quarter of a stop, near enough."* Four presses to a stop over
/// [`EXPOSURE_STOPS`] stops is forty-eight.
const EXPOSURE_PRESSES: f32 = EXPOSURE_STOPS * 4.0;

/// **The track, in pixels: one pixel a press.**
///
/// This is the number that makes the two surfaces agree instead of nearly
/// agreeing. A pointer on this track can ask for any of the 48 positions along
/// it and a keyboard stepping a quarter stop at a time can ask for any of the
/// 48 values between the ends, so **neither surface can reach a value the
/// other cannot** — and one pixel is the coarsest step that still reads as a
/// movement rather than as a sequence of positions, which is
/// [`BEAT_STALENESS`]' own rule about the beat's travel
/// ([P-0077](../../../docs/principles/0077-continuous-motion-is-how-a-stopped-panel-announces-itself.md)).
///
/// A width picked for looks would have been a number with nothing behind it,
/// which is what the mock's `width: 48px` says here as well.
pub const EXPOSURE_TRACK_W: f32 = EXPOSURE_PRESSES;

/// **What the look controls read this frame**: the operator that is running,
/// and the level going into it.
///
/// # The same seam as [`View::transport`], and it is not the Master bay's `out`
///
/// The look is the engine's — `karakuri_engine::frame::Look`, which is exactly
/// what `Present::set_tonemap` is told — and `src/` has no engine (ADR-0156),
/// so whoever owns one reads it and writes this per frame.
///
/// **`white_point` is not here**, and it is not an oversight: it is in the
/// record, it is Reinhard's parameter alone, and no surface has a control for
/// it, so a field here would be a value this crate carries and no control
/// names. That is the vocabulary's own line — `Operation::SetExposure` asks
/// for the level and nothing else, and whoever writes the record fills the
/// other two thirds in from the look that is running
/// ([ADR-0192](../../../docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)).
///
/// **And `exposure` here is the tone mapper's, never `Deck::set_out`.** The
/// two are levels that multiply in different places — the master out where the
/// mix writes the composited frame, this where the present pass reads it — and
/// with nothing in the master chain they are indistinguishable in every frame
/// this program can draw
/// ([ADR-0224](../../../docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md)).
/// The Master bay's row is the other one and nothing can move it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// The transfer that is running. One of [`TONEMAPS`].
    pub tonemap: Tonemap,
    /// The level going into it. Unbounded on the way **in** — a `--exposure`
    /// flag is not held to the track's ends — and drawn wherever
    /// [`unit_of`] puts it, which is at an end for anything past one.
    pub exposure: f32,
}

/// **The two look controls, laid out**: the capsule that names the operator,
/// and the track that sets the level.
///
/// # One derivation, for [`Outputs`]' reason
///
/// [`View::draw`] paints exactly these rectangles and [`crate::input::claim`]
/// hit-tests exactly these rectangles. Two copies of the arithmetic is a track
/// that lights under a pointer that cannot set it.
///
/// # Two controls and one type, because they are one group in the row
///
/// They are two operations and two presses
/// ([ADR-0192](../../../docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)),
/// and they are laid out together because the second's place is measured from
/// the first's — which is what a flex row is. Splitting them into two
/// functions would mean measuring the capsule twice, once to draw it and once
/// to find out where the track starts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LookRow {
    /// **The capsule**, which is what a press has to land in to move the tone
    /// map on.
    ///
    /// **As wide as the widest of the four names whatever word is in it**,
    /// which is [`StripBox::tally`]'s rule and not the blend chip's: a target
    /// sized to the word it is showing moves under the hand that is pressing
    /// it, and here it would take the exposure track with it.
    pub tone: Rect,
    /// Where `tone · aces` is painted, inside the capsule's padding. Narrower
    /// than the capsule for every operator but the widest, so the words are
    /// centred in it.
    pub tone_text: Rect,
    /// The faint `exp` before the track.
    pub label: Rect,
    /// **The track**, `.fader`'s 5px well lying down at
    /// [`EXPOSURE_TRACK_W`] long.
    pub track: Rect,
    /// What the level fills of it, from the left — [`unit_of`] of the
    /// exposure.
    pub fill: Rect,
    /// **What a press has to land in to set the exposure**: the track grown to
    /// a line's height and no wider.
    ///
    /// Grown, because a 5px-tall target is not something a hand finds — the
    /// `.mini`'s own argument (*"a 15px word is not something a hand finds,
    /// and `.mini`'s padding is what makes it one"*), met here by a band
    /// rather than by drawing a fatter track. **No wider**, because the value
    /// a press asks for is where along the track it landed, and a target that
    /// reached past either end would have two pixels of itself asking for the
    /// same value.
    pub grip: Rect,
    /// `1.00`, after the track. See [`look`] for why it is after it.
    pub value: Rect,
    /// **The values these rectangles were measured from**, carried for
    /// [`TransportRow::values`]' reason: whoever measured the type and whoever
    /// paints it are one statement.
    pub values: Look,
}

impl LookRow {
    /// Whether `p` is on the tone map's capsule.
    pub fn hit_tone(&self, p: karakuri_layout::Point) -> bool {
        self.tone.contains(Pos2::new(p.x, p.y))
    }

    /// Whether `p` is on the exposure track's band. See [`LookRow::grip`].
    pub fn hit_exposure(&self, p: karakuri_layout::Point) -> bool {
        self.grip.contains(Pos2::new(p.x, p.y))
    }

    /// **Whether `p` is on either of the two**, which is what
    /// [`crate::input::claim`] asks.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.hit_tone(p) || self.hit_exposure(p)
    }

    /// **What a press at `p` asks the tone map to become**, or `None` where
    /// there is no capsule under it.
    ///
    /// # The capsule cycles, and the operation names where it arrived
    ///
    /// Click it and the look moves to the next of [`TONEMAPS`] — `clamp`,
    /// `reinhard`, `aces`, `agx`, wrapping — and what comes out is
    /// [`Operation::SetTonemap`] naming the **destination**, never a step,
    /// because there is no step in the vocabulary to name. The affordance is
    /// [`Mixer::blend`]'s exactly
    /// ([ADR-0187](../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md))
    /// and so is the division it rests on: the cycle is [`next_tonemap`] here
    /// and nothing at all in `karakuri-operation`, which is P-0074's *"a
    /// toggle is an affordance, built over operations by whoever draws the
    /// control"*.
    ///
    /// **What a map is offered is the four values, not the cycle** — the same
    /// sentence ADR-0187 wrote about three, and the reason the manual's row
    /// says a map or a model *"would name the one it wants"*.
    ///
    /// # Why not a menu, now that there is one
    ///
    /// ADR-0187 could not have a popup: the blend chip is in a 53-wide strip,
    /// and a card had nowhere to be and no rule to be modal under. Both of
    /// those changed
    /// ([ADR-0225](../../../docs/adr/0225-a-menu-is-a-gesture-in-hand-rather-than-a-rectangle-on-the-panel.md)),
    /// so the alternative is genuinely available here and it is still refused.
    /// A menu is **a gesture in hand**: it takes every pointer event on the
    /// console until it is shut, and a boundary cannot be dragged while it is
    /// down. That is a price worth paying for a list of files an operator
    /// named and did not pay for four words that never change — and it puts a
    /// second press in front of a control that is pressed on the beat.
    pub fn tonemap(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_tone(p).then(|| Operation::SetTonemap {
            tonemap: next_tonemap(self.values.tonemap),
        })
    }

    /// **What a press at `p` asks the exposure to become**, or `None` where
    /// there is no track under it.
    ///
    /// # The press sets it outright, and that is the decision
    ///
    /// Where along the track the press landed *is* the value, through
    /// [`exposure_at`], and what comes out is [`Operation::SetExposure`]
    /// naming it. The manual is where the words for that come from — *"the
    /// value is absolute and the keys are the nudge … because a control that
    /// could only be nudged is a control no fader can reach"* — and the whole
    /// span is reachable in one press.
    ///
    /// **It is not [`Mixer::grab`], and the difference is not an
    /// inconsistency.** A fader answers `None` for a press on its *track*, on
    /// purpose: it has a knob, the knob keeps the offset it was taken hold of
    /// at, and a press that jumped the value would move the mix under a hand
    /// mid-gesture. There is no knob here and no gesture to be mid-way
    /// through — the press is the whole of it — so the two rules are answers
    /// to two different questions rather than one rule applied twice. Nothing
    /// is drawn that looks like a handle, for the same reason.
    ///
    /// **[`crate::input`]'s *a control claims what it acts on and no more* is
    /// kept exactly.** Every point of [`LookRow::grip`] asks for a value, so
    /// nothing is claimed in order to be thrown away.
    ///
    /// # It says one thing per press, so it needs no coalescing
    ///
    /// [ADR-0207](../../../docs/adr/0207-a-continuous-control-says-one-thing-per-frame.md)
    /// coalesces a swept MIDI fader to one operation a frame, and names the
    /// console's own fader as the other precedent — which emits only when the
    /// value changed, because it holds the value it last drew. Neither applies
    /// to a press: one press is one operation, and a second press in the same
    /// frame is a second thing an operator asked for.
    ///
    /// # And it can never be pending, which is why nothing marks a destination
    ///
    /// [ADR-0206](../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)
    /// gives a fader a mark for where a scheduled move is going.
    /// `karakuri_engine::deck::Control` is **per slot** — a gain, an opacity
    /// and a mask front — so there is no transition that can be scheduled on
    /// the look at all, and a mark here would be a presentation of a state
    /// that cannot occur.
    pub fn exposure(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_exposure(p).then(|| Operation::SetExposure {
            exposure: exposure_at((p.x - self.track.min.x) / self.track.width()),
        })
    }
}

/// **The next tone map round the cycle**, wrapping from the last back to the
/// first — the whole of the affordance the capsule is.
///
/// [`after`]'s division, one row up: the cycle is four lines here and nothing
/// in `karakuri-operation`, which owns the four operators and not the order a
/// pointer walks them in (P-0074).
///
/// **A match rather than an index into [`TONEMAPS`]**, for [`after`]'s reason:
/// a fifth operator does not compile until somebody says what follows it. The
/// price is that the order is written twice — here and in [`TONEMAPS`] — so
/// `tests/look.rs` walks the array through this and asserts they are the same
/// cycle, which is `tests/blend.rs`'s own measurement.
///
/// **`karakuri-cli` has this function over the engine's `TonemapOp`** and in
/// this order. Two crates naming the same order is what P-0074 costs a
/// vocabulary that depends on nothing, and the test above is what stops the
/// two drifting on this side.
fn next_tonemap(tonemap: Tonemap) -> Tonemap {
    match tonemap {
        Tonemap::Clamp => Tonemap::Reinhard,
        Tonemap::Reinhard => Tonemap::Aces,
        Tonemap::Aces => Tonemap::AgX,
        Tonemap::AgX => Tonemap::Clamp,
    }
}

/// **What a point `unit` of the way along the track asks for**, in exposure.
///
/// `2^((unit − ½) · stops)`: the middle of the track is `2^0`, which is
/// **exactly 1.0**, and the two ends are exactly [`EXPOSURE_MIN`] and
/// [`EXPOSURE_MAX`] — the subtraction is exact at a half and `exp2` of a whole
/// number is a power of two with no rounding in it.
///
/// # Logarithmic, and that is not a preference
///
/// Exposure is a ratio: `karakuri-cli`'s step is a *factor* and says why —
/// *"an additive step would be enormous at 0.1 and invisible at 8.0"*. A
/// linear track is that same mistake drawn in space rather than in time: over
/// [`EXPOSURE_MIN`] to [`EXPOSURE_MAX`] unity would sit **0.75 of a pixel**
/// from the left-hand end of [`EXPOSURE_TRACK_W`], so the half of the range
/// that darkens the picture would be one pixel wide and the other half would
/// be the whole control. Equal distances along this track are equal ratios,
/// which is the only reading under which both halves are usable.
pub fn exposure_at(unit: f32) -> f32 {
    ((crate::panel::unit(unit) - 0.5) * EXPOSURE_STOPS).exp2()
}

/// **Where an exposure sits on the track**, on `[0, 1]` — [`exposure_at`]
/// inverted, and clamped to the ends.
///
/// **A value past either end is drawn at that end**, which is deliberate and
/// is stated rather than hidden: `--exposure 200` is accepted unclamped by the
/// flag on purpose, and a fill that ran off the track would be a picture of a
/// value nobody could point at. The figure beside the track is what says the
/// number in that case, which is [`LookRow::value`]'s whole job. Zero and
/// negative — which the flag refuses everywhere — take `log2` to negative
/// infinity and land at the floor rather than anywhere surprising, and a NaN
/// is zero for [`crate::panel::unit`]'s reason.
pub fn unit_of(exposure: f32) -> f32 {
    crate::panel::unit(exposure.log2() / EXPOSURE_STOPS + 0.5)
}

/// **The look controls, derived**: the capsule that names the operator, the
/// track that sets the level, and the figure beside it.
///
/// # Where they sit, and why it is after the arrangement pill
///
/// `docs/manual/console.html`'s `.transport` is a flex row and these are the
/// last two items in it before the `.sep`, immediately after `arr · night ▾`.
/// The page's own argument for the place is that this row is already four
/// groups with a reason each — the clock, the four things that need an audio
/// input, the three that say what this console is *set up* as, and health past
/// the spacer — and a look is none of them: it needs no input, it is no file,
/// and it is played rather than arranged. So it is a fourth group at the end
/// of the left half, and the row then reads in the order a frame does, since
/// the tone map is the one transfer the pipeline ends in, applied immediately
/// before the single encode at the output
/// ([P-0064](../../../docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md),
/// [ADR-0037](../../../docs/adr/0037-tone-mapping-is-a-uniform-and-the-default-is-chosen-by-looking.md)).
///
/// **Nothing already drawn moves**, which is a consequence and not the reason:
/// [`arrangement`] is still one [`size::TRANSPORT_GAP`] after the bar, because
/// everything the mock draws between the two is still one of the six controls
/// [`transport`] names and does not draw.
///
/// # The order inside the group, and where the figure goes
///
/// The operator first and its level second, which is `Record::Look`'s own
/// order and the manual's row order — the exposure is *the level going into
/// that transfer*, so it reads as belonging to the thing on its left.
///
/// **The figure is after the track**, and that is the one piece of this layout
/// decided by what a hand does rather than by what the mock draws. It is the
/// only part of the group whose width moves with its value, so putting it last
/// leaves the capsule and the track exactly where they were: a target that
/// walked away from the pointer as it was set would be worst at the moment it
/// was being used most precisely. The capsule ahead of it is held to the
/// widest of the four names for the same reason
/// ([`LookRow::tone`]).
///
/// # No row and no pill means none of this
///
/// `None` wherever [`arrangement`] answers `None` — which is wherever
/// [`transport`] does — and `None` for a console with no engine behind it,
/// which is every test in this crate that does not hand a look in. The place
/// is measured from the pill's right edge, so a console that cannot draw the
/// pill cannot draw what comes after it either; that is one answer to *does
/// this row fit* rather than a second one.
///
/// # What it costs to ask
///
/// **Six galley lookups of its own**: the four operator names, because the
/// capsule is as wide as the widest of them; the `exp` label; and the figure.
///
/// **And it re-derives the row and the pill**, which is the honest cost of
/// being laid out from something else's right edge rather than from the row's:
/// [`transport`] is asked twice more per frame and [`arrangement`] once more,
/// where a paint-as-you-go pass that carried a cursor along the row would ask
/// each once. That is deliberate and it is [`Outputs`]' rule — the derivation
/// that draws a control is the one that hit-tests it, so a control cannot be
/// painted anywhere a press cannot reach — and it is what makes every
/// rectangle here something `tests/look.rs` can ask about without a device.
/// The measured price of the whole panel pass is a median 525 allocations a
/// frame (`crates/karakuri`'s `WRITTEN_ALLOCS`, taken 2026-08-26); this adds
/// on the order of forty, which is inside the factor of two that file will
/// quote a figure across and is why the figure has not been re-taken here —
/// re-taking one needs a window and three seconds of nobody touching it.
///
/// Paid on a pointer event and on a frame, and a console with no look behind
/// it pays none of it: the `look?` is the first line.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one.
pub fn look(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    arr: &Arrangement,
    look: Option<Look>,
) -> Option<LookRow> {
    let look = look?;
    // The pill, asked once and for both things it answers: whether the row
    // exists at all, and where the group before this one ended. `transport`
    // answers the first for the whole module and `arrangement` is laid out
    // from it, so this is that one answer read one item further along.
    let pill = arrangement(ctx, layout, values, arr)?;
    let row = transport(ctx, layout, values)?;
    let strip = to_egui(layout.rect(layout.find("transport")?));
    let width = |text: &str| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size::BASE, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };

    let mid = strip.center().y;
    let span_h = size::BASE * size::LINE;
    // The capsule is the widest of the four whatever it says — see
    // `LookRow::tone`.
    let widest = TONEMAPS
        .iter()
        .map(|op| width(&tone_text(*op)))
        .fold(0.0, f32::max);
    let tone = Rect::from_min_size(
        Pos2::new(
            pill.pill.max.x + size::TRANSPORT_GAP,
            mid - size::PILL_H * 0.5,
        ),
        egui::vec2(size::PILL_PAD_X * 2.0 + widest, size::PILL_H),
    );
    // Centred in a capsule it does not fill, which is what *as wide as the
    // widest* comes to for the other three.
    let words = tone_text(look.tonemap);
    let tone_text = Rect::from_center_size(
        Pos2::new(tone.center().x, mid),
        egui::vec2(width(&words), size::PILL_H),
    );

    let label = Rect::from_min_size(
        Pos2::new(tone.max.x + size::TRANSPORT_GAP, mid - span_h * 0.5),
        egui::vec2(width(EXPOSURE_LABEL), span_h),
    );
    let track = Rect::from_min_size(
        Pos2::new(label.max.x + size::TRIM_GAP, mid - size::FADER_H * 0.5),
        egui::vec2(EXPOSURE_TRACK_W, size::FADER_H),
    );
    let grip = Rect::from_min_max(
        Pos2::new(track.min.x, mid - size::PILL_H * 0.5),
        Pos2::new(track.max.x, mid + size::PILL_H * 0.5),
    );
    let value = Rect::from_min_size(
        Pos2::new(track.max.x + size::TRIM_GAP, mid - span_h * 0.5),
        egui::vec2(width(&exposure_text(look.exposure)), span_h),
    );

    // The rule `arrangement` and `outputs_row` both state: a control that does
    // not fit in the row it is drawn in is no control at all, rather than half
    // of one over the frame readout.
    if !strip.contains_rect(tone)
        || !strip.contains_rect(grip)
        || value.max.x + size::TRANSPORT_GAP > row.frame.min.x
    {
        return None;
    }

    Some(LookRow {
        tone,
        tone_text,
        label,
        track,
        fill: filled(track, Axis::Row, unit_of(look.exposure)),
        grip,
        value,
        values: look,
    })
}

/// **The capsule's words**: `tone · aces`.
///
/// One string rather than two galleys laid end to end, for [`pill_text`]'s
/// reason: the mock writes one run of text and laying it out as one keeps the
/// spaces round the `·` the type's own.
fn tone_text(tonemap: Tonemap) -> String {
    format!("{TONEMAP_LABEL} · {}", tonemap.name())
}

/// **The level, as the Master bay's own row writes it** — `out 1.00`, two
/// places. The two are the same kind of reading and are deliberately written
/// the same way; where they stop being the same *number* is
/// [ADR-0224](../../../docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md).
fn exposure_text(exposure: f32) -> String {
    format!("{exposure:.2}")
}

/// **The look controls, painted.**
///
/// Where everything goes is [`look`]'s, so this paints and derives nothing.
/// Term for term from `style.css`:
///
/// - `.pill` — the capsule, which is [`arrangement_into`]'s treatment and this
///   is the same pill three items along. No `▾`, because nothing opens.
/// - the `exp` before the track — `style="color:var(--c-faint)"` in the
///   markup, which is `pal.faint`, and it is `.trim .lbl`'s job one bay up.
/// - `.fader` — `background: var(--c-well)` with
///   `box-shadow: inset 0 0 0 1px var(--c-hair)` and `border-radius: 999px`,
///   which on a 5px box is a capsule.
/// - `.fader b` — `linear-gradient(90deg, var(--c-mint), var(--c-lav))`,
///   through [`gradient`], which is the one place that ramp is drawn and is
///   what the mixer's own faders are painted with.
/// - the figure — `.val`, `pal.text`, *a value*.
///
/// **`.fader s` is deliberately not drawn.** The mock's knob is a handle and
/// this control has none — see [`LookRow::exposure`].
fn look_into(ui: &Ui, pal: &Palette, row: &LookRow) {
    let painter = ui.painter();
    painter.rect_stroke(
        row.tone,
        CornerRadius::same((size::PILL_H * 0.5) as u8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    let centred = |rect: Rect, galley: std::sync::Arc<egui::Galley>, colour: Color32| {
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            colour,
        );
    };
    let words = tone_text(row.values.tonemap);
    centred(
        row.tone_text,
        painter.layout_no_wrap(
            words,
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.dim,
        ),
        pal.dim,
    );
    centred(
        row.label,
        painter.layout_no_wrap(
            EXPOSURE_LABEL.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.faint,
        ),
        pal.faint,
    );

    let radius = CornerRadius::same((size::FADER_H * 0.5) as u8);
    painter.rect_filled(row.track, radius, pal.well);
    painter.rect_stroke(
        row.track,
        radius,
        Stroke::new(size::HAIRLINE, pal.hair),
        StrokeKind::Inside,
    );
    gradient(painter, row.fill, Axis::Row, pal.mint, pal.lav, true);

    centred(
        row.value,
        painter.layout_no_wrap(
            exposure_text(row.values.exposure),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.text,
        ),
        pal.text,
    );
}

/// **The word at the head of the Outputs row**, in the source's own
/// capitalisation for the reason [`Kind::Bay`]'s title is: the mock
/// upper-cases in CSS, and that is done at paint time here so the word a
/// reader searches for is the word in the source.
///
/// It is **not** a [`bay_head`]. The mock's markup is an inline span with
/// `.bay-head`'s type on it — `font-size: 10px`, `letter-spacing: 0.16em`,
/// `text-transform: uppercase`, `--c-faint` — inside a row that has no head at
/// all (ADR-0159), so it is that typography and none of that structure: no
/// hairline under it, no pills or grip beside it, and the row's own padding
/// rather than a head's.
const OUTPUTS_LABEL: &str = "Outputs";

/// **The one sink the console has**, and the manual's name for it: *"The
/// picture in the Program bay is a sink like any other and is the first
/// row."*
const PROGRAM_VIEW: &str = "program view";

/// **The Outputs row, laid out**: where the word goes, where the console's one
/// control is, and whether that control is lit.
///
/// # One derivation, because a control drawn where it cannot be clicked is
/// silent
///
/// [`View::draw`] paints exactly these rectangles and [`crate::input::claim`]
/// hit-tests exactly these rectangles, the way [`preview_cells`] serves both
/// [`preview_rects`] and the frame. Two copies of this arithmetic is a dot
/// that lights up under a pointer that cannot switch it, and nothing on screen
/// says so.
///
/// # `on` is read, never stored
///
/// The manual: *"The picture is a sink, listed in Outputs as program view, and
/// **it is on screen exactly when that sink is on**."* So the sink's state is
/// [`Layout::visible`] on the picture's node and there is no second copy of it
/// to drift — a fold from the keyboard lights the dot down, and the dot folds
/// the same node the keyboard does.
/// [ADR-0161](../../../docs/adr/0161-a-solo-is-stored-because-it-cannot-be-derived.md)
/// stored `soloed` because it could not be derived; this can.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Outputs {
    /// Where the word OUTPUTS is painted: its top-left, and the box the
    /// galley fills.
    pub label: Rect,
    /// **The control**: `.sink`'s capsule, dot and name together, which is
    /// what a press has to land in. The mock gives the whole chip the click,
    /// not the dot alone — a 7px dot is not a target a hand finds, which is
    /// [`GRAB`]'s argument one control along.
    pub sink: Rect,
    /// The `.dot` inside it.
    pub dot: Rect,
    /// The node this sink switches: the picture, `program-view`.
    pub id: NodeId,
    /// Whether the picture is on screen — `layout.visible(id)`, read here.
    pub on: bool,
}

impl Outputs {
    /// **What a press on the control asks for.** Two operations and no toggle:
    /// [`Op::Fold`] while the picture is on, [`Op::Unfold`] while it is off.
    /// The toggle is this method — an affordance over two operations — and the
    /// vocabulary underneath it stays two things a MIDI map or an MCP call can
    /// ask for by name. See [`Op`].
    ///
    /// **A press on a dark dot always lights it**, whatever darkened it — the
    /// picture folded on its own, the Program bay folded around it, a solo
    /// that left it out. That is [`Op::Unfold`]'s rule and not a special case
    /// here: an unfold makes its node *visible*, so it undoes the way to it as
    /// well as the node. This control is why the rule is written that way, and
    /// the manual's own note on this row is the argument — *"Nothing is
    /// refused here, so nothing has to be explained: a control that quietly
    /// declines the last of something is a rule an operator can only find by
    /// experiment."* A press that lit nothing would be that rule with no words
    /// at all.
    pub fn op(&self) -> Op {
        match self.on {
            true => Op::Fold(self.id),
            false => Op::Unfold(self.id),
        }
    }

    /// Whether `p` is on the control.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.sink.contains(Pos2::new(p.x, p.y))
    }
}

/// **The Outputs row's furniture, derived**: the word, and the one sink.
///
/// `None` where there is no row to draw in — the row folded away, a solo
/// somewhere else, or a window too small to hold the chip — which is
/// [`picture_rect`]'s rule stated on a control instead of a picture: a
/// rectangle with nothing in it is not something to paint or to click.
///
/// # What is in the mock's row and is deliberately not here
///
/// The mock draws four `.sink`s and a `+ add output` pill. **One of them
/// exists.** Drawing the others is the scaffolding this module's
/// documentation refuses — a control that looks finished does not get
/// replaced, and each of these is a control over something that has not been
/// built:
///
/// - `projector · DELL U2720Q` is a second window on another display. That is
///   a second `Sink` and a second surface, and neither exists.
/// - `Syphon` and `NDI · no plugin` are plugin sinks. There is no plugin
///   system, and `NDI`'s own row says so — it is drawn `.absent`, which is a
///   control explaining that the thing it would switch is not installed.
/// - `+ add output` adds a region while the panel is running, which the arena
///   cannot do: `docs/roadmap.md` records it as **one gap drawn five times**
///   (`+ lane`, `+ add`, `+ add output`, `+` on the scope list, and the
///   inspector's `2 up`), and it arrives with the arena operations, not with
///   this row.
///
/// # The tooltip is not drawn, and not half-drawn either
///
/// Every `.sink` in the mock carries a `data-tip`, and the manual makes a
/// point of it: *"hover says three things: what the control is, what state it
/// is in, and what a click will do."* A tooltip needs `egui` to **own a
/// widget** — a `Response` with a hover state and a layer above the panel —
/// and this console paints, with no widget anywhere in it
/// ([`crate::input`]). Giving one control a widget is a decision about who
/// owns the pointer, and it is its own; so no tooltip is drawn here and no
/// half of one is left behind.
///
/// # The row does not wrap, and the mock's does
///
/// `.outputs` is `flex-wrap: wrap`, which is a rule about what happens to the
/// *fifth* thing in a row that is too narrow to hold it. There is one sink and
/// it fits at every width the console draws — 172 of a narrowest 990 — so
/// wrapping is a rule with nothing to apply to, and it is a decision to take
/// with the second sink rather than a behaviour to write for one.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one. `ctx` is asked for the type: the chip's width is the width of the name
/// in it, and where the name goes is where the word before it ended.
pub fn outputs(ctx: &egui::Context, layout: &karakuri_layout::Layout) -> Option<Outputs> {
    // **Fonts are not valid until `egui` has run a pass**, and it says so
    // outright. A pointer event can reach this before the first frame — the
    // window is up and the loop has not drawn yet — so the answer there is
    // that there is no control, which is also the true one: a control that has
    // never been drawn is not one a press can be on.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let row = to_egui(layout.rect(layout.find("outputs")?));
    let id = layout.find("program-view")?;
    let label = ctx.fonts_mut(|f| f.layout_job(label_job(Color32::PLACEHOLDER)).size().x);
    let name = ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            PROGRAM_VIEW.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    });
    outputs_row(row, label, name).map(|(label, sink, dot)| Outputs {
        label,
        sink,
        dot,
        id,
        on: layout.visible(id),
    })
}

/// The arithmetic of the row, away from the type it measures and the layout it
/// reads — the two rectangles [`outputs`] hands out and the dot inside the
/// second.
///
/// Term for term from `.outputs` and `.sink` in `style.css`:
///
/// - `.outputs { display: flex; align-items: center; gap: 8px;
///   padding: 8px 11px }` — the word and the chip laid left to right from
///   [`size::OUTPUTS_PAD_X`], one [`size::OUTPUTS_GAP`] between them, and both
///   **centred in the row** rather than sat on its padding.
/// - `.sink { padding: 1px 10px; gap: 6px; border-radius: 999px }` — a
///   [`size::SINK_H`] capsule holding a [`size::SINK_DOT`] dot, a
///   [`size::SINK_GAP`], and the name.
///
/// **The centring is where the 34 comes back.** The row is 34 and a sink is
/// 18.5, so there is (34 - 18.5) / 2 = 7.75 of row above the chip and 7.75
/// below — more than the six pixels [`GRAB`] widens the boundary above it by,
/// which is the whole reason this control can be clicked at all. `.outputs`'s
/// own padding is 8 and the arrangement rounded 8 + 18.5 + 8 down to 34, so
/// the quarter pixel the CSS and the row disagree by is spent here rather than
/// argued about: `align-items: center` is what the CSS says, and it is what
/// leaves the two clearances equal.
fn outputs_row(row: Rect, label_w: f32, name_w: f32) -> Option<(Rect, Rect, Rect)> {
    let mid = row.center().y;
    let label = Rect::from_min_size(
        Pos2::new(row.min.x + size::OUTPUTS_PAD_X, mid - size::HEAD_SIZE * 0.5),
        egui::vec2(label_w, size::HEAD_SIZE),
    );
    let sink = Rect::from_min_size(
        Pos2::new(label.max.x + size::OUTPUTS_GAP, mid - size::SINK_H * 0.5),
        egui::vec2(
            size::SINK_PAD_X * 2.0 + size::SINK_DOT + size::SINK_GAP + name_w,
            size::SINK_H,
        ),
    );
    let dot = Rect::from_center_size(
        Pos2::new(sink.min.x + size::SINK_PAD_X + size::SINK_DOT * 0.5, mid),
        egui::vec2(size::SINK_DOT, size::SINK_DOT),
    );
    // The same rule [`picture_rect`] states, on the chip rather than on the
    // row because the chip is what is drawn and clicked: a row folded away has
    // a rectangle with no extent in it, and one too narrow for the chip has
    // nowhere to put it. Either way there is no control.
    match row.contains_rect(sink) {
        true => Some((label, sink, dot)),
        false => None,
    }
}

/// The word OUTPUTS as one laid-out run, so that measuring it and painting it
/// cannot be two different runs of type.
fn label_job(colour: Color32) -> LayoutJob {
    spaced(
        &OUTPUTS_LABEL.to_uppercase(),
        size::HEAD_SIZE,
        colour,
        size::HEAD_TRACKING,
    )
}

// ---------------------------------------------------------------------------
// The Mixer bay
// ---------------------------------------------------------------------------

/// **The word at the head of the Mixer bay**, in the source's own
/// capitalisation for [`Kind::Bay`]'s reason: the mock upper-cases in CSS, and
/// that is done at paint time so the word a reader searches for is the word in
/// the source.
const MIXER_TITLE: &str = "Mixer";

/// `.trim .lbl`: the `g`, which is the only word in this bay that is neither
/// the deck's nor the engine's — it is the mock's.
const TRIM_LABEL: &str = "g";

/// **How long the panel has been animating**, and the one value everything
/// that moves on it derives its own rate from.
///
/// # One phase, panel-wide, because two clocks drift and one does not
///
/// [P-0075](../../../docs/principles/0075-a-pending-transition-shows-where-it-is-where-it-is-going-and-that-it-has-not-arrived.md)
/// asks for exactly this: *"Everything pending moves together. Two controls
/// moving out of step looks broken rather than informative, and it is not a
/// smaller cost either: N independent animations are N deadlines for a
/// scheduler to service where one phase is one."* Two strips parked at once
/// are two rolls, and they are the same roll because they are read off the
/// same number.
///
/// # It is elapsed time and not a wrapped fraction
///
/// A phase already reduced to *where we are in the cycle* fixes the cycle, and
/// then a second presentation at another rate cannot be derived from it at all
/// — it would need a phase of its own, which is the drift this exists to
/// prevent. So the value is the whole elapsed interval and each presentation
/// takes its own [`Phase::cycle`] out of it. The console has one moving thing
/// today; it is about to have the beat and the rest of the mixer, and *this is
/// where the second one is either free or a second clock*.
///
/// # It is a `Duration` and not an `Instant`, which is the whole seam
///
/// `src/` reads no clock. [`Transport`]'s own doc says why in the general
/// case — *"an `Instant` here would put a clock in it, and then the row would
/// be reading wall time in a repository whose first principle is that nothing
/// does"*
/// ([P-0002](../../../docs/principles/0002-simulation-time-comes-from-a-record-never-from-a-clock.md))
/// — and every `Instant::now` in this crate is in `crates/karakuri/src/main.rs`, which
/// owns the window. An `Instant` is a *reading*; a `Duration` is a number, and
/// a number is what a caller writes and a test chooses. So this arrives per
/// frame the way [`Transport`] and [`View::mixer`] arrive
/// ([ADR-0156](../../../docs/adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)),
/// and a test asserting an animation writes the phase it wants rather than
/// catching one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Phase(Duration);

impl Phase {
    /// The origin, and what a console with no clock behind it is at — every
    /// test in this crate, and the first frame of a window.
    pub const ZERO: Phase = Phase(Duration::ZERO);

    /// A phase from the interval that has elapsed since whoever owns the clock
    /// started counting. Where the origin is does not matter and is not
    /// asked: every presentation is periodic in it.
    pub const fn since(elapsed: Duration) -> Phase {
        Phase(elapsed)
    }

    /// **Where this phase sits inside one `period`**, as a fraction in
    /// `[0, 1)`.
    ///
    /// The one operation a presentation performs on a phase, and the reason
    /// the carrier is an interval: a rate is chosen here, by the thing that
    /// moves, rather than baked into the value the harness writes.
    ///
    /// In `f64` and returned as `f32`. A session runs for hours and a phase is
    /// seconds from an origin — at 3600 s an `f32` step is already coarser
    /// than a millisecond, and the fractional part is what survives the
    /// division, so the division is the one place the extra bits are worth
    /// having.
    pub fn cycle(self, period: Duration) -> f32 {
        let period = period.as_secs_f64();
        match period > 0.0 {
            true => (self.0.as_secs_f64() / period).fract() as f32,
            false => 0.0,
        }
    }
}

/// **How often anything pending on this panel sets off toward where it is
/// going**, and the period every other number in the presentation is a
/// fraction of.
///
/// One second, which is P-0075's own *"once a second"* and is the rate that
/// reads as *waiting* rather than as a fault. See
/// [ADR-0190](../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md).
///
/// **Two users, one rate, and that is the rule rather than a coincidence**:
/// the tally's word rolling toward a residency, and a fader's fill reaching
/// toward a scheduled value ([`Reach`],
/// [ADR-0206](../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)).
/// P-0075 asks for one phase panel-wide because two controls moving out of
/// step looks broken rather than informative, and a second period here is how
/// that would happen with nothing failing to compile.
pub const ROLL_PERIOD: Duration = Duration::from_millis(1000);

/// **How much of the period the word is moving for**: 400 ms out and back,
/// and 600 ms at rest.
///
/// The rest is not slack. It is what makes the roll read as *an attempt that
/// keeps being made* rather than as a chip that wobbles — and it is what makes
/// most of a parked strip's frames the strip's settled appearance, so the
/// operator reads the effective residency off a still chip nearly two thirds
/// of the time (P-0075's first clause).
pub const ROLL_TRAVEL: Duration = Duration::from_millis(400);

/// **How far toward the destination the moving thing gets**: a fraction of the
/// pitch between the two words on the tally, and a fraction of the gap between
/// the two values on a fader.
///
/// Less than one, and that is the presentation: *"a slot machine that never
/// quite lands"*. Landing is what arrival looks like, so a roll that reached
/// 1.0 would state the opposite of the truth once a second — and a band that
/// covered the gap to a fader's mark would say a scheduled fade had already
/// run. At 0.4 the current word keeps most of its rows and the destination
/// shows enough of its own to be read.
pub const ROLL_REACH: f32 = 0.4;

/// **How many steps the travel is drawn in**, which is the only reason
/// [`ROLL_STALENESS`] is a number at all.
const ROLL_STEPS: u32 = 12;

/// **How stale the roll may get**, which is what a presentation declares under
/// [P-0072](../../../docs/principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md)
/// and what the harness turns into a deadline.
///
/// [`ROLL_TRAVEL`] in [`ROLL_STEPS`] steps — 33.33 ms, about thirty a second.
/// It is **one number for the whole period** rather than a fine one while the
/// word moves and a coarse one while it rests: two numbers is two live regions
/// wearing one name, and choosing between them frame by frame is a scheduler's
/// job (P-0072's second half) rather than a presentation's. A presentation
/// declares what it needs; what the panel can afford is decided somewhere
/// else.
pub const ROLL_STALENESS: Duration =
    Duration::from_micros(ROLL_TRAVEL.as_millis() as u64 * 1000 / ROLL_STEPS as u64);

/// **How far a pending presentation has got toward its destination this
/// frame**, as a fraction of the distance to it: `0.0` at rest, [`ROLL_REACH`]
/// at the top of the travel, and never `1.0`.
///
/// One curve, two readers — the pitch between the tally's two words
/// ([`tally_into`]) and the gap between a fader's value and its mark
/// ([`reach`]).
///
/// A raised cosine over [`ROLL_TRAVEL`], flat for the rest of
/// [`ROLL_PERIOD`]. It is that curve rather than a triangle for one reason
/// worth having: it leaves and arrives at zero *with zero velocity*, so the
/// word does not snap into the rest it holds for the next 600 ms, and the
/// frame the travel begins on is not a jump.
///
/// **A pure function of the phase, which is the point of the phase being a
/// value.** A test asserts it at a phase it chose; nothing samples a clock to
/// find out what the panel is doing.
pub fn roll_at(phase: Phase) -> f32 {
    let travel = ROLL_TRAVEL.as_secs_f32() / ROLL_PERIOD.as_secs_f32();
    let t = phase.cycle(ROLL_PERIOD);
    match t < travel {
        true => ROLL_REACH * 0.5 * (1.0 - (t / travel * std::f32::consts::TAU).cos()),
        false => 0.0,
    }
}

/// **Where a slot sits between compiled and composited**, which is the mock's
/// `.tally` and `karakuri_engine`'s `Residency` — three states and no fourth.
///
/// The words are the mock's abbreviations and the variants are the engine's
/// names, because each document owns one end: `live`, `prim` and `alloc` are
/// what fits in a strip 53 wide, and `Live`, `Priming` and `Allocated` are
/// what `Deck::residency` answers.
///
/// **The mock draws a fourth and it is not a residency.** `.strip.empty` reads
/// `empty` in `.tally.off`, and a `Deck` has no such slot: every one of its
/// `slot_count` slots holds a `HotSwap`. An empty strip is a *track on the
/// page nothing fills*, and this bay draws nothing at all in one rather than a
/// strip full of dashes — see [`mixer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tally {
    /// `.tally.live` — stepped and composited. On air.
    Live,
    /// `.tally.priming` — stepped and warming its buffers, drawn only while
    /// something auditions it.
    Priming,
    /// `.tally.alloc` — compiled, buffers held, not stepping, keeping its `t`.
    Allocated,
}

impl Tally {
    /// **Every residency there is**, so that anything which has to hold all
    /// three cannot be given two.
    ///
    /// One customer today and it is [`mixer`], which measures the chip against
    /// the widest word rather than the current one. A `match` cannot express
    /// *the widest of them*, and a list written at the call site would be a
    /// second list of the residencies — the exact drift a fourth variant would
    /// walk straight past. Here it is one line under the enum, and a fourth
    /// variant that is not added to it is a `[Tally; 3]` that no longer
    /// compiles.
    pub const ALL: [Tally; 3] = [Tally::Live, Tally::Priming, Tally::Allocated];

    /// The mock's own word, lower-case here and upper-cased at paint time
    /// because `.tally` is `text-transform: uppercase` — the rule
    /// [`Kind::Bay`]'s title states.
    pub fn word(self) -> &'static str {
        match self {
            Tally::Live => "live",
            Tally::Priming => "prim",
            Tally::Allocated => "alloc",
        }
    }
}

/// **What shape of the frame a layer reaches**: the second `.mini` in a strip,
/// and `karakuri_engine`'s `MaskKind` — three kinds and no fourth.
///
/// # It is a mark rather than a word, and the mark is drawn rather than typed
///
/// The mock writes `&#9711;` and `&#9681;` — a circle, and a circle with one
/// half filled — and it has to: a strip is 53 wide inside its padding, and
/// `over` beside `linear` is 75 before either mini's border. So the mask says
/// its state in a shape.
///
/// **Drawn rather than set as a glyph**, which is [`grip_dots`]'s argument one
/// control along: whether `◯` and `◑` are in `egui`'s default face is a
/// question with no good answer, and a circle is the same mark either way. It
/// also settles the third one. The mock names `◯` in its own tooltip —
/// *"Mask: none"* — and draws `◑` on the strip beside it without saying which
/// kind that is, and it draws no third mark anywhere; a glyph for the third
/// would be a character picked out of a font, where a **mark** is the shape
/// the mask makes and can be argued from the mask.
///
/// # There is no `ALL` beside it, where [`Tally`] and `BlendMode` have one
///
/// Both of those constants exist for a **reader**: `Tally::ALL` is what
/// [`mixer`] measures the tally capsule against, because a `match` cannot
/// express *the widest of them*, and `BlendMode::ALL` is what a map file is
/// offered. Nothing measures this chip against its three shapes — it holds a
/// mark rather than a word and is [`size::MINI_SIZE`] wide whichever shape it
/// is showing — and no map target names a shape, which is why
/// `karakuri_operation::WipeKind` has no `ALL` either and says so at
/// [`WipeKind::name`]. A list written here would be a second statement of the
/// order with no reader, and the order is [`next_shape`]'s: a `match`, so a
/// fourth shape does not compile until somebody says what follows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mask {
    /// No mask: the layer reaches the whole frame. The mock's `◯`.
    None,
    /// A straight edge across the frame — the mock's `◑`, which is what a hard
    /// edge down the middle of a circle looks like.
    Linear,
    /// A circle. The same outline with a filled centre, which is the same
    /// family as the mock's two and is the shape a radial mask makes.
    Radial,
}

/// **What a slot's meter last read**, which is two of the four numbers
/// `karakuri_engine::meter::Level` carries.
///
/// Plain numbers, for [`Transport`]'s reason: this crate takes no engine
/// (ADR-0156), so whoever owns the deck reads a level and writes this.
///
/// # What is left out, and why each
///
/// - **`frames_behind`.** `Deck::level` reads back without ever waiting, so a
///   reading is a few frames old and the engine says by how many. It is not
///   carried, and not because it does not matter: the engine's own
///   measurements are **1** frame behind under pacing, *"2 or 3"* on a vsync
///   window, and *"tens: 13 to 101"* with nothing pacing the loop at all — and
///   all three are normal. A staleness threshold picked in this crate would
///   blank a healthy meter on one loop and pass a dead one on another, because
///   the console does not know which loop it is on. What the meter draws is a
///   fact about a frame that has already been drawn, exactly as
///   [`Transport::frame_ms`] is, and it does not go stale by standing still
///   the way a *rate* does — which is the whole of why `frame_ms` is a number
///   and `fps` is an `Option`. So **handing over no reading at all is the
///   caller's decision**, on the same terms `fps: None` is, and the caller is
///   the one thing that knows its own loop.
/// - **`bad_texels`.** It is the denominator the mean was taken over rather
///   than a reading — *"`mean` and `peak` are computed over the texels this
///   did not count"* — and the mock's 6px meter has nowhere to say it. A meter
///   that drew it would be reporting on the measurement instead of on the
///   image.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Level {
    /// Mean luminance over the whole frame — *"the figure to match faders
    /// on"*, and what the meter's column is as tall as.
    pub mean: f32,
    /// The brightest single texel. What the peak mark sits on.
    pub peak: f32,
}

/// **What one mixer strip reads this frame**: what the deck is playing, where
/// the slot sits, and the four numbers in front of it.
///
/// # The same seam as [`Transport`], and it is not crossed
///
/// Every field is a string, a number or a word, and none of them is a `Deck`.
/// `src/` takes no device and no engine (ADR-0156), so whoever owns the deck
/// reads `Deck::residency`, `gain`, `opacity`, `blend`, `mask` and `level` and
/// writes one of these per slot per frame — exactly as whoever owns the device
/// registers a texture and writes [`View::picture`].
///
/// **No repaint arm for the values arriving**, which is [`Transport`]'s
/// reason: these move when the engine draws a frame, and a caller drawing
/// engine frames is already asking for frames for the picture two bays along.
///
/// **A drag on one of the two faders is a different thing and does have an
/// arm** — [`crate::repaint::Change::Emitted`]. That is not the value
/// arriving; it is an operation leaving, on a gesture an operator is making,
/// and it is on the list because [`crate::repaint::Change`] is one list of
/// everything that can change what the console shows and a fader that reached
/// no repaint would leave the strip drawn at the value before the drag.
///
/// # Five of these are controls and the rest are readouts, and the source
/// says which
///
/// **The two faders are played.** A press on the trim's knob or the fader's
/// knob takes it in hand ([`Mixer::grab`]), and dragging it emits
/// [`Operation::SetGain`] or `SetOpacity` for the caller to turn into a
/// record. Nothing here applies it, and nothing here keeps the value: the
/// fields below are still written by whoever owns the deck, every frame, so a
/// fader that has just been dragged shows the new number **because the deck
/// changed**.
///
/// **The blend chip is the third, and it cycles.** A press on it emits
/// [`Operation::SetBlendMode`] naming the mode after this one ([`Mixer::blend`]),
/// and nothing here applies that either. It is one control emitting three
/// operations, which is P-0074's own worked example of an affordance — see
/// [ADR-0187](../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md).
///
/// **The tally chip is the fourth, and it cycles too.** A press on it emits
/// [`Operation::SetResidency`] naming the next of the three ([`Mixer::tally`])
/// — counted from [`Strip::requested`] rather than from [`Strip::tally`],
/// which is what makes a press on a parked chip the withdrawal of its own
/// prime request without a case in the code for it
/// ([ADR-0195](../../../docs/adr/0195-the-tally-chip-cycles-from-the-request-so-the-parked-case-needs-no-case.md)).
///
/// **The mask mini is the fifth, and it cycles too.** A press on it emits
/// [`Operation::SetMaskShape`] naming the shape after this one ([`Mixer::mask`])
/// — **and the angle this strip is already wearing**, which is
/// [`Strip::mask_angle`]: the chip chooses a shape and the angle is not its
/// business, so it hands back the one it was given rather than a default that
/// would straighten a diagonal front
/// ([ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)).
///
/// **Everything else in the bay is a readout.** The meter and the number are
/// drawn from what the deck says and a press on either reaches nothing — and
/// so does a press on a fader's *track*, off the knob, which would otherwise
/// be a jump nobody asked for. `tests/mixer.rs` asserts both directions rather
/// than leaving either to be inferred from the absence of a hit test.
///
/// **[`Strip::gain_to`] and [`Strip::opacity_to`] are readouts too**, and the
/// order is the tally's: the roll was drawn a commit before the chip became a
/// control, because a control that could not yet say it was pending would look
/// dead for exactly as long as that commit. Nothing here schedules a move,
/// nothing here cancels one, and no press on a mark reaches anything — a
/// scheduled fade is armed from a key, a map or an MCP call, and what this
/// strip owes it is that it is not invisible until it happens
/// ([ADR-0206](../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)).
#[derive(Debug, Clone, PartialEq)]
pub struct Strip {
    /// **What the deck is playing**, in `.strip-name`.
    ///
    /// A `String`, and **the harness's word rather than the engine's**:
    /// nothing reachable from a `Deck` carries a name for the material in a
    /// slot. `Deck::slot` hands out a `HotSwap`, `HotSwap::set` hands out a
    /// `Set`, and a `Set` names its *nodes* (`Set::node_names`) and its
    /// *published controls* (`Published::name`) and has no name of its own —
    /// which is right, because a Set is built from a list of `.kir` files and
    /// only whoever passed that list knows what to call the result. So the
    /// caller names its own material, and a name invented in `src/` would be a
    /// label the console made up about somebody else's Set.
    ///
    /// Elided rather than wrapped where it does not fit, which is
    /// `.strip-name`'s own `overflow: hidden; text-overflow: ellipsis;
    /// white-space: nowrap`: a strip is 53 wide inside its padding and most
    /// real names are wider. An empty string draws no name at all, which is a
    /// harness with nothing to say rather than a strip with nothing in it.
    pub name: String,
    /// Where the slot sits — `Deck::residency`, which is the **effective**
    /// residency and not `requested_residency`. The governor moves a slot down
    /// without anybody asking it to, and a tally that did not follow it would
    /// be showing what was asked for over a slot doing something else.
    ///
    /// **What the chip draws, and not what a press on it counts from.**
    /// [`Mixer::tally`] steps from [`Strip::requested`]: the word says where
    /// the deck *is*, and the cycle is about what was last asked for. The two
    /// are the same value on every settled slot and they part exactly where it
    /// matters — see [`Mixer::tally`] for the parked case, which is the whole
    /// argument.
    pub tally: Tally,
    /// **What was asked for** — `Deck::requested_residency`, the other half of
    /// the pair [`Strip::tally`] is one of, and **what a press on the chip
    /// counts from** ([`Mixer::tally`]).
    ///
    /// # Why the strip carries both and derives nothing else
    ///
    /// [P-0075](../../../docs/principles/0075-a-pending-transition-shows-where-it-is-where-it-is-going-and-that-it-has-not-arrived.md)
    /// asks a pending control to say three things — where it is, where it is
    /// going, and that it has not arrived — and the first two *are* these two
    /// values. The third is [`Strip::pending`], which is a comparison of them.
    ///
    /// **There is deliberately no `parked: bool` beside them.** A third field
    /// would be the same fact stored twice, and the copy is the one that goes
    /// stale: the harness could write a `tally` and a `requested` that
    /// disagree and a `parked` that says they do not, and nothing in this
    /// crate could tell. One derivation with several readers is this crate's
    /// habit — [`StripBox::fader_at`] is the same shape — and it is P-0075's
    /// *derived every frame, never stored* read one level down, in the surface
    /// rather than in the engine.
    ///
    /// **Two fields rather than one `Tally` grown into a pair.** `Tally` is
    /// the mock's `.tally` and the engine's `Residency`, and both of those
    /// name *one* place a slot can sit; a variant meaning "allocated, and
    /// asked to prime" would be a fourth residency in a type whose own
    /// documentation says there are three and no fourth. The relation between
    /// two residencies is not a residency.
    pub requested: Tally,
    /// **The trim** — `Deck::gain`: linear, floored at zero, and deliberately
    /// open above 1.0 because the mix is HDR.
    ///
    /// Its own field and not the fader's twin. *"`gain` is a trim and
    /// `opacity` is the fader"*: opacity at zero silences under every blend
    /// mode and gain at zero does not silence `over`, which
    /// `karakuri-engine/src/mix.rs` states outright — so these are two
    /// controls, and the bay draws them as two rather than as two identical
    /// sliders. A `g` and a mini fader lying down against the tall one in the
    /// middle of the strip.
    pub gain: f32,
    /// **Where a scheduled move is taking the trim**, or `None` for a trim
    /// nothing is moving — `Deck::transitions_on(slot)`'s
    /// `Control::Gain` entry, and its `Transition::to`.
    ///
    /// # It is the destination and nothing else about the move
    ///
    /// A `Transition` also carries the instant it starts, how long it lasts
    /// and the curve it takes. None of the three is here, and the reason is
    /// the same seam every other field on this type is on: **the console has
    /// no beat count**, so a start in beats and a length in beats are two
    /// numbers it could not turn into anything a strip draws. What it can draw
    /// is where the control is going, which is P-0075's second clause and the
    /// whole of what the fader is being asked to say.
    ///
    /// **Armed and running are the same state here, deliberately.** A
    /// transition is in the deck's list from the moment it is scheduled until
    /// the beat it finishes on, and `Deck::transitions_on` answers with it
    /// throughout — so a fade quantised to the next bar and a fade halfway
    /// through are both *a request that has not arrived*, which is exactly the
    /// relation P-0075 is about. What separates them on the surface is that
    /// the value under the knob is moving in the second case, and the mark
    /// stands still in both.
    ///
    /// **One per control, because the engine allows one**: `Deck::schedule`
    /// cancels whatever was moving that pair before pushing, so a harness that
    /// finds two has read the wrong slot.
    ///
    /// # What it cannot say, and it is the trim's existing gap
    ///
    /// The track shows `[0, 1]` and the mix is HDR, so a move from 1.5 to 2.0
    /// is two values the track draws in one place — see [`StripBox::trim_at`],
    /// where that gap is already written down. [`Strip::gain_pending`] answers
    /// `None` there rather than declaring a staleness for an animation that
    /// would not move a pixel.
    pub gain_to: Option<f32>,
    /// **The fader** — `Deck::opacity`: a proportion in `[0, 1]`, and the one
    /// control that silences a slot under every blend mode, which is what
    /// makes it the way out of material that has gone NaN.
    pub opacity: f32,
    /// **Where a scheduled move is taking the fader**, or `None` for a fader
    /// nothing is moving — `Deck::transitions_on(slot)`'s `Control::Opacity`
    /// entry, on [`Strip::gain_to`]'s terms and for its reasons.
    ///
    /// # And the third control has nowhere to say it
    ///
    /// `Control` has three members and this strip carries two of them.
    /// `Control::MaskPosition` — *how far a mask's front has travelled* — is
    /// the one a wipe is made of, and **the strip has no control and no
    /// readout for it at all**: the mask `.mini` names a *shape*, and the
    /// position, the angle and the softness are an inspector row that does not
    /// exist yet ([`Strip::mask`] says so, and [`Strip::mask_angle`] is the
    /// same fact from the other side). A destination carried here for it would
    /// be a number with nowhere to be drawn, and drawing it on the shape chip
    /// would say a shape was changing when none is.
    ///
    /// So an armed wipe is invisible on this panel, it is an under-draw rather
    /// than a decision that it does not matter, and it is
    /// [ADR-0206](../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)'s
    /// named consequence: what closes it is a mask-position control, and it
    /// closes the same day that control lands.
    pub opacity_to: Option<f32>,
    /// **The blend in force**, as one of the vocabulary's three — and the word
    /// drawn on the chip is [`BlendMode::name`].
    ///
    /// # Why this is not the engine's word
    ///
    /// It was a `&'static str` — `Blend::name`, handed straight through, on
    /// the argument that the console has nothing to do with the blend but draw
    /// the engine's word. **That stopped being true when the chip became a
    /// control** ([ADR-0187](../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md)):
    /// to say what the *next* mode is, [`Mixer::blend`] has to know which of
    /// the three this one is, and a string it cannot exhaustively match is the
    /// wrong carrier for that. A `&str` would make the chip's arithmetic a
    /// comparison against three literals with a fourth case that has no
    /// answer.
    ///
    /// **So the type is the vocabulary's, and the harness converts.**
    /// `karakuri-console` already depends on `karakuri-operation`, and
    /// `crates/karakuri/src/main.rs` turns `Deck::blend`'s `karakuri_engine::deck::Blend`
    /// into one of these with a `match`. The failure that buys is worth
    /// stating: a fourth engine blend mode with no operation variant stops
    /// compiling **at the harness**, rather than drawing a word on a chip no
    /// control can reach and no map can ask for. That is the vocabulary doing
    /// its job — P-0074's *"the two rules … are not jointly satisfiable unless
    /// the vocabulary owns the lists."*
    ///
    /// The mock's own tooltip lists four — *"add, over, screen, multiply"* —
    /// and there are three. The disagreement was the mock's to settle and
    /// [ADR-0187](../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md)
    /// settled it: `docs/manual/console.html` now lists the three that exist.
    pub blend: BlendMode,
    /// The mask in force — `Deck::mask(slot).kind()`. Its angle, position and
    /// softness are not drawn: `.mini` is a chip that says *which shape*, and
    /// three numbers about that shape are the inspector's row, not this one.
    ///
    /// **What a press on the chip counts from** ([`Mixer::mask`]), the way
    /// [`Strip::requested`] is what the tally's press counts from.
    pub mask: Mask,
    /// **The angle the mask is already wearing** — `Deck::mask(slot).angle()`,
    /// in radians. **Read to build an operation, and drawn nowhere.**
    ///
    /// # Why the strip carries a number no part of it paints
    ///
    /// [`Operation::SetMaskShape`] carries a shape **and an angle**, because
    /// the vocabulary's row is one an operator can say the whole of and a
    /// record is written whole
    /// ([ADR-0201](../../../docs/adr/0201-the-mask-is-two-rows-because-a-control-change-can-only-set.md)).
    /// The chip names the shape and nothing on this strip names the angle — so
    /// a press has to carry a value the control does not control, and the only
    /// honest one is **the value it already has**. Sending `0.0` would make
    /// choosing a shape silently straighten a diagonal wipe: a press that
    /// changed something nobody asked it to, which is the class of failure
    /// [ADR-0192](../../../docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)
    /// and ADR-0201 are both about, arriving one layer further out.
    ///
    /// **It is [`Strip::requested`]'s arrangement exactly**: a value the strip
    /// carries because a press has to be computed from it, written by whoever
    /// owns the deck like every other field here, and never a value this crate
    /// keeps. See
    /// [ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md).
    ///
    /// **Not the position and not the softness**, which the record also
    /// carries: neither is in the operation at all, and the half a shape
    /// operation does not ask for is filled in from a reading of the mask that
    /// is running, where the record is written — `karakuri_operation_record`'s
    /// `Current`. The angle is here because the *operation* names it; those
    /// two are not, because it does not.
    pub mask_angle: f32,
    /// **What the slot's meter last read**, or `None` for no reading at all.
    ///
    /// `Deck::level` is already `None` for *no meter, no measurement yet, a
    /// slot that is neither Live nor being auditioned, and a slot whose
    /// material was just replaced by a resize or a swap* — every case where a
    /// held reading would be about a different image. So `None` here draws the
    /// meter's well and nothing in it, which is the mock's own `alloc` strip:
    /// a `.vmeter` with no `b` and no `u` inside it.
    pub level: Option<Level>,
}

impl Strip {
    /// **Where this slot has been asked to go and has not got to**, or `None`
    /// for a slot that is where it was asked to be.
    ///
    /// The one derivation the pending presentation reads, and it answers a
    /// *word* rather than a `bool` because that is what the surface has to
    /// draw: P-0075's second clause is that the destination is identifiable
    /// from the surface itself, so the thing worth deriving is the destination
    /// and not the fact that there is one.
    ///
    /// # Why it is an inequality and not the engine's pair
    ///
    /// `Deck::is_parked` is exactly `requested == Residency::Priming &&
    /// effective == Residency::Allocated`, and today this answers `Some` on
    /// precisely those slots — `deck.rs`'s module doc closes the other cases
    /// itself: *"the governor may hold a slot below what was asked for, and
    /// may never put one above it"*, and *"effective Live and requested Live
    /// are the same set of slots"*. So the two forms agree slot for slot, and
    /// there is no frame on which they differ.
    ///
    /// They differ in what a **second** kind of disagreement would do to them.
    /// Written as the engine's pair, a surface matching `Priming` over
    /// `Allocated` draws a settled chip over any other outstanding request —
    /// an under-draw, silent, and exactly the failure P-0075 exists to name.
    /// Written as an inequality it draws the new one without being taught,
    /// because the presentation was never about *parked*: it is about a
    /// request that has not landed, which is
    /// [P-0060](../../../docs/principles/0060-name-the-property-not-the-shape.md)
    /// — the property rather than the one shape it currently takes.
    ///
    /// **`park` is still the word**, and it is the status line's: `karakuri-cli`
    /// spells it out for an operator in prose, which is the same clause met by
    /// a different means (ADR-0188).
    pub fn pending(&self) -> Option<Tally> {
        match self.requested == self.tally {
            true => None,
            false => Some(self.requested),
        }
    }

    /// **Where the trim is going and has not got to**, or `None` for a trim
    /// that is where it has been asked to be.
    ///
    /// [`Strip::pending`] one control along, and the same shape: the field is
    /// the request, the derivation is the third clause, and nothing is stored
    /// that says *a move is pending* — that fact is `self.gain_to` against
    /// `self.gain`, read every frame off values the harness rewrote this
    /// frame.
    ///
    /// # It compares what the track can show, and that is not the same as the
    /// raw pair
    ///
    /// The trim draws `[0, 1]` and gain is open above unity, so a scheduled
    /// move from 1.5 to 2.0 is two values with **one position** on this track
    /// ([`StripBox::trim_at`]). Compared raw it is pending; compared through
    /// [`unit`] it is not, and this compares through [`unit`].
    ///
    /// The reason is [`View::animating`] rather than the mark: a `Some` here
    /// declares a staleness, and a staleness bought for an animation that
    /// cannot move a pixel is the cost ADR-0193 refused to pay for a bay
    /// nobody can see, arriving through the other door. **It is still an
    /// under-draw and it is the trim's existing gap, not a new one** — the
    /// same track that cannot show 1.0 against 3.0 cannot show a move between
    /// them, and both close together, on the pass that lets a hand push the
    /// control past the top.
    ///
    /// A destination that is not a number is zero, which is [`unit`]'s rule
    /// and is where a NaN out of an engine stops being a `NaN`-wide band.
    pub fn gain_pending(&self) -> Option<f32> {
        self.gain_to
            .filter(|to| unit(*to) != unit(self.gain))
            .map(unit)
    }

    /// **Where the fader is going and has not got to**, on
    /// [`Strip::gain_pending`]'s terms.
    ///
    /// Opacity is a proportion, so the clamp is the identity on every value a
    /// deck can hold and the comparison is the plain one. It is written
    /// through [`unit`] anyway, because a harness writing a destination the
    /// engine never held is the case both of these are the last line of
    /// defence for — and two derivations that agree except in the case nobody
    /// tests are worse than one shape written twice.
    pub fn opacity_pending(&self) -> Option<f32> {
        self.opacity_to
            .filter(|to| unit(*to) != unit(self.opacity))
            .map(unit)
    }
}

/// **A fader, laid out**: the track, the length of it the value fills, and the
/// knob sitting on the value.
///
/// One type and one derivation for both of a strip's faders — see [`fader`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fader {
    /// The well: `.fader`'s 5px capsule lying down, or `.vfader`'s 17px one
    /// standing up.
    pub track: Rect,
    /// Which way it runs. [`Axis::Row`] fills from the left and
    /// [`Axis::Column`] fills from the **bottom**, because a fader stands up.
    pub axis: Axis,
    /// What the value fills, from the track's own zero.
    pub fill: Rect,
    /// The knob, centred on the fill's moving edge.
    pub knob: Rect,
    /// **How far the knob's centre moves between the two ends**: the track
    /// less the inset the fill sits inside it by, along the axis.
    ///
    /// A field, and not a fill stored beside its track — the thing
    /// [`StripBox`] refuses. The fill says where the value *is* and says
    /// nothing about how far it could go; the zero end is recoverable from the
    /// fill (a row's `fill.min.x`, a column's `fill.max.y`, neither of which
    /// moves) and the length is not recoverable from anything here. It is what
    /// turns a pointer back into a value — see [`Mixer::grab`] — so the
    /// derivation that draws the fader and the one that grabs it are one.
    pub travel: f32,
}

/// **A meter, laid out**: the well, the column the mean fills, and the peak's
/// mark.
///
/// # It is not a [`Fader`], and the difference is not the handle alone
///
/// They share exactly one thing and it is [`filled`] — a value turned into a
/// length up a track — which is why that is a function of its own with three
/// call sites rather than a shared type. What they do not share is what each
/// **is**: a fader's knob is a grab target and [`Mixer::grab`] is what makes
/// it one, while a peak mark is a reading and never will be. A shared type
/// would be a type half of whose fields are about a gesture the other half can
/// never have — and [`Fader::travel`], which is the length a *hand* moves the
/// value along, is the field that says so.
///
/// They disagree about the track as well. `.vfader b` sits
/// [`size::VFADER_INSET`] inside its well and `.vmeter b` fills its own edge
/// to edge; `.vfader`'s fill is a capsule and `.vmeter` is `overflow: hidden`
/// around a square column; and a knob is meant to stand proud of its track
/// where nothing in a meter may leave the well.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Meter {
    /// `.vmeter`'s 6px capsule.
    pub well: Rect,
    /// [`Level::mean`], up from the bottom.
    pub fill: Rect,
    /// [`Level::peak`], a [`size::VMETER_PEAK_H`] bar across the well.
    pub peak: Rect,
}

/// **A scheduled move on a fader, laid out**: the mark on the destination, and
/// how far this frame's attempt to get there has reached.
///
/// The presentation
/// [P-0075](../../../docs/principles/0075-a-pending-transition-shows-where-it-is-where-it-is-going-and-that-it-has-not-arrived.md)
/// is met with on a track, decided in
/// [ADR-0206](../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md):
/// **the knob and the fill go on saying where the control is, a mark says
/// where it is going, and the fill keeps setting off toward the mark and
/// falling back**. Nothing about the value moves, which is a stricter first
/// clause than the tally's roll manages — the word there is lifted off its
/// centre line and this is not.
///
/// # Two rectangles and no phase
///
/// It is a [`Fader`]'s neighbour and it is measured the same way: a pure
/// function of the two values and the displacement, with no clock and no
/// `Phase` in it — [`roll_at`] is applied by whoever is painting, exactly as
/// it is for the tally's word. A test asks for a reach at a displacement it
/// chose.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reach {
    /// **The destination**, as a hairline across the track at the position the
    /// knob would sit at if the move had landed.
    ///
    /// **As long as the knob is wide**, so it is the knob's own footprint
    /// reduced to a line — and so it always has the same two pixels of strip
    /// to be read against that the knob stands proud on
    /// ([`size::VFADER_KNOB_OUT`]), whatever the fill is doing underneath. A
    /// mark the width of the *track* would cross the fill's own lavender end
    /// in its own colour on a move down from the top.
    ///
    /// It is painted **over** the knob rather than under it. Where the two
    /// nearly coincide the knob would otherwise swallow it — 9 pixels of knob
    /// on a 98-pixel travel is every move under about a tenth of the fader,
    /// and a quarter of the trim — and a mark that disappears exactly when the
    /// control is nearly there says *arrived* at the one moment it must not.
    pub mark: Rect,
    /// **What the reach has covered at this displacement**: from the value's
    /// own edge toward [`Reach::mark`], and never as far as it — [`ROLL_REACH`]
    /// of the way at the top of the travel, nothing at rest.
    ///
    /// Across the fill's own width rather than the track's, because it is the
    /// fill reaching. Empty at rest, which is a rectangle with no area and
    /// nothing painted.
    pub band: Rect,
}

/// **One strip's furniture**: a rectangle for each of the six things stacked
/// in it, and the two tracks a value rides.
///
/// The fills and the knobs are **not** fields, because each is a function of a
/// value this already knows where to put — see [`StripBox::trim_at`],
/// [`StripBox::fader_at`] and [`StripBox::meter_at`]. A fill stored beside its
/// track is two statements about one number.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StripBox {
    /// `.strip` itself: the 9px well everything else is inside.
    pub rect: Rect,
    /// `.strip-name`, the full width of the strip's content box because the
    /// CSS says `width: 100%`.
    pub name: Rect,
    /// `.tally`'s capsule, as wide as **the widest** of the three residency
    /// words inside its padding — the same box whichever one it is showing,
    /// so the chip does not resize when the deck moves and does not resize
    /// under a word rolling through it. See [`mixer`], where it is measured.
    ///
    /// The word is centred in it ([`tally_into`]), and the capsule is centred
    /// in the strip, so widening the box does not move the word: it grows
    /// symmetrically around type that was already on the strip's centre line.
    ///
    /// **It is the chip a press acts on** and not only the box a word is
    /// painted into — [`Mixer::tally`] hit-tests exactly this rectangle, the
    /// way [`StripBox::blend`] is hit-tested. Being the widest word's width
    /// rather than the shown word's is what the blend chip cannot say: this
    /// target stands still while the deck moves under it and while a word
    /// rolls through it.
    pub tally: Rect,
    /// The `g` in `.trim`.
    pub trim_label: Rect,
    /// `.trim`'s `.fader`: the horizontal track, [`size::FADER_H`] tall.
    pub trim: Rect,
    /// `.vfader`: the tall track, [`size::FADER_COL_H`] high.
    pub fader: Rect,
    /// `.vmeter`, beside it.
    pub meter: Rect,
    /// `.strip-num`: the opacity as a number.
    pub num: Rect,
    /// The blend `.mini`, which is **the chip a press acts on** and not only
    /// the box a word is painted into — [`Mixer::blend`] hit-tests exactly
    /// this rectangle. As wide as the word in it, inside `.mini`'s padding and
    /// border.
    pub blend: Rect,
    /// The mask `.mini`, which is **the chip a press acts on** and not only
    /// the box a mark is drawn into — [`Mixer::mask`] hit-tests exactly this
    /// rectangle, the way [`StripBox::blend`] and [`StripBox::tally`] are.
    /// It holds a mark rather than a word, so it is the same width whichever
    /// shape it is showing: a target that stands still while the deck moves
    /// under it, which the tally's capsule buys by being the widest word's and
    /// the blend chip cannot say at all.
    pub mask: Rect,
}

impl StripBox {
    /// **The trim at a gain**, which is [`fader`] on the horizontal track.
    ///
    /// The gain is shown over `[0, 1]`, and that range is read off
    /// `karakuri-midi`'s `GAIN_RANGE` rather than chosen here: *"`[0, 1]` for
    /// gain even though the mix is HDR and values above 1.0 are ordinary …
    /// a fader whose top is unity is what a fader means."* So a gain pushed
    /// past unity fills the track and stops, and this bay cannot yet show the
    /// difference between 1.0 and 3.0 — a real gap, and it belongs with the
    /// pass that lets a hand push the control past the top.
    pub fn trim_at(&self, gain: f32) -> Fader {
        fader(
            self.trim,
            Axis::Row,
            gain,
            0.0,
            egui::vec2(size::FADER_KNOB_W, size::FADER_KNOB_H),
        )
    }

    /// **The fader at an opacity**, which is [`fader`] on the vertical track.
    pub fn fader_at(&self, opacity: f32) -> Fader {
        fader(
            self.fader,
            Axis::Column,
            opacity,
            size::VFADER_INSET,
            egui::vec2(size::VFADER_KNOB_W, size::VFADER_KNOB_H),
        )
    }

    /// **The trim's scheduled move**, from the gain it is at to the one it was
    /// asked for, `rolled` of the way — which is [`reach`] on the horizontal
    /// track.
    ///
    /// Off [`StripBox::trim_at`] twice rather than off any arithmetic of its
    /// own: the destination's mark is *where the knob would be*, so it is the
    /// same derivation asked a second question, and a mark that drifted from
    /// the knob it stands for would be a fader with two ideas of what a value
    /// looks like.
    pub fn trim_reach(&self, gain: f32, to: f32, rolled: f32) -> Reach {
        reach(self.trim_at(gain), self.trim_at(to), rolled)
    }

    /// **The fader's scheduled move**, on [`StripBox::trim_reach`]'s terms and
    /// off [`StripBox::fader_at`].
    pub fn fader_reach(&self, opacity: f32, to: f32, rolled: f32) -> Reach {
        reach(self.fader_at(opacity), self.fader_at(to), rolled)
    }

    /// **The meter at a reading.**
    pub fn meter_at(&self, level: Level) -> Meter {
        // **The peak mark stays inside the well.** `.vmeter u` is placed by
        // its own `bottom`, so a 2px bar at a peak of 1.0 would sit from the
        // top of the well to two pixels above it — and `.vmeter` is
        // `overflow: hidden`, which clips it away exactly at the reading that
        // matters most. Its travel is the well's height less its own, which is
        // the clamp `Transport::beat` makes on the far end of a grid.
        let travel = (self.meter.height() - size::VMETER_PEAK_H).max(0.0);
        let top = self.meter.max.y - size::VMETER_PEAK_H - travel * unit(level.peak);
        Meter {
            well: self.meter,
            fill: filled(self.meter, Axis::Column, level.mean),
            peak: Rect::from_min_size(
                Pos2::new(self.meter.min.x, top),
                egui::vec2(self.meter.width(), size::VMETER_PEAK_H),
            ),
        }
    }
}

/// **The mixer's strips, laid out**: the values, and where each one goes.
///
/// # It borrows the values rather than carrying a copy
///
/// [`TransportRow`] carries the [`Transport`] it was measured from, for
/// [`Picture`]'s reason: whoever measured the type and whoever paints it are
/// then one statement, so a row laid out for one value and painted with
/// another cannot be written by accident. A [`Strip`] carries a name, so it is
/// not `Copy` and a copy per frame would be a `String` allocated per strip per
/// frame. The borrow says the same thing for nothing — these boxes were
/// measured from *these* strips — and the compiler holds it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mixer<'a> {
    /// **The values these rectangles were measured from**, in slot order.
    pub strips: &'a [Strip],
    /// Where each of them is. `None` past the last strip: a track on the page
    /// no deck fills, which is drawn as nothing at all.
    boxes: [Option<StripBox>; DECKS],
}

impl<'a> Mixer<'a> {
    /// How many strips there are — the deck's slot count, as far as one page
    /// of this bay reaches.
    pub fn count(&self) -> usize {
        self.boxes.iter().filter(|at| at.is_some()).count()
    }

    /// One strip's furniture. Panics on a strip this bay has not got, which is
    /// a caller having invented a slot — the rule [`TransportRow::dot`] states
    /// about a dot of a grid.
    pub fn strip(&self, index: usize) -> StripBox {
        let count = self.count();
        self.boxes
            .get(index)
            .copied()
            .flatten()
            .unwrap_or_else(|| panic!("strip {index} of a mixer of {count}"))
    }

    /// **What a press at `p` takes hold of**, or `None` where there is nothing
    /// under it that a hand can move.
    ///
    /// # It is the knob, and the track is deliberately not a target
    ///
    /// **A press on the track, off the knob, does nothing.** A fader at 0.3
    /// whose top is clicked would jump to 1.0 — a change to the mix nobody
    /// asked for, made on stage — and this bay's controls are played during a
    /// performance. So the answer is `None` there, and it is a decision rather
    /// than a hit test that stops at the knob by accident.
    ///
    /// It also decides who the *event* belongs to, since
    /// [`crate::input::claim`]'s rule 3 asks this: **the panel claims what it
    /// acts on**, so a press on a track goes to `egui`, which owns no widget
    /// there and does nothing with it — which is the same nothing, arrived at
    /// without the panel claiming a press it would throw away.
    ///
    /// # One derivation, asked twice
    ///
    /// [`crate::input::claim`] asks this and so does the caller that acts on
    /// the press, exactly as [`Outputs::op`] is asked after [`Outputs::hit`]
    /// (ADR-0176). Two copies of *where the knob is* is a control drawn where
    /// it cannot be grabbed, with nothing on screen saying so.
    ///
    /// **The value is part of the geometry here**, which the Outputs chip does
    /// not have to deal with: the knob sits on the fill's moving edge, so
    /// where it is depends on what the deck said this frame. That is the same
    /// [`Strip`] the bay was laid out from, so the knob a hand sees and the
    /// knob it grabs are the same one.
    pub fn grab(&self, p: karakuri_layout::Point) -> Option<Grab> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (strip, at))| {
                let at = at?;
                // The manual's *deck* is the code's *slot*, and a deck holds
                // `MAX_SLOTS` of them — `DECKS`, which is 4 — so the index is
                // a `u8` with room to spare. See `Knob::operation`.
                let deck = index as u8;
                grabbed(at.trim_at(strip.gain), deck, Knob::Trim, p)
                    .or_else(|| grabbed(at.fader_at(strip.opacity), deck, Knob::Fader, p))
            })
    }

    /// **What a press at `p` asks the blend to become**, or `None` where
    /// there is no blend chip under it.
    ///
    /// # The chip cycles, and the operation names where it arrived
    ///
    /// Click it and the deck's blend moves to the next of
    /// [`BlendMode::ALL`], wrapping from the last back to the first. What
    /// comes out is [`Operation::SetBlendMode`] naming the **destination** —
    /// never a step, because there is no step in the vocabulary to name.
    ///
    /// That is P-0074's own worked example rather than an exception to it:
    /// *"A toggle is an affordance, built over operations by whoever draws the
    /// control, and it belongs there … a mini that cycles the blend is one
    /// control emitting three. The operator sees a toggle; the vocabulary
    /// never does."* The cycle is [`after`], which is four lines in this file
    /// and nothing at all in `karakuri-operation`.
    ///
    /// **What a MIDI map is offered is the three values, not the cycle** —
    /// [ADR-0187](../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md),
    /// which is the decision this affordance forces and the reason it is
    /// recorded at all.
    ///
    /// # One derivation, asked twice, and the whole chip is the target
    ///
    /// [`crate::input::claim`]'s rule 3 asks this and so does the caller that
    /// acts on the press — the arrangement [`Outputs::op`] and [`Mixer::grab`]
    /// both have, where the derivation that claims a press is asked again
    /// rather than copied. [`StripBox::blend`] is the chip's own rectangle,
    /// the one the word is painted into, so a chip a hand sees and a chip it
    /// clicks are the same one.
    ///
    /// **The whole chip is the target and not the glyphs in it**, which is
    /// [`Outputs::sink`]'s rule one bay along: a 15px word is not something a
    /// hand finds, and `.mini`'s padding is what makes it one.
    ///
    /// # It names the strip's own deck
    ///
    /// The slot index, cast the way [`Mixer::grab`] casts it — the manual's
    /// *deck* is the code's *slot* (ADR-0180), and a deck holds `MAX_SLOTS` of
    /// them, so the index is a `u8` with room to spare.
    pub fn blend(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (strip, at))| {
                at.filter(|at| at.blend.contains(p))
                    .map(|_| Operation::SetBlendMode {
                        deck: index as u8,
                        blend: after(strip.blend),
                    })
            })
    }

    /// **What a press at `p` asks the residency to become**, or `None` where
    /// there is no tally chip under it.
    ///
    /// # The chip cycles, and it cycles from what was *requested*
    ///
    /// Click it and the deck is asked for the next of [`Tally::ALL`] —
    /// `live`, `prim`, `alloc`, wrapping — and what comes out is
    /// [`Operation::SetResidency`] naming that **destination**. The
    /// affordance is the blend chip's ([`Mixer::blend`], ADR-0187) and so is
    /// the division it rests on: the cycle is [`next`] here and nothing at all
    /// in `karakuri-operation`, which is P-0074's *"a toggle is an affordance,
    /// built over operations by whoever draws the control"*.
    ///
    /// **The step is taken from [`Strip::requested`] and not from
    /// [`Strip::tally`]**, and that is the decision rather than a detail. The
    /// two disagree exactly while a request has not landed, and cycling from
    /// the request is what makes that case come out right **with no case in
    /// the code for it**: a parked slot's request is `Priming`, so the next is
    /// `Allocated` — which *is* the withdrawal of the prime request, said by
    /// the ordinary arithmetic. Cycling from the effective residency would
    /// answer `Live` there, and a press meant to take a request back would put
    /// the deck on air.
    ///
    /// That is what
    /// [P-0076](../../../docs/principles/0076-a-surface-owns-the-affordance-never-the-authority.md)
    /// permits a surface: *"a press on a control whose transition is pending
    /// may ask for the withdrawal"* — a control choosing which destination a
    /// press names, arrived at out of one rule rather than a branch, and
    /// **not** a lock. The chip refuses nothing; every request is handed over
    /// and the engine decides.
    ///
    /// It is also what `karakuri-cli`'s `w` already does: `toggle_priming`
    /// reads `Deck::requested_residency` to choose its direction, so a parked
    /// slot's `w` withdraws rather than re-asking. Two surfaces reading
    /// different halves of the pair would disagree about what a press means
    /// on exactly the slots where it matters.
    ///
    /// # One derivation, asked twice, and the whole chip is the target
    ///
    /// [`crate::input::claim`]'s rule 3 asks this and so does the caller that
    /// acts on the press — [`Outputs::op`], [`Mixer::grab`] and
    /// [`Mixer::blend`] are the same arrangement. [`StripBox::tally`] is the
    /// capsule the word is painted into, and it is **the widest of the three
    /// words whatever it is showing** ([`mixer`]), so this target does not
    /// move when the deck moves under it and does not move while a word is
    /// rolling through it — which the blend chip, sized to the word it shows,
    /// cannot say.
    ///
    /// # It names the strip's own deck
    ///
    /// The slot index, cast the way [`Mixer::grab`] and [`Mixer::blend`] cast
    /// it — the manual's *deck* is the code's *slot* (ADR-0180), and a deck
    /// holds `MAX_SLOTS` of them, so the index is a `u8` with room to spare.
    pub fn tally(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (strip, at))| {
                at.filter(|at| at.tally.contains(p))
                    .map(|_| Operation::SetResidency {
                        deck: index as u8,
                        residency: residency(next(strip.requested)),
                    })
            })
    }

    /// **What a press at `p` asks the mask's shape to become**, or `None`
    /// where there is no mask mini under it.
    ///
    /// # The chip cycles, and the operation names where it arrived
    ///
    /// Click it and the deck's mask moves to the next shape — none, linear,
    /// radial, wrapping — and what comes out is
    /// [`Operation::SetMaskShape`] naming that **destination**. The affordance
    /// is the blend chip's ([`Mixer::blend`], ADR-0187) and the tally's
    /// (ADR-0195), and so is the division under it: the cycle is
    /// [`next_shape`] here and nothing at all in `karakuri-operation`, which
    /// is P-0074's *"a toggle is an affordance, built over operations by
    /// whoever draws the control"*.
    ///
    /// **The order starts at `None`**, which is
    /// `karakuri_engine::deck::MaskKind::ALL`'s and is written down there:
    /// *"`None` first, because it is the default and a cycle should start
    /// where a slot starts."* This crate has no engine (ADR-0156), so the
    /// order is restated in [`next_shape`] rather than read from it.
    ///
    /// # It carries the angle it does not control, and that is the decision
    ///
    /// [`Operation::SetMaskShape`] is a shape **and an angle**, and this chip
    /// names only the shape — *"`.mini` is a chip that says which shape, and
    /// three numbers about that shape are the inspector's row, not this one."*
    /// So the angle handed back is [`Strip::mask_angle`], the one the slot is
    /// already wearing: a press chooses a shape and changes nothing else.
    /// Sending `0.0` would make choosing a shape straighten a diagonal front,
    /// which is a surface asking for a change nobody made — see
    /// [ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md).
    ///
    /// **The position and the softness are not here at all.** The operation
    /// does not name them, and the record that does is filled in from a
    /// reading of the running mask where the record is written (ADR-0201).
    /// A surface carrying a value no operation asks for would be this crate
    /// keeping half a deck.
    ///
    /// # One derivation, asked twice, and the whole chip is the target
    ///
    /// [`crate::input::claim`]'s rule 3 asks this and so does the caller that
    /// acts on the press — [`Outputs::op`], [`Mixer::grab`], [`Mixer::blend`]
    /// and [`Mixer::tally`] are the same arrangement. [`StripBox::mask`] is
    /// the chip's own rectangle, the one the mark is drawn into, and it is
    /// [`size::MINI_SIZE`] wide inside `.mini`'s padding whichever shape it is
    /// showing — so this target, like the tally's capsule and unlike the blend
    /// chip's, does not move under the value it draws.
    ///
    /// # It names the strip's own deck
    ///
    /// The slot index, cast the way [`Mixer::grab`], [`Mixer::blend`] and
    /// [`Mixer::tally`] cast it — the manual's *deck* is the code's *slot*
    /// (ADR-0180), and a deck holds `MAX_SLOTS` of them, so the index is a
    /// `u8` with room to spare.
    pub fn mask(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (strip, at))| {
                at.filter(|at| at.mask.contains(p))
                    .map(|_| Operation::SetMaskShape {
                        deck: index as u8,
                        kind: wipe_kind(next_shape(strip.mask)),
                        angle: strip.mask_angle,
                    })
            })
    }

    /// Every strip and its box, in slot order.
    pub fn placed(&self) -> impl Iterator<Item = (&'a Strip, StripBox)> {
        let boxes = self.boxes;
        self.strips
            .iter()
            .zip(boxes)
            .filter_map(|(strip, at)| at.map(|at| (strip, at)))
    }
}

/// **The Mixer bay's strips, derived**: one per slot the deck has, and none at
/// all where there is no deck.
///
/// # The page has [`DECKS`] tracks whatever the deck holds
///
/// `.mixer-strips` is `grid-template-columns: repeat(4, 1fr)`, and that four
/// is the same four [`DECKS`] is — `MAX_SLOTS` is 4, so no deck can fill a
/// fifth. **A strip is one of four tracks wide even where there is one
/// strip**, and that is read off the arrangement rather than chosen here: the
/// right pane's minimum width is written as *"four mixer strips still side by
/// side … four of them with three 4px gaps inside `.mixer-strips`' 6 + 6 is
/// 172"*. Tracks that followed the strip *count* would make a one-slot deck's
/// strip 244 wide in a pane sized for four 61-wide ones, and would re-derive
/// that minimum every time a slot was installed.
///
/// So a track with no strip in it **draws nothing at all** — not an empty
/// strip. That is the opposite of what [`preview`] does with a cell that is
/// off, and the two are not in tension: a preview cell is the region's
/// own face and says `off`, where a strip is six readings and an empty one is
/// six readings nobody took. The mock draws such a strip — `.strip.empty`,
/// with `—` for a name, `empty` for a tally and both tracks bare — and
/// inventing one is ADR-0177's row of zeroes with a different glyph.
///
/// **The example's deck has one slot, so it draws one strip**, and that is the
/// example rather than a gap in it — exactly as three of its preview cells
/// read `off`.
///
/// # What is in the mock's bay and is deliberately not here
///
/// - **The head's `4 of 4 · page 1`.** *"The strip is a paged list whose
///   length is a number, and the header says which page you are on"* — so both
///   halves of that pill are about paging, and there is none. Every strip the
///   deck has is drawn, on the one page, so the pill could only ever read
///   `n of n · page 1`: two numbers that are always equal and a third that is
///   always 1. That is [`Kind::Bay`]'s own rule about a pill stating a value
///   the console does not have, and the number that *is* known — how many
///   strips there are — is on the face of the bay already. It arrives with
///   paging.
/// - **`.xfade`, the whole crossfade row under the strips**: an A/B track with
///   a handle on it, and `wipe`, `iris`, `next bar`, `8 beats` and `go`. That
///   is a transition being armed and then fired, and every one of them is a
///   control over machinery the console cannot reach — nothing here holds what
///   is armed, and `go` would fire nothing. It is 61 of the bay's 316 and it
///   stays empty.
/// - **`.strip.focus` and `.wfocus`, which are the two focuses**: the deck
///   selection, which persists and is what a key press is addressed to, and
///   keyboard focus, which is transient. The mock draws them differently on
///   purpose — a solid lavender ring and a dashed sun outline — because
///   *"drawing them the same erases which of the two a reader is looking at"*.
///   **Neither exists in this console**: nothing here selects a deck and
///   nothing takes keyboard focus, so a ring would be drawn around a state
///   that is not kept.
/// - **Every tooltip.** Four of this bay's controls carry one, and a tooltip
///   needs `egui` to own a widget — the sentence [`outputs`] writes about a
///   control, one bay along.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one. `ctx` is asked for the type, because the tally's capsule and the
/// blend's mini are as wide as the words in them.
pub fn mixer<'a>(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    strips: &'a [Strip],
) -> Option<Mixer<'a>> {
    // **No deck behind the console, so there are no strips.** Every test in
    // this crate is here, and so is the whole of `cargo test -p
    // karakuri-console`. Drawing four empty strips would be inventing six
    // readings a slot; this is `View::picture`'s rule, one bay along.
    if strips.is_empty() {
        return None;
    }
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // `outputs` and `transport`.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let region = to_egui(layout.rect(layout.find("mixer")?));
    let row = strips_row(region)?;
    let width = |job: LayoutJob| ctx.fonts_mut(|f| f.layout_job(job).size().x);
    let label = width(span_at(
        TRIM_LABEL,
        size::TRIM_LABEL_SIZE,
        Color32::PLACEHOLDER,
    ));
    // **The chip is as wide as the widest residency, not as the one it is
    // showing.** Measured once for the bay rather than per strip: it is the
    // same three words in every strip, so a per-strip measurement would be
    // three galley layouts a strip for one answer.
    //
    // It read `strip.tally` alone until the tally learned to say that a
    // request had not landed, and that was a defect rather than a
    // simplification: a chip sized to the current word is a capsule that
    // changes width when the deck moves under it, and one that changes width
    // *while a word is rolling through it* would be a control resizing on its
    // own animation. What the mock does not have is the reason it survived —
    // a still page draws each chip once, so shrink-to-fit and this are the
    // same picture there and only one of them is the same picture over time.
    let tally = Tally::ALL
        .into_iter()
        .map(|tally| width(tally_job(tally, Color32::PLACEHOLDER)))
        .fold(0.0f32, f32::max);
    let mut boxes = [None; DECKS];
    for (index, strip) in strips.iter().take(DECKS).enumerate() {
        let blend = width(span_at(
            strip.blend.name(),
            size::MINI_SIZE,
            Color32::PLACEHOLDER,
        ));
        boxes[index] = strip_box(
            track(row, DECKS, index, size::STRIP_GAP, Axis::Row),
            label,
            tally,
            blend,
        );
    }
    Some(Mixer { strips, boxes })
}

/// **Where the strips go**: the `.mixer-strips` grid inside a mixer region.
///
/// The region less [`size::HEAD_H`] for the bay head painted over the top of
/// it, inset by [`size::STRIPS_PAD`] left, right and top, and **exactly
/// [`size::STRIP_H`] tall** rather than whatever is left over. The bay is
/// taller than its strips by design — `.xfade` is the other 61 of it — and a
/// mixer with room to grow (it is the only visible child of a soloed right
/// pane, and a fixed child with room takes it, ADR-0157) grows the bay and not
/// the strips: a `.strip` is a column of fixed type around a `.fader-col`
/// whose 104 is stated in the CSS, so there is nothing in it that gets bigger
/// any more than there is anything that gets smaller.
///
/// `None` where the region cannot hold it — folded away, soloed away, or a
/// window too small — which is [`picture_rect`]'s rule stated on a row of
/// strips.
fn strips_row(region: Rect) -> Option<Rect> {
    let row = Rect::from_min_size(
        Pos2::new(
            region.min.x + size::STRIPS_PAD,
            region.min.y + size::HEAD_H + size::STRIPS_PAD,
        ),
        egui::vec2(region.width() - size::STRIPS_PAD * 2.0, size::STRIP_H),
    );
    match row.width() > 0.0 && region.contains_rect(row) {
        true => Some(row),
        false => None,
    }
}

/// The arithmetic of one strip, away from the type it measures and the layout
/// it reads.
///
/// Term for term from `.strip` and what is in it, in `style.css`:
///
/// - `.strip { display: flex; flex-direction: column; align-items: center;
///   gap: 5px; padding: 7px 4px }` — six things stacked from the top of the
///   strip's content box, one [`size::STRIP_GAP_Y`] between each pair, each
///   **centred across the strip** rather than filling it. Two are the
///   exception, and the CSS states both: `.strip-name` and `.trim` are
///   `width: 100%`.
/// - `.trim { gap: 5px; padding: 0 3px }` with `.trim .fader { flex: 1 }` —
///   the `g`, then the track, which takes what is left and is centred in the
///   row because the label is the taller of the two.
/// - `.fader-col { display: flex; gap: 6px; height: 104px }` around a
///   `.vfader` of 17 and a `.vmeter` of 6, both `height: 100%` — so the column
///   is 29 wide, centred, and its `align-items: flex-end` has nothing left to
///   align.
/// - `.strip-mode { gap: 3px }` — two minis, centred.
///
/// `None` where the track is too narrow to hold what is in it, which is
/// [`picture_rect`]'s rule stated on a strip: the fader column is the widest
/// fixed thing in it, and a strip that cannot hold that has no readings to
/// show. At the right pane's own minimum of 172 a track is exactly 37 and the
/// column is exactly 29 inside 4 + 4 of padding, so the arrangement's minimum
/// and this are one number or neither.
fn strip_box(track: Rect, label_w: f32, tally_w: f32, blend_w: f32) -> Option<StripBox> {
    let inner = Rect::from_min_max(
        Pos2::new(
            track.min.x + size::STRIP_PAD_X,
            track.min.y + size::STRIP_PAD_Y,
        ),
        Pos2::new(
            track.max.x - size::STRIP_PAD_X,
            track.max.y - size::STRIP_PAD_Y,
        ),
    );
    let column_w = size::VFADER_W + size::FADER_COL_GAP + size::VMETER_W;
    if inner.width() < column_w {
        return None;
    }
    // Each row starts one gap after the one before it ended, and is as tall as
    // its own type — which is what a flex column is.
    let mut y = inner.min.y;
    let mut row = |h: f32| {
        let at = Rect::from_min_size(Pos2::new(inner.min.x, y), egui::vec2(inner.width(), h));
        y = at.max.y + size::STRIP_GAP_Y;
        at
    };
    let name = row(size::STRIP_NAME_SIZE * size::LINE);
    let tally = centred_in(row(size::TALLY_H), tally_w + size::TALLY_PAD_X * 2.0);
    let trim = row(size::TRIM_H);
    let column = centred_in(row(size::FADER_COL_H), column_w);
    let num = row(size::STRIP_NUM_SIZE * size::LINE);
    let blend_w = blend_w + size::MINI_PAD_X * 2.0 + size::HAIRLINE * 2.0;
    // The mask's mini holds a mark rather than a word, and the mark is drawn
    // at the size the glyph it stands in for would have been — see `Mask`.
    let mask_w = size::MINI_SIZE + size::MINI_PAD_X * 2.0 + size::HAIRLINE * 2.0;
    let mode = centred_in(row(size::MINI_H), blend_w + size::MODE_GAP + mask_w);

    let trim_label = Rect::from_min_size(
        Pos2::new(trim.min.x + size::TRIM_PAD_X, trim.min.y),
        egui::vec2(label_w, trim.height()),
    );
    let trim_track = Rect::from_min_size(
        Pos2::new(
            trim_label.max.x + size::TRIM_GAP,
            trim.center().y - size::FADER_H * 0.5,
        ),
        egui::vec2(
            trim.max.x - size::TRIM_PAD_X - trim_label.max.x - size::TRIM_GAP,
            size::FADER_H,
        ),
    );
    match trim_track.width() > 0.0 {
        true => Some(StripBox {
            rect: track,
            name,
            tally,
            trim_label,
            trim: trim_track,
            fader: Rect::from_min_size(column.min, egui::vec2(size::VFADER_W, column.height())),
            meter: Rect::from_min_size(
                Pos2::new(column.max.x - size::VMETER_W, column.min.y),
                egui::vec2(size::VMETER_W, column.height()),
            ),
            num,
            blend: Rect::from_min_size(mode.min, egui::vec2(blend_w, mode.height())),
            mask: Rect::from_min_size(
                Pos2::new(mode.max.x - mask_w, mode.min.y),
                egui::vec2(mask_w, mode.height()),
            ),
        }),
        false => None,
    }
}

/// A box `w` wide centred across `row`, which is `align-items: center` on one
/// child of a flex column.
fn centred_in(row: Rect, w: f32) -> Rect {
    Rect::from_min_size(
        Pos2::new(row.center().x - w * 0.5, row.min.y),
        egui::vec2(w, row.height()),
    )
}

/// **How much of a track a value fills**, from the track's own zero — the left
/// end of a row, and the **bottom** of a column, because a fader stands up and
/// a meter fills from the floor.
///
/// **The whole of what a [`Fader`] and a [`Meter`] share**, and it is a
/// function rather than a shared type for the reason [`Meter`] gives. Three
/// call sites the day it is written: the trim's fill, the opacity fader's
/// fill, and the meter's column.
fn filled(track: Rect, axis: Axis, at: f32) -> Rect {
    let at = unit(at);
    match axis {
        Axis::Row => Rect::from_min_max(
            track.min,
            Pos2::new(track.min.x + track.width() * at, track.max.y),
        ),
        Axis::Column => Rect::from_min_max(
            Pos2::new(track.min.x, track.max.y - track.height() * at),
            track.max,
        ),
    }
}

/// **The fader, and it is one component with two call sites on the day it is
/// written**: the horizontal trim and the vertical opacity. That is this
/// repository's rule about an abstraction satisfied when it lands rather than
/// promised for later, which [`bay_head`] is the other instance of.
///
/// What the two disagree about is an argument each — the axis, the inset the
/// fill sits inside its track by (`.fader b` fills its 5px track edge to edge
/// where `.vfader b` is `left: 3px; right: 3px; bottom: 3px` inside its 17px
/// one), and the knob's size. Everything else is this.
///
/// **One number drives the fill and the knob**, which is what stops the two
/// disagreeing: the knob is centred on the fill's moving edge. The mock sets
/// them by hand and a few percent apart — `width: 72%` with `left: 66%`,
/// `height: 97%` with `bottom: 94%` — which is an author centring a 9px knob
/// on a fill's end in percentages, and this is the same mark with the
/// arithmetic done once.
///
/// So the knob overhangs its track by half its length at either end, and the
/// mock's own boxes have the room: [`size::STRIP_GAP_Y`] above and below the
/// fader column, and [`size::TRIM_PAD_X`] plus [`size::STRIP_PAD_X`] either
/// side of the trim.
fn fader(track: Rect, axis: Axis, at: f32, inset: f32, knob: egui::Vec2) -> Fader {
    let inside = track.shrink(inset);
    let fill = filled(inside, axis, at);
    let edge = match axis {
        Axis::Row => Pos2::new(fill.max.x, track.center().y),
        Axis::Column => Pos2::new(track.center().x, fill.min.y),
    };
    Fader {
        track,
        axis,
        fill,
        knob: Rect::from_center_size(edge, knob),
        travel: match axis {
            Axis::Row => inside.width(),
            Axis::Column => inside.height(),
        },
    }
}

/// **A scheduled move on a laid-out fader**: the mark on where it is going,
/// and the band reaching `rolled` of the way from where it is.
///
/// **Two faders in and no value arithmetic**, which is what keeps this honest:
/// `now` and `to` are the same track measured at the two values, so the mark
/// lands exactly where the knob would and the band starts exactly where the
/// fill ends. The knob's centre is the fill's moving edge — [`fader`] says so
/// — and that one number is the whole of what this reads out of each.
///
/// **The displacement is a fraction of the gap rather than a distance**, so
/// the reach is in proportion to the move: a fade across the fader sets off a
/// long way and a fade of a hundredth sets off a pixel. That is the honest
/// picture and it is also the limit — under about a hundredth of the travel
/// the whole disagreement is a pixel, and what carries the message there is
/// the mark rather than the motion. See
/// [ADR-0206](../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md).
///
/// Two call sites the day it is written, which is this repository's rule about
/// an abstraction: the trim and the opacity fader, exactly as [`fader`] itself
/// has.
fn reach(now: Fader, to: Fader, rolled: f32) -> Reach {
    let (from, dest) = (now.knob.center(), to.knob.center());
    match now.axis {
        Axis::Row => {
            let head = from.x + (dest.x - from.x) * rolled;
            Reach {
                mark: Rect::from_center_size(
                    Pos2::new(dest.x, now.track.center().y),
                    egui::vec2(size::HAIRLINE, now.knob.height()),
                ),
                band: Rect::from_min_max(
                    Pos2::new(from.x.min(head), now.fill.min.y),
                    Pos2::new(from.x.max(head), now.fill.max.y),
                ),
            }
        }
        Axis::Column => {
            let head = from.y + (dest.y - from.y) * rolled;
            Reach {
                mark: Rect::from_center_size(
                    Pos2::new(now.track.center().x, dest.y),
                    egui::vec2(now.knob.width(), size::HAIRLINE),
                ),
                band: Rect::from_min_max(
                    Pos2::new(now.fill.min.x, from.y.min(head)),
                    Pos2::new(now.fill.max.x, from.y.max(head)),
                ),
            }
        }
    }
}

/// **One laid-out fader, taken hold of at `p`** — or `None` where `p` is not
/// on its knob.
///
/// The inverse of [`fader`], off the same three numbers it laid out: the
/// track's zero end, which is the fill's fixed edge; the travel, which is the
/// length the value rides; and where the knob's centre is now, which is the
/// fill's moving edge. **The offset is the pointer less that centre**, so a
/// press keeps whatever it grabbed at and the value does not jump.
fn grabbed(fader: Fader, deck: u8, knob: Knob, p: Pos2) -> Option<Grab> {
    if !fader.knob.contains(p) {
        return None;
    }
    // Which end is zero is [`filled`]'s rule, read backwards: a row fills from
    // the left and a column from the **bottom**, because a fader stands up.
    let (zero, edge, coord) = match fader.axis {
        Axis::Row => (fader.fill.min.x, fader.fill.max.x, p.x),
        Axis::Column => (fader.fill.max.y, fader.fill.min.y, p.y),
    };
    Grab::new(deck, knob, fader.axis, zero, fader.travel, coord - edge)
}

/// **The next blend mode round the cycle**, wrapping from the last back to the
/// first — and the whole of the affordance the blend chip is.
///
/// It is four lines here and nothing at all in `karakuri-operation`, which is
/// P-0074's division: *"A toggle is an affordance, built over operations by
/// whoever draws the control, and it belongs there."* The vocabulary owns the
/// three values; this owns the order a pointer walks them in.
///
/// **A match rather than an index into [`BlendMode::ALL`]**, for the reason
/// [`BlendMode::name`] is one: a fourth mode does not compile until somebody
/// says what follows it. The cost is that the order is written twice — here
/// and in `ALL` — so `tests/blend.rs` walks `ALL` through this and asserts
/// they are the same cycle, which is the measurement that keeps the two from
/// drifting rather than a comment promising they will not.
fn after(blend: BlendMode) -> BlendMode {
    match blend {
        BlendMode::Add => BlendMode::Over,
        BlendMode::Over => BlendMode::Max,
        BlendMode::Max => BlendMode::Add,
    }
}

/// **The next residency round the cycle**, wrapping from the last back to the
/// first — the whole of the affordance the tally chip is, and the order the
/// mock's own tooltip lists: *"one of three residencies — live, priming,
/// allocated"*.
///
/// [`after`]'s division, one control along: the cycle is three lines here and
/// nothing in `karakuri-operation`, which owns the three values and not the
/// order a pointer walks them in (P-0074).
///
/// **A match rather than an index into [`Tally::ALL`]**, for [`after`]'s
/// reason: a fourth residency does not compile until somebody says what
/// follows it. The price is that the order is written twice — here and in
/// `ALL` — so `tests/tally.rs` walks `ALL` through this and asserts they are
/// the same cycle.
///
/// **What it is asked about is [`Strip::requested`]**, and why is
/// [`Mixer::tally`].
fn next(tally: Tally) -> Tally {
    match tally {
        Tally::Live => Tally::Priming,
        Tally::Priming => Tally::Allocated,
        Tally::Allocated => Tally::Live,
    }
}

/// **The console's word for a residency, as the vocabulary's** — and it is the
/// whole of what the console has to know about the difference.
///
/// [`Tally`] is the mock's `.tally` and the engine's `Residency` seen from the
/// surface; [`karakuri_operation::Residency`] is what an operation may name.
/// They are the same three states and two crates' words for them, so the
/// translation is a match — and a match rather than a cast so that a fourth
/// state on either side stops the build here, where the two lists meet, rather
/// than at a chip drawing a word no operation can carry.
///
/// The mirror image of it is `crates/karakuri/src/main.rs`'s `tally`, which turns the
/// *engine's* `Residency` into a [`Tally`] on the way in. Three names for
/// three states is the cost `karakuri-operation` pays for depending on nothing
/// (P-0074), and this is one of the two places it is paid.
fn residency(tally: Tally) -> Residency {
    match tally {
        Tally::Live => Residency::Live,
        Tally::Priming => Residency::Priming,
        Tally::Allocated => Residency::Allocated,
    }
}

/// **The next mask shape round the cycle**, wrapping from the last back to the
/// first — the whole of the affordance the mask mini is.
///
/// [`after`]'s division, one control along: the cycle is three lines here and
/// nothing in `karakuri-operation`, which owns the three shapes and not the
/// order a pointer walks them in (P-0074).
///
/// **A match, for [`after`]'s reason**: a fourth shape does not compile until
/// somebody says what follows it. Unlike [`after`] and [`next`] the price is
/// not a second copy of the order — [`Mask`] has no `ALL` and neither does
/// `karakuri_operation::WipeKind`, because nothing reads one (see [`Mask`]) —
/// so this is the only statement of the order in this crate, and
/// `tests/mask.rs` is what checks it against the order it is a copy of.
///
/// **It starts at [`Mask::None`]**, which is `karakuri_engine::deck::MaskKind::ALL`'s
/// own order and its own reason: *"`None` first, because it is the default and
/// a cycle should start where a slot starts."* A cycle that began at `Linear`
/// would be a control whose first press on a fresh slot moves it somewhere it
/// has never been.
fn next_shape(mask: Mask) -> Mask {
    match mask {
        Mask::None => Mask::Linear,
        Mask::Linear => Mask::Radial,
        Mask::Radial => Mask::None,
    }
}

/// **The console's word for a mask shape, as the vocabulary's** — [`residency`]
/// one control along, and for its reason exactly.
///
/// [`Mask`] is the mock's second `.mini` and the engine's `MaskKind` seen from
/// the surface; [`karakuri_operation::WipeKind`] is what an operation may name.
/// They are the same three shapes and two crates' words for them, so the
/// translation is a match — and a match rather than a cast so that a fourth
/// shape on either side stops the build here, where the two lists meet, rather
/// than at a chip drawing a mark no operation can carry.
///
/// The mirror image of it is `crates/karakuri/src/main.rs`'s reading of
/// `Deck::mask(slot).kind()`, which turns the *engine's* `MaskKind` into a
/// [`Mask`] on the way in.
fn wipe_kind(mask: Mask) -> WipeKind {
    match mask {
        Mask::None => WipeKind::None,
        Mask::Linear => WipeKind::Linear,
        Mask::Radial => WipeKind::Radial,
    }
}

/// The tally's word as one laid-out run, so that measuring it and painting it
/// cannot be two different runs of type.
fn tally_job(tally: Tally, colour: Color32) -> LayoutJob {
    spaced(
        &tally.word().to_uppercase(),
        size::TALLY_SIZE,
        colour,
        size::TALLY_TRACKING,
    )
}

/// A plain span at a size that is not the console's [`size::BASE`] — this bay
/// has four of them, which is why it takes one.
fn span_at(text: &str, size: f32, colour: Color32) -> LayoutJob {
    LayoutJob::simple_singleline(
        text.to_owned(),
        FontId::new(size, FontFamily::Proportional),
        colour,
    )
}

/// `.strip-name` as one laid-out run: `overflow: hidden; text-overflow:
/// ellipsis; white-space: nowrap` is exactly one row, broken anywhere, with an
/// ellipsis standing for what did not fit.
fn name_job(name: &str, width: f32, colour: Color32) -> LayoutJob {
    let mut job = span_at(name, size::STRIP_NAME_SIZE, colour);
    job.wrap = egui::epaint::text::TextWrapping {
        max_width: width,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    job
}

/// **The Mixer bay's strips, painted.**
///
/// Where everything goes is [`mixer`]'s, so this paints and derives nothing.
fn mixer_into(ui: &Ui, pal: &Palette, mixer: &Mixer, phase: Phase) {
    for (strip, at) in mixer.placed() {
        strip_into(ui, pal, strip, at, phase);
    }
}

/// **One strip**, term for term from `style.css`:
///
/// - `.strip` — `background: var(--c-well)`, `border-radius: 9px`.
/// - `.strip-name` — `font-size: 10px`, `var(--c-dim)`, centred, elided.
/// - `.tally` — three washes and three inks, which is [`tally_into`]'s.
/// - `.trim .lbl` — `font-size: 9px`, `var(--c-faint)`.
/// - `.fader` and `.vfader` — [`fader_into`]'s.
/// - `.vmeter` — [`meter_into`]'s.
/// - `.strip-num` — `var(--c-text)`, *a value*, and `font-weight: 500` is not
///   honoured because `egui`'s default proportional face has no bold.
/// - `.strip-mode` — two [`mini_into`]s.
fn strip_into(ui: &Ui, pal: &Palette, strip: &Strip, at: StripBox, phase: Phase) {
    // Clipped to the strip and half the gap around it: a fader's knob is
    // meant to stand proud of its track, and `.mixer-strips`' 4px gap is where
    // that goes — but nothing in one strip may reach the strip beside it.
    let painter = ui
        .painter()
        .with_clip_rect(at.rect.expand(size::STRIP_GAP * 0.5));
    painter.rect_filled(
        at.rect,
        CornerRadius::same(size::STRIP_RADIUS as u8),
        pal.well,
    );

    if !strip.name.is_empty() {
        let galley = painter.layout_job(name_job(&strip.name, at.name.width(), pal.dim));
        centre_galley(&painter, at.name, galley, pal.dim);
    }

    tally_into(&painter, pal, at.tally, strip.tally, strip.pending(), phase);

    let galley = painter.layout_job(span_at(TRIM_LABEL, size::TRIM_LABEL_SIZE, pal.faint));
    painter.galley(
        Pos2::new(
            at.trim_label.min.x,
            at.trim_label.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.faint,
    );
    // **The same displacement for both faders and for the chip above them**,
    // because it is the same phase: P-0075's *everything pending moves
    // together*, which on this strip is three presentations reading one
    // number. A strip with a fade on each fader reaches twice, in step.
    let rolled = roll_at(phase);
    fader_into(
        &painter,
        pal,
        at.trim_at(strip.gain),
        false,
        strip
            .gain_pending()
            .map(|to| at.trim_reach(strip.gain, to, rolled)),
    );
    fader_into(
        &painter,
        pal,
        at.fader_at(strip.opacity),
        strip.tally == Tally::Live,
        strip
            .opacity_pending()
            .map(|to| at.fader_reach(strip.opacity, to, rolled)),
    );
    meter_into(&painter, pal, at.meter, strip.level.map(|l| at.meter_at(l)));

    // `.strip-num`: the opacity to two places, which is what the mock writes.
    // The value itself and not the fader's clamped one — the fader clamps
    // because a track has ends and a number does not.
    let galley = painter.layout_job(span_at(
        &format!("{:.2}", strip.opacity),
        size::STRIP_NUM_SIZE,
        pal.text,
    ));
    centre_galley(&painter, at.num, galley, pal.text);

    mini_into(&painter, pal, at.blend, true, |painter, colour| {
        let galley = painter.layout_job(span_at(strip.blend.name(), size::MINI_SIZE, colour));
        centre_galley(painter, at.blend, galley, colour);
    });
    mini_into(&painter, pal, at.mask, false, |painter, colour| {
        mask_mark(painter, at.mask.center(), colour, strip.mask);
    });
}

/// A galley centred in a box, both ways — which is `align-items: center` on a
/// flex column done where the type's real height is known.
fn centre_galley(
    painter: &egui::Painter,
    rect: Rect,
    galley: std::sync::Arc<egui::Galley>,
    colour: Color32,
) {
    painter.galley(
        Pos2::new(
            rect.center().x - galley.size().x * 0.5,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        colour,
    );
}

/// `.tally`: a capsule with the residency's word in it, in the residency's own
/// colours.
///
/// - `.tally.live` — `color-mix(in srgb, var(--c-pink) 18%, transparent)`
///   behind `var(--c-pink)`, with `box-shadow: 0 0 10px var(--c-glowp)`. The
///   halo is an [`egui::epaint::Shadow`], the mechanism the transport's lit
///   beat and the Outputs row's dot both use.
/// - `.tally.priming` — a 20% wash of `var(--c-sun)` behind `var(--c-sun)`,
///   and **no halo**: priming is warming out of sight, not on air.
/// - `.tally.alloc` — `var(--c-tint)` behind `var(--c-dim)`, which is the
///   palette's own *"the wash behind a node head, and the allocated tally"* —
///   the first use of `--c-tint` in this crate, and it was transcribed against
///   this day.
///
/// # And the roll, where the request has not landed
///
/// `pending` is [`Strip::pending`] — the residency this slot was asked for and
/// has not reached. While it is `Some`, the word rolls part of the way toward
/// it and falls back, once a second, and never arrives: [`roll_at`] is the
/// displacement and [ADR-0190](../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md)
/// is why it is a roll and not two lamps side by side — 53 pixels, and the
/// pair the rule exists for is 85.125 of them.
///
/// # The clip is new, not narrowed
///
/// Nothing called `with_clip_rect` on a tally before this: the chip painted a
/// filled rect and a galley that fitted inside it, so there was nothing to
/// clip and no clip to get wrong. A second word travelling through the box is
/// the first thing here that is drawn to be cut off, and the cut is what makes
/// the roll a roll rather than two words overlapping the trim row underneath.
/// It is introduced deliberately and it is on the type alone — see the note at
/// the clip itself, because the halo has to go on spilling.
fn tally_into(
    painter: &egui::Painter,
    pal: &Palette,
    rect: Rect,
    tally: Tally,
    pending: Option<Tally>,
    phase: Phase,
) {
    let radius = CornerRadius::same((size::TALLY_H * 0.5) as u8);
    let ink = |tally| match tally {
        Tally::Live => (tint(pal.pink, 18), pal.pink),
        Tally::Priming => (tint(pal.sun, 20), pal.sun),
        Tally::Allocated => (pal.tint, pal.dim),
    };
    let (fill, ink_now) = ink(tally);
    if tally == Tally::Live {
        painter.add(
            egui::epaint::Shadow {
                offset: [0, 0],
                blur: size::TALLY_GLOW,
                spread: 0,
                color: pal.glow_pink,
            }
            .as_shape(rect, radius),
        );
    }
    painter.rect_filled(rect, radius, fill);

    // **The clip is the capsule, and it is on the words alone.** The halo is
    // drawn to spill — `.tally.live`'s `box-shadow: 0 0 10px` is 10px of it
    // outside the box — so a clip taken before the shadow would trim the one
    // shape in this chip that is meant to leave it. Everything after this
    // line is type that may be halfway out of the box on purpose.
    let painter = painter.with_clip_rect(rect.intersect(painter.clip_rect()));

    let galley = painter.layout_job(tally_job(tally, ink_now));
    // **The pitch is the travel's, not the geometry's**, and it is at least
    // the box: at the natural row pitch the two words would both be partly
    // visible with nothing between them, which at 9px is mud. One box height
    // apart leaves a blank band of exactly the slack the chip already has —
    // 13.5 less a 10.0 ink row is 3.5 — for every displacement, because the
    // band is the difference of two constants and not a function of how far
    // the roll has got. It costs the chip nothing: the second word is
    // outside the capsule at rest and clipped away.
    let pitch = size::TALLY_H.max(galley.size().y);
    let rolled = match pending {
        Some(_) => roll_at(phase) * pitch,
        None => 0.0,
    };
    let word_into = |painter: &egui::Painter, galley: std::sync::Arc<egui::Galley>, colour, dy| {
        painter.galley(
            Pos2::new(
                rect.center().x - galley.size().x * 0.5,
                rect.center().y - galley.size().y * 0.5 + dy,
            ),
            galley,
            colour,
        );
    };
    word_into(&painter, galley, ink_now, -rolled);
    // **The destination in its own ink**, which is the second half of what is
    // being said: the word names where the slot is going and the colour is the
    // one that slot will be drawn in when it gets there. It comes up from
    // below — one pitch under the settled word — so a still frame of a chip
    // that is not rolling is the chip as it was.
    if let Some(to) = pending {
        let (_, ink_to) = ink(to);
        let galley = painter.layout_job(tally_job(to, ink_to));
        word_into(&painter, galley, ink_to, pitch - rolled);
    }
}

/// A fader: the well, the fill and the knob.
///
/// - `.fader` / `.vfader` — `background: var(--c-well)` with
///   `box-shadow: inset 0 0 0 1px var(--c-hair)`, which is a 1px stroke on the
///   inside, exactly as a preview cell's is.
/// - `.fader b` / `.vfader b` — `linear-gradient(90deg, var(--c-mint),
///   var(--c-lav))` lying down and `linear-gradient(0deg, …)` standing up,
///   which is the same colour ramp the tempo carries and is drawn the same way
///   — see [`gradient`].
/// - `.fader s` / `.vfader s` — `background: var(--c-panel)` with
///   `box-shadow: 0 0 0 1px var(--c-line)`, which is a 1px stroke on the
///   **outside** because a `0 0 0 1px` shadow sits around the box rather than
///   inside it. Its `0 1px 4px rgba(0,0,0,0.18)` drop shadow is dropped for
///   the reason `room` collapses the day palette's pair: `epaint` draws one
///   shadow and the rim is the one that says where the knob is.
///
/// `live` is the one thing about a fader that is not the fader's:
/// `.strip.live .vfader s` is `box-shadow: 0 0 0 1px var(--c-pink),
/// 0 0 9px var(--c-glowp)`, so the knob of a slot that is on air takes a pink
/// rim and a pink halo and no other knob does. It is the mock saying *the
/// fader you are about to move is the one the audience is watching*.
fn fader_into(
    painter: &egui::Painter,
    pal: &Palette,
    fader: Fader,
    live: bool,
    reach: Option<Reach>,
) {
    let track_r = CornerRadius::same((fader.track.width().min(fader.track.height()) * 0.5) as u8);
    painter.rect_filled(fader.track, track_r, pal.well);
    painter.rect_stroke(
        fader.track,
        track_r,
        Stroke::new(size::HAIRLINE, pal.hair),
        StrokeKind::Inside,
    );
    gradient(painter, fader.fill, fader.axis, pal.mint, pal.lav, true);

    // **The band, over the fill and under the knob**: it is the fill setting
    // off, so it is drawn where the fill is drawn and the truth stays on top
    // of it. `.vfader em`'s 20% wash of `var(--c-lav)` — the mock's own ink
    // for *an address*, which is what a destination is — square rather than a
    // capsule, because it is a stretch of the track rather than a value's own
    // shape.
    //
    // **The mock draws it standing up only**, on deck B's fader: the trim's is
    // the same band lying down, and no strip in the mock has a gain fade armed
    // on it to draw one in.
    //
    // **Nothing at all while it rests**, which is where two thirds of an armed
    // strip's frames are: at a displacement of zero the band has no area, and
    // an empty rectangle painted every frame is a shape on the frame's budget
    // (ADR-0164) that draws nothing.
    if let Some(reach) = reach.filter(|reach| positive(reach.band)) {
        painter.rect_filled(reach.band, CornerRadius::ZERO, tint(pal.lav, 20));
    }

    let knob_r = CornerRadius::same((fader.knob.width().min(fader.knob.height()) * 0.5) as u8);
    if live {
        painter.add(
            egui::epaint::Shadow {
                offset: [0, 0],
                blur: size::VFADER_KNOB_GLOW,
                spread: 0,
                color: pal.glow_pink,
            }
            .as_shape(fader.knob, knob_r),
        );
    }
    painter.rect_filled(fader.knob, knob_r, pal.panel);
    painter.rect_stroke(
        fader.knob,
        knob_r,
        Stroke::new(
            size::HAIRLINE,
            match live {
                true => pal.pink,
                false => pal.line,
            },
        ),
        StrokeKind::Outside,
    );

    // **The mark last, over everything, and it is one pixel.** `.vfader i`'s
    // `background: var(--c-lav)`. It is the only thing here that may not be
    // lost: painted under the knob it would vanish inside it for every move
    // shorter than the knob is long, which is where a mark saying *not yet* is
    // needed most. A hairline over a 9px knob hides nothing of where the
    // control is.
    if let Some(reach) = reach {
        painter.rect_filled(reach.mark, CornerRadius::ZERO, pal.lav);
    }
}

/// The meter: the well, the mean's column and the peak's mark — and **nothing
/// in the well where there is no reading**, which is the mock's own `alloc`
/// strip.
///
/// - `.vmeter` — the fader's well, at 6 wide.
/// - `.vmeter b` — `linear-gradient(0deg, var(--c-mint), var(--c-sun))`, and
///   square rather than a capsule: `.vmeter` is `overflow: hidden` and the
///   column is clipped by the well rather than rounded itself.
/// - `.vmeter u` — `background: var(--c-pink)`, *on air*, which is the colour
///   the peak shares with the live tally and the lit beat.
fn meter_into(painter: &egui::Painter, pal: &Palette, well: Rect, meter: Option<Meter>) {
    let radius = CornerRadius::same((well.width() * 0.5) as u8);
    painter.rect_filled(well, radius, pal.well);
    painter.rect_stroke(
        well,
        radius,
        Stroke::new(size::HAIRLINE, pal.hair),
        StrokeKind::Inside,
    );
    let Some(meter) = meter else {
        return;
    };
    let painter = painter.with_clip_rect(meter.well);
    gradient(&painter, meter.fill, Axis::Column, pal.mint, pal.sun, false);
    painter.rect_filled(meter.peak, CornerRadius::ZERO, pal.pink);
}

/// One `.mini`: a capsule with a 1px border and whatever goes in it.
///
/// `sel` is `.mini.sel` — `color: var(--c-lav)`, `border-color: transparent`,
/// and a 15% wash of the same behind it. **The blend's mini is always `.sel`
/// and the mask's never is**, which is the mock's and reads: the blend names
/// which of `Blend::ALL` is in force, where the mask is a picker showing the
/// shape it is set to.
fn mini_into(
    painter: &egui::Painter,
    pal: &Palette,
    rect: Rect,
    sel: bool,
    contents: impl FnOnce(&egui::Painter, Color32),
) {
    let radius = CornerRadius::same((size::MINI_H * 0.5) as u8);
    match sel {
        true => {
            painter.rect_filled(rect, radius, tint(pal.lav, 15));
        }
        false => {
            painter.rect_stroke(
                rect,
                radius,
                Stroke::new(size::HAIRLINE, pal.line),
                StrokeKind::Inside,
            );
        }
    }
    contents(
        painter,
        match sel {
            true => pal.lav,
            false => pal.faint,
        },
    );
}

/// The mask's mark, drawn rather than typed — see [`Mask`].
///
/// A circle [`size::MINI_SIZE`] across, which is the size the glyph it stands
/// in for would have been, and then what the mask does to it: nothing, one
/// half filled, or a filled centre.
fn mask_mark(painter: &egui::Painter, centre: Pos2, colour: Color32, mask: Mask) {
    let r = size::MINI_SIZE * 0.5;
    match mask {
        Mask::None => {}
        // A half-disc is a circle with half of it not painted, so it is a clip
        // rather than a path: `epaint` has no arc and a polygon of one would
        // be a curve this file approximated by hand.
        Mask::Linear => {
            painter
                .with_clip_rect(Rect::from_min_max(
                    Pos2::new(centre.x, centre.y - r),
                    Pos2::new(centre.x + r, centre.y + r),
                ))
                .circle_filled(centre, r, colour);
        }
        Mask::Radial => {
            painter.circle_filled(centre, r * 0.5, colour);
        }
    }
    painter.circle_stroke(centre, r, Stroke::new(size::HAIRLINE, colour));
}

/// **A two-colour ramp along `axis`**, which is `linear-gradient` and a
/// painter that has none.
///
/// The same problem [`bpm_job`] answers for a line of type, and a different
/// answer because this is a shape rather than a run of glyphs: a `Mesh` of two
/// triangles with a colour at each corner, which `epaint` interpolates across
/// exactly. One shape, and no stepping.
///
/// `capsule` is `border-radius: 999px` on the fill: a fader's fill is a
/// capsule and its two ends are circles at the ramp's own ends, where a
/// meter's column is square and is clipped by the well around it instead.
/// Everything is clipped to `rect`, so an end cap on a fill shorter than it is
/// wide is a sliver rather than a bulge.
fn gradient(
    painter: &egui::Painter,
    rect: Rect,
    axis: Axis,
    from: Color32,
    to: Color32,
    capsule: bool,
) {
    if !(rect.width() > 0.0 && rect.height() > 0.0) {
        return;
    }
    let painter = painter.with_clip_rect(rect);
    // `90deg` runs left to right and `0deg` runs **up**, so a column's `from`
    // is at the bottom — which is also where its zero is.
    let colour_at = |p: Pos2| match axis {
        Axis::Row => match p.x <= rect.center().x {
            true => from,
            false => to,
        },
        Axis::Column => match p.y >= rect.center().y {
            true => from,
            false => to,
        },
    };
    let mut mesh = egui::epaint::Mesh::default();
    for corner in [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ] {
        mesh.colored_vertex(corner, colour_at(corner));
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));

    if capsule {
        let r = rect.width().min(rect.height()) * 0.5;
        let (start, end) = match axis {
            Axis::Row => (
                Pos2::new(rect.min.x + r, rect.center().y),
                Pos2::new(rect.max.x - r, rect.center().y),
            ),
            Axis::Column => (
                Pos2::new(rect.center().x, rect.max.y - r),
                Pos2::new(rect.center().x, rect.min.y + r),
            ),
        };
        painter.circle_filled(start, r, from);
        painter.circle_filled(end, r, to);
    }
}

/// `color-mix(in srgb, X n%, transparent)`, as the alpha it is: `n`% of 255,
/// rounded. The mock's own wash behind an armed control, a live tally and a
/// selected mini, and `room`'s documentation is where the equivalence is
/// argued.
fn tint(colour: Color32, percent: u8) -> Color32 {
    let alpha = ((percent as u32 * 255 + 50) / 100) as u8;
    Color32::from_rgba_unmultiplied(colour.r(), colour.g(), colour.b(), alpha)
}

// ---------------------------------------------------------------------------
// The Library bay
// ---------------------------------------------------------------------------

/// **The word at the head of the Library bay**, in the source's own
/// capitalisation for [`Kind::Bay`]'s reason: the mock upper-cases in CSS, and
/// that is done at paint time so the word a reader searches for is the word in
/// the source.
const LIBRARY_TITLE: &str = "Library";

/// **The Library bay, laid out**: where the rows go, how many of them there is
/// room for, and where the count under them goes.
///
/// # What the bay is standing on, and it is a listing rather than a walk
///
/// The manual: *"A scope and a walk, not one flat list: favourites, my sets,
/// app presets, a folder."* **One of those four exists** — `my sets`, which is
/// [`karakuri_store::Store::list_sets`](../../../crates/karakuri-store/src/store.rs)
/// and is a directory of Set files. So this draws that list and no scope
/// chooser above it, for the reason [`outputs`] draws one sink of four: a
/// control over machinery that does not exist is the scaffolding this module's
/// documentation refuses.
///
/// # What is in the mock's bay and is deliberately not here
///
/// - **The `.scopes` row** — `favourites`, `my sets`, `presets`, `folder` and
///   a `+`. Every chip is a control and three of the four name a collection
///   nothing in this workspace can produce. **What a folder scope reads is no
///   longer the open question it was**: `console.html`'s *A folder scope reads
///   Sets, and a bundle is not a third thing* settles that a folder row is a
///   **take** — a Set file and a bundle are one file, and a `.kir` is a part
///   rather than something a library lists. What it leaves is named there as
///   owed rather than drawn from: *"no operation in the vocabulary can ask a
///   folder for its listing"*, because `Operation::ListSets` carries what a
///   Set holds and has nowhere to put a directory. So the chip waits on an
///   operation and not on a decision. The `+` is the arena's own gap drawn a
///   fifth time, which [`outputs`] already names.
/// - **The `.path` row**, `~/sets/tour-2026/night-b › opening`. It is the
///   walk *inside* a folder scope, so it says nothing until that scope is
///   decided.
/// - **The `.lib-filters` fields**, `holds…` and `layer…`. Two text controls,
///   and the vocabulary has the operation they would emit —
///   `Operation::ListSets { holds, layer }` — but nothing in the store answers
///   it: `list_sets` reads names off a directory and no index anywhere says
///   what a Set *holds*.
/// - **`.lib-row`'s `.star`.** A favourite is a fact about a Set that nothing
///   in this workspace keeps — there is no such field on a `SetEntry`, no
///   record that carries one, and no metadata card that mentions one. Drawing
///   a hollow star on every row would assert that nothing is a favourite,
///   which is a reading nobody took. **`console.html`'s *What keeps a
///   favourite, and where it does not travel* decided where the value lives
///   without giving this anything to read**: beside the Sets, in the library
///   itself, so that a star does not travel with a Set file and the file stays
///   byte for byte what it was. That page says the rest itself — *"Nothing in
///   the vocabulary names a favourite, so there is nothing yet for a key, a
///   map or a model to reach — and a row invented from this drawing would be
///   the specification written backwards."*
/// - **`.lib-row .dim`, the time beside each name.** This one is different
///   from the others and is worth the sentence: the *value* exists —
///   `SetEntry::written` is the Set file's own mtime — and what does not exist
///   is a spelling for it. The one answer in this workspace is
///   `karakuri_environment::setfile::written_at`, local time to the second
///   where the mock's column is `16:09`. **It is reachable now** — ADR-0214
///   moved it out of a package with no library target, which is the reason
///   this comment used to give — but not from here: `karakuri-console` takes
///   no engine, no store and no environment by design, so the host formats it
///   and hands it in, the way every other derived value in this module
///   arrives. Writing a second spelling here would be the kind of second
///   answer this repository deletes rather than adds.
/// - **`.lib-row.cursor`, and the `load → A` pill in the foot.** These two are
///   the load route, and `console.html`'s *How a Set reaches a deck* has since
///   settled the question they used to be blocked on: *"what was missing was
///   never the operation but the route"*, and the route is the arrangement —
///   the cursor says which Set, the deck selection says which deck, so **a
///   load is *"a cursor and a key with no pointer anywhere in it"***. That is
///   why the pill is not a control here and would not become one: it is a
///   readout that says where a press lands *before* the press, and a drag from
///   a row onto a strip is named there as *"a second route to the same
///   command, and never the first"*. `tests/library.rs` is where that is held.
///
///   **What is missing is three things, and none of them is a drawing.** The
///   cursor is a selection this console does not keep — the same sentence
///   [`mixer`] writes about the deck selection, which ADR-0219 records as
///   living *"in the specification and not in `karakuri-console`'s code"*. The
///   pill's letter *is* that selection, so drawing one without it is inventing
///   a value, which is ADR-0177's row of zeroes a bullet above. And the press
///   has no key: `console.html` says outright *"The key is owed and this page
///   does not choose the letter"*, `docs/manual/operations.html` has `—` in
///   that row's key column, and a letter chosen from here would be the
///   specification written backwards (ADR-0198).
///
///   **Under all three, the operation has nothing to do.**
///   `Operation::LoadSet` is `Written::Silent(Silent::NoRecord)`, so a surface
///   that emitted it would have to perform it itself (ADR-0198) — and nothing
///   in this workspace loads a Set into a *running* deck, which is the row's
///   own sentence on the operations page and why it is marked `launch`.
///   `Deck::install` is the one function that puts a Set in a slot, and its
///   documentation is that it is *"deliberately not reachable from a key or a
///   surface"*; `--set` and `--load-set` are launch flags.
///
///   **None of it blocks the listing**: what is missing is a way to play from
///   this bay, not a way to draw it.
///
/// # The foot's number is the mock's own, read the mock's way
///
/// `5 of 27` is how many rows are listed against how many the scope holds, and
/// both halves are here: the total is what the harness handed over, and the
/// count is how many rows the bay had room for. So a library taller than its
/// list says so in the one place the mock puts it, and nothing scrolls —
/// which is honest, because there is no scroll position anywhere in this
/// crate and inventing one would be a control.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LibraryBay {
    /// `.lib-list`'s content box: the region under the bay head and above the
    /// foot, inside [`size::LIB_LIST_PAD`], where the rows are laid from the
    /// top with no gap between them.
    pub list: Rect,
    /// **How many rows are drawn**, which is how many fit in [`list`](Self::list) —
    /// never more than [`total`](Self::total), and never zero, because a bay
    /// with no room for one row is no bay at all.
    pub rows: usize,
    /// **How many Sets the store holds**, which is what the harness handed
    /// over. The second half of the foot's `n of m`.
    pub total: usize,
    /// `.lib-foot`, along the bottom edge of the bay, with its rule on top.
    pub foot: Rect,
}

impl LibraryBay {
    /// The `index`th row's rectangle, counting from the top of the list.
    ///
    /// Derived rather than stored for [`TransportRow::dot`]'s reason: the rows
    /// are a stride and a count, and a `Vec` of them would be an allocation a
    /// frame does not need. `index` past [`rows`](Self::rows) is a rectangle
    /// past the end of the list, which is a caller's error and not a state —
    /// the one caller iterates `0..rows`.
    pub fn row(&self, index: usize) -> Rect {
        Rect::from_min_size(
            Pos2::new(
                self.list.min.x,
                self.list.min.y + size::LIB_ROW_H * index as f32,
            ),
            egui::vec2(self.list.width(), size::LIB_ROW_H),
        )
    }

    /// What the foot reads: `n of m`, the mock's own `5 of 27`.
    pub fn count(&self) -> String {
        format!("{} of {}", self.rows, self.total)
    }
}

/// **The Library bay's rows, derived** — see [`LibraryBay`] for what is drawn
/// here and for the six things in the mock's bay that are not.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one. Unlike [`outputs`], [`transport`] and [`mixer`] this asks `egui` for
/// nothing: every box in the bay is the full width of the list, so no
/// rectangle here is the width of the type in it.
///
/// `None` where there is nothing to list, and `None` where there is no room to
/// list it: a console with no store behind it is every test in this crate and
/// the whole of `cargo test -p karakuri-console`, and what the bay draws then
/// is the card and its head and nothing else — this is [`mixer`]'s rule, one
/// column along, and [`View::picture`]'s before that.
pub fn library(layout: &karakuri_layout::Layout, sets: &[String]) -> Option<LibraryBay> {
    // **No store behind the console, so there is nothing to list.** Drawing an
    // empty list with `0 of 0` under it would be a reading of a library nobody
    // opened, which is the row of zeroes ADR-0177 is about.
    if sets.is_empty() {
        return None;
    }
    library_box(to_egui(layout.rect(layout.find("library")?)), sets.len())
}

/// The arithmetic of the bay, away from the layout it reads.
///
/// Term for term from `style.css`:
///
/// - `.lib-foot { padding: 5px 10px; font-size: 10px; border-top: 1px solid
///   var(--c-hair) }` — a [`size::LIB_FOOT_H`] row along the bottom of the
///   bay, its rule the top pixel of it.
/// - `.lib-list { display: flex; flex-direction: column; padding: 3px }` —
///   what is left between the bay head and that row, inset by
///   [`size::LIB_LIST_PAD`] on all four sides.
/// - `.lib-row { padding: 3px 7px }` — [`size::LIB_ROW_H`] each, stacked from
///   the top of the list with no gap, because `.lib-list` states none.
///
/// # The foot is at the bottom and the leftover is the list's
///
/// The mock's bay is a flow: its children stack from the top and whatever is
/// left over is under the last of them. **Here the leftover goes to the list
/// instead**, and the mock says which: the Library is the bay that carries
/// `style="flex:1"` in its column and a `.grip` in its head, which is *"the
/// bay that absorbs its column's height"* — and what absorbs it is the list,
/// since a foot of one line at a fixed type size has nothing in it that gets
/// bigger. A foot left floating under the last row would also put its
/// `border-top` between the list and bare card, which is a rule separating
/// something from nothing.
///
/// `None` where the region cannot hold the foot and one row, which is
/// [`picture_rect`]'s rule stated on a listing.
fn library_box(region: Rect, total: usize) -> Option<LibraryBay> {
    let foot = Rect::from_min_max(
        Pos2::new(region.min.x, region.max.y - size::LIB_FOOT_H),
        region.max,
    );
    let list = Rect::from_min_max(
        Pos2::new(
            region.min.x + size::LIB_LIST_PAD,
            region.min.y + size::HEAD_H + size::LIB_LIST_PAD,
        ),
        Pos2::new(
            region.max.x - size::LIB_LIST_PAD,
            foot.min.y - size::LIB_LIST_PAD,
        ),
    );
    // **Narrower than its own padding is no list**, which is
    // [`picture_rect`]'s rule stated across the axis. There is no matching
    // check down it: a bay too short for the foot is already a bay too short
    // for a row, and `rows` below is what answers that.
    if list.width() <= 0.0 {
        return None;
    }
    let fits = (list.height() / size::LIB_ROW_H).floor().max(0.0) as usize;
    let rows = fits.min(total);
    (rows > 0).then_some(LibraryBay {
        list,
        rows,
        total,
        foot,
    })
}

/// **The Library bay's rows, painted.**
///
/// Where everything goes is [`library`]'s, so this paints and derives nothing.
///
/// Term for term from `style.css`:
///
/// - `.lib-row` — `color: var(--c-dim)`, a name at [`size::BASE`], one
///   [`size::LIB_ROW_PAD_X`] in from the left of the list and centred across
///   the row's own height.
/// - `.lib-foot` — `color: var(--c-faint)` at [`size::LIB_FOOT_SIZE`], one
///   [`size::LIB_FOOT_PAD_X`] in and centred, over a
///   `border-top: 1px solid var(--c-hair)`.
///
/// **A name too long for the track is clipped rather than elided**, which is
/// the mock's own answer: `.lib-row` sets no `text-overflow` where `.path` and
/// `.strip-name` both do, so there is no ellipsis to draw. The clip is
/// `.lib-list`'s box, which is the same `with_clip_rect` the picture, a
/// preview cell and a tally are each drawn inside.
fn library_into(ui: &Ui, pal: &Palette, bay: &LibraryBay, sets: &[String]) {
    let painter = ui.painter().with_clip_rect(bay.list);
    for (index, name) in sets.iter().take(bay.rows).enumerate() {
        let row = bay.row(index);
        let galley = painter.layout_job(span_at(name, size::BASE, pal.dim));
        painter.galley(
            Pos2::new(
                row.min.x + size::LIB_ROW_PAD_X,
                row.center().y - galley.size().y * 0.5,
            ),
            galley,
            pal.dim,
        );
    }

    let painter = ui.painter().with_clip_rect(bay.foot);
    let rule = bay.foot.min.y + size::HAIRLINE * 0.5;
    painter.line_segment(
        [
            Pos2::new(bay.foot.min.x, rule),
            Pos2::new(bay.foot.max.x, rule),
        ],
        Stroke::new(size::HAIRLINE, pal.hair),
    );
    let galley = painter.layout_job(span_at(&bay.count(), size::LIB_FOOT_SIZE, pal.faint));
    painter.galley(
        Pos2::new(
            bay.foot.min.x + size::LIB_FOOT_PAD_X,
            bay.foot.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.faint,
    );
}

// ---------------------------------------------------------------------------
// The Inspector
// ---------------------------------------------------------------------------

/// **How many panes the inspector has**, which is `lib.rs`'s [`arrangement`]
/// and the mock's `.insp-split` read as one number: the CSS is
/// `grid-template-columns: 1fr 9px 1fr`, two tracks and the bar between them,
/// and the arrangement builds `inspector-1` and `inspector-2` to match.
///
/// A [`Spec`](karakuri_layout::Spec) builds a
/// [`Layout`](karakuri_layout::Layout) once and the arena has no insert, so
/// the count is settled at build time — the mock's `2 up ▾` is an operator
/// choosing it while running, and that is a control this pass does not add.
pub const PANES: usize = 2;

/// **The arrangement's name for each pane**, in the order the mock draws them.
///
/// Public for [`DECK_LETTERS`]'s reason: a harness that says *which pane* has
/// to say it in the names the arrangement addresses them by, and a second list
/// written out there would go on saying `inspector-1` the day this one does
/// not.
pub const PANE_NAMES: [&str; PANES] = ["inspector-1", "inspector-2"];

/// **The three levels a node head shows, in the order it shows them.**
///
/// `karakuri_operation::Authority` deliberately carries no `ALL` — *"no map
/// target names an authority … it arrives with the first reader"* — and this
/// is that first reader, so the list is written here rather than there. The
/// order is the vocabulary's own declaration order, most restrictive first,
/// which is also the mock's `man / sug / auto`.
///
/// **A fourth level cannot slip past it**: [`auth_word`] is a `match`, so a
/// level added to the vocabulary does not compile until it has a word here,
/// and `every_authority_the_vocabulary_names_is_on_the_node_head` holds this
/// array against that match.
const AUTHORITIES: [Authority; 3] = [
    Authority::Manual,
    Authority::Suggesting,
    Authority::Automatic,
];

/// **The node head's abbreviation for one level**, which is the console's own
/// word and deliberately not `Authority::name`: the vocabulary spells these
/// *manual*, *suggesting* and *automatic* because that is what a record
/// carries, and the manual's node head reads `man / sug / auto`.
///
/// A `match` for [`blend_mode`](crate::view)'s reason one crate along: a
/// fourth level in the vocabulary stops the build here rather than drawing a
/// blank chip.
fn auth_word(authority: Authority) -> &'static str {
    match authority {
        Authority::Manual => "man",
        Authority::Suggesting => "sug",
        Authority::Automatic => "auto",
    }
}

/// **The word on the sync chip**, which is `karakuri_operation::Sync::name`
/// and not a second spelling: *free*, *tempo*, *beat* are the manual's three
/// and the mock's three.
fn sync_word(sync: Sync) -> &'static str {
    sync.name()
}

/// **Every sync mode, in the order the deck head's chip walks them** — the
/// vocabulary's own declaration order, which is `Sync::ALL`'s in
/// `karakuri_engine::transport` and the mock's *free, tempo, beat*.
///
/// [`AUTHORITIES`]' array one head along, and it exists for the same two
/// reasons: [`DeckHead::mode`] steps *positions* in this list, which a `match`
/// cannot express once the step may be refused, and `tests/deck_head.rs` walks
/// it to assert the cycle reaches every mode. A fourth mode is still caught at
/// build time, by [`sync_word`]'s `match` on the way to a word and by
/// [`anchor_letter`]'s on the way to a letter.
///
/// **Public because [`Pane::allows`] is stated in this order**, and whoever
/// fills that field is in another crate: an order a caller has to match and
/// cannot name is an order two crates would each write down.
pub const SYNCS: [Sync; 3] = [Sync::Free, Sync::Tempo, Sync::Beat];

/// **The next mode the chip can actually get to**, walking [`SYNCS`] from the
/// one after `at` and taking the first this deck's material can honour.
///
/// # It skips rather than offering, and that is the mock's own sentence
///
/// *"Click to cycle, and the cycle skips a mode this material cannot honour
/// instead of offering it"*. Whether a mode is honourable is
/// `karakuri_engine::deck::Deck::sync_allowed`'s and arrives here as
/// [`Pane::allows`] — the surface owns the affordance and never the authority
/// ([P-0076](../../../docs/principles/0076-a-surface-owns-the-affordance-never-the-authority.md)),
/// so this chooses which destination to name and refuses nothing.
///
/// # Landing back on `at` is a state and not a failure
///
/// The walk starts at `at + 1`, so it comes back to `at` only when every other
/// mode is refused — and the operation that comes out then names the mode the
/// deck is in, which **re-anchors**. That is the one re-anchor a cycle can
/// reach and it is the one where re-anchoring does nothing: the only material
/// that refuses two modes is material that accumulates *and* reads the beat,
/// where the mode left is `Free` and `Free` reads no anchor. The control that
/// can ask for it deliberately is [`DeckHead::anchor`]
/// ([ADR-0218](../../../docs/adr/0218-re-anchoring-is-set-sync-naming-the-mode-the-deck-is-in-and-a-cycle-cannot-say-it.md)).
///
/// `Free` is refused by nothing, so the loop always finds something and there
/// is no `None` to answer.
fn next_sync(at: Sync, allows: [bool; SYNCS.len()]) -> Sync {
    let from = SYNCS.iter().position(|s| *s == at).unwrap_or(0);
    (1..=SYNCS.len())
        .map(|step| (from + step) % SYNCS.len())
        .find(|index| allows[*index])
        .map(|index| SYNCS[index])
        .unwrap_or(at)
}

/// **The letter the anchor readout leads with**: the mock's `T128` under tempo
/// sync and `B128 +0.25` under beat, and nothing at all under free — *"a free
/// deck shows neither, because free is the absence of a transport rather than
/// a setting, and a column reading free on every deck would be four words of
/// nothing."*
fn anchor_letter(sync: Sync) -> Option<&'static str> {
    match sync {
        Sync::Free => None,
        Sync::Tempo => Some("T"),
        Sync::Beat => Some("B"),
    }
}

/// **What one pane of the inspector is showing**, handed in by whoever has a
/// deck — the same seam [`Strip`] crosses, one bay along.
///
/// `src/` takes no device and no engine (ADR-0156), so nothing here asks a
/// `Set` anything: every field is a value somebody who *can* ask read off one
/// and wrote down. `crates/karakuri` is where that reading is, and it is where
/// the two omissions below are decided as well.
#[derive(Debug, Clone, PartialEq)]
pub struct Pane {
    /// **Which deck this pane is pointed at**, as an index into
    /// [`DECK_LETTERS`] — the mock's `deck A`.
    ///
    /// **The pane is pointed rather than choosing**, and that is the mock's
    /// `showing … ▾` not being drawn: the chooser is a control and this pass
    /// adds none, so whoever fills this says which deck each pane shows. The
    /// console keeps no selection to choose *with* — the same sentence
    /// [`mixer`] writes about `.lib-row.cursor` and the deck selection.
    pub deck: usize,
    /// **What that deck is playing**, which is the same name the deck's mixer
    /// strip carries and comes from the same place — see [`Strip::name`], and
    /// the short of it is that a `Set` has no name of its own and only
    /// whoever built it knows what to call it.
    pub material: String,
    /// What this deck's clock is locked to: `karakuri_engine::transport::Sync`
    /// as the vocabulary's copy of the same three.
    pub sync: Sync,
    /// **Which of [`SYNCS`] this deck's material can honour**, in that order —
    /// what the sync chip's cycle skips over.
    ///
    /// **The answer and not the two facts it is computed from**, which is the
    /// seam every other field here crosses read one step further along.
    /// `karakuri_engine::deck::Deck::sync_allowed` is what decides it, off
    /// whether the Set in the slot is closed form and whether it reads
    /// `beats`, and `src/` has no engine (ADR-0156) — so whoever owns one asks
    /// it three times and writes the three answers here. Handing in the two
    /// properties instead would put a third copy of `Transport::allows`' rule
    /// in a crate that owns no material, and a control is not the authority on
    /// what it may ask for
    /// ([P-0076](../../../docs/principles/0076-a-surface-owns-the-affordance-never-the-authority.md)).
    ///
    /// **A property of the Set, so it moves when a build lands in the slot** —
    /// the engine says so at `sync_allowed`, and it is why this is read beside
    /// [`Pane::sync`] rather than once.
    ///
    /// All three `true` is the whole of *nothing is refused*, which is what a
    /// console with no engine behind it and every test in this crate that does
    /// not say otherwise hands in; `Free` is refused by no material at all, so
    /// the first entry is never `false` in a reading anything took.
    pub allows: [bool; SYNCS.len()],
    /// **The tempo the deck was engaged at**, which is what its rate is
    /// measured against. Drawn only under [`Sync::Tempo`] and [`Sync::Beat`] —
    /// see [`anchor_letter`].
    pub anchor_bpm: f32,
    /// **The scrub's own value, in beats**, signed. Drawn only under
    /// [`Sync::Beat`], *"since that is the only mode that reads the offset"*.
    ///
    /// In **beats** and one deck's, where the transport row's offset is in
    /// milliseconds and is the whole instrument's — `style.css` says the unit
    /// is what tells them apart, *"so neither is ever drawn without one"*, and
    /// the mock's own `+0.25` is what this is drawn as.
    pub scrub_beats: f64,
    /// Whether this deck's Set folds its renderers into one result or
    /// overdraws them: `karakuri_engine::set::Layering`, as a bit.
    ///
    /// **A readout here and a control in the mock.** The chip's tooltip is
    /// *"Click to overdraw them instead"*, and layering is a **build**
    /// decision in the engine — `Set::layering` answers off whether the Set
    /// was built with a merge — so the press is a rebuild rather than a write.
    /// The word is drawn; the press is not.
    pub composite: bool,
    /// **The node groups**, in node order, which is the order a Set addresses
    /// its own nodes in.
    pub nodes: Vec<Node>,
}

/// **One node group**: the head that names a node and says who may move it,
/// and whatever is under it.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// The mock's `.addr` — `L1:0`, `L2:0`, `L4`. **Written by whoever read
    /// the Set**, because the layer names are `karakuri-ir`'s `Kind` and this
    /// crate depends on neither it nor the engine.
    pub addr: String,
    /// What the node is called: `Set::node_names`, or the mock's `renderers`
    /// for the group that folds several.
    pub name: String,
    /// **Who may move this node**, or `None` where the group is more than one
    /// node and so has no one value.
    ///
    /// The second case is the mock's own `L4 renderers` head: authority is set
    /// **per node** (ADR-0211, ADR-0216) and that head stands over every
    /// renderer the Set has, so a chip on it would be one of *n* answers drawn
    /// as *the* answer. A Set with one renderer has one node under that head
    /// and the chip is drawn.
    pub authority: Option<Authority>,
    /// The mock's `.rend-row`: every renderer this Set has, and which of them
    /// is live. Empty on every group that is not the renderers'.
    pub renderers: Vec<Renderer>,
    /// The published controls that belong to this node.
    pub params: Vec<Param>,
}

/// **One renderer chip.**
#[derive(Debug, Clone, PartialEq)]
pub struct Renderer {
    pub name: String,
    /// **Whether this is the one that reaches the screen**, and it is only
    /// ever true under [`Pane::composite`]: *"On, a deck's renderers fold into
    /// one result and one of them is live; off, they are all overdrawn."*
    /// Under overdraw every renderer draws, so marking one would assert a
    /// choice the layering does not make.
    pub live: bool,
}

/// **One parameter row**: what the mock's `.param` reads.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    /// **The position in the deck's published interface**, counting from one
    /// and spanning nodes — the mock's `.ord`, and the number a MIDI control
    /// is learned against: *"knob 3 is knob 3 whatever Set is loaded"*.
    pub ord: usize,
    /// What the Set published it as, which may be an alias for the key inside
    /// the node.
    pub name: String,
    /// What it holds.
    pub value: f32,
    /// **Where it sits in its published range**, `[0, 1]` — the fader's fill.
    ///
    /// A position and not a range plus a value, because the fader is the only
    /// reader and a second derivation of *where along the track* is a second
    /// answer. Whoever publishes the control has the range.
    pub at: f32,
}

/// **The Inspector's pane, laid out**: the head that says which deck, the deck
/// head under it, and what is left for the node groups.
///
/// # What is in the mock's pane and is deliberately not here
///
/// This is [ADR-0200](../../../docs/adr/0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md)
/// applied to the bay it named as the next one and the hardest: *draw every
/// part of the mock that has a value behind it and omit the rest outright — no
/// placeholder, and no empty case the mock did not itself draw.* Nine things
/// are omitted and each one is named with what it waits on. **Three of the
/// nine have since been drawn**, and they are the deck head's — see
/// [`DeckHead`], which is where their argument now lives.
///
/// **Two are controls and are still not here.**
///
/// - **`showing … ▾`**, the chooser in the pane head. Which deck a pane shows
///   is a selection this console does not keep — the same sentence [`mixer`]
///   writes about the mock's `.lib-row.cursor` — so the pane is *pointed* by
///   whoever fills [`Pane::deck`] and the caret is not drawn. The word
///   `showing` and the deck it names are a readout and are.
/// - **`keep`**, the pill beside it: *"Keep deck A as a Set, exactly as it is
///   on screen … It goes into the library under a name."* That is a write into
///   the store, which is the Library bay's `load → A` from the other end and
///   the same still-open question — *how a Set gets between the library and a
///   deck*.
///
/// **A third was `composite`, and it is a readout rather than an omission** —
/// [`Pane::composite`]: layering is a build decision in the engine, so a press
/// on it is a rebuild rather than a write and there is no operation in the
/// vocabulary for it to name. The word is drawn; the press is not.
///
/// **And the fourth was the anchor, which the mock draws as a readout and
/// [ADR-0218](../../../docs/adr/0218-re-anchoring-is-set-sync-naming-the-mode-the-deck-is-in-and-a-cycle-cannot-say-it.md)
/// made a control.** A press on it emits `SetSync` naming the mode the deck is
/// already in, which re-anchors, and its face goes on being a reading of two
/// numbers the deck has. The sync chip beside it and the scrub's two arrows
/// landed with it — the whole of the deck head is [`deck_head`] now, and this
/// type is the pane's three rows.
///
/// **Two have no value in this workspace at all**, which is ADR-0191's rule —
/// a panel drawing a state the engine never entered is a drawing of one.
///
/// - **`.param.bound`'s `.pval.src`**, a bound parameter showing its source
///   instead of a number. The value exists in the engine —
///   `Set::bindings` hands back a `Binding` with the signal driving each param
///   — and **nothing in `crates/karakuri` binds anything**: that binary takes
///   two `.kir` paths and no `--bind`, so `Set::bindings` is empty in every
///   run of it and a bound row is a state this program cannot enter. The
///   mock's other two sources, `midi 21` and `seq 1`, are not bindings at all
///   — no MIDI map and no sequencer lane exists to be one.
/// - **The `.sens` row under a bound parameter** — the signal, the curve, the
///   range and `take back`. It waits on the row above it, and `take back` is a
///   control besides.
///
/// **Two are the shape of the mock disagreeing with the shape of a Set**, and
/// they are the two things this pass found:
///
/// - **A published control that names no node has no row.** The manual groups
///   parameters *"by node, the way a Set is addressed everywhere else"*, and
///   the mock draws every `.param` inside a `.node-group`. But
///   `Published::at` is an `Option` and the **default** interface — the one
///   every Set in `crates/karakuri` has, since that binary has no `--publish`
///   — is made entirely of wildcards: *"one control per key, not one per
///   declaration"*, addressed at every node that declares the key. A wildcard
///   covering exactly one node is that node's and is drawn there; one covering
///   several belongs to several groups and is **omitted**, because the mock
///   draws no row outside a group and inventing a place for one is a
///   specification written backwards. It waits on the page saying where such a
///   row goes.
/// - **The `L4` group's authority chip.** See [`Node::authority`].
///
/// **And one is the pane running out of room.** See [`InspectorPane::shown`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InspectorPane {
    /// `.half-head`, along the top of the pane, with its rule on the bottom.
    pub head: Rect,
    /// `.deck-head`, under it — the deck's own clock and its fold.
    pub deck_head: Rect,
    /// What is left under the two heads, where the node groups stack from the
    /// top with a hairline between them.
    pub body: Rect,
    /// **How many node groups are drawn**, which is how many fit **whole** in
    /// [`body`](Self::body).
    ///
    /// # The pane overflows, and this is the console's first scroll position
    ///
    /// A Set of any size does not fit in 237 pixels of pane, and **there is no
    /// scroll position anywhere in this crate**: the Library bay says so at
    /// its own foot (*"nothing scrolls — which is honest, because there is no
    /// scroll position anywhere in this crate and inventing one would be a
    /// control"*), and this is the first region of the console that genuinely
    /// wants one rather than merely tolerating the lack.
    ///
    /// So what does not fit is not drawn, and a group is drawn whole or not at
    /// all — the same `floor` [`LibraryBay::rows`] takes, on rows of unequal
    /// height. **A half group is worse than a missing one**: a node head with
    /// two of its five parameters under it reads as a node with two
    /// parameters, where a group that is simply not there reads as a pane that
    /// has run out, which is what it is.
    ///
    /// **Zero is a state and not a `None`.** A pane too short for its first
    /// group still says which deck it is showing and what that deck's clock is
    /// doing, which is the whole content of the two heads; it is
    /// [`inspector`]'s `None` that means *there is no pane here to draw*.
    pub shown: usize,
}

impl InspectorPane {
    /// Where the `index`th of the drawn groups goes, and how tall it is.
    ///
    /// Derived rather than stored for [`LibraryBay::row`]'s reason — the
    /// groups are a walk and a `Vec` of rectangles would be an allocation a
    /// frame does not need — but a walk rather than a stride, because a group
    /// is as tall as what is in it. `index` past [`shown`](Self::shown) is a
    /// caller's error and not a state; the one caller iterates `0..shown`.
    pub fn group(&self, nodes: &[Node], index: usize) -> Rect {
        let top = self.body.min.y
            + nodes
                .iter()
                .take(index)
                .map(|node| group_h(node) + size::HAIRLINE)
                .sum::<f32>();
        Rect::from_min_size(
            Pos2::new(self.body.min.x, top),
            egui::vec2(self.body.width(), group_h(&nodes[index])),
        )
    }
}

/// **How tall one node group is**: its head, the renderer row if it has one,
/// and a [`size::PARAM_H`] row per parameter.
///
/// The hairline between two groups is **not** in here and is added by whoever
/// stacks them — `.node-group`'s `border-bottom` is `0` on the last of them,
/// so *n* groups carry *n - 1* rules, which is [`size::PREVIEW_GAP`]'s reading
/// of a gap one axis along.
fn group_h(node: &Node) -> f32 {
    size::NODE_HEAD_H
        + match node.renderers.is_empty() {
            true => 0.0,
            false => size::REND_ROW_H,
        }
        + size::PARAM_H * node.params.len() as f32
}

/// **One pane of the Inspector, derived** — see [`InspectorPane`] for what is
/// drawn here and for the nine things in the mock's pane that are not.
///
/// `index` is which pane, into [`PANE_NAMES`]. `layout` must be solved:
/// [`Layout::rect`](karakuri_layout::Layout::rect) refuses to answer from a
/// dirty one. Like [`library`] this asks `egui` for nothing: every box in the
/// pane is either the full width of the pane or a track of the mock's own
/// grid, so no rectangle here is the width of the type in it.
///
/// `None` where there is no pane by that name, and `None` where there is no
/// room for the two heads — which is [`picture_rect`]'s rule stated on a pane.
/// A console with no deck behind it hands over no panes at all, and what the
/// bay draws then is its card, its head and the bar between the two panes,
/// exactly as it did before this pass — [`mixer`]'s rule, one bay along.
///
/// **The bay head is taken off the top here and not in [`pane_box`]**, which
/// is what [`strips_row`] and [`library_box`] do one bay along: the head is
/// painted *over* the region rather than laid out beside it, so every body in
/// this file starts at `region.min.y + size::HEAD_H` and the arithmetic under
/// it is written as if the head were not there. A pane is the one body in the
/// arrangement whose region is not the bay's own — `inspector-1` is a child of
/// the split — and that is what hid this: the pane is the full height of the
/// bay, head included, so a `.half-head` drawn at `region.min` lands on top of
/// the word `Inspector`.
pub fn inspector(
    layout: &karakuri_layout::Layout,
    index: usize,
    pane: &Pane,
) -> Option<InspectorPane> {
    let region = to_egui(layout.rect(layout.find(PANE_NAMES.get(index)?)?));
    let under_head = Rect::from_min_max(
        Pos2::new(region.min.x, region.min.y + size::HEAD_H),
        region.max,
    );
    pane_box(under_head, &pane.nodes)
}

/// The arithmetic of a pane, away from the layout it reads.
///
/// Term for term from `style.css`:
///
/// - `.half-head { padding: 5px 10px; border-bottom: 1px solid var(--c-hair) }`
///   — a [`size::HALF_HEAD_H`] row along the top of the pane, its rule the
///   bottom pixel of it.
/// - `.deck-head { padding: 5px 10px }` — a [`size::DECK_HEAD_H`] row under
///   it, with no rule of its own: *"Its box is `.node-head`'s without the
///   tint"*, and the tint is what separates it from the group below.
/// - what is left is the node groups', stacked from the top.
///
/// **The leftover is the last group's and not the pane's**, which is the
/// opposite of what [`library_box`] does with its foot, and the reason is the
/// same read the other way: the mock's pane is a flow with nothing under the
/// groups at all, so there is no row for a leftover to sit under. It shows as
/// the bay's own card below the last group, which is every other empty body in
/// this pass.
fn pane_box(region: Rect, nodes: &[Node]) -> Option<InspectorPane> {
    let head = Rect::from_min_max(
        region.min,
        Pos2::new(region.max.x, region.min.y + size::HALF_HEAD_H),
    );
    let deck_head = Rect::from_min_max(
        Pos2::new(region.min.x, head.max.y),
        Pos2::new(region.max.x, head.max.y + size::DECK_HEAD_H),
    );
    let body = Rect::from_min_max(Pos2::new(region.min.x, deck_head.max.y), region.max);
    // **Narrower than a parameter row's own padding is no pane**, which is
    // [`library_box`]'s width check with the mock's own indent in it. There is
    // no matching check down the pane: a pane too short for a group draws its
    // two heads and no group, which is what `shown` answers.
    if body.width() <= size::PARAM_PAD_L + size::PARAM_PAD_R {
        return None;
    }
    // **And a pane too short for its two heads is no pane**, which is what
    // this function's caller promises. `positive` is not enough on its own:
    // the two heads are stated heights, so they stay positive while running
    // off the bottom of a region shorter than their sum.
    if !positive(head) || !positive(deck_head) || deck_head.max.y > region.max.y {
        return None;
    }
    // A walk and not a division: the groups are of unequal height, so *how
    // many fit* is asked one at a time and stops at the first that does not.
    // A group that would cross the bottom edge is not drawn at all — see
    // `InspectorPane::shown`.
    let mut used = 0.0;
    let mut shown = 0;
    for node in nodes {
        let rule = match shown {
            0 => 0.0,
            _ => size::HAIRLINE,
        };
        let next = used + rule + group_h(node);
        if next > body.height() {
            break;
        }
        used = next;
        shown += 1;
    }
    Some(InspectorPane {
        head,
        deck_head,
        body,
        shown,
    })
}

/// **What the anchor reads**, and `None` under free sync.
///
/// The mock's own two spellings: `T128` where a deck is tempo-synced, and
/// `B128 +0.25` where it is beat-synced and sitting a quarter beat ahead of
/// the room. The tempo is whole because the mock writes it whole —
/// `T<b>128</b>` — and the offset carries two places and a sign because the
/// mock's `<em>+0.25</em>` does. **The offset is in beats and the transport
/// row's is in milliseconds**, so neither is ever drawn without knowing which
/// it is; here that is the `B` in front of it.
fn anchor_text(pane: &Pane) -> Option<String> {
    let letter = anchor_letter(pane.sync)?;
    Some(match pane.sync {
        Sync::Beat => format!("{letter}{:.0} {:+.2}", pane.anchor_bpm, pane.scrub_beats),
        _ => format!("{letter}{:.0}", pane.anchor_bpm),
    })
}

/// **How far one press of the scrub moves a deck**, in beats.
///
/// A quarter beat, which is the vocabulary's own figure —
/// [`Operation::ScrubDeck`] writes it at the field (*"How far, in beats. A
/// quarter beat is what a key press asks for"*) and the row it fills is
/// titled *Scrub a deck a quarter beat*. The console page says the same of
/// this control: *"a quarter beat a press, into that same offset"*.
///
/// **Signed at the call site and not here.** The left arrow asks for minus
/// this and the right for plus it, so the amount is one number and the
/// direction is which chip was pressed — see [`DeckHead::scrub`], and the
/// manual's *"the one control here meant to go backwards"*.
pub const SCRUB_BEATS: f64 = 0.25;

/// **The deck head's controls, laid out**: the chip that names the mode, the
/// anchor that re-asks for it, the two arrows that scrub, and the fold at the
/// right.
///
/// # One derivation, for [`Outputs`]' reason
///
/// [`View::draw`] paints exactly these rectangles and [`crate::input::claim`]
/// hit-tests exactly these rectangles. Two copies of a flex row's running sum
/// is an arrow that lights under a pointer that cannot move the deck.
///
/// # Four rectangles and one type, because they are one row
///
/// [`LookRow`]'s reason one bay over: each one's place is measured from the
/// last, which is what a flex row is, and splitting them into four functions
/// would mean measuring the chip before each of them again to find out where
/// it starts. The fold is in here for the same reason and is **not** a control
/// — see [`DeckHead::composite`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeckHead {
    /// **The sync chip**, which is what a press has to land in to move the
    /// mode on. `.mini`'s box round [`sync_word`], at the left of the row.
    pub mode: Rect,
    /// **The anchor**, or `None` under [`Sync::Free`], where there is nothing
    /// to read and nothing to re-anchor — see [`anchor_text`].
    ///
    /// **The text run grown to a chip's height**, which is [`LookRow::grip`]'s
    /// treatment of a 5px track and its argument: `.anchor` is a bare span at
    /// [`size::ANCHOR_SIZE`] with no padding of its own, and 13.5 pixels of
    /// type is not a target a hand finds. It is grown to [`size::MINI_H`], so
    /// it is the same 15.5 as the chips either side of it and sits in the same
    /// [`size::DECK_HEAD_PAD_Y`] the row gives them. **No wider than the
    /// words**, because the row is a flex row and a target that reached past
    /// its own text would take the arrows' places with it.
    pub anchor: Option<Rect>,
    /// **A quarter beat back.** One `.scrub i`.
    pub back: Rect,
    /// **A quarter beat forward.** The other.
    pub forward: Rect,
    /// **The fold at the right of the row, and it is not a control** —
    /// [`Pane::composite`] says why: layering is a *build* decision, so a
    /// press is a rebuild rather than a write and there is no operation in the
    /// vocabulary this chip could name. The word is drawn; the press is not,
    /// and [`DeckHead::owns`] does not claim it.
    ///
    /// It is here because the row is laid out **to** it: it is `.sep`'s
    /// `flex: 1` pushing it against the right-hand padding, and what says the
    /// controls on the left fit is that they end before it.
    pub composite: Rect,
    /// Which deck this head belongs to, as [`Operation::SetSync`] and
    /// [`Operation::ScrubDeck`] each name one — [`Pane::deck`], carried so
    /// that a press answers with the deck it was measured for.
    pub deck: usize,
    /// **What the mode chip is showing**, and what re-anchoring re-asks for.
    /// Carried for [`LookRow::values`]' reason: whoever measured this row and
    /// whoever acts on a press in it are one statement.
    pub locked: Sync,
    /// **What this deck's material can honour**, [`Pane::allows`] as it was
    /// read — the whole of what the cycle skips on.
    pub allows: [bool; SYNCS.len()],
}

impl DeckHead {
    /// Whether `p` is on the sync chip.
    pub fn hit_mode(&self, p: karakuri_layout::Point) -> bool {
        self.mode.contains(Pos2::new(p.x, p.y))
    }

    /// Whether `p` is on the anchor, which a free deck does not draw.
    pub fn hit_anchor(&self, p: karakuri_layout::Point) -> bool {
        self.anchor
            .is_some_and(|at| at.contains(Pos2::new(p.x, p.y)))
    }

    /// **Which arrow `p` is on, as the amount it asks for** — or `None` off
    /// both, and `None` on either while the scrub is inert.
    ///
    /// **Inert is not claimed**, which is [`crate::input`]'s *a control claims
    /// what it acts on and no more*, and it is why this answers for liveness
    /// as well as for position. The arrows are drawn on every deck, because a
    /// scrub is *"an offset added to the room's position under beat sync"* and
    /// a pair of chips that vanished on two modes out of three would move the
    /// rest of the row under the hand every time the chip beside them was
    /// pressed. Drawn and not claimed is the arrangement a fader's track is
    /// already in.
    fn arrow(&self, p: karakuri_layout::Point) -> Option<f64> {
        if self.locked != Sync::Beat {
            return None;
        }
        let p = Pos2::new(p.x, p.y);
        match (self.back.contains(p), self.forward.contains(p)) {
            (true, _) => Some(-SCRUB_BEATS),
            (_, true) => Some(SCRUB_BEATS),
            _ => None,
        }
    }

    /// **Whether `p` is on any of the three**, which is what
    /// [`crate::input::claim`] asks. The fold is not one of them.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.hit_mode(p) || self.hit_anchor(p) || self.arrow(p).is_some()
    }

    /// **What a press at `p` asks this deck's clock to become**, or `None`
    /// where there is no sync chip under it.
    ///
    /// # The chip cycles, the operation names where it arrived, and the cycle
    /// skips
    ///
    /// Click it and the deck is asked for the next of [`SYNCS`] its material
    /// can honour — *free*, *tempo*, *beat*, wrapping — and what comes out is
    /// [`Operation::SetSync`] naming the **destination**, never a step,
    /// because there is no step in the vocabulary to name. The affordance is
    /// [`Mixer::blend`]'s and [`LookRow::tonemap`]'s exactly
    /// ([ADR-0187](../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md)),
    /// and so is the division it rests on: the cycle is [`next_sync`] here and
    /// nothing at all in `karakuri-operation`, which is P-0074's *"a toggle is
    /// an affordance, built over operations by whoever draws the control"*.
    ///
    /// **The skip is the one thing this cycle has that the other two do not**,
    /// and it is not a refusal: what may be asked for is the engine's
    /// ([P-0076](../../../docs/principles/0076-a-surface-owns-the-affordance-never-the-authority.md)),
    /// and this chip is choosing which of the destinations *it* offers to
    /// name, out of a reading somebody else took. A mode this material cannot
    /// honour is passed over rather than handed on to be refused, which is the
    /// mock's own *"skips a mode this material cannot honour instead of
    /// offering it"*.
    ///
    /// **What a map is offered is the three modes, not the cycle** — the
    /// sentence ADR-0187 wrote about three blend modes, and the reason the
    /// anchor beside this chip can be a fourth way to say one of them.
    pub fn sync(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_mode(p).then(|| Operation::SetSync {
            deck: self.deck as u8,
            sync: next_sync(self.locked, self.allows),
        })
    }

    /// **What a press on the anchor asks for**: [`Operation::SetSync`] naming
    /// the mode this deck is **already in**, which re-anchors it — or `None`
    /// off the anchor, and `None` on a free deck, which draws none.
    ///
    /// # It is one operation asked for from two ends, and that is the record
    ///
    /// `karakuri_engine::transport::Transport::engage`'s own documentation is
    /// where this comes from: *"Re-engaging the mode a slot is already in
    /// re-anchors it, which is how an operator says 'call **this** the
    /// reference tempo' without a second control."* `Transport::engaged`
    /// recomputes the anchor from the session tempo every time and clears the
    /// scrub with it, so naming the mode that is running is a real move rather
    /// than a press that does nothing.
    ///
    /// **A cycle structurally cannot ask for it**, which is why this is a
    /// second target on the row rather than a second press on the first:
    /// [`next_sync`] starts at the mode *after* the one the deck is in, so the
    /// one state it can never arrive at is the state it is in. That is a fault
    /// of the affordance and not a gap in the vocabulary, and it is why there
    /// is no `ReAnchor` variant here to name — an operation whose meaning is
    /// *again* is the shape P-0074 rules out
    /// ([ADR-0218](../../../docs/adr/0218-re-anchoring-is-set-sync-naming-the-mode-the-deck-is-in-and-a-cycle-cannot-say-it.md)).
    ///
    /// **A free deck has no anchor and loses nothing.** `Free` is the absence
    /// of a transport rather than a setting and reads no anchor at all, so
    /// there is nothing for a press to re-ask for — [`anchor_text`] draws
    /// nothing there and this answers `None` off the same `Option`.
    pub fn reanchor(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_anchor(p).then_some(Operation::SetSync {
            deck: self.deck as u8,
            sync: self.locked,
        })
    }

    /// **What a press on an arrow asks for**: [`Operation::ScrubDeck`] moving
    /// this deck a [`SCRUB_BEATS`] back or forward — or `None` off both, and
    /// `None` wherever the scrub is inert.
    ///
    /// # The one control on this panel that moves by an amount
    ///
    /// Every other control here names a destination, which is P-0074's rule,
    /// and the vocabulary says at the variant why this one does not: *"it is
    /// relative because nothing in this instrument can set a position"*.
    /// Scrubbing moves closed-form material by an amount; accumulating
    /// material cannot be moved to a position at all, so an absolute
    /// `at_beat` would be an operation that does not exist for two thirds of
    /// the material. So this is not the exception to P-0074 it looks like —
    /// there is no destination in the language for it to name.
    ///
    /// **Signed and unbounded**, which is the deck head's own spelling: the
    /// offset this writes is drawn in the anchor beside it, in **beats** and
    /// for one deck, where the transport row's offset is in milliseconds and
    /// is the whole instrument's. Nothing clamps it here and there is nothing
    /// to clamp it to.
    ///
    /// **Inert under anything but beat sync**, because that is the only mode
    /// that reads the offset. The arrows keep their shape and the console page
    /// says why rather than greying them out: *"Nothing here re-runs a deck's
    /// history to place it."*
    ///
    /// # One press is one operation
    ///
    /// [ADR-0207](../../../docs/adr/0207-a-continuous-control-says-one-thing-per-frame.md)
    /// coalesces a swept MIDI fader to one operation a frame; neither half of
    /// it applies to a press, and a second press in the same frame is a second
    /// quarter beat an operator asked for. That is the whole reason the amount
    /// is a constant rather than a distance along anything.
    pub fn scrub(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.arrow(p).map(|beats| Operation::ScrubDeck {
            deck: self.deck as u8,
            beats,
        })
    }
}

/// **The deck head's controls, derived** — the chips in
/// [`InspectorPane::deck_head`], laid along it the way `.deck-head`'s flex row
/// lays them.
///
/// # Measured off a laid-out pane rather than off the layout
///
/// [`inspector`] answers where the pane is and this answers where the chips in
/// its second row are: two questions about one laid-out pane, which is exactly
/// what the mixer bay's four controls are about one laid-out strip. A second
/// derivation from the layout would be a second answer that could disagree
/// with the one the frame drew.
///
/// # What it costs to ask
///
/// **Three galley lookups per pane**: the mode's word, the anchor's two
/// numbers and the fold's word. The two arrows cost none — they are marks
/// rather than words, which is [`Mixer::mask`]'s own saving one bay over — and
/// nothing here asks after the node groups below.
///
/// **It is asked twice on a frame**, once here and once for the galleys
/// [`deck_head_into`] paints, which is [`look`]'s honest cost written down one
/// bay along: the derivation that draws a control is the one that hit-tests
/// it, so a control cannot be painted anywhere a press cannot reach. Two panes
/// at three lookups is on the order of ten allocations a frame against the
/// panel pass's measured median of 525 (`crates/karakuri`'s `WRITTEN_ALLOCS`,
/// taken 2026-08-26), which is inside the factor of two that file quotes a
/// figure across.
///
/// # `None` is a row that cannot hold its own controls
///
/// [`look`]'s rule and [`arrangement`]'s: *a control that does not fit in the
/// row it is drawn in is no control at all, rather than half of one*. The fold
/// is pushed against the right-hand padding and the three controls run from
/// the left, so what says the row fits is that the arrows end before the fold
/// begins. A pane narrow enough to fail that draws its two heads' words and no
/// chips at all, where it used to draw chips cut in half by
/// [`inspector_into`]'s clip rectangle — which is a picture of a control that
/// cannot be pressed
/// ([P-0027](../../../docs/principles/0027-a-silently-wrong-image-loses-to-a-loud-failure.md)).
pub fn deck_head(ctx: &egui::Context, at: &InspectorPane, pane: &Pane) -> Option<DeckHead> {
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // [`transport`], [`outputs`] and [`mixer`] — and on the frame before the
    // first one there is nothing drawn here to press.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let row = at.deck_head;
    let mid = row.center().y;
    let width = |text: &str, size: f32| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };
    // `.mini`'s box, which is `mini_word`'s arithmetic: the padding either
    // side of the word, with no border counted, because that is what this row
    // has always been drawn with and this is the same chip.
    let mini = |text: &str, x: f32| {
        Rect::from_min_size(
            Pos2::new(x, mid - size::MINI_H * 0.5),
            egui::vec2(
                width(text, size::MINI_SIZE) + size::MINI_PAD_X * 2.0,
                size::MINI_H,
            ),
        )
    };

    let mode = mini(sync_word(pane.sync), row.min.x + size::DECK_HEAD_PAD_X);
    let anchor = anchor_text(pane).map(|text| {
        Rect::from_min_size(
            Pos2::new(mode.max.x + size::DECK_HEAD_GAP, mid - size::MINI_H * 0.5),
            egui::vec2(width(&text, size::ANCHOR_SIZE), size::MINI_H),
        )
    });
    // One `.deck-head` gap after whichever of the two came last — a free deck
    // draws no anchor, and a flex row closes up rather than leaving a hole
    // where one would have been.
    let arrows = anchor.map_or(mode.max.x, |at| at.max.x) + size::DECK_HEAD_GAP;
    // `.scrub i`'s box: the mark is as wide as the glyph it stands in for,
    // which is [`Mixer::mask`]'s rule, inside its own padding and its border.
    let arrow_w = size::SCRUB_SIZE + size::SCRUB_PAD_X * 2.0 + size::HAIRLINE * 2.0;
    let arrow = |x: f32| {
        Rect::from_min_size(
            Pos2::new(x, mid - size::SCRUB_H * 0.5),
            egui::vec2(arrow_w, size::SCRUB_H),
        )
    };
    let back = arrow(arrows);
    let forward = arrow(back.max.x + size::SCRUB_GAP);

    // `.sep`'s `flex: 1` puts the fold hard against the right of the row.
    let fold = mini(COMPOSITE_LABEL, row.min.x);
    let composite = mini(
        COMPOSITE_LABEL,
        row.max.x - size::DECK_HEAD_PAD_X - fold.width(),
    );

    if !row.contains_rect(mode)
        || !row.contains_rect(composite)
        || forward.max.x + size::DECK_HEAD_GAP > composite.min.x
    {
        return None;
    }

    Some(DeckHead {
        mode,
        anchor,
        back,
        forward,
        composite,
        deck: pane.deck,
        locked: pane.sync,
        allows: pane.allows,
    })
}

/// **What the pane head reads**: the mock's `deck A · drift_night`.
fn showing_text(pane: &Pane) -> String {
    format!(
        "deck {} · {}",
        DECK_LETTERS.get(pane.deck).copied().unwrap_or("?"),
        pane.material
    )
}

/// The word the mock puts in front of it.
const SHOWING_LABEL: &str = "showing";

/// **The word on the fold chip**, which is the mock's own and is drawn whether
/// or not it changes anything: *"A deck publishing a single renderer draws the
/// chip anyway and says that it changes nothing either way, because a deck
/// that grows a second one needs the control already where it was."*
const COMPOSITE_LABEL: &str = "composite";

/// **One pane of the Inspector, painted.**
///
/// Where everything goes is [`inspector`]'s, so this paints and derives
/// nothing but the position of one chip after another along a row, which is
/// what a flex row is.
///
/// Term for term from `style.css`:
///
/// - `.half-head` — `color: var(--c-faint)` for the label, `.what`'s
///   `color: var(--c-text)` for the deck and its material, over a
///   `border-bottom: 1px solid var(--c-hair)`.
/// - `.deck-head` — a `.mini` for the sync mode, `.anchor` at
///   [`size::ANCHOR_SIZE`] beside it, and the fold's `.mini` pushed to the
///   right by `.sep`'s `flex: 1`.
/// - `.node-head` — `background: var(--c-tint)`, `.addr`'s
///   `color: var(--c-lav)`, the name in `var(--c-dim)`, and `.auth`'s three
///   words at the right.
/// - `.rend-row` — `.rend` chips, the live one in `var(--c-pink)` over a 15%
///   wash of it.
/// - `.param` — the mock's four tracks, with the fader taking what the other
///   three leave.
///
/// **Everything is clipped to the pane**, which is what makes the overflow
/// safe: a group that fits and a name that does not are the same clip, and it
/// is the same `with_clip_rect` the picture, a preview cell and the library's
/// list are each drawn inside.
fn inspector_into(ui: &Ui, pal: &Palette, at: &InspectorPane, pane: &Pane) {
    let painter = ui.painter().with_clip_rect(at.head);
    let mut x = at.head.min.x + size::HALF_HEAD_PAD_X;
    let label = painter.layout_job(span_at(SHOWING_LABEL, size::BASE, pal.faint));
    let y = at.head.center().y - label.size().y * 0.5;
    x += label.size().x + size::HALF_HEAD_GAP;
    painter.galley(
        Pos2::new(at.head.min.x + size::HALF_HEAD_PAD_X, y),
        label,
        pal.faint,
    );
    let what = painter.layout_job(span_at(&showing_text(pane), size::BASE, pal.text));
    painter.galley(
        Pos2::new(x, at.head.center().y - what.size().y * 0.5),
        what,
        pal.text,
    );
    // `.half-head`'s own `border-bottom`, the bottom pixel of the row.
    let rule = at.head.max.y - size::HAIRLINE * 0.5;
    painter.line_segment(
        [
            Pos2::new(at.head.min.x, rule),
            Pos2::new(at.head.max.x, rule),
        ],
        Stroke::new(size::HAIRLINE, pal.hair),
    );

    // **Derived here and hit-tested by `claim` off the same call**, which is
    // the rule every other control on this panel is drawn under. `None` is a
    // row too narrow to hold its chips, and it draws none rather than half of
    // each — see [`deck_head`].
    if let Some(head) = deck_head(ui.ctx(), at, pane) {
        deck_head_into(ui, pal, &head, pane);
    }

    let painter = ui.painter().with_clip_rect(at.body);
    for index in 0..at.shown {
        let node = &pane.nodes[index];
        let rect = at.group(&pane.nodes, index);
        node_into(&painter, pal, rect, node);
        // `.node-group`'s `border-bottom: 1px solid var(--c-hair)`, which
        // `:last-child` does not carry — so it goes *between* two groups and
        // the one after the last drawn group is not drawn either, because
        // there is nothing under it to separate from.
        if index + 1 < at.shown {
            let rule = rect.max.y + size::HAIRLINE * 0.5;
            painter.line_segment(
                [Pos2::new(rect.min.x, rule), Pos2::new(rect.max.x, rule)],
                Stroke::new(size::HAIRLINE, pal.hair),
            );
        }
    }
}

/// **The deck head, painted**: the sync chip, the anchor beside it, the two
/// scrub arrows and the fold at the right.
///
/// Where everything goes is [`deck_head`]'s, so this paints and derives
/// nothing — which is the change this pass made to it: the row used to be a
/// running sum here and a press had nowhere to ask what it had landed on.
///
/// Term for term from `style.css`:
///
/// - `.mini` — the mode chip and the fold, through [`mini_into`], selected
///   because a mode is always one of three and the fold is on or off.
/// - `.anchor` — `font-size: 9px` in `--c-faint`, the run [`anchor_text`]
///   writes.
/// - `.scrub i` — `padding: 0 4px; border-radius: 999px; border: 1px solid
///   var(--c-line)` with `--c-dim` inside it, and `.scrub.idle i`'s
///   `--c-faint` over `--c-hair` where the deck is not beat-synced. **Never
///   grey without a reason**, which is the stylesheet's own note on this pair:
///   an inert scrub keeps its shape, and what says why is the page.
///
/// **The arrows are drawn rather than typed**, which is [`CHEVRON_W`]'s reason
/// three bays along: whether a black left-pointing small triangle is in
/// `egui`'s default face is a question with no good answer, and a triangle is
/// the same mark either way.
fn deck_head_into(ui: &Ui, pal: &Palette, at: &DeckHead, pane: &Pane) {
    let painter = ui.painter().with_clip_rect(at.mode.union(at.composite));
    let word = |rect: Rect, text: &str, sel: bool| {
        let galley = painter.layout_no_wrap(
            text.to_owned(),
            FontId::new(size::MINI_SIZE, FontFamily::Proportional),
            pal.faint,
        );
        mini_into(&painter, pal, rect, sel, |painter, colour| {
            painter.galley(
                Pos2::new(
                    rect.min.x + size::MINI_PAD_X,
                    rect.center().y - galley.size().y * 0.5,
                ),
                galley,
                colour,
            );
        });
    };
    word(at.mode, sync_word(pane.sync), true);

    if let (Some(rect), Some(text)) = (at.anchor, anchor_text(pane)) {
        let galley = painter.layout_job(span_at(&text, size::ANCHOR_SIZE, pal.faint));
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            pal.faint,
        );
    }

    // Live is the one mode that reads the offset; the other two keep the
    // chips and lose the ink, which is `.scrub.idle`.
    let live = pane.sync == Sync::Beat;
    let (ink, edge) = match live {
        true => (pal.dim, pal.line),
        false => (pal.faint, pal.hair),
    };
    for (rect, back) in [(at.back, true), (at.forward, false)] {
        painter.rect_stroke(
            rect,
            CornerRadius::same((size::SCRUB_H * 0.5) as u8),
            Stroke::new(size::HAIRLINE, edge),
            StrokeKind::Inside,
        );
        arrow_mark(&painter, rect.center(), ink, back);
    }

    word(at.composite, COMPOSITE_LABEL, pane.composite);
}

/// **One scrub arrow's mark**, drawn rather than typed — a triangle with its
/// point to the left when `back`, to the right when not.
///
/// [`size::SCRUB_SIZE`] across and the same tall, which is [`Mask`]'s rule for
/// a mark that stands in for a glyph: the box is the size the glyph would have
/// been. It is not [`CHEVRON_W`]'s 2:1, because the ink of a left-pointing
/// small triangle is about as wide as it is tall where a down-pointing one is
/// wider than it is deep.
fn arrow_mark(painter: &egui::Painter, centre: Pos2, colour: Color32, back: bool) {
    let r = size::SCRUB_SIZE * 0.5;
    let point = match back {
        true => centre.x - r,
        false => centre.x + r,
    };
    let base = match back {
        true => centre.x + r,
        false => centre.x - r,
    };
    painter.add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(point, centre.y),
            Pos2::new(base, centre.y - r),
            Pos2::new(base, centre.y + r),
        ],
        colour,
        Stroke::NONE,
    ));
}

/// **One node group**: the head, the renderer row where there is one, and a
/// row per parameter.
fn node_into(painter: &egui::Painter, pal: &Palette, rect: Rect, node: &Node) {
    let head = Rect::from_min_max(
        rect.min,
        Pos2::new(rect.max.x, rect.min.y + size::NODE_HEAD_H),
    );
    // `.node-head`'s `background: var(--c-tint)`, which is the wash that tells
    // a head from the rows under it — the same tint the *allocated* tally
    // carries.
    painter.rect_filled(head, CornerRadius::ZERO, pal.tint);
    let mut x = head.min.x + size::NODE_HEAD_PAD_X;
    let addr = painter.layout_job(span_at(&node.addr, size::BASE, pal.lav));
    let w = addr.size().x;
    painter.galley(
        Pos2::new(x, head.center().y - addr.size().y * 0.5),
        addr,
        pal.lav,
    );
    x += w + size::NODE_HEAD_GAP;
    let name = painter.layout_job(span_at(&node.name, size::BASE, pal.dim));
    painter.galley(
        Pos2::new(x, head.center().y - name.size().y * 0.5),
        name,
        pal.dim,
    );
    if let Some(authority) = node.authority {
        auth_into(painter, pal, head, authority);
    }

    let mut y = head.max.y;
    if !node.renderers.is_empty() {
        rend_row_into(
            painter,
            pal,
            Rect::from_min_max(
                Pos2::new(rect.min.x, y),
                Pos2::new(rect.max.x, y + size::REND_ROW_H),
            ),
            &node.renderers,
        );
        y += size::REND_ROW_H;
    }
    for param in &node.params {
        param_into(
            painter,
            pal,
            Rect::from_min_max(
                Pos2::new(rect.min.x, y),
                Pos2::new(rect.max.x, y + size::PARAM_H),
            ),
            param,
        );
        y += size::PARAM_H;
    }
}

/// **`man / sug / auto`, right-aligned on the node head**, with the one the
/// node is on filled: `.auth span.sel`'s `color: var(--c-mint)` over a 15%
/// wash of it, and the other two in `var(--c-faint)` with no box at all.
///
/// **All three and not only the one**, which is the manual's own row: *"`man /
/// sug / auto` on each node head, never a global mode."* The two that are not
/// selected are the affordance and this pass does not claim a press on them —
/// what is drawn is which of the three this node is on.
fn auth_into(painter: &egui::Painter, pal: &Palette, head: Rect, authority: Authority) {
    let mut chips = Vec::with_capacity(AUTHORITIES.len());
    let mut total = 0.0;
    for (n, level) in AUTHORITIES.into_iter().enumerate() {
        let galley = painter.layout_no_wrap(
            auth_word(level).to_owned(),
            FontId::new(size::AUTH_SIZE, FontFamily::Proportional),
            pal.faint,
        );
        let w = galley.size().x + size::AUTH_PAD_X * 2.0;
        total += w + match n {
            0 => 0.0,
            _ => size::AUTH_GAP,
        };
        chips.push((level, galley, w));
    }
    let mut x = head.max.x - size::NODE_HEAD_PAD_X - total;
    for (level, galley, w) in chips {
        let rect = Rect::from_min_size(
            Pos2::new(x, head.center().y - size::AUTH_H * 0.5),
            egui::vec2(w, size::AUTH_H),
        );
        let sel = level == authority;
        let colour = match sel {
            true => pal.mint,
            false => pal.faint,
        };
        if sel {
            painter.rect_filled(
                rect,
                // `border-radius: 999px` on a box this short is a capsule.
                CornerRadius::same((size::AUTH_H * 0.5) as u8),
                tint(pal.mint, 15),
            );
        }
        painter.galley(
            Pos2::new(
                rect.min.x + size::AUTH_PAD_X,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            colour,
        );
        x += w + size::AUTH_GAP;
    }
}

/// **The renderer chips**, one per renderer the Set has, from the left.
///
/// The live one carries `.rend.sel`: `color: var(--c-pink)` over a 15% wash of
/// it, no border, and the pink halo `box-shadow: 0 0 9px var(--c-glowp)` —
/// which is the same 9 the lit beat and a live fader knob carry. The rest are
/// `.rend`'s `border: 1px solid var(--c-line)` around `var(--c-dim)`.
///
/// **One row and what fits of it.** `.rend-row` wraps in the mock and the
/// console does not: a wrapped row is a group taller than [`group_h`] said it
/// was, and the pane's own arithmetic is what says whether a group is drawn at
/// all. A chip past the right-hand edge is clipped, which is the same answer
/// the pane gives a group past the bottom.
fn rend_row_into(painter: &egui::Painter, pal: &Palette, row: Rect, renderers: &[Renderer]) {
    let mut x = row.min.x + size::REND_ROW_PAD_L;
    let y = row.min.y + size::REND_ROW_PAD_T;
    for rend in renderers {
        let galley = painter.layout_no_wrap(
            rend.name.clone(),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.dim,
        );
        let w = galley.size().x + size::REND_PAD_X * 2.0;
        let rect = Rect::from_min_size(Pos2::new(x, y), egui::vec2(w, size::REND_H));
        let radius = CornerRadius::same((size::REND_H * 0.5) as u8);
        match rend.live {
            true => {
                painter.add(
                    egui::epaint::Shadow {
                        offset: [0, 0],
                        blur: size::TALLY_GLOW - 1,
                        spread: 0,
                        color: pal.glow_pink,
                    }
                    .as_shape(rect, radius),
                );
                painter.rect_filled(rect, radius, tint(pal.pink, 15));
            }
            false => {
                painter.rect_stroke(
                    rect,
                    radius,
                    Stroke::new(size::HAIRLINE, pal.line),
                    StrokeKind::Inside,
                );
            }
        }
        painter.galley(
            Pos2::new(
                rect.min.x + size::REND_PAD_X,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            match rend.live {
                true => pal.pink,
                false => pal.dim,
            },
        );
        x += w + size::REND_GAP;
    }
}

/// **One parameter row**, in the mock's own four tracks: the ordinal at
/// [`size::PARAM_ORD_W`] right-aligned, the name at [`size::PARAM_NAME_W`],
/// the fader taking what is left, and the value at [`size::PARAM_VAL_W`]
/// right-aligned.
///
/// **The value is two places**, which is the mock's `2.40`, `0.71`, `1.20` —
/// every number in its `.pval` column. It is not a second spelling of the
/// transport's tempo: that one is a BPM and this is a parameter, and the mock
/// writes the two differently for that reason.
fn param_into(painter: &egui::Painter, pal: &Palette, row: Rect, param: &Param) {
    let left = row.min.x + size::PARAM_PAD_L;
    let right = row.max.x - size::PARAM_PAD_R;
    let ord = painter.layout_job(span_at(
        &param.ord.to_string(),
        size::PARAM_ORD_SIZE,
        pal.faint,
    ));
    painter.galley(
        Pos2::new(
            left + size::PARAM_ORD_W - ord.size().x,
            row.center().y - ord.size().y * 0.5,
        ),
        ord,
        pal.faint,
    );
    let name_x = left + size::PARAM_ORD_W + size::PARAM_GAP;
    // `.param .pname`'s `overflow: hidden; text-overflow: ellipsis` — one row,
    // broken anywhere, with an ellipsis for what did not fit. The mock says so
    // for this column and not for the library's, which is why one elides and
    // the other clips.
    let mut job = span_at(&param.name, size::BASE, pal.dim);
    job.wrap = egui::epaint::text::TextWrapping {
        max_width: size::PARAM_NAME_W,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let name = painter.layout_job(job);
    painter.galley(
        Pos2::new(name_x, row.center().y - name.size().y * 0.5),
        name,
        pal.dim,
    );
    let value = painter.layout_job(span_at(
        &format!("{:.2}", param.value),
        size::BASE,
        pal.text,
    ));
    painter.galley(
        Pos2::new(
            right - value.size().x,
            row.center().y - value.size().y * 0.5,
        ),
        value,
        pal.text,
    );
    let track = Rect::from_min_size(
        Pos2::new(
            name_x + size::PARAM_NAME_W + size::PARAM_GAP,
            row.center().y - size::FADER_H * 0.5,
        ),
        egui::vec2(
            (right - size::PARAM_VAL_W - size::PARAM_GAP)
                - (name_x + size::PARAM_NAME_W + size::PARAM_GAP),
            size::FADER_H,
        ),
    );
    if positive(track) {
        // `.fader b` fills its 5px track edge to edge, so the inset is zero —
        // the one argument that tells this fader from the mixer's vertical
        // one, which `fader` takes for exactly this reason.
        fader_into(
            painter,
            pal,
            fader(
                track,
                Axis::Row,
                param.at,
                0.0,
                egui::vec2(size::FADER_KNOB_W, size::FADER_KNOB_H),
            ),
            false,
            None,
        );
    }
}

/// The console's view: which room it is in, and the frame's plan, kept so a
/// frame does not allocate one.
pub struct View {
    pub room: Room,
    /// **What to draw in the Program bay's picture this frame**, or `None` for
    /// a console with no engine behind it — which is every test in this crate
    /// and the whole of what `cargo test -p karakuri-console` sees.
    ///
    /// Set per frame by whoever owns the device, because that is who knows
    /// whether the texture it names is still the right size. A stale id here
    /// is a freed registration, so it is written beside the frame that made it
    /// rather than kept.
    pub picture: Option<Picture>,
    /// **What to draw in each of the four deck preview cells this frame**, in
    /// slot order, or `None` for a cell whose deck is off.
    ///
    /// **The same seam as [`View::picture`], four times over**: registering a
    /// texture takes a device, this crate has none, so whoever owns the device
    /// registers them and writes this per frame beside the frame that made
    /// them. A stale id here is a freed registration. Every test in this crate
    /// leaves every entry `None`, which is what `cargo test -p
    /// karakuri-console` sees and is a console with no engine behind it.
    ///
    /// **All `None` is a state, not an absence.** The manual's *previews 2 of
    /// 4* is an operator choosing how many decks audition, so a cell with
    /// nothing in it is a cell that is off and it is drawn saying so — see the
    /// module documentation, and [`preview_rects`] for where the rectangles
    /// come from.
    pub previews: [Option<Picture>; DECKS],
    /// **What the transport row reads this frame**, or `None` for a console
    /// with no engine behind it — which is every test in this crate, and what
    /// the row draws then is nothing at all.
    ///
    /// **The same seam as [`View::picture`]**, one row up and without a
    /// device: the tempo, the beat and the frame's cost are a clock and an
    /// engine, and `src/` has neither (ADR-0156). So whoever owns them reads
    /// them and writes this per frame, beside the frame that measured it.
    /// See [`Transport`] and [`transport`].
    pub transport: Option<Transport>,
    /// **What the arrangement pill in that row reads, and what its menu is
    /// doing.**
    ///
    /// **Half of it is the same seam as [`View::transport`] and half of it is
    /// not**, which is the one field here that is both. The name in use and
    /// the names filed are a file and a directory, so whoever owns the store
    /// reads them and writes them here — [`View::library`]'s seam, one row up.
    /// The menu is this crate's own and moves only through
    /// [`Arrangement`]'s methods: what the *control* is doing is not something
    /// the program can be the model of record for.
    ///
    /// [`Arrangement::NONE`] until somebody says otherwise, which is every
    /// test in this crate and is a console with no store behind it: the
    /// default arrangement, nothing filed, and the menu shut. See
    /// [`Arrangement`] and [`arrangement`].
    pub arrangement: Arrangement,
    /// **What the two look controls in that row read this frame**, or `None`
    /// for a console with no engine behind it — which is every test in this
    /// crate that does not hand one in, and what the row draws there is
    /// nothing at all.
    ///
    /// **The same seam as [`View::transport`]**, two items along the same row:
    /// the operator and the level are `karakuri_engine::frame::Look`, `src/`
    /// has no engine (ADR-0156), so whoever owns one reads it and writes this
    /// per frame beside the frame it was drawn under.
    ///
    /// It is a second field rather than two more fields on [`Transport`]
    /// because it is a different reading of a different thing: a tempo and a
    /// frame cost are what the *session* is doing, and a look is what the
    /// picture is being put through. `transport` answering `None` and this
    /// answering `None` are two facts, and a console driving one and not the
    /// other is a state the type should be able to say. See [`Look`] and
    /// [`look`].
    pub look: Option<Look>,
    /// **What each mixer strip reads this frame**, one per slot the deck has,
    /// in slot order — and **empty** for a console with no deck behind it,
    /// which is every test in this crate and what the bay draws then is
    /// nothing at all.
    ///
    /// **The same seam as [`View::transport`]**, and empty rather than
    /// `Option<Vec<_>>` because an empty list of strips is already the whole
    /// of *no deck*: a deck has one to four slots (`MAX_SLOTS`, asserted in
    /// `Deck::new`), so there is no deck that has none and no second way to
    /// say it.
    ///
    /// A `Vec` a caller keeps and rewrites, rather than a fixed array of
    /// `Option`s like [`View::previews`]: a preview cell is off or on and the
    /// row is always four, where the strips are *as many as the deck has* and
    /// a `None` in the middle of them would be a slot no `Deck` can have.
    /// [`View::new`] gives it room for [`DECKS`] so the frame path never grows
    /// it. See [`Strip`] and [`mixer`].
    pub mixer: Vec<Strip>,
    /// **What the Library bay lists this frame**: the name of every Set the
    /// store holds, in the order the store listed them — and **empty** for a
    /// console with no store behind it, which is every test in this crate and
    /// what the bay draws then is nothing at all.
    ///
    /// **The same seam as [`View::mixer`]**, and empty rather than
    /// `Option<Vec<_>>` for the same reason: an empty listing is already the
    /// whole of *no library*, and a store that holds nothing and a store that
    /// is not there are the same bay — one with no row to draw.
    ///
    /// **Read once rather than per frame**, by whoever owns the store. A
    /// listing is a directory read, which is not a thing to do on a frame
    /// path (P-0072), and nothing in this crate can do it anyway: opening a
    /// store is `karakuri-store`'s and `src/` depends on neither it nor the
    /// engine (ADR-0156). What crosses the seam is a list of names.
    ///
    /// A name and nothing else, because a name is what exists: see [`library`]
    /// for the star and the time the mock draws beside it, and for why neither
    /// is here.
    pub library: Vec<String>,
    /// **What each Inspector pane is showing this frame**, one per pane the
    /// console has room to point at something — and **empty** for a console
    /// with no deck behind it, which is every test in this crate and what the
    /// bay draws then is its card, its head and the bar between its panes.
    ///
    /// **The same seam as [`View::mixer`]**, and empty rather than
    /// `[Option<Pane>; PANES]` for the same reason: an empty list is already
    /// the whole of *no deck*, and a pane pointed at nothing and a pane that
    /// is not there are one bay — one with no head to draw.
    ///
    /// **A pane is *pointed*, not choosing.** The mock's `showing … ▾` is a
    /// control and this pass adds none (ADR-0200), so which deck each pane
    /// shows is whoever fills this saying so — and pane *i* draws
    /// `inspector[i]`, in [`PANE_NAMES`]' order.
    ///
    /// **Read once rather than per frame**, which is
    /// `karakuri_engine::set::Set::published`'s own instruction — *"Allocates,
    /// so not the frame path. A console reads this when a Set lands, not per
    /// frame."* — and is [`View::library`]'s rule for a second reason: what is
    /// in here does not move. Nothing in this workspace writes a published
    /// value, binds a signal in the panel binary, or grants an authority
    /// (ADR-0216), so a Set that has landed reads the same on every frame
    /// after it. **The field is rewritable per frame like every other one**;
    /// what a re-read waits on is a writer, and it is the same writer the
    /// `man / sug / auto` chip waits on. See [`Pane`] and [`inspector`].
    pub inspector: Vec<Pane>,
    /// **The shape of what is being rendered**, which is what the Program bay
    /// arranges its body for — [`program_bay`], and [`picture_rect`] for why
    /// it is two numbers rather than a ratio.
    ///
    /// **The same seam as [`View::picture`], and it belongs beside it**: the
    /// picture's rectangle is handed in by whoever built the `Present`, and
    /// this is the number that rectangle was derived from. Written per frame
    /// by that same caller and in the same breath, so that the canvas the
    /// texture was sized for and the canvas the bay arranged itself for cannot
    /// be two different numbers — which would put the cells in one arrangement
    /// and the picture in the other.
    ///
    /// [`MOCK_CANVAS`] until somebody says otherwise, which is every test in
    /// this crate and is a console with no engine behind it: there is no
    /// picture to draw, and the four cells still have to go somewhere.
    pub canvas: (u32, u32),
    /// **How long the panel has been animating**, written per frame by
    /// whoever has the clock — see [`Phase`], which carries the whole
    /// argument for why this is a value and not an `Instant`.
    ///
    /// **The same seam as [`View::transport`]**, and the one this crate is
    /// least able to cross: a clock in `src/` is P-0002 broken in the file
    /// whose own doc says so. [`Phase::ZERO`] until somebody says otherwise,
    /// which is every test in this crate and is a console with no clock behind
    /// it — a panel drawn at the origin of every animation on it.
    pub phase: Phase,
    placed: Vec<Placed>,
}

impl View {
    pub fn new(room: Room) -> View {
        View {
            room,
            picture: None,
            previews: [None; DECKS],
            transport: None,
            // The default arrangement, nothing filed and the menu shut, which
            // is every test in this crate and is a console with no store
            // behind it.
            arrangement: Arrangement::NONE,
            // No engine behind the console, so there is no look to draw — the
            // transport's own answer, one group along.
            look: None,
            // As many strips as a deck can ever have, so the frame path never
            // grows it — the same reason `placed` is built with a capacity.
            mixer: Vec::with_capacity(DECKS),
            // Nothing until somebody lists a store, which is every test in
            // this crate. No capacity is reserved: how many Sets a store holds
            // is not a number this crate has, and the list is written once
            // rather than per frame.
            library: Vec::new(),
            // As many panes as the inspector has, so the frame path never
            // grows it — the same reason `mixer` is built with a capacity.
            inspector: Vec::with_capacity(PANES),
            canvas: MOCK_CANVAS,
            phase: Phase::ZERO,
            // Every region the console has, so the frame path never grows it.
            placed: Vec::with_capacity(REGIONS.len()),
        }
    }

    /// **Every live region that is declaring this frame**, each with what one
    /// update of it costs and how stale it may get.
    ///
    /// # This is P-0072's naming, and the unit is a region
    ///
    /// [P-0072](../../../docs/principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md):
    /// *"What must be live during a performance is named, and each named thing
    /// declares two numbers: what its update costs, and how stale it may get,
    /// in milliseconds."* This is that naming, and [`crate::budget`] holds the
    /// numbers with the arguments for where each came from. **A region is
    /// redrawn whole or not at all**, so what appears here is a node of the
    /// arrangement — by the name every surface addresses it by — and never an
    /// animation, a control or a slice of a frame.
    ///
    /// # Two regions, at two rates, for two different reasons
    ///
    /// **The transport row**, whenever the beat grid is drawn: the light
    /// travels the grid once a bar and it is the panel's continuous motion,
    /// which [P-0077](../../../docs/principles/0077-continuous-motion-is-how-a-stopped-panel-announces-itself.md)
    /// says is how a stopped panel announces itself. It declares
    /// [`BEAT_STALENESS`] and **it does not ask whether anything is pending**,
    /// which is the whole point of it: a signal that only ran while something
    /// was happening would be quiet exactly when the panel had gone quiet.
    ///
    /// **The mixer bay**, while something in it is outstanding. Inside it the
    /// tally's word rolls toward a residency that has not been granted and
    /// each of a strip's two faders reaches toward a value a transition has
    /// not reached yet — three presentations at one rate, off one [`Phase`],
    /// so they are one term and not three
    /// ([ADR-0190](../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md),
    /// [ADR-0206](../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)).
    /// A second *rate* would be a second declaration; a second *user* of one
    /// rate is not — which is why the beat is a second entry here and the two
    /// faders are not.
    ///
    /// **This is the first frame on which either sum has two terms**, and
    /// `tests/schedulable.rs` reads them: `Σ (cost / staleness)` is 0.0889
    /// against 1.0, and `max(cost)` is still one number because both regions
    /// declare the same whole panel pass (ADR-0210).
    ///
    /// # Pending is not enough: the region that shows it has to be laid out
    ///
    /// **A region that is not on screen declares nothing**, however much is
    /// pending behind it. The strips are rewritten every frame from the deck,
    /// so *is anything pending* is a fact about the deck; P-0072 is about what
    /// **must be live**, and a bay the operator has folded away is not live.
    /// A declaration made for it buys a repaint of something nobody can see —
    /// measured, before this asked: with the picture and the preview row
    /// folded the window sat at 28.7 to 29.0 frames a second, and folding the
    /// mixer bay on top of that moved the price of a frame and not the rate.
    /// It draws nothing at all now. The alternative — declare it anyway and
    /// let a scheduler drop it — is
    /// [ADR-0193](../../../docs/adr/0193-a-region-that-is-not-laid-out-declares-nothing-rather-than-being-dropped-later.md),
    /// which is where it lost, and it lost on the sentence being **false**
    /// rather than unaffordable: a region nobody can see is not showing
    /// anything, so it cannot be showing anything out of date.
    ///
    /// This is
    /// [P-0073](../../../docs/principles/0073-a-node-claims-only-what-its-visible-content-can-use.md)
    /// in time rather than in space — *a node claims only what its visible
    /// content can use*, where what is claimed is a share of the frame budget
    /// rather than a share of the viewport.
    ///
    /// **The question is asked of the layout, which already answers it.**
    /// [`Layout::visible`](karakuri_layout::Layout::visible) is ADR-0183's
    /// disjunction — the operator's fold and the drawing's `set_aside`, read
    /// as one — walked up the ancestors, so a folded right pane takes the
    /// mixer with it and no second derivation of *is this laid out* is written
    /// here. It is **not** [`mixer`]'s `None`, which is a different question
    /// and a stricter one: that answers *can the strips be laid out in this
    /// rectangle this pass*, and says no for a window merely too small and for
    /// the frame before `egui` has fonts. Neither is a reason to stop the
    /// roll — only being out of the layout is — and the layout answers this
    /// one without a solve, which matters because the fold is applied and this
    /// is asked before the next one.
    ///
    /// # Nothing arbitrates between two of these
    ///
    /// P-0072's second half is a scheduler and there is not one. What this
    /// feeds is [`View::animating`], which takes the soonest staleness and
    /// nothing else, and `tests/schedulable.rs`, which sums over whatever this
    /// answers and asserts the two conditions the principle states.
    pub fn declares(&self, layout: &karakuri_layout::Layout) -> impl Iterator<Item = Declared> {
        // An array rather than a `Vec`, so asking what the panel declares
        // allocates nothing on a path that is walked every frame — and so that
        // the second live region was one more element rather than a change of
        // shape, which is what it turned out to be. In `REGIONS`' order, so
        // the declarations read down the panel.
        [self.transport_declares(layout), self.mixer_declares(layout)]
            .into_iter()
            .flatten()
    }

    /// **What the transport row declares**: the beat's staleness for as long
    /// as the grid is being drawn, and nothing when it is not.
    ///
    /// # Two conditions, and neither of them is *pending*
    ///
    /// **The row is laid out**, which is ADR-0193 asked of a row rather than
    /// of a bay — [`Layout::visible`](karakuri_layout::Layout::visible)
    /// answers for `transport` exactly as it answers for `mixer`, and a folded
    /// row is not showing a beat, so it cannot be showing one out of date.
    /// The transport is a direct child of the unnamed root, so the only thing
    /// that encloses it is the root itself (ADR-0204).
    ///
    /// **There are values behind it.** [`View::transport`] is `None` for a
    /// console with no engine, and [`transport`] draws no row at all then —
    /// there is no light on a grid that is not there. That is the same seam as
    /// [`View::picture`] and not a second rule.
    ///
    /// **Nothing else is asked, and that is the declaration's whole content.**
    /// P-0077's forced clause is that *something is moving continuously while
    /// the console is live*, so a beat that declared only while something was
    /// pending would be the signal going quiet at the moment it is worth
    /// having. It is also why this is not folded into
    /// [`View::mixer_declares`]: two rates, two deadlines, two regions.
    fn transport_declares(&self, layout: &karakuri_layout::Layout) -> Option<Declared> {
        let row = layout
            .find("transport")
            .is_some_and(|id| layout.visible(id));
        (row && self.transport.is_some()).then_some(Declared {
            region: "transport",
            cost: PANEL_PASS,
            staleness: BEAT_STALENESS,
        })
    }

    /// **What the mixer bay declares**: the roll's staleness while anything in
    /// it is pending *and* the bay is laid out, and nothing otherwise.
    ///
    /// Three pending things, one rate — see [`View::declares`], which carries
    /// the whole argument.
    fn mixer_declares(&self, layout: &karakuri_layout::Layout) -> Option<Declared> {
        let bay = layout.find("mixer").is_some_and(|id| layout.visible(id));
        let moving = bay
            && self.mixer.iter().any(|strip| {
                strip.pending().is_some()
                    || strip.gain_pending().is_some()
                    || strip.opacity_pending().is_some()
            });
        moving.then_some(Declared {
            region: "mixer",
            cost: PANEL_PASS,
            staleness: ROLL_STALENESS,
        })
    }

    /// **The soonest staleness any live region on this panel will tolerate**,
    /// and `None` when nothing on it is moving.
    ///
    /// # The view is what knows the rate, so the harness is told rather than
    /// guessing
    ///
    /// [`View::declares`] is where the regions and their two numbers are, and
    /// this is the one of the two numbers a window can act on today:
    /// `crate::repaint::Change::Animating` turns it into a deadline. A rate
    /// written into the harness instead would be a presentation's number kept
    /// where the presentation is not — change the roll and the window goes on
    /// servicing the old one, with nothing failing to compile and nothing to
    /// assert against.
    ///
    /// **The soonest, not the sum**, and that is the whole of what is decided
    /// here: a deadline is met by drawing, and one frame drawn in time for the
    /// soonest is in time for every other. Choosing which region a frame is
    /// *for* is a scheduler's, and there is not one.
    ///
    /// # `None` is the panel saying it is still, and that is now a narrower
    /// state than *nothing pending*
    ///
    /// `None` is not an absence of information: it is the panel saying it is
    /// still, and the window then sleeps. **A console with no parked slot and
    /// no scheduled move on a fader costs exactly what it cost before either
    /// existed**, which is a claim `tests/parked.rs` and `tests/armed.rs` make
    /// rather than a hope — and both of them ask it of a console with no
    /// engine behind it, which is what every test in this crate is.
    ///
    /// **With an engine behind it and the transport row on screen this never
    /// answers `None`**, because the beat is moving and says so
    /// ([`View::transport_declares`],
    /// [P-0077](../../../docs/principles/0077-continuous-motion-is-how-a-stopped-panel-announces-itself.md)).
    /// That is P-0072's first clause narrowing rather than failing: a panel
    /// with something moving on it is a panel with something changing on it,
    /// and the reason it is moving is a declaration rather than an accident.
    pub fn animating(&self, layout: &karakuri_layout::Layout) -> Option<Duration> {
        self.declares(layout).map(|live| live.staleness).min()
    }

    /// Draw the whole console. The `ui` is the root one
    /// [`egui::Context::run_ui`] hands the frame's closure.
    pub fn draw(&mut self, ui: &mut Ui, panel: &mut Panel) {
        let pal = self.room.palette();
        // **The bay arranges itself before anything else this frame reads the
        // layout**, the cursor below included: the boundary between the
        // picture and the row is not there while the cells are beside the
        // picture, and a resize cursor drawn from the solve before the
        // rearrangement is a cursor for a divider that has gone.
        plan_into(panel, self.canvas, &mut self.placed);
        self.cursor(ui.ctx(), panel);

        // **Where the four cells go, asked once and off the same derivation
        // the picture came through.** Not from `placed`: beside the picture
        // the row is set aside, so `deck-previews` is not in the plan at all
        // and a cell drawn from its entry would be a cell drawn in one of the
        // two arrangements only.
        let cells = program_bay(panel.layout(), self.canvas).and_then(|bay| bay.cells);
        let picture = self.picture;
        let previews = self.previews;
        let values = self.transport;
        let arr = &self.arrangement;
        let look_at = self.look;
        let strips = self.mixer.as_slice();
        let sets = self.library.as_slice();
        let panes = self.inspector.as_slice();
        let phase = self.phase;
        let frame = egui::Frame::NONE.fill(pal.ground);
        egui::CentralPanel::default().frame(frame).show(ui, |ui| {
            for placed in &self.placed {
                let rect = to_egui(placed.rect);
                match placed.region.kind {
                    Kind::Bay { title, pills, grip } => {
                        card(ui, &pal, rect);
                        bay_head(ui, &pal, rect, title, pills, grip);
                        // The inspector is the one bay that is a split, and
                        // its panes' boundary is drawn as the mock's
                        // `.divider-v` rather than left as bare ground: it is
                        // inside a card, where the ground does not reach.
                        pane_dividers(ui, &pal, panel, placed.id, rect);
                    }
                    // The other row with something in it, and everything in it
                    // is a readout: where each one goes is `transport`'s
                    // answer, and with no engine behind the console there is
                    // no answer and the card is as empty as every other body
                    // in this pass.
                    Kind::Transport => {
                        card(ui, &pal, rect);
                        if let Some(row) = transport(ui.ctx(), panel.layout(), values) {
                            transport_into(ui, &pal, &row);
                        }
                        // **The look's two controls, inside the row they
                        // belong to.** Unlike the arrangement pill this has
                        // nothing that hangs out of the row, so it is painted
                        // here rather than after the loop — and the pill's
                        // menu, which does hang out, is painted after and
                        // therefore over it, which is the order a card that is
                        // above the bays wants. Where it goes is `look`'s
                        // answer and not this pass's: the same call
                        // `input::claim` makes.
                        if let Some(row) = look(ui.ctx(), panel.layout(), values, arr, look_at) {
                            look_into(ui, &pal, &row);
                        }
                    }
                    // The one row with something in it, and the something is
                    // one control. Where it goes is `outputs`'s answer and
                    // not this pass's: the same call `input::claim` makes, so
                    // the chip that is painted is the chip that is clicked.
                    Kind::Outputs => {
                        card(ui, &pal, rect);
                        if let Some(row) = outputs(ui.ctx(), panel.layout()) {
                            outputs_into(ui, &pal, &row);
                        }
                    }
                    // The one bay with something in its body, and it is a bay
                    // in every other respect: the same card and the same head
                    // the other six get, and then as many strips as the deck
                    // has. With no deck behind the console there are none and
                    // the body is as empty as every other one in this pass —
                    // `mixer`'s answer, not this pass's, so that *no deck
                    // means nothing at all* is decided in one place.
                    Kind::Mixer => {
                        card(ui, &pal, rect);
                        bay_head(ui, &pal, rect, MIXER_TITLE, &[], false);
                        if let Some(bay) = mixer(ui.ctx(), panel.layout(), strips) {
                            mixer_into(ui, &pal, &bay, phase);
                        }
                    }
                    // The other bay with something in its body, and it is a
                    // bay in every other respect: the same card and the same
                    // head, and then as many rows as the bay has room for.
                    // With no store behind the console there are none and the
                    // body is as empty as every other one in this pass —
                    // `library`'s answer, not this pass's, so that *no store
                    // means nothing at all* is decided in one place.
                    Kind::Library => {
                        card(ui, &pal, rect);
                        bay_head(ui, &pal, rect, LIBRARY_TITLE, &[], true);
                        if let Some(bay) = library(panel.layout(), sets) {
                            library_into(ui, &pal, &bay, sets);
                        }
                    }
                    // A pane draws nothing of its own. It has no card — it is
                    // inside the bay's — and no head, and its body is as empty
                    // as every other body in this pass.
                    Kind::Pane => {}
                    // The one body that is not empty, because its texels are
                    // not this crate's to invent: a texture is there or it is
                    // not, and where it is not the bay's card shows through
                    // exactly as every other empty body does. No placeholder,
                    // for the reason this module's documentation gives.
                    Kind::Picture => {
                        if let Some(picture) = picture {
                            // Clipped to the region: the rectangle came from
                            // outside, and a stale one is a picture painted
                            // over the inspector rather than a wrong picture.
                            ui.painter().with_clip_rect(rect).image(
                                picture.id,
                                picture.rect,
                                WHOLE_TEXTURE,
                                Color32::WHITE,
                            );
                        }
                    }
                    // **The one arm that draws nothing, and it is not an
                    // empty body.** The four cells are the bay's and not this
                    // region's: they are drawn below, once, from wherever
                    // `program_bay` put them — which is under the picture on a
                    // narrow bay and down the sides of a wide one, where this
                    // region has no extent and no entry in the plan at all. A
                    // cell drawn from here would be a cell drawn in one of the
                    // two arrangements only.
                    Kind::Previews => {}
                }
            }

            // **The deck previews, wherever they went.** Painted after the
            // loop rather than in it for the reason the arm above gives, and
            // painting last costs nothing: they sit inside the Program bay's
            // card, which the loop has already painted, and nothing else on
            // the panel overlaps them. A cell is the region's own face rather
            // than a placeholder — the module documentation is where that
            // argument is.
            if let Some(cells) = cells {
                for (deck, cell) in cells.into_iter().enumerate() {
                    preview(ui, &pal, cell, deck, previews[deck]);
                }
            }

            // **The Inspector's panes, after the loop for the same reason.**
            // `Kind::Pane` is a unit variant and stays one — `tests/view.rs`
            // asserts that both panes are one — so it cannot carry *which*
            // pane, and comparing `inspector-1` against a name on the frame
            // path is the string in the loop this table exists to avoid. They
            // sit inside the Inspector bay's card, which the loop has already
            // painted, and nothing else on the panel overlaps them.
            for (index, pane) in panes.iter().enumerate().take(PANES) {
                if let Some(at) = inspector(panel.layout(), index, pane) {
                    inspector_into(ui, &pal, &at, pane);
                }
            }

            // **The arrangement pill, and its menu over everything.** Painted
            // after the loop for a reason the two above do not have: the menu
            // hangs *out* of the row it belongs to and down over the bays, so
            // a card painted from the `Kind::Transport` arm would go on before
            // the Library bay and end up under it. The pill itself is inside
            // its row and would not care; the two halves of one control are
            // one call, and where it goes is `arrangement`'s answer — the same
            // call `input::claim` makes, so the pill that is painted is the
            // pill that is clicked.
            if let Some(pill) = arrangement(ui.ctx(), panel.layout(), values, arr) {
                arrangement_into(ui, &pal, &pill, arr);
            }
        });
    }

    /// Say what is under the pointer.
    ///
    /// **`egui` sets it, not `winit`.** `egui_winit`'s
    /// `handle_platform_output` already writes the window's cursor from
    /// `PlatformOutput` every frame, so a `Window::set_cursor` call beside it
    /// is a second writer and the last one each frame wins — which is a
    /// flicker that depends on event order. One writer, and it is the one that
    /// is already there.
    fn cursor(&self, ctx: &egui::Context, panel: &mut Panel) {
        panel.solve();
        // A boundary in hand keeps the resize cursor even where the pointer
        // has run off it, for the reason `input`'s rule 1 keeps the events:
        // the gesture is what is happening, not the position.
        //
        // **And a gesture that is not a boundary drag suppresses it**, for the
        // same reason and in the other direction. Falling through to the hit
        // test with a fader in hand would flick a resize cursor on the moment
        // a fader held against its top let the pointer wander across a
        // boundary, which is a cursor for a gesture that is not happening.
        //
        // There is no cursor of its own for a fader: the console's whole
        // cursor vocabulary is *arrow, or resize over a boundary*, and
        // inventing a third mark here would be one control saying something no
        // other one on the panel says.
        let axis = match panel.in_hand() {
            Some(InHand::Boundary(axis)) => Some(axis),
            Some(InHand::Fader) => None,
            None => match panel.layout().hit(panel.cursor(), GRAB) {
                Hit::Divider { split, .. } => panel.layout().axis(split),
                _ => None,
            },
        };
        match axis {
            Some(Axis::Row) => ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal),
            Some(Axis::Column) => ctx.set_cursor_icon(egui::CursorIcon::ResizeVertical),
            None => {}
        }
    }
}

/// **One deck preview cell**: the mock's `.preview`, with whatever is behind
/// it and the letter that says which deck it is.
///
/// Painted the same whether a deck is running in it or not, because *off* is a
/// state an operator chooses rather than a thing not built yet — the module
/// documentation is where that argument is written out.
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
/// - `display: flex; align-items: flex-end; padding: 3px 5px;
///   font-size: 9px` — the letter in the bottom-left corner, inside that
///   padding, at [`size::PREVIEW_SIZE`].
///
/// # The two colours the label takes, and what each is standing for
///
/// - **A picture behind it:** `.preview.a` and `.preview.b` set
///   `color: rgba(255,255,255,0.7)`, which is white because the mock's cell is
///   always a dark gradient. The console's cell holds real texels of unknown
///   brightness in either room, and there is no *white at 70%* in the palette
///   — so this is `pal.text`, `--c-text`, the mock's own colour for a value.
///   It follows the room, which is the one thing the mock's literal white
///   cannot do.
/// - **Nothing behind it:** `.preview`'s own `color: var(--c-faint)`, which is
///   `pal.faint` — *"a heading, and anything switched off"*, and this cell is
///   switched off.
///
/// And the word follows the colour: the mock writes `C &middot; off` in a cell
/// with nothing in it, so a cell that is off says `off` rather than leaving
/// the reader to tell a dark thumbnail from an empty well.
fn preview(ui: &Ui, pal: &Palette, cell: Rect, deck: usize, picture: Option<Picture>) {
    let radius = CornerRadius::same(size::PREVIEW_RADIUS as u8);
    // Clipped to the cell for the reason the picture is clipped to its region:
    // the rectangle in `picture` came from outside, and a stale one is a
    // thumbnail painted across the bay rather than a wrong thumbnail.
    let painter = ui.painter().with_clip_rect(cell);
    painter.rect_filled(cell, radius, pal.well);

    if let Some(picture) = picture {
        painter.image(picture.id, picture.rect, WHOLE_TEXTURE, Color32::WHITE);
    }
    painter.rect_stroke(
        cell,
        radius,
        Stroke::new(size::HAIRLINE, pal.hair),
        StrokeKind::Inside,
    );

    let letter = DECK_LETTERS[deck];
    let (label, colour) = match picture {
        Some(_) => (letter.to_owned(), pal.text),
        None => (format!("{letter} · off"), pal.faint),
    };
    let galley = painter.layout_no_wrap(
        label,
        FontId::new(size::PREVIEW_SIZE, FontFamily::Proportional),
        colour,
    );
    painter.galley(
        Pos2::new(
            cell.min.x + size::PREVIEW_PAD_X,
            cell.max.y - size::PREVIEW_PAD_Y - galley.size().y,
        ),
        galley,
        colour,
    );
}

/// **The Outputs row's contents**: the word, and the one `.sink`.
///
/// Where everything goes is [`outputs`]'s, so this paints and derives nothing.
/// Term for term from `style.css`:
///
/// - `.sink` — `background: var(--c-well)`, `color: var(--c-dim)`, and
///   `border-radius: 999px`, which on a box this short is a capsule.
/// - `.sink.on` — `color: var(--c-text)` over
///   `color-mix(in srgb, var(--c-mint) 14%, transparent)`, which is
///   `pal.mint` at 14% alpha exactly (see `room`'s documentation).
/// - `.dot` — `var(--c-faint)` off, and `var(--c-mint)` with
///   `box-shadow: 0 0 8px var(--c-glow)` on. The halo is an
///   [`egui::epaint::Shadow`] with the blur the CSS names and a corner radius
///   of half the dot, which is a blurred circle — the same mechanism the bay's
///   card draws its drop shadow with, and the first thing in this crate to use
///   `--c-glow`, which `room` transcribed against the day a control was drawn.
fn outputs_into(ui: &Ui, pal: &Palette, row: &Outputs) {
    let painter = ui.painter();

    // The word: `.bay-head`'s type on a row that has no head — see
    // `OUTPUTS_LABEL`.
    let galley = painter.layout_job(label_job(pal.faint));
    painter.galley(row.label.min, galley, pal.faint);

    let (ink, fill, dot) = match row.on {
        true => (pal.text, tint(pal.mint, 14), pal.mint),
        false => (pal.dim, pal.well, pal.faint),
    };
    painter.rect_filled(
        row.sink,
        CornerRadius::same((size::SINK_H * 0.5) as u8),
        fill,
    );
    if row.on {
        painter.add(
            egui::epaint::Shadow {
                offset: [0, 0],
                blur: 8,
                spread: 0,
                color: pal.glow,
            }
            .as_shape(row.dot, CornerRadius::same((size::SINK_DOT * 0.5) as u8)),
        );
    }
    painter.circle_filled(row.dot.center(), size::SINK_DOT * 0.5, dot);

    let galley = painter.layout_no_wrap(
        PROGRAM_VIEW.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        ink,
    );
    painter.galley(
        Pos2::new(
            row.dot.max.x + size::SINK_GAP,
            row.sink.center().y - galley.size().y * 0.5,
        ),
        galley,
        ink,
    );
}

/// A bay's card: `.bay`'s panel fill, 11px radius and drop shadow.
fn card(ui: &Ui, pal: &Palette, rect: Rect) {
    let radius = CornerRadius::same(size::BAY_RADIUS as u8);
    ui.painter().add(pal.shadow.as_shape(rect, radius));
    ui.painter().rect_filled(rect, radius, pal.panel);
}

/// **The bay head, and the whole of what one is.**
///
/// A title on the left, the bay's own controls on the right, and a grip at the
/// far right where the mock draws one — laid out into the top
/// [`size::HEAD_H`] of `rect`, with `.bay-head`'s hairline under it.
///
/// Seven call sites on the day it is written, which is [`REGIONS`]'s seven
/// bays.
///
/// The pills and the grip are laid out **right to left** from the right edge,
/// which is what `justify-content: space-between` on a two-child flex row
/// comes to: the title takes the left and the group takes the right, and the
/// group's own order is its writing order once it is placed.
pub fn bay_head(
    ui: &Ui,
    pal: &Palette,
    rect: Rect,
    title: &str,
    pills: &[&str],
    grip: bool,
) -> Rect {
    let head = Rect::from_min_max(
        rect.min,
        Pos2::new(rect.max.x, (rect.min.y + size::HEAD_H).min(rect.max.y)),
    );
    let painter = ui.painter().with_clip_rect(head);

    // `border-bottom: 1px solid var(--c-hair)`.
    let rule = head.max.y - size::HAIRLINE * 0.5;
    painter.line_segment(
        [Pos2::new(head.min.x, rule), Pos2::new(head.max.x, rule)],
        Stroke::new(size::HAIRLINE, pal.hair),
    );

    let mid = head.center().y;
    let mut right = head.max.x - size::HEAD_PAD_X;

    if grip {
        right -= grip_dots(ui, pal, Pos2::new(right, mid));
        right -= size::PILL_GAP;
    }
    for pill in pills.iter().rev() {
        right -= pill_at(ui, pal, Pos2::new(right, mid), pill);
        right -= size::PILL_GAP;
    }

    // `text-transform: uppercase` plus `letter-spacing: 0.16em`, at
    // `--c-faint`. `egui`'s default proportional face has no bold, so
    // `font-weight: 700` is not honoured — see `room`'s documentation.
    let job = spaced(
        &title.to_uppercase(),
        size::HEAD_SIZE,
        pal.faint,
        size::HEAD_TRACKING,
    );
    let galley = painter.layout_job(job);
    painter.galley(
        Pos2::new(head.min.x + size::HEAD_PAD_X, mid - galley.size().y * 0.5),
        galley,
        pal.faint,
    );

    head
}

/// One `.pill`, right-aligned to `right`. Returns its width, so a caller
/// laying a row of them out right to left can step back by it.
fn pill_at(ui: &Ui, pal: &Palette, right: Pos2, text: &str) -> f32 {
    let painter = ui.painter();
    let galley = painter.layout_no_wrap(
        text.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.dim,
    );
    let w = galley.size().x + size::PILL_PAD_X * 2.0;
    let rect = Rect::from_min_size(
        Pos2::new(right.x - w, right.y - size::PILL_H * 0.5),
        egui::vec2(w, size::PILL_H),
    );
    painter.rect_stroke(
        rect,
        // `border-radius: 999px` on a box this short is a capsule.
        CornerRadius::same((size::PILL_H * 0.5) as u8),
        Stroke::new(1.0, pal.line),
        StrokeKind::Inside,
    );
    painter.galley(
        Pos2::new(
            rect.min.x + size::PILL_PAD_X,
            right.y - galley.size().y * 0.5,
        ),
        galley,
        pal.dim,
    );
    w
}

/// The mock's `.grip`, `⋮⋮` — drawn rather than typed, because whether a
/// vertical ellipsis is in `egui`'s default face is a question with no good
/// answer and six dots is the same mark either way. Returns its width.
fn grip_dots(ui: &Ui, pal: &Palette, right: Pos2) -> f32 {
    const COLS: usize = 2;
    const ROWS: usize = 3;
    const R: f32 = 1.0;
    const STEP: f32 = 3.5;
    let w = STEP * (COLS - 1) as f32 + R * 2.0;
    let h = STEP * (ROWS - 1) as f32;
    let painter = ui.painter();
    for c in 0..COLS {
        for r in 0..ROWS {
            painter.circle_filled(
                Pos2::new(
                    right.x - w + R + c as f32 * STEP,
                    right.y - h * 0.5 + r as f32 * STEP,
                ),
                R,
                pal.faint,
            );
        }
    }
    w
}

/// The mock's `.divider-v` between a bay's subdivisions: a `--c-hair` capsule
/// in the gap, below the head.
///
/// A no-op for every bay that is a leaf, which is six of the seven.
fn pane_dividers(ui: &Ui, pal: &Palette, panel: &Panel, id: NodeId, rect: Rect) {
    let layout = panel.layout();
    let Some(axis) = layout.axis(id) else {
        return;
    };
    let top = (rect.min.y + size::HEAD_H).min(rect.max.y);
    for (index, _) in layout.visible_children(id).enumerate().skip(1) {
        let Some(gap) = layout.boundary(id, index - 1) else {
            continue;
        };
        let gap = to_egui(gap);
        let bar = match axis {
            Axis::Row => {
                Rect::from_min_max(Pos2::new(gap.min.x, top), Pos2::new(gap.max.x, rect.max.y))
            }
            Axis::Column => Rect::from_min_max(
                Pos2::new(rect.min.x, gap.min.y.max(top)),
                Pos2::new(rect.max.x, gap.max.y.max(top)),
            ),
        };
        if bar.width() > 0.0 && bar.height() > 0.0 {
            ui.painter().rect_filled(
                bar,
                CornerRadius::same((size::PANE_DIVIDER * 0.5) as u8),
                pal.hair,
            );
        }
    }
}

/// A run of text with `letter-spacing`, which `egui` states per format run
/// rather than per style.
fn spaced(text: &str, size: f32, colour: Color32, tracking: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: FontId::new(size, FontFamily::Proportional),
            extra_letter_spacing: tracking,
            color: colour,
            ..Default::default()
        },
    );
    job
}

/// The arrangement's rectangle, in `egui`'s. Both are top-left origin in
/// logical pixels, so this is only two types meeting.
pub fn to_egui(r: karakuri_layout::Rect) -> Rect {
    Rect::from_min_size(Pos2::new(r.x, r.y), egui::vec2(r.w, r.h))
}

#[cfg(test)]
mod tests {
    //! **The Inspector's own tests, and they are in `src/` rather than in
    //! `tests/`.**
    //!
    //! Every other region of this console is asserted from a file in `tests/`,
    //! one per region, and this bay would have been `tests/inspector.rs`. It
    //! is here because [`pane_box`] and [`group_h`] are private — the pane's
    //! arithmetic is what these check, and the public [`inspector`] is that
    //! arithmetic plus a layout lookup. Moving them out would mean making the
    //! derivation public to test it, which is a wider surface for a narrower
    //! reason.
    //!
    //! What is *not* here and belongs in `tests/inspector.rs` when somebody
    //! writes it: the bay drawn against a real arrangement at the two
    //! viewports, on `tests/library.rs`'s pattern.

    use super::*;

    /// A pane with `params` parameters under one node and no renderers.
    fn one_node(params: usize) -> Pane {
        Pane {
            deck: 0,
            material: "drift_shell + soft_points".to_owned(),
            sync: Sync::Tempo,
            // Nothing refused, which is what a pane about the arithmetic of
            // its rows says about material it is not asking after.
            allows: [true; SYNCS.len()],
            anchor_bpm: 128.0,
            scrub_beats: 0.0,
            composite: false,
            nodes: vec![Node {
                addr: "L1:0".to_owned(),
                name: "drift_shell".to_owned(),
                authority: Some(Authority::Manual),
                renderers: Vec::new(),
                params: (0..params)
                    .map(|n| Param {
                        ord: n + 1,
                        name: format!("p{n}"),
                        value: 0.5,
                        at: 0.5,
                    })
                    .collect(),
            }],
        }
    }

    /// A rectangle the size of one inspector pane at the narrowest console the
    /// mock will draw: `.console`'s `min-width: 1010px` holds the centre at
    /// 484 and each pane at 237.
    fn pane_at(height: f32) -> Rect {
        Rect::from_min_size(Pos2::new(0.0, 0.0), egui::vec2(237.0, height))
    }

    /// **The two heads are the mock's boxes and the leftover is the groups'.**
    ///
    /// `.half-head` is `5 + 16.5 + 5` over its own `border-bottom`, and
    /// `.deck-head` is `5 + 15.5 + 5` with none — the stylesheet's own
    /// arithmetic for that row. Everything under them is the body.
    #[test]
    fn a_pane_is_two_heads_and_what_is_left() {
        let pane = one_node(2);
        let at = pane_box(pane_at(400.0), &pane.nodes).expect("a pane with room in it");
        assert_eq!(at.head.height(), 27.5);
        assert_eq!(at.deck_head.height(), 25.5);
        assert_eq!(at.head.max.y, at.deck_head.min.y);
        assert_eq!(at.deck_head.max.y, at.body.min.y);
        assert_eq!(at.body.max.y, 400.0);
    }

    /// **The inspector's own minimum is the least this draws**, and the two
    /// are one derivation: a pane at the minimum less the bay head has room
    /// for exactly the one group of two parameters the minimum is written
    /// from, and a pixel less has room for none of it.
    ///
    /// This is what the 151.5 in `lib.rs` *means*: bay head 27, `.half-head`
    /// 27.5, `.deck-head` 25.5, `.node-head` 26.5 and two `.param` rows at
    /// 22.5.
    #[test]
    fn the_bays_minimum_is_the_least_a_pane_can_draw() {
        let pane = one_node(2);
        let pane_h = 151.5 - size::HEAD_H;
        let at = pane_box(pane_at(pane_h), &pane.nodes).expect("a pane at the minimum");
        assert_eq!(
            at.shown, 1,
            "one node and two of its parameters is what the minimum is written from"
        );
        let short = pane_box(pane_at(pane_h - 0.5), &pane.nodes).expect("still two heads");
        assert_eq!(
            short.shown, 0,
            "half a pixel under the minimum and the group no longer fits"
        );
    }

    /// **A group is drawn whole or not at all**, which is the pane having no
    /// scroll position: a node head with two of its five parameters under it
    /// reads as a node with two parameters.
    #[test]
    fn a_group_that_does_not_fit_whole_is_not_drawn() {
        let pane = Pane {
            nodes: vec![one_node(2).nodes[0].clone(), one_node(5).nodes[0].clone()],
            ..one_node(2)
        };
        // Room for the first group, the hairline, and all but the last row of
        // the second.
        let first = size::NODE_HEAD_H + size::PARAM_H * 2.0;
        let second = size::NODE_HEAD_H + size::PARAM_H * 5.0;
        let body = first + size::HAIRLINE + second - size::PARAM_H;
        let at = pane_box(
            pane_at(size::HALF_HEAD_H + size::DECK_HEAD_H + body),
            &pane.nodes,
        )
        .expect("a pane with room in it");
        assert_eq!(at.shown, 1, "the second group is short by one row");

        // One row more and both are drawn.
        let at = pane_box(
            pane_at(size::HALF_HEAD_H + size::DECK_HEAD_H + body + size::PARAM_H),
            &pane.nodes,
        )
        .expect("a pane with room in it");
        assert_eq!(at.shown, 2);
    }

    /// **Where a group goes is the sum of the groups above it and the rules
    /// between them**, and the rule is between rather than under: *n* groups
    /// carry *n - 1* of them, because `.node-group:last-child` has none.
    #[test]
    fn a_group_stacks_under_the_one_before_it_with_a_rule_between() {
        let pane = Pane {
            nodes: vec![
                one_node(2).nodes[0].clone(),
                one_node(1).nodes[0].clone(),
                one_node(3).nodes[0].clone(),
            ],
            ..one_node(2)
        };
        let at = pane_box(pane_at(400.0), &pane.nodes).expect("a pane with room in it");
        assert_eq!(at.shown, 3);
        let first = at.group(&pane.nodes, 0);
        let second = at.group(&pane.nodes, 1);
        let third = at.group(&pane.nodes, 2);
        assert_eq!(first.min.y, at.body.min.y);
        assert_eq!(second.min.y, first.max.y + size::HAIRLINE);
        assert_eq!(third.min.y, second.max.y + size::HAIRLINE);
        assert_eq!(first.height(), size::NODE_HEAD_H + size::PARAM_H * 2.0);
        assert_eq!(second.height(), size::NODE_HEAD_H + size::PARAM_H);
    }

    /// **A renderer row costs a group its own height**, and a group without
    /// one costs nothing: the mock draws `.rend-row` in the `L4` group and
    /// nowhere else.
    #[test]
    fn a_renderer_row_is_a_row_of_the_group_it_is_in() {
        let mut node = one_node(1).nodes[0].clone();
        let without = group_h(&node);
        node.renderers = vec![Renderer {
            name: "soft_points".to_owned(),
            live: true,
        }];
        assert_eq!(group_h(&node), without + size::REND_ROW_H);
    }

    /// **The anchor is the mock's own two spellings, and free shows neither.**
    ///
    /// *"`T128` is the tempo a deck was engaged at … `B128 +0.25` is that with
    /// the deck sitting a quarter beat ahead of the room. A free deck shows
    /// neither."*
    #[test]
    fn the_anchor_reads_what_the_mock_reads() {
        let mut pane = one_node(1);
        pane.sync = Sync::Tempo;
        pane.scrub_beats = 0.25;
        assert_eq!(
            anchor_text(&pane).as_deref(),
            Some("T128"),
            "tempo sync does not read the offset, so it is not drawn"
        );
        pane.sync = Sync::Beat;
        assert_eq!(anchor_text(&pane).as_deref(), Some("B128 +0.25"));
        pane.sync = Sync::Free;
        assert_eq!(
            anchor_text(&pane),
            None,
            "free is the absence of a transport rather than a setting"
        );
    }

    /// **The pane head names the deck it is pointed at and what is in it**,
    /// which is the mock's `deck A · drift_night`.
    #[test]
    fn the_pane_head_says_which_deck_it_is_showing() {
        let mut pane = one_node(1);
        pane.deck = 1;
        assert_eq!(showing_text(&pane), "deck B · drift_shell + soft_points");
    }

    /// **Every level the vocabulary names is on the node head.**
    ///
    /// `karakuri_operation::Authority` carries no `ALL` — *"it arrives with
    /// the first reader"* — so [`AUTHORITIES`] is this crate's list, and the
    /// thing that can go wrong is the list falling behind the vocabulary while
    /// [`auth_word`] is updated. This holds the two together: every level
    /// [`auth_word`] can spell is in the array exactly once, and the array is
    /// in the vocabulary's own declaration order.
    #[test]
    fn every_authority_the_vocabulary_names_is_on_the_node_head() {
        // A `match` that a fourth level would not compile past, which is what
        // makes this a check on the *array* rather than on the enum.
        let expected = [
            Authority::Manual,
            Authority::Suggesting,
            Authority::Automatic,
        ];
        assert_eq!(
            AUTHORITIES.len(),
            expected.len(),
            "a level the vocabulary names is missing from the node head"
        );
        for (n, level) in expected.into_iter().enumerate() {
            assert_eq!(AUTHORITIES[n], level, "the three are in declaration order");
            assert!(
                !auth_word(level).is_empty(),
                "every level has the console's own abbreviation for it"
            );
        }
        let words: Vec<&str> = AUTHORITIES.into_iter().map(auth_word).collect();
        assert_eq!(
            words,
            vec!["man", "sug", "auto"],
            "the manual's node head reads `man / sug / auto`"
        );
    }

    /// **A pane narrower than a parameter row's own padding is no pane**,
    /// which is the picture's rule stated across the axis.
    #[test]
    fn a_pane_with_no_room_across_it_draws_nothing() {
        let pane = one_node(2);
        let narrow = Rect::from_min_size(
            Pos2::new(0.0, 0.0),
            egui::vec2(size::PARAM_PAD_L + size::PARAM_PAD_R, 400.0),
        );
        assert!(pane_box(narrow, &pane.nodes).is_none());
    }

    /// **There are two panes and there is no third.**
    ///
    /// The mock's `2 up ▾` is an operator choosing how many while running, and
    /// a [`Spec`](karakuri_layout::Spec) builds a
    /// [`Layout`](karakuri_layout::Layout) once — so the count is the
    /// arrangement's, and a caller asking for a pane it has not got gets
    /// `None` rather than one of the two it has.
    ///
    /// And a console with no deck behind it hands over no pane at all, which
    /// is every test in this crate.
    #[test]
    fn there_are_two_panes_and_no_third() {
        let mut layout = crate::layout();
        layout.set_viewport(karakuri_layout::Rect {
            x: 0.0,
            y: 0.0,
            w: 1920.0,
            h: 1080.0,
        });
        layout.solve();
        let pane = one_node(2);
        assert_eq!(PANES, 2, "the mock's `.insp-split` is `1fr 9px 1fr`");
        for index in 0..PANES {
            assert!(
                inspector(&layout, index, &pane).is_some(),
                "pane {index} is in the arrangement and has room in it"
            );
        }
        assert!(
            inspector(&layout, PANES, &pane).is_none(),
            "a third pane is a pane the arrangement has not got"
        );
        assert!(View::new(Room::Day).inspector.is_empty());
    }

    /// **A pane starts under the bay head and not on top of it.**
    ///
    /// The one thing `pane_box`'s own tests cannot see. They are handed a
    /// rectangle and the head has already been taken off it — the fixture
    /// says so in the arithmetic, `151.5 - size::HEAD_H` — so every one of
    /// them passes whether or not the caller does the subtraction. It did
    /// not: `.half-head` was drawn at the top of `inspector-1`, which is the
    /// top of the bay, which is where [`bay_head`] paints the word
    /// `Inspector`. Read against the arrangement rather than against a
    /// fixture, because the fixture is what could not tell.
    #[test]
    fn a_pane_starts_under_the_bay_head() {
        let mut layout = crate::layout();
        layout.set_viewport(karakuri_layout::Rect {
            x: 0.0,
            y: 0.0,
            w: 1920.0,
            h: 1080.0,
        });
        layout.solve();
        let pane = one_node(2);
        let bay = to_egui(layout.rect(layout.find("inspector").expect("the bay is named")));
        for index in 0..PANES {
            let at = inspector(&layout, index, &pane).expect("a pane with room in it");
            assert_eq!(
                at.head.min.y,
                bay.min.y + size::HEAD_H,
                "pane {index} starts where the bay head ends"
            );
            assert!(
                bay.contains_rect(at.head) && bay.contains_rect(at.deck_head),
                "pane {index} draws inside the bay it is in"
            );
        }
    }
}
