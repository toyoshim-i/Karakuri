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

/// Which fader a hand has hold of, named after the operation each of them is a
/// translator for.
///
/// The strip's two are two controls and not one, because `karakuri-engine`'s
/// `mix.rs` says they are two things: *opacity at zero silences under every
/// blend mode, and gain at zero does not silence `over`*. See
/// [`crate::view::Strip::gain`].
///
/// # The deck is carried here, and [`Knob::Out`] is why
///
/// It was an argument beside this — `operation(deck, value)` — and a `deck`
/// beside a knob that has none would be a slot number invented to satisfy a
/// signature. The master out is one level on the whole fold rather than on a
/// member of it (`karakuri_engine::deck::Deck::set_out`, *"Not per slot"*), so
/// which deck is part of what a knob *is* rather than something every knob has.
/// A variant that carries it and one that does not is the compiler holding
/// that, where an `Option<u8>` would leave `Trim` free to be `None`.
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
    /// The master out — the Master bay's `.fader`, lying down, and the one knob on
    /// this panel that names no deck: it is the level the composited frame leaves
    /// the mix at, at the entry to the master chain ([`Operation::SetMasterOut`],
    /// and
    /// `docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md`
    /// for why it is not the tone mapper's exposure).
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
    /// A published parameter's fader, in an Inspector pane —
    /// [`Operation::WriteParam`]. It is the one knob that names something inside a
    /// Set rather than a level on it, which is why it carries a name and this enum
    /// is no longer `Copy`: a parameter is addressed by name, which is ADR-0280's
    /// own reason for `mix::Change`, and the address is `karakuri-operation`'s so
    /// there is one spelling of it.
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

    /// Which deck this knob is on, or `None` for the one that is on none.
    ///
    /// For a caller with a sentence to print. Nothing routes through it: the deck
    /// is inside the operation already, which is [`Dragged::Fader`]'s rule about
    /// one value and not two.
    pub fn deck(&self) -> Option<u8> {
        match self {
            Knob::Trim { deck } | Knob::Fader { deck } | Knob::Param { deck, .. } => Some(*deck),
            Knob::Out | Knob::Chain { .. } => None,
        }
    }
}

/// A fader taken in hand: which deck's which control, the track its value
/// rides, and where along the knob the pointer took hold.
///
/// # It is built by the view and held by the model, and that is the seam
///
/// A boundary's geometry is the layout's, so [`press`](Panel::press) hit-tests
/// it here and [`moved`](Panel::moved) re-reads it every move. A fader's is
/// not. Where a strip's tracks are is [`crate::view::mixer`]'s answer, it
/// depends on the values the harness handed in *and* on a text shaper for the
/// words in the same strip, and this module has neither and takes neither. So
/// the view resolves a press to one of these and the model takes it in hand —
/// the same shape [`crate::view::Outputs::op`] has, where the derivation that
/// claims a press is asked a second time to say what the press meant, rather
/// than copied.
///
/// The consequence is stated rather than hidden: a drag maps the pointer
/// against the track it took hold of, so a window resized under a fader that is
/// being held goes on using the track the press was made on until the button
/// comes up. A hand cannot do both at once, and the alternative — this module
/// re-deriving a strip's geometry — is the toolkit inside the model.
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

    /// What a pointer at `coord` puts this fader at, on `[0, 1]`.
    ///
    /// `crate::view`'s `filled` inverted, and it runs the same two ways: a row
    /// fills from the left and a column from the bottom.
    ///
    /// # Both ends are exactly reachable
    ///
    /// A knob dragged to the floor gives exactly `0.0` and one dragged to the top
    /// exactly `1.0` — the subtraction is zero at one end and the division is
    /// `travel / travel` at the other, and a pointer past either end is clamped to
    /// it. That is `karakuri-midi`'s `GAIN_RANGE` argument reached from the other
    /// side: *"`[0, 1]` … chosen so that both ends of a fader are exact … a fader
    /// whose top is unity is what a fader means."*
    ///
    /// A gain above 1.0 cannot be asked for here, and that is a known gap rather
    /// than a ceiling: `Deck::set_gain` is deliberately unclamped because the mix
    /// is HDR, and ADR-0178 records that this control is drawn over `[0, 1]` and
    /// cannot show the difference between 1.0 and 3.0. The top of this drag is
    /// exactly unity, so what the gap costs is *reach* rather than *precision*.
    pub fn value(&self, coord: f32) -> f32 {
        let edge = coord - self.offset;
        unit(match self.axis {
            Axis::Row => (edge - self.zero) / self.travel,
            Axis::Column => (self.zero - edge) / self.travel,
        })
    }
}

