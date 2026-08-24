//! The panel's model: the arrangement, the drag in progress, and the
//! operations, each of which names what it acts on ([`Op`]).
//!
//! This is what a view drives. It holds a [`Layout`] and a pointer, and
//! nothing else: no window, no device, no toolkit. Everything here runs on a
//! machine with no graphics adapter, which is what lets *what a pointer does
//! to a divider* be answered by a test rather than by looking at it.
//!
//! # An operation returns what happened, not a line to print
//!
//! [`press`](Panel::press), [`moved`](Panel::moved),
//! [`released`](Panel::released) and [`op`](Panel::op) each return a value
//! saying what they did — [`Pressed`], [`Dragged`], [`Released`],
//! [`Outcome`] — and none of them formats a sentence. That is the one thing
//! the model deliberately does not do: `examples/layout.rs` turns a
//! [`Dragged`] into the line it prints, an egui view turns the same value into
//! whatever it draws, and neither has to parse the other's English. It is also
//! what lets a test assert that a stop held a drag, rather than counting the
//! strings a drag produced.
//!
//! # Nothing here shadows the layout
//!
//! Every question [`Layout`] can answer is asked of it, and it now answers
//! every one this file used to answer for itself. A folded region is
//! [`Layout::is_collapsed`], a solo is [`Layout::soloed`], the split enclosing
//! a region is [`Layout::parent`], the children a divider index counts are
//! [`Layout::visible_children`], where a boundary is is [`Layout::boundary`],
//! every boundary is [`Layout::boundaries`], and where a drag landed is what
//! [`Layout::set_divider`] returned.
//!
//! **Nothing here stores a fact about the arrangement.** [`Node`] is the tree
//! flattened for a caller that iterates it, and the only thing it carries that
//! the arena does not is how deep the flattening got.
//!
//! The two things held across events are the pointer and the drag — which
//! boundary is in hand and where along it the pointer took hold. Both are the
//! pointer's state and not the layout's.
//!
//! # Solve once, then read
//!
//! [`Layout::rect`], [`Layout::hit`] and [`Layout::boundary`] each carry a
//! `debug_assert!` that the layout is not dirty, so an operation followed by a
//! read is a panic in a debug build. Everything here that reads calls [`solve`](Panel::solve)
//! first, which on a frame where nothing moved is a flag test, and every
//! operation leaves the layout clean behind it.

use karakuri_layout::{Axis, Hit, Layout, NodeId, Point, Rect};

/// How far either side of a boundary still grabs it. Wider than any divider
/// the console draws, which is [`Layout::hit`]'s whole argument for taking a
/// grab at all: a 9px gap is not a target a hand finds.
pub const GRAB: f32 = 6.0;

/// How far short of the ask a boundary has to land before a **stop** is what
/// put it there, in pixels.
///
/// A drag that nothing stops still does not land exactly where it was asked:
/// [`Layout::set_divider`] writes a size — or, for a flexible pair, a weight
/// chosen to reproduce one — and reads the position back out of the next
/// solve, so a landing is a few ten-thousandths of a pixel off its ask by
/// arithmetic alone. This is the line between that and a constraint, and there
/// is nothing between the two to be careful about: it is two orders of
/// magnitude above single-precision noise at these coordinates and an order
/// below anything an eye or a display can resolve, so a `held` is a stop and
/// nothing else.
const STOPPED: f32 = 0.05;

/// How far the boundary has to have moved since the last thing this drag said
/// before it is worth saying anything again, in pixels.
///
/// **A drag reports what the layout did, not what the pointer asked**, and at
/// sixty asks a second the difference is a readout a person can follow against
/// one they cannot. Half a pixel is the smallest move that can change what is
/// drawn — below it the boundary is on the same physical pixel it was — so
/// this reports every move a viewer could see and none that they could not. A
/// stop starting or stopping to hold the drag is reported whatever the
/// distance: that is a change in *what is happening*, not in where the
/// boundary is.
const WORTH_SAYING: f32 = 0.5;

