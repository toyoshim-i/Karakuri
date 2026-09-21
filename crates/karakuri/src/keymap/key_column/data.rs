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

/// The digit, declared rather than scanned.
///
/// [`bound`] answers *which keys does this program bind* out of
/// [`super::KEY_BINDINGS`] and [`GRAMMAR_KEYS`] now, and the digit is the one
/// key of the grammar neither carries: `crate::grammar` binds it with a guard
/// rather than a literal, deliberately, because ten literals would say the
/// digits are bound and say nothing about what they reach — a digit reaches a
/// different row in every bay, and the dispatch table is what knows which
/// (ADR-0259, ADR-0333). So this one entry is still conditioned on
/// `karakuri_console::focus::BUILT` rather than asserted outright, in [`bound`]
/// itself.
pub const DIGIT: &str = "digit";

/// How the page spells each key of the grammar, and what a badge naming one
/// resolves to.
///
/// The digits are all one key and both arrow pairs are the arrows, which is
/// [`ROWS`]' own shape: a row is reached by *the arrows in the Mixer*, and
/// which pair depends on whether the thing addressed is an item laid out in a
/// row or a level standing on its own. That is the one thing this table gives
/// up, and it is written down rather than left to be found: a badge naming
/// `&larr;&rarr;` on a row the arrows reach only by stepping a level passes
/// here. What the axis is, is `karakuri_console::focus::Built::across`, and
/// nothing holds the page against it.
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

/// The bay a badge's bay-name resolves to, in the arrangement's own names —
/// which is what `karakuri_console::focus::BUILT` is keyed by.
///
/// The page writes them the way a person says them, with the article the bay's
/// own sentence uses: *in the Mixer*, *in Staging*. Two spellings for nine
/// bays, and the article is the page's rather than something to normalise away.
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

/// How a badge names a press that is addressed in every bay, and what it
/// resolves to.
///
/// `space` at bay level is the fold and `g` is the split enclosing the focused
/// bay: neither is one bay's, and neither is global — the operand is the bay
/// that has focus, which is the third category ADR-0343 names. So the page
/// spells it `&middot; in any bay` and it resolves to
/// `karakuri_console::focus::ANY`, which is not one of [`BAYS`]' nine and is
/// deliberately not in that table: a badge naming one of the nine is a claim
/// about one place, and this is a claim about all of them.
///
/// The reverse check reads it as all nine, which is the stronger reading and
/// the honest one — see
/// [`every_key_route_the_page_marks_built_is_bound_by_the_instrument`].
pub const ANY_BAY: (&str, &str) = ("any bay", karakuri_console::focus::ANY);

