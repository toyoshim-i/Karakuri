//! The mix, through the record stream on the way.
//!
//! The faders, the blend modes, residency and the output look are the state an
//! operator moves during a performance and the only state the engine had no
//! record vocabulary for at all, and the gap was written down rather than
//! papered over. A
//! session that replayed everything else would replay the material and not the
//! *performance*: the same Sets, on the same beat, all at whatever gain they
//! happened to start at, with nothing ever going on or off air.
//!
//! ```text
//!   a key press ─→ Record::Gain      ─┐
//!                  Record::Opacity    │
//!                  Record::Blend      │
//!                  Record::Preview    ┼→ Change ─→ Deck / Present
//!                  Record::Residency  │
//!                  Record::Look      ─┘
//! ```
//!
//! **Built and read back, never applied directly**, which is the same
//! arrangement `karakuri-environment`'s `audio.rs` has and is there for the
//! same reason: the path the
//! engine is driven through is the record's rather than one that happens to
//! agree with it. A decode that only a test exercises is a decode that is
//! correct until the day it matters.
//!
//! ## The `String`s here are on the frame path, and this is what they cost
//!
//! This used to say they were not: a fader moved when a hand moved it, so a
//! record's `String` was a key press's allocation. The MIDI map ended that.
//! `Live::run_surface` is called from `Live::frame`, and `crate::midi` puts the
//! number on it — *"A fader sweep is several hundred messages, this runs inside
//! `Live::frame`"* — so a swept control's record is built once per message, on
//! the render thread.
//!
//! **What allocates is the record and nothing either side of it.**
//! `karakuri_midi::map` says of its own routing that nothing there allocates,
//! every operation a map line can name carrying scalars only, and
//! `session::Recorder::push` allocates nothing and never blocks. Of the four
//! continuous targets a map line can reach, `gain` and `opacity` become records
//! of scalars and touch no heap at all. The other two become a record carrying
//! a name — `exposure` writes `Record::Look` with the tone map operator's, and
//! `mask-position` writes `Record::Mask` with the shape's — each a `String`
//! copied from a `&'static str` of at most eight bytes. `Live::record` clones
//! the record for the recorder, so with `--record-session` attached it is two
//! such allocations per message and otherwise one.
//!
//! **What bounds it is the message count, not the value range.** A control
//! change is 7-bit, so a sweep passes through at most 128 *distinct* values —
//! but nothing between the port and here drops a repeat, and a knob held
//! against its stop keeps sending, so what arrives is however many messages the
//! device sends: a few hundred a second, which is the figure `midi::INBOX` is
//! sized from. That is a few hundred eight-byte allocations a second at worst,
//! twice that while recording, each freed in the same frame or on the writer
//! thread, with no lock and nothing unbounded in it.
//!
//! **Bounded is not the same as allowed, and it is the message count that is
//! now gone.** The figures above are what a sweep *would* cost and are why:
//! [P-0001](../../../docs/principles/0001-nothing-allocates-or-compiles-a-shader-on-the-render-thread.md)
//! says the frame path allocates no heap memory, with no clause for a small
//! one, so the question a byte-sized allocation per message raised was settled
//! by taking away the *per message* rather than by writing the clause.
//! `crate::midi`'s router **coalesces a continuous control per frame** — the
//! last value a fader sent within a frame is the one that becomes an
//! operation — so a sweep builds one of these records a frame, and two only
//! while recording. A pad is untouched, because two presses in one frame are
//! two things that happened
//! (`docs/adr/0207-a-continuous-control-says-one-thing-per-frame.md`, which
//! also carries what the record stream loses and the two alternatives it
//! turned down: moving the engine's list of names into `karakuri-store`, and
//! an exception in the first rule with no measured threshold behind it).
//!
//! The numbers are kept rather than deleted because they are the reason the
//! coalescer exists, and whoever removes it should meet them.
//! `karakuri-environment`'s `audio.rs`'s
//! record is reused in place because that one was on the frame path from the
//! start; this one arrived on it later and is now on it a frame at a time.
//!
//! ## What of this moved to the vocabulary, and what did not
//!
//! **`gain_record` and `preview_record` are gone**, and they are gone rather
//! than deprecated: their whole content was `Record::Gain { slot, value }` and
//! `Record::Preview { slot }`, which is now what
//! `karakuri_operation_record::written` answers for `Operation::SetGain` and
//! `Operation::SetPreview`. Two derivations of one record is the drift this
//! module was written to end, in miniature, so the second one went.
//!
//! **`opacity_record`, `blend_record` and `residency_record` went the same way,
//! and what moved was not a conversion but a reading of what a gesture is made
//! of.** They were the last three records this program built twice, and their
//! second caller was `crossfade` and `wipe` — one operation each and four or
//! five records each. That count is why the *gestures* cannot convert; it was
//! never a reason their **parts** could not. Silencing the incoming deck is
//! `Operation::SetOpacity`, forcing `over` is `Operation::SetBlendMode` and
//! putting it on air is `Operation::SetResidency`, whatever the gesture around
//! them still owes. So each gesture asks `Live::operate` for the parts that are
//! decided and builds only the parts that are not, and the second derivation is
//! gone rather than kept in step by a test.
//!
//! **`mask_record` went the same way, and it is the one that needed a page
//! change first.** It was `wipe`'s and had no operation at all; the mask now
//! has two — a shape and a position, because a row carrying both could only
//! ever be reached by a press
//! (`docs/adr/0201-the-mask-is-two-rows-because-a-control-change-can-only-set.md`)
//! — so the gesture asks `Live::operate` for each of them and the hand-built
//! record is gone. It writes two `Record::Mask` where it wrote one, which is
//! what routing it honestly costs: each row writes the record whole.
//!
//! **The rest are still here and each has a reason.** `transition_record` is
//! `fade_slot`'s and `wipe`'s and `select_record` is `cycle_renderer`'s: `Operation::FadeDeck`,
//! `Operation::Crossfade`, `Operation::Wipe` and `Operation::SelectRenderer`
//! all need the grid quantised onto a musical instant, plus the quantum and the
//! length that `Operation::SetTransition` sets and no record carries.
//! `look_record` builds the launch look, which is a complete look rather than
//! an ask. `canvas_record` names a record no operation writes. Each of them
//! goes the day its operation's conversion is settled — see ADR-0194.
//!
//! **`transport_record` is the one that has already gone, and it did not get
//! deleted.** It was `cycle_sync`'s, for an anchor clamp the vocabulary was
//! thought to have no way to apply; `Operation::SetSync` now converts, so `y`
//! routes through `Live::operate` like every other settled key and this
//! function has no gesture behind it. What it is now is the **engine's side of
//! that record** — a `Transport` as the `Record::Transport` that carries it —
//! which is exactly what [`current_tempo`]'s test needs to hold the conversion
//! against `Transport::engaged`. A derivation kept as the thing a second
//! derivation is checked against is not a second derivation.
//!
//! ## Opacity, which used to be deliberately not here
//!
//! `Deck::set_opacity` existed with no key, no flag and no record, and this
//! module said so: a record type for a control the operator cannot move is one
//! more record nobody writes, which is the condition it exists to end rather
//! than extend. **It got a record when it got a control**, and it got a control
//! when [`karakuri_engine::deck::Blend`] made it mean something a gain does not
//! — the fader across the blend rather than the level the material arrives at.
//! Under `add` the two multiply together and a stream carrying either would
//! replay the same; under `over` one dims a deck slot's layer and the other
//! stops it hiding what is beneath.

