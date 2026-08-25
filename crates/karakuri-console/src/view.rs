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
//! [`Kind::Previews`] paints four cells whether or not a deck is running in
//! any of them, and that is **not** the scaffolding the paragraph above
//! forbids. A greyed-out control is a placeholder for a control that does not
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
//! The transport and the outputs are **rows, not bays**: they carry no
//! heading, because they have none in the mock — both carry `class="bay"`,
//! which is the card styling, and neither carries a `.bay-head`
//! ([ADR-0159](../../../docs/adr/0159-the-consoles-words-are-the-manuals-and-the-middle-one-is-not-a-pane.md)).
//!
//! # The transport row is four readouts and no controls at all
//!
//! [`Kind::Transport`] draws the tempo, the beat grid, the bar and the frame
//! readout — **the four things in the mock's row that are a value somebody
//! measured rather than a control over something that does not exist.** The
//! other six are named in [`transport`], one by one, with what is missing
//! behind each; the paragraph above is the whole of the argument and this row
//! is where it costs the most, because six of the mock's ten items go.
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
//! # The bay head is one component with seven call sites
//!
//! [`bay_head`] is written once and [`REGIONS`] calls it seven times, which is
//! this repository's rule about an abstraction needing two call sites,
//! satisfied on the day it is written rather than promised for later.
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

use egui::epaint::text::{LayoutJob, TextFormat};
use egui::{Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, Stroke, StrokeKind, Ui};
use karakuri_layout::{Axis, Hit, NodeId};

use crate::panel::{Op, Panel, GRAB};
use crate::room::{size, Palette, Room};

