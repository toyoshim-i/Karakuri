pub mod history;
pub mod operate;
pub mod procedure;
pub mod set;
pub mod wire;

use std::sync::mpsc;

use karakuri_ir::Kind;
use karakuri_operation::gate::{self, Allowed};
use karakuri_operation::{NodeAddress, Operation};
use serde_json::{json, Value};

pub use operate::OperateRequest;
pub use procedure::check_procedure;
pub use set::{check_set_configuration, LISTED};

pub(crate) use history::*;
pub(crate) use operate::*;
pub(crate) use procedure::*;
pub(crate) use set::*;
pub(crate) use wire::*;

use crate::{
    layer_list, layer_name, layer_named, layer_of, operate_tool, operated, DiagnosticReport, News,
    Slots, State, ID_PATTERN, LAYERS, MAX_ID,
};

pub(crate) fn tools() -> Value {
    // The layers, from [`LAYERS`] rather than written out beside it. A client
    // is offered exactly what [`layer_named`] accepts and what [`Slots::path`]
    // resolves, because it is the same list — the drift this closes is the one
    // that left three of a slot's five layers unaddressable while the files
    // were sitting right there.
    let layers: Vec<&str> = LAYERS.iter().map(|layer| layer_name(*layer)).collect();
    json!([
        {
            "name": "read_procedure",
            "description":
                "The source of one node of one deck slot. `layer` says which: L1 is what \
                 the elements are and how they move, L2 a deformation applied to them, L3 \
                 the camera, L4 how they are drawn, and Field a distance function the \
                 others evaluate. Read before writing: the edit is usually small, and what \
                 is already there is the best guide to the language.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "slot": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "deck slot, from 0",
                    },
                    "layer": { "type": "string", "enum": layers },
                    "index": {
                        "type": "integer",
                        "minimum": 0,
                        "description":
                            "which node of that layer, from 0, in the order the slot's files \
                             were named. A slot draws with as many L4s as it likes — the same \
                             cloud as sprites and as strokes is one slot with two — and may \
                             simulate with more than one L1, look from more than one L3 and \
                             hold more than one Field. Omit for the first.",
                    },
                },
                "required": ["slot", "layer"],
            },
        },
        {
            "name": "write_procedure",
            "description":
                "Check a procedure and, if it compiles, write it. It is then compiled on a \
                 worker thread, swapped in at a frame boundary, and measured for thirty \
                 frames — if it costs more than the frame budget it is dropped and the \
                 previous one comes back at the time it was parked at. So an expensive \
                 mistake is survivable. **The diagnostics are the point of the return \
                 value**: if it does not compile, what comes back is what the checker \
                 said, against the source. **The check is of this procedure alone, and \
                 the slot is rebuilt whole**: everything between nodes — what a renderer \
                 consumes against what a geometry emits, what each `uses` slot is bound \
                 to, the capacities, the cost of a field inlined into its caller — is \
                 decided when the slot is assembled, so a clean write can still be \
                 followed by a build failure `swap_outcome` reports. **A `uses` \
                 declaration needs an `edge`, and `wire_input` writes one**: adding \
                 `uses <name> : <Geometry|Field|Camera|Source>` to a procedure leaves \
                 the slot unable to build until an edge says which node fills it, so \
                 write the procedure and then call `wire_input`. The builds between \
                 the two are refusals `swap_outcome` reports, and what was on air \
                 stays on air through them. **Taking one back is the half that is \
                 missing**: nothing here unbinds an edge, so a procedure rewritten \
                 without a `uses` it had been wired for leaves an edge naming a slot \
                 nothing declares, and the slot refuses to build for that instead.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "slot": { "type": "integer", "minimum": 0 },
                    "layer": { "type": "string", "enum": layers },
                    "index": {
                        "type": "integer",
                        "minimum": 0,
                        "description":
                            "which node of that layer, from 0. Omit for the first. The \
                             source's own `kind` line must name the same layer as this \
                             address, which is what stops a deformation being written over \
                             a renderer.",
                    },
                    "source": { "type": "string", "description": "the whole procedure" },
                },
                "required": ["slot", "layer", "source"],
            },
        },
        {
            "name": "wire_input",
            "description":
                "Bind one node's declared input to another node of the same deck slot \
                 — the `edge` a `uses` declaration needs before the slot can build. A \
                 procedure declares each input under a name of its own (`uses far : \
                 Geometry`, `uses shape : Field`, `uses view : Camera`, `uses only : \
                 Source`) and never names the node that fills it; the Set says that, \
                 and this is how it is said. **Both ends are node names, not \
                 addresses**: `read_procedure`'s `layer` and `index` are a position, \
                 and a position moves when a slot's files are reordered — which would \
                 silently change which geometry a morph blends towards. A node is \
                 called what the Set named it, or what its own procedure calls itself \
                 where nothing named it; `read_set` and `list_sets` answer in those \
                 names. **An edge already binding this input is replaced**, and every \
                 other edge is left alone — so changing your mind is one call and \
                 never a refusal about a slot being bound twice. **Nothing here \
                 unbinds one**: an edge outlives the `uses` that needed it, so a \
                 procedure rewritten without a `uses` it was wired for leaves an edge \
                 naming a slot nothing declares, and the slot refuses to build for \
                 that instead. **The names are the Set's and this server cannot check \
                 them**: a node the Set does not hold, a slot the node does not \
                 declare, or a far end of the wrong kind is refused where the slot is \
                 built — in the same sentence `--edge` meets — and comes back through \
                 `swap_outcome` with everything else a rebuild decided. The rebuild \
                 itself is a procedure write's: compiled on a worker thread, swapped \
                 at a frame boundary, judged on that Set's own measured frame, and \
                 left in the slot with the slot stopped if it costs too much.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "slot": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "deck slot, from 0: whose Set this edge is about",
                    },
                    "node": {
                        "type": "string",
                        "description":
                            "the node that declares the input, by name — `morph` in \
                             `--edge morph.far=sphere_shell`",
                    },
                    "input": {
                        "type": "string",
                        "description":
                            "what that node's procedure calls the input — `far`, from \
                             `uses far : Geometry`. It is `input` and not `slot` \
                             because `slot` means the deck slot in every tool here, \
                             and one word means one thing across this surface.",
                    },
                    "to": {
                        "type": "string",
                        "description":
                            "the node bound to it, by name — `sphere_shell`. A slot \
                             declared `: Geometry` or `: Source` takes an L1, `: \
                             Field` a Field, `: Camera` an L3 or the built-in camera; \
                             a far end of the wrong kind is refused where the slot is \
                             built and names what the Set holds.",
                    },
                },
                "required": ["slot", "node", "input", "to"],
            },
        },
        {
            "name": "swap_outcome",
            "description":
                "What the swap machinery has said recently: whether a written procedure \
                 landed, was overloaded, or failed to build. Call it after a \
                 write to find out what happened — a write returning cleanly means it \
                 compiled, not that it is on screen. **Overloaded is a state rather \
                 than an outcome**: one frame of that Set costs more than a frame may, \
                 so it is still in the slot and the slot has stopped updating — it \
                 takes no step and draws no frame, and the picture holds the last frame \
                 it drew. Nothing puts anything back and nothing ends it on its own: \
                 the ways out are a write that fits, an earlier version landed on the \
                 node, or the operator's hand on the fader. So a procedure this names \
                 is one worth writing again, cheaper.",
            "inputSchema": { "type": "object", "properties": {} },
        },
        {
            "name": "save_set",
            "description":
                "Keep what a slot is playing, as a Set file. It writes the material \
                 **on screen** — the versions the \
                 slot is running, by content hash, with the whole of the wiring and the \
                 state around them: the parameters, the capacities, the bindings and the \
                 seeds the live Set holds now, the edges binding each `uses` slot, \
                 whether it composites and which renderer is live, and the camera — and \
                 not what any file on disk says, which is exactly what the operator's \
                 `k` key writes. That distinction is the \
                 point: a procedure that was written and never picked up is on disk and \
                 not on screen, and this saves the screen. **What you save goes \
                 into the store's sandbox, `<store>/sandbox/`, and not into the \
                 operator's library**: the library is written by the operator's own act \
                 and nothing else, and what lands in the sandbox is the edit history of \
                 this session — the files a person goes looking for afterwards. So \
                 `read_set`, `list_sets` and `--load-set` do not reach what you write \
                 here; if the operator wants one of these in their library they save it \
                 themselves, or move the file. **It waits for the \
                 disk and tells you what happened**, so what comes back names the id it \
                 was saved under; do not report a set as kept until it does, and use the \
                 id it gives you rather than the one you asked for.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "slot": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "deck slot, from 0",
                    },
                    "id": {
                        "type": "string",
                        "pattern": ID_PATTERN,
                        "maxLength": MAX_ID,
                        "description":
                            "what to call it: letters, digits, `-` and `_`, and it \
                             becomes part of a file name. **Nothing in the sandbox is \
                             overwritten**, so what you save is filed under the moment it \
                             was saved with your name behind it — `20260905-143052-271_my_take` \
                             — and two saves under one name are two files rather than one. \
                             That is the point of the directory: it is a session's edit \
                             history, and a snapshot a later snapshot can replace is not \
                             one. Omit it and the set is named after the moment alone, \
                             which is what the operator's key press gets. **The answer \
                             names the id the file was actually written under**; use that \
                             one.",
                    },
                },
                "required": ["slot"],
            },
        },
        {
            "name": "read_set",
            "description":
                "What a saved Set holds, and what each procedure in it declares — read \
                 out of the library without loading anything and without compiling \
                 anything. A Set is a slot's material kept under a name: the operator's \
                 `k` key writes one and `--load-set ID` plays one back. **This reads the \
                 operator's library and not the sandbox `save_set` writes into**, so a \
                 set you kept yourself is not here. For every node this says which layer it is on — L1 is what \
                 the elements are and how they move, L2 a deformation, L3 the camera, \
                 L4 how they are drawn, Field a distance function the others evaluate — \
                 what the procedure calls itself, and what it *declares*: each \
                 parameter with the two numbers a value must lie between and the value \
                 it takes when nothing turns it; the element count an L1 may run at, \
                 lowest, highest and the count it runs at unless a Set says otherwise; \
                 and the attributes it emits, which are what a renderer drawn over it \
                 can consume. **Ranges are declarations, not settings**: a range says \
                 what a value will be refused outside of, not where this Set has it. \
                 Call it to choose between things you have kept, and to find out what \
                 there is to turn on one, without fetching its source and compiling it.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "pattern": ID_PATTERN,
                        "maxLength": MAX_ID,
                        "description":
                            "the id the set was filed under: letters, digits, `-` and \
                             `_`. It is what `save_set` came back naming, what \
                             `--save-set ID` was given, or the stamp a save that named \
                             nothing was called after.",
                    },
                },
                "required": ["id"],
            },
        },
        {
            "name": "list_sets",
            "description":
                "What this store holds: every Set saved into it, most recently written \
                 first, with an address and a name per node. A Set is a slot's material \
                 kept under a name — the operator's `k` key writes one and `--load-set \
                 ID` plays one back; this is **the operator's library and not the \
                 sandbox `save_set` writes into** — and until this there was no \
                 way to find out what had been kept: `read_set` answers about an id you \
                 already have, and the ids of everything saved before this conversation \
                 are not something a model can guess. **Call this first, then `read_set` \
                 on the one you want.** What comes back is a line per set and not what \
                 any of it declares: the parameters, the element counts and what a node \
                 emits are `read_set`'s answer, because they need a card per artifact and \
                 a listing that read them all would be reading a library to print an \
                 index. **What a node is called here is what `read_set` calls it** — the \
                 name the set gave it, the name its procedure gives itself where the set \
                 gave none, and the short hash of its source where there is neither, \
                 which is an ordinary state and not a damaged store. **The list is \
                 capped**: what comes back says how many matched and how many are shown, \
                 and if those differ you are looking at part of a library — narrow it \
                 with the filters rather than assuming the rest is not there.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "holds": {
                        "type": "string",
                        "description":
                            "list only sets holding a node whose name contains this, \
                             matched without regard to case — `drift_shell` finds every \
                             set built on that geometry. Omit to list everything.",
                    },
                    "layer": {
                        "type": "string",
                        "enum": layers,
                        "description":
                            "list only sets holding a node on this layer — `L2` for the \
                             ones that deform something, `Field` for the ones with a \
                             distance function. Given with `holds`, both must be true of \
                             the set, though not of the same node. Omit to list \
                             everything.",
                    },
                },
            },
        },
        {
            // **The eighth, and it is one of the seven's kind rather than
            // `operate`'s**: it reads the store — `history::list` walks
            // `<store>/history/` newest first and opens no file — which is
            // what `list_sets` and `read_set` do and what nothing on the
            // render loop's frame can do without paying for a directory walk
            // on the path that must not wait
            // ([ADR-0199](../../../docs/adr/0199-mcp-names-its-operations-and-performs-them-itself.md),
            // `docs/adr/0342-…`). The reply is the listing, which is where a
            // read's answer goes: back to the surface that asked
            // ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
            "name": "walk_history",
            "description":
                "Every version of one Set's material that compiled, most recent first. \
                 Every build this instrument accepts is filed under `<store>/history/` \
                 whether or not it stayed on screen — the gate is **compiling** and not \
                 landing, so the version that cost too much to run is in here too — and \
                 a row is the name the store filed it under: when it was written, which \
                 slot, which node of that slot, and what the procedure called itself. \
                 **This is the listing and not the landing**: put one of these versions \
                 back with `operate` naming *Put a node's previous version back* and \
                 `{\"deck\": <the row's slot>, \"revision\": {\"picked\": \"<row>\"}}`, \
                 which is what a row is a name for. **Narrowed to one Set and never to a deck**: two decks playing one \
                 Set have one history between them, and a version written while a slot \
                 was running no Set is filed under none and is matched by no id. **The \
                 walk is capped and says so**: it stops entering day directories once it \
                 has enough, so an answer that says the walk stopped short is part of \
                 what is there rather than all of it.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "set": {
                        "type": "string",
                        "pattern": ID_PATTERN,
                        "maxLength": MAX_ID,
                        "description":
                            "the Set whose versions to walk, by the id it is filed under \
                             — `list_sets` names them. Required: a walk that names no Set \
                             lists nothing, because those versions are filed under no Set \
                             rather than under all of them.",
                    },
                },
                "required": ["set"],
            },
        },
        // **The ninth, and it is generated** — see [`operate_tool`] and
        // [`SPELLED`]. The eight above are written out because each of them
        // performs something only this server can; this one is the vocabulary,
        // and a hand-written copy of it beside the vocabulary is the drift
        // `karakuri-operation` exists to end.
        operate_tool(),
    ])
}

