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
//! The exhaustiveness moved and did not go. This used to be one match over
//! eight `Action`s in the CLI's `Live` — a plain name rather than a link, since
//! ADR-0215 keeps the window and the device with the surface and this module is
//! one crate over from it now — and *a control added to one and not the other
//! does not compile* was its whole claim. Against a fifty-variant
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
//! ## Two tiers, and two ways to open a port
//!
//! Which map is [`map_for`]: the operator's own under
//! `<store>/maps/default.map`, then the `examples/surface.map` that ships, and
//! `None` for a machine with neither. That is ADR-0227's two tiers, and it is
//! the one place that record found the shape ragged — a map had a preset tier
//! and no operator tier at all, being *"whatever path they hand to
//! `--midi-map FILE`"*.
//!
//! Which port is the difference between the two constructors, and it is
//! the difference between the two programs rather than a convenience.
//! [`Surface::open`] takes a selector and is the command line's: a flag is a
//! contract made before the run, so asking for a port and getting another is a
//! run that is not the run that was asked for. [`Surface::first`] takes
//! whatever is plugged in and is the panel's, for the reason `crate::audio`'s
//! default input is the panel's: an instrument with somebody standing in front
//! of it opens something rather than nothing, and what it opened is a sentence
//! it says out loud.
//!
//! [`Surface::first`] also takes a wake, because the panel's loop sleeps —
//! see [`karakuri_midi::Port::waking`], which carries that argument whole.
//!
//! ## Unmapped messages are printed, and that is `karakuri-cli`'s learn mode
//!
//! An unmapped message prints the line that would map it, so discovering a
//! surface from the command line is turning every knob once and pasting the
//! output into a file. The panel has a real one now — arm `learn`, point
//! at a control, move a knob, and [`Surface::learn`] writes the line into the
//! operator's own map
//! (`docs/adr/0336-a-learn-is-a-map-edit-and-the-tips-midi-line-is-the-live-map.md`).
//! The printed line stays, because `karakuri-cli` has no pointer to point with
//! and because it is what tells an operator what their controller sends at
//! all.
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
//! A pad is untouched. Two presses in one frame are two operations that
//! both mean something, and the line between the two halves is
//! `karakuri_midi::Map::is_continuous` rather than a list kept here.
//!
//! ## The surface is written to as well, and the frame does not wait for it
//!
//! [`Router::shown`] is the other direction: every mapped control's current
//! value, read back through [`Feedback`] and turned into wire messages, so a
//! surface's LEDs and motorised faders follow the deck. It runs on the frame
//! the change lands — the drain and the send are one pass — and it sends
//! only what *moved*, which is the difference between it and writing the whole
//! map out sixty times a second.
//!
//! Nothing on the frame path blocks (P-0094): what [`Surface::show`] does
//! with the bytes is `karakuri_midi::Out::send`, a bounded queue to a thread
//! that owns the connection, dropping and counting when it is full — ADR-0067's
//! shape, and `karakuri_midi::device`'s own documentation carries why a bound
//! is right for this stream and not for a session's.
//!
//! And nothing is written into the record stream. A surface being shown
//! where the deck is produces no `Operation` and no `Record`: the wire is the
//! only thing that changes, so a session recorded from a controller still
//! replays with neither controller nor map attached
//! ([P-0092](../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)).
//!
//! Everything said here is said once per control, and that is not tidiness.
//! A fader sweep is several hundred messages, this runs inside `Live::frame`,
//! and `eprintln!` takes a lock and issues a write — so a line per message is a
//! blocking I/O storm on the render thread, which is the first rule this
//! repository has. The same dedup covers the slot check for the same reason:
//! `cc 1 -> gain 4` on a deck of four is the likeliest typo there is, since
//! `ch` on the line above it *is* one-based.

use std::collections::HashSet;
use std::path::Path;

use karakuri_midi::{Control, Echo, Half, Map, Out, Port, Shown};

/// The wire, parsed — `karakuri_midi::Message`, re-exported because a learn is
/// a gesture the *program* drives and a program has to be able to say what
/// arrived.
///
/// It is the one type of that crate this door hands on. Naming `karakuri-midi`
/// in `crates/karakuri`'s manifest would put `midir` in the instrument's own
/// dependency list for a two-line match, and the charter is that everything
/// outside the process is reached through here (ADR-0215).
pub use karakuri_midi::Message;
use karakuri_operation::{Operation, ParamAt, ParamValue};

/// A deck's published interface, asked what is at a position — the only
/// readback on this route, and the reason it is here rather than in
/// `karakuri-midi`.
///
/// A `param` line holds a position and never a key (ADR-0268): *knob 3 is knob
/// 3 whatever Set is loaded*. Turning position 3 into a `ParamAt` is a question
/// about the Set that is in the deck right now, so `karakuri-midi` answers
/// [`karakuri_midi::Map::parameter`] and stops, and this is what finishes it.
///
/// A trait rather than the deck itself, and that is what keeps [`Router`]'s
/// tests free of a device: `karakuri_engine::deck::Deck` cannot be built
/// without one, and every decision on this route is about what arrived rather
/// than about what is running. [`Decks`] is the real implementation and is four
/// lines.
pub trait Interface {
    /// The control at `position` — counting from one, the number the Inspector
    /// draws — of the deck in `slot`, and the range the Set published it over.
    ///
    /// `None` where that deck has no such control, which is the ordinary state
    /// after a load: a map learned against a Set with nine controls, played against
    /// one with four, has five lines that reach nothing. Reported once per position
    /// rather than dropped in silence.
    fn control_at(&self, slot: u8, position: u16) -> Option<(ParamAt, [f32; 2])>;
}

/// [`Interface`] over a real deck, which is the implementation both programs
/// use and the only one that is not a test's.
///
/// It reads exactly what the Inspector reads and numbers it exactly as the
/// Inspector numbers it — `Set::published()` in order, counting from one — so
/// the position a learn writes is the position the pane draws. That is one
/// derivation asked twice rather than two, and the alternative is a map line
/// that means a different control from the one the number is beside.
pub struct Decks<'a>(pub &'a karakuri_engine::deck::Deck);

