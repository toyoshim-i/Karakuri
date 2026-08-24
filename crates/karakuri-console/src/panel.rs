//! The panel's model: the arrangement, the drag in progress, and the
//! operations that act on whatever is under the pointer.
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
//! Where [`Layout`] can answer a question it is asked; where it cannot, the
//! answer is derived from the tree rather than kept in a field of its own. A
//! folded region is [`Layout::is_collapsed`], a solo is
//! [`Layout::is_soloed`], and where a drag landed is what
//! [`Layout::set_divider`] returned. [`Node::parent`] is the one derivation
//! that has to be stored, because `Layout` has no `parent` and a hit only ever
//! resolves to a leaf.
//!
//! The two things held across events are the pointer and the drag — which
//! boundary is in hand and where along it the pointer took hold. Both are the
//! pointer's state and not the layout's.
//!
//! # Solve once, then read
//!
//! [`Layout::rect`] and [`Layout::hit`] carry a `debug_assert!` that the
//! layout is not dirty, so an operation followed by a read is a panic in a
//! debug build. Everything here that reads calls [`solve`](Panel::solve)
//! first, which on a frame where nothing moved is a flag test, and every
//! operation leaves the layout clean behind it.

use karakuri_layout::{Axis, Hit, Layout, NodeId, Point, Rect};

/// How far either side of a boundary still grabs it. Wider than any divider
/// the console draws, which is [`Layout::hit`]'s whole argument for taking a
/// grab at all: a 9px gap is not a target a hand finds.
pub const GRAB: f32 = 6.0;

