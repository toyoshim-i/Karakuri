//! Mix state translation, recording, and application.
//!
//! Bridges live performance gestures (faders, blend modes, residency, look, mask transitions)
//! to engine updates via [`karakuri_operation_record::Record`].
//! Continuous controls coalesce per frame (ADR-0207, Principle 0091).
//! Translates between engine types (`karakuri-engine`) and UI/operation types (`karakuri-operation`).

use karakuri_engine::binding::Curve;
use karakuri_engine::chain_swap::ChainSlot;
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
pub mod shipped;

/// Resolve and check every slot of a described chain, holding no device.
///
/// `resolve` answers what an address's source is — the shipped three without a
/// store, anything else out of one — and a slot whose address nothing holds is
/// refused with the address in the message, which is what ADR-0340 asks of a
/// replay meeting a procedure the store does not have. A source that does not
/// check is refused with the slot's position and its address.
///
/// The one derivation of a chain's [`ChainSlot`]s: [`build_chain`] compiles
/// these against a device on the calling thread, and [`apply_chain`] hands them
/// to the chain worker.
pub fn check_chain(
    slots: &[SlotSpec],
    resolve: &dyn Fn(&str) -> Option<String>,
) -> Result<Vec<ChainSlot>, String> {
    let sources = resolve_chain(slots, resolve)?;
    let mut checked = Vec::with_capacity(slots.len());
    for (at, (spec, source)) in slots.iter().zip(&sources).enumerate() {
        checked.push(ChainSlot {
            spec: spec.clone(),
            checked: crate::compile::check(source)
                .map_err(|e| format!("master chain slot {at}: {}: {e}", spec.procedure))?,
        });
    }
    Ok(checked)
}

/// Compile a described chain into one the engine can run, on the calling
/// thread.
///
/// Creates a shader module and a render pipeline per slot, which is why the
/// real-time hosts do not call this: they call [`apply_chain`], which does the
/// same work on `karakuri-chain`. This is the synchronous path — the offline
/// renderer and replay, where no frame is waiting (ADR-0354).
///
/// The chain it returns carries no targets, so the `Present` it is installed on
/// allocates them.
pub fn build_chain(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    slots: &[SlotSpec],
    resolve: &dyn Fn(&str) -> Option<String>,
) -> Result<Chain, String> {
    let checked = check_chain(slots, resolve)?;
    let mut built = Vec::with_capacity(checked.len());
    for (at, want) in checked.iter().enumerate() {
        built.push(
            Slot::build(
                device,
                layout,
                want.spec.procedure.clone(),
                &want.checked,
                want.spec.cut,
                want.spec.params.clone(),
            )
            .map_err(|e| format!("master chain slot {at}: {e}"))?,
        );
    }
    Ok(Chain::new(built))
}

/// The source behind every slot of a described chain, in order, or the one
/// refusal for an address nothing holds.
///
/// Holds no device, so whether a run can resolve a chain at all is settled
/// before anything is compiled: the same question is answered the same way on
/// the frame path, at replay, and anywhere a chain is checked before it is
/// installed. The refusal names the slot's position and its address (ADR-0340),
/// and it refuses at the first such slot.
///
/// `resolve` is [`resolve_procedure`] bound to whatever store the caller has.
pub fn resolve_chain(
    slots: &[SlotSpec],
    resolve: &dyn Fn(&str) -> Option<String>,
) -> Result<Vec<String>, String> {
    slots
        .iter()
        .enumerate()
        .map(|(at, spec)| {
            resolve(&spec.procedure).ok_or_else(|| {
                format!("master chain slot {at}: nothing holds `{}`", spec.procedure)
            })
        })
        .collect()
}

/// What an address resolves to, for [`resolve_chain`], [`build_chain`] and
/// [`apply_chain`] — the one resolution every host uses.
///
/// The shipped three first and without a store at all — a windowed run that has
/// never saved anything can still put a preset in its chain — and then whatever
/// store the caller has. A store is optional and that is the point: a run
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

/// Put a described chain on a `Present` without compiling anything on the
/// calling thread.
///
/// Two paths, and which one is taken is P-0091's question rather than a
/// convenience. `Record::MasterChain` is written whole — a stream that moved
/// one slot without saying where the others stood describes a chain a replay
/// cannot put back — so the ordinary case of applying one is a record whose
/// *shape* is the shape already running with one number different. That is a
/// `queue.write_buffer` per slot and nothing else, and it returns having
/// applied it.
///
/// A record whose shape differs is a build, and the build is asked for here and
/// happens on `karakuri-chain`: sources are resolved and checked on this thread
/// — neither touches a device — and the procedures, the pipelines and the
/// targets are made on the worker. Until the build lands the chain that is
/// running keeps drawing and `Present::chain_spec` still reads it, so a surface
/// that draws the chain draws the outgoing list until the frame the new one is
/// installed on. Calling again with a list already being built is a no-op, so a
/// host may ask on every frame.
///
/// `Ok` means the list was applied or a build was asked for, not that a build
/// succeeded: a slot that refuses at compile time is a
/// [`ChainEvent::Refused`](karakuri_engine::ChainEvent) on `swap`, and the
/// chain keeps what it had. `Err` is a slot whose address nothing holds or
/// whose source does not check, and then nothing was asked for.
///
/// [`ChainSwap::begin_frame`](karakuri_engine::ChainSwap::begin_frame) is what
/// installs the result, and must be called before the frame's encoder exists.
pub fn apply_chain(
    swap: &mut karakuri_engine::ChainSwap,
    present: &mut karakuri_engine::Present,
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
    if swap.is_building(slots) {
        return Ok(());
    }
    swap.request(present, check_chain(slots, resolve)?);
    Ok(())
}

