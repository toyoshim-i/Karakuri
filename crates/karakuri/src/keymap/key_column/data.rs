use karakuri_console::focus::ANY;

pub const PAGE: &str = "docs/manual/operations.html";

/// What marks a row on the page — the marker `panel_column.rs`, `vocabulary.rs`
/// and `mcp.rs` all match, for the reason the first of them gives: sections are
/// `<h2>` and a heading somebody adds for looks is neither.
pub const ROW: &str = r#"<div class="op-head">"#;

/// The badge text of a route that names nothing. A `plan` or `gap` badge is
/// allowed to be this; a `has` badge is not, because it would claim an operator
/// reaches the operation and decline to say what to press.
pub const NOWHERE: &str = "&mdash;";

/// Digit key token representing numeric selection in grammar guards (ADR-0259, ADR-0333).
pub const DIGIT: &str = "digit";

/// Mapping from manual HTML key spelling to internal key category name.
pub const SPELLED: &[(&str, &str)] = &[
    ("&uarr;&darr;", "arrows"),
    ("&larr;&rarr;", "arrows"),
    ("up", "arrows"),
    ("down", "arrows"),
    ("left", "arrows"),
    ("right", "arrows"),
    ("space", "space"),
    ("enter", "enter"),
];

/// How a badge names the bay a press is addressed in, and the whole of the
/// grammar this column's designed half is written in: a key, a separator and a
/// bay. `space &middot; in the Mixer`.
pub const IN_THE: &str = " &middot; in ";

/// Mapping from manual page bay names (including articles) to console bay IDs.
pub const BAYS: &[(&str, &str)] = &[
    ("the Transport", "transport"),
    ("the Library", "library"),
    ("Staging", "staging"),
    ("the Program", "program"),
    ("the Inspector", "inspector"),
    ("the Mixer", "mixer"),
    ("the Master", "master"),
    ("the Sequencer", "sequencer"),
    ("the Outputs", "outputs"),
];

/// Manual badge name and identifier for actions addressed across any focused bay (ADR-0343).
pub const ANY_BAY: (&str, &str) = ("any bay", karakuri_console::focus::ANY);

