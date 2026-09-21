//! The panel's model: the arrangement, the drag in progress, and the
//! operations, each of which names what it acts on ([`Op`]).
//!
//! This is what a view drives. It holds a [`Layout`] and a pointer, and nothing
//! else: no window, no device, no toolkit. Everything here runs on a machine
//! with no graphics adapter, which is what lets *what a pointer does to a
//! divider* be answered by a test rather than by looking at it.
//!
//! # An operation returns what happened, not a line to print
//!
//! [`press`](Panel::press), [`moved`](Panel::moved),
//! [`released`](Panel::released) and [`op`](Panel::op) each return a value
//! saying what they did — [`Pressed`], [`Dragged`], [`Released`], [`Outcome`] —
//! and none of them formats a sentence. That is the one thing the model
//! deliberately does not do: `examples/layout.rs` turns a [`Dragged`] into the
//! line it prints, an egui view turns the same value into whatever it draws,
//! and neither has to parse the other's English. It is also what lets a test
//! assert that a stop held a drag, rather than counting the strings a drag
//! produced.
//!
//! # Nothing here shadows the layout
//!
//! Every question [`Layout`] can answer is asked of it, and it now answers
//! every one this file used to answer for itself. A folded region is
//! [`Layout::is_collapsed`], a solo is [`Layout::soloed`], the split enclosing
//! a region is [`Layout::parent`], the children a divider index counts are
//! [`Layout::placed_children`], where a boundary is is [`Layout::boundary`],
//! every boundary is [`Layout::boundaries`], and where a drag landed is what
//! [`Layout::set_divider`] returned.
//!
//! Nothing here stores a fact about the arrangement. [`Node`] is the tree
//! flattened for a caller that iterates it, and the only thing it carries that
//! the arena does not is how deep the flattening got.
//!
//! The two things held across events are the pointer and the drag — what is in
//! hand and where along it the pointer took hold. Both are the pointer's state
//! and not the layout's.
//!
//! # Three kinds of drag, and one thing in hand
//!
//! A boundary is one of them; a mixer strip's fader is the second; a Set
//! carried out of the Library bay is the third, and they are one `Option`
//! ([`Drag`], private) rather than three. That is not tidiness:
//! [`crate::input`]'s rule 1 — *a drag in hand keeps its claim, wherever the
//! pointer has wandered to* — asks [`dragging`](Panel::dragging), and a second
//! `Option` beside the first would be four states with two of them impossible
//! and one rule that had to remember to ask about both. One thing in hand means
//! rule 1 covers a fader by construction on the day it is written, and it
//! covered the carry the same way.
//!
//! What the three disagree about is what a drag moves. A boundary drag moves
//! the arrangement, which is this crate's, so [`moved`](Panel::moved) writes it
//! and reports where it landed — including a fold: a pane pulled out past its
//! own minimum is closed, and one whose closed edge is pulled back in is
//! opened, both performed here and reported as [`Dragged::Pane`] (ADR-0300). A
//! fader drag moves the engine's value, which this crate does not have and
//! cannot reach (ADR-0156) — so it writes nothing at all and reports a
//! [`karakuri_operation::Operation`], which is the whole of what a GUI
//! component is for: pointer motion into a number, sent to a target, and the
//! value read back and drawn (ADR-0180). Nothing here remembers what the value
//! became. The strip is drawn from what the deck says on the next frame, and a
//! number kept here would be a second copy of the deck's state.
//!
//! A carry moves nothing at all until it is let go, and it is the one of the
//! three whose *destination* is part of what it asks for. A fader knows which
//! deck at the press — the knob is on a strip — so [`Released::Let`] carries no
//! value and no target. A carry knows only what it picked up; which deck it
//! lands on is whatever strip the pointer is over when the button comes up, so
//! [`released`](Panel::released) is handed that answer the way
//! [`grab`](Panel::grab) is handed a [`Grab`]: derived by [`crate::view`],
//! which is where a strip's geometry is, and never re-derived here. See
//! [`Released::Dropped`] and [`Released::Nowhere`].
//!
//! # Solve once, then read
//!
//! [`Layout::rect`], [`Layout::hit`] and [`Layout::boundary`] each carry a
//! `debug_assert!` that the layout is not dirty, so an operation followed by a
//! read is a panic in a debug build. Everything here that reads calls
//! [`solve`](Panel::solve) first, which on a frame where nothing moved is a
//! flag test, and every operation leaves the layout clean behind it.

