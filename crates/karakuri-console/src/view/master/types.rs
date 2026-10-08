//! Master bay data types and geometric structures (ADR-0224, ADR-0340).

use egui::{Pos2, Rect};
use karakuri_layout::Point;
use karakuri_operation::Operation;

use crate::panel::{Grab, Knob};
use crate::room::size;
use crate::view::widgets::fader::grabbed;
use crate::view::Fader;

/// The word at the head of the Master bay, in the source's own capitalisation
/// for [`crate::view::Kind::Bay`]'s reason: the mock upper-cases in CSS, and that is done at
/// paint time so the word a reader searches for is the word in the source.
pub(crate) const MASTER_TITLE: &str = "Master";

/// `.master-row`'s first item: the `out` before the track, which is the bay's
/// own word for the level and not the engine's — `Deck::out` is what it moves.
pub(super) const MASTER_LABEL: &str = "out";

/// The word on the control at the end of the chain's list.
pub(super) const ADD_LABEL: &str = "+ add";

/// The glyph at the end of a slot's row, which takes that slot out of the
/// chain — `docs/manual/console.html`'s own mark for it.
pub(crate) const REMOVE_GLYPH: &str = "\u{2212}";

/// Layout of the Master bay's body: out row, chain slot wells, and `+ add`.
/// Shared by rendering and hit-testing to prevent drift.
#[derive(Debug, Clone, PartialEq)]
pub struct MasterRow {
    /// The faint `out` at the head of the row.
    pub label: Rect,
    /// The fader: `.fader`'s 5px well lying down, what the level fills of it, and
    /// the knob centred on the fill's moving edge.
    pub fader: Fader,
    /// Fixed-width rectangle for the level figure, sized to the widest possible reading.
    pub value: Rect,
    /// The value these rectangles were measured from, carried for
    /// [`LookRow::values`]' reason: whoever measured the type and whoever paints it
    /// are one statement.
    pub out: f32,
    /// Visible chain slot wells in order (ADR-0340). Rows drop from bottom up if space is short.
    pub slots: Vec<SlotRow>,
    /// Built-in VR projection stage, present when in VR mode.
    pub vr_stage: Option<VrProjectionRow>,
    /// `+ add` at the end of the list, or `None` where the bay has no room for
    /// it. A press puts [`AddCard`] down and asks for nothing on its own; the
    /// procedure is named by picking an item of that card.
    pub add: Option<Rect>,
    /// The chooser's card, or `None` while it is up.
    pub card: Option<AddCard>,
    /// Landing rectangle for drag-and-drop carry release over the slot list (ADR-0273).
    pub list: Option<Rect>,
}

/// Layout of a chain slot: well, title bar (head), status dot, procedure name, solo, mute, cut chip, remove button, and param rows.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotRow {
    /// Where this slot sits in the chain, counted from the mix's output, which is
    /// the address every chain operation takes.
    pub at: u32,
    /// The well the slot is drawn in.
    pub well: Rect,
    /// Title bar rectangle across the top of the well.
    pub head: Rect,
    /// `.fx .dot`.
    pub dot: Rect,
    /// The procedure's name.
    pub name: Rect,
    /// The word in it, measured once and painted from the same string.
    pub words: String,
    /// Solo toggle button rectangle.
    pub solo: Rect,
    /// Whether this slot is soloed.
    pub is_soloed: bool,
    /// Mute toggle button rectangle.
    pub mute: Rect,
    /// Whether this slot is muted.
    pub is_muted: bool,
    /// Whether this slot is online/active according to mixer-style solo/mute arbitration.
    pub is_online: bool,
    /// Whether this slot is folded/collapsed.
    pub is_folded: bool,
    /// Cut chip rectangle for procedures declaring `retains`, or `None`.
    pub cut: Option<Rect>,
    /// Which cut the slot reads, `None` where it reads none. The chip is laid
    /// out from it and the operation a press asks for carries it.
    pub reading: Option<karakuri_operation::Cut>,
    /// The `−` at the end of the row, which takes this slot out of the chain.
    pub remove: Rect,
    /// One row per parameter the procedure declares, in declaration order.
    pub params: Vec<ParamRow>,
}

/// Layout of a slot parameter row: ordinal index, label key, slider track, and value figure.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamRow {
    /// Which slot this row moves.
    pub at: u32,
    /// 1-based parameter ordinal index (1, 2, 3...) matching Inspector.
    pub ord: usize,
    /// The key the procedure declares, which is what the operation carries.
    pub key: String,
    /// The range the procedure declares it over. The track's position is a
    /// fraction of it and the operation carries the value.
    pub range: [f32; 2],
    /// What the slot holds it at, in the procedure's own units.
    pub value: f32,
    /// Ordinal index cell rectangle.
    pub ord_rect: Rect,
    /// The key, drawn before the track.
    pub label: Rect,
    /// The track.
    pub fader: Fader,
    /// The figure, in a box as wide as the widest reading — [`MasterRow::value`]'s
    /// rule and its reason.
    pub amount: Rect,
}