/// One node of the arrangement, flattened in tree order.
///
/// **Two facts, and both of them are about the flattening rather than about
/// the node.** Everything else — whether it is a view, what encloses it, where
/// it solved to — is [`Layout`]'s to answer and is asked of it by `id`. This
/// carried a `parent` until [`Layout::parent`] existed, which is the shape of
/// the mistake: a second copy of the arena, rebuilt whenever the layout was
/// replaced, and wrong the day the arena can insert or remove.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Node {
    pub id: NodeId,
    /// How deep in the tree, for a caller that indents. The one thing tree
    /// order costs to work out and the arena does not store.
    pub depth: usize,
}

/// A boundary in hand: which one, and where along it the pointer took hold.
struct Drag {
    split: NodeId,
    index: usize,
    axis: Axis,
    /// Pointer coordinate minus the boundary's, at the moment of the press.
    /// Subtracted from every later coordinate so the boundary does not jump to
    /// the pointer on the first move.
    offset: f32,
    /// Where the boundary was when this drag last reported anything, and
    /// whether what it reported was that a stop was holding it.
    ///
    /// **A drag is reported when the boundary moves, not when the pointer
    /// does.** A pointer dragged on past a stop asks for a new position sixty
    /// times a second and the boundary does not move for any of them; a report
    /// per ask is a flood that says the same thing every time, and it is worst
    /// exactly where a person is looking hardest. So a stop is reported once,
    /// and the next [`Dragged`] is the one where the boundary moves again.
    said: Option<f32>,
    held: bool,
}

/// What an operation acts on. Named as operations rather than as keys, so a
/// caller with no keyboard can ask for one.
///
/// # An operation carries what it acts on, and the pointer is a way of
/// choosing that
///
/// **Every operation here used to mean *the region under the pointer***:
/// `Fold` was *fold whatever [`Panel::cursor`] is over*, and the model
/// resolved the pointer itself, once per operation. That reads fine while the
/// keyboard is the only surface, and it is wrong as soon as anything else asks
/// — because *under the pointer* is not part of what folding **is**. It is one
/// way of naming which region to fold.
///
/// The console's Outputs row is the case that proves it. Its one control folds
/// `program-view` — the picture — whatever the pointer happens to be over,
/// because the pointer is over the control, at the other end of the panel. A
/// pointer-relative vocabulary has no way to say that: it would need a second
/// operation meaning *fold that one instead*, and then two names for one act.
///
/// So the four operations that act on a node take the node, and the four that
/// act on the arrangement as a whole take nothing:
///
/// - [`Fold`](Op::Fold), [`Unfold`](Op::Unfold),
///   [`FoldEnclosing`](Op::FoldEnclosing) and [`Solo`](Op::Solo) name a
///   [`NodeId`].
/// - [`UnfoldAll`](Op::UnfoldAll), [`Unsolo`](Op::Unsolo),
///   [`Reset`](Op::Reset) and [`Report`](Op::Report) have nothing to name.
///
/// **Resolving the pointer is the caller's**, and it is [`Panel::under`]: the
/// harness does it for a key press, and the Outputs control does not need it.
/// That also moves two answers out of [`Outcome`] and into the caller, where
/// they were always about the pointer rather than about the operation — *the
/// pointer is on a divider* and *there is nothing under the pointer* are
/// things a resolution finds, not things a fold reports.
///
/// # Folding and unfolding are not mirror images, and that is the point
///
/// [`Fold`](Op::Fold) folds one node and nothing else. [`Unfold`](Op::Unfold)
/// makes one node **visible**, which takes its collapsed ancestors with it and
/// a solo that is hiding it. **Hiding a thing hides one thing; showing a thing
/// means showing the way to it** — the ordinary shape of revealing a node in a
/// tree, and the reason the two are not each other's inverse.
///
/// The alternative is `expand(id)` and nothing else, and the console's one
/// control is what settles it against: with the Program bay folded around the
/// picture, the Outputs dot is dark, and a press that expanded the picture
/// inside a still-folded bay would light nothing, move nothing and say
/// nothing. `docs/manual/console.html` rules on exactly that shape, in the
/// note about this very row — *"Nothing is refused here, so nothing has to be
/// explained: a control that quietly declines the last of something is a rule
/// an operator can only find by experiment."* A control that appears not to
/// respond is that rule with no words at all.
///
/// [`UnfoldAll`](Op::UnfoldAll) keeps its own meaning and is not folded into
/// this: it is what reaches a fold **nobody has a name for**, and `Unfold`
/// undoes only what stands between one named node and the screen.
///
/// # Fold and unfold are two operations, and there is no toggle
///
/// A toggle is an affordance built **over** two operations by whoever draws it
/// — the Outputs dot asks for [`Fold`](Op::Fold) when the picture is on and
/// [`Unfold`](Op::Unfold) when it is off — and it is not one itself. The
/// vocabulary is what every surface routes into: a MIDI map with a button per
/// direction, an MCP call that says which one it wants, and a keyboard, all
/// have to be able to say *fold this* and mean it. `Fold` folded-or-unfolded
/// could never be asked for the direction it will take, which is the same
/// reason a `Toggle` variant is not here.
///
/// It cost nothing where the pointer is the surface: a folded region has no
/// rectangle, so the pointer could never be over one, so `f` on the keyboard
/// only ever folded. What changes is that a *named* target can be folded
/// already, and now the caller says which way it means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// Fold this region away.
    Fold(NodeId),
    /// **Make this region visible**, undoing whatever stands between it and
    /// the screen: `id` itself, every collapsed ancestor of it, and a solo
    /// that is keeping it off screen.
    ///
    /// **Not the same as [`UnfoldAll`](Op::UnfoldAll)**: a sink turned back on
    /// brings back *that* picture and the way to it, and nothing else that
    /// happens to be folded. **Not the mirror of [`Fold`](Op::Fold)** either —
    /// see the note above.
    Unfold(NodeId),
    /// Fold the split enclosing this region.
    FoldEnclosing(NodeId),
    /// Unfold everything folded. A folded region has no rectangle, so the
    /// pointer cannot reach it to unfold it.
    UnfoldAll,
    /// Solo this region.
    Solo(NodeId),
    /// Undo the solo.
    Unsolo,
    /// A fresh arrangement, at the same viewport.
    Reset,
    /// Every node and where it solved to.
    Report,
}

