use crate::mix::Input;
use crate::swap::HotSwap;
use crate::transport::Transport;

/// Maximum number of slots a deck can hold.
pub const MAX_SLOTS: usize = 4;

/// Slot index within the deck mixer.
///
/// Distinct from [`crate::master::Slot`], which represents an effect pass in the master chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeckSlot(pub u8);

impl DeckSlot {
    /// Returns a validated slot index if `slot < count`.
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

/// Clamps gain to non-negative finite values, treating NaN as zero.
pub(crate) fn clamp_gain(gain: f32) -> f32 {
    if gain.is_nan() {
        0.0
    } else {
        gain.max(0.0)
    }
}

/// Clamps opacity to the unit interval `[0.0, 1.0]`, treating NaN as zero.
pub(crate) fn clamp_opacity(opacity: f32) -> f32 {
    if opacity.is_nan() {
        0.0
    } else {
        opacity.clamp(0.0, 1.0)
    }
}

/// Spatial mask applied to a slot layer's opacity during composition.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Mask {
    kind: MaskKind,
    /// Orientation angle for linear masks in radians.
    angle: f32,
    /// Reveal progression in `[0.0, 1.0]`.
    position: f32,
    /// Transition edge softness in `[0.0, 1.0]`.
    softness: f32,
}

/// Shape geometry for a slot mask.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MaskKind {
    /// Full frame reveal without masking.
    #[default]
    None,
    /// Linear transition boundary across the frame.
    Linear,
    /// Centered radial circular mask.
    Radial,
}

impl MaskKind {
    /// All mask geometries in cycle order.
    pub const ALL: [MaskKind; 3] = [MaskKind::None, MaskKind::Linear, MaskKind::Radial];

    /// Wire and display name for this mask geometry.
    pub fn name(self) -> &'static str {
        match self {
            MaskKind::None => "none",
            MaskKind::Linear => "linear",
            MaskKind::Radial => "radial",
        }
    }

    /// Parses a mask geometry name, or returns None if unrecognised.
    pub fn from_name(name: &str) -> Option<MaskKind> {
        MaskKind::ALL.iter().copied().find(|k| k.name() == name)
    }

    /// Numeric index encoded for shader uniform consumption.
    pub(crate) fn index(self) -> u32 {
        match self {
            MaskKind::None => 0,
            MaskKind::Linear => 1,
            MaskKind::Radial => 2,
        }
    }
}

impl Default for Mask {
    fn default() -> Mask {
        Mask {
            kind: MaskKind::None,
            angle: 0.0,
            position: 1.0,
            softness: 0.0,
        }
    }
}

impl Mask {
    /// Creates a mask with values clamped to normalized shader ranges.
    pub fn new(kind: MaskKind, angle: f32, position: f32, softness: f32) -> Mask {
        Mask {
            kind,
            angle: if angle.is_finite() { angle } else { 0.0 },
            position: clamp_unit(position),
            softness: clamp_unit(softness),
        }
    }

    pub fn kind(self) -> MaskKind {
        self.kind
    }

    pub fn angle(self) -> f32 {
        self.angle
    }

    pub fn position(self) -> f32 {
        self.position
    }

    pub fn softness(self) -> f32 {
        self.softness
    }

    /// Returns a copy of the mask with its reveal position set to `position`.
    pub fn at(self, position: f32) -> Mask {
        Mask {
            position: clamp_unit(position),
            ..self
        }
    }

    /// Returns true if the mask completely hides the slot content.
    pub(crate) fn hides_everything(self) -> bool {
        self.kind != MaskKind::None && self.position == 0.0
    }
}

/// Clamps values to `[0.0, 1.0]`, treating NaN as zero.
fn clamp_unit(x: f32) -> f32 {
    if x.is_nan() {
        0.0
    } else {
        x.clamp(0.0, 1.0)
    }
}

/// Blend mode used to fold a slot layer into the composite accumulation target (Add, Over, Max).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Blend {
    /// Additive sum of colour values.
    #[default]
    Add,
    /// Alpha over blending against accumulated coverage.
    Over,
    /// Component-wise maximum value.
    Max,
}

impl Blend {
    /// All blend modes in cycle order.
    pub const ALL: [Blend; 3] = [Blend::Add, Blend::Over, Blend::Max];

    /// Wire and display name for this blend mode.
    pub fn name(self) -> &'static str {
        match self {
            Blend::Add => "add",
            Blend::Over => "over",
            Blend::Max => "max",
        }
    }

    /// Parses a blend mode name, or returns None if unrecognised.
    pub fn from_name(name: &str) -> Option<Blend> {
        Blend::ALL.iter().copied().find(|b| b.name() == name)
    }

    /// Numeric index encoded for shader uniform consumption.
    pub(crate) fn index(self) -> u32 {
        match self {
            Blend::Add => 0,
            Blend::Over => 1,
            Blend::Max => 2,
        }
    }

    /// Returns true if the given gain and opacity silence the slot in this blend mode.
    pub(crate) fn silent_at(self, gain: f32, opacity: f32) -> bool {
        opacity == 0.0 || (gain == 0.0 && self != Blend::Over)
    }
}

/// Lifecycle and composition state of a slot in the deck.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Residency {
    /// Stepped, drawn, and folded into the composite mix.
    Live,
    /// Stepped and drawn into preview targets, but excluded from the composite mix.
    Priming,
    /// Stepped and drawn for monitoring, but unbudgeted for on-air composition.
    Allocated,
}

/// Resident slot entry holding a Set, render targets, and mixer edge controls.
pub(crate) struct Slot {
    pub(crate) swap: HotSwap,
    /// Requested residency set by the operator.
    pub(crate) requested: Residency,
    /// Effective residency granted by the governor.
    pub(crate) effective: Residency,
    /// Linear colour gain applied before blending.
    pub(crate) gain: f32,
    /// Opacity fader scaling blend contribution in `[0.0, 1.0]`.
    pub(crate) opacity: f32,
    /// Blend mode for folding this slot into the accumulation target.
    pub(crate) blend: Blend,
    /// Spatial reveal mask.
    pub(crate) mask: Mask,
    /// Transport mapping governing clock advancement.
    pub(crate) transport: Transport,
    /// Slot online state in composite mix (arbitrated by mixer solo/mute or directly set).
    pub(crate) online: bool,
    pub(crate) target: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
}

impl Slot {
    /// Constructs the composite input descriptor for this slot.
    pub(crate) fn edge(&self) -> Input {
        Input {
            gain: self.gain,
            opacity: self.opacity,
            blend: self.blend,
            mask: self.mask,
            live: true,
        }
    }
}
