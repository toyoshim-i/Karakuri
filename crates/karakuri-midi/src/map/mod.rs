//! Declarative mapping table translating incoming MIDI messages to operations and vice versa.
//!
//! ## Mapping Specification
//!
//! Maps MIDI Control Change (`cc`), 14-bit CC (`cc14`), and Note (`note`) events to discrete
//! or continuous operations:
//!
//! ```text
//!   # slot faders, on the channel the surface is set to
//!   cc 1 ch 1 -> gain 0
//!   cc 5      -> opacity 0
//!   cc 9      -> mask-position 0
//!   cc 20     -> exposure
//!
//!   # pads
//!   note 36 -> residency 0 live
//!   note 40 -> residency 1 priming
//!   note 44 -> blend 0 over
//!   note 48 -> residency 2 allocated
//!   note 56 -> tap
//!
//!   # a control of the Set on a deck, by its place in the interface
//!   cc 30   -> param 0 3
//! ```
//!
//! ## Invariants & Grammar Rules
//!
//! - **State-targeting semantics**: Mapping rules target absolute states or setpoints rather than
//!   relative toggle steps (Principle 0090, ADR-0196).
//! - **Positional parameter addressing**: Published Set parameters are addressed by deck and slot
//!   position (`cc -> param <deck> <position>`) rather than procedure name (ADR-0268).
//! - **14-bit CC handling**: High-resolution faders (`cc14 <msb> <lsb>`) map 14-bit ranges
//!   (`[0, 16383]`). Lone MSBs execute coarse updates immediately; incoming LSBs refine the value
//!   without timer delays.
//! - **Bi-directional feedback (`Echo`)**: Maps surface controls to outgoing wire representations
//!   to drive LED indicators and motorized faders.

use std::collections::HashMap;

use karakuri_operation::{BlendMode, Operation, Residency};

pub(crate) mod parse;
pub mod types;

pub(crate) use parse::*;
pub use types::*;
/// Mapping table associating incoming MIDI messages with target operations and parameters.
#[derive(Debug, Default, Clone)]
pub struct Map {
    entries: HashMap<Key, Entry>,
    /// Maps LSB keys to their corresponding MSB controller indices.
    fine: HashMap<Key, u8>,
}

impl Map {
    /// Parse a map file. Every line is reported on its own terms: one bad line
    /// is a line an operator can fix, and refusing the file for it would make a
    /// typo cost a whole surface mid-set.
    ///
    /// Returns the map and the complaints, and **both are meant to be used**:
    /// a caller that dropped the second would leave an operator pressing a pad
    /// that was never mapped, with nothing said.
    pub fn parse(text: &str) -> (Map, Vec<String>) {
        let mut map = Map::default();
        let mut notes = Vec::new();
        for (i, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            match parse_line(line) {
                Ok((key, entry)) => {
                    if let (Key::Cc { channel, .. }, Some(lsb)) = (key, entry.lsb) {
                        map.fine.insert(
                            Key::Cc {
                                channel,
                                controller: lsb,
                            },
                            match key {
                                Key::Cc { controller, .. } => controller,
                                Key::Note { note, .. } => note,
                            },
                        );
                    }
                    if map.entries.insert(key, entry).is_some() {
                        notes.push(format!(
                            "line {}: this message was already mapped; the later line wins",
                            i + 1
                        ));
                    }
                }
                Err(message) => notes.push(format!("line {}: {message}", i + 1)),
            }
        }
        // **A controller cannot be a line of its own and half of a pair**, and
        // which of the two the file meant is not something the order of the
        // lines should decide. `entries` wins, because a plain line is the
        // whole of what it says; the pair keeps its coarse half and loses its
        // fine one, which is a fader at 128 positions rather than a fader that
        // went quiet — and it is said here rather than discovered.
        for key in map.fine.keys() {
            if map.entries.contains_key(key) {
                notes.push(format!(
                    "`{}` is a line of its own and is also the LSB half of a `cc14` line; the \
                     plain line wins and the pair stays 7-bit",
                    key.spelled()
                ));
            }
        }
        (map, notes)
    }

    /// Binds `message` to `target` and returns the generated map line string.
    pub fn learn(&mut self, message: crate::Message, target: &str) -> Result<String, String> {
        use crate::Message;
        let key = match message {
            Message::ControlChange { controller, .. } => Key::Cc {
                channel: None,
                controller,
            },
            Message::NoteOn { note, .. } | Message::NoteOff { note, .. } => Key::Note {
                channel: None,
                note,
            },
        };
        let line = format!("{} -> {target}", key.spelled());
        let (key, entry) = parse_line(&line)?;
        self.entries.insert(key, entry);
        Ok(line)
    }

