//! The arena, the solve, and the operations that change what is stored.
//!
//! Read the crate documentation first: **solving never writes back into the
//! model**, and everything below is arranged so that it cannot.
//!
//! The arrangement — the nodes, the root, and what a [`Layout::solo`] saved —
//! is one struct, [`Arrangement`]; the buffers a solve writes are another,
//! [`Solved`]. [`Layout::solve`] destructures itself into the two, which gives
//! it disjoint borrows, and hands the solve `&Arrangement` with `&mut Solved`.
//! So the solve is a pair of free functions that *cannot* write a node: there
//! is no `&mut` to one anywhere in the call, and an edit that tried to store a
//! solved size is a borrow error rather than a slow leak nobody sees. Writing
//! a node needs [`Layout::node_mut`], and every caller of it is an operation.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::spec::Spec;
use crate::{Axis, Point, Rect, Sizing};

/// A handle into a [`Layout`]'s arena.
///
/// Opaque on purpose: a name is resolved once by [`Layout::find`], at build or
/// load time, and everything on the frame path carries the id. A name lookup
/// per frame per region is a string comparison the panel never has to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(usize);

/// What a point is touching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    /// A view's interior.
    View(NodeId),
    /// The boundary between visible children `index` and `index + 1` of
    /// `split` — the pair a [`Layout::set_divider`] with the same `index`
    /// moves.
    Divider { split: NodeId, index: usize },
    /// Outside the viewport, or somewhere no visible node claims.
    Nothing,
}

/// What a node is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum Kind {
    View {
        name: String,
    },
    Split {
        /// Optional, where a view's is required: most splits are structure
        /// nobody addresses, and the ones that are — the console's left pane
        /// is one — are addressed by exactly the same name a view is.
        name: Option<String>,
        axis: Axis,
        divider: f32,
        children: Vec<NodeId>,
    },
}

/// One node of the arena. The constraints are along the *parent's* axis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Node {
    kind: Kind,
    sizing: Sizing,
    min: f32,
    /// `f32::INFINITY` means unbounded, and is written as `null` rather than as
    /// a number — see [`unbounded`].
    #[serde(with = "unbounded")]
    max: f32,
    collapsed: bool,
    parent: Option<NodeId>,
}

impl Node {
    /// The name this node answers to, if it has one.
    fn name(&self) -> Option<&str> {
        match &self.kind {
            Kind::View { name } => Some(name),
            Kind::Split { name, .. } => name.as_deref(),
        }
    }
}

/// An unbounded maximum is `f32::INFINITY`, and JSON has no spelling for it:
/// `serde_json` writes an infinity as `null` and then refuses to read a `null`
/// back as an `f32`, so a layout saved with any default maximum would not load.
/// Writing it as an explicit absence round trips, and reads as what it means.
mod unbounded {
    use super::{Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(v: &f32, s: S) -> Result<S::Ok, S::Error> {
        match v.is_finite() {
            true => s.serialize_some(v),
            false => s.serialize_none(),
        }
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<f32, D::Error> {
        Ok(Option::<f32>::deserialize(d)?.unwrap_or(f32::INFINITY))
    }
}

/// The arrangement itself: what the operator arranged, and nothing that is
/// derived from it.
///
/// **This half is what a solve may not write.** It is a separate struct so
/// that saying so is the compiler's job rather than a comment's: the solve
/// takes `&Arrangement`, so every node in it is read-only for the whole of the
/// solve, and no amount of care is required to keep it that way.
#[derive(Debug, Clone)]
struct Arrangement {
    nodes: Vec<Node>,
    root: NodeId,
    /// Whether [`Layout::solo`] is in force, and the collapsed flags it
    /// replaced. Both are saved: an arrangement stored while soloed comes back
    /// soloed, and [`Layout::unsolo`] still has something to restore.
    soloed: bool,
    saved: Vec<bool>,
}

/// What a solve writes, and the only thing it writes.
///
/// Sized once at build, so a solved frame allocates nothing.
#[derive(Debug, Clone)]
struct Solved {
    /// Parallel to the arena's nodes. The solve writes here and
    /// [`Layout::rect`] reads here.
    rects: Vec<Rect>,
    /// Scratch for one split's children, sized to the whole arena so it fits
    /// any split. A split's sizes are finished before the solve descends into
    /// any child, so one buffer serves the whole recursion.
    sizes: Vec<f32>,
    frozen: Vec<bool>,
}

/// An arrangement of regions, and the rectangles it currently solves to.
///
/// `serde` on this is the whole of saving and restoring an operator's
/// arrangement: the stored sizes, the collapsed flags and the viewport are all
/// here, and the solved rectangles are not — they are derived, and a derived
/// value on disk is a second answer waiting to disagree with the first.
#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "Wire")]
pub struct Layout {
    arrangement: Arrangement,
    viewport: Rect,
    solved: Solved,
    dirty: bool,
}

/// The wire form. It exists so that a deserialised `Layout` arrives with its
/// rectangle and scratch buffers already sized: leaving them out of the derive
/// alone would leave them empty, and the first solve after a load would
/// allocate. It is also where a saved arrangement is checked for the name rule
/// [`Layout::new`] states — on this side it is a rejected file rather than a
/// panic, because a file is data and a [`Spec`] is code.
#[derive(Deserialize)]
struct Wire {
    nodes: Vec<Node>,
    root: NodeId,
    viewport: Rect,
    #[serde(default)]
    soloed: bool,
    #[serde(default)]
    saved: Vec<bool>,
}

/// The same shape, borrowed, so saving copies nothing. Named `Layout` on the
/// wire because that is what it is; only the buffers it leaves out differ.
#[derive(Serialize)]
#[serde(rename = "Layout")]
struct WireOut<'a> {
    nodes: &'a [Node],
    root: NodeId,
    viewport: Rect,
    soloed: bool,
    saved: &'a [bool],
}

impl Serialize for Layout {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        WireOut {
            nodes: &self.arrangement.nodes,
            root: self.arrangement.root,
            viewport: self.viewport,
            soloed: self.arrangement.soloed,
            saved: &self.arrangement.saved,
        }
        .serialize(s)
    }
}

