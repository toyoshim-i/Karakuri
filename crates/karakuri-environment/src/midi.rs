//! MIDI surface input and output routing.
//!
//! Maps hardware MIDI messages to [`karakuri_operation::Operation`] commands and
//! converts deck state back into feedback messages for LED/fader controllers.
//!
//! - Continuous controls coalesce per frame (ADR-0207).
//! - 14-bit CC pairs combine MSB and LSB values.
//! - Learn mode edits or appends to the active map file (ADR-0336).
//! - Feedback updates send non-blocking delta changes on frame commit (Principle 0092, Principle 0094).

use std::collections::HashSet;
use std::path::Path;

use karakuri_midi::{Control, Echo, Half, Map, Shown};

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

/// Published interface query for resolving 1-indexed MIDI parameter positions
/// to deck nodes and value ranges (ADR-0268).
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
    /// Notices generated during routing in the current frame, deduplicated per control.
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
    /// Output index of coalesced continuous control operations in the current frame.
    coalescing: Vec<(Continuous, usize)>,
    /// Cached MSB values for 14-bit CC pairs, keyed by (channel, controller).
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

    /// Resolves value resolution for a message, tracking MSB state for 14-bit CC pairs.
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

/// Returns the deck index an operation targets, or `None` if it targets no deck.
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

/// Updates map text with a newly learned binding, replacing existing mapping in-place
/// or appending to the end if not previously present.
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

mod surface;

pub use surface::Surface;

#[cfg(test)]
mod tests;
