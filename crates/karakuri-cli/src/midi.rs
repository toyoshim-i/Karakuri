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
//! other does not compile* was its whole claim. Against a fifty-variant
//! vocabulary that claim would be false, and the guarantee lives where it is
//! now true: `karakuri_operation_record::written` is one exhaustive match over
//! all fifty, so an operation nobody has said what to do with stops the build
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
//! ## A continuous control says one thing per frame, and a pad says everything
//!
//! A sweep is several hundred messages and a frame renders once, so a fader's
//! earlier values are positions it passed through rather than places it was.
//! [`Router::emit`] keeps the last value each continuous control sent within a
//! frame and drops the ones before it — which is what takes a record's
//! `String` off the frame path on `exposure` and `mask-position`, where
//! building one per message was an allocation per message inside `Live::frame`
//! and the first rule this repository has says there is none
//! (`docs/adr/0207-a-continuous-control-says-one-thing-per-frame.md`, and
//! `crate::mix` carries the measurement).
//!
//! **A pad is untouched.** Two presses in one frame are two operations that
//! both mean something, and the line between the two halves is
//! `karakuri_midi::Map::is_continuous` rather than a list kept here.
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
    /// Where in `out` each continuous control this frame already spoke put its
    /// operation, so a later message from the same control overwrites it
    /// instead of adding one. Cleared at the top of every [`Router::route`];
    /// see [`Router::emit`], which is where the whole of coalescing is.
    ///
    /// **A `Vec` with room for the whole map and a linear scan**, not a
    /// `HashMap`: a map's continuous controls are single figures, and a
    /// `HashMap` that had to grow would allocate on the frame path — which is
    /// what this field exists to stop. The capacity is an upper bound rather
    /// than a guess, because every key here comes from a target and a target
    /// comes from an entry.
    coalescing: Vec<(Continuous, usize)>,
}

/// **What state a continuous operation names**: its variant, and the deck it
/// names if it names one.
///
/// The key a frame's messages are coalesced by, and it is deliberately not the
/// knob's number on the wire. Two lines can put two knobs on one `gain 0` —
/// and a line that names no channel puts one knob on every channel — where
/// what reaches the deck is one value either way, so the control being
/// coalesced is the thing that moves rather than the hand on it.
///
/// **`discriminant` rather than a match over the continuous operations.** A
/// list here would be a second answer to [`Map::is_continuous`]'s question,
/// kept in step by hand against a fifty-variant vocabulary; this is the
/// same "which one is it" the compiler already knows. `deck_of` is beside it
/// because `gain 0` and `gain 1` are two faders, and it is the function this
/// module already had for the question.
type Continuous = (std::mem::Discriminant<Operation>, Option<usize>);

impl Router {
    pub fn new(map: Map) -> Router {
        let controls = map.len();
        Router {
            map,
            notices: Vec::new(),
            seen_unmapped: HashSet::new(),
            seen_no_slot: HashSet::new(),
            coalescing: Vec::with_capacity(controls),
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
    ///
    /// **A continuous control says one thing per frame**, which is
    /// [`Router::emit`]: the last value a fader sent within a frame is the one
    /// that becomes an operation and the ones before it are dropped. A pad is
    /// untouched — two presses in one frame are two operations.
    pub fn route(&mut self, messages: &[Message], slot_count: usize, out: &mut Vec<Operation>) {
        out.clear();
        self.notices.clear();
        self.coalescing.clear();
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
                _ => self.emit(*message, operation, out),
            }
        }
    }