/// What one tool call came to: an answer, or a wait that belongs outside the
/// state lock. See [`Pending`].
pub(crate) enum Called {
    Answered(Result<String, String>),
    Saving(mpsc::Receiver<News>),
    /// An edge the loop has been asked for, and what this server has to add to
    /// whatever it answers — see [`Pending::Wiring`].
    Wiring {
        news: mpsc::Receiver<News>,
        note: String,
    },
    /// An operation the loop has been asked to perform — see [`OperateRequest`]. No
    /// note beside it: what this server knows about the run that the loop will not
    /// say is the class the audit refused on, and a refusal never reaches here.
    Operating(mpsc::Receiver<News>),
}

/// What one tool call names, in the vocabulary — or the refusal its arguments
/// earned.
///
/// A tool call is a request from outside the process naming a thing to do,
/// which is a MIDI message's shape rather than a key press's, and
/// [ADR-0196](../../../docs/adr/0196-a-map-line-names-a-state-and-an-old-line-is-refused.md)
/// is what that surface did with it: message becomes `Operation`, and something
/// else performs it. The second half of that does not exist here and cannot.
/// `karakuri_operation_record::written` answers `Silent` for all six of these —
/// `Question` for the four that ask and `OnLanding` for the two whose record is
/// written where the work lands — so `Live::operate` would print *no record*
/// and do nothing, which is
/// [ADR-0198](../../../docs/adr/0198-a-gesture-converts-in-the-parts-that-are-decided.md)'s
/// finding about twelve keys, holding here for all six tools. There is also no
/// `Live` on this thread to route into: this server reaches the render loop for
/// exactly one thing, over the channel [`SaveRequest`] travels on.
///
/// So what routes is the naming. The wire's own words — a tool name and a JSON
/// object — become the operation the manual specifies, once, here; and
/// [`perform`] dispatches on that operation rather than on the string. A tool
/// whose payload the vocabulary cannot say does not compile, and the row on
/// `docs/manual/operations.html` that a tool claims is the row its operation's
/// title names rather than one a second list asserts.
///
/// The order arguments are refused in is the order they were refused in before
/// this routed, deliberately: a change of route may not change what a tool
/// answers, and the refusals here are the surface's product — a model that is
/// told which mistake it made fixes its own call. So the slot is checked where
/// each tool checked it, `checked_id` runs where each tool ran it, and nothing
/// new is decided in front of anything old.
pub(crate) fn asked(name: &str, args: &Value, slots: &Slots) -> Result<Asked, String> {
    Ok(match name {
        "read_procedure" => match address(args, slots) {
            Ok((deck, node)) => Asked::Named(Operation::ReadProcedure { deck, node }),
            Err(refusal) => Asked::Refused(refusal),
        },
        "write_procedure" => match written_procedure(args, slots) {
            Ok(operation) => Asked::Named(operation),
            Err(refusal) => Asked::Refused(refusal),
        },
        "wire_input" => match wired_input(args, slots) {
            Ok(operation) => Asked::Named(operation),
            Err(refusal) => Asked::Refused(refusal),
        },
        "swap_outcome" => Asked::Named(Operation::SwapOutcome),
        "read_set" => match named_set(args) {
            Ok(operation) => Asked::Named(operation),
            Err(refusal) => Asked::Refused(refusal),
        },
        "list_sets" => match listing(args) {
            Ok(operation) => Asked::Named(operation),
            Err(refusal) => Asked::Refused(refusal),
        },
        // **The store's other listing**, beside `list_sets` for its reason: the
        // walk is a directory read this thread can do and the render loop
        // cannot afford, and its answer is rows rather than a report that
        // something was performed (`docs/adr/0342-…`).
        "walk_history" => match walked_history(args) {
            Ok(operation) => Asked::Named(operation),
            Err(refusal) => Asked::Refused(refusal),
        },
        "save_set" => match kept(args, slots) {
            Ok(operation) => Asked::Named(operation),
            Err(refusal) => Asked::Refused(refusal),
        },
        // **The ninth tool, and the one that names rather than does.** The
        // eight above each turn a tool's own arguments into the operation the
        // manual specifies; this one is handed the operation's own name and
        // looks it up — see [`operated`] and [`SPELLED`]. It is here at the end
        // rather than first so that a tool with a name of its own is still
        // matched by that name, which is what keeps `operate` from becoming a
        // second spelling of any of them.
        "operate" => match operated(args, slots) {
            Ok(operation) => Asked::Named(operation),
            Err(refusal) => Asked::Refused(refusal),
        },
        other => return Err(format!("no tool `{other}`")),
    })
}