/// Every route this program binds, and the rows of [`PAGE`] it reaches.
///
/// `None` for a bay is a global key — one whose meaning does not depend on
/// where the address is, which is the operand rule ADR-0259 closes the global
/// list by. `Some(bay)` is a key of the grammar, addressed to that bay, and the
/// key is spelled as the grammar spells it rather than as a letter.
///
/// # Why the key alone no longer determines a row
///
/// `space` in the Mixer is a residency, a blend mode, a mask shape or a level's
/// default; `space` in the Library is a scope. One key, two bays, six rows — so
/// a mapping keyed on the key alone would either name all six for both bays or
/// name none. That is the whole of why this table grew a column, and it is
/// ADR-0259's own consequence: *"`ROWS` becomes keyed by a (bay, key) pair with
/// the globals under no bay."*
///
/// # What is written down and what is derived
///
/// The rows are written down and cannot be derived: `Op::Fold` folds a bay or a
/// pane depending on what the pointer is over, and only the page separates
/// those two. The pairs are derived — `karakuri_console::focus::reaches`
/// flattens the dispatch table — and
/// [`the_grammar_the_page_names_is_the_grammar_the_console_declares`] holds the
/// two against each other in both directions, so a bay whose grammar is built
/// and has no rows here fails, and a pair here the console does not declare
/// fails.
///
/// The rows are the page's headings byte for byte.
pub const ROWS: &[(Option<&str>, &str, &[&str])] = &[
    // ------------------------------------------------------------------
    // The globals: a letter whose operation has no operand for focus to
    // supply, or whose only operand is the choice the key itself spells.
    // ------------------------------------------------------------------
    // The room's colours. Nothing in the arrangement moves and no
    // `Outcome` says so, which is why it is not an operation.
    (None, "n", &[]),
    (None, "r", &["Reset the arrangement"]),
    // `Op::UnfoldAll` — the page carries the region and the everything
    // under one heading, as `vocabulary.rs` does.
    (None, "z", &["Bring back what is folded"]),
    // **The three that need a room**, and they are the keys here that
    // reach neither the arrangement nor the deck. `b` is a tap and `,`
    // and `.` are the octave; each performs against the audio session
    // this program opened, and each says so when there is none rather
    // than doing nothing (`crate::tapped`, `crate::scaled`).
    (None, "b", &["Tap the beat"]),
    (None, ",", &["Halve or double the grid"]),
    (None, ".", &["Halve or double the grid"]),
    // **The save, whose operand is the deck selection.**
    //
    // **It is the key column and not the panel column that this makes
    // `has`.** The Library bay draws no *keep* control, so the row's panel
    // badge stays `plan` — a key is not a control, and a badge that named
    // one would be a claim about something that is not drawn. **Which is
    // also why it is still a letter**: ADR-0259 makes this *"an act on a
    // control the Library bay does not draw yet"*, and a grammar key
    // cannot be addressed to a control nobody draws.
    (None, "k", &["Keep what a deck is playing"]),
    // **The two that move the address**, and neither names a row: focus is
    // a pointer this console owns and moving one is not an operation
    // (ADR-0332).
    (None, "esc", &[]),
    (None, "tab", &[]),
    // **The three that are live only while a name is being typed.**
    // `enter` and `space` are grammar keys the rest of the time and appear
    // under their bays below; this is the rub-out, which is nothing else.
    //
    // No *letter* names a save or a restore, and this key names neither:
    // it is the rub-out inside a name being typed. The two rows are
    // reached by addressing the pill that lists them, which is the
    // Transport's `enter` below (ADR-0221, ADR-0350).
    (None, "backspace", &[]),
    // ------------------------------------------------------------------
    // Addressed to the focused bay, wherever it is
    // ------------------------------------------------------------------
    // **`space` on a bay is the fold, in every one of the nine**, which is
    // ADR-0259's rule and the narrow reason a folded bay keeps its place
    // in the ring. It is one route naming all nine rather than nine routes
    // naming one row: a badge reading `space &middot; in the Mixer` on
    // *Fold a bay away* would name one of nine places the press works.
    (Some(ANY), "space", &["Fold a bay away"]),
    // **And `g` is the split enclosing the focused bay**, which is a pane
    // every time — a bay's parent is a split and never another bay. It is
    // the one route to this row from the keyboard alone, and it is why the
    // letter survived `f` (ADR-0343).
    (Some(ANY), "g", &["Fold a pane away"]),
    // ------------------------------------------------------------------
    // The Transport's grammar
    // ------------------------------------------------------------------
    // A headless row, so `0` names the row itself and a digit names one of
    // the controls left to right. Naming one asks for nothing — and a
    // digit under one of the two pills names a row of the card it put
    // down, which asks for nothing either.
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
    // `enter` puts one of the two cards down and runs a row of it. The
    // audio-in pill's card is the machine's inputs and a row of it
    // attaches one; the arrangement pill's menu is *save*, *start a new
    // one* and the names filed, and a row of it saves under the name in
    // use — asking for one where there is none — or puts a filed
    // arrangement back. The reset is not here: `r` reaches that row from
    // anywhere, so the menu draws *start a new one* and the grammar
    // declines on it (ADR-0350).
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
    // **A digit names the nth row and `0` the head**, and neither asks for
    // an operation: the cursor is a pointer nothing in the vocabulary
    // moves, which is `console.html`'s *How a Set reaches a deck*.
    (Some("library"), DIGIT, &[]),
    // The rows, walked — today's `up` and `down`, and the same nothing.
    (Some("library"), "arrows", &[]),
    // **`space` on the head's scope chips and on a row's star.** The chips
    // are drawn by `karakuri-console` and pressed by nobody: `SelectScope`
    // is emitted from `Readout::pointer` and never from a control, so that row's
    // panel column stays `plan` and this key is what makes its key column
    // `has`.
    (
        Some("library"),
        "space",
        &[
            "Choose which scope the library shows",
            "Star a Set, or take the star off",
        ],
    ),
    // **`enter` on a row is the load**, with both operands on screen
    // before the press — the deck selection says which deck and the
    // address says which Set. **`enter` on a row's `params` chip opens
    // what that Set holds and declares**, which is the second act a row
    // has and the reason a row's controls are numbered at all.
    //
    // **One key, and a preset row reaches a second page row through it.**
    // Taking a Set in is not a row of its own — ADR-0229's *one operation,
    // two moments* — so a press on a `presets` row performs *Send a Set to
    // somebody, and take one in* at the moment of the press and then the
    // load. That row's key badge names no key: what an operator reaches
    // from the keyboard is a **load**.
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
    // **Nothing here has a state at all**, so this bay has no `space`
    // route below the fold — ADR-0259's own finding, and the second of the
    // two bays that are lists of things that happened rather than things
    // you set.
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
    // Three deep, and the deepest bay on the panel: `1 2 3` is the first
    // pane's second thing's third control.
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
    // ------------------------------------------------------------------
    // The Mixer's grammar
    // ------------------------------------------------------------------
    // **A digit names the nth strip, and naming a strip is the deck
    // selection** — which is why that row keeps a key badge rather than
    // losing one.
    (Some("mixer"), DIGIT, &["Select a deck"]),
    // **The arrows walk the strips and step the two levels**, which is the
    // one entry where the same key reaches a row two ways: `&larr;&rarr;`
    // on the row of strips is the selection, and `&uarr;&darr;` on an
    // addressed trim or fader is a tenth. See [`SPELLED`] for what that
    // costs the check.
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
    // **`enter` on the head's `go` capsule runs the transition on the
    // addressed strip**, which is the deck selection: this deck is covered
    // and the next one round arrives over it. **Only the wipe**, because
    // the row draws one capsule and `Operation::Wipe` is what it asks for
    // — the fade and the crossfade are drawn nowhere on this bay and are
    // reached by no key: a scheduled move is not authored on a real-time
    // surface, so both rows read `gap` in the panel column and in the key
    // column (ADR-0353).
    (Some("mixer"), "enter", &["Wipe the next deck in"]),
    (Some("mixer"), "m", &["Mute a deck"]),
    (Some("mixer"), "s", &["Solo a deck"]),
    (Some("mixer"), "u", &["Clear solo"]),
    // ------------------------------------------------------------------
    // The Master chain's grammar
    // ------------------------------------------------------------------
    // The bay's items are the out fader, the chain's slots and `+ add`,
    // and a slot is a rung: a digit names one of its parameter rows, its
    // cut chip or its `−`. Naming is not an operation, so the digit
    // reaches no row.
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
    // **Sixteen steps outrun ten digits**, so the digits reach a lane's
    // label and the first eight of its cells and the rest are walked —
    // which ADR-0259 calls *"honest and very nearly useless"*. Walking is
    // not an operation, so neither key names a row.
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
    // `enter` on `+ lane` puts the chooser down and `enter` on one of its
    // entries points the lane at that target, which is the one row this
    // bay's head reaches: the digits and `↑↓` name what is on the card and
    // `esc` takes it away (ADR-0351).
    //
    // **And `enter` on a lane takes that lane out**, which is the minus at
    // the end of its row: a lane draws its label and sixteen cells before
    // that glyph, so the digits stop short of it and an item's own act is
    // the rung that reaches it (ADR-0352 read on a row the digits outrun).
    (
        Some("sequencer"),
        "enter",
        &["Point a lane at what it drives", "Remove a lane"],
    ),
    // ------------------------------------------------------------------
    // The Outputs row's grammar
    // ------------------------------------------------------------------
    // **The simplest of the nine**: items are the sinks, each has exactly
    // one state, and `space` is the whole of it. The picture's on and off
    // is one operation and one fold, which is `Readout::sink`.
    (Some("outputs"), DIGIT, &[]),
    (Some("outputs"), "arrows", &[]),
    (Some("outputs"), "space", &["Choose where the frame goes"]),
];

/// Routes that reach no row in [`ROWS`], recorded in checked evaluation order.
pub const NO_ROW: &[(Option<&str>, &str)] = &[
    (None, "n"),
    (None, "esc"),
    (None, "tab"),
    (None, "backspace"),
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