impl ParamRow {
    /// Where along its declared range this row's value sits, on `[0, 1]`, which
    /// is what a fader draws. A range of no width is at zero.
    pub fn along(&self) -> f32 {
        let [low, high] = self.range;
        match high > low {
            true => ((self.value - low) / (high - low)).clamp(0.0, 1.0),
            false => 0.0,
        }
    }

    /// What a drag on this row asks for.
    ///
    /// The track's position is a fraction of the declared range, and the amount
    /// the operation carries is what the slot is set to.
    pub fn knob(&self) -> Knob {
        Knob::Chain {
            at: self.at,
            key: self.key.clone(),
            range: self.range,
        }
    }
}

impl SlotRow {
    /// Operation requested by pressing the cut chip (P-0090), or `None` if outside.
    pub fn chip(&self, p: Point) -> Option<Operation> {
        let chip = self.cut?;
        if !chip.contains(Pos2::new(p.x, p.y)) {
            return None;
        }
        Some(Operation::SetChainParam {
            at: self.at,
            param: karakuri_operation::ChainParam::Cut(self.next_cut()),
        })
    }

    /// The cut after the one this slot reads, wrapping. It is the chip's whole
    /// arithmetic, and a key and a press cycle the same list.
    pub fn next_cut(&self) -> karakuri_operation::Cut {
        let all = karakuri_operation::Cut::ALL;
        let showing = self
            .reading
            .and_then(|cut| all.iter().position(|c| *c == cut))
            .unwrap_or(0);
        all[(showing + 1) % all.len()]
    }

    /// What a press on this slot's `−` asks for, or `None` where `p` is not on
    /// it.
    pub fn minus(&self, p: Point) -> Option<Operation> {
        self.remove
            .contains(Pos2::new(p.x, p.y))
            .then_some(Operation::RemoveChainEffect { at: self.at })
    }

    /// Hit-tests the Solo button on this slot's header.
    pub fn solo(&self, p: Point) -> Option<u32> {
        self.solo.contains(Pos2::new(p.x, p.y)).then_some(self.at)
    }

    /// Hit-tests the Mute button on this slot's header.
    pub fn mute(&self, p: Point) -> Option<u32> {
        self.mute.contains(Pos2::new(p.x, p.y)).then_some(self.at)
    }

    /// Hit-tests the title bar for folding, returning `Some(at)` when clicked outside
    /// the interactive controls (Solo, Mute, Cut chip, Remove).
    pub fn fold(&self, p: Point) -> Option<u32> {
        let at = Pos2::new(p.x, p.y);
        if !self.head.contains(at) {
            return None;
        }
        if self.solo.contains(at)
            || self.mute.contains(at)
            || self.remove.contains(at)
            || self.cut.is_some_and(|c| c.contains(at))
        {
            return None;
        }
        Some(self.at)
    }
}

/// Master chain state snapshot mirrored for console display (ADR-0156).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Chain {
    /// The slots, in the chain's own order. Empty is the default chain.
    pub slots: Vec<ChainSlot>,
}

/// One slot of the chain, as the bay reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct ChainSlot {
    /// What the row names the procedure: the name the library gives that address,
    /// or the short address where nothing names it.
    pub name: String,
    /// Which cut the slot reads, and `None` where its procedure declares no
    /// `retains` — which is whether the row draws a chip at all.
    pub cut: Option<karakuri_operation::Cut>,
    /// The parameters the procedure declares, in declaration order.
    pub params: Vec<SlotParam>,
}

/// One declared parameter of a slot, as the bay reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotParam {
    /// The name the procedure declares for it, which is what the operation
    /// carries.
    pub key: String,
    /// The range the procedure declares it over, low then high. The fader is laid
    /// out from it.
    pub range: [f32; 2],
    /// What the slot holds it at, in the procedure's own units.
    pub value: f32,
    /// What the procedure declares it at, which is where `space` on this row
    /// returns it to.
    pub default: f32,
}

/// One item of the `+ add` chooser: a `kind L5` procedure the library holds.
#[derive(Debug, Clone, PartialEq)]
pub struct AddChoice {
    /// The content address of the procedure's source, which is what
    /// [`Operation::AddChainEffect`] carries.
    pub procedure: String,
    /// The words on the item, which is the name the library lists it under.
    pub words: String,
    /// Whether the procedure declares `retains`, which is whether a slot of it
    /// takes a cut.
    pub retains: bool,
}

