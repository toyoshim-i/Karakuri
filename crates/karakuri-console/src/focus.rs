//! The bay a key press is addressed to, and what each bay remembers.
//!
//!
//! [ADR-0259](../../../docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
//! decided that a key press is addressed to whatever holds focus, that `Tab`
//! and `shift-Tab` move focus between bays along the arrangement's own walk,
//! and that a bay's address is a path — a digit names the nth thing one level
//! below it and `0` names the bay's head. This module is the whole of that: the
//! ring, the pointer that walks it and the address each bay remembers
//! ([ADR-0332](../../../docs/adr/0332-focus-is-a-pointer-the-console-owns-and-the-three-pointers-are-instances-of-it.md)),
//! and the four keys that act inside a bay, in the Mixer and the Library
//! ([ADR-0333](../../../docs/adr/0333-the-console-resolves-the-address-and-the-window-loop-names-the-operation.md))
//! and then in the seven that were left
//! ([ADR-0343](../../../docs/adr/0343-the-grammar-reaches-all-nine-bays-and-space-on-a-bay-is-the-fold.md)).
//!
//! # The three pointers are three readings of one thing
//!
//! `View::selection`, `View::cursor_row` and `View::scope` were three private
//! fields with the same paragraph written at each of them — *nothing downstream
//! can be the model of record for it*. They are three readings of [`Address`]
//! now, on two bays:
//!
//! | pointer | bay | where in the address | | --- | --- | --- | |
//! `View::selection` | `mixer` | the item the bay was last on — a strip | |
//! `View::cursor_row` | `library` | the item the bay was last on — a row | |
//! `View::scope` | `library` | the control last named under the head |
//!
//! Nothing about what any of them means changes, which is the record's own
//! clause: what each of them *refuses* — a deck the mixer draws no strip for, a
//! row past the listing, a scope with no chip — stays at the method that
//! refuses it. What moved is where the number is kept.
//!
//! # The head is `0` and is not remembered
//!
//! [`Address::remembered`] answers *the nth item last named under a path*, and
//! `0` is never one of them. The head is where a bay keeps the controls that
//! are about the bay rather than about anything in it, so there is one of it
//! and there is nothing to remember; a bay that remembered *the head* under the
//! same key as *the third strip* would lose the deck selection the first time
//! an operator pressed `0`. The head is still a rung the address descends
//! through — which is exactly how the Library's scope is reached, at `[HEAD]` —
//! and it is [`Address::at`] that says so.
//!
//! # The ring is derived and `REGIONS` is what it is checked against
//!
//! [`ring`] walks the arrangement: a column's children top to bottom, a row's
//! left to right, and it stops at the first node that is a bay rather than
//! descending into it. ADR-0259 asks for exactly that — *"a ring derived from
//! the solved tree is the honest implementation and the constant is a thing to
//! check against, not the source"* — because an operator who has dragged a
//! divider or folded a pane has moved the traversal with it and nothing has to
//! be told. `tests/focus.rs` is where the constant does the checking.
//!
//! A folded bay stays in the ring, and the reason is the narrow one: it is in
//! the ring so that there is something to press to open it, not so that it can
//! be operated. So [`ring`] asks [`Layout::children`] and never
//! [`Layout::placed_children`] — the visible half is the paint's question and
//! not the walk's.
//!
//! # What this module does not do
//!
//! It binds no key. `crates/karakuri/src/main.rs` is where `Tab` and `esc`
//! reach these methods, for [`crate::panel`]'s reason: a surface is where the
//! buck stops and this crate is asked rather than asking.
//!
//! The six keys are all here. A digit, the arrows, `space` and `enter` are the
//! grammar ADR-0259 designs; [`press`] resolves an address against what a bay
//! is drawing and answers what the host has to do about it, and [`BUILT`] is
//! the table it is written in — one row per bay, all nine of them since
//! [ADR-0343](../../../docs/adr/0343-the-grammar-reaches-all-nine-bays-and-space-on-a-bay-is-the-fold.md).
//!
//! `space` on a bay is the fold, and it is the one press that means the same
//! thing wherever focus is. So it is declared once, under [`ANY`], rather than
//! nine times over — which is what lets *Fold a bay away* carry one key badge
//! saying `space · in any bay`.

use std::collections::BTreeMap;

use karakuri_operation::{
    Authority, BeatSource, BlendMode, ChainParam as ParamOfChain, Operation, Output, ParamValue,
    Revision, StepMode,
};

use karakuri_layout::{Layout, NodeId};

use crate::panel::{Op, Panel};
use crate::view::{region, Ask, AudioAsk, Kind, Mask, Region, Tally, View};

/// The digit that names a bay's head, which is the one digit that is not an
/// item.
///
/// A constant rather than a literal, because two very different things are
/// spelled `0` in this module — the head, and the first element of a path — and
/// only one of them is this.
pub const HEAD: usize = 0;

/// The Mixer bay, by the name the arrangement gives it — whose remembered item
/// is the deck selection.
///
/// A constant rather than a literal at the two methods that read it, for
/// [`crate::view::REGIONS`]' own reason: a bay nobody can find by name is a
/// pointer that silently stops pointing, and `tests/focus.rs` is what asserts
/// the ring holds it.
pub const MIXER: &str = "mixer";

/// The Library bay — whose remembered item is the row under the cursor and
/// whose head's remembered control is the marked scope. [`MIXER`]'s reason.
pub const LIBRARY: &str = "library";

/// Whether a region is one of the nine bays.
///
/// The four that are not are what a bay *contains*: the Program bay's picture
/// and preview row and the Inspector's two panes are its items, which is
/// ADR-0259's own finding — *"for two of the nine bays the arrangement's own
/// tree is the item list"*. So the walk stops at the bay and the four are
/// reached by a digit rather than by `Tab`.
///
/// A `match` with no wildcard, so a tenth kind is a compile error here rather
/// than a region that quietly never takes focus.
pub const fn is_bay(kind: Kind) -> bool {
    match kind {
        Kind::Bay { .. }
        | Kind::Transport
        | Kind::Library
        | Kind::Staging
        | Kind::Mixer
        | Kind::Master
        | Kind::Sequencer
        | Kind::Outputs => true,
        Kind::Picture | Kind::Previews | Kind::Pane => false,
    }
}

/// The tab ring: every bay of `layout`, in the order the walk reaches them —
/// down a column first and only then across to the next one.
///
/// The order is the arrangement's own geometry rather than a list anybody
/// maintains, which is what keeps it right after a divider is dragged: a
/// column's children are taken top to bottom, a row's left to right, and the
/// walk stops at a bay instead of descending into it.
///
/// A folded bay is in it — see the module documentation. A node the arrangement
/// names and [`REGIONS`](crate::view::REGIONS) does not is structure, so
/// `left-pane`, `centre` and `right-pane` are walked through and not into the
/// ring, which is the same reading `View::draw` makes of them.
pub fn ring(layout: &Layout) -> Vec<&'static Region> {
    let mut bays = Vec::new();
    descend(layout, layout.root(), &mut bays);
    bays
}

/// [`ring`]'s walk, which stops at a bay rather than descending into it.
fn descend(layout: &Layout, id: NodeId, bays: &mut Vec<&'static Region>) {
    if let Some(found) = layout.name(id).and_then(region) {
        if is_bay(found.kind) {
            bays.push(found);
            return;
        }
    }
    for child in layout.children(id) {
        descend(layout, *child, bays);
    }
}

/// Where the dashed focus ring goes on `bay`, or `None` where the arrangement
/// gives that bay no rectangle to put one on.
///
/// The head, or the whole row for a bay that draws none. The mock draws
/// `.wfocus` as a dashed sun outline and the deck selection as a solid lavender
/// ring, on purpose — *"drawing them the same way would erase which of the two
/// a reader is looking at"* — and `console.html`'s mark for a focused bay is
/// the head wearing it. The Transport and the Outputs row are headless
/// (ADR-0159), and ADR-0259 reads them the same way it reads their `0`: *"a
/// headless row, so `0` names the row itself"*, so the row stands in for the
/// head and the ring goes round the row.
///
/// `None` for a folded bay, and that is the drawing ADR-0259 leaves open. A
/// folded region has no rectangle and no divider is drawn beside it
/// ([ADR-0204](../../../docs/adr/0204-the-root-and-the-body-row-stay-unnamed-and-a-folded-root-is-not-hit-testable.md)),
/// so there is nothing on the panel to ring; `console.html` draws the mark that
/// is owed — the head alone — beside the note that defines it, and no bay is
/// folded in the panel it draws. The bay stays in the ring either way, which is
/// what `space` is for.
///
/// `layout` must be solved: `Layout::rect` refuses to answer from a dirty one.
pub fn mark(layout: &Layout, bay: &'static Region) -> Option<egui::Rect> {
    let id = layout.find(bay.name)?;
    if !layout.visible(id) {
        return None;
    }
    let rect = crate::view::to_egui(layout.rect(id));
    Some(match crate::view::head_of(bay) {
        Some(_) => crate::view::head_box(rect),
        None => rect,
    })
}

/// Where the mark for a *folded* bay goes, or `None` for a bay that is not
/// folded, or one the arrangement is not drawing an edge for.
///
/// A folded bay keeps its place in the ring so that it can be opened, not so
/// that it can be operated — so it has to be markable, and a folded region has
/// no rectangle to mark. `docs/manual/console.html` draws what is owed: *the
/// head alone*, wearing the dashed ring, saying two things and no more — there
/// is a bay here, and `space` opens it.
///
/// The head is grown from the edge the fold leaves. A closed child keeps its
/// divider ([`Layout::is_placed`]), so it still solves to a rectangle — one
/// with no extent along its parent's axis, sitting exactly where the bay was.
/// This is that edge given a head's height, held inside the parent so that a
/// bay folded against the bottom of a column marks upward instead of off the
/// end of it. Every bay of this arrangement hangs in a column, so the width is
/// the edge's own; the parent's is the fallback for an arrangement whose bays
/// are a row, and it is written rather than assumed.
///
/// `None` unless the operator folded *this* bay. A bay inside a folded pane is
/// invisible and is not collapsed, and there is nothing of it on the panel to
/// mark — [`Layout::is_collapsed`] is the bit this asks and [`Layout::visible`]
/// is the one it does not.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one.
pub fn folded_head(layout: &Layout, bay: &'static Region) -> Option<egui::Rect> {
    let id = layout.find(bay.name)?;
    if layout.visible(id) || !layout.is_collapsed(id) {
        return None;
    }
    let parent = layout.parent(id)?;
    if !layout.visible(parent) {
        return None;
    }
    let edge = crate::view::to_egui(layout.rect(id));
    let inside = crate::view::to_egui(layout.rect(parent));
    let width = match edge.width() > 1.0 {
        true => edge.width(),
        false => inside.width(),
    };
    let head = egui::Rect::from_min_size(edge.min, egui::vec2(width, crate::room::size::HEAD_H));
    if head.height() > inside.height() || head.width() > inside.width() {
        return None;
    }
    Some(head.translate(egui::vec2(
        (inside.max.x - head.max.x).min(0.0),
        (inside.max.y - head.max.y).min(0.0),
    )))
}

/// The word a folded bay's mark carries beside its title, which is the whole of
/// what it says: there is a bay here, and this is the press that opens it.
/// `console.html` draws exactly this string.
pub const OPENS: &str = "space opens";

/// A bay's remembered address: where the address is inside this bay, and the
/// nth thing it last named at each level.
///
/// The path is the record's own: `[]` is the bay itself, `[2]` is its second
/// item, `[2, 3]` is that item's third control, and [`HEAD`] in place of an
/// item names the bay's head. Digits count from one because they count what the
/// bay drew, which is ADR-0259's change from the four deck keys.
///
/// # Two fields, because they answer two questions
///
/// [`Address::at`] is *where the address is now* — the dashed ring, moved by a
/// digit and by `esc`. [`Address::remembered`] is *where it was* — the solid
/// ring, which is what makes the deck selection survive your hands being in the
/// library. One field could not be both: `esc` from deck B's fader leaves the
/// address at the Mixer and the selection on deck B, and a path that had been
/// popped would have taken the selection with it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Address {
    /// Where the address is now, from the bay down. Empty is the bay itself, which
    /// is where every bay starts and where `esc` stops.
    at: Vec<usize>,
    /// The nth item last named under each path, and never [`HEAD`] — see the module
    /// documentation, which is where that is argued.
    ///
    /// A map rather than one number, because a bay has more than one place to
    /// remember: the Library remembers which row it was on *and* which chip of its
    /// head is marked, and those are two levels of one address rather than two
    /// pointers.
    memory: BTreeMap<Vec<usize>, usize>,
}

impl Address {
    /// Where the address is now, from the bay down — empty for the bay itself.
    pub fn at(&self) -> &[usize] {
        &self.at
    }

    /// The nth item last named under `under`, or `None` for a path nothing has been
    /// named under.
    ///
    /// One-based, which is the digit that named it: `1` is the first thing the bay
    /// drew at that level. A caller that wants a position subtracts one, and the
    /// three places that arithmetic is written all subtract with a floor — never
    /// because zero is reachable, which it is not ([`Address::remember`] refuses
    /// [`HEAD`] and nothing else writes here), but because a wrapped `usize` in a
    /// paint path is a panic in an event handler and this crate's rule is the guard
    /// rather than the message.
    pub fn remembered(&self, under: &[usize]) -> Option<usize> {
        self.memory.get(under).copied()
    }

    /// Remember that the `nth` thing under `under` was named, and answer whether
    /// that moved anything.
    ///
    /// [`HEAD`] is refused rather than stored, which is the module's own rule: the
    /// head is not one of the things a bay lists, so there is nothing to remember
    /// about it and remembering it would overwrite the item this bay was on.
    ///
    /// The `bool` is `View::select`'s: a caller repaints on a move and not on a
    /// press.
    pub fn remember(&mut self, under: &[usize], nth: usize) -> bool {
        if nth == HEAD {
            return false;
        }
        self.memory.insert(under.to_vec(), nth) != Some(nth)
    }

    /// Descend to the `nth` thing below where the address is, remembering it on the
    /// way — which is what a digit does.
    ///
    /// Nothing here says the nth thing exists: the bay that draws it is what
    /// refuses a digit past its end, exactly as `View::select` refuses a deck the
    /// mixer has no strip for. This is the path and not the panel.
    pub fn down(&mut self, nth: usize) {
        self.remember(&self.at.clone(), nth);
        self.at.push(nth);
    }

    /// Up one level, and answer whether there was one to leave.
    ///
    /// `false` at the bay, which is what `esc` says out loud: there is no unfocused
    /// state to fall out into, so the key acts on nothing and the caller is the one
    /// that says so
    /// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).
    /// What it does not do is quit, which is the whole of ADR-0259's change to that
    /// key.
    pub fn up(&mut self) -> bool {
        self.at.pop().is_some()
    }

    /// Back to the bay, without forgetting where it was.
    ///
    /// `esc` repeated is the ordinary way there and this is the other one: an
    /// address on something the bay has stopped drawing names nothing, and acting
    /// on whatever has taken that position is the failure
    /// [`crate::view::View::point_at`] refuses one level down. The memory is left
    /// alone, for [`Address::up`]'s reason — what a bay *was* on is the solid ring
    /// and is not what went stale.
    pub fn to_the_bay(&mut self) {
        self.at.clear();
    }

    /// Put the address on the `nth` item, remembering it — what an arrow does when
    /// the address had already descended to one.
    ///
    /// It is [`Address::down`] with the level replaced rather than pushed: a walk
    /// moves along a level and never into one.
    pub fn to_item(&mut self, nth: usize) {
        self.at.clear();
        self.down(nth);
    }

    /// Put the address on the `nth` thing at the level it has already reached,
    /// remembering it — [`Address::to_item`] at any depth, which is what an arrow
    /// does inside a card.
    ///
    /// `false` at the bay, where there is no level to walk along.
    pub fn to_row(&mut self, nth: usize) -> bool {
        if self.at.pop().is_none() {
            return false;
        }
        self.down(nth);
        true
    }
}

/// Which bay the keyboard is talking to, and what each bay remembers.
///
/// One pointer and nine addresses. The pointer is the dashed ring the mock
/// draws; an address is the solid one, seen once per bay.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Focus {
    /// The bay focus is on, or `None` for a console nobody has tabbed on yet —
    /// which resolves to *the first bay the traversal reaches* rather than to a bay
    /// named here.
    ///
    /// There is no unfocused state and this is not one. ADR-0259 rejected starting
    /// focus on a named bay because that is a second rule to keep in step with the
    /// walk — *"the first divider dragged would have parted them"* — so the start
    /// is the walk's own answer and [`Focus::bay`] is where it is asked. The same
    /// reading is what puts focus back in the ring when a bay leaves the
    /// arrangement.
    at: Option<&'static str>,
    /// Every bay that has ever been addressed, and what it remembers. A bay with no
    /// entry has been addressed by nobody, which is where a run starts.
    addresses: BTreeMap<&'static str, Address>,
}

