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
    /// What the operator has opened to a model, and the one piece of state in this
    /// struct that is neither the panel's nor a reading of the engine.
    ///
    /// It is a handle rather than a value because the whole point of it is that a
    /// *second* reader has it: `karakuri_mcp::serve` takes a clone and reads it on
    /// every call, so an opening is live rather than a snapshot taken at startup.
    /// `View::opening` is this handle read once a frame; this is the model of
    /// record.
    ///
    /// This process serves MCP when `--mcp` names a port, and the server is handed
    /// this same handle rather than a copy, so the four pills and the audit read
    /// one value. Without the flag the pills still write it and only this program
    /// and its tests read it back.
    pub(crate) opening: Opening,
    /// What MCP access policy is configured per slot, shared with `karakuri_mcp::serve`.
    pub(crate) slot_policies: SlotPolicies,
    /// Store root for saving arrangements and persistent state, if opened.
    pub(crate) store_root: Option<std::path::PathBuf>,
    /// What the last write did, which the transport row's health capsule draws —
    /// `view::Transport::health`, kept here because that value is rebuilt whole
    /// every frame by `transport` and a verdict arrives on one frame in a thousand.
    ///
    /// The one reading in this struct that is a stream rather than a state.
    /// Everything else the row draws is asked of the deck on the frame it is drawn
    /// on; a `swap::Event` exists once, in the drain `staging` makes, and is gone.
    /// So the last one is remembered here for the same reason `view.staging`'s rows
    /// are remembered in the view: it is the drain that forces it, not a
    /// preference.
    ///
    /// `None` until a build produces a verdict, which is most of most runs.
    pub(crate) health: Option<view::Stage>,
    /// The session's four sequencer banks, and the one piece of state in this
    /// struct that is neither the panel's nor a reading of the engine —
    /// `Readout::opening`'s category, over a pattern instead of over what a model
    /// may reach.
    ///
    /// A pattern is authored state and the engine holds none of it (ADR-0222: a
    /// lane is a fifth *route*, so what a lane does reaches the deck as
    /// `Operation::SetOpacity` like everything else). It is kept here because this
    /// window is what polls it and what the press arms edit; the console draws a
    /// copy handed to `View::sequencer` per frame and applies nothing to it
    /// (ADR-0156).
    ///
    /// Nothing saves or loads one yet. ADR-0227 settles where a pattern is kept and
    /// ADR-0320 leaves the file form to the record that has something to serialise,
    /// which is the row that saves one — so these four banks live for the run and
    /// no longer.
    pub(crate) sequencer: karakuri_pattern::Banks,
    /// Where the poll left the playhead, so a lane emits at a step boundary and
    /// never twice for one step.
    ///
    /// It is not in the pattern, and that is the shape rather than an accident: a
    /// pattern is a thing that gets saved and where the playhead has got to is not
    /// (`karakuri_pattern::Playhead`).
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
            // **A pane closed by pulling its boundary out through its own
            // edge, or brought back by pulling that edge in**
            // (`docs/adr/0300-a-pane-folds-by-dragging-its-boundary-out-and-comes-back-by-dragging-it-in.md`).
            // `Panel` performed it, because the arrangement is its own, so
            // this says what happened and asks for nothing: a fold writes no
            // session record.
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

    /// The pointer went up, and what the gesture it ended asked for.
    ///
    /// `onto` is where the pointer is — the caller's answer, because a strip's
    /// geometry is `karakuri-console`'s view and not its model
    /// (`Panel::released`). Two of the three drags do not read it.
    ///
    /// It answers an `Acted` where it used to answer nothing, and the drop is why:
    /// a boundary coming to rest and a fader being let go both ask for nothing —
    /// everything either of them wanted was asked for while it was moving — and a
    /// carry asks for its whole operation here or nowhere.
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
            // **No value in the line, because there is none to print.** Where
            // a fader came to rest is the deck's, and the last thing the drag
            // asked for was printed when it was asked for.
            Some(Released::Let { knob }) => {
                println!(
                    "release: {} lets go of the {}",
                    knob_where(&knob),
                    knob_word(&knob)
                );
                Acted::Nothing
            }
            // **A Set was let go over a strip**, and the load leaves by the
            // door every other control's operation leaves by — `played` is
            // what performs it and says what the deck did about it, exactly as
            // it does for a load from the keyboard. The line here is the
            // *gesture* ending: two ways
            // in, one name, and the same sentences after the naming.
            Some(Released::Dropped(operation)) => {
                if let Operation::LoadSet { deck, set } = &operation {
                    println!(
                        "release: `{set}` was let go over deck {}",
                        deck_letter(*deck)
                    );
                }
                Acted::Emitted(Some(operation))
            }
            // **A carry let go over nothing asks for nothing**, and it says so
            // rather than saying nothing: a row picked up, carried and then
            // silently forgotten reads as a panel that missed the press. The
            // cursor is left on the row that was taken, which is where
            // `enter` in the library would load from next.
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
        // **A reset puts the default arrangement on screen and the default has
        // no name**, so the pill stops naming the file it was showing. Here
        // rather than at either control, because `r` and the menu's *start a
        // new one* are one operation and this is the one place both arrive —
        // and it keys off the outcome rather than off the `Op`, so an op that
        // asked for a reset and did not get one leaves the name alone.
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

