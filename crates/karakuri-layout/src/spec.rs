//! The declarative form an arrangement is written in.
//!
//! A [`Spec`] is a nested tree; a [`Layout`](crate::Layout) is the flat arena
//! it builds into. Tests and the panel's default arrangement are written here
//! because a nested literal is readable in a way an arena of parent pointers is
//! not.
//!
//! **`Spec` is deliberately not serialisable.** Saving an operator's
//! arrangement is `serde` on the whole `Layout`, which already carries the
//! sizes a drag produced and the flags a collapse set; a second serialisable
//! tree would be a second on-disk format for the same thing, and the two would
//! disagree the first time one of them grew a field.

use crate::{Axis, LayoutSplit, Sizing};

/// One node of an arrangement, before it is built.
///
/// The constraints on a node — [`Sizing`], `min`, `max`, `collapsed`,
/// [`keeps_its_edge`](Spec::keeps_its_edge) — apply along its *parent's* axis,
/// so they are set on the child rather than by the parent that arranges it.
/// The root's are ignored: the root is the viewport.
#[derive(Debug, Clone, PartialEq)]
pub enum Spec {
    /// A leaf. Whatever draws the panel looks `name` up; nothing here knows
    /// what is inside one, which is why a view's name is required: it is what
    /// says what to draw.
    View {
        name: String,
        sizing: Sizing,
        min: f32,
        max: f32,
        collapsed: bool,
        edge: bool,
    },
    /// An axis, a divider thickness, and children laid out along it.
    ///
    /// **A split may carry a name too, and most do not.** A region that can be
    /// operated has to be addressable by name — the console's left pane *is* a
    /// split, and "fold the left pane away" is an operation the keyboard, a
    /// MIDI map and MCP all reach by that name. The rest are structure nobody
    /// addresses, so the name is a [`named`](Spec::named) away rather than a
    /// requirement.
    Split {
        name: Option<String>,
        split_identity: Option<LayoutSplit>,
        axis: Axis,
        divider: f32,
        children: Vec<Spec>,
        sizing: Sizing,
        min: f32,
        max: f32,
        collapsed: bool,
        edge: bool,
    },
}

impl Spec {
    /// A named leaf. Flexible with weight 1, unconstrained, open — the
    /// defaults a caller does not have to say anything to get.
    pub fn view(name: impl Into<String>) -> Spec {
        Spec::View {
            name: name.into(),
            sizing: Sizing::Flex(1.0),
            min: 0.0,
            max: f32::INFINITY,
            collapsed: false,
            edge: false,
        }
    }

    /// A split along `axis`, with `divider` thick boundaries between its
    /// visible children.
    pub fn split(axis: Axis, divider: f32, children: Vec<Spec>) -> Spec {
        Spec::Split {
            name: None,
            split_identity: None,
            axis,
            divider,
            children,
            sizing: Sizing::Flex(1.0),
            min: 0.0,
            max: f32::INFINITY,
            collapsed: false,
            edge: false,
        }
    }

    /// Children left to right.
    pub fn row(divider: f32, children: Vec<Spec>) -> Spec {
        Spec::split(Axis::Row, divider, children)
    }

    /// Children top to bottom.
    pub fn column(divider: f32, children: Vec<Spec>) -> Spec {
        Spec::split(Axis::Column, divider, children)
    }

    /// Name this node, so [`Layout::find`](crate::Layout::find) resolves it and
    /// every surface can address it.
    ///
    /// On a split this gives it the name it did not have; on a view it replaces
    /// the one it was built with. **A name is used once in an arrangement** —
    /// [`Layout::new`](crate::Layout::new) refuses one that is not.
    pub fn named(self, name: impl Into<String>) -> Spec {
        let mut spec = self;
        let s = name.into();
        match &mut spec {
            Spec::View { name: n, .. } => *n = s,
            Spec::Split {
                name: n,
                split_identity,
                ..
            } => {
                *n = Some(s.clone());
                if split_identity.is_none() {
                    *split_identity = Some(LayoutSplit::Named(s));
                }
            }
        }
        spec
    }

