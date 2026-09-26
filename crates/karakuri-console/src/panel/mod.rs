//! The panel model managing arrangement, active drags, and operations ([`Op`]).
//!
//! Operations emit outcomes rather than formatted text. Boundary drags update layout (ADR-0300),
//! fader drags emit engine operations (ADR-0156, ADR-0180), and queries require clean layouts ([`Panel::solve`]).

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

    /// Returns true if any drag gesture is active (boundary, fader, or carry).
    pub fn dragging(&self) -> bool {
        self.in_hand().is_some()
    }

    /// Returns the currently active drag item ([`InHand`]), or `None` if idle.
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

    /// Returns the pair of regions flanking the boundary at `index` of `split`.
    ///
    /// Supports referencing closed zero-extent panes beside dividers (ADR-0300).
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

    /// Returns the carried item name for constructing a [`Landing`], or `None` if not carrying.
    pub fn carried(&self) -> Option<&str> {
        match self.drag.as_ref()? {
            Drag::Carry(carrying) => Some(&carrying.set),
            _ => None,
        }
    }

    /// Updates the active drag with pointer position `p`, returning the resulting [`Dragged`] change.
    ///
    /// Returns `None` if idle, if the move caused no state change, or during carry gestures.
    pub fn moved(&mut self, p: Point) -> Option<Dragged> {
        self.cursor = p;
        match self.drag.as_ref()? {
            Drag::Boundary(_) => self.moved_boundary(p),
            Drag::Fader(_) => self.moved_fader(p),
            Drag::Carry(_) => None,
        }
    }

    /// Handles boundary motion, triggering pane fold/unfold if overshoot exceeds [`PULLED_THROUGH`].
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
                // Record new boundary position and mark held to suppress duplicate events.
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

    /// Sets a newly reopened pane to its minimum declared extent via layout clamping.
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

    /// Handles pointer release, returning the [`Released`] outcome.
    ///
    /// `onto` is the view-resolved drop destination when releasing a carry gesture ([`Landing`]).
    pub fn released(&mut self, onto: Option<Landing>) -> Option<Released> {
        match self.drag.take()? {
            Drag::Boundary(drag) => {
                self.solve();
                let (split, index) = (drag.split, drag.index);
                // `Gone` indicates an operation during drag removed this boundary pair.
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

    /// Resolves the layout element under the pointer without grab tolerance.
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

    /// Restores a saved arrangement into the existing viewport and resets active drag state.
    ///
    /// Accepts a validated `Layout` from the store (ADR-0156, ADR-0158).
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