impl TryFrom<Wire> for Layout {
    type Error = String;

    fn try_from(w: Wire) -> Result<Layout, String> {
        if let Some(name) = duplicate_name(&w.nodes) {
            return Err(duplicate_message(name));
        }
        let mut saved = w.saved;
        saved.resize(w.nodes.len(), false);
        let mut layout = Layout::assemble(w.nodes, w.root, saved);
        layout.arrangement.soloed = w.soloed;
        layout.viewport = sane(w.viewport);
        layout.solve();
        Ok(layout)
    }
}

/// The first name that appears twice, in declaration order, or `None` where
/// every name is its own. Linear because an arrangement is tens of nodes and a
/// hash set would cost more than it saved.
fn duplicate_name(nodes: &[Node]) -> Option<&str> {
    nodes.iter().enumerate().find_map(|(i, n)| {
        let name = n.name()?;
        nodes[..i]
            .iter()
            .filter_map(Node::name)
            .any(|earlier| earlier == name)
            .then_some(name)
    })
}

fn duplicate_message(name: &str) -> String {
    format!(
        "two nodes are named {name:?}: a name addresses one region, and every surface \
         that is not a pointer reaches a region by its name"
    )
}

/// A viewport with a negative — or NaN — size is a caller's arithmetic, not a
/// state this crate has an answer for. It becomes zero, because every rectangle
/// this crate produces is non-negative and a negative one would propagate
/// through every child.
fn sane(r: Rect) -> Rect {
    Rect::new(r.x, r.y, r.w.max(0.0), r.h.max(0.0))
}

impl Layout {
    /// Build an arrangement from its declarative form.
    ///
    /// The viewport starts empty, so every rectangle is zero until
    /// [`set_viewport`](Layout::set_viewport) says otherwise.
    ///
    /// # Names are unique, and this is where that is checked
    ///
    /// **Panics if two nodes carry the same name**, whether they are views,
    /// splits or one of each. A name is how every surface that is not a
    /// pointer — the keyboard, a MIDI map, MCP — reaches a region, so a name
    /// that answers to two regions is an arrangement that cannot be operated,
    /// and [`find`](Layout::find) resolving whichever came first would make it
    /// look like it could. It is a mistake in a [`Spec`], which is code, so it
    /// is caught at the earliest moment it exists rather than at the first
    /// operation aimed at the wrong pane.
    ///
    /// Deserialising a layout whose names collide fails with the same message
    /// instead of panicking: a file is data, and data is rejected.
    pub fn new(spec: Spec) -> Layout {
        let mut nodes = Vec::new();
        let root = build(&mut nodes, spec, None);
        if let Some(name) = duplicate_name(&nodes) {
            panic!("{}", duplicate_message(name));
        }
        let saved = vec![false; nodes.len()];
        let mut layout = Layout::assemble(nodes, root, saved);
        layout.solve();
        layout
    }

