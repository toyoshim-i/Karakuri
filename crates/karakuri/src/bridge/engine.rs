use super::*;

/// The look this window opens under, and it is where [`Engine::look`] starts
/// rather than what every frame is drawn under.
///
/// [`compose`] writes the tone-map uniform on every frame from the
/// [`Committed`] the closure hands back, so a harness with nothing to say about
/// the look still has to say something. This file now has something to say: the
/// transport row's two look controls move [`Engine::look`] through a record, so
/// what a frame is committed under is that field and this is only its first
/// value.
///
/// Aces, and it is still not this program inventing an aesthetic. ADR-0037
/// picked the default *by looking* and left the trade open — *"ACES works on
/// stage … AgX is kind to material"* — and recorded that the choice is only
/// about what happens when nobody chooses, *"and can be changed on the night"*.
/// Until this pass nobody could change it here; now a press can, and the
/// constant is what the night starts at.
pub(crate) const LOOK: Look = Look {
    op: TonemapOp::Aces,
    exposure: 1.0,
    white_point: 1.0,
};

/// The engine behind the Program bay: a deck of [`SLOTS`] Sets, the present
/// pass, and the two textures it lands in.
///
/// Scaffolding still in what it is wired to — no audio, no MIDI, no store, no
/// arguments — and a watcher on each slot, which is the one thing here that is
/// not the shortest path to texels and is there because the Staging lane's rows
/// are verdicts on builds ([`watched`]). What `karakuri-cli` does around this
/// is a program; what is here is the shortest path from two `.kir` files to
/// texels — and now back again, which is what a watched slot is.
///
/// The deck is full, and the slots are channels rather than exhibits. It has
/// every slot a `Deck` can hold, because a strip is a slot and a mixer is its
/// channels; what is *in* them is this program's one pair at four salts, which
/// is what a slot nobody has loaded anything into holds ([`Engine::new`]).
/// [`ON_AIR`] is Live and is the whole of the picture. Every other slot rests
/// at `Residency::Allocated` — contributing nothing to the mix, and stepped and
/// drawn into its own cell all the same — and [`ASKED_TO_PRIME`] is
/// additionally asked to warm up and parked by the budget in
/// [`Engine::ask_to_prime`], which is what puts a pending request on this panel
/// for the mixer's tally to draw. The three cost a step and a draw each, and
/// none of it reaches the governor, which reads a per-Set cost. This sentence
/// has been wrong twice in the same direction — it said the three cost nothing
/// while they were being drawn, and *a draw each and no step* while they were
/// being stepped — so what it is now is the whole of a frame for every slot,
/// which is what
/// [ADR-0269](../../../docs/adr/0269-a-slot-that-is-drawn-is-stepped-and-a-preview-runs-at-the-rooms-tempo.md)
/// makes it. The number that goes with it is not one this file can carry: it is
/// `karakuri-engine`'s `tests/deck.rs`, which prints a deck of four against a
/// deck of one on the machine reading it.
///
/// All four preview cells are on, whatever the decks are doing. A cell is drawn
/// because there is a slot behind it ([`Engine::aim`]), and this deck is full,
/// so four cells show four slots' own material, all four of them running: deck
/// A stepping on air, deck B warming or parked, C and D warming with nobody
/// having asked. That is
/// [ADR-0258](../../../docs/adr/0258-the-look-comes-before-the-fader-so-a-cell-draws-every-slot-and-says-which-nothing-it-is.md)
/// met on this surface — the operator watches a candidate's cell to decide
/// whether it is worth a fader, and then raises the fader. It used to be gated
/// on `Residency::Live`, which left the three cells worth looking at dark; the
/// gap ADR-0241 named was this line.
pub(crate) struct Engine {
    pub(crate) deck: Deck,
    /// Elements per geometry, read off the L1's own `capacity` declaration rather
    /// than named here — see [`Engine::new`]. Kept because the reading
    /// [`Costs::say`] prints names it, and a workload figure that is not the one
    /// the run used is worse than none.
    pub(crate) capacity: u32,
    pub(crate) present: Present,
    /// The Program bay's picture.
    pub(crate) picture: Presented,
    /// The four deck preview cells, one per deck slot.
    pub(crate) previews: [Presented; DECKS],
    /// Cached bind groups for each slot view into the tone-mapping pipeline.
    pub(crate) slot_bind_groups: [Option<wgpu::BindGroup>; DECKS],
    /// The look every sink is drawn under this frame, and the one piece of engine
    /// state this program *moves*.
    ///
    /// It was [`LOOK`] handed straight to `compose` every frame, with the reason
    /// written at that constant: this file had no session, no `look` record and no
    /// key that changed it. It has a control now — the transport row's tone map
    /// capsule and its exposure track — so a press becomes `Operation::SetTonemap`
    /// or `SetExposure`, which become one `Record::Look`, which [`apply`] writes
    /// here; the next frame hands this to `compose` and the present pass uploads
    /// it. That is P-0090 on this value exactly: the control ends in the record
    /// every other surface's does, and nothing calls `Present::set_tonemap` behind
    /// its back.
    ///
    /// It lives here rather than beside the panel because it is what the *engine*
    /// is drawing under: `view::Look` is the console's reading of it, written per
    /// frame from this the way a strip is written from the deck, and a second copy
    /// that the console owned would be the reading and the state as one thing
    /// (ADR-0156).
    ///
    /// `white_point` is carried and never asked for: no surface has a control for
    /// it, so it is read back into every record and written out again unchanged
    /// ([ADR-0192](../../../docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)).
    pub(crate) look: Look,
    /// What the master chain is set to, and [`Engine::look`]'s twin at the other
    /// end of that chain.
    ///
    /// Held here for `look`'s reason exactly: a press becomes
    /// `Operation::SetFeedback`, `SetBloom` or `SetRgbShift`, which become one
    /// `Record::MasterChain`, which [`apply`] writes here; the frame loop puts it
    /// on the `Present` and the slots' uniforms are written from it. Nothing calls
    /// that setter behind the record's back, which is P-0090 on this value.
    ///
    /// A list where the out is a bare `f32` on the deck, and the two are apart for
    /// the reason their records are: the level at the chain's entry is ridden by a
    /// fader and the chain's slots are moved by a press
    /// ([ADR-0317](../../../docs/adr/0317-the-master-chain-is-three-fixed-passes-and-feedback-reads-either-cut.md),
    /// [ADR-0340](../../../docs/adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md)).
    ///
    /// A description and not the built chain, which is the one thing that changed
    /// when the chain became a list: `karakuri_engine::Present` holds the compiled
    /// slots and is still the only writer of them, and this is what a record says
    /// the chain should be. The frame loop puts one on the other where the two
    /// differ.
    pub(crate) chain: Vec<karakuri_engine::SlotSpec>,
    /// How many registrations have been freed, over both textures. The atlas leak
    /// this exists to prevent is invisible from outside: a resize that registers
    /// without freeing leaves a bind group per drag frame and nothing says so, so
    /// the count is kept and `mod gpu` asserts on it. It is the whole engine's
    /// tally rather than either texture's, which is why it lives here and is handed
    /// to [`Presented::fit`].
    pub(crate) freed: usize,
    /// One [`Aiming`] per slot, in slot order: how a load or a rewiring reaches
    /// that slot's build worker, and where that watcher is pointed.
    ///
    /// This is the whole of what putting a library Set on a running deck took, and
    /// what it is *not* is the point of it. `Deck::install` is the one function
    /// that puts a built Set in a slot and says of itself that it is *"deliberately
    /// not reachable from a key or a surface: a live run changes its material by
    /// editing a file and letting the worker build it, which is what the budget
    /// watchdog is attached to."* So nothing here builds a Set: [`loading`] writes
    /// the library Set's procedures into the scratch and sends an aim, and the same
    /// worker that watches for a save picks it up. The swap lands at a frame
    /// boundary, is judged there on what one frame of that Set costs, and rolls
    /// back on its own if that is over the budget — none of which had to be written
    /// for the library, because a load is now literally an edit this program made.
    ///
    /// In slot order, so the index is the deck letter: `aimed[0]` is deck A's, and
    /// it is the same index `Deck::events`, the strips and the preview cells are
    /// all in. Kept beside the deck rather than inside it for the reason the whole
    /// of [`Engine`] is on this side: the channel is `karakuri-environment`'s and
    /// the engine takes no environment.
    pub(crate) aimed: Vec<Aiming>,
    /// The run's wiring — every edge a `wire_input` has written, for the whole run
    /// and not per slot.
    ///
    /// One list because `--edge` is one list: an edge names the node that declares
    /// the input and what its procedure calls it, and a Set that has not got that
    /// node passes it over where it is built. See [`rewired`].
    ///
    /// What a rebuild carries and what a save records, which is why it lives here
    /// rather than inside a watcher: [`Aiming::re_aim`] restates it to the worker
    /// and [`playing_values`] writes it into the file, and those are one list or
    /// they are two answers to what the run is wired with.
    ///
    /// It lives beside the aims rather than beside the saves, and that is what lets
    /// a press reach it: a rewiring writes this list and re-aims a slot, and both
    /// halves are here. It was `Keeping`'s until 2026-09-09, when the Inspector's
    /// `uses` line gave the list a second writer that is not a model's request —
    /// see `docs/adr/0329-…`.
    pub(crate) edges: Vec<karakuri_engine::set::Edge>,
    /// Where this deck says which files its slots are running, for the readers that
    /// are not on this thread — see [`Aiming::pointing`], which is a clone of this,
    /// and [`karakuri_mcp::Slots`].
    ///
    /// The engine keeps it so that the two readers ask one handle. The MCP server
    /// was handed the launch working copies and the landing on a row of the edit
    /// history built a second `Slots` of its own out of the aims (ADR-0308); the
    /// first went stale on the first library load and the second was the workaround
    /// for it. There is one now, this is it, and [`restored`] reads it rather than
    /// rebuilding one.
    pub(crate) pointing: karakuri_mcp::Slots,
    /// Every node the run launched with, in file order, with the bytes each one was
    /// compiled from — see [`karakuri_environment::compile::Placed`].
    ///
    /// One list for four slots, because the four files hold the same bytes.
    /// [`working_copies`] writes the one pair the command line settled into every
    /// slot, so a node's layer, its index, its address and its source are the same
    /// answer four times; the only per-slot difference is the *path*, which each
    /// watcher is given from `slots[slot]` and which no part of a saved node
    /// carries. A second compile per slot would be four answers to one question
    /// with a window between them — see [`Placed::source`], which is where that
    /// hazard is written.
    ///
    /// This is what [`Playing`] is seeded from, and it is the reason a deck can be
    /// saved on the first frame rather than only after something has been rebuilt.
    pub(crate) placed: Vec<karakuri_environment::compile::Placed>,
}

