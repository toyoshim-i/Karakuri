use super::*;

// ---------------------------------------------------------------------------
// The grammar: what the six keys reach inside a bay
// ---------------------------------------------------------------------------

/// The four grammar keys addressed to the current focus target (ADR-0259).
///
/// Excludes global navigation keys (`Tab` and `esc`). Classified for table dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Grammar {
    /// A digit — the nth thing one level below the address, and `0` the head.
    Digit,
    /// An arrow — the neighbour of the addressed thing, along the axis it is drawn
    /// on, or the next value of a level.
    Arrows,
    /// `space` — the addressed thing's next state, or a level's declared default,
    /// or a bay's fold.
    Space,
    /// `enter` — the act the addressed thing is for.
    Enter,
}

impl Grammar {
    /// All four, in ADR-0259's own order.
    pub const ALL: [Grammar; 4] = [
        Grammar::Digit,
        Grammar::Arrows,
        Grammar::Space,
        Grammar::Enter,
    ];
}

/// Direction for navigation arrow keys along vertical or horizontal axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrow {
    Up,
    Down,
    Left,
    Right,
}

impl Arrow {
    /// Whether this arrow lies along a row — `←→` — rather than down a column.
    pub const fn across(self) -> bool {
        matches!(self, Arrow::Left | Arrow::Right)
    }

    /// Which way along its axis: `-1` for up and left, `1` for down and right. A
    /// list runs down and a row runs right, so *further on* is one direction and
    /// the page needs no second rule for it.
    pub const fn step(self) -> i32 {
        match self {
            Arrow::Up | Arrow::Left => -1,
            Arrow::Down | Arrow::Right => 1,
        }
    }
}

/// A grammar key press carrying key-specific parameters (digit index or arrow direction).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    Digit(usize),
    Arrow(Arrow),
    Space,
    Enter,
    AltEnter,
    CtrlEnter,
}

impl Press {
    /// Which of the four keys this is — what the table is written in.
    pub const fn key(self) -> Grammar {
        match self {
            Press::Digit(_) => Grammar::Digit,
            Press::Arrow(_) => Grammar::Arrows,
            Press::Space => Grammar::Space,
            Press::Enter | Press::AltEnter | Press::CtrlEnter => Grammar::Enter,
        }
    }
}

