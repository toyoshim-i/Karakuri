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
    /// How deep in the tree, for a caller that indents. The one thing tree order
    /// costs to work out and the arena does not store.
    pub depth: usize,
}

/// What the pointer has hold of, and there is exactly one of it — see the
/// module documentation for why this is one `Option` on [`Panel`] and not two.
#[derive(Debug)]
pub(crate) enum Drag {
    Boundary(Boundary),
    Fader(Fading),
    Carry(Carrying),
}

/// A boundary in hand: which one, and where along it the pointer took hold.
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

/// A fader in hand: the control, its track and the grab — and the last value
/// this drag asked for.
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
    /// The trim — the mock's `.trim .fader`, lying down — [`Operation::SetGain`].
    Trim {
        /// Which deck's, which is the manual's word for what the code calls a slot —
        /// the vocabulary's own choice, recorded in ADR-0180: *"The manual's deck is
        /// the code's slot … the vocabulary takes the manual's word, since the manual
        /// is its specification."*
        deck: u8,
    },
    /// The fader — `.vfader`, standing up — [`Operation::SetOpacity`].
    Fader {
        /// Which deck's, on [`Knob::Trim`]'s terms.
        deck: u8,
    },
    /// Master chain output level before tone mapping ([`Operation::SetMasterOut`], ADR-0224).
    Out,
    /// One declared parameter of one slot of the master chain, on the parameter
    /// row the Master bay draws for it.
    Chain {
        /// Which slot of the master chain, by its position — the address a chain
        /// operation takes.
        at: u32,
        /// The parameter this track moves, by the name the slot's procedure declares
        /// for it.
        key: String,
        /// The range the procedure declares the parameter over, low then high. A
        /// track position is `[0, 1]`, and the operation carries the value at that
        /// fraction of this range.
        range: [f32; 2],
    },
    /// Published parameter fader in an Inspector pane ([`Operation::WriteParam`], ADR-0280).
    Param {
        /// Which deck's, on [`Knob::Trim`]'s terms.
        deck: u8,
        /// The control the interface published. A `None` node is a wildcard, and
        /// `Set::write_param` is where ADR-0223's authority refusal meets it.
        param: ParamAt,
        /// What it was published over. [`Knob::operation`]'s `value` is a track
        /// position and this is what turns it into a number.
        range: [f32; 2],
    },
}

impl Knob {
    /// The operation this knob asks for at `value`, and the whole of the
    /// translation a fader performs.
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
            // The operation names the slot by its position and carries what
            // that one parameter is set to, never a step. The cut a slot reads
            // is set by a press on the chip and not by a drag on the track.
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
            // `crate::view::Param::at` inverted, which is the relation
            // `Grab::value` has to `crate::view::filled` one field along.
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
    /// The track's zero end along the axis: the left of a row, and the *bottom* of
    /// a column, because a fader stands up.
    zero: f32,
    /// How far the knob's centre travels between the ends. Positive.
    travel: f32,
    /// Pointer coordinate less the knob's centre, at the moment of the press.
    /// Subtracted from every later coordinate so the value does not jump to the
    /// pointer on the first move — [`Boundary::offset`]'s rule on a value instead
    /// of on a position.
    offset: f32,
}

impl Grab {
    /// A fader taken hold of. `travel` is the length of track the knob's centre
    /// moves along, and a track with none is refused: a control with nowhere to go
    /// is not one a hand has hold of.
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

    /// Which control this is, and which deck's where it is a deck's.
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
    /// Fold this region away.
    Fold(NodeId),
    /// Makes `id` visible, expanding collapsed ancestors and undoing masking solos.
    Unfold(NodeId),
    /// Fold the split enclosing this region.
    FoldEnclosing(NodeId),
    /// Unfold everything folded. A folded region has no rectangle, so the pointer
    /// cannot reach it to unfold it.
    UnfoldAll,
    /// Solo this region.
    Solo(NodeId),
    /// Undo the solo.
    Unsolo,
    /// A fresh arrangement, at the same viewport.
    Reset,
    /// Produces layout placements for diagnostic and test verification (ADR-0164, ADR-0205).
    Report,
}

