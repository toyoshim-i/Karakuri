//! Metric / cost accounting and Readout HUD / pointer event translation logic.

pub(crate) use karakuri_console::input::{claim, wheeled, Claim, Turned};
pub(crate) use karakuri_console::panel::{
    Dragged, InHand, Knob, Landing, Op, Outcome, Panel, Pressed, Released,
};
pub(crate) use karakuri_console::room::Room;
#[cfg(test)]
pub(crate) use karakuri_console::view::outputs;
pub(crate) use karakuri_console::view::{
    self, arrangement as arrangement_pill, audio_in as audio_in_pill, bay_grip,
    deck_head as deck_head_row, deck_name, inspector as inspector_pane, keep_pill,
    library as library_bay, look as look_row, master as master_row, mcp_pill, mixer as mixer_bay,
    outputs_with_plugin_name, program_bay, program_head, sequencer as sequencer_bay, slot_mcp_pill,
    staging as staging_bay, tracker_group, transition as transition_row,
    transport as transport_row, Aim, Ask, AudioAsk, AudioIn, Chose, Chosen, Go, McpPill, Picked,
    Read, Scope, Taken, View, Wiring, DECKS, DECK_LETTERS, REGIONS,
};
#[cfg(test)]
pub(crate) use karakuri_engine::governor::Reason;
pub(crate) use karakuri_engine::governor::Report;
use karakuri_environment::{Opening, SlotPolicies};
use karakuri_layout::{NodeId, Point};
use karakuri_operation::Operation;

use crate::demonstration_banks;

mod costs;
mod dispatch;
mod hud;

pub(crate) use costs::*;
pub(crate) use dispatch::*;
pub(crate) use hud::*;

// ---------------------------------------------------------------------------
// The readout: English for what the model returned. No window, no device.
// ---------------------------------------------------------------------------

/// The panel and the view, plus the words for what just happened.
pub(crate) struct Readout {
    pub(crate) panel: Panel,
    pub(crate) view: View,
    /// Model opening handle shared with `karakuri_mcp::serve`.
    ///
    /// Live handle read per frame by `View::opening` and shared directly with the MCP
    /// server so UI pills and audits observe consistent opening state.
    pub(crate) opening: Opening,
    /// What MCP access policy is configured per slot, shared with `karakuri_mcp::serve`.
    pub(crate) slot_policies: SlotPolicies,
    /// Store root for saving arrangements and persistent state, if opened.
    pub(crate) store_root: Option<std::path::PathBuf>,
    /// Cached build verdict for the transport row's health capsule.
    ///
    /// Stores the last ephemeral `swap::Event` drained from staging, persisting
    /// it across frames until a new build verdict arrives.
    pub(crate) health: Option<view::Stage>,
    /// Session sequencer banks polled and edited by the console window.
    ///
    /// Authored pattern state held outside the engine (ADR-0156, ADR-0222, ADR-0227, ADR-0320).
    /// Passed to `View::sequencer` each frame; ephemeral for the duration of the run.
    pub(crate) sequencer: karakuri_pattern::Banks,
    /// Playhead position tracking step boundaries for pattern polling.
    ///
    /// Kept separate from pattern data so runtime playback state is not serialized
    /// with authored pattern content.
    pub(crate) playhead: karakuri_pattern::Playhead,
}

impl Readout {
    pub(crate) fn new(width: f32, height: f32) -> Readout {
        Readout {
            panel: Panel::new(width, height),
            view: View::new(Room::Day),
            // Four classes shut, which is what a run starts with (ADR-0235).
            opening: Opening::closed(),
            slot_policies: SlotPolicies::new(),
            store_root: None,
            // Nothing has been written yet, so the capsule is not drawn.
            health: None,
            sequencer: demonstration_banks(),
            // **Nothing polled yet**, which draws no playhead column and makes
            // the first poll a boundary — a bay nobody has run is not a bay at
            // step zero.
            playhead: karakuri_pattern::Playhead::default(),
        }
    }

    // -- the words ------------------------------------------------------

    pub(crate) fn label(&self, id: NodeId) -> String {
        match self.panel.layout().name(id) {
            Some(name) => name.to_owned(),
            None => "(unnamed split)".to_owned(),
        }
    }