/// One slot's watcher, and where it is pointed.
///
/// `karakuri-cli`'s `Aiming` is the same pair for the same reason, restated
/// here because that program is a binary with no library target and there is
/// nothing to call.
///
/// The aim is kept and not only the sender, because a [`watch::Aim`] is every
/// field of the slot's identity and *anything left out comes back as the
/// outgoing slot's* — a fold silently un-selected, a camera back at
/// `Orbit::default()`, salts that repaint every element. A rewiring changes one
/// field of an aim, so all the others have to be restated from somewhere, and
/// this is that somewhere: what the watcher was constructed with until the
/// first aim, and the last aim after that.
///
/// It is also where the Set a slot is running lives, which is the roadmap's
/// *per-slot `Option<String>` beside the deck* answered where a per-slot value
/// that must survive a re-aim already lives: [`watch::Aim::set`] is moved by
/// every load and restated by every rewiring, and a second copy on [`Gfx`]
/// would be a second answer to *what is this slot running*
/// (`docs/principles/0087-name-the-property-never-the-shape.md`).
/// [`Gfx::material`] is not that answer and never was — it is the mixer strip's
/// readout, and at launch it is the pair the run was started with.
///
/// This program had the sender and not the aim, which was harmless for as long
/// as the only thing that sent one was [`loading`] — a load states every field
/// off the Set file it read. It stops being harmless the moment anything
/// changes *one* field, which is what `wire_input` does: a rewiring that
/// restated the launch pair would have thrown away the Set the operator had
/// just loaded.
pub(crate) struct Aiming {
    /// The other end of [`watch::Watch::aimed_by`]'s channel, for this slot's
    /// watcher and no other. A watcher re-pointed through somebody else's sender
    /// would rebuild a deck nobody named.
    pub(crate) aim: std::sync::mpsc::Sender<watch::Aim>,
    /// Where that watcher is pointed, kept in step with what has been sent.
    pub(crate) at: watch::Aim,
    /// Where that answer is published for the readers that are not on this thread,
    /// which today are the MCP server and the landing on a row of the edit history
    /// — see [`karakuri_mcp::Slots`].
    ///
    /// A publication and not a second answer. `at` above is the derivation
    /// ([`Aiming`]'s own head); this is a clone of the run's one handle, and
    /// nothing writes it except [`Aiming::publish`], which reads `at`. The server
    /// used to be handed the launch working copies instead, and after a library
    /// load it resolved every address against the files the deck had stopped
    /// running — a `read_procedure` that answered about the wrong material, a
    /// `write_procedure` that wrote where no watcher was looking, and a node the
    /// loaded Set does hold refused for not existing (`docs/principles/0094-…`, and
    /// ADR-0308's *Doubted*, which recorded it and worked around it for the landing
    /// alone).
    pub(crate) pointing: karakuri_mcp::Slots,
    /// Which slot this is, so a publication lands on the row it is about. It is the
    /// index [`Engine::aimed`] is in, which is the deck letter.
    pub(crate) slot: usize,
}

impl Aiming {
    /// A watcher, and the handle where this slot's files are published.
    ///
    /// It publishes at construction as well as on every re-point, because a window
    /// remade makes these again and puts every slot back on the pair the run
    /// launched with (ADR-0304): a handle left holding the layout a load had put
    /// there would outlive the deck that was running it.
    pub(crate) fn new(
        aim: std::sync::mpsc::Sender<watch::Aim>,
        at: watch::Aim,
        pointing: karakuri_mcp::Slots,
        slot: usize,
    ) -> Aiming {
        let aiming = Aiming {
            aim,
            at,
            pointing,
            slot,
        };
        aiming.publish();
        aiming
    }

    /// Say where this watcher is pointed, from the aim and from nothing else. One
    /// write, after the caller has finished writing files and before the aim goes
    /// out, so a call arriving mid-load sees one layout or the other and never half
    /// of either.
    pub(crate) fn publish(&self) {
        self.pointing.re_point(self.slot, &self.at);
    }

    /// Point the watcher at what it is already looking at, with one field changed,
    /// and answer whether it is still there to be pointed.
    ///
    /// # This is the whole of what a control over a slot's shape is
    ///
    /// A [`watch::Aim`] is every field of what a slot *is* — its files, how it
    /// layers its renderers, which one is folded to, its capacity, its seed and its
    /// salts, its camera, its wiring, its grants and the Set it is filed under.
    /// Anything that changes one of them changes what the slot runs, and there is
    /// exactly one way to say so: restate the rest and send the aim. The worker
    /// rebuilds off the render thread, the build lands at a frame boundary and the
    /// watchdog judges it there, on what one frame of that Set was measured to
    /// cost, like every other build (ADR-0228, ADR-0313, ADR-0314). Nothing is
    /// installed and nothing on the render thread allocates, and the values
    /// somebody moved cross the swap (ADR-0282).
    ///
    /// So a *setter on the engine* is not what a control over one of these fields
    /// waits on, and reading `Set::merge`'s or `Set::source_salts`' absent writer
    /// as a blocker is reading the wrong half of the sentence: the mechanism that
    /// exists is the one this instrument already changes material with
    /// ([P-0085](../../../docs/principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md)).
    ///
    /// `change` takes the aim rather than the caller building one, because
    /// [`restated`] is what makes a re-aim safe and it reads `self.at`: a caller
    /// that assembled its own would be the fourteen-field restatement written a
    /// second time, which is exactly the mistake `Watch::repointed` destructures
    /// with no `..` to stop.
    ///
    /// `Err` is a build worker that has ended — the receiver is gone — which is a
    /// run shutting down. It is reported rather than swallowed: the change is in
    /// this program's aim either way, and *nothing will rebuild* is a different
    /// fact from *the slot is recompiling*.
    pub(crate) fn changed(&mut self, change: impl FnOnce(&mut watch::Aim)) -> Result<(), ()> {
        change(&mut self.at);
        // **Said again although a rewiring moves no file**, which is the point
        // of putting it here rather than at the one call that does: this is one
        // of the two places an aim leaves this program, and a publication that
        // covered only the other would be a rule somebody has to remember.
        self.publish();
        self.aim.send(restated(&self.at)).map_err(|_| ())
    }

    /// The run's wiring, said again, which is [`changed`](Self::changed) with the
    /// one field a `wire_procedure` moves.
    ///
    /// A method rather than the closure at the call site because `rewired` maps
    /// over slots and a named field is what the reader of that map wants to see.
    pub(crate) fn re_aim(&mut self, edges: Vec<karakuri_engine::set::Edge>) -> Result<(), ()> {
        self.changed(|at| at.edges = edges)
    }

