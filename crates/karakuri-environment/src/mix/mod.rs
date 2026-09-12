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
//!                  Record::Residency  ┼→ Change ─→ Deck / Present
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
//! [P-0091](../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)
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
//! a `preview` record, and the first is now what
//! `karakuri_operation_record::written` answers for `Operation::SetGain`. Two
//! derivations of one record is the drift this module was written to end, in
//! miniature, so the second one went. **The preview half went further**:
//! ADR-0240 retired *Choose what the output shows* and the record with it —
//! switching a preview is a bay-internal move rather than an engine one, so
//! there is nothing to record and no `Change` to decode into.
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
//! **`select_record` and `transition_record` went the same way, and the
//! second of them went in two steps.** They were `cycle_renderer`'s,
//! `fade_slot`'s and `wipe`'s, held while `Operation::FadeDeck`,
//! `Operation::Crossfade` and `Operation::SelectRenderer` needed the grid
//! quantised onto a musical instant plus the quantum and the length
//! `Operation::SetTransition` sets and no record carries. **The quantum and
//! the length turned out not to be missing but unassigned**, and they are the
//! surface's: `karakuri_operation_record::Current` carries them the way it
//! carries the look and the mask, [`current_transition`] is the reading that
//! hands them over, and the three operations write their own records now. A
//! selection is one record, so `select_record` had nothing left to be and went
//! then; `transition_record` stayed one caller longer, because
//! `Operation::Wipe` was still owed the shape its front takes and its soft
//! edge. **Those turned out to be unassigned too.** The shape is the same
//! operation's third setting and travels the same road — [`current_transition`]
//! carries it, which is why that function takes a `MaskKind` and an angle — and
//! the soft edge is read off the deck by [`current_mask`], which was already
//! the reading `Operation::SetMaskShape` takes. So `wipe` is one `operate` call
//! and this function has no caller left.
//!
//! **What `wipe` did keep is the one decision a gesture was making rather than
//! a record it was building**, and it kept it for a moment: it wrote the blend
//! mode only where the slot was still at the mode a slot starts in, and the
//! put-on-air only where the slot was not already live, so `m` in front of `c`
//! left the operator's mode alone. Routing the gesture took a deck to ask away
//! from it. [`current_mix`] is the reading that hands the answer over, and the
//! condition now lives beside the records it governs in the conversion's own
//! `Wipe` arm.
//!
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
use karakuri_engine::master::{Chain, Cut, Slot, SlotSpec};
use karakuri_engine::present::TonemapOp;
use karakuri_engine::set::Authority;
use karakuri_engine::transition::Control;
use karakuri_engine::transport::{Sync, Transport};
use karakuri_engine::Look;
use karakuri_signal::oscillator::Oscillator;
use karakuri_store::record::{DeckSlot, Record};