impl Interface for Decks<'_> {
    fn control_at(&self, slot: u8, position: u16) -> Option<(ParamAt, [f32; 2])> {
        let set = self.0.slot(karakuri_engine::DeckSlot(slot)).set();
        // **Counting from one**, and `checked_sub` rather than `- 1`: a zero
        // is refused at parse time, so reaching here with one is a caller that
        // built a `Parameter` by hand.
        let at = usize::from(position).checked_sub(1)?;
        let control = set.published().into_iter().nth(at)?;
        Some((
            ParamAt {
                node: control
                    .at
                    .map(|(kind, index)| karakuri_operation::NodeAddress {
                        layer: layer_of(kind),
                        index,
                    }),
                key: control.key,
            },
            control.range,
        ))
    }
}

/// What a mapped control is at right now, so the surface can be shown it — the
/// readback MIDI out needs, and [`Interface`]'s twin at the other end of the
/// same route.
///
/// Two traits and not one, because they are asked different questions by
/// different callers: [`Interface`] resolves a *position* on the way in and is
/// asked once per message that arrives, and this is asked once per mapped
/// control per frame on the way out. They are also answered by different things
/// — a `param` line's way in needs the deck alone, and the way out needs the
/// room's look as well, because `exposure` is a map line and is not a deck's.
///
/// `None` is a control this program cannot read, and nothing is sent for it.
/// `tap` is the permanent one — a beat has no state — and a slot past the end
/// of the deck is the ordinary one.
pub trait Feedback {
    /// Where `control` is, or `None` for one there is nothing to show.
    fn shown(&self, control: Control) -> Option<Shown>;
}

/// [`Feedback`] over a real deck and the look it is drawn under, which is the
/// implementation both programs use and the only one that is not a test's.
///
/// It reads a published control exactly as [`Decks`] does — `Set::published()`
/// in order, counting from one — so what is shown on a knob is the control that
/// knob moves, and the Inspector's number is the number in the line.
pub struct Lit<'a> {
    pub deck: &'a karakuri_engine::deck::Deck,
    /// The room's exposure, which is not the deck's and is the reason this is not
    /// `Decks`. Each program holds its own: the panel's is `Engine::look` and
    /// `karakuri-cli`'s is `Live::look`.
    pub exposure: f32,
}

impl Feedback for Lit<'_> {
    fn shown(&self, control: Control) -> Option<Shown> {
        // A slot this deck does not hold is nothing to show rather than a
        // panic: `Deck::gain` and its neighbours index directly, and a map
        // written against a deck of four played on a deck of one is the
        // ordinary state the router already says one sentence about.
        let slot = |deck: u8| karakuri_engine::DeckSlot::new(deck, self.deck.slot_count());
        Some(match control {
            Control::Gain { deck } => Shown::At(self.deck.gain(slot(deck)?)),
            Control::Opacity { deck } => Shown::At(self.deck.opacity(slot(deck)?)),
            Control::Exposure => Shown::At(self.exposure),
            Control::MaskPosition { deck } => Shown::At(self.deck.mask(slot(deck)?).position()),
            // **A pad shows whether the deck is in the state that pad names**,
            // which is what makes a residency row on a surface a readout as
            // well as a control: one of the three is lit.
            Control::Residency { deck, residency } => {
                Shown::On(crate::mix::residency(self.deck.residency(slot(deck)?)) == residency)
            }
            Control::Blend { deck, blend } => {
                Shown::On(crate::mix::blend_mode(self.deck.blend(slot(deck)?)) == blend)
            }
            Control::Param { deck, position } => {
                let set = self.deck.slot(slot(deck)?).set();
                let at = usize::from(position).checked_sub(1)?;
                let published = set.published().into_iter().nth(at)?;
                Shown::Published {
                    value: set.value_at(published.at, &published.key)?,
                    declared: published.range,
                }
            }
            // A beat has no state to show, so nothing is ever sent for a
            // `tap` line.
            Control::Tap => return None,
        })
    }
}

/// The compiler's layer as the vocabulary spells it — [`crate::mcp`]'s
/// `kind_of` backwards, and written here because that one is private to the
/// module that serves a model.
fn layer_of(kind: karakuri_ir::Kind) -> karakuri_operation::Layer {
    use karakuri_ir::Kind;
    match kind {
        Kind::L1 => karakuri_operation::Layer::L1,
        Kind::L2 => karakuri_operation::Layer::L2,
        Kind::L3 => karakuri_operation::Layer::L3,
        Kind::L4 => karakuri_operation::Layer::L4,
        Kind::Field => karakuri_operation::Layer::Field,
        Kind::L5 => karakuri_operation::Layer::L5,
    }
}