impl Focus {
    /// The bay focus is on, resolved against `layout`.
    ///
    /// The first bay of the ring where nothing has been focused yet, and again
    /// where what was focused is no longer a bay of this arrangement — a pointer at
    /// a region that is not there is a ring drawn nowhere, which is
    /// `View::select`'s rule read at the bay instead of at a strip.
    ///
    /// `None` only for an arrangement with no bay in it at all, which no
    /// arrangement this crate builds is.
    pub fn bay(&self, layout: &Layout) -> Option<&'static Region> {
        let bays = ring(layout);
        match self.at {
            Some(name) => bays
                .iter()
                .find(|found| found.name == name)
                .or_else(|| bays.first())
                .copied(),
            None => bays.first().copied(),
        }
    }

    /// Put focus on `bay`, and answer whether it moved.
    ///
    /// Resolved against `layout` for [`Focus::bay`]'s reason: a bay this
    /// arrangement does not hold is refused rather than stored, so nothing here can
    /// name a ring drawn nowhere.
    pub fn put(&mut self, layout: &Layout, bay: &str) -> bool {
        let Some(found) = ring(layout).into_iter().find(|found| found.name == bay) else {
            return false;
        };
        let was = self.bay(layout).map(|region| region.name);
        self.at = Some(found.name);
        was != Some(found.name)
    }

    /// `Tab`, and `shift-Tab` at `step` of `-1`: the next bay of the ring,
    /// wrapping.
    ///
    /// It always moves between bays, whatever depth the address had reached inside
    /// the one it leaves, and it never descends — which is why nothing here touches
    /// an [`Address`]. The bay it leaves keeps where it was, and that is the whole
    /// of what a remembered address is for.
    ///
    /// `shift-Tab` is this walk run backwards and nothing else. One key is the walk
    /// and the other is the walk reversed, so an operator who overshoots gets back
    /// exactly where they were — which is what a second rule for the backward
    /// direction would have cost.
    ///
    /// `false` where there is nothing to move to: an arrangement with one bay or
    /// none. A ring of one that wrapped onto itself would be a press that changed
    /// nothing and asked for a frame.
    pub fn tab(&mut self, layout: &Layout, step: i32) -> bool {
        let bays = ring(layout);
        if bays.len() < 2 {
            return false;
        }
        let here = self.bay(layout).map_or(0, |bay| {
            bays.iter()
                .position(|found| found.name == bay.name)
                .unwrap_or(0)
        });
        let len = bays.len() as i32;
        let to = ((here as i32 + step).rem_euclid(len)) as usize;
        let moved = to != here;
        self.at = Some(bays[to].name);
        moved
    }

    /// What `bay` remembers, or `None` for a bay nobody has addressed.
    pub fn address(&self, bay: &str) -> Option<&Address> {
        self.addresses.get(bay)
    }

    /// What `bay` remembers, to write into — created empty on the first write,
    /// which is what makes *a bay nobody has addressed* a state rather than a row
    /// of defaults.
    pub fn address_mut(&mut self, bay: &'static str) -> &mut Address {
        self.addresses.entry(bay).or_default()
    }

    /// `esc`: up one level of the focused bay's address.
    ///
    /// `false` at bay level, which is the refusal ADR-0259 asks to be said out
    /// loud: there is no rung below the bay and no unfocused state to fall out
    /// into, and a key that declines silently is indistinguishable from one that is
    /// not bound. It never leaves the ring and it never quits.
    pub fn up(&mut self, layout: &Layout) -> bool {
        let Some(bay) = self.bay(layout) else {
            return false;
        };
        match self.addresses.get_mut(bay.name) {
            Some(address) => address.up(),
            None => false,
        }
    }
}

// ---------------------------------------------------------------------------
// The grammar: what the six keys reach inside a bay
// ---------------------------------------------------------------------------

/// The four keys of the grammar that are addressed to whatever the bay's
/// address is on.
///
/// `Tab` and `esc` are the other two of ADR-0259's six and are not here: they
/// move the address rather than acting on it, so they reach the same thing in
/// every bay and no bay has to declare them.
///
/// This is the classifier the table is written in and the check reads;
/// [`Press`] is one press with what the key itself said.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Grammar {
    /// A digit — the nth thing one level below the address, and `0` the head.
    Digit,
    /// An arrow — the neighbour of the addressed thing, along the axis it is drawn
    /// on, or the next value of a level.
    Arrows,
    /// `space` — the addressed thing's next state, or a level's declared default,
    /// or a bay's fold.
    Space,
    /// `enter` — the act the addressed thing is for.
    Enter,
}

impl Grammar {
    /// All four, in ADR-0259's own order.
    pub const ALL: [Grammar; 4] = [
        Grammar::Digit,
        Grammar::Arrows,
        Grammar::Space,
        Grammar::Enter,
    ];
}

/// The bay a press that acts the same in every bay is addressed in, which is
/// not one of the nine.
///
/// `space` at bay level is the fold, and folding is a rule about *a bay* — so
/// the route it names is not the Mixer's or the Master's but every bay's, and
/// the page spells it `space &middot; in any bay`. It is a constant rather than
/// a literal for [`MIXER`]'s reason, and `crates/karakuri/src/main.rs`'s
/// `key_column` resolves the page's spelling to it.
///
/// It is a route and not a region, so nothing looks it up in
/// [`crate::view::REGIONS`]: a badge that named a bay here would be naming one
/// of nine places a press works.
pub const ANY: &str = "any";

/// Which way an arrow points. Four rather than two, because the axis is half of
/// what an arrow means: the Mixer's strips are a row and the Library's rows are
/// a column, so `←→` walk one and `↑↓` the other, and a level is stepped by
/// `↑↓` whichever bay it is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrow {
    Up,
    Down,
    Left,
    Right,
}

impl Arrow {
    /// Whether this arrow lies along a row — `←→` — rather than down a column.
    pub const fn across(self) -> bool {
        matches!(self, Arrow::Left | Arrow::Right)
    }

    /// Which way along its axis: `-1` for up and left, `1` for down and right. A
    /// list runs down and a row runs right, so *further on* is one direction and
    /// the page needs no second rule for it.
    pub const fn step(self) -> i32 {
        match self {
            Arrow::Up | Arrow::Left => -1,
            Arrow::Down | Arrow::Right => 1,
        }
    }
}

/// One press of the grammar, with what the key itself said.
///
/// A digit carries which digit and an arrow carries which way; `space` and
/// `enter` carry nothing, because the address is the whole of what they are
/// addressed to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    Digit(usize),
    Arrow(Arrow),
    Space,
    Enter,
}

impl Press {
    /// Which of the four keys this is — what the table is written in.
    pub const fn key(self) -> Grammar {
        match self {
            Press::Digit(_) => Grammar::Digit,
            Press::Arrow(_) => Grammar::Arrows,
            Press::Space => Grammar::Space,
            Press::Enter => Grammar::Enter,
        }
    }
}

/// A control a bay draws, as the grammar addresses it.
///
/// One variant per control the nine bays draw at a rung the address reaches, in
/// the order the bay draws them — which is what makes a digit name the nth of
/// them. It is not a list of every rectangle on the panel: a readout is not a
/// control, and a control ADR-0259's walk does not name is not addressed — each
/// of those is written down at the bay's row in [`BUILT`] rather than left to
/// be noticed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    // -- the Mixer ---------------------------------------------------------
    /// A strip's residency chip — `space` asks the deck for the next of the three.
    Tally,
    /// A strip's trim, which is a level: arrows step it and `space` returns it to
    /// the value it was declared at.
    Trim,
    /// A strip's channel fader, and [`Control::Trim`]'s rules exactly.
    Fader,
    /// A strip's blend chip — `space` cycles add, over and max.
    Blend,
    /// A strip's mask mini — `space` cycles the shape.
    Mask,
    /// The transition row's shape pill, in the Mixer's head: the settings are about
    /// the bay rather than about any one strip, which is what a head is for.
    Shape,
    /// The transition row's quantum pill. [`Control::Shape`]'s rules.
    Quantum,
    /// The transition row's length pill. [`Control::Shape`]'s rules.
    Length,
    /// The transition row's `go` capsule — the act, on the addressed strip, which
    /// is the deck selection.
    Go,

    // -- the Library -------------------------------------------------------
    /// The Library head's scope chips — `space` steps to the next and wraps.
    Scope,
    /// A row's star — `space` puts it on this Set or takes it off.
    Star,
    /// A row's `params` chip — `enter` opens what that Set holds and declares.
    Params,

    // -- the Transport -----------------------------------------------------
    /// The tempo figure, which is a track a press positions rather than a level
    /// anything steps.
    Tempo,
    /// The `½ ×2` pair, whose only operand is the direction the key spells.
    Grid,
    /// The latency offset, which is a level.
    Offset,
    /// The beat grid — a readout.
    Beat,
    /// The bar counter — a readout.
    Bar,
    /// The frame cost — a readout.
    Cost,
    /// The audio-in pill, whose press puts a card of inputs down.
    Audio,
    /// The arrangement pill, whose press puts its menu down.
    Arrangement,
    /// The tone map pill — `space` cycles the four operators.
    Tonemap,
    /// The exposure track, which is a level.
    Exposure,

    // -- the two cards the Transport's address walks -----------------------
    /// One input of the audio-in pill's card, by the name the machine answered
    /// with — `enter` attaches it as the beat source.
    Input,
    /// The arrangement menu's *save*. `enter` files the arrangement under the name
    /// in use, and asks for one where there is none.
    Save,
    /// The arrangement menu's *start a new one*.
    New,
    /// One name the arrangement menu lists — `enter` puts that arrangement back.
    Filed,

    // -- the Program bay ---------------------------------------------------
    /// The Program head's `solo` — `space` is *soloed* and *not*.
    Solo,
    /// The Program head's class pill — `space` opens the class or shuts it.
    Class,
    /// The picture. Its on and off is the Outputs row's one control.
    Picture,
    /// One of the four deck preview cells — a monitor, and a monitor is a thing you
    /// look at.
    Cell,

    // -- the Inspector -----------------------------------------------------
    /// A pane's deck head, which holds controls of its own.
    DeckHead,
    /// One of a pane's node groups, which holds controls of its own.
    Node,
    /// The deck head's sync chip — `space` cycles what the material allows.
    Sync,
    /// The deck head's anchor, scrubbed a quarter beat by the arrows.
    Anchor,
    /// The deck head's composite chip — `space` names the layering it is not in.
    Composite,
    /// A node head's authority chip — `space` cycles man, sug and auto.
    Authority,
    /// A node's renderer chips — `space` steps which one is live.
    Renderer,
    /// One parameter row of a node group, which is a level, and whose act is taking
    /// an attachment back.
    Param,

    // -- the Master chain --------------------------------------------------
    /// The master out, which is a level.
    Out,
    /// One slot of the master chain, which holds controls of its own.
    Slot,
    /// One parameter row of a slot, which is a level: the arrows step it a tenth of
    /// the range its procedure declares and `space` returns it to the value that
    /// procedure declared it at.
    ChainParam,
    /// A slot's cut chip — `space` cycles `mix` and `exit`. It is addressed on
    /// every slot and draws on a slot whose procedure declares `retains`; on one
    /// that does not it declines and says so.
    ChainCut,
    /// The `−` at the end of a slot's row — `enter` takes that slot out of the
    /// chain.
    ChainRemove,
    /// The Master bay's `+ add` — `enter` puts the chooser down, and the chooser's
    /// entries are the rung under it.
    AddEffect,
    /// One entry of the `+ add` chooser: a `kind L5` procedure the library holds.
    /// `enter` appends a slot of it and takes the card away.
    ChainProcedure,

    // -- the Sequencer -----------------------------------------------------
    /// The head's grid mode pill — `space` names the other of the two.
    Mode,
    /// One of the head's four bank pills — `space` arms that pattern.
    Bank,
    /// The foot's `+ lane` — `enter` puts the chooser down, and the chooser's
    /// entries are the rung under it.
    AddLane,
    /// One entry of the `+ lane` chooser: a target a lane can drive. `enter` points
    /// a lane at it and takes the card away.
    LaneTarget,
    /// A lane's label, whose mute is a state.
    Label,
    /// One step of a lane — a row of alike cells the arrows walk across while the
    /// lanes they sit in are walked down.
    Step,

    // -- the Outputs row ---------------------------------------------------
    /// A sink, which has exactly one state.
    Sink,

    // -- the Staging lane --------------------------------------------------
    /// A candidate row's *keep this candidate*.
    Keep,
    /// A candidate row's *put the node's previous version back*.
    Back,
}

