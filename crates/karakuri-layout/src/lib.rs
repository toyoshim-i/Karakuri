//! The console panel's region model: which rectangle every region of the panel
//! occupies, and what a pointer at a coordinate is touching.
//!
//! It is pure geometry and constraint solving. **It does not draw**, it does
//! not know a toolkit, a window or a graphics device, and it does not know what
//! a view *contains* — a view is a name and a set of size constraints, and
//! whatever renders it looks the name up. Everything here runs on a machine
//! with no graphics adapter at full speed, which is what lets the panel's
//! behaviour be answered by a test rather than by looking at it.
//!
//! A [`Layout`] is an arena of nodes with one root. A node is either a **view**
//! (a leaf) or a **split** (an axis, a divider thickness, and children). Every
//! node carries, *along its parent's axis*, a [`Sizing`], a `min`, a `max` and
//! the two flags that take it out of the layout; its extent across that axis
//! is always its parent's.
//! Nothing in this crate knows the console's arrangement: the arrangement is
//! one [`Spec`] value built with the same pieces any other arrangement is.
//!
//! **A name addresses a region, and either kind of node may carry one.** A
//! view's is required — it is what says what to draw. A split's is optional
//! and set with [`Spec::named`], because most splits are structure nobody
//! addresses and some are not: the console's left pane *is* a split, holding
//! the library and the staging lane stacked, and "fold the left pane away" is
//! an operation the keyboard, a MIDI map and MCP each reach by that name.
//! [`Layout::find`] resolves any named node, and a name is used **once** in an
//! arrangement — [`Layout::new`] refuses one that is not, rather than picking
//! whichever node came first.
//!
//! ```
//! use karakuri_layout::{Layout, Rect, Spec};
//!
//! let mut layout = Layout::new(Spec::row(
//!     4.0,
//!     vec![
//!         Spec::view("library").fixed(240.0).min(160.0).max(400.0),
//!         Spec::view("program").flex(1.0).min(200.0),
//!     ],
//! ));
//! let program = layout.find("program").unwrap();
//!
//! layout.set_viewport(Rect::new(0.0, 0.0, 1280.0, 720.0));
//! layout.solve();
//! assert_eq!(layout.rect(program), Rect::new(244.0, 0.0, 1036.0, 720.0));
//! ```
//!
//! # A region is out of the layout for two reasons, and they are two bits
//!
//! **The operator folds a region away** — [`Layout::collapse`],
//! [`Layout::expand`], [`Layout::toggle`] — and that is part of the
//! arrangement: it is saved with it, and a load brings it back exactly.
//!
//! **Whoever is drawing takes a region out** — [`Layout::set_aside`] — because
//! it has put that region somewhere else, or has nowhere to put it. That is
//! not part of the arrangement. It is a function of the geometry the caller
//! has in front of it, so it is **never saved**: a load lays every node out,
//! and the first caller to draw the loaded arrangement re-derives it.
//!
//! The solve treats them identically — zero extent, no divider, and nothing
//! inside them drawn — and everything else treats them as two.
//! [`Layout::visible`] is the disjunction and is what every reader deciding
//! what to lay out asks; [`Layout::is_collapsed`] and
//! [`Layout::is_set_aside`] each answer for their own bit and are for
//! reporting it or clearing it. **One bit for both would be cheaper and is
//! wrong in both directions**: unfolding while a region is set aside would put
//! an empty strip back where the caller had already drawn that region, and a
//! fold the operator never made would be written into their saved
//! arrangement.
//!
//! Writing the bit a node already carries marks nothing dirty, because a
//! caller re-derives it every frame and a layout that went dirty every frame
//! would cost a still panel the price of a moving one.
//!
//! # The tree answers upward and across, not only downward
//!
//! A caller can walk from [`Layout::root`] down and solve, and for a while
//! that was all it could do: the parent of a node, the visible children of a
//! split, where a boundary is, and where all of them are were each left to
//! whoever was asking. What that produced is a caller with a model of its own
//! — the console kept a parent per node, rebuilt it whenever the layout was
//! replaced, and wrote its own copy of the [`Axis`] arithmetic — and a second
//! model of one tree is two answers that drift.
//!
//! So the questions that go the other way are answered here:
//! [`parent`](Layout::parent), [`visible_children`](Layout::visible_children)
//! — the ones a divider index counts —
//! [`boundary`](Layout::boundary) and [`boundaries`](Layout::boundaries),
//! [`sizing`](Layout::sizing), [`soloed`](Layout::soloed),
//! [`is_set_aside`](Layout::is_set_aside) and [`is_view`](Layout::is_view). None of them takes `&mut self`, and the two
//! that return a set of nodes return an iterator, because a view asks them
//! every frame.
//!
//! # Solving never writes back into the model
//!
//! This is the one rule the crate is built around, and it is the thing a future
//! change will want to violate, so it is written here rather than left to be
//! inferred.
//!
//! A solve reads the stored sizes and the viewport and produces rectangles. It
//! never stores a rectangle back as a size. Only an explicit operation — a drag
//! ([`Layout::set_divider`]), a [`collapse`](Layout::collapse), an
//! [`expand`](Layout::expand), a [`set_aside`](Layout::set_aside) — changes
//! what a node stores. **Taking a region out of the layout is an operation
//! like a fold and not a step of the solve**, however derived the value being
//! written is: the caller works it out from the geometry and then *tells* the
//! layout, so the solve still holds the arrangement by shared reference and
//! still cannot write a node.
//!
//! **The compiler enforces it, so it is not a rule anyone has to remember.**
//! A `Layout` is two halves: the arrangement — the nodes, the root, and what a
//! [`solo`](Layout::solo) saved — and the buffers a solve writes.
//! [`Layout::solve`] destructures itself into them, which makes the borrows
//! disjoint, and calls a free function with the arrangement by **shared**
//! reference and the buffers by mutable one. Everything the solve does is in
//! that function, so a line added inside it that stored a solved size into a
//! node does not compile: there is no `&mut` to a node in scope, and no method
//! on a shared arrangement that would hand one out.
//!
//! What that buys is the behaviour an operator will notice within a minute of
//! using the panel: **a window dragged small and then large again comes back to
//! exactly the arrangement it left.** The small window never destroyed
//! anything; it only produced small rectangles. Every layout that clamps its
//! model to fit the space it currently has loses that, and it loses it silently
//! and permanently — the operator's arrangement is gone, and there is nothing
//! to restore it from because the only copy was overwritten by a resize nobody
//! asked to be an edit.
//!
//! The temptation is real, because writing back makes several things one line
//! shorter: a drag could store the rectangle it produced, and a clamp could
//! store the clamped size instead of re-deriving it every solve. Neither is
//! worth the property above. The borrow checker refuses the first of them, and
//! `tests/arrangement.rs` holds the test that fails if this is ever violated by
//! a route the types do leave open — an operation that writes when it should
//! not.