/// The map, the slot check, and what has already been said. No port.
pub struct Router {
    map: Map,
    /// What this frame has to say, for the caller to say. Collected rather than
    /// printed, so that "once per control, not once per message" is something a
    /// test can read rather than something a reader has to trust — and `eprintln!`
    /// inside a frame is what the rule is about in the first place. Allocates only
    /// on a first discovery, of which there are at most a surface's worth.
    notices: Vec<String>,
    /// What has been reported as unmapped or out of range: `(channel, number,
    /// is_cc)` for the first, `slot` for the second. Two sets because they are two
    /// vocabularies — a `cc 1` and a slot 1 are not the same thing said twice.
    seen_unmapped: HashSet<(u8, u8, bool)>,
    seen_no_slot: HashSet<usize>,
    /// A `param` line reaching past the end of a deck's interface, said once per
    /// `(deck, position)`. A third set for the two above's reason: a slot 9 and a
    /// position 9 are not the same thing said twice, and this is the ordinary state
    /// after a load rather than a typo — a map learned against a Set with nine
    /// controls has five dead lines against one with four, and it is worth exactly
    /// one sentence each.
    seen_no_control: HashSet<(u8, u16)>,
    /// Where in `out` each continuous control this frame already spoke put its
    /// operation, so a later message from the same control overwrites it instead of
    /// adding one. Cleared at the top of every [`Router::route`]; see
    /// [`Router::emit`], which is where the whole of coalescing is.
    ///
    /// A `Vec` with room for the whole map and a linear scan, not a `HashMap`: a
    /// map's continuous controls are single figures, and a `HashMap` that had to
    /// grow would allocate on the frame path — which is what this field exists to
    /// stop. The capacity is an upper bound rather than a guess, because every key
    /// here comes from a target and a target comes from an entry.
    coalescing: Vec<(Continuous, usize)>,
    /// The MSB last seen for each 14-bit control, keyed by the channel it arrived
    /// on and the pair's own controller number (`karakuri_midi::Wide::control`).
    ///
    /// The one piece of state a `cc14` pair needs, and it is here because
    /// `karakuri_midi::Map` is a pure function of one message: the two halves are
    /// two messages with a frame boundary free to fall between them. See that
    /// module's *A fader is 128 positions, or 16384*, which carries the lone-MSB
    /// rule this holds the state for.
    ///
    /// A `Vec` with the map's own length reserved and a linear scan, for
    /// `coalescing`'s reason exactly: a map's 14-bit controls are single figures
    /// and a `HashMap` that had to grow would allocate on the frame path. It is
    /// pushed to once per control per run and read after that.
    halves: Vec<((u8, u8), u8)>,
    /// Every mapped control, as something to show a surface — the map read the
    /// other way, built once because it allocates and this is read inside a frame.
    /// Rebuilt on a learn, which is the only other moment a map changes.
    echoes: Vec<Echo>,
    /// What each control above was last shown at, one per `echoes` entry and `None`
    /// for one never shown.
    ///
    /// This is what makes MIDI out *"on the frame the change lands"* rather than
    /// *"every frame"*: a control whose position has not moved by a step of its own
    /// fader is a message that would say nothing, and a surface's queue is better
    /// spent on the ones that did move.
    lit: Vec<Option<u16>>,
}

/// What state a continuous operation names: its variant, and the deck it names
/// if it names one.
///
/// The key a frame's messages are coalesced by, and it is deliberately not the
/// knob's number on the wire. Two lines can put two knobs on one `gain 0` — and
/// a line that names no channel puts one knob on every channel — where what
/// reaches the deck is one value either way, so the control being coalesced is
/// the thing that moves rather than the hand on it.
///
/// `discriminant` rather than a match over the continuous operations. A list
/// here would be a second answer to [`Map::is_continuous`]'s question, kept in
/// step by hand against a fifty-variant vocabulary; this is the same "which one
/// is it" the compiler already knows. `deck_of` is beside it because `gain 0`
/// and `gain 1` are two faders, and it is the function this module already had
/// for the question. And the position, where the operation is a write into a
/// Set. A deck's discriminant and slot are enough for every other continuous
/// control, because a deck has one gain and one exposure; it has as many
/// parameters as its Set published, and two knobs on two of them would coalesce
/// into one under a key that could not tell them apart. This is the map's own
/// address for the control rather than a second one — the position on the line
/// — and it is `None` for the six targets that are not a parameter.
type Continuous = (
    std::mem::Discriminant<Operation>,
    Option<usize>,
    Option<u16>,
);

/// At what resolution a message is read — [`Router::paired`]'s answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Paired {
    /// Seven bits, which is every plain `cc` line, every `note`, and a `cc14`
    /// line's MSB half on its own.
    Seven,
    /// Fourteen bits, `(msb << 7) | lsb`.
    Wide(u16),
    /// An LSB half with no MSB held for its control. It moves nothing.
    Lone,
}

impl Router {
    pub fn new(map: Map) -> Router {
        let controls = map.len();
        let echoes = map.echoes();
        Router {
            map,
            notices: Vec::new(),
            seen_unmapped: HashSet::new(),
            seen_no_slot: HashSet::new(),
            seen_no_control: HashSet::new(),
            coalescing: Vec::with_capacity(controls),
            halves: Vec::with_capacity(controls),
            lit: vec![None; echoes.len()],
            echoes,
        }
    }

    /// How many mappings loaded, for the line printed on startup.
    pub fn mappings(&self) -> usize {
        self.map.len()
    }

    /// Route `messages` into `out`, saying whatever needs saying.
    ///
    /// `out` is cleared first: an operation left from the previous frame would be
    /// applied again on this one — harmless for a fader that has not moved, and not
    /// for a press.
    ///
    /// A message naming a slot this deck does not have is dropped here rather than
    /// in an index — `Deck::gain` and friends index directly and would panic on the
    /// render thread — and said once. A map is written by hand against a deck the
    /// operator remembers.
    ///
    /// A continuous control says one thing per frame, which is [`Router::emit`]:
    /// the last value a fader sent within a frame is the one that becomes an
    /// operation and the ones before it are dropped. A pad is untouched — two
    /// presses in one frame are two operations.
    pub fn route(
        &mut self,
        messages: &[Message],
        slot_count: usize,
        interface: &dyn Interface,
        out: &mut Vec<Operation>,
    ) {
        out.clear();
        self.notices.clear();
        self.coalescing.clear();
        for message in messages {
            // **Two accessors and they are disjoint by target**, which
            // `karakuri_midi::Map::parameter` carries the argument for: every
            // target but one addresses something the vocabulary spells
            // outright, and `param` addresses a *position* in a deck's
            // published interface, which only the deck can resolve.
            // **A 14-bit pair, assembled** — or the message as it stands,
            // which is every 7-bit line and a `cc14` line's MSB half. See
            // [`Router::paired`], where the whole of the pairing is.
            let asked = match self.paired(*message) {
                // A lone LSB with no MSB held: there is nothing to refine, and
                // it is not a discovery either — the line that names it is
                // loaded and its other half has simply not arrived yet.
                Paired::Lone => continue,
                Paired::Wide(value) => (
                    self.map.operation_wide(*message, value),
                    self.map.parameter_wide(*message, value),
                ),
                Paired::Seven => (self.map.operation(*message), self.map.parameter(*message)),
            };
            let (operation, position) = match asked {
                (Some(operation), _) => (Some(operation), None),
                (None, Some(asked)) => (
                    self.resolved(asked, slot_count, interface),
                    Some(asked.position),
                ),
                (None, None) => {
                    // A release is unmapped by construction — every pad
                    // acts on the press — so reporting one would call the
                    // other half of every hit a discovery.
                    if !matches!(message, Message::NoteOff { .. }) {
                        self.report_unmapped(*message);
                    }
                    continue;
                }
            };
            // Resolution failed and said so; nothing more to do with it.
            let Some(operation) = operation else {
                continue;
            };
            match deck_of(&operation) {
                Some(slot) if slot >= slot_count => self.report_no_slot(slot, slot_count),
                _ => self.emit(*message, operation, position, out),
            }
        }
    }