/// What [`asked`] made of one call: the operation, or what the caller is told
/// instead.
///
/// The refusal is a tool result and not a protocol error, which is why it is
/// carried in the `Ok` half rather than returned — see [`tool_result`]. The
/// `Err` of [`asked`] is the one thing that really is a protocol mistake: a
/// tool this server does not publish.
pub(crate) enum Asked {
    Named(Operation),
    Refused(String),
}

/// The deck one slot number names.
///
/// `Operation` carries `deck: u8`, and every `slot` in
/// `karakuri_store::record::Record` is a `u8` too, so a slot past 255 is not a
/// deck anything in this program can address. Refused in the words
/// [`Slots::nodes`] and [`Slots::holds`] refuse an absent slot in, because it
/// is the same mistake and an operator is told one story about it
/// ([`crate::no_such_slot`]).
///
/// Called after every argument the tool used to parse before it reached the
/// slot, so that a call with two mistakes in it is still told about the same
/// one it was told about before.
pub(crate) fn deck_named(slot: usize, slots: &Slots) -> Result<u8, String> {
    slots.holds(slot)?;
    u8::try_from(slot).map_err(|_| karakuri_environment::no_such_slot(slot, slots.count()))
}

/// `read_procedure`'s arguments as the deck and node they name.
pub(crate) fn address(args: &Value, slots: &Slots) -> Result<(u8, NodeAddress), String> {
    let (slot, layer, index) = slot_layer_index(args)?;
    let deck = deck_named(slot, slots)?;
    Ok((
        deck,
        NodeAddress {
            layer: layer_of(layer),
            index: index as u32,
        },
    ))
}

