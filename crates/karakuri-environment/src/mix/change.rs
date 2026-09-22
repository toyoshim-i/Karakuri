use karakuri_engine::binding::Curve;
use karakuri_engine::deck::{Blend, Mask, MaskKind, Residency};
use karakuri_engine::master::{Cut, SlotSpec};
use karakuri_engine::transition::Control;
use karakuri_engine::transport::Sync;
use karakuri_engine::Look;
use karakuri_store::record::{DeckSlot, Record};

use super::translate::*;

/// What one mix record says, decoded into what the engine takes.
///
/// The engine's own types, not the record's: a `Residency` rather than the
/// string it was spelled with, a [`Look`] rather than three loose fields. That
/// is where the decode ends and it is the whole of what the caller applies.
///
/// Not `Copy`, and it stopped being so when a change first named a parameter.
/// Every variant here moves the deck *around* a Set — a fader, a mode, a
/// residency, a look — and all of those are numbers and small enums.
/// [`Change::Ride`] reaches inside one, and a parameter is addressed by name:
/// the `String` it carries is what a `Copy` bound cannot survive. Clone is
/// kept, and nothing on the frame path needs two of one change.
#[derive(Clone, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub enum Change {
    Gain {
        slot: usize,
        value: f32,
    },
    /// The fader. Separate from `Gain` because the blend mode makes them separate —
    /// see [`Blend`].
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
    /// Which renderer of a slot's Set becomes the live one, at a musical instant.
    /// Carried as its parts rather than as a
    /// `karakuri_engine::transition::Selection` for [`Change::Transition`]'s reason
    /// — the applier is the one holding the deck.
    ///
    /// The renderer is not checked here. This decoder knows how many slots the deck
    /// has and nothing about what is in them; how many renderers a slot draws with
    /// is a property of the Set it is playing, which the applier has in hand and
    /// this does not. It is checked there, in [`crate::no_such_renderer`]'s words.
    Select {
        slot: usize,
        renderer: usize,
        start: f64,
    },
    Residency {
        slot: usize,
        level: Residency,
    },
    /// Parameter modification on a playing slot, expanded into individual component writes (ADR-0268).
    Ride {
        slot: usize,
        writes: Vec<karakuri_engine::ParamWrite>,
    },
    /// What drives one parameter of a slot that is playing, or nothing — the
    /// attachment and the take-back, which are one record and are one change here
    /// for the same reason.
    ///
    /// The address is carried and the binding is decoded. A
    /// `karakuri_engine::binding::Binding` already holds the layer, the index and
    /// the key, so an attachment needs nothing beside it; a take-back has no
    /// binding to hold them, so they are fields here. That is one address written
    /// twice in the attach case and it is the honest arrangement — the alternative
    /// is an applier that reaches inside a `Binding` to find out what to remove,
    /// which is the same fields read from a worse place.
    ///
    /// Decoded through `setfile::binding_from_source`, which builds the `bind` the
    /// payload spells and hands it to `binding_from_record` — so `signal=bpm`, an
    /// `octaves` without `fbm`, and a `noise` object on a binding that is not to
    /// `noise` are refused here in the words a Set file and a `--bind` are refused
    /// in, and there is one decoder rather than two.
    Source {
        slot: usize,
        layer: karakuri_ir::Kind,
        index: Option<u32>,
        key: String,
        /// `None` is *Take a parameter back*.
        binding: Option<karakuri_engine::binding::Binding>,
    },
    /// Who may move one node of a slot's Set. The writer ADR-0211 said the engine
    /// owed and `Record::Authority` has been waiting for.
    ///
    /// Addressed `(layer, index)` with no wildcard, on
    /// `karakuri_engine::swap::AuthorityAt`'s terms: a bare name means *every node
    /// declaring it*, and there is no such thing as an authority every node happens
    /// to declare.
    Authority {
        slot: usize,
        layer: karakuri_ir::Kind,
        index: u32,
        authority: karakuri_engine::set::Authority,
    },
    Look(Look),
    /// The level at the master chain's entry, which names no slot: it is what the
    /// fold *produced*, after every deck's edge has been applied.
    /// `karakuri_engine::deck::Deck::set_out` is what it decodes to, and says *"Not
    /// per slot"* at the setter (ADR-0224).
    MasterOut(f32),
    /// What the master chain is, whole — the ordered list of its slots, and it
    /// names no deck slot for [`Change::MasterOut`]'s reason, one pass downstream
    /// of it.
    ///
    /// A description and not a built chain: each entry is an address, a cut and a
    /// map of params, because a compiled chain is pipelines and buffers and this
    /// decoder holds no device. [`build_chain`] is what turns one into a
    /// `karakuri_engine::master::Chain`, and
    /// `karakuri_engine::present::Present::set_chain` is what installs it.
    ///
    /// Carried whole for [`Change::Transport`]'s reason: the record says every slot
    /// and a replay must not fill one of them in from the build it is running on
    /// (ADR-0340).
    MasterChain(Vec<SlotSpec>),
    /// What a slot's clock does with the session's. Carried as a value rather than
    /// applied as a mode change, because the record says all three and a replay
    /// must not recompute one of them from the machine it is on.
    Transport {
        slot: usize,
        sync: Sync,
        anchor_bpm: f32,
        scrub_beats: f64,
    },
}