mod layout;
mod spec;

pub use layout::{Hit, Layout, NodeId};
pub use spec::Spec;

use serde::{Deserialize, Serialize};

/// A point in the panel's coordinate space — the same space the viewport is
/// given in, pixels by convention and top-left origin.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Point {
        Point { x, y }
    }
}

/// A rectangle, by origin and size. `w` and `h` are never negative in anything
/// this crate produces — see [`Layout::rect`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    /// Half-open on both axes: the near edges are inside and the far edges are
    /// not. That is what makes two rectangles that share an edge — which is
    /// every pair of neighbours here, since children tile their parent — claim
    /// a point between exactly one of them, never both and never neither.
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x < self.x + self.w && p.y >= self.y && p.y < self.y + self.h
    }
}

/// The direction a split lays its children out in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Axis {
    /// Left to right.
    Row,
    /// Top to bottom.
    Column,
}

impl Axis {
    /// The extent of `r` along this axis.
    ///
    /// **These four are public because every caller of this crate needs
    /// them.** A [`Layout`] answers in [`Rect`]s and takes coordinates *along
    /// a split's axis* — [`Layout::set_divider`] is one — so a caller that has
    /// an axis and a rectangle is doing this arithmetic whatever this crate
    /// says. While they were `pub(crate)`, the console wrote its own copy of
    /// all four, which is a copy of this crate's private code that every
    /// future caller would have written again.
    pub fn extent(self, r: Rect) -> f32 {
        match self {
            Axis::Row => r.w,
            Axis::Column => r.h,
        }
    }

    /// The near edge of `r` along this axis.
    pub fn origin(self, r: Rect) -> f32 {
        match self {
            Axis::Row => r.x,
            Axis::Column => r.y,
        }
    }

    /// The far edge of `r` along this axis, which is its origin plus its
    /// extent. A boundary is one of these — the far edge of the child before
    /// it — which is why it is a method rather than left as a sum.
    pub fn far(self, r: Rect) -> f32 {
        self.origin(r) + self.extent(r)
    }

    /// The coordinate of `p` along this axis.
    pub fn coord(self, p: Point) -> f32 {
        match self {
            Axis::Row => p.x,
            Axis::Column => p.y,
        }
    }

    /// The slice of `parent` running from `start` for `size` along this axis,
    /// taking the whole of `parent` across it.
    pub(crate) fn slice(self, parent: Rect, start: f32, size: f32) -> Rect {
        match self {
            Axis::Row => Rect::new(start, parent.y, size, parent.h),
            Axis::Column => Rect::new(parent.x, start, parent.w, size),
        }
    }
}

/// How a node claims extent along its parent's axis.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Sizing {
    /// Keeps its size when the viewport changes. A side pane is this.
    ///
    /// It keeps it as long as *something* in the split can absorb the change.
    /// Where a split holds nothing flexible, its fixed children share the
    /// difference in proportion — up to their maxima, past which the space is
    /// left empty rather than forced on them. See [`Layout::solve`].
    ///
    /// **And it never claims more than what is visible inside it can use.** A
    /// split that stores 378 and has been folded down to a single 72-tall
    /// child claims 72, and its siblings get the difference; unfold the child
    /// and it claims its 378 again, because nothing was written back. The same
    /// cap applies to the node's `min`, for the same reason: a minimum is what
    /// a region needs while it has something to show.
    Fixed(f32),
    /// Absorbs what is left, in proportion to the weight. The centre is this.
    Flex(f32),
}