impl AddChoice {
    /// Builds [`Operation::AddChainEffect`] with default cut if `retains` is declared (ADR-0348).
    pub fn operation(&self) -> Operation {
        Operation::AddChainEffect {
            procedure: self.procedure.clone(),
            cut: self.retains.then(karakuri_operation::Cut::default),
        }
    }
}

/// What the `+ add` chooser offers this frame, read off the [`View`] once and
/// handed in — [`Choices`]' shape one bay along. The item that is painted and
/// the item a press lands on are one derivation.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AddChoices {
    /// Every procedure on offer, in the order the library lists them.
    pub items: Vec<AddChoice>,
    /// Whether the card is down — the console's own state, like [`Target::open`].
    pub open: bool,
}

impl AddChoices {
    /// Nothing to add, which is a console whose library is listing no `kind L5`
    /// procedure — every test in this crate that does not hand one in.
    pub fn none() -> AddChoices {
        AddChoices::default()
    }
}

/// The `+ add` chooser's card, laid out — [`LaneCard`]'s shape one bay along and
/// the same mechanism: a card of items hanging off the control that opened it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AddCard {
    /// The card itself.
    pub card: Rect,
    /// How many items it draws — [`AddChoices::items`]' length.
    pub items: usize,
}

impl AddCard {
    /// Where one item is, from the top of the card — [`LaneCard::item`]'s own
    /// reading, with no rule in it: one kind of thing is on this card.
    ///
    /// Panics on an item this card has not got.
    pub fn item(&self, index: usize) -> Rect {
        assert!(
            index < self.items,
            "item {index} of a card of {}",
            self.items
        );
        Rect::from_min_size(
            Pos2::new(
                self.card.min.x + size::LIB_LIST_PAD,
                self.card.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * index as f32,
            ),
            egui::vec2(
                self.card.width() - size::LIB_LIST_PAD * 2.0,
                size::LIB_ROW_H,
            ),
        )
    }

    /// Which item `p` is on, or `None` for a point on the card's padding or off
    /// the card altogether — [`LaneCard::picked`]'s own answer.
    pub fn picked(&self, p: Point) -> Option<usize> {
        let at = Pos2::new(p.x, p.y);
        (0..self.items).find(|index| self.item(*index).contains(at))
    }
}

/// What a press on the `+ add` control or on its card asks for.
///
/// [`Chose`]'s shape one bay along, and the same division: every arm is either
/// the console's own state or an operation, and nothing here is both.
#[derive(Debug, Clone, PartialEq)]
pub enum Added {
    /// The pill was pressed with the card up: put it down.
    Open,
    /// A press that dismisses the card and asks for nothing.
    Shut,
    /// An item was picked, and this is what it asks for.
    Add(Operation),
}

/// Built-in VR projection parameter identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VrParamKey {
    Rings,
    Facets,
    Spin,
    Mirror,
    Zoom,
}

impl VrParamKey {
    pub const ALL: [Self; 5] = [
        Self::Rings,
        Self::Facets,
        Self::Spin,
        Self::Mirror,
        Self::Zoom,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Rings => "rings",
            Self::Facets => "facets",
            Self::Spin => "spin",
            Self::Mirror => "mirror",
            Self::Zoom => "zoom",
        }
    }

    pub const fn range(self) -> [f32; 2] {
        match self {
            Self::Rings => [1.0, 16.0],
            Self::Facets => [0.0, 16.0],
            Self::Spin => [-2.0, 2.0],
            Self::Mirror => [0.0, 1.0],
            Self::Zoom => [0.2, 4.0],
        }
    }

    pub fn value(self, proj: &crate::view::VrProjection) -> f32 {
        match self {
            Self::Rings => proj.rings,
            Self::Facets => proj.facets,
            Self::Spin => proj.spin,
            Self::Mirror => proj.mirror,
            Self::Zoom => proj.zoom,
        }
    }
}

/// User actions on the Built-in VR Projection stage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VrAsk {
    /// Cycles VR projection mode (Wall -> Dome -> Kaleidosky -> Wall).
    CycleMode,
    /// Toggles fold state of the VR projection stage.
    ToggleFold,
    /// Sets a VR projection parameter value directly from track click or drag.
    SetParam { key: VrParamKey, value: f32 },
}

/// Layout of the Built-in VR Projection stage well in the Master bay.
#[derive(Debug, Clone, PartialEq)]
pub struct VrProjectionRow {
    pub well: Rect,
    pub head: Rect,
    pub dot: Rect,
    pub name: Rect,
    pub badge: Rect,
    pub mode_pill: Rect,
    pub mode: crate::view::VrProjectionMode,
    pub is_folded: bool,
    pub params: Vec<VrParamRow>,
}

