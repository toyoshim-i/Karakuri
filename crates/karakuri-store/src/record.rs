//! ndjson records.
//!
//! One record per line, and concatenation is composition. The same record types
//! serve three files:
//!
//! - a **Set file** (`.kbset`) is a state projection — what is loaded and
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
//! **`slot` is three things in this vocabulary, and only the prose keeps them apart.**
//! [`Record::Slot`] is **a node of a Set** — a procedure at a `(layer, index)` address,
//! optionally with a name. The `slot: u8` field on [`Record::Gain`], [`Record::Opacity`],
//! [`Record::Blend`], [`Record::Residency`], [`Record::Procedure`], [`Record::Authority`],
//! [`Record::Ride`], [`Record::Source`], [`Record::Mask`],
//! [`Record::Transition`], [`Record::Select`], [`Record::Transport`]
//! and [`Record::Save`] is **a member of the deck** — an index into the mixer, and nothing
//! about the Set in it. [`Record::Edge`]'s `slot: String` is the third: **an input a node
//! declares**, which is what `uses far : Geometry` names.
//!
//! None of the three is renamed.
//! `docs/adr/0049-slot-means-two-things-and-the-clash-is-recorded.md` recorded the first
//! clash rather than resolving it, and the bill is the reason it stayed recorded: `slot`
//! is a field of thirteen record types across all three files, so no rename of it is a
//! rename of one field. What the prose does instead is **never write the deck one bare**:
//! it is *a deck slot*, everywhere, so
//! `docs/contributing.md` §4's *"they have to be
//! disjoint by name"* is met by the sentence where the field cannot meet it.
//!
//! **A wire field can be renamed, and one has been.** [`Record::Transport`]'s scrub was
//! spelled `offset_beats` and collided with the operator's latency offset; it is
//! `scrub_beats` now, on disk as well as in Rust, because before v1 the bill is a bill and
//! not an argument — see that variant's own documentation and
//! `docs/principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md`. What made
//! that affordable and `slot` not is the size of the bill, not a rule against the move.
//!
//! **The one collision there was is settled, and it is the metadata name that
//! moved.** `docs/ir-spec.md` listed a metadata `preview` carrying a `path`
//! beside a deck record `preview` carrying an `Option<u8>` slot — which slot
//! was being auditioned. Two shapes under one `t`, and silently so: the deck
//! record's `slot` was an `Option`, so the specified line decoded as the deck
//! record with its `path` dropped and nothing said — the one record for which
//! "an unknown `t` is ignored" protected nothing, because the `t` was not
//! unknown. The library asset is spelled `thumbnail` now, which is the word
//! the library already used for it — see
//! `docs/adr/0134-the-metadata-preview-becomes-thumbnail.md`. **The deck
//! record is gone**: `docs/adr/0240-the-output-shows-the-mix-and-residency-keys-belong-to-the-mixer.md`
//! retired the operation behind it, and switching a preview turned out to be a
//! bay-internal move rather than an engine one, so there is nothing to record.
//! The name is free and is not being reused: `thumbnail` is the word.

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
    /// **A `kind L5` procedure** — a frame effect, `[Texture] -> Texture`.
    ///
    /// One kind with two roles, and only the nested one is a Set's node: a
    /// master chain slot is one input with a surface on it and is master state
    /// rather than a Set's, which is why a Set file carries nothing for the
    /// chain. See
    /// `docs/adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md`.
    L5,
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

/// **One node of a Set, as a value rather than as two fields beside each
/// other** — the address [`Record::Ride`] carries and the only place in this
/// vocabulary a node address is a thing of its own.
///
/// **A struct so that half an address cannot be written down.** Every other
/// record here spells the address as a `layer` beside an `index`, and
/// [`Record::Param`] pays for it: `index` absent is a wildcard, `layer` is then
/// read by nobody, and the pair has to be documented as *present or absent as a
/// unit* because nothing else can hold it to that. An `Option<NodeAt>` holds it
/// structurally — `docs/contributing.md` §4's third tier — and the wildcard is
/// the `None`, which is what a wildcard is: a write that names no node, and so
/// names no layer either.
///
/// **Named for `karakuri_operation::NodeAt`, which is the same address one
/// crate along**, and deliberately not renamed on the way across: an operation
/// says `{layer, index}` and the record it becomes says `{layer, index}`, so
/// there is one thing to learn rather than two spellings of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeAt {
    pub layer: Layer,
    /// **Which node of that layer. Absent is 0 and 0 is not written**, on
    /// [`Record::Procedure`]'s terms and emphatically not
    /// [`Record::Param`]'s: this address names one node, and the "every node
    /// declaring the key" reading lives one level up, in the absence of the
    /// whole [`NodeAt`].
    #[serde(default, skip_serializing_if = "is_zero")]
    pub index: u32,
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

/// What [`Record::Camera`]'s `height` reads as when a file does not carry one,
/// which is every file written before 2026-09-09.
///
/// **`karakuri_engine::camera::Orbit::default().height`, restated.** This crate
/// has no dependency on the engine and is not about to grow one for a float, so
/// the two are a copy — held together by a test in `karakuri-environment`,
/// which already depends on both and is where a Set file meets an `Orbit`.
fn default_camera_height() -> f32 {
    2.0
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

/// **What drives one parameter**, as [`Record::Source`] carries it: the four
/// fields [`Record::Bind`] carries beside its address.
///
/// **A struct rather than four fields on the record**, because it is present
/// or absent as a unit — an attachment names all four or there is no
/// attachment — which is [`NodeAt`]'s argument one record along and
/// `docs/contributing.md` §4's structural tier. A take-back spelled as four
/// absent fields would be a record that could be written half detached.
///
/// **Its four are [`Record::Bind`]'s four, restated and not shared.** One
/// `#[serde(flatten)]`ed struct across both would make a Set file's record and
/// a session's one shape, which is exactly what
/// `docs/adr/0280-a-parameter-written-to-a-live-set-is-a-session-record.md`
/// refused for `param` and `ride`: two files, two records, and the projection
/// is what makes them two facts. The semantics are shared where they belong
/// instead — `karakuri_environment::setfile::binding_from_record` is the one
/// decoder, and the session record is turned into a `bind` on the way into it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
    /// A name on the bus — `energy`, `beat`, `band3`, `noise` — or
    /// `control:<name>` for a published control of the same Set.
    pub signal: String,
    /// `lin`, `pow2`, `sqrt` or `smooth`. A `String` for [`Record::Bind`]'s
    /// reason: what a curve is allowed to be is the engine's to say.
    pub curve: String,
    /// What the signal is mapped onto, low then high.
    pub range: [f32; 2],
    /// Only a `signal` of `"noise"` reads this — see [`Record::Bind`], whose
    /// field this is field for field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub noise: Option<BindNoise>,
}

/// **The master chain**, as [`Record::MasterChain`] carries it: the ordered
/// list of its slots and nothing else.
///
/// A struct rather than the variant's own fields because
/// `#[serde(deny_unknown_fields)]` has no variant form — see
/// [`Record::MasterChain`], which is where the refusal it buys is argued.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chain {
    /// **The slots, in the order they run.** Empty is a real value and the
    /// default one: an empty chain draws nothing and the frame is the mix.
    pub slots: Vec<ChainSlot>,
}