use karakuri_engine::binding::Curve;
use karakuri_engine::deck::{Blend, Mask, MaskKind, Residency};
use karakuri_engine::present::TonemapOp;
use karakuri_engine::set::Authority;
use karakuri_engine::transition::Control;
use karakuri_engine::transport::{Sync, Transport};
use karakuri_engine::Look;
use karakuri_signal::oscillator::Oscillator;
use karakuri_store::record::Record;

/// What one mix record says, decoded into what the engine takes.
///
/// The engine's own types, not the record's: a `Residency` rather than the
/// string it was spelled with, a [`Look`] rather than three loose fields. That
/// is where the decode ends and it is the whole of what the caller applies.
#[derive(Clone, Copy, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub enum Change {
    Gain {
        slot: usize,
        value: f32,
    },
    /// The fader. Separate from `Gain` because the blend mode makes them
    /// separate — see [`Blend`].
    Opacity {
        slot: usize,
        value: f32,
    },
    Blend {
        slot: usize,
        mode: Blend,
    },
    Mask {
        slot: usize,
        mask: Mask,
    },
    /// Which slot the output is showing, or `None` for the mix. Not a mix
    /// control; see [`Record::Preview`] for why it is in the stream anyway and
    /// for when it will stop being.
    Preview {
        slot: Option<usize>,
    },
    /// A scheduled move. Carried as its parts rather than as a
    /// `karakuri_engine::Transition`, because building one needs the value the
    /// control is at *now* and that is the applier's to read, not the decoder's.
    Transition {
        slot: usize,
        control: Control,
        to: f32,
        start: f64,
        beats: f64,
        curve: Curve,
    },
    /// **Which renderer of a slot's Set becomes the live one**, at a musical
    /// instant. Carried as its parts rather than as a
    /// `karakuri_engine::transition::Selection` for [`Change::Transition`]'s
    /// reason — the applier is the one holding the deck.
    ///
    /// **The renderer is not checked here.** This decoder knows how many slots
    /// the deck has and nothing about what is in them; how many renderers a
    /// slot draws with is a property of the Set it is playing, which the
    /// applier has in hand and this does not. It is checked there, in
    /// [`crate::no_such_renderer`]'s words.
    Select {
        slot: usize,
        renderer: usize,
        start: f64,
    },
    Residency {
        slot: usize,
        level: Residency,
    },
    Look(Look),
    /// What a slot's clock does with the session's. Carried as a value rather
    /// than applied as a mode change, because the record says all three and a
    /// replay must not recompute one of them from the machine it is on.
    Transport {
        slot: usize,
        sync: Sync,
        anchor_bpm: f32,
        scrub_beats: f64,
    },
}

/// **The engine's list, as the vocabulary's** — one function per list, and
/// the one place the two copies of each are made to agree.
///
/// `karakuri-operation` owns a copy of every list a destination is drawn from,
/// which is the cost P-0074 says the vocabulary pays: *"The two rules — be
/// engine-neutral, and have no toggles — are not jointly satisfiable unless
/// the vocabulary owns the lists."* A copy needs somewhere the two meet, and
/// this is that place: this package is where the two are seen together,
/// because a record is what the engine is driven through here and the
/// vocabulary is what every surface asks in.
///
/// **`From` impls, which is what ADR-0180 said, are not available here.** Both
/// types are foreign to this package — `Blend` is `karakuri-engine`'s and
/// `BlendMode` is `karakuri-operation`'s — so the orphan rule refuses the impl
/// and there is nothing to be done about it short of one of those two crates
/// depending on the other, which is the thing neither of them may do. Plain
/// functions, then, exactly as the panel program's own
/// `blend_mode` already is. See ADR-0194.
///
/// **A match apiece, so a value added to the engine stops the build here**
/// rather than reaching a surface that draws a chip nothing can read. That is
/// `Blend::name`'s argument and `residency_wire_name`'s, applied to a list
/// instead of to a spelling.
pub fn blend_mode(blend: Blend) -> karakuri_operation::BlendMode {
    match blend {
        Blend::Add => karakuri_operation::BlendMode::Add,
        Blend::Over => karakuri_operation::BlendMode::Over,
        Blend::Max => karakuri_operation::BlendMode::Max,
    }
}

/// The engine's residency level, as the vocabulary's. See [`blend_mode`].
pub fn residency(level: Residency) -> karakuri_operation::Residency {
    match level {
        Residency::Live => karakuri_operation::Residency::Live,
        Residency::Priming => karakuri_operation::Residency::Priming,
        Residency::Allocated => karakuri_operation::Residency::Allocated,
    }
}

/// The engine's mask shape, as the vocabulary's. See [`blend_mode`].
pub fn wipe_kind(kind: MaskKind) -> karakuri_operation::WipeKind {
    match kind {
        MaskKind::None => karakuri_operation::WipeKind::None,
        MaskKind::Linear => karakuri_operation::WipeKind::Linear,
        MaskKind::Radial => karakuri_operation::WipeKind::Radial,
    }
}

/// The engine's sync mode, as the vocabulary's. See [`blend_mode`].
pub fn sync(mode: Sync) -> karakuri_operation::Sync {
    match mode {
        Sync::Free => karakuri_operation::Sync::Free,
        Sync::Tempo => karakuri_operation::Sync::Tempo,
        Sync::Beat => karakuri_operation::Sync::Beat,
    }
}

