//! The surface, on the way to the record stream.
//!
//! `karakuri-midi` says what the operator asked for; this says what that is
//! worth. What arrives is a [`karakuri_operation::Operation`] — the same name
//! a key press and a console fader carry — so a control surface can do nothing
//! a keyboard cannot and a session recorded from one replays with neither
//! attached.
//!
//! ```text
//!   a knob ─→ Message ─→ Router ─→ Operation ─→ Live::operate
//!                                                  └→ written() ─→ Record ─→ Deck
//! ```
//!
//! **The exhaustiveness moved and did not go.** This used to be one match over
//! eight `Action`s in [`crate::Live`], and *a control added to one and not the
//! other does not compile* was its whole claim. Against a 46-variant
//! vocabulary that claim would be false, and the guarantee lives where it is
//! now true: `karakuri_operation_record::written` is one exhaustive match over
//! all 46, so an operation nobody has said what to do with stops the build
//! there rather than reaching a router arm nobody wrote
//! (`docs/adr/0196-a-map-line-names-a-state-and-an-old-line-is-refused.md`).
//!
//! ## The map is a file and is not in the stream
//!
//! Which knob is which belongs to the hardware in the room. Two rooms with two
//! surfaces play the same session; replaying one room's wiring in the other
//! would be replaying the furniture. It is the same argument `residency` makes
//! from the other end, where the *request* is recorded and the effective level
//! is recomputed on whatever machine is running.
//!
//! ## Two halves, and the tested one is the one with decisions in it
//!
//! [`Router`] owns the map, the slot check and what gets said out loud. It
//! takes messages and returns actions, so it is a pure function of what arrived
//! and needs no port, no device and no window — which is the whole reason it is
//! not inside [`Surface`]. `Surface` is a `Router` with a port in front of it
//! and has nothing in it to be wrong about.
//!
//! ## Unmapped messages are printed, and that is the learn mode
//!
//! There is no UI to assign a knob in, and none is built. Until there is, the
//! way to find out what a controller sends is to turn it and read: an
//! unmapped message prints the line that would map it, so discovering a surface
//! is turning every knob once and pasting the output into a file.
//!
//! **Everything said here is said once per control**, and that is not tidiness.
//! A fader sweep is several hundred messages, this runs inside `Live::frame`,
//! and `eprintln!` takes a lock and issues a write — so a line per message is a
//! blocking I/O storm on the render thread, which is the first rule this
//! repository has. The same dedup covers the slot check for the same reason:
//! `cc 1 -> gain 4` on a deck of four is the likeliest typo there is, since
//! `ch` on the line above it *is* one-based.

use std::collections::HashSet;
use std::path::Path;

use karakuri_midi::{Map, Message, Port};
use karakuri_operation::Operation;

/// The map, the slot check, and what has already been said. No port.
pub struct Router {
    map: Map,
    /// What this frame has to say, for the caller to say. **Collected rather
    /// than printed**, so that "once per control, not once per message" is
    /// something a test can read rather than something a reader has to trust —
    /// and `eprintln!` inside a frame is what the rule is about in the first
    /// place. Allocates only on a first discovery, of which there are at most
    /// a surface's worth.
    notices: Vec<String>,
    /// What has been reported as unmapped or out of range: `(channel, number,
    /// is_cc)` for the first, `slot` for the second. Two sets because they are
    /// two vocabularies — a `cc 1` and a slot 1 are not the same thing said
    /// twice.
    seen_unmapped: HashSet<(u8, u8, bool)>,
    seen_no_slot: HashSet<usize>,
}

impl Router {
    pub fn new(map: Map) -> Router {
        Router {
            map,
            notices: Vec::new(),
            seen_unmapped: HashSet::new(),
            seen_no_slot: HashSet::new(),
        }
    }

    /// How many mappings loaded, for the line printed on startup.
    pub fn mappings(&self) -> usize {
        self.map.len()
    }