    /// The one place the derived buffers are sized, so `new` and a
    /// deserialisation cannot disagree about it.
    fn assemble(nodes: Vec<Node>, root: NodeId, saved: Vec<bool>) -> Layout {
        let n = nodes.len();
        Layout {
            arrangement: Arrangement {
                nodes,
                root,
                soloed: false,
                saved,
            },
            viewport: Rect::new(0.0, 0.0, 0.0, 0.0),
            solved: Solved {
                rects: vec![Rect::new(0.0, 0.0, 0.0, 0.0); n],
                sizes: vec![0.0; n],
                frozen: vec![false; n],
            },
            dirty: true,
        }
    }

    /// The root, which always fills the viewport exactly. Its own [`Sizing`],
    /// `min` and `max` are ignored — there is nothing to be relative to.
    pub fn root(&self) -> NodeId {
        self.arrangement.root
    }

    /// Resolve a name to its id. A view always has one; a split has one where
    /// the arrangement gave it one.
    ///
    /// Names are the caller's, and [`Layout::new`] refuses an arrangement that
    /// uses one twice, so the answer here is the only node that could be meant.
    pub fn find(&self, name: &str) -> Option<NodeId> {
        self.arrangement
            .nodes
            .iter()
            .position(|n| n.name() == Some(name))
            .map(NodeId)
    }

    /// A node's name: a view's, a named split's, or `None` for a split the
    /// arrangement left unnamed.
    pub fn name(&self, id: NodeId) -> Option<&str> {
        self.node(id.0).name()
    }

    /// A split's children in order, or an empty slice for a view. Collapsed
    /// children are still here — they have a zero-extent rectangle, not no
    /// rectangle.
    pub fn children(&self, id: NodeId) -> &[NodeId] {
        match &self.node(id.0).kind {
            Kind::Split { children, .. } => children,
            Kind::View { .. } => &[],
        }
    }

    /// A split's axis, or `None` for a view.
    pub fn axis(&self, id: NodeId) -> Option<Axis> {
        self.arrangement.split_of(id.0).map(|(axis, _)| axis)
    }

    /// The divider thickness a split declares. What is actually drawn is this
    /// until the split is too narrow to hold that many, at which point the
    /// dividers shrink with it rather than the children overflowing.
    pub fn divider(&self, id: NodeId) -> Option<f32> {
        self.arrangement.split_of(id.0).map(|(_, divider)| divider)
    }

    /// A node's `[min, max]` along its parent's axis. `max` is
    /// `f32::INFINITY` where it is unbounded.
    ///
    /// Exposed because a caller that has just been told by
    /// [`set_divider`](Layout::set_divider) that a drag landed somewhere other
    /// than where it was aimed has no other way to say which constraint
    /// stopped it.
    pub fn bounds(&self, id: NodeId) -> (f32, f32) {
        (self.node(id.0).min, self.node(id.0).max)
    }

    /// The viewport as it was last set.
    pub fn viewport(&self) -> Rect {
        self.viewport
    }

    /// Give the layout the space it has. Marks it dirty; nothing is recomputed
    /// until [`solve`](Layout::solve).
    ///
    /// **This is not an edit.** No stored size changes, however small the
    /// viewport gets — see the crate documentation.
    pub fn set_viewport(&mut self, viewport: Rect) {
        let viewport = sane(viewport);
        if viewport != self.viewport {
            self.viewport = viewport;
            self.dirty = true;
        }
    }

    /// The rectangle `id` last solved to.
    ///
    /// Every rectangle has non-negative width and height, and every visible
    /// node's lies inside the viewport.
    pub fn rect(&self, id: NodeId) -> Rect {
        debug_assert!(!self.dirty, "rect() read a stale solve; call solve() first");
        self.solved.rects[id.0]
    }

    /// Whether `id` is drawn: false if it or any ancestor is collapsed.
    pub fn visible(&self, id: NodeId) -> bool {
        let mut cur = Some(id);
        while let Some(NodeId(i)) = cur {
            if self.node(i).collapsed {
                return false;
            }
            cur = self.node(i).parent;
        }
        true
    }