    /// Point it at something else entirely, keeping the aim that was sent.
    ///
    /// The one route a load takes, and the reason [`loading`] is handed this rather
    /// than the sender: a load that sent an aim and left `at` behind would leave
    /// the *next* rewiring restating the material the run launched with, which is
    /// the hardest version of this mistake to see.
    pub(crate) fn re_point(&mut self, aim: watch::Aim) -> Result<(), ()> {
        self.at = aim;
        self.publish();
        self.aim.send(restated(&self.at)).map_err(|_| ())
    }
}

/// One aim, said again — because [`watch::Aim`] is not `Clone` and a re-point
/// restates every field of it.
///
/// No `..` on either side of this, which is `Watch::repointed`'s own rule met
/// from the sending end: it destructures with no `..` so that a field added to
/// `Aim` cannot be left behind, and a *sender* that filled the new field with a
/// default would defeat that from here. The compiler names every one of them,
/// so the day another arrives this stops compiling rather than quietly
/// re-aiming a slot at it.
pub(crate) fn restated(aim: &watch::Aim) -> watch::Aim {
    let watch::Aim {
        head,
        rest,
        layering,
        live,
        capacity,
        seed_salt,
        salts,
        camera,
        overrides,
        published,
        bindings,
        edges,
        authorities,
        set,
    } = aim;
    watch::Aim {
        head: head.clone(),
        rest: rest.clone(),
        layering: *layering,
        live: *live,
        capacity: *capacity,
        seed_salt: *seed_salt,
        salts: salts.clone(),
        camera: *camera,
        overrides: overrides.clone(),
        published: published.clone(),
        bindings: bindings.clone(),
        edges: edges.clone(),
        authorities: authorities.clone(),
        // **What this slot is running, carried through a rewiring untouched.**
        // A rewiring changes an edge and not the material, so the versions
        // written after it belong to the same Set as the ones before it —
        // dropped here, a slot would go back to filing under no Set at all on
        // the first `wire_input` after a load.
        set: set.clone(),
    }
}

// ---------------------------------------------------------------------------
// Keeping what a deck is playing
// ---------------------------------------------------------------------------

/// What each deck is running, as the nodes a Set file names.
///
/// `karakuri-cli`'s `Running` is the same fact held the same way, and this is
/// the second surface rather than a copy with a different opinion — that
/// program is a binary with no library target, so there is nothing to call.
/// What differs is the shape and only the shape: that one holds addresses and
/// zips the launch bytes back on at save time, and this one holds the
/// [`setfile::SavedNode`] whole, because the panel has one launch list for four
/// slots and nothing to zip it against.
///
/// It is seeded before the first frame, from [`Engine::placed`] — so every deck
/// can be written down from the outset rather than only after something has
/// been rebuilt. A slot that is `None` is one whose last build's sources did
/// not reach the store, which the watcher said at the time; it saves nothing
/// rather than guessing.
///
/// A type of its own rather than a field on [`App`], because what a slot is
/// running is one fact with one transition: a build lands and it moves. It held
/// two lists until ADR-0316 — what is playing and what a rollback would bring
/// back — because a rollback was the only thing that could name what it
/// restored. Nothing restores anything now, so the second list was a version
/// kept against an event that cannot happen; putting a version back is
/// `Revision::Previous`, which reads the store's history and lands a build like
/// any other, and this then records it like any other.
pub(crate) struct Playing {
    pub(crate) playing: Vec<Option<Vec<setfile::SavedNode>>>,
}

impl Playing {
    /// Every slot seeded from the material this run compiled, addressed by the
    /// bytes that compile read.
    ///
    /// No store, no disk and nothing that can fail. The bytes ride along in
    /// [`setfile::SavedNode::source`] and reach the store at the moment a file
    /// names them, which is [`setfile::Sources::into_nodes`] — so a run that never
    /// saves writes no artifact.
    pub(crate) fn at_launch(
        placed: &[karakuri_environment::compile::Placed],
        slots: usize,
    ) -> Playing {
        let nodes: Vec<setfile::SavedNode> = placed
            .iter()
            .map(|node| setfile::SavedNode {
                layer: setfile::kind_name(node.layer),
                index: node.index,
                hash: node.hash(),
                name: node.named.name.clone(),
                source: Some(std::sync::Arc::clone(&node.source)),
                meta: Some(std::sync::Arc::clone(&node.meta)),
            })
            .collect();
        Playing {
            playing: (0..slots)
                .map(|_| match nodes.is_empty() {
                    true => None,
                    false => Some(nodes.iter().map(copied).collect()),
                })
                .collect(),
        }
    }

    /// What `slot` is running, or `None` for a slot with no address to name.
    pub(crate) fn at(&self, slot: usize) -> Option<&Vec<setfile::SavedNode>> {
        self.playing.get(slot).and_then(Option::as_ref)
    }

    /// A build landed. `nodes` is `None` when that build's sources never reached
    /// the store — the watcher says so at the time, and the addresses it would have
    /// named do not exist.
    ///
    /// That is still a swap, and taking it as one is the whole of why this is a
    /// method rather than an assignment at the call site: the slot is on something
    /// new, and it has no address until the next build lands.
    ///
    /// It is still a swap when the slot is stopped for cost, too. A version the
    /// watchdog stopped is in the slot and is what a save of that slot must write
    /// down (ADR-0316); what is not true of it is that the slot is running, which
    /// is the lane's to say and not this list's.
    pub(crate) fn landed(&mut self, slot: usize, nodes: Option<Vec<setfile::SavedNode>>) {
        if slot >= self.playing.len() {
            return;
        }
        self.playing[slot] = nodes;
    }
}

/// One saved node, again — because [`setfile::SavedNode`] is not `Clone` and a
/// save consumes the list it is handed while the run goes on holding it.
///
/// The bytes and the card are `Arc`s and are shared rather than duplicated,
/// which is what those two fields are `Arc`s for.
pub(crate) fn copied(node: &setfile::SavedNode) -> setfile::SavedNode {
    setfile::SavedNode {
        layer: node.layer,
        index: node.index,
        hash: node.hash,
        name: node.name.clone(),
        source: node.source.clone(),
        meta: node.meta.clone(),
    }
}

/// What a build that just landed is running, from the addresses the watcher
/// reported and the names the slot is spelled with.
///
/// The bytes are `None` for every one of them, and that is
/// [`setfile::SavedNode::source`]'s own rule rather than an omission: the
/// watcher put this build's sources in the store as it built them, so there is
/// nothing left here to carry.
///
/// The names come off the aim the slot is pointed at, zipped by position.
/// `watch::Built::nodes` is built from the sort's `Placed`, which keeps file
/// order, and [`Aiming::at`]'s head and rest are that same file list — so entry
/// `n` of one is entry `n` of the other. A name belongs to the *use* rather
/// than to the procedure, so nothing a hash carries could hold it, and the
/// alternative is a Set loaded with named nodes coming back after its first
/// rebuild with the `edge` records pointing at nothing.
pub(crate) fn built_nodes(built: &watch::Built, at: &watch::Aim) -> Vec<setfile::SavedNode> {
    let names: Vec<Option<String>> = std::iter::once(at.head.name.clone())
        .chain(at.rest.iter().map(|node| node.name.clone()))
        .collect();
    built
        .nodes
        .iter()
        .enumerate()
        .map(|(node, (layer, index, hash))| setfile::SavedNode {
            layer,
            index: *index,
            hash: *hash,
            name: names.get(node).cloned().flatten(),
            source: None,
            meta: None,
        })
        .collect()
}

/// What a Set file says about the Set that is playing, read off that Set.
///
/// `karakuri-cli`'s `playing_values` is this function and its doc is the
/// argument for every line: eight of the nine are read from the Set and not
/// from anything this program was told, because a writer with its own copy of
/// the rule records numbers the run was not using and the file then describes a
/// picture nobody has seen. The capacities are the Set's per geometry, the
/// params are every declaration of every node at the value it is holding, the
/// layering and the fold are the Set's rather than a flag's, and the salts are
/// what it *is* salted with rather than what a position would derive.
///
/// The ninth is `edges`, which is the run's — see [`App::edges`]. It is not the
/// Set's for the reason `mcp::WireRequest` states: the wiring a slot rebuilds
/// with is not on disk anywhere, and the run is the only thing that holds it.
///
/// Restated here rather than called, for [`number_for`]'s reason.
pub(crate) fn playing_values(
    set: &karakuri_engine::Set,
    edges: &[karakuri_engine::set::Edge],
) -> setfile::Owned {
    setfile::Owned {
        // Filled where a store is open, and nowhere else — see [`Save::run`].
        nodes: Vec::new(),
        capacities: set.source_capacities(),
        params: set
            .params()
            .map(|(layer, index, key, value)| {
                karakuri_engine::ParamWrite::at(layer, index, key, value)
            })
            .collect(),
        bindings: set.bindings().to_vec(),
        edges: edges.to_vec(),
        // **`Set::orbit` and not the `Set::camera` field**: three of the six
        // are the camera node's parameters, so the field is what was last
        // stated and the map is what a hand, a binding or a carried ride left
        // there (ADR-0318).
        camera: set.orbit(),
        layering: set.layering(),
        live: selected_renderer(set.inputs()),
        seeds: set.source_salts().to_vec(),
    }
}

