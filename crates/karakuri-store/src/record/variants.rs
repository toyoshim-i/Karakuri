use serde::{Deserialize, Serialize};

use super::types::*;
use crate::hash::Hash;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Record {
    /// File schema version header, placed at line 0 of versioned `.kbset` and
    /// session ndjson files.
    #[serde(rename = "header")]
    Header {
        version: u32,
    },
    Set {
        id: String,
        v: u32,
    },
    Slot {
        /// Target node address within the Set. Default index 0 is omitted when serialized.
        #[serde(flatten)]
        at: NodeAddress,
        /// What this Set calls the node, for whatever wants to point at it.
        ///
        /// On the terms HTML gives an `id`, and the analogy settles where it lives: an
        /// `id` belongs to the element rather than to the tag, so a name belongs to the
        /// *use* and is written here rather than in the `.kir` — which is why one
        /// procedure loaded twice is two names and not a collision. Absent is the
        /// ordinary case and costs nothing: a name is a cost you pay when you want to
        /// point at something, and it is written here only where somebody chose one.
        /// What absent does *not* mean is a node nothing can point at — every node has
        /// a name whether or not one was written, and an unwritten one is derived from
        /// the procedure and disambiguated where the Set is built, which is what lets
        /// [`Record::Edge`] name both its ends in a file that wrote no names at all.
        /// See `docs/ir-spec.md`, "Naming a source, on the terms HTML gives an `id`".
        ///
        /// It is not part of the address. `(layer, index)` says which node the record
        /// is about and the name is one of the things it says about that node, exactly
        /// as `proc` is — so a file naming one node twice has named it twice and the
        /// later name wins, the way the later `proc` does. Keying the fold by the name
        /// would turn one renamed node into two that were never there; see `key_for` in
        /// `project.rs`.
        ///
        /// Uniqueness within a Set is not this record's to enforce. It is checked where
        /// every source is in hand, beside the composition check — a duplicate name is
        /// a Set that will not build, not a line that will not parse, and the
        /// vocabulary's job is to carry what a file said.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(rename = "proc")]
        proc_hash: Hash,
    },
    /// A node of an *authoring* Set file (`.kset`): everything [`Record::Slot`]
    /// says about a node, with the relative path of its `.kir` where the content
    /// address goes.
    ///
    /// A `t` of its own rather than a `slot` carrying a path instead of a `proc`,
    /// which is the shape anybody reaching for "the same record with one field
    /// swapped" arrives at. `docs/contributing.md` §4 rules out precisely that —
    /// *"reusing a tag for a differently shaped record in a different file"* — and
    /// the bill for ignoring it is paid by every reader that dispatches on `t`
    /// alone: a decoder meeting `slot` would have to know which file it came out of
    /// before it knew whether `proc` was there, and the one that forgot to ask
    /// would build a Set with a node missing rather than refuse a file. Two tags,
    /// and a decoder dispatching on `t` never meets a `slot` without an address.
    ///
    /// So a `.kbset` holding one of these is a file disagreeing with its own
    /// extension, which is a sentence the store can say because it is the whole of
    /// what that extension asserts — everything in a `.kbset` is already resolved,
    /// and `<store>/sets/` holds `.kbset` and only `.kbset` because a swap that can
    /// partially fail is not a swap
    /// (`docs/adr/0231-a-sets-two-forms-take-two-extensions-and-the-store-holds-only-the-resolved-one.md`,
    /// `docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md`).
    ///
    /// This is therefore the one variant that can only appear in a file the store
    /// will not hold, and it is refused for a reason none of its neighbours are
    /// refused for. A `gain` is the deck's, a `param_decl` is an artifact's; a
    /// `part` is this Set's own — it says what a `slot` says about the same node —
    /// and what is wrong with it in `sets/` is only that it has not been resolved
    /// yet. Hence a question of its own, [`Record::is_authoring`], and a sentence
    /// of its own from `StoreError::PartInSet`: an operator who wrote one is told
    /// which form their file is, not that it carries time.
    ///
    /// Resolution is a read, a hash and a store put — never a compile. A `slot`'s
    /// `proc` is the content address of the `.kir` source, so turning one of these
    /// into one is: read the file, hash the bytes, `put_artifact`, emit a `slot`
    /// naming that address. `karakuri_environment::setfile`'s `resolve` is where
    /// that happens, which is before the file reaches a store rather than during a
    /// load.
    Part {
        /// The node's layer, exactly as [`Record::Slot`] carries it.
        layer: Layer,
        /// Which node of that layer, on [`Record::Slot`]'s terms and by the same
        /// reading: absent is 0 and 0 is not written.
        #[serde(default, skip_serializing_if = "is_zero")]
        index: u32,
        /// What this Set calls the node — [`Record::Slot`]'s field, unchanged, because
        /// a name belongs to the *use* and the use is the same one. It survives
        /// resolution: the `slot` written in its place carries it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        /// Procedure source path relative to the containing setfile directory.
        path: String,
    },
    /// Overrides the `.kir` default. Outside the range the artifact declares, the
    /// Set is rejected at build time.
    Capacity {
        /// Which geometry this record resizes, on the same terms [`Record::Slot`] uses
        /// it. A Set holds more than one source now, each running at the default its
        /// own procedure declares, so a layer alone cannot say which of them is being
        /// resized: two geometries at two capacities were inexpressible in this format
        /// however they were spelled on the way in, which is what made this field part
        /// of the naming work rather than a follow-on to it — see
        /// `docs/adr/0111-a-name-lives-in-the-set-file-and-may-be-written-on-the-command-line.md`.
        ///
        /// `index` absent is node 0, not a wildcard, which is [`Record::Slot`]'s rule
        /// rather than [`Record::Param`]'s: this record names one node, and "every
        /// geometry at 524288" is not something a capacity has ever said. It is also
        /// why honouring the field cannot retarget anything — a Set that held one
        /// geometry had only node 0 to resize, so every file ever written means what it
        /// always meant, and 0 is not written, so it round-trips byte for byte. See
        /// [`NodeAddress`] for why the two are one field.
        #[serde(flatten)]
        at: NodeAddress,
        value: u32,
    },
    Param {
        /// Which node, or every node declaring `key`.
        ///
        /// `Some` addresses one node. `None` is a wildcard, not node 0 — and that is
        /// the difference from [`Record::Slot`] and [`Record::Procedure`], where absent
        /// *is* 0 because those records name exactly one node and always did. This one
        /// addresses a *value*, and a bare name reaching every declaration is both what
        /// it has always meant and the useful default: one knob moving every renderer
        /// that has an `exposure`.
        ///
        /// It is also what keeps every Set file ever written reading the same way.
        /// `layer` on this record was a placeholder that the loader ignored — the
        /// writer put `L1` on everything and said so — so honouring it now would
        /// silently retarget those files. The address is `(layer, index)` present or
        /// absent as a unit, so `layer` becomes load-bearing exactly when an `index`
        /// appears beside it, which is only in files this build wrote — see
        /// [`mod@node_or_every_node`] for how the wildcard survives being folded into
        /// one `Option<NodeAddress>` field without a bare `#[serde(flatten)]` mistaking
        /// a placeholder `layer` for an address.
        #[serde(flatten, with = "node_or_every_node")]
        at: Option<NodeAddress>,
        key: String,
        value: Value,
    },
    /// Attaches a signal to one `param` of one layer. The value written every frame
    /// is the signal put through `curve`, mapped onto `range`, and then blended
    /// against the param's own value by the sample's confidence — see "Set file
    /// format" in `docs/ir-spec.md`.
    Bind {
        /// Which procedure declares the `param`, and — unlike [`Record::Param`]'s —
        /// always load-bearing, wildcard or not: a binding resolves through the nodes
        /// of *one layer* (`karakuri_engine::binding::Binding` carries a required
        /// `layer` and an optional `index`, and `Set::bind` walks that layer's nodes
        /// alone), so there has never been a binding that meant *every layer*. That is
        /// why this stays a plain [`Layer`] rather than gaining [`NodeAddress`]'s
        /// treatment: `NodeAddress` and [`mod@node_or_every_node`] both hold *one whole
        /// address, or none*, and a bound layer with no index is not "no address" — it
        /// is a real, narrower one, on `karakuri_operation::BindAt`'s own words: *"an
        /// attachment's wildcard is a layer's, because the signal is written into that
        /// layer's uniform buffer."*
        layer: Layer,
        /// Which node of that layer, or every node of it declaring `key`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<u32>,
        key: String,
        signal: String,
        curve: String,
        range: [f32; 2],
        /// Only a `signal` of `"noise"` reads this, because a noise generator is the
        /// one signal with parameters of its own. Absent means the default generator
        /// rather than no generator: there is nothing else for the name to mean.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        noise: Option<BindNoise>,
    },
    Camera {
        kind: String,
        /// Which camera node of the L3 layer these numbers are about, on
        /// [`Record::Capacity`]'s and [`Record::Seed`]'s terms rather than
        /// [`Record::Param`]'s: this record describes one producer, and "every camera
        /// at radius 9" is not something a camera has ever said.
        ///
        /// Absent is node 0, and 0 is not written, so every file ever written
        /// round-trips byte for byte and keeps meaning what it meant: a Set held one
        /// camera, so node 0 was the only one there was to describe. It becomes
        /// load-bearing when a Set holds several, which is what an `edge` per renderer
        /// made expressible.
        #[serde(default, skip_serializing_if = "is_zero")]
        index: u32,
        radius: f32,
        speed: f32,
        /// Where the eye rides above the target — the third of the orbit's three
        /// placement numbers, and the newest field in this record.
        ///
        /// It carried two of the three until 2026-09-09, so a Set kept with its camera
        /// looking down and loaded again was looking along the equator and nothing said
        /// so. The three became parameters of the camera node that day
        /// (`docs/adr/0318-the-built-in-cameras-three-placement-numbers-are-parameter-rows.md`),
        /// which is what made the gap a defect rather than a limit: a number a hand can
        /// move and a file cannot record is a knob that walks back on its own.
        ///
        /// Absent reads as 2.0, which is what every file written before this field
        /// meant — the engine's `Orbit::default().height`, restated here because this
        /// crate depends on nothing and cannot ask for it. The two are held together by
        /// a test in `karakuri-environment`, which depends on both; see
        /// `default_camera_height`.
        ///
        /// Written unconditionally, on `radius`'s and `speed`'s terms rather than
        /// `index`'s: a placement number is what this record is for, and one omitted at
        /// its default would make a file's silence mean two things depending on which
        /// of the three it was about.
        #[serde(default = "default_camera_height")]
        height: f32,
    },
    /// This Set composites its renderers rather than overdrawing them.
    ///
    /// A Set built with `--merge N` gives every renderer a cleared target of its
    /// own and folds them through an L5 `karakuri_engine::node::Merge`; a Set
    /// without one runs them over a single attachment, the first clearing and the
    /// rest loading. That is `karakuri_engine::set::Layering`, and it was the one
    /// thing a Set knew about itself that a Set file could not say — so a
    /// composited Set saved and loaded back overdrew, and `Set::select_renderer`
    /// came back with nothing to select between: a saved variant pool was an
    /// unselectable one.
    ///
    /// A node with no procedure is described by a record of its own, and that is
    /// `camera`'s rule rather than a new one. What kept this out was "a Set file
    /// records the nodes of a Set, not how they meet each other", and that stopped
    /// holding when [`Record::Edge`] arrived — an edge records exactly how two
    /// nodes meet. The reading that does hold is already in the format: the
    /// built-in camera has no [`Record::Slot`], because it has no procedure to
    /// reference, and a [`Record::Camera`] describes it instead. An L5 has no
    /// `karakuri_ir::ast::Kind` for the same reason — the compositing is fixed,
    /// `karakuri-engine/src/shaders/composite.wgsl`, so there is nothing for a
    /// `kind L5` file to lower — and so it is described here rather than referenced
    /// by a `slot`.
    ///
    /// Its presence is the whole statement, mirroring the engine's own `Set::merge:
    /// Option<node::Merge>`. There is no boolean field, because a record that could
    /// say `false` would be a second spelling of its own absence — and two
    /// spellings of one fact leave a reader asking which of them a writer meant by
    /// writing the other.
    ///
    /// And no `index`. [`Record::Camera`] carries one because a Set may hold
    /// several cameras and a record has to say which of them it is about; a Set
    /// holds exactly one L5 — it is the node its single `Texture` output comes out
    /// of — so there is nothing here for an index to distinguish, and a field that
    /// could only ever be 0 is an invitation to write a file naming a node that
    /// cannot exist.
    Merge {
        /// Which renderer is the only live one, in draw order — the same numbering
        /// [`Record::Select`]'s `renderer` and a [`Record::Slot`]'s `index` use.
        ///
        /// Absent means every input is live, which is the state a merge nobody has
        /// selected in is in: `mix::Input::default()` sets `live` true, so a Set that
        /// was never selected in writes no `live` and reads back identically. It is
        /// emphatically not node 0 on [`Record::Slot`]'s rule — reading an absent field
        /// as 0 would silence every renderer but the first in every composited Set that
        /// was saved without a selection, which is a picture nobody asked for and
        /// nothing said had changed.
        ///
        /// `gain`, `opacity`, `blend` and `mask` are deliberately not fields beside it.
        /// `mix::Input` carries all four, and nothing outside
        /// `crates/karakuri-engine/tests/merge.rs` can set one — there is no flag, no
        /// key and no MCP tool, `Set::set_input` has exactly that one caller — so a
        /// field for them would have no producer and a Set file would record four
        /// defaults nobody chose. That is the argument `docs/ir-spec.md` makes for
        /// keeping `parent` off a metadata card rather than writing it empty: a record
        /// whose producer does not exist waits for it, because an empty one is worse
        /// than none. The record grows a per-input row when something can set one — one
        /// line per edge, on the terms `slot` and `capacity` are one line per node —
        /// and until then `live` is here because a selection is the one input control
        /// an operator can actually reach.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        live: Option<u32>,
    },
    /// Salts the hash builtins for one node, so re-seeding changes randomness
    /// without touching anything structural.
    Seed {
        stream: Layer,
        /// Which node of that layer, which is what lets a salt be *assigned* rather
        /// than derived.
        ///
        /// The salt is what makes two identical grids differ in colour without being
        /// arranged to, and it used to be derived — `hash(set_salt, ordinal)` — which
        /// `docs/ir-spec.md` called provisional for one reason: reordering `--set`
        /// changed which geometry got which randomness. A derived value cannot be
        /// recorded, because there is nothing stable to record it against; an addressed
        /// one can, and this is the address. `--save-set` writes one of these per
        /// geometry now, and `--load-set` gives each back to the source its index
        /// names.
        ///
        /// Absent is node 0 on [`Record::Slot`]'s terms rather than [`Record::Param`]'s
        /// — a seed salts the node it names — so a file salting one source per layer
        /// reads as it always did and is written back unchanged.
        #[serde(default, skip_serializing_if = "is_zero")]
        index: u32,
        value: u64,
    },
    /// Binds one node's declared input slot to another node of this Set.
    ///
    /// `slot` here is the third sense the module doc names — an input a node
    /// declares, not a node of a Set and not a member of the deck.
    ///
    /// A procedure declares what it takes and refuses to say where it comes from —
    /// `uses far : Geometry` names an input slot the way `consumes position` names
    /// an attribute, without naming which node supplies it, because a `.kir` that
    /// named one would be coupled to one Set and would stop being a library part.
    /// This record is the other half, and it is here rather than in the `.kir` for
    /// the reason a [`Record::Slot`]'s name is: an edge belongs to the *use*, and a
    /// Set file is what a use is recorded as.
    ///
    /// Addressed by name at both ends, which is the one record that is. Everything
    /// else here says which node it is about with `(layer, index)`, and an edge
    /// cannot: a position moves when the list is reordered, and reordering silently
    /// changing which geometry a morph blends towards is the exact failure this
    /// record exists to end. Every node has a name whether or not one was written —
    /// a name nobody wrote is derived from the procedure where the Set is built —
    /// so both ends always resolve.
    ///
    /// An input slot bound twice is refused where the Set is built rather than
    /// here, on the terms a duplicate node name is: the vocabulary's job is to
    /// carry what a file said.
    Edge {
        /// The node that declares the input slot.
        node: String,
        /// What that node's procedure calls that input slot.
        slot: InputPort,
        /// The node bound to it.
        to: String,
    },
    /// Inlined `.kir` source, for bundling an artifact with the Set that uses it.
    Src {
        hash: Hash,
        line: u32,
        s: String,
    },
    // -- The mix: durable state that belongs to the *session* -------------
    //
    // Everything above describes one Set and goes into a Set file. These
    // fifteen describe the deck the Sets are playing on, and a Set file must
    // not contain them — a Set does not know what fader it is under or whether
    // it is on air, and one that carried its gain would restore that gain
    // wherever it was next loaded. They are state all the same, which is what
    // separates them from the three below: `is_set_state` says no to all
    // eighteen of them and means two different things by it. (It says no to the
    // four metadata records further down as well, for a third reason that is
    // not about state at all — see the group comment above `Record::Meta`; the
    // count here is these eighteen and not that twenty-two. It read "seven" and
    // "twelve" from the commit that gave the mix a vocabulary until this one:
    // every record added since went in without the count moving, because a
    // prose count is not checked by anything. Both are counted off the
    // variants above, and the classification behind `is_set_state` is an
    // exhaustive match now, so that at least the *classification* cannot drift
    // the same way silently.)
    /// A deck slot's linear gain into the mix.
    ///
    /// The slot is an index into the deck rather than anything about the Set in it.
    /// Moving a Set to another slot moves it under another fader, which is what a
    /// fader is.
    Gain {
        slot: DeckSlot,
        value: f32,
    },
    /// A deck slot's fader: how much of its blend lands in the mix, `[0, 1]`.
    ///
    /// Separate from [`Record::Gain`] because they are separate controls, which
    /// only became visible once there was a blend mode that was not `add`. Gain is
    /// the level the material arrives at; opacity is how much of the blend takes
    /// effect, including how much that slot's layer covers. Under `add` the two
    /// multiply together and a stream could have carried either — under `over` a
    /// session that replayed one as the other would replay a deck slot's layer that
    /// hides as one that dims.
    Opacity {
        /// The deck slot whose fader this is.
        slot: DeckSlot,
        value: f32,
    },
    /// A deck slot's mute state.
    Mute {
        slot: DeckSlot,
        muted: bool,
    },
    /// A deck slot's solo state.
    Solo {
        slot: DeckSlot,
        soloed: bool,
    },
    /// A deck slot's online mix state.
    Online {
        slot: DeckSlot,
        online: bool,
    },
    /// How a deck slot's layer meets the ones under it: `add`, `over` or `max`.
    ///
    /// Slot order is stacking order, so this is the one mix control whose meaning
    /// depends on where the slot sits — which is why it is recorded per slot rather
    /// than as a property of the Set in it. Moving a Set to another slot moves it
    /// to another place in the stack.
    ///
    /// A `String` on the same terms as `residency` and `sync`: what a mode is
    /// allowed to be is the engine's to say, and a stream from a newer build
    /// reaches the engine's diagnostic rather than the parser.
    Blend {
        slot: DeckSlot,
        mode: String,
    },
    /// What a deck slot is asked to do: `live`, `priming`, or `allocated`.
    ///
    /// The request, never the effective level. The governor recomputes the second
    /// every pass from the budget of the machine that is running, so a session
    /// recorded on a fast machine and replayed on a slow one must re-derive it
    /// rather than replay it — recording what was decided would replay one
    /// machine's budget onto another's. This is the same choice [`Record::Tempo`]
    /// makes in the other direction and for the opposite reason: there, what was
    /// decided is the reproducible thing.
    ///
    /// A `String` rather than an enum, on the same terms as `curve` and
    /// `noise.kind`: an unrecognised level is the engine's to diagnose against what
    /// it actually supports. This one has earned it — five residency levels were
    /// named and three were built, because two of the five turned out to be
    /// transitions rather than states:
    /// `docs/adr/0062-warming-and-cooling-are-transitions-not-states.md`.
    Residency {
        slot: DeckSlot,
        level: String,
    },
    /// A deck slot's MCP modification policy: `auto`, `on` or `off`.
    Policy {
        slot: DeckSlot,
        policy: String,
    },
    /// The output look: tone map operator, exposure, and the operator's white
    /// point. One record rather than three because it is one value in the engine,
    /// written to one uniform, and a stream that could set the exposure without
    /// saying which operator it applies to would be describing a look nobody can
    /// reconstruct.
    ///
    /// Session-wide and deliberately not per slot: tone mapping happens once, after
    /// the mix, which is the whole argument in `karakuri-engine`'s `present`
    /// module.
    Look {
        op: String,
        exposure: f32,
        white_point: f32,
    },
    /// Master composite output gain applied before master post-processing effects (ADR-0224).
    MasterOut {
        value: f32,
    },
    /// Configuration of ordered L5 post-processing slots in the master chain (ADR-0340).
    MasterChain(Chain),
    /// The procedure a deck slot is playing, from this moment on.
    ///
    /// Written when a hot swap lands, which since ADR-0316 is the one moment the
    /// material a session is playing changes: the budget's verdict leaves the
    /// installed Set where it is and stops the slot, and there is no rollback to
    /// write a second record for. Without it a session recorded the material
    /// *once*, before the first frame, and replayed the whole run with whatever it
    /// started with: a set in which a procedure was rewritten at minute ten
    /// replayed as though it never had.
    ///
    /// The session's, not the Set's, which is the distinction `is_set_state` exists
    /// for. A Set file's `slot` record says what a Set *is*; this says what a deck
    /// slot *became*, at a point in time, which is a fact about a performance.
    ///
    /// `proc` is a content address and the source is in the store, on the same
    /// terms a Set file's `slot` record uses — so a rewrite costs one line here and
    /// a few kilobytes once, however many times the same procedure comes back.
    ///
    /// One thing it cannot carry, and ADR-0316 changed which. A swap-*in* is
    /// documented to start cold, so a replay meeting this record builds afresh and
    /// replays exactly. What this cannot say is that the slot was stopped: a
    /// version over the frame budget is recorded like any other, because it is what
    /// the slot holds, and a replay judges nothing — so it runs at full rate
    /// material the performance had frozen. That is a gap in this vocabulary and is
    /// recorded as one rather than patched here.
    Procedure {
        slot: DeckSlot,
        /// Which node the deck slot is playing this procedure on, when it has more than
        /// one.
        ///
        /// A deck slot draws with one L1 and however many L4s — several renderers over
        /// one geometry, in draw order — so naming a layer is no longer enough to name
        /// a procedure. `index` zero for the L1 and for the first renderer, which is
        /// every stream written before stacks existed: absent means zero and zero is
        /// not written, so an old stream replays byte for byte and a new one adds a
        /// field only where it says something. See [`NodeAddress`] for why the two are
        /// one field.
        #[serde(flatten)]
        at: NodeAddress,
        #[serde(rename = "proc")]
        proc_hash: Hash,
    },
    /// Who may move one node of the Set a deck slot is playing — `manual`,
    /// `suggesting` or `automatic`.
    ///
    /// A session fact about a node, which is [`Record::Procedure`]'s shape exactly
    /// and is why this is not a Set file's record. Every other record that names a
    /// node of a Set — [`Record::Slot`], [`Record::Capacity`], [`Record::Param`],
    /// [`Record::Bind`], [`Record::Seed`] — is written into a Set file and carries
    /// no `slot`, because what a Set *is* does not depend on which deck slot it is
    /// playing in. A Set does not know which agent is watching it either: authority
    /// is an arrangement between an operator and whatever else is in the room, made
    /// during a performance, and a Set file that carried one would hand a node to
    /// an agent wherever it was next loaded. So it is addressed the way `procedure`
    /// is — a deck slot, a layer and an index — and it goes in a session stream and
    /// never in a Set file.
    ///
    /// The address is a node and deliberately not a deck slot. Rule 06 of the
    /// manual: *"There is no switch that hands the whole instrument to an agent,
    /// because the useful arrangement is almost always partial."* A flag beside the
    /// `slot` and nothing else would be that switch.
    ///
    /// `index` is which node of that layer, on [`Record::Procedure`]'s terms:
    /// absent means 0 and 0 is not written, so this record costs an existing stream
    /// nothing and says something only where it is written.
    ///
    /// A `String` rather than an enum, on `residency`'s terms and `curve`'s: what a
    /// level is allowed to be is the engine's to say, and a stream from a newer
    /// build should reach a diagnostic about what this build supports rather than a
    /// parser that refuses the line.
    ///
    /// Nothing writes one yet, and what is missing is the engine. A node's
    /// authority has to survive a rebuild, which means it is restated on
    /// `karakuri_engine::swap::Request` the way that request's `params`,
    /// `published`, `bindings`, `names` and `edges` are — *"a rebuild that lets a
    /// value be re-derived is a rebuild that quietly discards what was loaded"*.
    /// Written here first because the vocabulary is what a surface asks in, and
    /// because the record is what says the fact is the session's: see
    /// `docs/adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md`.
    Authority {
        slot: DeckSlot,
        /// Which node this authority is over. `index` absent is 0 and 0 is not written,
        /// on [`Record::Procedure`]'s terms. See [`NodeAddress`] for why the two are
        /// one field.
        #[serde(flatten)]
        at: NodeAddress,
        /// `manual`, `suggesting` or `automatic` — the words
        /// `karakuri_operation::Authority::name` writes. Named for the concept rather
        /// than shortened, because there is no shorter word for it that is not already
        /// taken: `mode` is the word rule 06 refuses (*"never a global mode"*) and
        /// `level` is `residency`'s.
        authority: String,
    },
    /// Real-time parameter automation recorded during live performance (ADR-0280).
    Ride {
        /// The deck slot whose Set is being written — *a deck slot*, on
        /// [`Record::Procedure`]'s and [`Record::Authority`]'s terms.
        slot: DeckSlot,
        /// Which node, or every node declaring `key`. Absent is the wildcard; see
        /// [`NodeAddress`] for why the address is one field.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        at: Option<NodeAddress>,
        key: String,
        value: Value,
    },
    /// What is driving one parameter of a deck slot that is playing, or nothing —
    /// an attachment made during a performance, and the taking of one back.
    ///
    /// The session's twin of [`Record::Bind`], and the fourth record in this
    /// vocabulary to name a node of the Set a deck slot is playing.
    /// [`Record::Procedure`], [`Record::Authority`] and [`Record::Ride`] are the
    /// first three, and the reason is the same one all four times: a `bind` says
    /// what a Set *is* and carries no `slot`, because what a Set is does not depend
    /// on which deck slot it is playing in. This says what an operator *did*, to
    /// one deck slot, at one instant.
    ///
    /// One record for the attachment and the take-back, and `source` present or
    /// absent is which. They are one fact — *what is driving this parameter* — and
    /// two records would be two `t`s that the projection dropped for one reason and
    /// that the applier had to keep in step by care. An absent `source` is the
    /// removal and not a suspension: there is no suspended state anywhere in the
    /// engine to record, and the value a parameter is left at is its own, which is
    /// what a binding blends *from* at every confidence
    /// ([P-0084](../../../docs/principles/0084-a-confident-wrong-automatic-judgement-is-worse-than-not-judging.md)).
    ///
    /// The address is [`Record::Bind`]'s and not [`Record::Ride`]'s, and the
    /// difference is not a drift. A `ride` writes a value, and a value has a
    /// wildcard that names no layer — every node declaring the key, across every
    /// layer — which is why its address is one `Option<NodeAddress>`. A binding is
    /// a layer's: `karakuri_engine::binding::Binding` carries a required `layer`
    /// and an optional `index`, `Set::bind` resolves the nodes through it, and
    /// there has never been a binding that meant *every layer*. So `layer` here is
    /// load-bearing, which is the one thing [`Record::Param`]'s never was.
    ///
    /// ```ndjson
    /// {"t":"source","slot":0,"layer":"L1","key":"turbulence","source":{"signal":"energy","curve":"pow2","range":[0.1,2.4]}}
    /// {"t":"source","slot":0,"layer":"L1","key":"turbulence"} ```
    ///
    /// A hand on a bound parameter's value does not appear here. A `ride` writes
    /// the parameter's own value and the attachment goes on blending from it, so
    /// the two records never race: only this one attaches and only this one
    /// detaches. Decided in
    /// `docs/adr/0319-an-attachment-is-a-session-record-and-taking-a-parameter-back-removes-it.md`.
    Source {
        /// The deck slot whose Set is being written — on [`Record::Ride`]'s terms.
        slot: DeckSlot,
        /// Which layer declares the `param`, and it is read.
        layer: Layer,
        /// Which node of that layer, or every node declaring `key` — see
        /// [`Record::Bind`], which this follows exactly.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<u32>,
        key: String,
        /// What drives it, or nothing. Absent is *Take a parameter back*.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<Source>,
    },
    /// What size the session renders at, in texels — the canvas every `VideoSource`
    /// draws into and every deck slot is sized to match.
    ///
    /// Not the size of any window. A window is a preview and is fitted to this
    /// rather than the other way round, so dragging one changes what an operator
    /// can see and nothing about what is drawn. Without this record the two were
    /// the same number: a session played in a small window and replayed with a
    /// large `--size` rendered different pixels, and nothing in the stream said
    /// which of them was the performance.
    ///
    /// Written once, at the head, and a stream carries no second one. Changing it
    /// reallocates every deck slot's target and the HDR target, which is an
    /// allocation on the render thread — the one thing this engine's frame path
    /// forbids. So the canvas is a property of a run rather than a control an
    /// operator moves during one, and this is the only record here that is session
    /// state without being something a hand can reach mid-set.
    Canvas {
        width: u32,
        height: u32,
    },
    /// What shape of the frame a deck slot's layer reaches.
    ///
    /// A mask multiplies that layer's opacity per texel, which is what makes it a
    /// mask rather than a second fader — everything opacity does, done to part of
    /// the frame. `position` is how far the front has travelled and is the number a
    /// `transition` moves, so a wipe is this record plus a `transition` on `mask`
    /// and needs nothing of its own.
    ///
    /// `kind` is a `String` on the same terms as `blend` and `residency`: what a
    /// shape is allowed to be is the engine's to say. `angle` is in radians and is
    /// the linear front's alone.
    Mask {
        slot: DeckSlot,
        /// `none`, `linear` or `radial`.
        kind: String,
        /// Which way a linear front runs, in radians.
        angle: f32,
        /// How far it has travelled, `[0, 1]`. 0 reveals nothing, 1 reveals everything
        /// — both exactly.
        position: f32,
        /// How wide the soft edge is, `[0, 1]`. 0 is a hard edge.
        softness: f32,
    },
    /// A mix control moving over musical time: a fade, a cut, or half of a
    /// crossfade.
    ///
    /// One record for the whole move, and the values it produces are not recorded
    /// at all. A value per frame would be 216,000 lines an hour describing
    /// something the grid already determines — the same argument `tick` makes,
    /// where the engine advances by a step count and everything downstream is a
    /// function of it.
    ///
    /// `start` is an absolute position on the session's beat count rather than "in
    /// two bars", because a relative instant is a different instant depending on
    /// when it is read and a beat count is the same one on every run. Quantising to
    /// the next bar happens where the operator asked, once.
    ///
    /// `from` is deliberately absent, and is read where the move is *scheduled*
    /// rather than where it starts. Capturing it at the start would mean capturing
    /// it on the first frame at or after a musical instant, and a machine running
    /// at a different rate would capture it at a different beat — which is the one
    /// property this record exists to have. Nothing can move the control in
    /// between: a hand cancels the move, another move replaces it.
    ///
    /// `control` and `curve` are strings on the same terms as `curve` on a `bind`:
    /// what a name is allowed to be is the engine's to say.
    Transition {
        /// The deck slot whose control is moving.
        slot: DeckSlot,
        /// `gain`, `opacity` or `mask` — the last being the `position` a `mask` record
        /// carries, which is what a wipe moves.
        control: String,
        /// Where the control ends up.
        to: f32,
        /// The musical instant it begins, in beats.
        start: f64,
        /// How long it lasts, in beats. Zero is a cut.
        beats: f64,
        /// `lin`, `pow2`, `sqrt` or `smooth`.
        curve: String,
    },
    /// Which renderer of a composited deck slot is the live one, from a musical
    /// instant on.
    ///
    /// A record of its own rather than a `control` on [`Record::Transition`], and
    /// the engine's `transition::Control` already carries the reason: the things a
    /// transition moves are positions, and this is a choice. "Half way to renderer
    /// 2" does not name a picture, which is why there is no `beats` and no `curve`
    /// here — a selection is a cut, and a cut is a fade of zero beats with nothing
    /// left for a curve to shape. The other half of the argument is the address:
    /// every `transition` is about one `(slot, control)` pair and a selection is
    /// about one renderer *inside* the Set a deck slot is playing, which is a
    /// second index no control carries.
    ///
    /// `start` alone, on `transition`'s terms: an absolute position on the
    /// session's beat count, because "on the next bar" is a different instant
    /// depending on when it is read. Quantising happens once, where the operator
    /// asked.
    ///
    /// What it selects is an edge into the Set's L5, so it says something only
    /// where the deck slot was built to composite — `--merge N`. A deck slot that
    /// overdraws has no L5 for an edge to go into, and this record is then carried,
    /// replayed and without effect, exactly as `Set::set_input` is: refusing it
    /// would make a replay fail on a line that describes a performance that
    /// happened.
    ///
    /// It is still not folded into a Set file, and the reason has moved. The
    /// layering *is* a Set file record now — [`Record::Merge`], whose `live` field
    /// holds exactly the choice this one makes — so a composited Set no longer
    /// loads back as overdraw with nothing for a selection to be about. What no
    /// *session* record carries is the layering: nothing in a stream says that the
    /// Set in slot 3 composites, and nothing says which deck slot the Set at the
    /// head of a stream was played in, so a projection folding a session down
    /// cannot tell whether a selection it meets is about the Set it is writing. A
    /// selection stays a property of the *run*, like a gain, until a session record
    /// says otherwise. See `project::key_for`, and `docs/manual.md`, "Selecting one
    /// renderer of a slot".
    Select {
        /// The deck slot whose Set the selection is inside.
        slot: DeckSlot,
        /// Which renderer of that deck slot, in draw order — the same numbering
        /// `--param L4:1:name=value` and a `slot` record's `index` use.
        ///
        /// No wildcard and no `Option`. Absent is not "every renderer": a record that
        /// named none of them would be the un-selection this build does not have — see
        /// the manual for why it is one-way — and a reader meeting an absent field
        /// would have to guess which of the two it meant.
        renderer: u32,
        /// The musical instant it lands on, in beats.
        start: f64,
    },
    /// What a deck slot's clock does with the session's — `free`, `tempo` or
    /// `beat`, with the two numbers that make the mode mean something.
    ///
    /// `anchor_bpm` is the tempo at which this material runs at 1x, and it has to
    /// be recorded because material has no intrinsic tempo: a `.kir` declares
    /// parameters and a capacity, not a bar length, so "one beat of music is how
    /// many seconds of material" is an operator's answer rather than the
    /// artifact's. `scrub_beats` is where the operator scrubbed the slot to, in
    /// beats off the room's position — signed, unbounded, and the one value in this
    /// format that is meant to go backwards.
    ///
    /// It was spelled `offset_beats`, and the rename reaches the wire. The word
    /// `offset` already names the operator's *latency* offset — the milliseconds of
    /// `--latency-offset-ms` — and one word for two controls on one panel is what
    /// `docs/contributing.md` §4 forbids. The beat side had a name it was not
    /// using: its control is the scrub, its operation is `ScrubDeck`, and
    /// `Transport`'s own field doc already called it *"the operator's scrub"*.
    /// Keeping the old spelling on disk behind a `serde(rename)` was the
    /// alternative, and it is the compatibility alias
    /// `docs/adr/0117-before-v1-pay-the-cost-of-changing-toward-the-ideal.md`
    /// refused for `field(p)`: two spellings for one thing, forever. Before v1 the
    /// bill is a bill and not an argument
    /// (`docs/principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md`),
    /// and the bill here is loud rather than silent — a stream carrying
    /// `offset_beats` fails its line with serde's missing-field error, not with a
    /// scrub silently read as zero.
    ///
    /// Both are carried even under `free`, where neither does anything, so that a
    /// deck slot moved back onto the grid returns to where the operator left it
    /// rather than to a default.
    Transport {
        slot: DeckSlot,
        sync: String,
        anchor_bpm: f32,
        scrub_beats: f64,
    },
    /// A deck slot's material was written out as a Set file, under `id`.
    ///
    /// The first record in this vocabulary whose subject is outside the stream.
    /// Everything else here describes the deck, and a reader that obeys it
    /// reproduces the performance; this one describes a file that was created in a
    /// store, and a reader that obeyed it would create a file instead. So a replay
    /// does not perform it and says which id it passed over — see
    /// `docs/ir-spec.md`, "Records with an effect outside the stream".
    ///
    /// The deck slot and the id, and deliberately not the node hashes. The Set file
    /// under that id already names every node it holds, and a copy of them here
    /// would be a second place for one fact — free to be right on the day it was
    /// written and wrong the moment the two are read apart. What this record is for
    /// is saying *that* a save happened and *what it is called*; what was saved is
    /// a question the Set file answers.
    ///
    /// Written at the frame the save landed, not at the key press. The store write
    /// is off the render thread, so it finishes some frames later and can fail; a
    /// record written at the press would claim a file that the disk went on to
    /// refuse. This is the same rule [`Record::Procedure`] follows — a record that
    /// describes a change already made.
    Save {
        slot: DeckSlot,
        /// What the Set file is called in the store: `sets/<id>.kbset`. A `String`
        /// because it is a name somebody chose, and the only thing in this record that
        /// can be looked up.
        id: String,
    },
    // -- What a frame saw or decided --------------------------------------
    /// How far this frame advances. Emitted from real time when live, read back
    /// verbatim on replay — which is what keeps substepping deterministic. Always 1
    /// in v0.2, capped at [`MAX_STEPS`].
    Tick {
        steps: u8,
    },
    /// One frame's worth of measured signals, on the same terms as
    /// [`Record::Tick`]: derived from a device when live, read back verbatim on
    /// replay, and the engine cannot tell which happened.
    ///
    /// Live audio is not reproducible and the record stream is required to be, so
    /// the two can only both be true if the measurement joins the stream. This is
    /// that record, and its shape follows from what a frame is:
    ///
    /// - One line per frame, so it interleaves with `tick` and every edit lands at
    ///   an exact frame position, the way the session stream format already promises.
    /// - Named fields for named signals and a positional array for the bands,
    ///   because `band0`…`bandN` *are* positions — an array is the naming scheme
    ///   rather than a second one. A map of name to value would also encode which
    ///   names it carries, at the cost of repeating those names 200,000 times an hour
    ///   and of letting a stream invent a name the bus has rules about (`noise` is
    ///   not a bus name, and a generic map is where that rule would be broken).
    /// - One confidence for the frame, not one per signal: these values all came out of
    ///   the same block of samples at the same instant, so their staleness is one
    ///   number. A per-signal confidence would be four copies of it.
    ///
    /// The array's length is the band count, so a stream carrying more bands than a
    /// reader knows about still decodes — a fixed-length array would make a band
    /// count a breaking format change, which is the opposite of what the
    /// unknown-`t` rule is for.
    Audio {
        /// Broadband level, `[0, 1]`.
        energy: f32,
        /// Transient envelope, `[0, 1]`: 1.0 at a detected onset, decaying from there.
        /// Not an impulse — an impulse one analysis block wide would be missed by some
        /// frames and seen twice by others.
        onset: f32,
        /// Per-band level, `[0, 1]`, low band first. Position is the name.
        bands: Vec<f32>,
        /// How much of this frame to believe, `[0, 1]`. Full while a device is open and
        /// delivering — a silent room is 0.0 energy at confidence 1.0 — falling as the
        /// last block goes stale, and 0.0 when nothing has arrived, which leaves every
        /// bound parameter at its own value.
        confidence: f32,
    },
    /// What the local oscillator's tempo and phase are corrected to, this frame.
    /// The same terms as [`Record::Tick`] and [`Record::Audio`]: derived live, read
    /// back verbatim on replay.
    ///
    /// v0.2 had no tempo record at all: the session tempo arrived by CLI flag and
    /// nothing in the stream could say what it was. This closes that, and it closes
    /// it with the *correction* rather than with the estimate, for a reason worth
    /// stating: an analyser is allowed to improve, and a session recorded today has
    /// to replay the same way after it does. Recording what was decided rather than
    /// what was heard is what makes that true. It is also why replay does not need
    /// the audio.
    ///
    /// The first one in a stream is what sets the session tempo, so this record is
    /// both "the tempo is now this" and "the tempo has moved a little"; a
    /// correction with `shift` 0.0 and `confidence` 0.0 is a free-running tempo
    /// being stated.
    Tempo {
        /// The tempo from now on. A tempo change does not move a beat that has already
        /// happened — see `Oscillator::correct`.
        bpm: f32,
        /// Phase shift in beats, positive meaning the next beat arrives sooner. Almost
        /// always tiny: the grid is predicted and trimmed, not chased.
        shift: f32,
        /// How much the estimate behind this correction was believed. Carried so a
        /// replay can show an operator what the live run showed, and because a
        /// correction that was applied at low confidence is a different event from the
        /// same numbers applied at high confidence.
        confidence: f32,
    },
    // -- A third file's vocabulary: an artifact's metadata ----------------
    //
    // `<hash>.meta.ndjson` — what an artifact *declares*, regenerated from its
    // `.kir` plus a compile pass and never hand-authored. Neither Set state nor
    // session state: a Set file says what a value *is* and a session says what a
    // performance *did*, and these say what a procedure *offers* before anything
    // has instantiated it. `Store::write_set` refuses them, and
    // `Record::is_metadata` is the question it asks — a separate one from
    // `is_set_state`, which asks *may this go in a Set file* and now answers no
    // to these for a third reason. See the head of `Record::is_set_state`.
    //
    // **The same enum, and the names disjoint from both other vocabularies** —
    // `param_decl` beside `param`, `capacity_decl` beside `capacity`, which is
    // what `docs/ir-spec.md`'s "one `t` means one shape, across every file"
    // asks. That rule is about names; sharing the *type* is what makes the
    // refusal possible at all. A separate enum was tried on paper and cannot
    // work: a metadata line fed to the Set decoder would deserialise to
    // [`Record::Unknown`], which is precisely the value the format promises to
    // pass over — so a Set file with a `param_decl` in it would be written
    // without complaint, and read back as a line nobody could name.
    //
    // **The one name that was not disjoint is the one that moved.** The
    // specification's ninth record was a metadata `preview` carrying a `path`,
    // against a deck `preview` carrying an `Option<u8>` slot — and sharing one
    // enum made that collision *silent*, because the deck record's `slot` was
    // an `Option` and the specified line decoded as the deck record with
    // `path` dropped. It is `thumbnail` now. Renaming the deck record instead
    // was the alternative and was worse at the time: it was written into
    // session streams, so moving it would break reading them, where nothing
    // has ever written the metadata one. A suffix — `preview_path` — was the
    // other, and it would say these are two versions of one concept, which is
    // what `param_decl` beside `param` legitimately is and what a stored asset
    // beside a live audition is not. **The deck record has since been retired
    // outright** (ADR-0240), which frees the name and changes nothing here:
    // `thumbnail` is the word the library already used.
    //
    // Four of the nine records the specification lists, because four is what a
    // compile pass can produce. `perf` is a measurement and wants a probe;
    // `origin`, `parent` and `tag` have no producer until something generates
    // procedures, and `thumbnail` names a stored asset nothing renders yet.
    // Writing an empty `parent` would be worse than leaving room for it — see
    // `docs/ir-spec.md`, "Metadata file format".
    /// The head of a metadata file: which artifact this describes, and what it
    /// calls itself.
    ///
    /// `hash` is the content address of the `.kir` the rest of the file was read
    /// off, and it is written down rather than left implicit in the file name: a
    /// metadata file copied, renamed or quoted out of context still says what it is
    /// about, and a reader can tell a stale card from a current one without
    /// trusting a path.
    ///
    /// `name` is the procedure's own declared name — the `.kir`'s `proc <name>` —
    /// and never a name a Set gave a node, which belongs to the *use* and is
    /// recorded in a Set file's [`Record::Slot`].
    Meta {
        hash: Hash,
        name: String,
        kind: Layer,
        v: u32,
    },
    /// One parameter a procedure declares: its range and, where the declaration is
    /// a number this build can state, its default.
    ///
    /// A declaration and not a value, which is the whole of what separates it from
    /// [`Record::Param`] — that one says what a Set turned a knob to, this one says
    /// the knob exists and what it may be turned between.
    ParamDecl {
        key: String,
        /// The declared type's spelling, as `.kir` writes it: `float`, `vec3`. A
        /// `String` rather than an enum because the metadata file is read by a decoder
        /// that does not parse `.kir` and has no use for a closed set — and because a
        /// type the language grows is then a value this crate carries without a
        /// release.
        #[serde(rename = "type")]
        ty: String,
        min: f32,
        max: f32,
        /// Absent means the default is not a number this build can state, never that
        /// there is no default: `param <name> : <ty> [<min>, <max>] = <expr>` makes the
        /// expression mandatory, so every declared param has one.
        /// `karakuri_ir::Param::default_scalar` folds a literal and a leading negation
        /// and nothing else.
        ///
        /// The record is still written, which is the decision. Dropping it instead
        /// would make a reader believe the artifact declares no such param, and that is
        /// a false statement about the declaration where a missing key is a true
        /// statement about what is known. It also has to survive the unknown-key rule:
        /// a decoder that ignores keys it does not recognise cannot tell an absent
        /// `default` from one it skipped, and under either reading the answer is *"not
        /// known"* — which is the answer. Under the other choice the two readings
        /// differ, because a record that is not there cannot be skipped into existence.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default: Option<f32>,
    },
    /// The element count an L1 declares it can run between, and what it runs at
    /// when nothing says otherwise.
    ///
    /// One per file at most: `capacity` is a header declaration and only an L1 has
    /// one. No record at all is a procedure that declares none — every kind but L1,
    /// and an L1 written before the declaration existed.
    CapacityDecl {
        min: u32,
        max: u32,
        default: u32,
    },
    /// The attributes a procedure writes, in declaration order.
    ///
    /// Absent rather than empty where a procedure declares no `emit` — an L4 emits
    /// nothing, and a record saying so tells a reader exactly what its absence
    /// tells them. The rule across this group is one record per declaration, and a
    /// declaration nobody wrote produces none.
    Emit {
        attrs: Vec<String>,
    },
    /// Forward compatibility: an unrecognised `t` is ignored, not an error.
    #[serde(other)]
    Unknown,
}

/// Beyond this the simulation is allowed to fall behind rather than catch up.
/// Unbounded catch-up turns a load spike into a death spiral.
pub const MAX_STEPS: u8 = 4;