    /// Route `messages` into `out`, saying whatever needs saying.
    ///
    /// `out` is cleared first: an operation left from the previous frame would
    /// be applied again on this one — harmless for a fader that has not moved,
    /// and not for a press.
    ///
    /// A message naming a slot this deck does not have is dropped here rather
    /// than in an index — `Deck::gain` and friends index directly and would
    /// panic on the render thread — and said once. A map is written by hand
    /// against a deck the operator remembers.
    pub fn route(&mut self, messages: &[Message], slot_count: usize, out: &mut Vec<Operation>) {
        out.clear();
        self.notices.clear();
        for message in messages {
            let Some(operation) = self.map.operation(*message) else {
                // A release is unmapped by construction — every pad acts on
                // the press — so reporting one would call the other half of
                // every hit a discovery.
                if !matches!(message, Message::NoteOff { .. }) {
                    self.report_unmapped(*message);
                }
                continue;
            };
            match deck_of(&operation) {
                Some(slot) if slot >= slot_count => self.report_no_slot(slot, slot_count),
                _ => out.push(operation),
            }
        }
    }

    fn report_unmapped(&mut self, message: Message) {
        let (number, is_cc) = match message {
            Message::ControlChange { controller, .. } => (controller, true),
            Message::NoteOn { note, .. } | Message::NoteOff { note, .. } => (note, false),
        };
        if !self
            .seen_unmapped
            .insert((message.channel(), number, is_cc))
        {
            return;
        }
        self.notices.push(format!(
            "unmapped — `{} {number} ch {} -> ...`",
            if is_cc { "cc" } else { "note" },
            // The number printed on the device, which is the one an operator
            // would type into a map. `Message` carries the wire's 0-15.
            message.channel() + 1
        ));
    }

    /// What the last [`Router::route`] found worth saying, in order. Empty on
    /// almost every frame.
    pub fn notices(&self) -> &[String] {
        &self.notices
    }

    /// **The keys' own refusal**, from [`crate::no_such_slot`] rather than
    /// spelled again here. It was spelled again here — with an em dash where the
    /// keys use a colon — which made a map pointing at slot 9 and a digit key
    /// pressed at slot 9 two sentences about one mistake. Said once per slot per
    /// run; see `seen_no_slot`.
    fn report_no_slot(&mut self, slot: usize, slot_count: usize) {
        if !self.seen_no_slot.insert(slot) {
            return;
        }
        self.notices.push(crate::no_such_slot(slot, slot_count));
    }
}

/// The deck an operation names, if it names one.
///
/// **The six arms are every operation a map line can produce**, and
/// `karakuri_midi::map`'s `parse_target` is that list — `cc -> exposure` and
/// `note -> tap` name no deck, and the other forty-two operations have no
/// spelling in the grammar at all. The wildcard is what the vocabulary being
/// forty-eight wide costs here, and it is safe rather than merely convenient:
/// **the record path is the backstop.** A slot this deck does not hold is
/// refused by `mix::change` with [`crate::no_such_slot`] — this very sentence —
/// and nothing moves, where the old `Action` path indexed a `Vec` directly and
/// panicked on the render thread.
///
/// So what this buys is not safety but silence: the refusal is said **once per
/// slot per run** rather than once per message, and `cc 1 -> gain 4` on a deck
/// of four is the likeliest typo a map has.
///
/// **`SetMaskPosition` is here because for it the backstop is not silent.**
/// The other five reach `mix::change` and are refused once; a mask operation is
/// stopped a step earlier, at `Live::operate`, which cannot read the mask of a
/// slot the deck does not hold and answers `Owed::NotRead` — and `operate`
/// prints that, every time. On a fader sweep against a mistyped slot that is a
/// blocking write per message inside `Live::frame`, which is the exact cost
/// this router exists to keep off the frame path.
fn deck_of(operation: &Operation) -> Option<usize> {
    match operation {
        Operation::SetGain { deck, .. }
        | Operation::SetOpacity { deck, .. }
        | Operation::SetResidency { deck, .. }
        | Operation::SetBlendMode { deck, .. }
        | Operation::SetMaskPosition { deck, .. } => Some(usize::from(*deck)),
        Operation::SetPreview { showing } => showing.map(usize::from),
        _ => None,
    }
}

/// An open surface: a [`Router`] with a port in front of it.
pub struct Surface {
    port: Port,
    router: Router,
    /// Scratch for [`Port::drain`]. Owned, and given a capacity at
    /// construction rather than grown into one, because this is read on the
    /// frame path — see [`INBOX`].
    inbox: Vec<Message>,
}