    /// The two regions a boundary is between. A split is often unnamed — the
    /// console's body row is, deliberately — so `split #0` alone does not say which
    /// boundary the pointer has hold of, and the pair does.
    pub(crate) fn pair(&self, split: NodeId, index: usize) -> String {
        match self.panel.pair(split, index) {
            Some((a, b)) => format!("{} | {}", self.label(a), self.label(b)),
            None => "no pair".to_owned(),
        }
    }

    // -- input ----------------------------------------------------------

    pub(crate) fn press(&mut self, p: Point) {
        match self.panel.press(p) {
            Pressed::Grabbed {
                split,
                index,
                axis,
                at,
                offset,
            } => println!(
                "press ({:.0}, {:.0}): the boundary {} — divider #{} of {}, {:?} — is at \
                 {:.1}, grabbed {:+.1} from it",
                p.x,
                p.y,
                self.pair(split, index),
                index,
                self.label(split),
                axis,
                at,
                offset
            ),
            Pressed::NoPair { .. } => {
                println!("press ({:.0}, {:.0}): a divider with no pair", p.x, p.y)
            }
            Pressed::Region { id, rect } => println!(
                "press ({:.0}, {:.0}): region {} at {:.0},{:.0} {:.0}x{:.0}",
                p.x,
                p.y,
                self.label(id),
                rect.x,
                rect.y,
                rect.w,
                rect.h
            ),
            Pressed::Nothing => println!("press ({:.0}, {:.0}): nothing", p.x, p.y),
        }
    }

    /// A move with something in hand. A boundary drag says what it did here; a
    /// fader drag hands its operation back, because acting on one takes the deck
    /// and the deck is not the readout's.
    pub(crate) fn moved(&mut self, p: Point) -> Option<Operation> {
        match self.panel.moved(p)? {
            boundary @ Dragged::Boundary { .. } => {
                println!("{}", self.say_drag(boundary));
                None
            }
            // Pane fold/unfold via boundary drag (ADR-0300). Panel updates its own
            // arrangement; no session record is written.
            Dragged::Pane(op) => {
                let (what, id) = match op {
                    Op::Fold(id) => ("folded — drag its edge back in", id),
                    Op::Unfold(id) => ("back, at its own minimum", id),
                    other => unreachable!("a pane drag asks for a fold, not {other:?}"),
                };
                println!("  drag: {} {what}", self.label(id));
                None
            }
            Dragged::Fader(operation) => Some(operation),
        }
    }

    /// Handles pointer release gesture, returning any resulting action (`Acted`).
    ///
    /// Boundary rests and fader releases complete passively, while set drops
    /// emit their corresponding load operations onto the landed strip/bay.
    pub(crate) fn released(&mut self, onto: Option<Landing>) -> Acted {
        match self.panel.released(onto) {
            Some(Released::Rests { split, index, at }) => {
                println!("release: {} rests at {at:.1}", self.pair(split, index));
                Acted::Nothing
            }
            Some(Released::Gone { split, index }) => {
                println!("release: {} is gone", self.pair(split, index));
                Acted::Nothing
            }
            // Fader rest position belongs to the deck; last drag operation was already logged.
            Some(Released::Let { knob }) => {
                println!(
                    "release: {} lets go of the {}",
                    knob_where(&knob),
                    knob_word(&knob)
                );
                Acted::Nothing
            }
            // Set dropped over a strip; delegates load dispatch to `played`.
            Some(Released::Dropped(operation)) => {
                if let Operation::LoadSet { deck, set } = &operation {
                    println!(
                        "release: `{set}` was let go over deck {}",
                        deck_letter(*deck)
                    );
                }
                Acted::Emitted(Some(operation))
            }
            // Drag dropped over empty space; logs explanation and leaves selection intact.
            Some(Released::Nowhere { set }) => {
                println!(
                    "release: `{set}` was let go over nothing, so nothing was loaded — a drop \
                     names its deck by landing on that deck's strip, or on its preview cell in \
                     the Program bay, and it names the master chain by landing on the chain's \
                     list in the Master bay"
                );
                Acted::Nothing
            }
            // A row let go over a target that will not take it, which is the
            // one refusal a release makes: the chain's list is a target for
            // every carry and what it holds is a `kind L5` procedure. The
            // reason travels with the release (P-0083).
            Some(Released::Refused { set, why }) => {
                println!("release: `{set}` was let go over the master chain — {why}");
                Acted::Nothing
            }
            None => Acted::Nothing,
        }
    }

