use karakuri_operation::{BlendMode, Residency};

/// A continuous control's shape, which is a property of what it moves rather
/// than of the mapping that reaches it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shape {
    /// Equal steps in value. A fader is one: the same physical move means the
    /// same amount wherever it happens.
    Linear,
    /// Equal steps in ratio. Exposure is one — a stop is a doubling — and a
    /// linear map over `[1/64, 64]` would spend its bottom 2% on everything
    /// below unity.
    Ratio,
}

/// Deck slot position (0..3) addressed by a map line.
///
/// Dedicated type isolating deck slot positions from bare integer indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct DeckSlot(pub(crate) u8);

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

/// Destination and configuration mapped to a MIDI control.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Target {
    Gain {
        slot: DeckSlot,
        range: [f32; 2],
    },
    Opacity {
        slot: DeckSlot,
        range: [f32; 2],
    },
    Exposure {
        range: [f32; 2],
    },
    MaskPosition {
        slot: DeckSlot,
        range: [f32; 2],
    },
    Residency {
        slot: DeckSlot,
        residency: Residency,
    },
    Blend {
        slot: DeckSlot,
        blend: BlendMode,
    },
    Tap,
    /// Published Set parameter on a deck, addressed by 1-based interface position (`cc 30 -> param 0 3`).
    Param {
        slot: DeckSlot,
        position: u16,
        range: Option<[f32; 2]>,
    },
}

impl Target {
    pub(crate) fn shape(self) -> Shape {
        match self {
            Target::Exposure { .. } => Shape::Ratio,
            _ => Shape::Linear,
        }
    }

    /// Canonical text representation of the target in a map file.
    pub fn spelled(self) -> String {
        match self {
            Target::Gain { slot, .. } => format!("gain {slot}"),
            Target::Opacity { slot, .. } => format!("opacity {slot}"),
            Target::Exposure { .. } => "exposure".to_string(),
            Target::MaskPosition { slot, .. } => format!("mask-position {slot}"),
            Target::Residency { slot, residency } => {
                format!("residency {slot} {}", residency.name())
            }
            Target::Blend { slot, blend } => format!("blend {slot} {}", blend.name()),
            Target::Tap => "tap".to_string(),
            Target::Param { slot, position, .. } => format!("param {slot} {position}"),
        }
    }

    /// Whether this is moved by a fader or hit by a pad. A mapping that puts a
    /// pad on a fader's target is refused at parse time rather than putting a
    /// deck on air every time a knob passes a threshold.
    pub(crate) fn continuous(self) -> bool {
        matches!(
            self,
            Target::Gain { .. }
                | Target::Opacity { .. }
                | Target::Exposure { .. }
                | Target::MaskPosition { .. }
                | Target::Param { .. }
        )
    }
}

/// Default value range for continuous controls.
pub(crate) const GAIN_RANGE: [f32; 2] = [0.0, 1.0];
pub(crate) const OPACITY_RANGE: [f32; 2] = [0.0, 1.0];
pub(crate) const EXPOSURE_RANGE: [f32; 2] = [0.25, 4.0];
pub(crate) const MASK_POSITION_RANGE: [f32; 2] = [0.0, 1.0];

/// Which message a mapping is keyed by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Key {
    /// A control change. `channel` is `None` for "whatever channel it arrives
    /// on", which is the right default: a surface is usually the only thing
    /// plugged in, and an operator who has to discover their controller's
    /// channel before anything works has a map that does not load.
    Cc {
        channel: Option<u8>,
        controller: u8,
    },
    Note {
        channel: Option<u8>,
        note: u8,
    },
}

impl Key {
    /// Canonical text representation of the key on the left side of a map line.
    pub(crate) fn spelled(self) -> String {
        self.spelled_with(None)
    }

    /// [`Key::spelled`], and the LSB half where this is a `cc14` line's — the
    /// whole left-hand side, so a readout and a written-back line say what the
    /// file says. `None` is a plain `cc` or a `note`.
    pub(crate) fn spelled_with(self, lsb: Option<u8>) -> String {
        let (head, channel) = match (self, lsb) {
            (
                Key::Cc {
                    channel,
                    controller,
                },
                None,
            ) => (format!("cc {controller}"), channel),
            (
                Key::Cc {
                    channel,
                    controller,
                },
                Some(lsb),
            ) => (format!("cc14 {controller} {lsb}"), channel),
            (Key::Note { channel, note }, _) => (format!("note {note}"), channel),
        };
        match channel {
            Some(channel) => format!("{head} ch {}", channel + 1),
            None => head,
        }
    }
}

/// Control message targeting a deck's published parameter interface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Parameter {
    /// Zero-based deck slot index.
    pub deck: u8,
    /// 1-based position in the deck's published parameter interface.
    pub position: u16,
    /// Normalized position on parameter span in `[0, 1]`.
    pub at: f32,
    /// Explicit range override from map line, or None to use published range.
    pub range: Option<[f32; 2]>,
}

impl Parameter {
    /// Evaluates requested value across `range` (if specified) or `declared` range.
    pub fn value(self, declared: [f32; 2]) -> f32 {
        let [lo, hi] = self.range.unwrap_or(declared);
        lo + self.at * (hi - lo)
    }
}

/// 14-bit control component and parent pair identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wide {
    /// The MSB half's controller index naming the pair.
    pub control: u8,
    /// Half of the 14-bit pair represented by this message.
    pub half: Half,
}

