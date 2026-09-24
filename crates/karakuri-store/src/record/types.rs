use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::hash::Hash;

/// Name of an input port on a procedure (e.g. `far` in `uses far : Geometry`),
/// carried by [`Record::Edge`]'s `slot` field.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Layer {
    L1,
    L2,
    L3,
    L4,
    /// A `kind Field` procedure without dedicated passes or buffers.
    Field,
    /// A `kind L5` procedure — a frame effect (`[Texture] -> Texture`).
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

/// Predicate for `serde`'s `skip_serializing_if` to omit zero values.
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

/// Address identifying a node in a Set by its layer and index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeAddress {
    pub layer: Layer,
    /// Node index within that layer. Zero is omitted during serialization.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub index: u32,
}

/// Backward-compatible (de)serialization for [`Record::Param`]'s optional node address.
///
/// Ensures backward compatibility with existing formats where wildcard writes
/// serialize an empty index while preserving the layer field.
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

/// Identifier for a deck position in the mixer, carried by the `slot` field of mixer records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeckSlot(pub u8);

impl DeckSlot {
    /// Creates a [`DeckSlot`] if `slot < count`, or returns `None`.
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

/// Noise generator configuration declared for a parameter binding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BindNoise {
    /// Noise kind: `white`, `value`, `perlin`, or `fbm`.
    #[serde(default = "default_noise_kind")]
    pub kind: String,
    /// Cycles per beat, making period tempo-relative.
    #[serde(default = "default_noise_rate")]
    pub rate: f32,
    /// Stream ID to decorrelate bindings.
    #[serde(default)]
    pub stream: u64,
    /// Octave count for `fbm` noise.
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

/// Default camera height when omitted from serialized records.
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

/// Signal driving a parameter in a [`Record::Source`] session event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
    /// Bus signal name (e.g. `energy`, `beat`, `noise`) or `control:<name>`.
    pub signal: String,
    /// Mapping curve (`lin`, `pow2`, `sqrt`, `smooth`).
    pub curve: String,
    /// Target mapping range `[low, high]`.
    pub range: [f32; 2],
    /// Optional noise generator configuration if `signal == "noise"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub noise: Option<BindNoise>,
}

/// The master chain slots in sequential execution order.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chain {
    /// Chain slots executed in order.
    pub slots: Vec<ChainSlot>,
}

/// One slot configuration in the master chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChainSlot {
    /// Content address of the procedure.
    #[serde(rename = "proc")]
    pub procedure: String,
    /// Retained frame input (`mix` or `exit`), or `None` if omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cut: Option<String>,
    /// Parameter overrides for the slot procedure, keyed by parameter name.
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