/// The engine's tone map operator, as the vocabulary's. See [`blend_mode`].
pub fn tonemap(op: TonemapOp) -> karakuri_operation::Tonemap {
    match op {
        TonemapOp::Clamp => karakuri_operation::Tonemap::Clamp,
        TonemapOp::Reinhard => karakuri_operation::Tonemap::Reinhard,
        TonemapOp::Aces => karakuri_operation::Tonemap::Aces,
        TonemapOp::AgX => karakuri_operation::Tonemap::AgX,
    }
}

/// The engine's authority level, as the vocabulary's. See [`blend_mode`].
///
/// **Written before there is a reader for it**, which is why the lint has to be
/// told, and it is here anyway for the reason the five above it are here at
/// all: the two spellings have to be *checked* against each other somewhere,
/// this is the only crate that can see both, and the check below needs a
/// conversion to check. Nothing on the CLI's paths reads `Set::authority` yet —
/// the console's `man / sug / auto` chip and a live save that writes a
/// `Record::Authority` were the two readers this waited for, and **the chip
/// arrived on 2026-08-29** — the Inspector bay draws a node's authority, so this
/// has a caller outside the tests and the attribute it carried is gone.
pub fn authority(level: Authority) -> karakuri_operation::Authority {
    match level {
        Authority::Manual => karakuri_operation::Authority::Manual,
        Authority::Suggesting => karakuri_operation::Authority::Suggesting,
        Authority::Automatic => karakuri_operation::Authority::Automatic,
    }
}

/// **The look that is running, as the reading the conversion needs.**
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

/// **What one slot's clock is doing, as the reading the conversion needs.**
/// `Operation::ScrubDeck` moves the scrub by an amount and `Record::Transport`
/// is absolute, so the conversion reads where the slot is.
pub fn current_transport(transport: &Transport) -> karakuri_operation_record::Transport {
    karakuri_operation_record::Transport {
        sync: sync(transport.sync()),
        anchor_bpm: transport.anchor_bpm(),
        scrub_beats: transport.scrub_beats(),
    }
}

/// **The tempo the room is going at, as the reading the conversion needs.**
///
/// `Operation::SetSync` anchors a slot at the session tempo, because engaging
/// a mode must not move the picture: the material is at 1x at that instant and
/// stays there until the room's tempo does. `karakuri-operation-record` cannot
/// reach the oscillator any more than it can reach the engine, so the tempo is
/// handed in, and this is the fourth of these.
///
/// **It takes the oscillator rather than an `f32`, and that is the whole of
/// the function.** `Transport::engaged` clamps the anchor into
/// [`karakuri_signal::oscillator::BPM_RANGE`] and the conversion does not
/// clamp at all; the two agree because an `Oscillator`'s tempo is already
/// inside that range — `Oscillator::new` and `Oscillator::correct` are its
/// only writers and both clamp — so a caller cannot reach a value where the
/// clamp would fire without first writing down a tempo no session ever
/// reported. Asking for the grid rather than a number is what makes that
/// structural instead of a hope
/// ([P-0026](../../../docs/principles/0026-a-guarantee-is-structural-or-it-is-a-convention-that-says-so.md)),
/// and what is left over is held by
/// [`tests::a_sync_mode_writes_exactly_what_the_engine_would_engage`].
pub fn current_tempo(grid: &Oscillator) -> f32 {
    grid.bpm()
}

/// **The mask a slot is wearing, as the reading the conversion needs.**
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

/// A scheduled move, as the record that carries it.
pub fn transition_record(
    slot: usize,
    control: Control,
    to: f32,
    start: f64,
    beats: f64,
    curve: Curve,
) -> Record {
    Record::Transition {
        slot: slot as u8,
        control: control.name().to_string(),
        to,
        start,
        beats,
        curve: curve.name().to_string(),
    }
}

/// A scheduled selection — which renderer of a slot goes live, and when — as
/// the record that carries it.
///
/// No length and no curve, and that is the record rather than an omission: a
/// selection is a choice and a choice is a cut. See
/// `karakuri_engine::transition::Selection`.
pub fn select_record(slot: usize, renderer: usize, start: f64) -> Record {
    Record::Select {
        slot: slot as u8,
        renderer: renderer as u32,
        start,
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
        slot: slot as u8,
        sync: transport.sync().name().to_string(),
        anchor_bpm: transport.anchor_bpm(),
        scrub_beats: transport.scrub_beats(),
    }
}

/// Every residency level there is, with its length in its type. `Residency` is
/// the engine's and has no iterator, so this is the list — and
/// [`residency_wire_name`] below is the exhaustive match that stops a level
/// from reaching the wire without a name.
pub const LEVELS: [Residency; 3] = [Residency::Live, Residency::Priming, Residency::Allocated];

/// The wire spelling of a residency level. Lower case and stable; the status
/// line's `LIVE`/`prim`/`park` are a different vocabulary for a different
/// reader and are deliberately not this one.
///
/// A match rather than a table lookup, so a level added to the engine does not
/// compile until it has a spelling. [`parse_residency`] is derived from this
/// one over [`LEVELS`], so the two directions cannot disagree — the remaining
/// hand-written thing is `LEVELS` itself, and a level missing from it is a
/// record that fails to decode with a message naming what was available,
/// rather than one that decodes as the wrong level.
pub fn residency_wire_name(level: Residency) -> &'static str {
    match level {
        Residency::Live => "live",
        Residency::Priming => "priming",
        Residency::Allocated => "allocated",
    }
}

/// **A wire spelling back to the engine's residency** — [`residency_wire_name`]
/// read the other way, over [`LEVELS`], so the two directions cannot disagree.
///
/// **`pub` for a second surface.** [`change`] below is the one caller in this
/// crate; the other is the panel program, whose `apply` decodes a
/// `Record::Residency` a control just wrote. That program transcribed these
/// three words for as long as they lived in a package with no library target
/// (ADR-0214), which is the transcription this `pub` deletes rather than
/// carries.
pub fn parse_residency(name: &str) -> Option<Residency> {
    LEVELS
        .iter()
        .copied()
        .find(|level| residency_wire_name(*level) == name)
}

/// Every wire spelling, for an error message that says what was available.
fn residency_wire_names() -> String {
    LEVELS
        .iter()
        .map(|level| residency_wire_name(*level))
        .collect::<Vec<_>>()
        .join(", ")
}