/// A value on `[0, 1]`, and a NaN is zero.
///
/// `f32::clamp` passes a NaN straight through, which would be a `NaN`-wide fill
/// on a track and a `NaN` in an operation aimed at the mix. The engine takes
/// the same reading one level down — *"a fader whose value is not a number is a
/// broken control, and of the two available readings … only one of them is a
/// fader"* — and this is where a value that did not come through
/// `Deck::set_opacity` takes it: a reading handed to [`crate::view`] by a
/// caller, and a pointer coordinate handed to [`Grab::value`].
pub(crate) fn unit(at: f32) -> f32 {
    match at.is_nan() {
        true => 0.0,
        false => at.clamp(0.0, 1.0),
    }
}

/// What an operation acts on. Named as operations rather than as keys, so a
/// caller with no keyboard can ask for one.
///
/// # An operation carries what it acts on, and the pointer is a way of
/// choosing that
///
/// Every operation here used to mean *the region under the pointer*:
/// `Fold` was *fold whatever [`Panel::cursor`] is over*, and the model
/// resolved the pointer itself, once per operation. That reads fine while the
/// keyboard is the only surface, and it is wrong as soon as anything else asks
/// — because *under the pointer* is not part of what folding is. It is one
/// way of naming which region to fold.
///
/// The console's Outputs row is the case that proves it. Its one control folds
/// `program-view` — the picture — whatever the pointer happens to be over,
/// because the pointer is over the control, at the other end of the panel. A
/// pointer-relative vocabulary has no way to say that: it would need a second
/// operation meaning *fold that one instead*, and then two names for one act.
///
/// So the four operations that act on a node take the node, and the four that
/// act on the arrangement as a whole take nothing:
///
/// - [`Fold`](Op::Fold), [`Unfold`](Op::Unfold),
///   [`FoldEnclosing`](Op::FoldEnclosing) and [`Solo`](Op::Solo) name a
///   [`NodeId`].
/// - [`UnfoldAll`](Op::UnfoldAll), [`Unsolo`](Op::Unsolo),
///   [`Reset`](Op::Reset) and [`Report`](Op::Report) have nothing to name.
///
/// Resolving the pointer is the caller's, and it is [`Panel::under`]: the
/// harness does it for a key press, and the Outputs control does not need it.
/// That also moves two answers out of [`Outcome`] and into the caller, where
/// they were always about the pointer rather than about the operation — *the
/// pointer is on a divider* and *there is nothing under the pointer* are
/// things a resolution finds, not things a fold reports.
///
/// # Folding and unfolding are not mirror images, and that is the point
///
/// [`Fold`](Op::Fold) folds one node and nothing else. [`Unfold`](Op::Unfold)
/// makes one node visible, which takes its collapsed ancestors with it and
/// a solo that is hiding it. Hiding a thing hides one thing; showing a thing
/// means showing the way to it — the ordinary shape of revealing a node in a
/// tree, and the reason the two are not each other's inverse.
///
/// The alternative is `expand(id)` and nothing else, and the console's one
/// control is what settles it against: with the Program bay folded around the
/// picture, the Outputs dot is dark, and a press that expanded the picture
/// inside a still-folded bay would light nothing, move nothing and say
/// nothing. `docs/manual/console.html` rules on exactly that shape, in the
/// note about this very row — *"Nothing is refused here, so nothing has to be
/// explained: a control that quietly declines the last of something is a rule
/// an operator can only find by experiment."* A control that appears not to
/// respond is that rule with no words at all.
///
/// [`UnfoldAll`](Op::UnfoldAll) keeps its own meaning and is not folded into
/// this: it is what reaches a fold nobody has a name for, and `Unfold`
/// undoes only what stands between one named node and the screen.
///
/// # Fold and unfold are two operations, and there is no toggle
///
/// A toggle is an affordance built over two operations by whoever draws it
/// — the Outputs dot asks for [`Fold`](Op::Fold) when the picture is on and
/// [`Unfold`](Op::Unfold) when it is off — and it is not one itself. The
/// vocabulary is what every surface routes into: a MIDI map with a button per
/// direction, an MCP call that says which one it wants, and a keyboard, all
/// have to be able to say *fold this* and mean it. `Fold` folded-or-unfolded
/// could never be asked for the direction it will take, which is the same
/// reason a `Toggle` variant is not here.
///
/// It cost nothing where the pointer is the surface: a folded region has no
/// rectangle, so the pointer could never be over one, so `f` on the keyboard
/// only ever folded. What changes is that a *named* target can be folded
/// already, and now the caller says which way it means.
///
/// # This is not `karakuri_operation::Operation`, and what keeps it apart is a
/// name
///
/// The vocabulary names the same five things — `FoldBay`, `FoldPane`,
/// `Unfold`, `Solo`, `MoveBoundary` — and names a region by `String`, because
/// it is a leaf crate with no dependencies and cannot say [`NodeId`]. A
/// `NodeId` has a private field and only a `Layout` hands one out, so the two
/// cannot be one type: a caller holding a layout has the handle, and a caller
/// with none has the name.
///
/// These operations write no record, so nothing downstream of this module
/// changes when they move — `karakuri-operation-record` answers
/// `Silent(Surface)` for all four. Routing into the vocabulary here would mean
/// the panel *accepting* a named region rather than emitting one, and the
/// survey that costed it is
/// [ADR-0197](../../../docs/adr/0197-the-consoles-op-stays-and-what-blocks-it-is-the-page-rather-than-the-code.md).
/// Three things are in the way and every one is a question about
/// `docs/manual/operations.html`: [`Reset`](Op::Reset) and
/// [`Report`](Op::Report) have no row on it, *Move a boundary* is a drag and
/// never an operation, and two splits in this arrangement have no name —
/// the root column and the body row — which a pointer reaches through
/// `Hit::Divider { split, .. }` and a `String` cannot say at all.
/// `tests/vocabulary.rs` asserts all three, both ways round.
///
/// [`FoldEnclosing`](Op::FoldEnclosing) was counted as a fourth, for naming
/// its target by relation where every row names one outright. It is not one:
/// naming a target by relation is the caller's and not the operation's — the
/// rule `Operation::Crossfade` states at its own variant, and the one
/// ADR-0175 applied here — so what is left on this side is
/// `Fold(self.layout.parent(id))`, the fold the page already specifies on a
/// node this model works out. The page owes it no row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// Fold this region away.
    Fold(NodeId),
    /// Make this region visible, undoing whatever stands between it and the screen:
    /// `id` itself, every collapsed ancestor of it, and a solo that is keeping it
    /// off screen.
    ///
    /// Not the same as [`UnfoldAll`](Op::UnfoldAll): a sink turned back on brings
    /// back *that* picture and the way to it, and nothing else that happens to be
    /// folded. Not the mirror of [`Fold`](Op::Fold) either — see the note above.
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
    /// Every node and where it solved to.
    ///
    /// No surface binds a key to this, and none has since 2026-08-31.
    /// `crates/karakuri/src/main.rs` bound `p` to it and printed the table;
    /// `docs/manual/operations.html` specifies `p` as half of *Nudge the latency
    /// offset*, a badge naming two keys with one of them bound would be a badge
    /// that lies, and a panel diagnostic asked what shortcut it needed had no
    /// answer worth the letter. So the letter went and the operation stayed.
    ///
    /// What still reads it is this crate's own suite, and neither test is a test
    /// *of* this variant: [`tests/panel.rs`](../../tests/panel.rs) asks for a
    /// report immediately after a solo with nothing solving in between, which is
    /// what catches [`Panel::op`] leaving a solve owed — it is the only operation
    /// that reads every rectangle, so it is the only one that can; and
    /// [`tests/repaint.rs`](../../tests/repaint.rs) asks it because
    /// [`Outcome::Report`] is the one outcome that carries a `Vec` and must still
    /// answer [`crate::repaint::Repaint::Never`], which is the case ADR-0164's
    /// still-panel clause is most exposed to.
    ///
    /// And it is not the startup legend's table. That one is each region's *min and
    /// max* — the constraints, printed once, before anything has been dragged. This
    /// is each region's solved rectangle and whether it is folded, at whatever
    /// moment it is asked. Nothing else in the system answers the second.
    ///
    /// `tests/vocabulary.rs` goes on pinning that no row on
    /// `docs/manual/operations.html` names it, which is permanent (ADR-0205) and is
    /// about the page rather than about a key.
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