    /// A position in a deck's published interface, resolved to the control it is,
    /// or `None` with the reason said once.
    ///
    /// Two refusals and they are different facts, which is why each has its own
    /// sentence and its own set. A deck this deck does not have is the same mistake
    /// `cc 1 -> gain 4` is and gets the keys' own words. A position the Set in that
    /// deck has no control at is not a mistake at all in the same way: it is what
    /// every map does after a load that shortened the interface, and a knob that
    /// goes quiet with nothing said is the one outcome
    /// [P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
    /// rules out.
    ///
    /// The value is scaled here and the range comes from the Set —
    /// `Published::range`, which narrows the procedure's declaration — unless the
    /// line wrote one, which wins. The arithmetic is
    /// `karakuri_midi::Parameter::value`'s rather than repeated here, so a fader's
    /// ends are exact on this target for the reason they are on every other one.
    fn resolved(
        &mut self,
        asked: karakuri_midi::Parameter,
        slot_count: usize,
        interface: &dyn Interface,
    ) -> Option<Operation> {
        if usize::from(asked.deck) >= slot_count {
            self.report_no_slot(usize::from(asked.deck), slot_count);
            return None;
        }
        let Some((param, declared)) = interface.control_at(asked.deck, asked.position) else {
            self.report_no_control(asked.deck, asked.position);
            return None;
        };
        Some(Operation::WriteParam {
            deck: asked.deck,
            param,
            value: ParamValue::Scalar(asked.value(declared)),
        })
    }