    /// Act, say what happened, and hand the outcome back — the repaint decision is
    /// taken from what the operation did, not from the key that asked for it. `p`
    /// over an empty panel and `z` with nothing folded both reach the model and
    /// move nothing.
    pub(crate) fn op(&mut self, op: Op) -> Outcome {
        let outcome = self.panel.op(op);
        // Reset puts default arrangement on screen, clearing the file name.
        if matches!(outcome, Outcome::Reset) {
            self.view.arrangement.name = None;
        }
        self.say_op(op, &outcome);
        outcome
    }

    pub(crate) fn room(&mut self) {
        self.view.room = self.view.room.other();
        println!("room: {}", self.view.room.word());
    }
}

/// Key bindings and their corresponding legend descriptions.
///
/// Verified against window event dispatch in `key_column` tests (ADR-0259, ADR-0333, ADR-0343).
pub(crate) const KEYS: &[(&str, &str)] = &[
    // **The two that move the address**, and they are the same in every bay
    // because they are not addressed to one.
    (
        "tab",
        "focus the next bay, and shift-tab the one before — the ring is the arrangement's own \
         order, down a column and then across",
    ),
    (
        "esc",
        "up one level of the focused bay's address — or, while a name is being typed, abandon \
         the name. it does not quit: close the window",
    ),
    // **The four that act inside a bay**, and they are the same four in every
    // bay because they are rules about kinds of thing rather than about bays
    // (ADR-0259). Which bays have them is
    // `karakuri_console::focus::BUILT` — the mixer and the library today.
    (
        "digit",
        "the nth thing one level below the address, counting what the bay drew from one — and 0 \
         the bay's own head",
    ),
    (
        "up",
        "the neighbour above, or a level's next value — a tenth on the trim and on the fader. \
         the trim is not held at 1.0, because the mix is HDR; the fader is held inside 0 and 1",
    ),
    ("down", "and the neighbour below, or a tenth down"),
    (
        "left",
        "the neighbour to the left, where a bay draws its items in a row — the mixer's strips \
         are one, and naming a strip is the deck selection",
    ),
    ("right", "and the neighbour to the right"),
    (
        "space",
        "the addressed thing's next state — a residency, a blend mode, a mask shape, a library \
         scope — or, on a level, the value it was declared at. on a bay it is the fold, in every \
         one of the nine. a space in a name while the arrangement pill is asking for one",
    ),
    (
        "enter",
        "the act the addressed thing is for: on a library row, load that Set onto the selected \
         deck. it takes the name the arrangement pill is asking for, while it asks",
    ),
    ("backspace", "rub out a letter of that name"),
    // Focused-bay pane folding binding (ADR-0259, ADR-0343).
    (
        "g",
        "fold the split enclosing the focused bay — the pane it sits in. space folds the bay \
         itself",
    ),
    // **The seven that survive as global letters**, plus the one the grammar
    // has not reached yet. A key is global where the operation it names has no
    // operand for focus to supply, or where its only operand is the choice the
    // key itself spells.
    (
        "z",
        "unfold everything folded — the pointer cannot reach one to unfold it",
    ),
    ("r", "reset to a fresh arrangement"),
    ("n", "the room: day or night"),
    (
        "b",
        "tap the beat — three taps set the tempo, any tap sets the phase",
    ),
    (
        ",",
        "halve the grid, and the tracker's octave window with it",
    ),
    (
        ".",
        "double it — refused where the result leaves 60..200 BPM",
    ),
    (
        "k",
        "keep what the selected deck is playing — a Set filed under the time you saved it. the \
         library bay draws no keep control, so the grammar has nothing to reach here yet",
    ),
    ("m", "mute the addressed deck in the mixer"),
    ("s", "solo the addressed deck in the mixer"),
    ("u", "clear any active solo across all decks in the mixer"),
];
