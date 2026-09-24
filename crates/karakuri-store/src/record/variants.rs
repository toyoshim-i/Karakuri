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
        /// What this Set calls the node, for whatever wants to point at it (see `docs/ir-spec.md`).
        ///
        /// A name belongs to the use rather than the procedure, allowing identical procedures
        /// to be loaded under distinct names without collision. It is not part of the address.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(rename = "proc")]
        proc_hash: Hash,
    },
    /// A node of an authoring Set file (`.kset`), referencing procedure source by relative path.
    Part {
        /// Target node layer.
        layer: Layer,
        /// Node index within that layer. Zero is omitted during serialization.
        #[serde(default, skip_serializing_if = "is_zero")]
        index: u32,
        /// Optional human-readable node name.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        /// Procedure source path relative to the containing setfile directory.
        path: String,
    },
    /// Overrides the default capacity of a node geometry.
    Capacity {
        /// Target node address.
        #[serde(flatten)]
        at: NodeAddress,
        value: u32,
    },
    /// Parameter value for a node or every node declaring the key.
    Param {
        /// Target node address, or `None` to target all nodes declaring `key`.
        #[serde(flatten, with = "node_or_every_node")]
        at: Option<NodeAddress>,
        key: String,
        value: Value,
    },
    /// Attaches an input signal to a node parameter.
    Bind {
        /// Target layer declaring the parameter.
        layer: Layer,
        /// Target node index within the layer, or `None` for all nodes in the layer.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<u32>,
        key: String,
        signal: String,
        curve: String,
        range: [f32; 2],
        /// Noise configuration if `signal == "noise"`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        noise: Option<BindNoise>,
    },
    /// Camera configuration for an L3 producer.
    Camera {
        kind: String,
        /// Target camera index in layer L3. Zero is omitted during serialization.
        #[serde(default, skip_serializing_if = "is_zero")]
        index: u32,
        radius: f32,
        speed: f32,
        /// Camera height offset relative to target.
        #[serde(default = "default_camera_height")]
        height: f32,
    },
    /// Configures renderer compositing for a Set through an L5 merge node.
    Merge {
        /// Zero-based index of the solo live renderer, or `None` if all renderers are live.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        live: Option<u32>,
    },
    /// Seed value for hash builtins on a specific node.
    Seed {
        stream: Layer,
        /// Target node index within the layer. Zero is omitted during serialization.
        #[serde(default, skip_serializing_if = "is_zero")]
        index: u32,
        value: u64,
    },
    /// Binds a declared input port of one node to the output of another node.
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
    // -- Session mix state -----------------------------------------------
    /// Linear gain of a deck slot into the mix.
    Gain {
        slot: DeckSlot,
        value: f32,
    },
    /// Fader level of a deck slot controlling blend opacity in `[0.0, 1.0]`.
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
    /// How a deck slot's layer meets the ones under it (`add`, `over`, or `max`).
    Blend {
        slot: DeckSlot,
        mode: String,
    },
    /// Requested residency level for a deck slot (`live`, `priming`, or `allocated`).
    Residency {
        slot: DeckSlot,
        level: String,
    },
    /// A deck slot's MCP modification policy: `auto`, `on` or `off`.
    Policy {
        slot: DeckSlot,
        policy: String,
    },
    /// Session tone mapping and exposure settings.
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
    /// Procedure assigned to a deck slot node.
    Procedure {
        slot: DeckSlot,
        /// Target node within the deck slot.
        #[serde(flatten)]
        at: NodeAddress,
        #[serde(rename = "proc")]
        proc_hash: Hash,
    },
    /// Operator/agent authority level for a node (`manual`, `suggesting`, or `automatic`).
    Authority {
        slot: DeckSlot,
        /// Target node within the deck slot.
        #[serde(flatten)]
        at: NodeAddress,
        authority: String,
    },
    /// Real-time parameter automation recorded during live performance.
    Ride {
        /// Target deck slot.
        slot: DeckSlot,
        /// Target node address, or `None` for wildcard broadcast.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        at: Option<NodeAddress>,
        key: String,
        value: Value,
    },
    /// Attaches or detaches a modulation signal for a parameter of a playing deck slot.
    Source {
        slot: DeckSlot,
        layer: Layer,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<u32>,
        key: String,
        /// Signal configuration to attach, or `None` to detach.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<Source>,
    },
    /// Session render canvas resolution in texels.
    Canvas {
        width: u32,
        height: u32,
    },
    /// Spatial framing mask for a deck slot layer.
    Mask {
        slot: DeckSlot,
        /// `none`, `linear` or `radial`.
        kind: String,
        /// Which way a linear front runs, in radians.
        angle: f32,
        /// Travel position in `[0.0, 1.0]`.
        position: f32,
        /// Edge softness in `[0.0, 1.0]`.
        softness: f32,
    },
    /// Smooth automated transition of a mix control over musical time.
    Transition {
        /// The deck slot whose control is moving.
        slot: DeckSlot,
        /// Control name (`gain`, `opacity`, or `mask`).
        control: String,
        /// Where the control ends up.
        to: f32,
        /// The musical instant it begins, in beats.
        start: f64,
        /// How long it lasts, in beats. Zero is a cut.
        beats: f64,
        /// Interpolation curve (`lin`, `pow2`, `sqrt`, or `smooth`).
        curve: String,
    },
    /// Instant cut selecting which renderer of a composited slot is live.
    Select {
        /// The deck slot whose Set the selection is inside.
        slot: DeckSlot,
        /// Zero-based renderer index within the slot.
        renderer: u32,
        /// The musical instant it lands on, in beats.
        start: f64,
    },
    /// Deck slot transport timing and sync configuration.
    Transport {
        slot: DeckSlot,
        sync: String,
        anchor_bpm: f32,
        scrub_beats: f64,
    },
    /// Records that a deck slot's state was saved to a Set file with the given ID.
    Save {
        slot: DeckSlot,
        id: String,
    },
    // -- What a frame saw or decided --------------------------------------
    /// How far this frame advances. Emitted from real time when live, read back
    /// verbatim on replay — which is what keeps substepping deterministic. Always 1
    /// in v0.2, capped at [`MAX_STEPS`].
    Tick {
        steps: u8,
    },
    /// Measured audio signals and frequency bands for the current frame.
    Audio {
        /// Broadband level in `[0.0, 1.0]`.
        energy: f32,
        /// Transient envelope in `[0.0, 1.0]`.
        onset: f32,
        /// Frequency band levels in `[0.0, 1.0]`, ordered lowest to highest band.
        bands: Vec<f32>,
        /// Measurement confidence in `[0.0, 1.0]`.
        confidence: f32,
    },
    /// Tempo and phase corrections applied to the session clock for the current frame.
    Tempo {
        /// Updated tempo in BPM.
        bpm: f32,
        /// Phase shift in beats.
        shift: f32,
        /// Confidence of the tempo estimate in `[0.0, 1.0]`.
        confidence: f32,
    },
    // -- Artifact metadata (<hash>.meta.ndjson) ---------------------------
    /// Header of an artifact metadata card (`<hash>.meta.ndjson`).
    Meta {
        hash: Hash,
        name: String,
        kind: Layer,
        v: u32,
    },
    /// Parameter declared by a procedure.
    ParamDecl {
        key: String,
        #[serde(rename = "type")]
        ty: String,
        min: f32,
        max: f32,
        /// Optional default scalar value when statically evaluable.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default: Option<f32>,
    },
    /// Supported element counts declared by an L1 procedure.
    CapacityDecl {
        min: u32,
        max: u32,
        default: u32,
    },
    /// Attributes emitted by a procedure in declaration order.
    Emit {
        attrs: Vec<String>,
    },
    /// Unrecognised record type preserved for forward compatibility.
    #[serde(other)]
    Unknown,
}