    /// One operation per continuous control per frame, carrying the last value that
    /// control sent; everything else is pushed as it arrives.
    ///
    /// A sweep is several hundred messages and a frame renders once, so the values
    /// before the last are positions a fader passed *through* rather than places it
    /// was: `Live` applies each operation into the deck and the frame draws what
    /// the deck holds afterwards, and a replay applies every record between two
    /// `tick`s before drawing the frame they close. Neither side of the recording
    /// can show a value that was overwritten within a frame, so what is dropped
    /// here was never on screen and never reconstructible — see
    /// `docs/adr/0207-a-continuous-control-says-one-thing-per-frame.md`, which also
    /// carries what it costs the record stream.
    ///
    /// The line between a fader and a pad is [`karakuri_midi::Map::is_continuous`],
    /// which is `Target::continuous` — the same predicate the grammar refuses a
    /// `note` on a fader's target with. Two presses in one frame are two operations
    /// that both mean something: `residency 0 live` then `residency 0 allocated` is
    /// not the second one alone, and a note is a press rather than a position.
    ///
    /// Coalesced, not filtered for change. A repeat is dropped within a frame and
    /// never across two, which is the difference between this and the console's
    /// fader — *"a pointer dragged on past the end of a track asks for the end
    /// sixty times a second and the value is already there"*. The console holds the
    /// value it last drew; this holds nothing between frames and could not, because
    /// MIDI out is not built: a transition can move the mask front under a hand
    /// that is not moving, so a fader re-asserting the position it last sent is
    /// asking for something the deck may no longer be at.
    ///
    /// Nothing here allocates. `coalescing` has the map's own length reserved and
    /// is cleared rather than dropped, the scan is over single figures, and the
    /// overwrite drops a scalar — every operation a map line can name carries
    /// scalars only, which `karakuri_midi::map` says of itself.
    fn emit(
        &mut self,
        message: Message,
        operation: Operation,
        position: Option<u16>,
        out: &mut Vec<Operation>,
    ) {
        if !self.map.is_continuous(message) {
            out.push(operation);
            return;
        }
        let control = (
            std::mem::discriminant(&operation),
            deck_of(&operation),
            position,
        );
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

    /// Which resolution this message is read at, holding the MSB half of a
    /// 14-bit pair as it goes past.
    ///
    /// The rule is `karakuri_midi::map`'s and this is where its one piece of
    /// state lives:
    ///
    /// - A 7-bit line is [`Paired::Seven`] and nothing is held.
    /// - An MSB half is held *and* answered [`Paired::Seven`], so it moves
    ///   the control coarsely at `msb / 127` — both ends of the fader exact,
    ///   which is what keeps a surface that sends no LSB from being a fader
    ///   that cannot quite arrive.
    /// - An LSB half with an MSB held is [`Paired::Wide`], carrying
    ///   `(msb << 7) | lsb`.
    /// - An LSB half with nothing held is [`Paired::Lone`] and moves
    ///   nothing: there is nothing to refine yet.
    ///
    /// Nothing waits and nothing is timed. Every message that arrives is
    /// acted on as it arrives, so no fader is left between two values by a
    /// pair that did not finish — which is why there is no deadline here and
    /// no clock to hang one on.
    ///
    /// The channel is part of the key. Two surfaces on two channels
    /// sending the same pair of controller numbers are two faders, and a map
    /// line that named no channel maps both of them; holding one MSB for the
    /// two would refine each with the other's top bits.
    fn paired(&mut self, message: Message) -> Paired {
        let Some(wide) = self.map.wide(message) else {
            return Paired::Seven;
        };
        let Message::ControlChange { channel, value, .. } = message else {
            return Paired::Seven;
        };
        let key = (channel, wide.control);
        match wide.half {
            Half::Msb => {
                match self.halves.iter_mut().find(|(seen, _)| *seen == key) {
                    Some((_, held)) => *held = value,
                    None => self.halves.push((key, value)),
                }
                Paired::Seven
            }
            Half::Lsb => match self.halves.iter().find(|(seen, _)| *seen == key) {
                Some((_, msb)) => Paired::Wide((u16::from(*msb) << 7) | u16::from(value)),
                None => Paired::Lone,
            },
        }
    }

    /// What the surface should be shown, as wire messages, for every mapped control
    /// the deck has moved since the last call.
    ///
    /// `out` is cleared first and holds complete messages: one per 7-bit control
    /// and per pad, two for a `cc14` pair. The caller sends them, which is this
    /// module's own split — a `Router` has no port, so what can be wrong here is
    /// what is *shown* rather than what a device did with it, and a test needs no
    /// device to check it.
    ///
    /// Only what moved. A control whose position on its own fader has not changed
    /// says nothing, so a still deck writes nothing at all and a transition moving
    /// one mask front writes one message a frame. That is the difference between
    /// this and re-stating the whole map every frame, which would fill a surface's
    /// queue with the answer it already had.
    ///
    /// A control this program cannot read is skipped and not zeroed.
    /// `Feedback::shown` answers `None` for `tap`, for a slot past the end of the
    /// deck and for a `param` line reaching past a Set's interface, and darkening a
    /// pad because a value could not be read would be the surface asserting
    /// something about the deck.
    ///
    /// Nothing is recorded. No `Operation` is produced and no `Record` is written:
    /// a session recorded from a controller replays with neither controller nor map
    /// attached (P-0092), and MIDI out is the wire rather than the stream.
    ///
    /// Nothing here allocates once the run is warm: `echoes` and `lit` are built at
    /// construction, and `out` is the caller's own buffer, cleared rather than
    /// dropped.
    pub fn shown(&mut self, values: &dyn Feedback, out: &mut Vec<[u8; 3]>) {
        out.clear();
        let Router { echoes, lit, .. } = self;
        for (index, echo) in echoes.iter().enumerate() {
            let Some(shown) = values.shown(echo.control()) else {
                continue;
            };
            let position = echo.position(shown);
            if lit[index] == Some(position) {
                continue;
            }
            lit[index] = Some(position);
            echo.wire(position, out);
        }
    }

    /// The map read the other way, rebuilt — for a learn, which is the one thing
    /// that changes a map while a run is going. A control that has just been bound
    /// has never been shown, so every echo starts unlit again and the next frame
    /// states the whole surface once.
    fn relit(&mut self) {
        self.echoes = self.map.echoes();
        self.lit = vec![None; self.echoes.len()];
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

    /// The keys' own refusal, from [`crate::no_such_slot`] rather than spelled
    /// again here. It was spelled again here — with an em dash where the keys use a
    /// colon — which made a map pointing at slot 9 and a digit key pressed at slot
    /// 9 two sentences about one mistake. Said once per slot per run; see
    /// `seen_no_slot`.
    fn report_no_slot(&mut self, slot: usize, slot_count: usize) {
        if !self.seen_no_slot.insert(slot) {
            return;
        }
        self.notices.push(crate::no_such_slot(slot, slot_count));
    }

    /// A `param` line pointing past the end of a deck's interface, said once per
    /// position per run — see [`Router::resolved`], where the two refusals are told
    /// apart.
    fn report_no_control(&mut self, deck: u8, position: u16) {
        if !self.seen_no_control.insert((deck, position)) {
            return;
        }
        self.notices.push(format!(
            "`param {deck} {position}` reaches nothing — the Set in that deck published fewer \
             controls than that. the number is the one the Inspector draws beside the row"
        ));
    }
}

/// The deck an operation names, if it names one.
///
/// The seven arms are every operation a map line can produce that names a deck,
/// and `karakuri_midi::map`'s `parse_target` is that list — `cc -> exposure`
/// and `note -> tap` name no deck, and the other forty-one operations have no
/// spelling in the grammar at all. The wildcard is what the vocabulary being
/// fifty wide costs here, and it is safe rather than merely convenient: the
/// record path is the backstop. A slot this deck does not hold is refused by
/// `mix::change` with [`crate::no_such_slot`] — this very sentence — and
/// nothing moves, where the old `Action` path indexed a `Vec` directly and
/// panicked on the render thread.
///
/// So what this buys is not safety but silence: the refusal is said once per
/// slot per run rather than once per message, and `cc 1 -> gain 4` on a deck of
/// four is the likeliest typo a map has.
///
/// `WriteParam` is the seventh and it arrives already checked.
/// [`Router::resolved`] refuses a deck this deck does not have before it asks
/// the interface for anything, so this arm never catches one — it is here for
/// the *coalescing* key, which is what `deck_of` is asked for a second time:
/// two knobs on two decks' parameters must not collapse into one operation, and
/// a `None` here would collapse them.
///
/// `SetMaskPosition` is here because for it the backstop is not silent. The
/// other five reach `mix::change` and are refused once; a mask operation is
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
        | Operation::SetMaskPosition { deck, .. }
        | Operation::WriteParam { deck, .. } => Some(usize::from(*deck)),
        _ => None,
    }
}

/// The directory an operator's own maps are filed in, under the store root, and
/// the second tier
/// `docs/adr/0227-a-pattern-and-a-master-chain-setting-are-library-data-in-two-tiers.md`
/// found ragged: *"an operator's own map is whatever path they hand to
/// `--midi-map FILE`. It ships as a preset and it is saved nowhere in
/// particular."* This is that place, on
/// `docs/adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md`'s
/// shape — a name the operator typed, one path component, a place of its own
/// under the store root, and bytes nothing here parses on the way past.
pub const MAPS: &str = "maps";