/// What this surface can say of one operation, and why it cannot where it
/// cannot.
///
/// The `operate` tool takes an operation of `karakuri-operation` by its own
/// name — the heading `docs/manual/operations.html` specifies it under — and
/// hands it to the frame the panel performs every other surface's presses on.
/// It does not take all sixty-four, and the four reasons it does not are here
/// rather than in four scattered refusals
/// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).
///
/// No wildcard arm. [`sayable`] is a `match` over every variant, which is
/// `karakuri_operation::gate::standing`'s discipline and its reason: a
/// sixty-fifth operation does not compile until somebody has said whether this
/// surface can name it, and the page's MCP column cannot quietly go stale
/// beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sayable {
    /// `operate` takes it. The audit still answers.
    Operable,
    /// This server publishes a tool of its own for it, which does something only
    /// the server can — a file, a store, a listing. A second spelling of a tool is
    /// a second spelling
    /// ([P-0087](../../../docs/principles/0087-name-the-property-never-the-shape.md)),
    /// so `operate` refuses it and names the tool.
    Tool(&'static str),
    /// A model has no window. The row's MCP badge is `gap` and the sentence is
    /// [ADR-0315](../../../docs/adr/0315-a-model-has-no-window-so-the-twelve-surface-rows-mcp-badges-are-gap.md)'s,
    /// worded once here as it is worded once on the page.
    Window,
    /// This surface will not reach it, and the clause says why and where the route
    /// that does is. The row's MCP badge is `gap`, and what makes it `gap` rather
    /// than `plan` is that nothing is owed: no performer moving onto the drain's
    /// frame would change it, because what stops it is the shape of this protocol
    /// rather than a gap in this program
    /// ([ADR-0341](../../../docs/adr/0341-a-route-that-answers-is-built-and-a-send-that-ends-in-a-dialog-is-gap.md)).
    ///
    /// There was a sixth answer beside this one until 2026-09-10 — `Unperformed`,
    /// *the vocabulary names it and nothing on this frame performs it yet*, which
    /// is what a `plan` badge in the MCP column meant. It went when its last row
    /// did (ADR-0341): every operation this vocabulary names either has a
    /// performer, has a tool, is a window's, is unsettled, or is this. A `plan`
    /// badge in that column is now only an `Undecided` payload, and the day a row
    /// is added that a surface can say and the frame cannot perform, this `match`
    /// has no wildcard and stops the build until somebody puts the answer back.
    Never(&'static str),
}

/// Whether `operate` names this operation, and what it says where it does not.
/// Exhaustive, with no wildcard arm — see [`Sayable`].
pub(crate) fn sayable(operation: &Operation) -> Sayable {
    match operation {
        // ----- the thirty `operate` takes ----------------------------------
        //
        // Twenty-eight of them are refused by the audit until an operator opens
        // their class, which is a built route and not a missing one
        // (ADR-0235: *"a closed class is reached and answered with a refusal"*).
        // `Operation::RestoreProcedure` is the one the audit lets through and
        // the panel then performs, and `Operation::LoadSet` is the one whose
        // class is a predicate over its target.
        Operation::TapBeat
        | Operation::ScaleGrid { .. }
        | Operation::SetLatencyOffset { .. }
        | Operation::SetSync { .. }
        | Operation::ScrubDeck { .. }
        | Operation::SetFreeRunTempo { .. }
        | Operation::AttachBeatSource { .. }
        | Operation::SetResidency { .. }
        | Operation::LoadSet { .. }
        | Operation::SetCompositing { .. }
        | Operation::SetGain { .. }
        | Operation::SetOpacity { .. }
        | Operation::SetBlendMode { .. }
        | Operation::FadeDeck { .. }
        | Operation::Crossfade { .. }
        | Operation::Wipe { .. }
        | Operation::SetMaskShape { .. }
        | Operation::SelectRenderer { .. }
        | Operation::SetMasterOut { .. }
        | Operation::SetFeedback { .. }
        | Operation::SetBloom { .. }
        | Operation::SetRgbShift { .. }
        | Operation::SetTonemap { .. }
        | Operation::SetExposure { .. }
        | Operation::WriteParam { .. }
        | Operation::AttachSignal { .. }
        | Operation::TakeParamBack { .. }
        | Operation::SetProperty { .. }
        | Operation::SetAuthority { .. }
        | Operation::RestoreProcedure { .. }
        // **The five ADR-0334 left `plan` for want of a performer on this
        // frame, and each of them has one now** (ADR-0341). Four were already
        // written and sat in the window's own press arm — the star's refusal,
        // the projector's window, the `rec` pill's two ends and the publish
        // mark's re-aim — and `App::operated` calls them where the pointer's
        // button-up arm calls them; the mask position was a missing reading and
        // is one arm of `crates/karakuri`'s `reading`. They leave this list by
        // having a performer rather than by this rule bending, which is the
        // shape ADR-0334 said each of them would leave in.
        | Operation::SetMaskPosition { .. }
        | Operation::Publish { .. }
        | Operation::SetFavourite { .. }
        | Operation::RouteFrame { .. }
        | Operation::RecordSession { .. }
        // **And ADR-0338's load, which left the `plan` column on 2026-09-10 for
        // the reason *Narrow the published interface* did**: its performer was
        // never missing. `overlaid` sits in `App::performed`, in the arm the
        // fold and the library load are in, and this drain lands there — so
        // the sentence this row used to carry, *nothing re-aims a slot with
        // one layer replaced yet*, had stopped being true before it was read.
        | Operation::LoadProcedure { .. }
        // **And ADR-0338's keep, which left the `plan` column on 2026-09-10 by
        // gaining a performer.** `App::operated` hands it to
        // `Keeping::keep_procedure` where the pointer's button-up arm hands
        // the capsule's press to the same method, and the two differ in one
        // argument: a model's is `Asked::Model` and lands in
        // `<store>/sandbox/`, stamped and overwriting nothing, where an
        // operator's own act writes `<store>/procedures/`
        // (`docs/principles/0096-…`, `docs/adr/0261-…`).
        //
        // **A model is not refused here where its star is**, and ADR-0301 is
        // why: a favourite has no sandbox form to land in and a kept procedure
        // is a file, so it has one.
        //
        // **The answer arrives when the file does.** A keep is *"on a
        // worker"*, so the drain hands the reply to the write thread rather
        // than saying *performed* on the frame it arrived on — which is
        // `save_set`'s own arrangement one file kind along.
        | Operation::KeepProcedure { .. }
        | Operation::Quit => Sayable::Operable,

        // ----- the seven that have a tool of their own ---------------------
        Operation::ReadProcedure { .. } => Sayable::Tool("read_procedure"),
        Operation::WriteProcedure { .. } => Sayable::Tool("write_procedure"),
        Operation::WireInput { .. } => Sayable::Tool("wire_input"),
        Operation::SwapOutcome => Sayable::Tool("swap_outcome"),
        Operation::SaveSet { .. } => Sayable::Tool("save_set"),
        Operation::ReadSet { .. } => Sayable::Tool("read_set"),
        Operation::ListSets { .. } => Sayable::Tool("list_sets"),
        // **The eighth, and it joined the seven on 2026-09-10 by having its
        // payload settled** — it names the Set it is a walk of now, so this
        // surface can say it. It is a tool rather than an `operate` name for
        // the property that puts the other seven here and not for its shape: it
        // reads the store and answers with rows, which is what only this server
        // can do, and a walk performed on the drain's frame would be a
        // directory walk on the path that must not wait. See
        // `docs/adr/0342-…`.
        Operation::WalkHistory { .. } => Sayable::Tool("walk_history"),

        // ----- the seventeen a model has no window for ---------------------
        //
        // ADR-0315's twelve and the sequencer's five, which carry that record's
        // sentence on the page for the same reason: a route into a surface's
        // own state is a route into a window the model is not looking at.
        Operation::SelectDeck { .. }
        | Operation::SelectScope { .. }
        | Operation::SetTransition { .. }
        | Operation::KeepCandidate { .. }
        | Operation::FoldBay { .. }
        | Operation::FoldPane { .. }
        | Operation::Unfold { .. }
        | Operation::Solo { .. }
        | Operation::ResetArrangement
        | Operation::SaveArrangement { .. }
        | Operation::RestoreArrangement { .. }
        | Operation::SizeWindow { .. }
        | Operation::SetStep { .. }
        | Operation::SetLaneMute { .. }
        | Operation::PointLane { .. }
        | Operation::SetPatternGrid { .. }
        | Operation::SelectPattern { .. }
        // **A bay's own narrowing of what it is drawing**, which is
        // `Operation::SelectScope`'s answer arrived at from the other side:
        // which kinds of row the Library bay shows is a fact about a window,
        // and a model is not looking at one. What a model actually wants here
        // is a listing that holds procedures at all, and that is
        // `Operation::ListSets`' to grow rather than this row's
        // (`docs/adr/0338-a-procedure-is-a-row-of-the-library-and-one-loaded-over-a-layer-makes-a-set-with-no-name.md`).
        | Operation::FilterLibrary { .. }
        // **A pane's target is a pointer inside one console**, like the deck
        // selection two groups up and for its sentence.
        | Operation::PointPane { .. } => Sayable::Window,

        // ----- the group that emptied on 2026-09-10 ------------------------
        //
        // **There were three here — the payloads the vocabulary had not
        // settled — and all three left on one day** (`docs/adr/0342-…`).
        // `Operation::WalkHistory` left by having its payload settled and is a
        // tool above. The other two left by being asked the question this
        // classification is actually about: **not** *is the payload settled*
        // but *what can this surface reach*. `Operation::MoveBoundary` is a
        // surface's own state and is [`Sayable::Window`] with ADR-0315's
        // sentence — it was that record's own thirteenth row, held out of it
        // only because its payload is open. `Operation::WatchFiles` names an
        // event a model does not perform and is [`Sayable::Never`] below.
        //
        // **So `Sayable::Undecided` is gone**, the way `Unperformed` went in
        // ADR-0341 and for its reason: a variant nothing constructs is dead
        // code that says something false about the program, and this `match`
        // has no wildcard, so the day an operation arrives that a surface
        // cannot say the build stops until somebody puts an answer back.
        //
        // `Operation::SelectScope` is here for the window's reason too, which
        // is where it has always been.
        Operation::MoveBoundary { .. } => Sayable::Window,

        // ----- the two no route here will ever take -------------------------
        //
        // **This was the last group and it used to have a neighbour**: rows
        // the vocabulary named that nothing on the drain's frame performed
        // yet, which is what a `plan` badge in the MCP column meant.
        // `SetProperty` left it when the Inspector's deck head grew the two
        // chips that perform it (ADR-0328), the five ADR-0334 named followed
        // on 2026-09-10, and ADR-0338's two went the same day (ADR-0341) — so
        // that group and its answer are gone, and this row is what is left:
        // the one that is not waiting for anything.
        Operation::TransferSet { .. } => Sayable::Never(
            "both halves of it are outside what this protocol carries. A send names no \
             destination and never will — it is a read, and a read's answer goes where the \
             surface that asked puts answers, which on the panel is the system's own save \
             dialog and a model cannot answer one. A take names a file, and paths never cross \
             this protocol. The route is the command line: `--package ID > FILE.kbset` sends \
             one and `--take-in FILE` takes one in",
        ),
        // **A model does not edit a file in another program**, which is the
        // whole of what that row names: somebody saving a source a watcher is
        // looking at, and the rebuild that follows. What a model has instead is
        // the act itself — `write_procedure` **is** its edit — so there is
        // nothing here it would say that the write does not already say, and
        // nothing is owed. That is `docs/adr/0205-…`'s kind of answer: a route
        // whose only content is a second spelling of one that exists is not a
        // route somebody has yet to build (`docs/adr/0342-…`).
        Operation::WatchFiles { .. } => Sayable::Never(
            "a model does not edit a file in another program: it calls `write_procedure`, \
             which **is** its edit — checked, written, built on a worker and swapped at a \
             frame boundary — and there is nothing this row would add to it. The row names \
             an event rather than an act, somebody saving a source a watcher is looking at, \
             and the watching itself is the command line's: `--watch` turns it on for a run",
        ),
    }
}

/// One named operation, done.
///
/// It takes an [`Allowed`] and not an [`Operation`], which is the audit made
/// structural. `karakuri_operation::gate::audit` is the only thing that builds
/// one and its field is private to that crate, so there is no way to reach this
/// function with an operation nobody checked — a path that skipped the gate
/// does not compile rather than passing review.
/// [ADR-0235](../../../docs/adr/0235-mcp-reaches-every-operation-and-what-could-stop-the-show-is-refused-until-the-operator-opens-it.md):
/// *"an audit skipped on one path is the whole mechanism gone."*
///
/// The dispatch is over the vocabulary rather than over the tool's name, which
/// is the whole of what routing buys this surface: the arm that reads a
/// procedure is chosen by [`Operation::ReadProcedure`], so a tool renamed on
/// the wire goes on doing what its operation says, and a tool that named a
/// different operation would visibly do something else.
///
/// The last arm cannot happen — [`asked`] builds seven operations and this
/// matches those seven. It is written out rather than left to a wildcard for
/// [`absent`]'s reason: the arm that cannot happen is the one that stops saying
/// so quietly when the shape around it changes, and if an eighth tool ever
/// arrives without an arm here the client is told which operation nothing
/// performs rather than being answered by the wrong one.
pub(crate) fn perform(allowed: &Allowed<'_>, state: &mut State) -> Called {
    match allowed.operation() {
        Operation::ReadProcedure { deck, node } => {
            Called::Answered(read_procedure(*deck, *node, state))
        }
        Operation::WriteProcedure { deck, node, source } => {
            Called::Answered(write_procedure(*deck, *node, source, state))
        }
        Operation::SwapOutcome => Called::Answered(swap_outcome(state)),
        // **Refused here for what this server can decide and waited for
        // elsewhere**, which is `save_set`'s shape and for the same reason: the
        // wiring a slot rebuilds with is the render loop's, and the names in it
        // are the Set's.
        Operation::WireInput {
            deck,
            node,
            slot,
            to,
        } => match wire_input(*deck, node, slot.as_str(), to, state) {
            Ok((news, note)) => Called::Wiring { news, note },
            Err(refusal) => Called::Answered(Err(refusal)),
        },
        // Answered here like a read and unlike `save_set`: a card is a file, the
        // render loop does not hold one, and there is nothing to wait for.
        Operation::ReadSet { id } => Called::Answered(read_set(id, state)),
        // A directory read and a file read per set, and nothing else — see
        // [`list_sets`]. Answered here for the same reason `read_set` is.
        Operation::ListSets { holds, layer } => {
            Called::Answered(list_sets(holds.as_deref(), *layer, state))
        }
        // **The store's other listing, and no file is opened at all** — see
        // [`walk_history`]. Answered here for `list_sets`' reason, and it is
        // the reason this row is a tool rather than a name `operate` takes:
        // handing a directory walk to the render loop would put it on the path
        // that must not wait.
        Operation::WalkHistory { set } => Called::Answered(walk_history(set.as_deref(), state)),
        // **Refused before it is sent and waited for elsewhere.** Everything
        // this module can decide by itself — a slot that does not exist, an `id`
        // that is not a name — was decided in [`asked`] under the lock like any
        // other tool's arguments, and only the wait for somebody else's thread
        // is deferred.
        Operation::SaveSet { deck, id } => match save_set(*deck, id.as_deref(), state) {
            Ok(news) => Called::Saving(news),
            Err(refusal) => Called::Answered(Err(refusal)),
        },
        // **Everything `operate` names, and it is one arm because it is one
        // route.** The seven above are matched by their own operations, so
        // nothing reaches here that has a tool of its own; what does reach here
        // has been through [`operated`], which took it only if [`sayable`] says
        // this surface can name it, and then through [`audited`]. So the answer
        // is *hand it to the frame every other surface's presses are performed
        // on* — and the last arm is the one that cannot happen, kept for
        // [`absent`]'s reason.
        other => match sayable(other) {
            Sayable::Operable => match operate(other, state) {
                Ok(news) => Called::Operating(news),
                Err(refusal) => Called::Answered(Err(refusal)),
            },
            _ => Called::Answered(Err(format!(
                "`{}` is an operation this server publishes no tool for",
                other.title()
            ))),
        },
    }
}

pub(crate) fn call_tool(request: &Value, state: &mut State) -> Result<Called, String> {
    let params = request.get("params").ok_or("no params")?;
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or("no tool name")?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    if name == "check_procedure" {
        let source = args
            .get("source")
            .and_then(Value::as_str)
            .ok_or("`source` is required")?;
        let report = check_procedure(source);
        let report_json = serde_json::to_string_pretty(&report).unwrap_or_default();
        return Ok(Called::Answered(if report.success {
            Ok(report_json)
        } else {
            Err(report_json)
        }));
    }
    if name == "check_set" {
        let id = args
            .get("id")
            .and_then(Value::as_str)
            .ok_or("`id` is required")?;
        let report = check_set_configuration(&state.store, id);
        let report_json = serde_json::to_string_pretty(&report).unwrap_or_default();
        return Ok(Called::Answered(if report.success {
            Ok(report_json)
        } else {
            Err(report_json)
        }));
    }

    Ok(match asked(name, &args, &state.slots)? {
        // **The gate, and there is one of it.** Named, then audited, then done
        // — every tool crosses this seam because [`perform`] takes what
        // [`audited`] returns and nothing else can make one.
        Asked::Named(operation) => match audited(&operation, state) {
            Ok(allowed) => perform(&allowed, state),
            Err(refused) => Called::Answered(Err(refused)),
        },
        Asked::Refused(refusal) => Called::Answered(Err(refusal)),
    })
}

/// This surface's one call into the audit.
///
/// The classification, the four classes and the refusal sentence are
/// `karakuri_operation::gate`'s and not this module's, which is
/// [ADR-0236](../../../docs/adr/0236-a-map-is-the-layer-between-a-surface-and-the-vocabulary-and-the-audit-is-one-of-the-things-it-does.md)
/// refusing to let one surface hold the rule: *"a rule held by one surface
/// binds one surface"*, and a sequencer lane is already decided as a fifth
/// route that would otherwise arrive with a second copy of the table. What is
/// this module's is the two things only it can supply — the opening the run was
/// handed and what it has read of what is running.
///
/// And it has read nothing, which is said rather than defaulted.
/// `Running::unread()` is honest: this server holds `Slots`, a store root and a
/// watch flag, and no residency at all. Exactly one row turns on that reading —
/// `Operation::LoadSet`, whose class is *a deck in live mode* — and this
/// surface publishes no tool that names it, so nothing is refused today that
/// was not refused yesterday. The day a `load_set` tool lands it is refused
/// with *which decks are live was not read* until somebody wires the reading,
/// which is
/// `docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md`'s
/// answer rather than a guess that the deck is idle.
pub(crate) fn audited<'a>(operation: &'a Operation, state: &State) -> Result<Allowed<'a>, String> {
    gate::audit(operation, state.opening.read(), gate::Running::unread())
}

