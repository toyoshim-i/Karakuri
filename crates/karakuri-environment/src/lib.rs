//! External environment integration layer for Karakuri.
//!
//! Provides platform and environment services for audio input, MIDI control, file watching,
//! compilation, MCP server endpoints, session recording, and history management (ADR-0214, ADR-0215).
//!
//! Surfaces (CLI, console GUI) consume this crate without embedding device- or platform-specific
//! handling into their own binaries.

// Each module's own header is the documentation for it, and there is
// deliberately no second sentence here: a `///` on one of these declarations
// would be a doc fragment written in *this* file's scope, and rustdoc resolves
// a module's `//!` links in whichever scope the fragment came from — which
// silently unresolved six working links the first time this list was
// written. A `//` comment such as this one is not a fragment and costs nothing.
//
// - `audio` — a microphone, the beat it is tracking, and the record for both.
// - `clock` — real time, as the step count a frame writes into its `tick`.
// - `compile` — a `.kir` off a disk, through the pipeline, with its bytes kept.
// - `history` — the edit history: a directory per day, a chain per procedure.
// - `meta` — an artifact's card: what a compile pass can say, and where it lands.
// - `midi` — a port, and what the operator asked for through it.
// - `mix` — the performance as records: faders, blends, residency, the look.
// - `places` — where the presets and the store are, told or gone looking for.
// - `render` — a frame written to a PNG: the window's path, minus the window.
// - `scratch` — the copies a live run edits, so an original is untouched.
// - `session` — the recorder that writes the stream and the split that reads it.
// - `setfile` — the material as a record: what a Set was, written down and read back.
// - `tempo_source` — another program's clock, and what to believe of it.
// - `watch` — a slot's files, polled, and the rebuild a change asks for.
pub mod audio;
pub mod clock;
pub mod compile;
pub mod history;
pub mod meta;
pub mod midi;
pub mod mix;
pub mod places;
pub mod render;
pub mod scratch;
pub mod session;
pub mod setfile;
pub mod tempo_source;
pub mod watch;

/// What the operator has opened to an automatic route, shared between whoever
/// draws the toggle and whoever reads it on every call.
///
/// [`karakuri_operation::gate::Open`] is the value — four classes, all closed
/// to begin, and no way to write one down that starts open. This is the handle
/// that makes it *live*: ADR-0235's whole shape is that an operator opens a
/// class ahead of a show or between numbers, so a server handed a snapshot at
/// startup could not implement the decision at all. A bay head's pill writes
/// through one of these and [`mcp::serve`] reads through another clone of the
/// same one.
///
/// It is not the audit and it holds no table — the classification and the
/// refusal are `karakuri_operation::gate`'s, one copy for every route
/// (ADR-0236). This is only where the operator's answer is kept while a run is
/// going, which is this package's clause rather than the vocabulary's: it is
/// state a surface writes and another surface reads, and nothing here knows
/// which surface either is.
///
/// Nothing writes one yet. The bay-head toggles are the console's and are not
/// built; until they are, a run holds a handle that stays
/// [`Open::CLOSED`](karakuri_operation::gate::Open::CLOSED) and every closed
/// class is refused, which is the state ADR-0235 says a run starts in.
/// `docs/contributing.md` §4.
#[derive(Debug, Clone, Default)]
pub struct Opening(std::sync::Arc<std::sync::RwLock<karakuri_operation::gate::Open>>);

impl Opening {
    /// A run's opening, with all four classes closed. The name says the state
    /// rather than leaving it to a `Default` a caller reads as *empty*: what this
    /// is is the closed one.
    pub fn closed() -> Opening {
        Opening::default()
    }

    /// What is open now. Read on every call rather than held, because a class the
    /// operator closed between two calls has to be closed for the second.
    ///
    /// A poisoned lock reads as closed. The alternative is a panic on a call path,
    /// and the safe answer to *is this open* when the thing holding it fell over is
    /// no.
    pub fn read(&self) -> karakuri_operation::gate::Open {
        self.0
            .read()
            .map(|open| *open)
            .unwrap_or(karakuri_operation::gate::Open::CLOSED)
    }

    /// What the operator just said. Whoever draws the toggle calls this; a poisoned
    /// lock drops the write rather than panicking on the surface.
    pub fn set(&self, open: karakuri_operation::gate::Open) {
        if let Ok(mut held) = self.0.write() {
            *held = open;
        }
    }
}

/// Slot-level MCP modification policies and mix activity.
#[derive(Debug, Clone)]
pub struct SlotPolicies(std::sync::Arc<std::sync::RwLock<[karakuri_operation::SlotAccess; 4]>>);