/// The extension a map file takes, so the three paths built from it cannot
/// drift apart. `.map` and not `.midi-map`: the file is a map of a control
/// surface and `examples/surface.map` has spelled it this way since the grammar
/// existed.
pub const MAP_SUFFIX: &str = "map";

/// What a map is called when nobody has named one. The panel takes no
/// `--midi-map`, so this is the name it looks for in the store — and it is also
/// the name a learned map is written back under, which is the day this constant
/// starts earning its keep.
pub const DEFAULT_MAP: &str = "default";

/// The map that ships, in the app-preset tier beside the material.
/// `examples/surface.map` — the file `karakuri_midi::map`'s own test calls
/// *"what an operator copies before they have one"*, and P-0096's first tier,
/// which nothing in this program writes.
pub const SHIPPED_MAP: &str = "surface.map";

/// Which map a program with no flag for one opens, in the two tiers ADR-0227
/// puts library data in: the operator's own first, what ships second, and
/// `None` for a machine with neither.
///
/// The operator's wins, which is the order every other pair of tiers in this
/// program is read in and is the only order that lets a learned map matter: a
/// map saved under the store is a map somebody made here, and a preset that
/// shadowed it would make learning a gesture with no effect the next time the
/// program started.
///
/// Existence and not readability. A file that is there and will not parse is
/// `karakuri_midi::Map::parse`'s to complain about, line by line, with the rest
/// of the map loading — refusing it here would turn one bad line into no
/// surface at all. A file that is not there is not a fault at either tier:
/// `presets` is `None` on a machine with no library at all
/// (`crate::places::presets`), and a store that has never been learned into has
/// no `maps/`.
pub fn map_for(store: &Path, presets: Option<&Path>) -> Option<std::path::PathBuf> {
    let own = store.join(MAPS).join(format!("{DEFAULT_MAP}.{MAP_SUFFIX}"));
    if own.is_file() {
        return Some(own);
    }
    let shipped = presets?.join(SHIPPED_MAP);
    shipped.is_file().then_some(shipped)
}

/// What a learned map file holds afterwards — the half of [`Surface::learn`]
/// that has no device and no disk in it.
///
/// `held` is what the operator's map file already says, or `None` where there
/// is none yet; `seed` is the map in force, written out, for that case.
///
/// # It replaces the knob's own line and appends everything else
///
/// This was an append and nothing else, and the test said why not. A re-learn
/// of one knob left both lines in the file; `Map::parse`'s *the later line
/// wins* means the map is still right, but it also reports the shadowed line —
/// so a knob learned five times printed four complaints on every start, about a
/// file the operator never wrote by hand. The complaint is correct and the file
/// is what was wrong.
///
/// So a line whose left-hand side is this same message is replaced where it
/// sits, and a knob nothing is mapped to is appended. What that buys beyond
/// silence is that the file does not grow on a gesture an operator will make
/// dozens of times in a session, and that a learned line stays where they last
/// saw it.
///
/// Everything else in the file is bytes. Comments, blank lines, the order of
/// the rest, a line for another knob — none of it is parsed, re-emitted or
/// moved. The shipped map an operator starts from is two-thirds prose
/// explaining what a line means, and rewriting the file from the table would
/// turn the one document that teaches the format into forty bare lines on the
/// first press.
///
/// Split out to be tested, which is the only way this can be: a [`Surface`]
/// cannot be built without a port, and *what a learn does to a file* is the
/// half worth checking on every machine.
fn appended(held: Option<String>, seed: &str, key: &str, line: &str) -> String {
    let mut text = held.unwrap_or_else(|| seed.to_owned());
    let wanted = squashed(key);
    let mut replaced = false;
    let lines: Vec<String> = text
        .lines()
        .map(|raw| {
            // The comment half is the operator's and is never read: a line
            // they commented out is a line they took out.
            let live = raw.split('#').next().unwrap_or("");
            match live.split_once("->") {
                Some((from, _)) if !replaced && squashed(from) == wanted => {
                    replaced = true;
                    line.to_owned()
                }
                _ => raw.to_owned(),
            }
        })
        .collect();
    if replaced {
        let mut out = lines.join("\n");
        out.push('\n');
        return out;
    }
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(line);
    text.push('\n');
    text
}

