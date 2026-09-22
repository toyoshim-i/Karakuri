use karakuri_engine::binding::Curve;
use karakuri_engine::deck::{Blend, Mask, MaskKind, Residency};
use karakuri_engine::transport::Transport;
use karakuri_engine::Look;
use karakuri_signal::oscillator::Oscillator;
use karakuri_store::record::{DeckSlot, Record};

use super::change::MASK_SOFTNESS;
use super::translate::*;

/// The look that is running, as the reading the conversion needs.
///
/// `Operation::SetExposure` carries an exposure and nothing else, because that
/// is what a control change can say; `Record::Look` carries all three because
/// that is what a replay can reconstruct a session from. This is what closes
/// the gap between them, and it is the argument of ADR-0192 in one function.
pub fn current_look(look: &Look) -> karakuri_operation_record::Look {
    karakuri_operation_record::Look {
        tonemap: tonemap(look.op),
        exposure: look.exposure,
        white_point: look.white_point,
    }
}

/// The shape every scheduled fade takes, which is what [`current_transition`]
/// is handed and what `Record::Transition`'s `curve` ends up spelling.
///
/// `smooth` rather than `lin`, and the reason is in `binding.rs`: its
/// derivative is zero at both ends, so a fade neither jumps off the floor nor
/// slams into the ceiling. A crossfade of two linear ramps has a visible corner
/// at each end; two smooth ones do not. On no control anywhere, because the
/// other three curves are for *signals* — a fade wants easing and nothing else,
/// and a fourth cycling key for a choice nobody would revisit is a key in the
/// way.
///
/// It is here rather than in each surface, and that is the one thing that
/// changed about it: `karakuri-cli` held it as a private const of its own, and
/// the day `crates/karakuri`'s window began scheduling moves too there would
/// have been two copies of one decision with nothing holding them together —
/// two surfaces easing the same fade differently, in a value a replay carries
/// (`docs/contributing.md` §4). Both binaries already reach this module for
/// [`current_transition`], so this is where the one copy goes.
pub const FADE_CURVE: Curve = Curve::Smooth;

/// What one slot's clock is doing, as the reading the conversion needs.
/// `Operation::ScrubDeck` moves the scrub by an amount and `Record::Transport`
/// is absolute, so the conversion reads where the slot is.
pub fn current_transport(transport: &Transport) -> karakuri_operation_record::Transport {
    karakuri_operation_record::Transport {
        sync: sync(transport.sync()),
        anchor_bpm: transport.anchor_bpm(),
        scrub_beats: transport.scrub_beats(),
    }
}

/// The tempo the room is going at, as the reading the conversion needs.
///
/// `Operation::SetSync` anchors a slot at the session tempo, because engaging a
/// mode must not move the picture: the material is at 1x at that instant and
/// stays there until the room's tempo does. `karakuri-operation-record` cannot
/// reach the oscillator any more than it can reach the engine, so the tempo is
/// handed in, and this is the fourth of these.
///
/// It takes the oscillator rather than an `f32`, and that is the whole of the
/// function. `Transport::engaged` clamps the anchor into
/// [`karakuri_signal::oscillator::BPM_RANGE`] and the conversion does not clamp
/// at all; the two agree because an `Oscillator`'s tempo is already inside that
/// range — `Oscillator::new` and `Oscillator::correct` are its only writers and
/// both clamp — so a caller cannot reach a value where the clamp would fire
/// without first writing down a tempo no session ever reported. Asking for the
/// grid rather than a number is what makes that structural instead of a hope
/// (`docs/contributing.md` §4), and what is left over is held by
/// [`tests::a_sync_mode_writes_exactly_what_the_engine_would_engage`].
pub fn current_tempo(grid: &Oscillator) -> f32 {
    grid.bpm()
}

/// The mask a slot is wearing, as the reading the conversion needs.
///
/// `Operation::SetMaskShape` carries a shape and `Operation::SetMaskPosition`
/// carries a position, because those are the two things a surface can say
/// separately; `Record::Mask` carries both and the soft edge, because that is
/// what a replay can reconstruct a picture from. This is what closes the gap
/// between them, on [`current_look`]'s terms.
pub fn current_mask(mask: Mask) -> karakuri_operation_record::Mask {
    karakuri_operation_record::Mask {
        kind: wipe_kind(mask.kind()),
        angle: mask.angle(),
        position: mask.position(),
        // **The soft edge is this program's rather than the slot's**, and it
        // is the one field here that is not read back. No operation names a
        // softness — it has one constant behind it and no control, which is
        // why it is not in the vocabulary — and a slot nobody has masked
        // reports the engine's default of zero, so reading it back would give
        // the first wipe of a run the hard aliased front `MASK_SOFTNESS`
        // exists to not have. What this program writes is what it has always
        // written.
        softness: MASK_SOFTNESS,
    }
}

