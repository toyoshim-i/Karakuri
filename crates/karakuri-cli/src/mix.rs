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
//! arrangement `audio.rs` has and is there for the same reason: the path the
//! engine is driven through is the record's rather than one that happens to
//! agree with it. A decode that only a test exercises is a decode that is
//! correct until the day it matters.
//!
//! Not on the frame path — a fader moves when a hand moves it — so the `String`
//! a `residency` or a `look` record carries is a key press's allocation and not
//! a frame's. `audio.rs`'s record is reused in place precisely because that one
//! *is* on the frame path; this one does not need to be, and pretending it did
//! would be complexity bought with nothing.
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
//! **The rest are still here and each has a reason.** `opacity_record`,
//! `blend_record`, `residency_record`, `mask_record` and `transition_record`
//! are called by `crossfade` and `wipe`, which are **one operation each and
//! four or five records each** — and those two conversions are the ones the
//! new crate cannot make, because they need the grid quantised onto a musical
//! instant and the quantum and the length that `Operation::SetTransition` sets
//! and no record carries. `look_record` builds the launch look, which is a
//! complete look rather than an ask. `transport_record` is `cycle_sync`'s, for
//! the anchor clamp the vocabulary has no way to apply. Each of them goes the
//! day its operation's conversion is settled — see ADR-0194.
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
//! replay the same; under `over` one dims a layer and the other stops it
//! hiding what is beneath.

use karakuri_engine::binding::Curve;
use karakuri_engine::deck::{Blend, Mask, MaskKind, Residency};
use karakuri_engine::present::TonemapOp;
use karakuri_engine::transition::Control;
use karakuri_engine::transport::{Sync, Transport};
use karakuri_engine::Look;
use karakuri_store::record::Record;

use crate::{op_wire_name, op_wire_names};

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
        offset_beats: f64,
    },
}

/// **The engine's list, as the vocabulary's** — one function per list, and
/// the one place the two copies of each are made to agree.
///
/// `karakuri-operation` owns a copy of every list a destination is drawn from,
/// which is the cost P-0074 says the vocabulary pays: *"The two rules — be
/// engine-neutral, and have no toggles — are not jointly satisfiable unless
/// the vocabulary owns the lists."* A copy needs somewhere the two meet, and
/// this is that place: `karakuri-cli` is the only crate in the workspace that
/// sees both, because it is the only one that depends on the engine and on the
/// vocabulary at once.
///
/// **`From` impls, which is what ADR-0180 said, are not available here.** Both
/// types are foreign to this package — `Blend` is `karakuri-engine`'s and
/// `BlendMode` is `karakuri-operation`'s — so the orphan rule refuses the impl
/// and there is nothing to be done about it short of one of those two crates
/// depending on the other, which is the thing neither of them may do. Plain
/// functions, then, exactly as `karakuri-console/examples/panel.rs`'s
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
/// `Operation::ScrubDeck` moves an offset by an amount and `Record::Transport`
/// is absolute, so the conversion reads where the slot is.
pub fn current_transport(transport: &Transport) -> karakuri_operation_record::Transport {
    karakuri_operation_record::Transport {
        sync: sync(transport.sync()),
        anchor_bpm: transport.anchor_bpm(),
        offset_beats: transport.offset_beats(),
    }
}

/// A slot's opacity — the fader — as the record that carries it.
pub fn opacity_record(slot: usize, value: f32) -> Record {
    Record::Opacity {
        slot: slot as u8,
        value,
    }
}

/// A slot's blend mode, as the record that carries it.
pub fn blend_record(slot: usize, mode: Blend) -> Record {
    Record::Blend {
        slot: slot as u8,
        mode: mode.name().to_string(),
    }
}