    /// Assign a typed split identity, allowing unnamed or named splits
    /// to be referenced by [`LayoutSplit`].
    pub fn split_identity(self, identity: LayoutSplit) -> Spec {
        let mut spec = self;
        if let Spec::Split { split_identity, .. } = &mut spec {
            *split_identity = Some(identity);
        }
        spec
    }

    /// Name a split and assign a typed [`LayoutSplit::Named`] identity.
    pub fn named_split(self, name: impl Into<String>) -> Spec {
        self.named(name)
    }

    /// Claim `size` along the parent's axis and keep it when the viewport
    /// changes.
    pub fn fixed(self, size: f32) -> Spec {
        self.with(|sizing, _, _, _| *sizing = Sizing::Fixed(size))
    }

    /// Absorb what is left, in proportion to `weight`.
    pub fn flex(self, weight: f32) -> Spec {
        self.with(|sizing, _, _, _| *sizing = Sizing::Flex(weight))
    }

    /// The smallest extent this node is given while the viewport can afford
    /// it. Below the sum of the minima everything scales down together — a
    /// minimum is not a floor the layout may overflow to honour.
    pub fn min(self, min: f32) -> Spec {
        self.with(|_, m, _, _| *m = min)
    }

    /// The largest extent this node is given. `f32::INFINITY` is the default
    /// and means unbounded.
    pub fn max(self, max: f32) -> Spec {
        self.with(|_, _, m, _| *m = max)
    }

    /// Start collapsed — folded to zero extent, and out of its parent's
    /// layout unless it also [`keeps_its_edge`](Spec::keeps_its_edge).
    pub fn collapsed(self) -> Spec {
        self.with(|_, _, _, c| *c = true)
    }

    /// **A fold on this node leaves its edge behind.**
    ///
    /// A node that is folded normally leaves its parent's layout entirely: no
    /// extent, and **no divider beside it**, so nothing about it is on screen
    /// and the only way back is
    /// [`expand`](crate::Layout::expand). A node that keeps its edge is folded
    /// to zero extent and stays one of the children its parent tiles, so the
    /// divider beside it is still drawn and still
    /// [`hit`](crate::Layout::hit)-testable — which is a **pointer route back
    /// in**, on a region that has no rectangle to press.
    ///
    /// **It is declared on the node rather than chosen by whoever folds it**,
    /// because two callers deciding it apart is two answers to *what does
    /// folding this do* — the mistake
    /// [ADR-0183](../../../docs/adr/0183-a-node-is-out-of-the-layout-for-two-reasons-and-they-are-two-bits.md)
    /// records one field along. A key, a pointer, a MIDI map, a restored
    /// arrangement and MCP all fold the same node the same way.
    ///
    /// **A solo folds nothing this way**, whatever a node declares: a solo
    /// leaves one region holding the whole viewport, and an edge left behind
    /// is a strip of that viewport it does not hold. See
    /// [`Layout::is_closed`](crate::Layout::is_closed).
    ///
    /// It costs the divider: a parent with a closed child still spends one
    /// divider on it, where a fold that took the node out spends none.
    pub fn keeps_its_edge(self) -> Spec {
        let mut spec = self;
        match &mut spec {
            Spec::View { edge, .. } | Spec::Split { edge, .. } => *edge = true,
        }
        spec
    }

    /// The one place the two variants' shared fields are reached, so a builder
    /// method is a line rather than a match.
    fn with(mut self, f: impl FnOnce(&mut Sizing, &mut f32, &mut f32, &mut bool)) -> Spec {
        match &mut self {
            Spec::View {
                sizing,
                min,
                max,
                collapsed,
                ..
            }
            | Spec::Split {
                sizing,
                min,
                max,
                collapsed,
                ..
            } => f(sizing, min, max, collapsed),
        }
        self
    }
}
