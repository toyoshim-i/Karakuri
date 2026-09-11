//! **Every control `karakuri-console` claims, against the presses this
//! window answers.**
//!
//! The console draws a control and turns a press on it into an operation
//! or into an intent; [`Readout::pointer`] has to ask each control and act
//! on what comes back. **Nothing checked that it did.** Between `74719c3`
//! and `094804c` the transition row offered three setting pills,
//! `karakuri_console::input::claim` claimed presses on them, and this
//! file's press handler had no branch for the row at all — while
//! `docs/manual/operations.html` marked the operation `has`, which
//! [ADR-0213](../../../docs/adr/0213-the-interface-milestones-meter-is-the-panel-column-and-has-means-an-operator-reaches-it.md)
//! defines as *an operator running the instrument reaches it*. Every test
//! in the workspace was green for the whole of that period.
//!
//! `karakuri-console/tests/panel_column.rs` says at its head why it could
//! not have caught that: it checks *"the necessary half and not the
//! sufficient one"*, because *"a control written here and never wired
//! there would pass, and the badge would be a lie the page tells on its
//! own authority"*. **This is the other half**, and it is a seam rather
//! than a badge — it says nothing about the page and everything about the
//! wire between the crate that draws a control and the binary that
//! performs it.
//!
//! # What a control is, and where the list of them lives
//!
//! **In `karakuri-console`, as a value.** `input::PROBES` is that crate's
//! own register of what rule 4 hit-tests: one row per derivation, naming
//! it, counting the controls a pointer reaches through it, and carrying
//! the probe. So [`ASKED`] is one entry per row of it, in that order, and
//! the array is `PROBES.len()` long — **a control added to that crate
//! arrives here as a compile error**, which is the direction this module
//! was written for and the direction it used to answer by scanning.
//!
//! **It did scan, until 2026-09-07.** There was no list in
//! `karakuri-console` of what its controls offer, and a list written here
//! would have been the second copy of one
//! ([`docs/contributing.md` §4](../../../docs/contributing.md)) — so this
//! module read that crate's source instead, took every indented `pub fn`
//! inside an `impl` whose parameters named a `Point` and whose return type
//! was not `bool`, and required a four-column table entry for each. A table
//! in that crate is the first copy, so the scan and the four columns went
//! with it ([ADR-0274](../../../docs/adr/0274-a-control-is-a-row-in-the-consoles-own-table.md)).
//!
//! **What an entry needs is now three columns and not four.** The old ones
//! were forced by the scan: method names are not unique — `owns` was on
//! seven types, `hit` on five, `grab` on three — and every receiver in the
//! press handler is a local called `bay`, `row`, `pill` or `head`, so an
//! entry had to name the type, the method, the derivation *and* the local
//! to be identifiable at all. A row of `PROBES` is a value with a name, so
//! what is left to write down is only how this file reaches it: the
//! derivation it calls, and the calls that ask it. **The derivation is
//! still a column** because two rows can be asked through one local —
//! `pill.ask(` is both cards' — and it is what tells those two apart.
//!
//! # Why the check is here and can be nowhere else
//!
//! [`key_column`]'s reason, unchanged: the press handler is in this file,
//! **nothing in this workspace may depend on this package** — it is a
//! binary with no library target on purpose, as the crate header says — so
//! there is no crate that can see both halves of the seam except this one.
//! It cannot even be an integration test under `crates/karakuri/tests/`,
//! because a package with no library target has nothing for one to `use`.
//!
//! **Below `mod tests`, and that is not a matter of taste.** The tests
//! under this file's first `#[cfg(test)]` call these hit tests
//! **directly**, bypassing the press handler — `bay.tally(…)`,
//! `bay.mask(…)`, the transition row's `go` and more.
//!
//! **Read as though it were the handler, that region satisfies two of
//! [`ASKED`]'s entries: the mixer strip's and the transition row's** —
//! wrongly, and only in part. Counted by taking this file from its first
//! `#[cfg(test)]` line to the end, flattening it the way [`code`] flattens
//! what is above that line, and putting
//! [`every_control_in_the_table_is_asked_by_the_press_handler`]'s rule
//! over each entry, no entry is satisfied whole: the region derives
//! `mixer_bay(` and asks `bay.tally(` and `bay.mask(` after it, and it
//! derives `transition_row(` and asks nothing of it the rule can see —
//! `rustfmt` wraps that call over two lines, so the flattened text reads
//! `row .go(` where the rule's spelling is `row.go(`.
//!
//! **Passing is not what the bounds are for.** Two of the console's
//! controls could stop being asked by this window and this check would
//! still call them wired, because a test would be answering for the
//! handler. One control that stopped being asked is the defect this module
//! was written for. **Two bounds keep the region out, and either would do
//! it alone today** — [`code`] stops at the first `#[cfg(test)]`, and
//! [`body`] then cuts one function out of what is left — and each was
//! measured against the defect with the other taken away.
//!
//! The same trap has been sprung once already from the other side, where a
//! test-only item placed *above* the window loop moved [`key_column`]'s
//! stop line past every key arm and left both of its `bound`-built checks
//! passing over an empty set — back when `bound` read this file's text for
//! the window loop's literal keys, before they moved into
//! `crate::keymap::KEY_BINDINGS`. [`shipped`] and [`checked`] carry that account,
//! and they sit below `mod tests` for it, and for [`code`]'s share of the
//! same risk today.
//!
//! # What it cannot see, and which way each one fails
//!
//! - **A call after an early return reads as wired.** The press handler
//!   takes four early returns — the two cards, which are drawn *over* the
//!   bays — before the look controls are asked, and this file reads text
//!   rather than pressing anything, so an ask stranded behind one of them
//!   is indistinguishable here from an ask that runs. It is a *false
//!   negative*, and it is not closeable by reading: the returns are
//!   conditional, which is the shape of the handler and not a defect in it.
//!   What answers it is a press on a running panel, which is `mod gpu`'s.
//! - **Asking is not applying.** That `TransitionRow::go` is asked says
//!   nothing about the deck moving afterwards.
//!   `panel_column.rs`'s `UNREACHABLE` records the one time those came
//!   apart: `SetSync` was emitted, claimed, printed, and the deck did not
//!   move, for a release. A *false negative* again, and `mod gpu`'s
//!   press-to-deck tests are what stand under it.
//! - **A second control inside a derivation already rowed.** A sixth chip
//!   on a strip is one more thing a press reaches and one more call this
//!   handler owes, and neither the row's count nor this entry's list rises
//!   on its own. That is the same blind spot `input::PROBES` documents on
//!   its own side, and it is why a control's clearance test is still what
//!   every new one owes. The scan this module used to run saw a *new
//!   method* where this sees a new row, so it would have caught that one
//!   and could not catch a control added to a method already tabled;
//!   neither reading is the whole, and this one costs no directory listing.
//! - **Neither cut handles `/* … */`, and neither handles a `//` inside a
//!   string.** That one **is** closed, by refusal rather than by parsing:
//!   [`the_two_cuts_are_the_whole_of_the_comment_syntax_they_meet`] fails
//!   on either, naming the line, so the day somebody writes one the scan
//!   says it has stopped being able to read rather than reading wrongly.
//!   `karakuri-engine/tests/gpu_tests_are_under_mod_gpu.rs`'s
//!   `blank_comments_and_strings` is the workspace's answer where the
//!   syntax genuinely has to be read; it is a hundred lines in another
//!   package's integration test, which no target here can `use`, and
//!   copying it would buy nothing this refusal does not — this file
//!   contains neither today, and it is one this repository owns.
//! - **Only the seam.** Whether the operation a control hands back is the
//!   right one is `panel_column.rs`'s and `vocabulary.rs`'s; whether a key
//!   reaches it is [`key_column`]'s; whether the page's badge is honest is
//!   both of those. This file asks one question: is every control the
//!   console draws asked by the window that draws it.