/// What a control answers to, which is the one thing the grammar has to know
/// about a control and the whole of what decides which of the four keys act on
/// it.
///
/// ADR-0259's kinds, as the grammar reads them: a state is a closed list
/// `space` cycles, a level is a continuum the arrows step and `space` returns
/// to its default, an act is a control that performs rather than sets, and the
/// last three are what the walk found that the record's seven do not have a
/// word for — a row of alike cells, a continuum with no value it was declared
/// at, and a control that is drawn and answers nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answers {
    /// A closed list: `space` cycles it and the arrows decline, because the values
    /// of a closed list are not laid out on an axis.
    State,
    /// A continuum: the arrows step it and `space` returns it to the value it was
    /// declared at.
    Level,
    /// A control that performs: `enter` does it, and `space` and the arrows
    /// decline.
    Act(Act),
    /// One of a row (or a column) of alike controls the arrows walk and `space`
    /// sets — the Sequencer's steps, and the one place a bay walks two axes:
    /// `across` is this control's and [`Built::across`] is its items'.
    Cells { across: bool },
    /// A continuum the arrows step, with no value it was declared at — so `space`
    /// declines with this sentence where a level returns to its default.
    Track(&'static str),
    /// Drawn, addressed, and answering nothing, with the sentence that says why — a
    /// digit lands on it and the ring is drawn, which is ADR-0259's own finding
    /// about the four preview cells.
    Nothing(&'static str),
}

impl Control {
    /// What this control answers to. A `match` with no wildcard, so a control added
    /// to the panel is a compile error here rather than one the grammar quietly
    /// declines on.
    pub const fn answers(self) -> Answers {
        match self {
            Control::Tally | Control::Blend | Control::Mask => Answers::State,
            Control::Trim | Control::Fader => Answers::Level,
            Control::Shape | Control::Quantum | Control::Length => Answers::State,
            Control::Go => Answers::Act(Act::Go),
            Control::Scope => Answers::State,
            Control::Star => Answers::State,
            Control::Params => Answers::Act(Act::Read),
            // **The figure is a track and not a level**: a press positions it
            // inside a band that is a guard on a hand (ADR-0291) and the arrows
            // step it by one beat a minute, which is
            // [ADR-0350](../../../docs/adr/0350-the-transports-two-cards-are-walked-and-the-tempo-figure-steps-by-a-beat-a-minute.md).
            // What the grid runs at is not a value this row declares, so there
            // is no state for `space` to return it to.
            Control::Tempo => Answers::Track(
                "the tempo figure has no value it was declared at, so there is nothing for \
                 space to return it to — the arrows step it a beat a minute, a press names one \
                 outright, and b taps the beat",
            ),
            // The operand is the direction the key spells, which is the
            // operand rule's own second clause: `,` and `.` stay global.
            Control::Grid => Answers::Nothing(
                "the grid's only operand is the direction, which is what the two keys spell — \
                 press , to halve it and . to double it",
            ),
            Control::Offset => Answers::Level,
            Control::Beat => Answers::Nothing("the beat grid is a readout — b taps the beat"),
            Control::Bar => Answers::Nothing("the bar counter is a readout"),
            Control::Cost => Answers::Nothing("the frame cost is a readout"),
            Control::Audio | Control::Arrangement => Answers::Act(Act::Open),
            Control::Input => Answers::Act(Act::Attach),
            Control::Save => Answers::Act(Act::Save),
            // **The reset is one row of the page and `r` is the key that
            // reaches it**, so this row of the menu is drawn and is not
            // performed here.
            Control::New => Answers::Nothing(
                "start a new one is the reset, and r starts one from anywhere on this panel",
            ),
            Control::Filed => Answers::Act(Act::Restore),
            Control::Tonemap => Answers::State,
            Control::Exposure => Answers::Level,
            Control::Solo | Control::Class => Answers::State,
            // **The picture's on and off is the Outputs row's one control**,
            // and it is one row of the page with one badge. A second press for
            // it here would be a route no badge could name.
            Control::Picture => Answers::Nothing(
                "the picture's on and off is the Outputs row's sink — tab to the outputs row \
                 and press space on it",
            ),
            Control::Cell => Answers::Nothing(
                "a preview cell is a monitor fixed to a deck, so there is nothing here to set \
                 or to perform — the residency is the mixer's tally chip",
            ),
            // The two rungs of the Inspector that hold controls rather than
            // being ones: a digit descends and the press is a move.
            Control::DeckHead | Control::Node => Answers::Nothing(
                "this is a rung and not a control — press a digit to name one of the controls \
                 in it",
            ),
            Control::Sync | Control::Composite | Control::Authority | Control::Renderer => {
                Answers::State
            }
            Control::Anchor | Control::Param => Answers::Level,
            Control::Out => Answers::Level,
            // A rung and not a control, on the Inspector's two rungs' terms.
            Control::Slot => Answers::Nothing(
                "a chain slot is a rung and not a control — press a digit to name one of the \
                 controls in it",
            ),
            Control::ChainParam => Answers::Level,
            Control::ChainCut => Answers::State,
            Control::ChainRemove => Answers::Act(Act::Remove),
            Control::AddEffect => Answers::Act(Act::Open),
            Control::ChainProcedure => Answers::Act(Act::Add),
            Control::Mode | Control::Bank => Answers::State,
            Control::AddLane => Answers::Act(Act::Open),
            Control::LaneTarget => Answers::Act(Act::Point),
            Control::Label => Answers::State,
            Control::Step => Answers::Cells { across: true },
            Control::Sink => Answers::State,
            Control::Keep => Answers::Act(Act::Keep),
            Control::Back => Answers::Act(Act::Back),
        }
    }

    /// Whether this control is a level — a continuum the arrows step and `space`
    /// returns to its default.
    pub const fn level(self) -> bool {
        matches!(self.answers(), Answers::Level)
    }

    /// What `enter` on this control performs, or `None` for one that sets rather
    /// than performs.
    ///
    /// It is [`Control::answers`] with one addition, and the addition is the one
    /// place a control is two of ADR-0259's kinds at once: a parameter row is a
    /// level and it also performs, because the sensitivity row under it carries
    /// `take back` and that is the act of the control the row draws rather than a
    /// control of its own — a parameter with nothing holding it draws no
    /// sensitivity row at all. A fourth rung for one chip would be a rung whose
    /// only inhabitant is sometimes there.
    pub const fn acts(self) -> Option<Act> {
        match self.answers() {
            Answers::Act(act) => Some(act),
            _ => match self {
                Control::Param => Some(Act::TakeBack),
                _ => None,
            },
        }
    }

    /// Which of the four keys act on this control.
    pub const fn reached_by(self, key: Grammar) -> bool {
        match key {
            Grammar::Enter => self.acts().is_some(),
            // **Two rungs are not controls**, and a digit descends through
            // them — which is the only way the Inspector's third rung is
            // reached at all. **`+ lane` is a control and a rung both**: it
            // performs — `enter` puts the chooser down — and a digit then
            // names the nth entry of the card it put there.
            Grammar::Digit => matches!(
                self,
                Control::DeckHead
                    | Control::Node
                    | Control::AddLane
                    | Control::Audio
                    | Control::Arrangement
                    | Control::Slot
                    | Control::AddEffect
            ),
            // **A card's rows are walked**, so the arrows reach the control
            // that opens the card and the rows under it, where they reach
            // neither of the other two acts.
            Grammar::Arrows
                if matches!(
                    self,
                    Control::AddLane
                        | Control::LaneTarget
                        | Control::Audio
                        | Control::Arrangement
                        | Control::Input
                        | Control::Save
                        | Control::New
                        | Control::Filed
                        | Control::AddEffect
                        | Control::ChainProcedure
                ) =>
            {
                true
            }
            _ => match (self.answers(), key) {
                // A state has a next value and a level has a default, so
                // `space` acts on both — which is what makes it the key of the
                // grammar an operator reaches for. A cell is set by it too.
                (Answers::State | Answers::Level | Answers::Cells { .. }, Grammar::Space) => true,
                // A level's neighbour is its next value and a cell's is the
                // cell beside it. A track's is its next value too. A state has
                // neither.
                (Answers::Level | Answers::Track(_) | Answers::Cells { .. }, Grammar::Arrows) => {
                    true
                }
                _ => false,
            },
        }
    }
}

/// What `enter` on an addressed thing performs.
///
/// One variant per act the nine bays draw, which is what keeps `enter` a rule
/// about a kind rather than a per-bay verb: where an item has two acts they are
/// two controls and a digit chooses between them, which is the Staging lane's
/// `n 1` and `n 2`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    /// The Set under the Library's cursor, loaded onto the selected deck — with
    /// both operands on screen before the press, which is what that argument was
    /// always about.
    Load,
    /// What one Set holds and declares, opened on the row the cursor is on.
    Read,
    /// The candidate on this row, kept.
    Keep,
    /// The node's previous version, put back.
    Back,
    /// The transition the Mixer's head is set to, run on the addressed strip.
    Go,
    /// The attachment on a parameter row, taken back — after it the row is a handle
    /// again.
    TakeBack,
    /// The chooser under the addressed control, put down. After it the card's
    /// entries are the rung below the control and the address descends into them by
    /// digit or by arrow.
    Open,
    /// A lane, pointed at the target on the addressed entry. After it the card is
    /// gone and the address is back on the control that opened it.
    Point,
    /// The input on the addressed row of the audio-in card, attached as the beat
    /// source.
    Attach,
    /// The arrangement, filed under the name in use — or a name asked for, where
    /// there is none in use.
    Save,
    /// The arrangement named on the addressed row, put back.
    Restore,
    /// The addressed slot, taken out of the master chain.
    Remove,
    /// A slot of the addressed procedure, appended to the master chain. After it
    /// the card is gone and the address is back on `+ add`.
    Add,
}

/// What the things at one rung are made of: the controls drawn first, and the
/// control every one after them is.
///
/// A fixed list is not enough for two of the nine. An Inspector pane draws one
/// deck head and then a node group per node of the Set in the slot, and a
/// Sequencer lane draws one label and then a cell per step of the mode — so a
/// rung is *a prefix and a repeat*, and how many of the repeat there are is the
/// bay's own reading rather than anything written here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Of {
    /// The controls drawn before the counted ones, in draw order.
    pub first: &'static [Control],
    /// The control every thing after them is, or `None` where a thing draws only
    /// the controls above.
    pub then: Option<Control>,
    /// The controls drawn *after* the counted ones, in draw order.
    ///
    /// The Master bay is what asks for it: its items are the out fader, then one
    /// per slot of the chain, then `+ add`, and a rung that is a prefix and a
    /// repeat cannot say that. Every entry here is drawn whenever the rung is, so
    /// the count a bay reports includes all of them — which is what lets the nth
    /// be resolved from the end
    /// ([ADR-0352](../../../docs/adr/0352-the-chains-list-is-the-master-bays-items-and-a-slot-is-taken-out-by-a-glyph-on-its-row.md)).
    pub last: &'static [Control],
}

impl Of {
    /// A rung with nothing on it.
    pub const NONE: Of = Of {
        first: &[],
        then: None,
        last: &[],
    };

    /// A rung that is a fixed list and nothing after it.
    pub const fn just(first: &'static [Control]) -> Of {
        Of {
            first,
            then: None,
            last: &[],
        }
    }

    /// The `nth` control of this rung, counting from one, against a rung drawing
    /// `drawn` of them — or `None` past the end.
    ///
    /// `drawn` is the bay's reading and not this table's: a pane with two nodes
    /// draws three things at its second rung and a pane with nine draws ten, and a
    /// digit counts what was drawn.
    pub fn nth(&self, nth: usize, drawn: usize) -> Option<Control> {
        if nth == HEAD || nth > drawn {
            return None;
        }
        if let Some(control) = self.first.get(nth - 1) {
            return Some(*control);
        }
        // **The suffix is counted from the end**, which is what makes a rung
        // whose middle repeats resolvable at all: how many of the repeat there
        // are is the bay's reading, and the controls after them are however
        // many this table names.
        let after = drawn - nth;
        if let Some(control) = self.last.len().checked_sub(after + 1) {
            return Some(self.last[control]);
        }
        self.then
    }

    /// How many things this rung draws where the repeat runs `repeats` times.
    pub fn len(&self, repeats: usize) -> usize {
        self.first.len()
            + match self.then {
                Some(_) => repeats,
                None => 0,
            }
            + self.last.len()
    }

    /// Whether any control of this rung is reached by `key`, against a rung drawing
    /// everything it can.
    fn reaches(&self, key: Grammar) -> bool {
        self.first.iter().any(|c| c.reached_by(key))
            || self.then.is_some_and(|c| c.reached_by(key))
            || self.last.iter().any(|c| c.reached_by(key))
    }
}

/// What a bay's items are.
///
/// Two shapes, and the split is ADR-0259's own: a headless row's *"items are
/// the controls left to right"*, and every other bay's items are alike things
/// with controls under them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Items {
    /// The bay's items are its controls, so a digit at bay level names one outright
    /// and there is no item rung at all — the Transport, the Outputs row, the
    /// Master chain and the Program bay.
    ///
    /// An [`Of`] and not a plain list, because the Master's items are the out
    /// fader, one per slot of the chain, and `+ add`.
    Controls(Of),
    /// The bay lists alike things and a digit names the nth of them; these are the
    /// controls one of them draws.
    Alike(Of),
}

/// One bay's grammar: what each level of its address is made of.
///
/// The table [`BUILT`] is written in, and the thing
/// `crates/karakuri/src/main.rs`'s `key_column` reads in place of this
/// program's `match` — a digit reaches a different row in every bay, so a scan
/// of the arms cannot say which row a press lands on and a dispatch is what can
/// ([ADR-0259](../../../docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md),
/// [ADR-0333](../../../docs/adr/0333-the-console-resolves-the-address-and-the-window-loop-names-the-operation.md),
/// [ADR-0343](../../../docs/adr/0343-the-grammar-reaches-all-nine-bays-and-space-on-a-bay-is-the-fold.md)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Built {
    /// The bay, by the name the arrangement gives it.
    pub bay: &'static str,
    /// The controls of the bay's head, in the order the head draws them — empty for
    /// a head that holds none and for a bay that draws none.
    pub head: &'static [Control],
    /// What the bay's items are.
    pub items: Items,
    /// The third rung: the control the address descends *through*, and what is
    /// under it. ADR-0259 calls the Inspector three deep and says it is *"what
    /// proves the address has to be a path rather than two levels"*.
    ///
    /// Four bays have one, and which rung it hangs off is what differs. The
    /// Inspector's is under an *item* — a pane's deck head and its node groups.
    /// The Sequencer's is under a *head* control, `+ lane`. The Transport's two
    /// and the Master's `+ add` are under a headless row's own controls, and the
    /// Master's slots are there too. Every one of those but the slots is a card
    /// the address descends into and `esc` takes away.
    pub under: &'static [(Control, Of)],
    /// What `enter` on an item performs, or `None` where an item is not an act.
    pub act: Option<Act>,
    /// Whether naming an item is itself an operation. The Mixer's is: a digit that
    /// names a strip *is* the deck selection, which is why that row keeps a key
    /// badge rather than losing one.
    pub selects: bool,
    /// Which way the arrows walk this bay's items — `true` for a row, so `←→`, and
    /// `false` for a column, so `↑↓`. It is a reading of how the bay draws them and
    /// not a preference.
    pub across: bool,
}

impl Built {
    /// The controls of one of this bay's items, or of the bay itself where its
    /// items are its controls.
    pub fn item(&self) -> Of {
        match self.items {
            Items::Controls(of) => of,
            Items::Alike(of) => of,
        }
    }

    /// What is under `control`, or [`Of::NONE`] for a control nothing is under.
    pub fn beneath(&self, control: Control) -> Of {
        self.under
            .iter()
            .find(|(found, _)| *found == control)
            .map_or(Of::NONE, |(_, of)| *of)
    }

    /// Whether one of the four keys acts anywhere in this bay, derived from what
    /// the bay is made of rather than listed beside it — a second list would be a
    /// second answer to *what does `space` do here*.
    ///
    /// The fold is not counted here. `space` at bay level folds every one of the
    /// nine, so counting it would make this answer `true` for `space` in a bay
    /// whose controls answer nothing — and the page would then owe nine badges for
    /// one rule. It is declared once, under [`ANY`], by [`reaches`].
    pub fn reaches(&self, key: Grammar) -> bool {
        match key {
            // **A digit names the nth item, and `0` the head.** Every bay the
            // manual lists draws items, and every bay has a head or something
            // standing in for one, so a digit reaches somewhere in any bay
            // whose grammar is built at all.
            Grammar::Digit => true,
            // **The arrows walk those items**, whatever the items are made of,
            // and step whatever levels they hold on top of that.
            Grammar::Arrows => true,
            _ => {
                self.head.iter().any(|c| c.reached_by(key))
                    || self.item().reaches(key)
                    || self.under.iter().any(|(_, of)| of.reaches(key))
                    || (key == Grammar::Enter && self.act.is_some())
            }
        }
    }
}

