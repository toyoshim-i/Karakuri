use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::hash::Hash;

/// The name a procedure's header gives one declared input — `far` in `uses far
/// : Geometry` — carried by [`Record::Edge`]'s `slot` field.
///
/// Its own type because `slot` names three unrelated things in this module's
/// vocabulary: a node of a Set, a member of the deck, and this. See the module
/// documentation and
/// `docs/adr/0344-slot-is-disambiguated-into-three-types-and-adr-0049s-wait-is-over.md`.
///
/// On the wire it is unchanged: a bare JSON string. `#[derive(Serialize,
/// Deserialize)]` on a one-field tuple struct writes and reads the inner value
/// directly — the same newtype behaviour `karakuri_layout::layout::NodeId`
/// already relies on — so a `.kbset` file or a session stream sees no
/// difference from the plain `String` this replaced.
///
/// Mirrors `karakuri_ir::typed::InputPort` and `karakuri_operation::InputPort`
/// rather than depending on either. This crate has no dependency on
/// `karakuri-ir` and none is being added for one field; the three are the same
/// concept with three definitions, on `karakuri_operation::NodeAddress`'s own
/// precedent (see [`NodeAddress`]'s documentation) for a type more than one
/// dependency-isolated crate needs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InputPort(pub String);

impl InputPort {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for InputPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for InputPort {
    fn from(name: String) -> InputPort {
        InputPort(name)
    }
}

impl From<&str> for InputPort {
    fn from(name: &str) -> InputPort {
        InputPort(name.to_string())
    }
}

/// Which kind of procedure a record addresses.
///
/// A format addition, not a format change: `Field` was added after the other
/// four, and every stream written before it is read unchanged because a value
/// nobody wrote cannot appear. That is the same move the `blend` declaration
/// made by existing with one legal value — the shape is chosen so that growing
/// it costs nothing to what came before.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Layer {
    L1,
    L2,
    L3,
    L4,
    /// A `kind Field` procedure. It addresses no node — a field has no pass and no
    /// buffers, it lowers into whoever evaluates it — but its `param`s are
    /// declared, addressable and an operator's to ride, so a record naming them
    /// needs somewhere to say so.
    Field,
    /// A `kind L5` procedure — a frame effect, `[Texture] -> Texture`.
    ///
    /// One kind with two roles, and only the nested one is a Set's node: a master
    /// chain slot is one input with a surface on it and is master state rather than
    /// a Set's, which is why a Set file carries nothing for the chain. See
    /// `docs/adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md`.
    L5,
}

impl Layer {
    /// Every layer variant, in canonical order.
    pub const ALL: [Layer; 6] = [
        Layer::L1,
        Layer::L2,
        Layer::L3,
        Layer::L4,
        Layer::Field,
        Layer::L5,
    ];

    /// Name of the layer as serialized and displayed.
    pub const fn name(&self) -> &'static str {
        match self {
            Layer::L1 => "L1",
            Layer::L2 => "L2",
            Layer::L3 => "L3",
            Layer::L4 => "L4",
            Layer::Field => "Field",
            Layer::L5 => "L5",
        }
    }
}

impl std::fmt::Display for Layer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// An error returned when parsing a [`Layer`] from a string fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseLayerError(String);

impl std::fmt::Display for ParseLayerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unknown record layer: `{}`; expected one of L1, L2, L3, L4, Field, L5",
            self.0
        )
    }
}

impl std::error::Error for ParseLayerError {}

impl std::str::FromStr for Layer {
    type Err = ParseLayerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "L1" => Ok(Layer::L1),
            "L2" => Ok(Layer::L2),
            "L3" => Ok(Layer::L3),
            "L4" => Ok(Layer::L4),
            "FIELD" => Ok(Layer::Field),
            "L5" => Ok(Layer::L5),
            _ => Err(ParseLayerError(s.to_string())),
        }
    }
}

/// `serde`'s `skip_serializing_if` wants a predicate by path, and
/// `u32::is_zero` is unstable. One line so that an index of 0 — which is every
/// record written before a layer could hold more than one node, whether that is
/// a slot's two renderers or a Set's two geometries — leaves the stream exactly
/// as it was.
pub(crate) fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// A parameter value. Ranges are declared in the `.kir`; this is just the
/// value.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Scalar(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Color([f32; 4]),
}

impl Value {
    pub fn components(&self) -> &[f32] {
        match self {
            Value::Scalar(v) => std::slice::from_ref(v),
            Value::Vec2(v) => v.as_slice(),
            Value::Vec3(v) => v.as_slice(),
            Value::Vec4(v) => v.as_slice(),
            Value::Color(v) => v.as_slice(),
        }
    }

