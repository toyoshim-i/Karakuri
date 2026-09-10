//! **The bay a key press is addressed to, and what each bay remembers.**
//!
//! [ADR-0259](../../../docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
//! decided that a key press is addressed to whatever holds focus, that `Tab`
//! and `shift-Tab` move focus between bays along the arrangement's own walk,
//! and that a bay's address is a **path** — a digit names the nth thing one
//! level below it and `0` names the bay's head. This module is the first slice
//! of that ([ADR-0332](../../../docs/adr/0332-focus-is-a-pointer-the-console-owns-and-the-three-pointers-are-instances-of-it.md)):
//! the ring, the pointer that walks it, and the address each bay remembers.
//!
//! # The three pointers are three readings of one thing
//!
//! `View::selection`, `View::cursor_row` and `View::scope` were three private
//! fields with the same paragraph written at each of them — *nothing
//! downstream can be the model of record for it*. They are three readings of
//! [`Address`] now, on two bays:
//!
//! | pointer | bay | where in the address |
//! | --- | --- | --- |
//! | `View::selection` | `mixer` | the item the bay was last on — a strip |
//! | `View::cursor_row` | `library` | the item the bay was last on — a row |
//! | `View::scope` | `library` | the control last named under the head |
//!
//! **Nothing about what any of them means changes**, which is the record's own
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
//! an operator pressed `0`. The head is still a rung the address **descends**
//! through — which is exactly how the Library's scope is reached, at
//! `[HEAD]` — and it is [`Address::at`] that says so.
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
//! **A folded bay stays in the ring**, and the reason is the narrow one: it is
//! in the ring so that there is something to press to open it, not so that it
//! can be operated. So [`ring`] asks [`Layout::children`] and never
//! [`Layout::placed_children`] — the visible half is the paint's question and
//! not the walk's.
//!
//! # What this module does not do
//!
//! **It binds no key.** `crates/karakuri/src/main.rs` is where `Tab` and `esc`
//! reach these methods, for [`crate::panel`]'s reason: a surface is where the
//! buck stops and this crate is asked rather than asking.
//!
//! **The six keys are not here yet.** A digit, the arrows, `space` and `enter`
//! are the grammar ADR-0259 designs and none of them is bound; [`Address`] is
//! the shape they will write into, and what they write is the second slice.

use std::collections::BTreeMap;

use karakuri_operation::{BlendMode, Operation};

use karakuri_layout::{Layout, NodeId};

use crate::panel::Panel;
use crate::view::{region, Kind, Mask, Region, Tally, View};

/// **The digit that names a bay's head**, which is the one digit that is not
/// an item.
///
/// A constant rather than a literal, because two very different things are
/// spelled `0` in this module — the head, and the first element of a path —
/// and only one of them is this.
pub const HEAD: usize = 0;

/// **The Mixer bay, by the name the arrangement gives it** — whose remembered
/// item is the deck selection.
///
/// A constant rather than a literal at the two methods that read it, for
/// [`crate::view::REGIONS`]' own reason: a bay nobody can find by name is a
/// pointer that silently stops pointing, and `tests/focus.rs` is what asserts
/// the ring holds it.
pub const MIXER: &str = "mixer";

/// **The Library bay** — whose remembered item is the row under the cursor and
/// whose head's remembered control is the marked scope. [`MIXER`]'s reason.
pub const LIBRARY: &str = "library";

/// **Whether a region is one of the nine bays.**
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

/// **The tab ring: every bay of `layout`, in the order the walk reaches
/// them** — down a column first and only then across to the next one.
///
/// The order is the arrangement's own geometry rather than a list anybody
/// maintains, which is what keeps it right after a divider is dragged: a
/// column's children are taken top to bottom, a row's left to right, and the
/// walk stops at a bay instead of descending into it.
///
/// **A folded bay is in it** — see the module documentation. **A node the
/// arrangement names and [`REGIONS`](crate::view::REGIONS) does not is
/// structure**, so `left-pane`, `centre` and `right-pane` are walked through
/// and not into the ring, which is the same reading `View::draw` makes of
/// them.
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