pub mod types;
pub use types::*;

use karakuri_layout::{Axis, Hit, Layout, NodeId, Point, Rect};
use karakuri_operation::Operation;

pub struct Panel {
    layout: Layout,
    /// The tree, flattened, rebuilt whenever the layout is.
    nodes: Vec<Node>,
    cursor: Point,
    drag: Option<Drag>,
}

impl Panel {
    /// The console's arrangement, in a viewport of `width` by `height`.
    pub fn new(width: f32, height: f32) -> Panel {
        let mut panel = Panel {
            layout: crate::layout(),
            nodes: Vec::new(),
            cursor: Point::new(-1.0, -1.0),
            drag: None,
        };
        panel.rebuild();
        panel.set_viewport(width, height);
        panel
    }

    /// The arrangement, to read. Every read of a rectangle wants
    /// [`solve`](Panel::solve) called first.
    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    /// The tree, flattened in tree order.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Where the pointer is. An [`Op`] acts on whatever this is over.
    pub fn cursor(&self) -> Point {
        self.cursor
    }

    /// Whether anything is in hand — a boundary or a fader.
    ///
    /// It exists for [`crate::input::claim`]: a drag in progress keeps the pointer
    /// whatever the pointer is currently over, so the rule has to be able to ask.
    /// It answers for both kinds because there is one thing in hand, which is why
    /// rule 1 covers a fader held against its top while the pointer runs on across
    /// two bays without a word being added to it.
    pub fn dragging(&self) -> bool {
        self.in_hand().is_some()
    }

    /// What the pointer has hold of, or `None` where nothing is in hand.
    ///
    /// # It replaced a `drag_axis() -> Option<Axis>`, and the second drag is why
    ///
    /// That method answered *the axis of the boundary in hand*, and it had one
    /// caller: [`crate::view::View`]'s cursor, which draws a resize cursor from it.
    /// A fader drag runs along an axis too and must not produce one — a resize
    /// cursor over a gesture that resizes nothing — so its honest answer there is
    /// `None`.
    ///
    /// And a `None` that means *not a boundary* cannot be told from a `None` that
    /// means *nothing at all*, which is the distinction both of its readers need.
    /// The cursor has to know that a fader in hand suppresses the hit test, or it
    /// flicks a resize cursor on the moment a fader held against its top lets the
    /// pointer wander across a boundary; and a window loop has to know that a move
    /// which emitted no operation was a fader that did not change rather than a
    /// pointer with nothing in hand, or every mouse move on the panel answers a
    /// repaint question meant for a drag. One question with three answers, rather
    /// than a predicate beside an `Option`.
    ///
    /// Which fader it is stays private, on [`Panel::cursor`]'s terms: a caller
    /// holding it would be shadowing the drag, and it is not needed — the operation
    /// a move emits carries the deck and the control already.
    pub fn in_hand(&self) -> Option<InHand> {
        Some(match self.drag.as_ref()? {
            Drag::Boundary(b) => InHand::Boundary(b.axis),
            Drag::Fader(_) => InHand::Fader,
            Drag::Carry(_) => InHand::Carrying,
        })
    }

    /// Move the pointer without dragging anything. [`moved`](Panel::moved) is the
    /// same thing with a boundary in hand.
    pub fn set_cursor(&mut self, p: Point) {
        self.cursor = p;
    }

    /// Walk the arrangement and flatten it. Order is the tree's, so a caller that
    /// colours or indents by position reads top to bottom.
    fn rebuild(&mut self) {
        let mut nodes = Vec::new();
        walk(&self.layout, self.layout.root(), 0, &mut nodes);
        self.nodes = nodes;
    }

