use super::*;

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