/// **Where the dashed focus ring goes on `bay`**, or `None` where the
/// arrangement gives that bay no rectangle to put one on.
///
/// **The head, or the whole row for a bay that draws none.** The mock draws
/// `.wfocus` as a dashed sun outline and the deck selection as a solid lavender
/// ring, on purpose — *"drawing them the same way would erase which of the two
/// a reader is looking at"* — and `console.html`'s mark for a focused bay is
/// the head wearing it. The Transport and the Outputs row are headless
/// (ADR-0159), and ADR-0259 reads them the same way it reads their `0`: *"a
/// headless row, so `0` names the row itself"*, so the row stands in for the
/// head and the ring goes round the row.
///
/// **`None` for a folded bay, and that is the drawing ADR-0259 leaves open.** A
/// folded region has no rectangle and no divider is drawn beside it
/// ([ADR-0204](../../../docs/adr/0204-the-root-and-the-body-row-stay-unnamed-and-a-folded-root-is-not-hit-testable.md)),
/// so there is nothing on the panel to ring; `console.html` draws the mark that
/// is owed — the head alone — beside the note that defines it, and no bay is
/// folded in the panel it draws. **The bay stays in the ring either way**,
/// which is what `space` is for.
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

/// **A bay's remembered address**: where the address is inside this bay, and
/// the nth thing it last named at each level.
///
/// The path is the record's own: `[]` is the bay itself, `[2]` is its second
/// item, `[2, 3]` is that item's third control, and [`HEAD`] in place of an
/// item names the bay's head. Digits count from **one** because they count what
/// the bay drew, which is ADR-0259's change from the four deck keys.
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
    /// Where the address is now, from the bay down. Empty is the bay itself,
    /// which is where every bay starts and where `esc` stops.
    at: Vec<usize>,
    /// **The nth item last named under each path**, and never [`HEAD`] — see
    /// the module documentation, which is where that is argued.
    ///
    /// A map rather than one number, because a bay has more than one place to
    /// remember: the Library remembers which row it was on *and* which chip of
    /// its head is marked, and those are two levels of one address rather than
    /// two pointers.
    memory: BTreeMap<Vec<usize>, usize>,
}

impl Address {
    /// Where the address is now, from the bay down — empty for the bay itself.
    pub fn at(&self) -> &[usize] {
        &self.at
    }

    /// **The nth item last named under `under`**, or `None` for a path nothing
    /// has been named under.
    ///
    /// One-based, which is the digit that named it: `1` is the first thing the
    /// bay drew at that level. A caller that wants a position subtracts one,
    /// and the three places that arithmetic is written all subtract with a
    /// floor — **never because zero is reachable**, which it is not
    /// ([`Address::remember`] refuses [`HEAD`] and nothing else writes here),
    /// but because a wrapped `usize` in a paint path is a panic in an event
    /// handler and this crate's rule is the guard rather than the message.
    pub fn remembered(&self, under: &[usize]) -> Option<usize> {
        self.memory.get(under).copied()
    }

    /// **Remember that the `nth` thing under `under` was named**, and answer
    /// whether that moved anything.
    ///
    /// **[`HEAD`] is refused rather than stored**, which is the module's own
    /// rule: the head is not one of the things a bay lists, so there is nothing
    /// to remember about it and remembering it would overwrite the item this
    /// bay was on.
    ///
    /// The `bool` is `View::select`'s: a caller repaints on a move and not on a
    /// press.
    pub fn remember(&mut self, under: &[usize], nth: usize) -> bool {
        if nth == HEAD {
            return false;
        }
        self.memory.insert(under.to_vec(), nth) != Some(nth)
    }

    /// **Descend to the `nth` thing below where the address is**, remembering
    /// it on the way — which is what a digit does.
    ///
    /// Nothing here says the nth thing exists: the bay that draws it is what
    /// refuses a digit past its end, exactly as `View::select` refuses a deck
    /// the mixer has no strip for. This is the path and not the panel.
    pub fn down(&mut self, nth: usize) {
        self.remember(&self.at.clone(), nth);
        self.at.push(nth);
    }

    /// **Up one level, and answer whether there was one to leave.**
    ///
    /// `false` at the bay, which is what `esc` says out loud: there is no
    /// unfocused state to fall out into, so the key acts on nothing and the
    /// caller is the one that says so
    /// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).
    /// **What it does not do is quit**, which is the whole of ADR-0259's change
    /// to that key.
    pub fn up(&mut self) -> bool {
        self.at.pop().is_some()
    }

