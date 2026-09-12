//! Declarative specification format for constructing panel arrangements.
//!
//! A [`Spec`] is a nested builder tree that compiles into a flat [`Layout`](crate::Layout) arena.

use crate::{Axis, Sizing};

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
    /// An optional name allows the container to be addressed directly (e.g. for folding entire panes).
    Split {
        name: Option<String>,
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

    /// Names this node for resolution via [`Layout::find`](crate::Layout::find).
    ///
    /// Node names must be unique within an arrangement; duplicates are refused during layout construction.
    pub fn named(self, name: impl Into<String>) -> Spec {
        let mut spec = self;
        match &mut spec {
            Spec::View { name: n, .. } => *n = name.into(),
            Spec::Split { name: n, .. } => *n = Some(name.into()),
        }
        spec
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

    /// Configures the node to retain its divider boundary when collapsed.
    ///
    /// When collapsed, the node is reduced to zero extent while remaining in its parent's
    /// placed children, keeping the divider visible and hit-testable for mouse drag expansion.
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