/// What one mix record says, decoded into what the engine takes.
///
/// The engine's own types, not the record's: a `Residency` rather than the
/// string it was spelled with, a [`Look`] rather than three loose fields. That
/// is where the decode ends and it is the whole of what the caller applies.
///
/// **Not `Copy`, and it stopped being so when a change first named a parameter.**
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
    /// **A parameter an operator moved on a slot that is playing.** The one
    /// change here that reaches inside a Set rather than moving the deck around
    /// it, and the one that takes a `Vec`.
    ///
    /// **One record, one or three writes**, because a parameter is driven one
    /// component at a time and a `vec3` value is one line that names three of
    /// them
    /// ([ADR-0268](../../../docs/adr/0268-a-vector-parameter-is-driven-one-component-at-a-time.md)).
    /// Expanded here rather than by the applier for the reason every other
    /// variant is decoded here: two appliers would be two answers. Expanded
    /// **without asking what the Set declares**, which is where this parts
    /// company with `setfile::from_lines` — that reader has just read the
    /// `slot` records and has the procedures in hand, and this one is looking
    /// at a Set that is already on air and holds none of them. A component key
    /// nothing declares lands as `Ok(0)` from
    /// `karakuri_engine::deck::Deck::write_param`, which the applier says out
    /// loud; the width the record wrote is the only thing that could name the
    /// components, and it does.
    Ride {
        slot: usize,
        writes: Vec<karakuri_engine::ParamWrite>,
    },
    /// **What drives one parameter of a slot that is playing, or nothing** —
    /// the attachment and the take-back, which are one record and are one
    /// change here for the same reason.
    ///
    /// **The address is carried and the binding is decoded.** A
    /// `karakuri_engine::binding::Binding` already holds the layer, the index
    /// and the key, so an attachment needs nothing beside it; a take-back has
    /// no binding to hold them, so they are fields here. That is one address
    /// written twice in the attach case and it is the honest arrangement —
    /// the alternative is an applier that reaches inside a `Binding` to find
    /// out what to remove, which is the same fields read from a worse place.
    ///
    /// **Decoded through `setfile::binding_from_source`**, which builds the
    /// `bind` the payload spells and hands it to `binding_from_record` — so
    /// `signal=bpm`, an `octaves` without `fbm`, and a `noise` object on a
    /// binding that is not to `noise` are refused here in the words a Set file
    /// and a `--bind` are refused in, and there is one decoder rather than
    /// two.
    Source {
        slot: usize,
        layer: karakuri_ir::Kind,
        index: Option<u32>,
        key: String,
        /// **`None` is *Take a parameter back*.**
        binding: Option<karakuri_engine::binding::Binding>,
    },
    /// **Who may move one node of a slot's Set.** The writer ADR-0211 said the
    /// engine owed and `Record::Authority` has been waiting for.
    ///
    /// Addressed `(layer, index)` with no wildcard, on
    /// `karakuri_engine::swap::AuthorityAt`'s terms: a bare name means *every
    /// node declaring it*, and there is no such thing as an authority every
    /// node happens to declare.
    Authority {
        slot: usize,
        layer: karakuri_ir::Kind,
        index: u32,
        authority: karakuri_engine::set::Authority,
    },
    Look(Look),
    /// **The level at the master chain's entry**, which names no slot: it is
    /// what the fold *produced*, after every deck's edge has been applied.
    /// `karakuri_engine::deck::Deck::set_out` is what it decodes to, and says
    /// *"Not per slot"* at the setter (ADR-0224).
    MasterOut(f32),
    /// **What the master chain is**, whole — the ordered list of its slots,
    /// and it names no deck slot for [`Change::MasterOut`]'s reason, one pass
    /// downstream of it.
    ///
    /// **A description and not a built chain**: each entry is an address, a cut
    /// and a map of params, because a compiled chain is pipelines and buffers
    /// and this decoder holds no device. [`build_chain`] is what turns one into
    /// a `karakuri_engine::master::Chain`, and
    /// `karakuri_engine::present::Present::set_chain` is what installs it.
    ///
    /// Carried whole for [`Change::Transport`]'s reason: the record says every
    /// slot and a replay must not fill one of them in from the build it is
    /// running on (ADR-0340).
    MasterChain(Vec<SlotSpec>),
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

/// **The three procedures this repository ships as the master chain's presets**,
/// and their content addresses.
///
/// They were `master.wgsl`'s three fragment entry points until 2026-09-10 and
/// are `.kir` files now (ADR-0340). They are compiled in rather than read from
/// disk for one reason: an address has to be the same number on every machine
/// and in every working directory, and a file read relative to a cwd is not
/// that. **Putting them in a store is a separate act**, done by whoever is
/// recording — `store.put_artifact(source)` — exactly as a Set's sources are,
/// so a run that records nothing creates nothing.
pub mod shipped;