/// **One slot of the master chain**, as [`Chain`] carries it.
///
/// A procedure, the cut it answers where it declares `retains`, and what its
/// params are set to. Nothing else: `src` is implicit — the chain's position
/// supplies it — and a chain slot's L5 declares no `uses`, so there is no edge
/// to record. See
/// `docs/adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChainSlot {
    /// **A content address**, so the procedures become part of the state a
    /// replay reproduces: a stream naming one the store does not hold is
    /// refused with the address in the message. The same terms a Set file's
    /// `slot` record uses.
    #[serde(rename = "proc")]
    pub procedure: String,
    /// **Which retained frame this slot reads** — `mix` or `exit` — and
    /// absent where its procedure declares no `retains`.
    ///
    /// **A `String` and not an enum**, which is [`Record::Blend`]'s and
    /// [`Record::Residency`]'s rule: a word an older stream does not know is
    /// the engine's to diagnose against what it actually supports, rather than
    /// a parse failure that takes the whole line with it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cut: Option<String>,
    /// **What this slot's params are set to**, keyed by the name the procedure
    /// declared. A param nobody moved is absent and the declaration's own
    /// default is what runs — the record carries what was asked for.
    ///
    /// A `BTreeMap` so the line is byte-stable: a record written twice from the
    /// same chain has to be the same bytes, which is what
    /// `a_record_round_trips_to_the_same_bytes` asks of every record here.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub params: std::collections::BTreeMap<String, f32>,
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
    /// **A node of an *authoring* Set file** (`.kset`): everything
    /// [`Record::Slot`] says about a node, with the **relative path** of its
    /// `.kir` where the content address goes.
    ///
    /// **A `t` of its own rather than a `slot` carrying a path instead of a
    /// `proc`**, which is the shape anybody reaching for "the same record with
    /// one field swapped" arrives at.
    /// `docs/contributing.md` §4 rules
    /// out precisely that — *"reusing a tag for a differently shaped record in
    /// a different file"* — and the bill for ignoring it is paid by every
    /// reader that dispatches on `t` alone: a decoder meeting `slot` would have
    /// to know which file it came out of before it knew whether `proc` was
    /// there, and the one that forgot to ask would build a Set with a node
    /// missing rather than refuse a file. Two tags, and a decoder dispatching
    /// on `t` never meets a `slot` without an address.
    ///
    /// **So a `.kbset` holding one of these is a file disagreeing with its own
    /// extension**, which is a sentence the store can say because it is the
    /// whole of what that extension asserts — everything in a `.kbset` is
    /// already resolved, and `<store>/sets/` holds `.kbset` and only `.kbset`
    /// because a swap that can partially fail is not a swap
    /// (`docs/adr/0231-a-sets-two-forms-take-two-extensions-and-the-store-holds-only-the-resolved-one.md`,
    /// `docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md`).
    ///
    /// **This is therefore the one variant that can only appear in a file the
    /// store will not hold**, and it is refused for a reason none of its
    /// neighbours are refused for. A `gain` is the deck's, a `param_decl` is an
    /// artifact's; a `part` is this Set's own — it says what a `slot` says
    /// about the same node — and what is wrong with it in `sets/` is only that
    /// it has not been resolved yet. Hence a question of its own,
    /// [`Record::is_authoring`], and a sentence of its own from
    /// `StoreError::PartInSet`: an operator who wrote one is told which form
    /// their file is, not that it carries time.
    ///
    /// **Resolution is a read, a hash and a store put — never a compile.** A
    /// `slot`'s `proc` is the content address of the `.kir` source, so turning
    /// one of these into one is: read the file, hash the bytes, `put_artifact`,
    /// emit a `slot` naming that address. `karakuri_environment::setfile`'s
    /// `resolve` is where that happens, which is before the file reaches a
    /// store rather than during a load.
    Part {
        /// The node's layer, exactly as [`Record::Slot`] carries it.
        layer: Layer,
        /// Which node of that layer, on [`Record::Slot`]'s terms and by the
        /// same reading: absent is 0 and 0 is not written.
        #[serde(default, skip_serializing_if = "is_zero")]
        index: u32,
        /// What this Set calls the node — [`Record::Slot`]'s field, unchanged,
        /// because a name belongs to the *use* and the use is the same one.
        /// It survives resolution: the `slot` written in its place carries it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        /// **The `.kir`'s path, relative to the authoring file's own
        /// directory.**
        ///
        /// A `String` and not a `PathBuf` because it is a wire field: what is
        /// kept here is what the file said, byte for byte, and a path repaired
        /// on the way in is the one thing that must not happen — the reader
        /// that quietly fixed it would be the reader that opened something
        /// nobody named.
        ///
        /// **Whether it is a path this machine may open is not this
        /// vocabulary's question**, on the terms every other check here is left
        /// to the place that has what it needs: the answer depends on where the
        /// file itself is, which a record does not know. It is `setfile`'s
        /// wall, it refuses rather than repairs, and it names what it refused —
        /// an absolute path, a `..` that escapes, and a symlink pointing out
        /// are three spellings of one escape.
        path: String,
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
        /// spelled on the way in, which is what made this field part of the
        /// naming work rather than a follow-on to it — see
        /// `docs/adr/0111-a-name-lives-in-the-set-file-and-may-be-written-on-the-command-line.md`.
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
        /// **Which camera node of the L3 layer these numbers are about**, on
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
        /// **Where the eye rides above the target** — the third of the orbit's
        /// three placement numbers, and the newest field in this record.
        ///
        /// **It carried two of the three until 2026-09-09**, so a Set kept with
        /// its camera looking down and loaded again was looking along the
        /// equator and nothing said so. The three became parameters of the
        /// camera node that day
        /// (`docs/adr/0318-the-built-in-cameras-three-placement-numbers-are-parameter-rows.md`),
        /// which is what made the gap a defect rather than a limit: a number a
        /// hand can move and a file cannot record is a knob that walks back on
        /// its own.
        ///
        /// **Absent reads as 2.0**, which is what every file written before
        /// this field meant — the engine's `Orbit::default().height`, restated
        /// here because this crate depends on nothing and cannot ask for it.
        /// The two are held together by a test in `karakuri-environment`, which
        /// depends on both; see `default_camera_height`.
        ///
        /// **Written unconditionally**, on `radius`'s and `speed`'s terms
        /// rather than `index`'s: a placement number is what this record is
        /// for, and one omitted at its default would make a file's silence mean
        /// two things depending on which of the three it was about.
        #[serde(default = "default_camera_height")]
        height: f32,
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
    /// **`slot` here is the third sense the module doc names** — an input a
    /// node declares, not a node of a Set and not a member of the deck.
    ///
    /// A procedure declares what it takes and refuses to say where it comes
    /// from — `uses far : Geometry` names an input slot the way `consumes position`
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
    /// An input slot bound twice is refused where the Set is built rather than here,
    /// on the terms a duplicate node name is: the vocabulary's job is to carry
    /// what a file said.
    Edge {
        /// The node that declares the input slot.
        node: String,
        /// What that node's procedure calls that input slot.
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
    /// the blend takes effect, including how much that slot's layer covers.
    /// Under `add` the two multiply together and a stream could have carried
    /// either — under `over` a session that replayed one as the other would
    /// replay a deck slot's layer that hides as one that dims.
    Opacity {
        /// The deck slot whose fader this is.
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
    /// what it actually supports. This one has earned it — five residency
    /// levels were named and three were built, because two of the five turned
    /// out to be transitions rather than states:
    /// `docs/adr/0062-warming-and-cooling-are-transitions-not-states.md`.
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
    /// **One level on the composited frame, at the entry to the master
    /// chain.**
    ///
    /// **No `slot`, and it is the second record in this group that has none.**
    /// A gain, an opacity, a blend and a residency each describe one member of
    /// the deck; this describes what the fold *produced*, after every one of
    /// them has been applied — `karakuri_engine::deck::Deck::set_out` is what
    /// it decodes to and says *"Not per slot"* at the setter. [`Record::Look`]
    /// is the other one, and the two are session-wide for two different
    /// reasons: tone mapping happens once because it happens *after* the mix,
    /// and this happens once because it is what the mix wrote.
    ///
    /// **Not a field on [`Record::Look`], which is the shape it would fit
    /// and the one it must not have.** The look is one value in the engine
    /// written to one uniform; this is a different multiplication in a
    /// different pass, and the master effects go between the two. Folding it
    /// in would make an exposure change rewrite the master out and a master
    /// out change rewrite the operator, and it would have to be pulled back
    /// out the day the chain is not empty — the whole argument of
    /// `docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md`,
    /// which is also where the cost of saying so today is written down: with
    /// nothing in the chain, no frame tells the two levels apart.
    ///
    /// **One value and no operator beside it.** When this was written there
    /// was nothing else about the master chain a stream could say, so a record
    /// that carried more would have been recording defaults nobody chose.
    ///
    /// **The chain landed on 2026-09-09 and this record did not grow — a
    /// second one did.** [`Record::MasterChain`] carries the chain's slots and
    /// what each is set to, and the two are apart for the reason this record is
    /// not a field of [`Record::Look`], one paragraph up: they are different values
    /// in different passes, and folding them would make a fader ride rewrite
    /// four settings sixty times a second and a settings change rewrite the
    /// level a hand is holding. This one is a **level**, rode continuously by
    /// a fader and classed with the mix faders in `gate.rs`; that one is a
    /// **setting**, moved by a press and classed with the master effects.
    ///
    /// `value` is floored at zero and deliberately open above 1.0, on
    /// [`Record::Gain`]'s terms and through the engine's same `clamp_gain`:
    /// the mix is HDR and this level is applied to values a tone mapper has
    /// not seen.
    MasterOut {
        value: f32,
    },
    /// **What the master chain is**, whole: the ordered list of its slots.
    ///
    /// The chain sits between [`Record::MasterOut`]'s level at its entry and
    /// [`Record::Look`]'s exposure at the tone mapper's input —
    /// `karakuri_engine::master` is the implementation and
    /// `docs/adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md`
    /// is the decision. **The order is here because it is no longer a
    /// constant**: a slot holds one `kind L5` procedure, named by a content
    /// address, and a chain is whichever of them an operator put in it.
    ///
    /// **One record and not three**, which is [`Record::Look`]'s shape and its
    /// argument: `feedback` without `cut` beside it is not a picture anybody
    /// can reconstruct — the same 0.5 is a one-frame echo under `mix` and a
    /// compounding trail under `exit` — and a stream that could move one pass
    /// without saying where the others were would be describing a chain a
    /// replay could not put back. The place that turns an operation into this
    /// already knows the chain that is running and fills the rest in, exactly
    /// as `karakuri-cli`'s `set_exposure` fills in the operator.
    ///
    /// **Session-wide and not per slot**, [`Record::MasterOut`]'s reason: the
    /// chain reads what the fold produced, after every deck's edge has been
    /// applied.
    ///
    /// **A performance's state and not a library's.** What a chain is set to
    /// *while it is being played* is stream state, the way a Set's gain is;
    /// what a chain is set to *saved under a name and put back tomorrow* is
    /// library data in two tiers, and
    /// `docs/adr/0227-a-pattern-and-a-master-chain-setting-are-library-data-in-two-tiers.md`
    /// is where that was decided and left to the record that has something to
    /// serialise. This is the first half only; no store directory is added
    /// here.
    ///
    /// Every value is brought into the range its procedure declared, by the
    /// engine — `karakuri_engine::master::Slot::resolved` — which is the same
    /// division of labour [`Record::Gain`] has: the record carries what was
    /// asked for, the engine holds the range.
    ///
    /// **The four-field form is refused rather than read**, which is
    /// `#[serde(deny_unknown_fields)]` on this one variant and on no other.
    /// Between 2026-09-09 and M5.16 this record was
    /// `{feedback, cut, bloom, rgb_shift}`; the shape changed under the same
    /// `t`, so an unrecognised key here is not a field from the future, it is
    /// the old form — and dropping it silently, which is what this format does
    /// everywhere else, would play an **empty chain** where the session had
    /// three passes. A refusal that names the key it found is what
    /// `docs/adr/0340-…` asks for in place of that default, and the bill for
    /// reading both shapes is a decoder carrying two of them for the length of
    /// the project
    /// (`docs/principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md`).
    ///
    /// **A newtype variant and not a struct one**, which is the whole of how
    /// that refusal is spelled: `deny_unknown_fields` is a *struct* attribute
    /// and serde has no variant form of it, so the payload is a struct and the
    /// variant wraps it. On the wire it is the same object it would have been —
    /// `{"t":"master_chain","slots":[…]}` — because an internally tagged enum
    /// folds a newtype variant's struct into the tag's own object.
    MasterChain(Chain),
    /// **The procedure a deck slot is playing, from this moment on.**
    ///
    /// Written when a hot swap lands, which since ADR-0316 is the one moment
    /// the material a session is playing changes: the budget's verdict leaves
    /// the installed Set where it is and stops the slot, and there is no
    /// rollback to write a second record for. Without it a
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
    /// **One thing it cannot carry**, and ADR-0316 changed which. A swap-*in*
    /// is documented to start cold, so a replay meeting this record builds
    /// afresh and replays exactly. What this cannot say is that the slot was
    /// **stopped**: a version over the frame budget is recorded like any other,
    /// because it is what the slot holds, and a replay judges nothing — so it
    /// runs at full rate material the performance had frozen. That is a gap in
    /// this vocabulary and is recorded as one rather than patched here.
    Procedure {
        slot: u8,
        layer: Layer,
        /// **Which node of that layer**, when a deck slot has more than one.
        ///
        /// A deck slot draws with one L1 and however many L4s — several renderers
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
    /// **Who may move one node of the Set a deck slot is playing** —
    /// `manual`, `suggesting` or `automatic`.
    ///
    /// **A session fact about a node, which is [`Record::Procedure`]'s shape
    /// exactly and is why this is not a Set file's record.** Every other record
    /// that names a node of a Set — [`Record::Slot`], [`Record::Capacity`],
    /// [`Record::Param`], [`Record::Bind`], [`Record::Seed`] — is written into
    /// a Set file and carries no `slot`, because what a Set *is* does not
    /// depend on which deck slot it is playing in. A Set does not know which
    /// agent is watching it either: authority is an arrangement between an
    /// operator and whatever else is in the room, made during a performance,
    /// and a Set file that carried one would hand a node to an agent wherever
    /// it was next loaded. So it is addressed the way `procedure` is — a deck
    /// slot, a layer and an index — and it goes in a session stream and never
    /// in a Set file.
    ///
    /// **The address is a node and deliberately not a deck slot.** Rule 06 of
    /// the manual: *"There is no switch that hands the whole instrument to an
    /// agent, because the useful arrangement is almost always partial."* A flag
    /// beside the `slot` and nothing else would be that switch.
    ///
    /// `index` is which node of that layer, on [`Record::Procedure`]'s terms:
    /// absent means 0 and 0 is not written, so this record costs an existing
    /// stream nothing and says something only where it is written.
    ///
    /// A `String` rather than an enum, on `residency`'s terms and `curve`'s:
    /// **what a level is allowed to be is the engine's to say**, and a stream
    /// from a newer build should reach a diagnostic about what this build
    /// supports rather than a parser that refuses the line.
    ///
    /// **Nothing writes one yet, and what is missing is the engine.** A node's
    /// authority has to survive a rebuild, which means it is restated on
    /// `karakuri_engine::swap::Request` the way that request's `params`,
    /// `published`, `bindings`, `names` and `edges` are — *"a rebuild that lets
    /// a value be re-derived is a rebuild that quietly discards what was
    /// loaded"*. Written here first because the vocabulary is what a surface
    /// asks in, and because the record is what says the fact is the session's:
    /// see
    /// `docs/adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md`.
    Authority {
        slot: u8,
        layer: Layer,
        /// **Which node of that layer.** Absent is 0 and 0 is not written, on
        /// [`Record::Procedure`]'s terms.
        #[serde(default, skip_serializing_if = "is_zero")]
        index: u32,
        /// `manual`, `suggesting` or `automatic` — the words
        /// `karakuri_operation::Authority::name` writes. Named for the concept
        /// rather than shortened, because there is no shorter word for it that
        /// is not already taken: `mode` is the word rule 06 refuses (*"never a
        /// global mode"*) and `level` is `residency`'s.
        authority: String,
    },
    /// **A parameter an operator moved on a deck slot that is playing** — one
    /// knob turn, at the frame it happened.
    ///
    /// **The session's twin of [`Record::Param`], and the third record in this
    /// vocabulary to name a node of the Set a deck slot is playing.**
    /// [`Record::Procedure`] was the first and [`Record::Authority`] the
    /// second, and the reason is the same one all three times: a `param` says
    /// what a Set *is* and carries no `slot`, because what a Set is does not
    /// depend on which deck slot it is playing in. This says what an operator
    /// *did*, to one deck slot, at one instant — a fact about a performance,
    /// which is what a session stream is made of. A `param` in a stream can
    /// only mean the Set at the head of it, and a deck holds four.
    ///
    /// **Two spellings of one act is the cost, and it is the cost `slot` and
    /// `procedure` already charge** — both say *this node runs this
    /// procedure*, in a Set file and in a session, and the format keeps them
    /// apart rather than growing one a `slot` field. What makes it two facts
    /// rather than one fact written twice is the projection: folding a session
    /// down to a Set file drops this, on `select`'s terms, because nothing in a
    /// stream says which deck slot the Set at its head was played in. See
    /// `karakuri_store::project::key_for`.
    ///
    /// **`at` absent is every node declaring `key`**, which is
    /// [`Record::Param`]'s wildcard and the useful default: one knob moving
    /// every renderer that has an `exposure`. It is refused where the nodes it
    /// lands on are not under one authority — `karakuri_engine::set::Set::write_param`
    /// is where that is decided and the only place it is decided, so this
    /// record reaches it by the road a `--param` and a `param` reach it by.
    ///
    /// **`key` is a component key where the parameter is a vector** — `glow.x`
    /// and never `glow` — because a parameter is driven one component at a time
    /// ([ADR-0268](../../../docs/adr/0268-a-vector-parameter-is-driven-one-component-at-a-time.md)).
    /// A wide [`Value`] is still legal here for the reason it is legal on a
    /// `param`: it is one line a person or a model writes, and the reader
    /// expands it into one write per component.
    ///
    /// **What a replay does with it**, and this is the whole reason it exists:
    /// a knob turn used to write nothing at all, so a session recorded an
    /// operator riding a parameter for a minute and replayed it at the value
    /// the Set was loaded with —
    /// [P-0092](../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)'s
    /// *"mutating a live Set in place, which opens a hole in the record
    /// stream"*, exactly. Decided in
    /// `docs/adr/0280-a-parameter-written-to-a-live-set-is-a-session-record.md`.
    Ride {
        /// The deck slot whose Set is being written — *a deck slot*, on
        /// [`Record::Procedure`]'s and [`Record::Authority`]'s terms.
        slot: u8,
        /// **Which node, or every node declaring `key`.** Absent is the
        /// wildcard; see [`NodeAt`] for why the address is one field.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        at: Option<NodeAt>,
        key: String,
        value: Value,
    },
    /// **What is driving one parameter of a deck slot that is playing, or
    /// nothing** — an attachment made during a performance, and the taking of
    /// one back.
    ///
    /// **The session's twin of [`Record::Bind`], and the fourth record in this
    /// vocabulary to name a node of the Set a deck slot is playing.**
    /// [`Record::Procedure`], [`Record::Authority`] and [`Record::Ride`] are
    /// the first three, and the reason is the same one all four times: a `bind`
    /// says what a Set *is* and carries no `slot`, because what a Set is does
    /// not depend on which deck slot it is playing in. This says what an
    /// operator *did*, to one deck slot, at one instant.
    ///
    /// **One record for the attachment and the take-back**, and `source`
    /// present or absent is which. They are one fact — *what is driving this
    /// parameter* — and two records would be two `t`s that the projection
    /// dropped for one reason and that the applier had to keep in step by
    /// care. An absent `source` is the removal and not a suspension: there is
    /// no suspended state anywhere in the engine to record, and the value a
    /// parameter is left at is its own, which is what a binding blends *from*
    /// at every confidence
    /// ([P-0084](../../../docs/principles/0084-a-confident-wrong-automatic-judgement-is-worse-than-not-judging.md)).
    ///
    /// **The address is [`Record::Bind`]'s and not [`Record::Ride`]'s**, and
    /// the difference is not a drift. A `ride` writes a value, and a value has
    /// a wildcard that names no layer — every node declaring the key, across
    /// every layer — which is why its address is one `Option<NodeAt>`. A
    /// binding is a layer's: `karakuri_engine::binding::Binding` carries a
    /// required `layer` and an optional `index`, `Set::bind` resolves the
    /// nodes through it, and there has never been a binding that meant *every
    /// layer*. So `layer` here is load-bearing, which is the one thing
    /// [`Record::Param`]'s never was.
    ///
    /// ```ndjson
    /// {"t":"source","slot":0,"layer":"L1","key":"turbulence","source":{"signal":"energy","curve":"pow2","range":[0.1,2.4]}}
    /// {"t":"source","slot":0,"layer":"L1","key":"turbulence"}
    /// ```
    ///
    /// **A hand on a bound parameter's value does not appear here.** A `ride`
    /// writes the parameter's own value and the attachment goes on blending
    /// from it, so the two records never race: only this one attaches and only
    /// this one detaches. Decided in
    /// `docs/adr/0319-an-attachment-is-a-session-record-and-taking-a-parameter-back-removes-it.md`.
    Source {
        /// The deck slot whose Set is being written — on [`Record::Ride`]'s
        /// terms.
        slot: u8,
        /// Which layer declares the `param`, and it is read.
        layer: Layer,
        /// Which node of that layer, or every node declaring `key` — see
        /// [`Record::Bind`], which this follows exactly.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<u32>,
        key: String,
        /// **What drives it, or nothing.** Absent is *Take a parameter back*.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<Source>,
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
    /// Changing it reallocates every deck slot's target and the HDR target, which is
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
    /// A mask multiplies that layer's opacity per texel, which is what makes it
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
        /// The deck slot whose control is moving.
        slot: u8,
        /// `gain`, `opacity` or `mask` — the last being the `position` a
        /// `mask` record carries, which is what a wipe moves.
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
    /// the Set a deck slot is playing, which is a second index no control
    /// carries.
    ///
    /// **`start` alone, on `transition`'s terms**: an absolute position on the
    /// session's beat count, because "on the next bar" is a different instant
    /// depending on when it is read. Quantising happens once, where the
    /// operator asked.
    ///
    /// **What it selects is an edge into the Set's L5**, so it says something
    /// only where the deck slot was built to composite — `--merge N`. A deck
    /// slot that overdraws has no L5 for an edge to go into, and this record is then
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
        /// The deck slot whose Set the selection is inside.
        slot: u8,
        /// Which renderer of that deck slot, in draw order — the same numbering
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
    /// **What a deck slot's clock does with the session's** — `free`, `tempo`
    /// or `beat`, with the two numbers that make the mode mean something.
    ///
    /// `anchor_bpm` is the tempo at which this material runs at 1x, and it has
    /// to be recorded because **material has no intrinsic tempo**: a `.kir`
    /// declares parameters and a capacity, not a bar length, so "one beat of
    /// music is how many seconds of material" is an operator's answer rather
    /// than the artifact's. `scrub_beats` is where the operator scrubbed the
    /// slot to, in beats off the room's position — signed, unbounded, and the
    /// one value in this format that is meant to go backwards.
    ///
    /// **It was spelled `offset_beats`, and the rename reaches the wire.** The
    /// word `offset` already names the operator's *latency* offset — the
    /// milliseconds of `--latency-offset-ms` — and one word for two controls on
    /// one panel is what
    /// `docs/contributing.md` §4
    /// forbids. The beat side had a name it was not using: its control is the
    /// scrub, its operation is `ScrubDeck`, and `Transport`'s own field doc
    /// already called it *"the operator's scrub"*. Keeping the old spelling on
    /// disk behind a `serde(rename)` was the alternative, and it is the
    /// compatibility alias
    /// `docs/adr/0117-before-v1-pay-the-cost-of-changing-toward-the-ideal.md`
    /// refused for `field(p)`: two spellings for one thing, forever. Before v1
    /// the bill is a bill and not an argument
    /// (`docs/principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md`),
    /// and the bill here is loud rather than silent — a stream carrying
    /// `offset_beats` fails its line with serde's missing-field error, not with
    /// a scrub silently read as zero.
    ///
    /// Both are carried even under `free`, where neither does anything, so that
    /// a deck slot moved back onto the grid returns to where the operator left it
    /// rather than to a default.
    Transport {
        slot: u8,
        sync: String,
        anchor_bpm: f32,
        scrub_beats: f64,
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
    /// **The deck slot and the id, and deliberately not the node hashes.** The Set
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
        /// What the Set file is called in the store: `sets/<id>.kbset`.
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
    /// **v0.2 had no tempo record at all**: the
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
    /// What a Set is *before it is resolved*: the authoring form, `.kset`,
    /// which names its `.kir` files by relative path and lives beside them.
    ///
    /// **A vocabulary of its own and not a corner of [`Vocabulary::Set`]**,
    /// though what it says is a Set's own state and nothing else's. The
    /// question every reader here asks is *which file may this line be in*, and
    /// the answer for a [`Record::Part`] is one file the store may never hold:
    /// `<store>/sets/` is `.kbset` and only `.kbset`, because reading a
    /// `.kbset` resolves nothing against the filesystem and that is what makes
    /// a swap unable to half-fail. Classified `Set`, it would be written into a
    /// store by the one function that exists to stop that
    /// (`Store::write_set`), and the file would then be a `.kbset` naming a
    /// neighbour it may no longer have.
    Authoring,
    /// A `t` this build does not know. **Not a vocabulary but the absence of
    /// one**, and it is a case of its own because every reader treats it as a
    /// line to carry rather than a line to place: passing it over is the
    /// format's promise, so a Set file round-tripped through a build that does
    /// not know every record in it comes back with all of them.
    Unknown,
}

impl Record {
    /// Whether this record belongs in a Set file. **Twenty-three say no, for
    /// four different reasons, and keeping them apart is the point of the
    /// name** — it is `is_set_state` rather than `is_state` because most of
    /// what it refuses is state. (It read "twenty-one" until [`Record::Part`]
    /// arrived and the variants were counted: eighteen, plus the four metadata
    /// records, plus this one. A prose count is not checked by anything, which
    /// the group comment above [`Record::Gain`] confesses to about its own two.
    /// The *classification* cannot drift the same way — see [`Vocabulary`].)
    ///
    /// - [`Record::Tick`], [`Record::Audio`] and [`Record::Tempo`] are not
    ///   state at all: they are what a *frame* saw or decided. A Set file
    ///   carries no time, and one holding an audio frame would be claiming a
    ///   particular moment's microphone reading is part of what a Set is.
    /// - [`Record::Gain`], [`Record::Opacity`], [`Record::Blend`],
    ///   [`Record::Residency`], [`Record::Look`], [`Record::Canvas`],
    ///   [`Record::Procedure`], [`Record::Authority`], [`Record::Source`],
    ///   [`Record::Transport`],
    ///   [`Record::Mask`], [`Record::Transition`] and
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
    ///   [`Record::Select`] says which renderer of a deck slot is live, which is a
    ///   fact about a run — and it used to be in this group twice over,
    ///   because the layering that makes the question mean anything was not in
    ///   a Set file at all. It is now: [`Record::Merge`] records it, and its
    ///   `live` field is where a *Set* says which renderer is the live one.
    ///   What remains is the first reason and it is enough — a selection is
    ///   addressed to a deck slot and scheduled at an instant, and no session
    ///   record says which deck slot's Set composites or which deck slot a
    ///   folded Set was played in.
    ///
    /// - [`Record::Meta`], [`Record::ParamDecl`], [`Record::CapacityDecl`] and
    ///   [`Record::Emit`] are a **third file's** vocabulary: what an artifact
    ///   *declares*, before anything has instantiated it. Not the session's and
    ///   not any Set's, which is why they are a third reason rather than a
    ///   longer second one.
    ///
    /// - [`Record::Part`] is a **fourth reason and the only one that is not
    ///   about what the record says**. It is this Set's own state, in the
    ///   vocabulary of the *authoring* form: a node named by relative path
    ///   rather than by content address. Nothing is wrong with what it says —
    ///   it says what a [`Record::Slot`] says — and everything is wrong with
    ///   where it is, because `<store>/sets/` holds `.kbset` and only `.kbset`
    ///   so that a swap has nothing left to resolve. Refused through
    ///   [`Record::is_authoring`], for the reason a `param_decl` is refused
    ///   through [`Record::is_metadata`]: a third sentence is owed, and it is
    ///   about which of a Set's two forms the file is.
    ///
    /// **Four reasons for one answer, and `Record::Save` still does not add
    /// another.** The question here is *may this line go in a Set file*, and it
    /// has one answer per record however many reasons stand behind a no.
    /// Whether a record **reaches outside the stream** is a different question
    /// about the same vocabulary, and `Record::Save` is so far the only record
    /// for which the answer is yes; folding that in would give one function two
    /// jobs. It lives in `docs/ir-spec.md` under "Records with an effect
    /// outside the stream", where a replay reads it.
    ///
    /// A session stream carries the eighteen of the first two groups and none
    /// of the third or the fourth. That is the difference between the two files, stated from
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
            // **The one record that is a Set's own state and still not a Set
            // file's**, because the file it belongs to is the one a store may
            // not contain. See [`Vocabulary::Authoring`] and
            // [`Record::is_authoring`].
            Record::Part { .. } => Vocabulary::Authoring,
            Record::Tick { .. } | Record::Audio { .. } | Record::Tempo { .. } => Vocabulary::Frame,
            Record::Gain { .. }
            | Record::Opacity { .. }
            | Record::Blend { .. }
            | Record::Residency { .. }
            | Record::Look { .. }
            | Record::MasterOut { .. }
            | Record::MasterChain(_)
            | Record::Canvas { .. }
            | Record::Procedure { .. }
            | Record::Authority { .. }
            | Record::Ride { .. }
            | Record::Source { .. }
            | Record::Transport { .. }
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
    /// **`thumbnail` is the fifth name because `preview` was taken**, by the
    /// deck's record for which slot was being auditioned. The specification
    /// called the library asset `preview` too, and one `t` cannot carry two
    /// shapes: the deck record's `slot` was an `Option`, so the specified line
    /// decoded as the deck record and lost its `path` without a word. The
    /// metadata name moved rather than the deck's, which was written into
    /// session streams (ADR-0134); the deck's record has since been retired
    /// outright (ADR-0240) and the name is free, but `thumbnail` is the word
    /// the library already used and it stays. Nothing here has a `Thumbnail`
    /// variant, because nothing renders one yet — it joins `origin`, `parent`,
    /// `perf` and `tag` on the list of records the specification describes and
    /// nothing writes.
    pub fn is_metadata(&self) -> bool {
        matches!(self.vocabulary(), Vocabulary::Metadata)
    }

    /// **Whether this record belongs to the authoring form of a Set file**
    /// (`.kset`) rather than to the resolved one. One says yes:
    /// [`Record::Part`].
    ///
    /// **A third question for a third sentence**, which is the reason there
    /// were two — [`Record::is_metadata`] exists so that a rejected
    /// `param_decl` is told it is a declaration rather than told it carries
    /// time, and a `part` needs the same courtesy for a reason further from
    /// either: it is neither a declaration nor time nor the deck's. It is this
    /// Set's own state, written in the form that names its parts by path, and
    /// what is wrong with it inside a store is only that nothing has resolved
    /// it yet. `StoreError::PartInSet` says that, and says which of the two
    /// forms the file is — where `TickInSet`'s sentence would send an operator
    /// looking for a `tick` they did not write.
    ///
    /// Read off [`Vocabulary`] like the other two, so the three cannot
    /// disagree about one record and a new variant with no arm does not
    /// compile.
    pub fn is_authoring(&self) -> bool {
        matches!(self.vocabulary(), Vocabulary::Authoring)
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

    /// **A `ride` round-trips through the line a session writes**, bytes and
    /// all, in both of its two shapes: addressed, and the wildcard.
    ///
    /// `round_trip_verbatim` for [`Record::Select`]'s reason with one more of
    /// its own — this record's whole shape is what it leaves out. `at` absent
    /// is *every node declaring the key*, and an `at` acquiring a serde default
    /// would turn every wildcard ever recorded into a write addressed at `L1:0`
    /// while still comparing equal to itself. The bare wildcard line is
    /// therefore the one that has to survive untouched.
    #[test]
    fn a_ride_round_trips_addressed_and_as_a_wildcard() {
        // Addressed, and at a node that is not 0 — the index is written only
        // where it says something, so an addressed write at node 0 and a
        // wildcard are told apart by `at` and never by `index`.
        let rec = round_trip_verbatim(
            r#"{"t":"ride","slot":2,"at":{"layer":"L4","index":1},"key":"glow.x","value":0.4}"#,
        );
        assert_eq!(
            rec,
            Record::Ride {
                slot: 2,
                at: Some(NodeAt {
                    layer: Layer::L4,
                    index: 1
                }),
                key: "glow.x".to_string(),
                value: Value::Scalar(0.4),
            }
        );
        // The session's, and for `procedure`'s and `authority`'s reason rather
        // than `select`'s: it is state, and it is the deck's. A Set file that
        // carried one would restore a knob position wherever it was loaded.
        assert!(!rec.is_set_state());
        assert!(!rec.is_metadata());

        // Node 0 of a layer, addressed. The `index` is absent because it is
        // zero, and the `at` is present because a node was named — which is the
        // distinction `Record::Param` cannot draw at all.
        let rec = round_trip_verbatim(
            r#"{"t":"ride","slot":0,"at":{"layer":"L1"},"key":"radius","value":2.6}"#,
        );
        assert_eq!(
            rec,
            Record::Ride {
                slot: 0,
                at: Some(NodeAt {
                    layer: Layer::L1,
                    index: 0
                }),
                key: "radius".to_string(),
                value: Value::Scalar(2.6),
            }
        );

        // The wildcard: no node, and so no layer either.
        let rec = round_trip_verbatim(r#"{"t":"ride","slot":1,"key":"exposure","value":2.0}"#);
        assert_eq!(
            rec,
            Record::Ride {
                slot: 1,
                at: None,
                key: "exposure".to_string(),
                value: Value::Scalar(2.0),
            }
        );

        // A wide value on one line, which is what a model writes and what a
        // reader with the Set in hand expands — `param`'s rule exactly
        // (ADR-0268).
        let rec =
            round_trip_verbatim(r#"{"t":"ride","slot":0,"key":"glow","value":[0.4,0.7,1.0]}"#);
        assert_eq!(
            rec,
            Record::Ride {
                slot: 0,
                at: None,
                key: "glow".to_string(),
                value: Value::Vec3([0.4, 0.7, 1.0]),
            }
        );
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

    /// **The scrub round-trips through the line the spec prints, bytes and
    /// all** — and it is `scrub_beats` on the wire, not just in Rust.
    ///
    /// This record had no round-trip test at all until the rename, which is
    /// exactly the gap that lets a serialised field's writer and reader drift
    /// apart in silence: a half-applied rename compiles in neither direction
    /// here, but a `serde(rename)` on one side and not the other compiles in
    /// both and only fails on disk.
    ///
    /// `round_trip_verbatim` rather than `round_trip`, on
    /// [`Record::Procedure`]'s terms: every field is required and written in
    /// declaration order, so a field appearing, vanishing or moving is a
    /// changed stream and only the byte comparison sees it.
    #[test]
    fn a_transport_round_trips_through_the_line_the_spec_prints() {
        let line =
            r#"{"t":"transport","slot":0,"sync":"beat","anchor_bpm":126.0,"scrub_beats":-0.25}"#;
        let rec = round_trip_verbatim(line);
        assert_eq!(
            rec,
            Record::Transport {
                slot: 0,
                sync: "beat".to_string(),
                anchor_bpm: 126.0,
                scrub_beats: -0.25,
            }
        );
        assert!(
            !rec.is_set_state(),
            "a transport is session state, not something a Set file may carry"
        );
    }

    /// **The old spelling fails loudly, and that is the whole bill.**
    ///
    /// `offset_beats` was renamed to `scrub_beats` on disk rather than kept
    /// behind a `serde(rename)`, so a stream written before the rename does not
    /// replay. What this pins is *how* it does not replay: serde has no default
    /// for the field, so the line is a parse error naming the missing field —
    /// [`crate::store::StoreError::Record`] with its line number — and not a
    /// scrub silently read as zero, which is the failure mode
    /// `docs/adr/0134-the-metadata-preview-becomes-thumbnail.md` was written
    /// about. An unknown key is dropped without comment; a *missing required*
    /// one is not, and that asymmetry is what makes this rename affordable.
    #[test]
    fn a_stream_with_the_old_offset_beats_fails_its_line_rather_than_reading_a_zero_scrub() {
        let old =
            r#"{"t":"transport","slot":0,"sync":"beat","anchor_bpm":126.0,"offset_beats":-0.25}"#;
        let err = serde_json::from_str::<Record>(old)
            .expect_err("the pre-rename spelling parsed — the rename left an alias behind");
        assert!(
            err.to_string().contains("scrub_beats"),
            "the error does not name the field that is missing: {err}"
        );
    }

    /// **The four-field `master_chain` is refused, and the refusal names the
    /// shape it found.**
    ///
    /// The record changed shape under the same `t` between 2026-09-09 and
    /// M5.16 — four scalars became a list — and
    /// `docs/adr/0340-…` refused reading both, on
    /// P-0085: a compatibility cost before v1 is a bill and not an argument,
    /// and this bill is a handful of sessions against a decoder carrying two
    /// shapes for the length of the project. What it owed instead is this: not
    /// a silent default, which here would play an **empty chain** where the
    /// session had three passes, but a refusal that says which shape it met.
    #[test]
    fn the_four_field_master_chain_is_refused_naming_the_shape() {
        let old = r#"{"t":"master_chain","feedback":0.5,"cut":"exit","bloom":0.6,"rgb_shift":0.4}"#;
        let err = serde_json::from_str::<Record>(old)
            .expect_err("the four-field form parsed — the list did not replace it");
        let said = err.to_string();
        assert!(
            said.contains("feedback"),
            "the refusal does not name the shape it found: {said}"
        );
        assert!(
            said.contains("slots"),
            "the refusal does not name the shape it wants: {said}"
        );
    }

    /// **A slot list round-trips**, and an empty one is a real value rather
    /// than an absence: the default chain is empty and that is what keeps the
    /// default look free.
    #[test]
    fn a_master_chain_slot_list_round_trips() {
        let line = r#"{"t":"master_chain","slots":[{"proc":"sha256:a3","cut":"exit","params":{"amount":0.5}},{"proc":"sha256:77","params":{"amount":0.8}}]}"#;
        let Record::MasterChain(chain) = round_trip(line) else {
            panic!("not a master_chain");
        };
        assert_eq!(chain.slots.len(), 2);
        assert_eq!(chain.slots[0].procedure, "sha256:a3");
        assert_eq!(chain.slots[0].cut.as_deref(), Some("exit"));
        assert_eq!(chain.slots[0].params.get("amount"), Some(&0.5));
        // **No cut where the procedure declares no `retains`**, and the field
        // is absent rather than null: a slot that answered a cut it was not
        // asked for is refused by the engine, so writing one would be writing
        // a record this build refuses to obey.
        assert_eq!(chain.slots[1].cut, None);
        assert_eq!(
            round_trip(r#"{"t":"master_chain","slots":[]}"#),
            Record::MasterChain(Chain::default())
        );
    }

    /// **A slot carrying a key this build has never heard of is refused too**,
    /// for [`Record::MasterChain`]'s own reason one level down: the four-field
    /// form's keys would otherwise land here as a slot nobody wrote.
    #[test]
    fn a_chain_slot_with_an_unknown_key_is_refused() {
        let line = r#"{"t":"master_chain","slots":[{"proc":"sha256:a3","amount":0.5}]}"#;
        let err = serde_json::from_str::<Record>(line)
            .expect_err("an unknown key on a chain slot was dropped");
        assert!(err.to_string().contains("amount"), "{err}");
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

    /// **A `part` round-trips through the line the spec prints**, bytes and
    /// all, and answers all three questions the way the authoring form needs.
    ///
    /// `round_trip_verbatim` for [`Record::Select`]'s reason and one more: an
    /// `index` or a `name` acquiring a serde default here would put a field
    /// into every authoring file anybody hand-writes, and the whole of what a
    /// `.kset` is for is being a file a person writes by hand.
    ///
    /// The three assertions are the classification, and they are what keeps
    /// this record out of a store: it is not Set state (so `Store::write_set`
    /// refuses it), it is not an artifact's metadata (so it is refused by the
    /// question that says *which* form the file is, not by the one that says
    /// it declares something), and it *is* the authoring form's.
    #[test]
    fn a_part_round_trips_through_the_line_the_spec_prints() {
        let line = r#"{"t":"part","layer":"L1","path":"drift_shell.kir"}"#;
        let rec = round_trip_verbatim(line);
        assert_eq!(
            rec,
            Record::Part {
                layer: Layer::L1,
                index: 0,
                name: None,
                path: "drift_shell.kir".to_string(),
            }
        );
        assert!(!rec.is_set_state());
        assert!(!rec.is_metadata());
        assert!(rec.is_authoring());

        // And the addressed spelling, which is the same record about the
        // second renderer of a stack with a name this Set gave it.
        let named =
            r#"{"t":"part","layer":"L4","index":1,"name":"veil","path":"parts/soft_points.kir"}"#;
        assert_eq!(
            round_trip_verbatim(named),
            Record::Part {
                layer: Layer::L4,
                index: 1,
                name: Some("veil".to_string()),
                path: "parts/soft_points.kir".to_string(),
            }
        );
    }

    /// **`slot` and `part` are two tags and not one tag with two shapes**,
    /// which is what `docs/contributing.md` §4 asks of a name and what a
    /// decoder dispatching on
    /// `t` alone depends on.
    ///
    /// The failure this defends against is silent: give a `slot` an optional
    /// `path` instead, and the authoring line above parses as a `slot` with no
    /// `proc` — a node with no procedure, in a file that says it is resolved.
    /// Here it cannot parse at all, which is the whole difference.
    #[test]
    fn a_part_is_not_a_slot_with_a_path_where_the_address_goes() {
        let part = r#"{"t":"part","layer":"L1","path":"drift_shell.kir"}"#;
        let slot = r#"{"t":"slot","layer":"L1","proc":"sha256:00"}"#;
        assert!(matches!(
            serde_json::from_str::<Record>(part).expect("a part parses"),
            Record::Part { .. }
        ));
        assert!(
            serde_json::from_str::<Record>(r#"{"t":"slot","layer":"L1","path":"drift_shell.kir"}"#)
                .is_err(),
            "a `slot` carrying a path instead of an address is not a record this vocabulary has"
        );
        assert!(
            serde_json::from_str::<Record>(slot).is_err(),
            "and the address a `slot` does carry is a hash, so this fixture is a bad one \
             rather than a second shape"
        );
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

    /// **A `source` round-trips both ways round**, bytes and all — with an
    /// attachment and without one — and it is the session's rather than any
    /// Set's.
    ///
    /// `round_trip_verbatim` rather than `round_trip`, on
    /// [`Record::Authority`]'s terms: `index` is absent for a wildcard and
    /// `source` is absent for a take-back, and a field appearing where nothing
    /// wrote one is exactly the failure this catches — a `"source":null` on
    /// the take-back line would round-trip by value and be a different line on
    /// the wire.
    ///
    /// The `is_set_state` assertion is what this record exists to make: a Set
    /// file carrying one would attach a signal wherever that file was next
    /// loaded, and into whatever deck slot it landed in.
    #[test]
    fn a_source_round_trips_with_an_attachment_and_without_one() {
        let attached = r#"{"t":"source","slot":0,"layer":"L1","key":"turbulence","source":{"signal":"energy","curve":"pow2","range":[0.1,2.4]}}"#;
        let rec = round_trip_verbatim(attached);
        assert_eq!(
            rec,
            Record::Source {
                slot: 0,
                layer: Layer::L1,
                index: None,
                key: "turbulence".to_string(),
                source: Some(Source {
                    signal: "energy".to_string(),
                    curve: "pow2".to_string(),
                    range: [0.1, 2.4],
                    noise: None,
                }),
            },
            "an absent index is every node of that layer declaring the key, which is \
             `Record::Bind`'s rule and not `Record::Slot`'s"
        );
        assert!(
            !rec.is_set_state(),
            "a `source` is the session's: it names a deck slot, and a Set file carrying \
             one would attach a signal wherever it was next loaded"
        );
        assert!(!rec.is_metadata());

        // **The take-back, and the absence is on the wire.** A `"source"`
        // written as `null` would be a second spelling of nothing.
        let taken = r#"{"t":"source","slot":2,"layer":"L4","index":1,"key":"exposure"}"#;
        assert_eq!(
            round_trip_verbatim(taken),
            Record::Source {
                slot: 2,
                layer: Layer::L4,
                index: Some(1),
                key: "exposure".to_string(),
                source: None,
            },
            "a take-back is this record with its attachment absent"
        );

        // **A generator rides on the attachment**, field for field with
        // `Record::Bind`'s — `stream` is written even at 0, which is that
        // record's own shape and is why it is on the line here.
        let noisy = r#"{"t":"source","slot":1,"layer":"L1","key":"spawn_rate","source":{"signal":"noise","curve":"lin","range":[0.0,600.0],"noise":{"kind":"fbm","rate":0.5,"stream":0,"octaves":3}}}"#;
        let Record::Source {
            source: Some(source),
            ..
        } = round_trip_verbatim(noisy)
        else {
            panic!("not an attachment");
        };
        assert_eq!(
            source.noise,
            Some(BindNoise {
                kind: "fbm".to_string(),
                rate: 0.5,
                stream: 0,
                octaves: 3,
            })
        );
    }

    /// **An authority round-trips through its wire name**, bytes and all, and
    /// is the session's rather than any Set's.
    ///
    /// `round_trip_verbatim` rather than `round_trip`, on
    /// [`Record::Procedure`]'s terms: the address is `(layer, index)` with the
    /// index absent at zero, so a field appearing where nothing wrote one is
    /// the failure this catches and a value comparison cannot see it.
    ///
    /// **The word is carried, not interpreted.** `manual`, `suggesting` and
    /// `automatic` are `karakuri_operation::Authority::name`'s, and a level
    /// this build does not know reaches the engine's diagnostic rather than
    /// this decoder's refusal — which is why the third line here parses at all.
    ///
    /// The `is_set_state` assertion is the one this record exists to make: a
    /// Set does not know which agent is watching it, so a Set file carrying an
    /// authority would hand a node over every time it was loaded.
    #[test]
    fn an_authority_round_trips_and_an_absent_index_stays_absent() {
        let first = r#"{"t":"authority","slot":0,"layer":"L1","authority":"manual"}"#;
        let rec = round_trip_verbatim(first);
        assert_eq!(
            rec,
            Record::Authority {
                slot: 0,
                layer: Layer::L1,
                index: 0,
                authority: "manual".to_string(),
            },
            "an absent index is the first node of that layer"
        );
        assert!(
            !rec.is_set_state(),
            "authority is the session's: a Set does not know which agent is watching it, \
             and a Set file carrying one would hand that node over wherever it was loaded"
        );
        assert!(!rec.is_metadata());

        let second =
            r#"{"t":"authority","slot":2,"layer":"L4","index":1,"authority":"suggesting"}"#;
        assert_eq!(
            round_trip_verbatim(second),
            Record::Authority {
                slot: 2,
                layer: Layer::L4,
                index: 1,
                authority: "suggesting".to_string(),
            },
            "the second renderer of deck slot 2"
        );

        // A `kind Field` node takes one: it draws nothing and its params are
        // still an operator's to ride, which is why `Layer` has the arm.
        assert_eq!(
            round_trip_verbatim(
                r#"{"t":"authority","slot":1,"layer":"Field","authority":"automatic"}"#
            ),
            Record::Authority {
                slot: 1,
                layer: Layer::Field,
                index: 0,
                authority: "automatic".to_string(),
            }
        );
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
