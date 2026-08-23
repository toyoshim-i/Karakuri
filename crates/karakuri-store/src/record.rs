//! ndjson records.
//!
//! One record per line, and concatenation is composition. The same record types
//! serve three files:
//!
//! - a **Set file** (`.set.ndjson`) is a state projection — what is loaded and
//!   what every value currently is. It carries no time, so it never contains
//!   [`Record::Tick`].
//! - a **session stream** is a timeline — a Set file followed by ticks and the
//!   edits between them. Every edit lands at an exact frame position because it
//!   sits between two known ticks.
//! - an **artifact's metadata** (`<hash>.meta.ndjson`) is what one procedure
//!   *declares*, regenerated from its `.kir` plus a compile pass. It is neither
//!   of the other two and is read by a decoder of its own — see
//!   [`Record::is_metadata`].
//!
//! A Set file is the session stream with the ticks dropped and the state folded
//! down. See the Set file, session stream and metadata file sections of
//! `docs/ir-spec.md`.
//!
//! **One type for all three, and the names disjoint across them.** The
//! specification's rule is that one `t` means one shape in every file, which is
//! a rule about *names* — `param_decl` beside `param` — and sharing the Rust
//! type is what lets a decoder tell a record in the wrong file from a record it
//! has never heard of. Three enums could not: a `param_decl` read by the Set
//! decoder would come back [`Record::Unknown`], and the format promises to pass
//! over exactly that.
//!
//! **The one collision there was is settled, and it is the metadata name that
//! moved.** `docs/ir-spec.md` listed a metadata `preview` carrying a `path`
//! beside [`Record::Preview`], the deck's record for which slot is being
//! auditioned. Two shapes under one `t`, and silently so: the deck record's
//! `slot` is an `Option`, so the specified line decoded as
//! `Preview { slot: None }` with its `path` dropped and nothing said — the one
//! record for which "an unknown `t` is ignored" protected nothing, because the
//! `t` was not unknown. The library asset is spelled `thumbnail` now, which is
//! the word `docs/roadmap.md` already used for it. The deck record could not be
//! the one to move: it is written into session streams that exist on disk,
//! where nothing has ever written the metadata one.

use serde::{Deserialize, Serialize};

use crate::hash::Hash;

/// Which kind of procedure a record addresses.
///
/// **A format addition, not a format change**: `Field` was added after the
/// other four, and every stream written before it is read unchanged because a
/// value nobody wrote cannot appear. That is the same move the `blend`
/// declaration made by existing with one legal value — the shape is chosen so
/// that growing it costs nothing to what came before.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Layer {
    L1,
    L2,
    L3,
    L4,
    /// A `kind Field` procedure. **It addresses no node** — a field has no pass
    /// and no buffers, it lowers into whoever evaluates it — but its `param`s
    /// are declared, addressable and an operator's to ride, so a record naming
    /// them needs somewhere to say so.
    Field,
}

/// `serde`'s `skip_serializing_if` wants a predicate by path, and `u32::is_zero`
/// is unstable. One line so that an index of 0 — which is every record written
/// before a layer could hold more than one node, whether that is a slot's two
/// renderers or a Set's two geometries — leaves the stream exactly as it was.
fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// A parameter value. Ranges are declared in the `.kir`; this is just the value.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Scalar(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
}

/// The noise generator a `bind` declares for itself.
///
/// Every field has a default, and they are the defaults of the generator in
/// `karakuri-signal`: a `bind` that names `noise` and says nothing else gets
/// one cycle per beat of perlin on stream 0. A generator omitted field by
/// field is a generator that was under-specified, not one that was refused —
/// this is the one record whose parameters an LLM has to invent numbers for,
/// and every number here has a defensible one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BindNoise {
    /// `white`, `value`, `perlin`, or `fbm`. A `String` for the same reason
    /// `curve` is one: an unrecognised value is the engine's to diagnose
    /// against what it actually supports, not the decoder's to reject before
    /// anything can say what the alternatives were.
    #[serde(default = "default_noise_kind")]
    pub kind: String,
    /// Cycles per beat, so period is tempo-relative.
    #[serde(default = "default_noise_rate")]
    pub rate: f32,
    /// Decorrelates one binding from another. Two bindings sharing a stream
    /// move together.
    #[serde(default)]
    pub stream: u64,
    /// `fbm` only, ignored by the other three kinds. Not in v0.2 of the spec,
    /// which listed `fbm` as a `kind` while giving it nowhere to say how many
    /// octaves — the spec is corrected rather than the field being dropped,
    /// because an octave count baked into the engine is exactly the "fixed
    /// property nobody can reach" that Spawn timing rejects Poisson for.
    #[serde(default = "default_noise_octaves")]
    pub octaves: u32,
}

fn default_noise_kind() -> String {
    "perlin".to_string()
}

fn default_noise_rate() -> f32 {
    1.0
}

fn default_noise_octaves() -> u32 {
    4
}