/// Interactive panel controls addressable by the focus ladder (ADR-0259).
///
/// Variants correspond to addressable elements in draw order, excluding non-interactive readouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    // -- the Mixer ---------------------------------------------------------
    /// A strip's residency chip — `space` asks the deck for the next of the three.
    Tally,
    /// A strip's trim, which is a level: arrows step it and `space` returns it to
    /// the value it was declared at.
    Trim,
    /// A strip's channel fader, and [`Control::Trim`]'s rules exactly.
    Fader,
    /// A strip's blend chip — `space` cycles add, over and max.
    Blend,
    /// A strip's mask mini — `space` cycles the shape.
    Mask,
    /// The transition row's shape pill, in the Mixer's head: the settings are about
    /// the bay rather than about any one strip, which is what a head is for.
    Shape,
    /// The transition row's quantum pill. [`Control::Shape`]'s rules.
    Quantum,
    /// The transition row's length pill. [`Control::Shape`]'s rules.
    Length,
    /// The transition row's `go` capsule — the act, on the addressed strip, which
    /// is the deck selection.
    Go,

    // -- the Library -------------------------------------------------------
    /// The Library head's scope chips — `space` steps to the next and wraps.
    Scope,
    /// A row's star — `space` puts it on this Set or takes it off.
    Star,
    /// A row's `params` chip — `enter` opens what that Set holds and declares.
    Params,

    // -- the Transport -----------------------------------------------------
    /// The tempo figure, which is a track a press positions rather than a level
    /// anything steps.
    Tempo,
    /// The `½ ×2` pair, whose only operand is the direction the key spells.
    Grid,
    /// The latency offset, which is a level.
    Offset,
    /// The beat grid — a readout.
    Beat,
    /// The bar counter — a readout.
    Bar,
    /// The frame cost — a readout.
    Cost,
    /// The audio-in pill, whose press puts a card of inputs down.
    Audio,
    /// The arrangement pill, whose press puts its menu down.
    Arrangement,
    /// The tone map pill — `space` cycles the four operators.
    Tonemap,
    /// The exposure track, which is a level.
    Exposure,

    // -- the two cards the Transport's address walks -----------------------
    /// One input of the audio-in pill's card, by the name the machine answered
    /// with — `enter` attaches it as the beat source.
    Input,
    /// The arrangement menu's *save*. `enter` files the arrangement under the name
    /// in use, and asks for one where there is none.
    Save,
    /// The arrangement menu's *start a new one*.
    New,
    /// One name the arrangement menu lists — `enter` puts that arrangement back.
    Filed,

    // -- the Program bay ---------------------------------------------------
    /// The Program head's `solo` — `space` is *soloed* and *not*.
    Solo,
    /// The Program head's class pill — `space` opens the class or shuts it.
    Class,
    /// The picture. Its on and off is the Outputs row's one control.
    Picture,
    /// One of the four deck preview cells — a monitor, and a monitor is a thing you
    /// look at.
    Cell,

    // -- the Inspector -----------------------------------------------------
    /// A pane's deck head, which holds controls of its own.
    DeckHead,
    /// One of a pane's node groups, which holds controls of its own.
    Node,
    /// The deck head's sync chip — `space` cycles what the material allows.
    Sync,
    /// The deck head's anchor, scrubbed a quarter beat by the arrows.
    Anchor,
    /// The deck head's composite chip — `space` names the layering it is not in.
    Composite,
    /// A node head's authority chip — `space` cycles man, sug and auto.
    Authority,
    /// A node's renderer chips — `space` steps which one is live.
    Renderer,
    /// One parameter row of a node group, which is a level, and whose act is taking
    /// an attachment back.
    Param,

    // -- the Master chain --------------------------------------------------
    /// The master out, which is a level.
    Out,
    /// One slot of the master chain, which holds controls of its own.
    Slot,
    /// One parameter row of a slot, which is a level: the arrows step it a tenth of
    /// the range its procedure declares and `space` returns it to the value that
    /// procedure declared it at.
    ChainParam,
    /// A slot's cut chip — `space` cycles `mix` and `exit`. It is addressed on
    /// every slot and draws on a slot whose procedure declares `retains`; on one
    /// that does not it declines and says so.
    ChainCut,
    /// The `−` at the end of a slot's row — `enter` takes that slot out of the
    /// chain.
    ChainRemove,
    /// The Master bay's `+ add` — `enter` puts the chooser down, and the chooser's
    /// entries are the rung under it.
    AddEffect,
    /// One entry of the `+ add` chooser: a `kind L5` procedure the library holds.
    /// `enter` appends a slot of it and takes the card away.
    ChainProcedure,

    // -- the Sequencer -----------------------------------------------------
    /// The head's grid mode pill — `space` names the other of the two.
    Mode,
    /// One of the head's four bank pills — `space` arms that pattern.
    Bank,
    /// The foot's `+ lane` — `enter` puts the chooser down, and the chooser's
    /// entries are the rung under it.
    AddLane,
    /// One entry of the `+ lane` chooser: a target a lane can drive. `enter` points
    /// a lane at it and takes the card away.
    LaneTarget,
    /// A lane's label, whose mute is a state.
    Label,
    /// One step of a lane — a row of alike cells the arrows walk across while the
    /// lanes they sit in are walked down.
    Step,

    // -- the Outputs row ---------------------------------------------------
    /// A sink, which has exactly one state.
    Sink,

    // -- the Staging lane --------------------------------------------------
    /// A candidate row's *keep this candidate*.
    Keep,
    /// A candidate row's *put the node's previous version back*.
    Back,
}

