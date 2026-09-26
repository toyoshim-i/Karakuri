use karakuri_layout::{Axis, NodeId, Rect};
use karakuri_operation::{ChainParam, Operation, ParamAt, ParamValue};

pub const GRAB: f32 = 6.0;
pub const HELD_TOLERANCE: f32 = 0.5;
pub const RESTORE_PADDING: f32 = 4.0;
pub(crate) const STOPPED: f32 = 0.05;
pub(crate) const PULLED_THROUGH: f32 = GRAB;
pub(crate) const WORTH_SAYING: f32 = 0.5;

pub struct Node {
    pub id: NodeId,
    /// Depth in layout hierarchy tree.
    pub depth: usize,
}

/// Currently held drag gesture on [`Panel`].
#[derive(Debug)]
pub(crate) enum Drag {
    Boundary(Boundary),
    Fader(Fading),
    Carry(Carrying),
}

/// Active boundary divider drag state.
#[derive(Debug)]
pub(crate) struct Boundary {
    pub(crate) split: NodeId,
    pub(crate) index: usize,
    pub(crate) axis: Axis,
    pub(crate) offset: f32,
    pub(crate) said: Option<f32>,
    pub(crate) held: bool,
    pub(crate) acted: bool,
}

/// Active fader drag state.
#[derive(Debug)]
pub(crate) struct Fading {
    pub(crate) grab: Grab,
    pub(crate) said: Option<f32>,
}

/// A Set in hand, carried from a Library row to a mixer strip or master chain.
///
/// Holds the payload identifier (`set`) and procedure flag until released over a target.
#[derive(Debug)]
pub(crate) struct Carrying {
    pub(crate) set: String,
    pub(crate) procedure: bool,
}

/// Identifies which fader control is held and maps it to the corresponding operation.
#[derive(Debug, Clone, PartialEq)]
pub enum Knob {
    /// Horizontal gain fader ([`Operation::SetGain`]).
    Trim {
        /// Target deck index (ADR-0180).
        deck: u8,
    },
    /// Vertical opacity fader ([`Operation::SetOpacity`]).
    Fader {
        /// Target deck index.
        deck: u8,
    },
    /// Master chain output level before tone mapping ([`Operation::SetMasterOut`], ADR-0224).
    Out,
    /// Master chain parameter fader on specified slot.
    Chain {
        /// Slot index within the master chain.
        at: u32,
        /// Parameter identifier declared by slot procedure.
        key: String,
        /// Declared value range `[min, max]`.
        range: [f32; 2],
    },
    /// Published parameter fader in an Inspector pane ([`Operation::WriteParam`], ADR-0280).
    Param {
        /// Target deck index.
        deck: u8,
        /// Published interface parameter address.
        param: ParamAt,
        /// Declared value range `[min, max]`.
        range: [f32; 2],
    },
}

impl Knob {
    /// Maps fader position `value` to the corresponding engine [`Operation`].
    pub fn operation(&self, value: f32) -> Operation {
        match self {
            Knob::Trim { deck } => Operation::SetGain {
                deck: *deck,
                gain: value,
            },
            Knob::Fader { deck } => Operation::SetOpacity {
                deck: *deck,
                opacity: value,
            },
            Knob::Out => Operation::SetMasterOut { out: value },
            Knob::Chain {
                at,
                key,
                range: [low, high],
            } => Operation::SetChainParam {
                at: *at,
                param: ChainParam::Declared {
                    key: key.clone(),
                    value: low + (high - low) * value.clamp(0.0, 1.0),
                },
            },
            Knob::Param {
                deck,
                param,
                range: [low, high],
            } => Operation::WriteParam {
                deck: *deck,
                param: param.clone(),
                value: ParamValue::Scalar(low + (high - low) * value.clamp(0.0, 1.0)),
            },
        }
    }

    /// Returns which deck this knob controls, or `None` if it is not deck-specific.
    pub fn deck(&self) -> Option<u8> {
        match self {
            Knob::Trim { deck } | Knob::Fader { deck } | Knob::Param { deck, .. } => Some(*deck),
            Knob::Out | Knob::Chain { .. } => None,
        }
    }
}

/// Resolved fader drag state constructed by the view and tracked by the model.
#[derive(Debug, Clone, PartialEq)]
pub struct Grab {
    pub(crate) knob: Knob,
    pub(crate) axis: Axis,
    /// Origin coordinate of track along its axis (left for rows, bottom for columns).
    zero: f32,
    /// Knob center travel distance between track endpoints.
    travel: f32,
    /// Pointer grab offset from knob center along track axis.
    offset: f32,
}

impl Grab {
    /// Creates a grab state if travel distance is positive.
    pub fn new(knob: Knob, axis: Axis, zero: f32, travel: f32, offset: f32) -> Option<Grab> {
        match travel > 0.0 {
            true => Some(Grab {
                knob,
                axis,
                zero,
                travel,
                offset,
            }),
            false => None,
        }
    }