/// A slot's mask, as the record that carries it.
pub fn mask_record(slot: usize, mask: Mask) -> Record {
    Record::Mask {
        slot: slot as u8,
        kind: mask.kind().name().to_string(),
        angle: mask.angle(),
        position: mask.position(),
        softness: mask.softness(),
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

/// A slot's **requested** residency, as the record that carries it. See
/// [`Record::Residency`] for why the effective one is not recordable.
pub fn residency_record(slot: usize, level: Residency) -> Record {
    Record::Residency {
        slot: slot as u8,
        level: residency_wire_name(level).to_string(),
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
        offset_beats: transport.offset_beats(),
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

fn parse_residency(name: &str) -> Option<Residency> {
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
            offset_beats,
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
                offset_beats: *offset_beats,
            }))
        }
        Record::Look {
            op,
            exposure,
            white_point,
        } => {
            let op = crate::parse_op(op)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// **The two copies of every list, checked against each other — and this
    /// is the only place in the workspace where that can happen.**
    ///
    /// `karakuri-operation` owns its own `BlendMode`, `Residency`, `Sync` and
    /// `Tonemap` because a vocabulary that refuses to name a value cannot say
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
        for op in crate::TONEMAPS {
            assert_eq!(
                tonemap(op).name(),
                op_wire_name(op),
                "the vocabulary and the flag spell one tone map operator two ways, and \
                 `Record::Look` carries the name"
            );
        }
    }

    /// **The conversion and this module answer with the same record**, for the
    /// operations both of them can still write.
    ///
    /// `crossfade` and `wipe` build their parts through the builders here
    /// because their own conversions are not settled, so `Record::Opacity`,
    /// `Record::Blend` and `Record::Residency` are still made two ways in this
    /// program. That is an interim and it is stated as one: the conversion is
    /// the canonical derivation, and these builders go as each operation lands.
    /// Until then this is what stops the two drifting.
    #[test]
    fn the_builders_left_here_agree_with_the_conversion() {
        use karakuri_operation::Operation;
        use karakuri_operation_record::{Current, Written};

        let same = |operation: Operation, record: Record| {
            assert_eq!(
                karakuri_operation_record::written(&operation, &Current::default()),
                Written::Records(vec![record]),
                "`{}` is built two ways in this program and the two have drifted",
                operation.title()
            );
        };
        same(
            Operation::SetOpacity {
                deck: 2,
                opacity: 0.25,
            },
            opacity_record(2, 0.25),
        );
        same(
            Operation::SetBlendMode {
                deck: 1,
                blend: blend_mode(Blend::Over),
            },
            blend_record(1, Blend::Over),
        );
        same(
            Operation::SetResidency {
                deck: 3,
                residency: residency(Residency::Priming),
            },
            residency_record(3, Residency::Priming),
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
                opacity_record(0, 0.25),
                Change::Opacity {
                    slot: 0,
                    value: 0.25,
                },
            ),
            (
                mask_record(2, Mask::new(MaskKind::Linear, 1.5, 0.25, 0.1)),
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
                blend_record(3, Blend::Over),
                Change::Blend {
                    slot: 3,
                    mode: Blend::Over,
                },
            ),
            (
                residency_record(1, Residency::Priming),
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
                    offset_beats: 0.0,
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
            let record = residency_record(0, level);
            assert_eq!(
                change(&record, 1).expect("built here"),
                Some(Change::Residency { slot: 0, level }),
                "{level:?} did not survive its own wire name"
            );
        }
    }

    /// **Every mask shape round-trips**, so a shape added to the engine and not
    /// to the wire vocabulary is a layer silently unmasked.
    #[test]
    fn every_mask_shape_has_a_wire_name_that_decodes_back() {
        for kind in MaskKind::ALL {
            let mask = Mask::new(kind, 0.5, 0.75, 0.2);
            assert_eq!(
                change(&mask_record(1, mask), 4).expect("built here"),
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
    /// to the wire vocabulary is a layer silently composited the wrong way —
    /// which under `over` is a layer that was supposed to hide and does not.
    #[test]
    fn every_blend_mode_has_a_wire_name_that_decodes_back() {
        for mode in Blend::ALL {
            let record = blend_record(2, mode);
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
                    offset_beats: -0.75,
                }),
                "{} did not survive its own wire name",
                sync.name()
            );
        }
    }

    /// Every tone map operator likewise, through `look`. `TONEMAPS` is one
    /// table now, so this fails if an operator is added to the enum and not to
    /// it.
    #[test]
    fn every_tonemap_operator_has_a_wire_name_that_decodes_back() {
        for op in crate::TONEMAPS {
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

    /// A record that is not the mix's is not an error. `audio.rs` decodes
    /// those, and both decoders see every record a session carries.
    #[test]
    fn a_record_that_is_not_the_mixs_is_left_alone() {
        assert_eq!(change(&Record::Tick { steps: 1 }, 4), Ok(None));
    }
}