impl VrProjectionRow {
    /// Hit-tests title bar, mode pill, and parameter tracks for VR stage interactions.
    pub fn ask(&self, p: Point) -> Option<VrAsk> {
        let at = Pos2::new(p.x, p.y);
        if self.mode_pill.contains(at) {
            return Some(VrAsk::CycleMode);
        }
        if self.head.contains(at) {
            return Some(VrAsk::ToggleFold);
        }
        if !self.is_folded {
            for param in &self.params {
                if param.fader.track.contains(at)
                    || param.amount.contains(at)
                    || param.label.contains(at)
                {
                    let along = ((at.x - param.fader.track.min.x) / param.fader.track.width())
                        .clamp(0.0, 1.0);
                    let [low, high] = param.range;
                    let value = low + (high - low) * along;
                    return Some(VrAsk::SetParam {
                        key: param.key,
                        value,
                    });
                }
            }
        }
        None
    }
}

/// Layout of a single VR projection parameter row.
#[derive(Debug, Clone, PartialEq)]
pub struct VrParamRow {
    pub key: VrParamKey,
    pub ord: usize,
    pub ord_rect: Rect,
    pub label: Rect,
    pub fader: Fader,
    pub amount: Rect,
    pub value: f32,
    pub range: [f32; 2],
}

impl VrParamRow {
    pub fn along(&self) -> f32 {
        let [low, high] = self.range;
        if (high - low).abs() < f32::EPSILON {
            0.0
        } else {
            ((self.value - low) / (high - low)).clamp(0.0, 1.0)
        }
    }
}

impl MasterRow {
    /// Resolves which knob is grabbed at point `p`. The track itself is not a target.
    pub fn grab(&self, p: Point) -> Option<Grab> {
        let at = Pos2::new(p.x, p.y);
        grabbed(self.fader, Knob::Out, at).or_else(|| {
            self.slots
                .iter()
                .flat_map(|slot| slot.params.iter())
                .find_map(|row| grabbed(row.fader, row.knob(), at))
        })
    }

    /// What a press on a slot's cut chip or on its `−` asks for, or `None` —
    /// see [`SlotRow::chip`] and [`SlotRow::minus`].
    pub fn chip(&self, p: Point) -> Option<Operation> {
        self.slots
            .iter()
            .find_map(|slot| slot.chip(p).or_else(|| slot.minus(p)))
    }

    /// Resolves which slot's Solo button was pressed, if any.
    pub fn solo(&self, p: Point) -> Option<u32> {
        self.slots.iter().find_map(|slot| slot.solo(p))
    }

    /// Resolves which slot's Mute button was pressed, if any.
    pub fn mute(&self, p: Point) -> Option<u32> {
        self.slots.iter().find_map(|slot| slot.mute(p))
    }

    /// Resolves which slot's title bar was clicked to toggle fold state, if any.
    pub fn fold(&self, p: Point) -> Option<u32> {
        self.slots.iter().find_map(|slot| slot.fold(p))
    }

    /// Resolves user interactions with the Built-in VR projection stage.
    pub fn vr_ask(&self, p: Point) -> Option<VrAsk> {
        self.vr_stage.as_ref().and_then(|vr| vr.ask(p))
    }

    /// Resolves `+ add` click or card pick/dismissal at point `p` (Rule 2).
    pub fn chose(&self, p: Point, choices: &AddChoices) -> Option<Added> {
        if let Some(card) = self.card {
            return Some(match card.picked(p) {
                Some(item) => match choices.items.get(item) {
                    Some(choice) => Added::Add(choice.operation()),
                    None => Added::Shut,
                },
                None => Added::Shut,
            });
        }
        let add = self.add?;
        add.contains(Pos2::new(p.x, p.y)).then_some(Added::Open)
    }

    /// Returns the landing rectangle if `p` is over the chain's slot list (ADR-0273).
    pub fn dropped(&self, p: Point) -> Option<Rect> {
        self.list.filter(|list| list.contains(Pos2::new(p.x, p.y)))
    }

    /// Whether `p` is on one of the things here a hand can move, which is what
    /// [`crate::input::claim`] asks — the knobs, the cut chips, the `−` glyphs,
    /// `+ add` and whatever the card is drawing.
    pub fn owns(&self, p: Point) -> bool {
        self.grab(p).is_some()
            || self.chip(p).is_some()
            || self.solo(p).is_some()
            || self.mute(p).is_some()
            || self.fold(p).is_some()
            || self.vr_stage.as_ref().is_some_and(|vr| vr.well.contains(Pos2::new(p.x, p.y)))
            || self
                .add
                .is_some_and(|add| add.contains(Pos2::new(p.x, p.y)))
            // The card's own rectangle, and not every press while it is down:
            // `crate::input::claim`'s rule 2 already claims the console for a
            // card that is down.
            || self
                .card
                .is_some_and(|card| card.card.contains(Pos2::new(p.x, p.y)))
    }
}