/// What a move with something in hand did, and the two arms are the two kinds
/// of drag — see the module documentation.
///
/// Produced only when something happened. For a boundary that is when it moved,
/// or when the stop holding it changed: a drag is reported when the layout does
/// something, not when the pointer does, so a pointer held against a stop
/// reports once and then nothing. For a fader it is when the value changed,
/// exactly: a pointer dragged on past the end of a track asks for the end sixty
/// times a second and the value is already there.
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
    /// A boundary drag closed a pane, or brought one back — and the [`Op`] this
    /// module performed, rather than one it is handing over.
    ///
    /// [`Op::Fold`] where a pane was pulled out through its own edge, and
    /// [`Op::Unfold`] where a closed pane's edge was pulled back in. It names the
    /// node, so a caller with a sentence to print has everything it needs and reads
    /// the rest off [`Panel::layout`].
    ///
    /// # Performed here, where [`Fader`](Dragged::Fader) is not, and the module
    /// documentation is why
    ///
    /// *A boundary drag moves the arrangement, which is this crate's, so `moved`
    /// writes it and reports where it landed; a fader drag moves the engine's
    /// value, which this crate does not have and cannot reach.* A fold is the
    /// arrangement, so it is written here and this says what was written. The
    /// alternative — handing an unperformed `Op` back and waiting for the caller —
    /// would leave the layout disagreeing with itself for one event, on the pane
    /// the *next* move is about to ask a question of.
    ///
    /// # Why it is an [`Op`] at all, rather than a size the drag wrote
    ///
    /// A drag that only zeroed the pane's extent would leave nothing
    /// [`Layout::is_collapsed`] could answer yes to, so `z` ([`Op::UnfoldAll`])
    /// would not bring it back — and `z` is the way back the manual promises for
    /// everything that folds. Going through [`Op::Fold`] is what makes the close a
    /// fold rather than a very small pane, and it costs nothing: these operations
    /// write no session record either way (`karakuri-operation-record` answers
    /// `Silent::Surface` for all four).
    Pane(Op),
    /// A fader turned the drag into one operation of the vocabulary, which is the
    /// whole of what it did.
    ///
    /// Nothing was written anywhere: the value is the engine's, this crate has no
    /// engine (ADR-0156), and *applying* it is the caller's — a record, and the
    /// same record every other surface's control ends in
    /// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    ///
    /// One value and not two. Which deck and which control are inside the operation
    /// already, and a copy of either beside it would be two statements about one
    /// thing — [`crate::view::StripBox`]'s rule about a fill stored beside its
    /// track.
    Fader(Operation),
}