    /// **Back to the bay, without forgetting where it was.**
    ///
    /// `esc` repeated is the ordinary way there and this is the other one: an
    /// address on something the bay has stopped drawing names nothing, and
    /// acting on whatever has taken that position is the failure
    /// [`crate::view::View::point_at`] refuses one level down. The memory is
    /// left alone, for [`Address::up`]'s reason — what a bay *was* on is the
    /// solid ring and is not what went stale.
    pub fn to_the_bay(&mut self) {
        self.at.clear();
    }

    /// **Put the address on the `nth` item**, remembering it — what an arrow
    /// does when the address had already descended to one.
    ///
    /// It is [`Address::down`] with the level replaced rather than pushed: a
    /// walk moves along a level and never into one.
    pub fn to_item(&mut self, nth: usize) {
        self.at.clear();
        self.down(nth);
    }
}

/// **Which bay the keyboard is talking to, and what each bay remembers.**
///
/// One pointer and nine addresses. The pointer is the dashed ring the mock
/// draws; an address is the solid one, seen once per bay.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Focus {
    /// **The bay focus is on**, or `None` for a console nobody has tabbed on
    /// yet — which resolves to *the first bay the traversal reaches* rather
    /// than to a bay named here.
    ///
    /// **There is no unfocused state and this is not one.** ADR-0259 rejected
    /// starting focus on a named bay because that is a second rule to keep in
    /// step with the walk — *"the first divider dragged would have parted
    /// them"* — so the start is the walk's own answer and [`Focus::bay`] is
    /// where it is asked. The same reading is what puts focus back in the ring
    /// when a bay leaves the arrangement.
    at: Option<&'static str>,
    /// Every bay that has ever been addressed, and what it remembers. A bay
    /// with no entry has been addressed by nobody, which is where a run starts.
    addresses: BTreeMap<&'static str, Address>,
}

impl Focus {
    /// **The bay focus is on**, resolved against `layout`.
    ///
    /// The first bay of the ring where nothing has been focused yet, and again
    /// where what was focused is no longer a bay of this arrangement — a
    /// pointer at a region that is not there is a ring drawn nowhere, which is
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

    /// **Put focus on `bay`**, and answer whether it moved.
    ///
    /// Resolved against `layout` for [`Focus::bay`]'s reason: a bay this
    /// arrangement does not hold is refused rather than stored, so nothing here
    /// can name a ring drawn nowhere.
    pub fn put(&mut self, layout: &Layout, bay: &str) -> bool {
        let Some(found) = ring(layout).into_iter().find(|found| found.name == bay) else {
            return false;
        };
        let was = self.bay(layout).map(|region| region.name);
        self.at = Some(found.name);
        was != Some(found.name)
    }

    /// **`Tab`, and `shift-Tab` at `step` of `-1`**: the next bay of the ring,
    /// wrapping.
    ///
    /// **It always moves between bays, whatever depth the address had reached
    /// inside the one it leaves, and it never descends** — which is why nothing
    /// here touches an [`Address`]. The bay it leaves keeps where it was, and
    /// that is the whole of what a remembered address is for.
    ///
    /// **`shift-Tab` is this walk run backwards and nothing else.** One key is
    /// the walk and the other is the walk reversed, so an operator who
    /// overshoots gets back exactly where they were — which is what a second
    /// rule for the backward direction would have cost.
    ///
    /// `false` where there is nothing to move to: an arrangement with one bay
    /// or none. A ring of one that wrapped onto itself would be a press that
    /// changed nothing and asked for a frame.
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

    /// **What `bay` remembers**, or `None` for a bay nobody has addressed.
    pub fn address(&self, bay: &str) -> Option<&Address> {
        self.addresses.get(bay)
    }

    /// **What `bay` remembers, to write into** — created empty on the first
    /// write, which is what makes *a bay nobody has addressed* a state rather
    /// than a row of defaults.
    pub fn address_mut(&mut self, bay: &'static str) -> &mut Address {
        self.addresses.entry(bay).or_default()
    }