    /// The panel fills `width` by `height` from the origin. A resize is not an
    /// edit: it produces smaller rectangles and stores nothing, so a window dragged
    /// small and large again comes back to the arrangement it left.
    pub fn set_viewport(&mut self, width: f32, height: f32) {
        self.layout.set_viewport(Rect::new(0.0, 0.0, width, height));
    }

    /// The one solve. Everything that reads calls this first; on a frame where
    /// nothing changed it is a flag test.
    pub fn solve(&mut self) {
        self.layout.solve();
    }

    /// Take a region out of the layout or restore it for drawer visibility ([`Layout::set_aside`]).
    /// Returns true if the visibility state changed (ADR-0183).
    pub fn set_aside(&mut self, id: NodeId, aside: bool) -> bool {
        let moved = self.layout.is_set_aside(id) != aside;
        self.layout.set_aside(id, aside);
        moved
    }

    /// The two regions a boundary is between. A split is often unnamed — the
    /// console's body row is, deliberately — so a split and an index alone do not
    /// say which boundary a pointer has hold of, and the pair does.
    ///
    /// [`Layout::placed_children`] is what an index counts, and this is that twice:
    /// the readout wants both names, and asking for the pair is what every caller
    /// of it was doing.
    ///
    /// A closed pane is one of them, which is the whole of how a drag reaches a
    /// region that has no rectangle: the pane is placed at zero extent with its
    /// divider still beside it, so the pair either side of that divider names it
    /// (ADR-0300).
    pub fn pair(&self, split: NodeId, index: usize) -> Option<(NodeId, NodeId)> {
        let mut children = self.layout.placed_children(split).skip(index);
        Some((children.next()?, children.next()?))
    }

    // -- input ----------------------------------------------------------

    /// The pointer went down at `p`. A press in a gap takes the boundary in hand;
    /// anywhere else it only says what is there.
    pub fn press(&mut self, p: Point) -> Pressed {
        self.solve();
        self.cursor = p;
        match self.layout.hit(p, GRAB) {
            Hit::Divider { split, index } => {
                let axis = self.layout.axis(split).expect("a divider is on a split");
                let Some(gap) = self.layout.boundary(split, index) else {
                    return Pressed::NoPair { split, index };
                };
                let boundary = axis.origin(gap);
                let offset = axis.coord(p) - boundary;
                self.drag = Some(Drag::Boundary(Boundary {
                    split,
                    index,
                    axis,
                    offset,
                    said: None,
                    held: false,
                    acted: false,
                }));
                Pressed::Grabbed {
                    split,
                    index,
                    axis,
                    at: boundary,
                    offset,
                }
            }
            Hit::View(id) => Pressed::Region {
                id,
                rect: self.layout.rect(id),
            },
            Hit::Nothing => Pressed::Nothing,
        }
    }

    /// Take a mixer strip's fader in hand using its resolved [`Grab`].
    pub fn grab(&mut self, p: Point, grab: Grab) {
        self.cursor = p;
        let said = Some(grab.value(grab.axis.coord(p)));
        self.drag = Some(Drag::Fader(Fading { grab, said }));
    }

    /// Take a Set or procedure out of the Library bay for dragging.
    pub fn carry(&mut self, p: Point, set: String, procedure: bool) {
        self.cursor = p;
        self.drag = Some(Drag::Carry(Carrying { set, procedure }));
    }

    /// What is in hand, by the name the Library row was listed under, or `None`
    /// where the gesture in progress is not a carry.
    ///
    /// For a caller building the [`Landing`] a release takes, which needs the
    /// payload.
    pub fn carried(&self) -> Option<&str> {
        match self.drag.as_ref()? {
            Drag::Carry(carrying) => Some(&carrying.set),
            _ => None,
        }
    }