/// The three procedures this repository ships as the master chain's presets,
/// and their content addresses.
///
/// They were `master.wgsl`'s three fragment entry points until 2026-09-10 and
/// are `.kir` files now (ADR-0340). They are compiled in rather than read from
/// disk for one reason: an address has to be the same number on every machine
/// and in every working directory, and a file read relative to a cwd is not
/// that. Putting them in a store is a separate act, done by whoever is
/// recording — `store.put_artifact(source)` — exactly as a Set's sources are,
/// so a run that records nothing creates nothing.
///
/// One mix record as the change it asks for.
///
/// Three answers, and they are three different things:
///
/// - `Ok(None)` — not a mix record. A `tick` or an `audio` is not this
///   module's to act on and not an error either.
/// - `Err(_)` — a mix record this build cannot obey. An unknown residency
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
    //
    // **`DeckSlot::new` is the check, not a second copy of it.** A record's
    // `DeckSlot` was constructed off the wire without a `slot_count` to check
    // against — only here, where the deck it addresses is known, can *is
    // this slot in range* be answered — so this closure hands the raw number
    // back to the one place that answers it rather than comparing `slot_count`
    // again itself.
    let in_range = |slot: DeckSlot| -> Result<usize, String> {
        DeckSlot::new(slot.0, slot_count)
            .map(|slot| slot.index())
            .ok_or_else(|| crate::no_such_slot(slot.index(), slot_count))
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
        Record::Residency { slot, level } => {
            let slot = in_range(*slot)?;
            let level = parse_residency(level).ok_or_else(|| {
                format!("residency `{level}` — expected {}", residency_wire_names())
            })?;
            Ok(Some(Change::Residency { slot, level }))
        }
        // **A knob turn, decoded into the writes it is.** The address crosses
        // as a unit — `karakuri_store::record::NodeAddress` is `Option`al on the
        // record and `karakuri_engine::ParamWrite::at` is `Option`al here, and
        // absent means the same wildcard on both sides — so there is nothing to
        // check and nothing that can be half an address.
        //
        // **The width says the component keys and nothing else does.**
        // `karakuri_ir::component_key` is the one spelling of `glow.x` in this
        // workspace and this is the second reader of it; a scalar is one write
        // under the key as written, which is what a knob sends and what a
        // component key already is.
        Record::Ride {
            slot,
            at,
            key,
            value,
        } => {
            let slot = in_range(*slot)?;
            let at = at.map(|node| (crate::meta::kind_of(node.layer), node.index));
            let writes = match value {
                karakuri_store::record::Value::Scalar(v) => vec![karakuri_engine::ParamWrite {
                    at,
                    key: key.clone(),
                    value: *v,
                }],
                _ => component_writes(at, key, value.components()),
            };
            Ok(Some(Change::Ride { slot, writes }))
        }
        // **An attachment, or the taking of one back.** The one decoder is
        // `crate::setfile::binding_from_source`, which builds the `bind` this
        // payload spells and hands it to `binding_from_record` — so a live
        // attachment cannot come to mean something a Set file's `bind` does
        // not, and the three diagnostics that reader owns are said here in its
        // words.
        //
        // **A take-back decodes to nothing rather than to a refusal.** There
        // is no signal to check and no curve to parse; what it names is an
        // address, and whether anything was attached there is the applier's to
        // report because only the applier is holding the Set.
        Record::Source {
            slot,
            layer,
            index,
            key,
            source,
        } => {
            let slot = in_range(*slot)?;
            let binding = match source {
                None => None,
                Some(source) => Some(crate::setfile::binding_from_source(
                    *layer, *index, key, source,
                )?),
            };
            Ok(Some(Change::Source {
                slot,
                layer: crate::meta::kind_of(*layer),
                index: *index,
                key: key.clone(),
                binding,
            }))
        }
        // **A word out of a closed list, and the diagnostic is the engine's to
        // give** — `Record::Authority` carries a `String` so that a stream
        // from a newer build reaches a sentence about what this build supports
        // rather than a parser that refuses the line.
        Record::Authority {
            slot,
            at,
            authority,
        } => {
            let slot = in_range(*slot)?;
            let level = karakuri_engine::set::Authority::from_name(authority).ok_or_else(|| {
                format!(
                    "authority `{authority}` — expected {}",
                    karakuri_engine::set::Authority::ALL
                        .iter()
                        .map(|a| a.name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
            Ok(Some(Change::Authority {
                slot,
                layer: crate::meta::kind_of(at.layer),
                index: at.index,
                authority: level,
            }))
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
        // **The two ends of the master chain, and neither names a slot.**
        //
        // They were both under the wildcard below until 2026-09-09, and what
        // that cost was a replay: a session that pulled the master out to 0.5,
        // or turned a feedback trail up, rendered offscreen with the level at
        // 1.0 and the chain off — a stream this program wrote and could not
        // reproduce, which is the one thing
        // `docs/principles/0092-the-same-inputs-produce-the-same-frame.md`
        // is about. The live path applied both because `crates/karakuri` reads
        // the records itself; every path through *this* decoder did not.
        //
        // **No clamp on either**, which is this module's rule everywhere: the
        // engine holds the range — `clamp_gain` for the level and
        // `Chain::clamped` for the chain — so a second opinion here would be a
        // range written down twice.
        Record::MasterOut { value } => Ok(Some(Change::MasterOut(*value))),
        Record::MasterChain(chain) => {
            let mut slots = Vec::with_capacity(chain.slots.len());
            for slot in &chain.slots {
                // **The cut comes back off the wire word, refused rather than
                // defaulted**, exactly as the tone map operator above does and
                // for the same reason: a cut this build has not got is a stream
                // saying something it cannot draw, and a default would silently
                // play the other picture — one echo where the session had a
                // trail.
                let cut = match &slot.cut {
                    None => None,
                    Some(word) => Some(Cut::parse(word).ok_or_else(|| {
                        format!(
                            "feedback cut `{word}` — expected {}",
                            Cut::ALL
                                .iter()
                                .map(|c| format!("`{}`", c.name()))
                                .collect::<Vec<_>>()
                                .join(" or ")
                        )
                    })?),
                };
                slots.push(SlotSpec {
                    procedure: slot.procedure.clone(),
                    cut,
                    params: slot.params.clone(),
                });
            }
            Ok(Some(Change::MasterChain(slots)))
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

/// A wide value as one write per component, under the keys ADR-0268 made:
/// `glow.x`, `glow.y`, `glow.z`.
///
/// `karakuri_ir::component_key` and not a `format!` here, because that function
/// is the one place the spelling lives — `karakuri_ir::Param::keys` publishes
/// the interface with it, `Set::params` is keyed by it, and a second spelling
/// would be a key that agrees with the engine's until somebody changes one of
/// them.
fn component_writes(
    at: Option<(karakuri_ir::Kind, u32)>,
    key: &str,
    values: &[f32],
) -> Vec<karakuri_engine::ParamWrite> {
    values
        .iter()
        .enumerate()
        .map(|(i, value)| karakuri_engine::ParamWrite {
            at,
            key: karakuri_ir::component_key(key, i),
            value: *value,
        })
        .collect()
}

/// How wide a wipe's soft edge is.
///
/// Not zero, and not a key. A hard front is an aliased staircase wherever it is
/// not axis-aligned, and this is the narrowest edge that hides that at the
/// resolutions this renders at — narrow enough that a wipe still reads as a
/// wipe rather than a gradient.
pub const MASK_SOFTNESS: f32 = 0.02;