    /// **`esc`: up one level of the focused bay's address.**
    ///
    /// `false` at bay level, which is the refusal ADR-0259 asks to be said out
    /// loud: there is no rung below the bay and no unfocused state to fall out
    /// into, and a key that declines silently is indistinguishable from one
    /// that is not bound. **It never leaves the ring and it never quits.**
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

/// **The four keys of the grammar that are addressed to whatever the bay's
/// address is on.**
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
    /// An arrow — the neighbour of the addressed thing, along the axis it is
    /// drawn on, or the next value of a level.
    Arrows,
    /// `space` — the addressed thing's next state, or a level's declared
    /// default.
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

/// **Which way an arrow points.** Four rather than two, because the axis is
/// half of what an arrow means: the Mixer's strips are a row and the Library's
/// rows are a column, so `←→` walk one and `↑↓` the other, and a level is
/// stepped by `↑↓` whichever bay it is in.
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

    /// **Which way along its axis**: `-1` for up and left, `1` for down and
    /// right. A list runs down and a row runs right, so *further on* is one
    /// direction and the page needs no second rule for it.
    pub const fn step(self) -> i32 {
        match self {
            Arrow::Up | Arrow::Left => -1,
            Arrow::Down | Arrow::Right => 1,
        }
    }
}

/// **One press of the grammar, with what the key itself said.**
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

/// **A control a bay draws, as the grammar addresses it.**
///
/// One variant per control the two built bays draw, in the order the bay draws
/// them — which is what makes a digit name the nth of them. **It is not a list
/// of every control on the panel**: a bay whose grammar is not built has no row
/// in [`BUILT`] and no control here, which is the honest way for the seven
/// remaining bays to be absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// A strip's residency chip — `space` asks the deck for the next of the
    /// three.
    Tally,
    /// A strip's trim, which is a **level**: arrows step it and `space` returns
    /// it to the value it was declared at.
    Trim,
    /// A strip's channel fader, and [`Control::Trim`]'s rules exactly.
    Fader,
    /// A strip's blend chip — `space` cycles add, over and max.
    Blend,
    /// A strip's mask mini — `space` cycles the shape.
    Mask,
    /// The Library head's scope chips — `space` steps to the next and wraps.
    Scope,
}

impl Control {
    /// **Whether this control is a level** — a continuum the arrows step and
    /// `space` returns to its default — rather than a state, whose values are a
    /// closed list `space` cycles.
    ///
    /// It is the one distinction the grammar makes about a control, and it is
    /// ADR-0259's: *"a level is a control whose values are a continuum"*.
    pub const fn level(self) -> bool {
        matches!(self, Control::Trim | Control::Fader)
    }

    /// Which of the four keys act on this control.
    pub const fn reached_by(self, key: Grammar) -> bool {
        match key {
            // Every control has a next state or a default, so `space` acts on
            // all of them. That is not a coincidence worth hiding: it is what
            // makes `space` the key of the grammar an operator reaches for.
            Grammar::Space => true,
            // A level's neighbour is its next value. A state has none — the
            // values of a closed list are not laid out on an axis — so the
            // arrows decline there and say so.
            Grammar::Arrows => self.level(),
            // Nothing below a control is drawn on this panel, and nothing here
            // performs.
            Grammar::Digit | Grammar::Enter => false,
        }
    }
}

/// **What `enter` on one of a bay's items performs**, where an item is an act.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    /// The Set under the Library's cursor, loaded onto the selected deck —
    /// with both operands on screen before the press, which is what that
    /// argument was always about.
    Load,
}

/// **One bay's grammar: what each level of its address is made of.**
///
/// The table [`BUILT`] is written in, and the thing
/// `crates/karakuri/src/main.rs`'s `key_column` reads in place of this
/// program's `match` — a digit reaches a different row in every bay, so a scan
/// of the arms cannot say which row a press lands on and a dispatch is what can
/// ([ADR-0259](../../../docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md),
/// [ADR-0333](../../../docs/adr/0333-the-console-resolves-the-address-and-the-window-loop-names-the-operation.md)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Built {
    /// The bay, by the name the arrangement gives it.
    pub bay: &'static str,
    /// The controls of the bay's head, in the order the head draws them —
    /// empty for a head that holds none.
    pub head: &'static [Control],
    /// The controls of one of the bay's items, in the order an item draws
    /// them — empty for an item that draws none the grammar reaches.
    pub item: &'static [Control],
    /// **What `enter` on an item performs**, or `None` where an item is not an
    /// act.
    pub act: Option<Act>,
    /// **Whether naming an item is itself an operation.** The Mixer's is: a
    /// digit that names a strip *is* the deck selection, which is why that row
    /// keeps a key badge rather than losing one.
    pub selects: bool,
    /// **Which way the arrows walk this bay's items** — `true` for a row, so
    /// `←→`, and `false` for a column, so `↑↓`. It is a reading of how the bay
    /// draws them and not a preference: the mixer's strips are side by side and
    /// the library's rows are stacked.
    pub across: bool,
}

