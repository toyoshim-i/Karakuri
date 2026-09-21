use super::*;

// ---------------------------------------------------------------------------
// The grammar: what the six keys reach inside a bay
// ---------------------------------------------------------------------------

/// The four keys of the grammar that are addressed to whatever the bay's
/// address is on.
///
/// `Tab` and `esc` are the other two of ADR-0259's six and are not here: they
/// move the address rather than acting on it, so they reach the same thing in
/// every bay and no bay has to declare them.
///
/// This is the classifier the table is written in and the check reads;
/// [`Press`] is one press with what the key itself said.
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

/// Which way an arrow points. Four rather than two, because the axis is half of
/// what an arrow means: the Mixer's strips are a row and the Library's rows are
/// a column, so `←→` walk one and `↑↓` the other, and a level is stepped by
/// `↑↓` whichever bay it is in.
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

/// One press of the grammar, with what the key itself said.
///
/// A digit carries which digit and an arrow carries which way; `space` and
/// `enter` carry nothing, because the address is the whole of what they are
/// addressed to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    Digit(usize),
    Arrow(Arrow),
    Space,
    Enter,
}

impl Press {
    /// Which of the four keys this is — what the table is written in.
    pub const fn key(self) -> Grammar {
        match self {
            Press::Digit(_) => Grammar::Digit,
            Press::Arrow(_) => Grammar::Arrows,
            Press::Space => Grammar::Space,
            Press::Enter => Grammar::Enter,
        }
    }
}

/// A control a bay draws, as the grammar addresses it.
///
/// One variant per control the nine bays draw at a rung the address reaches, in
/// the order the bay draws them — which is what makes a digit name the nth of
/// them. It is not a list of every rectangle on the panel: a readout is not a
/// control, and a control ADR-0259's walk does not name is not addressed — each
/// of those is written down at the bay's row in [`BUILT`] rather than left to
/// be noticed.
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

/// What a control answers to, which is the one thing the grammar has to know
/// about a control and the whole of what decides which of the four keys act on
/// it.
///
/// ADR-0259's kinds, as the grammar reads them: a state is a closed list
/// `space` cycles, a level is a continuum the arrows step and `space` returns
/// to its default, an act is a control that performs rather than sets, and the
/// last three are what the walk found that the record's seven do not have a
/// word for — a row of alike cells, a continuum with no value it was declared
/// at, and a control that is drawn and answers nothing.
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
            // **The figure is a track and not a level**: a press positions it
            // inside a band that is a guard on a hand (ADR-0291) and the arrows
            // step it by one beat a minute, which is
            // [ADR-0350](../../../docs/adr/0350-the-transports-two-cards-are-walked-and-the-tempo-figure-steps-by-a-beat-a-minute.md).
            // What the grid runs at is not a value this row declares, so there
            // is no state for `space` to return it to.
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
            // **The reset is one row of the page and `r` is the key that
            // reaches it**, so this row of the menu is drawn and is not
            // performed here.
            Control::New => Answers::Nothing(
                "start a new one is the reset, and r starts one from anywhere on this panel",
            ),
            Control::Filed => Answers::Act(Act::Restore),
            Control::Tonemap => Answers::State,
            Control::Exposure => Answers::Level,
            Control::Solo | Control::Class => Answers::State,
            // **The picture's on and off is the Outputs row's one control**,
            // and it is one row of the page with one badge. A second press for
            // it here would be a route no badge could name.
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

    /// What `enter` on this control performs, or `None` for one that sets rather
    /// than performs.
    ///
    /// It is [`Control::answers`] with one addition, and the addition is the one
    /// place a control is two of ADR-0259's kinds at once: a parameter row is a
    /// level and it also performs, because the sensitivity row under it carries
    /// `take back` and that is the act of the control the row draws rather than a
    /// control of its own — a parameter with nothing holding it draws no
    /// sensitivity row at all. A fourth rung for one chip would be a rung whose
    /// only inhabitant is sometimes there.
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
            // **Two rungs are not controls**, and a digit descends through
            // them — which is the only way the Inspector's third rung is
            // reached at all. **`+ lane` is a control and a rung both**: it
            // performs — `enter` puts the chooser down — and a digit then
            // names the nth entry of the card it put there.
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
            // **A card's rows are walked**, so the arrows reach the control
            // that opens the card and the rows under it, where they reach
            // neither of the other two acts.
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
    /// The addressed lane, taken out of the pattern — what the minus at the end of
    /// the lane's row asks for, reached by `enter` on the lane itself. A second
    /// variant rather than the one above because the two are reached at different
    /// rungs and name different operations.
    RemoveLane,
    /// A slot of the addressed procedure, appended to the master chain. After it
    /// the card is gone and the address is back on `+ add`.
    Add,
}

