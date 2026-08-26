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
//!   cc 20     -> exposure
//!
//!   # pads
//!   note 36 -> residency 0 live
//!   note 40 -> residency 1 priming
//!   note 44 -> blend 0 over
//!   note 48 -> preview 0
//!   note 52 -> preview mix
//!   note 56 -> tap
//! ```
//!
//! ## A line names a state, never a step
//!
//! **The value word is the grammar**, and it is what
//! [P-0074](../../../docs/principles/0074-an-operation-says-what-it-wants-never-which-way-to-move.md)
//! costs on this surface: a pad says `residency 2 live`, not *flip slot 2*. A
//! surface that could only step has no way to *arrive*, and two surfaces
//! stepping one control disagree about where they are — so a pad that means
//! *over* is one pad, and a mini that cycles the three is an affordance
//! whoever draws it builds over three lines of this file.
//!
//! `preview N | mix` had the shape already. `on-air N`, `prime N` and a bare
//! `blend N` were the three that did not, and **a file still holding one is
//! refused on that line with the line to write instead** — never loaded and
//! silently re-read, because `on-air 0` means *flip it* in a file written last
//! month and would mean *put it live* today
//! ([ADR-0196](../../../docs/adr/0196-a-map-line-names-a-state-and-an-old-line-is-refused.md)).
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
//! **It is 7-bit.** A control change carries 128 positions and this reads them
//! as 128 positions; the 14-bit MSB/LSB convention is not implemented. That is
//! a real coarseness on a gain — about 0.8% of the range per step — and the
//! honest note is that it has not been a problem to look at rather than that it
//! is fine in principle.

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

/// What a message is mapped to, before its value is known.
///
/// **A press carries its destination and a fader carries its range**, which is
/// the whole difference between the two halves of this list: a note's line said
/// everything it had to say at parse time, so `Target::Residency` holds the
/// state it names and `Target::Blend` holds the mode. Nothing here is a step.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Target {
    Gain { slot: u8, range: [f32; 2] },
    Opacity { slot: u8, range: [f32; 2] },
    Exposure { range: [f32; 2] },
    Residency { slot: u8, residency: Residency },
    Blend { slot: u8, blend: BlendMode },
    Preview { slot: Option<u8> },
    Tap,
}

impl Target {
    fn shape(self) -> Shape {
        match self {
            Target::Exposure { .. } => Shape::Ratio,
            _ => Shape::Linear,
        }
    }

    /// Whether this is moved by a fader or hit by a pad. A mapping that puts a
    /// pad on a fader's target is refused at parse time rather than putting a
    /// deck on air every time a knob passes a threshold.
    fn continuous(self) -> bool {
        matches!(
            self,
            Target::Gain { .. } | Target::Opacity { .. } | Target::Exposure { .. }
        )
    }
}

/// **The default range of every continuous control**, chosen so that both ends
/// of a fader are exact.
///
/// `[0, 1]` for gain even though the mix is HDR and values above 1.0 are
/// ordinary: 127 maps to exactly 1.0 this way, and a fader whose top is unity
/// is what a fader means. Reaching past it is `]`'s job, or an explicit range
/// in the mapping. The alternative — a default of `[0, 2]` so a surface could
/// push a layer — puts unity at 1.008 and nowhere else, and a mixer whose
/// faders cannot be matched is worse than one that cannot be pushed.
///
/// Exposure's `[0.25, 4]` is two stops either side of unity, and it is a ratio
/// scale, so the middle of the fader is exactly 1.0.
const GAIN_RANGE: [f32; 2] = [0.0, 1.0];
const OPACITY_RANGE: [f32; 2] = [0.0, 1.0];
const EXPOSURE_RANGE: [f32; 2] = [0.25, 4.0];

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

