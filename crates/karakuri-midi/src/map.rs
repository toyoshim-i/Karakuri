//! What a message means, as a table an operator writes.
//!
//! **A surface's numbers are the surface's.** There is no controller this
//! engine knows the layout of, and inventing one would be a table that fits one
//! device and misleads about every other. So the mapping is a file, the file is
//! the operator's, and what this module owns is the translation — not the
//! layout.
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
//! ## A line names a state, never a step
//!
//! **The value word is the grammar**, and it is what
//! [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
//! costs on this surface: a pad says `residency 2 live`, not *flip slot 2*. A
//! surface that could only step has no way to *arrive*, and two surfaces
//! stepping one control disagree about where they are — so a pad that means
//! *over* is one pad, and a mini that cycles the three is an affordance
//! whoever draws it builds over three lines of this file.
//!
//! `on-air N`, `prime N` and a bare `blend N` were the three that did not have
//! the shape, and **a file still holding one is refused on that line with the
//! line to write instead** — never loaded and silently re-read, because
//! `on-air 0` means *flip it* in a file written last month and would mean *put
//! it live* today
//! ([ADR-0196](../../../docs/adr/0196-a-map-line-names-a-state-and-an-old-line-is-refused.md)).
//! **`preview N | mix` had the shape and is gone**: ADR-0240 retired *Choose
//! what the output shows*, so `preview` is not a control at all any more and
//! is refused by the arm every unknown word is — the line is reported with its
//! number and the rest of the map loads.
//!
//! ## A parameter is reached by position, and this crate cannot finish the line
//!
//! `cc -> param <deck> <position>` is a control of the Set on that deck, by its
//! place in the published interface, counting from one — the number the
//! Inspector draws beside the row. **A position and never a name**
//! ([ADR-0268](../../../docs/adr/0268-a-vector-parameter-is-driven-one-component-at-a-time.md)):
//! *knob 3 is knob 3 whatever Set is loaded*, where a name would be a mapping
//! paid for again on every swap — and the same Set in two decks is two
//! addresses, because the deck is in the address.
//!
//! **It is the one target [`Map::operation`] answers `None` to**, and
//! [`Map::parameter`] answers it instead. A position becomes a `ParamAt` only
//! against the Set that is in the deck right now, which is a readback and this
//! module has none by charter; whoever holds the deck finishes it
//! (`karakuri_environment::midi::Interface`). The two accessors are disjoint by
//! target and a test holds that they are, so a target added to the grammar and
//! to neither is a compile-time list short rather than a knob that goes quiet.
//!
//! **The range on that line is optional and nowhere else is.** Every other
//! continuous target moves a control of the console's own, whose range this
//! crate can state; a published control's range is the *Set's*, so `None`
//! means *the range the Set published it over*, and a range on the line
//! overrides it as it does on a gain.
//!
//! **It is also what a learn writes** — [`Map::learn`], the one thing here
//! that changes a map
//! (`docs/adr/0336-a-learn-is-a-map-edit-and-the-tips-midi-line-is-the-live-map.md`).
//!
//! ## Half the mask, because half of it can be said here
//!
//! `cc -> mask-position N` is the front of a deck's mask, `[0, 1]`, and a hand
//! on it stops the wipe that was carrying it
//! ([P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
//! **The mask's other row has no line here**, and the reason is in this
//! grammar rather than in the vocabulary:
//! [`karakuri_operation::Operation::SetMaskShape`] carries an angle as well as
//! a kind, a line can say a slot number, a value word out of a list, or a
//! trailing `[lo, hi]` — and **none of those is a bare number**, so an angle
//! cannot be written. A pad that named a kind alone would have to invent the
//! angle beside it, and this crate reads nothing back to invent it *from*,
//! which is [ADR-0192](../../../docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)'s
//! fault one field along. So the row is left with no route rather than given a
//! lossy one, and what it would cost to give it one — a float form, an
//! angle-less operation, or the gap — is
//! [ADR-0202](../../../docs/adr/0202-the-map-reaches-the-masks-front-and-the-shape-has-no-spelling.md),
//! which names the three and takes none. **The third is taken**
//! ([ADR-0209](../../../docs/adr/0209-the-masks-shape-keeps-its-empty-midi-badge-until-a-control-shows-an-angle.md)):
//! this grammar stays as it is, and what would reopen it is a control that
//! *shows* an angle rather than any change here.
//!
//! ## What this deliberately does not do
//!
//! **It produces a [`karakuri_operation::Operation`], never a record and never
//! a call.** The engine is driven through the record stream and
//! `karakuri-operation-record` is where a record is built; a second place
//! building them would be two spellings of one rule, and the rule is the
//! invariant that a surface can do nothing a key cannot. What reaches the
//! engine from a fader is the same `gain` record a keypress writes, so **a
//! session recorded from a controller replays with no controller attached** —
//! and the map is not in the stream, because which knob was turned is a
//! property of the room's hardware rather than of the performance.
//!
//! **It reads nothing back.** [`Map::operation`] is a pure function of one
//! message, which is this module's whole test story: `parse`, then
//! `operation`, with no world to set up and nothing to mock. It is also why
//! `cc -> exposure` names [`karakuri_operation::Operation::SetExposure`] and
//! not a whole look — a map has no way to know the tone map operator a record
//! carries beside it, and the place that writes the record fills it in
//! ([ADR-0192](../../../docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)).
//!
//! ## A fader is 128 positions, or 16384 where the line says `cc14`
//!
//! A control change carries seven bits, and `cc 1 -> gain 0` reads them as 128
//! positions — about 0.8% of a gain's range per step, which is a real
//! coarseness and was the whole of what this crate did. **`cc14 <msb> <lsb> ->
//! <control>` is the pair**, the MIDI convention where one controller carries
//! the top seven bits and a second carries the bottom seven: the value is
//! `(msb << 7) | lsb` over 16384 positions, scaled onto the target's range
//! exactly as a 7-bit line's is.
//!
//! **Both numbers are written out.** The convention pairs `n` with `n + 32`
//! and controllers exist that do not honour it, so the line names the two it
//! means and an operator checks them against their device's manual rather than
//! against a convention. A pair that is one controller twice, or whose second
//! half is past 127, is refused at parse **naming both numbers**.
//!
//! ### The lone MSB moves the control coarsely and the LSB refines it
//!
//! A device sends the MSB first and the LSB after it, and the two are two
//! messages with a frame boundary free to fall between them. **An MSB on its
//! own is read as the 7-bit value it is** — `msb / 127`, the same reading
//! `cc <msb>` would give — and the LSB that follows re-states the control at
//! `((msb << 7) | lsb) / 16383`. So **both ends stay exact**: a fader at the
//! top sends MSB 127 and reaches 1.0 whether or not its LSB arrives, and a
//! surface that sends MSBs only is a 7-bit fader on the same line. **A lone
//! LSB moves nothing** — [`Map::operation`] answers `None` for it — because
//! there is nothing to refine until an MSB has been seen for that control.
//!
//! Nothing is held back and nothing is timed: **no message waits for its
//! partner**, so no fader is ever left between two values by a pair that did
//! not finish. Holding the MSB for a window instead is the alternative, and it
//! needs a clock on a route that has none (see this crate's *Latency is not
//! compensated here*) and leaves a coarse-only device stuck at its last
//! position for as long as the window lasts.
//!
//! **The one piece of state a pair needs is the caller's.** The MSB last seen
//! for a control has to live somewhere between two messages, and this module
//! is a pure function of one message: [`Map::wide`] says which half arrived
//! and which pair it belongs to, and [`Map::operation_wide`] takes the
//! assembled value. `karakuri_environment::midi::Router` is what holds the
//! halves, beside the frame's coalescing it already holds.
//!
//! ## The surface is shown what the deck holds
//!
//! The map read the other way: [`Map::echoes`] is every mapped control as an
//! [`Echo`], which says *what to read* ([`Echo::control`], an address rather
//! than a value) and *how to say it on the wire* ([`Echo::position`] and
//! [`Echo::wire`]). That is what makes an LED follow a residency and a
//! motorised fader follow a gain a transition is moving.
//!
//! **This crate still reads nothing back.** An `Echo` is handed a [`Shown`] —
//! where the control is, in its own units — by whoever holds the deck, and
//! answers the bytes. The inverse of [`scale`] is the whole of the
//! arithmetic, and it is exact at both ends for the same ranges the forward
//! direction is.