/// What a press found under the pointer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pressed {
    /// A boundary is now in hand, and every later [`Panel::moved`] moves it.
    Grabbed {
        split: NodeId,
        index: usize,
        axis: Axis,
        /// Where the boundary was, along the split's axis.
        at: f32,
        /// Where along it the pointer took hold: the pointer's coordinate less
        /// the boundary's.
        offset: f32,
    },
    /// A divider the tree cannot produce a pair for, so there is nothing to
    /// drag. Nothing is in hand.
    NoPair { split: NodeId, index: usize },
    /// The interior of a region.
    Region { id: NodeId, rect: Rect },
    /// Outside the viewport, or somewhere no visible region claims.
    Nothing,
}

/// What a move with a boundary in hand did to that boundary.
///
/// Produced only when the boundary moved, or when the stop holding it changed:
/// a drag is reported when the layout does something, not when the pointer
/// does, so a pointer held against a stop reports once and then nothing. A
/// move that asked for a position the boundary is already at produces nothing
/// at all.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dragged {
    pub split: NodeId,
    pub index: usize,
    pub axis: Axis,
    /// Where the drag asked the boundary to go: the pointer's coordinate along
    /// the axis, less the offset it grabbed at.
    pub asked: f32,
    /// Where it went, which is what [`Layout::set_divider`] returned.
    pub landed: f32,
    /// `Some(by)` when a stop held it, where `by` is `landed - asked` — how
    /// far short of the ask, and which way. `None` when it landed where it was
    /// asked to.
    pub held: Option<f32>,
}

/// What a release left behind: the boundary that was in hand, and where it
/// came to rest.
///
/// [`Panel::released`] returns `None` when nothing was in hand, because a
/// release that lets go of nothing did nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Released {
    Rests {
        split: NodeId,
        index: usize,
        /// Along the split's axis.
        at: f32,
    },
    /// The boundary is no longer there — an operation during the drag folded
    /// one of the pair away.
    Gone { split: NodeId, index: usize },
}

