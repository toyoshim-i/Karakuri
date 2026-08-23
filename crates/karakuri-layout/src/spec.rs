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

use crate::{Axis, Sizing};

/// One node of an arrangement, before it is built.
///
/// The constraints on a node — [`Sizing`], `min`, `max`, `collapsed` — apply
/// along its *parent's* axis, so they are set on the child rather than by the
/// parent that arranges it. The root's are ignored: the root is the viewport.
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
        axis: Axis,
        divider: f32,
        children: Vec<Spec>,
        sizing: Sizing,
        min: f32,
        max: f32,
        collapsed: bool,
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

    /// Start collapsed — folded to zero extent, with no divider beside it.
    pub fn collapsed(self) -> Spec {
        self.with(|_, _, _, c| *c = true)
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