/// **Compile a described chain into one the engine can run.**
///
/// `resolve` answers what an address's source is — the shipped three without a
/// store, anything else out of one — and a slot whose address nothing holds is
/// refused **with the address in the message**, which is what ADR-0340 asks of
/// a replay meeting a procedure the store does not have.
///
/// **Where the work happens is the caller's answer and not this function's.**
/// It compiles and it builds pipelines, so it belongs off the render thread —
/// a Set's build runs on `HotSwap`'s worker for exactly this reason
/// (`docs/principles/0091-cost-is-known-before-it-is-paid.md`,
/// `docs/adr/0033-…`) — and the built list is installed at a frame boundary by
/// `Present::set_chain`.
pub fn build_chain(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    slots: &[SlotSpec],
    resolve: &dyn Fn(&str) -> Option<String>,
) -> Result<Chain, String> {
    let mut built = Vec::with_capacity(slots.len());
    for (at, spec) in slots.iter().enumerate() {
        let source = resolve(&spec.procedure)
            .ok_or_else(|| format!("master chain slot {at}: nothing holds `{}`", spec.procedure))?;
        let checked = crate::compile::check(&source)
            .map_err(|e| format!("master chain slot {at}: {}: {e}", spec.procedure))?;
        built.push(
            Slot::build(
                device,
                layout,
                spec.procedure.clone(),
                &checked,
                spec.cut,
                spec.params.clone(),
            )
            .map_err(|e| format!("master chain slot {at}: {e}"))?,
        );
    }
    Ok(Chain::new(built))
}

/// **What an address resolves to**, for [`apply_chain`] and [`build_chain`].
///
/// The shipped three first and without a store at all — a windowed run that has
/// never saved anything can still put a preset in its chain — and then whatever
/// store the caller has. **A store is optional and that is the point**: a run
/// recording nothing creates nothing (`Placed::put`'s own division).
pub fn resolve_procedure(
    store: Option<&karakuri_store::store::Store>,
    address: &str,
) -> Option<String> {
    if let Some(source) = shipped::source(address) {
        return Some(source.to_string());
    }
    let hash: karakuri_store::hash::Hash = address.parse().ok()?;
    let bytes = store?.get_artifact(&hash).ok()?;
    String::from_utf8(bytes).ok()
}

/// **Put a described chain on a `Present`**, building a list only where the
/// list itself changed.
///
/// **Two paths, and which one is taken is P-0091's question rather than a
/// convenience.** `Record::MasterChain` is written whole — a stream that moved
/// one slot without saying where the others stood describes a chain a replay
/// cannot put back — so the ordinary case of applying one is a record whose
/// *shape* is the shape already running with one number different. That is a
/// `queue.write_buffer` per slot and nothing else. A record whose shape differs
/// is a build: sources resolved, procedures compiled, pipelines made, targets
/// allocated.
///
/// **The build is on the caller's thread and this says so rather than hiding
/// it.** A Set's build runs on `HotSwap`'s worker
/// (`docs/adr/0033-freeing-on-the-render-thread-is-the-same-invariant-as-allocating.md`);
/// a chain's runs here, at the point in the frame loop where a record is
/// applied, which is before the frame's encoder exists on every path that calls
/// it. What that costs is a `naga` pass and a pipeline per slot, on the frames
/// an operator changed the *list* — which is a press, not a fader ride.
/// Compiling it on a worker instead is what M5.16's second pass owes when the
/// Library can drop a procedure on the chain.
pub fn apply_chain(
    present: &mut karakuri_engine::Present,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    slots: &[SlotSpec],
    resolve: &dyn Fn(&str) -> Option<String>,
) -> Result<(), String> {
    let shape: Vec<(String, Option<Cut>)> =
        slots.iter().map(|s| (s.procedure.clone(), s.cut)).collect();
    let params: Vec<std::collections::BTreeMap<String, f32>> =
        slots.iter().map(|s| s.params.clone()).collect();
    if present.set_chain_params(queue, &shape, &params) {
        return Ok(());
    }
    let chain = build_chain(device, present.chain_layout(), slots, resolve)?;
    present.set_chain(device, queue, chain);
    Ok(())
}