/// One tool call's answer, in the shape the protocol gives a tool.
///
/// A tool failure is a result, not a protocol error. A model that is told "the
/// call was malformed" learns nothing; one handed the checker's diagnostics can
/// fix its own source, which is the whole loop.
///
/// A function rather than a `json!` at each call site, because `save_set`'s
/// answer is built after the lock is gone — see [`Pending`] — and two spellings
/// of this shape would be two chances to disagree about `isError`.
pub(crate) fn tool_result(outcome: Result<String, String>) -> Value {
    match outcome {
        Ok(text) => {
            if let Ok(report) = serde_json::from_str::<DiagnosticReport>(&text) {
                json!({ "content": [{ "type": "text", "text": text }], "isError": false, "report": report })
            } else {
                json!({ "content": [{ "type": "text", "text": text }], "isError": false })
            }
        }
        Err(text) => {
            if let Ok(report) = serde_json::from_str::<DiagnosticReport>(&text) {
                json!({ "content": [{ "type": "text", "text": text }], "isError": true, "report": report })
            } else {
                json!({ "content": [{ "type": "text", "text": text }], "isError": true })
            }
        }
    }
}

/// `index` is optional and defaults to 0, unlike the wildcard an absent `index`
/// means on a `param` record. The difference is the same one that runs through
/// the whole address: this names *a procedure to read or rewrite*, and there is
/// no such thing as rewriting every renderer at once with one source — where a
/// `param` addresses a *value*, and one value reaching every declaration is
/// both meaningful and the useful default.
///
/// It defaults because the first node of a layer is what a client that says
/// nothing means, on every layer: a slot holding one camera and one shape has
/// nothing else `index` could name, and one holding two has an order its files
/// were given in.
///
/// The layer is parsed here into the compiler's own `Kind` and travels as one
/// from here on, so the layer this resolves a file for and the layer a written
/// source is checked against are the same value rather than two readings of one
/// string.
pub(crate) fn slot_layer_index(args: &Value) -> Result<(usize, Kind, usize), String> {
    let slot = args
        .get("slot")
        .and_then(Value::as_u64)
        .ok_or("`slot` is required and is a number")? as usize;
    let named = args
        .get("layer")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("`layer` is required and is one of {}", layer_list()))?;
    let layer = layer_named(named)
        .ok_or_else(|| format!("no layer `{named}`: a slot's nodes are {}", layer_list()))?;
    let index = match args.get("index") {
        None => 0,
        Some(v) => v
            .as_u64()
            .ok_or("`index` is a number: which node of that layer, from 0")?
            as usize,
    };
    Ok((slot, layer, index))
}