/// The nine bays' grammar, and what each is made of.
///
/// ADR-0259 walks all nine and this is the whole of that walk: ADR-0333 built
/// the Mixer and the Library, and ADR-0343 the seven that were left. A bay with
/// nothing of a kind is recorded as a finding rather than smoothed over, which
/// is the record's own instruction — the Staging lane holds no state at all,
/// the Outputs row holds nothing but states, and the Program bay's four cells
/// answer neither of the two keys that act on a thing.
pub const BUILT: &[Built] = &[
    // -- Transport ---------------------------------------------------------
    //
    // **A headless row (ADR-0159), so `0` names the row itself** and there is
    // nothing under it: this bay's controls *are* its items, which is what
    // `Items::Controls` is.
    //
    // **The order is the record's walk, left to right**, with the offset among
    // the tracker group's three where it is drawn. **Four drawn things are not
    // items**: the `tap` capsule, which ADR-0259 keeps global because *"tapping
    // is a hand keeping time, which a `Tab` first would spoil"*, and the `rec`,
    // `learn` and `map` pills, which the walk does not name — the first two
    // reach rows the key column marks `gap` for reasons that are not about
    // letters, and the third is a readout.
    Built {
        bay: TRANSPORT,
        head: &[],
        items: Items::Controls(Of::just(&[
            Control::Tempo,
            Control::Grid,
            Control::Offset,
            Control::Beat,
            Control::Bar,
            Control::Cost,
            Control::Audio,
            Control::Arrangement,
            Control::Tonemap,
            Control::Exposure,
        ])),
        // **Two of those controls are rungs as well**, and they are the only
        // ones under a bay whose items are its controls: the audio-in pill's
        // card lists the machine's inputs, and the arrangement pill's menu
        // lists *save*, *start a new one* and every name filed. `enter` puts
        // each card down, a digit names the nth row of it, `↑↓` walk the rows
        // and `esc` takes the card away
        // ([ADR-0350](../../../docs/adr/0350-the-transports-two-cards-are-walked-and-the-tempo-figure-steps-by-a-beat-a-minute.md)).
        under: &[
            (
                Control::Audio,
                Of {
                    last: &[],
                    first: &[],
                    then: Some(Control::Input),
                },
            ),
            (
                Control::Arrangement,
                Of {
                    last: &[],
                    first: &[Control::Save, Control::New],
                    then: Some(Control::Filed),
                },
            ),
        ],
        act: None,
        selects: false,
        across: true,
    },
    // -- Library -----------------------------------------------------------
    //
    // **`0` is the head and its controls are the scope chips**; items are the
    // listed Sets, the digits name the first nine and `↑↓` walk them.
    //
    // **A row's own controls are its star and its `params` chip**, which is
    // what ADR-0333 left owed: ADR-0259's *"a row has no state"* was written
    // before the star was drawn, and a star is a state.
    //
    // **The row menu is drawn and is not one of them.** Its items are a card,
    // `input::claim`'s rule 2 gives every press on the console to the panel
    // while one is down, and a card the grammar can open and cannot then walk
    // is a control that traps the address.
    Built {
        bay: LIBRARY,
        head: &[Control::Scope],
        items: Items::Alike(Of::just(&[Control::Star, Control::Params])),
        under: &[],
        act: Some(Act::Load),
        selects: false,
        across: false,
    },
    // -- Staging -----------------------------------------------------------
    //
    // **Nothing here has a state at all**, so `space` reaches nothing in this
    // bay below its fold — ADR-0259's own finding, and the second of the two
    // bays that are lists of things that happened rather than things you set.
    // A row's controls are its two acts, which is the record's `n 1` and `n 2`.
    Built {
        bay: STAGING,
        head: &[],
        items: Items::Alike(Of::just(&[Control::Keep, Control::Back])),
        under: &[],
        act: None,
        selects: false,
        across: false,
    },
    // -- Program -----------------------------------------------------------
    //
    // **`0` is the head, whose controls are `solo` and the class pill.**
    // `space` on `solo` is the state *soloed* and *not*, which is `s` and `u`
    // collapsed into the one control they always described.
    //
    // **Items are the picture and the four preview cells**, and neither the
    // picture nor a cell answers a key: a cell is a monitor (ADR-0243,
    // ADR-0240) and the picture's on and off is the Outputs row's one control.
    // It is the clearest case in the walk of items with neither a state nor an
    // act, and it is correct.
    Built {
        bay: PROGRAM,
        head: &[Control::Solo, Control::Class],
        items: Items::Controls(Of::just(&[
            Control::Picture,
            Control::Cell,
            Control::Cell,
            Control::Cell,
            Control::Cell,
        ])),
        under: &[],
        act: None,
        selects: false,
        across: true,
    },
    // -- Inspector ---------------------------------------------------------
    //
    // **Three deep, and the deepest bay on the panel.** Items are its panes; a
    // pane's controls are its deck head and its node groups; a node group's
    // controls are its authority chip, its renderer chips and its parameter
    // rows. So `1 2 3` is the first pane's second thing's third control.
    //
    // **A parameter's act is taking an attachment back**, which is the
    // sensitivity row's `take back` reached one rung up rather than a fourth
    // rung of its own: a row with no attachment has no sensitivity row under
    // it, so the act is the parameter's where there is one to take back.
    Built {
        bay: INSPECTOR,
        head: &[],
        items: Items::Alike(Of {
            last: &[],
            first: &[Control::DeckHead],
            then: Some(Control::Node),
        }),
        under: &[
            (
                Control::DeckHead,
                Of::just(&[Control::Sync, Control::Anchor, Control::Composite]),
            ),
            (
                Control::Node,
                Of {
                    last: &[],
                    first: &[Control::Authority, Control::Renderer],
                    then: Some(Control::Param),
                },
            ),
        ],
        act: None,
        selects: false,
        across: false,
    },
    // -- Mixer -------------------------------------------------------------
    //
    // **Items are the four strips and a strip's controls are its five**, in
    // the order `view::mixer` draws them down the strip. So `2 3` is deck B's
    // fader, which is ADR-0259's own example and the control the mock draws
    // `.wfocus` on.
    //
    // **The transition row is the head's**, which is the question ADR-0333
    // left open. A head is *"where a bay keeps the controls that are about the
    // bay rather than about anything in it"*, and `Operation::SetTransition`
    // is the one row of this bay that carries no slot: the settings decide what
    // the next move means wherever it lands. The `go` capsule is the fourth,
    // and it acts on the **addressed strip** — the deck selection, which is
    // this bay's remembered address.
    Built {
        bay: MIXER,
        head: &[
            Control::Shape,
            Control::Quantum,
            Control::Length,
            Control::Go,
        ],
        items: Items::Alike(Of::just(&[
            Control::Tally,
            Control::Trim,
            Control::Fader,
            Control::Blend,
            Control::Mask,
        ])),
        under: &[],
        act: None,
        selects: true,
        across: true,
    },
    // -- Master ------------------------------------------------------------
    //
    // **Items are the out fader, the chain's slots and `+ add`**, in the order
    // the bay draws them. `↑↓` step `out` and `space` returns it to its
    // default.
    //
    // **A slot is a rung as well as an item**: its controls are its parameter
    // rows, its cut chip and its `−`. `↑↓` on a parameter row step it a tenth
    // of the range its procedure declares and `space` returns it to what that
    // procedure declared it at; `space` on the cut chip cycles `mix` and
    // `exit`; `enter` on the `−` takes the slot out of the chain.
    //
    // **The cut chip keeps its number on a slot that draws none**, so that a
    // digit means the same control on every slot of the chain; the chip then
    // declines and says that the slot's procedure declares no `retains`
    // ([ADR-0352](../../../docs/adr/0352-the-chains-list-is-the-master-bays-items-and-a-slot-is-taken-out-by-a-glyph-on-its-row.md)).
    //
    // **`+ add` is a rung too**, and the second on this panel after `+ lane`:
    // `enter` puts its chooser down and the card's entries — the `kind L5`
    // procedures the library holds — are what is under it (ADR-0351).
    Built {
        bay: MASTER,
        head: &[],
        items: Items::Controls(Of {
            first: &[Control::Out],
            then: Some(Control::Slot),
            last: &[Control::AddEffect],
        }),
        under: &[
            (
                Control::Slot,
                Of {
                    first: &[],
                    then: Some(Control::ChainParam),
                    last: &[Control::ChainCut, Control::ChainRemove],
                },
            ),
            (
                Control::AddEffect,
                Of {
                    first: &[],
                    then: Some(Control::ChainProcedure),
                    last: &[],
                },
            ),
        ],
        act: None,
        selects: false,
        across: false,
    },
    // -- Sequencer ---------------------------------------------------------
    //
    // **`0` is the head**: the grid mode pill, the four bank pills that name
    // which pattern, and `+ lane`, which the record calls the head's act.
    //
    // **Sixteen steps outrun ten digits**, which is the one place the digits
    // fail outright: a lane's controls are its label and its steps, the digits
    // reach the label and the first eight steps, and the rest are walked. **It
    // is also the one bay that uses both axes** — the lanes are a column and a
    // lane's cells are a row — which is `Answers::Cells`.
    //
    // **`+ lane` is a rung as well as a control**, and the only one under a
    // head: `enter` on it puts the chooser down and the card's entries are
    // what is under it, one per target a lane can drive, in the order the card
    // lists them. They are a column, so `↑↓` walk them
    // ([ADR-0351](../../../docs/adr/0351-the-lane-chooser-is-a-rung-of-the-address.md)).
    Built {
        bay: SEQUENCER,
        head: &[
            Control::Mode,
            Control::Bank,
            Control::Bank,
            Control::Bank,
            Control::Bank,
            Control::AddLane,
        ],
        items: Items::Alike(Of {
            last: &[],
            first: &[Control::Label],
            then: Some(Control::Step),
        }),
        under: &[(
            Control::AddLane,
            Of {
                last: &[],
                first: &[],
                then: Some(Control::LaneTarget),
            },
        )],
        act: None,
        selects: false,
        across: false,
    },
    // -- Outputs -----------------------------------------------------------
    //
    // **Headless again, so `0` names the row itself.** Items are the sinks and
    // each has exactly one state, so `space` is the whole of this bay: no item
    // here has an act and none has a level. It is the simplest of the nine and
    // it is what the grammar looks like with one kind in it.
    Built {
        bay: OUTPUTS,
        head: &[],
        items: Items::Controls(Of::just(&[Control::Sink])),
        under: &[],
        act: None,
        selects: false,
        across: true,
    },
];

/// The picture, by the name the arrangement gives it — the node the Program
/// bay's `solo` acts on and the Outputs row's one sink turns on and off. A
/// constant for [`MIXER`]'s reason.
const PICTURE: &str = "program-view";

/// The Transport row, by the name the arrangement gives it. [`MIXER`]'s reason.
pub const TRANSPORT: &str = "transport";
/// The Staging lane. [`MIXER`]'s reason.
pub const STAGING: &str = "staging";
/// The Program bay. [`MIXER`]'s reason.
pub const PROGRAM: &str = "program";
/// The Inspector. [`MIXER`]'s reason.
pub const INSPECTOR: &str = "inspector";
/// The Master chain. [`MIXER`]'s reason.
pub const MASTER: &str = "master";
/// The Sequencer. [`MIXER`]'s reason.
pub const SEQUENCER: &str = "sequencer";
/// The Outputs row. [`MIXER`]'s reason.
pub const OUTPUTS: &str = "outputs";

/// What `bay` is made of, or `None` for one whose grammar is not built.
pub fn built(bay: &str) -> Option<&'static Built> {
    BUILT.iter().find(|found| found.bay == bay)
}

/// Every (bay, key) pair the grammar binds, flattened — the dispatch table as a
/// check can read it.
///
/// `crates/karakuri/src/main.rs`'s `key_column` holds the *rows* each pair
/// reaches, because a page heading is what a check reads and is not something
/// this program says to anybody; this is the half that says which pairs exist,
/// and the two are held against each other in both directions.
///
/// [`ANY`] is the first pair and is not a bay. `space` at bay level is the
/// fold, and it works in every one of the nine — so it is one route naming all
/// of them rather than nine routes naming one row, which is what a badge
/// reading `space &middot; in any bay` says.
pub fn reaches() -> Vec<(&'static str, Grammar)> {
    let mut found = vec![(ANY, Grammar::Space)];
    for bay in BUILT {
        for key in Grammar::ALL {
            if bay.reaches(key) {
                found.push((bay.bay, key));
            }
        }
    }
    found
}

/// Where a bay's address has actually got to, resolved against what the bay is
/// drawing now.
///
/// A path is a list of digits and the bay is what says whether they name
/// anything: a listing that shrank between two frames leaves an address on a
/// row that is gone, exactly as it leaves the library cursor past the end.
/// `None` is that case, and the press says so rather than acting on the nearest
/// thing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Addressed {
    /// The bay itself, which is where every bay starts and where `esc` stops.
    Bay,
    /// The bay's head — `0`.
    Head,
    /// The nth control of the head.
    OfHead(usize, Control),
    /// A rung under one of the head's controls: the number of the head control the
    /// address descended through, then the number of the thing under it and which
    /// it is. The Sequencer's `+ lane` chooser is the one of these.
    UnderHead {
        through: usize,
        nth: usize,
        control: Control,
    },
    /// A rung under one of the bay's own controls, in a bay whose items are its
    /// controls: the number of the control the address descended through, then the
    /// number of the row under it and which row it is. The Transport's two cards
    /// are the ones of these.
    InCard {
        through: usize,
        nth: usize,
        control: Control,
    },
    /// The nth item — only in a bay that lists items.
    Item(usize),
    /// A control: the item it is under, `None` in a bay whose items are its
    /// controls, then the control's own number and which it is.
    Of {
        item: Option<usize>,
        nth: usize,
        control: Control,
    },
    /// The third rung: the item, the number of the control the address descended
    /// through, then the number of the thing under it and which it is.
    Under {
        item: usize,
        through: usize,
        nth: usize,
        control: Control,
    },
}

/// Where `at` has got to in `built`, against a bay whose rungs draw what
/// `drawn` says.
///
/// `drawn` is asked for a path and answers how many things the bay draws under
/// it: `[]` is how many items, `[n]` how many controls the nth item draws, and
/// `[n, c]` how many things are under that control. It is a closure rather than
/// a count because a bay is three deep and one number cannot say that.
pub fn addressed(
    built: &Built,
    at: &[usize],
    drawn: impl Fn(&[usize]) -> usize,
) -> Option<Addressed> {
    let items = drawn(&[]);
    match *at {
        [] => Some(Addressed::Bay),
        [HEAD] => Some(Addressed::Head),
        [HEAD, nth] => built
            .head
            .get(nth.checked_sub(1)?)
            .copied()
            .map(|control| Addressed::OfHead(nth, control)),
        // **The rung under a head control**, which the Sequencer's chooser is
        // the one of: it is matched before the item arms below because `HEAD`
        // is zero and a path of three digits starting with one would otherwise
        // read as an item, a control and a thing under it.
        [HEAD, through, nth] => {
            let above = built.head.get(through.checked_sub(1)?).copied()?;
            built
                .beneath(above)
                .nth(nth, drawn(&[HEAD, through]))
                .map(|control| Addressed::UnderHead {
                    through,
                    nth,
                    control,
                })
        }
        // **The rung under one of a headless row's own controls**, which the
        // Transport's two cards are: the control rung is the first, so a card
        // it puts down is the second. A card that is not down draws no rows,
        // so `drawn` answers zero and this resolves to nothing.
        [through, nth] if matches!(built.items, Items::Controls(_)) => {
            let above = built.item().nth(through, items)?;
            built
                .beneath(above)
                .nth(nth, drawn(&[through]))
                .map(|control| Addressed::InCard {
                    through,
                    nth,
                    control,
                })
        }
        // A headless row's items are its controls, so the first rung *is* the
        // control rung and there is nothing below it.
        [nth] if matches!(built.items, Items::Controls(_)) => {
            built.item().nth(nth, items).map(|control| Addressed::Of {
                item: None,
                nth,
                control,
            })
        }
        [nth] if nth <= items => Some(Addressed::Item(nth)),
        [nth, control] if nth <= items && matches!(built.items, Items::Alike(_)) => built
            .item()
            .nth(control, drawn(&[nth]))
            .map(|found| Addressed::Of {
                item: Some(nth),
                nth: control,
                control: found,
            }),
        [nth, through, control] if nth <= items && matches!(built.items, Items::Alike(_)) => {
            let above = built.item().nth(through, drawn(&[nth]))?;
            built
                .beneath(above)
                .nth(control, drawn(&[nth, through]))
                .map(|found| Addressed::Under {
                    item: nth,
                    through,
                    nth: control,
                    control: found,
                })
        }
        _ => None,
    }
}

/// What the deck is holding right now, read at the press rather than off the
/// strip a frame copied.
///
/// The three states a strip's chips cycle, and the angle the mask is wearing.
/// This crate owns the cycle and not the reading: a scheduled fade landing
/// between the frame and the press would leave `view::Strip` a value the deck
/// has already left behind, which is `crates/karakuri/src/main.rs`'s own rule
/// at the three keys this replaces — *"read off the deck and not off the
/// strip"*.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Held {
    /// What the deck was last asked for, which is what the tally cycles from — see
    /// `view::Mixer::tally`, whose argument this is one surface along.
    pub requested: Tally,
    pub blend: BlendMode,
    pub mask: Mask,
    /// The angle the slot is already wearing, carried through unchanged (ADR-0203).
    pub mask_angle: f32,
}

/// Which way a press took a level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Up,
    Down,
    /// `space` — back to the value the control was declared at, which is the one
    /// state a continuum has.
    Default,
}

/// Which level a press landed on, since the host names a different operation
/// for each and reads a different thing to step from.
///
/// The four here are the levels whose value is the world's — a deck's, the
/// engine's, the audio session's — so the host reads them and does the
/// arithmetic, which is ADR-0333's seam and `karakuri-cli`'s parity argument.
/// The two the console steps itself are the Inspector's, and they are not here
/// for `view::ParamGrip`'s reason: a parameter's address, range and value are
/// all on the pane the frame drew, and there is no second reading to take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Trim(u8),
    Fader(u8),
    /// The Master bay's out.
    Out,
    /// The Transport's free-run tempo, stepped a beat a minute (ADR-0350).
    Tempo,
    /// The Transport's exposure.
    Exposure,
    /// The Transport's latency offset.
    Offset,
}

