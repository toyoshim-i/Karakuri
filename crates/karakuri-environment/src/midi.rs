//! MIDI surface input and output routing.
//!
//! Maps hardware MIDI messages to [`karakuri_operation::Operation`] commands and
//! converts deck state back into feedback messages for LED/fader controllers (ADR-0207, ADR-0336).

use std::collections::HashSet;
use std::path::Path;

use karakuri_midi::{Control, Echo, Half, Map, Shown};

/// Re-exported MIDI wire message representation.
pub use karakuri_midi::Message;
use karakuri_operation::{Operation, ParamAt, ParamValue};

/// Published interface query for resolving 1-indexed MIDI parameter positions
/// to deck nodes and value ranges (ADR-0268).
pub trait Interface {
    /// Resolves the control at 1-indexed `position` of the deck in `slot`.
    fn control_at(&self, slot: u8, position: u16) -> Option<(ParamAt, [f32; 2])>;
}

/// [`Interface`] implementation querying a live engine deck.
pub struct Decks<'a>(pub &'a karakuri_engine::deck::Deck);

impl Interface for Decks<'_> {
    fn control_at(&self, slot: u8, position: u16) -> Option<(ParamAt, [f32; 2])> {
        let set = self.0.slot(karakuri_engine::DeckSlot(slot)).set();
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

/// Queries current values of mapped controls for MIDI feedback output.
pub trait Feedback {
    /// Where `control` is, or `None` for one there is nothing to show.
    fn shown(&self, control: Control) -> Option<Shown>;
}

/// [`Feedback`] implementation querying deck parameters and global exposure.
pub struct Lit<'a> {
    pub deck: &'a karakuri_engine::deck::Deck,
    pub exposure: f32,
}

impl Feedback for Lit<'_> {
    fn shown(&self, control: Control) -> Option<Shown> {
        let slot = |deck: u8| karakuri_engine::DeckSlot::new(deck, self.deck.slot_count());
        Some(match control {
            Control::Gain { deck } => Shown::At(self.deck.gain(slot(deck)?)),
            Control::Opacity { deck } => Shown::At(self.deck.opacity(slot(deck)?)),
            Control::Exposure => Shown::At(self.exposure),
            Control::MaskPosition { deck } => Shown::At(self.deck.mask(slot(deck)?).position()),
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
    /// Set of `(deck, position)` pairs reported as missing from published interfaces.
    seen_no_control: HashSet<(u8, u16)>,
    /// Output index of coalesced continuous control operations in the current frame.
    coalescing: Vec<(Continuous, usize)>,
    /// Cached MSB values for 14-bit CC pairs, keyed by (channel, controller).
    halves: Vec<((u8, u8), u8)>,
    /// Every mapped control as an echo definition, rebuilt when learning a new map.
    echoes: Vec<Echo>,
    /// Last shown position per `echoes` entry, or `None` if never displayed.
    lit: Vec<Option<u16>>,
}

/// Key identifying a continuous control target for intra-frame coalescing:
/// operation discriminant, optional deck index, and optional parameter index.
type Continuous = (
    std::mem::Discriminant<Operation>,
    Option<usize>,
    Option<u16>,
);

/// Resolution format when evaluating a MIDI message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Paired {
    /// 7-bit resolution (standard CC, Note, or isolated MSB).
    Seven,
    /// 14-bit resolution combining MSB and LSB values.
    Wide(u16),
    /// Unpaired LSB without preceding MSB.
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

    /// Number of loaded MIDI mappings.
    pub fn mappings(&self) -> usize {
        self.map.len()
    }

    /// Routes incoming `messages` into `out` operations, coalescing continuous controls per frame (ADR-0207).
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
            let asked = match self.paired(*message) {
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
                    if !matches!(message, Message::NoteOff { .. }) {
                        self.report_unmapped(*message);
                    }
                    continue;
                }
            };
            let Some(operation) = operation else {
                continue;
            };
            match deck_of(&operation) {
                Some(slot) if slot >= slot_count => self.report_no_slot(slot, slot_count),
                _ => self.emit(*message, operation, position, out),
            }
        }
    }

    /// Resolves a published deck parameter position to an operation, reporting missing targets once (Principle 0094).
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

    /// Emits one operation per continuous control per frame, coalescing earlier values (ADR-0207).
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

    /// Translates deck changes into MIDI wire feedback messages for external surface hardware (Principle 0092).
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

    /// Reports an out-of-range slot reference once per session.
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

/// Directory under the store root where user MIDI map files are stored (ADR-0221, ADR-0227).
pub const MAPS: &str = "maps";

/// Extension used for MIDI surface map files.
pub const MAP_SUFFIX: &str = "map";

/// Default map file name when no explicit map is supplied.
pub const DEFAULT_MAP: &str = "default";

/// Pre-packaged default MIDI surface map shipped with the application.
pub const SHIPPED_MAP: &str = "surface.map";

/// Resolves the active MIDI map path: operator map in store first, shipped preset second (ADR-0227).
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