impl Default for SlotPolicies {
    fn default() -> Self {
        Self(std::sync::Arc::new(std::sync::RwLock::new(
            [karakuri_operation::SlotAccess::default(); 4],
        )))
    }
}

impl SlotPolicies {
    /// Initialized with default access (policy = Auto, in_mix = false).
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads the access status for a slot.
    pub fn access(&self, slot: usize) -> karakuri_operation::SlotAccess {
        self.0
            .read()
            .ok()
            .and_then(|guard| guard.get(slot).copied())
            .unwrap_or_default()
    }

    /// Reads the policy configured for a slot.
    pub fn policy(&self, slot: usize) -> karakuri_operation::SlotPolicy {
        self.access(slot).policy
    }

    /// Checks if a slot is writable by MCP, returning structured refusal details if rejected.
    pub fn check_writable_detail(
        &self,
        slot: usize,
    ) -> Result<(), karakuri_operation::RefusalDetail> {
        let access = self.access(slot);
        if access.is_writable() {
            Ok(())
        } else {
            Err(access
                .refusal_detail(slot)
                .unwrap_or_else(|| karakuri_operation::RefusalDetail {
                    code: karakuri_operation::RefusalCode::SlotPolicyOff,
                    message: format!("slot {slot} is not writable by MCP"),
                    slot: Some(slot),
                    deck: u8::try_from(slot).ok(),
                    lane: None,
                    class: None,
                    policy: Some(access.policy),
                    in_mix: Some(access.in_mix),
                }))
        }
    }

    /// Checks if a slot is writable by MCP, returning an error message if refused.
    pub fn check_writable(&self, slot: usize) -> Result<(), String> {
        self.check_writable_detail(slot).map_err(|d| d.message)
    }

    /// Sets the policy for a slot.
    pub fn set_policy(&self, slot: usize, policy: karakuri_operation::SlotPolicy) {
        if let Ok(mut guard) = self.0.write() {
            if let Some(entry) = guard.get_mut(slot) {
                entry.policy = policy;
            }
        }
    }

    /// Sets whether a slot is contributing to the mix.
    pub fn set_in_mix(&self, slot: usize, in_mix: bool) {
        if let Ok(mut guard) = self.0.write() {
            if let Some(entry) = guard.get_mut(slot) {
                entry.in_mix = in_mix;
            }
        }
    }

    /// Reads all 4 slot access states.
    pub fn all(&self) -> [karakuri_operation::SlotAccess; 4] {
        self.0.read().map(|guard| *guard).unwrap_or_default()
    }
}

/// How long the end of a run waits for saves still being written.
///
/// Long enough that a save of a few dozen lines and a handful of artifacts
/// finishes on any disk that is answering, and short enough that one which is
/// not answering costs a quit five seconds rather than the window. See the
/// surface's `Live::awaited_saves` for why the wait exists at all.
pub const SAVE_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// Whose act a save is, which is the whole of what decides where it lands.
///
/// `<store>/sets/` is the operator's library and is written by an operator's
/// own act; a save asked for over MCP lands in `<store>/sandbox/` instead. The
/// rule is
/// [P-0096](../../../docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md)
/// and the decision is
/// [ADR-0261](../../../docs/adr/0261-a-model-asked-save-lands-in-a-sandbox-because-the-operators-library-is-the-operators-own-act.md).
///
/// An argument rather than a thing inferred from `reply`. Both surfaces already
/// know which this is at the call site — a request off `mcp::Reporter` passes
/// `Some(reply)` and a key press passes `None` — so `reply.is_some()` would
/// answer correctly today and would be answering a different question: *is
/// anybody waiting who is not at the terminal*. The next control that waits for
/// an answer without being a model would file its saves in the sandbox and
/// nothing would fail. This is the actor, named.
///
/// There is no third arm and a MIDI target would take [`Asked::Operator`]: a
/// control change is a hand on a control, and every route an operator's hands
/// reach is one act arriving by a different door.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asked {
    /// The operator's own act: `--save-set`, the `k` key, a control they pressed.
    Operator,
    /// A model, over MCP.
    Model,
}

