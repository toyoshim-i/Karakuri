use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::spec::Spec;
use crate::{Axis, Rect, Sizing};

/// A handle into a [`Layout`]'s arena.
///
/// Opaque on purpose: a name is resolved once by [`Layout::find`], at build or
/// load time, and everything on the frame path carries the id. A name lookup
/// per frame per region is a string comparison the panel never has to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub(crate) usize);

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
pub(crate) enum Kind {
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
pub(crate) struct Node {
    pub(crate) kind: Kind,
    pub(crate) sizing: Sizing,
    pub(crate) min: f32,
    /// `f32::INFINITY` means unbounded, and is written as `null` rather than as
    /// a number — see [`unbounded`].
    #[serde(with = "unbounded")]
    pub(crate) max: f32,
    /// Folded by the operator: what [`Layout::collapse`] writes, what a
    /// [`Spec`] can start a node with, and what a saved arrangement carries.
    pub(crate) collapsed: bool,
    /// Indicates whether a fold on this node preserves its divider edge rather
    /// than removing it from its parent's layout.
    ///
    /// Corresponds to [`Spec::keeps_its_edge`] and [`Layout::is_closed`]. This
    /// property is structural and serialized alongside bounds constraints.
    #[serde(default)]
    pub(crate) edge: bool,
    /// Set aside by whoever is drawing, because it has put that region
    /// somewhere else or has nowhere to put it.
    ///
    /// Ephemeral state skipped during serialization. Derived dynamically from
    /// caller geometry on the next solve rather than persisted across sessions.
    #[serde(skip)]
    pub(crate) aside: bool,
    pub(crate) parent: Option<NodeId>,
}

impl Node {
    /// The name this node answers to, if it has one.
    pub(crate) fn name(&self) -> Option<&str> {
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

/// The immutable arrangement definition, separate from derived layout solutions.
///
/// Kept distinct from solved layout state so that solver routines borrow
/// `&Arrangement` immutably, guaranteeing that solving never mutates model nodes.
#[derive(Debug, Clone)]
pub(crate) struct Arrangement {
    pub(crate) nodes: Vec<Node>,
    pub(crate) root: NodeId,
    /// What [`Layout::solo`] is holding, and the collapsed flags it replaced.
    /// Both are saved: an arrangement stored while soloed comes back soloed,
    /// and [`Layout::unsolo`] still has something to restore.
    ///
    /// The node rather than a flag, because the flags a solo leaves behind do
    /// not identify it — see [`Layout::soloed`].
    pub(crate) soloed: Option<NodeId>,
    pub(crate) saved: Vec<bool>,
}

impl Arrangement {
    /// The axis and declared divider of a split, or `None` for a view. Returns
    /// by value so a caller can hold it across a write to another field.
    pub(crate) fn split_of(&self, i: usize) -> Option<(Axis, f32)> {
        match &self.nodes[i].kind {
            Kind::Split { axis, divider, .. } => Some((*axis, *divider)),
            Kind::View { .. } => None,
        }
    }

    pub(crate) fn child_count(&self, i: usize) -> usize {
        match &self.nodes[i].kind {
            Kind::Split { children, .. } => children.len(),
            Kind::View { .. } => 0,
        }
    }

    pub(crate) fn child(&self, i: usize, k: usize) -> usize {
        match &self.nodes[i].kind {
            Kind::Split { children, .. } => children[k].0,
            Kind::View { .. } => unreachable!("a view has no children"),
        }
    }

    /// Returns true if node `i` is omitted from layout (either collapsed or set aside).
    pub(crate) fn out_of_layout(&self, i: usize) -> bool {
        self.nodes[i].collapsed || self.nodes[i].aside
    }

    /// Returns true if node `i` is collapsed with [`Spec::keeps_its_edge`] and
    /// no solo is active, preserving its divider edge.
    pub(crate) fn is_closed(&self, i: usize) -> bool {
        self.nodes[i].collapsed && self.nodes[i].edge && self.soloed.is_none()
    }

    /// Returns true if node `i` is placed in its parent's layout line (non-collapsed,
    /// or closed while keeping its edge).
    pub(crate) fn placed(&self, i: usize) -> bool {
        !self.nodes[i].aside && (!self.nodes[i].collapsed || self.is_closed(i))
    }

    pub(crate) fn placed_count(&self, i: usize) -> usize {
        (0..self.child_count(i))
            .filter(|k| self.placed(self.child(i, *k)))
            .count()
    }

    /// The `nth` placed child of a split, which is what a divider index and
    /// [`Layout::set_divider`] count in.
    pub(crate) fn placed_child(&self, i: usize, nth: usize) -> Option<usize> {
        (0..self.child_count(i))
            .map(|k| self.child(i, k))
            .filter(|c| self.placed(*c))
            .nth(nth)
    }

    /// The divider thickness actually used, which is the declared one until the
    /// split is too narrow to hold that many. Shrinking the dividers rather
    /// than overflowing keeps children tiling their parent at every extent,
    /// including zero.
    pub(crate) fn effective_divider(&self, split: usize, extent: f32) -> f32 {
        let Some((_, divider)) = self.split_of(split) else {
            return 0.0;
        };
        let gaps = self.placed_count(split).saturating_sub(1) as f32;
        if gaps > 0.0 && divider * gaps > extent {
            extent / gaps
        } else {
            divider
        }
    }

    /// What is left for the children once the dividers between them are taken
    /// out. Never negative.
    pub(crate) fn avail(&self, split: usize, extent: f32) -> f32 {
        let gaps = self.placed_count(split).saturating_sub(1) as f32;
        (extent - self.effective_divider(split, extent) * gaps).max(0.0)
    }

    /// Whether node `i` survives a solo on `kept`: it is `kept`, an ancestor of
    /// it, or inside it.
    pub(crate) fn on_solo_path(&self, i: usize, kept: NodeId) -> bool {
        self.is_ancestor(i, kept.0) || self.is_ancestor(kept.0, i)
    }

    /// Whether `a` is `b` or an ancestor of it.
    pub(crate) fn is_ancestor(&self, a: usize, b: usize) -> bool {
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

/// What a solve writes, and the only thing it writes.
///
/// Sized once at build, so a solved frame allocates nothing.
#[derive(Debug, Clone)]
pub(crate) struct Solved {
    /// Parallel to the arena's nodes. The solve writes here and
    /// [`Layout::rect`] reads here.
    pub(crate) rects: Vec<Rect>,
    /// Scratch for one split's children, sized to the whole arena so it fits
    /// any split. A split's sizes are finished before the solve descends into
    /// any child, so one buffer serves the whole recursion.
    pub(crate) sizes: Vec<f32>,
    pub(crate) frozen: Vec<bool>,
    /// Usable extent per node along its parent's axis, computed bottom-up by
    /// [`measure`] at the start of each solve.
    ///
    /// Stored in scratch buffers rather than on nodes to preserve immutability
    /// of the arrangement during layout passes.
    pub(crate) usable: Vec<f32>,
}

/// Flatten a [`Spec`] into the arena, parent before children so that a name
/// resolves in declaration order — which matters only for the message
/// [`Layout::new`] refuses a duplicate with, since after that check there is at
/// most one node per name.
pub(crate) fn build(nodes: &mut Vec<Node>, spec: Spec, parent: Option<NodeId>) -> NodeId {
    let (kind, sizing, min, max, collapsed, edge, children) = match spec {
        Spec::View {
            name,
            sizing,
            min,
            max,
            collapsed,
            edge,
        } => (
            Kind::View { name },
            sizing,
            min,
            max,
            collapsed,
            edge,
            Vec::new(),
        ),
        Spec::Split {
            name,
            axis,
            divider,
            children,
            sizing,
            min,
            max,
            collapsed,
            edge,
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
            edge,
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
        edge,
        // Nothing a `Spec` can say sets this: it is not the arrangement's, it
        // is the caller's, and it is stated per frame rather than declared.
        aside: false,
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