    /// **One operation per continuous control per frame, carrying the last
    /// value that control sent**; everything else is pushed as it arrives.
    ///
    /// A sweep is several hundred messages and a frame renders once, so the
    /// values before the last are positions a fader passed *through* rather
    /// than places it was: `Live` applies each operation into the deck and the
    /// frame draws what the deck holds afterwards, and a replay applies every
    /// record between two `tick`s before drawing the frame they close. Neither
    /// side of the recording can show a value that was overwritten within a
    /// frame, so what is dropped here was never on screen and never
    /// reconstructible — see
    /// `docs/adr/0207-a-continuous-control-says-one-thing-per-frame.md`, which
    /// also carries what it costs the record stream.
    ///
    /// **The line between a fader and a pad is
    /// [`karakuri_midi::Map::is_continuous`]**, which is `Target::continuous`
    /// — the same predicate the grammar refuses a `note` on a fader's target
    /// with. Two presses in one frame are two operations that both mean
    /// something: `residency 0 live` then `residency 0 allocated` is not the
    /// second one alone, and a note is a press rather than a position.
    ///
    /// **Coalesced, not filtered for change.** A repeat is dropped within a
    /// frame and never across two, which is the difference between this and
    /// the console's fader — *"a pointer dragged on past the end of a track
    /// asks for the end sixty times a second and the value is already there"*.
    /// The console holds the value it last drew; this holds nothing between
    /// frames and could not, because MIDI out is not built: a transition can
    /// move the mask front under a hand that is not moving, so a fader
    /// re-asserting the position it last sent is asking for something the deck
    /// may no longer be at.
    ///
    /// **Nothing here allocates.** `coalescing` has the map's own length
    /// reserved and is cleared rather than dropped, the scan is over single
    /// figures, and the overwrite drops a scalar — every operation a map line
    /// can name carries scalars only, which `karakuri_midi::map` says of
    /// itself.
    fn emit(&mut self, message: Message, operation: Operation, out: &mut Vec<Operation>) {
        if !self.map.is_continuous(message) {
            out.push(operation);
            return;
        }
        let control = (std::mem::discriminant(&operation), deck_of(&operation));
        if let Some((_, at)) = self.coalescing.iter().find(|(seen, _)| *seen == control) {
            // **In place, so the control keeps the position it first spoke
            // in.** The alternative — dropping the earlier one and pushing the
            // later — shifts a swept fader behind every pad hit during the
            // sweep, and a frame's operations are applied in the order they
            // are given.
            out[*at] = operation;
            return;
        }
        self.coalescing.push((control, out.len()));
        out.push(operation);
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
/// fifty wide costs here, and it is safe rather than merely convenient:
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

    /// **A sweep in one frame is one operation, carrying the value the fader
    /// ended the frame at.** Several hundred messages arrive between two
    /// frames; each one built a record, and on `exposure` and `mask-position`
    /// that record carries a name, which is a heap allocation per message
    /// inside `Live::frame` — the first rule this repository has. The values
    /// before the last were never on screen: the frame draws what the deck
    /// holds once, after all of them have been applied.
    #[test]
    fn a_sweep_in_one_frame_is_one_operation_carrying_the_last_value() {
        let mut r = router("cc 20 -> exposure");
        let sweep: Vec<Message> = (0..128).map(|v| cc(20, v)).collect();
        let out = routed(&mut r, &sweep, 4);
        assert_eq!(out.len(), 1, "{out:?}");
        // The top of the default range, which is what `cc 20 127` asks for.
        assert_eq!(out[0], Operation::SetExposure { exposure: 4.0 });
    }

    /// **Two presses in one frame are two operations**, and that is the half
    /// of this that must not coalesce: `residency 0 live` then `residency 0
    /// allocated` is not the second one alone in intent, and a note is a press
    /// rather than a position. The same control, so a coalescer that keyed on
    /// the pad would keep one of them.
    #[test]
    fn two_presses_of_one_pad_in_one_frame_are_two_operations() {
        let mut r = router("note 36 -> residency 0 live\nnote 37 -> residency 0 allocated");
        let out = routed(&mut r, &[note(36), note(37)], 4);
        assert_eq!(
            out,
            vec![
                Operation::SetResidency {
                    deck: 0,
                    residency: karakuri_operation::Residency::Live
                },
                Operation::SetResidency {
                    deck: 0,
                    residency: karakuri_operation::Residency::Allocated
                },
            ]
        );
        // And the same pad twice, which is a press repeated rather than a
        // value repeated: both are hits.
        let mut r = router("note 36 -> preview 1");
        assert_eq!(routed(&mut r, &[note(36), note(36)], 4).len(), 2);
    }

    /// **Two faders are two controls.** Coalescing is per control, so a frame
    /// in which four of them moved says four things — collapsing to one
    /// operation a frame would leave three faders dead whenever a hand was on
    /// a fourth.
    #[test]
    fn two_continuous_controls_in_one_frame_are_one_operation_each() {
        let mut r = router("cc 1 -> gain 0\ncc 2 -> gain 1");
        let messages: Vec<Message> = (0..64).flat_map(|v| [cc(1, v), cc(2, 127 - v)]).collect();
        let out = routed(&mut r, &messages, 4);
        assert_eq!(out.len(), 2, "{out:?}");
        // Each carries its own last value, and the first control keeps the
        // place it first spoke in.
        assert_eq!(
            out,
            vec![
                Operation::SetGain {
                    deck: 0,
                    gain: 63.0 / 127.0
                },
                Operation::SetGain {
                    deck: 1,
                    gain: 64.0 / 127.0
                },
            ]
        );
    }

    /// **Coalescing is per frame and not a filter on change.** The same value
    /// on the next frame is asked for again, because nothing here holds what a
    /// control last sent and nothing could: MIDI out is not built, so a
    /// transition can move the mask front under a hand that is not moving, and
    /// a fader re-asserting its position is asking for somewhere the deck may
    /// no longer be.
    #[test]
    fn a_value_repeated_on_the_next_frame_is_not_swallowed() {
        let mut r = router("cc 9 -> mask-position 0");
        let held = Operation::SetMaskPosition {
            deck: 0,
            position: 1.0,
        };
        for _ in 0..3 {
            // A knob held against its stop keeps sending; three frames of it.
            let out = routed(&mut r, &[cc(9, 127), cc(9, 127)], 4);
            assert_eq!(out, vec![held.clone()], "{out:?}");
        }
    }

    /// **A pad hit during a sweep keeps its place in the frame.** The fader
    /// holds the position it first spoke in and carries the value it ended at,
    /// so a press that arrived between two of its messages is still applied
    /// after it — the order a frame's operations are given is the order they
    /// are applied in.
    #[test]
    fn a_press_between_two_fader_messages_keeps_its_order() {
        let mut r = router("cc 1 -> gain 0\nnote 36 -> preview 1");
        let out = routed(&mut r, &[cc(1, 0), note(36), cc(1, 127)], 4);
        assert_eq!(
            out,
            vec![
                Operation::SetGain { deck: 0, gain: 1.0 },
                Operation::SetPreview { showing: Some(1) },
            ]
        );
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
