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
mod tests;