/// What a press of one of the four keys asks for.
///
/// The console resolves the address and says what was landed on; the window
/// loop names the operation, because the levels above need the world in front
/// of them and two of the answers reach the store (ADR-0333).
#[derive(Debug, Clone, PartialEq)]
pub enum Asked {
    /// Nothing there answers this key, and the sentence that says why — a key that
    /// declines silently and a key that is not bound are the same experience
    /// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).
    Nothing(&'static str),
    /// The address moved and nothing was asked of anything.
    Moved,
    /// One operation, named outright — the deck selection and the controls whose
    /// next state is this console's own affordance.
    Emitted(Operation),
    /// A move of the arrangement, which is not the vocabulary's: the fold `space`
    /// performs on a bay, and the Program head's solo.
    Panel(Op),
    /// The picture's on and off, which is one press asking for two things: the
    /// operation that names the output, and the fold that carries it out.
    Routed(Operation, Op),
    /// A level, and which way the press went. The host reads the value off the
    /// world and names the destination, because the size of a step and the clamp on
    /// it are its arithmetic and `karakuri-cli`'s.
    Stepped { level: Level, step: Step },
    /// The Library head's scope, stepped — the host performs it and re-reads the
    /// listing, which is a directory read and not a thing this crate can do at all
    /// (ADR-0156).
    Scope,
    /// The Set under the Library's cursor, loaded onto the selected deck — the
    /// host's for the scope's reason, and because a preset row is taken into the
    /// store on the way.
    Load,
    /// A press on the audio-in pill or on one of its rows, in that control's own
    /// words. The host performs it through the method a pointer press already
    /// reaches, because opening the card enumerates the machine's inputs and this
    /// crate takes no device (ADR-0156).
    Listened(AudioAsk),
    /// A press on the arrangement pill or on one of its menu rows, in that
    /// control's own words. The host performs it through the method a pointer press
    /// already reaches: the names filed are a directory and the save writes a file.
    Arranged(Ask),
}

/// The sentence for a key pressed in a bay whose grammar is not built. No bay
/// is, as of ADR-0343, and the arm stays because a tenth region added to the
/// panel would arrive here rather than at a panic.
const NOT_BUILT: &str = "the six keys are not built in this bay yet — space still folds it, and \
                         tab moves on to the next";

/// What a press asks for, and the address moved to answer it.
///
/// The one entry point for the four keys of the grammar. It resolves the
/// focused bay's address against what that bay is drawing, moves the address
/// where the key says to, and answers what the host has to do about it.
///
/// # Every write goes through the method that already refused it
///
/// A digit that names a strip calls `View::select`, one that names a row calls
/// `View::point_at`, an arrow that walks the library calls `View::walk` and
/// `space` on a chip calls `View::step_scope` — so a deck the mixer draws no
/// strip for, a row past the listing and a scope with no chip are refused
/// exactly where they were refused before, and the address descends only where
/// the refusal did not fire. The grammar adds a route and no exception.
///
/// `held` is asked what the deck is holding, and only where a press needs a
/// state to cycle from. A closure rather than a value, because which strip to
/// read is what the address says and the address is what this function
/// resolves. It answers `None` where this deck has no slot behind that strip,
/// and the press then declines and says so.
pub fn press(
    view: &mut View,
    panel: &Panel,
    key: Press,
    held: impl FnOnce(u8) -> Option<Held>,
) -> Asked {
    let Some(bay) = view.focused(panel) else {
        return Asked::Nothing("this arrangement draws no bay to address a key to");
    };
    // **A folded bay answers `space` and nothing else.** That is the narrow
    // reason it keeps its place in the ring — it is there so that there is
    // something to press to open it, not so that it can be operated — and a
    // digit, `enter` and the arrows decline *whatever the address had reached*
    // inside it, because nothing is drawn for them to land on (ADR-0259).
    //
    // **`space` opens it from wherever the address was**, for the same reason:
    // the press is about the bay and not about what is under it.
    if panel
        .layout()
        .find(bay.name)
        .is_some_and(|id| !panel.layout().visible(id))
    {
        return match key {
            Press::Space => fold(panel, bay),
            _ => Asked::Nothing(
                "this bay is folded away, so there is nothing drawn for this key to land on — \
                 space opens it, and tab moves on",
            ),
        };
    }
    // **The fold is answered before the bay's own grammar**, and it is the one
    // press that acts the same in all nine: `space` at bay level is the fold
    // wherever focus is, which is what lets one badge name all nine places it
    // works (ADR-0259, ADR-0343).
    if key == Press::Space
        && view
            .focus()
            .address(bay.name)
            .is_none_or(|address| address.at().is_empty())
    {
        return fold(panel, bay);
    }
    let Some(built) = built(bay.name) else {
        return Asked::Nothing(NOT_BUILT);
    };
    let at: Vec<usize> = view
        .focus()
        .address(bay.name)
        .map_or_else(Vec::new, |address| address.at().to_vec());
    let items = drawn(view, built, &[]);
    let Some(here) = addressed(built, &at, |path| drawn(view, built, path)) else {
        // The address is on something the bay has stopped drawing. Put it back
        // at the bay rather than acting on whatever has taken that position —
        // `View::point_at`'s rule about a row past the listing, one level up.
        view.focus_mut().address_mut(bay.name).to_the_bay();
        return Asked::Nothing(
            "the address was on something this bay has stopped drawing, so it is back at the \
             bay — press a digit to name what is there now",
        );
    };
    match (here, key) {
        // ------------------------------------------------------------------
        // A digit — the nth thing one level below the address, `0` the head
        // ------------------------------------------------------------------
        (Addressed::Bay, Press::Digit(HEAD)) => match built.head.is_empty() {
            true => Asked::Nothing(
                "this bay draws no head, so 0 names the row itself — press 1 for the first of \
                 the controls in it",
            ),
            false => {
                address_into(view, bay.name, HEAD);
                Asked::Moved
            }
        },
        (Addressed::Bay, Press::Digit(nth)) => name_item(view, bay.name, built, nth, items),
        (Addressed::Head, Press::Digit(nth)) => match built.head.get(nth.wrapping_sub(1)) {
            Some(_) => {
                address_into(view, bay.name, nth);
                Asked::Moved
            }
            None => Asked::Nothing(
                "this bay's head does not draw that many controls — the digits count what is \
                 drawn, from one",
            ),
        },
        (Addressed::Item(item), Press::Digit(nth)) => {
            match built.item().nth(nth, drawn(view, built, &[item])) {
                Some(_) => {
                    address_into(view, bay.name, nth);
                    Asked::Moved
                }
                None => Asked::Nothing(
                    "this item does not draw that many controls — the digits count what is \
                     drawn, from one",
                ),
            }
        }
        // **A digit below a control**, which only the Inspector has: the deck
        // head and a node group are rungs rather than controls, and a digit is
        // how the third rung is reached.
        (
            Addressed::Of {
                item: Some(item),
                nth: through,
                control,
            },
            Press::Digit(nth),
        ) if control.reached_by(Grammar::Digit) => {
            match built
                .beneath(control)
                .nth(nth, drawn(view, built, &[item, through]))
            {
                Some(_) => {
                    address_into(view, bay.name, nth);
                    Asked::Moved
                }
                None => Asked::Nothing(
                    "this does not draw that many controls — the digits count what is drawn, \
                     from one",
                ),
            }
        }
        // **A digit below one of the Master's chain slots**, which is a rung
        // and not a card: a slot's controls are its parameter rows, its cut
        // chip and its `−`, and every one of them is counted whenever the slot
        // is drawn.
        (
            Addressed::Of {
                item: None,
                nth: through,
                control: Control::Slot,
            },
            Press::Digit(nth),
        ) => match built
            .beneath(Control::Slot)
            .nth(nth, drawn(view, built, &[through]))
        {
            Some(_) => {
                address_into(view, bay.name, nth);
                Asked::Moved
            }
            None => Asked::Nothing(SLOT_SHORT),
        },
        // **A digit below the control a card hangs from**: the Sequencer's
        // `+ lane` on the head, and the Transport's two pills and the Master's
        // `+ add` on a headless row. A digit names the nth row of the card
        // exactly as it names the nth of anything else drawn.
        (Addressed::OfHead(through, control), Press::Digit(nth)) => {
            name_row(view, bay.name, built, control, &[HEAD, through], nth)
        }
        (
            Addressed::Of {
                item: None,
                nth: through,
                control,
            },
            Press::Digit(nth),
        ) => name_row(view, bay.name, built, control, &[through], nth),
        (
            Addressed::Of { .. }
            | Addressed::Under { .. }
            | Addressed::UnderHead { .. }
            | Addressed::InCard { .. },
            Press::Digit(_),
        ) => Asked::Nothing(NOTHING_BELOW),

        // ------------------------------------------------------------------
        // The arrows — the neighbour, or the next value
        // ------------------------------------------------------------------
        (Addressed::Bay | Addressed::Item(_), Press::Arrow(arrow)) => {
            match arrow.across() == built.across {
                true => walk(view, panel, bay.name, built, arrow, items, &at),
                false => Asked::Nothing(match built.across {
                    true => "this bay's items are a row, so left and right walk them",
                    false => "this bay's items are a column, so up and down walk them",
                }),
            }
        }
        // **A chain parameter row is a level**, stepped a tenth of the range
        // its procedure declares.
        (
            Addressed::InCard {
                through,
                nth,
                control: Control::ChainParam,
            },
            Press::Arrow(arrow),
        ) => match arrow {
            Arrow::Up => chain_param(view, through, nth, Step::Up),
            Arrow::Down => chain_param(view, through, nth, Step::Down),
            _ => Asked::Nothing("a level is stepped up and down, not across"),
        },
        (
            Addressed::InCard {
                control: Control::ChainCut | Control::ChainRemove,
                ..
            },
            Press::Arrow(_),
        ) => Asked::Nothing(
            "this control's values are a closed list and a list has no axis — space cycles it",
        ),
        // **A card's rows are walked from the row it remembers**, which is the
        // bay-level rule one rung down: the arrows work with no digit pressed
        // first, and the address follows them into the card. Which card it is,
        // is the control the address descended through.
        (Addressed::OfHead(through, control), Press::Arrow(arrow))
            if card_of(control).is_some() =>
        {
            walk_card(
                view,
                bay.name,
                built,
                control,
                &[HEAD, through],
                None,
                arrow,
            )
        }
        (Addressed::UnderHead { through, nth, .. }, Press::Arrow(arrow)) => {
            match built.head.get(through.wrapping_sub(1)).copied() {
                Some(above) if card_of(above).is_some() => walk_card(
                    view,
                    bay.name,
                    built,
                    above,
                    &[HEAD, through],
                    Some(nth),
                    arrow,
                ),
                _ => Asked::Nothing(
                    "this control's values are a closed list and a list has no axis — space \
                     cycles it",
                ),
            }
        }
        (
            Addressed::Of {
                item: None,
                nth: through,
                control,
            },
            Press::Arrow(arrow),
        ) if card_of(control).is_some() => {
            walk_card(view, bay.name, built, control, &[through], None, arrow)
        }
        (Addressed::InCard { through, nth, .. }, Press::Arrow(arrow)) => {
            match built.item().nth(through, items) {
                Some(above) => {
                    walk_card(view, bay.name, built, above, &[through], Some(nth), arrow)
                }
                None => Asked::Nothing("this control is not drawn"),
            }
        }
        (Addressed::OfHead(_, control), Press::Arrow(arrow)) => stepped(view, None, control, arrow),
        (
            Addressed::Of {
                item,
                nth: _,
                control,
            },
            Press::Arrow(arrow),
        ) => stepped(view, item, control, arrow),
        (
            Addressed::Under {
                item,
                through,
                nth,
                control,
            },
            Press::Arrow(arrow),
        ) => under_arrow(view, item, through, nth, control, arrow),
        // **A head is not a row of things laid out on an axis.** Its controls
        // are named by digit and acted on by `space`; an arrow here would have
        // to mean *the next control*, which is a second meaning for the key
        // that walks a bay's items.
        (Addressed::Head, Press::Arrow(_)) => Asked::Nothing(
            "a head's controls are named by a digit rather than walked — press 1 for the first \
             of them",
        ),

        // ------------------------------------------------------------------
        // `space` — the addressed thing's next state
        // ------------------------------------------------------------------
        (Addressed::Head, Press::Space) => Asked::Nothing(
            "a head is not a control — press a digit to name one of the controls in it",
        ),
        (Addressed::Item(_), Press::Space) => Asked::Nothing(
            "this item has no state of its own — press a digit to name one of the controls in \
             it, where it draws any",
        ),
        (Addressed::OfHead(nth, control), Press::Space) => {
            cycled(view, panel, None, nth, control, held)
        }
        (Addressed::Of { item, nth, control }, Press::Space) => {
            cycled(view, panel, item, nth, control, held)
        }
        // **A chain slot's two settings**: the cut chip cycles, and a
        // parameter row goes back to what its procedure declared it at.
        (
            Addressed::InCard {
                through,
                control: Control::ChainCut,
                ..
            },
            Press::Space,
        ) => chain_cut(view, through),
        (
            Addressed::InCard {
                through,
                nth,
                control: Control::ChainParam,
            },
            Press::Space,
        ) => chain_param(view, through, nth, Step::Default),
        // **A card's rows perform rather than set**, so `space` names the
        // sentence the row carries rather than a next state.
        (Addressed::InCard { control, .. }, Press::Space) => match control.answers() {
            Answers::Nothing(why) => Asked::Nothing(why),
            _ => Asked::Nothing(
                "this row performs rather than sets, so it has no next state — enter runs it",
            ),
        },
        (
            Addressed::Under {
                item,
                through,
                nth,
                control,
            },
            Press::Space,
        ) => under_space(view, item, through, nth, control),
        // **An entry of the chooser performs rather than sets**, so there is
        // no next state for `space` to name.
        (Addressed::UnderHead { .. }, Press::Space) => Asked::Nothing(
            "this control performs rather than sets, so it has no next state — enter runs it",
        ),
        (Addressed::Bay, Press::Space) => fold(panel, bay),

        // ------------------------------------------------------------------
        // `enter` — the act the addressed thing is for
        // ------------------------------------------------------------------
        (Addressed::Item(item), Press::Enter) => match built.act {
            Some(act) => performed(view, Some(item), act),
            None => Asked::Nothing(
                "this item performs nothing — enter is the act a control is for, and this bay's \
                 items are things you set rather than things you run",
            ),
        },
        // **`enter` on the control a card hangs from puts the card down**,
        // which is what makes the card's rows a rung the address descends
        // into.
        (Addressed::OfHead(_, control), Press::Enter) if card_of(control).is_some() => {
            open_card(view, control)
        }
        // **And `enter` on one of the rows performs that row and takes the
        // card away**, which is the pointer's own pair of moves in the
        // pointer's own order.
        (
            Addressed::UnderHead {
                through,
                nth,
                control,
            },
            Press::Enter,
        ) => match built.head.get(through.wrapping_sub(1)).copied() {
            Some(above) if card_of(above).is_some() => {
                card_enter(view, bay.name, built, above, control, nth)
            }
            _ => Asked::Nothing(
                "nothing here performs — enter is the act the addressed control is for, \
                     and this one sets rather than performs",
            ),
        },
        (Addressed::OfHead(_, control), Press::Enter) => act_of(view, None, control),
        // **`enter` on a slot's `−` takes that slot out of the chain**, which
        // is the one act on a rung that is not a card.
        (
            Addressed::InCard {
                through,
                control: Control::ChainRemove,
                ..
            },
            Press::Enter,
        ) => chain_remove(view, through),
        (
            Addressed::Of {
                item: None,
                control,
                ..
            },
            Press::Enter,
        ) if card_of(control).is_some() => open_card(view, control),
        (Addressed::Of { item, control, .. }, Press::Enter) => act_of(view, item, control),
        (
            Addressed::InCard {
                through,
                nth,
                control,
            },
            Press::Enter,
        ) => match built.item().nth(through, items) {
            Some(above) => card_enter(view, bay.name, built, above, control, nth),
            None => Asked::Nothing("this control is not drawn"),
        },
        (
            Addressed::Under {
                item,
                through,
                nth,
                control,
            },
            Press::Enter,
        ) => under_enter(view, item, through, nth, control),
        (Addressed::Bay | Addressed::Head, Press::Enter) => Asked::Nothing(
            "nothing here performs — enter is the act the addressed control is for, and a bay \
             is not one",
        ),
    }
}

/// One of the four cards the address descends into, and the whole of what one
/// card does not share with the next.
///
/// A card is a rung hanging off a control: `enter` on the control puts the card
/// down, a digit names the nth row of it, `↑↓` walk the rows from the one the
/// card remembers — clamped, never wrapped — `←→` are refused, `enter` on a row
/// performs that row, and `esc` or `Tab` takes the card away and leaves the
/// address on the control it hangs from. That is written once, in
/// [`open_card`], [`name_row`], [`walk_card`] and [`card_enter`]; a row of
/// [`CARDS`] is what those four read.
///
/// The four are the audio-in pill's inputs and the arrangement pill's menu
/// ([ADR-0350](../../../docs/adr/0350-the-transports-two-cards-are-walked-and-the-tempo-figure-steps-by-a-beat-a-minute.md)),
/// the Sequencer's `+ lane` chooser
/// ([ADR-0351](../../../docs/adr/0351-the-lane-chooser-is-a-rung-of-the-address.md))
/// and the Master's `+ add` chooser
/// ([ADR-0352](../../../docs/adr/0352-the-chains-list-is-the-master-bays-items-and-a-slot-is-taken-out-by-a-glyph-on-its-row.md)).
struct Card {
    /// The control the card hangs from, and what [`card_of`] finds it by.
    control: Control,
    /// Whether the card is down.
    down: fn(&View) -> bool,
    /// How many rows it is drawing, which is zero while it is up.
    rows: fn(&View) -> usize,
    /// How `enter` on the control puts it down, and what takes it away.
    puts: Puts,
    /// What a press addressed below the control says while the card is up: the
    /// rung is not drawn, and the sentence names the press that draws it.
    shut: &'static str,
    /// What `enter` on the control says with the card already down. It names the
    /// keys that reach the rows.
    standing: &'static str,
    /// What a digit that named a row the card is not drawing says.
    short: &'static str,
    /// What an arrow says on a card that is down with nothing on it to walk.
    empty: &'static str,
    /// What `←→` say, naming the pair that walks the rows.
    column: &'static str,
    /// What an arrow says where the address is already at the end.
    end: &'static str,
    /// A sentence this card's own state carries in place of [`Card::short`] and
    /// [`Card::empty`], or `None` for a card with no such state. The arrangement
    /// menu's is the one: while it is asking for a name it is a field rather than
    /// a list, and it draws no rows.
    busy: Option<fn(&View) -> Option<&'static str>>,
}

/// How a card is put down and taken away.
#[derive(Clone, Copy)]
enum Puts {
    /// This console's own state: the method that puts the card down, the method
    /// that takes it away, and the sentence for a card with nothing to offer,
    /// which is what refuses to put it down.
    Console {
        put: fn(&mut View) -> bool,
        shut: fn(&mut View) -> bool,
        nothing: &'static str,
    },
    /// The host's: `enter` leaves in the control's own word for it and the host
    /// puts the card down, through the method a pointer press already reaches
    /// (ADR-0156). `shut` is still this console's, because `esc` and `Tab` take
    /// the card away without asking the host for anything.
    Host {
        asked: fn() -> Asked,
        shut: fn(&mut View),
    },
}

impl Card {
    /// What a digit that named a row this card is not drawing says: the card is
    /// up, its own state is in the way, or it draws fewer rows than that.
    fn named(&self, view: &View) -> &'static str {
        match (self.down)(view) {
            false => self.shut,
            true => self.asking(view).unwrap_or(self.short),
        }
    }