/// No such slot, in the words every surface says it in.
///
/// Extracted where a second caller appeared, rather than copied to it: focusing
/// a slot that does not exist and saving one are the same mistake and were
/// about to be two sentences about it.
///
/// And "every surface" is now literally every one of them, which it was not
/// when this sentence was first written. There were four spellings of one
/// refusal — the keys said `no slot 9: this deck holds slots 0-3`, `--mcp` said
/// `holds 0-3`, MIDI said `no slot 9 — this deck holds slots 0-3`, and a `gain`
/// record naming a slot said `slot 9: this deck holds slots 0-3` — so a model
/// calling `save_set {"slot":9}` and an operator pressing `9` got different
/// sentences for the same mistake on the same control. That was tolerable while
/// each surface reached different controls; it stopped being tolerable when
/// `save_set` made one control reachable from two of them: the refusals are the
/// same sentences whoever meets them — see
/// `docs/principles/0090-a-surface-offers-it-never-decides.md`. `mcp.rs`,
/// `midi.rs` and `mix.rs` all call this now. Each of them pins it with an
/// `assert_eq!` against this function rather than trusting this comment — see
/// `mcp::tests::a_slot_a_layer_and_a_renderer_resolve_and_anything_else_is_refused`,
/// `mcp::wire_tests::a_save_for_a_slot_that_does_not_exist_is_refused_here`,
/// `midi::tests::an_unmapped_control_and_a_missing_slot_are_each_reported_once`
/// and `mix::tests::a_slot_past_the_deck_is_refused_with_the_range_it_missed`.
/// Every one of them asked only `contains(...)` before, which is why four
/// spellings could live side by side unnoticed.
///
/// The engine's own `no slot` messages are deliberately *not* routed here: they
/// are `assert!`s on a call that should never have been made, addressed to
/// whoever is holding the debugger, and a refusal an operator reads and a panic
/// a programmer reads are two audiences that happen to share a phrase.
pub fn no_such_slot(slot: usize, slot_count: usize) -> String {
    match slot_count {
        // Cannot happen — a run with no slots does not reach a window — and
        // written anyway, because `slot_count - 1` on it is an underflow and a
        // panic, which is what the arm that "cannot happen" costs when the shape
        // around it changes.
        0 => format!("no slot {slot}: this deck holds none"),
        n => format!("no slot {slot}: this deck holds slots 0-{}", n - 1),
    }
}

/// A renderer the slot does not draw with, in the words every surface says it
/// in.
///
/// [`no_such_slot`]'s shape, one address down: the thing named, then what there
/// was to name. A selection is the first control that addresses *inside* a
/// slot, so it is the first refusal that needed this — and it is a free
/// function beside that one rather than a sentence in `Live`, because the key
/// press and a replayed record both meet it and telling one operator two
/// stories about one mistake is what `no_such_slot` exists to have stopped.
///
/// The count comes from the Set on screen, not from the flags the run started
/// with, which is what makes it true after a hot swap: a rebuilt slot draws
/// with however many renderers its new sources declare.
pub fn no_such_renderer(slot: usize, at: usize, count: usize) -> String {
    match count {
        // Cannot happen — a Set with no renderer does not build — and written
        // anyway, because `count - 1` on it underflows and panics, which is
        // what the arm that "cannot happen" costs when the shape around it
        // changes. The same reasoning as [`no_such_slot`]'s empty deck.
        0 => format!("no renderer {at}: slot {slot} draws with none"),
        n => format!(
            "no renderer {at}: slot {slot} draws with renderers 0-{}",
            n - 1
        ),
    }
}

/// Error message for parameter names not declared by the Set in `slot` (ADR-0268, Principle 0083).
pub fn no_such_param(slot: usize, key: &str) -> String {
    format!("no parameter `{key}`: the Set in slot {slot} declares none by that name")
}

/// Why a slot has nothing to save, in the words the operator is given.
///
/// Named, and with the flag that changes the answer. Every refusal around this
/// one names the file or the range it is about; this one used to name neither
/// the id the run came from nor anything the operator could do, which leaves
/// them pressing a key that reports a fact about the world rather than a way
/// out of it.
///
/// A free function over the two facts it turns on, for the reason the surface's
/// `drained_saves` is one: it has to reach a model as well as a terminal now,
/// which makes it worth a test, and `Live` needs a window and a GPU.
///
/// `no_files` is whether this slot has any startup sources at all — see the
/// surface's `Live::startup`, which is empty exactly for a slot filled straight
/// from a Set file by hash.
pub fn nothing_to_save(slot: usize, loaded_set: Option<&str>, no_files: bool) -> String {
    match loaded_set {
        // **Both flags, because `editable()` is both.** It is `editable()` that
        // materialises a loaded Set into the scratch and puts it in
        // `args.sets`, and that is `--watch || --mcp` — so `--load-set X --mcp
        // PORT` with no `--watch` already saves like any other slot. Naming only
        // `--watch` sent an operator who had `--mcp` off to restart a set for a
        // flag they did not need.
        Some(id) if no_files => format!(
            "slot {slot}: nothing to save — it was filled from set `{id}` by hash, with no \
             files behind it and nothing able to rebuild it. Start the run with `--watch` \
             or `--mcp` and this slot saves like any other"
        ),
        _ => format!(
            "slot {slot}: nothing to save — this slot's sources are not in the store, which \
             was said at startup, and no rebuild of it has landed since"
        ),
    }
}