/// Which renderer a Set is folded to, as a `merge` record spells it: `Some(i)`
/// where exactly one input is live, and `None` where every one of them is.
///
/// Every-live is checked first, and that decides the one-renderer case. A
/// composited Set holding a single renderer has one live input, which is both
/// "all of them" and "exactly one" — and it is the first, because such a Set is
/// one nobody has selected in. Writing `live 0` for it would record a choice
/// that was never made.
pub(crate) fn selected_renderer(inputs: &[karakuri_engine::mix::Input]) -> Option<u32> {
    if inputs.iter().all(|input| input.live) {
        return None;
    }
    let mut live = inputs.iter().enumerate().filter(|(_, input)| input.live);
    match (live.next(), live.next()) {
        (Some((at, _)), None) => Some(at as u32),
        _ => None,
    }
}

/// Whether this deck holds `slot`. The companion of
/// [`karakuri_environment::no_such_slot`], which is the sentence it is refused
/// in.
///
/// A raw `usize` in and a raw `usize` checked, on [`held`]'s own terms
/// elsewhere in this file: the callers here (`Keeping::save_set`'s and
/// `Keeping::keep_procedure`'s slot arguments, an MCP request's own number)
/// have no address yet to hand a [`DeckSlot`] — only a number to validate
/// before one can be made. It checks through [`DeckSlot::new`] rather than
/// `slot < slot_count` again, which is the same question [`held`] and
/// `karakuri-cli`'s own `slot_in_range` answer.
pub(crate) fn slot_in_range(slot: usize, slot_count: usize) -> bool {
    u8::try_from(slot)
        .ok()
        .is_some_and(|slot| DeckSlot::new(slot, slot_count).is_some())
}

/// One live save, from the frame that asked for it to the file on disk.
pub(crate) struct Save {
    pub(crate) slot: usize,
    /// Whose act this save is, which decides the directory it lands in and is
    /// decided at the call site — see [`Keeping::save_set`] and
    /// [`karakuri_environment::Asked`].
    pub(crate) asked: Asked,
    pub(crate) id: String,
    /// The store root, not an open store: opening it creates directories, which is
    /// I/O, which belongs on the thread below rather than on a frame.
    pub(crate) root: std::path::PathBuf,
    pub(crate) sources: setfile::Sources,
    /// What the file will say, with `nodes` still empty. The nodes are the one part
    /// of a Set file that needs a store — a hash per source — so they are filled in
    /// where one is opened and never here.
    pub(crate) values: setfile::Owned,
}

impl Save {
    /// Write it. Everything here is off the render thread: opening a store creates
    /// directories, and the Set file itself is written and renamed into place.
    pub(crate) fn run(self) -> Result<(), String> {
        let Save {
            asked,
            id,
            root,
            sources,
            mut values,
            ..
        } = self;
        let store = Store::open(&root).map_err(|e| format!("store `{}`: {e}", root.display()))?;
        // **Before the file that references them**, which is what
        // `setfile::Node` carrying a hash asks of every caller: the writer
        // cannot check that a hash resolves without reading the store back, so
        // putting them is the caller's promise.
        values.nodes = sources.into_nodes(&store)?;
        setfile::save(&store, asked, &id, values.saving())
    }
}

/// What a live save came back with, at the frame it arrives.
pub(crate) struct Saved {
    pub(crate) slot: usize,
    /// Carried through so the sentence at the end names the right place: the
    /// library's line points at the Library bay, and the sandbox's cannot, because
    /// the bay does not list one.
    pub(crate) asked: Asked,
    pub(crate) id: String,
    /// `Ok` and the file is on disk under `id`. A failure is printed and nothing
    /// claims otherwise: a program saying a save happened when the disk refused is
    /// the shape of lie this codebase is arranged against.
    pub(crate) outcome: Result<(), String>,
    /// Where a client that asked for this save is waiting, and `None` when a hand
    /// pressed `k`.
    ///
    /// It rides the save rather than being looked up when the outcome lands. A map
    /// from an id to whoever asked would be a second place that knows which save is
    /// which, and the outcome already carries everything needed to find its way
    /// home.
    pub(crate) reply: Option<mcp::Reply>,
}

/// One node's procedure kept, from the frame that asked for it to the file on
/// disk and back.
///
/// [`Save`] and [`Saved`] folded into one type, and that is the difference in
/// the act rather than a shortcut: a Set save gathers a whole slot's worth of
/// readings off the live deck, so what is *asked for* and what *came back* are
/// two shapes; a keep is one node's bytes and a name, so the request and the
/// outcome carry the same three fields and the outcome is what is added.
///
/// Nothing here is a reading of the engine. The bytes are the run's own — what
/// [`Playing`] holds for that slot — and where they go is decided by [`Asked`],
/// which is the call site's. So the whole of this crosses onto the write thread
/// with no deck behind it.
pub(crate) struct Kept {
    /// Whose act this keep is, which decides the directory it lands in —
    /// [`Save::asked`]'s field and its argument: an operator's own act writes
    /// `<store>/procedures/` and a model's writes `<store>/sandbox/` (P-0096,
    /// ADR-0261).
    pub(crate) asked: Asked,
    /// What it is filed as — the name typed into this pane's head where one was,
    /// and a stamp where the capsule typed nothing (ADR-0128, ADR-0287, ADR-0292).
    pub(crate) name: String,
    /// The store root, not an open store — [`Save::root`]'s reason: opening it is
    /// I/O and belongs on the thread below.
    pub(crate) root: std::path::PathBuf,
    /// The bytes, where this node is still on the version the run launched with,
    /// and `None` where a build put it in the store instead — which is
    /// [`setfile::SavedNode::source`]'s own rule. Either way the address below
    /// names it, so the write thread has one place to go for what it has not got.
    pub(crate) source: Option<std::sync::Arc<str>>,
    /// The address of those bytes, which is what a rebuilt node carries instead of
    /// them: the watcher put its source in the store as it built it, so
    /// `<hash>.kir` at the store root is where a keep reads it back from.
    pub(crate) hash: karakuri_store::hash::Hash,
    /// What the pane calls this node — `L2:0` — carried for the sentence and
    /// nothing else. An address is what an operator is looking at when they press,
    /// and an outcome naming a file with no node beside it is an answer to a
    /// question nobody asked.
    pub(crate) addr: String,
    /// Where the file went, once it has gone there — `Ok` and it is on disk, `Err`
    /// and nothing claims otherwise, which is [`Saved::outcome`]'s rule.
    pub(crate) outcome: Result<std::path::PathBuf, String>,
    /// Where a client that asked for this keep is waiting, and `None` when a hand
    /// pressed the capsule — [`Saved::reply`]'s field and its reason.
    pub(crate) reply: Option<mcp::Reply>,
}

impl Kept {
    /// Write it. Everything here is off the render thread: opening a store creates
    /// directories, an artifact may have to be read back, and the file itself is
    /// written and renamed into place.
    ///
    /// The bytes are found in one of two places and never a third. A node still on
    /// its launch version carries them; a node a build landed has them in the store
    /// under the address it carries instead. Neither is a re-read of the `.kir` on
    /// disk, which is [`setfile::SavedNode::source`]'s own sentence: a file
    /// rewritten since the compile is a version nobody has seen, and a keep of it
    /// would put a picture nobody watched into a library.
    pub(crate) fn run(self) -> Kept {
        let Kept {
            asked,
            name,
            root,
            source,
            hash,
            addr,
            reply,
            ..
        } = self;
        let outcome = (|| {
            let store =
                Store::open(&root).map_err(|e| format!("store `{}`: {e}", root.display()))?;
            let bytes = match &source {
                Some(source) => source.as_bytes().to_vec(),
                None => store
                    .get_artifact(&hash)
                    .map_err(|e| format!("the source of `{addr}`: {e}"))?,
            };
            match asked {
                Asked::Operator => store.write_procedure(&name, &bytes),
                Asked::Model => store.write_sandbox_procedure(&name, &bytes),
            }
            .map_err(|e| e.to_string())
        })();
        Kept {
            asked,
            name,
            root,
            source,
            hash,
            addr,
            outcome,
            reply,
        }
    }