use super::*;
use karakuri_console::input::PROBES;

/// **Every row of `karakuri_console::input::PROBES`, and how this file
/// reaches what it claims.**
///
/// The row's name, **the derivation this file calls to get the receiver**,
/// and every call the press handler makes on that receiver for the
/// controls the row covers. One entry per row and in the console's own
/// order, which is what
/// [`the_table_is_the_consoles_own_rows_in_the_consoles_own_order`] holds
/// it to — so an entry cannot quietly answer for its neighbour.
///
/// **The array is `PROBES.len()` long, and that is the whole of the
/// backward direction.** A control the console starts claiming and this
/// window never asks used to be found by reading that crate's source for
/// `pub fn`s taking a `Point`; it is now a row in a value, and a row with
/// no entry here does not compile.
///
/// **The asks are written down rather than derived**, and they have to be:
/// nothing in a row's name says `mixer_bay`, and the derivations are a
/// deliberate arrangement rather than a convention — a bay is derived
/// *once* and asked six times, because six derivations of one laid-out
/// strip would be six answers.
///
/// **Three entries name calls made on a button *up* rather than a press.**
/// A carry names its deck by where it is let go, so the mixer bay and the
/// Program bay are laid out at the release and asked which rectangle the
/// pointer is over — `bay.dropped(` and `cells.dropped(`. This module
/// reads text rather than pressing anything, so what it checks is the same
/// seam it checks for every other call, and which arm of the press handler
/// makes it is not something it can see. `mod gpu`'s press-to-deck tests
/// are what stand under that.
///
/// **The preview cells' entry is that release and nothing else**, because
/// a press on a cell asks for nothing: ADR-0240 retired `SetPreview`, and
/// `ProgramBay::cell`'s own documentation still says *"A press on a cell
/// asks for nothing, and that is the decision rather than a gap."* The
/// cells are claimed all the same, because they are drawn over a boundary
/// and `input::claim` has to keep a press on them off `egui`
/// ([ADR-0273](../../../docs/adr/0273-the-carry-lands-on-two-sets-of-rectangles-and-wears-a-face.md)).
const ASKED: [(&str, &str, &[&str]); PROBES.len()] = [
    // **The Outputs row's sinks**, and the two calls are one control: the
    // row answers which chip the point is on, and then how this surface
    // carries out what that chip asked for.
    //
    // **`row.hit(` was the first of the two until 2026-09-09**, when the
    // row stopped drawing one sink and started drawing the list it has
    // (ADR-0324). `chip_at` is the same question over every chip, and it
    // is what `input::claim` asks as well — one derivation, so a chip that
    // lights under the pointer is a chip a press reaches. `row.op(` stays
    // because the program view's on and off is still the fold: the
    // operation names the output and the fold is how the console performs
    // it, which is one fact rather than two.
    (
        "the Outputs row's sinks",
        "outputs(",
        &["row.chip_at(", "row.op("],
    ),
    // **The two cards, and they are the only two whose order matters**:
    // each draws over the bays, so while one is down a press inside it
    // belongs to the card. `ask` is the whole of each pill's offer — the
    // press on the capsule, the press on a row, and the dismissal — which
    // is why neither names `item`, the sub-question each `ask` composes.
    // They are asked through one local, so it is the derivation above each
    // that tells the two entries apart.
    ("the audio-in pill", "audio_in_pill(", &["pill.ask("]),
    // **The tracker group's three, derived once for all of them**, and
    // the first row added to the console's table since it landed. The
    // offset's figure is as wide as the number in it and the octave is
    // laid out from where the tap ends, so a second derivation would put
    // the chip a press lands on somewhere the mark is not.
    (
        "the tracker group's three",
        "tracker_group(",
        &["group.tapped(", "group.octave(", "group.nudge("],
    ),
    // **The `learn` pill**, and its one call is `hit` rather than an
    // `ask`: what a press names is a `bool` and not an operation, because
    // a learn edits the *map* rather than being a member of the vocabulary
    // the map addresses (ADR-0236, ADR-0336). `pill.next(` is the state it
    // names and is read at the press.
    (
        "the transport row's learn pill",
        "view::learn_pill(",
        &["pill.hit(", "pill.next("],
    ),
    // **The `map` pill is a readout, and it is derived all the same.**
    // `input::claim` already keeps a press on it off `egui`, so the press
    // handler could have let it fall through — and then this table would
    // have carried an entry with no derivation and no ask, which passes
    // vacuously. So the press is recognised at the control and swallowed
    // there, which is a fact a reader can check. There is no ask because
    // there is nothing to ask: reaching a different map while running is
    // not built, and the pill says which file is loaded.
    (
        "the transport row's map pill",
        "view::map_pill(",
        &["row.pill.contains("],
    ),
    ("the arrangement pill", "arrangement_pill(", &["pill.ask("]),
    // **The two look controls, derived once for both** — the exposure
    // track's place is measured from the tone map capsule's, so they are
    // two questions about one laid-out group.
    (
        "the look group's two",
        "look_row(",
        &["row.tonemap(", "row.exposure("],
    ),
    // **The `rec` pill at the end of that row**, and the derivation is the
    // *row* rather than a pill of its own: the capsule takes the row's
    // right padding and the health capsule and the frame readout are laid
    // out backwards from it, so there is one derivation of that end and
    // this entry names it. One call, because what a press means is the
    // pill's own state and `record` reads it — a toggle is one control and
    // not two.
    (
        "the transport row's rec pill",
        "transport_row(",
        &["row.record("],
    ),
    // **The tempo figure at the head of that row**, and it is the `rec`
    // pill's entry one item along: the same derivation, because the figure
    // is the row's first readout and the control is that reading. One call
    // — where along the number the press landed *is* the tempo, and
    // `tempo` is where the band that refuses a mis-click sits.
    (
        "the transport row's tempo figure",
        "transport_row(",
        &["row.tempo("],
    ),
    // **The Mixer bay's five, derived once and asked five times on a
    // press**, plus the drop at the release. `select` is asked last and is
    // the strip itself — what none of the other four claimed.
    (
        "a mixer strip's five",
        "mixer_bay(",
        &[
            "bay.grab(",
            "bay.blend(",
            "bay.tally(",
            "bay.mask(",
            "bay.select(",
            "bay.dropped(",
        ],
    ),
    // **The transition row's four, and the reason this module exists.**
    // These are the controls that were drawn, claimed and never asked
    // between `74719c3` and `094804c`. Deleting this row's branch from the
    // press handler is the injection this module was watched to fail
    // against; deleting `go`'s `match` alone, and leaving the other three
    // wired, is the second.
    (
        "the transition row's four",
        "transition_row(",
        &["row.shape(", "row.quantum(", "row.length(", "row.go("],
    ),
    // **The Master bay's five**, and they are the Mixer bay's `grab` and
    // `blend` on a different type bound to a different local — which is why
    // the local is part of what is written down and the method alone is
    // not. Two calls for five controls: `grab` answers for the out knob and
    // the three effect rows' knobs, and `chip` for the feedback row's cut.
    (
        "the Master bay's five",
        "master_row(",
        &["row.grab(", "row.chip("],
    ),
    // **The Inspector pane heads' name**, and it is the capsule's
    // arrangement at the other end of the same row: the pane is derived
    // per index and the run from the pane, so the derivation named here is
    // the inner one. The press emits nothing — it opens the field, and the
    // operation is the commit's, at Return.
    (
        "the Inspector pane heads' name",
        "deck_name(",
        &["named.hit("],
    ),
    // **The Inspector pane heads' `keep`**, and it is the deck head's
    // arrangement one row up: the pane is derived per index and the
    // capsule from the pane, so the derivation named here is the inner
    // one. What the press hands back is an operation; the write itself is
    // at the call site, because a disk write is not a thing to do on a
    // frame.
    (
        "the Inspector pane heads' keep",
        "keep_pill(",
        &["pill.keep("],
    ),
    // **The `▾` beside it and the card it puts down**, which is the `uses`
    // line's arrangement one row up: the pick is asked before the mark,
    // because a press inside the card belongs to the card and the mark the
    // card came out of is above it. Both go through the same derivation on
    // the view, so the mark that is painted is the mark a press lands on.
    (
        "the Inspector pane heads' deck pulldown",
        "pane_pulldown(",
        &["target.hit(", ".picked("],
    ),
    // **The deck head's seven, one Inspector pane at a time.** The pane is
    // derived per index and the head from the pane, so the derivation
    // named here is the inner one: `inspector_pane` answers a rectangle
    // and `deck_head_row` answers the control. **Seven controls and six
    // calls**: the scrub's two arrows are two of the seven and `scrub`
    // answers for both of them. The last three are the row's build chips
    // and each re-aims the slot rather than moving the mix, which is why
    // the arms that act on them are beside the library load's and not
    // beside the sync chip's (ADR-0314, ADR-0328).
    (
        "a deck head's seven",
        "deck_head_row(",
        &[
            "head.sync(",
            "head.reanchor(",
            "head.scrub(",
            "head.resized(",
            "head.re_salted(",
            "head.compositing(",
        ],
    ),
    // **The renderer chips**, and the pane is the whole derivation: which
    // group and which chip are inside `select_renderer`, which is the
    // `params` chip's arrangement in the Library bay. So the derivation
    // named here is the *outer* one, unlike the two rows above it.
    (
        "the renderer chips",
        "inspector_pane(",
        &["at_pane.select_renderer("],
    ),
    // **A parameter row's fader, one Inspector pane at a time**, and it is
    // the Master bay's `grab` on a third type bound to a third local: the
    // pane is derived per index and the fader from the pane, so the
    // derivation named here is the outer one, as the chips' above is.
    (
        "a parameter row's fader",
        "inspector_pane(",
        &["at_pane.grab("],
    ),
    // **A parameter row's publish mark**, which is the leftmost cell of the
    // row the fader is on and is derived from the same laid-out pane: one
    // press arm, one call, and what it asks for is the whole interface
    // rather than this entry (ADR-0329).
    (
        "a parameter row's publish mark",
        "inspector_pane(",
        &["at_pane.publishing("],
    ),
    // **A node group's `uses` capsule and its card**, and the two are one
    // row for the Library bay's load button and deck pulldown's reason:
    // they are one derivation and one ask. The capsule emits nothing — it
    // puts the card down — and the card's rows emit `WireInput`, which is
    // why `Readout::wiring_at` is the name here rather than either half.
    (
        "a node group's `uses` capsule and its card",
        "inspector_pane(",
        &["laid.uses_chip(", "laid.wired("],
    ),
    // **A node head's three `man / sug / auto` chips**, and the pane is
    // the whole derivation for the renderer chips' reason two rows up:
    // which group, which head and which chip are inside
    // `InspectorPane::set_authority`.
    (
        "a node head's three authority chips",
        "inspector_pane(",
        &["at_pane.set_authority("],
    ),
    // **The `keep` capsule at the right of the same head**, and the pane is
    // the whole derivation for the chips' reason: which group and where
    // the capsule is are inside `InspectorPane::keep_procedure`. The two
    // heads that carry none answer `None` there rather than being checked
    // here (ADR-0338, decision 4).
    (
        "a node head's keep capsule",
        "inspector_pane(",
        &["at_pane.keep_procedure("],
    ),
    // **The sensitivity row's curve chip and `take back`**, the two of its
    // four chips that are controls — the other two are readouts and answer
    // `None`, which is `view::SensChip`'s decision and not this file's.
    // One derivation for both, as the authority chips' row above is.
    (
        "a sensitivity row's curve and take back",
        "inspector_pane(",
        &["at_pane.sensitivity("],
    ),
    // **The Program bay head's `solo`**, and the two calls are the Outputs
    // sink's arrangement one bay over: the head answers whether the point
    // is on the capsule, and then which of the two operations it is.
    (
        "the Program bay head's solo",
        "program_head(",
        &["head.hit(", "head.op("],
    ),
    // **The grip in each bay head**, the capsule above's neighbour in the
    // same head: one derivation asked once per bay, because four heads
    // cannot be one laid-out box. A pane has no row here and needs none —
    // it folds by its own boundary, which rule 3 claims (ADR-0300).
    (
        "the grip in a bay head",
        "bay_grip(",
        &["grip.hit(", "grip.op("],
    ),
    // **The four deck preview cells**, asked at the release alone — see
    // the head of this table.
    (
        "the deck preview cells",
        "program_bay(",
        &["cells.dropped("],
    ),
    // **The Library bay's five**, and the bay is derived once per question
    // rather than held across them, exactly as `input::claim` derives it
    // five times: a value held across all five would outlive the question
    // it answers. So one derivation is named by five entries, and each is
    // told apart by the call rather than by it.
    (
        "the Library bay's scope chips",
        "library_bay(",
        &["bay.chip("],
    ),
    (
        "the Library bay's filter fields",
        "library_bay(",
        &["bay.filter("],
    ),
    // **The six kind chips one row under the field**, the same derivation
    // again and told apart by the call — the scope chips' own arrangement
    // two rows up. One entry for six capsules, because `LibraryBay::kind`
    // is the whole of that row's offer: which chip a press landed on is
    // inside it, and what leaves names all six (ADR-0338).
    (
        "the Library bay's kind chips",
        "library_bay(",
        &["bay.kind("],
    ),
    // **A row's badges are a readout and still a row here**, which is the
    // `map` pill's own sentence in this table read one bay along — with one
    // difference that is the whole of the entry: this press is **not**
    // swallowed. A badge sits inside the row it is drawn on, so a press on
    // it takes that row in hand exactly as a press on the name does, and
    // the list's own entry below is what asks for it. What the row here
    // buys is the tooltip: the words that say what a badge means and that
    // the control which narrows by kind is the chip row above (ADR-0338).
    // So it names no ask, and the derivation it names is the one the rows
    // beneath it are hit-tested against.
    ("the Library bay's row badges", "library_bay(", &[]),
    (
        "the params chip in the Library bay's foot",
        "library_bay(",
        &["bay.read("],
    ),
    // **The `load` button and the deck pulldown beside it**, and they are
    // one call rather than two: `LibraryBay::aim` is the whole of that
    // control's offer — the press on the button, the press on the
    // capsule, the press on a row of the list and the dismissal — which is
    // the shape the two cards in the transport row are already in, and why
    // neither entry names `Load::picked`, the sub-question `aim` composes.
    // They are told apart by nothing but this table, exactly as the
    // audio-in and arrangement pills are.
    // **The `load` button and the deck pulldown beside it**, one entry
    // for two controls because they are one derivation and one ask:
    // `LibraryBay::aim` is the whole of that control's offer — the press
    // on the button, the press on the capsule, the press on a row of the
    // list it puts down, and the dismissal — which is the shape the two
    // cards in the transport row are each in, and why this names no
    // `Load::picked`, the sub-question `aim` composes. The console's row
    // claims two; this table's rows are that table's rows, so there is one
    // here (ADR-0305).
    (
        "the Library bay's load button and deck pulldown",
        "library_bay(",
        &["bay.aim("],
    ),
    // **The star at the left of each row**, asked before the row it is in
    // so that the smaller box wins — the same derivation again, told apart
    // from the row below it by the call.
    ("the Library bay's stars", "library_bay(", &["bay.starred("]),
    // **The list's row names three calls**, because one rectangle means
    // three things: a press on a library row takes a Set in hand, a press
    // on a `history` row lands that version on its node, and a *secondary*
    // press on either puts that row's menu down. The first two are one
    // control in `input::PROBES` — exactly one of them can answer, told
    // apart by which listing goes in — and the third is a second control
    // on the same rectangle, which is why that row claims two. All three
    // are owed here.
    //
    // **`bay.menu_ask(` is one entry for two arms of the handler**, the
    // secondary press that opens the card and the press of either button
    // that picks from it, exactly as `bay.aim(` is one entry for the
    // pulldown's four.
    (
        "the Library bay's list",
        "library_bay(",
        &["bay.take(", "bay.land(", "bay.menu_ask("],
    ),
    // **The four class pills**, one derivation asked four times: they are
    // in four different regions and cannot be one laid-out box, but they
    // are one type and one question.
    ("the class pills", "mcp_pill(", &["pill.hit("]),
    // **The Sequencer bay's, one derivation and two calls**: a cell, a
    // label, the mode pill and a bank pill are four questions about one
    // laid-out bay, and `Sequencer::press` answers all four — which of them
    // it was is inside the operation it hands back, and the line
    // `sequenced` prints says so.
    //
    // **`bay.chose(` is the second call for the same reason `bay.aim(` is
    // one entry for the pulldown's four**: the `+ lane` pill's press is not
    // an operation but a card going down, so it answers an enum, and the
    // card's own picks and dismissals come back through the same method.
    (
        "the Sequencer bay's cells, labels, mode pill, bank pills and + lane",
        "sequencer_bay(",
        &["bay.press(", "bay.chose("],
    ),
    // **The Staging lane's two, and they are the Library bay's star and
    // its row one bay up**: a row that is itself a control, with a smaller
    // box inside it asked first. The lane is derived once per question
    // rather than held across both, which is that bay's rule for its
    // reason, so the two entries name one derivation and are told apart by
    // the call — `bay.back(` is the capsule and `bay.keep(` is the row it
    // sits in (ADR-0326).
    (
        "the Staging lane's back capsules",
        "staging_bay(",
        &["bay.back("],
    ),
    ("the Staging lane's rows", "staging_bay(", &["bay.keep("]),
];