/// What the things at one rung are made of: the controls drawn first, and the
/// control every one after them is.
///
/// A fixed list is not enough for two of the nine. An Inspector pane draws one
/// deck head and then a node group per node of the Set in the slot, and a
/// Sequencer lane draws one label and then a cell per step of the mode — so a
/// rung is *a prefix and a repeat*, and how many of the repeat there are is the
/// bay's own reading rather than anything written here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Of {
    /// The controls drawn before the counted ones, in draw order.
    pub first: &'static [Control],
    /// The control every thing after them is, or `None` where a thing draws only
    /// the controls above.
    pub then: Option<Control>,
    /// The controls drawn *after* the counted ones, in draw order.
    ///
    /// The Master bay is what asks for it: its items are the out fader, then one
    /// per slot of the chain, then `+ add`, and a rung that is a prefix and a
    /// repeat cannot say that. Every entry here is drawn whenever the rung is, so
    /// the count a bay reports includes all of them — which is what lets the nth
    /// be resolved from the end
    /// ([ADR-0352](../../../docs/adr/0352-the-chains-list-is-the-master-bays-items-and-a-slot-is-taken-out-by-a-glyph-on-its-row.md)).
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

    /// The `nth` control of this rung, counting from one, against a rung drawing
    /// `drawn` of them — or `None` past the end.
    ///
    /// `drawn` is the bay's reading and not this table's: a pane with two nodes
    /// draws three things at its second rung and a pane with nine draws ten, and a
    /// digit counts what was drawn.
    pub fn nth(&self, nth: usize, drawn: usize) -> Option<Control> {
        if nth == HEAD || nth > drawn {
            return None;
        }
        if let Some(control) = self.first.get(nth - 1) {
            return Some(*control);
        }
        // **The suffix is counted from the end**, which is what makes a rung
        // whose middle repeats resolvable at all: how many of the repeat there
        // are is the bay's reading, and the controls after them are however
        // many this table names.
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

/// What a bay's items are.
///
/// Two shapes, and the split is ADR-0259's own: a headless row's *"items are
/// the controls left to right"*, and every other bay's items are alike things
/// with controls under them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Items {
    /// The bay's items are its controls, so a digit at bay level names one outright
    /// and there is no item rung at all — the Transport, the Outputs row, the
    /// Master chain and the Program bay.
    ///
    /// An [`Of`] and not a plain list, because the Master's items are the out
    /// fader, one per slot of the chain, and `+ add`.
    Controls(Of),
    /// The bay lists alike things and a digit names the nth of them; these are the
    /// controls one of them draws.
    Alike(Of),
}