    /// Whether `id` itself is collapsed, regardless of its ancestors.
    pub fn is_collapsed(&self, id: NodeId) -> bool {
        self.node(id.0).collapsed
    }

    /// Whether a [`solo`](Layout::solo) is in force.
    pub fn is_soloed(&self) -> bool {
        self.arrangement.soloed
    }

    // -- operations ------------------------------------------------------

    /// Fold `id` away: zero extent, and no divider beside it.
    ///
    /// Its stored size is untouched, which is the whole of why
    /// [`expand`](Layout::expand) can restore it exactly. Folding a split
    /// folds everything inside it: nothing under a collapsed node is
    /// [`visible`](Layout::visible), and none of it takes any space.
    pub fn collapse(&mut self, id: NodeId) {
        self.set_collapsed(id, true);
    }

    /// Unfold `id`, back to the size it was storing all along.
    pub fn expand(&mut self, id: NodeId) {
        self.set_collapsed(id, false);
    }

    /// Flip `id`, returning whether it is now collapsed.
    pub fn toggle(&mut self, id: NodeId) -> bool {
        let now = !self.node(id.0).collapsed;
        self.set_collapsed(id, now);
        now
    }

    fn set_collapsed(&mut self, id: NodeId, collapsed: bool) {
        if self.node(id.0).collapsed != collapsed {
            self.node_mut(id.0).collapsed = collapsed;
            self.dirty = true;
        }
    }

    /// Collapse everything that is neither `id`, nor on the path from the root
    /// to it, nor inside it — so `id` is left holding the whole viewport.
    ///
    /// The collapsed state it replaces is kept whole, panes that were already
    /// collapsed included, and [`unsolo`](Layout::unsolo) puts it back. A solo
    /// while already soloed re-aims without saving again: however many times it
    /// is called, one `unsolo` returns to the arrangement before the first.
    pub fn solo(&mut self, id: NodeId) {
        let a = &mut self.arrangement;
        if !a.soloed {
            for i in 0..a.nodes.len() {
                a.saved[i] = a.nodes[i].collapsed;
            }
            a.soloed = true;
        }
        for i in 0..a.nodes.len() {
            a.nodes[i].collapsed = !a.on_solo_path(i, id);
        }
        self.dirty = true;
    }

    /// Restore exactly the collapsed state [`solo`](Layout::solo) replaced.
    /// A no-op when nothing is soloed.
    pub fn unsolo(&mut self) {
        let a = &mut self.arrangement;
        if !a.soloed {
            return;
        }
        for i in 0..a.nodes.len() {
            a.nodes[i].collapsed = a.saved[i];
        }
        a.soloed = false;
        self.dirty = true;
    }

    /// Move the boundary between visible children `index` and `index + 1` of
    /// `split` to `position`, an **absolute coordinate along the split's
    /// axis**, and return where it actually landed.
    ///
    /// Absolute rather than a delta on purpose. A pointer drag that runs past a
    /// stop and comes back would accumulate drift under deltas — every frame
    /// past the stop contributes a difference the clamp throws away, and the
    /// pointer comes back to find the boundary somewhere it was never dragged
    /// to. With an absolute position there is nothing to accumulate: the same
    /// pointer coordinate always names the same boundary position.
    ///
    /// **A drag stops at the first constraint and never pushes through to a
    /// further neighbour.** Only the two children either side of the boundary
    /// change; their combined extent is what it was.
    ///
    /// The boundary is the *far edge of child `index`* — the near edge of the
    /// divider drawn between them, which is also where [`hit`](Layout::hit)
    /// centres that divider's grab area.
    ///
    /// **This is an edit, and it edits what is on screen.** It reads the
    /// solved rectangles, so a drag made while the viewport is too small to
    /// show the pair — which a pointer cannot do, since
    /// [`hit`](Layout::hit) finds no divider there, but a keyboard or a remote
    /// caller can — writes the sizes that viewport implies. That is not the
    /// viewport writing back into the model; it is a caller asking for a
    /// boundary to be somewhere, at a moment when everywhere is the same
    /// place.
    pub fn set_divider(&mut self, split: NodeId, index: usize, position: f32) -> f32 {
        self.solve();
        let (axis, _) = match self.arrangement.split_of(split.0) {
            Some(s) => s,
            None => return position,
        };
        let (a, b) = match (
            self.arrangement.visible_child(split.0, index),
            self.arrangement.visible_child(split.0, index + 1),
        ) {
            (Some(a), Some(b)) => (a, b),
            _ => return position,
        };

        let start = axis.origin(self.solved.rects[a]);
        let span = axis.extent(self.solved.rects[a]) + axis.extent(self.solved.rects[b]);

        // The pair's own bounds, and the pair's share of the split, are the
        // whole of what constrains this. Nothing beyond `b` is consulted,
        // because nothing beyond `b` moves.
        let lo = self.node(a).min.max(span - self.node(b).max).max(0.0);
        let hi = self.node(a).max.min(span - self.node(b).min).max(lo);
        let size_a = (position - start).clamp(lo, hi);
        let size_b = span - size_a;

        self.resize_pair(split.0, a, b, size_a, size_b, span);
        self.dirty = true;
        self.solve();
        axis.origin(self.solved.rects[a]) + axis.extent(self.solved.rects[a])
    }