/// Every key this window binds, and the sentence the legend prints for it.
///
/// The list an operator reads and the list the tests check are one list.
/// `key_column::the_keys_this_file_lists_are_the_keys_the_window_loop_binds`
/// reads the `match` in `window_event` out of this file's own text and asserts
/// it is exactly these keys, so a key bound and not printed — or a key printed
/// and not bound — fails there rather than being found by an operator pressing
/// it and getting nothing.
///
/// That test was already here and the legend was a second copy of its list,
/// which is the copy that drifted: the printed list stayed at nine keys while
/// ten more were bound, and the operator who read it was told this program
/// folds, solos, resets and quits.
///
/// The rows of `docs/manual/operations.html` each key reaches are
/// `key_column::ROWS`, which is keyed off this table and stays in the test
/// module: a page heading is what a check reads, and it is not something this
/// program says to anybody.
///
/// The order is the order they print in, and it is the grammar's shape since
/// 2026-09-10: the two keys that move the address, then the four that act
/// inside a bay and the rub-out beside them, then the letters that survive
/// globally.
///
/// `digit` is one entry and not ten, because a digit is one key of the grammar:
/// `1` is the mixer's first strip and the library's first row, so ten entries
/// would print ten sentences saying the same thing and the page would still
/// have to name the bay. It is also the one key
/// [`key_column::bound`](key_column) cannot read out of this file's text — the
/// arm is a guard rather than ten literals — so it is contributed by
/// `karakuri_console::focus::BUILT`, which is the dispatch table the grammar is
/// written in (ADR-0333).
///
/// `p` is the latency offset here and was the report until 2026-08-31. The page
/// specifies the offset as `o` and `p`; a badge naming two keys with one of
/// them bound would be a badge that lies, and a panel diagnostic with no useful
/// shortcut to point at loses the letter rather than keeping it. The operation
/// it named is still `karakuri_console::panel::Op::Report` and nothing in this
/// program asks for it.
///
/// Twelve letters left on 2026-09-10 and five more with the other seven bays,
/// and none of them was retired before the grammar reached its row. `0`–`3` are
/// the Mixer's `1`–`4`, `[ ] \` and `; '` are the arrows and `space` on the
/// addressed trim and fader, `m` is `space` on the blend chip, `e` is `space`
/// on the Library's head and `l` is `enter` on one of its rows (ADR-0259,
/// ADR-0333). Then `f` and `s` went with `Readout::target` — `space` on a bay
/// is the fold and `space` on the Program head's `solo` is the solo — `u` with
/// `s`, because one control is both states, and `o` and `p` became the arrows
/// on the Transport's offset (ADR-0343).
///
/// What is left is the seven globals the operand rule keeps, plus `g`, which is
/// addressed to the focused bay because the grammar reaches no pane, and `k`,
/// which the Library bay draws no control for.
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
    // **The one letter addressed to the focus**, which is the third category
    // ADR-0259 creates and ADR-0343 names: its operand is the focused bay, so
    // it is not global under the operand rule and it is not one of the six
    // keys either. It is here because the grammar reaches no pane — `Tab`
    // stops at a bay — and *Fold a pane away* has to be reachable from the
    // keyboard alone.
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