/// What a release left behind: the boundary that was in hand, and where it came
/// to rest.
///
/// [`Panel::released`] returns `None` when nothing was in hand, because a
/// release that lets go of nothing did nothing.
///
/// Not `Copy`, since [`Released::Dropped`] landed, and that is the `String` in
/// a Set's name arriving here rather than a decision taken about this enum: an
/// operation naming a Set carries the name, `karakuri-operation` is a leaf
/// crate that spells one as a `String`, and a release that answered with
/// anything less would be this module inventing an id type for a store it
/// cannot see (ADR-0156). Nothing else here changed shape — see [`Carrying`].
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
    /// A fader was let go, and which one — the deck inside the [`Knob`] where it is
    /// a deck's.
    ///
    /// There is no value here, and that is the decision rather than an omission.
    /// Where a boundary comes to rest is read back out of the layout, which owns
    /// it; where a fader comes to rest is the *deck's*, and the last thing this
    /// drag asked for was emitted when it was asked for. A value reported here
    /// would be this crate answering a question about somebody else's state, which
    /// is the second copy the whole module is written to refuse.
    Let { knob: Knob },
    /// A carried Set was let go over a rectangle that names a deck, and the load it
    /// asks for.
    ///
    /// Two sets of rectangles name one: a mixer strip
    /// ([`crate::view::Mixer::dropped`]) and a deck preview cell
    /// ([`crate::view::ProgramBay::dropped`]), which is `console.html`'s *"the same
    /// command rather than a second command"* and arrives here as the one `deck`
    /// the caller resolved. Which of the two it was is not in this value and is not
    /// owed one: the operation names a deck and a Set, and a copy of the route
    /// beside it would be a third operand nothing downstream can use
    /// ([ADR-0273](../../../docs/adr/0273-the-carry-lands-on-two-sets-of-rectangles-and-wears-a-face.md)).
    ///
    /// The one release on this panel that carries a value, and
    /// [`Let`](Released::Let) above is why it is not a contradiction. A fader
    /// emitted every value it asked for while it was moving, so a value here would
    /// be a second copy of the last of them; a carry emits nothing while it moves,
    /// because a Set half-way to a strip has not been loaded anywhere. The drop is
    /// the whole of what the gesture asks for, so it is asked for here or nowhere.
    ///
    /// Which deck is the caller's answer and not this module's, handed to
    /// [`Panel::released`] — see there, and [`Grab`] for the same seam at the other
    /// end of a gesture.
    ///
    /// Nothing is refused. A drop on a deck that is live replaces what the room is
    /// watching and this still asks for it: *"what may be asked for is the
    /// instrument's to decide"* (`docs/manual/console.html`, *How a Set reaches a
    /// deck*), and the gate is where that answer is
    /// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    /// This drag has no reading of residency at all — a strip's `tally` is not
    /// asked, here or in [`crate::view::Mixer::dropped`] — so there is nowhere for
    /// a second rule to hide.
    Dropped(Operation),
    /// A carried Set was let go over nothing, and nothing is asked for.
    ///
    /// The third outcome a release can have, and it exists because a carry is the
    /// one gesture on this panel that can be cancelled: a boundary dragged off its
    /// track still lands somewhere legal and a fader dragged past its end is still
    /// at its end, so both of them come to rest whatever the pointer did. A row let
    /// go over the transport row, over another bay, or off the viewport has nowhere
    /// to land, and a load aimed at the nearest strip would be a deck nobody
    /// pointed at. A cell whose letter names no slot is one of those places: the
    /// row is four cells whatever the deck holds, so the fourth cell of a
    /// three-slot deck is a rectangle with nothing behind the letter on it
    /// ([`crate::view::ProgramBay::dropped`]).
    ///
    /// The Set is named because the caller has a sentence to say, and saying
    /// nothing at all is the failure this arm is against: a gesture that is picked
    /// up, carried and then silently forgotten reads as a panel that missed the
    /// press.
    Nowhere { set: String },
    /// A carried row was let go over a rectangle that is a target, and the target
    /// will not take it.
    ///
    /// The one refusal a release on this panel makes, and it is about the
    /// *payload* rather than the destination: the master chain's list is a target
    /// for every carry and holds `kind L5` procedures, so a row that is not one is
    /// refused (ADR-0340). The mark says *where* and never *whether* (ADR-0273),
    /// so the ring is drawn and the refusal comes at the release.
    ///
    /// The reason travels with it
    /// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).
    /// What kinds a library row has is the view's reading, so the caller supplies
    /// it.
    Refused { set: String, why: &'static str },
}