/// A Set id a client may name, or why not.
///
/// A Set id is one path component. [`crate::history::stamped_id`] says so where
/// it explains why the date is spelled `20260816` rather than `2026/08/16`, and
/// the store spells the file `<dir>/<id>.kbset` without checking that what it
/// was handed is one. That is the operator's own business on `--save-set`,
/// where the id came out of their own shell. It is not a model's: this is the
/// same rule [`Slots`] exists for — paths never cross the protocol — and
/// `../../../somewhere/else` is a path.
///
/// Letters, digits, `-` and `_`, which is what a stamp is made of and what a
/// name anybody would type is made of. Refused rather than sanitised: a set
/// filed under a name its caller did not ask for is a worse answer than one
/// that is told to pick another.
///
/// A name a client picks twice no longer overwrites, and the reason it once did
/// no longer holds. This paragraph said the opposite until 2026-09-05, and the
/// argument it made was sound on its own premise: a name a caller typed is an
/// instruction, `--save-set ID` has always obeyed it by overwriting, and
/// `save_set` did what `--save-set` did. What changed is where a model's save
/// lands. It writes `<store>/sandbox/` rather than the operator's library
/// ([P-0096](../../../docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md)),
/// and nothing in that directory is an id an operator typed — it is a session's
/// edit history, and a snapshot a later snapshot can replace is not one. So
/// `crate::filed_as` puts the stamp in front of whatever name a client chose
/// and the accept names what was written; the renaming this paragraph refused
/// is still refused here, because refusing a bad id and naming a good file are
/// two different jobs and this one is still the first. What remains true
/// unchanged is that the behaviour is documented — in the tool description a
/// model reads and in `docs/manual.md`. Undocumented was the thing that was not
/// allowed.
///
/// No `con`, `nul`, `aux`, `com1` check. They are reserved device names on
/// Windows and would be a file that is not a file. There is no Windows target
/// today and no `cfg` for one here; this sentence is the record that the case
/// is known, so that whoever ports this finds it written down rather than finds
/// it on a projector.
///
/// Public since 2026-09-08, because it stopped being the model's wall alone:
/// the Inspector pane head's name is an operator typing an id, which
/// `crate::filed_as` passes straight through for `Asked::Operator` (ADR-0292).
/// One wall, so a `/` cannot become a path on either route.
pub fn checked_id(id: &str) -> Result<String, String> {
    if id.is_empty() {
        return Err(
            "`id` is empty: a set is filed under a name, or under none at all if \
                    `id` is left out"
                .into(),
        );
    }
    // **Bytes, and the message says bytes.** `str::len` is bytes and this said
    // "characters", which is the same number for everything that gets past the
    // charset check below and a different one for what does not — so the one
    // caller the message existed for, the one sending something this refuses,
    // was told a number it could not count to.
    if id.len() > MAX_ID {
        return Err(format!(
            "`id` is {} bytes and the most is {MAX_ID}: it becomes a file name",
            id.len()
        ));
    }
    if let Some(bad) = id
        .chars()
        .find(|c| !c.is_ascii_alphanumeric() && *c != '-' && *c != '_')
    {
        return Err(format!(
            "`id` holds `{bad}`, and a set id is letters, digits, `-` and `_`: it is one \
             path component and it names a file in the store"
        ));
    }
    Ok(id.to_string())
}