    /// A move with something in hand, and what it did — see [`Dragged`]. `None`
    /// where nothing is in hand, and `None` where this move changed nothing.
    ///
    /// Absolute, both ways. A boundary takes the pointer's coordinate along the
    /// split's axis, less the offset it grabbed at, straight into
    /// [`Layout::set_divider`]; a fader takes the same coordinate through
    /// [`Grab::value`]. Nothing accumulates in either, which is what makes a drag
    /// past a stop — or past the end of a track — and back come home exactly.
    ///
    /// A carry answers `None` to every move, and that is the third kind of drag
    /// rather than a case this forgot: nothing has happened, because nothing
    /// happens until the Set is let go somewhere. [`Dragged`] has no arm for it and
    /// is not owed one — an arm meaning *a hand is carrying something and nothing
    /// happened* is `None` with a name on it, and a caller that wants to draw the
    /// carry asks [`in_hand`](Panel::in_hand), which is the question *is a gesture
    /// in progress* and already exists.
    pub fn moved(&mut self, p: Point) -> Option<Dragged> {
        self.cursor = p;
        match self.drag.as_ref()? {
            Drag::Boundary(_) => self.moved_boundary(p),
            Drag::Fader(_) => self.moved_fader(p),
            Drag::Carry(_) => None,
        }
    }

    /// A move with a boundary in hand.
    ///
    /// # A pane is closed by pulling its boundary out, and opened by pulling it in
    ///
    /// The overshoot a stop keeps — `landed - asked` — is the whole of what this
    /// reads. Its sign says which side of the boundary the pointer is pressing
    /// into, its size says how far past the stop the hand has gone on, and whether
    /// that side is already [`Layout::is_closed`] says whether the drag is closing
    /// it or opening it. See [`PULLED_THROUGH`] for the distance and
    /// [`Boundary::acted`] for why it happens once.
    ///
    /// Only a region whose arrangement says a fold leaves its edge behind is closed
    /// this way ([`Layout::keeps_its_edge`]), and only one that is at its own
    /// minimum — a boundary held by a *neighbour's* maximum is not one that has run
    /// this region out of room, and folding it there would be a fold nobody was
    /// asking for.
    fn moved_boundary(&mut self, p: Point) -> Option<Dragged> {
        let Some(Drag::Boundary(drag)) = self.drag.as_ref() else {
            return None;
        };
        let (split, index, axis, offset) = (drag.split, drag.index, drag.axis, drag.offset);
        let acted = drag.acted;
        let asked = axis.coord(p) - offset;
        let landed = self.layout.set_divider(split, index, asked);
        // `set_divider` solves before it returns, so the reads below are of a
        // clean layout.
        let by = landed - asked;
        let held = by.abs() >= STOPPED;

        if let Some(op) = (!acted)
            .then(|| self.pane_pulled(split, index, axis, by))
            .flatten()
        {
            self.op(op);
            if let Op::Unfold(pane) = op {
                self.open_at_minimum(split, index, pane);
            }
            self.solve();
            let at = self
                .layout
                .boundary(split, index)
                .map(|gap| axis.origin(gap));
            if let Some(Drag::Boundary(drag)) = self.drag.as_mut() {
                drag.acted = true;
                // **Where the boundary is now, said once** — by this very
                // value. The fold moved it to the pane's own edge, or out to
                // the pane's minimum, and [`Dragged::Pane`] is this drag
                // saying so; a second report of the same position, from the
                // next of sixty pointer events a second, would be the flood
                // [`Boundary::said`] exists to stop. `held`, because the
                // boundary is not following the pointer and will not until
                // the hand comes back to it.
                drag.said = at;
                drag.held = true;
            }
            return Some(Dragged::Pane(op));
        }

        let Some(Drag::Boundary(drag)) = self.drag.as_mut() else {
            return None;
        };
        let say = match drag.said {
            None => true,
            Some(said) => (landed - said).abs() >= WORTH_SAYING || drag.held != held,
        };
        if !say {
            return None;
        }
        drag.said = Some(landed);
        drag.held = held;

        Some(Dragged::Boundary {
            split,
            index,
            axis,
            asked,
            landed,
            held: held.then_some(by),
        })
    }