/// A message's spelling with its spacing taken out, so `cc 30` and `cc 30` are
/// one knob. Not a parse: two spellings of one key that this cannot tell apart
/// leave a duplicate, which `Map::parse` then reports — the outcome this is an
/// improvement on rather than a guarantee against.
fn squashed(spelling: &str) -> String {
    spelling.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// An open surface: a [`Router`] with a port in front of it.
pub struct Surface {
    port: Port,
    /// The same device's output port, where it has one — what makes a surface's
    /// LEDs and motorised faders follow the deck.
    ///
    /// `None` is a surface that only sends: a controller with no output port, one
    /// whose output another program holds, or a device whose two ports are named so
    /// differently that [`paired_out`] could not match them. Every one of those is
    /// a state and not a fault — MIDI in is what a map is for, and the run goes on
    /// exactly as it did before this existed.
    out: Option<Out>,
    /// Whether a dropped message has been said out loud yet. A full queue is worth
    /// one sentence a run and not one a frame — the same rule the router's own
    /// notices keep, for the same reason.
    said_dropped: bool,
    /// Scratch for [`Router::shown`], owned so the frame path does not allocate, on
    /// [`Surface::inbox`](Surface)'s terms exactly.
    outbox: Vec<[u8; 3]>,
    router: Router,
    /// What the map is called, or `None` for a surface running without one. The
    /// file's stem rather than its path: it is what a readout names — the transport
    /// row's `map · <name>` pill — and a path is not a name (P-0087).
    map_name: Option<String>,
    /// Scratch for [`Port::drain`]. Owned, and given a capacity at construction
    /// rather than grown into one, because this is read on the frame path — see
    /// [`INBOX`].
    inbox: Vec<Message>,
}

/// Messages a frame is sized for.
///
/// A surface's fastest gesture is a fader sweep, which a device sends at a few
/// hundred messages a second — so a 60 Hz frame sees single figures, and a
/// stall of a second's worth of eight faders moving together is still under
/// this. Reserved at construction so the frame path does not `realloc`, which
/// is the same reason `karakuri-environment`'s `audio.rs` sizes its buffers up
/// front rather than letting them find their own high-water mark.
const INBOX: usize = 256;

impl Surface {
    /// Open a port and load a map, or say why not.
    ///
    /// A missing map is not an error: a surface with no map still prints what it
    /// sends, which is exactly the state an operator is in before they have written
    /// one.
    pub fn open(port: &str, map_path: Option<&Path>) -> Result<Surface, String> {
        let (surface, notes) = Surface::assembled(Port::open(port)?, map_path)?;
        for note in &notes {
            eprintln!("  midi map: {note}");
        }
        eprintln!(
            "midi in: `{}`, {} mapping{}{}",
            surface.port_name(),
            surface.mappings(),
            if surface.mappings() == 1 { "" } else { "s" },
            if surface.mappings() == 0 {
                " — turn a knob and this will print the line that would map it"
            } else {
                ""
            }
        );
        // **Said only when there is one.** A surface with no output port is
        // the state every run before MIDI out existed was in, and a line
        // announcing its absence would be a complaint about most controllers.
        if let Some(out) = surface.out_name() {
            eprintln!("midi out: `{out}` — mapped controls follow the deck");
        }
        Ok(surface)
    }

    /// Open the first input there is, and say nothing out loud.
    ///
    /// [`Surface::open`] is for a program told which port to take before the run,
    /// and it prints its own line because a command line has already gone past.
    /// This is for one with somebody standing in front of it: it takes whatever is
    /// plugged in — [`karakuri_midi::Port::open`]'s empty selector — and hands the
    /// caller back the map's parse notes to put in its own legend. It is
    /// `crate::audio`'s `default` one door along, and the argument is the same: a
    /// surface an operator plugged in and a program that waited to be told about it
    /// are not the same instrument.
    ///
    /// `wake` is called once per message on the MIDI thread. A caller whose loop
    /// sleeps has nothing else to tell it a knob moved; see
    /// [`karakuri_midi::Port::waking`], which is where that whole argument is.
    ///
    /// Returns the surface and the map's complaints, in order. Both are meant to be
    /// used — a caller that dropped the second would leave an operator pressing a
    /// pad that never loaded, with nothing said.
    pub fn first(
        map_path: Option<&Path>,
        wake: impl Fn() + Send + 'static,
    ) -> Result<(Surface, Vec<String>), String> {
        Surface::assembled(Port::waking("", wake)?, map_path)
    }

    /// The half that has no device in it: a port, a map file, and the router over
    /// the two. Said once so that the two constructors above cannot come to
    /// disagree about what loading a map means.
    ///
    /// A missing map is not an error: a surface with no map still reports what it
    /// sends, which is exactly the state an operator is in before they have written
    /// one. A map file that is *named* and will not open is, because somebody said
    /// that one.
    fn assembled(port: Port, map_path: Option<&Path>) -> Result<(Surface, Vec<String>), String> {
        let (map, notes) = match map_path {
            Some(path) => {
                let text = std::fs::read_to_string(path)
                    .map_err(|e| format!("reading MIDI map `{}`: {e}", path.display()))?;
                Map::parse(&text)
            }
            None => (Map::default(), Vec::new()),
        };
        Ok((
            Surface {
                out: paired_out(port.name()),
                said_dropped: false,
                outbox: Vec::with_capacity(INBOX),
                port,
                router: Router::new(map),
                map_name: map_path
                    .and_then(Path::file_stem)
                    .map(|stem| stem.to_string_lossy().into_owned()),
                inbox: Vec::with_capacity(INBOX),
            },
            notes,
        ))
    }

    /// The port that was opened, as the device named it.
    pub fn port_name(&self) -> &str {
        self.port.name()
    }

    /// What the map in use is called, or `None` for a surface running without one —
    /// see [`Surface::map_name`](Surface::map_name)'s field.
    pub fn map_name(&self) -> Option<&str> {
        self.map_name.as_deref()
    }

    /// How many mappings loaded — [`Router::mappings`], through the surface that
    /// holds it rather than a second count.
    pub fn mappings(&self) -> usize {
        self.router.mappings()
    }

    /// Everything that arrived since the last frame, as operations this deck can
    /// answer.
    pub fn take(&mut self, slot_count: usize, interface: &dyn Interface, out: &mut Vec<Operation>) {
        self.port.drain(&mut self.inbox);
        // Split rather than borrowed together: `route` writes to the router and
        // reads the inbox, and both are fields of `self`. `mem::take` would
        // hand the allocation back only if nothing panicked in between.
        let Surface { router, inbox, .. } = self;
        router.route(inbox, slot_count, interface, out);
        for notice in router.notices() {
            eprintln!("  midi: {notice}");
        }
    }

    /// The first hand on a control this frame, for a caller that is learning rather
    /// than playing — or `None` where nothing arrived.
    ///
    /// Learn needs the message and not what it is worth: nothing is mapped to the
    /// knob yet, so [`Surface::take`] would report it as a discovery and hand back
    /// no operation. This is the same drain, one step earlier and narrowed to the
    /// one message a learn is about.
    ///
    /// A release is not a hand on a control, which is this crate's rule everywhere:
    /// every pad acts on the press, so learning from a `NoteOff` would bind the
    /// knob on the way back up.
    ///
    /// The first and not the last, which is the opposite of what a *fader* wants
    /// and is right here: a sweep is one gesture, and the value is thrown away
    /// anyway — what a learn takes from the message is which knob it was. Taking
    /// the last would make a learn depend on where the hand stopped.
    ///
    /// It drains, so a frame spent learning is a frame nothing is played from — see
    /// `App::mapped`, where that is the point rather than a side effect.
    pub fn learning(&mut self) -> Option<Message> {
        self.port.drain(&mut self.inbox);
        self.inbox
            .iter()
            .copied()
            .find(|message| !matches!(message, Message::NoteOff { .. }))
    }

    /// Bind `message` to `target` and put the line in the operator's own map,
    /// giving back what to say about it.
    ///
    /// `to` is where the operator's map lives — `<store>/maps/default.map`,
    /// [`map_for`]'s first tier — and it is always that file whatever map this run
    /// loaded. A run playing the shipped `examples/surface.map` and learning a
    /// control writes the line into the store, which is
    /// [P-0096](../../../docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md)
    /// held rather than argued: nothing in this program writes the preset tier, and
    /// the first learn is what promotes a map into the operator's.
    ///
    /// # What it does to the file is [`appended`], and that is the decision
    ///
    /// A line for this same knob is replaced where it sits; a knob nothing is
    /// mapped to is appended; everything else in the file is bytes. The alternative
    /// — rewriting the file from the table, which is what a map editor would do —
    /// loses every comment in it, and the shipped map an operator starts from is
    /// two-thirds prose explaining what a line means.
    ///
    /// The file is created with what is loaded where it does not exist yet, so a
    /// learn against the shipped map does not leave the store holding one line and
    /// the rest silently lost on the next start.
    pub fn learn(&mut self, message: Message, target: &str, to: &Path) -> Result<String, String> {
        let line = self.router.map.learn(message, target)?;
        if let Some(dir) = to.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("making `{}`: {e}", dir.display()))?;
        }
        // **Seeded from what is loaded, once.** Reaching here with no file is
        // either the first learn of the run or a store somebody has just
        // emptied; either way the lines in force are the ones this surface is
        // playing, and a file holding only the newest of them would be a map
        // that shrank on a press.
        let held = std::fs::read_to_string(to).ok();
        let key = line.split_once("->").map(|(from, _)| from).unwrap_or("");
        let text = appended(held, &self.seed(), key, &line);
        std::fs::write(to, text).map_err(|e| format!("writing `{}`: {e}", to.display()))?;
        self.map_name = to
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned());
        // **The map changed, so the map read the other way changed too.** A
        // knob just bound has never been shown, and the control it took over
        // may have been shown on another knob a moment ago.
        self.router.relit();
        Ok(line)
    }

    /// The map in force, as lines, for seeding an operator's map file that does not
    /// exist yet.
    ///
    /// Sorted, because a `HashMap`'s order is not an order and a file that came out
    /// shuffled on every run is a file nobody can diff.
    fn seed(&self) -> String {
        let mut lines: Vec<String> = self.router.map.lines().collect();
        lines.sort();
        let mut text = String::from(
            "# Written by `karakuri` on the first learn of a run, from the map it was \
             playing.\n# Every line below is one this program loaded; edit it by hand as \
             freely as any other.\n\n",
        );
        for line in lines {
            text.push_str(&line);
            text.push('\n');
        }
        text
    }

    /// Which message reaches `target`, or `None` for a control nothing is mapped to
    /// — `karakuri_midi::Map::bound` through the surface that holds the map, so a
    /// caller drawing a tooltip needs no second handle on it.
    pub fn bound(&self, target: &str) -> Option<String> {
        self.router.map.bound(target)
    }

    /// The output port that was opened, or `None` for a surface that only sends —
    /// for the line a program says in its legend.
    pub fn out_name(&self) -> Option<&str> {
        self.out.as_ref().map(Out::name)
    }

    /// Show the surface where the deck is — [`Router::shown`] with the port in
    /// front of it, called once a frame beside the drain.
    ///
    /// Every mapped control the deck has moved since the last call is written to
    /// the wire: a `cc` per continuous control at its 7-bit or 14-bit position, and
    /// a note per pad whose state changed. Nothing is sent for a control nothing is
    /// mapped to, because the map is the list, and nothing is written into the
    /// session stream at all.
    ///
    /// It does not wait. `Out::send` is a bounded queue to a thread that owns the
    /// connection, and a frame that finds it full drops rather than blocking
    /// (P-0094, ADR-0067's shape). The drop is counted and said once a run, because
    /// a surface that quietly stopped following the deck is the failure nobody
    /// notices.
    ///
    /// A surface with no output port does nothing here, and costs one branch.
    pub fn show(&mut self, values: &dyn Feedback) {
        let Some(out) = self.out.as_ref() else {
            return;
        };
        let Surface { router, outbox, .. } = self;
        router.shown(values, outbox);
        for message in outbox.iter() {
            out.send(*message);
        }
        if !self.said_dropped && self.out.as_ref().is_some_and(|out| out.dropped() > 0) {
            self.said_dropped = true;
            eprintln!(
                "  midi out: the surface is not keeping up and messages are being dropped — a \
                 control may show where the deck was rather than where it is"
            );
        }
    }
}

/// The output port that goes with an input, or `None`.
///
/// A device's two ports carry the manufacturer's name and often differ past it
/// — "nanoKONTROL2 SLIDER/KNOB" in and "nanoKONTROL2 CTRL" out — so the whole
/// name is tried first and the first word after it. The first word is the
/// device, which is what makes this a pairing rather than a guess: two
/// controllers plugged in at once have two different first words, and a device
/// whose output is named nothing like its input is answered `None` rather than
/// paired with somebody else's port.
///
/// It is deliberately silent. A surface with no output is the state every run
/// before this existed was in, and a program that printed a complaint for it
/// would be complaining about most controllers.
fn paired_out(input: &str) -> Option<Out> {
    let first = input.split_whitespace().next().unwrap_or(input);
    Out::open(input).or_else(|_| Out::open(first)).ok()
}

#[cfg(test)]
mod tests;