/// The whole of a texture, in `egui`'s texture coordinates. The picture fills
/// its rectangle: the fitting happened when the engine drew into it, and doing
/// it again here would be two answers to *how does a 16:9 canvas sit in this
/// box*.
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
const DECK_LETTERS: [&str; DECKS] = ["A", "B", "C", "D"];

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
    Picture,
    /// **The row of deck previews** under the picture, which is [`DECKS`]
    /// cells side by side.
    ///
    /// A pane in every other respect, and a kind of its own for the reason
    /// [`Kind::Picture`] is one, already written above: [`View::draw`] has to
    /// know *which* pane the cells go in, and the alternative is comparing a
    /// name on the frame path, which puts a string where the table already
    /// says what a region is.
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
        kind: Kind::Bay {
            title: "Library",
            pills: &[],
            grip: true,
        },
    },
    Region {
        name: "staging",
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
        kind: Kind::Bay {
            title: "Mixer",
            pills: &[],
            grip: false,
        },
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
/// Solves first, because [`karakuri_layout::Layout::rect`] refuses to answer
/// from a dirty layout; on a frame where nothing moved that is a flag test.
pub fn plan_into(panel: &mut Panel, out: &mut Vec<Placed>) {
    panel.solve();
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
/// that takes a device is the example's.
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

/// **Where the picture goes**: inside the `program-view` region, below the bay
/// head that is painted over the top of it and inside `.program-body`'s
/// padding.
///
/// `None` where there is no picture to draw, which is the manual's *"it is on
/// screen exactly when that sink is on — so there is no state where it is
/// hidden and still costing a pass"*: a caller that renders into this
/// rectangle records no pass at all when there is no rectangle. A folded
/// picture is the case that matters and it is not a case of its own — see the
/// note in the body.
///
/// # The insets are the bay's own derivation, read backwards
///
/// The arrangement gives `program-view` 27 + 9 + 262 at the mock's width — bay
/// head, `.program-body`'s padding above the picture, and the 16:9 picture
/// itself. So the picture is that region less [`size::HEAD_H`] and one
/// [`size::PROGRAM_BODY_PAD`] at the top, less a pad either side, and **less
/// nothing at the bottom**: what the CSS puts under the picture is
/// `.program-body`'s `gap: 8px`, which is the split's divider and belongs to
/// neither child, and the 9 further down is that body's bottom padding, which
/// belongs to `deck-previews`. So the only 9s in this region are the one above
/// the picture and the one either side of it.
///
/// At the narrowest console the mock will draw this is exactly 466 x 262,
/// which is 16:9. At any wider window it is wider than 16:9 and the picture
/// letterboxes into it, which is the engine's job and not this crate's.
///
/// `layout` must be solved: `Layout::rect` refuses to answer from a dirty one.
pub fn picture_rect(layout: &karakuri_layout::Layout) -> Option<Rect> {
    let id = layout.find("program-view")?;
    let region = to_egui(layout.rect(id));
    let rect = Rect::from_min_max(
        Pos2::new(
            region.min.x + size::PROGRAM_BODY_PAD,
            region.min.y + size::HEAD_H + size::PROGRAM_BODY_PAD,
        ),
        Pos2::new(region.max.x - size::PROGRAM_BODY_PAD, region.max.y),
    );
    // **One rule, and it is the size rather than the visibility.** A folded
    // region keeps its rectangle and loses its extent along its parent's axis,
    // and so does one inside a fold and one a solo left out — so `visible`
    // would be a second answer to a question this already asks, and the two
    // would agree until somebody found the case where they did not. What is
    // left over is the case `visible` never covered anyway: the inset is
    // bigger than the region, which is a rectangle `egui` draws inside out
    // rather than refuses.
    match rect.width() > 0.0 && rect.height() > 0.0 {
        true => Some(rect),
        false => None,
    }
}

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
/// - **Rejected:** fill the track and letterbox the texels inside it. That
///   keeps the row looking like a grid at every width, and pays for it by
///   stretching the cell away from the shape of its picture — a 16:9 audition
///   in a 200x63 well, with bars the console has drawn itself inside a
///   rectangle the engine already fitted. Two fits for one question, which is
///   the thing `WHOLE_TEXTURE` refuses one level up.
///
/// `layout` must be solved: `Layout::rect` refuses to answer from a dirty one.
pub fn preview_rects(layout: &karakuri_layout::Layout) -> Option<[Rect; DECKS]> {
    let id = layout.find("deck-previews")?;
    preview_cells(to_egui(layout.rect(id)))
}

/// The four cells inside a `deck-previews` region, or `None` where there is no
/// room for them.
///
/// **One derivation with two call sites**: [`preview_rects`], which a caller
/// sizes textures from, and [`View::draw`], which paints them. A second copy
/// of this arithmetic is a row of cells drawn somewhere the textures are not.
fn preview_cells(region: Rect) -> Option<[Rect; DECKS]> {
    let pad = size::PROGRAM_BODY_PAD;
    let row = Rect::from_min_max(
        Pos2::new(region.min.x + pad, region.min.y),
        Pos2::new(region.max.x - pad, region.max.y - pad),
    );
    // The gaps are **between** the tracks and nowhere else, which is what
    // `grid-template-columns: repeat(4, 1fr)` with a `gap` is: four tracks and
    // three gaps, not four tracks each carrying one.
    let gaps = size::PREVIEW_GAP * (DECKS - 1) as f32;
    let track = (row.width() - gaps) / DECKS as f32;
    // 16:9, as large as the track and the row both allow.
    let h = row.height().min(track * 9.0 / 16.0);
    let w = h * 16.0 / 9.0;
    // The same rule `picture_rect` states, and stated on the cell rather
    // than on the region because the cell is what gets drawn: a row too short
    // or too narrow to hold one is a rectangle `egui` draws inside out rather
    // than refuses.
    if !(w > 0.0 && h > 0.0) {
        return None;
    }
    Some(std::array::from_fn(|deck| {
        let track_x = row.min.x + (track + size::PREVIEW_GAP) * deck as f32;
        Rect::from_min_size(
            Pos2::new(
                track_x + (track - w) * 0.5,
                row.min.y + (row.height() - h) * 0.5,
            ),
            egui::vec2(w, h),
        )
    }))
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
/// is the one the example leaves `None`.
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

    /// **Which dot is lit**: the beat within the current bar, from zero.
    ///
    /// `rem_euclid` rather than `%` because [`Transport::beats`] can be
    /// negative — `Oscillator::behind` reads the same grid at an earlier time,
    /// and a slot warming behind the session is exactly what that is for — and
    /// `%` on a negative is negative, which is a dot index no grid has.
    ///
    /// **The clamp on the way out is not belt and braces.** `(-1e-18_f64)
    /// .rem_euclid(4.0)` is `4.0` exactly: the true remainder is a hair under
    /// the divisor and rounds up to it. That is one dot past the end of the
    /// grid, which is a rectangle drawn beside it or a panic on an index, for
    /// a value that is *just before* the downbeat.
    pub fn beat(&self) -> u32 {
        let dots = self.dots();
        (self.beats.rem_euclid(dots as f64) as u32).min(dots - 1)
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
/// [`View::draw`] paints exactly these rectangles. Nothing hit-tests them
/// **because nothing in this row is a control** — see below — so this has one
/// call site today where [`outputs`] had two on the day it was written. It is
/// a value rather than a paint-as-you-go pass all the same, because that is
/// what makes the arithmetic something `tests/transport.rs` can ask about
/// without a device, which is the whole of how this crate is checked.
///
/// # Nothing here is a control, and that is the answer rather than an omission
///
/// [`crate::input::claim`] asks [`outputs`] and nothing else, and it stays
/// that way: a point in this row that is not inside a boundary's [`GRAB`] is
/// `egui`'s. Every one of the mock's controls in this row is in the list below
/// of things that are not drawn, so what is left — a tempo, a beat, a bar and
/// a frame time — is four readouts, and a readout is not something a press
/// acts on. `tests/transport.rs` asserts it over the row rather than leaving
/// it to be inferred from the absence of a hit test.
///
/// # What is in the mock's row and is deliberately not here
///
/// The mock draws ten things and **four of them exist**. Each of the other six
/// is a control over machinery that is in neither this crate nor the example,
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
///   nobody has plugged in is worse than no menu.
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
/// Four of the mock's ten items carry a `data-tip` and two of the four drawn
/// ones do. A tooltip needs `egui` to own a widget, this console paints, and
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
    /// **Which of them is lit**: [`Transport::beat`], which is always a dot
    /// this grid has.
    pub on: u32,
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
            Pos2::new(
                self.grid.min.x + (size::BEAT_W + size::BEAT_GAP) * index as f32,
                self.grid.min.y,
            ),
            egui::vec2(size::BEAT_W, size::BEAT_H),
        )
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
            on: t.beat(),
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
///   uses for its own glow.
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
        let lit = index == row.on;
        if lit {
            painter.add(
                egui::epaint::Shadow {
                    offset: [0, 0],
                    blur: size::BEAT_GLOW,
                    spread: 0,
                    color: pal.glow_pink,
                }
                .as_shape(dot, radius),
            );
        }
        painter.rect_filled(
            dot,
            radius,
            match lit {
                true => pal.pink,
                false => pal.line,
            },
        );
    }

    centred(row.bar, painter.layout_job(span(&bar_text(t), pal.dim)));
    centred(
        row.frame,
        painter.layout_job(frame_job(t, pal.text, pal.faint)),
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
    placed: Vec<Placed>,
}

impl View {
    pub fn new(room: Room) -> View {
        View {
            room,
            picture: None,
            previews: [None; DECKS],
            transport: None,
            // Every region the console has, so the frame path never grows it.
            placed: Vec::with_capacity(REGIONS.len()),
        }
    }

    /// Draw the whole console. The `ui` is the root one
    /// [`egui::Context::run_ui`] hands the frame's closure.
    pub fn draw(&mut self, ui: &mut Ui, panel: &mut Panel) {
        let pal = self.room.palette();
        self.cursor(ui.ctx(), panel);
        plan_into(panel, &mut self.placed);

        let picture = self.picture;
        let previews = self.previews;
        let values = self.transport;
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
                    // The other body that is not empty, and the only one that
                    // draws something where nothing was handed in. It is the
                    // region's face rather than a placeholder — the module
                    // documentation is where that argument is.
                    Kind::Previews => {
                        if let Some(cells) = preview_cells(rect) {
                            for (deck, cell) in cells.into_iter().enumerate() {
                                preview(ui, &pal, cell, deck, previews[deck]);
                            }
                        }
                    }
                }
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
        let axis = match panel.drag_axis() {
            Some(axis) => Some(axis),
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
        true => (
            pal.text,
            Color32::from_rgba_unmultiplied(pal.mint.r(), pal.mint.g(), pal.mint.b(), TINT_14),
            pal.mint,
        ),
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

/// `color-mix(in srgb, X 14%, transparent)`, as the alpha it is: 14% of 255,
/// rounded. The mock's own wash behind an armed control, and `room`'s
/// documentation is where the equivalence is argued.
const TINT_14: u8 = 36;

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