/// Mapping from (optional bay, key) route to reached manual operation row titles (ADR-0259).
pub const ROWS: &[(Option<&str>, &str, &[&str])] = &[
    // Focus navigation keys do not correspond to manual operation rows (ADR-0332).
    (None, "esc", &[]),
    (None, "tab", &[]),
    // Rub-out key active during text entry (ADR-0221, ADR-0350).
    (None, "backspace", &[]),
    // Space folds/unfolds any focused bay across all nine bays (ADR-0259).
    (Some(ANY), "space", &["Fold a bay away"]),
    // **And `g` is the split enclosing the focused bay**, which is a pane
    // every time — a bay's parent is a split and never another bay. It is
    // the one route to this row from the keyboard alone, and it is why the
    // letter survived `f` (ADR-0343).
    (Some(ANY), "g", &["Fold a pane away"]),
    // `Op::UnfoldAll` — the page carries the region and the everything
    // under one heading, as `vocabulary.rs` does.
    (Some(ANY), "z", &["Bring back what is folded"]),
    // ------------------------------------------------------------------
    // The Transport's grammar
    // ------------------------------------------------------------------
    // **The beat, tapped.**
    (Some("transport"), "b", &["Tap the beat"]),
    // **The grid, an octave either way.**
    (Some("transport"), ",", &["Halve or double the grid"]),
    (Some("transport"), ".", &["Halve or double the grid"]),
    // **Reset the arrangement back to default.**
    (Some("transport"), "r", &["Reset the arrangement"]),
    // **Room theme cycle.**
    (Some("transport"), "n", &[]),
    // Transport digit navigation addresses controls without invoking operations.
    (Some("transport"), DIGIT, &[]),
    // `↑↓` on the tempo figure, on the exposure and on the offset, which
    // are this row's three continua. The tempo steps by one beat a minute
    // and the arrows walk the rows of a card that is down (ADR-0350).
    (
        Some("transport"),
        "arrows",
        &[
            "Set the free-run tempo",
            "Exposure",
            "Nudge the latency offset",
        ],
    ),
    // **`space` on the tone map cycles the four operators**, and on the
    // exposure it is the value the control was declared at. The tempo
    // figure is not here: it is a track with no value it was declared at,
    // so there is nothing for `space` to return it to.
    (Some("transport"), "space", &["Tone map", "Exposure"]),
    // Enter attaches audio input or executes arrangement save/restore options (ADR-0350).
    (
        Some("transport"),
        "enter",
        &[
            "Attach a beat source",
            "Save the arrangement",
            "Put a saved arrangement back",
        ],
    ),
    // ------------------------------------------------------------------
    // The Library's grammar
    // ------------------------------------------------------------------
    // Library digit keys move cursor without invoking operations.
    (Some("library"), DIGIT, &[]),
    // The rows, walked — today's `up` and `down`, and the same nothing.
    (Some("library"), "arrows", &[]),
    // Space toggles library scope filter or stars/unstars a Set row.
    (
        Some("library"),
        "space",
        &[
            "Choose which scope the library shows",
            "Star a Set, or take the star off",
        ],
    ),
    // Enter loads a Set into selected deck or opens its parameter declaration (ADR-0229).
    (
        Some("library"),
        "enter",
        &[
            "Load material into a deck",
            "Read what one Set holds and declares",
        ],
    ),
    // ------------------------------------------------------------------
    // The Staging lane's grammar
    // ------------------------------------------------------------------
    // Staging lane items are stateless log entries; navigation produces no operations (ADR-0259).
    (Some("staging"), DIGIT, &[]),
    (Some("staging"), "arrows", &[]),
    // **A row's two acts are two controls and a digit chooses between
    // them**, which is the record's `n 1` and `n 2`.
    (
        Some("staging"),
        "enter",
        &["Keep a candidate", "Put a node's previous version back"],
    ),
    // ------------------------------------------------------------------
    // The Program bay's grammar
    // ------------------------------------------------------------------
    // Program bay cells do not bind digits or arrow keys.
    (Some("program"), DIGIT, &[]),
    (Some("program"), "arrows", &[]),
    // **`space` on the head's `solo`**, which is `s` and `u` collapsed
    // into the one control they always described (ADR-0259). The class
    // pill beside it opens a class rather than cycling a state, and it
    // reaches no row of this page.
    (Some("program"), "space", &["Solo a region"]),
    // ------------------------------------------------------------------
    // The Inspector's grammar
    // ------------------------------------------------------------------
    // Inspector numeric keys navigate panel controls hierarchically.
    (Some("inspector"), DIGIT, &[]),
    // **`↑↓` on the anchor scrub a quarter beat, and on a parameter row
    // write it** — a tenth of what the control publishes.
    (
        Some("inspector"),
        "arrows",
        &["Scrub a deck a quarter beat", "Write a parameter"],
    ),
    // **`space` on the four chips**, each naming the state it arrives at
    // rather than a flip, which is the chips' own rule (P-0090).
    (
        Some("inspector"),
        "space",
        &[
            "Set a deck's sync mode",
            "Composite a deck's renderers",
            "Choose which renderer of a deck is live",
            "Set a node's authority",
        ],
    ),
    // **`enter` on a parameter row takes the attachment back**, which is
    // the act of the control the row draws: a parameter with nothing
    // holding it draws no sensitivity row at all.
    (Some("inspector"), "enter", &["Take a parameter back"]),
    // **Keep what the selected deck is playing.**
    (Some("inspector"), "k", &["Keep what a deck is playing"]),
    // ------------------------------------------------------------------
    // The Mixer's grammar
    // ------------------------------------------------------------------
    // Mixer digit keys select the nth deck strip.
    (Some("mixer"), DIGIT, &["Select a deck"]),
    // Arrows navigate between strips horizontally and adjust gain/opacity levels vertically.
    (
        Some("mixer"),
        "arrows",
        &["Select a deck", "Gain", "Opacity"],
    ),
    // **`space` is the whole of this bay's five controls and three of its
    // head's.** The transition row is the head's (ADR-0343): the settings
    // decide what the next move means wherever it lands, which is what a
    // head is for.
    (
        Some("mixer"),
        "space",
        &[
            "Put a deck on air, prime it, or take it off",
            "Gain",
            "Opacity",
            "Blend mode",
            "Set a deck's mask shape",
            "Choose the wipe shape, the quantum, the length",
        ],
    ),
    // Enter on the go capsule executes a wipe transition on the selected deck (ADR-0353).
    (Some("mixer"), "enter", &["Wipe the next deck in"]),
    (Some("mixer"), "m", &["Mute a deck"]),
    (Some("mixer"), "s", &["Solo a deck"]),
    (Some("mixer"), "u", &["Clear solo"]),
    // ------------------------------------------------------------------
    // The Master chain's grammar
    // ------------------------------------------------------------------
    // Master bay digit navigation addresses slots/parameters without invoking operations.
    (Some("master"), DIGIT, &[]),
    // The arrows step the two levels: the out fader, and a parameter row
    // of a slot by a tenth of the range its procedure declares. They also
    // walk the `+ add` chooser's entries, which is a move rather than an
    // operation.
    (
        Some("master"),
        "arrows",
        &["Master out", "Set a chain effect's parameter"],
    ),
    // `space` returns each of those levels to its default and cycles a
    // slot's cut chip, which is the one row that is both: a parameter and
    // a cut are the two things `Operation::SetChainParam` sets (ADR-0348).
    (
        Some("master"),
        "space",
        &["Master out", "Set a chain effect's parameter"],
    ),
    // `enter` on `+ add` puts the chooser down and `enter` on one of its
    // entries appends a slot of that procedure; `enter` on a slot's `−`
    // takes that slot out (ADR-0352).
    (
        Some("master"),
        "enter",
        &[
            "Add an effect to the master chain",
            "Remove an effect from the master chain",
        ],
    ),
    // ------------------------------------------------------------------
    // The Sequencer's grammar
    // ------------------------------------------------------------------
    // Sequencer digit and arrow navigation addresses lane steps without invoking operations (ADR-0259).
    (Some("sequencer"), DIGIT, &[]),
    (Some("sequencer"), "arrows", &[]),
    // `space` is four of this bay's six controls: the head's mode and
    // bank pills, a lane's label and a lane's cells, each named as the
    // state it arrives at. `+ lane` and a lane's minus are not here — they
    // perform rather than set, so they are `enter`'s.
    (
        Some("sequencer"),
        "space",
        &[
            "Choose what a step is worth",
            "Choose which pattern the sequencer plays",
            "Mute a lane",
            "Toggle a step",
        ],
    ),
    // Enter binds target parameter on new lanes or removes an existing lane (ADR-0351, ADR-0352).
    (
        Some("sequencer"),
        "enter",
        &["Point a lane at what it drives", "Remove a lane"],
    ),
    // ------------------------------------------------------------------
    // The Outputs row's grammar
    // ------------------------------------------------------------------
    // Outputs bay items toggle sink routing via space.
    (Some("outputs"), DIGIT, &[]),
    (Some("outputs"), "arrows", &[]),
    (Some("outputs"), "space", &["Choose where the frame goes"]),
];

/// Routes that reach no row in [`ROWS`], recorded in checked evaluation order.
pub const NO_ROW: &[(Option<&str>, &str)] = &[
    (None, "esc"),
    (None, "tab"),
    (None, "backspace"),
    (Some("transport"), "n"),
    (Some("transport"), DIGIT),
    (Some("library"), DIGIT),
    (Some("library"), "arrows"),
    (Some("staging"), DIGIT),
    (Some("staging"), "arrows"),
    (Some("program"), DIGIT),
    (Some("program"), "arrows"),
    (Some("inspector"), DIGIT),
    (Some("master"), DIGIT),
    (Some("sequencer"), DIGIT),
    (Some("sequencer"), "arrows"),
    (Some("outputs"), DIGIT),
    (Some("outputs"), "arrows"),
];