    /// The one sentence this outcome is said in, formed here so that the words a
    /// test reads, the words an operator reads and the words a model is handed are
    /// the same run of text — [`Sent::said`]'s rule and [`Keeping::took_save`]'s.
    pub(crate) fn said(&self) -> Result<String, String> {
        match &self.outcome {
            Ok(path) => Ok(match self.asked {
                Asked::Operator => format!(
                    "  keep: {} kept as procedure `{}` — `{}`. The Library bay lists it, and a \
                     press on that row writes it over that layer of what a deck is playing",
                    self.addr,
                    self.name,
                    path.display()
                ),
                // **The sandbox's sentence says where it is and what does not
                // read it**, which is a Set save's own division one file kind
                // along: a model told only that a procedure was kept would go
                // looking for a library row that is not there (P-0083,
                // ADR-0261).
                Asked::Model => format!(
                    "  keep: {} kept as `{}` in the sandbox — `{}`. A keep asked for over MCP is \
                     written there rather than in the operator's library, so the Library bay does \
                     not list it and no load off a row reaches it",
                    self.addr,
                    self.name,
                    path.display()
                ),
            }),
            Err(e) => Err(format!(
                "  keep: {}: procedure `{}` was not kept: {e}",
                self.addr, self.name
            )),
        }
    }
}

/// What a send came back with, at the frame it arrives.
///
/// [`Saved`]'s shape one act along, and the fields differ where the two acts
/// do: a send files under no id in this store, so there is no `Asked` to carry
/// — the answer to *whose library is this* is *nobody's*, which is the whole of
/// what sending is — and there is no `mcp::Reply`, because no tool asks for
/// one.
pub(crate) struct Sent {
    /// The Set that was packaged, which is the row the menu was opened on.
    pub(crate) id: String,
    /// Where the operator sent it, or `None` where they dismissed the dialog
    /// without naming anywhere.
    ///
    /// `None` is an outcome and not a failure, which is why it is here rather than
    /// an `Err` in [`outcome`](Self::outcome): nothing went wrong, nothing was
    /// written, and the sentence a reader needs is the third one rather than a
    /// refusal (P-0083 is about what a *rejection* carries, and this is not one).
    pub(crate) to: Option<std::path::PathBuf>,
    /// `Ok` and the bundle is on the disk at [`to`](Self::to). A failure is printed
    /// and nothing claims otherwise, which is [`Saved::outcome`]'s own rule.
    pub(crate) outcome: Result<(), String>,
}

impl Sent {
    /// The one sentence this outcome is said in, formed here so that the words a
    /// test reads and the words an operator reads are the same run of text —
    /// [`Keeping::took_save`]'s *one sentence for both audiences*, with one
    /// audience.
    ///
    /// Three outcomes and three sentences. Written; refused, naming what the disk
    /// or the store said; and *no file was named*, which is not a refusal and does
    /// not read like one — nothing went wrong, and rule 04 of the manual is that a
    /// press that did nothing says so rather than going quiet.
    pub(crate) fn said(&self) -> String {
        let Sent { id, to, outcome } = self;
        match (to, outcome) {
            (None, _) => format!(
                "  send: `{id}` was not written — the save dialog was dismissed, and nothing was \
                 asked of the disk"
            ),
            (Some(to), Ok(())) => format!("  send: `{id}` written to `{}`", to.display()),
            (Some(to), Err(e)) => {
                format!("  send: `{id}` was not written to `{}`: {e}", to.display())
            }
        }
    }
}

/// A save that will not happen, to the terminal and to whoever asked if that
/// was not a hand.
///
/// One sentence and one home. Every refusal here reaches two audiences, and the
/// way that goes wrong is a copy of the words for the second one — free to be
/// right on the day it is written and wrong at the next correction.
pub(crate) fn refused(reply: Option<mcp::Reply>, said: String) {
    println!("{said}");
    if let Some(reply) = reply {
        reply.settled(Err(said));
    }
}