    /// Write a drag's two sizes into the model. This is an explicit operation,
    /// so it may write; a solve may not.
    ///
    /// A [`Sizing::Fixed`] child simply stores the new size. A
    /// [`Sizing::Flex`] one stores a weight chosen so the next solve reproduces
    /// that size exactly, which is what makes a drag out and back land where it
    /// started rather than a little off each time.
    fn resize_pair(&mut self, split: usize, a: usize, b: usize, sa: f32, sb: f32, span: f32) {
        match (self.node(a).sizing, self.node(b).sizing) {
            (Sizing::Fixed(_), Sizing::Fixed(_)) => {
                self.node_mut(a).sizing = Sizing::Fixed(sa);
                self.node_mut(b).sizing = Sizing::Fixed(sb);
            }
            // Two flexible neighbours: hold the pair's total weight and split
            // it in the new proportion. The pool and the split's total weight
            // are then both unchanged, so the pair keeps its combined extent
            // and divides it as dragged — exactly, and with no reference to
            // any sibling.
            (Sizing::Flex(wa), Sizing::Flex(wb)) => {
                let total = wa.max(0.0) + wb.max(0.0);
                if span > f32::EPSILON && total > 0.0 {
                    self.node_mut(a).sizing = Sizing::Flex(total * sa / span);
                    self.node_mut(b).sizing = Sizing::Flex(total * sb / span);
                }
            }
            (Sizing::Flex(_), Sizing::Fixed(_)) => {
                self.node_mut(b).sizing = Sizing::Fixed(sb);
                self.reweight(split, a, sa);
            }
            (Sizing::Fixed(_), Sizing::Flex(_)) => {
                self.node_mut(a).sizing = Sizing::Fixed(sa);
                self.reweight(split, b, sb);
            }
        }
    }

    /// Give flexible child `c` the weight that makes it solve to `size`, given
    /// what its siblings now claim.
    ///
    /// Its siblings' flexible weights total `others` and share the pool with
    /// it, so `size = pool * w / (others + w)` inverts to the line below. With
    /// no flexible sibling the child takes the whole pool whatever its weight
    /// is, and the weight is left alone.
    fn reweight(&mut self, split: usize, c: usize, size: f32) {
        let Some((axis, _)) = self.arrangement.split_of(split) else {
            return;
        };
        let avail = self
            .arrangement
            .avail(split, axis.extent(self.solved.rects[split]));
        let mut fixed = 0.0;
        let mut others = 0.0;
        for k in 0..self.arrangement.child_count(split) {
            let child = self.arrangement.child(split, k);
            if self.node(child).collapsed {
                continue;
            }
            match self.node(child).sizing {
                Sizing::Fixed(s) => fixed += s.max(0.0),
                Sizing::Flex(w) if child != c => others += w.max(0.0),
                Sizing::Flex(_) => {}
            }
        }
        let pool = avail - fixed;
        if others > 0.0 && pool - size > f32::EPSILON {
            self.node_mut(c).sizing = Sizing::Flex(others * size / (pool - size));
        }
    }

    // -- the solve -------------------------------------------------------

