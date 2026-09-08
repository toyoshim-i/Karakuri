//! The program: everything this instrument deals with that is not itself.
//!
//! **A module belongs here if what it deals with lives outside this process —
//! a disk, a device, a port, a socket, another process — or is the record of
//! what happened.** That sentence is the charter, and it is a test rather than
//! a description: the next module anyone proposes for this package is checked
//! against it, and the answer is not a vote. It is stated in
//! `docs/adr/0215-the-package-is-karakuri-environment-and-a-module-belongs-if-what-it-deals-with-is-outside-this-process.md`,
//! which is also where the name comes from and where the five nouns it was
//! weighed against are measured.
//!
//! The second half of the sentence is not decoration. Two of the modules in
//! scope — the metadata card and the mixer's wire spellings — touch no device
//! and open no file, and are entirely the record of what happened; a test with
//! only the first clause would have left them behind in the command line,
//! which is wrong. Both clauses are load-bearing and neither stands in for the
//! other.
//!
//! ## Why this is a package and not a module of the binary
//!
//! `docs/adr/0214-the-program-moves-out-of-the-cli-and-two-thin-binaries-sit-over-it.md`
//! is the decision, and its argument is a bill that had already been paid five
//! times. `karakuri-cli` has no library target, so nothing in this workspace
//! could depend on any of this: the frame loop was written twice, an operation
//! record needed a whole new crate to land in, the Library bay's time column
//! was **cut** rather than draw a third date format, and the console's example
//! transcribes a store path and a residency parser by hand. Each of those
//! arrived looking like a local question. None of them was.
//!
//! So: **two thin binaries sit over this package.** `karakuri-cli` is one
//! today — it keeps its flags, its terminal, its keys and its `--headless`
//! runs, and it parses arguments and calls in here. The panel is the second
//! when it exists, and it is the destination; `README.md`'s sentence that the
//! CLI is scaffolding rather than the destination is unchanged by any of this.
//! Nothing in this crate is the command line's and nothing in it is the
//! panel's. Both are surfaces, which is what the vocabulary has said since
//! `docs/adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md`.
//!
//! What follows from that is the rule to hold when adding to this crate: **no
//! module here may know which surface it is under.** A window, an event loop
//! and a key handler are a surface's; the disk, the ports and the record are
//! this package's, and the day a module here needs to ask which binary called
//! it is the day the boundary has been drawn in the wrong place.
//!
//! ## What is here, and what is not yet
//!
//! ADR-0215 applied its test to thirteen modules and thirteen passed, and all
//! thirteen are here — [`audio`], [`compile`], [`history`], [`mcp`], [`meta`],
//! [`midi`], [`mix`], [`render`], [`scratch`], [`session`], [`setfile`],
//! [`tempo_source`] and [`watch`] — because ADR-0214 left open whether the move
//! lands in one commit or several and answered its own question with *several
//! is the likelier*. The seven that came first were the seven that named
//! nothing else in `karakuri-cli`; the metadata card and the Set file came
//! second, and they brought the two items they named with them — the card
//! writer that puts a card in a store, and a Set file's per-layer node names,
//! which are part of that file's shape. The watcher, the MCP server, the mixer
//! and the MIDI map came last and brought eighteen items with them, which is
//! why this file has code in it at all: five of the eighteen belong to no
//! module here and are reached from several. Each slice is a package boundary
//! and not a redesign: `use` paths changed, `pub` appeared where crate-private
//! had been enough, and the code inside the functions did not.
//!
//! **[`places`] is the fourteenth and was none of the thirteen**, because it
//! is the first module written *for* this package rather than moved into it.
//! It is checked against the charter like anything else proposed here, and
//! passes on the first clause without argument: where the shipped presets are
//! and where the store is are two directories on a disk, and a directory is
//! outside this process by any reading of that sentence. That it is also what
//! deletes the two `.karakuri` transcriptions ADR-0214 named is the occasion
//! rather than the reason — a module that passed the test only because it was
//! convenient would be the test not being applied.
//!
//! **What is left in `karakuri-cli/src/` is `main.rs` and nothing else** — the
//! window, the arguments, the key handler and `Live`. That is the line
//! ADR-0214 said it would not name in advance, and ADR-0215 named the two ends
//! of it: `Live` holds a window and a device and stays with the surface, while
//! `Clock` is owed a move it has not had yet, because wall-clock time comes
//! from outside this process.
//!
//! ## The five items here that are no module's
//!
//! [`no_such_slot`], [`no_such_renderer`] and [`nothing_to_save`] are refusals,
//! and
//! `docs/principles/0090-a-surface-offers-it-never-decides.md`
//! says a refusal a person can reach from two surfaces is one sentence. These
//! are reached from four — the keys, [`mcp`], [`midi`] and a replayed record
//! through [`mix`] — and [`accepted_save`] is the sentence beside them that is
//! not a refusal, said to a terminal and to a waiting client at once. **They
//! are at the crate root because they are nobody's module**: putting
//! `no_such_slot` in [`mix`] would make [`mcp`] and [`midi`] depend on the
//! mixer for a sentence, which is a shape rather than a home. [`SAVE_WAIT`] is
//! here for the other half of that reason — the run bounds its quit by it and
//! [`mcp`] bounds a client's wait by it plus five, and neither of them owns it.
//!
//! **These are one-line restatements; the canonical text is one file each in
//! `docs/principles/` and one record each in `docs/adr/`, and where this
//! comment disagrees with them, this comment is the one that is wrong.**