/// Told when [`accepted_save`] has said its sentence, so that this crate can
/// call back into a waiting client without naming what it is.
///
/// `karakuri-mcp`'s `Reply` is the one implementation there is — a model's call
/// is the one caller of `accepted_save` with anyone waiting on the other end.
/// The trait exists only so `accepted_save` can stay in this crate
/// (`no_such_slot` and `nothing_to_save`'s neighbours, called from the same
/// four surfaces) without this crate depending on `karakuri-mcp`, which depends
/// on this crate for [`setfile`], [`compile`], [`meta`], [`history`] and
/// [`watch`].
pub trait SaveReply {
    /// The sentence [`accepted_save`] said, verbatim.
    fn accepted(&self, said: &str);
}

/// A save has been taken and named, said to the terminal and to whoever asked
/// for it if that was not a hand. Returns the id it will be filed under.
///
/// Said before the store thread starts, because the operator pressed a key and
/// the answer to "did it take" is owed now rather than when the disk gets round
/// to it. Where it went is said on arrival — see the surface's
/// `Live::took_save`.
///
/// The same sentence to a waiting client, and this half is the one a timeout
/// depends on. [`mcp::Reply`] carries two messages because "accepted, under
/// this id, outcome not yet known" is a third fact the protocol's one boolean
/// cannot hold: a client whose deadline passes with this message in hand is
/// told to go looking under the id, and one without it is told *nothing was
/// saved and asking again is safe* — which is a false claim to a model about a
/// save that is running and will land.
///
/// A free function over the four facts it turns on, for the reason
/// [`nothing_to_save`] and the surface's `drained_saves` are, and the reason
/// bites harder here. Those are refusals — said *instead of* a save, and
/// reachable without a window. This is said *during* one, and inline it sat
/// below the surface's `playing_values`, which reads `deck.slot(slot)` and is
/// the single line of its `Live::save_set` that genuinely needs a GPU. So it
/// was the one half of the accept-then-settle sequence no test could reach:
/// deleting the `accepted` call left the whole suite green, because every
/// `--mcp` test drives a stand-in loop that sends `accepted` itself. Above that
/// line it is testable, and
/// `mcp::tests::a_save_the_loop_has_taken_names_its_id_to_a_client_that_times_out`
/// is what deleting the call now costs.
///
/// What it is filed under is `filed_as`, the private function below, which
/// turns on [`Asked`] as well as on whether a name was given — and the sentence
/// below says which of the store's two directories it is going into, because
/// that is the half a model cannot infer.
pub fn accepted_save(
    slot: usize,
    asked: Asked,
    id: Option<String>,
    sources: &setfile::Sources,
    root: &std::path::Path,
    reply: Option<&dyn SaveReply>,
) -> String {
    let id = filed_as(asked, id);
    let said = format!(
        "slot {slot}: saving {} node{} as set `{id}` in {}",
        sources.len(),
        if sources.len() == 1 { "" } else { "s" },
        match asked {
            Asked::Operator => root.display().to_string(),
            // **Named rather than left as the root**, because this sentence is
            // the one a model is handed and the directory is the whole of what
            // changed for it: told only the root, a client would look under
            // `sets/`, find nothing, and report the save as lost.
            Asked::Model => root
                .join(karakuri_store::store::Store::SANDBOX)
                .display()
                .to_string(),
        }
    );
    eprintln!("{said}");
    if let Some(reply) = reply {
        reply.accepted(&said);
    }
    id
}

/// Resolves the save file identifier based on caller type and optional user-supplied name (ADR-0128).
///
/// Operator saves preserve explicit IDs or generate timestamps. Model saves always prefix a timestamp.
fn filed_as(asked: Asked, id: Option<String>) -> String {
    match (asked, id) {
        (Asked::Operator, Some(id)) => id,
        (Asked::Operator, None) => history::stamped_id(),
        (Asked::Model, None) => history::stamped_id(),
        (Asked::Model, Some(name)) => format!("{}_{name}", history::stamped_id()),
    }
}