/// One operator's table.
#[derive(Debug, Default, Clone)]
pub struct Map {
    entries: HashMap<Key, Target>,
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
                Ok((key, target)) => {
                    if map.entries.insert(key, target).is_some() {
                        notes.push(format!(
                            "line {}: this message was already mapped; the later line wins",
                            i + 1
                        ));
                    }
                }
                Err(message) => notes.push(format!("line {}: {message}", i + 1)),
            }
        }
        (map, notes)
    }

    /// What this message asks for, or `None` if nothing is mapped to it.
    ///
    /// **A pure function of one message.** No readback, no state, no engine —
    /// which is what lets every test here be `parse` then this, and is the
    /// reason `cc -> exposure` names an exposure rather than a look
    /// (ADR-0192).
    ///
    /// A release is deliberately nothing. Every pad here is a press naming a
    /// state or a selection, so acting on the release too would undo it —
    /// and a momentary control that wants both ends is a different target from
    /// these, not the same one read twice.
    pub fn operation(&self, message: crate::Message) -> Option<Operation> {
        use crate::Message;
        let (key, value) = match message {
            Message::ControlChange {
                channel,
                controller,
                value,
            } => (
                self.find(Key::Cc {
                    channel: Some(channel),
                    controller,
                })
                .or_else(|| {
                    self.find(Key::Cc {
                        channel: None,
                        controller,
                    })
                })?,
                Some(value),
            ),
            Message::NoteOn { channel, note, .. } => (
                self.find(Key::Note {
                    channel: Some(channel),
                    note,
                })
                .or_else(|| {
                    self.find(Key::Note {
                        channel: None,
                        note,
                    })
                })?,
                None,
            ),
            Message::NoteOff { .. } => return None,
        };

        // A fader mapped to a pad's target, or the reverse, cannot happen:
        // `parse_line` refuses it. This is the same fact stated where the value
        // is used, so a target added to one list and not the other is a `None`
        // rather than a wrong operation.
        // `key.shape()` rather than the shape spelled out per arm: the same
        // function decides which validation a range gets at parse time, so a
        // target that is validated as a ratio cannot be scaled as a line.
        //
        // **Nothing below allocates.** Every operation a map line can name
        // carries scalars only, which is what lets `karakuri-cli` route a fader
        // sweep inside `Live::frame` without a heap touch per message.
        let shape = key.shape();
        match (key, value) {
            (Target::Gain { slot, range }, Some(v)) => Some(Operation::SetGain {
                deck: slot,
                gain: scale(v, range, shape),
            }),
            (Target::Opacity { slot, range }, Some(v)) => Some(Operation::SetOpacity {
                deck: slot,
                opacity: scale(v, range, shape),
            }),
            (Target::Exposure { range }, Some(v)) => Some(Operation::SetExposure {
                exposure: scale(v, range, shape),
            }),
            (Target::Residency { slot, residency }, None) => Some(Operation::SetResidency {
                deck: slot,
                residency,
            }),
            (Target::Blend { slot, blend }, None) => {
                Some(Operation::SetBlendMode { deck: slot, blend })
            }
            (Target::Preview { slot }, None) => Some(Operation::SetPreview { showing: slot }),
            (Target::Tap, None) => Some(Operation::TapBeat),
            _ => None,
        }
    }

    fn find(&self, key: Key) -> Option<Target> {
        self.entries.get(&key).copied()
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

/// A `0..=127` position on a range.
///
/// **Exact at both ends for every range worth writing**, which is not the same
/// as for every range: `lo + t*(hi - lo)` at `t = 1` is `hi` whenever the
/// subtraction and the addition are, and both defaults are. A hand-written
/// `[5.4778967, 6.2798347]` comes back an ulp low at the top. The claim is
/// worth the qualification because it is the one that matters — a fader that
/// cannot reach silence or unity cannot be matched against another slot.
fn scale(value: u8, range: [f32; 2], shape: Shape) -> f32 {
    let t = f32::from(value.min(127)) / 127.0;
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

fn parse_line(line: &str) -> Result<(Key, Target), String> {
    let (from, to) = line
        .split_once("->")
        .ok_or_else(|| "expected `<message> -> <control>`".to_string())?;
    let key = parse_key(from.trim())?;
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
    Ok((key, target))
}

fn parse_key(from: &str) -> Result<Key, String> {
    let mut words = from.split_whitespace();
    let kind = words
        .next()
        .ok_or_else(|| "expected `cc` or `note`".to_string())?;
    let number: u8 = words
        .next()
        .ok_or_else(|| format!("`{kind}` needs a number"))?
        .parse()
        .map_err(|_| format!("`{kind}` needs a number in 0-127"))?;
    if number > 127 {
        return Err(format!("`{kind} {number}` is past 127"));
    }
    let channel = match (words.next(), words.next()) {
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
        "cc" => Key::Cc {
            channel,
            controller: number,
        },
        "note" => Key::Note {
            channel,
            note: number,
        },
        other => return Err(format!("`{other}` is not `cc` or `note`")),
    })
}

fn parse_target(to: &str) -> Result<Target, String> {
    let (words, range) = split_range(to)?;
    let mut words = words.split_whitespace();
    let name = words
        .next()
        .ok_or_else(|| "expected a control after `->`".to_string())?;
    let slot = |words: &mut std::str::SplitWhitespace| -> Result<u8, String> {
        let n = words
            .next()
            .ok_or_else(|| format!("`{name}` needs a slot number"))?;
        n.parse()
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
        "preview" => match words.next() {
            Some("mix") => Target::Preview { slot: None },
            Some(n) => Target::Preview {
                slot: Some(
                    n.parse()
                        .map_err(|_| format!("`preview {n}`: expected a slot number or `mix`"))?,
                ),
            },
            None => return Err("`preview` needs a slot number or `mix`".to_string()),
        },
        "tap" => Target::Tap,
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
                "`{other}` is not a control — expected gain, opacity, exposure, residency, \
                 blend, preview or tap"
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

/// **One of a value list the vocabulary owns**, or a complaint naming all of
/// them.
///
/// The list is `karakuri_operation`'s — [`Residency::ALL`], [`BlendMode::ALL`]
/// — rather than a second list here, which is what
/// [P-0074](../../../docs/principles/0074-an-operation-says-what-it-wants-never-which-way-to-move.md)
/// buys: a vocabulary that names destinations has to own the values a
/// destination is drawn from, and a map file is then offered them without a
/// parser's copy to keep in step. The words are the same ones the record
/// carries and the status line prints, so what an operator writes is what they
/// read back.
///
/// `missing` describes the whole line and `what` names the kind of word, so a
/// bare `blend 0` — which is the old spelling — is answered with the three
/// lines that replace it rather than with a bare *expected a mode*.
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

    /// **Both ends of a fader are exact.** A gain that cannot be put at
    /// silence, or at unity, is a fader that cannot be matched against another
    /// slot — and matching them is what a fader is for.
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

    /// **Exposure is a ratio, so the middle of the fader is unity.** A linear
    /// map over `[0.25, 4]` would put 1.0 at a fifth of the way up and spend
    /// three quarters of the travel above it, which is not what a stop is.
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

    /// **A pad and a fader cannot be mapped to each other's targets.** A knob
    /// wired to `on-air` would toggle the slot on every message it sent, which
    /// is sixty times a second while it is moving.
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

    /// **A specific channel wins over `any`.** Otherwise a map that says
    /// "controller 1 anywhere, except on channel 3 where it means something
    /// else" would resolve by whichever the hash happened to reach.
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

    /// **A release does nothing.** Every pad here toggles or selects on the
    /// press, so acting on the release too would undo it — a pad that turned a
    /// slot on and off again before the operator's finger came up.
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
             note 36 -> residency 1 live\n\
             note 37 -> blend 3 over\n\
             note 39 -> preview 1\n\
             note 40 -> preview mix\n\
             note 41 -> tap");
        assert_eq!(m.len(), 8);
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
        assert_eq!(
            m.operation(note(39)),
            Some(Operation::SetPreview { showing: Some(1) })
        );
        assert_eq!(
            m.operation(note(40)),
            Some(Operation::SetPreview { showing: None })
        );
        assert_eq!(m.operation(note(41)), Some(Operation::TapBeat));
    }

    /// **A pad names a state, and every state the vocabulary holds is
    /// writable.** The defect this is against is the one P-0074 describes: a
    /// map that could only say *step it* leaves `allocated` and `max`
    /// unreachable from a surface, and two surfaces stepping one control
    /// disagree about where they are. The words are read off
    /// `Residency::ALL` and `BlendMode::ALL` rather than spelled here twice,
    /// so a value added to the vocabulary is checked by this test on the day
    /// it lands.
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

    /// **A file written against the old grammar is refused, line by line, with
    /// the line to write instead.**
    ///
    /// `on-air 0` meant *flip slot 0*; as a destination it would mean *put
    /// slot 0 live*, and a checked-in map would go on loading and do something
    /// else mid-set. That is the one outcome this format must not have, so the
    /// three old spellings are refused where an operator can read them — and a
    /// refusal that only said *not a control* would leave them guessing at the
    /// grammar that replaced it (ADR-0196).
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

    /// **One bad line is a line, not a file.** A typo in a map mid-set must not
    /// cost the surface, and the operator has to be told which line it was.
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

    /// **The example map in the repository loads clean.** A format documented
    /// by a file nobody parses is a format that drifts from the parser, and
    /// `examples/surface.map` is what an operator copies before they have one
    /// of their own.
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

    /// **A range with one value in it, or a value that is not one.** Both load
    /// as a fader: `[1, 1]` as one whose whole travel is a single number, and
    /// `[nan, 1]` as one that writes NaN into a gain — where nothing downstream
    /// rejects it and the mix does not treat it as silence. Refused where an
    /// operator can read the line rather than discovered on a fader.
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

    /// **The later line wins, and says so.** Two lines for one knob is a map
    /// being edited, and an operator who is told which one took effect can fix
    /// it; one who is not has a knob doing the wrong thing with no clue where.
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

    /// **Every number a line carries is checked against the wire's range.** A
    /// controller number past 127 cannot arrive, and a channel outside 1-16
    /// does not exist — a map that accepted either would have a line that never
    /// answers, which reads exactly like a broken knob.
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
}