impl Engine {
    /// Sized from the arrangement rather than from the window, by the same two
    /// calls the frame aims with — see [`aims`]. The window this opens at gives the
    /// picture and deck A's cell their first rectangles, so no frame has to correct
    /// a guess and there is no second derivation here to drift from the one in
    /// [`Engine::aim`].
    // Eight, for [`watched`]'s reason: the last three are the run-wide handles
    // this constructor hands every watcher it makes, and each has a different
    // owner in [`main`].
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        slots: &[Sources],
        layout: &karakuri_layout::Layout,
        scale: f32,
        // **Where every watcher puts what it builds, and where it reports it.**
        // `None` is a harness with no store to write into, which is every test
        // under `mod gpu` below: nothing there saves, and a run that created a
        // store to draw four cells would be the side effect
        // `karakuri_environment::scratch` refuses for a `--render`.
        stored: Option<(std::sync::Arc<Store>, std::sync::mpsc::Sender<watch::Built>)>,
        // **The run's one edit history**, made and seeded in [`main`] and
        // handed to every watcher this makes — see [`watched`]. `None` is a
        // harness with no store, which is every test under `mod gpu` below, on
        // `stored`'s terms exactly.
        //
        // **One `Shared` for the run and not one per window.** A window remade
        // makes these watchers again, and a second `Snapshots` would have an
        // empty dedup memory: the first rebuild after a remake would file every
        // untouched procedure as a new version.
        snapshots: Option<history::Shared>,
        // **Where this deck says which files each of its slots is running**,
        // handed in rather than made here: [`main`] gives the same handle to
        // [`karakuri_mcp::serve`] and to this, so a model's
        // address and the file a watcher is polling are one answer — which is
        // exactly the arrangement the opening already has. It is not
        // `Option` and does not depend on `--mcp`, because the landing on a row
        // of the edit history reads it too ([`restored`]).
        pointing: karakuri_mcp::Slots,
    ) -> Engine {
        assert!(
            slots.len() == SLOTS,
            "a deck of {SLOTS} slots was handed {} pairs to run from",
            slots.len()
        );
        // **Parsed once and built [`SLOTS`] times, and that is a fact rather
        // than an assumption now.** Every entry in `slots` is a copy of the
        // one pair the command line settled ([`working_copies`]), so the four
        // files hold the same bytes at startup and one `Checked` is the same
        // answer four times. What is *not* the same is the path each slot's
        // watcher polls, which is the whole of what per-slot copies buy and is
        // read off `slots[slot]` in the loop below.
        // **Compiled through the sort every other surface compiles through**,
        // which is what this used to do by hand and is the whole of what a save
        // needed: `checked` gave back a `Checked` and dropped the bytes it read,
        // and a node's address is a function of exactly those bytes
        // ([`karakuri_environment::compile::Placed::source`]). Re-reading the
        // path later to hash it is the defect that function's own doc is
        // written against — between here and the first frame sit a device, four
        // `Set::build`s and, now, an MCP server.
        //
        // **Once, for slot 0, and used by all four.** See [`Engine::placed`].
        let (material, placed) = karakuri_environment::compile::sort_slot(
            ON_AIR,
            &karakuri_environment::compile::Named::bare(&slots[ON_AIR].l1),
            &[karakuri_environment::compile::Named::bare(
                &slots[ON_AIR].l4,
            )],
        );
        let l1 = material
            .l1s
            .first()
            .expect("the launch pair declares a geometry")
            .clone();
        let l4 = material
            .l4s
            .first()
            .expect("the launch pair declares a renderer")
            .clone();
        let capacity = capacity_of(&l1);
        // **The same material in every slot, at its own salt and in its own
        // file.** A slot cannot hold *nothing*: `HotSwap::new` takes a live
        // `Set` and `Deck::new` takes one `HotSwap` per slot, so an empty slot
        // is not a state this engine has and the nearest thing to it is a slot
        // holding material nobody has asked for. What this program has to give
        // them is one pair — [`Sources`] is the whole command line — so each
        // gets it at its own salt ([`slot_salt`]): four slots of one procedure
        // at four seeds are four simulations, and four slots at one seed would
        // be one picture drawn four times, which is not a mixer either.
        // Building three of them from other `.kir` files would be this program
        // choosing material for the operator, which is the library's job and
        // not a constructor's
        // ([ADR-0228](../../../docs/adr/0228-a-library-load-re-points-the-slots-source-and-never-installs-a-set.md)).
        //
        // **The salt is no longer the only difference, and that is the fix.**
        // Each slot runs from its own copy of that pair ([`working_copies`]),
        // so the four are the same *material* and four different *files* — an
        // edit reaches the deck whose file it is.
        let built = |salt| {
            Set::build(&gpu.device, &gpu.queue, &l1, &l4, capacity, salt)
                .expect("the pair builds a Set")
        };
        // **A watcher per slot, over that slot's own two files**, which is
        // `karakuri-cli`'s wiring and not a second one — one `HotSwap::new`
        // over a `watch::Watch`, at the engine's own default budget.
        //
        // **`slots[slot]` and not one pair repeated**, which is the sentence
        // this comment used to be the other way round. It said every slot was
        // spelled with the same pair and quoted `watch`'s *"Two slots given
        // the same files both rebuild, which is right: the same edit reached
        // both of them"* — true of that module, and the wrong thing for this
        // program to be doing, because it made one save rebuild four slots and
        // fill the Staging lane with three rows that can never leave (a parked
        // slot's trial is frozen, so it reaches no verdict). Each watcher now
        // polls the copy made for its own slot, and no watcher can see
        // another's file at all, which is what `watch`'s *"a slot is the unit
        // that gets replaced"* asks for.
        //
        // **This is the Staging lane's producer**, and it is the whole of what
        // it took. `HotSwap::fixed` keeps a `Receiver` whose `Sender` was
        // dropped at construction, so nothing is ever installed and no
        // `swap::Event` of any variant is emitted — which is why the lane drew
        // its empty state and could reach no other. Nothing about the frame
        // path changed: `install_if_ready` polls the same channel with
        // `try_recv` either way, and everything a rebuild costs — the file
        // read, the four validation stages, the compile and `Set::build` — is
        // on the worker thread this spawns (P-0091).
        //
        // **In slot order, and the loop is the whole of what four slots
        // took**: a `Vec` of `HotSwap` is what `Deck::new` has always taken,
        // and `Engine::aimed` is documented as being in the same order the
        // strips and the preview cells are.
        let mut swaps = Vec::with_capacity(SLOTS);
        let mut aimed = Vec::with_capacity(SLOTS);
        for (slot, running) in slots.iter().enumerate().take(SLOTS) {
            let salt = slot_salt(slot);
            let (swap, aim) = watched(
                gpu,
                running,
                built(salt),
                slot,
                salt,
                stored
                    .as_ref()
                    .map(|(store, tx)| (std::sync::Arc::clone(store), tx.clone())),
                pointing.clone(),
                snapshots.clone(),
            );
            swaps.push(swap);
            aimed.push(aim);
        }
        let mut deck = Deck::new(&gpu.device, swaps, CANVAS.0, CANVAS.1);
        // **Every slot but deck A rests at `Allocated`, which is what a
        // channel nobody has asked anything of is.**
        //
        // `Deck::new` brings every slot up Live, and that is right for a deck
        // built to *play* what is in it — a deck of one is then a bare Set,
        // bit for bit (ADR-0038). This deck is built **full** rather than
        // built to play four, so leaving them Live would put three
        // simulations nobody asked for on the render thread and three layers
        // nobody asked for into the fold: the reading [`Costs::say`] prints
        // would stop being one Set a frame, and every slot comes up under
        // `Blend::Add` at unity, so what the Program bay drew would be four
        // simulations summed — the same material at four times its exposure,
        // which is a mixer set wrong rather than a mixer.
        //
        // `Residency::Allocated` is the state the engine already has for this,
        // rather than one invented here — *off air, asked of nothing* — and
        // `deck::Frame::render` reads the effective residency into the
        // composite's `live` flag, so an allocated slot contributes nothing to
        // the mix. It is drawn (ADR-0258) and it steps (ADR-0269), so what it
        // costs is its buffers, its L1 and its L4, and what it does not cost is
        // a term in the fold. The strip reads ALLOC and the cell reads material
        // running. That is
        // [P-0085](../../../docs/principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md):
        // *a slot with nothing in it* is a residency this deck already has a
        // word for.
        //
        // **It is the request that is written**, which is the operator's half
        // and the same half [`Engine::ask_to_prime`] writes — see
        // `Deck::set_residency`. So an operator brings a channel up by cycling
        // its tally chip or by loading material into it, and nothing has to
        // undo a decision this constructor made. The governor is told nothing
        // by it either: a slot whose request is Allocated is `Reason::OffAir`,
        // which is *the governor was not asked about this slot*.
        for slot in 0..SLOTS {
            if slot != ON_AIR {
                deck.set_residency(EngineSlot(slot as u8), Residency::Allocated);
            }
        }
        // **The meters are on, and that is a decision rather than a default.**
        // Five of the six things a mixer strip shows are settings the deck was
        // told; the meter is the only one that is a *measurement*, so with it
        // off this bay would draw five readouts that never move beside a well
        // that is always empty — which is the scaffolding-that-looks-finished
        // this panel refuses, read from the other side. It costs a pipeline,
        // two buffers and a ring of staging buffers per slot, allocated here
        // and never on the render thread, which is the same terms `Deck::new`
        // above is on; there are [`SLOTS`] slots, so it is four of each. The
        // three that are not Live report no level — `Deck::level` is `None`
        // for a slot that is not being drawn — which is the meter saying what
        // it measured rather than a strip with a gap in it.
        deck.enable_meters(&gpu.device);
        let present = Present::new(&gpu.device, PICTURE_FORMAT, CANVAS.0, CANVAS.1);
        let (picture_at, preview_ats) = aims(layout, present.size());
        let picture = Presented::new(gpu, renderer, "program view", picture_at, scale);
        let previews = [
            Presented::new(
                gpu,
                renderer,
                "deck A preview",
                preview_ats.and_then(|c| c.first().copied()),
                scale,
            ),
            Presented::new(
                gpu,
                renderer,
                "deck B preview",
                preview_ats.and_then(|c| c.get(1).copied()),
                scale,
            ),
            Presented::new(
                gpu,
                renderer,
                "deck C preview",
                preview_ats.and_then(|c| c.get(2).copied()),
                scale,
            ),
            Presented::new(
                gpu,
                renderer,
                "deck D preview",
                preview_ats.and_then(|c| c.get(3).copied()),
                scale,
            ),
        ];
        let slot_bind_groups = std::array::from_fn(|slot| {
            deck.slot_view(EngineSlot(slot as u8))
                .map(|view| present.create_bind_group_for(&gpu.device, view))
        });
        Engine {
            deck,
            // **Nothing wired by hand yet**, which is the state a run begins
            // in: the launch aims carry whatever `--edge` said, and this is the
            // list a rewiring writes and every later re-aim restates.
            edges: Vec::new(),
            capacity,
            present,
            picture,
            previews,
            slot_bind_groups,
            look: LOOK,
            // **Nothing in it at all**, which is the default chain: with no
            // slot the mix writes straight into the target the present pass
            // reads, so the frame this program opens on is the frame it drew
            // before the chain existed — bit for bit and for free.
            chain: Vec::new(),
            freed: 0,
            aimed,
            pointing,
            placed,
        }
    }

    /// Ask deck B to warm up, and let the budget answer. The one governor pass this
    /// program makes, taken at startup where the stall it costs is free, and the
    /// whole of why a strip on this panel can read one residency and have been
    /// asked for another.
    ///
    /// The other two slots are not in this, and that is the change. Deck B used to
    /// be the only other slot there was, so *the deck has a second slot* and *the
    /// panel can show a park* were one sentence; they are two now. C and D rest at
    /// `Residency::Allocated` — [`Engine::new`] says why — were asked for nothing,
    /// and come back from the pass as `Reason::OffAir`, which is the governor
    /// reporting that it was not asked about them. They cost the arithmetic below
    /// nothing: `committed_ms` is the sum over Live slots and deck A is the only
    /// one, so this sets the same budget it set with two slots, off the same
    /// measurement, for the same reason.
    ///
    /// It is `karakuri-cli`'s order rather than a second one: measure every slot
    /// before anything is decided about any of them, ask through
    /// [`Deck::set_residency`], and call [`Deck::govern`], which is the only thing
    /// in the engine that writes an *effective* residency. `set_residency` writes
    /// the request and grants it, so a harness that never governs has a deck whose
    /// two residencies agree on every slot and every frame — which is what this
    /// file was, and is why the roll ADR-0190 drew was tested and unreachable. The
    /// report comes back whole for the same reason `karakuri-cli`'s
    /// `report_governing` prints one: nothing is printed in the engine, so what an
    /// operator reads and what a test asserts are the same values.
    ///
    /// # The budget is what moves, and it is moved from what was measured
    ///
    /// A deck starts on `governor::DEFAULT_COMPUTE_BUDGET_MS` — one 60 Hz frame of
    /// measured per-Set cost — and what one of these Sets measures at is this
    /// machine's business rather than anything this file can know. So the budget is
    /// set from the measurement that was just taken: what deck A is already
    /// committed to, plus half of what warming deck B was measured at. Half of a
    /// cost is not that cost, so the request cannot fit — the refusal is arithmetic
    /// on every machine rather than on the ones where the numbers happen to come
    /// out.
    ///
    /// It used to be an eighth of it, and the change is ADR-0269's. The governor
    /// could once admit a slot at one step in `SLOWEST_PRIME_ONE_IN` frames and
    /// charge `cost / n` for it, so parking a request meant leaving headroom under
    /// `cost / 8`. There is no rate left to undercut: a drawn slot steps every
    /// frame, every slot is drawn, and a request either fits at its whole measured
    /// cost or is parked.
    ///
    /// A number computed from the measurement rather than a constant, because a
    /// constant is the fixture the product cannot produce (`docs/contributing.md`
    /// §3, *a check you have not watched fail is guessing*, read from the other
    /// side): a budget typed in here parks the request on this machine and admits
    /// it on a faster one, and a *measurement* typed in —
    /// `HotSwap::set_measured_cost` is public and would take one — is this file
    /// writing down the number the probe exists to take.
    ///
    /// Nothing here writes a residency, a strip or a park. The deck is asked and
    /// the governor answers; [`mixer`] reads both residencies back off the deck the
    /// way it reads the gain, and `view::Strip::pending` derives the disagreement.
    /// `Deck::is_parked` is not called in this file at all outside `mod gpu`.
    ///
    /// Where a measurement is missing the budget is left where it was, and the
    /// governor parks the request anyway for a different and more serious reason —
    /// an unmeasured Live slot means the committed cost is unknown, which suspends
    /// priming wholesale. The caller prints the reason it got rather than the one
    /// this comment expects.
    pub(crate) fn ask_to_prime(&mut self, gpu: &Gpu) -> Report {
        // **Before the first frame, and this is the only place it can be.**
        // Measuring means stepping and ends in a rewind, so `measure_slots`
        // skips any slot whose Set has already run — a deck measured late
        // stays unbudgetable rather than losing what it has simulated.
        self.deck.measure_slots(&gpu.device, &gpu.queue);
        // **And the estimate beside the measurement**, on the same terms: two
        // draws per cold slot, before the first frame and never on the render
        // thread. The governor budgets on this where it answers and on the
        // measurement where it does not
        // ([ADR-0296](../../../docs/adr/0296-the-governor-budgets-on-the-estimate-where-it-answers-and-on-the-measurement-where-it-does-not.md)),
        // and it is taken at the deck's own size, so a deck on a small output
        // stops being judged against a frame nobody is drawing.
        self.deck.estimate_slots(&gpu.device, &gpu.queue);
        self.deck
            .set_residency(EngineSlot(ASKED_TO_PRIME as u8), Residency::Priming);
        // **The number the governor will spend for this slot**, which since
        // ADR-0303 is not the same quantity as its measurement.
        //
        // This read the measurement alone and could not any more: the
        // measurement is taken at the size this file names — the preview cell,
        // which is what a slot is auditioned in — and the governor spends the
        // *estimate* at the output's size wherever one answers (ADR-0296). A
        // budget stated in one of those and spent in the other is not a
        // comparison, and on this machine it is not a small error either: the
        // reference workload estimates 45 ms at 1280x720 and measures 12 at a
        // 252x142 cell, so a budget off the measurement puts a single live
        // slot permanently over it.
        //
        // So the budget is taken on the same precedence the governor decides
        // on — estimate where it answers, measurement where it refuses. That
        // is not a new policy; it is ADR-0296's, applied to the budget's own
        // currency so that both sides of the comparison are about one frame.
        // **Whether that is the right repair is the maintainer's**, and the
        // two alternatives are in ADR-0303: extrapolating the measurement, or
        // stating the budget per size.
        let budgeted = |slot: &HotSwap| {
            slot.estimated_cost()
                .and_then(Estimate::ms)
                .or_else(|| slot.measured_cost().map(|cost| cost.ms))
        };
        if let (Some(committed), Some(warming)) = (
            budgeted(self.deck.slot(EngineSlot(ON_AIR as u8))),
            budgeted(self.deck.slot(EngineSlot(ASKED_TO_PRIME as u8))),
        ) {
            self.deck.set_compute_budget_ms(committed + warming / 2.0);
        }
        self.deck.govern()
    }

    /// Aim every sink at its own rectangle, and hand back what the console should
    /// draw in each — the picture, and one entry per preview cell.
    ///
    /// # A cell is aimed because there is a slot behind it, and residency has
    /// nothing to do with it
    ///
    /// This read `Deck::preview` once and aimed the one preview sink at the cell of
    /// the deck the output was auditioning: the sinks all took the same composited
    /// frame, so a sink left in deck A's cell would have drawn deck C's material
    /// under the letter `A` the moment somebody auditioned C. ADR-0240 retired the
    /// audition — the picture is the master mix and every cell is its own deck's
    /// monitor — and the sinks stopped taking the composited frame: each cell is
    /// drawn from `Deck::slot_view` for the slot it is lettered for, which is a
    /// texture that cannot be of the wrong deck.
    ///
    /// Then it gated the aim on `Residency::Live`, and that was the defect this
    /// pass removes. The reason given was that an off-air slot "is not stepping and
    /// has nothing new in its view", which was true only because the engine refused
    /// to draw one. It is the exact case
    /// [ADR-0258](../../../docs/adr/0258-the-look-comes-before-the-fader-so-a-cell-draws-every-slot-and-says-which-nothing-it-is.md)
    /// exists for: an operator decides whether to put a candidate on air by
    /// watching its cell, and a cell that is dark until the candidate is already on
    /// air answers the question after it stops being asked. The deck draws every
    /// slot into its own target on every frame now, so there is something new in
    /// every view, every frame.
    ///
    /// What is left to decide is whether there is a slot at all, and that is
    /// `slot_bind_groups[slot]`: `Deck::slot_view` is `None` past `slot_count`, so
    /// a deck of fewer than [`DECKS`] slots leaves the surplus cells with nothing
    /// to sample. The aim asks the same question the draw asks, so the two cannot
    /// disagree — a cell aimed but not drawn would be a texture from an earlier
    /// frame held under a letter, and a cell drawn but not aimed is a pass into
    /// nothing.
    pub(crate) fn aim(
        &mut self,
        gpu: &Gpu,
        renderer: &mut egui_wgpu::Renderer,
        layout: &karakuri_layout::Layout,
        scale: f32,
        // **The projector window's size, or `None` while it is closed.** The
        // second output, and the second half of what [`render_size`] is a
        // maximum over — handed in because the window is `Gfx`'s and this is
        // the engine.
        projector: Option<(u32, u32)>,
    ) -> (Option<Picture>, [Option<Picture>; DECKS]) {
        let (picture_at, preview_ats) = aims(layout, CANVAS);
        let picture = self
            .picture
            .aim(gpu, renderer, picture_at, scale, &mut self.freed);
        // Which cells have a slot behind them, read once for the frame: the
        // loop below both aims and reports off the same answer, and the draw in
        // `compose` reads the same `slot_bind_groups`.
        let behind: [bool; DECKS] =
            std::array::from_fn(|slot| self.slot_bind_groups[slot].is_some());
        let mut previews = [None; DECKS];
        let freed = &mut self.freed;
        for (slot, (sink, out)) in self
            .previews
            .iter_mut()
            .zip(previews.iter_mut())
            .enumerate()
        {
            let at = match behind[slot] {
                true => preview_ats.and_then(|cells| cells.get(slot).copied()),
                false => None,
            };
            let pic = sink.aim(gpu, renderer, at, scale, freed);
            if behind[slot] {
                *out = pic;
            }
        }
        // **What a slot's measurement is a measurement of**, told to the deck
        // here because this is the statement that knows it.
        //
        // This application has two resolutions and no third one: the output
        // size, which the mix is composited once at and which the picture and
        // every other sink is a resize of (ADR-0247), and the size of a deck
        // cell, which is what a slot is auditioned in. A per-slot measurement
        // used to be taken at a constant 1280x720 — a size nothing renders at
        // — and ADR-0303 removed it, so the size is named by whoever knows
        // the layout, which is this file and not the engine.
        //
        // **The first aimed cell, and every slot is told the same one.** The
        // four cells are one row of equal boxes and a probe measures a deck
        // with one target; a per-slot size would make four slots' numbers
        // incomparable, which is precisely what a governor summing them must
        // not have. With no cell aimed at all — the Program bay folded away —
        // nothing is said and the last size stands, because a bay that is not
        // laid out is not a statement that a measurement is about nothing.
        //
        // **Once a frame, and it is a store rather than a measurement.** The
        // cell moves when a divider moves or a bay folds, and the build worker
        // reads this per build, so a size taken once at startup would measure
        // every candidate of a session against whatever the window opened at.
        let cell = self.previews.iter().find(|p| p.aimed).map(|p| p.size);
        if let Some(cell) = cell {
            self.deck.set_measure_size(cell);
        }

        // **The frame's own size, derived from the outputs that are on** —
        // ADR-0247, and [`render_size`] is where the rule is. It is taken here
        // rather than anywhere else because this is the statement that has
        // just decided the picture's rectangle, and the picture's rectangle
        // *is* its size as an output.
        //
        // **A reallocation, so it is the frame's first act and not something
        // done mid-pass**, which is [`Presented::fit`]'s sentence one level
        // out: `aim` runs before `compose`, and P-0091 is about the render
        // thread. What it costs was measured on 2026-09-09 rather than
        // argued — **0.145 ms** for `Deck::resize` and `Present::resize`
        // together on a four-slot deck, host clock, biased high, and within a
        // few percent of the same at 466x262 and at 1920x1080 because what it
        // pays for is nine objects rather than their texels. It is paid on the
        // frames a size changed and on no others: every frame of a divider
        // drag on the Program bay's height is one of them, and it is the same
        // frame `Presented::fit` was already remaking the picture's texture
        // on.
        //
        // **Both, always, and in one statement.** `Frame::render` checks its
        // sizes and panics at the call site, so a deck resized without the
        // present pass is a loud failure a frame later — which is the right
        // failure and the wrong place to find out.
        let outputs = [self.picture.aimed.then_some(self.picture.size), projector];
        if let Some(at) = render_size(&outputs) {
            if at != self.present.size() {
                // **Nothing is printed here**, and that is the frame path
                // rather than reticence: every frame of a divider drag on the
                // Program bay's height changes this size, so a line would be
                // sixty a second and sixty formats a second with it. What the
                // frame is composited at is a *readout* — `Costs::say` prints
                // it beside the numbers it is about, which is what P-0095 asks
                // of a measurement, and the console page's own size pill is
                // where an operator reads it.
                self.present.resize(&gpu.device, &gpu.queue, at.0, at.1);
                self.deck.resize(&gpu.device, at.0, at.1);
                // **And every cell's bind group, because `Deck::resize`
                // replaced the views they were made from.**
                //
                // A bind group holds its view alive, so a stale one samples a
                // texture nothing draws into any more: the cells would go on
                // showing the last frame at the old size, under letters
                // saying their decks are running. That is the silent wrong
                // picture P-0094 refuses, and it is exactly the symptom
                // `Deck::resize`'s own comment names for the meters one line
                // along. It was found by
                // `every_cell_with_a_slot_behind_it_is_aimed_whatever_its_residency`
                // the first time the deck was resized from here.
                self.slot_bind_groups = std::array::from_fn(|slot| {
                    self.deck
                        .slot_view(EngineSlot(slot as u8))
                        .map(|view| self.present.create_bind_group_for(&gpu.device, view))
                });
            }
        }
        (picture, previews)
    }
}

