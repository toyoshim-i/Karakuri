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