    /// Recompute every rectangle, if anything has changed since the last one.
    ///
    /// Allocates nothing: the rectangle buffer and the per-split scratch were
    /// sized when the layout was built. Clean is a flag test, so calling this
    /// once a frame costs nothing on a frame where nothing moved.
    ///
    /// One split, given the extent it has along its axis:
    ///
    /// 1. Collapsed children take **zero** extent. There is no handle strip —
    ///    re-opening a pane is a named operation, not a mouse target.
    /// 2. Dividers sit only *between visible children*, so what is left to
    ///    distribute is `extent - divider * (visible - 1)`, floored at zero.
    /// 3. [`Sizing::Fixed`] children claim their stored size; [`Sizing::Flex`]
    ///    children share what is left in proportion to their weights.
    /// 4. Each child is clamped to its `[min, max]`. Clamping changes the
    ///    total, so this iterates: whoever hit a bound freezes there and the
    ///    remainder is redistributed among the rest. Each pass freezes at least
    ///    one child, so it terminates. When nothing flexible is left unfrozen
    ///    and there is still a discrepancy, the fixed children take it in
    ///    proportion — which is also why a split with no flexible child at all
    ///    still tiles its parent rather than leaving a gap.
    /// 5. If the viewport is smaller than the sum of the minima, everything
    ///    scales down in proportion, floored at zero. A minimum is a
    ///    preference, not a licence to overflow, and **a rectangle is never
    ///    negative.**
    ///
    /// The one case where children do not account for their parent exactly is
    /// the mirror of step 5: where *every* visible child is sitting at its
    /// maximum and there is still room, the remainder is left as empty space
    /// after the last of them. A maximum is honoured rather than overridden —
    /// a pane that says it is never wider than 480 is not made 1280 wide by
    /// being the only one left on screen — so the alternative would be to hand
    /// the space to whichever child the code happened to reach last, which is
    /// an arrangement nobody asked for and no operator can undo.
    ///
    /// **None of it can write a size back into the arrangement**, and that is
    /// structural rather than careful: the two lines below split this layout
    /// into the arrangement and the buffers, and everything past them holds the
    /// arrangement by shared reference.
    pub fn solve(&mut self) {
        if !self.dirty {
            return;
        }
        // The split borrow that makes P-0071 a compiler error. `arrangement`
        // is re-borrowed as `&` here and stays that way for the whole solve,
        // so a line that stored a solved size into a node would not compile.
        let Layout {
            arrangement,
            viewport,
            solved,
            dirty,
        } = self;
        let arrangement: &Arrangement = arrangement;
        solved.rects[arrangement.root.0] = *viewport;
        solve_subtree(arrangement, solved, arrangement.root.0);
        *dirty = false;
    }

    // -- hit testing -----------------------------------------------------

    /// What is under `p`, with a divider's grab area widened by `grab` on each
    /// side.
    ///
    /// **A divider's grab area is wider than the divider it draws.** A boundary
    /// drawn one pixel wide is not a target a hand can find, and widening what
    /// is drawn instead would spend the panel's space on something that is
    /// there to be dragged rather than to be seen. So a point inside the grab
    /// area belongs to the divider, not to the view under it.
    pub fn hit(&self, p: Point, grab: f32) -> Hit {
        debug_assert!(!self.dirty, "hit() read a stale solve; call solve() first");
        if !self.viewport.contains(p) {
            return Hit::Nothing;
        }
        let mut cur = self.arrangement.root.0;
        loop {
            let Some((axis, _)) = self.arrangement.split_of(cur) else {
                return Hit::View(NodeId(cur));
            };
            if let Some(index) = self.divider_at(cur, axis, p, grab) {
                return Hit::Divider {
                    split: NodeId(cur),
                    index,
                };
            }
            let mut next = None;
            for k in 0..self.arrangement.child_count(cur) {
                let c = self.arrangement.child(cur, k);
                if !self.node(c).collapsed && self.solved.rects[c].contains(p) {
                    next = Some(c);
                    break;
                }
            }
            match next {
                Some(c) => cur = c,
                // Inside the split but claimed by no child: only reachable at a
                // rounding-width seam, and a seam is nothing rather than a
                // guess.
                None => return Hit::Nothing,
            }
        }
    }

    /// The index of the divider `p` grabs, if any. The gap is read from the
    /// solved rectangles rather than recomputed, so it is exactly the gap that
    /// was drawn.
    fn divider_at(&self, split: usize, axis: Axis, p: Point, grab: f32) -> Option<usize> {
        let along = axis.coord(p);
        let mut index = 0;
        let mut prev: Option<usize> = None;
        for k in 0..self.arrangement.child_count(split) {
            let c = self.arrangement.child(split, k);
            if self.node(c).collapsed {
                continue;
            }
            if let Some(a) = prev {
                let near = axis.origin(self.solved.rects[a]) + axis.extent(self.solved.rects[a]);
                let far = axis.origin(self.solved.rects[c]);
                if along >= near - grab && along <= far + grab {
                    return Some(index);
                }
                index += 1;
            }
            prev = Some(c);
        }
        None
    }