    /// Returns the controlled knob target.
    pub fn knob(&self) -> Knob {
        self.knob.clone()
    }

    /// Computes the normalized `[0, 1]` value for `coord` along the fader track (ADR-0178).
    pub fn value(&self, coord: f32) -> f32 {
        let edge = coord - self.offset;
        unit(match self.axis {
            Axis::Row => (edge - self.zero) / self.travel,
            Axis::Column => (self.zero - edge) / self.travel,
        })
    }
}

/// Clamps `at` to `[0.0, 1.0]`, mapping `NaN` to `0.0`.
pub(crate) fn unit(at: f32) -> f32 {
    match at.is_nan() {
        true => 0.0,
        false => at.clamp(0.0, 1.0),
    }
}

/// High-level panel operations decoupled from input keys or pointer positions (ADR-0175, ADR-0197).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// Collapse this region.
    Fold(NodeId),
    /// Makes `id` visible, expanding collapsed ancestors and undoing masking solos.
    Unfold(NodeId),
    /// Collapse the enclosing split.
    FoldEnclosing(NodeId),
    /// Expand all collapsed regions.
    UnfoldAll,
    /// Solo this region.
    Solo(NodeId),
    /// Undo the active solo.
    Unsolo,
    /// Reset arrangement to default layout at same viewport.
    Reset,
    /// Produces layout placements for diagnostic and test verification (ADR-0164, ADR-0205).
    Report,
}

/// What a press found under the pointer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pressed {
    /// Divider boundary is grabbed.
    Grabbed {
        split: NodeId,
        index: usize,
        axis: Axis,
        /// Solved position along split axis.
        at: f32,
        /// Pointer grab offset along boundary axis.
        offset: f32,
    },
    /// Boundary divider without valid pair.
    NoPair { split: NodeId, index: usize },
    /// The interior of a region.
    Region { id: NodeId, rect: Rect },
    /// Outside the viewport, or unclaimed area.
    Nothing,
}

/// Outcome of a drag move with a boundary or fader in hand.
#[derive(Debug, Clone, PartialEq)]
pub enum Dragged {
    /// A boundary moved.
    Boundary {
        split: NodeId,
        index: usize,
        axis: Axis,
        /// Target coordinate requested by drag.
        asked: f32,
        /// Solved coordinate returned by layout solver.
        landed: f32,
        /// Displacement held by stop constraint if movement was resisted.
        held: Option<f32>,
    },
    /// A boundary drag triggered a fold or unfold operation on a pane.
    Pane(Op),
    /// A fader drag emitted an engine operation without modifying local state (ADR-0156, P-0090).
    Fader(Operation),
}

/// Result of releasing an active drag gesture (ADR-0156).
#[derive(Debug, Clone, PartialEq)]
pub enum Released {
    Rests {
        split: NodeId,
        index: usize,
        /// Coordinate along the split axis.
        at: f32,
    },
    /// Boundary removed during drag by another operation.
    Gone { split: NodeId, index: usize },
    /// Fader released; final value already emitted during motion.
    Let { knob: Knob },
    /// Carried item dropped onto a valid target deck or chain (ADR-0273, P-0090).
    Dropped(Operation),
    /// Carried item dropped over no valid target.
    Nowhere { set: String },
    /// Target refused the carried payload (ADR-0273, ADR-0340, P-0083).
    Refused { set: String, why: &'static str },
}

/// Target destination where a carried item was released (ADR-0273).
#[derive(Debug, Clone, PartialEq)]
pub enum Landing {
    /// Target deck index.
    Deck(u8),
    /// Master chain append result or rejection reason.
    Chain(Result<Operation, &'static str>),
}

/// Whether a node is drawing, and if not, why not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Visible,
    /// Collapsed by fold operation.
    Folded,
    /// Hidden inside a collapsed parent region.
    InsideAFold,
}

/// Placed layout node geometry reported by [`Op::Report`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub id: NodeId,
    pub depth: usize,
    pub rect: Rect,
    pub state: Visibility,
}

/// Result of executing an [`Op`].
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// Fold outcome with resulting collapse state and root status.
    Folded {
        id: NodeId,
        folded: bool,
        root: bool,
    },
    /// Expanded node identifiers in tree order.
    Unfolded(Vec<NodeId>),
    /// Region soloed.
    Soloed(NodeId),
    /// Solo undone; `was` indicates whether a solo was active.
    Unsoloed { was: bool },
    /// Arrangement reset to defaults.
    Reset,
    /// Saved arrangement applied ([`Panel::restore`]).
    Restored,
    /// Layout placement report ([`Op::Report`]).
    Report(Vec<Placement>),
    /// Operation had no valid target.
    Nothing,
}

/// Active drag interaction type ([`Panel::in_hand`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InHand {
    /// Divider boundary drag along the given axis.
    Boundary(Axis),
    /// Fader parameter drag.
    Fader,
    /// Set or procedure carried out of the Library bay.
    Carrying,
}