/// Put a described chain on a `Present`, compiling it on the calling thread.
///
/// The synchronous path, and [`karakuri_engine::HotSwap::install`] is its
/// counterpart one layer down: a run with no frame waiting on the clock — the
/// offline renderer, a replay — builds where it stands rather than carrying a
/// worker. The cheap path is the same one [`apply_chain`] takes and for the
/// same reason.
///
/// Returns having applied the list or having refused it. A refusal names the
/// slot and the chain keeps what it had.
pub fn install_chain(
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
    drop(present.set_chain(device, queue, chain));
    Ok(())
}

/// Converts an engine [`Blend`] mode to the corresponding vocabulary [`karakuri_operation::BlendMode`].
/// See ADR-0194.
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

/// The engine's sync mode, as the vocabulary's. See [`blend_mode`]. The
/// engine's curve as the vocabulary's, for the one of these lists that had no
/// wire to reach until a fade converted.
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
/// There is no function the other way, and that is not an omission: a press
/// carries the vocabulary's cut into a record as a *word*, and
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
/// Written before there is a reader for it, which is why the lint has to be
/// told, and it is here anyway for the reason the five above it are here at
/// all: the two spellings have to be *checked* against each other somewhere,
/// this is the only crate that can see both, and the check below needs a
/// conversion to check. Nothing on the CLI's paths reads `Set::authority` yet —
/// the console's `man / sug / auto` chip and a live save that writes a
/// `Record::Authority` were the two readers this waited for, and the chip
/// arrived on 2026-08-29 — the Inspector bay draws a node's authority, so this
/// has a caller outside the tests and the attribute it carried is gone.
pub fn authority(level: Authority) -> karakuri_operation::Authority {
    match level {
        Authority::Manual => karakuri_operation::Authority::Manual,
        Authority::Suggesting => karakuri_operation::Authority::Suggesting,
        Authority::Automatic => karakuri_operation::Authority::Automatic,
    }
}

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

/// The master chain that is running, as the reading the conversion needs.
///
/// [`current_look`]'s function one pass along: an operation names one slot of
/// the chain and `Record::MasterChain` carries the whole list, so the list that
/// is running is what completes the record. See
/// `docs/adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md`.
pub fn current_chain(slots: &[SlotSpec]) -> karakuri_operation_record::Chain {
    karakuri_operation_record::Chain {
        slots: slots
            .iter()
            .map(|s| karakuri_store::record::ChainSlot {
                procedure: s.procedure.clone(),
                cut: s.cut.map(|c| c.name().to_string()),
                params: s.params.clone(),
            })
            .collect(),
    }
}

/// What one `kind L5` source offers a chain, or `None` for a source that is not
/// one: the content address a slot of it is named by, and whether it declares
/// `retains`.
///
/// A surface offering a procedure to the chain needs both: an add carries the
/// address and carries a cut exactly where the procedure declares `retains`
/// (`docs/adr/0348-a-chain-slots-cut-is-set-through-the-parameter-row.md`). Both
/// are facts about the file rather than about the store it came from, so this
/// takes the source and not a path.
///
/// It checks rather than scanning for a word: `retains` is a header declaration
/// of the language and `check` is the one reader of it.
pub fn l5_offer(source: &str) -> Option<(String, bool)> {
    let checked = crate::compile::check(source).ok()?;
    (checked.kind == karakuri_ir::Kind::L5).then(|| (shipped::address(source), checked.retains))
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
/// record that fails to decode with a message naming what was available, rather
/// than one that decodes as the wrong level.
pub fn residency_wire_name(level: Residency) -> &'static str {
    match level {
        Residency::Live => "live",
        Residency::Priming => "priming",
        Residency::Allocated => "allocated",
    }
}

/// A wire spelling back to the engine's residency — [`residency_wire_name`]
/// read the other way, over [`LEVELS`], so the two directions cannot disagree.
///
/// `pub` for a second surface. [`change`] below is the one caller in this
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
const MASK_SOFTNESS: f32 = 0.02;

/// Every tone map operator there is. The one list, and its length is in its
/// type, so adding an operator to it is a deliberate act rather than an
/// oversight in a `Vec`. The record's own vocabulary for the output look, which
/// is why it is here rather than with the keys that cycle it.
pub const TONEMAPS: [TonemapOp; 4] = [
    TonemapOp::Clamp,
    TonemapOp::Reinhard,
    TonemapOp::Aces,
    TonemapOp::AgX,
];

/// Returns the canonical wire name and display name for a tonemap operator.
fn spellings(op: TonemapOp) -> (&'static str, &'static str) {
    match op {
        TonemapOp::Clamp => ("clamp", "clamp"),
        TonemapOp::Reinhard => ("reinhard", "Reinhard"),
        TonemapOp::Aces => ("aces", "ACES"),
        TonemapOp::AgX => ("agx", "AgX"),
    }
}

/// How a human reads it. Free to be capitalised the way the papers are, because
/// nothing parses it.
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