impl Built {
    /// **Whether one of the four keys acts anywhere in this bay**, derived from
    /// what the bay is made of rather than listed beside it — a second list
    /// would be a second answer to *what does `space` do here*.
    pub fn reaches(&self, key: Grammar) -> bool {
        match key {
            // **A digit names the nth item, and `0` the head.** Every bay the
            // manual lists draws items — ADR-0259's walk finds them in all
            // nine — and every bay has a head or something standing in for one,
            // so a digit reaches somewhere in any bay whose grammar is built at
            // all. What it reaches *below* an item is the two lists above.
            Grammar::Digit => true,
            // **The arrows walk those items**, whatever the items are made of,
            // and step whatever levels they hold on top of that. So this is
            // true for the same reason a digit is, and a bay with no level in
            // it still answers them.
            Grammar::Arrows => true,
            // **`space` is the only one of the four that a bay can be without**,
            // and it is the one worth deriving: a bay whose controls are all
            // levels has it for the default, a bay whose controls are all
            // states has it for the cycle, and a bay drawing no control the
            // grammar reaches has it for nothing.
            Grammar::Space => {
                self.item.iter().any(|c| c.reached_by(key))
                    || self.head.iter().any(|c| c.reached_by(key))
            }
            // **And `enter` is the other**: a bay whose items perform nothing
            // has no act for it, which is four of the nine in ADR-0259's own
            // walk.
            Grammar::Enter => self.act.is_some(),
        }
    }
}

/// **Every bay whose grammar is built, and what it is made of.**
///
/// Two of the nine. ADR-0259 walks all of them and this is the first slice:
/// the Mixer and the Library have the most built rows between them, and the
/// seven that are absent are absent rather than empty — a bay with a row here
/// and nothing under it would be a claim that the six keys reach it.
pub const BUILT: &[Built] = &[
    // **Items are the four strips and a strip's controls are its five**, in
    // the order `view::mixer` draws them down the strip: the tally, the trim,
    // the fader, the blend chip and the mask mini. So `2 3` is deck B's fader,
    // which is ADR-0259's own example and the control the mock draws `.wfocus`
    // on.
    //
    // **The Mixer's head holds no controls.** `view::head_of` gives it a title
    // and no pills, so `0` reaches the bay's own controls and there are none —
    // which the press says out loud rather than doing nothing.
    //
    // **The transition row is not here**, and that is a gap rather than an
    // omission: the shape, the quantum and the length are drawn in the bay's
    // body under the strips, so they are neither the head's nor a strip's, and
    // ADR-0259 hangs the fade, the crossfade and the wipe on them as acts on
    // the *addressed strip*. Where that row sits in the address is the
    // question ADR-0333 leaves open.
    Built {
        bay: MIXER,
        head: &[],
        item: &[
            Control::Tally,
            Control::Trim,
            Control::Fader,
            Control::Blend,
            Control::Mask,
        ],
        act: None,
        selects: true,
        across: true,
    },
    // **`0` is the head and its controls are the scope chips**; items are the
    // listed Sets, the digits name the first nine and `↑↓` walk them.
    //
    // **A row draws no control the grammar reaches**, which is ADR-0259's *"a
    // row has no state, so `space` reaches nothing in this bay below its
    // head"*. The star, the `params` chip and the row menu are drawn and are
    // not in this list: each is reached by a press and none of them is a state
    // of the row, so what a digit under a row would name is a question the
    // record does not answer and this table does not invent.
    Built {
        bay: LIBRARY,
        head: &[Control::Scope],
        item: &[],
        act: Some(Act::Load),
        selects: false,
        across: false,
    },
];