impl Default for BindNoise {
    fn default() -> BindNoise {
        BindNoise {
            kind: default_noise_kind(),
            rate: default_noise_rate(),
            stream: 0,
            octaves: default_noise_octaves(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Record {
    Set {
        id: String,
        v: u32,
    },
    Slot {
        layer: Layer,
        /// **Which node of that layer**, on the same terms
        /// [`Record::Procedure`] uses it: a Set draws with one L1 and however
        /// many L4s, so a layer alone no longer names a procedure.
        ///
        /// Absent means 0 and 0 is not written, so a file from before stacks
        /// existed round-trips byte for byte. It is what the *projection* folds
        /// on — several `slot` records on one layer are several nodes rather
        /// than one node corrected several times, and a fold keyed by layer
        /// alone would keep the last of them. Reading order would have been
        /// enough for the file format and is not enough for the fold, because a
        /// fold has no order to appeal to.
        #[serde(default, skip_serializing_if = "is_zero")]
        index: u32,
        /// **What this Set calls the node**, for whatever wants to point at it.
        ///
        /// On the terms HTML gives an `id`, and the analogy settles where it
        /// lives: an `id` belongs to the element rather than to the tag, so a
        /// name belongs to the *use* and is written here rather than in the
        /// `.kir` — which is why one procedure loaded twice is two names and
        /// not a collision. Absent is the ordinary case and costs nothing:
        /// **a name is a cost you pay when you want to point at something**,
        /// and it is written here only where somebody chose one. What absent
        /// does *not* mean is a node nothing can point at — every node has a
        /// name whether or not one was written, and an unwritten one is
        /// derived from the procedure and disambiguated where the Set is
        /// built, which is what lets [`Record::Edge`] name both its ends in a
        /// file that wrote no names at all. See `docs/ir-spec.md`, "Naming a
        /// source, on the terms HTML gives an `id`".
        ///
        /// **It is not part of the address.** `(layer, index)` says which node
        /// the record is about and the name is one of the things it says about
        /// that node, exactly as `proc` is — so a file naming one node twice
        /// has named it twice and the later name wins, the way the later
        /// `proc` does. Keying the fold by the name would turn one renamed
        /// node into two that were never there; see `key_for` in
        /// `project.rs`.
        ///
        /// Uniqueness within a Set is not this record's to enforce. It is
        /// checked where every source is in hand, beside the composition
        /// check — a duplicate name is a Set that will not build, not a line
        /// that will not parse, and the vocabulary's job is to carry what a
        /// file said.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(rename = "proc")]
        proc_hash: Hash,
    },
    /// Overrides the `.kir` default. Outside the range the artifact declares,
    /// the Set is rejected at build time.
    Capacity {
        layer: Layer,
        /// **Which geometry of that layer**, on the same terms
        /// [`Record::Slot`] uses it. A Set holds more than one source now,
        /// each running at the default its own procedure declares, so a layer
        /// alone cannot say which of them is being resized: two geometries at
        /// two capacities were inexpressible in this format however they were
        /// spelled on the way in — see `docs/roadmap.md`, "Naming what a Set
        /// holds".
        ///
        /// **Absent is node 0, not a wildcard**, which is [`Record::Slot`]'s
        /// rule rather than [`Record::Param`]'s: this record names one node,
        /// and "every geometry at 524288" is not something a capacity has ever
        /// said. It is also why honouring the field cannot retarget anything —
        /// a Set that held one geometry had only node 0 to resize, so every
        /// file ever written means what it always meant, and 0 is not written,
        /// so it round-trips byte for byte.
        #[serde(default, skip_serializing_if = "is_zero")]
        index: u32,
        value: u32,
    },
    Param {
        layer: Layer,
        /// **Which node of that layer, or every node declaring `key`.**
        ///
        /// `Some(n)` addresses one node. **Absent is a wildcard, not node 0** —
        /// and that is the difference from [`Record::Slot`] and
        /// [`Record::Procedure`], where absent *is* 0 because those records
        /// name exactly one node and always did. This one addresses a *value*,
        /// and a bare name reaching every declaration is both what it has
        /// always meant and the useful default: one knob moving every renderer
        /// that has an `exposure`.
        ///
        /// It is also what keeps every Set file ever written reading the same
        /// way. `layer` on this record was a placeholder that the loader
        /// ignored — the writer put `L1` on everything and said so — so
        /// honouring it now would silently retarget those files. The address is
        /// `(layer, index)` present or absent as a unit, so `layer` becomes
        /// load-bearing exactly when an `index` appears beside it, which is
        /// only in files this build wrote.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<u32>,
        key: String,
        value: Value,
    },
    /// Attaches a signal to one `param` of one layer. The value written every
    /// frame is the signal put through `curve`, mapped onto `range`, and then
    /// blended against the param's own value by the sample's confidence — see
    /// "Set file format" in `docs/ir-spec.md`.
    Bind {
        layer: Layer,
        /// Which node of that layer, or every node declaring `key` — see
        /// [`Record::Param`], which this follows exactly.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<u32>,
        key: String,
        signal: String,
        curve: String,
        range: [f32; 2],
        /// Only a `signal` of `"noise"` reads this, because a noise generator
        /// is the one signal with parameters of its own. Absent means the
        /// default generator rather than no generator: there is nothing else
        /// for the name to mean.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        noise: Option<BindNoise>,
    },
    Camera {
        kind: String,
        /// **Which camera node of the L3 layer these six numbers produce**, on
        /// [`Record::Capacity`]'s and [`Record::Seed`]'s terms rather than
        /// [`Record::Param`]'s: this record describes one producer, and "every
        /// camera at radius 9" is not something a camera has ever said.
        ///
        /// **Absent is node 0, and 0 is not written**, so every file ever
        /// written round-trips byte for byte and keeps meaning what it meant: a
        /// Set held one camera, so node 0 was the only one there was to
        /// describe. It becomes load-bearing when a Set holds several, which is
        /// what an `edge` per renderer made expressible.
        #[serde(default, skip_serializing_if = "is_zero")]
        index: u32,
        radius: f32,
        speed: f32,
    },
    /// **This Set composites its renderers rather than overdrawing them.**
    ///
    /// A Set built with `--merge N` gives every renderer a cleared target of
    /// its own and folds them through an L5 `karakuri_engine::node::Merge`; a
    /// Set without one runs them over a single attachment, the first clearing
    /// and the rest loading. That is `karakuri_engine::set::Layering`, and it
    /// was the one thing a Set knew about itself that a Set file could not
    /// say — so a composited Set saved and loaded back overdrew, and
    /// `Set::select_renderer` came back with nothing to select between: a
    /// saved variant pool was an unselectable one.
    ///
    /// **A node with no procedure is described by a record of its own, and
    /// that is `camera`'s rule rather than a new one.** What kept this out was
    /// "a Set file records the nodes of a Set, not how they meet each other",
    /// and that stopped holding when [`Record::Edge`] arrived — an edge
    /// records exactly how two nodes meet. The reading that does hold is
    /// already in the format: **the built-in camera has no [`Record::Slot`],
    /// because it has no procedure to reference, and a [`Record::Camera`]
    /// describes it instead.** An L5 has no `karakuri_ir::ast::Kind` for the
    /// same reason — the compositing is fixed, `karakuri-engine/src/shaders/composite.wgsl`, so
    /// there is nothing for a `kind L5` file to lower — and so it is described
    /// here rather than referenced by a `slot`.
    ///
    /// **Its presence is the whole statement**, mirroring the engine's own
    /// `Set::merge: Option<node::Merge>`. There is no boolean
    /// field, because a record that could say `false` would be a second
    /// spelling of its own absence — and two spellings of one fact leave a
    /// reader asking which of them a writer meant by writing the other.
    ///
    /// **And no `index`.** [`Record::Camera`] carries one because a Set may
    /// hold several cameras and a record has to say which of them it is about;
    /// a Set holds exactly one L5 — it is the node its single `Texture` output
    /// comes out of — so there is nothing here for an index to distinguish,
    /// and a field that could only ever be 0 is an invitation to write a file
    /// naming a node that cannot exist.
    Merge {
        /// **Which renderer is the only live one**, in draw order — the same
        /// numbering [`Record::Select`]'s `renderer` and a [`Record::Slot`]'s
        /// `index` use.
        ///
        /// **Absent means every input is live**, which is the state a merge
        /// nobody has selected in is in: `mix::Input::default()` sets `live`
        /// true, so a Set that was never selected in writes no `live` and
        /// reads back identically. It is emphatically not node 0 on
        /// [`Record::Slot`]'s rule — reading an absent field as 0 would
        /// silence every renderer but the first in every composited Set that
        /// was saved without a selection, which is a picture nobody asked for
        /// and nothing said had changed.
        ///
        /// **`gain`, `opacity`, `blend` and `mask` are deliberately not fields
        /// beside it.** `mix::Input` carries all four, and nothing outside
        /// `crates/karakuri-engine/tests/merge.rs` can set one — there is no
        /// flag, no key and no MCP tool, `Set::set_input` has exactly that one
        /// caller — so a field for them would have no producer and a Set file
        /// would record four defaults nobody chose. That is the argument
        /// `docs/ir-spec.md` makes for keeping `parent` off a metadata card
        /// rather than writing it empty: a record whose producer does not
        /// exist waits for it, because an empty one is worse than none.
        /// **The record grows a per-input row when something can set one** —
        /// one line per edge, on the terms `slot` and `capacity` are one line
        /// per node — and until then `live` is here because a selection is the
        /// one input control an operator can actually reach.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        live: Option<u32>,
    },
    /// Salts the hash builtins for one node, so re-seeding changes randomness
    /// without touching anything structural.
    Seed {
        stream: Layer,
        /// **Which node of that layer**, which is what lets a salt be
        /// *assigned* rather than derived.
        ///
        /// The salt is what makes two identical grids differ in colour without
        /// being arranged to, and it used to be derived — `hash(set_salt,
        /// ordinal)` — which `docs/ir-spec.md` called provisional for one
        /// reason: reordering `--set` changed which geometry got which
        /// randomness. A derived value cannot be recorded, because there is
        /// nothing stable to record it against; an addressed one can, and this
        /// is the address. `--save-set` writes one of these per geometry now,
        /// and `--load-set` gives each back to the source its index names.
        ///
        /// Absent is node 0 on [`Record::Slot`]'s terms rather than
        /// [`Record::Param`]'s — a seed salts the node it names — so a file
        /// salting one source per layer reads as it always did and is written
        /// back unchanged.
        #[serde(default, skip_serializing_if = "is_zero")]
        index: u32,
        value: u64,
    },
    /// **Binds one node's declared input slot to another node of this Set.**
    ///
    /// A procedure declares what it takes and refuses to say where it comes
    /// from — `uses far : Geometry` names a slot the way `consumes position`
    /// names an attribute, without naming which node supplies it, because a
    /// `.kir` that named one would be coupled to one Set and would stop being a
    /// library part. This record is the other half, and it is here rather than
    /// in the `.kir` for the reason a [`Record::Slot`]'s name is: an edge
    /// belongs to the *use*, and a Set file is what a use is recorded as.
    ///
    /// **Addressed by name at both ends, which is the one record that is.**
    /// Everything else here says which node it is about with `(layer, index)`,
    /// and an edge cannot: a position moves when the list is reordered, and
    /// reordering silently changing which geometry a morph blends towards is
    /// the exact failure this record exists to end. Every node has a name
    /// whether or not one was written — a name nobody wrote is derived from the
    /// procedure where the Set is built — so both ends always resolve.
    ///
    /// A slot bound twice is refused where the Set is built rather than here,
    /// on the terms a duplicate node name is: the vocabulary's job is to carry
    /// what a file said.
    Edge {
        /// The node that declares the slot.
        node: String,
        /// What that node's procedure calls it.
        slot: String,
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
    // thirteen describe the deck the Sets are playing on, and a Set file must
    // not contain them — a Set does not know what fader it is under or whether
    // it is on air, and one that carried its gain would restore that gain
    // wherever it was next loaded. They are state all the same, which is what
    // separates them from the three below: `is_set_state` says no to all
    // sixteen of them and means two different things by it. (It says no to the
    // four metadata records further down as well, for a third reason that is
    // not about state at all — see the group comment above `Record::Meta`; the
    // count here is these sixteen and not that twenty. It read "seven" and
    // "twelve" from the commit that gave the mix a vocabulary until this one:
    // every record added since went in without the count moving, because a
    // prose count is not checked by anything. Both are counted off the
    // variants above, and the classification behind `is_set_state` is an
    // exhaustive match now, so that at least the *classification* cannot drift
    // the same way silently.)
    /// A deck slot's linear gain into the mix.
    ///
    /// The slot is an index into the deck rather than anything about the Set
    /// in it. Moving a Set to another slot moves it under another fader,
    /// which is what a fader is.
    Gain {
        slot: u8,
        value: f32,
    },
    /// A deck slot's fader: how much of its blend lands in the mix, `[0, 1]`.
    ///
    /// **Separate from [`Record::Gain`] because they are separate controls**,
    /// which only became visible once there was a blend mode that was not
    /// `add`. Gain is the level the material arrives at; opacity is how much of
    /// the blend takes effect, including how much the layer covers. Under `add`
    /// the two multiply together and a stream could have carried either — under
    /// `over` a session that replayed one as the other would replay a layer
    /// that hides as a layer that dims.
    Opacity {
        slot: u8,
        value: f32,
    },
    /// How a deck slot's layer meets the ones under it: `add`, `over` or `max`.
    ///
    /// Slot order is stacking order, so this is the one mix control whose
    /// meaning depends on where the slot sits — which is why it is recorded per
    /// slot rather than as a property of the Set in it. Moving a Set to another
    /// slot moves it to another place in the stack.
    ///
    /// A `String` on the same terms as `residency` and `sync`: what a mode is
    /// allowed to be is the engine's to say, and a stream from a newer build
    /// reaches the engine's diagnostic rather than the parser.
    Blend {
        slot: u8,
        mode: String,
    },
    /// What a deck slot is asked to do: `live`, `priming`, or `allocated`.
    ///
    /// **The request, never the effective level.** The governor recomputes the
    /// second every pass from the budget of the machine that is running, so a
    /// session recorded on a fast machine and replayed on a slow one must
    /// re-derive it rather than replay it — recording what was decided would
    /// replay one machine's budget onto another's. This is the same choice
    /// [`Record::Tempo`] makes in the other direction and for the opposite
    /// reason: there, what was decided is the reproducible thing.
    ///
    /// A `String` rather than an enum, on the same terms as `curve` and
    /// `noise.kind`: an unrecognised level is the engine's to diagnose against
    /// what it actually supports. This one has earned it — `docs/roadmap.md`
    /// planned five residency levels and three were built.
    Residency {
        slot: u8,
        level: String,
    },
    /// The output look: tone map operator, exposure, and the operator's white
    /// point. One record rather than three because it is one value in the
    /// engine, written to one uniform, and a stream that could set the
    /// exposure without saying which operator it applies to would be
    /// describing a look nobody can reconstruct.
    ///
    /// Session-wide and deliberately not per slot: tone mapping happens once,
    /// after the mix, which is the whole argument in `karakuri-engine`'s
    /// `present` module.
    Look {
        op: String,
        exposure: f32,
        white_point: f32,
    },
    /// **The procedure a deck slot is playing, from this moment on.**
    ///
    /// Written when a hot swap lands and when one is rolled back — the two
    /// moments the material a session is playing actually changes. Without it a
    /// session recorded the material *once*, before the first frame, and
    /// replayed the whole run with whatever it started with: a set in which a
    /// procedure was rewritten at minute ten replayed as though it never had.
    ///
    /// **The session's, not the Set's**, which is the distinction `is_set_state`
    /// exists for. A Set file's `slot` record says what a Set *is*; this says
    /// what a deck slot *became*, at a point in time, which is a fact about a
    /// performance.
    ///
    /// `proc` is a content address and the source is in the store, on the same
    /// terms a Set file's `slot` record uses — so a rewrite costs one line here
    /// and a few kilobytes once, however many times the same procedure comes
    /// back.
    ///
    /// **One thing it cannot carry**: a rollback restores the outgoing Set at
    /// the `t` it was parked at, and a replay meeting this record builds afresh
    /// from the source, so `t` restarts there. A swap-*in* is documented to
    /// start cold and so replays exactly; only the rollback differs, and a
    /// rollback means the candidate was over budget, which is already an
    /// exceptional frame.
    Procedure {
        slot: u8,
        layer: Layer,
        /// **Which node of that layer**, when a slot has more than one.
        ///
        /// A slot draws with one L1 and however many L4s — several renderers
        /// over one geometry, in draw order — so naming a layer is no longer
        /// enough to name a procedure. Zero for the L1 and for the first
        /// renderer, which is every stream written before stacks existed:
        /// absent means zero and zero is not written, so an old stream replays
        /// byte for byte and a new one adds a field only where it says
        /// something.
        #[serde(default, skip_serializing_if = "is_zero")]
        index: u32,
        #[serde(rename = "proc")]
        proc_hash: Hash,
    },
    /// **What size the session renders at**, in texels — the canvas every
    /// `VideoSource` draws into and every deck slot is sized to match.
    ///
    /// Not the size of any window. A window is a preview and is fitted to this
    /// rather than the other way round, so dragging one changes what an
    /// operator can see and nothing about what is drawn. Without this record
    /// the two were the same number: a session played in a small window and
    /// replayed with a large `--size` rendered different pixels, and nothing in
    /// the stream said which of them was the performance.
    ///
    /// **Written once, at the head, and a stream carries no second one.**
    /// Changing it reallocates every slot's target and the HDR target, which is
    /// an allocation on the render thread — the one thing this engine's frame
    /// path forbids. So the canvas is a property of a run rather than a control
    /// an operator moves during one, and this is the only record here that is
    /// session state without being something a hand can reach mid-set.
    Canvas {
        width: u32,
        height: u32,
    },
    /// **What shape of the frame a deck slot's layer reaches.**
    ///
    /// A mask multiplies the layer's opacity per texel, which is what makes it
    /// a mask rather than a second fader — everything opacity does, done to
    /// part of the frame. `position` is how far the front has travelled and is
    /// the number a `transition` moves, so **a wipe is this record plus a
    /// `transition` on `mask`** and needs nothing of its own.
    ///
    /// `kind` is a `String` on the same terms as `blend` and `residency`: what
    /// a shape is allowed to be is the engine's to say. `angle` is in radians
    /// and is the linear front's alone.
    Mask {
        slot: u8,
        /// `none`, `linear` or `radial`.
        kind: String,
        /// Which way a linear front runs, in radians.
        angle: f32,
        /// How far it has travelled, `[0, 1]`. 0 reveals nothing, 1 reveals
        /// everything — both exactly.
        position: f32,
        /// How wide the soft edge is, `[0, 1]`. 0 is a hard edge.
        softness: f32,
    },
    /// **A mix control moving over musical time**: a fade, a cut, or half of a
    /// crossfade.
    ///
    /// One record for the whole move, and **the values it produces are not
    /// recorded at all**. A value per frame would be 216,000 lines an hour
    /// describing something the grid already determines — the same argument
    /// `tick` makes, where the engine advances by a step count and everything
    /// downstream is a function of it.
    ///
    /// `start` is an absolute position on the session's beat count rather than
    /// "in two bars", because a relative instant is a different instant
    /// depending on when it is read and a beat count is the same one on every
    /// run. Quantising to the next bar happens where the operator asked, once.
    ///
    /// **`from` is deliberately absent**, and is read where the move is
    /// *scheduled* rather than where it starts. Capturing it at the start would
    /// mean capturing it on the first frame at or after a musical instant, and
    /// a machine running at a different rate would capture it at a different
    /// beat — which is the one property this record exists to have. Nothing can
    /// move the control in between: a hand cancels the move, another move
    /// replaces it.
    ///
    /// `control` and `curve` are strings on the same terms as `curve` on a
    /// `bind`: what a name is allowed to be is the engine's to say.
    Transition {
        slot: u8,
        /// `gain` or `opacity`.
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
    /// **Which renderer of a composited deck slot is the live one**, from a
    /// musical instant on.
    ///
    /// A record of its own rather than a `control` on [`Record::Transition`],
    /// and the engine's `transition::Control` already carries the reason: the
    /// things a transition moves are **positions**, and this is a **choice**.
    /// "Half way to renderer 2" does not name a picture, which is why there is
    /// no `beats` and no `curve` here — a selection is a cut, and a cut is a
    /// fade of zero beats with nothing left for a curve to shape. The other
    /// half of the argument is the address: every `transition` is about one
    /// `(slot, control)` pair and a selection is about one renderer *inside*
    /// the Set a slot is playing, which is a second index no control carries.
    ///
    /// **`start` alone, on `transition`'s terms**: an absolute position on the
    /// session's beat count, because "on the next bar" is a different instant
    /// depending on when it is read. Quantising happens once, where the
    /// operator asked.
    ///
    /// **What it selects is an edge into the Set's L5**, so it says something
    /// only where the slot was built to composite — `--merge N`. A slot that
    /// overdraws has no L5 for an edge to go into, and this record is then
    /// carried, replayed and without effect, exactly as `Set::set_input` is:
    /// refusing it would make a replay fail on a line that describes a
    /// performance that happened.
    ///
    /// **It is still not folded into a Set file, and the reason has moved.**
    /// The layering *is* a Set file record now — [`Record::Merge`], whose
    /// `live` field holds exactly the choice this one makes — so a composited
    /// Set no longer loads back as overdraw with nothing for a selection to be
    /// about. What no *session* record carries is the layering: nothing in a
    /// stream says that the Set in slot 3 composites, and nothing says which
    /// deck slot the Set at the head of a stream was played in, so a
    /// projection folding a session down cannot tell whether a selection it
    /// meets is about the Set it is writing. A selection stays a property of
    /// the *run*, like a gain, until a session record says otherwise. See
    /// `project::key_for`, and `docs/manual.md`, "Selecting one renderer of a
    /// slot".
    Select {
        slot: u8,
        /// Which renderer of that slot, in draw order — the same numbering
        /// `--param L4:1:name=value` and a `slot` record's `index` use.
        ///
        /// **No wildcard and no `Option`.** Absent is not "every renderer": a
        /// record that named none of them would be the un-selection this build
        /// does not have — see the manual for why it is one-way — and a reader
        /// meeting an absent field would have to guess which of the two it
        /// meant.
        renderer: u32,
        /// The musical instant it lands on, in beats.
        start: f64,
    },
    /// **Which slot is being auditioned**, or none of them for the mix.
    ///
    /// Not a mix control — it changes nothing about how the Sets are combined,
    /// only which of them the output is showing — but it is session state and it
    /// is in the stream for one reason: **today the preview is the output**.
    /// A replay that ignored it would show the mix where the operator was
    /// looking at one slot, which is replaying a different picture than the one
    /// that happened.
    ///
    /// That reason has an expiry date. When output routing gives the deck a
    /// second output, this becomes the monitor's choice and stops being the
    /// programme's, and the record stops belonging in a session stream — where
    /// `gain` and `blend` will still belong. Recorded here as a fact about what
    /// was shown, not as a claim that auditioning is part of a performance.
    ///
    /// `slot` is `None` for the mix rather than a sentinel index, so a deck of
    /// a different size cannot read one as the other.
    Preview {
        slot: Option<u8>,
    },
    /// **What a deck slot's clock does with the session's** — `free`, `tempo`
    /// or `beat`, with the two numbers that make the mode mean something.
    ///
    /// `anchor_bpm` is the tempo at which this material runs at 1x, and it has
    /// to be recorded because **material has no intrinsic tempo**: a `.kir`
    /// declares parameters and a capacity, not a bar length, so "one beat of
    /// music is how many seconds of material" is an operator's answer rather
    /// than the artifact's. `offset_beats` is the scrub — signed, unbounded,
    /// and the one value in this format that is meant to go backwards.
    ///
    /// Both are carried even under `free`, where neither does anything, so that
    /// a slot moved back onto the grid returns to where the operator left it
    /// rather than to a default.
    Transport {
        slot: u8,
        sync: String,
        anchor_bpm: f32,
        offset_beats: f64,
    },
    /// **A deck slot's material was written out as a Set file**, under `id`.
    ///
    /// The first record in this vocabulary whose subject is **outside the
    /// stream**. Everything else here describes the deck, and a reader that
    /// obeys it reproduces the performance; this one describes a file that was
    /// created in a store, and a reader that obeyed it would create a file
    /// instead. So a replay does not perform it and says which id it passed
    /// over — see `docs/ir-spec.md`, "Records with an effect outside the
    /// stream".
    ///
    /// **The slot and the id, and deliberately not the node hashes.** The Set
    /// file under that id already names every node it holds, and a copy of them
    /// here would be a second place for one fact — free to be right on the day
    /// it was written and wrong the moment the two are read apart. What this
    /// record is for is saying *that* a save happened and *what it is called*;
    /// what was saved is a question the Set file answers.
    ///
    /// **Written at the frame the save landed, not at the key press.** The
    /// store write is off the render thread, so it finishes some frames later
    /// and can fail; a record written at the press would claim a file that the
    /// disk went on to refuse. This is the same rule [`Record::Procedure`]
    /// follows — a record that describes a change already made.
    Save {
        slot: u8,
        /// What the Set file is called in the store: `sets/<id>.set.ndjson`.
        /// A `String` because it is a name somebody chose, and the only thing
        /// in this record that can be looked up.
        id: String,
    },
    // -- What a frame saw or decided --------------------------------------
    /// How far this frame advances. Emitted from real time when live, read back
    /// verbatim on replay — which is what keeps substepping deterministic.
    /// Always 1 in v0.2, capped at [`MAX_STEPS`].
    Tick {
        steps: u8,
    },
    /// One frame's worth of **measured** signals, on the same terms as
    /// [`Record::Tick`]: derived from a device when live, read back verbatim on
    /// replay, and the engine cannot tell which happened.
    ///
    /// Live audio is not reproducible and the record stream is required to be,
    /// so the two can only both be true if the measurement joins the stream.
    /// This is that record, and its shape follows from what a frame is:
    ///
    /// - **One line per frame**, so it interleaves with `tick` and every edit
    ///   lands at an exact frame position, the way the session stream format
    ///   already promises.
    /// - **Named fields for named signals** and a positional array for the
    ///   bands, because `band0`…`bandN` *are* positions — an array is the
    ///   naming scheme rather than a second one. A map of name to value would
    ///   also encode which names it carries, at the cost of repeating those
    ///   names 200,000 times an hour and of letting a stream invent a name the
    ///   bus has rules about (`noise` is not a bus name, and a generic map is
    ///   where that rule would be broken).
    /// - **One confidence for the frame**, not one per signal: these values all
    ///   came out of the same block of samples at the same instant, so their
    ///   staleness is one number. A per-signal confidence would be four copies
    ///   of it.
    ///
    /// The array's length is the band count, so a stream carrying more bands
    /// than a reader knows about still decodes — a fixed-length array would
    /// make a band count a breaking format change, which is the opposite of
    /// what the unknown-`t` rule is for.
    Audio {
        /// Broadband level, `[0, 1]`.
        energy: f32,
        /// Transient envelope, `[0, 1]`: 1.0 at a detected onset, decaying from
        /// there. Not an impulse — an impulse one analysis block wide would be
        /// missed by some frames and seen twice by others.
        onset: f32,
        /// Per-band level, `[0, 1]`, low band first. Position is the name.
        bands: Vec<f32>,
        /// How much of this frame to believe, `[0, 1]`. Full while a device is
        /// open and delivering — **a silent room is 0.0 energy at confidence
        /// 1.0** — falling as the last block goes stale, and 0.0 when nothing
        /// has arrived, which leaves every bound parameter at its own value.
        confidence: f32,
    },
    /// What the local oscillator's tempo and phase are corrected to, this
    /// frame. The same terms as [`Record::Tick`] and [`Record::Audio`]: derived
    /// live, read back verbatim on replay.
    ///
    /// **v0.2 had no tempo record at all**, which `docs/roadmap.md` notes: the
    /// session tempo arrived by CLI flag and nothing in the stream could say
    /// what it was. This closes that, and it closes it with the *correction*
    /// rather than with the estimate, for a reason worth stating: an analyser
    /// is allowed to improve, and a session recorded today has to replay the
    /// same way after it does. Recording what was decided rather than what was
    /// heard is what makes that true. It is also why replay does not need the
    /// audio.
    ///
    /// The first one in a stream is what sets the session tempo, so this record
    /// is both "the tempo is now this" and "the tempo has moved a little"; a
    /// correction with `shift` 0.0 and `confidence` 0.0 is a free-running
    /// tempo being stated.
    Tempo {
        /// The tempo from now on. A tempo change does not move a beat that has
        /// already happened — see `Oscillator::correct`.
        bpm: f32,
        /// Phase shift in beats, positive meaning the next beat arrives sooner.
        /// Almost always tiny: the grid is predicted and trimmed, not chased.
        shift: f32,
        /// How much the estimate behind this correction was believed. Carried
        /// so a replay can show an operator what the live run showed, and
        /// because a correction that was applied at low confidence is a
        /// different event from the same numbers applied at high confidence.
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
    // against [`Record::Preview`] above, which is the deck's and carries a
    // `slot` — and sharing one enum made that collision *silent*, because the
    // deck record's `slot` is an `Option` and the specified line decoded as
    // `Preview { slot: None }` with `path` dropped. It is `thumbnail` now.
    // Renaming the deck record instead was the alternative and is worse: it is
    // written into session streams that exist on disk, so moving it would break
    // reading them, where nothing has ever written the metadata one. A suffix —
    // `preview_path` — was the other, and it would say these are two versions
    // of one concept, which is what `param_decl` beside `param` legitimately is
    // and what a stored asset beside a live audition is not.
    //
    // Four of the nine records the specification lists, because four is what a
    // compile pass can produce. `perf` is a measurement and wants a probe;
    // `origin`, `parent` and `tag` have no producer until something generates
    // procedures, and `thumbnail` names a stored asset nothing renders yet.
    // Writing an empty `parent` would be worse than leaving room for it — see
    // `docs/ir-spec.md`, "Metadata file format".
    /// **The head of a metadata file**: which artifact this describes, and what
    /// it calls itself.
    ///
    /// `hash` is the content address of the `.kir` the rest of the file was
    /// read off, and it is written down rather than left implicit in the file
    /// name: a metadata file copied, renamed or quoted out of context still
    /// says what it is about, and a reader can tell a stale card from a current
    /// one without trusting a path.
    ///
    /// `name` is the procedure's own declared name — the `.kir`'s `proc <name>`
    /// — and never a name a Set gave a node, which belongs to the *use* and is
    /// recorded in a Set file's [`Record::Slot`].
    Meta {
        hash: Hash,
        name: String,
        kind: Layer,
        v: u32,
    },
    /// **One parameter a procedure declares**: its range and, where the
    /// declaration is a number this build can state, its default.
    ///
    /// A declaration and not a value, which is the whole of what separates it
    /// from [`Record::Param`] — that one says what a Set turned a knob to, this
    /// one says the knob exists and what it may be turned between.
    ParamDecl {
        key: String,
        /// The declared type's spelling, as `.kir` writes it: `float`, `vec3`.
        /// A `String` rather than an enum because the metadata file is read by
        /// a decoder that does not parse `.kir` and has no use for a closed set
        /// — and because a type the language grows is then a value this crate
        /// carries without a release.
        #[serde(rename = "type")]
        ty: String,
        min: f32,
        max: f32,
        /// **Absent means the default is not a number this build can state**,
        /// never that there is no default: `param <name> : <ty> [<min>, <max>]
        /// = <expr>` makes the expression mandatory, so every declared param
        /// has one. `karakuri_ir::Param::default_scalar` folds a literal and a
        /// leading negation and nothing else.
        ///
        /// **The record is still written, which is the decision.** Dropping it
        /// instead would make a reader believe the artifact declares no such
        /// param, and that is a false statement about the declaration where a
        /// missing key is a true statement about what is known. It also has to
        /// survive the unknown-key rule: a decoder that ignores keys it does
        /// not recognise cannot tell an absent `default` from one it skipped,
        /// and under either reading the answer is *"not known"* — which is the
        /// answer. Under the other choice the two readings differ, because a
        /// record that is not there cannot be skipped into existence.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default: Option<f32>,
    },
    /// **The element count an L1 declares it can run between**, and what it runs
    /// at when nothing says otherwise.
    ///
    /// One per file at most: `capacity` is a header declaration and only an L1
    /// has one. No record at all is a procedure that declares none — every kind
    /// but L1, and an L1 written before the declaration existed.
    CapacityDecl {
        min: u32,
        max: u32,
        default: u32,
    },
    /// **The attributes a procedure writes**, in declaration order.
    ///
    /// Absent rather than empty where a procedure declares no `emit` — an L4
    /// emits nothing, and a record saying so tells a reader exactly what its
    /// absence tells them. The rule across this group is one record per
    /// declaration, and a declaration nobody wrote produces none.
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

/// **Which of the three files' vocabularies a record belongs to**, decided in
/// one exhaustive match that both of the questions below are read off.
///
/// **One classification and two questions, because the two drifted apart
/// once.** [`Record::is_set_state`] and [`Record::is_metadata`] ask different
/// things — *may this go in a Set file* and *does this belong in a card* — and
/// each owes an operator a different sentence, which is why there are two of
/// them. What there is not is two answers: the four metadata variants were
/// added to the enum and classified in neither function, and because
/// `is_set_state` was a `matches!` over the exceptions it answered *true* for
/// all four, under a headline saying they belong in a Set file.
///
/// Two exhaustive matches would have caught that and would still leave the next
/// record classifiable one way here and another way there. Classified once,
/// they cannot disagree, and a variant with no arm does not compile —
/// `origin`, `parent`, `perf`, `tag` and `thumbnail` are specified and will
/// arrive.
enum Vocabulary {
    /// What a Set *is*: written to a Set file and read back out of one.
    Set,
    /// Not state at all — what a *frame* saw or decided.
    Frame,
    /// The **session's** state rather than any Set's: the deck the Sets are
    /// playing on.
    Session,
    /// What an *artifact* declares, in its `<hash>.meta.ndjson`.
    Metadata,
    /// A `t` this build does not know. **Not a vocabulary but the absence of
    /// one**, and it is a case of its own because every reader treats it as a
    /// line to carry rather than a line to place: passing it over is the
    /// format's promise, so a Set file round-tripped through a build that does
    /// not know every record in it comes back with all of them.
    Unknown,
}

impl Record {
    /// Whether this record belongs in a Set file. **Twenty say no, for three
    /// different reasons, and keeping them apart is the point of the name** —
    /// it is `is_set_state` rather than `is_state` because most of what it
    /// refuses is state.
    ///
    /// - [`Record::Tick`], [`Record::Audio`] and [`Record::Tempo`] are not
    ///   state at all: they are what a *frame* saw or decided. A Set file
    ///   carries no time, and one holding an audio frame would be claiming a
    ///   particular moment's microphone reading is part of what a Set is.
    /// - [`Record::Gain`], [`Record::Opacity`], [`Record::Blend`],
    ///   [`Record::Residency`], [`Record::Look`], [`Record::Canvas`],
    ///   [`Record::Procedure`],
    ///   [`Record::Transport`],
    ///   [`Record::Preview`], [`Record::Mask`], [`Record::Transition`] and
    ///   [`Record::Select`] are
    ///   the **session's**
    ///   rather than any Set's. The last two are the ones that are not durable
    ///   state at all but *events* — a move and a choice, each scheduled at an
    ///   instant — and they are
    ///   here rather than beside `tick` because what they move is the deck.
    ///   Folding a session down to a Set file drops them for both reasons at
    ///   once. A Set does
    ///   not know what fader it is under; one that carried its gain would
    ///   restore that gain wherever it was next loaded, which is a Set file
    ///   reaching outside the Set.
    ///
    ///   [`Record::Canvas`] is in this group for a reason worth stating apart:
    ///   a Set renders at whatever size it is given, and one that carried a
    ///   canvas would make loading it resize every *other* Set in the deck.
    ///
    ///   [`Record::Save`] joins that second group: it says a deck slot's
    ///   material was written out, which is a fact about a performance and
    ///   about no Set. A Set file carrying one would claim, every time it was
    ///   loaded, that a save had just happened.
    ///
    ///   [`Record::Select`] says which renderer of a slot is live, which is a
    ///   fact about a run — and it used to be in this group twice over,
    ///   because the layering that makes the question mean anything was not in
    ///   a Set file at all. It is now: [`Record::Merge`] records it, and its
    ///   `live` field is where a *Set* says which renderer is the live one.
    ///   What remains is the first reason and it is enough — a selection is
    ///   addressed to a deck slot and scheduled at an instant, and no session
    ///   record says which slot's Set composites or which slot a folded Set
    ///   was played in.
    ///
    /// - [`Record::Meta`], [`Record::ParamDecl`], [`Record::CapacityDecl`] and
    ///   [`Record::Emit`] are a **third file's** vocabulary: what an artifact
    ///   *declares*, before anything has instantiated it. Not the session's and
    ///   not any Set's, which is why they are a third reason rather than a
    ///   longer second one.
    ///
    /// **Three reasons for one answer, and `Record::Save` still does not add a
    /// fourth.** The question here is *may this line go in a Set file*, and it
    /// has one answer per record however many reasons stand behind a no.
    /// Whether a record **reaches outside the stream** is a different question
    /// about the same vocabulary, and `Record::Save` is so far the only record
    /// for which the answer is yes; folding that in would give one function two
    /// jobs. It lives in `docs/ir-spec.md` under "Records with an effect
    /// outside the stream", where a replay reads it.
    ///
    /// A session stream carries the sixteen of the first two groups and none of
    /// the third. That is the difference between the two files, stated from
    /// this side, and it is what [`Record::is_metadata`] is a separate function
    /// for: `Store::write_set` refuses a `param_decl` through *that* question so
    /// that the sentence it prints is about declarations. `!is_set_state()`
    /// refuses it too — the third bullet is what that means — but the refusal it
    /// reaches is the one naming a tick, and telling an operator a
    /// `capacity_decl` was rejected for carrying time sends them looking in the
    /// wrong place. Two questions, two sentences, one classification.
    ///
    /// **What this said before, because an inverted reason outlives the code it
    /// was written about.** The four metadata variants were added to the enum
    /// and not to this function, which was a `matches!` over the exceptions —
    /// so `is_set_state()` answered *true* for all four, under a headline that
    /// says they belong in a Set file. The paragraph here claimed a bare
    /// `!is_set_state()` "would refuse them for the wrong reason": it would not
    /// have refused them at all, it would have written them to disk, and only
    /// `Store::write_set` asking [`Record::is_metadata`] first kept that from
    /// happening. Neither question is answered by hand any more — both are read
    /// off [`Vocabulary`], whose match is exhaustive — so the next record
    /// cannot arrive the same way, and cannot be classified one way here and
    /// another way there. `origin`, `parent`, `perf`, `tag` and `thumbnail` are
    /// specified and will arrive; `project::key_for` and `setfile::from_lines`
    /// already stop compiling until somebody classifies them, and this does
    /// too now.
    ///
    /// **That is a claim about the readers that have to place a record, and
    /// not about every match on `Record` in the workspace.** `karakuri-cli`'s
    /// `mix::change` ends with `_ => Ok(None)`, so a new variant silently
    /// becomes "not a mix change" there. It is deliberate and it is the safe
    /// default — that function's whole contract is that a record it does not
    /// act on is not an error, so that a stream from a newer build replays
    /// rather than failing — but it means a new *deck* record can arrive,
    /// be classified `Session` here, and still go unacted on with nothing
    /// saying so. The exhaustive matches are the ones that decide which file a
    /// record belongs in; the wildcard is in the one that decides what to do
    /// with it.
    ///
    /// **[`Record::Unknown`] is the one `true` that is not a claim about
    /// state.**
    /// An unrecognised `t` is passed over rather than refused, which is the
    /// format's promise and the reason `project::key_for` gives one its own
    /// passthrough key. Answering false for it would make `Store::write_set`
    /// reject a file it had just read.
    pub fn is_set_state(&self) -> bool {
        matches!(self.vocabulary(), Vocabulary::Set | Vocabulary::Unknown)
    }

    /// Which file's vocabulary this record is part of. **The one place any
    /// record is classified** — see [`Vocabulary`] for why it is one place.
    fn vocabulary(&self) -> Vocabulary {
        match self {
            // Written to a Set file and read back out of one, and this is the
            // arm that grows when the Set vocabulary does.
            Record::Set { .. }
            | Record::Slot { .. }
            | Record::Capacity { .. }
            | Record::Param { .. }
            | Record::Bind { .. }
            | Record::Camera { .. }
            | Record::Merge { .. }
            | Record::Seed { .. }
            | Record::Edge { .. }
            | Record::Src { .. } => Vocabulary::Set,
            Record::Tick { .. } | Record::Audio { .. } | Record::Tempo { .. } => Vocabulary::Frame,
            Record::Gain { .. }
            | Record::Opacity { .. }
            | Record::Blend { .. }
            | Record::Residency { .. }
            | Record::Look { .. }
            | Record::Canvas { .. }
            | Record::Procedure { .. }
            | Record::Transport { .. }
            | Record::Preview { .. }
            | Record::Transition { .. }
            | Record::Select { .. }
            | Record::Mask { .. }
            | Record::Save { .. } => Vocabulary::Session,
            // The arm `is_set_state` was missing when it was a `matches!` over
            // the exceptions: these were added to the enum without being
            // classified anywhere, and nothing said so.
            Record::Meta { .. }
            | Record::ParamDecl { .. }
            | Record::CapacityDecl { .. }
            | Record::Emit { .. } => Vocabulary::Metadata,
            Record::Unknown => Vocabulary::Unknown,
        }
    }

    /// **Whether this record belongs in an artifact's `<hash>.meta.ndjson`.**
    /// Four say yes, and they are the four a compile pass can produce.
    ///
    /// A separate question from [`Record::is_set_state`], which asks *may this
    /// go in a Set file* and answers no to these as well. Two functions because
    /// two **sentences** are owed, not because the classification is in doubt:
    /// `Store::write_set` asks this one first so that a rejected `param_decl`
    /// is told it is a declaration, rather than told it carries time. One
    /// classification behind both, [`Vocabulary`], because the two drifted
    /// apart once already — see the head of [`Record::is_set_state`].
    ///
    /// The specification lists five more — `origin`, `parent`, `perf`, `tag`
    /// and `thumbnail` — and nothing produces any of them yet. When one arrives
    /// it joins [`Vocabulary::Metadata`]'s arm and this doc's count moves with
    /// it; it cannot be forgotten on the way, because a variant with no arm
    /// does not compile.
    ///
    /// **`thumbnail` is the fifth name because `preview` was taken**, by
    /// [`Record::Preview`] — the deck's, for which slot is being auditioned.
    /// The specification called the library asset `preview` too, and one `t`
    /// cannot carry two shapes: the deck record's `slot` is an `Option`, so the
    /// specified line decoded as `Preview { slot: None }` and lost its `path`
    /// without a word. The metadata name moved rather than the deck's, which is
    /// written into session streams that exist on disk. Nothing here has a
    /// `Thumbnail` variant, because nothing renders one yet — it joins
    /// `origin`, `parent`, `perf` and `tag` on the list of records the
    /// specification describes and nothing writes.
    pub fn is_metadata(&self) -> bool {
        matches!(self.vocabulary(), Vocabulary::Metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The wire line the spec documents, parsed and written back.** Every
    /// other record has one of these; without it a rename or a reordered field
    /// breaks every recorded session and nothing says so.
    #[test]
    fn a_transition_round_trips_through_the_line_the_spec_prints() {
        let line = r#"{"t":"transition","slot":0,"control":"opacity","to":0.0,"start":64.0,"beats":8.0,"curve":"smooth"}"#;
        let rec = round_trip(line);
        assert_eq!(
            rec,
            Record::Transition {
                slot: 0,
                control: "opacity".to_string(),
                to: 0.0,
                start: 64.0,
                beats: 8.0,
                curve: "smooth".to_string(),
            }
        );
        // And it is the session's rather than a Set's, for both reasons at
        // once: it is the deck's, and it is an event rather than state.
        assert!(!rec.is_set_state());
    }

    /// **A selection round-trips through the line the spec prints**, bytes and
    /// all.
    ///
    /// `round_trip_verbatim` rather than `round_trip`, because the whole shape
    /// of this record is what it does *not* carry: a `beats` or a `curve`
    /// added later with a serde default would still compare equal to itself
    /// and would change every session ever recorded. A selection is a cut and
    /// has neither — see [`Record::Select`].
    #[test]
    fn a_selection_round_trips_through_the_line_the_spec_prints() {
        let line = r#"{"t":"select","slot":1,"renderer":2,"start":64.0}"#;
        let rec = round_trip_verbatim(line);
        assert_eq!(
            rec,
            Record::Select {
                slot: 1,
                renderer: 2,
                start: 64.0,
            }
        );
        // The session's, on both of the counts `is_set_state` separates: it is
        // the deck's, and it is an event rather than state. A Set file that
        // carried one would also be claiming a layering it cannot record.
        assert!(!rec.is_set_state());
        assert!(!rec.is_metadata());
    }

    /// **A merge round-trips through the line the spec prints**, bytes and all,
    /// with and without the one field it has.
    ///
    /// `round_trip_verbatim` rather than `round_trip`, because the shape of
    /// this record is mostly what it does *not* carry. A `gain`, an `opacity`,
    /// a `blend` or a `mask` added later with a serde default would compare
    /// equal to itself and would rewrite every composited Set ever saved — and
    /// the bare `{"t":"merge"}` is the line a Set nobody has selected in
    /// writes, so it is the one that has to survive untouched.
    #[test]
    fn a_merge_round_trips_and_an_absent_live_stays_absent() {
        // A composited Set nobody has selected in: every input live, which is
        // `mix::Input::default()`, so there is nothing for `live` to say.
        let rec = round_trip_verbatim(r#"{"t":"merge"}"#);
        assert_eq!(
            rec,
            Record::Merge { live: None },
            "the presence of the record is the whole statement"
        );

        // And a variant pool with a renderer chosen in it, which is the fact
        // `Record::Select` could not leave behind before this record existed.
        assert_eq!(
            round_trip_verbatim(r#"{"t":"merge","live":1}"#),
            Record::Merge { live: Some(1) }
        );

        // Renderer 0 is a choice somebody made and is written, where an absent
        // `live` is nobody having chosen. The two are different pictures — one
        // renderer against all of them — so the field must not be skipped at
        // zero the way an `index` is.
        assert_eq!(
            round_trip_verbatim(r#"{"t":"merge","live":0}"#),
            Record::Merge { live: Some(0) }
        );
    }

    /// **A merge is what a Set *is*, and belongs in a Set file.**
    ///
    /// The assertion this record exists to make. A composited Set saved and
    /// loaded back overdrew, because the layering was the one thing a Set knew
    /// about itself that the file could not say — so `Set::select_renderer`
    /// came back with nothing to select between and a saved variant pool was
    /// an unselectable one.
    #[test]
    fn a_merge_is_set_state_and_not_an_artifacts_declaration() {
        let rec = Record::Merge { live: Some(2) };
        assert!(
            rec.is_set_state(),
            "the layering is what a Set is, so `Store::write_set` has to take it"
        );
        // And it is not a card's: nothing about it is something a compile pass
        // reads off a `.kir`. An L5 has no procedure to compile.
        assert!(!rec.is_metadata());
    }

    /// The wire line the spec prints, parsed and written back.
    #[test]
    fn a_mask_round_trips_through_the_line_the_spec_prints() {
        let line =
            r#"{"t":"mask","slot":1,"kind":"linear","angle":0.0,"position":0.5,"softness":0.1}"#;
        let rec = round_trip(line);
        assert_eq!(
            rec,
            Record::Mask {
                slot: 1,
                kind: "linear".to_string(),
                angle: 0.0,
                position: 0.5,
                softness: 0.1,
            }
        );
        assert!(!rec.is_set_state());
    }

    /// The wire line the spec prints, parsed and written back.
    #[test]
    fn a_canvas_round_trips_through_the_line_the_spec_prints() {
        let line = r#"{"t":"canvas","width":1920,"height":1080}"#;
        let rec = round_trip(line);
        assert_eq!(
            rec,
            Record::Canvas {
                width: 1920,
                height: 1080,
            }
        );
        // **The one that would be most tempting to put in a Set file**, and the
        // one it would do the most damage in: a Set renders at whatever size it
        // is handed, so a Set file carrying a canvas would resize every *other*
        // Set in the deck by being loaded.
        assert!(!rec.is_set_state());
    }

    /// The wire line the spec prints, parsed and written back.
    ///
    /// **And the one assertion this record exists to make**: it is not Set
    /// state, so `Store::write_set` refuses it. A `save` folded into a Set file
    /// would be a file claiming, every time it was opened, that a save had just
    /// happened — which is the outside-the-stream effect a replay is defined
    /// not to perform, arriving by the one door nobody watches.
    #[test]
    fn a_save_round_trips_through_the_line_the_spec_prints() {
        let line = r#"{"t":"save","slot":1,"id":"20260816-143052-271"}"#;
        let rec = round_trip(line);
        assert_eq!(
            rec,
            Record::Save {
                slot: 1,
                id: "20260816-143052-271".to_string(),
            }
        );
        assert!(!rec.is_set_state());
    }

    fn round_trip(line: &str) -> Record {
        let rec: Record = serde_json::from_str(line).expect("parse");
        let back = serde_json::to_string(&rec).expect("serialise");
        let again: Record = serde_json::from_str(&back).expect("reparse");
        assert_eq!(rec, again);
        rec
    }

    /// The same, and **the bytes have to match too**.
    ///
    /// `round_trip` compares the parsed values, which cannot see a field that
    /// was added with a default: a record gaining `index: 0` re-serialises with
    /// `"index":0` in it and still equals itself. That is a changed stream for
    /// every session ever recorded, and the only assertion that catches it is
    /// this one. Only for records whose fields are written in declaration order
    /// with nothing optional set.
    fn round_trip_verbatim(line: &str) -> Record {
        let rec = round_trip(line);
        assert_eq!(
            serde_json::to_string(&rec).expect("serialise"),
            line,
            "the record did not come back as the bytes it went in as"
        );
        rec
    }

    /// **`procedure` names a node, and `index` is what says which.**
    ///
    /// A deck slot draws with one L1 and however many L4s, so a layer alone
    /// stopped being enough. The compatibility claim is the whole point of the
    /// field being optional: a stream recorded before stacks existed carries no
    /// `index`, must parse as index 0, and must come back **byte for byte** —
    /// `round_trip` alone would not notice `"index":0` appearing in every
    /// `procedure` line of every session ever recorded.
    #[test]
    fn a_procedure_round_trips_and_an_absent_index_stays_absent() {
        let hash = "sha256:486779000000000000000000000000000000000000000000000000000000abcd";
        let old = format!(r#"{{"t":"procedure","slot":0,"layer":"L4","proc":"{hash}"}}"#);
        let Record::Procedure { index, slot, .. } = round_trip_verbatim(&old) else {
            panic!("not a procedure");
        };
        assert_eq!((slot, index), (0, 0), "an absent index is the first node");

        let stacked =
            format!(r#"{{"t":"procedure","slot":2,"layer":"L4","index":1,"proc":"{hash}"}}"#);
        let Record::Procedure { index, slot, .. } = round_trip_verbatim(&stacked) else {
            panic!("not a procedure");
        };
        assert_eq!((slot, index), (2, 1), "the second renderer of slot 2");
    }

    /// The same for `slot`, which gained the same field for the same reason —
    /// and needs it for one more: the projection folds a Set file's records by
    /// key, so several renderers keyed by layer alone would fold to the last.
    ///
    /// **And a slot may say what the Set calls the node**, which is what a mask
    /// points at once a Set holds more than one source. A name is a cost paid
    /// only where something points, so an unnamed node is the ordinary case and
    /// has to leave the line it was absent from untouched: `"name":null` on
    /// every slot record ever written is what the second assertion refuses.
    #[test]
    fn a_slot_round_trips_and_an_absent_index_or_name_stays_absent() {
        let hash = "sha256:9c1b04000000000000000000000000000000000000000000000000000000abcd";
        let old = format!(r#"{{"t":"slot","layer":"L4","proc":"{hash}"}}"#);
        let Record::Slot { index, name, .. } = round_trip_verbatim(&old) else {
            panic!("not a slot");
        };
        assert_eq!(index, 0);
        assert_eq!(
            name, None,
            "a slot from before names existed is unnamed, not named nothing"
        );

        let stacked = format!(r#"{{"t":"slot","layer":"L4","index":2,"proc":"{hash}"}}"#);
        let Record::Slot { index, .. } = round_trip_verbatim(&stacked) else {
            panic!("not a slot");
        };
        assert_eq!(index, 2);

        // The spec's own example of a named source, verbatim — so the field
        // order is asserted here too, and a name written after `proc` would be
        // a file this reader wrote and the specification did not print.
        let named =
            format!(r#"{{"t":"slot","layer":"L1","index":1,"name":"veil","proc":"{hash}"}}"#);
        let Record::Slot { index, name, .. } = round_trip_verbatim(&named) else {
            panic!("not a slot");
        };
        assert_eq!((index, name.as_deref()), (1, Some("veil")));
    }

    #[test]
    fn tick_round_trips() {
        assert_eq!(
            round_trip(r#"{"t":"tick","steps":1}"#),
            Record::Tick { steps: 1 }
        );
    }

    /// **A capacity names a geometry, and `index` is what says which.**
    ///
    /// A Set holds more than one source, each at the default its own procedure
    /// declares, so a layer alone stopped being able to say which of them is
    /// being resized — two geometries at two capacities were inexpressible
    /// however they were spelled on the way in. The line the specification
    /// prints carries no index and is every capacity record ever written, so it
    /// has to come back **byte for byte**; `round_trip` alone would not notice
    /// `"index":0` appearing in all of them.
    #[test]
    fn a_capacity_addresses_a_geometry_and_an_absent_index_stays_absent() {
        assert_eq!(
            round_trip_verbatim(r#"{"t":"capacity","layer":"L1","value":524288}"#),
            Record::Capacity {
                layer: Layer::L1,
                index: 0,
                value: 524288
            }
        );
        // The thing that could not be said at all before: a second geometry,
        // at its own capacity, in the same file as the first.
        assert_eq!(
            round_trip_verbatim(r#"{"t":"capacity","layer":"L1","index":1,"value":65536}"#),
            Record::Capacity {
                layer: Layer::L1,
                index: 1,
                value: 65536
            }
        );
    }

    /// **A seed names the node it salts.**
    ///
    /// The salt used to be derived from `--set` order, which `docs/ir-spec.md`
    /// called provisional for exactly one reason: reordering the command line
    /// changed which geometry got which randomness. A derived value has nothing
    /// stable to be recorded against — this is the address that lets it be
    /// assigned instead, and `--save-set` now writes one of these per geometry.
    /// The line the spec prints has to survive that unchanged, which is what
    /// the absent index below is for.
    #[test]
    fn a_seed_addresses_a_source_and_an_absent_index_stays_absent() {
        assert_eq!(
            round_trip_verbatim(r#"{"t":"seed","stream":"L1","value":19274}"#),
            Record::Seed {
                stream: Layer::L1,
                index: 0,
                value: 19274
            }
        );
        assert_eq!(
            round_trip_verbatim(r#"{"t":"seed","stream":"L1","index":1,"value":4}"#),
            Record::Seed {
                stream: Layer::L1,
                index: 1,
                value: 4
            }
        );
    }

    #[test]
    fn unknown_records_are_ignored_not_rejected() {
        // Forward compatibility: a newer engine's record must not break an
        // older reader.
        assert_eq!(
            serde_json::from_str::<Record>(r#"{"t":"phrase","at":4.0}"#).unwrap(),
            Record::Unknown
        );
    }

    #[test]
    fn a_bind_without_a_noise_object_round_trips_and_stays_without_one() {
        // The field is absent rather than `null` on the way out: a Set file is
        // read by humans and generated by LLMs, and a `"noise":null` on every
        // ordinary binding teaches both that it is a thing to fill in.
        let rec = round_trip(
            r#"{"t":"bind","layer":"L1","key":"turbulence","signal":"energy","curve":"pow2","range":[0.1,2.4]}"#,
        );
        assert_eq!(
            rec,
            Record::Bind {
                layer: Layer::L1,
                index: None,
                key: "turbulence".into(),
                signal: "energy".into(),
                curve: "pow2".into(),
                range: [0.1, 2.4],
                noise: None,
            }
        );
        assert!(!serde_json::to_string(&rec).unwrap().contains("noise"));
    }

    #[test]
    fn a_noise_bind_round_trips_with_its_generator() {
        // The example out of `docs/ir-spec.md`'s "Binding noise", verbatim.
        let rec = round_trip(
            r#"{"t":"bind","layer":"L1","key":"spawn_rate","signal":"noise",
                "noise":{"kind":"perlin","rate":0.5,"stream":3},"curve":"lin","range":[4000,16000]}"#,
        );
        let Record::Bind { noise: Some(n), .. } = rec else {
            panic!("expected a bind carrying a noise generator");
        };
        assert_eq!(n.kind, "perlin");
        assert_eq!(n.rate, 0.5);
        assert_eq!(n.stream, 3);
        // Absent in the spec's own example, so it has to have a default or the
        // example does not decode.
        assert_eq!(n.octaves, 4);
    }

    #[test]
    fn an_empty_noise_object_is_the_default_generator() {
        // Field by field: a generator an LLM under-specified is one that runs,
        // not one that is refused. `{}` is the extreme case of that.
        let rec: Record = serde_json::from_str(
            r#"{"t":"bind","layer":"L1","key":"k","signal":"noise","curve":"lin","range":[0,1],"noise":{}}"#,
        )
        .expect("parse");
        let Record::Bind { noise: Some(n), .. } = rec else {
            panic!("expected a bind carrying a noise generator");
        };
        assert_eq!(n, BindNoise::default());
    }

    /// A decoded frame reproduces the values it was emitted with. That is the
    /// whole promise of putting the measurement in the stream: replay writes
    /// the same uniforms live did, so it has to be the same numbers.
    #[test]
    fn an_audio_frame_round_trips_with_every_value_it_carried() {
        let rec = round_trip(
            r#"{"t":"audio","energy":0.42,"onset":0.75,
                "bands":[0.9,0.4,0.2,0.11,0.05,0.02,0.01,0.0],"confidence":1.0}"#,
        );
        assert_eq!(
            rec,
            Record::Audio {
                energy: 0.42,
                onset: 0.75,
                bands: vec![0.9, 0.4, 0.2, 0.11, 0.05, 0.02, 0.01, 0.0],
                confidence: 1.0,
            }
        );
    }

    /// A silent room is a measurement and reads as one: zeroes at full
    /// confidence. A dead input is the same zeroes at no confidence. Two
    /// different lines, and a decoder that lost the difference would make an
    /// unplugged interface look like a quiet one.
    #[test]
    fn silence_and_absence_are_different_lines() {
        let silent = round_trip(
            r#"{"t":"audio","energy":0.0,"onset":0.0,"bands":[0.0,0.0],"confidence":1.0}"#,
        );
        let absent = round_trip(
            r#"{"t":"audio","energy":0.0,"onset":0.0,"bands":[0.0,0.0],"confidence":0.0}"#,
        );
        assert_ne!(silent, absent);
        let Record::Audio { confidence, .. } = silent else {
            panic!("expected an audio record");
        };
        assert_eq!(confidence, 1.0);
    }

    /// The band count is the array's length, so a stream from something that
    /// measures more bands than this reader knows about still decodes rather
    /// than failing — the same forward compatibility the unknown-`t` rule is
    /// for, one level down.
    #[test]
    fn a_band_count_this_reader_does_not_expect_still_decodes() {
        let rec: Record = serde_json::from_str(
            r#"{"t":"audio","energy":0.5,"onset":0.0,"bands":[0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2],"confidence":1.0}"#,
        )
        .expect("parse");
        let Record::Audio { bands, .. } = rec else {
            panic!("expected an audio record");
        };
        assert_eq!(bands.len(), 12);
    }

    /// A correction round-trips exactly, because replay applies it verbatim: a
    /// shift that decoded to a different number would put the beat somewhere
    /// else than the live run did.
    #[test]
    fn a_tempo_correction_round_trips() {
        assert_eq!(
            round_trip(r#"{"t":"tempo","bpm":128.25,"shift":-0.0125,"confidence":0.82}"#),
            Record::Tempo {
                bpm: 128.25,
                shift: -0.0125,
                confidence: 0.82,
            }
        );
        // A free-running tempo being stated: no shift, no claim.
        assert_eq!(
            round_trip(r#"{"t":"tempo","bpm":120.0,"shift":0.0,"confidence":0.0}"#),
            Record::Tempo {
                bpm: 120.0,
                shift: 0.0,
                confidence: 0.0,
            }
        );
    }

    /// The wire line the spec prints, parsed and written back.
    ///
    /// **Byte for byte, with the `hash` filled in**: the specification prints
    /// this line with the address elided — `"sha256:a3f2c1…"` — so the one
    /// value that cannot be copied off the page is the artifact's identity, and
    /// it is built here the way `karakuri-store`'s metadata tests build it. The
    /// order of the keys around it is the specification's.
    ///
    /// The head of a card is the record with the most to lose by drifting: it
    /// says *which* artifact everything below it describes, so a reordered
    /// field is a file every reader still parses and no reader can match
    /// against the `.kir` it was read off.
    #[test]
    fn a_meta_round_trips_through_the_line_the_spec_prints() {
        let hash = Hash::of(b"proc drift_shell { kind L1 }");
        let line =
            format!(r#"{{"t":"meta","hash":"{hash}","name":"drift_shell","kind":"L1","v":1}}"#);
        let rec = round_trip_verbatim(&line);
        assert_eq!(
            rec,
            Record::Meta {
                hash,
                name: "drift_shell".to_string(),
                kind: Layer::L1,
                v: 1,
            }
        );
        assert!(rec.is_metadata());
        assert!(!rec.is_set_state());
    }

    /// The wire line the spec prints, parsed and written back.
    ///
    /// **Byte for byte**, because a metadata file is written by one build and
    /// read by another: `round_trip` compares the parsed values and would not
    /// notice the keys coming back in a different order, which is a different
    /// file for every card ever regenerated. Nothing optional here, so the
    /// whole line is the record.
    #[test]
    fn a_param_decl_round_trips_through_the_line_the_spec_prints() {
        let line =
            r#"{"t":"param_decl","key":"radius","type":"float","min":0.1,"max":8.0,"default":2.0}"#;
        let rec = round_trip_verbatim(line);
        assert_eq!(
            rec,
            Record::ParamDecl {
                key: "radius".to_string(),
                ty: "float".to_string(),
                min: 0.1,
                max: 8.0,
                default: Some(2.0),
            }
        );
        // A declaration and not a value: it is the artifact's vocabulary, so a
        // Set file must not carry it and `Store::write_set` asks this.
        assert!(rec.is_metadata());
        assert!(!rec.is_set_state());
    }

    /// The wire line the spec prints, parsed and written back.
    ///
    /// **Byte for byte**, for the reason above — and this is the record with
    /// the most to lose by it: three bare numbers under three interchangeable
    /// keys, where a reordered field produces a file that still parses
    /// everywhere and says a capacity nobody declared.
    #[test]
    fn a_capacity_decl_round_trips_through_the_line_the_spec_prints() {
        let line = r#"{"t":"capacity_decl","min":65536,"max":1048576,"default":262144}"#;
        let rec = round_trip_verbatim(line);
        assert_eq!(
            rec,
            Record::CapacityDecl {
                min: 65536,
                max: 1048576,
                default: 262144,
            }
        );
        assert!(rec.is_metadata());
        assert!(!rec.is_set_state());
    }

    /// The wire line the spec prints, parsed and written back.
    ///
    /// **Byte for byte**, which for one field is the key and the order of the
    /// list inside it: the attributes are written in declaration order, so a
    /// card that reordered them would describe a struct the procedure does not
    /// write.
    #[test]
    fn an_emit_round_trips_through_the_line_the_spec_prints() {
        let line = r#"{"t":"emit","attrs":["position","velocity","age"]}"#;
        let rec = round_trip_verbatim(line);
        assert_eq!(
            rec,
            Record::Emit {
                attrs: vec![
                    "position".to_string(),
                    "velocity".to_string(),
                    "age".to_string(),
                ],
            }
        );
        assert!(rec.is_metadata());
        assert!(!rec.is_set_state());
    }

    #[test]
    fn ticks_are_not_state() {
        assert!(!Record::Tick { steps: 1 }.is_set_state());
        // Nor is anything else a frame measured or decided.
        assert!(!Record::Audio {
            energy: 0.5,
            onset: 0.0,
            bands: vec![0.1],
            confidence: 1.0
        }
        .is_set_state());
        assert!(!Record::Tempo {
            bpm: 128.0,
            shift: 0.0,
            confidence: 0.9
        }
        .is_set_state());
        assert!(Record::Set {
            id: "drift_01".into(),
            v: 1
        }
        .is_set_state());
    }
}