/// Interaction semantics of a control defining which grammar keys act on it (ADR-0259).
///
/// Categorizes controls into cyclical states, stepped levels, actions, cell rows, uninitialized continuums, or inert items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answers {
    /// A closed list: `space` cycles it and the arrows decline, because the values
    /// of a closed list are not laid out on an axis.
    State,
    /// A continuum: the arrows step it and `space` returns it to the value it was
    /// declared at.
    Level,
    /// A control that performs: `enter` does it, and `space` and the arrows
    /// decline.
    Act(Act),
    /// One of a row (or a column) of alike controls the arrows walk and `space`
    /// sets — the Sequencer's steps, and the one place a bay walks two axes:
    /// `across` is this control's and [`Built::across`] is its items'.
    Cells { across: bool },
    /// A continuum the arrows step, with no value it was declared at — so `space`
    /// declines with this sentence where a level returns to its default.
    Track(&'static str),
    /// Drawn, addressed, and answering nothing, with the sentence that says why — a
    /// digit lands on it and the ring is drawn, which is ADR-0259's own finding
    /// about the four preview cells.
    Nothing(&'static str),
}

impl Control {
    /// What this control answers to. A `match` with no wildcard, so a control added
    /// to the panel is a compile error here rather than one the grammar quietly
    /// declines on.
    pub const fn answers(self) -> Answers {
        match self {
            Control::Tally | Control::Blend | Control::Mask => Answers::State,
            Control::Trim | Control::Fader => Answers::Level,
            Control::Shape | Control::Quantum | Control::Length => Answers::State,
            Control::Go => Answers::Act(Act::Go),
            Control::Scope => Answers::State,
            Control::Star => Answers::State,
            Control::Params => Answers::Act(Act::Read),
            // Tempo track: stepped by 1 BPM via arrows within safety bounds (ADR-0291, ADR-0350); `space` is inert.
            Control::Tempo => Answers::Track(
                "the tempo figure has no value it was declared at, so there is nothing for \
                 space to return it to — the arrows step it a beat a minute, a press names one \
                 outright, and b taps the beat",
            ),
            // The operand is the direction the key spells, which is the
            // operand rule's own second clause: `,` and `.` stay global.
            Control::Grid => Answers::Nothing(
                "the grid's only operand is the direction, which is what the two keys spell — \
                 press , to halve it and . to double it",
            ),
            Control::Offset => Answers::Level,
            Control::Beat => Answers::Nothing("the beat grid is a readout — b taps the beat"),
            Control::Bar => Answers::Nothing("the bar counter is a readout"),
            Control::Cost => Answers::Nothing("the frame cost is a readout"),
            Control::Audio | Control::Arrangement => Answers::Act(Act::Open),
            Control::Input => Answers::Act(Act::Attach),
            Control::Save => Answers::Act(Act::Save),
            // New arrangement reset is handled globally via hotkey `r`.
            Control::New => Answers::Nothing(
                "start a new one is the reset, and r starts one from anywhere on this panel",
            ),
            Control::Filed => Answers::Act(Act::Restore),
            Control::Tonemap => Answers::State,
            Control::Exposure => Answers::Level,
            Control::Solo | Control::Class => Answers::State,
            // Picture output is controlled via the Outputs row sinks.
            Control::Picture => Answers::Nothing(
                "the picture's on and off is the Outputs row's sink — tab to the outputs row \
                 and press space on it",
            ),
            Control::Cell => Answers::Nothing(
                "a preview cell is a monitor fixed to a deck, so there is nothing here to set \
                 or to perform — the residency is the mixer's tally chip",
            ),
            // The two rungs of the Inspector that hold controls rather than
            // being ones: a digit descends and the press is a move.
            Control::DeckHead | Control::Node => Answers::Nothing(
                "this is a rung and not a control — press a digit to name one of the controls \
                 in it",
            ),
            Control::Sync | Control::Composite | Control::Authority | Control::Renderer => {
                Answers::State
            }
            Control::Anchor | Control::Param => Answers::Level,
            Control::Out => Answers::Level,
            // A rung and not a control, on the Inspector's two rungs' terms.
            Control::Slot => Answers::Nothing(
                "a chain slot is a rung and not a control — press a digit to name one of the \
                 controls in it",
            ),
            Control::ChainParam => Answers::Level,
            Control::ChainCut => Answers::State,
            Control::ChainRemove => Answers::Act(Act::Remove),
            Control::AddEffect => Answers::Act(Act::Open),
            Control::ChainProcedure => Answers::Act(Act::Add),
            Control::Mode | Control::Bank => Answers::State,
            Control::AddLane => Answers::Act(Act::Open),
            Control::LaneTarget => Answers::Act(Act::Point),
            Control::Label => Answers::State,
            Control::Step => Answers::Cells { across: true },
            Control::Sink => Answers::State,
            Control::Keep => Answers::Act(Act::Keep),
            Control::Back => Answers::Act(Act::Back),
        }
    }

    /// Whether this control is a level — a continuum the arrows step and `space`
    /// returns to its default.
    pub const fn level(self) -> bool {
        matches!(self.answers(), Answers::Level)
    }

    /// Action executed by `enter`, or `None` for setter controls (ADR-0259).
    ///
    /// Parameter rows act as both levels and actions ("take back" sensitivity act).
    pub const fn acts(self) -> Option<Act> {
        match self.answers() {
            Answers::Act(act) => Some(act),
            _ => match self {
                Control::Param => Some(Act::TakeBack),
                _ => None,
            },
        }
    }

    /// Which of the four keys act on this control.
    pub const fn reached_by(self, key: Grammar) -> bool {
        match key {
            Grammar::Enter => self.acts().is_some(),
            // Intermediate traversal rungs: allows digits to descend, and `+ lane` performs on `enter`.
            Grammar::Digit => matches!(
                self,
                Control::DeckHead
                    | Control::Node
                    | Control::AddLane
                    | Control::Audio
                    | Control::Arrangement
                    | Control::Slot
                    | Control::AddEffect
            ),
            // Card items support directional navigation.
            Grammar::Arrows
                if matches!(
                    self,
                    Control::AddLane
                        | Control::LaneTarget
                        | Control::Audio
                        | Control::Arrangement
                        | Control::Input
                        | Control::Save
                        | Control::New
                        | Control::Filed
                        | Control::AddEffect
                        | Control::ChainProcedure
                ) =>
            {
                true
            }
            _ => match (self.answers(), key) {
                // A state has a next value and a level has a default, so
                // `space` acts on both — which is what makes it the key of the
                // grammar an operator reaches for. A cell is set by it too.
                (Answers::State | Answers::Level | Answers::Cells { .. }, Grammar::Space) => true,
                // A level's neighbour is its next value and a cell's is the
                // cell beside it. A track's is its next value too. A state has
                // neither.
                (Answers::Level | Answers::Track(_) | Answers::Cells { .. }, Grammar::Arrows) => {
                    true
                }
                _ => false,
            },
        }
    }
}

/// Action performed when activating an addressed control with `enter`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    /// The Set under the Library's cursor, loaded onto the selected deck — with
    /// both operands on screen before the press, which is what that argument was
    /// always about.
    Load,
    /// What one Set holds and declares, opened on the row the cursor is on.
    Read,
    /// The candidate on this row, kept.
    Keep,
    /// The node's previous version, put back.
    Back,
    /// The transition the Mixer's head is set to, run on the addressed strip.
    Go,
    /// The attachment on a parameter row, taken back — after it the row is a handle
    /// again.
    TakeBack,
    /// The chooser under the addressed control, put down. After it the card's
    /// entries are the rung below the control and the address descends into them by
    /// digit or by arrow.
    Open,
    /// A lane, pointed at the target on the addressed entry. After it the card is
    /// gone and the address is back on the control that opened it.
    Point,
    /// The input on the addressed row of the audio-in card, attached as the beat
    /// source.
    Attach,
    /// The arrangement, filed under the name in use — or a name asked for, where
    /// there is none in use.
    Save,
    /// The arrangement named on the addressed row, put back.
    Restore,
    /// The addressed slot, taken out of the master chain.
    Remove,
    /// Removes the addressed lane from the pattern via `enter`.
    RemoveLane,
    /// A slot of the addressed procedure, appended to the master chain. After it
    /// the card is gone and the address is back on `+ add`.
    Add,
}