/// Where a carried row was let go. The caller answers it: which rectangle a point
/// is in is the view's derivation and not this module's.
///
/// Three sets of rectangles name a landing: a mixer strip and a deck preview cell
/// each name a deck, and the Master bay's chain list names the chain
/// ([ADR-0273](../../../docs/adr/0273-the-carry-lands-on-two-sets-of-rectangles-and-wears-a-face.md)).
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

/// What an [`Op`] did.
///
/// One enum over every operation rather than one type each, because a caller
/// dispatches an `Op` from a key or a message and matches the outcome in the
/// same place. Which variant an operation can produce is on the operation.
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
    /// [`Panel::restore`]: a saved arrangement is in, at the viewport the window
    /// already had.
    ///
    /// No name, where the operation that asked for it carries one: this crate never
    /// saw the name. It was handed a `Layout` by whoever read the store, and an
    /// outcome that repeated a name back would be repeating the caller's own
    /// argument to it.
    Restored,
    /// [`Op::Report`].
    Report(Vec<Placement>),
    /// The operation had nothing to act on: [`Op::FoldEnclosing`] on the root,
    /// which is the one node with nothing enclosing it.
    ///
    /// It used to be the answer to *nothing under the pointer* as well, and that
    /// half of it moved out to the caller with the pointer itself — see [`Op`] and
    /// [`Panel::under`]. What is left is the case that is about the arrangement
    /// rather than about where a hand is.
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
    /// A Set, carried out of the Library bay. No axis for the fader's reason and no
    /// payload for [`Panel::cursor`]'s: which Set is in hand is the drag's, and a
    /// caller holding a copy of it would be shadowing the drag it is about to ask
    /// to end.
    ///
    /// It is a third answer rather than a second `Fader`, and what needs it is a
    /// caller deciding what a *move* meant: a move with a fader in hand can emit an
    /// operation and a move with a carry in hand never can, so a window loop that
    /// could not tell them apart would answer the repaint question for one of them
    /// wrongly. `crates/karakuri/src/main.rs` is where that is asked.
    Carrying,
}