/// Whether a node is drawing, and if not, why not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Visible,
    /// Folded itself.
    Folded,
    /// Not folded, but inside something that is.
    InsideAFold,
}

/// One node of the arrangement and where it solved to. What [`Op::Report`]
/// produces, one per [`Node`], in the same order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub id: NodeId,
    pub depth: usize,
    pub rect: Rect,
    pub state: Visibility,
}

/// What an [`Op`] did.
///
/// One enum over every operation rather than one type each, because a caller
/// dispatches an `Op` from a key or a message and matches the outcome in the
/// same place. Which variant an operation can produce is on the operation.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// [`Op::Fold`], [`Op::Unfold`] or [`Op::FoldEnclosing`]: `folded` is what
    /// the node **is** afterwards, read back out of the layout rather than
    /// assumed from the operation, and `root` that what folded was the root —
    /// so the panel is now empty, and only [`Op::UnfoldAll`] brings it back.
    Folded {
        id: NodeId,
        folded: bool,
        root: bool,
    },
    /// [`Op::UnfoldAll`]: everything that was folded, in tree order. Empty
    /// when nothing was.
    Unfolded(Vec<NodeId>),
    /// [`Op::Solo`].
    Soloed(NodeId),
    /// [`Op::Unsolo`]. `was` is false when there was no solo to undo, and
    /// nothing changed.
    Unsoloed { was: bool },
    /// [`Op::Reset`].
    Reset,
    /// [`Op::Report`].
    Report(Vec<Placement>),
    /// The operation had nothing to act on: [`Op::FoldEnclosing`] on the root,
    /// which is the one node with nothing enclosing it.
    ///
    /// **It used to be the answer to *nothing under the pointer* as well**,
    /// and that half of it moved out to the caller with the pointer itself —
    /// see [`Op`] and [`Panel::under`]. What is left is the case that is about
    /// the arrangement rather than about where a hand is.
    Nothing,
}