/// What the next scheduled move means, as the reading the conversion needs.
///
/// `Operation::FadeDeck` carries a deck and a destination, because that is what
/// a control can say; `Record::Transition` carries the instant, the length and
/// the shape too, because that is what a replay can reconstruct a move from.
/// The three that are missing are `Operation::SetTransition`'s — a surface's
/// own setting deciding what the *next* fade means — so they are handed in, and
/// this is the fifth of these.
///
/// It takes the grid and a quantum rather than a start, and that is the whole
/// of the function. `karakuri_engine::transition::quantise` is where the next
/// musical instant is decided, once, at the moment the operator asked;
/// `karakuri-operation-record` cannot reach it any more than it can reach the
/// oscillator, and a conversion that divided by a quantum of its own would be a
/// second grid. Asking for the oscillator and the quantum instead of a beat
/// count is what makes that structural rather than a hope
/// (`docs/contributing.md` §4), which is [`current_tempo`]'s arrangement
/// exactly.
///
/// A caller with no opinion about the grid gives a quantum of 0, which
/// `quantise` documents as *"now"* and answers with the beat count it was
/// handed — so ASAP is a setting a surface already has rather than anything
/// this signature had to invent. See
/// [`tests::a_quantum_of_zero_starts_the_move_on_the_beat_it_was_asked_on`].
///
/// The wipe shape is the fourth setting to come through here, and it is the
/// third of `Operation::SetTransition`'s three. `mask` and `angle` are what the
/// `z` key holds — the shape the *next* wipe takes, never the shape a deck's
/// layer is wearing — and they arrive by this route for the reason the quantum
/// and the length do: all three are one operation's, that operation writes no
/// record, and a surface is the only thing holding them. `Operation::Wipe` is
/// the one conversion that reads them, and it reads the soft edge off the deck
/// instead, through [`current_mask`].
///
/// The engine's `MaskKind` as the vocabulary's, on the way in. The caller hands
/// over what it is holding and [`wipe_kind`] is the one place the two lists are
/// made to agree, exactly as [`curve`] is for the shape of the move.
pub fn current_transition(
    grid: &Oscillator,
    quantum: f64,
    beats: f64,
    shape: Curve,
    mask: MaskKind,
    angle: f32,
) -> karakuri_operation_record::Transition {
    karakuri_operation_record::Transition {
        start: karakuri_engine::transition::quantise(grid.beats(), quantum),
        beats,
        curve: curve(shape),
        wipe_kind: wipe_kind(mask),
        wipe_angle: angle,
    }
}

/// Where a slot already sits in the mix, as the reading the conversion needs —
/// the sixth of these, and the one that is read so a record can be left *out*.
///
/// `Operation::Wipe` puts the deck it reveals under `over` and on air, and both
/// of those are a state the deck may be in already. Under `add` or under `max`
/// the same gesture is a wipe *on* rather than a wipe *over* — a different
/// picture and a legitimate one — so a wipe writes the blend mode only where
/// the slot is still at the mode a slot starts in, and the put-on-air only
/// where the slot is not already live. That is the decision `karakuri-cli`'s
/// `c` made for itself while it built those records by hand; the conversion has
/// no deck to ask, so what it needs is this
/// ([P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md):
/// safety is never bought with the operator's authority).
///
/// What the deck reports, not what it was asked for. `Deck::residency` answers
/// the level the slot is at — the governor may hold one below the request —
/// which is the same value the surface reading this used to compare against,
/// and the right one: what a wipe needs to know is whether the put-on-air it is
/// about to write would change anything.
///
/// Both values cross into the vocabulary's lists on the way, through
/// [`blend_mode`] and [`residency`], which is [`current_transition`]'s
/// arrangement for the wipe shape and [`current_mask`]'s for the mask's.
pub fn current_mix(blend: Blend, level: Residency) -> karakuri_operation_record::Mix {
    karakuri_operation_record::Mix {
        blend: blend_mode(blend),
        residency: residency(level),
    }
}

/// The output look, as the record that carries it.
pub fn look_record(look: &Look) -> Record {
    Record::Look {
        op: op_wire_name(look.op).to_string(),
        exposure: look.exposure,
        white_point: look.white_point,
    }
}

/// What the run renders at, as the record that carries it.
///
/// There is no decoder beside this writer, and that asymmetry is the design
/// rather than a gap: everything else here round-trips through [`change`]
/// because it can be applied to a running deck, and a canvas cannot. A replay
/// reads this before it builds anything — see `session::Session::canvas`.
pub fn canvas_record(width: u32, height: u32) -> Record {
    Record::Canvas { width, height }
}

/// A slot's transport, as the record that carries it.
pub fn transport_record(slot: usize, transport: &Transport) -> Record {
    Record::Transport {
        slot: DeckSlot(slot as u8),
        sync: transport.sync().name().to_string(),
        anchor_bpm: transport.anchor_bpm(),
        scrub_beats: transport.scrub_beats(),
    }
}
