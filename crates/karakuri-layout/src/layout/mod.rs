//! Layout arena, solver, and operations.
//! Separates [`Arrangement`] model storage from solved [`Solved`] layout state.

mod solver;
mod types;
mod wire;

use serde::Deserialize;

use solver::{measure, solve_subtree};
use types::{build, Arrangement, Kind, Node, Solved};
pub use types::{Hit, NodeId};
use wire::{duplicate_name, sane, LoadError, Wire};

use crate::spec::Spec;
use crate::{Axis, Point, Rect, Sizing};

/// An arrangement of regions, and the rectangles it currently solves to.
/// Excludes transient states (`set_aside`) from serialization.
#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "Wire")]
pub struct Layout {
    pub(crate) arrangement: Arrangement,
    pub(crate) viewport: Rect,
    pub(crate) solved: Solved,
    pub(crate) dirty: bool,
}

impl Layout {
    /// Builds an arrangement from its declarative form.
    ///
    /// # Panics
    /// Panics if node names are not globally unique across views and splits.
    pub fn new(spec: Spec) -> Layout {
        let mut nodes = Vec::new();
        let root = build(&mut nodes, spec, None);
        if let Some(name) = duplicate_name(&nodes) {
            panic!(
                "{}",
                LoadError::DuplicateName {
                    name: name.to_owned()
                }
            );
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
                soloed: None,
                saved,
            },
            viewport: Rect::new(0.0, 0.0, 0.0, 0.0),
            solved: Solved {
                rects: vec![Rect::new(0.0, 0.0, 0.0, 0.0); n],
                sizes: vec![0.0; n],
                frozen: vec![false; n],
                usable: vec![0.0; n],
            },
            dirty: true,
        }
    }

    /// The root, which always fills the viewport exactly. Its own [`Sizing`],
    /// `min` and `max` are ignored — there is nothing to be relative to.
    pub fn root(&self) -> NodeId {
        self.arrangement.root
    }

    /// Resolves a node name to its [`NodeId`].
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

    /// Returns a split's children in declaration order, or an empty slice for leaf views.
    pub fn children(&self, id: NodeId) -> &[NodeId] {
        match &self.node(id.0).kind {
            Kind::Split { children, .. } => children,
            Kind::View { .. } => &[],
        }
    }

    /// Returns an iterator over placed children of `id` in layout order.
    /// Addresses non-collapsed nodes and closed nodes keeping their edge.
    pub fn placed_children(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.children(id)
            .iter()
            .copied()
            .filter(|c| self.arrangement.placed(c.0))
    }

    /// Returns the split `id` hangs from, or `None` for the root.
    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.node(id.0).parent
    }

    /// A split's axis, or `None` for a view.
    pub fn axis(&self, id: NodeId) -> Option<Axis> {
        self.arrangement.split_of(id.0).map(|(axis, _)| axis)
    }

    /// Returns true if `id` is a leaf view.
    pub fn is_view(&self, id: NodeId) -> bool {
        self.arrangement.split_of(id.0).is_none()
    }

    /// The divider thickness a split declares. What is actually drawn is this
    /// until the split is too narrow to hold that many, at which point the
    /// dividers shrink with it rather than the children overflowing.
    pub fn divider(&self, id: NodeId) -> Option<f32> {
        self.arrangement.split_of(id.0).map(|(_, divider)| divider)
    }

    /// Returns `[min, max]` size constraints of `id` along its parent's axis.
    ///
    /// Unbounded maxima are represented as `f32::INFINITY`.
    pub fn bounds(&self, id: NodeId) -> (f32, f32) {
        (self.node(id.0).min, self.node(id.0).max)
    }

    /// Returns the [`Sizing`] rule for `id` along its parent's axis.
    pub fn sizing(&self, id: NodeId) -> Sizing {
        self.node(id.0).sizing
    }

    /// The viewport as it was last set.
    pub fn viewport(&self) -> Rect {
        self.viewport
    }

    /// Sets the viewport extent for solving layout.
    ///
    /// Does not mutate stored node sizes. Marks the layout as dirty.
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

    /// Returns true if `id` and all its ancestors are currently in layout.
    ///
    /// Returns false if `id` or any ancestor is collapsed or set aside.
    pub fn visible(&self, id: NodeId) -> bool {
        let mut cur = Some(id);
        while let Some(NodeId(i)) = cur {
            if self.arrangement.out_of_layout(i) {
                return false;
            }
            cur = self.node(i).parent;
        }
        true
    }

    /// Returns true if `id` itself is collapsed by the operator, regardless of
    /// ancestor visibility or `set_aside` status.
    pub fn is_collapsed(&self, id: NodeId) -> bool {
        self.node(id.0).collapsed
    }

    /// Returns true if `id` is currently set aside via [`set_aside`](Layout::set_aside).
    pub fn is_set_aside(&self, id: NodeId) -> bool {
        self.node(id.0).aside
    }

    /// Returns true if `id` is placed in its parent's tiling layout (non-collapsed,
    /// or closed while preserving its edge).
    pub fn is_placed(&self, id: NodeId) -> bool {
        self.arrangement.placed(id.0)
    }

    /// Returns true if `id` is collapsed while preserving its divider edge (and no solo active).
    pub fn is_closed(&self, id: NodeId) -> bool {
        self.arrangement.is_closed(id.0)
    }

    /// Returns true if `id` is configured to preserve its divider edge when collapsed.
    pub fn keeps_its_edge(&self, id: NodeId) -> bool {
        self.node(id.0).edge
    }

    /// Returns the active solo target node, if one is currently in effect.
    pub fn soloed(&self) -> Option<NodeId> {
        self.arrangement.soloed
    }

    /// Whether a [`solo`](Layout::solo) is in force. The yes-or-no of
    /// [`soloed`](Layout::soloed), which is what an operation that undoes one
    /// asks.
    pub fn is_soloed(&self) -> bool {
        self.arrangement.soloed.is_some()
    }

    // -- operations ------------------------------------------------------

    /// Collapses `id`, removing its visible extent while preserving stored size for expansion.
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

    /// Sets or clears the transient `aside` flag for `id` (excluded from serialization).
    pub fn set_aside(&mut self, id: NodeId, aside: bool) {
        if self.node(id.0).aside != aside {
            self.node_mut(id.0).aside = aside;
            self.dirty = true;
        }
    }

    /// Isolates `id` to occupy the full viewport by collapsing all sibling subtrees.
    ///
    /// Preserves prior collapsed states so [`unsolo`](Layout::unsolo) can restore
    /// them exactly. Does not alter transient `aside` states.
    pub fn solo(&mut self, id: NodeId) {
        let a = &mut self.arrangement;
        if a.soloed.is_none() {
            for i in 0..a.nodes.len() {
                a.saved[i] = a.nodes[i].collapsed;
            }
        }
        a.soloed = Some(id);
        for i in 0..a.nodes.len() {
            a.nodes[i].collapsed = !a.on_solo_path(i, id);
        }
        self.dirty = true;
    }

    /// Restore exactly the collapsed state [`solo`](Layout::solo) replaced.
    /// A no-op when nothing is soloed.
    pub fn unsolo(&mut self) {
        let a = &mut self.arrangement;
        if a.soloed.is_none() {
            return;
        }
        for i in 0..a.nodes.len() {
            a.nodes[i].collapsed = a.saved[i];
        }
        a.soloed = None;
        self.dirty = true;
    }

    /// Moves the boundary between placed children `index` and `index + 1` of `split` to `position`.
    /// Returns the clamped coordinate where the divider actually settled.
    pub fn set_divider(&mut self, split: NodeId, index: usize, position: f32) -> f32 {
        self.solve();
        let (axis, _) = match self.arrangement.split_of(split.0) {
            Some(s) => s,
            None => return position,
        };
        let (a, b) = match (
            self.arrangement.placed_child(split.0, index),
            self.arrangement.placed_child(split.0, index + 1),
        ) {
            (Some(a), Some(b)) => (a, b),
            _ => return position,
        };

        // A boundary adjacent to a closed node cannot be dragged. Closed nodes
        // maintain zero extent and require explicit reopening.
        if self.arrangement.is_closed(a) || self.arrangement.is_closed(b) {
            return axis.far(self.solved.rects[a]);
        }

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
        axis.far(self.solved.rects[a])
    }

    /// Writes resized sizes into child nodes following a divider drag.
    ///
    /// Fixed children store the size directly; flexible children adjust weights
    /// to preserve proportion on subsequent solves.
    fn resize_pair(&mut self, split: usize, a: usize, b: usize, sa: f32, sb: f32, span: f32) {
        match (self.node(a).sizing, self.node(b).sizing) {
            (Sizing::Fixed(_), Sizing::Fixed(_)) => {
                self.node_mut(a).sizing = Sizing::Fixed(sa);
                self.node_mut(b).sizing = Sizing::Fixed(sb);
            }
            // Distribute total flexible weight between the two neighbours proportionally.
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

    /// Gives flexible child `c` the weight that solves to `size`, given remaining sibling claims.
    ///
    /// Fixed siblings are accounted at stored size. Recomputes flexible weight
    /// proportionally from available pool.
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
            if self.arrangement.out_of_layout(child) {
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

    /// Recomputes region rectangles if dirty flags indicate modifications.
    /// Uses bottom-up measurement followed by top-down recursive subtree solving.
    pub fn solve(&mut self) {
        if !self.dirty {
            return;
        }
        // Destructure to borrow `arrangement` immutably alongside mutable scratch in `solved`.
        let Layout {
            arrangement,
            viewport,
            solved,
            dirty,
        } = self;
        let arrangement: &Arrangement = arrangement;
        // Compute usable extents bottom-up across all nodes before assigning rectangles top-down.
        measure(arrangement, solved, arrangement.root.0, None);
        solved.rects[arrangement.root.0] = *viewport;
        solve_subtree(arrangement, solved, arrangement.root.0);
        *dirty = false;
    }

    // -- boundaries ------------------------------------------------------

    /// Returns the rectangle for the divider gap between placed children `index` and `index + 1` of `split`.
    ///
    /// Returns `None` if `split` is a leaf view or if either child in the pair is absent.
    pub fn boundary(&self, split: NodeId, index: usize) -> Option<Rect> {
        debug_assert!(
            !self.dirty,
            "boundary() read a stale solve; call solve() first"
        );
        let axis = self.axis(split)?;
        let before = self.arrangement.placed_child(split.0, index)?;
        let after = self.arrangement.placed_child(split.0, index + 1)?;
        let start = axis.far(self.solved.rects[before]);
        let size = (axis.origin(self.solved.rects[after]) - start).max(0.0);
        Some(axis.slice(self.solved.rects[split.0], start, size))
    }

    /// Returns an iterator over all boundary `(split, index)` pairs in traversal order.
    ///
    /// Does not allocate or require layout resolution. Folded boundaries yield zero extent.
    pub fn boundaries(&self) -> impl Iterator<Item = (NodeId, usize)> + '_ {
        (0..self.arrangement.nodes.len()).flat_map(move |i| {
            (0..self.arrangement.placed_count(i).saturating_sub(1)).map(move |k| (NodeId(i), k))
        })
    }

    // -- hit testing -----------------------------------------------------

    /// Hit-tests point `p` against the layout, applying `grab` margin around dividers.
    pub fn hit(&self, p: Point, grab: f32) -> Hit {
        debug_assert!(!self.dirty, "hit() read a stale solve; call solve() first");
        if !self.viewport.contains(p) {
            return Hit::Nothing;
        }
        // Only the root: every other node is reached through the filter in the
        // descent, so this is the whole of *the node reported is laid out*.
        if self.arrangement.out_of_layout(self.arrangement.root.0) {
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
                if !self.arrangement.out_of_layout(c) && self.solved.rects[c].contains(p) {
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
            // The children the split tiles, not the ones it draws: a closed
            // node is placed at zero extent and the gap beside it is the
            // boundary this is looking for.
            if !self.arrangement.placed(c) {
                continue;
            }
            if let Some(a) = prev {
                let near = axis.far(self.solved.rects[a]);
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