    /// Translates `message` to an [`Operation`], or `None` if unmapped.
    pub fn operation(&self, message: crate::Message) -> Option<Operation> {
        operating(self.target(message)?, coarse(message))
    }

    /// Returns 14-bit pair component information for `message`, or `None` if not mapped as `cc14`.
    pub fn wide(&self, message: crate::Message) -> Option<Wide> {
        let crate::Message::ControlChange {
            channel,
            controller,
            ..
        } = message
        else {
            return None;
        };
        if let Some(entry) = self.entry(channel, controller) {
            return entry.lsb.map(|_| Wide {
                control: controller,
                half: Half::Msb,
            });
        }
        let msb = *self
            .fine
            .get(&Key::Cc {
                channel: Some(channel),
                controller,
            })
            .or_else(|| {
                self.fine.get(&Key::Cc {
                    channel: None,
                    controller,
                })
            })?;
        // **The pair is checked from the other end**, so a knob whose MSB half
        // was re-learned as a plain `cc` line does not go on being refined by
        // a controller that is no longer its LSB. `learn` writes into
        // `entries` and leaves `fine` alone, which is what makes this a
        // question rather than an assumption.
        (self.entry(channel, msb)?.lsb == Some(controller)).then_some(Wide {
            control: msb,
            half: Half::Lsb,
        })
    }

    /// Translates an assembled 14-bit value (`0..=16383`) into an [`Operation`].
    pub fn operation_wide(&self, message: crate::Message, value: u16) -> Option<Operation> {
        operating(self.wide_target(message)?, Some(fine(value)))
    }

    /// Translates an assembled 14-bit value into a [`Parameter`].
    pub fn parameter_wide(&self, message: crate::Message, value: u16) -> Option<Parameter> {
        parametered(self.wide_target(message)?, Some(fine(value)))
    }

    /// The target a 14-bit message's **pair** is on, whichever half arrived.
    fn wide_target(&self, message: crate::Message) -> Option<Target> {
        let crate::Message::ControlChange { channel, .. } = message else {
            return None;
        };
        let wide = self.wide(message)?;
        self.entry(channel, wide.control).map(|entry| entry.target)
    }

    /// The entry a control change reaches: the channel it arrived on wins over
    /// a line that named no channel, which is [`Map::target`]'s own order.
    fn entry(&self, channel: u8, controller: u8) -> Option<Entry> {
        self.entries
            .get(&Key::Cc {
                channel: Some(channel),
                controller,
            })
            .or_else(|| {
                self.entries.get(&Key::Cc {
                    channel: None,
                    controller,
                })
            })
            .copied()
    }

    /// Returns sorted feedback descriptors for all mapped controls.
    pub fn echoes(&self) -> Vec<Echo> {
        let mut echoes: Vec<Echo> = self
            .entries
            .iter()
            .map(|(key, entry)| Echo {
                key: *key,
                lsb: entry.lsb,
                target: entry.target,
            })
            .collect();
        echoes.sort_by_key(Echo::order);
        echoes
    }

    /// Returns [`Parameter`] destination and value for a `param` mapping, or `None` if unmapped.
    pub fn parameter(&self, message: crate::Message) -> Option<Parameter> {
        parametered(self.target(message)?, coarse(message))
    }

