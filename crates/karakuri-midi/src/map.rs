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
//!   note 36 -> on-air 0
//!   note 40 -> prime 1
//!   note 44 -> blend 0
//!   note 48 -> preview 0
//!   note 52 -> preview mix
//!   note 56 -> tap
//! ```
//!
//! ## What this deliberately does not do
//!
//! **It produces an [`Action`], never a record and never a call.** The engine is
//! driven through the record stream and `karakuri-cli` is where a record is
//! built; a second place building them would be two spellings of one rule, and
//! the rule is the invariant that a surface can do nothing a key cannot. What
//! reaches the engine from a fader is the same `gain` record a keypress writes,
//! so **a session recorded from a controller replays with no controller
//! attached** — and the map is not in the stream, because which knob was turned
//! is a property of the room's hardware rather than of the performance.
//!
//! **It is 7-bit.** A control change carries 128 positions and this reads them
//! as 128 positions; the 14-bit MSB/LSB convention is not implemented. That is
//! a real coarseness on a gain — about 0.8% of the range per step — and the
//! honest note is that it has not been a problem to look at rather than that it
//! is fine in principle.

use std::collections::HashMap;

/// What the operator asked for, in terms of the deck rather than of the wire.
///
/// **Engine-neutral on purpose.** No `Residency`, no `Blend`, no `Record`: this
/// crate names the gesture and `karakuri-cli` decides what it is worth, which
/// is what keeps every control reachable from a surface reachable from a key by
/// construction rather than by two lists agreeing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Gain {
        slot: u8,
        value: f32,
    },
    Opacity {
        slot: u8,
        value: f32,
    },
    Exposure {
        value: f32,
    },
    /// On air, or off it. A press, and the deck decides which way — the same
    /// thing the space bar does.
    ToggleOnAir {
        slot: u8,
    },
    /// Ask a slot to warm off air, or withdraw the request.
    TogglePriming {
        slot: u8,
    },
    CycleBlend {
        slot: u8,
    },
    /// Audition a slot, or `None` for the mix. Direct rather than a cycle: a
    /// surface has a pad per slot and reaching slot 3 through three presses is
    /// a keyboard's compromise, not a surface's.
    Preview {
        slot: Option<u8>,
    },
    Tap,
}

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
#[derive(Debug, Clone, Copy, PartialEq)]
enum Target {
    Gain { slot: u8, range: [f32; 2] },
    Opacity { slot: u8, range: [f32; 2] },
    Exposure { range: [f32; 2] },
    ToggleOnAir { slot: u8 },
    TogglePriming { slot: u8 },
    CycleBlend { slot: u8 },
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
    /// pad on a fader's target is refused at parse time rather than producing
    /// an `on-air` toggle every time a knob passes a threshold.
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
    /// A release is deliberately nothing. Every pad here is a press that
    /// toggles or selects, so acting on the release too would undo the press —
    /// and a momentary control that wants both ends is a different target from
    /// these, not the same one read twice.
    pub fn action(&self, message: crate::Message) -> Option<Action> {
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
        // rather than a wrong action.
        // `key.shape()` rather than the shape spelled out per arm: the same
        // function decides which validation a range gets at parse time, so a
        // target that is validated as a ratio cannot be scaled as a line.
        let shape = key.shape();
        match (key, value) {
            (Target::Gain { slot, range }, Some(v)) => Some(Action::Gain {
                slot,
                value: scale(v, range, shape),
            }),
            (Target::Opacity { slot, range }, Some(v)) => Some(Action::Opacity {
                slot,
                value: scale(v, range, shape),
            }),
            (Target::Exposure { range }, Some(v)) => Some(Action::Exposure {
                value: scale(v, range, shape),
            }),
            (Target::ToggleOnAir { slot }, None) => Some(Action::ToggleOnAir { slot }),
            (Target::TogglePriming { slot }, None) => Some(Action::TogglePriming { slot }),
            (Target::CycleBlend { slot }, None) => Some(Action::CycleBlend { slot }),
            (Target::Preview { slot }, None) => Some(Action::Preview { slot }),
            (Target::Tap, None) => Some(Action::Tap),
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
        "on-air" => Target::ToggleOnAir {
            slot: slot(&mut words)?,
        },
        "prime" => Target::TogglePriming {
            slot: slot(&mut words)?,
        },
        "blend" => Target::CycleBlend {
            slot: slot(&mut words)?,
        },
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
        other => {
            return Err(format!(
                "`{other}` is not a control — expected gain, opacity, exposure, on-air, \
                 prime, blend, preview or tap"
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
            m.action(cc(1, 0)),
            Some(Action::Gain {
                slot: 0,
                value: 0.0
            })
        );
        assert_eq!(
            m.action(cc(1, 127)),
            Some(Action::Gain {
                slot: 0,
                value: 1.0
            })
        );
        // And an explicit range, both ends, so the default is not the only one
        // that lands.
        let m = map("cc 1 -> gain 0 [0.5, 2.5]");
        assert_eq!(
            m.action(cc(1, 0)),
            Some(Action::Gain {
                slot: 0,
                value: 0.5
            })
        );
        assert_eq!(
            m.action(cc(1, 127)),
            Some(Action::Gain {
                slot: 0,
                value: 2.5
            })
        );
    }

    /// **Exposure is a ratio, so the middle of the fader is unity.** A linear
    /// map over `[0.25, 4]` would put 1.0 at a fifth of the way up and spend
    /// three quarters of the travel above it, which is not what a stop is.
    #[test]
    fn exposure_moves_in_stops_rather_than_in_equal_steps() {
        let m = map("cc 20 -> exposure");
        let at = |v: u8| match m.action(cc(20, v)) {
            Some(Action::Exposure { value }) => value,
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
        let (m, notes) = Map::parse("cc 1 -> on-air 0\nnote 36 -> gain 0");
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
            m.action(Message::ControlChange {
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

    fn on_any(m: &Map, channel: u8) -> Option<Action> {
        m.action(Message::ControlChange {
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
            m.action(Message::ControlChange {
                channel: 0,
                controller: 1,
                value: 127
            }),
            Some(Action::Gain {
                slot: 3,
                value: 1.0
            })
        );
        assert_eq!(
            m.action(Message::ControlChange {
                channel: 5,
                controller: 1,
                value: 127
            }),
            Some(Action::Gain {
                slot: 0,
                value: 1.0
            })
        );
    }

    /// **A release does nothing.** Every pad here toggles or selects on the
    /// press, so acting on the release too would undo it — a pad that turned a
    /// slot on and off again before the operator's finger came up.
    #[test]
    fn a_release_is_not_a_second_press() {
        let m = map("note 36 -> on-air 0");
        assert_eq!(m.action(note(36)), Some(Action::ToggleOnAir { slot: 0 }));
        assert_eq!(
            m.action(Message::NoteOff {
                channel: 0,
                note: 36
            }),
            None
        );
        // Including the spelling that arrives as a note-on at velocity 0.
        assert_eq!(
            m.action(Message::parse(&[0x90, 36, 0]).expect("a note-on")),
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
             note 36 -> on-air 1\n\
             note 37 -> prime 2\n\
             note 38 -> blend 3\n\
             note 39 -> preview 1\n\
             note 40 -> preview mix\n\
             note 41 -> tap");
        assert_eq!(m.len(), 9);
        assert!(matches!(
            m.action(cc(1, 127)),
            Some(Action::Gain { slot: 2, .. })
        ));
        assert!(matches!(
            m.action(cc(2, 127)),
            Some(Action::Opacity { slot: 3, .. })
        ));
        assert!(matches!(m.action(cc(3, 64)), Some(Action::Exposure { .. })));
        assert_eq!(m.action(note(36)), Some(Action::ToggleOnAir { slot: 1 }));
        assert_eq!(m.action(note(37)), Some(Action::TogglePriming { slot: 2 }));
        assert_eq!(m.action(note(38)), Some(Action::CycleBlend { slot: 3 }));
        assert_eq!(m.action(note(39)), Some(Action::Preview { slot: Some(1) }));
        assert_eq!(m.action(note(40)), Some(Action::Preview { slot: None }));
        assert_eq!(m.action(note(41)), Some(Action::Tap));
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
        assert!(m.action(cc(1, 127)).is_some());
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
            m.action(cc(1, 127)),
            Some(Action::Gain {
                slot: 3,
                value: 1.0
            }),
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