    /// What an arrow says on a card that is down and drawing nothing.
    fn bare(&self, view: &View) -> &'static str {
        self.asking(view).unwrap_or(self.empty)
    }

    /// The sentence this card's own state carries in place of a count's, or
    /// `None` where nothing is in the way.
    fn asking(&self, view: &View) -> Option<&'static str> {
        self.busy.and_then(|state| state(view))
    }

    /// Take this card away. A card that is already up is left as it is.
    fn shut(&self, view: &mut View) {
        match self.puts {
            Puts::Console { shut, .. } => {
                shut(view);
            }
            Puts::Host { shut, .. } => shut(view),
        }
    }
}

/// The sentence a press addressed to a Transport card's rung carries while the
/// card is up: the rung is not drawn, and `enter` on the pill is the press that
/// draws it.
const CARD_SHUT: &str = "this card is not down, so there is nothing here to name — press enter on \
                         this control to put it down";

/// The sentence a digit carries when a Transport card is drawing fewer rows than
/// that.
const CARD_SHORT: &str =
    "this card is not drawing a row with that number — the digits count what is on it, from one";

/// The sentence `enter` on a Transport pill carries with its card already down.
const CARD_STANDING: &str = "this card is already down — press a digit to name a row, or walk it \
                             with up and down, and enter runs the row the address is on";

/// The sentence the arrangement menu carries while it is a field rather than a
/// list. The keyboard is the name's while it is asking for one, which is the one
/// flow on this panel that takes letters (ADR-0259).
const NAMING: &str = "this menu is asking for a name — type it and press return, or press esc to \
                      leave the arrangement unsaved";

/// The sentence a digit carries when the `+ add` chooser is not offering that
/// many.
const CHAIN_CARD_SHORT: &str = "the chooser is not offering a procedure with that number — the \
                                digits count what is on the card, from one";

/// The sentence an arrow carries on a chooser that is down with nothing on it.
const NOTHING_TO_WALK: &str = "the chooser is offering nothing to walk";

/// The four cards, one row each.
const CARDS: &[Card] = &[
    Card {
        control: Control::Audio,
        down: inputs_down,
        rows: inputs_rows,
        puts: Puts::Host {
            asked: open_inputs,
            shut: shut_inputs,
        },
        shut: CARD_SHUT,
        standing: CARD_STANDING,
        short: CARD_SHORT,
        empty: CARD_SHORT,
        column: "a card's rows are a column, so up and down walk them",
        end: "the address is already at the end of this card",
        busy: None,
    },
    Card {
        control: Control::Arrangement,
        down: menu_down,
        rows: menu_rows,
        puts: Puts::Host {
            asked: open_menu,
            shut: shut_menu,
        },
        shut: CARD_SHUT,
        standing: CARD_STANDING,
        short: CARD_SHORT,
        empty: CARD_SHORT,
        column: "a card's rows are a column, so up and down walk them",
        end: "the address is already at the end of this card",
        busy: Some(menu_asking),
    },
    Card {
        control: Control::AddLane,
        down: View::lane_open,
        rows: lane_rows,
        puts: Puts::Console {
            put: View::open_lane,
            shut: View::shut_lane,
            nothing: "there is nothing for a lane to drive — this console draws no mixer strip \
                      and the inspector is showing no published control",
        },
        shut: "the chooser is not down, so there is nothing here to name — press enter on + lane \
               to put it down",
        standing: "the chooser is already down — press a digit to name a target, or walk it with \
                   up and down, and enter points the lane at the one the address is on",
        short: "the chooser is not offering that many targets — the digits count what is on the \
                card, from one",
        empty: NOTHING_TO_WALK,
        column: "the chooser's targets are a column, so up and down walk them",
        end: "the address is already at the end of the chooser",
        busy: None,
    },
    Card {
        control: Control::AddEffect,
        down: View::chain_add_open,
        rows: chain_rows,
        puts: Puts::Console {
            put: View::open_chain_add,
            shut: View::shut_chain_add,
            nothing: "this library is listing no kind L5 procedure, so there is nothing to add \
                      — the master chain holds kind L5 procedures",
        },
        shut: "the chooser is not down, so there is nothing here to name — press enter on + add \
               to put it down",
        standing: "this chooser is already down — press a digit to name a procedure, or walk it \
                   with up and down, and enter adds the one the address is on",
        short: CHAIN_CARD_SHORT,
        empty: NOTHING_TO_WALK,
        column: "the chooser's procedures are a column, so up and down walk them",
        end: "the address is already at the end of the chooser",
        busy: None,
    },
];

/// Whether the audio-in pill's card is down.
fn inputs_down(view: &View) -> bool {
    view.audio.as_ref().is_some_and(crate::view::AudioIn::open)
}

/// How many inputs that card is listing.
fn inputs_rows(view: &View) -> usize {
    view.audio.as_ref().map_or(0, crate::view::AudioIn::rows)
}

/// The audio-in card, in the pill's own word for putting it down.
fn open_inputs() -> Asked {
    Asked::Listened(AudioAsk::Open)
}

/// Take the audio-in card away.
fn shut_inputs(view: &mut View) {
    if let Some(audio) = view.audio.as_mut() {
        audio.shut();
    }
}

/// Whether the arrangement pill's menu is down, a name being asked for included.
fn menu_down(view: &View) -> bool {
    view.arrangement.open()
}

/// How many rows that menu is drawing.
fn menu_rows(view: &View) -> usize {
    view.arrangement.rows()
}

/// The menu asking for a name, which is the state that draws no rows.
fn menu_asking(view: &View) -> Option<&'static str> {
    view.arrangement.naming().map(|_| NAMING)
}

/// The arrangement menu, in the pill's own word for putting it down.
fn open_menu() -> Asked {
    Asked::Arranged(Ask::Open)
}

/// Take the arrangement menu away, typed name and all.
fn shut_menu(view: &mut View) {
    view.arrangement.shut();
}

/// How many targets the `+ lane` chooser is offering, and none while it is up.
fn lane_rows(view: &View) -> usize {
    match view.lane_open() {
        true => view.lane_choices().items.len(),
        false => 0,
    }
}

/// How many procedures the `+ add` chooser is offering, and none while it is up.
fn chain_rows(view: &View) -> usize {
    match view.chain_add_open() {
        true => view.chain_choices().items.len(),
        false => 0,
    }
}

/// The card `control` hangs, or `None` for a control that hangs none — the one
/// reading the digit, the arrows and `enter` all ask.
fn card_of(control: Control) -> Option<&'static Card> {
    CARDS.iter().find(|card| card.control == control)
}

/// How many rows the card `control` hangs is drawing, and zero for a control that
/// hangs none — the reading [`drawn`] takes at a card's rung.
fn card_rows(view: &View, control: Control) -> usize {
    card_of(control).map_or(0, |card| (card.rows)(view))
}

/// The card on the focused bay's address, and whether the address has descended
/// into its rows — or `None` where the address is on no control that hangs a
/// card, or where the card that control hangs is up.
///
/// The reading `esc` takes. A card on the address's path is a level of the
/// address; a card that is down anywhere else is not, and `esc` leaves it alone
/// ([ADR-0332](../../../docs/adr/0332-focus-is-a-pointer-the-console-owns-and-the-three-pointers-are-instances-of-it.md)).
///
/// `panel` must be solved: the focused bay is read off the arrangement.
pub fn card_on_path(view: &View, panel: &Panel) -> Option<(Control, bool)> {
    let bay = view.focused(panel)?;
    let built = built(bay.name)?;
    let at = view.focus().address(bay.name).map(Address::at)?;
    // A card hangs off a head control or off a headless row's own control, so
    // the control is the second step of the path or the first, and the address
    // is inside the card where there is one step after it.
    let (control, inside) = match *at {
        [HEAD, through] => (head_control(built, through)?, false),
        [HEAD, through, _] => (head_control(built, through)?, true),
        [through] if matches!(built.items, Items::Controls(_)) => {
            (row_control(view, built, through)?, false)
        }
        [through, _] if matches!(built.items, Items::Controls(_)) => {
            (row_control(view, built, through)?, true)
        }
        _ => return None,
    };
    let card = card_of(control)?;
    (card.down)(view).then_some((control, inside))
}

/// The `through`th control of `built`'s head, counting from one.
fn head_control(built: &Built, through: usize) -> Option<Control> {
    built.head.get(through.checked_sub(1)?).copied()
}

/// The `through`th control of a headless row, counting from one.
fn row_control(view: &View, built: &Built, through: usize) -> Option<Control> {
    built.item().nth(through, drawn(view, built, &[]))
}

/// Take the card `control` hangs away, and do nothing for a control that hangs
/// none. A card that is already up is left as it is.
pub fn shut_card(view: &mut View, control: Control) {
    if let Some(card) = card_of(control) {
        card.shut(view);
    }
}

/// Take every card away, wherever the address is — the reading `Tab` takes,
/// because a move of focus is the address leaving every card it could be in.
pub fn shut_cards(view: &mut View) {
    for card in CARDS {
        card.shut(view);
    }
}

/// `enter` on the control a card hangs from: the card, put down.
///
/// Declines where the card is already down, and where it has nothing to offer —
/// a card with no rows offers nothing to pick and nothing to leave by, which is
/// [`crate::view::View::open_lane`]'s own rule. The address stays on the control;
/// a digit or an arrow is what descends into the rows.
fn open_card(view: &mut View, control: Control) -> Asked {
    let Some(card) = card_of(control) else {
        return Asked::Nothing("this control puts no card down");
    };
    if (card.down)(view) {
        return Asked::Nothing(card.standing);
    }
    match card.puts {
        Puts::Console { put, nothing, .. } => match put(view) {
            true => Asked::Moved,
            false => Asked::Nothing(nothing),
        },
        Puts::Host { asked, .. } => asked(),
    }
}

/// A digit below the control a card hangs from: the nth row of the card, with the
/// address descended onto it.
///
/// `under` is the path the card's rung is at — `[HEAD, through]` for a card on a
/// head control and `[through]` for one on a headless row's own control. The
/// digits count what the card drew, from one, and a digit past that declines with
/// the card's own sentence.
fn name_row(
    view: &mut View,
    bay: &'static str,
    built: &Built,
    control: Control,
    under: &[usize],
    nth: usize,
) -> Asked {
    let Some(card) = card_of(control) else {
        return Asked::Nothing(NOTHING_BELOW);
    };
    let rows = drawn(view, built, under);
    match built.beneath(control).nth(nth, rows) {
        Some(_) => {
            address_into(view, bay, nth);
            Asked::Moved
        }
        None => Asked::Nothing(card.named(view)),
    }
}

/// An arrow on the control a card hangs from, or on one of the card's rows: the
/// neighbouring row.
///
/// `from` is the row the address is on, or `None` where it is still on the
/// control — and then the walk starts from the row the card remembers, which is
/// [`walk`]'s rule at bay level one rung down: the arrows work with no digit
/// pressed first. `under` is the path the card's rung is at, as [`name_row`]
/// takes it.
///
/// A card is a column of rows, so `←→` are refused and the refusal names the pair
/// that works. The walk is clamped at both ends and never wraps.
fn walk_card(
    view: &mut View,
    bay: &'static str,
    built: &Built,
    control: Control,
    under: &[usize],
    from: Option<usize>,
    arrow: Arrow,
) -> Asked {
    let Some(card) = card_of(control) else {
        return Asked::Nothing("this control puts no card down");
    };
    if !(card.down)(view) {
        return Asked::Nothing(card.shut);
    }
    if arrow.across() {
        return Asked::Nothing(card.column);
    }
    let rows = drawn(view, built, under);
    if rows == 0 {
        return Asked::Nothing(card.bare(view));
    }
    let at = from.unwrap_or_else(|| {
        view.focus()
            .address(bay)
            .and_then(|address| address.remembered(under))
            .unwrap_or(1)
            .clamp(1, rows)
    });
    let to = (at as i64 + i64::from(arrow.step())).clamp(1, rows as i64) as usize;
    let address = view.focus_mut().address_mut(bay);
    // A walk moves along a level and never into one, so an address that had
    // already descended has its last step replaced rather than pushed.
    match from.is_some() {
        true => {
            address.to_row(to);
        }
        false => address.down(to),
    }
    match from.is_none() || to != at {
        true => Asked::Moved,
        false => Asked::Nothing(card.end),
    }
}

/// `enter` on one of a card's rows: what that row performs, with the card taken
/// away and the address back on the control it hangs from.
///
/// `above` is that control and `row` is the control the addressed row is. A row
/// that declines performs nothing, so the card stays down and the address stays
/// on the row.
///
/// A card this console owns is taken away here, before the answer leaves; a card
/// the host owns leaves in the answer, and the host takes it away through the
/// method a pointer press on that row already reaches (ADR-0333). `above` is
/// allowed to hang no card — a slot of the master chain is a rung and not one —
/// and then the row answers for itself and nothing is taken away.
fn card_enter(
    view: &mut View,
    bay: &'static str,
    built: &Built,
    above: Control,
    row: Control,
    nth: usize,
) -> Asked {
    // The rows a card draws before the counted ones are its own verbs, so the
    // nth thing it lists is that many further down the rung.
    let verbs = built.beneath(above).first.len();
    let asked = row_act(view, row, nth, verbs);
    if matches!(asked, Asked::Nothing(_)) {
        return asked;
    }
    if let Some(Puts::Console { shut, .. }) = card_of(above).map(|card| card.puts) {
        shut(view);
    }
    view.focus_mut().address_mut(bay).up();
    asked
}

/// What `enter` on one row of a card asks for — the one thing the four cards do
/// not share.
///
/// `nth` counts the rung from one and `verbs` is how many rows the card draws
/// before the ones it lists, so the nth listed thing is at `nth - verbs - 1`. A
/// control that is not a card's row answers with its own sentence.
fn row_act(view: &View, row: Control, nth: usize, verbs: usize) -> Asked {
    match row {
        Control::Input => match view
            .audio
            .as_ref()
            .and_then(|audio| audio.inputs.get(nth.wrapping_sub(verbs + 1)))
        {
            Some(name) => Asked::Listened(AudioAsk::Operation(Operation::AttachBeatSource {
                source: BeatSource::AudioInput(name.clone()),
            })),
            None => Asked::Nothing("this card is not drawing an input with that number"),
        },
        // **Saving again means the name in use**, and with none in use the
        // menu asks for one: the field takes the keyboard whole while it is
        // asking, which is where the name is typed and where `return` files it
        // (ADR-0259).
        Control::Save => Asked::Arranged(match view.arrangement.name.clone() {
            Some(name) => Ask::Operation(Operation::SaveArrangement { name }),
            None => Ask::Name,
        }),
        Control::Filed => match view.arrangement.filed.get(nth.wrapping_sub(verbs + 1)) {
            Some(name) => Asked::Arranged(Ask::Operation(Operation::RestoreArrangement {
                name: name.clone(),
            })),
            None => Asked::Nothing("this menu is not drawing a name with that number"),
        },
        // **The bank is this bay's own reading** rather than whichever pattern
        // is armed by the time the operation is performed, which is every other
        // arm of this bay's.
        Control::LaneTarget => {
            let Some(bank) = view.sequencer.as_ref().map(|seq| seq.bank as u8) else {
                return Asked::Nothing("this console has no pattern behind it");
            };
            let choices = view.lane_choices();
            match choices.items.get(nth.wrapping_sub(verbs + 1)) {
                Some(choice) => Asked::Emitted(Operation::PointLane {
                    pattern: bank,
                    target: choice.target.clone(),
                }),
                None => Asked::Nothing(
                    "the chooser is not offering a target with that number — the digits count \
                     what is on the card, from one",
                ),
            }
        }
        Control::ChainProcedure => {
            let choices = view.chain_choices();
            match choices.items.get(nth.wrapping_sub(verbs + 1)) {
                Some(choice) => Asked::Emitted(choice.operation()),
                None => Asked::Nothing(CHAIN_CARD_SHORT),
            }
        }
        other => match other.answers() {
            Answers::Nothing(why) => Asked::Nothing(why),
            _ => {
                Asked::Nothing("nothing here performs — enter is the act the addressed row is for")
            }
        },
    }
}