    pub fn as_slice(&self) -> &[f32] {
        self.components()
    }

    pub fn len(&self) -> usize {
        self.components().len()
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn get(&self, index: usize) -> Option<f32> {
        self.components().get(index).copied()
    }
}

impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Value::Scalar(v)
    }
}

impl From<[f32; 2]> for Value {
    fn from(v: [f32; 2]) -> Self {
        Value::Vec2(v)
    }
}

impl From<[f32; 3]> for Value {
    fn from(v: [f32; 3]) -> Self {
        Value::Vec3(v)
    }
}

impl From<[f32; 4]> for Value {
    fn from(v: [f32; 4]) -> Self {
        Value::Vec4(v)
    }
}

/// One node of a Set, as a value rather than as two fields beside each other.
/// [`Record::Slot`], [`Record::Capacity`], [`Record::Procedure`] and
/// [`Record::Authority`] each hold one directly; [`Record::Param`] and
/// [`Record::Ride`] hold an `Option<NodeAddress>`, for the wildcard described
/// below.
///
/// [`Record::Bind`] and [`Record::Source`] deliberately do not, and the reason
/// is a fact about what their wildcard means rather than an oversight:
/// `karakuri_operation::BindAt`'s own documentation draws the line — *"a value
/// has a wildcard that names no layer, because a value lands wherever the name
/// is declared; an attachment's wildcard is a layer's, because the signal is
/// written into that layer's uniform buffer."* [`Record::Param`]'s wildcard is
/// [`karakuri_engine::set::Set::set_param`]'s, which walks every node of every
/// layer — a real absence of address, which is what `None` means here. A
/// binding's wildcard walks one layer's nodes alone (`Set::bind`), so its
/// `layer` is never noise the way `Param`'s can be: `Record::Bind` keeps its
/// own `layer: Layer` field beside an `index: Option<u32>` rather than gaining
/// this type, on purpose — see that field's own documentation.
///
/// A struct so that half an address cannot be written down. Before this type
/// reached it, [`Record::Param`] spelled the address as a `layer` beside an
/// `index`, and paid for it: `index` absent is a wildcard, `layer` is then read
/// by nobody, and the pair had to be documented as *present or absent as a
/// unit* because nothing else held it to that. `Option<NodeAddress>` holds it
/// structurally — `docs/contributing.md` §4's third tier — and the wildcard is
/// the `None`, which is what a wildcard is: a write that names no node, and so
/// names no layer either. The wire keeps the old shape regardless: a `.kbset`
/// file or a session stream still sees `layer` on every `param` line, including
/// a wildcard's, because [`Record::Param`] reads it back through
/// [`mod@node_or_every_node`] rather than a bare `#[serde(flatten)]` — see that
/// module's own documentation for why the generic `Option` behaviour cannot be
/// trusted with a field old files always wrote.
///
/// Named for `karakuri_operation::NodeAddress`, which is the same address one
/// crate along, and deliberately not renamed on the way across: an operation
/// says `{layer, index}` and the record it becomes says `{layer, index}`, so
/// there is one thing to learn rather than two spellings of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeAddress {
    pub layer: Layer,
    /// Which node of that layer. Absent is 0 and 0 is not written, on
    /// [`Record::Procedure`]'s terms and emphatically not [`Record::Param`]'s: this
    /// address names one node, and the "every node declaring the key" reading lives
    /// one level up, in the absence of the whole [`NodeAddress`].
    #[serde(default, skip_serializing_if = "is_zero")]
    pub index: u32,
}

/// Backward-compatible (de)serialization for [`Record::Param`]'s
/// wildcard-capable node address.
///
/// A bare `#[serde(flatten)]` on `Option<NodeAddress>` cannot be used here.
/// Serde's rule for a flattened `Option` is *any leftover key is reason enough
/// to parse `Some`* — and `layer` is a leftover key on every wildcard `param`
/// line ever written, because the writer put `L1` on everything before this
/// address existed and still does today (see the comment on the placeholder in
/// `karakuri-environment/src/setfile.rs`). Trusting the generic behaviour would
/// read every existing wildcard `param` as an address to node 0, silently
/// narrowing a write that has always meant *every node declaring the key*.
///
/// So this reads and writes the pair by hand: `index` absent is the wildcard
/// and `None`, full stop, whatever `layer` says; `index` present is
/// `Some(NodeAddress { layer, index })`, exactly what [`NodeAddress`] would
/// have parsed on its own. Writing follows the same rule in reverse — `layer:
/// L1` is written for a wildcard, matching every writer before this type
/// existed, so the wire shape `Record::Param`'s own documentation describes is
/// unchanged.
pub(crate) mod node_or_every_node {
    use super::{Layer, NodeAddress};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    struct Wire {
        layer: Layer,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<u32>,
    }

