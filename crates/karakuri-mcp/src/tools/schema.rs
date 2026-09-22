use serde_json::{json, Value};

use crate::{layer_name, operate_tool, ID_PATTERN, LAYERS, MAX_ID};

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