/// The sentence a digit carries below a control nothing is drawn under.
const NOTHING_BELOW: &str = "nothing below this control is drawn, so a digit here reaches \
                             nothing — esc goes back up";

/// `space` on a bay: the fold.
///
/// The one press that means the same thing in all nine, which is what a badge
/// reading `space &middot; in any bay` says and what ADR-0333 declined to bind
/// while it meant it in two. A folded bay answers it and nothing else, which is
/// the narrow reason a folded bay is in the ring at all: it is there so that
/// there is something to press to open it, not so that it can be operated.
fn fold(panel: &Panel, bay: &'static Region) -> Asked {
    let Some(id) = panel.layout().find(bay.name) else {
        return Asked::Nothing("this arrangement has no node for the focused bay");
    };
    Asked::Panel(match panel.layout().is_collapsed(id) {
        true => Op::Unfold(id),
        false => Op::Fold(id),
    })
}

/// How many things `bay` draws under `path` — the one reading in this module
/// that is a bay's own rather than the grammar's, and it is a reading rather
/// than a rule.
///
/// `[]` is the bay's items, `[n]` the nth item's controls, `[n, c]` what is
/// under that control. A bay drawing nothing answers zero, and the press then
/// declines with the bay's own sentence rather than acting on a rectangle that
/// is not there.
fn drawn(view: &View, built: &Built, path: &[usize]) -> usize {
    match (built.bay, path) {
        (_, []) => match built.items {
            // A headless row's items are its controls, and how many of them
            // are drawn is the bay's own count.
            Items::Controls(of) => match built.bay {
                // **The out fader, one per slot of the chain, and `+ add`** —
                // the bay's own reading, because how many slots there are is
                // what an operator put in the chain. A console with no chain
                // behind it draws the out row and nothing under it.
                MASTER => match view.master_out.is_some() {
                    true => {
                        1 + view
                            .master_chain
                            .as_ref()
                            .map_or(0, |chain| chain.slots.len() + 1)
                    }
                    false => 0,
                },
                OUTPUTS => 1,
                _ => of.len(0),
            },
            Items::Alike(_) => match built.bay {
                MIXER => view.mixer.len(),
                LIBRARY => view.library.len(),
                STAGING => view.staging.len(),
                INSPECTOR => view.inspector.len(),
                SEQUENCER => lanes(view),
                _ => 0,
            },
        },
        // A pane draws one deck head and a group per node.
        (INSPECTOR, [pane]) => {
            1 + view
                .inspector
                .get(pane.wrapping_sub(1))
                .map_or(0, |pane| pane.nodes.len())
        }
        // A node group draws its authority chip, its renderer chips and a row
        // per parameter; the deck head draws its three.
        (INSPECTOR, [pane, through]) => match through {
            1 => 3,
            _ => view
                .inspector
                .get(pane.wrapping_sub(1))
                .and_then(|pane| pane.nodes.get(through.wrapping_sub(2)))
                .map_or(0, |node| 2 + node.params.len()),
        },
        // A lane draws its label and a cell per step of the mode.
        (SEQUENCER, [lane]) if *lane <= lanes(view) => 1 + steps(view),
        // **A card hanging off a head control**, which the Sequencer's
        // `+ lane` chooser is the one of: the card draws one row per thing it
        // offers, and none at all while it is up. A rung that is not on the
        // panel draws nothing, and the press then declines with the sentence
        // that says how to put it down.
        (_, [HEAD, through]) => match built.head.get(through.wrapping_sub(1)).copied() {
            Some(control) => card_rows(view, control),
            None => 0,
        },
        // **The Master's two rungs**: a slot draws its parameter rows, its cut
        // chip and its `−`, and `+ add` draws one row per `kind L5` procedure
        // the library holds.
        //
        // **The cut chip is counted on every slot**, drawn or not, so that a
        // digit names the same control on every slot of the chain; a slot
        // whose procedure declares no `retains` draws none and the chip
        // declines (ADR-0352).
        (MASTER, [through]) => match built.item().nth(*through, drawn(view, built, &[])) {
            Some(Control::Slot) => match chain_slot(view, *through) {
                Some(slot) => slot.params.len() + 2,
                None => 0,
            },
            Some(control) => card_rows(view, control),
            None => 0,
        },
        // **The Transport's two cards**: the audio-in card draws one row per
        // input the machine answered with, and the arrangement menu draws
        // *save*, *start a new one* and one row per name filed.
        (TRANSPORT, [through]) => match built.item().nth(*through, built.item().first.len()) {
            Some(control) => card_rows(view, control),
            None => 0,
        },
        (_, [_]) => built.item().first.len(),
        _ => 0,
    }
}

/// The slot of the running chain the Master bay's `through`th item is, or `None`
/// where that item is not a slot or the chain has not got one there.
///
/// The bay's items are the out fader, then one per slot, then `+ add`, so the
/// slot's position in the chain — which is the address every chain operation
/// takes — is two less than the item's number.
fn chain_slot(view: &View, through: usize) -> Option<&crate::view::ChainSlot> {
    view.master_chain
        .as_ref()?
        .slots
        .get(through.checked_sub(2)?)
}

/// The position in the chain the Master bay's `through`th item names.
fn chain_at(through: usize) -> Option<u32> {
    u32::try_from(through.checked_sub(2)?).ok()
}

/// The sentence a digit carries when a slot does not draw that many controls.
const SLOT_SHORT: &str = "this slot does not draw that many controls — the digits count what is \
                          drawn, from one";

/// One press of an arrow on a chain parameter row, or `space` on one: a tenth of
/// the range the slot's procedure declares, or the value it declared.
///
/// [`param`]'s rule one bay along — *"a tenth of the published range"* — read on
/// a range that is declared in a `.kir` rather than published by a Set
/// ([ADR-0343](../../../docs/adr/0343-the-grammar-reaches-all-nine-bays-and-space-on-a-bay-is-the-fold.md)).
fn chain_param(view: &View, through: usize, nth: usize, step: Step) -> Asked {
    let (Some(at), Some(slot)) = (chain_at(through), chain_slot(view, through)) else {
        return Asked::Nothing("this slot is not drawn");
    };
    let Some(param) = slot.params.get(nth.wrapping_sub(1)) else {
        return Asked::Nothing("this parameter row is not drawn");
    };
    let [low, high] = param.range;
    let value = match step {
        Step::Default => param.default,
        Step::Up => param.value + (high - low) * PARAM_STEP,
        Step::Down => param.value - (high - low) * PARAM_STEP,
    };
    Asked::Emitted(Operation::SetChainParam {
        at,
        param: ParamOfChain::Declared {
            key: param.key.clone(),
            value: value.clamp(low, high),
        },
    })
}

/// `space` on a slot's cut chip: the other of the two cuts.
fn chain_cut(view: &View, through: usize) -> Asked {
    let (Some(at), Some(slot)) = (chain_at(through), chain_slot(view, through)) else {
        return Asked::Nothing("this slot is not drawn");
    };
    let Some(cut) = slot.cut else {
        return Asked::Nothing(
            "this slot's procedure declares no retains, so it reads no retained frame and has \
             no cut to set",
        );
    };
    let all = karakuri_operation::Cut::ALL;
    let showing = all.iter().position(|c| *c == cut).unwrap_or(0);
    Asked::Emitted(Operation::SetChainParam {
        at,
        param: ParamOfChain::Cut(all[(showing + 1) % all.len()]),
    })
}

/// `enter` on a slot's `−`: the slot, taken out of the chain.
fn chain_remove(view: &View, through: usize) -> Asked {
    let (Some(at), Some(_)) = (chain_at(through), chain_slot(view, through)) else {
        return Asked::Nothing("this slot is not drawn");
    };
    Asked::Emitted(Operation::RemoveChainEffect { at })
}

/// How many lanes the Sequencer is drawing.
fn lanes(view: &View) -> usize {
    view.sequencer
        .as_ref()
        .map_or(0, |seq| seq.pattern.lanes().len())
}

/// How many cells a lane draws, which is the pattern's mode and not a constant:
/// sixteen at a sixteenth and eight at an eighth (ADR-0306).
fn steps(view: &View) -> usize {
    view.sequencer
        .as_ref()
        .map_or(0, |seq| seq.pattern.mode().count())
}

/// Push `nth` onto the focused bay's address.
fn address_into(view: &mut View, bay: &'static str, nth: usize) {
    view.focus_mut().address_mut(bay).down(nth);
}

/// A digit at bay level, naming the nth item — and the deck selection where
/// naming one is that.
fn name_item(view: &mut View, bay: &'static str, built: &Built, nth: usize, items: usize) -> Asked {
    if nth > items {
        return Asked::Nothing(match built.selects {
            true => {
                "this deck has no strip with that number — the digits count the strips the \
                     mixer drew, from one"
            }
            false => {
                "this bay is not drawing that many things — the digits count what is drawn, \
                      from one"
            }
        });
    }
    match (built.selects, built.bay) {
        (true, _) => {
            view.select((nth - 1) as u8);
        }
        (false, LIBRARY) => {
            view.point_at(nth - 1);
        }
        _ => {}
    }
    // **Refused rather than clamped, and the address does not descend on a
    // refusal** — the guard above is that refusal, and it is the bay's own
    // count rather than a second rule.
    address_into(view, bay, nth);
    match built.selects {
        true => Asked::Emitted(Operation::SelectDeck {
            deck: (nth - 1) as u8,
        }),
        false => Asked::Moved,
    }
}

/// An arrow at bay or item level, walking the bay's items.
///
/// From the item the bay remembers, not from the address, which is the one
/// place the two fields of an [`Address`] are read together on purpose: at bay
/// level there is no addressed item, and arrows that declined until a digit had
/// been pressed would take away the two keys the Library bay has today
/// (ADR-0333).
fn walk(
    view: &mut View,
    panel: &Panel,
    bay: &'static str,
    built: &Built,
    arrow: Arrow,
    items: usize,
    at: &[usize],
) -> Asked {
    if items == 0 {
        return Asked::Nothing("this bay is drawing nothing to walk");
    }
    let step = arrow.step();
    let moved = match (built.selects, built.bay) {
        // **The strips are walked and not wrapped**, which is `View::walk`'s
        // rule one bay over: a walk is not a cycle, and a press held down must
        // not jump the length of the row.
        (true, _) => {
            let from = i64::from(view.selection());
            let to = (from + i64::from(step)).clamp(0, items as i64 - 1) as u8;
            view.select(to)
        }
        (false, LIBRARY) => {
            let showing = crate::view::library(
                panel.layout(),
                &view.scopes,
                &view.library,
                view.opened(),
                view.pointed(),
                view.library_scroll(),
            )
            .map_or(0..0, |bay| bay.drawn());
            view.walk(step, showing)
        }
        // **Every other bay walks the address alone**, because the item it is
        // on is not a pointer anything downstream reads: the deck selection
        // and the library cursor are the two that are, and each is a bay's
        // remembered address seen from outside (ADR-0332).
        _ => {
            let from = remembered(view, bay, items);
            let to = (from as i64 + i64::from(step)).clamp(1, items as i64) as usize;
            to != from
        }
    };
    // The address follows the walk where it had descended to an item, and stays
    // at the bay where it had not — and the memory follows it either way, which
    // is what the solid ring is.
    let nth = match (built.selects, built.bay) {
        (true, _) => usize::from(view.selection()) + 1,
        (false, LIBRARY) => view.cursor_row() + 1,
        _ => {
            (remembered(view, bay, items) as i64 + i64::from(step)).clamp(1, items as i64) as usize
        }
    };
    match at.is_empty() {
        true => {
            view.focus_mut().address_mut(bay).remember(&[], nth);
        }
        false => view.focus_mut().address_mut(bay).to_item(nth),
    }
    match (built.selects, moved) {
        // **Emitted whether or not the mark moved**, which is the four deck
        // keys' rule this replaces: what a press asked for is what is emitted.
        (true, _) => Asked::Emitted(Operation::SelectDeck {
            deck: view.selection(),
        }),
        (false, true) => Asked::Moved,
        (false, false) => Asked::Nothing("the cursor is already at the end of what is drawn"),
    }
}

/// Which item a bay is remembering, one-based and inside what it is drawing —
/// the solid ring, read where the walk needs somewhere to start.
fn remembered(view: &View, bay: &str, items: usize) -> usize {
    view.focus()
        .address(bay)
        .and_then(|address| address.remembered(&[]))
        .unwrap_or(1)
        .clamp(1, items.max(1))
}

/// An arrow on a control: the next value of a level, or the neighbouring cell
/// of a row of them.
#[allow(clippy::too_many_arguments)]
fn stepped(view: &View, item: Option<usize>, control: Control, arrow: Arrow) -> Asked {
    match control.answers() {
        // A track is stepped exactly as a level is; what it has not got is a
        // value `space` returns it to.
        Answers::Level | Answers::Track(_) => match arrow {
            Arrow::Up | Arrow::Down => {
                let step = match arrow {
                    Arrow::Up => Step::Up,
                    _ => Step::Down,
                };
                level(view, item, control, step)
            }
            _ => Asked::Nothing("a level is stepped up and down, not across"),
        },
        Answers::Cells { across } => match arrow.across() == across {
            true => Asked::Nothing(
                "the cells of a lane are walked from the cell the address is on — press a digit \
                 to name one first",
            ),
            false => Asked::Nothing("a lane's cells are a row, so left and right walk them"),
        },
        _ => Asked::Nothing(
            "this control's values are a closed list and a list has no axis — space cycles it",
        ),
    }
}

/// An arrow on the third rung: the Inspector's levels, and nothing else.
#[allow(clippy::too_many_arguments)]
fn under_arrow(
    view: &View,
    item: usize,
    through: usize,
    nth: usize,
    control: Control,
    arrow: Arrow,
) -> Asked {
    if !matches!(arrow, Arrow::Up | Arrow::Down) {
        return Asked::Nothing("a level is stepped up and down, not across");
    }
    let up = matches!(arrow, Arrow::Up);
    match control {
        // **A quarter beat either way**, which is the deck head's own arrows.
        Control::Anchor => match view.inspector.get(item.wrapping_sub(1)) {
            Some(pane) => Asked::Emitted(Operation::ScrubDeck {
                deck: pane.deck as u8,
                beats: match up {
                    true => crate::view::SCRUB_BEATS,
                    false => -crate::view::SCRUB_BEATS,
                },
            }),
            None => Asked::Nothing("this pane is not drawn"),
        },
        // **A tenth of the published range**, which is the trim's tenth read on
        // a control whose range is declared rather than fixed — and the value
        // it steps from is the pane's, for `view::ParamGrip`'s reason.
        Control::Param => param(view, item, through, nth, up),
        _ => Asked::Nothing(
            "this control's values are a closed list and a list has no axis — space cycles it",
        ),
    }
}

/// A parameter row, stepped a tenth of what it publishes.
fn param(view: &View, item: usize, through: usize, nth: usize, up: bool) -> Asked {
    let Some(param) = view
        .inspector
        .get(item.wrapping_sub(1))
        .and_then(|pane| pane.nodes.get(through.wrapping_sub(2)))
        .and_then(|node| node.params.get(nth.wrapping_sub(3)))
    else {
        return Asked::Nothing("this parameter row is not drawn");
    };
    let deck = match view.inspector.get(item.wrapping_sub(1)) {
        Some(pane) => pane.deck as u8,
        None => return Asked::Nothing("this pane is not drawn"),
    };
    let to = param.at()
        + match up {
            true => PARAM_STEP,
            false => -PARAM_STEP,
        };
    Asked::Emitted(Operation::WriteParam {
        deck,
        param: param.param.clone(),
        value: ParamValue::Scalar(param.valued(to)),
    })
}