// Each module's own header is the documentation for it, and there is
// deliberately no second sentence here: a `///` on one of these declarations
// would be a doc fragment written in *this* file's scope, and rustdoc resolves
// a module's `//!` links in whichever scope the fragment came from — which
// silently unresolved six working links the first time this list was
// written. A `//` comment such as this one is not a fragment and costs nothing.
//
// - `audio` — a microphone, the beat it is tracking, and the record for both.
// - `compile` — a `.kir` off a disk, through the pipeline, with its bytes kept.
// - `history` — the edit history: a directory per day, a chain per procedure.
// - `mcp` — a socket, and the Model Context Protocol a model speaks over it.
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
pub mod compile;
pub mod history;
pub mod mcp;
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

/// **What the operator has opened to an automatic route**, shared between
/// whoever draws the toggle and whoever reads it on every call.
///
/// [`karakuri_operation::gate::Open`] is the value — four classes, all closed
/// to begin, and no way to write one down that starts open. This is the handle
/// that makes it *live*: ADR-0235's whole shape is that **an operator opens a
/// class ahead of a show or between numbers**, so a server handed a snapshot at
/// startup could not implement the decision at all. A bay head's pill writes
/// through one of these and [`mcp::serve`] reads through another clone of the
/// same one.
///
/// **It is not the audit and it holds no table** — the classification and the
/// refusal are `karakuri_operation::gate`'s, one copy for every route
/// (ADR-0236). This is only where the operator's answer is kept while a run is
/// going, which is this package's clause rather than the vocabulary's: it is
/// state a surface writes and another surface reads, and nothing here knows
/// which surface either is.
///
/// **Nothing writes one yet.** The bay-head toggles are the console's and are
/// not built; until they are, a run holds a handle that stays
/// [`Open::CLOSED`](karakuri_operation::gate::Open::CLOSED) and every closed
/// class is refused, which is the state ADR-0235 says a run starts in.
/// `docs/contributing.md` §4.
#[derive(Debug, Clone, Default)]
pub struct Opening(std::sync::Arc<std::sync::RwLock<karakuri_operation::gate::Open>>);

impl Opening {
    /// A run's opening, with all four classes closed. **The name says the
    /// state** rather than leaving it to a `Default` a caller reads as *empty*:
    /// what this is is the closed one.
    pub fn closed() -> Opening {
        Opening::default()
    }

    /// What is open now. Read on every call rather than held, because a class
    /// the operator closed between two calls has to be closed for the second.
    ///
    /// **A poisoned lock reads as closed.** The alternative is a panic on a
    /// call path, and the safe answer to *is this open* when the thing holding
    /// it fell over is no.
    pub fn read(&self) -> karakuri_operation::gate::Open {
        self.0
            .read()
            .map(|open| *open)
            .unwrap_or(karakuri_operation::gate::Open::CLOSED)
    }

    /// What the operator just said. Whoever draws the toggle calls this; a
    /// poisoned lock drops the write rather than panicking on the surface.
    pub fn set(&self, open: karakuri_operation::gate::Open) {
        if let Ok(mut held) = self.0.write() {
            *held = open;
        }
    }
}

