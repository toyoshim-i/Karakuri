use super::*;

/// The nine bays' focus ladder specifications (ADR-0259, ADR-0333, ADR-0343).
pub const BUILT: &[Built] = &[
    // -- Transport ---------------------------------------------------------
    // Headless row (ADR-0159): controls are its items (`Items::Controls`, walked L-to-R).
    // The `tap` capsule remains global (ADR-0259), while `rec`, `learn`, and `map` pills are omitted from traversal.
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
        // Audio-in and arrangement pills open modal cards walked by digits/arrows and dismissed by `esc` (ADR-0350).
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
    // Head `0` controls are scope chips; items are Sets walked by digits/arrows.
    // Row controls are star and `params` (ADR-0333). Row menu cards capture input while open.
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
    // Stateless event list: `space` is inactive below the fold (ADR-0259); row controls are its two acts (`n 1`, `n 2`).
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
    // Three levels deep: panes -> node groups -> authority/renderer chips and parameter rows.
    // Parameter act is "take back", exposed directly on the parameter rung when attached.
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
    // Items are the 4 strips; controls are 5 per strip (ADR-0259).
    // Head holds transition controls (ADR-0333); `go` acts on the addressed strip.
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
    // Items are out fader, chain slots, and `+ add`. Slot controls include parameters, cut chip, and `−` (ADR-0352).
    // Cut chip index is stable even when undrawn. `+ add` opens procedure chooser card (ADR-0351).
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
    // Head `0` holds grid mode, bank pills, and `+ lane`.
    // Lanes use 2D cells (`Answers::Cells`): digits reach label and first 8 steps, arrows walk the rest.
    // `+ lane` opens target chooser card walked vertically (ADR-0351).
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
        // `enter` on a lane removes it from the pattern (ADR-0352), acting as the lane's primary action.
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