/// The two halves of a 14-bit Control Change message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Half {
    /// Top 7 bits (MSB).
    Msb,
    /// Bottom 7 bits (LSB).
    Lsb,
}

/// Address of a mapped control target on a deck or global instrument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Gain {
        deck: u8,
    },
    Opacity {
        deck: u8,
    },
    Exposure,
    MaskPosition {
        deck: u8,
    },
    /// Pad controlling deck residency state.
    Residency {
        deck: u8,
        residency: Residency,
    },
    /// Pad controlling blend mode.
    Blend {
        deck: u8,
        blend: BlendMode,
    },
    /// Published parameter addressed by 1-based position.
    Param {
        deck: u8,
        position: u16,
    },
    /// Tap beat trigger.
    Tap,
}

/// Current hardware control state to reflect on a feedback surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shown {
    /// Continuous control at `value` in native units.
    At(f32),
    /// Published Set control at `value` with Set `declared` range.
    Published { value: f32, declared: [f32; 2] },
    /// State of a pad toggle/selector.
    On(bool),
}

/// Descriptor of a mapped control for MIDI output feedback.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Echo {
    pub(crate) key: Key,
    pub(crate) lsb: Option<u8>,
    pub(crate) target: Target,
}

impl Echo {
    /// Address of the control represented by this feedback mapping.
    pub fn control(self) -> Control {
        match self.target {
            Target::Gain { slot, .. } => Control::Gain { deck: slot.0 },
            Target::Opacity { slot, .. } => Control::Opacity { deck: slot.0 },
            Target::Exposure { .. } => Control::Exposure,
            Target::MaskPosition { slot, .. } => Control::MaskPosition { deck: slot.0 },
            Target::Residency { slot, residency } => Control::Residency {
                deck: slot.0,
                residency,
            },
            Target::Blend { slot, blend } => Control::Blend {
                deck: slot.0,
                blend,
            },
            Target::Tap => Control::Tap,
            Target::Param { slot, position, .. } => Control::Param {
                deck: slot.0,
                position,
            },
        }
    }

    /// Returns normalized wire position for `shown` (0..127 for 7-bit, 0..16383 for 14-bit).
    pub fn position(self, shown: Shown) -> u16 {
        let steps = f32::from(self.steps());
        let (range, value) = match (self.target, shown) {
            (_, Shown::On(on)) => return if on { 127 } else { 0 },
            (Target::Gain { range, .. }, Shown::At(value))
            | (Target::Opacity { range, .. }, Shown::At(value))
            | (Target::Exposure { range }, Shown::At(value))
            | (Target::MaskPosition { range, .. }, Shown::At(value)) => (range, value),
            (Target::Param { range, .. }, Shown::Published { value, declared }) => {
                (range.unwrap_or(declared), value)
            }
            // A press asked about as a position, or a fader asked about as a
            // published control: the caller answered a control it was not
            // asked about, and 0 is the end nothing lights at.
            _ => return 0,
        };
        let [lo, hi] = range;
        let at = match self.target.shape() {
            Shape::Linear => (value - lo) / (hi - lo),
            // `parse_target` refuses a ratio range that reaches zero, which is
            // what makes both logarithms defined.
            Shape::Ratio => (value / lo).ln() / (hi / lo).ln(),
        };
        if !at.is_finite() {
            return 0;
        }
        (at.clamp(0.0, 1.0) * steps).round() as u16
    }

    /// Appends wire messages representing `position` to `into` (1 message for 7-bit/pad, 2 for 14-bit pair).
    pub fn wire(self, position: u16, into: &mut Vec<[u8; 3]>) {
        let channel = match self.key {
            Key::Cc { channel, .. } | Key::Note { channel, .. } => channel.unwrap_or(0) & 0x0f,
        };
        match (self.key, self.lsb) {
            (Key::Note { note, .. }, _) => {
                into.push([0x90 | channel, note, if position == 0 { 0 } else { 127 }]);
            }
            (Key::Cc { controller, .. }, None) => {
                into.push([0xb0 | channel, controller, position.min(127) as u8]);
            }
            (Key::Cc { controller, .. }, Some(lsb)) => {
                let value = position.min(16383);
                into.push([0xb0 | channel, controller, (value >> 7) as u8]);
                into.push([0xb0 | channel, lsb, (value & 0x7f) as u8]);
            }
        }
    }

    /// The top of this control's span: 16383 for a pair and 127 for
    /// everything else.
    fn steps(self) -> u16 {
        match self.lsb {
            Some(_) => 16383,
            None => 127,
        }
    }

    /// A stable order for [`Map::echoes`] — pads after faders, then by the
    /// number on the wire and the channel, so the same map lights a surface in
    /// the same order on every run.
    pub(crate) fn order(&self) -> (u8, u8, u8) {
        match self.key {
            Key::Cc {
                channel,
                controller,
            } => (0, controller, channel.unwrap_or(0)),
            Key::Note { channel, note } => (1, note, channel.unwrap_or(0)),
        }
    }
}

/// Single mapping table entry associating a message key with a target and optional 14-bit LSB.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Entry {
    pub(crate) target: Target,
    /// Controller index of the LSB half for a 14-bit CC pair.
    pub(crate) lsb: Option<u8>,
}