/// What `bay` is made of, or `None` for one whose grammar is not built.
pub fn built(bay: &str) -> Option<&'static Built> {
    BUILT.iter().find(|found| found.bay == bay)
}

/// **Every (bay, key) pair the grammar binds**, flattened — the dispatch table
/// as a check can read it.
///
/// `crates/karakuri/src/main.rs`'s `key_column` holds the *rows* each pair
/// reaches, because a page heading is what a check reads and is not something
/// this program says to anybody; this is the half that says which pairs exist,
/// and the two are held against each other in both directions.
pub fn reaches() -> Vec<(&'static str, Grammar)> {
    let mut found = Vec::new();
    for bay in BUILT {
        for key in Grammar::ALL {
            if bay.reaches(key) {
                found.push((bay.bay, key));
            }
        }
    }
    found
}

/// **Where a bay's address has actually got to**, resolved against what the bay
/// is drawing now.
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
    /// A control of the head, and which.
    OfHead(Control),
    /// The nth item, counting from one.
    Item(usize),
    /// A control of the nth item, and which.
    OfItem(usize, Control),
}

/// Where `at` has got to in `built`, against a bay drawing `items` of them.
pub fn addressed(built: &Built, at: &[usize], items: usize) -> Option<Addressed> {
    match *at {
        [] => Some(Addressed::Bay),
        [HEAD] => Some(Addressed::Head),
        [HEAD, nth] => built
            .head
            .get(nth.checked_sub(1)?)
            .copied()
            .map(Addressed::OfHead),
        [nth] if nth <= items => Some(Addressed::Item(nth)),
        [nth, control] if nth <= items => built
            .item
            .get(control.checked_sub(1)?)
            .copied()
            .map(|control| Addressed::OfItem(nth, control)),
        _ => None,
    }
}

/// **What the deck is holding right now**, read at the press rather than off
/// the strip a frame copied.
///
/// The three states a strip's chips cycle, and the angle the mask is wearing.
/// **This crate owns the cycle and not the reading**: a scheduled fade landing
/// between the frame and the press would leave `view::Strip` a value the deck
/// has already left behind, which is `crates/karakuri/src/main.rs`'s own rule
/// at the three keys this replaces — *"read off the deck and not off the
/// strip"*.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Held {
    /// What the deck was last **asked** for, which is what the tally cycles
    /// from — see `view::Mixer::tally`, whose argument this is one surface
    /// along.
    pub requested: Tally,
    pub blend: BlendMode,
    pub mask: Mask,
    /// The angle the slot is already wearing, carried through unchanged
    /// (ADR-0203).
    pub mask_angle: f32,
}

/// **Which way a press took a level.**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Up,
    Down,
    /// `space` — back to the value the control was declared at, which is the
    /// one state a continuum has.
    Default,
}

/// **Which level a press landed on**, since the host names a different
/// operation for each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Trim,
    Fader,
}