/// Is anything making texels this frame? — which is the whole of what decides
/// whether the loop asks for another frame.
///
/// The rule, rather than the expression: anything that makes texels this frame
/// keeps the loop awake, and the list is closed. Everything the engine draws
/// into is read here — the picture, which is the one sink `compose` is handed,
/// and all [`DECKS`] preview cells, which [`monitor`] draws in that frame's
/// `finally` off each slot's own target — so a target added later and not added
/// to this is the same bug again, and it is the bug this file has already
/// shipped once: `live` was the picture alone, so folding the picture away left
/// deck A auditioning under it while the loop stopped asking for frames. It
/// fails in whichever direction the mistake is made — a window that goes on
/// drawing what nobody asked for, or a panel that keeps changing while the loop
/// sleeps.
///
/// A function rather than an expression in the frame path for the reason
/// [`Readout::pointer`] is a method: `window_event` cannot be called from a
/// test, so the part worth asserting is lifted out to where a test can reach it
/// — see `anything_that_makes_texels_keeps_the_loop_awake`.
pub(crate) fn live(view: &View) -> bool {
    // **The projector is the third thing on the list**, and leaving it off is
    // the bug this function's own paragraph describes, one output along: with
    // the picture folded away and a projector on, the loop would stop asking
    // for frames while a window on another display went on showing whatever
    // was last presented into it — the show stopped, with nothing said.
    view.picture.is_some() || view.previews.iter().any(Option::is_some) || view.projector
}