/// **How long the end of a run waits for saves still being written.**
///
/// Long enough that a save of a few dozen lines and a handful of artifacts
/// finishes on any disk that is answering, and short enough that one which is
/// not answering costs a quit five seconds rather than the window. See
/// the surface's `Live::awaited_saves` for why the wait exists at all.
pub const SAVE_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// **Whose act a save is**, which is the whole of what decides where it lands.
///
/// `<store>/sets/` is the operator's library and is written by an operator's own
/// act; a save asked for over MCP lands in `<store>/sandbox/` instead. The rule
/// is
/// [P-0096](../../../docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md)
/// and the decision is
/// [ADR-0261](../../../docs/adr/0261-a-model-asked-save-lands-in-a-sandbox-because-the-operators-library-is-the-operators-own-act.md).
///
/// **An argument rather than a thing inferred from `reply`.** Both surfaces
/// already know which this is at the call site — a request off `mcp::Reporter`
/// passes `Some(reply)` and a key press passes `None` — so `reply.is_some()`
/// would answer correctly today and would be answering a different question:
/// *is anybody waiting who is not at the terminal*. The next control that waits
/// for an answer without being a model would file its saves in the sandbox and
/// nothing would fail. This is the actor, named.
///
/// **There is no third arm and a MIDI target would take [`Asked::Operator`]**:
/// a control change is a hand on a control, and every route an operator's hands
/// reach is one act arriving by a different door.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asked {
    /// The operator's own act: `--save-set`, the `k` key, a control they
    /// pressed.
    Operator,
    /// A model, over MCP.
    Model,
}

/// **No such slot**, in the words every surface says it in.
///
/// Extracted where a second caller appeared, rather than copied to it: focusing
/// a slot that does not exist and saving one are the same mistake and were about
/// to be two sentences about it.
///
/// **And "every surface" is now literally every one of them**, which it was not
/// when this sentence was first written. There were four spellings of one
/// refusal — the keys said `no slot 9: this deck holds slots 0-3`, `--mcp` said
/// `holds 0-3`, MIDI said `no slot 9 — this deck holds slots 0-3`, and a `gain`
/// record naming a slot said `slot 9: this deck holds slots 0-3` — so a model
/// calling `save_set {"slot":9}` and an operator pressing `9` got different
/// sentences for the same mistake on the same control. That was tolerable while
/// each surface reached different controls; it stopped being tolerable when
/// `save_set` made one control reachable from two of them: the refusals are
/// the same sentences whoever meets them — see
/// `docs/principles/0090-a-surface-offers-it-never-decides.md`.
/// `mcp.rs`, `midi.rs` and
/// `mix.rs` all call this now. Each of them pins it with an `assert_eq!`
/// against this function rather than trusting this comment — see
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

/// **A renderer the slot does not draw with**, in the words every surface says
/// it in.
///
/// [`no_such_slot`]'s shape, one address down: the thing named, then what there
/// was to name. A selection is the first control that addresses *inside* a
/// slot, so it is the first refusal that needed this — and it is a free
/// function beside that one rather than a sentence in `Live`, because the key
/// press and a replayed record both meet it and telling one operator two
/// stories about one mistake is what `no_such_slot` exists to have stopped.
///
/// **The count comes from the Set on screen, not from the flags the run
/// started with**, which is what makes it true after a hot swap: a rebuilt
/// slot draws with however many renderers its new sources declare.
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

/// **A parameter the Set a slot is playing does not declare**, in the words
/// every surface says it in.
///
/// [`no_such_renderer`]'s shape and its reason, one address along: a write is
/// the second control that addresses *inside* a slot, and a key press, a
/// mapped knob, an MCP call and a replayed `ride` record all meet the same
/// answer. `karakuri_engine::deck::Deck::write_param` answers `Ok(0)` for it
/// rather than an error, because zero declarations reached is a fact the caller
/// says out loud rather than a refusal — a name a rebuild no longer declares
/// must not take the show down
/// ([P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
///
/// **The key as written, and it is the component key where there is one.**
/// `glow` is not a parameter and `glow.x` is
/// ([ADR-0268](../../../docs/adr/0268-a-vector-parameter-is-driven-one-component-at-a-time.md)),
/// so echoing what was asked for is what tells the two apart
/// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).
pub fn no_such_param(slot: usize, key: &str) -> String {
    format!("no parameter `{key}`: the Set in slot {slot} declares none by that name")
}

