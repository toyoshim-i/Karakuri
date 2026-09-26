//! Console panel layout model: computes region bounds and resolves pointer hit tests.
//! Evaluates tree hierarchies of view leaves and split containers under size constraints.

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
    /// Returns the extent of `r` along this axis.
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
    /// Fixed pixel dimension along the parent's layout axis.
    Fixed(f32),
    /// Proportional flex weight absorbing remaining space.
    Flex(f32),
}