/// One bay's grammar: what each level of its address is made of.
///
/// The table [`BUILT`] is written in, and the thing
/// `crates/karakuri/src/main.rs`'s `key_column` reads in place of this
/// program's `match` — a digit reaches a different row in every bay, so a scan
/// of the arms cannot say which row a press lands on and a dispatch is what can
/// ([ADR-0259](../../../docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md),
/// [ADR-0333](../../../docs/adr/0333-the-console-resolves-the-address-and-the-window-loop-names-the-operation.md),
/// [ADR-0343](../../../docs/adr/0343-the-grammar-reaches-all-nine-bays-and-space-on-a-bay-is-the-fold.md)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Built {
    /// The bay, by the name the arrangement gives it.
    pub bay: &'static str,
    /// The controls of the bay's head, in the order the head draws them — empty for
    /// a head that holds none and for a bay that draws none.
    pub head: &'static [Control],
    /// What the bay's items are.
    pub items: Items,
    /// The third rung: the control the address descends *through*, and what is
    /// under it. ADR-0259 calls the Inspector three deep and says it is *"what
    /// proves the address has to be a path rather than two levels"*.
    ///
    /// Four bays have one, and which rung it hangs off is what differs. The
    /// Inspector's is under an *item* — a pane's deck head and its node groups.
    /// The Sequencer's is under a *head* control, `+ lane`. The Transport's two
    /// and the Master's `+ add` are under a headless row's own controls, and the
    /// Master's slots are there too. Every one of those but the slots is a card
    /// the address descends into and `esc` takes away.
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

    /// Whether one of the four keys acts anywhere in this bay, derived from what
    /// the bay is made of rather than listed beside it — a second list would be a
    /// second answer to *what does `space` do here*.
    ///
    /// The fold is not counted here. `space` at bay level folds every one of the
    /// nine, so counting it would make this answer `true` for `space` in a bay
    /// whose controls answer nothing — and the page would then owe nine badges for
    /// one rule. It is declared once, under [`ANY`], by [`reaches`].
    pub fn reaches(&self, key: Grammar) -> bool {
        match key {
            // **A digit names the nth item, and `0` the head.** Every bay the
            // manual lists draws items, and every bay has a head or something
            // standing in for one, so a digit reaches somewhere in any bay
            // whose grammar is built at all.
            Grammar::Digit => true,
            // **The arrows walk those items**, whatever the items are made of,
            // and step whatever levels they hold on top of that.
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

/// The nine bays' focus ladder specifications (ADR-0259, ADR-0333, ADR-0343).
pub const BUILT: &[Built] = &[
    // -- Transport ---------------------------------------------------------
    //
    // **A headless row (ADR-0159), so `0` names the row itself** and there is
    // nothing under it: this bay's controls *are* its items, which is what
    // `Items::Controls` is.
    //
    // **The order is the record's walk, left to right**, with the offset among
    // the tracker group's three where it is drawn. **Four drawn things are not
    // items**: the `tap` capsule, which ADR-0259 keeps global because *"tapping
    // is a hand keeping time, which a `Tab` first would spoil"*, and the `rec`,
    // `learn` and `map` pills, which the walk does not name — the first two
    // reach rows the key column marks `gap` for reasons that are not about
    // letters, and the third is a readout.
    Built {
        bay: TRANSPORT,
        head: &[],
        items: Items::Controls(Of::just(&[
            Control::Tempo,
            Control::Grid,
            Control::Offset,
            Control::Beat,
            Control::Bar,
            Control::Cost,
            Control::Audio,
            Control::Arrangement,
            Control::Tonemap,
            Control::Exposure,
        ])),
        // **Two of those controls are rungs as well**, and they are the only
        // ones under a bay whose items are its controls: the audio-in pill's
        // card lists the machine's inputs, and the arrangement pill's menu
        // lists *save*, *start a new one* and every name filed. `enter` puts
        // each card down, a digit names the nth row of it, `↑↓` walk the rows
        // and `esc` takes the card away
        // ([ADR-0350](../../../docs/adr/0350-the-transports-two-cards-are-walked-and-the-tempo-figure-steps-by-a-beat-a-minute.md)).
        under: &[
            (
                Control::Audio,
                Of {
                    last: &[],
                    first: &[],
                    then: Some(Control::Input),
                },
            ),
            (
                Control::Arrangement,
                Of {
                    last: &[],
                    first: &[Control::Save, Control::New],
                    then: Some(Control::Filed),
                },
            ),
        ],
        act: None,
        selects: false,
        across: true,
    },
    // -- Library -----------------------------------------------------------
    //
    // **`0` is the head and its controls are the scope chips**; items are the
    // listed Sets, the digits name the first nine and `↑↓` walk them.
    //
    // **A row's own controls are its star and its `params` chip**, which is
    // what ADR-0333 left owed: ADR-0259's *"a row has no state"* was written
    // before the star was drawn, and a star is a state.
    //
    // **The row menu is drawn and is not one of them.** Its items are a card,
    // `input::claim`'s rule 2 gives every press on the console to the panel
    // while one is down, and a card the grammar can open and cannot then walk
    // is a control that traps the address.
    Built {
        bay: LIBRARY,
        head: &[Control::Scope],
        items: Items::Alike(Of::just(&[Control::Star, Control::Params])),
        under: &[],
        act: Some(Act::Load),
        selects: false,
        across: false,
    },
    // -- Staging -----------------------------------------------------------
    //
    // **Nothing here has a state at all**, so `space` reaches nothing in this
    // bay below its fold — ADR-0259's own finding, and the second of the two
    // bays that are lists of things that happened rather than things you set.
    // A row's controls are its two acts, which is the record's `n 1` and `n 2`.
    Built {
        bay: STAGING,
        head: &[],
        items: Items::Alike(Of::just(&[Control::Keep, Control::Back])),
        under: &[],
        act: None,
        selects: false,
        across: false,
    },
    // -- Program -----------------------------------------------------------
    Built {
        bay: PROGRAM,
        head: &[Control::Solo, Control::Class],
        items: Items::Controls(Of::just(&[
            Control::Picture,
            Control::Cell,
            Control::Cell,
            Control::Cell,
            Control::Cell,
        ])),
        under: &[],
        act: None,
        selects: false,
        across: true,
    },
    // -- Inspector ---------------------------------------------------------
    //
    // **Three deep, and the deepest bay on the panel.** Items are its panes; a
    // pane's controls are its deck head and its node groups; a node group's
    // controls are its authority chip, its renderer chips and its parameter
    // rows. So `1 2 3` is the first pane's second thing's third control.
    //
    // **A parameter's act is taking an attachment back**, which is the
    // sensitivity row's `take back` reached one rung up rather than a fourth
    // rung of its own: a row with no attachment has no sensitivity row under
    // it, so the act is the parameter's where there is one to take back.
    Built {
        bay: INSPECTOR,
        head: &[],
        items: Items::Alike(Of {
            last: &[],
            first: &[Control::DeckHead],
            then: Some(Control::Node),
        }),
        under: &[
            (
                Control::DeckHead,
                Of::just(&[Control::Sync, Control::Anchor, Control::Composite]),
            ),
            (
                Control::Node,
                Of {
                    last: &[],
                    first: &[Control::Authority, Control::Renderer],
                    then: Some(Control::Param),
                },
            ),
        ],
        act: None,
        selects: false,
        across: false,
    },
    // -- Mixer -------------------------------------------------------------
    //
    // **Items are the four strips and a strip's controls are its five**, in
    // the order `view::mixer` draws them down the strip. So `2 3` is deck B's
    // fader, which is ADR-0259's own example and the control the mock draws
    // `.wfocus` on.
    //
    // **The transition row is the head's**, which is the question ADR-0333
    // left open. A head is *"where a bay keeps the controls that are about the
    // bay rather than about anything in it"*, and `Operation::SetTransition`
    // is the one row of this bay that carries no slot: the settings decide what
    // the next move means wherever it lands. The `go` capsule is the fourth,
    // and it acts on the **addressed strip** — the deck selection, which is
    // this bay's remembered address.
    Built {
        bay: MIXER,
        head: &[
            Control::Shape,
            Control::Quantum,
            Control::Length,
            Control::Go,
        ],
        items: Items::Alike(Of::just(&[
            Control::Tally,
            Control::Trim,
            Control::Fader,
            Control::Blend,
            Control::Mask,
        ])),
        under: &[],
        act: None,
        selects: true,
        across: true,
    },
    // -- Master ------------------------------------------------------------
    //
    // **Items are the out fader, the chain's slots and `+ add`**, in the order
    // the bay draws them. `↑↓` step `out` and `space` returns it to its
    // default.
    //
    // **A slot is a rung as well as an item**: its controls are its parameter
    // rows, its cut chip and its `−`. `↑↓` on a parameter row step it a tenth
    // of the range its procedure declares and `space` returns it to what that
    // procedure declared it at; `space` on the cut chip cycles `mix` and
    // `exit`; `enter` on the `−` takes the slot out of the chain.
    //
    // **The cut chip keeps its number on a slot that draws none**, so that a
    // digit means the same control on every slot of the chain; the chip then
    // declines and says that the slot's procedure declares no `retains`
    // ([ADR-0352](../../../docs/adr/0352-the-chains-list-is-the-master-bays-items-and-a-slot-is-taken-out-by-a-glyph-on-its-row.md)).
    //
    // **`+ add` is a rung too**, and the second on this panel after `+ lane`:
    // `enter` puts its chooser down and the card's entries — the `kind L5`
    // procedures the library holds — are what is under it (ADR-0351).
    Built {
        bay: MASTER,
        head: &[],
        items: Items::Controls(Of {
            first: &[Control::Out],
            then: Some(Control::Slot),
            last: &[Control::AddEffect],
        }),
        under: &[
            (
                Control::Slot,
                Of {
                    first: &[],
                    then: Some(Control::ChainParam),
                    last: &[Control::ChainCut, Control::ChainRemove],
                },
            ),
            (
                Control::AddEffect,
                Of {
                    first: &[],
                    then: Some(Control::ChainProcedure),
                    last: &[],
                },
            ),
        ],
        act: None,
        selects: false,
        across: false,
    },
    // -- Sequencer ---------------------------------------------------------
    //
    // **`0` is the head**: the grid mode pill, the four bank pills that name
    // which pattern, and `+ lane`, which the record calls the head's act.
    //
    // **Sixteen steps outrun ten digits**, which is the one place the digits
    // fail outright: a lane's controls are its label and its steps, the digits
    // reach the label and the first eight steps, and the rest are walked. **It
    // is also the one bay that uses both axes** — the lanes are a column and a
    // lane's cells are a row — which is `Answers::Cells`.
    //
    // **`+ lane` is a rung as well as a control**, and the only one under a
    // head: `enter` on it puts the chooser down and the card's entries are
    // what is under it, one per target a lane can drive, in the order the card
    // lists them. They are a column, so `↑↓` walk them
    // ([ADR-0351](../../../docs/adr/0351-the-lane-chooser-is-a-rung-of-the-address.md)).
    Built {
        bay: SEQUENCER,
        head: &[
            Control::Mode,
            Control::Bank,
            Control::Bank,
            Control::Bank,
            Control::Bank,
            Control::AddLane,
        ],
        items: Items::Alike(Of {
            last: &[],
            first: &[Control::Label],
            then: Some(Control::Step),
        }),
        under: &[(
            Control::AddLane,
            Of {
                last: &[],
                first: &[],
                then: Some(Control::LaneTarget),
            },
        )],
        // **`enter` on a lane takes that lane out of the pattern**, which is the
        // minus the bay draws at the end of the row reached by the one key that
        // can reach it. The Master chain's minus is the last of a slot's
        // controls and a digit names it
        // ([ADR-0352](../../../docs/adr/0352-the-chains-list-is-the-master-bays-items-and-a-slot-is-taken-out-by-a-glyph-on-its-row.md));
        // a lane draws its label and sixteen cells before its minus, so a
        // suffix here would be the eighteenth control of a rung the digits stop
        // at nine of. An item's own act is the rung above that, and it is the
        // Library row's `enter` read on a lane.
        act: Some(Act::RemoveLane),
        selects: false,
        across: false,
    },
    // -- Outputs -----------------------------------------------------------
    Built {
        bay: OUTPUTS,
        head: &[],
        items: Items::Controls(Of::just(&[Control::Sink])),
        under: &[],
        act: None,
        selects: false,
        across: true,
    },
];

/// The picture, by the name the arrangement gives it — the node the Program
/// bay's `solo` acts on and the Outputs row's one sink turns on and off. A
/// constant for [`MIXER`]'s reason.
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

/// Every (bay, key) pair the grammar binds, flattened — the dispatch table as a
/// check can read it.
///
/// `crates/karakuri/src/main.rs`'s `key_column` holds the *rows* each pair
/// reaches, because a page heading is what a check reads and is not something
/// this program says to anybody; this is the half that says which pairs exist,
/// and the two are held against each other in both directions.
///
/// [`ANY`] is the first pair and is not a bay. `space` at bay level is the
/// fold, and it works in every one of the nine — so it is one route naming all
/// of them rather than nine routes naming one row, which is what a badge
/// reading `space &middot; in any bay` says.
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