/// Messages a frame is sized for.
///
/// A surface's fastest gesture is a fader sweep, which a device sends at a few
/// hundred messages a second — so a 60 Hz frame sees single figures, and a
/// stall of a second's worth of eight faders moving together is still under
/// this. Reserved at construction so the frame path does not `realloc`, which
/// is the same reason `audio.rs` sizes its buffers up front rather than
/// letting them find their own high-water mark.
const INBOX: usize = 256;

impl Surface {
    /// Open a port and load a map, or say why not.
    ///
    /// A missing map is not an error: a surface with no map still prints what
    /// it sends, which is exactly the state an operator is in before they have
    /// written one.
    pub fn open(port: &str, map_path: Option<&Path>) -> Result<Surface, String> {
        let port = Port::open(port)?;
        let (map, notes) = match map_path {
            Some(path) => {
                let text = std::fs::read_to_string(path)
                    .map_err(|e| format!("reading MIDI map `{}`: {e}", path.display()))?;
                Map::parse(&text)
            }
            None => (Map::default(), Vec::new()),
        };
        for note in &notes {
            eprintln!("  midi map: {note}");
        }
        let router = Router::new(map);
        eprintln!(
            "midi in: `{}`, {} mapping{}{}",
            port.name(),
            router.mappings(),
            if router.mappings() == 1 { "" } else { "s" },
            if router.mappings() == 0 {
                " — turn a knob and this will print the line that would map it"
            } else {
                ""
            }
        );
        Ok(Surface {
            port,
            router,
            inbox: Vec::with_capacity(INBOX),
        })
    }