/// **[`ASKED`] is the console's own rows, in the console's own order.**
///
/// The array's length is `PROBES.len()`, so a row added or removed there
/// is a compile error here; this is the other half of that, and it is what
/// makes an entry identifiable at all. Written in the same order, an entry
/// and a row are the same control by position — written in any other, the
/// two would agree on a count and disagree about which control each line
/// is about, and every message this module prints would name the wrong
/// one.
#[test]
fn the_table_is_the_consoles_own_rows_in_the_consoles_own_order() {
    for (at, (name, derivation, _)) in ASKED.iter().enumerate() {
        assert_eq!(
            *name, PROBES[at].name,
            "entry {at} of `ASKED` says the press handler reaches `{name}` through \
             `{derivation}`, and row {at} of `karakuri_console::input::PROBES` is \
             `{}` — the two lists have gone out of order, and every entry from here \
             down is being checked against another control's row",
            PROBES[at].name
        );
    }
}

/// **The press handler dispatches pointer events directly as executable code.**
///
/// Exercises `Readout::pointer` directly across pointer events (move, down, up,
/// secondary, wheel) and asserts proper event dispatch and claim handling.
#[test]
fn the_press_handler_dispatches_pointer_events() {
    let ctx = super::tests::drawn_once();
    let mut readout = Readout::new(1440.0, 900.0);
    readout.panel.solve();

    // 1. Pointer move to an arbitrary point
    let (_, acted) = readout.pointer(&ctx, Pointer::Moved(Point::new(0.0, 0.0)));
    assert_eq!(acted, Acted::Nothing);

    // 2. Pointer down on unhandled / empty region
    let (_, acted) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(acted, Acted::Nothing);

    // 3. Pointer up
    let (_, acted) = readout.pointer(&ctx, Pointer::Up);
    assert_eq!(acted, Acted::Nothing);

    // 4. Secondary click
    let (_, acted) = readout.pointer(&ctx, Pointer::Secondary);
    assert_eq!(acted, Acted::Nothing);

    // 5. Wheel event
    let (_, acted) = readout.pointer(&ctx, Pointer::Wheel(1.0));
    assert_eq!(acted, Acted::Nothing);

    // 6. Test a real control press: Solo pill in program head
    let head =
        program_head(&ctx, readout.panel.layout(), Open::CLOSED).expect("the bay draws its pill");
    let solo_at = Point::new(head.solo.center().x, head.solo.center().y);

    let (claim, _) = readout.pointer(&ctx, Pointer::Moved(solo_at));
    assert_eq!(claim, Claim::Panel);

    let (claim, acted) = readout.pointer(&ctx, Pointer::Down);
    assert_eq!(claim, Claim::Panel);
    assert!(
        matches!(acted, Acted::Operated(_)),
        "press on solo pill should produce an operated action"
    );
}