/// One press of a parameter key, as a fraction of what the control publishes —
/// the trim's tenth read on a range that is declared rather than fixed.
/// `Param::valued` clamps it, which is that method's own rule.
const PARAM_STEP: f32 = 0.1;

/// A level the host steps, named here and stepped there.
fn level(view: &View, item: Option<usize>, control: Control, step: Step) -> Asked {
    match control {
        Control::Trim => Asked::Stepped {
            level: Level::Trim((item.unwrap_or(1) - 1) as u8),
            step,
        },
        Control::Fader => Asked::Stepped {
            level: Level::Fader((item.unwrap_or(1) - 1) as u8),
            step,
        },
        Control::Out => match view.master_out.is_some() {
            true => Asked::Stepped {
                level: Level::Out,
                step,
            },
            false => Asked::Nothing("this console has no master out behind it to step"),
        },
        // **The tempo the host steps is the one the grid is running**, read
        // off the oscillator at the press rather than off the figure a frame
        // drew — which is the three mix keys' own rule (ADR-0333). What the
        // step is, is `karakuri/src/bridge/handlers.rs`'s `tempo_key`.
        Control::Tempo => match view.transport.is_some() {
            true => Asked::Stepped {
                level: Level::Tempo,
                step,
            },
            false => {
                Asked::Nothing("this console has no engine behind it, so there is no grid to step")
            }
        },
        Control::Exposure => match view.look.is_some() {
            true => Asked::Stepped {
                level: Level::Exposure,
                step,
            },
            false => Asked::Nothing("this console has no look behind it, so there is no exposure"),
        },
        Control::Offset => match view.tracker.is_some() {
            true => Asked::Stepped {
                level: Level::Offset,
                step,
            },
            false => Asked::Nothing(
                "no audio session is open, so there is no latency offset to nudge — attach an \
                 input on the audio-in pill first",
            ),
        },
        _ => Asked::Nothing("this control is not a level"),
    }
}

/// `space` on a control — the next state, named here because the cycle is this
/// console's affordance (P-0090) and read off the deck by the caller where the
/// reading is not.
#[allow(clippy::too_many_arguments)]
fn cycled(
    view: &View,
    panel: &Panel,
    item: Option<usize>,
    nth: usize,
    control: Control,
    held: impl FnOnce(u8) -> Option<Held>,
) -> Asked {
    match control.answers() {
        Answers::Level => level(view, item, control, Step::Default),
        Answers::Nothing(why) | Answers::Track(why) => Asked::Nothing(why),
        Answers::Act(_) => Asked::Nothing(
            "this control performs rather than sets, so it has no next state — enter runs it",
        ),
        Answers::State | Answers::Cells { .. } => state(view, panel, item, nth, control, held),
    }
}

/// The next state of a control whose values are a closed list, and the
/// operation that names where it arrived.
#[allow(clippy::too_many_arguments)]
fn state(
    view: &View,
    panel: &Panel,
    item: Option<usize>,
    nth: usize,
    control: Control,
    held: impl FnOnce(u8) -> Option<Held>,
) -> Asked {
    match control {
        // -- the Mixer's five --------------------------------------------
        Control::Tally | Control::Blend | Control::Mask => {
            let deck = (item.unwrap_or(1) - 1) as u8;
            let Some(held) = held(deck) else {
                return Asked::Nothing(
                    "this deck has no slot behind that strip, so there is nothing to read a \
                     state off",
                );
            };
            Asked::Emitted(match control {
                Control::Tally => Operation::SetResidency {
                    deck,
                    residency: crate::view::residency(crate::view::next(held.requested)),
                },
                Control::Blend => Operation::SetBlendMode {
                    deck,
                    blend: crate::view::after(held.blend),
                },
                _ => Operation::SetMaskShape {
                    deck,
                    kind: crate::view::wipe_kind(crate::view::next_shape(held.mask)),
                    angle: held.mask_angle,
                },
            })
        }
        // -- the Mixer's head, which is the transition row -----------------
        Control::Shape => Asked::Emitted(Operation::SetTransition {
            setting: view.transition().next_wipe_shape(),
        }),
        Control::Quantum => Asked::Emitted(Operation::SetTransition {
            setting: view.transition().next_quantum(),
        }),
        Control::Length => Asked::Emitted(Operation::SetTransition {
            setting: view.transition().next_length(),
        }),
        // -- the Library ---------------------------------------------------
        Control::Scope => Asked::Scope,
        Control::Star => match view.rows().set(item.unwrap_or(1) - 1) {
            Some(id) => Asked::Emitted(Operation::SetFavourite {
                id: id.to_owned(),
                favourite: !view.starred.contains(id),
            }),
            None => Asked::Nothing(
                "this row is not a Set, so there is nothing to star — a row of history is a \
                 version",
            ),
        },
        // -- the Transport -------------------------------------------------
        Control::Tonemap => match view.look {
            Some(look) => Asked::Emitted(Operation::SetTonemap {
                tonemap: crate::view::next_tonemap(look.tonemap),
            }),
            None => Asked::Nothing("this console has no look behind it, so there is no tone map"),
        },
        // -- the Program bay -----------------------------------------------
        Control::Solo => match panel.layout().is_soloed() {
            true => Asked::Panel(Op::Unsolo),
            false => match panel.layout().find(PICTURE) {
                Some(id) => Asked::Panel(Op::Solo(id)),
                None => Asked::Nothing("this arrangement draws no picture to solo"),
            },
        },
        Control::Class => Asked::Nothing(
            "the class pill is opened by pressing it, and what it opens is a class rather than \
             a state to cycle — press it",
        ),
        // -- the Inspector -------------------------------------------------
        Control::Sync | Control::Composite | Control::Authority | Control::Renderer => {
            Asked::Nothing("this control is reached under a pane, not here")
        }
        // -- the Sequencer -------------------------------------------------
        Control::Mode => match view.sequencer.as_ref() {
            Some(seq) => Asked::Emitted(Operation::SetPatternGrid {
                pattern: seq.bank as u8,
                grid: match seq.pattern.mode() {
                    StepMode::Sixteenth => StepMode::Eighth,
                    StepMode::Eighth => StepMode::Sixteenth,
                },
            }),
            None => Asked::Nothing("this console has no pattern behind it"),
        },
        Control::Bank => Asked::Emitted(Operation::SelectPattern {
            // The head draws the mode pill and then the four banks, so the
            // bank a digit named is the one it counted to less that pill.
            pattern: (nth - 2) as u8,
        }),
        Control::Label => lane_state(view, item.unwrap_or(1)),
        Control::Step => step_state(view, item.unwrap_or(1), nth),
        // -- the Outputs row -----------------------------------------------
        Control::Sink => sink(panel),
        _ => Asked::Nothing("nothing here answers space"),
    }
}

/// `space` on a lane's label: the mute, named as the state it arrives at.
fn lane_state(view: &View, lane: usize) -> Asked {
    let Some(seq) = view.sequencer.as_ref() else {
        return Asked::Nothing("this console has no pattern behind it");
    };
    match seq.pattern.lanes().get(lane.wrapping_sub(1)) {
        Some(found) => Asked::Emitted(Operation::SetLaneMute {
            pattern: seq.bank as u8,
            lane: (lane - 1) as u8,
            muted: !found.muted(),
        }),
        None => Asked::Nothing("this pattern has no lane with that number"),
    }
}

/// `space` on a cell: the step, named as the state it arrives at — and the
/// stored slot rather than the drawn step, which is what keeps the payload
/// independent of the mode.
fn step_state(view: &View, lane: usize, nth: usize) -> Asked {
    let Some(seq) = view.sequencer.as_ref() else {
        return Asked::Nothing("this console has no pattern behind it");
    };
    let Some(found) = seq.pattern.lanes().get(lane.wrapping_sub(1)) else {
        return Asked::Nothing("this pattern has no lane with that number");
    };
    // The lane draws its label and then its cells, so the cell a digit named is
    // the one it counted to less that label.
    let step = nth.wrapping_sub(2);
    let mode = seq.pattern.mode();
    if step >= mode.count() {
        return Asked::Nothing("this lane is not drawing a cell with that number");
    }
    let slot = mode.slot_of(step);
    Asked::Emitted(Operation::SetStep {
        pattern: seq.bank as u8,
        lane: (lane - 1) as u8,
        step: slot as u8,
        on: !found.slot_on(slot),
    })
}

/// `space` on a sink: the picture's on and off, which is one press asking for
/// the operation that names the output and the fold that carries it out.
fn sink(panel: &Panel) -> Asked {
    let Some(id) = panel.layout().find(PICTURE) else {
        return Asked::Nothing("this arrangement draws no picture");
    };
    let on = panel.layout().visible(id);
    Asked::Routed(
        Operation::RouteFrame {
            output: Output::Program,
            on: !on,
        },
        match on {
            true => Op::Fold(id),
            false => Op::Unfold(id),
        },
    )
}

/// `space` on the third rung — the Inspector's chips, each naming the state it
/// arrives at rather than a flip, which is P-0090 and the chips' own rule.
fn under_space(view: &View, item: usize, through: usize, nth: usize, control: Control) -> Asked {
    let Some(pane) = view.inspector.get(item.wrapping_sub(1)) else {
        return Asked::Nothing("this pane is not drawn");
    };
    let deck = pane.deck as u8;
    match control {
        Control::Sync => Asked::Emitted(Operation::SetSync {
            deck,
            sync: crate::view::next_sync(pane.sync, pane.allows),
        }),
        Control::Composite => Asked::Emitted(Operation::SetCompositing {
            deck,
            compositing: !pane.composite,
        }),
        Control::Authority => match pane
            .nodes
            .get(through.wrapping_sub(2))
            .and_then(|node| node.authority)
        {
            Some(authority) => Asked::Emitted(Operation::SetAuthority {
                deck,
                node: authority.at,
                authority: next_authority(authority.level),
            }),
            None => Asked::Nothing(
                "this head stands over more than one node, so there is no one authority to set \
                 — open the fold and name the node",
            ),
        },
        Control::Renderer => match pane.nodes.get(through.wrapping_sub(2)) {
            Some(node) if !node.renderers.is_empty() && pane.composite => {
                let live = node.renderers.iter().position(|r| r.live).unwrap_or(0);
                Asked::Emitted(Operation::SelectRenderer {
                    deck,
                    renderer: ((live + 1) % node.renderers.len()) as u32,
                })
            }
            Some(_) => Asked::Nothing(
                "this deck overdraws its renderers, so they all draw and none of them is live \
                 — the composite chip in the deck head is what makes it a choice",
            ),
            None => Asked::Nothing("this pane is not drawing that node"),
        },
        Control::Anchor | Control::Param => {
            let _ = nth;
            Asked::Nothing(
                "this control is a level, and a level has no next state — the arrows step it",
            )
        }
        _ => Asked::Nothing("nothing here answers space"),
    }
}

/// The next of the three authorities, wrapping — the cycle a chip row is,
/// curated here because the list is the vocabulary's and the cycle is the
/// surface's (`view::AUTHORITIES`' own argument, one control along).
fn next_authority(level: Authority) -> Authority {
    let at = crate::view::AUTHORITIES
        .iter()
        .position(|found| *found == level)
        .unwrap_or(0);
    crate::view::AUTHORITIES[(at + 1) % crate::view::AUTHORITIES.len()]
}

/// `enter` on a control, where the control is an act.
fn act_of(view: &View, item: Option<usize>, control: Control) -> Asked {
    match control.answers() {
        Answers::Act(act) => performed(view, item, act),
        Answers::Nothing(why) => Asked::Nothing(why),
        _ => Asked::Nothing(
            "nothing here performs — enter is the act the addressed control is for, and this one \
             sets rather than performs",
        ),
    }
}

/// What `enter` performs, per act.
fn performed(view: &View, item: Option<usize>, act: Act) -> Asked {
    match act {
        Act::Load => Asked::Load,
        // **The two chain acts arrive on the rung under a control**, where
        // this function is reached from an *item*, so neither is asked here.
        // The arms are `Addressed::Of`'s and `Addressed::InCard`'s in
        // [`press`], which have the slot's position and the card's entry.
        Act::Remove | Act::Add => Asked::Nothing(
            "this is not the rung this act is on — press a digit to name a slot of the chain",
        ),
        Act::Read => match view.rows().set(item.unwrap_or(1) - 1) {
            Some(id) => Asked::Emitted(Operation::ReadSet { id: id.to_owned() }),
            None => Asked::Nothing(
                "this row is not a Set, so there is nothing to read — a row of history is a \
                 version",
            ),
        },
        Act::Keep => match view.staging.get(item.unwrap_or(1) - 1) {
            Some(candidate) => match (candidate.at, candidate.stage) {
                (Some(node), stage) if stage != crate::view::Stage::Overloaded => {
                    Asked::Emitted(Operation::KeepCandidate {
                        deck: candidate.deck as u8,
                        node,
                    })
                }
                (Some(_), _) => Asked::Nothing(
                    "this candidate cost more than the frame allows, so there is nothing to keep \
                     — the slot is stopped on the version it last drew",
                ),
                (None, _) => Asked::Nothing("this row names no node, so there is nothing to keep"),
            },
            None => Asked::Nothing("this lane is not drawing that many rows"),
        },
        Act::Back => match view.staging.get(item.unwrap_or(1) - 1) {
            Some(candidate) => match candidate.at {
                Some(node) => Asked::Emitted(Operation::RestoreProcedure {
                    deck: candidate.deck as u8,
                    revision: Revision::Previous(node),
                }),
                None => Asked::Nothing(
                    "this row names no node, so there is no previous version to put back",
                ),
            },
            None => Asked::Nothing("this lane is not drawing that many rows"),
        },
        // **Taking a parameter back is the third rung's**, because it needs
        // the pane, the node and the row the address walked through — see
        // [`under_enter`], which is the only caller that has all three.
        Act::TakeBack => Asked::Nothing(
            "a parameter is taken back on the row that draws it — press a digit to name the \
             pane, the node and the row",
        ),
        // **A card and its rows are reached on the control the card hangs
        // from**, because both have the address to move and the card to put
        // down or take away — see [`open_card`] and [`card_enter`], which
        // [`press`] reaches before this function.
        Act::Open | Act::Point => Asked::Nothing(
            "the chooser is reached in the Sequencer's head — press 0 and then the digit that \
             names + lane",
        ),
        // **The Transport's two cards are reached on the row that draws
        // them**, because each needs the pill the address descended through
        // and the card to take away — see [`card_enter`], which [`press`]
        // reaches before this function.
        Act::Attach | Act::Save | Act::Restore => Asked::Nothing(
            "this is a row of a card in the Transport — press enter on the pill that puts the \
             card down, then a digit to name the row",
        ),
        // **The addressed strip is covered and the next one round arrives over
        // it**, which is `karakuri-cli`'s `c` and the `go` capsule's own
        // reading of *the next deck*.
        Act::Go => {
            let decks = view.mixer.len();
            if decks < 2 {
                return Asked::Nothing(
                    "a transition needs somewhere to come from — this mixer draws one strip",
                );
            }
            if !view.transition().armed() {
                return Asked::Nothing(
                    "no shape is chosen, so there is nothing for the front to be — press space \
                     on the shape pill in this bay's head",
                );
            }
            let from = usize::from(view.selection()).min(decks - 1);
            Asked::Emitted(Operation::Wipe {
                from: from as u8,
                to: ((from + 1) % decks) as u8,
            })
        }
    }
}

/// `enter` on the third rung: taking a parameter back, which is the one act the
/// Inspector draws that the address reaches.
fn under_enter(view: &View, item: usize, through: usize, nth: usize, control: Control) -> Asked {
    if control != Control::Param {
        return Asked::Nothing(
            "nothing here performs — enter is the act the addressed control is for, and this one \
             sets rather than performs",
        );
    }
    let Some(pane) = view.inspector.get(item.wrapping_sub(1)) else {
        return Asked::Nothing("this pane is not drawn");
    };
    let Some(param) = pane
        .nodes
        .get(through.wrapping_sub(2))
        .and_then(|node| node.params.get(nth.wrapping_sub(3)))
    else {
        return Asked::Nothing("this parameter row is not drawn");
    };
    match param.bound.as_ref() {
        Some(source) => Asked::Emitted(Operation::TakeParamBack {
            deck: pane.deck as u8,
            param: source.at.clone(),
        }),
        None => Asked::Nothing(
            "nothing is holding this control, so there is nothing to take back — the arrows \
             write it",
        ),
    }
}