/// What the transport row reads this frame, out of the two things in this
/// file that know: the deck's oscillator, and what the last frame cost.
///
/// `Transport` here is `karakuri_console::view::Transport` — the console's row
/// of readouts — and not `karakuri_engine::transport::Transport`, which is a
/// slot's sync mode and is a different thing with the same word on it. This
/// takes a `&Deck` because it is on the side of the seam that is allowed one;
/// what crosses into the console is six numbers (ADR-0156).
///
/// # Where each number comes from, and that nothing is measured twice
///
/// - The tempo, the position and the grid. `Deck::signals` is the
///   session's one oscillator — the same one every binding reads — and
///   `Oscillator::bpm` and `Oscillator::beats` are its tempo and its musical
///   position. `beats` is unbounded and monotone, so which dot is lit and
///   which bar it is are arithmetic on it and the console does that
///   arithmetic. The deck advances it inside `render`, by the frame's own
///   measured step count ([`App::clock`], and not the fixed one a frame
///   carried until 2026-09-08), so this reads the position as of the end of
///   the last frame.
/// - The frame's cost. `Cost::whole` — the same three fields the reading
///   sums under *"the whole frame is a median"*, for the frame just drawn.
///   Nothing is timed twice: `Costs::push` kept the last `Cost` and this
///   divides nothing.
/// - The rate. `Costs::rate_now`, which is the reading's own `rate`
///   asked before its deadline rather than at it.
/// - How many beats a bar has. `karakuri_signal::oscillator::BEATS_PER_BAR`, which is
///   where the deck's own grid gets it, and which says of itself that it is
///   provisional until the IR format carries a time signature. Asked rather
///   than transcribed, so that the day it stops being 4 the beat grid stops
///   being four dots.
///
/// `None` before the first frame has been drawn, which is one frame of a run:
/// there is no frame cost yet, and a row that made one up would be inventing
/// exactly the reading this whole seam exists to refuse.
///
/// The rate is `None` unless something is live, and that is not caution
/// either. With nothing making texels this loop stops asking for frames, so
/// the last rate it measured would sit in the row describing a window that has
/// stopped drawing — the one number here that goes stale by standing still.
/// The frame time beside it does not: the last frame did cost that, whenever
/// it was.
pub(crate) fn transport(
    deck: &Deck,
    costs: &Costs,
    budget_ms: Option<f32>,
    live: bool,
    // **What the last write did**, which is the one thing in this row that is
    // not read off the deck here: a verdict exists once, in the drain
    // `staging` makes, so it is remembered in `Readout::health` and handed
    // over rather than asked for.
    health: Option<view::Stage>,
    // **Whether a session is being recorded**, which is the second thing in
    // this row that is not read off the deck: the recorder is this window's
    // and lives on [`App::recording`], so it is handed over rather than asked
    // for. It is always a value on this side — a program that holds a store
    // can always answer *am I recording* — and the console's `None` is for a
    // console nobody told.
    rec: view::Rec,
) -> Option<view::Transport> {
    let last = costs.last?;
    let grid = deck.signals().oscillator();
    Some(view::Transport {
        bpm: grid.bpm(),
        beats: grid.beats(),
        beats_per_bar: karakuri_signal::oscillator::BEATS_PER_BAR,
        fps: live
            .then(|| costs.rate_now())
            .flatten()
            .map(|rate| rate as f32),
        frame_ms: ms(last.whole()) as f32,
        budget_ms,
        health,
        rec: Some(rec),
    })
}

/// A `.kir` off disk, parsed and checked — the two stages `Set::build` wants a
/// `Checked` from, and no more. `karakuri-cli`'s `compile::load` is the same
/// two with a cost estimate and a source it keeps; neither is wanted here. How
/// many elements the geometry runs at, which is the L1's own declaration and
/// not a number written here.
///
/// `capacity [min, max] = default` is in the file and `Set::build` takes a
/// number, so somebody has to read one across. This used to be a `const
/// CAPACITY: u32 = 262144` — `drift_shell.kir`'s declared default, transcribed,
/// which was fine while that was the only file this could load and silently
/// wrong the moment it took a path: a procedure written for 131072 would have
/// run at 262144 and nothing would have said so.
///
/// The fallback is not the answer, it is the arm that cannot happen.
/// `karakuri_ir::DEFAULT_CAPACITY` is what is left when *nothing* declared one,
/// and `check_header` requires a `capacity` on every L1 — so a `Checked` that
/// passed always carries one and this `map_or` is the shape of the seam type
/// rather than a decision. Pointing the whole thing at `DEFAULT_CAPACITY` would
/// run every procedure at 262144 whatever it declared, which is the defect
/// `the_capacity_is_the_l1s_own_declaration_and_the_l4_declares_none` is run
/// against.
///
/// This is `karakuri-cli`'s `capacity_for` with no `--capacity` to override it,
/// and `karakuri_environment::watch`'s rebuild is the same line again. There is
/// no `--capacity` here on purpose: see [`USAGE`].
pub(crate) fn capacity_of(l1: &karakuri_ir::typed::Checked) -> u32 {
    l1.capacity
        .map_or(karakuri_ir::DEFAULT_CAPACITY, |declared| declared.default)
}