    /// Returns an iterator of map lines formatted for `.kmap` persistence.
    pub fn lines(&self) -> impl Iterator<Item = String> + '_ {
        self.entries.iter().map(|(key, entry)| {
            format!(
                "{} -> {}",
                key.spelled_with(entry.lsb),
                entry.target.spelled()
            )
        })
    }

    /// Returns the input message syntax bound to `target` (for tooltips and UI readouts).
    pub fn bound(&self, target: &str) -> Option<String> {
        self.entries
            .iter()
            .find(|(_, held)| held.target.spelled() == target)
            .map(|(key, entry)| key.spelled_with(entry.lsb))
    }

    /// Returns true if `message` maps to a continuous control (fader/dial).
    pub fn is_continuous(&self, message: crate::Message) -> bool {
        // **Either half of a pair is the fader it is half of.** An LSB is not
        // in `entries` — a pair is one mapping — so asking `target` alone
        // answered `false` for it, and a caller coalescing a frame's messages
        // read the fine half of a sweep as a *press* and emitted it beside the
        // coarse one. Still one predicate and not a list: `Target::continuous`
        // is asked about the same target either way round.
        self.target(message)
            .or_else(|| self.wide_target(message))
            .is_some_and(Target::continuous)
    }

    /// What this message is mapped to, before its value is known. The channel
    /// it arrived on wins over a line that named no channel.
    fn target(&self, message: crate::Message) -> Option<Target> {
        use crate::Message;
        match message {
            Message::ControlChange {
                channel,
                controller,
                ..
            } => self
                .find(Key::Cc {
                    channel: Some(channel),
                    controller,
                })
                .or_else(|| {
                    self.find(Key::Cc {
                        channel: None,
                        controller,
                    })
                }),
            Message::NoteOn { channel, note, .. } => self
                .find(Key::Note {
                    channel: Some(channel),
                    note,
                })
                .or_else(|| {
                    self.find(Key::Note {
                        channel: None,
                        note,
                    })
                }),
            // A release is nothing here for [`Map::operation`]'s reason: every
            // pad is a press, so a release names no target rather than the
            // target the press named.
            Message::NoteOff { .. } => None,
        }
    }

    fn find(&self, key: Key) -> Option<Target> {
        self.entries.get(&key).map(|entry| entry.target)
    }

    /// How many mappings loaded. For the line an operator reads on startup:
    /// a map that parsed to nothing and a map that was never given are the
    /// same silence otherwise.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Evaluates target operation for normalized position `at` (or None for buttons/pads).
fn operating(target: Target, at: Option<f32>) -> Option<Operation> {
    let shape = target.shape();
    {
        match (target, at) {
            (Target::Gain { slot, range }, Some(v)) => Some(Operation::SetGain {
                deck: slot.0,
                gain: scale(v, range, shape),
            }),
            (Target::Opacity { slot, range }, Some(v)) => Some(Operation::SetOpacity {
                deck: slot.0,
                opacity: scale(v, range, shape),
            }),
            (Target::Exposure { range }, Some(v)) => Some(Operation::SetExposure {
                exposure: scale(v, range, shape),
            }),
            (Target::MaskPosition { slot, range }, Some(v)) => Some(Operation::SetMaskPosition {
                deck: slot.0,
                position: scale(v, range, shape),
            }),
            (Target::Residency { slot, residency }, None) => Some(Operation::SetResidency {
                deck: slot.0,
                residency,
            }),
            (Target::Blend { slot, blend }, None) => Some(Operation::SetBlendMode {
                deck: slot.0,
                blend,
            }),
            (Target::Tap, None) => Some(Operation::TapBeat),
            // **The one target this cannot finish**, and it is answered by
            // [`Map::parameter`] instead. A published control is addressed by
            // its *position*, and a position becomes a `ParamAt` only against
            // the Set that is in the deck — which is a readback, and this
            // function has none by charter. Whoever holds the deck completes
            // it; see `karakuri_environment::midi::Router`.
            (Target::Param { .. }, _) => None,
            _ => None,
        }
    }
}

/// [`operating`] for the one target it cannot finish — the `param` line's
/// address and where the knob is on its span.
fn parametered(target: Target, at: Option<f32>) -> Option<Parameter> {
    let Target::Param {
        slot,
        position,
        range,
    } = target
    else {
        return None;
    };
    Some(Parameter {
        deck: slot.0,
        position,
        at: at?,
        range,
    })
}

/// Normalizes a 7-bit Control Change value to `[0.0, 1.0]`.
fn coarse(message: crate::Message) -> Option<f32> {
    match message {
        crate::Message::ControlChange { value, .. } => Some(f32::from(value.min(127)) / 127.0),
        _ => None,
    }
}

/// Normalizes an assembled 14-bit pair (`0..=16383`) to `[0.0, 1.0]`.
fn fine(value: u16) -> f32 {
    f32::from(value.min(16383)) / 16383.0
}

/// Scales normalized parameter `t` in `[0.0, 1.0]` onto `range` using `shape`.
fn scale(t: f32, range: [f32; 2], shape: Shape) -> f32 {
    let [lo, hi] = range;
    match shape {
        Shape::Linear => lo + t * (hi - lo),
        // `t` of 0 and 1 give `lo * 1` and `lo * (hi/lo)`, so the bottom is
        // exact and the top is exact wherever the division and the power are —
        // the default `[0.25, 4]` among them. `parse_target` refuses a ratio
        // range with a zero or a negative in it, which is what makes the
        // logarithm defined at all.
        Shape::Ratio => lo * (hi / lo).powf(t),
    }
}

#[cfg(test)]
mod tests;