    /// Put a pane a drag has just brought back at the smallest extent it declares,
    /// whichever side of the boundary it is on.
    ///
    /// *"A pane reopened by dragging inward comes back at its declared minimum, not
    /// at whatever it was before"* — because the hand that opened it is at the
    /// window's edge, and a pane that sprang back to the 340 it was months ago
    /// would jump out from under the pointer. What it was is still stored, and `z`
    /// still brings that back.
    ///
    /// The ask is the far end and the layout's own clamp is what stops it.
    /// [`Layout::set_divider`] clamps a drag to the pair's combined bounds, so
    /// asking for the boundary to go all the way to the pane's own side lands it at
    /// exactly the pane's minimum — or at the most the pair can give it, where the
    /// neighbour's own bounds allow less. Working the position out here instead
    /// would be a second copy of that clamp, one term of which is the *other*
    /// region's.
    fn open_at_minimum(&mut self, split: NodeId, index: usize, pane: NodeId) {
        self.solve();
        let Some((a, _)) = self.pair(split, index) else {
            return;
        };
        let position = match pane == a {
            true => f32::NEG_INFINITY,
            false => f32::INFINITY,
        };
        self.layout.set_divider(split, index, position);
        self.solve();
    }

    /// Evaluates whether a drag past the stop boundary constitutes a fold/unfold operation (ADR-0300).
    ///
    /// `by` is `landed - asked`. Returns `Some(Op::Fold)` or `Some(Op::Unfold)` when
    /// threshold `PULLED_THROUGH` is exceeded and node constraints are met.
    fn pane_pulled(&mut self, split: NodeId, index: usize, axis: Axis, by: f32) -> Option<Op> {
        let (a, b) = self.pair(split, index)?;

        // Opening takes precedence when dragging outward from a closed pane.
        if self.layout.is_closed(a) && by <= -PULLED_THROUGH {
            return Some(Op::Unfold(a));
        }
        if self.layout.is_closed(b) && by >= PULLED_THROUGH {
            return Some(Op::Unfold(b));
        }
        let (into, pulled) = match by >= 0.0 {
            true => (a, by),
            false => (b, -by),
        };
        if pulled < PULLED_THROUGH || !self.layout.keeps_its_edge(into) {
            return None;
        }
        let (min, _) = self.layout.bounds(into);
        let extent = axis.extent(self.layout.rect(into));
        (extent <= min + STOPPED).then_some(Op::Fold(into))
    }

    /// A move with a fader in hand: the pointer becomes a value, and the value
    /// becomes an operation. Nothing is written, here or anywhere in this crate —
    /// see [`Dragged::Fader`].
    fn moved_fader(&mut self, p: Point) -> Option<Dragged> {
        let Some(Drag::Fader(fading)) = self.drag.as_mut() else {
            return None;
        };
        let value = fading.grab.value(fading.grab.axis.coord(p));
        // **Exactly, and not within a threshold.** See `Fading::said`: a
        // boundary's half a pixel is about what is worth *printing* at sixty
        // asks a second, and every value that differs here is a different mix.
        if fading.said == Some(value) {
            return None;
        }
        fading.said = Some(value);
        Some(Dragged::Fader(fading.grab.knob.operation(value)))
    }