/// **The engine's list, as the vocabulary's** — one function per list, and
/// the one place the two copies of each are made to agree.
///
/// `karakuri-operation` owns a copy of every list a destination is drawn from,
/// which is the cost P-0090 says the vocabulary pays: *"The two rules — be
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
/// **The engine's curve as the vocabulary's**, for the one of these lists that
/// had no wire to reach until a fade converted.
///
/// `karakuri_operation::Curve` has existed since the vocabulary did —
/// `Operation::AttachSignal` carries one — and nothing ever needed its name,
/// because that operation writes no session record. A scheduled move does:
/// `Record::Transition`'s `curve` is what a replay reads the shape of a fade
/// back out of. So this is the fifth of these functions and it arrived last,
/// for the reason the others arrived when they did.
pub fn curve(shape: Curve) -> karakuri_operation::Curve {
    match shape {
        Curve::Lin => karakuri_operation::Curve::Lin,
        Curve::Pow2 => karakuri_operation::Curve::Pow2,
        Curve::Sqrt => karakuri_operation::Curve::Sqrt,
        Curve::Smooth => karakuri_operation::Curve::Smooth,
    }
}

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

/// The engine's feedback cut, as the vocabulary's. See [`blend_mode`].
///
/// **There is no function the other way**, and that is not an omission: a
/// press carries the vocabulary's cut into a record as a *word*, and
/// `karakuri_engine::master::Cut::parse` is what reads the word back — so the
/// return leg goes through the record rather than around it, which is where
/// every other closed list's does. [`change`]'s `master_chain` arm is that
/// reader.
pub fn cut(cut: Cut) -> karakuri_operation::Cut {
    match cut {
        Cut::Mix => karakuri_operation::Cut::Mix,
        Cut::Exit => karakuri_operation::Cut::Exit,
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

/// **The master chain that is running, as the reading the conversion needs.**
///
/// [`current_look`]'s function one pass upstream and its argument with one more
/// row in it: `Operation::SetBloom` carries an amount and nothing else, because
/// that is what a row of the Master bay can say, and `Record::MasterChain`
/// carries all four because that is what a replay can reconstruct a chain
/// from — the same 0.5 is a one-frame echo under `mix` and a compounding trail
/// under `exit`, so an amount without its cut is not a picture. See
/// `docs/adr/0317-the-master-chain-is-three-fixed-passes-and-feedback-reads-either-cut.md`.
///
/// **The amount is the engine's and not a track position.** A fader draws where
/// it is along its own travel and divides by `Feedback::MAX` to do it; a
/// reading is what the pass is *at*, which is what the record carries.
pub fn current_chain(slots: &[SlotSpec]) -> karakuri_operation_record::Chain {
    let shipped = shipped::addresses();
    // **The three rows read the three shipped slots**, and a row whose slot is
    // not in the chain reads zero — which is the honest reading of *this pass
    // is not running*, and is what the row will stop asking for when the rows
    // retire (ADR-0340 §7).
    let amount = |address: &str| {
        slots
            .iter()
            .find(|s| s.procedure == address)
            .and_then(|s| s.params.get("amount").copied())
            .unwrap_or(0.0)
    };
    let feedback_slot = slots.iter().find(|s| s.procedure == shipped.feedback);
    karakuri_operation_record::Chain {
        feedback: karakuri_operation::Feedback {
            amount: amount(&shipped.feedback),
            cut: cut(feedback_slot.and_then(|s| s.cut).unwrap_or_default()),
        },
        bloom: amount(&shipped.bloom),
        rgb_shift: amount(&shipped.rgb_shift),
        slots: slots
            .iter()
            .map(|s| karakuri_store::record::ChainSlot {
                procedure: s.procedure.clone(),
                cut: s.cut.map(|c| c.name().to_string()),
                params: s.params.clone(),
            })
            .collect(),
        shipped: shipped.clone(),
    }
}

/// **The shape every scheduled fade takes**, which is what [`current_transition`]
/// is handed and what `Record::Transition`'s `curve` ends up spelling.
///
/// `smooth` rather than `lin`, and the reason is in `binding.rs`: its
/// derivative is zero at both ends, so a fade neither jumps off the floor nor
/// slams into the ceiling. A crossfade of two linear ramps has a visible corner
/// at each end; two smooth ones do not. **On no control anywhere**, because the
/// other three curves are for *signals* — a fade wants easing and nothing else,
/// and a fourth cycling key for a choice nobody would revisit is a key in the
/// way.
///
/// **It is here rather than in each surface**, and that is the one thing that
/// changed about it: `karakuri-cli` held it as a private const of its own, and
/// the day `crates/karakuri`'s window began scheduling moves too there would
/// have been two copies of one decision with nothing holding them together —
/// two surfaces easing the same fade differently, in a value a replay carries
/// (`docs/contributing.md` §4). Both binaries already reach this module for
/// [`current_transition`], so this is where the one copy goes.
pub const FADE_CURVE: Curve = Curve::Smooth;

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
/// (`docs/contributing.md` §4),
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

/// **What the next scheduled move means, as the reading the conversion
/// needs.**
///
/// `Operation::FadeDeck` carries a deck and a destination, because that is
/// what a control can say; `Record::Transition` carries the instant, the
/// length and the shape too, because that is what a replay can reconstruct a
/// move from. The three that are missing are `Operation::SetTransition`'s —
/// a surface's own setting deciding what the *next* fade means — so they are
/// handed in, and this is the fifth of these.
///
/// **It takes the grid and a quantum rather than a start, and that is the
/// whole of the function.** `karakuri_engine::transition::quantise` is where
/// the next musical instant is decided, once, at the moment the operator
/// asked; `karakuri-operation-record` cannot reach it any more than it can
/// reach the oscillator, and a conversion that divided by a quantum of its own
/// would be a second grid. Asking for the oscillator and the quantum instead
/// of a beat count is what makes that structural rather than a hope
/// (`docs/contributing.md` §4),
/// which is [`current_tempo`]'s arrangement exactly.
///
/// **A caller with no opinion about the grid gives a quantum of 0**, which
/// `quantise` documents as *"now"* and answers with the beat count it was
/// handed — so ASAP is a setting a surface already has rather than anything
/// this signature had to invent. See
/// [`tests::a_quantum_of_zero_starts_the_move_on_the_beat_it_was_asked_on`].
///
/// **The wipe shape is the fourth setting to come through here, and it is the
/// third of `Operation::SetTransition`'s three.** `mask` and `angle` are what
/// the `z` key holds — the shape the *next* wipe takes, never the shape a
/// deck's layer is wearing — and they arrive by this route for the reason the
/// quantum and the length do: all three are one operation's, that operation
/// writes no record, and a surface is the only thing holding them.
/// `Operation::Wipe` is the one conversion that reads them, and it reads the
/// soft edge off the deck instead, through [`current_mask`].
///
/// **The engine's `MaskKind` as the vocabulary's, on the way in.** The caller
/// hands over what it is holding and [`wipe_kind`] is the one place the two
/// lists are made to agree, exactly as [`curve`] is for the shape of the move.
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

/// **Where a slot already sits in the mix, as the reading the conversion
/// needs** — the sixth of these, and the one that is read so a record can be
/// left *out*.
///
/// `Operation::Wipe` puts the deck it reveals under `over` and on air, and
/// both of those are a state the deck may be in already. Under `add` or under
/// `max` the same gesture is a wipe *on* rather than a wipe *over* — a
/// different picture and a legitimate one — so a wipe writes the blend mode
/// only where the slot is still at the mode a slot starts in, and the
/// put-on-air only where the slot is not already live. That is the decision
/// `karakuri-cli`'s `c` made for itself while it built those records by hand;
/// the conversion has no deck to ask, so what it needs is this
/// ([P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md):
/// safety is never bought with the operator's authority).
///
/// **What the deck reports, not what it was asked for.** `Deck::residency`
/// answers the level the slot is at — the governor may hold one below the
/// request — which is the same value the surface reading this used to compare
/// against, and the right one: what a wipe needs to know is whether the
/// put-on-air it is about to write would change anything.
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

/// **A wide value as one write per component**, under the keys ADR-0268 made:
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
mod tests;