    pub fn serialize<S>(at: &Option<NodeAddress>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let wire = match at {
            Some(at) => Wire {
                layer: at.layer,
                index: Some(at.index),
            },
            None => Wire {
                layer: Layer::L1,
                index: None,
            },
        };
        wire.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<NodeAddress>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = Wire::deserialize(deserializer)?;
        Ok(wire.index.map(|index| NodeAddress {
            layer: wire.layer,
            index,
        }))
    }
}

/// A member of the deck — an index into the mixer, and nothing about the Set
/// playing in it — carried by the `slot` field on [`Record::Gain`],
/// [`Record::Opacity`], [`Record::Blend`], [`Record::Residency`],
/// [`Record::Procedure`], [`Record::Authority`], [`Record::Ride`],
/// [`Record::Source`], [`Record::Mask`], [`Record::Transition`],
/// [`Record::Select`], [`Record::Transport`] and [`Record::Save`].
///
/// Its own type for the same reason [`InputPort`] got one: `slot` names three
/// unrelated things in this module's vocabulary, and this is the second of them
/// to stop being a bare primitive. See the module documentation and
/// `docs/adr/0344-slot-is-disambiguated-into-three-types-and-adr-0049s-wait-is-over.md`.
/// The field is not renamed — it already met `docs/contributing.md` §4's
/// *"disjoint by name"* rule the way the module documentation describes, so
/// this is a type change and not the thirteen-record rename that rule would
/// otherwise ask for.
///
/// On the wire it is unchanged: a bare JSON number. `#[derive(Serialize,
/// Deserialize)]` on a one-field tuple struct writes and reads the inner value
/// directly — [`InputPort`]'s own mechanism, and
/// `karakuri_layout::layout::NodeId`'s before it — so a `.kbset` file or a
/// session stream sees no difference from the plain `u8` this replaces.
///
/// Mirrors `karakuri_operation`'s own deck field rather than depending on it.
/// `Operation::deck` stays a bare `u8` on purpose — ADR-0344 names it
/// disjointly already, and it is an operator's *ask*, made before a deck exists
/// to check it against, where this is a record's *statement*, made after one
/// does. The wrap from one to the other happens at
/// `karakuri_operation_record::written`, which is the boundary where an
/// operation becomes a record.
///
/// A fallible constructor is the construction-time validation ADR-0344 asks
/// for. [`DeckSlot::new`] is the one place "is this a valid deck position" is
/// decided, so a range check written inline anywhere else in this workspace is
/// a second answer to a question this type already answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeckSlot(pub u8);