    /// The pointer went up. `None` where nothing was in hand.
    ///
    /// # `onto` is the destination, resolved by whoever can resolve it
    ///
    /// What the rectangle under the pointer is — a deck, by a mixer strip or one of
    /// the four deck preview cells, or the master chain's list — or `None` for
    /// none of them, and `None` for every release that is not a carry, because the
    /// other two drags have no destination to name. See [`Landing`]. A boundary
    /// comes to rest where the layout put it and the layout is
    /// asked; a fader's rest is the deck's and nobody is asked at all
    /// ([`Released::Let`]).
    ///
    /// It is an argument for the reason [`Grab`] is one. A strip's geometry is
    /// [`crate::view::mixer`]'s answer — it depends on the values the harness
    /// handed in and on a text shaper for the words in the same strip — and this
    /// module has neither and takes neither. The alternative is the toolkit inside
    /// the model. So the view is asked, as [`crate::view::Outputs::op`] is asked,
    /// at a release instead of at a press: [`crate::view::Mixer::dropped`] is the
    /// derivation, and it is the same laid-out bay the frame drew.
    ///
    /// One door and not two. A carry could have had a `dropped(onto)` of its own
    /// beside a `released()` that never takes a destination, and then a caller that
    /// reached for the wrong one would cancel every drop in silence. Here a caller
    /// that cannot answer passes `None`, which is the honest outcome for a release
    /// that landed on nothing anyway.
    pub fn released(&mut self, onto: Option<Landing>) -> Option<Released> {
        match self.drag.take()? {
            Drag::Boundary(drag) => {
                self.solve();
                let (split, index) = (drag.split, drag.index);
                // `Gone` is what an operation during the drag leaves behind:
                // fold either side of the boundary and there is no longer a
                // pair for this index, which is [`Layout::boundary`] returning
                // `None` rather than a coordinate for something else.
                Some(match self.layout.boundary(split, index) {
                    Some(gap) => Released::Rests {
                        split,
                        index,
                        at: drag.axis.origin(gap),
                    },
                    None => Released::Gone { split, index },
                })
            }
            // No solve: a fader drag never touched the layout, so there is
            // nothing owed and nothing to read back out of it.
            Drag::Fader(fading) => Some(Released::Let {
                knob: fading.grab.knob.clone(),
            }),
            // No solve either, and for the same reason: a carry moved nothing
            // here. `onto` was resolved against the layout the *caller* had
            // already solved to hit-test it, which is the one solve there is.
            Drag::Carry(carrying) => Some(match onto {
                // **The whole of what the gesture asks for**, built here for
                // `Knob::operation`'s reason: the translation from what a hand
                // did into one operation of the vocabulary is the model's, and
                // the two operands are the payload and the destination.
                Some(Landing::Deck(deck)) => Released::Dropped(match carrying.procedure {
                    true => Operation::LoadProcedure {
                        deck,
                        procedure: carrying.set,
                    },
                    false => Operation::LoadSet {
                        deck,
                        set: carrying.set,
                    },
                }),
                // The chain takes a procedure and appends it. The destination
                // carries no position: a release over a slot appends exactly as
                // a release over `+ add` does.
                Some(Landing::Chain(Ok(operation))) => Released::Dropped(operation),
                Some(Landing::Chain(Err(why))) => Released::Refused {
                    set: carrying.set,
                    why,
                },
                None => Released::Nowhere { set: carrying.set },
            }),
        }
    }

    /// What the pointer is over, for a caller about to name an [`Op`]'s target.
    ///
    /// The resolution [`op`](Panel::op) used to do for itself, out where it
    /// belongs: *the region under the pointer* is how a keyboard chooses what to
    /// fold, and it is not part of what folding means — see [`Op`]. A caller with a
    /// control under the pointer instead of a region, or with no pointer at all,
    /// never asks this.
    ///
    /// No grab, which is [`Layout::hit`] with a zero-width divider: this answers
    /// *what did the operator mean*, and a boundary's six pixels either side exist
    /// so that a hand can find a nine-pixel gap. An operation that took them would
    /// fold the wrong region six pixels from every edge. [`press`](Panel::press) is
    /// the other one and it uses [`GRAB`], because that one is the hand.
    pub fn under(&mut self) -> Hit {
        self.solve();
        self.layout.hit(self.cursor, 0.0)
    }