    // -- arena access ----------------------------------------------------

    fn node(&self, i: usize) -> &Node {
        &self.arrangement.nodes[i]
    }

    /// The only way to write a node, and every caller of it is an operation —
    /// a fold, a solo or a drag. The solve has no access to this, by
    /// construction rather than by convention: see [`Layout::solve`].
    fn node_mut(&mut self, i: usize) -> &mut Node {
        &mut self.arrangement.nodes[i]
    }
}

impl Arrangement {
    /// The axis and declared divider of a split, or `None` for a view. Returns
    /// by value so a caller can hold it across a write to another field.
    fn split_of(&self, i: usize) -> Option<(Axis, f32)> {
        match &self.nodes[i].kind {
            Kind::Split { axis, divider, .. } => Some((*axis, *divider)),
            Kind::View { .. } => None,
        }
    }

    fn child_count(&self, i: usize) -> usize {
        match &self.nodes[i].kind {
            Kind::Split { children, .. } => children.len(),
            Kind::View { .. } => 0,
        }
    }

    fn child(&self, i: usize, k: usize) -> usize {
        match &self.nodes[i].kind {
            Kind::Split { children, .. } => children[k].0,
            Kind::View { .. } => unreachable!("a view has no children"),
        }
    }

    fn visible_count(&self, i: usize) -> usize {
        (0..self.child_count(i))
            .filter(|k| !self.nodes[self.child(i, *k)].collapsed)
            .count()
    }

    /// The `nth` visible child of a split, which is what a divider index and
    /// [`Layout::set_divider`] count in.
    fn visible_child(&self, i: usize, nth: usize) -> Option<usize> {
        (0..self.child_count(i))
            .map(|k| self.child(i, k))
            .filter(|c| !self.nodes[*c].collapsed)
            .nth(nth)
    }

    /// The divider thickness actually used, which is the declared one until the
    /// split is too narrow to hold that many. Shrinking the dividers rather
    /// than overflowing keeps children tiling their parent at every extent,
    /// including zero.
    fn effective_divider(&self, split: usize, extent: f32) -> f32 {
        let Some((_, divider)) = self.split_of(split) else {
            return 0.0;
        };
        let gaps = self.visible_count(split).saturating_sub(1) as f32;
        if gaps > 0.0 && divider * gaps > extent {
            extent / gaps
        } else {
            divider
        }
    }

    /// What is left for the children once the dividers between them are taken
    /// out. Never negative.
    fn avail(&self, split: usize, extent: f32) -> f32 {
        let gaps = self.visible_count(split).saturating_sub(1) as f32;
        (extent - self.effective_divider(split, extent) * gaps).max(0.0)
    }

    /// Whether node `i` survives a solo on `kept`: it is `kept`, an ancestor of
    /// it, or inside it.
    fn on_solo_path(&self, i: usize, kept: NodeId) -> bool {
        self.is_ancestor(i, kept.0) || self.is_ancestor(kept.0, i)
    }

    /// Whether `a` is `b` or an ancestor of it.
    fn is_ancestor(&self, a: usize, b: usize) -> bool {
        let mut cur = Some(NodeId(b));
        while let Some(NodeId(i)) = cur {
            if i == a {
                return true;
            }
            cur = self.nodes[i].parent;
        }
        false
    }
}

/// The solve, as a function of the arrangement rather than a method on it.
///
/// `a` is shared and `s` is exclusive, which is the whole enforcement of
/// P-0071: there is no path from here to a mutable node.
fn solve_subtree(a: &Arrangement, s: &mut Solved, i: usize) {
    if a.split_of(i).is_none() {
        return;
    }
    solve_split(a, s, i);
    // The scratch is finished with by now, which is what lets one buffer
    // serve the whole recursion.
    for k in 0..a.child_count(i) {
        let c = a.child(i, k);
        solve_subtree(a, s, c);
    }
}