/// **Why a slot has nothing to save**, in the words the operator is given.
///
/// **Named, and with the flag that changes the answer.** Every refusal around
/// this one names the file or the range it is about; this one used to name
/// neither the id the run came from nor anything the operator could do, which
/// leaves them pressing a key that reports a fact about the world rather than a
/// way out of it.
///
/// A free function over the two facts it turns on, for the reason the surface's
/// `drained_saves` is one: it has to reach a model as well as a terminal now,
/// which makes it worth a test, and `Live` needs a window and a GPU.
///
/// `no_files` is whether this slot has any startup sources at all — see the
/// surface's `Live::startup`, which is empty exactly for a slot filled straight from a Set
/// file by hash.
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

/// **A save has been taken and named**, said to the terminal and to whoever
/// asked for it if that was not a hand. Returns the id it will be filed under.
///
/// Said before the store thread starts, because the operator pressed a key and
/// the answer to "did it take" is owed now rather than when the disk gets round
/// to it. Where it went is said on arrival — see the surface's `Live::took_save`.
///
/// **The same sentence to a waiting client, and this half is the one a timeout
/// depends on.** [`mcp::Reply`] carries two messages because "accepted, under
/// this id, outcome not yet known" is a third fact the protocol's one boolean
/// cannot hold: a client whose deadline passes with this message in hand is
/// told to go looking under the id, and one without it is told *nothing was
/// saved and asking again is safe* — which is a false claim to a model about a
/// save that is running and will land.
///
/// **A free function over the four facts it turns on**, for the reason
/// [`nothing_to_save`] and the surface's `drained_saves` are, and the reason
/// bites harder here. Those are refusals — said *instead of* a save, and
/// reachable without a window. This is said *during* one, and inline it sat
/// below the surface's `playing_values`, which reads `deck.slot(slot)` and is
/// the single line of its `Live::save_set` that genuinely needs a GPU. So it was the one half of the
/// accept-then-settle sequence no test could reach: deleting the `accepted`
/// call left the whole suite green, because every `--mcp` test drives a
/// stand-in loop that sends `accepted` itself. Above that line it is testable,
/// and `mcp::tests::a_save_the_loop_has_taken_names_its_id_to_a_client_that_times_out`
/// is what deleting the call now costs.
///
/// **What it is filed under is `filed_as`**, the private function below, which
/// turns on [`Asked`] as well as on whether a name was given — and the sentence
/// below says which of the store's two directories it is going into, because
/// that is the half a model cannot infer.
pub fn accepted_save(
    slot: usize,
    asked: Asked,
    id: Option<String>,
    sources: &setfile::Sources,
    root: &std::path::Path,
    reply: Option<&mcp::Reply>,
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

/// **What a save is filed under**, which is the caller's name, a stamp, or both.
///
/// **An operator's own act gets the name it asked for.** A key press cannot type
/// one and takes a stamp — `history::stamped_id`, whose convention this is: an
/// operator looks for the time they saved it — and a caller that *can* type one
/// is not made to take a timestamp. A name typed twice overwrites the library
/// entry under it, which is
/// [ADR-0128](../../../docs/adr/0128-a-set-saved-under-a-name-the-caller-chose-overwrites.md)'s
/// decision and unchanged: an id an operator types is an instruction.
///
/// **A model's save always carries the stamp**, and its chosen name rides behind
/// it as `<stamp>_<name>`. Two reasons, and the second is the one that decides
/// it.
///
/// 1. **Nothing in the sandbox is overwritten.** What lands there is an edit
///    history — the thing an operator goes looking for after a show when a model
///    has been editing live — and a snapshot a later snapshot can replace is not
///    a snapshot. ADR-0128's argument does not reach here because its premise
///    does not: there is no id an operator typed.
/// 2. **A directory of snapshots is read by time.** `history.rs` files every
///    kept version under the moment it was written and the operator's name for
///    it second, and this is the same directory read the same way. The separator
///    is `_` for that reason: it is the one a snapshot's own name already uses,
///    and `-` is what [`history::stamped_id`] appends when it breaks a tie.
///
/// **The cost is that a model is answered with an id it did not ask for**, which
/// `mcp::checked_id` calls the worse answer where the library is concerned and
/// where a name is an instruction. It is the right answer here: the accept and
/// the outcome both name the id the file was written under, so a model that
/// reads what it is told is never wrong about where its work is, and a model
/// that assumes its own name would have been wrong about a file it had already
/// destroyed.
fn filed_as(asked: Asked, id: Option<String>) -> String {
    match (asked, id) {
        (Asked::Operator, Some(id)) => id,
        (Asked::Operator, None) => history::stamped_id(),
        (Asked::Model, None) => history::stamped_id(),
        (Asked::Model, Some(name)) => format!("{}_{name}", history::stamped_id()),
    }
}