/// Composition of controls at a rung: fixed prefix controls followed by repeating elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Of {
    /// The controls drawn before the counted ones, in draw order.
    pub first: &'static [Control],
    /// The control every thing after them is, or `None` where a thing draws only
    /// the controls above.
    pub then: Option<Control>,
    /// Suffix controls drawn after the repeating elements in draw order (ADR-0352, e.g. Master `+ add`).
    pub last: &'static [Control],
}

impl Of {
    /// A rung with nothing on it.
    pub const NONE: Of = Of {
        first: &[],
        then: None,
        last: &[],
    };

    /// A rung that is a fixed list and nothing after it.
    pub const fn just(first: &'static [Control]) -> Of {
        Of {
            first,
            then: None,
            last: &[],
        }
    }

    /// Returns the 1-indexed `nth` control on this rung given `drawn` total elements, or `None` if out of bounds.
    pub fn nth(&self, nth: usize, drawn: usize) -> Option<Control> {
        if nth == HEAD || nth > drawn {
            return None;
        }
        if let Some(control) = self.first.get(nth - 1) {
            return Some(*control);
        }
        // Suffix index resolved relative to drawn bounds.
        let after = drawn - nth;
        if let Some(control) = self.last.len().checked_sub(after + 1) {
            return Some(self.last[control]);
        }
        self.then
    }