use std::collections::HashMap;

use karakuri_operation::{BlendMode, Operation, Residency};

/// A continuous control's shape, which is a property of what it moves rather
/// than of the mapping that reaches it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
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
struct DeckSlot(u8);

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
enum Target {
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
    fn shape(self) -> Shape {
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
    fn continuous(self) -> bool {
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
const GAIN_RANGE: [f32; 2] = [0.0, 1.0];
const OPACITY_RANGE: [f32; 2] = [0.0, 1.0];
const EXPOSURE_RANGE: [f32; 2] = [0.25, 4.0];
const MASK_POSITION_RANGE: [f32; 2] = [0.0, 1.0];

/// Which message a mapping is keyed by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Key {
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
    fn spelled(self) -> String {
        self.spelled_with(None)
    }

    /// [`Key::spelled`], and the LSB half where this is a `cc14` line's — the
    /// whole left-hand side, so a readout and a written-back line say what the
    /// file says. `None` is a plain `cc` or a `note`.
    fn spelled_with(self, lsb: Option<u8>) -> String {
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
    key: Key,
    lsb: Option<u8>,
    target: Target,
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
    fn order(&self) -> (u8, u8, u8) {
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
struct Entry {
    target: Target,
    /// Controller index of the LSB half for a 14-bit CC pair.
    lsb: Option<u8>,
}

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

fn parse_line(line: &str) -> Result<(Key, Entry), String> {
    let (from, to) = line
        .split_once("->")
        .ok_or_else(|| "expected `<message> -> <control>`".to_string())?;
    let (key, lsb) = parse_key(from.trim())?;
    let target = parse_target(to.trim())?;
    let is_note = matches!(key, Key::Note { .. });
    if is_note && target.continuous() {
        return Err("a note is a press, and this control takes a position; map a `cc`".to_string());
    }
    if !is_note && !target.continuous() {
        return Err(
            "a control change is a position, and this control takes a press; map a `note`"
                .to_string(),
        );
    }
    Ok((key, Entry { target, lsb }))
}

fn parse_key(from: &str) -> Result<(Key, Option<u8>), String> {
    let mut words = from.split_whitespace();
    let kind = words
        .next()
        .ok_or_else(|| "expected `cc`, `cc14` or `note`".to_string())?;
    let number: u8 = words
        .next()
        .ok_or_else(|| format!("`{kind}` needs a number"))?
        .parse()
        .map_err(|_| format!("`{kind}` needs a number in 0-127"))?;
    if number > 127 {
        return Err(format!("`{kind} {number}` is past 127"));
    }
    // **The LSB half, and both numbers are on the line.** The convention pairs
    // `n` with `n + 32` and controllers exist that do not honour it, so what a
    // line means is the two numbers it names rather than a convention plus
    // arithmetic — and every refusal below can then name both of them, which
    // is what an operator checks against their device's manual.
    let mut lsb = None;
    let mut after = words.next();
    if kind == "cc14" {
        let word = after.filter(|word| *word != "ch").ok_or_else(|| {
            format!(
                "`cc14 {number}` needs the controller its LSB half arrives on — write \
                 `cc14 {number} {}`, which is the usual pairing",
                u16::from(number) + 32
            )
        })?;
        let fine: u8 = word.parse().map_err(|_| {
            format!("`cc14 {number} {word}`: the LSB half is a controller number in 0-127")
        })?;
        if fine > 127 {
            return Err(format!("`cc14 {number} {fine}`: the LSB half is past 127"));
        }
        if fine == number {
            return Err(format!(
                "`cc14 {number} {fine}`: the two halves are one controller — an MSB and its LSB \
                 are two"
            ));
        }
        lsb = Some(fine);
        after = words.next();
    }
    let channel = match (after, words.next()) {
        (None, _) => None,
        (Some("ch"), Some(n)) => {
            // The front panel's spelling, 1-16, because that is what is printed
            // on the device an operator is reading it off. The wire's 0-15 is
            // `Message`'s and the translation happens here, once.
            let n: u8 = n
                .parse()
                .map_err(|_| format!("`ch {n}` needs a channel in 1-16"))?;
            if !(1..=16).contains(&n) {
                return Err(format!("`ch {n}` is outside 1-16"));
            }
            Some(n - 1)
        }
        (Some(other), _) => return Err(format!("expected `ch <1-16>`, found `{other}`")),
    };
    if words.next().is_some() {
        return Err("too many words before `->`".to_string());
    }
    Ok(match kind {
        "cc" | "cc14" => (
            Key::Cc {
                channel,
                controller: number,
            },
            lsb,
        ),
        "note" => (
            Key::Note {
                channel,
                note: number,
            },
            None,
        ),
        other => return Err(format!("`{other}` is not `cc`, `cc14` or `note`")),
    })
}

fn parse_target(to: &str) -> Result<Target, String> {
    let (words, range) = split_range(to)?;
    let mut words = words.split_whitespace();
    let name = words
        .next()
        .ok_or_else(|| "expected a control after `->`".to_string())?;
    let slot = |words: &mut std::str::SplitWhitespace| -> Result<DeckSlot, String> {
        let n = words
            .next()
            .ok_or_else(|| format!("`{name}` needs a slot number"))?;
        n.parse::<u8>()
            .map(DeckSlot::from)
            .map_err(|_| format!("`{name} {n}`: expected a slot number"))
    };
    let target = match name {
        "gain" => Target::Gain {
            slot: slot(&mut words)?,
            range: range.unwrap_or(GAIN_RANGE),
        },
        "opacity" => Target::Opacity {
            slot: slot(&mut words)?,
            range: range.unwrap_or(OPACITY_RANGE),
        },
        "exposure" => Target::Exposure {
            range: range.unwrap_or(EXPOSURE_RANGE),
        },
        // **The mask's front, and the mask's front only.** Hyphenated rather
        // than a bare `mask`, which is the word the shape would want — the two
        // are halves of one record and a grammar that spent the short word on
        // one of them would have nothing left for the other. Why the shape has
        // no line here at all is the module documentation and
        // `docs/adr/0202-the-map-reaches-the-masks-front-and-the-shape-has-no-spelling.md`.
        "mask-position" => Target::MaskPosition {
            slot: slot(&mut words)?,
            range: range.unwrap_or(MASK_POSITION_RANGE),
        },
        "residency" => {
            let slot = slot(&mut words)?;
            Target::Residency {
                slot,
                residency: value_word(
                    words.next(),
                    Residency::ALL,
                    Residency::name,
                    &format!("residency {slot}"),
                    "a state",
                )?,
            }
        }
        "blend" => {
            let slot = slot(&mut words)?;
            Target::Blend {
                slot,
                blend: value_word(
                    words.next(),
                    BlendMode::ALL,
                    BlendMode::name,
                    &format!("blend {slot}"),
                    "a mode",
                )?,
            }
        }
        "tap" => Target::Tap,
        // **A deck and a place in its published interface**, which is the one
        // target whose second number is not a slot: positions count from one
        // because that is the number the Inspector draws beside the row, and a
        // learned line an operator cannot check against the pane is a line
        // they cannot fix by hand.
        "param" => {
            let slot = slot(&mut words)?;
            let n = words
                .next()
                .ok_or_else(|| format!("`param {slot}` needs a position in the interface"))?;
            let position: u16 = n
                .parse()
                .map_err(|_| format!("`param {slot} {n}`: expected a position"))?;
            if position == 0 {
                return Err(format!(
                    "`param {slot} 0`: positions count from one, which is the number the \
                     Inspector draws beside the row"
                ));
            }
            Target::Param {
                slot,
                position,
                range,
            }
        }
        // **The two words that were affordances, refused by name.** A file
        // holding one is a file written against the old grammar, where
        // `on-air 0` meant *flip slot 0*; loading it and reading it as *put
        // slot 0 live* is the same line doing something else mid-set, which
        // is the one outcome this format must not have. The complaint carries
        // the line to write instead, because a complaint with a line number is
        // a line an operator can fix — see the module documentation.
        "on-air" | "prime" => {
            let n = words.next().unwrap_or("N");
            let (flipped, asked) = if name == "on-air" {
                ("a deck on and off", "live")
            } else {
                ("a request on and off", "priming")
            };
            return Err(format!(
                "`{name} {n}` flipped {flipped} rather than naming where it goes; write \
                 `residency {n} {asked}` or `residency {n} allocated`"
            ));
        }
        other => {
            return Err(format!(
                "`{other}` is not a control — expected gain, opacity, exposure, \
                 mask-position, param, residency, blend or tap"
            ))
        }
    };
    if words.next().is_some() {
        return Err(format!("`{name}` takes no more words"));
    }
    if range.is_some() && !target.continuous() {
        return Err(format!("`{name}` is a press and has no range"));
    }
    if target.shape() == Shape::Ratio {
        let [lo, hi] = range.unwrap_or(EXPOSURE_RANGE);
        if lo <= 0.0 || hi <= 0.0 {
            return Err(format!(
                "`{name}` is a ratio control, so its range cannot reach zero"
            ));
        }
    }
    if let Some([lo, hi]) = range {
        if !(lo.is_finite() && hi.is_finite()) || lo == hi {
            return Err(format!("`{name}`: a range needs two different finite ends"));
        }
    }
    Ok(target)
}

/// Parses a token against a predefined slice of vocabulary values, returning an error naming all alternatives if invalid.
fn value_word<T: Copy, const N: usize>(
    word: Option<&str>,
    all: [T; N],
    name: fn(T) -> &'static str,
    line: &str,
    what: &str,
) -> Result<T, String> {
    let listed = || {
        all.iter()
            .map(|v| format!("`{line} {}`", name(*v)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let Some(word) = word else {
        return Err(format!("`{line}` needs {what} — write one of {}", listed()));
    };
    all.iter()
        .copied()
        .find(|v| name(*v) == word)
        .ok_or_else(|| format!("`{line} {word}` is not {what} — write one of {}", listed()))
}

/// Split a trailing `[lo, hi]` off a control, if there is one.
fn split_range(to: &str) -> Result<(&str, Option<[f32; 2]>), String> {
    let Some(open) = to.find('[') else {
        return Ok((to, None));
    };
    let rest = &to[open..];
    let close = rest
        .find(']')
        .ok_or_else(|| "a range opened with `[` and did not close".to_string())?;
    let inside = &rest[1..close];
    if !rest[close + 1..].trim().is_empty() {
        return Err("a range has to be the last thing on the line".to_string());
    }
    let mut ends = inside.split(',');
    let parse = |s: Option<&str>| -> Result<f32, String> {
        s.ok_or_else(|| "a range is `[lo, hi]`".to_string())?
            .trim()
            .parse::<f32>()
            .map_err(|_| "a range is `[lo, hi]`, with numbers".to_string())
    };
    let lo = parse(ends.next())?;
    let hi = parse(ends.next())?;
    if ends.next().is_some() {
        return Err("a range is `[lo, hi]`, and no more".to_string());
    }
    Ok((&to[..open], Some([lo, hi])))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Message;

    fn map(text: &str) -> Map {
        let (map, notes) = Map::parse(text);
        assert!(notes.is_empty(), "{notes:?}");
        map
    }

    fn cc(controller: u8, value: u8) -> Message {
        Message::ControlChange {
            channel: 0,
            controller,
            value,
        }
    }

    fn note(note: u8) -> Message {
        Message::NoteOn {
            channel: 0,
            note,
            velocity: 100,
        }
    }

    /// A 14-bit pair, as a helper: the two messages a surface sends for one
    /// fader position, in the order it sends them.
    fn pair(msb: u8, lsb: u8, value: u16) -> [Message; 2] {
        [cc(msb, (value >> 7) as u8), cc(lsb, (value & 0x7f) as u8)]
    }

    /// Verifies that `cc14 <msb> <lsb>` is the pair.
    #[test]
    fn a_cc14_line_is_one_mapping_and_spells_itself_back() {
        let m = map("cc14 1 33 -> gain 0");
        assert_eq!(m.len(), 1, "a pair was counted as two mappings");
        assert_eq!(
            m.bound("gain 0").as_deref(),
            Some("cc14 1 33"),
            "the readout did not say the line that was written"
        );
        assert_eq!(
            m.lines().collect::<Vec<_>>(),
            vec!["cc14 1 33 -> gain 0".to_string()],
            "the line written back was not the line loaded"
        );
        // And the channel survives, in the front panel's 1-16.
        let m = map("cc14 1 33 ch 2 -> gain 0");
        assert_eq!(m.bound("gain 0").as_deref(), Some("cc14 1 33 ch 2"));
    }

    /// Verifies that a malformed pair is refused at parse, naming both numbers.
    #[test]
    fn a_malformed_pair_is_refused_at_parse_naming_both_numbers() {
        for (line, wanted) in [
            // The two halves are one controller.
            ("cc14 7 7 -> gain 0", vec!["7 7"]),
            // The LSB half is past what the wire carries.
            ("cc14 7 200 -> gain 0", vec!["7 200"]),
            // No LSB half at all, and the complaint carries the line to write.
            ("cc14 7 -> gain 0", vec!["cc14 7", "cc14 7 39"]),
            ("cc14 7 ch 2 -> gain 0", vec!["cc14 7", "cc14 7 39"]),
            // Not a number.
            ("cc14 7 lsb -> gain 0", vec!["7 lsb"]),
        ] {
            let (m, notes) = Map::parse(line);
            assert!(m.is_empty(), "`{line}` loaded");
            let note = notes
                .first()
                .unwrap_or_else(|| panic!("`{line}` said nothing"));
            for wanted in wanted {
                assert!(note.contains(wanted), "`{line}` said `{note}`");
            }
        }
        // And a pair on a press target is refused as a `cc` is, because it is
        // one: a pad takes a note.
        let (_, notes) = Map::parse("cc14 1 33 -> residency 0 live");
        assert!(
            notes.first().is_some_and(|note| note.contains("note")),
            "{notes:?}"
        );
    }

    /// Verifies that the pair assembles into 16384 positions and scales onto the range.
    #[test]
    fn a_pair_assembles_into_sixteen_thousand_positions_and_scales_onto_the_range() {
        let m = map("cc14 1 33 -> gain 0");
        // Both ends are exact, which is the property every fader here has.
        for (value, wanted) in [(0u16, 0.0f32), (16383, 1.0), (8191, 8191.0 / 16383.0)] {
            let [msb, lsb] = pair(1, 33, value);
            assert_eq!(m.wide(msb).map(|w| w.half), Some(Half::Msb));
            assert_eq!(m.wide(lsb).map(|w| w.half), Some(Half::Lsb));
            assert_eq!(
                m.operation_wide(lsb, value),
                Some(Operation::SetGain {
                    deck: 0,
                    gain: wanted
                }),
                "the pair {value} did not assemble onto the range"
            );
        }
        // **Two adjacent pairs are a 128th of a 7-bit step apart**, which is
        // the number this line exists for: a 7-bit fader moves a gain by
        // 1/127 and this one by 1/16383.
        let step = |value: u16| match m.operation_wide(pair(1, 33, value)[1], value) {
            Some(Operation::SetGain { gain, .. }) => gain,
            other => panic!("{other:?}"),
        };
        let one = step(8192) - step(8191);
        assert!(
            (one - 1.0 / 16383.0).abs() < 1e-6,
            "one step of a pair was {one}"
        );
        // And a line's own range is scaled onto exactly as a 7-bit line's is.
        let m = map("cc14 1 33 -> gain 0 [0, 2]");
        assert_eq!(
            m.operation_wide(pair(1, 33, 16383)[1], 16383),
            Some(Operation::SetGain { deck: 0, gain: 2.0 })
        );
    }

    /// Verifies that an MSB alone never leaves the fader between two values.
    #[test]
    fn an_msb_alone_does_not_leave_the_fader_between_two_values() {
        let m = map("cc14 1 33 -> gain 0");
        let seven = map("cc 1 -> gain 0");
        for msb in [0u8, 1, 64, 126, 127] {
            assert_eq!(
                m.operation(cc(1, msb)),
                seven.operation(cc(1, msb)),
                "an MSB alone read differently from the 7-bit line it is"
            );
        }
        // The top of the fader is unity and not a step short of it, which is
        // what `(msb << 7) / 16383` would have made it.
        assert_eq!(
            m.operation(cc(1, 127)),
            Some(Operation::SetGain { deck: 0, gain: 1.0 })
        );
        // **And a lone LSB moves nothing**: there is nothing to refine until
        // an MSB has been seen, and a caller holding no MSB has no pair.
        assert_eq!(m.operation(cc(33, 100)), None);
        assert_eq!(m.parameter(cc(33, 100)), None);
    }

    /// Verifies that a controller cannot be a plain line and half of a pair.
    #[test]
    fn a_controller_that_is_both_a_line_and_a_pairs_lsb_is_reported() {
        for text in [
            "cc14 1 33 -> gain 0\ncc 33 -> opacity 0",
            "cc 33 -> opacity 0\ncc14 1 33 -> gain 0",
        ] {
            let (m, notes) = Map::parse(text);
            assert!(
                notes.iter().any(|note| note.contains("cc 33")),
                "{notes:?} for `{text}`"
            );
            // The plain line is what controller 33 does.
            assert_eq!(
                m.operation(cc(33, 127)),
                Some(Operation::SetOpacity {
                    deck: 0,
                    opacity: 1.0
                })
            );
            assert_eq!(m.wide(cc(33, 127)), None, "the pair claimed the plain line");
            // And the pair's MSB half still moves its own control, coarsely.
            assert_eq!(
                m.operation(cc(1, 127)),
                Some(Operation::SetGain { deck: 0, gain: 1.0 })
            );
        }
    }

    /// Verifies that an echo says what to read and what the wire shows.
    #[test]
    fn an_echo_says_what_to_read_and_the_wire_shows_it() {
        let m = map(
            "cc 1 -> gain 0\ncc14 2 34 -> opacity 1\nnote 32 -> residency 0 live\ncc 20 -> exposure\n\
             cc 30 -> param 0 3\nnote 61 -> tap",
        );
        let echoes = m.echoes();
        assert_eq!(echoes.len(), 6);
        let of = |control: Control| {
            *echoes
                .iter()
                .find(|echo| echo.control() == control)
                .unwrap_or_else(|| panic!("no echo for {control:?}"))
        };
        let mut wire = Vec::new();

        // A 7-bit fader: one message, and both ends exact.
        let gain = of(Control::Gain { deck: 0 });
        assert_eq!(gain.position(Shown::At(0.0)), 0);
        assert_eq!(gain.position(Shown::At(1.0)), 127);
        gain.wire(gain.position(Shown::At(1.0)), &mut wire);
        assert_eq!(wire, vec![[0xb0, 1, 127]]);

        // A pair: two messages, **MSB first**, and the two halves of the
        // number the position is.
        wire.clear();
        let opacity = of(Control::Opacity { deck: 1 });
        assert_eq!(opacity.position(Shown::At(1.0)), 16383);
        assert_eq!(opacity.position(Shown::At(0.0)), 0);
        let half = opacity.position(Shown::At(0.5));
        opacity.wire(half, &mut wire);
        assert_eq!(
            wire,
            vec![
                [0xb0, 2, (half >> 7) as u8],
                [0xb0, 34, (half & 0x7f) as u8]
            ]
        );
        // And it comes back through the way in, to the value it was shown.
        assert_eq!(
            m.operation_wide(cc(34, 0), half),
            Some(Operation::SetOpacity {
                deck: 1,
                opacity: half as f32 / 16383.0
            })
        );

        // A pad: lit is a note-on at 127 and unlit is one at 0, which is this
        // crate's own reading of a release.
        wire.clear();
        let live = of(Control::Residency {
            deck: 0,
            residency: Residency::Live,
        });
        live.wire(live.position(Shown::On(true)), &mut wire);
        live.wire(live.position(Shown::On(false)), &mut wire);
        assert_eq!(wire, vec![[0x90, 32, 127], [0x90, 32, 0]]);

        // Exposure is a ratio control, so the middle of the fader is unity.
        let exposure = of(Control::Exposure);
        assert_eq!(exposure.position(Shown::At(1.0)), 64);
        assert_eq!(exposure.position(Shown::At(0.25)), 0);
        assert_eq!(exposure.position(Shown::At(4.0)), 127);

        // A published control is shown over the range the Set published it,
        // unless the line wrote one.
        let param = of(Control::Param {
            deck: 0,
            position: 3,
        });
        assert_eq!(
            param.position(Shown::Published {
                value: 4.0,
                declared: [0.0, 8.0]
            }),
            64
        );

        // A value past the end of the line's range shows the end: that is
        // where the fader would have to be.
        assert_eq!(gain.position(Shown::At(1.4)), 127);
        assert_eq!(gain.position(Shown::At(-1.0)), 0);
        assert_eq!(gain.position(Shown::At(f32::NAN)), 0);

        // A beat has no state, and the list carries it so that a target added
        // to the grammar and not to `Control` does not compile.
        assert_eq!(of(Control::Tap).control(), Control::Tap);
    }

    /// Verifies that a `param` line names a deck and a place in its interface.
    #[test]
    fn a_param_line_names_a_deck_and_a_position_and_is_not_an_operation_here() {
        let m = map("cc 30 -> param 0 3");
        assert_eq!(
            m.operation(cc(30, 127)),
            None,
            "a position was completed without a deck to resolve it against"
        );
        let asked = m.parameter(cc(30, 127)).expect("a param line answers here");
        assert_eq!(asked.deck, 0);
        assert_eq!(asked.position, 3);
        assert_eq!(asked.range, None, "no range on the line is the Set's own");
        // **The declared range is the caller's**, and both ends are exact on
        // it for the reason every other fader's are.
        assert_eq!(asked.value([0.0, 8.0]), 8.0);
        assert_eq!(
            m.parameter(cc(30, 0)).expect("bottom").value([0.0, 8.0]),
            0.0
        );
        // A range on the line overrides the Set's, as it does on a gain.
        let m = map("cc 30 -> param 1 5 [1, 3]");
        let asked = m.parameter(cc(30, 127)).expect("a param line with a range");
        assert_eq!(asked.range, Some([1.0, 3.0]));
        assert_eq!(
            asked.value([0.0, 8.0]),
            3.0,
            "the line's range has to win over the Set's"
        );
        // It is continuous, so a note on it is refused at parse time.
        let (_, notes) = Map::parse("note 30 -> param 0 3");
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(notes[0].contains("map a `cc`"), "{notes:?}");
    }

    /// Verifies that positions count from one.
    #[test]
    fn a_position_of_zero_is_refused_with_where_the_number_comes_from() {
        let (map, notes) = Map::parse("cc 30 -> param 0 0");
        assert!(map.is_empty(), "a position of zero loaded");
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(notes[0].contains("count from one"), "{notes:?}");
        assert!(notes[0].contains("Inspector"), "{notes:?}");
        // And the position has to be a number at all.
        let (_, notes) = Map::parse("cc 30 -> param 0 first");
        assert!(notes[0].contains("expected a position"), "{notes:?}");
        // A deck with no position after it is a line half written.
        let (_, notes) = Map::parse("cc 30 -> param 0");
        assert!(notes[0].contains("needs a position"), "{notes:?}");
    }

    /// Verifies that every target spells itself back into a line this parser accepts.
    #[test]
    fn a_spelled_target_parses_back_to_itself() {
        let lines = [
            "cc 1 -> gain 0",
            "cc 5 -> opacity 3",
            "cc 20 -> exposure",
            "cc 9 -> mask-position 2",
            "cc 30 -> param 1 7",
            "note 32 -> residency 0 live",
            "note 36 -> residency 1 priming",
            "note 40 -> residency 2 allocated",
            "note 44 -> blend 0 add",
            "note 48 -> blend 1 over",
            "note 52 -> blend 2 max",
            "note 61 -> tap",
        ];
        for line in lines {
            let (from, to) = line.split_once(" -> ").expect("a test line");
            let (_, entry) = parse_line(line).unwrap_or_else(|e| panic!("`{line}`: {e}"));
            assert_eq!(
                entry.target.spelled(),
                to,
                "`{line}` did not spell itself back"
            );
            // And the whole line round-trips through the table, which is what
            // `Map::bound` is asked for.
            let m = map(line);
            assert_eq!(m.bound(to).as_deref(), Some(from), "`{line}`");
            assert_eq!(m.bound("gain 9"), None, "an unmapped control was claimed");
        }
        // A channel survives the round trip, in the front panel's 1-16.
        let m = map("cc 5 ch 2 -> gain 0");
        assert_eq!(m.bound("gain 0").as_deref(), Some("cc 5 ch 2"));
    }

    /// Verifies that every mapped message answers exactly one of the two accessors.
    #[test]
    fn every_mapped_message_answers_exactly_one_of_the_two() {
        let text = "cc 1 -> gain 0\ncc 5 -> opacity 0\ncc 20 -> exposure\n\
                    cc 9 -> mask-position 0\ncc 30 -> param 0 3\n\
                    note 32 -> residency 0 live\nnote 44 -> blend 0 add\nnote 61 -> tap";
        let m = map(text);
        let messages = [
            cc(1, 64),
            cc(5, 64),
            cc(20, 64),
            cc(9, 64),
            cc(30, 64),
            note(32),
            note(44),
            note(61),
        ];
        for message in messages {
            let operation = m.operation(message).is_some();
            let parameter = m.parameter(message).is_some();
            assert!(
                operation ^ parameter,
                "{message:?} answered {operation} and {parameter}, which is not exactly one"
            );
        }
        // And an unmapped message answers neither.
        assert!(m.operation(cc(99, 0)).is_none() && m.parameter(cc(99, 0)).is_none());
    }

    /// Verifies that both ends of a fader are exact.
    #[test]
    fn a_fader_reaches_both_ends_of_its_range_exactly() {
        let m = map("cc 1 -> gain 0");
        assert_eq!(
            m.operation(cc(1, 0)),
            Some(Operation::SetGain { deck: 0, gain: 0.0 })
        );
        assert_eq!(
            m.operation(cc(1, 127)),
            Some(Operation::SetGain { deck: 0, gain: 1.0 })
        );
        // And an explicit range, both ends, so the default is not the only one
        // that lands.
        let m = map("cc 1 -> gain 0 [0.5, 2.5]");
        assert_eq!(
            m.operation(cc(1, 0)),
            Some(Operation::SetGain { deck: 0, gain: 0.5 })
        );
        assert_eq!(
            m.operation(cc(1, 127)),
            Some(Operation::SetGain { deck: 0, gain: 2.5 })
        );
    }

    /// Verifies that exposure is a ratio, so the middle of the fader is unity.
    #[test]
    fn exposure_moves_in_stops_rather_than_in_equal_steps() {
        let m = map("cc 20 -> exposure");
        let at = |v: u8| match m.operation(cc(20, v)) {
            Some(Operation::SetExposure { exposure }) => exposure,
            other => panic!("{other:?}"),
        };
        assert_eq!(at(0), 0.25);
        assert!((at(127) - 4.0).abs() < 1e-5, "{}", at(127));
        // Halfway is unity, which is the whole reason for the shape.
        let middle = at(63) + (at(64) - at(63)) / 2.0;
        assert!((middle - 1.0).abs() < 0.01, "{middle}");
        // And it is a ratio scale rather than a line: the step at the bottom
        // is smaller than the step at the top.
        assert!(at(1) - at(0) < at(127) - at(126));
    }

    /// Verifies that a mask's front reaches both ends of its travel exactly, and moves in
    /// equal steps.
    #[test]
    fn a_mask_front_reaches_both_ends_exactly_and_moves_in_equal_steps() {
        let m = map("cc 9 -> mask-position 0");
        let at = |v: u8| match m.operation(cc(9, v)) {
            Some(Operation::SetMaskPosition { deck: 0, position }) => position,
            other => panic!("{other:?}"),
        };
        assert_eq!(at(0), 0.0, "the front could not be sent back to hidden");
        assert_eq!(at(127), 1.0, "the front could not be carried all the way");
        // Equal steps in position, which is what separates this from
        // exposure's ratio: the middle of the fader is the middle of the
        // travel, and the step at the bottom is the step at the top. The
        // second is to within a float's last place rather than exactly —
        // `lo + t*(hi - lo)` rounds per step and a ratio scale is out by two
        // orders of magnitude here, not by an ulp.
        let middle = at(63) + (at(64) - at(63)) / 2.0;
        assert!((middle - 0.5).abs() < 1e-6, "{middle}");
        let (bottom, top) = (at(1) - at(0), at(127) - at(126));
        assert!((bottom - top).abs() < 1e-6, "{bottom} against {top}");
        // A written range is the operator's, exactly as a gain's is — a fader
        // that only crosses the middle of the frame is a line somebody will
        // write. It buys less here than on a gain, because the engine clamps a
        // mask to the unit interval on apply and a gain above unity is a real
        // place to be.
        let m = map("cc 9 -> mask-position 2 [0.25, 0.75]");
        assert_eq!(
            m.operation(cc(9, 0)),
            Some(Operation::SetMaskPosition {
                deck: 2,
                position: 0.25
            })
        );
        assert_eq!(
            m.operation(cc(9, 127)),
            Some(Operation::SetMaskPosition {
                deck: 2,
                position: 0.75
            })
        );
    }

    /// Verifies that the front takes a fader and refuses a pad.
    #[test]
    fn the_mask_front_takes_a_fader_and_refuses_a_pad() {
        let (m, notes) = Map::parse("note 62 -> mask-position 0");
        assert!(m.is_empty(), "a pad on the mask's front loaded");
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(notes[0].contains("map a `cc`"), "{}", notes[0]);
        // And the message it names does load, so the complaint is a route and
        // not a dead end.
        assert_eq!(map("cc 9 -> mask-position 0").len(), 1);
    }

    /// Verifies that the mask's other half has no line, and a file that tries to write one
    /// is refused rather than half-loaded.
    #[test]
    fn the_mask_shape_has_no_line_and_every_spelling_of_one_is_refused() {
        for line in [
            "note 62 -> mask-shape 0 linear",
            "note 62 -> mask 0 radial",
            "cc 9 -> mask 0",
            "cc 9 -> mask-angle 0",
        ] {
            let (m, notes) = Map::parse(line);
            assert!(m.is_empty(), "`{line}` loaded");
            assert_eq!(notes.len(), 1, "`{line}`: {notes:?}");
            assert!(
                notes[0].contains("is not a control"),
                "`{line}`: {}",
                notes[0]
            );
            // The complaint lists what there is, and the front is on the list
            // — the one half of the mask a line can reach.
            assert!(notes[0].contains("mask-position"), "`{line}`: {}", notes[0]);
        }
    }

    /// Verifies that a pad and a fader cannot be mapped to each other's targets.
    #[test]
    fn a_control_that_takes_a_press_refuses_a_fader_and_the_reverse() {
        let (m, notes) = Map::parse("cc 1 -> residency 0 live\nnote 36 -> gain 0");
        assert!(m.is_empty(), "a mismatched mapping loaded");
        assert_eq!(notes.len(), 2, "{notes:?}");
        assert!(notes[0].contains("map a `note`"), "{}", notes[0]);
        assert!(notes[1].contains("map a `cc`"), "{}", notes[1]);
    }

    /// A channel narrows a mapping and its absence widens it. Both directions,
    /// because "any channel" as a default is only safe if a stated channel
    /// still excludes the others.
    #[test]
    fn a_stated_channel_matches_only_that_channel_and_no_channel_matches_all() {
        let m = map("cc 1 ch 3 -> gain 0");
        let on = |channel| {
            m.operation(Message::ControlChange {
                channel,
                controller: 1,
                value: 127,
            })
        };
        assert!(on(2).is_some(), "channel 3 on the panel is 2 on the wire");
        assert_eq!(on(3), None, "a mapping on one channel answered another");

        let m = map("cc 1 -> gain 0");
        assert!(on_any(&m, 0).is_some() && on_any(&m, 9).is_some());
    }

    fn on_any(m: &Map, channel: u8) -> Option<Operation> {
        m.operation(Message::ControlChange {
            channel,
            controller: 1,
            value: 64,
        })
    }

    /// Verifies that a specific channel wins over `any`.
    #[test]
    fn a_mapping_with_a_channel_wins_over_one_without() {
        let m = map("cc 1 -> gain 0\ncc 1 ch 1 -> gain 3");
        assert_eq!(
            m.operation(Message::ControlChange {
                channel: 0,
                controller: 1,
                value: 127
            }),
            Some(Operation::SetGain { deck: 3, gain: 1.0 })
        );
        assert_eq!(
            m.operation(Message::ControlChange {
                channel: 5,
                controller: 1,
                value: 127
            }),
            Some(Operation::SetGain { deck: 0, gain: 1.0 })
        );
    }

    /// Verifies that a release does nothing.
    #[test]
    fn a_release_is_not_a_second_press() {
        let m = map("note 36 -> residency 0 live");
        assert_eq!(
            m.operation(note(36)),
            Some(Operation::SetResidency {
                deck: 0,
                residency: Residency::Live
            })
        );
        assert_eq!(
            m.operation(Message::NoteOff {
                channel: 0,
                note: 36
            }),
            None
        );
        // Including the spelling that arrives as a note-on at velocity 0.
        assert_eq!(
            m.operation(Message::parse(&[0x90, 36, 0]).expect("a note-on")),
            None
        );
    }

    /// Every control the vocabulary names parses and answers, so a control
    /// added to `Target` and not to the parser is a line that does not load
    /// rather than one that loads as something else.
    #[test]
    fn every_control_in_the_vocabulary_parses_and_answers() {
        let m = map("cc 1 -> gain 2\n\
             cc 2 -> opacity 3\n\
             cc 3 -> exposure\n\
             cc 4 -> mask-position 2\n\
             note 36 -> residency 1 live\n\
             note 37 -> blend 3 over\n\
             note 41 -> tap");
        assert_eq!(m.len(), 7);
        assert!(matches!(
            m.operation(cc(1, 127)),
            Some(Operation::SetGain { deck: 2, .. })
        ));
        assert!(matches!(
            m.operation(cc(2, 127)),
            Some(Operation::SetOpacity { deck: 3, .. })
        ));
        assert!(matches!(
            m.operation(cc(3, 64)),
            Some(Operation::SetExposure { .. })
        ));
        assert!(matches!(
            m.operation(cc(4, 127)),
            Some(Operation::SetMaskPosition {
                deck: 2,
                position: 1.0
            })
        ));
        assert_eq!(
            m.operation(note(36)),
            Some(Operation::SetResidency {
                deck: 1,
                residency: Residency::Live
            })
        );
        assert_eq!(
            m.operation(note(37)),
            Some(Operation::SetBlendMode {
                deck: 3,
                blend: BlendMode::Over
            })
        );
        assert_eq!(m.operation(note(41)), Some(Operation::TapBeat));
    }

    /// Verifies that a pad names a state, and every state the vocabulary holds is
    /// writable.
    #[test]
    fn a_pad_names_one_of_the_values_the_vocabulary_holds() {
        for (i, residency) in Residency::ALL.iter().enumerate() {
            let line = format!("note {} -> residency 2 {}", 36 + i, residency.name());
            assert_eq!(
                map(&line).operation(note(36 + i as u8)),
                Some(Operation::SetResidency {
                    deck: 2,
                    residency: *residency
                }),
                "`{line}` did not name {}",
                residency.name()
            );
        }
        for (i, blend) in BlendMode::ALL.iter().enumerate() {
            let line = format!("note {} -> blend 1 {}", 48 + i, blend.name());
            assert_eq!(
                map(&line).operation(note(48 + i as u8)),
                Some(Operation::SetBlendMode {
                    deck: 1,
                    blend: *blend
                }),
                "`{line}` did not name {}",
                blend.name()
            );
        }
    }

    /// Verifies that a file written against the old grammar is refused, line by line, with
    /// the line to write instead.
    #[test]
    fn a_line_in_the_old_grammar_is_refused_with_the_line_to_write_instead() {
        let (m, notes) = Map::parse(
            "note 32 -> on-air 0\n\
             note 36 -> prime 1\n\
             note 40 -> blend 2",
        );
        assert!(m.is_empty(), "a line in the old grammar loaded");
        assert_eq!(notes.len(), 3, "{notes:?}");
        for (i, wanted) in ["residency 0 live", "residency 1 priming", "blend 2 over"]
            .iter()
            .enumerate()
        {
            assert!(
                notes[i].starts_with(&format!("line {}:", i + 1)),
                "{}",
                notes[i]
            );
            assert!(
                notes[i].contains(wanted),
                "the complaint did not name `{wanted}`: {}",
                notes[i]
            );
        }
        // And the other half of each: taking a deck off air, and withdrawing a
        // prime request, are the destinations the old `false` had no name for.
        assert!(notes[0].contains("residency 0 allocated"), "{}", notes[0]);
        assert!(notes[1].contains("residency 1 allocated"), "{}", notes[1]);
    }

    /// A value word that is not one of the list is refused naming all of them,
    /// which is the same complaint a missing one gets — one bad line, on its
    /// own line, with somewhere to go.
    #[test]
    fn a_value_word_that_is_not_one_of_the_list_is_refused_naming_them() {
        let (m, notes) = Map::parse("note 36 -> residency 0 warming");
        assert!(m.is_empty());
        assert!(notes[0].contains("`residency 0 live`"), "{}", notes[0]);
        assert!(notes[0].contains("`residency 0 priming`"), "{}", notes[0]);
        assert!(notes[0].contains("`residency 0 allocated`"), "{}", notes[0]);
        let (m, notes) = Map::parse("note 36 -> blend 0 screen");
        assert!(m.is_empty());
        assert!(notes[0].contains("`blend 0 max`"), "{}", notes[0]);
    }

    /// Verifies that one bad line is a line, not a file.
    #[test]
    fn a_bad_line_is_reported_and_the_rest_of_the_file_loads() {
        let (m, notes) = Map::parse(
            "cc 1 -> gain 0\n\
             cc 2 -> wobble 1\n\
             cc 3 -> opacity 0",
        );
        assert_eq!(m.len(), 2, "a bad line took a good one with it");
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(notes[0].starts_with("line 2:"), "{}", notes[0]);
        assert!(notes[0].contains("wobble"), "{}", notes[0]);
    }

    /// Comments and blank lines are not errors, and a comment after a mapping
    /// does not become part of it.
    #[test]
    fn comments_and_blank_lines_are_skipped() {
        let m = map("# the slot faders\n\
             \n\
             cc 1 -> gain 0   # channel strip 1\n");
        assert_eq!(m.len(), 1);
        assert!(m.operation(cc(1, 127)).is_some());
    }

    /// A ratio control cannot have a range that reaches zero — the map is
    /// undefined there — and saying so at parse time is the only place an
    /// operator can act on it.
    #[test]
    fn a_ratio_range_that_reaches_zero_is_refused_where_it_is_written() {
        let (m, notes) = Map::parse("cc 3 -> exposure [0, 4]");
        assert!(m.is_empty());
        assert!(notes[0].contains("cannot reach zero"), "{}", notes[0]);
    }

    /// Verifies that the example map in the repository loads clean.
    #[test]
    fn the_example_map_in_the_repository_parses_with_no_complaints() {
        let text = include_str!("../../../examples/surface.map");
        let (map, notes) = Map::parse(text);
        assert!(notes.is_empty(), "{notes:#?}");
        // Every line of it, so a mapping quietly dropped by a duplicate key
        // shows up as a count rather than as a missing knob mid-set.
        let lines = text
            .lines()
            .filter(|l| {
                let l = l.split('#').next().unwrap_or("").trim();
                !l.is_empty()
            })
            .count();
        assert_eq!(map.len(), lines, "a mapping was overwritten by another");
    }

    /// Verifies that a range with one value in it, or a value that is not one.
    #[test]
    fn a_range_that_is_not_two_different_numbers_is_refused() {
        for line in [
            "cc 1 -> gain 0 [1, 1]",
            "cc 1 -> gain 0 [nan, 1]",
            "cc 1 -> gain 0 [0, inf]",
        ] {
            let (m, notes) = Map::parse(line);
            assert!(m.is_empty(), "`{line}` loaded");
            assert!(
                notes[0].contains("two different finite ends"),
                "`{line}`: {}",
                notes[0]
            );
        }
        // And the shape it is guarding: two different finite ends load.
        assert_eq!(map("cc 1 -> gain 0 [0, 2]").len(), 1);
    }

    /// Verifies that the later line wins, and says so.
    #[test]
    fn a_second_mapping_for_one_message_replaces_the_first_and_is_reported() {
        let (m, notes) = Map::parse("cc 1 -> gain 0\ncc 1 -> gain 3");
        assert_eq!(m.len(), 1);
        assert_eq!(
            m.operation(cc(1, 127)),
            Some(Operation::SetGain { deck: 3, gain: 1.0 }),
            "the earlier line won"
        );
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(notes[0].starts_with("line 2:"), "{}", notes[0]);
        // Two keys that only look alike are not a collision: a `cc 1` and a
        // `note 1` are two controls.
        let (_, notes) = Map::parse("cc 1 -> gain 0\nnote 1 -> tap");
        assert!(notes.is_empty(), "{notes:?}");
    }

    /// Verifies that every number a line carries is checked against the wire's range.
    #[test]
    fn a_number_the_wire_cannot_carry_is_refused_on_the_line() {
        for (line, wanted) in [
            ("cc 128 -> gain 0", "128"),
            ("note 200 -> tap", "200"),
            ("cc 1 ch 0 -> gain 0", "1-16"),
            ("cc 1 ch 17 -> gain 0", "1-16"),
            ("cc 1 zz 2 -> gain 0", "ch"),
            ("cc 1 ch 1 extra -> gain 0", "too many words"),
        ] {
            let (m, notes) = Map::parse(line);
            assert!(m.is_empty(), "`{line}` loaded");
            assert!(notes[0].contains(wanted), "`{line}`: {}", notes[0]);
        }
        // The boundaries either side, which is where an off-by-one lives.
        assert_eq!(map("cc 127 ch 1 -> gain 0").len(), 1);
        assert_eq!(map("cc 0 ch 16 -> gain 0").len(), 1);
    }

    /// A range on a press is a line whose author expected something else to
    /// happen, so it is refused rather than ignored.
    #[test]
    fn a_range_on_a_press_is_refused_rather_than_dropped() {
        let (m, notes) = Map::parse("note 36 -> tap [0, 1]");
        assert!(m.is_empty());
        assert!(notes[0].contains("no range"), "{}", notes[0]);
    }

    /// Verifies that every fader's target is continuous and no pad's is.
    #[test]
    fn a_fader_moves_a_continuous_control_and_a_pad_does_not() {
        let m = map(
            "cc 1 -> gain 0\ncc 2 -> opacity 0\ncc 3 -> exposure\ncc 4 -> mask-position 0\n\
             note 36 -> residency 0 live\nnote 37 -> blend 0 over\n\
             note 38 -> tap",
        );
        for controller in 1..=4 {
            assert!(m.is_continuous(cc(controller, 0)), "cc {controller}");
        }
        for n in 36..=38 {
            assert!(!m.is_continuous(note(n)), "note {n}");
        }
        // Unmapped is not continuous: there is nothing to coalesce, and the
        // router reports it rather than routing it.
        assert!(!m.is_continuous(cc(9, 0)));
        // Nor is a release, which names no target at all.
        assert!(!m.is_continuous(Message::NoteOff {
            channel: 0,
            note: 36
        }));
    }
}