    /// Act.
    pub fn op(&mut self, op: Op) -> Outcome {
        self.solve();
        let outcome = match op {
            // Two directions and no toggle, and the answer is read back out of
            // the layout rather than assumed: `folded` is what the node is
            // now, so an operation that found the node already there says the
            // truth rather than the intent.
            Op::Fold(id) => {
                self.layout.collapse(id);
                self.folded(id)
            }
            Op::Unfold(id) => {
                // Drop active solo if it hides `id` so unfolding is not masked.
                if self.layout.is_soloed() && !self.layout.visible(id) {
                    self.layout.unsolo();
                }
                // The node and the way to it. `expand` on a node that is not
                // collapsed changes nothing and marks nothing dirty, so this
                // is the path that was folded and no more than it.
                let mut node = Some(id);
                while let Some(step) = node {
                    self.layout.expand(step);
                    node = self.layout.parent(step);
                }
                self.folded(id)
            }
            Op::FoldEnclosing(id) => match self.layout.parent(id) {
                Some(split) => {
                    self.layout.collapse(split);
                    self.folded(split)
                }
                // The root is the one node with nothing enclosing it.
                None => Outcome::Nothing,
            },
            Op::UnfoldAll => {
                let folded: Vec<NodeId> = self
                    .nodes
                    .iter()
                    .map(|n| n.id)
                    .filter(|id| self.layout.is_collapsed(*id))
                    .collect();
                for id in &folded {
                    self.layout.expand(*id);
                }
                Outcome::Unfolded(folded)
            }
            Op::Solo(id) => {
                self.layout.solo(id);
                Outcome::Soloed(id)
            }
            Op::Unsolo => {
                let was = self.layout.is_soloed();
                self.layout.unsolo();
                Outcome::Unsoloed { was }
            }
            Op::Reset => {
                let viewport = self.layout.viewport();
                self.layout = crate::layout();
                self.layout.set_viewport(viewport);
                self.rebuild();
                self.drag = None;
                Outcome::Reset
            }
            Op::Report => {
                // No solve of its own: the one at the top of this method is
                // the caller's, and a second here would hide an operation that
                // left one owed rather than catch it. See the test.
                Outcome::Report(
                    self.nodes
                        .iter()
                        .map(|n| Placement {
                            id: n.id,
                            depth: n.depth,
                            rect: self.layout.rect(n.id),
                            state: match (self.layout.is_collapsed(n.id), self.layout.visible(n.id))
                            {
                                (true, _) => Visibility::Folded,
                                (false, false) => Visibility::InsideAFold,
                                (false, true) => Visibility::Visible,
                            },
                        })
                        .collect(),
                )
            }
        };
        self.solve();
        outcome
    }

    /// Put a saved arrangement in, at the viewport this window already has.
    ///
    /// It is [`Op::Reset`]'s arm with the arrangement handed in rather than built:
    /// the viewport is carried across, the tree is flattened again and any drag in
    /// hand is dropped, because the node a hand had hold of is not a node of this
    /// arrangement. The viewport in `layout` is discarded, and that is the decision
    /// rather than an omission — an arrangement carries the window it was saved at,
    /// and a console arranged on a laptop would otherwise come back on a projector
    /// with the laptop's margin round it.
    ///
    /// # Why this is a method and not a ninth [`Op`]
    ///
    /// An [`Op`] names a [`NodeId`] or nothing at all, and it is `Copy` and `Eq`
    /// because every one of its eight variants is a handle or a word. A restore's
    /// payload is a whole arrangement, which no key press, no map line and no
    /// pointer can produce — only a third party holding a store can, and this crate
    /// has no store and cannot have one (ADR-0156). So the operator's operation is
    /// `karakuri_operation::Operation::RestoreArrangement { name }`, whoever holds
    /// the store turns that name into a `Layout`, and this is where the `Layout`
    /// lands. `tests/vocabulary.rs` records the same thing from the other side: the
    /// row is in its `NO_OP` list, with the reason.
    ///
    /// Nothing here reads or refuses the bytes. A file that disagrees with itself
    /// never becomes a `Layout` at all — `karakuri_layout::Layout`'s own
    /// `TryFrom<Wire>` refuses it
    /// (`docs/adr/0158-a-saved-arrangement-that-disagrees-with-itself-is-refused-not-repaired.md`)
    /// — so by the time one arrives here it is an arrangement, and a second check
    /// would be a second answer to a question that has one.
    pub fn restore(&mut self, layout: Layout) -> Outcome {
        let viewport = self.layout.viewport();
        self.layout = layout;
        self.layout.set_viewport(viewport);
        self.rebuild();
        self.drag = None;
        self.solve();
        Outcome::Restored
    }

    /// What a fold or an unfold left behind, read out of the layout. One place,
    /// because the three operations that fold differ in what they aim at and not at
    /// all in what they report.
    fn folded(&self, id: NodeId) -> Outcome {
        Outcome::Folded {
            id,
            folded: self.layout.is_collapsed(id),
            root: id == self.layout.root(),
        }
    }
}

fn walk(layout: &Layout, id: NodeId, depth: usize, out: &mut Vec<Node>) {
    out.push(Node { id, depth });
    for child in layout.children(id) {
        walk(layout, *child, depth + 1, out);
    }
}