    /// How many things this rung draws where the repeat runs `repeats` times.
    pub fn len(&self, repeats: usize) -> usize {
        self.first.len()
            + match self.then {
                Some(_) => repeats,
                None => 0,
            }
            + self.last.len()
    }

    /// Whether any control of this rung is reached by `key`, against a rung drawing
    /// everything it can.
    pub(crate) fn reaches(&self, key: Grammar) -> bool {
        self.first.iter().any(|c| c.reached_by(key))
            || self.then.is_some_and(|c| c.reached_by(key))
            || self.last.iter().any(|c| c.reached_by(key))
    }
}

/// Structural classification of bay items: direct controls vs. compound items with sub-controls (ADR-0259).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Items {
    /// Bay items are direct controls without an intermediate item rung (Transport, Outputs, Master, Program).
    Controls(Of),
    /// The bay lists alike things and a digit names the nth of them; these are the
    /// controls one of them draws.
    Alike(Of),
}

/// Grammar specification defining address levels and dispatch for a bay (ADR-0259, ADR-0333, ADR-0343).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Built {
    /// The bay, by the name the arrangement gives it.
    pub bay: &'static str,
    /// The controls of the bay's head, in the order the head draws them — empty for
    /// a head that holds none and for a bay that draws none.
    pub head: &'static [Control],
    /// What the bay's items are.
    pub items: Items,
    /// Sub-controls descended through the specified control (third rung / modal cards) (ADR-0259).
    pub under: &'static [(Control, Of)],
    /// What `enter` on an item performs, or `None` where an item is not an act.
    pub act: Option<Act>,
    /// Whether naming an item is itself an operation. The Mixer's is: a digit that
    /// names a strip *is* the deck selection, which is why that row keeps a key
    /// badge rather than losing one.
    pub selects: bool,
    /// Which way the arrows walk this bay's items — `true` for a row, so `←→`, and
    /// `false` for a column, so `↑↓`. It is a reading of how the bay draws them and
    /// not a preference.
    pub across: bool,
}

impl Built {
    /// The controls of one of this bay's items, or of the bay itself where its
    /// items are its controls.
    pub fn item(&self) -> Of {
        match self.items {
            Items::Controls(of) => of,
            Items::Alike(of) => of,
        }
    }

    /// What is under `control`, or [`Of::NONE`] for a control nothing is under.
    pub fn beneath(&self, control: Control) -> Of {
        self.under
            .iter()
            .find(|(found, _)| *found == control)
            .map_or(Of::NONE, |(_, of)| *of)
    }

    /// Returns whether the grammar key acts within this bay (excluding global bay folding under [`ANY`]).
    pub fn reaches(&self, key: Grammar) -> bool {
        match key {
            // Digits select items or bay head (0).
            Grammar::Digit => true,
            // Arrow keys walk items or step levels.
            Grammar::Arrows => true,
            _ => {
                self.head.iter().any(|c| c.reached_by(key))
                    || self.item().reaches(key)
                    || self.under.iter().any(|(_, of)| of.reaches(key))
                    || (key == Grammar::Enter && self.act.is_some())
            }
        }
    }
}

/// Name identifier for the Program bay picture node.
pub const PICTURE: &str = "program-view";

/// The Transport row, by the name the arrangement gives it. [`MIXER`]'s reason.
pub const TRANSPORT: &str = "transport";
/// The Staging lane. [`MIXER`]'s reason.
pub const STAGING: &str = "staging";
/// The Program bay. [`MIXER`]'s reason.
pub const PROGRAM: &str = "program";
/// The Inspector. [`MIXER`]'s reason.
pub const INSPECTOR: &str = "inspector";
/// The Master chain. [`MIXER`]'s reason.
pub const MASTER: &str = "master";
/// The Sequencer. [`MIXER`]'s reason.
pub const SEQUENCER: &str = "sequencer";
/// The Outputs row. [`MIXER`]'s reason.
pub const OUTPUTS: &str = "outputs";

/// What `bay` is made of, or `None` for one whose grammar is not built.
pub fn built(bay: &str) -> Option<&'static Built> {
    BUILT.iter().find(|found| found.bay == bay)
}

/// Flattened (bay, key) bindings in the grammar dispatch table, including [`ANY`] for global bay fold.
pub fn reaches() -> Vec<(&'static str, Grammar)> {
    let mut found = vec![(ANY, Grammar::Space)];
    for bay in BUILT {
        for key in Grammar::ALL {
            if bay.reaches(key) {
                found.push((bay.bay, key));
            }
        }
    }
    found
}