/// **What a press of one of the four keys asks for.**
///
/// The console resolves the address and says what was landed on; the window
/// loop names the operation, because two of the six things a press can reach
/// need the deck in front of them and one needs the store (ADR-0333).
#[derive(Debug, Clone, PartialEq)]
pub enum Asked {
    /// **Nothing there answers this key**, and the sentence that says why — a
    /// key that declines silently and a key that is not bound are the same
    /// experience
    /// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).
    Nothing(&'static str),
    /// The address moved and nothing was asked of anything.
    Moved,
    /// One operation, named outright — the deck selection and the three
    /// controls whose next state is this console's own affordance.
    Emitted(Operation),
    /// **A level, and which way the press went.** The host reads the value off
    /// the deck and names the destination, because the size of a step and the
    /// clamp on it are its arithmetic and `karakuri-cli`'s.
    Stepped { deck: u8, level: Level, step: Step },
    /// The Library head's scope, stepped — the host performs it and re-reads
    /// the listing, which is a directory read and not a thing this crate can
    /// do at all (ADR-0156).
    Scope,
    /// The Set under the Library's cursor, loaded onto the selected deck — the
    /// host's for the scope's reason, and because a preset row is taken into
    /// the store on the way.
    Load,
}

/// The sentence for a key pressed in a bay whose grammar is not built.
const NOT_BUILT: &str = "the six keys are not built in this bay yet — tab to the mixer or the \
                         library, where they are, and the letters still reach the rest";

/// **What a press asks for, and the address moved to answer it.**
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
/// the refusal did not fire. **The grammar adds a route and no exception.**
///
/// `held` is asked what the deck is holding, and only where a press needs a
/// state to cycle from. **A closure rather than a value**, because which strip
/// to read is what the address says and the address is what this function
/// resolves: a caller that read one first would have to resolve the address to
/// know which, which is the whole of what it is asking for. It answers `None`
/// where this deck has no slot behind that strip, and the press then declines
/// and says so.
pub fn press(
    view: &mut View,
    panel: &Panel,
    key: Press,
    held: impl FnOnce(u8) -> Option<Held>,
) -> Asked {
    let Some(bay) = view.focused(panel) else {
        return Asked::Nothing("this arrangement draws no bay to address a key to");
    };
    let Some(built) = built(bay.name) else {
        return Asked::Nothing(NOT_BUILT);
    };
    let items = items(view, built);
    let at: Vec<usize> = view
        .focus()
        .address(bay.name)
        .map_or_else(Vec::new, |address| address.at().to_vec());
    let Some(here) = addressed(built, &at, items) else {
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
        (Addressed::Bay, Press::Digit(HEAD)) => {
            address_into(view, bay.name, HEAD);
            match built.head.is_empty() {
                true => Asked::Nothing(
                    "this bay's head holds no control the keyboard reaches — the address is on \
                     it, and esc goes back to the bay",
                ),
                false => Asked::Moved,
            }
        }
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
        (Addressed::Item(_), Press::Digit(nth)) => match built.item.get(nth.wrapping_sub(1)) {
            Some(_) => {
                address_into(view, bay.name, nth);
                Asked::Moved
            }
            None => Asked::Nothing(
                "this item does not draw that many controls — the digits count what is drawn, \
                 from one",
            ),
        },
        (Addressed::OfHead(_) | Addressed::OfItem(..), Press::Digit(_)) => Asked::Nothing(
            "nothing below a control is drawn on this panel, so a digit here reaches nothing — \
             esc goes back up",
        ),

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
        (Addressed::OfItem(nth, control), Press::Arrow(arrow)) => match (
            control.reached_by(Grammar::Arrows),
            arrow,
        ) {
            (true, Arrow::Up | Arrow::Down) => Asked::Stepped {
                deck: (nth - 1) as u8,
                level: match control {
                    Control::Trim => Level::Trim,
                    _ => Level::Fader,
                },
                step: match arrow {
                    Arrow::Up => Step::Up,
                    _ => Step::Down,
                },
            },
            (true, _) => Asked::Nothing("a level is stepped up and down, not across"),
            (false, _) => Asked::Nothing(
                "this control's values are a closed list and a list has no axis — space cycles it",
            ),
        },
        (Addressed::OfHead(_), Press::Arrow(_)) => Asked::Nothing(
            "this control's values are a closed list and a list has no axis — space cycles it",
        ),
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
        (Addressed::OfHead(Control::Scope), Press::Space) => Asked::Scope,
        (Addressed::OfItem(nth, control), Press::Space) => cycled(nth, control, held),
        (Addressed::Bay, Press::Space) => Asked::Nothing(
            "space on a bay is folded and unfolded, and that is not bound yet — f folds the \
             region under the pointer",
        ),
        (Addressed::Head, Press::Space) => Asked::Nothing(
            "a head is not a control — press a digit to name one of the controls in it",
        ),
        (Addressed::Item(_), Press::Space) => Asked::Nothing(
            "this item has no state of its own — press a digit to name one of the controls in \
             it, where it draws any",
        ),
        (Addressed::OfHead(_), Press::Space) => {
            Asked::Nothing("nothing in this head answers space yet")
        }

        // ------------------------------------------------------------------
        // `enter` — the act the addressed thing is for
        // ------------------------------------------------------------------
        (Addressed::Item(_), Press::Enter) => match built.act {
            Some(Act::Load) => Asked::Load,
            None => Asked::Nothing(
                "this item performs nothing — enter is the act a control is for, and this bay's \
                 acts are its transition row's",
            ),
        },
        (_, Press::Enter) => Asked::Nothing(
            "nothing here performs — enter is the act the addressed control is for, and this one \
             sets rather than performs",
        ),
    }
}

/// How many items `bay` is drawing — the one reading in this module that is a
/// bay's own rather than the grammar's, and it is a reading rather than a rule.
fn items(view: &View, built: &Built) -> usize {
    match built.bay {
        MIXER => view.mixer.len(),
        LIBRARY => view.library.len(),
        _ => 0,
    }
}

/// Push `nth` onto the focused bay's address.
fn address_into(view: &mut View, bay: &'static str, nth: usize) {
    view.focus_mut().address_mut(bay).down(nth);
}

/// **A digit at bay level, naming the nth item** — and the deck selection where
/// naming one is that.
fn name_item(view: &mut View, bay: &'static str, built: &Built, nth: usize, items: usize) -> Asked {
    if nth > items {
        return Asked::Nothing(match built.selects {
            true => {
                "this deck has no strip with that number — the digits count the strips the \
                     mixer drew, from one"
            }
            false => {
                "this bay is not drawing that many rows — the digits count what is drawn, \
                      from one"
            }
        });
    }
    match built.selects {
        true => view.select((nth - 1) as u8),
        false => view.point_at(nth - 1),
    };
    // **Refused rather than clamped, and the address does not descend on a
    // refusal** — the guard above is that refusal, and it is the bay's own
    // count rather than a second rule: `View::select` and `View::point_at`
    // answer *moved* rather than *accepted*, so a press that named the item the
    // bay was already on is indistinguishable there from one that named
    // nothing, and only the count can tell them apart.
    address_into(view, bay, nth);
    match built.selects {
        true => Asked::Emitted(Operation::SelectDeck {
            deck: (nth - 1) as u8,
        }),
        false => Asked::Moved,
    }
}

/// **An arrow at bay or item level, walking the bay's items.**
///
/// **From the item the bay remembers, not from the address**, which is the one
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
    let moved = match built.selects {
        // **The strips are walked and not wrapped**, which is `View::walk`'s
        // rule one bay over: a walk is not a cycle, and a press held down must
        // not jump the length of the row.
        true => {
            let from = i64::from(view.selection());
            let to = (from + i64::from(step)).clamp(0, items as i64 - 1) as u8;
            view.select(to)
        }
        false => {
            let drawn = crate::view::library(
                panel.layout(),
                &view.scopes,
                &view.library,
                view.opened(),
                view.pointed(),
                view.library_scroll(),
            )
            .map_or(0..0, |bay| bay.drawn());
            view.walk(step, drawn)
        }
    };
    // The address follows the walk where it had descended to an item, and stays
    // at the bay where it had not.
    if !at.is_empty() {
        let nth = match built.selects {
            true => usize::from(view.selection()) + 1,
            false => view.cursor_row() + 1,
        };
        view.focus_mut().address_mut(bay).to_item(nth);
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

/// **`space` on one of a strip's controls** — the next state, named here
/// because the cycle is this console's affordance (P-0090) and read off the
/// deck by the caller because the reading is not.
fn cycled(nth: usize, control: Control, held: impl FnOnce(u8) -> Option<Held>) -> Asked {
    let deck = (nth - 1) as u8;
    if control.level() {
        return Asked::Stepped {
            deck,
            level: match control {
                Control::Trim => Level::Trim,
                _ => Level::Fader,
            },
            step: Step::Default,
        };
    }
    let Some(held) = held(deck) else {
        return Asked::Nothing(
            "this deck has no slot behind that strip, so there is nothing to read a state off",
        );
    };
    match control {
        Control::Tally => Asked::Emitted(Operation::SetResidency {
            deck,
            residency: crate::view::residency(crate::view::next(held.requested)),
        }),
        Control::Blend => Asked::Emitted(Operation::SetBlendMode {
            deck,
            blend: crate::view::after(held.blend),
        }),
        Control::Mask => Asked::Emitted(Operation::SetMaskShape {
            deck,
            kind: crate::view::wipe_kind(crate::view::next_shape(held.mask)),
            angle: held.mask_angle,
        }),
        // The two levels answered above and the scope is not a strip's.
        Control::Trim | Control::Fader | Control::Scope => {
            Asked::Nothing("nothing on this strip answers space")
        }
    }
}