    /// Everything that arrived since the last frame, as operations this deck
    /// can answer.
    pub fn take(&mut self, slot_count: usize, out: &mut Vec<Operation>) {
        self.port.drain(&mut self.inbox);
        // Split rather than borrowed together: `route` writes to the router and
        // reads the inbox, and both are fields of `self`. `mem::take` would
        // hand the allocation back only if nothing panicked in between.
        let Surface { router, inbox, .. } = self;
        router.route(inbox, slot_count, out);
        for notice in router.notices() {
            eprintln!("  midi: {notice}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn router(text: &str) -> Router {
        let (map, notes) = Map::parse(text);
        assert!(notes.is_empty(), "{notes:?}");
        Router::new(map)
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

    fn routed(r: &mut Router, messages: &[Message], slots: usize) -> Vec<Operation> {
        let mut out = Vec::new();
        r.route(messages, slots, &mut out);
        out
    }

    /// **A message naming a slot the deck does not have is dropped here**, so
    /// the refusal is said once per slot rather than once per message — a
    /// fader sweep into `gain 4` on a deck of four is several hundred of them,
    /// and `cc 1 -> gain 4` is the likeliest typo a map has because the `ch`
    /// on the same line *is* one-based.
    #[test]
    fn an_operation_past_the_end_of_the_deck_is_dropped_rather_than_routed() {
        let mut r = router("cc 1 -> gain 3\ncc 2 -> gain 0");
        let out = routed(&mut r, &[cc(1, 127), cc(2, 64)], 2);
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(matches!(out[0], Operation::SetGain { deck: 0, .. }));
        // The boundary either side of it, which is where an off-by-one lives.
        assert_eq!(routed(&mut r, &[cc(1, 127)], 4).len(), 1);
        assert_eq!(routed(&mut r, &[cc(1, 127)], 3).len(), 0);
    }

    /// **The mask's front is checked here too**, and for a sharper reason than
    /// the other five: a `gain 4` that got past this is refused once by
    /// `mix::change`, where a `mask-position 4` is stopped at `Live::operate`
    /// — which cannot read a mask the deck does not hold, answers
    /// `Owed::NotRead` and **prints it every time**. Left out of `deck_of`,
    /// a fader sweep into a mistyped slot would be a blocking write per
    /// message inside `Live::frame`.
    #[test]
    fn a_mask_front_past_the_end_of_the_deck_is_dropped_here_rather_than_printed() {
        let mut r = router("cc 9 -> mask-position 4\ncc 10 -> mask-position 1");
        let out = routed(&mut r, &[cc(9, 127), cc(10, 64)], 2);
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(matches!(out[0], Operation::SetMaskPosition { deck: 1, .. }));
    }

    /// **Previous frame's operations do not arrive again.** `out` is a buffer
    /// the caller keeps, and one left unclear would apply every press on it a
    /// second time on a frame nobody touched the surface.
    #[test]
    fn a_frame_with_no_messages_produces_no_operations() {
        let mut r = router("note 36 -> residency 0 live");
        let mut out = Vec::new();
        r.route(&[note(36)], 4, &mut out);
        assert_eq!(out.len(), 1);
        r.route(&[], 4, &mut out);
        assert!(out.is_empty(), "{out:?}");
    }

    /// Every message routes to the operation its line names, and only mapped
    /// ones route at all. The seam this file exists for, end to end.
    #[test]
    fn a_mapped_message_becomes_its_operation_and_an_unmapped_one_becomes_nothing() {
        let mut r = router("cc 1 -> gain 2\nnote 36 -> preview 1\nnote 37 -> tap");
        let out = routed(
            &mut r,
            &[cc(1, 127), cc(9, 64), note(36), note(99), note(37)],
            4,
        );
        assert_eq!(
            out,
            vec![
                Operation::SetGain { deck: 2, gain: 1.0 },
                Operation::SetPreview { showing: Some(1) },
                Operation::TapBeat,
            ]
        );
    }

    /// **Said once per control, not once per message.** A fader sweep is
    /// several hundred messages and this runs inside a frame, so a line each
    /// would be a blocking write per message on the render thread. Counted
    /// through the sets rather than by capturing stderr, which is the only
    /// handle a test has on it.
    #[test]
    fn an_unmapped_control_and_a_missing_slot_are_each_reported_once() {
        let mut r = router("cc 1 -> gain 9");
        let sweep: Vec<Message> = (0..128).map(|v| cc(1, v)).collect();
        let unmapped: Vec<Message> = (0..128).map(|v| cc(2, v)).collect();
        routed(&mut r, &sweep, 4);
        routed(&mut r, &unmapped, 4);
        routed(&mut r, &sweep, 4);
        routed(&mut r, &unmapped, 4);
        // Nothing left to say by the fourth pass, which is the claim: the
        // first sweep said it and no message since has said it again.
        assert!(r.notices().is_empty(), "{:?}", r.notices());

        // And each of them was said exactly once, from a fresh router.
        let mut r = router("cc 1 -> gain 9");
        let mut said = Vec::new();
        for messages in [&sweep, &unmapped, &sweep, &unmapped] {
            routed(&mut r, messages, 4);
            said.extend(r.notices().iter().cloned());
        }
        assert_eq!(said.len(), 2, "{said:?}");
        // **The keys' own sentence, word for word.** This asked only for
        // `contains("no slot 9")`, which passed while this module said `no slot
        // 9 — this deck holds slots 0-3` and every other surface said `no slot
        // 9: …`. See [`crate::no_such_slot`].
        assert!(
            said.iter().any(|s| *s == crate::no_such_slot(9, 4)),
            "{said:?}"
        );
        assert!(said.iter().any(|s| s.contains("cc 2")), "{said:?}");
    }

    /// **The dedup key distinguishes everything that is a different control.**
    /// A `cc 1` and a `note 1` are two knobs, and the same number on two
    /// channels is two knobs on two devices — collapsing either would leave an
    /// operator turning something that never prints.
    #[test]
    fn two_different_controls_are_two_discoveries() {
        let mut r = router("");
        routed(
            &mut r,
            &[
                cc(1, 0),
                note(1),
                Message::ControlChange {
                    channel: 5,
                    controller: 1,
                    value: 0,
                },
            ],
            4,
        );
        assert_eq!(r.notices().len(), 3, "{:?}", r.notices());
    }

    /// A release is not a discovery. Every pad acts on the press, so reporting
    /// the release would print the other half of every hit as something the
    /// operator had not mapped.
    #[test]
    fn a_release_is_not_reported_as_unmapped() {
        let mut r = router("note 36 -> residency 0 live");
        routed(
            &mut r,
            &[
                Message::NoteOff {
                    channel: 0,
                    note: 36,
                },
                Message::NoteOff {
                    channel: 0,
                    note: 99,
                },
            ],
            4,
        );
        assert!(r.notices().is_empty(), "{:?}", r.notices());
    }
}