/// One node of the arrangement, flattened in tree order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Node {
    pub id: NodeId,
    /// **Derived, because [`Layout`] has no `parent`.** Folding the split that
    /// encloses the region under the pointer needs it, and a hit only ever
    /// resolves to a leaf.
    pub parent: Option<NodeId>,
    /// How deep in the tree, for a caller that indents.
    pub depth: usize,
    /// A leaf is a region something paints; a split is the thing whose gaps
    /// are visible between its children.
    pub leaf: bool,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// Fold the region under the pointer.
    Fold,
    /// Fold the split enclosing whatever is under the pointer.
    FoldEnclosing,
    /// Unfold everything folded. A folded region has no rectangle, so the
    /// pointer cannot reach it to unfold it.
    UnfoldAll,
    /// Solo the region under the pointer.
    Solo,
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
    /// [`Op::Fold`] or [`Op::FoldEnclosing`]: `folded` says which way it went,
    /// and `root` that what folded was the root — so the panel is now empty,
    /// and only [`Op::UnfoldAll`] brings it back.
    Folded {
        id: NodeId,
        folded: bool,
        root: bool,
    },
    /// [`Op::Fold`] with the pointer on a divider rather than in a region.
    /// Nothing folded.
    OnDivider { split: NodeId, index: usize },
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
    /// The operation found nothing under the pointer to act on.
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

    /// Move the pointer without dragging anything.
    /// [`moved`](Panel::moved) is the same thing with a boundary in hand.
    pub fn set_cursor(&mut self, p: Point) {
        self.cursor = p;
    }

    /// Walk the arrangement and flatten it. Order is the tree's, so a caller
    /// that colours or indents by position reads top to bottom.
    fn rebuild(&mut self) {
        let mut nodes = Vec::new();
        walk(&self.layout, self.layout.root(), None, 0, &mut nodes);
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
    pub fn pair(&self, split: NodeId, index: usize) -> Option<(NodeId, NodeId)> {
        let children = self.visible_children(split);
        match (children.get(index), children.get(index + 1)) {
            (Some(&a), Some(&b)) => Some((a, b)),
            _ => None,
        }
    }

    /// The split enclosing `id`, which [`Layout`] does not answer for — see
    /// [`Node::parent`].
    pub fn parent_of(&self, id: NodeId) -> Option<NodeId> {
        self.nodes.iter().find(|n| n.id == id)?.parent
    }

    /// The children a divider index counts, which is the visible ones.
    pub fn visible_children(&self, split: NodeId) -> Vec<NodeId> {
        self.layout
            .children(split)
            .iter()
            .copied()
            .filter(|c| !self.layout.is_collapsed(*c))
            .collect()
    }

    /// Where boundary `index` of `split` currently is, along the split's axis:
    /// the far edge of the visible child before it.
    ///
    /// Reads solved rectangles, so the caller has solved.
    pub fn boundary(&self, split: NodeId, index: usize) -> Option<f32> {
        let axis = self.layout.axis(split)?;
        let before = *self.visible_children(split).get(index)?;
        Some(far(axis, self.layout.rect(before)))
    }

    /// Every boundary in the arrangement. A window has a pointer to find them
    /// with; a caller with no pointer has this.
    pub fn dividers(&mut self) -> Vec<(NodeId, usize)> {
        self.solve();
        let splits: Vec<NodeId> = self
            .nodes
            .iter()
            .filter(|n| !n.leaf)
            .map(|n| n.id)
            .collect();
        let mut out = Vec::new();
        for split in splits {
            let visible = self.visible_children(split).len();
            for index in 0..visible.saturating_sub(1) {
                out.push((split, index));
            }
        }
        out
    }

    /// A point in the middle of a boundary's gap — what a hand aims at, and
    /// what a caller with no hand presses instead.
    pub fn grab_point(&self, split: NodeId, index: usize) -> Option<Point> {
        let axis = self.layout.axis(split)?;
        let (a, b) = self.pair(split, index)?;
        let (ra, rb) = (self.layout.rect(a), self.layout.rect(b));
        let along = (far(axis, ra) + near(axis, rb)) / 2.0;
        let across = match axis {
            Axis::Row => ra.y + ra.h / 2.0,
            Axis::Column => ra.x + ra.w / 2.0,
        };
        Some(match axis {
            Axis::Row => Point::new(along, across),
            Axis::Column => Point::new(across, along),
        })
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
                let Some(boundary) = self.boundary(split, index) else {
                    return Pressed::NoPair { split, index };
                };
                let offset = along(axis, p) - boundary;
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
        let asked = along(axis, p) - offset;
        let landed = self.layout.set_divider(split, index, asked);
        // `set_divider` solves before it returns, so the reads below are of a
        // clean layout.
        let by = landed - asked;
        let held = by.abs() >= 0.05;

        let drag = self.drag.as_mut()?;
        let say = match drag.said {
            None => true,
            Some(said) => (landed - said).abs() >= 0.5 || drag.held != held,
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
        Some(match self.boundary(split, index) {
            Some(at) => Released::Rests { split, index, at },
            None => Released::Gone { split, index },
        })
    }

    /// Act on whatever the pointer is over.
    pub fn op(&mut self, op: Op) -> Outcome {
        self.solve();
        let outcome = match op {
            Op::Fold => match self.layout.hit(self.cursor, 0.0) {
                Hit::View(id) => Outcome::Folded {
                    id,
                    folded: self.layout.toggle(id),
                    root: id == self.layout.root(),
                },
                Hit::Divider { split, index } => Outcome::OnDivider { split, index },
                Hit::Nothing => Outcome::Nothing,
            },
            Op::FoldEnclosing => {
                let target = match self.layout.hit(self.cursor, 0.0) {
                    // A divider already names its split; a region's enclosing
                    // split is its parent, which `Layout` does not answer for
                    // — see `Node::parent`.
                    Hit::Divider { split, .. } => Some(split),
                    Hit::View(id) => self.parent_of(id),
                    Hit::Nothing => None,
                };
                match target {
                    Some(id) => Outcome::Folded {
                        id,
                        folded: self.layout.toggle(id),
                        root: id == self.layout.root(),
                    },
                    None => Outcome::Nothing,
                }
            }
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
            Op::Solo => match self.layout.hit(self.cursor, 0.0) {
                Hit::View(id) => {
                    self.layout.solo(id);
                    Outcome::Soloed(id)
                }
                _ => Outcome::Nothing,
            },
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
}

fn walk(layout: &Layout, id: NodeId, parent: Option<NodeId>, depth: usize, out: &mut Vec<Node>) {
    out.push(Node {
        id,
        parent,
        depth,
        leaf: layout.axis(id).is_none(),
    });
    for child in layout.children(id) {
        walk(layout, *child, Some(id), depth + 1, out);
    }
}

/// [`Axis`]'s own `coord`, `origin` and `extent` are `pub(crate)`, so a caller
/// outside that crate writes them again. These four are that, once, where
/// everything that needs them can reach them.
pub fn along(axis: Axis, p: Point) -> f32 {
    match axis {
        Axis::Row => p.x,
        Axis::Column => p.y,
    }
}

/// The near edge of `r` along `axis`.
pub fn near(axis: Axis, r: Rect) -> f32 {
    match axis {
        Axis::Row => r.x,
        Axis::Column => r.y,
    }
}

/// The far edge of `r` along `axis`.
pub fn far(axis: Axis, r: Rect) -> f32 {
    match axis {
        Axis::Row => r.x + r.w,
        Axis::Column => r.y + r.h,
    }
}

/// The extent of `r` along `axis`.
pub fn extent(axis: Axis, r: Rect) -> f32 {
    match axis {
        Axis::Row => r.w,
        Axis::Column => r.h,
    }
}