/// Wait for one save's outcome, and say something true when it does not come.
///
/// A free function over the channel rather than a loop inside [`Pending`], for
/// the reason `main.rs`'s `drained_saves` is one: the bound is the whole of
/// what makes waiting here safe, and it has to be checkable without a render
/// loop, a window or a disk.
///
/// It waits, rather than returning on acceptance, and that was the decision
/// worth arguing. Answering the moment the loop has the request would make this
/// tool cheap and its answer worthless: a model told "saved" before the disk
/// has spoken will tell its user the set is kept, and the cases where that is a
/// lie — a full store, a network mount that stopped answering, a slot whose
/// sources are not savable — are precisely the ones anybody would want to hear
/// about. The other three tools already work this way: `write_procedure` hands
/// back the checker's verdict and not "it is being checked".
///
/// A timeout is neither success nor failure, and the text says so. The protocol
/// has one boolean and it cannot carry a third state, so `isError` is set — a
/// model reading `isError: false` reports the set as kept, which is the one
/// thing that must not happen here, while a model reading `true` looks again.
/// What the flag cannot carry, the sentence does.
pub(crate) fn awaited(
    news: &mpsc::Receiver<News>,
    wait: std::time::Duration,
) -> Result<String, String> {
    let deadline = std::time::Instant::now() + wait;
    let mut accepted: Option<String> = None;
    while let Some(left) = deadline.checked_duration_since(std::time::Instant::now()) {
        match news.recv_timeout(left) {
            Ok(News::Accepted(said)) => accepted = Some(said),
            Ok(News::Settled(outcome)) => return outcome,
            Err(mpsc::RecvTimeoutError::Timeout) => break,
            // **The loop dropped the request without answering it**, which is
            // what the end of a run looks like from here. Which of the two
            // sentences depends on whether it was ever taken: one that was
            // never taken saved nothing, and one that was may well have reached
            // the disk on the way out.
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(match accepted {
                    Some(said) => format!(
                        "{said}\n\nThe render loop then ended without saying what became of \
                         it. Whether that file was written is not something this server can \
                         still find out."
                    ),
                    None => "the render loop ended before it took this save: nothing was saved"
                        .to_string(),
                })
            }
        }
    }
    Err(match accepted {
        Some(said) => format!(
            "{said}\n\n**This is neither a success nor a failure.** The save was accepted \
             and had not reported back after {wait:?}. It is being written or it is not; \
             nothing here knows which, and no `save` record claims either way until it \
             lands. Do not report the set as kept — look for it under that id."
        ),
        None => format!(
            "the render loop had not taken this save after {wait:?} — it is running slowly \
             or not at all. Nothing was saved, and asking again is safe."
        ),
    })
}