/// The console panel: the arrangement, the pointer, and the drag in hand.
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

    /// The tree, flattened in tree order.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Where the pointer is. An [`Op`] acts on whatever this is over.
    pub fn cursor(&self) -> Point {
        self.cursor
    }

    /// Whether a boundary is in hand.
    ///
    /// It exists for [`crate::input::claim`]: a drag in progress keeps the
    /// pointer whatever the pointer is currently over, so the rule has to be
    /// able to ask.
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// The axis of the boundary in hand, for a view that draws a resize
    /// cursor. `None` where nothing is in hand.
    ///
    /// The axis and nothing more: *which* boundary it is stays private,
    /// because a caller that held it would be shadowing the drag.
    pub fn drag_axis(&self) -> Option<Axis> {
        self.drag.as_ref().map(|d| d.axis)
    }

    /// Move the pointer without dragging anything.
    /// [`moved`](Panel::moved) is the same thing with a boundary in hand.
    pub fn set_cursor(&mut self, p: Point) {
        self.cursor = p;
    }

    /// Walk the arrangement and flatten it. Order is the tree's, so a caller
    /// that colours or indents by position reads top to bottom.
    fn rebuild(&mut self) {
        let mut nodes = Vec::new();
        walk(&self.layout, self.layout.root(), 0, &mut nodes);
        self.nodes = nodes;
    }

    /// The panel fills `width` by `height` from the origin. A resize is not an
    /// edit: it produces smaller rectangles and stores nothing, so a window
    /// dragged small and large again comes back to the arrangement it left.
    pub fn set_viewport(&mut self, width: f32, height: f32) {
        self.layout.set_viewport(Rect::new(0.0, 0.0, width, height));
    }

    /// The one solve. Everything that reads calls this first; on a frame where
    /// nothing changed it is a flag test.
    pub fn solve(&mut self) {
        self.layout.solve();
    }

    /// The two regions a boundary is between. A split is often unnamed — the
    /// console's body row is, deliberately — so a split and an index alone do
    /// not say which boundary a pointer has hold of, and the pair does.
    ///
    /// [`Layout::visible_children`] is what an index counts, and this is that
    /// twice: the readout wants both names, and asking for the pair is what
    /// every caller of it was doing.
    pub fn pair(&self, split: NodeId, index: usize) -> Option<(NodeId, NodeId)> {
        let mut children = self.layout.visible_children(split).skip(index);
        Some((children.next()?, children.next()?))
    }

    // -- input ----------------------------------------------------------

    /// The pointer went down at `p`. A press in a gap takes the boundary in
    /// hand; anywhere else it only says what is there.
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
                self.drag = Some(Drag {
                    split,
                    index,
                    axis,
                    offset,
                    said: None,
                    held: false,
                });
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

    /// A move with a boundary in hand. **Absolute**: the pointer's coordinate
    /// along the split's axis, less the offset it grabbed at, straight into
    /// [`Layout::set_divider`]. Nothing accumulates, which is what makes a
    /// drag past a stop and back come home exactly.
    ///
    /// Returns what happened to the boundary, when this move changed anything
    /// worth reporting — see [`Dragged`]. `None` where nothing is in hand, and
    /// `None` where the boundary is where it already was.
    pub fn moved(&mut self, p: Point) -> Option<Dragged> {
        self.cursor = p;
        let drag = self.drag.as_ref()?;
        let (split, index, axis, offset) = (drag.split, drag.index, drag.axis, drag.offset);
        let asked = axis.coord(p) - offset;
        let landed = self.layout.set_divider(split, index, asked);
        // `set_divider` solves before it returns, so the reads below are of a
        // clean layout.
        let by = landed - asked;
        let held = by.abs() >= STOPPED;

        let drag = self.drag.as_mut()?;
        let say = match drag.said {
            None => true,
            Some(said) => (landed - said).abs() >= WORTH_SAYING || drag.held != held,
        };
        if !say {
            return None;
        }
        drag.said = Some(landed);
        drag.held = held;

        Some(Dragged {
            split,
            index,
            axis,
            asked,
            landed,
            held: held.then_some(by),
        })
    }

    /// The pointer went up. `None` where nothing was in hand.
    pub fn released(&mut self) -> Option<Released> {
        let drag = self.drag.take()?;
        self.solve();
        let (split, index) = (drag.split, drag.index);
        // `Gone` is what an operation during the drag leaves behind: fold
        // either side of the boundary and there is no longer a pair for this
        // index, which is [`Layout::boundary`] returning `None` rather than a
        // coordinate for something else.
        Some(match self.layout.boundary(split, index) {
            Some(gap) => Released::Rests {
                split,
                index,
                at: drag.axis.origin(gap),
            },
            None => Released::Gone { split, index },
        })
    }

    /// **What the pointer is over, for a caller about to name an
    /// [`Op`]'s target.**
    ///
    /// The resolution [`op`](Panel::op) used to do for itself, out where it
    /// belongs: *the region under the pointer* is how a keyboard chooses what
    /// to fold, and it is not part of what folding means — see [`Op`]. A
    /// caller with a control under the pointer instead of a region, or with no
    /// pointer at all, never asks this.
    ///
    /// **No grab**, which is [`Layout::hit`] with a zero-width divider: this
    /// answers *what did the operator mean*, and a boundary's six pixels
    /// either side exist so that a **hand** can find a nine-pixel gap. An
    /// operation that took them would fold the wrong region six pixels from
    /// every edge. [`press`](Panel::press) is the other one and it uses
    /// [`GRAB`], because that one is the hand.
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
                // **The solo first, because it is what the expanding would
                // otherwise be undone by.** A solo collapses everything it did
                // not name and keeps the flags it replaced; `unsolo` puts
                // exactly those back — so expanding under a live solo leaves
                // `Layout::soloed` naming a region that is no longer the only
                // one on screen, and the next unsolo quietly throws the expand
                // away. Dropped only when the solo is what is hiding `id`: a
                // solo that already has it on screen is not this operation's
                // business.
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

    /// What a fold or an unfold left behind, read out of the layout. One
    /// place, because the three operations that fold differ in what they aim
    /// at and not at all in what they report.
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