fn solve_split(a: &Arrangement, s: &mut Solved, split: usize) {
    let Some((axis, _)) = a.split_of(split) else {
        return;
    };
    let rect = s.rects[split];
    let extent = axis.extent(rect).max(0.0);
    let n = a.child_count(split);
    let divider = a.effective_divider(split, extent);
    let avail = a.avail(split, extent);

    for k in 0..n {
        let c = a.child(split, k);
        s.sizes[k] = 0.0;
        s.frozen[k] = a.nodes[c].collapsed;
    }

    // Step 4. One pass per child is enough, since every pass but the last
    // freezes one; the `+ 1` is the pass that settles and breaks.
    for _ in 0..=n {
        let mut frozen_sum = 0.0;
        let mut fixed_sum = 0.0;
        let mut weight = 0.0;
        for k in 0..n {
            if s.frozen[k] {
                frozen_sum += s.sizes[k];
                continue;
            }
            match a.nodes[a.child(split, k)].sizing {
                Sizing::Fixed(size) => fixed_sum += size.max(0.0),
                Sizing::Flex(w) => weight += w.max(0.0),
            }
        }

        if weight > 0.0 {
            let pool = (avail - frozen_sum - fixed_sum).max(0.0);
            for k in 0..n {
                if s.frozen[k] {
                    continue;
                }
                s.sizes[k] = match a.nodes[a.child(split, k)].sizing {
                    Sizing::Fixed(size) => size.max(0.0),
                    Sizing::Flex(w) => pool * w.max(0.0) / weight,
                };
            }
        } else {
            // Nothing flexible is left unfrozen, so the fixed children take
            // the discrepancy in proportion — in both directions, because a
            // split that came up short would otherwise leave a gap its
            // parent has no other child to fill.
            let pool = (avail - frozen_sum).max(0.0);
            let scale = if fixed_sum > 0.0 {
                pool / fixed_sum
            } else {
                0.0
            };
            for k in 0..n {
                if s.frozen[k] {
                    continue;
                }
                s.sizes[k] = match a.nodes[a.child(split, k)].sizing {
                    Sizing::Fixed(size) => size.max(0.0) * scale,
                    Sizing::Flex(_) => 0.0,
                };
            }
        }

        let mut bounded = false;
        for k in 0..n {
            if s.frozen[k] {
                continue;
            }
            let c = a.child(split, k);
            let min = a.nodes[c].min.max(0.0);
            let max = a.nodes[c].max;
            if s.sizes[k] < min {
                s.sizes[k] = min;
                s.frozen[k] = true;
                bounded = true;
            } else if s.sizes[k] > max {
                s.sizes[k] = max;
                s.frozen[k] = true;
                bounded = true;
            }
        }
        if !bounded {
            break;
        }
    }

    // Step 5.
    let mut total = 0.0;
    for k in 0..n {
        total += s.sizes[k];
    }
    if total > avail {
        let scale = if total > 0.0 { avail / total } else { 0.0 };
        for k in 0..n {
            s.sizes[k] *= scale;
        }
    }

    let mut cursor = axis.origin(rect);
    let mut placed = 0;
    let visible = a.visible_count(split);
    for k in 0..n {
        let c = a.child(split, k);
        s.rects[c] = axis.slice(rect, cursor, s.sizes[k].max(0.0));
        cursor += s.sizes[k].max(0.0);
        if !a.nodes[c].collapsed {
            placed += 1;
            if placed < visible {
                cursor += divider;
            }
        }
    }
}

/// Flatten a [`Spec`] into the arena, parent before children so that a name
/// resolves in declaration order — which matters only for the message
/// [`Layout::new`] refuses a duplicate with, since after that check there is at
/// most one node per name.
fn build(nodes: &mut Vec<Node>, spec: Spec, parent: Option<NodeId>) -> NodeId {
    let (kind, sizing, min, max, collapsed, children) = match spec {
        Spec::View {
            name,
            sizing,
            min,
            max,
            collapsed,
        } => (Kind::View { name }, sizing, min, max, collapsed, Vec::new()),
        Spec::Split {
            name,
            axis,
            divider,
            children,
            sizing,
            min,
            max,
            collapsed,
        } => (
            Kind::Split {
                name,
                axis,
                divider,
                children: Vec::new(),
            },
            sizing,
            min,
            max,
            collapsed,
            children,
        ),
    };
    let id = NodeId(nodes.len());
    nodes.push(Node {
        kind,
        sizing,
        min,
        max,
        collapsed,
        parent,
    });
    let built: Vec<NodeId> = children
        .into_iter()
        .map(|c| build(nodes, c, Some(id)))
        .collect();
    if let Kind::Split { children, .. } = &mut nodes[id.0].kind {
        *children = built;
    }
    id
}