/// Wait for one edge to be applied, and say something true when it is not.
///
/// Not [`awaited`], because a save and an edge are not waiting for the same
/// kind of thing. A save's third state is real and unavoidable — the loop took
/// it, the disk has not answered, and *neither a success nor a failure* is the
/// only honest report. An edge has no such state by construction: the loop
/// applies it at the frame it takes it and answers there, and everything slow
/// about it — the compile, the swap, the thirty judged frames — happens after
/// the answer and is `swap_outcome`'s to report. So the two sentences a timeout
/// can produce here are *it was not taken* and *it was taken and then the loop
/// went quiet*, and the second one is a loop at odds with what [`WireRequest`]
/// says it owes rather than an ordinary outcome.
///
/// A run whose loop does not drain [`Reporter::wires`] at all ends up in the
/// first of those, which is the point: a tool that reported success into a
/// channel nobody empties would be this surface claiming work that never
/// happened.
pub(crate) fn applied(
    news: &mpsc::Receiver<News>,
    wait: std::time::Duration,
) -> Result<String, String> {
    let deadline = std::time::Instant::now() + wait;
    let mut accepted: Option<String> = None;
    while let Some(left) = deadline.checked_duration_since(std::time::Instant::now()) {
        match news.recv_timeout(left) {
            Ok(News::Accepted(said)) => accepted = Some(said),
            Ok(News::Settled(outcome)) => return outcome,
            Err(mpsc::RecvTimeoutError::Timeout) => break,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(match accepted {
                    Some(said) => format!(
                        "{said}\n\nThe render loop then ended without saying what it did \
                         with the edge. Whether the slot was rewired is not something this \
                         server can still find out — `read_set` on a set saved since would \
                         say, and nothing else here will."
                    ),
                    None => "the render loop ended before it took this edge: nothing was \
                             rewired"
                        .to_string(),
                })
            }
        }
    }
    Err(match accepted {
        Some(said) => format!(
            "{said}\n\n**This is neither a success nor a failure.** The edge was taken and \
             the loop had not said what it did with it after {wait:?}, which is a frame it \
             should have answered on. Do not write the same edge again on the assumption \
             that it was lost — ask `swap_outcome` what the slot has been doing."
        ),
        None => format!(
            "the render loop had not taken this edge after {wait:?} — it is running slowly, \
             it is not running frames at all, or this run's loop does not take edges. \
             Nothing was rewired, and asking again is safe."
        ),
    })
}