/// What a press found under the pointer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pressed {
    /// A boundary is now in hand, and every later [`Panel::moved`] moves it.
    Grabbed {
        split: NodeId,
        index: usize,
        axis: Axis,
        /// Where the boundary was, along the split's axis.
        at: f32,
        /// Where along it the pointer took hold: the pointer's coordinate less the
        /// boundary's.
        offset: f32,
    },
    /// A divider the tree cannot produce a pair for, so there is nothing to drag.
    /// Nothing is in hand.
    NoPair { split: NodeId, index: usize },
    /// The interior of a region.
    Region { id: NodeId, rect: Rect },
    /// Outside the viewport, or somewhere no visible region claims.
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
        /// Where the drag asked the boundary to go: the pointer's coordinate along the
        /// axis, less the offset it grabbed at.
        asked: f32,
        /// Where it went, which is what [`Layout::set_divider`] returned.
        landed: f32,
        /// `Some(by)` when a stop held it, where `by` is `landed - asked` — how far
        /// short of the ask, and which way. `None` when it landed where it was asked
        /// to.
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
        /// Along the split's axis.
        at: f32,
    },
    /// The boundary is no longer there — an operation during the drag folded one of
    /// the pair away.
    Gone { split: NodeId, index: usize },
    /// A fader was released; the last value was already emitted during motion.
    Let { knob: Knob },
    /// A carried item was dropped onto a valid target deck or chain (ADR-0273, P-0090).
    Dropped(Operation),
    /// A carried item was dropped over no valid landing target.
    Nowhere { set: String },
    /// The target refused the carried payload (ADR-0273, ADR-0340, P-0083).
    Refused { set: String, why: &'static str },
}

/// Target destination where a carried item was released (ADR-0273).
#[derive(Debug, Clone, PartialEq)]
pub enum Landing {
    /// The deck a strip or a preview cell names.
    Deck(u8),
    /// The master chain's list, with what adding the carried row asks for — or the
    /// reason it asks for nothing, which is a row that is not a `kind L5`.
    Chain(Result<Operation, &'static str>),
}

/// Whether a node is drawing, and if not, why not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Visible,
    /// Folded itself.
    Folded,
    /// Not folded, but inside something that is.
    InsideAFold,
}

/// One node of the arrangement and where it solved to. What [`Op::Report`]
/// produces, one per [`Node`], in the same order.
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
    /// [`Op::Fold`], [`Op::Unfold`] or [`Op::FoldEnclosing`]: `folded` is what the
    /// node is afterwards, read back out of the layout rather than assumed from the
    /// operation, and `root` that what folded was the root — so the panel is now
    /// empty, and only [`Op::UnfoldAll`] brings it back.
    Folded {
        id: NodeId,
        folded: bool,
        root: bool,
    },
    /// [`Op::UnfoldAll`]: everything that was folded, in tree order. Empty when
    /// nothing was.
    Unfolded(Vec<NodeId>),
    /// [`Op::Solo`].
    Soloed(NodeId),
    /// [`Op::Unsolo`]. `was` is false when there was no solo to undo, and nothing
    /// changed.
    Unsoloed { was: bool },
    /// [`Op::Reset`].
    Reset,
    /// [`Panel::restore`]: a saved arrangement has been applied.
    Restored,
    /// [`Op::Report`].
    Report(Vec<Placement>),
    /// The operation had no valid target (e.g. [`Op::FoldEnclosing`] on root).
    Nothing,
}

/// What the pointer has hold of, and there is only ever one of them —
/// [`Panel::in_hand`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InHand {
    /// A boundary, and the axis it runs along — which is what a resize cursor is
    /// drawn from, and the whole of why the axis is here.
    Boundary(Axis),
    /// A fader. No axis, because there is no resize cursor for a value: see
    /// [`Panel::in_hand`].
    Fader,
    /// A Set or procedure carried out of the Library bay.
    Carrying,
}