impl DeckSlot {
    /// `slot` is in range for a deck of `count` members, or it is not — the one
    /// place that question is answered. `crates/karakuri-environment/src/mix.rs`'s
    /// `mix::change`, `karakuri-cli`'s and `karakuri`'s own `slot_in_range` all
    /// route their check through this rather than reimplementing `slot < count`,
    /// which is what makes it one answer instead of four.
    pub fn new(slot: u8, count: usize) -> Option<DeckSlot> {
        if (slot as usize) < count {
            Some(DeckSlot(slot))
        } else {
            None
        }
    }

    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

impl std::fmt::Display for DeckSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u8> for DeckSlot {
    fn from(slot: u8) -> DeckSlot {
        DeckSlot(slot)
    }
}

/// The noise generator a `bind` declares for itself.
///
/// Every field has a default, and they are the defaults of the generator in
/// `karakuri-signal`: a `bind` that names `noise` and says nothing else gets
/// one cycle per beat of perlin on stream 0. A generator omitted field by field
/// is a generator that was under-specified, not one that was refused — this is
/// the one record whose parameters an LLM has to invent numbers for, and every
/// number here has a defensible one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BindNoise {
    /// `white`, `value`, `perlin`, or `fbm`. A `String` for the same reason `curve`
    /// is one: an unrecognised value is the engine's to diagnose against what it
    /// actually supports, not the decoder's to reject before anything can say what
    /// the alternatives were.
    #[serde(default = "default_noise_kind")]
    pub kind: String,
    /// Cycles per beat, so period is tempo-relative.
    #[serde(default = "default_noise_rate")]
    pub rate: f32,
    /// Decorrelates one binding from another. Two bindings sharing a stream move
    /// together.
    #[serde(default)]
    pub stream: u64,
    /// `fbm` only, ignored by the other three kinds. Not in v0.2 of the spec, which
    /// listed `fbm` as a `kind` while giving it nowhere to say how many octaves —
    /// the spec is corrected rather than the field being dropped, because an octave
    /// count baked into the engine is exactly the "fixed property nobody can reach"
    /// that Spawn timing rejects Poisson for.
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
/// `karakuri_engine::camera::Orbit::default().height`, restated. This crate has
/// no dependency on the engine and is not about to grow one for a float, so the
/// two are a copy — held together by a test in `karakuri-environment`, which
/// already depends on both and is where a Set file meets an `Orbit`.
pub(crate) fn default_camera_height() -> f32 {
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

/// What drives one parameter, as [`Record::Source`] carries it: the four fields
/// [`Record::Bind`] carries beside its address.
///
/// A struct rather than four fields on the record, because it is present or
/// absent as a unit — an attachment names all four or there is no attachment —
/// which is [`NodeAddress`]'s argument one record along and
/// `docs/contributing.md` §4's structural tier. A take-back spelled as four
/// absent fields would be a record that could be written half detached.
///
/// Its four are [`Record::Bind`]'s four, restated and not shared. One
/// `#[serde(flatten)]`ed struct across both would make a Set file's record and
/// a session's one shape, which is exactly what
/// `docs/adr/0280-a-parameter-written-to-a-live-set-is-a-session-record.md`
/// refused for `param` and `ride`: two files, two records, and the projection
/// is what makes them two facts. The semantics are shared where they belong
/// instead — `karakuri_environment::setfile::binding_from_record` is the one
/// decoder, and the session record is turned into a `bind` on the way into it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
    /// A name on the bus — `energy`, `beat`, `band3`, `noise` — or `control:<name>`
    /// for a published control of the same Set.
    pub signal: String,
    /// `lin`, `pow2`, `sqrt` or `smooth`. A `String` for [`Record::Bind`]'s reason:
    /// what a curve is allowed to be is the engine's to say.
    pub curve: String,
    /// What the signal is mapped onto, low then high.
    pub range: [f32; 2],
    /// Only a `signal` of `"noise"` reads this — see [`Record::Bind`], whose field
    /// this is field for field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub noise: Option<BindNoise>,
}

/// The master chain, as [`Record::MasterChain`] carries it: the ordered list of
/// its slots and nothing else.
///
/// A struct rather than the variant's own fields because
/// `#[serde(deny_unknown_fields)]` has no variant form — see
/// [`Record::MasterChain`], which is where the refusal it buys is argued.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chain {
    /// The slots, in the order they run. Empty is a real value and the default one:
    /// an empty chain draws nothing and the frame is the mix.
    pub slots: Vec<ChainSlot>,
}

/// One slot of the master chain, as [`Chain`] carries it.
///
/// A procedure, the cut it answers where it declares `retains`, and what its
/// params are set to. Nothing else: `src` is implicit — the chain's position
/// supplies it — and a chain slot's L5 declares no `uses`, so there is no edge
/// to record. See
/// `docs/adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChainSlot {
    /// A content address, so the procedures become part of the state a replay
    /// reproduces: a stream naming one the store does not hold is refused with the
    /// address in the message. The same terms a Set file's `slot` record uses.
    #[serde(rename = "proc")]
    pub procedure: String,
    /// Which retained frame this slot reads — `mix` or `exit` — and absent where
    /// its procedure declares no `retains`.
    ///
    /// A `String` and not an enum, which is [`Record::Blend`]'s and
    /// [`Record::Residency`]'s rule: a word an older stream does not know is the
    /// engine's to diagnose against what it actually supports, rather than a parse
    /// failure that takes the whole line with it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cut: Option<String>,
    /// What this slot's params are set to, keyed by the name the procedure
    /// declared. A param nobody moved is absent and the declaration's own default
    /// is what runs — the record carries what was asked for.
    ///
    /// A `BTreeMap` so the line is byte-stable: a record written twice from the
    /// same chain has to be the same bytes, which is what
    /// `a_record_round_trips_to_the_same_bytes` asks of every record here.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub params: std::collections::BTreeMap<String, f32>,
}

/// The current schema version for `.kbset` and session ndjson streams.
///
/// Files without an explicit header are detected as Version 1 (legacy
/// unversioned).
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

/// Beyond this the simulation is allowed to fall behind rather than catch up.
/// Unbounded catch-up turns a load spike into a death spiral.
pub const MAX_STEPS: u8 = 4;