/// One mix record as the change it asks for.
///
/// Three answers, and they are three different things:
///
/// - `Ok(None)` — **not a mix record.** A `tick` or an `audio` is not this
///   module's to act on and not an error either.
/// - `Err(_)` — **a mix record this build cannot obey.** An unknown residency
///   level, an unknown tone map operator, a slot the deck does not have. The
///   decoder does not reject these; it reports them, because what a name is
///   allowed to be is the engine's business and only the engine can say what
///   the alternatives were. A stream from a newer build reaches here, not the
///   parser.
/// - `Ok(Some(_))` — what to do.
///
/// `slot_count` is the deck's, so a record naming a slot that does not exist is
/// caught here rather than panicking in an index four frames later.
pub fn change(record: &Record, slot_count: usize) -> Result<Option<Change>, String> {
    // **The keys' own refusal**, from [`crate::no_such_slot`] rather than
    // spelled again here. It was spelled again — `slot 9:` where every other
    // surface says `no slot 9:` — and a stream naming a slot this deck does not
    // hold is the same mistake as a hand or a model naming one.
    let in_range = |slot: u8| -> Result<usize, String> {
        let slot = usize::from(slot);
        if slot < slot_count {
            Ok(slot)
        } else {
            Err(crate::no_such_slot(slot, slot_count))
        }
    };
    match record {
        Record::Gain { slot, value } => Ok(Some(Change::Gain {
            slot: in_range(*slot)?,
            value: *value,
        })),
        Record::Opacity { slot, value } => Ok(Some(Change::Opacity {
            slot: in_range(*slot)?,
            value: *value,
        })),
        Record::Blend { slot, mode } => {
            let slot = in_range(*slot)?;
            let mode = Blend::from_name(mode).ok_or_else(|| {
                format!(
                    "blend `{mode}` — expected {}",
                    Blend::ALL
                        .iter()
                        .map(|b| b.name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
            Ok(Some(Change::Blend { slot, mode }))
        }
        Record::Mask {
            slot,
            kind,
            angle,
            position,
            softness,
        } => {
            let slot = in_range(*slot)?;
            let kind = MaskKind::from_name(kind).ok_or_else(|| {
                format!(
                    "mask `{kind}` — expected {}",
                    MaskKind::ALL
                        .iter()
                        .map(|k| k.name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
            Ok(Some(Change::Mask {
                slot,
                // The numbers are clamped rather than refused: unlike a
                // transition's start, every value outside the range has one
                // sensible reading — a front past the end has arrived, and one
                // before the start has not. `Mask::new` is where that lives, so
                // a record and a key press cannot disagree about it.
                mask: Mask::new(kind, *angle, *position, *softness),
            }))
        }
        Record::Transition {
            slot,
            control,
            to,
            start,
            beats,
            curve,
        } => {
            let slot = in_range(*slot)?;
            let control = Control::from_name(control).ok_or_else(|| {
                format!(
                    "transition control `{control}` — expected {}",
                    Control::ALL
                        .iter()
                        .map(|c| c.name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
            let curve = Curve::parse(curve).ok_or_else(|| {
                format!(
                    "transition curve `{curve}` — expected {}",
                    karakuri_engine::binding::CURVES
                        .iter()
                        .map(|c| c.name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
            // **The numbers, not only the names.** A `start` that is not a
            // number is a control pinned forever — `finished` is never true
            // past it — and a negative duration is a move that ends before it
            // begins. The engine clamps both as a backstop; this is where an
            // operator can be told, which is the whole reason a decode reports
            // rather than rejects.
            if !start.is_finite() {
                return Err(format!("transition start `{start}` is not a position"));
            }
            if !(beats.is_finite() && *beats >= 0.0) {
                return Err(format!(
                    "transition length `{beats}` — expected a number of beats, or 0 for a cut"
                ));
            }
            if !to.is_finite() {
                return Err(format!("transition to `{to}` is not a value"));
            }
            Ok(Some(Change::Transition {
                slot,
                control,
                to: *to,
                start: *start,
                beats: *beats,
                curve,
            }))
        }
        Record::Select {
            slot,
            renderer,
            start,
        } => {
            let slot = in_range(*slot)?;
            // **The number, on the transition's terms.** A start that is not a
            // position is a selection that never lands and never leaves the
            // queue — there is no `finished` to clear it — so it is refused
            // here, where an operator can be told, and clamped in the engine as
            // a backstop.
            if !start.is_finite() {
                return Err(format!("selection start `{start}` is not a position"));
            }
            Ok(Some(Change::Select {
                slot,
                renderer: *renderer as usize,
                start: *start,
            }))
        }
        Record::Preview { slot } => Ok(Some(Change::Preview {
            slot: slot.map(in_range).transpose()?,
        })),
        Record::Residency { slot, level } => {
            let slot = in_range(*slot)?;
            let level = parse_residency(level).ok_or_else(|| {
                format!("residency `{level}` — expected {}", residency_wire_names())
            })?;
            Ok(Some(Change::Residency { slot, level }))
        }
        Record::Transport {
            slot,
            sync,
            anchor_bpm,
            scrub_beats,
        } => {
            let slot = in_range(*slot)?;
            let sync = Sync::from_name(sync).ok_or_else(|| {
                format!(
                    "sync `{sync}` — expected {}",
                    Sync::ALL
                        .iter()
                        .map(|s| s.name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
            Ok(Some(Change::Transport {
                slot,
                sync,
                anchor_bpm: *anchor_bpm,
                scrub_beats: *scrub_beats,
            }))
        }
        Record::Look {
            op,
            exposure,
            white_point,
        } => {
            let op = parse_op(op)
                .ok_or_else(|| format!("tonemap `{op}` — expected {}", op_wire_names()))?;
            Ok(Some(Change::Look(Look {
                op,
                exposure: *exposure,
                white_point: *white_point,
            })))
        }
        // **A wildcard rather than an exhaustive match, and it is the one
        // place a new record is placed silently.** Everything not named above
        // is not this module's to act on — a `tick`, an `audio`, a
        // `param_decl` — and refusing what it does not recognise would make a
        // session written by a newer build unreplayable, which is the promise
        // the format makes and this arm keeps. The cost is that a new *deck*
        // record added to `Record` compiles here and quietly does nothing,
        // where `project::key_for`, `setfile::from_lines` and
        // `Record::vocabulary` would all stop compiling until it was
        // classified; the note in `Record::is_set_state` says so rather than
        // leaving that claim reading as absolute. Naming the ignored records
        // instead was rejected for the reason above: the list would have to
        // grow for every record in the format, including the ones no build
        // here has heard of.
        //
        // **`merge` lands here and is owed nothing, which is checked rather
        // than assumed.** It is `Vocabulary::Set` — see `Record::vocabulary` —
        // so `session::split` puts every one it meets before the first tick
        // into the head, and the head is decoded by `setfile::from_lines`,
        // which builds the replay's Set at the layering it names and folds it
        // to the renderer its `live` names. One arriving *after* a tick is a
        // stream saying a Set changed how its renderers meet each other
        // mid-performance, and there is no such move: a layering decides
        // whether an L5 and a target per renderer exist at all, so changing it
        // is a rebuild and a rebuild arrives as `procedure` records. Nothing
        // in this program writes one there — `Live::record` writes mix records
        // and `Live::record_procedure` writes `procedure` — and a deck has no
        // method that could obey it at a frame. So it is passed over exactly
        // as a `camera`, a `capacity` or a `seed` after the first tick is, and
        // for the same reason: it is a Set's fact arriving where a
        // performance's facts go.
        _ => Ok(None),
    }
}

/// How wide a wipe's soft edge is.
///
/// Not zero, and not a key. A hard front is an aliased staircase wherever it is
/// not axis-aligned, and this is the narrowest edge that hides that at the
/// resolutions this renders at — narrow enough that a wipe still reads as a
/// wipe rather than a gradient.
const MASK_SOFTNESS: f32 = 0.02;

/// Every tone map operator there is. The one list, and its length is in its
/// type, so adding an operator to it is a deliberate act rather than an
/// oversight in a `Vec`.
/// The record's own vocabulary for the output look, which is why it is here
/// rather than with the keys that cycle it.
pub const TONEMAPS: [TonemapOp; 4] = [
    TonemapOp::Clamp,
    TonemapOp::Reinhard,
    TonemapOp::Aces,
    TonemapOp::AgX,
];

/// Both of an operator's spellings: the one a stream and a flag use, and the
/// one a human reads.
///
/// **An exhaustive match, and that is the point.** There were two hand-written
/// lists — `--tonemap`'s parser and `op_name`'s display arm — and the `look`
/// record wanted a third. A lookup over a table would have been one list but
/// would still answer for an operator missing from it, by falling back to
/// something plausible; a match does not compile until every operator has both
/// names. Everything below derives from here, parsing included, so the two
/// directions cannot disagree.
fn spellings(op: TonemapOp) -> (&'static str, &'static str) {
    match op {
        TonemapOp::Clamp => ("clamp", "clamp"),
        TonemapOp::Reinhard => ("reinhard", "Reinhard"),
        TonemapOp::Aces => ("aces", "ACES"),
        TonemapOp::AgX => ("agx", "AgX"),
    }
}

/// How a human reads it. Free to be capitalised the way the papers are,
/// because nothing parses it.
pub fn op_name(op: TonemapOp) -> &'static str {
    spellings(op).1
}

/// How a stream and a flag spell it. Lower case, stable, and the only spelling
/// anything parses.
pub fn op_wire_name(op: TonemapOp) -> &'static str {
    spellings(op).0
}

/// The wire spelling back to an operator, by searching the one list with the
/// one spelling function. `None` for a name this build does not have, which is
/// the caller's to report against [`op_wire_names`].
pub fn parse_op(name: &str) -> Option<TonemapOp> {
    TONEMAPS
        .iter()
        .copied()
        .find(|op| op_wire_name(*op) == name)
}

/// Every wire spelling, for an error message that says what was available.
pub fn op_wire_names() -> String {
    TONEMAPS
        .iter()
        .map(|op| op_wire_name(*op))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The two copies of every list, checked against each other — and this
    /// is the only place in the workspace where that can happen.**
    ///
    /// `karakuri-operation` owns its own `BlendMode`, `Residency`, `Sync`,
    /// `Tonemap` and `Authority` because a vocabulary that refuses to name a
    /// value cannot say
    /// *set blend to over*, and the cost is stated rather than hidden: they
    /// are third spellings of lists the engine and the store already hold
    /// (P-0074, ADR-0180). A record carries the **name**, so a level spelled
    /// `prime` here and `priming` there is a record that decodes to a refusal
    /// on replay and moves nothing in the mix — a failure that would show up
    /// as a session replaying differently and nowhere earlier.
    ///
    /// `karakuri-operation` cannot check this (it has no engine, by charter)
    /// and neither can `karakuri-operation-record` (it has no engine either,
    /// deliberately). This package depends on both, so this is where the two
    /// lists meet and where they are made to agree.
    ///
    /// **Both directions per value, over the engine's own lists**, so a value
    /// added to the engine arrives here as a missing match arm in
    /// [`blend_mode`] and its neighbours rather than as a silent extra.
    #[test]
    fn the_vocabularys_copy_of_a_list_spells_it_the_way_the_store_reads_it() {
        for mode in Blend::ALL {
            assert_eq!(
                blend_mode(mode).name(),
                mode.name(),
                "the vocabulary and the engine spell one blend mode two ways, and \
                 `Record::Blend` carries the name"
            );
        }
        for level in LEVELS {
            assert_eq!(
                residency(level).name(),
                residency_wire_name(level),
                "the vocabulary and the record spell one residency level two ways, and \
                 `Record::Residency` carries the name"
            );
        }
        for mode in Sync::ALL {
            assert_eq!(
                sync(mode).name(),
                mode.name(),
                "the vocabulary and the engine spell one sync mode two ways, and \
                 `Record::Transport` carries the name"
            );
        }
        for op in TONEMAPS {
            assert_eq!(
                tonemap(op).name(),
                op_wire_name(op),
                "the vocabulary and the flag spell one tone map operator two ways, and \
                 `Record::Look` carries the name"
            );
        }
        // Over the engine's `ALL` and not the vocabulary's, which has none —
        // `karakuri_operation::Authority` says so at its `name`, on
        // `WipeKind`'s terms: that constant exists for a map target, and no map
        // line can say a node address. The engine's list is the one this has to
        // be exhaustive over anyway, for the reason stated above.
        for level in Authority::ALL {
            assert_eq!(
                authority(level).name(),
                level.name(),
                "the vocabulary and the engine spell one authority level two ways, and \
                 `Record::Authority` carries the name"
            );
        }
    }

    /// One record, as `karakuri-operation-record` writes it.
    ///
    /// **The canonical derivation**, and the only one this program has for
    /// these now: a test that wants a `Record::Opacity` asks the conversion
    /// for one rather than spelling it, so what it round-trips below is what a
    /// key press and a mapped pad actually write. Panics rather than returning
    /// nothing on an operation that is not one record, because a test silently
    /// given no record is a test that checks nothing.
    fn from_operation(operation: karakuri_operation::Operation) -> Record {
        from_operation_reading(operation, karakuri_operation_record::Current::default())
    }

    /// The same, for an operation whose record is not a function of the
    /// operation alone. The mask pair is the case: each half writes
    /// `Record::Mask` whole, so each needs the other half read back.
    fn from_operation_reading(
        operation: karakuri_operation::Operation,
        current: karakuri_operation_record::Current,
    ) -> Record {
        use karakuri_operation_record::Written;
        match karakuri_operation_record::written(&operation, &current) {
            Written::Records(records) if records.len() == 1 => {
                records.into_iter().next().expect("length just checked")
            }
            other => panic!("`{}` is not one record: {other:?}", operation.title()),
        }
    }

    /// A reading of a mask that is wearing exactly this, for the two
    /// operations that need one.
    fn reading_of(mask: Mask) -> karakuri_operation_record::Current {
        karakuri_operation_record::Current {
            mask: Some(karakuri_operation_record::Mask {
                kind: wipe_kind(mask.kind()),
                angle: mask.angle(),
                position: mask.position(),
                softness: mask.softness(),
            }),
            ..karakuri_operation_record::Current::default()
        }
    }

    /// **The records `crossfade` and `wipe` used to build by hand are the ones
    /// the conversion writes.**
    ///
    /// The two gestures asked this module for a `Record::Opacity`, a
    /// `Record::Blend` and a `Record::Residency` while their own operations
    /// were unsettled — and their own operations are unsettled still, because
    /// what is owed is the *scheduled move*, never the silencing or the
    /// put-on-air. Those three parts go through `Live::operate` now, and this
    /// is what says the change of route did not change a byte of what they
    /// write.
    ///
    /// **Literals on the right-hand side on purpose.** An expectation derived
    /// from `written` would assert that `written` equals itself; these are the
    /// records the deleted builders produced, spelled out, including the
    /// values the two gestures pass — `0.0` on the deck being silenced and
    /// `1.0` on the one arriving under a mask.
    #[test]
    fn the_records_the_gestures_built_by_hand_are_what_the_conversion_writes() {
        use karakuri_operation::Operation;

        assert_eq!(
            from_operation(Operation::SetOpacity {
                deck: 2,
                opacity: 0.0,
            }),
            Record::Opacity {
                slot: 2,
                value: 0.0,
            },
            "the silencing a crossfade writes is not what `mix::opacity_record` wrote"
        );
        assert_eq!(
            from_operation(Operation::SetOpacity {
                deck: 1,
                opacity: 1.0,
            }),
            Record::Opacity {
                slot: 1,
                value: 1.0,
            },
            "the opacity a wipe writes is not what `mix::opacity_record` wrote"
        );
        assert_eq!(
            from_operation(Operation::SetBlendMode {
                deck: 1,
                blend: blend_mode(Blend::Over),
            }),
            Record::Blend {
                slot: 1,
                mode: "over".to_string(),
            },
            "the blend mode a wipe forces is not what `mix::blend_record` wrote"
        );
        assert_eq!(
            from_operation(Operation::SetResidency {
                deck: 3,
                residency: residency(Residency::Live),
            }),
            Record::Residency {
                slot: 3,
                level: "live".to_string(),
            },
            "the put-on-air both gestures write is not what `mix::residency_record` wrote"
        );
    }

    use karakuri_engine::TonemapOp;

    /// The round trip, which is the whole claim: what the live path builds is
    /// what a replay would decode, for every mix record there is.
    #[test]
    fn every_mix_record_survives_the_json_between() {
        let cases = [
            (
                Record::Gain {
                    slot: 2,
                    value: 0.75,
                },
                Change::Gain {
                    slot: 2,
                    value: 0.75,
                },
            ),
            (
                from_operation(karakuri_operation::Operation::SetOpacity {
                    deck: 0,
                    opacity: 0.25,
                }),
                Change::Opacity {
                    slot: 0,
                    value: 0.25,
                },
            ),
            (
                from_operation_reading(
                    karakuri_operation::Operation::SetMaskShape {
                        deck: 2,
                        kind: karakuri_operation::WipeKind::Linear,
                        angle: 1.5,
                    },
                    reading_of(Mask::new(MaskKind::Linear, 1.5, 0.25, 0.1)),
                ),
                Change::Mask {
                    slot: 2,
                    mask: Mask::new(MaskKind::Linear, 1.5, 0.25, 0.1),
                },
            ),
            (
                transition_record(1, Control::Opacity, 0.0, 64.0, 8.0, Curve::Smooth),
                Change::Transition {
                    slot: 1,
                    control: Control::Opacity,
                    to: 0.0,
                    start: 64.0,
                    beats: 8.0,
                    curve: Curve::Smooth,
                },
            ),
            (
                select_record(2, 1, 64.0),
                Change::Select {
                    slot: 2,
                    renderer: 1,
                    start: 64.0,
                },
            ),
            (
                Record::Preview { slot: Some(2) },
                Change::Preview { slot: Some(2) },
            ),
            (
                Record::Preview { slot: None },
                Change::Preview { slot: None },
            ),
            (
                from_operation(karakuri_operation::Operation::SetBlendMode {
                    deck: 3,
                    blend: blend_mode(Blend::Over),
                }),
                Change::Blend {
                    slot: 3,
                    mode: Blend::Over,
                },
            ),
            (
                from_operation(karakuri_operation::Operation::SetResidency {
                    deck: 1,
                    residency: residency(Residency::Priming),
                }),
                Change::Residency {
                    slot: 1,
                    level: Residency::Priming,
                },
            ),
            (
                transport_record(3, &Transport::engaged(Sync::Beat, 126.0)),
                Change::Transport {
                    slot: 3,
                    sync: Sync::Beat,
                    anchor_bpm: 126.0,
                    scrub_beats: 0.0,
                },
            ),
            (
                look_record(&Look {
                    op: TonemapOp::AgX,
                    exposure: 1.5,
                    white_point: 4.0,
                }),
                Change::Look(Look {
                    op: TonemapOp::AgX,
                    exposure: 1.5,
                    white_point: 4.0,
                }),
            ),
        ];
        for (record, expected) in cases {
            let line = serde_json::to_string(&record).expect("serialise");
            let decoded: Record = serde_json::from_str(&line).expect("parse");
            assert_eq!(
                change(&decoded, 4).expect("a record this build built"),
                Some(expected),
                "through {line}"
            );
        }
    }

    /// **Every residency level round-trips, not just the one a test remembered
    /// to name.** A level added to the engine and not to the wire vocabulary
    /// would otherwise be a slot silently refusing to change state.
    #[test]
    fn every_residency_level_has_a_wire_name_that_decodes_back() {
        for level in LEVELS {
            let record = from_operation(karakuri_operation::Operation::SetResidency {
                deck: 0,
                residency: residency(level),
            });
            assert_eq!(
                change(&record, 1).expect("built here"),
                Some(Change::Residency { slot: 0, level }),
                "{level:?} did not survive its own wire name"
            );
        }
    }

    /// **Every mask shape round-trips**, so a shape added to the engine and not
    /// to the wire vocabulary is a deck slot silently unmasked.
    ///
    /// Through `Operation::SetMaskShape` rather than a record spelled here,
    /// which is `from_operation`'s rule and buys the third spelling with it:
    /// the engine's `MaskKind`, the vocabulary's `WipeKind` and the wire name
    /// all have to agree for this to decode back.
    #[test]
    fn every_mask_shape_has_a_wire_name_that_decodes_back() {
        for kind in MaskKind::ALL {
            let mask = Mask::new(kind, 0.5, 0.75, 0.2);
            let record = from_operation_reading(
                karakuri_operation::Operation::SetMaskShape {
                    deck: 1,
                    kind: wipe_kind(kind),
                    angle: 0.5,
                },
                reading_of(mask),
            );
            assert_eq!(
                change(&record, 4).expect("built here"),
                Some(Change::Mask { slot: 1, mask }),
                "{} did not survive its own wire name",
                kind.name()
            );
        }
    }

    /// **Every control and every curve a transition can name round-trips**, so
    /// one added to the engine and not to the wire vocabulary is a move that
    /// fails to decode rather than one that moves the wrong thing.
    #[test]
    fn every_transition_control_and_curve_has_a_wire_name_that_decodes_back() {
        for control in Control::ALL {
            for curve in karakuri_engine::binding::CURVES {
                let record = transition_record(0, control, 1.0, 0.0, 4.0, curve);
                assert_eq!(
                    change(&record, 1).expect("built here"),
                    Some(Change::Transition {
                        slot: 0,
                        control,
                        to: 1.0,
                        start: 0.0,
                        beats: 4.0,
                        curve,
                    }),
                    "{} / {} did not survive its own wire name",
                    control.name(),
                    curve.name()
                );
            }
        }
    }

    /// **Every blend mode round-trips**, so a mode added to the engine and not
    /// to the wire vocabulary is a deck slot silently composited the wrong way
    /// — which under `over` is a slot that was supposed to hide and does not.
    #[test]
    fn every_blend_mode_has_a_wire_name_that_decodes_back() {
        for mode in Blend::ALL {
            let record = from_operation(karakuri_operation::Operation::SetBlendMode {
                deck: 2,
                blend: blend_mode(mode),
            });
            assert_eq!(
                change(&record, 4).expect("built here"),
                Some(Change::Blend { slot: 2, mode }),
                "{} did not survive its own wire name",
                mode.name()
            );
        }
    }

    /// **Every sync mode round-trips**, so a mode added to the engine and not
    /// to the wire vocabulary is a slot silently left free rather than put
    /// where the record said.
    #[test]
    fn every_sync_mode_has_a_wire_name_that_decodes_back() {
        for sync in Sync::ALL {
            let mut transport = Transport::engaged(sync, 100.0);
            transport.scrub(-0.75);
            let record = transport_record(1, &transport);
            assert_eq!(
                change(&record, 4).expect("built here"),
                Some(Change::Transport {
                    slot: 1,
                    sync,
                    anchor_bpm: 100.0,
                    // Carried under every mode, including the two that do
                    // nothing with it — a slot moved back onto the grid returns
                    // to where the operator left it.
                    scrub_beats: -0.75,
                }),
                "{} did not survive its own wire name",
                sync.name()
            );
        }
    }

    /// **The conversion writes exactly the transport the engine would engage**,
    /// which is the one thing `karakuri-operation-record` cannot check about
    /// itself.
    ///
    /// `Transport::engaged` is where engaging a mode is *decided* — the anchor
    /// is the session tempo, the scrub is cleared — and it says so in order
    /// that *"a caller building a record of the change and a caller applying
    /// one agree by construction"*. The record-builder is `written`, and it
    /// **cannot call it**: `karakuri-operation-record` depends on
    /// `karakuri-operation` and `karakuri-store` and on nothing else, by
    /// charter (ADR-0180, ADR-0194), so an engine under it would be a
    /// serialiser and a `wgpu` every surface pays for. The agreement is
    /// therefore a convention, and this is the place that enforces it
    /// (P-0026) — the only crate in the workspace that can see the policy and
    /// the conversion at once, which is the argument
    /// [`the_vocabularys_copy_of_a_list_spells_it_the_way_the_store_reads_it`]
    /// makes about the name lists, one field along.
    ///
    /// **The tempos are the ones a session can actually reach**, taken off an
    /// oscillator rather than written here, and both of its writers are
    /// exercised: `Oscillator::new` for the launch tempo and
    /// `Oscillator::correct` for a tracked one. The out-of-range and NaN asks
    /// are in the list because they are what the clamp exists for — an
    /// oscillator swallows them, so `current_tempo` never reports one, and if
    /// it ever did this is what would notice that the record and the engine
    /// had come apart on it.
    #[test]
    fn a_sync_mode_writes_exactly_what_the_engine_would_engage() {
        let mut corrected = Oscillator::new(120.0);
        // A wild estimate, which `Oscillator::correct` says is a thing that
        // happens: it lands on the top of the range.
        corrected.correct(9_999.0, 0.0);
        let mut slow = Oscillator::new(f32::NAN);
        slow.correct(-4.0, 0.0);
        let grids = [
            Oscillator::new(120.0),
            Oscillator::new(126.5),
            Oscillator::new(0.0),
            Oscillator::new(f32::NAN),
            Oscillator::new(f32::INFINITY),
            Oscillator::new(4_000.0),
            corrected,
            slow,
        ];
        for grid in grids {
            let tempo = current_tempo(&grid);
            for mode in Sync::ALL {
                let record = from_operation_reading(
                    karakuri_operation::Operation::SetSync {
                        deck: 2,
                        sync: sync(mode),
                    },
                    karakuri_operation_record::Current {
                        tempo: Some(tempo),
                        ..karakuri_operation_record::Current::default()
                    },
                );
                assert_eq!(
                    record,
                    transport_record(2, &Transport::engaged(mode, tempo)),
                    "the record `written` writes for `{}` at {tempo} bpm is not the \
                     transport `Transport::engaged` would engage — the conversion and the \
                     engine's policy have drifted, and a replay would put the slot \
                     somewhere the operator's press did not",
                    mode.name()
                );
            }
        }
    }

    /// Every tone map operator likewise, through `look`. `TONEMAPS` is one
    /// table now, so this fails if an operator is added to the enum and not to
    /// it.
    #[test]
    fn every_tonemap_operator_has_a_wire_name_that_decodes_back() {
        for op in TONEMAPS {
            let look = Look {
                op,
                exposure: 1.0,
                white_point: 4.0,
            };
            let Some(Change::Look(decoded)) = change(&look_record(&look), 1).expect("built here")
            else {
                panic!("a look record did not decode as a look");
            };
            assert_eq!(
                decoded.op,
                op,
                "{} did not survive its wire name",
                op_wire_name(op)
            );
        }
    }

    /// A record this build cannot obey is **reported, not dropped**. Silently
    /// ignoring it would leave a session replaying at the wrong gain with
    /// nothing said, which is worse than refusing the line.
    #[test]
    fn a_record_this_build_cannot_obey_says_so_rather_than_vanishing() {
        let unknown_level = Record::Residency {
            slot: 0,
            level: "cooling".to_string(),
        };
        let message = change(&unknown_level, 4).expect_err("`cooling` is not a level here");
        assert!(message.contains("cooling"), "{message}");
        assert!(message.contains("live"), "{message}");

        let unknown_op = Record::Look {
            op: "filmic".to_string(),
            exposure: 1.0,
            white_point: 4.0,
        };
        let message = change(&unknown_op, 4).expect_err("`filmic` is not an operator here");
        assert!(message.contains("filmic"), "{message}");
        assert!(message.contains("aces"), "{message}");

        for (start, beats, to, wanted) in [
            (f64::NAN, 4.0, 1.0, "not a position"),
            (0.0, -4.0, 1.0, "expected a number of beats"),
            (0.0, f64::NAN, 1.0, "expected a number of beats"),
            (0.0, 4.0, f32::NAN, "not a value"),
        ] {
            let record = Record::Transition {
                slot: 0,
                control: "gain".to_string(),
                to,
                start,
                beats,
                curve: "lin".to_string(),
            };
            let message = change(&record, 4).expect_err("a number no move can use");
            assert!(message.contains(wanted), "{start}/{beats}/{to}: {message}");
        }

        // A selection that never lands, for the reason a transition's does not:
        // nothing clears a queued move whose instant cannot arrive.
        for start in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let record = Record::Select {
                slot: 0,
                renderer: 0,
                start,
            };
            let message = change(&record, 4).expect_err("a beat no selection can land on");
            assert!(message.contains("not a position"), "{start}: {message}");
        }

        let unknown_shape = Record::Mask {
            slot: 0,
            kind: "diagonal".to_string(),
            angle: 0.0,
            position: 1.0,
            softness: 0.0,
        };
        let message = change(&unknown_shape, 4).expect_err("`diagonal` is not a shape");
        assert!(message.contains("diagonal"), "{message}");
        assert!(message.contains("radial"), "{message}");

        let unknown_control = Record::Transition {
            slot: 0,
            control: "residency".to_string(),
            to: 1.0,
            start: 0.0,
            beats: 4.0,
            curve: "lin".to_string(),
        };
        let message = change(&unknown_control, 4).expect_err("`residency` is not a control");
        assert!(message.contains("residency"), "{message}");
        assert!(message.contains("opacity"), "{message}");

        let unknown_curve = Record::Transition {
            slot: 0,
            control: "gain".to_string(),
            to: 1.0,
            start: 0.0,
            beats: 4.0,
            curve: "bezier".to_string(),
        };
        let message = change(&unknown_curve, 4).expect_err("`bezier` is not a curve");
        assert!(message.contains("bezier"), "{message}");
        assert!(message.contains("smooth"), "{message}");

        let unknown_mode = Record::Blend {
            slot: 0,
            mode: "screen".to_string(),
        };
        let message = change(&unknown_mode, 4).expect_err("`screen` is not a mode here");
        assert!(message.contains("screen"), "{message}");
        assert!(message.contains("over"), "{message}");
    }

    /// **`None` is the mix and is not a slot**, so it survives the range check
    /// that every other slot-bearing record goes through rather than being
    /// caught by it. A sentinel index would have made "the mix" and "slot 255"
    /// the same line on the wire.
    #[test]
    fn a_preview_of_the_mix_is_not_a_slot_out_of_range() {
        assert_eq!(
            change(&Record::Preview { slot: None }, 1).expect("the mix is always available"),
            Some(Change::Preview { slot: None })
        );
        // And a real slot past the deck still is, in the one sentence.
        let message =
            change(&Record::Preview { slot: Some(4) }, 4).expect_err("slot 4 of a deck of 4");
        assert_eq!(message, crate::no_such_slot(4, 4));
    }

    /// **A slot the deck does not have is caught in the decode**, where there
    /// is something to say about it, rather than four frames later in an index.
    ///
    /// **In the words every other surface says it in**, which is the assertion
    /// that had to be an `assert_eq!`: this module spelled the refusal itself,
    /// as `slot 4: this deck holds slots 0-3` against the keys' `no slot 4:`,
    /// and `contains("slots 0-3")` passed under both. See
    /// [`crate::no_such_slot`].
    #[test]
    fn a_slot_past_the_deck_is_refused_with_the_range_it_missed() {
        let message = change(
            &Record::Gain {
                slot: 4,
                value: 1.0,
            },
            4,
        )
        .expect_err("slot 4 of a deck of 4");
        assert_eq!(message, crate::no_such_slot(4, 4));
        // And the boundary either side of it, which is where the off-by-one
        // this shares with the digit keys would live.
        assert!(change(
            &Record::Gain {
                slot: 3,
                value: 1.0
            },
            4
        )
        .is_ok());
        assert!(change(
            &Record::Gain {
                slot: 0,
                value: 1.0
            },
            1
        )
        .is_ok());
        assert!(change(
            &Record::Gain {
                slot: 1,
                value: 1.0
            },
            1
        )
        .is_err());
    }

    /// A record that is not the mix's is not an error.
    /// `karakuri-environment`'s `audio.rs` decodes those, and both decoders see
    /// every record a session carries.
    #[test]
    fn a_record_that_is_not_the_mixs_is_left_alone() {
        assert_eq!(change(&Record::Tick { steps: 1 }, 4), Ok(None));
    }
}
