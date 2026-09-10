//! **The panel column of the manual, against the operations this crate's
//! controls emit.**
//!
//! [ADR-0213](../../../docs/adr/0213-the-interface-milestones-meter-is-the-panel-column-and-has-means-an-operator-reaches-it.md)
//! made the panel column of `docs/manual/operations.html` the Interface
//! milestone's meter and defined what a badge in it means: **`has` means an
//! operator running the instrument reaches the operation.** It also said what
//! the first flip owes — *"a test asserts that each `has` row's operation is
//! actually emitted by a console control, and fails in both directions"* — and
//! this file is that test, owed by the five badges `cargo run -p karakuri`
//! made true.
//!
//! It is shaped like `karakuri-environment/src/mcp.rs`'s pair,
//! `every_tool_this_server_publishes_has_a_route_on_the_page` and
//! `every_mcp_route_the_page_claims_is_a_tool_this_server_publishes`, and it
//! sits beside [`vocabulary.rs`](../tests/vocabulary.rs), which reads the same
//! page for the *Arranging the console* section. It is a separate file from
//! that one because it asks a different question of a different type:
//! `vocabulary.rs` checks `panel::Op` against one **section**'s rows, and this
//! checks `karakuri_operation::Operation` against one **column**'s badges.
//!
//! # The whole problem is deciding what "a control emits it" means
//!
//! There is no list in this crate of the operations its controls emit, and a
//! list written here would be the second copy of one
//! (`docs/contributing.md` §4).
//! So the set is **read out of this crate's own source**, and the criterion is
//! stated here rather than left for a reader to infer from a regex.
//!
//! **The criterion.** Over every `.rs` file in `crates/karakuri-console/src`,
//! a line is taken as code after two cuts: a line whose first non-space
//! characters are `//` is dropped whole — which is every `///`, `//!` and `//`
//! — and what is left is truncated at its first `//`. In what survives, every
//! `Operation::` followed by an identifier is an **emission**, and the
//! identifier is the variant. That is what separates
//! `Knob::Trim => Operation::SetGain { .. }` from the fifteen `[`Operation::…`]`
//! links in the doc comments around it, which `grep 'Operation::'` cannot.
//!
//! # What the criterion cannot see, and which way each one fails
//!
//! Every one of these is a way the scan is wrong; what matters is that all but
//! the last of them **fails loudly** rather than passing quietly, and that is
//! why the two cuts are made in the narrowing direction.
//!
//! - **A block comment.** `/* … Operation::SetSync … */` is not a line comment
//!   and is read as an emission. It is a *false positive*: the phantom variant
//!   has no `sample` arm, so [`emissions`] panics naming it.
//! - **`//` inside a string on an emitting line.** The truncation would cut
//!   the emission away with it. That is a *false negative*, and a false
//!   negative cannot fail the direction that says every emission is on the
//!   page — it fails
//!   [`every_panel_route_the_page_marks_built_is_emitted_by_a_console_control`]
//!   instead, which reports it as the page claiming a control that does not
//!   exist. Wrong reason, right failure.
//! - **An emission that never says `Operation::`.** A `use
//!   karakuri_operation::Operation::SetGain;` and a bare `SetGain { .. }`, a
//!   type alias, a variant handed back from a helper in another crate. Invisible
//!   here, and a *false negative* again — so it surfaces the same way, from the
//!   other direction, the moment the page claims it.
//! - **A false positive on a row already marked `has`.** The one combination
//!   that passes in silence: text that is not an emission, naming an operation
//!   the page already claims. Nothing here catches that, and what does is that
//!   the `has` rows each have a test that **presses the control** —
//!   `fader.rs` for the trim and the fader, `blend.rs`, `tally.rs` and
//!   `mask.rs` for the three chips, and `arrangement_pill.rs` for the save and
//!   the restore. This file does not press anything; it is an inventory, and
//!   those five are the proof each item in it is real.
//! - **Construction is not reachability, and reachability is the definition.**
//!   The largest one by far. A `pub fn` in `src/` that builds an `Operation`
//!   and that nothing on the drawn panel calls reads exactly like one a hand
//!   can reach, because ADR-0213's *the operator reaches it* is a property of
//!   `crates/karakuri/src/main.rs` — where a claimed press becomes
//!   `Mixer::blend`, `tally`, `mask` and a drag becomes `Dragged::Fader` — and
//!   this crate takes no device and cannot depend on that binary (ADR-0156).
//!   **So this file checks the necessary half and not the sufficient one.**
//!   A control written here and never wired there would pass, and the badge
//!   would be a lie the page tells on its own authority.
//! - **Only the panel column, and only through `Operation`.** The six rows of
//!   *Arranging the console* reach the operator through `panel::Op` and
//!   through a drag that is no operation at all, not through `Operation`, so
//!   no scan for `Operation::` can see them and this file says nothing about
//!   their badges. `vocabulary.rs` is where that type meets this page, and it
//!   asks a running `Panel` what a hand reaches rather than reading source.
//!
//!   **It is an exemption in the code and not only a sentence here**, which is
//!   [`ELSEWHERE`]: the second assertion below reads every `has` badge in the
//!   column and demands an emission for it, so the day *Move a boundary* was
//!   marked built this file failed saying the page claimed a control that does
//!   not exist — for a drag that no control will ever emit, because the
//!   vocabulary carries it as `Undecided`. The sentence was true of the first
//!   assertion and false of the second.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use karakuri_operation::{BlendMode, Operation, Residency, WipeKind};

/// The specification, relative to the workspace root.
const PAGE: &str = "docs/manual/operations.html";

/// The source this crate's controls are read out of, relative to the same
/// root. Its own crate, read as text: there is no other way to ask *which
/// operations does this code construct* from inside a test binary.
const SRC: &str = "crates/karakuri-console/src";

/// What marks a row on the page — the marker `vocabulary.rs` and
/// `karakuri-environment/src/mcp.rs` both match, for the reason the first of
/// them gives: sections are `<h2>` and a heading somebody adds for looks is
/// neither.
const ROW: &str = r#"<div class="op-head">"#;

/// **The one section of the page whose panel badges are not this file's**, and
/// the heading is what identifies it because a section is an `<h2>` here as it
/// is everywhere else on the page.
///
/// *Arranging the console* is the console's own shape, and every route into it
/// is [`karakuri_console::panel::Op`] or a gesture on
/// `karakuri_console::panel::Panel` — never a `karakuri_operation::Operation`,
/// which is what this file scans for. So a `has` badge in that section is a
/// claim this file cannot judge and **would judge wrongly**: *Move a boundary*
/// is a drag rather than an operation, the vocabulary carries it as
/// `Undecided`, and nothing in [`SRC`] will ever construct it. That was the
/// header's last bullet said as prose; this is it said as code, because the
/// bullet was true of the first assertion below and not of the second, which
/// went on demanding an `Operation` for every badge in the column.
///
/// **Those badges are checked, and `tests/vocabulary.rs` is where.** It reads
/// this section and asks a running `Panel` what a hand on it reaches, both
/// ways round — the same pair as here, against the type the console performs.
const ELSEWHERE: &str = "<h2>Arranging the console</h2>";

/// The badge text of a route that names nowhere. A `plan` badge is allowed to
/// be this — three of them are, and ADR-0213 says which — but a `has` badge
/// cannot: it would claim an operator reaches the operation and decline to say
/// from where.
const NOWHERE: &str = "&mdash;";

/// **The rows a control on this panel emits and an operator still cannot
/// reach**, which is the one gap between *emitted* and ADR-0213's *reached*
/// that this file has ever had to carry. **It is empty**, and the entry it
/// held is what the mechanism was built for.
///
/// # What the one entry was, and how it left
///
/// Every emission on this list ends in a record and a movement: `written`
/// converts it, `crates/karakuri` applies it, and the deck is somewhere else
/// afterwards. *Set a deck's sync mode* did not, for one release: `SetSync`
/// converted to `Owed(NotSettled)`, which was `karakuri-operation-record`
/// saying that *what* its record carried was undecided — the anchor goes
/// through the engine's clamp, so whether the record carried what was asked
/// for or what was clamped read as *"a decision about the bytes on disk"*, and
/// [ADR-0218](../../../docs/adr/0218-re-anchoring-is-set-sync-naming-the-mode-the-deck-is-in-and-a-cycle-cannot-say-it.md)
/// left it open by name. So the deck head's sync chip and its anchor were
/// reachable *affordances* over an unwritable record: a press was claimed, the
/// operation was emitted, the window printed the question, and the deck did
/// not move.
///
/// **There were never two answers.** The clamp holds the anchor inside the
/// range every tempo in the system is held in, and a session's tempo is
/// already inside it, so the anchor asked for and the anchor clamped are the
/// same number. What was missing was a reading, `Current::tempo` is it, and
/// the conversion writes `Record::Transport` from the policy
/// `karakuri_engine::transport::Transport::engaged` had already fixed.
///
/// **`has` would have been a lie in exactly the way ADR-0213 was written to
/// prevent** — *"the row is claimed the day a person who launched the
/// instrument can perform that operation from the panel in front of them"* —
/// and drawing no control would have been a worse one, because the panel is
/// the only surface that can offer re-anchoring at all. So the badge stayed
/// `plan` while that was true and the exemption was written here with its
/// reason, which is what the first assertion's own failure message invites:
/// *"Flip the badge, or say here why the control is not reachable"*.
///
/// # It was written to delete itself, and it did
///
/// A list here is a second copy of something (`docs/contributing.md` §4), so
/// it was held
/// against both of its halves by
/// [`the_unreachable_exemption_is_still_the_state_of_the_page`]: the operation
/// had to still be emitted, and its badge had to still **not** be `has`. The
/// day the record was settled the badge flipped, that test failed, and it
/// named the line to remove. That is the whole of what happened here, and it
/// is why the array stays: the mechanism cost one line to keep and it is what
/// the next control to reach past the page will be caught by.
///
/// **Not derived from `karakuri-operation-record`**, which is where the answer
/// lives, because this package deliberately holds no dependency on it —
/// `Cargo.toml` says so at length, and reaching for one to spell a one-line
/// exemption would undo the closing of ADR-0156 that manifest records.
const UNREACHABLE: [&str; 0] = [];

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn page() -> String {
    let path = workspace().join(PAGE);
    fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "{} is the specification and could not be read: {e}",
            path.display()
        )
    })
}

/// **A value of the operation a variant name stands for**, so that the row
/// this file looks for is [`Operation::title`]'s answer and never a heading
/// transcribed here.
///
/// The same job `mcp.rs`'s `sample` does for a tool's arguments, and it fails
/// the same way: a control that starts emitting something new arrives as a
/// panic naming the variant, rather than being passed over. The field values
/// are arbitrary — nothing reads them — and the *variant* is compiler-checked,
/// so renaming one in `karakuri-operation` breaks this file at build time.
fn sample(variant: &str) -> Operation {
    match variant {
        "SetGain" => Operation::SetGain { deck: 0, gain: 0.0 },
        "SetOpacity" => Operation::SetOpacity {
            deck: 0,
            opacity: 0.0,
        },
        "SetBlendMode" => Operation::SetBlendMode {
            deck: 0,
            blend: BlendMode::Over,
        },
        "SetResidency" => Operation::SetResidency {
            deck: 0,
            residency: Residency::Live,
        },
        "SetMaskShape" => Operation::SetMaskShape {
            deck: 0,
            kind: WipeKind::Linear,
            angle: 0.0,
        },
        // The two the transport row's arrangement pill emits. They are the
        // first emissions from a row of *Arranging the console*, which
        // [`ELSEWHERE`] exempts from the other direction and not from this
        // one — so the day the pill landed, this file demanded the two badges
        // on that page and got them.
        "SaveArrangement" => Operation::SaveArrangement {
            name: String::new(),
        },
        "RestoreArrangement" => Operation::RestoreArrangement {
            name: String::new(),
        },
        // The two the transport row's look controls emit. They are one row of
        // *Mixing and output* each, so unlike the pair above they are **not**
        // in [`ELSEWHERE`] and both directions below judge them.
        "SetTonemap" => Operation::SetTonemap {
            tonemap: karakuri_operation::Tonemap::Aces,
        },
        "SetExposure" => Operation::SetExposure { exposure: 1.0 },
        // The Master bay's one control, and the one emission in this list
        // that names no deck: the master out is a level on the whole fold
        // rather than on a member of it, which is why `Knob::Out` carries no
        // slot for `Knob::Trim`'s and `Knob::Fader`'s to be filled in from
        // (ADR-0224).
        "SetMasterOut" => Operation::SetMasterOut { out: 1.0 },
        // The Master bay's other three, and they name no deck for the master
        // out's reason: the chain reads what the fold produced. Each is one
        // row of *Mixing and output* and each is emitted by one effect row —
        // the feedback row twice, from its track and from its cut chip, which
        // is `dedup_by_key`'s case again (ADR-0317).
        "SetFeedback" => Operation::SetFeedback {
            params: karakuri_operation::Feedback::default(),
        },
        "SetBloom" => Operation::SetBloom {
            params: karakuri_operation::Bloom::default(),
        },
        "SetRgbShift" => Operation::SetRgbShift {
            params: karakuri_operation::RgbShift::default(),
        },
        // The two the Inspector's deck head emits. `SetSync` comes from two
        // controls in that row — the chip that cycles and the anchor that
        // re-asks for the mode the deck is in (ADR-0218) — and one operation
        // is one row however many controls name it, which is what
        // `dedup_by_key` below is for. `SetSync` was also the one entry
        // [`UNREACHABLE`] ever held; see there for why it is not one now.
        "SetSync" => Operation::SetSync {
            deck: 0,
            sync: karakuri_operation::Sync::Beat,
        },
        "ScrubDeck" => Operation::ScrubDeck {
            deck: 0,
            beats: 0.25,
        },
        // **The Mixer bay's sixth, and the one emission here that names no
        // control**: a press anywhere on a strip that the trim, the fader, the
        // blend, the tally and the mask all declined selects that deck. It is
        // the console's own pointer — `SelectDeck` writes no record — so what
        // performs it is the surface, which is why the badge it demands is a
        // *panel* badge and there is nothing on the deck to check afterwards.
        "SelectDeck" => Operation::SelectDeck { deck: 0 },
        // **The transport row's audio-in pill**, whose card lists the inputs
        // the machine has and whose rows are the picking. One operation over
        // however many inputs there are, and the string is the description the
        // device answered to when the list was read — which is the same string
        // `AudioInput::open` matches a selector against, so what leaves the
        // panel is what opens the device.
        //
        // **The row it claims was a `launch` row until this control existed**:
        // `--audio-in` was the only way in and nothing could change it after
        // the run started. A card that lists the inputs and opens one is that
        // row reached during a set, which is a change to the page and is made
        // there rather than assumed here.
        "AttachBeatSource" => Operation::AttachBeatSource {
            source: karakuri_operation::BeatSource::AudioInput("default".to_owned()),
        },
        // **The Mixer bay's transition row**, and the one emission in this
        // list that names no deck *and* no slot of anything: the three
        // settings decide what the next fade, crossfade or wipe means
        // wherever it lands. Three pills emit it — the shape, the quantum and
        // the length — and one operation is one row however many controls
        // name it, which is what `dedup_by_key` below is for and is the same
        // arrangement `SetSync` is in.
        //
        // **The row's panel badge went `has` and its key badge did not.** The
        // page's key column for this row reads `z n j`, and those three keys
        // are bound elsewhere in the program the panel runs inside; the badge
        // stays `plan` until that is worked through, which is a decision about
        // the keyboard and not about this control.
        "SetTransition" => Operation::SetTransition {
            setting: karakuri_operation::TransitionSetting::Quantum { beats: 4.0 },
        },
        // **The `go` capsule at the end of that row**, and the one emission in
        // this list that names *two* decks: a wipe covers the deck the
        // selection is on with the next one round, which is
        // `view::TransitionRow::go`'s translation and never the operation's —
        // the vocabulary carries both decks for exactly that reason.
        //
        // **The one emission here that a press can be refused for.** A
        // one-deck mixer and a shape reading `no shape` both turn the press
        // away before an operation is built, and the badge is still `has`:
        // ADR-0213's meter is *an operator reaches the operation*, and a
        // control that refuses under a named condition is reached. It is
        // `karakuri-cli`'s `c` in that too.
        "Wipe" => Operation::Wipe { from: 0, to: 1 },
        // **The Library bay's scope chips**, and the one emission in this list
        // whose payload cannot say what the control chose: `SelectScope`
        // carries `Undecided`, deliberately, because *"an enum of the four
        // here would assert that the list can be finished"*. So the chip
        // travels beside the operation in `view::Chosen` rather than inside
        // it, and what this file sees is the operation — which is the right
        // thing for it to see, because what the badge claims is that an
        // operator reaches *the row*, and the row is one operation over
        // whichever scopes the list holds.
        // **The three the tracker group emits**, and two of them are the only
        // controls on this console whose operation reaches the *room* rather
        // than the deck, the arrangement or the store. A tap carries nothing
        // at all — the instant is when the operation arrives — and the octave
        // carries the direction; the value is either half, because what the
        // badge claims is that an operator reaches the row and the row is one
        // operation over both.
        "TapBeat" => Operation::TapBeat,
        "ScaleGrid" => Operation::ScaleGrid {
            by: karakuri_operation::GridScale::Halve,
        },
        // **The offset track in the same group**, and the value is any offset:
        // the operation is absolute, the press names where along the track it
        // landed, and what the badge claims is that an operator reaches the
        // row.
        "SetLatencyOffset" => Operation::SetLatencyOffset { ms: -15.0 },
        "SelectScope" => Operation::SelectScope {
            scope: karakuri_operation::Undecided,
        },
        // **The two filter fields under those chips**, and this is the same
        // row's other half: the chip says which library, the fields narrow what
        // that library answers. Unlike `SelectScope` the payload carries
        // everything the press decided — the two filters as they stand after
        // the step — so nothing travels beside it and what this file sees is
        // the whole answer. The value here is *any* `ListSets`, because what
        // the badge claims is that an operator reaches the row.
        "ListSets" => Operation::ListSets {
            holds: None,
            layer: None,
        },
        // **The star at the left of each row of that bay's list**, and the
        // one emission in this list whose payload is a **state** the control
        // read off the row it landed on: the operation is not a toggle
        // (ADR-0299), so the press asks for the state the row is not in, and
        // either value names the row. The id is any Set the store holds,
        // because what the badge claims is that an operator reaches the row.
        "SetFavourite" => Operation::SetFavourite {
            id: "night01".to_owned(),
            favourite: true,
        },
        // **The `read` chip in the Library bay's foot**, and the one emission
        // in this list whose operand is a *pointer of this console's own*: the
        // id is the Set under the cursor, which is where the `load` button
        // beside it reads its Set too. The value is any id, because what the
        // badge claims is that an operator reaches the row.
        //
        // **One of its two presses emits nothing**, and that is not a gap in
        // this inventory: opening asks for a reading and closing puts one
        // away, which changes what this bay is drawing and nothing else —
        // `view::Read`. What this file sees is the asking, which is the half
        // the page's badge is about.
        "ReadSet" => Operation::ReadSet {
            id: "night01".to_owned(),
        },
        // **The drag from a row of that bay onto a mixer strip**, and the one
        // emission in this list that no *press* produces: a press on a row
        // takes the Set in hand and asks for nothing, and this is built at the
        // release, out of the row that was carried and the strip it was let go
        // over (`panel::Released::Dropped`). What the badge claims is that an
        // operator reaches the row, so the value is any load.
        //
        // **It is the second route to a row the key already had**, which is
        // what `console.html` calls it — *"a second route to the same command,
        // and never the first"* — and the badge is about the *panel* column,
        // so the key's `has` beside it was never evidence for this one.
        "LoadSet" => Operation::LoadSet {
            deck: 0,
            set: "night01".to_owned(),
        },
        // **The Library bay's fifth scope chip**, and the one emission in this
        // list that is a *listing* asked for by a control whose four
        // neighbours ask for a different row: `history` is not a library of
        // Sets, so a press on it names *Walk the edit history* where a press
        // on `all` names *Choose which scope the library shows*
        // (`view::Chosen`, ADR-0308). The payload is `Undecided` for
        // `SelectScope`'s reason read on a history: what a walk would carry is
        // *which* history, and no surface spells a Set id — the panel narrows
        // it to whatever the load pulldown's deck is running, which is the
        // host's answer.
        "WalkHistory" => Operation::WalkHistory {
            step: karakuri_operation::Undecided,
        },
        // **A press on a row of that scope**, and the one emission in this
        // list whose operand is a *version* rather than a Set: the row is the
        // name the store filed the version under, and the deck is the
        // pulldown's. The value is any picked revision, because what the badge
        // claims is that an operator reaches the row.
        //
        // **Both arms of `Revision` have a producer since 2026-09-09**, and
        // the one here is either: the Staging lane's `back` capsule asks for
        // `Previous` and a `history` row asks for `Picked`, and what the badge
        // claims is that an operator reaches the *row*. This file counts
        // operations rather than arms, so the value is the one that was here
        // before the second producer landed (ADR-0326).
        "RestoreProcedure" => Operation::RestoreProcedure {
            deck: 0,
            revision: karakuri_operation::Revision::Picked(
                "20260908-143052-271_slot0_L4_beat_strokes".to_owned(),
            ),
        },
        // **A press on a Staging lane row**, which is the whole of that
        // control: the row is the keep and the capsule at its end is the
        // put-back above (ADR-0326). It is addressed by the node the row is
        // for, because a lane row is one node a build changed rather than one
        // slot; the value is any node, because what the badge claims is that
        // an operator reaches the row.
        "KeepCandidate" => Operation::KeepCandidate {
            deck: 0,
            node: karakuri_operation::NodeAt {
                layer: karakuri_operation::Layer::L1,
                index: 0,
            },
        },
        // **The `keep` capsule in each Inspector pane's head**, and the `id`
        // is `None` because the capsule types no name: this console's one
        // letter-taking flow is bounded to naming an arrangement, so a keep
        // files under a stamp exactly as the `k` key's does (ADR-0287). What
        // the badge claims is that an operator reaches the row.
        "SaveSet" => Operation::SaveSet { deck: 0, id: None },
        // **The tempo figure at the head of the transport row**, and it is
        // not this row's work — the arm is here because `sample` is one list
        // and an operation with no value in it stops every assertion in this
        // file, not only the one about its own row. The value is any tempo,
        // because what the badge claims is that an operator reaches the row.
        "SetFreeRunTempo" => Operation::SetFreeRunTempo { bpm: 128.0 },
        // **The `rec` pill at the end of the transport row**, and the one
        // emission in this list whose payload is what the *control* is showing
        // rather than an arbitrary value: the pill is a toggle, so a press
        // asks to start where nothing is running and to stop where something
        // is. Either payload names the row, and the badge claims an operator
        // reaches it — so this is a `Stop`, which is the gesture the mock's
        // tip already named.
        //
        // **`Start` carries `None`**, and it would if this were the arm: each
        // start files under a fresh stamp, because a second head under one id
        // is read back as edits (ADR-0289).
        "RecordSession" => Operation::RecordSession {
            recording: karakuri_operation::Recording::Stop,
        },
        // **The fold at the right of an Inspector pane's deck head**, and the
        // one emission in this list whose press is a *rebuild*: the layering
        // is a field of the aim a slot's watcher is pointed at, so the window
        // restates the rest of that aim and the worker recompiles the slot off
        // the render thread, judged against the budget like an edit or a
        // library load (ADR-0314). None of that is visible from this crate,
        // which names a destination and a deck and no more.
        //
        // **A destination and not a step**, which is the value's own argument:
        // the chip reads what the deck is doing and asks for the other, so
        // either payload names the row and this one is the press on a deck
        // that overdraws.
        "SetCompositing" => Operation::SetCompositing {
            deck: 0,
            compositing: true,
        },
        // **The renderer chips in the Inspector's node groups**, and one
        // operation is one row however many chips name it — the same
        // arrangement `SetSync` and `SetTransition` are in. A press on a deck
        // that overdraws, or on the one chip of a Set with a single renderer,
        // is drawn and not claimed, which is `Wipe`'s reading of a refusal
        // under a named condition.
        "SelectRenderer" => Operation::SelectRenderer {
            deck: 0,
            renderer: 0,
        },
        // **A parameter row's fader in an Inspector pane**, and the one
        // emission in this list that names something *inside* a Set rather
        // than a level on it: the address is `ParamAt`, a `None` node is the
        // wildcard the interface published, and a component of a vector is
        // its own row and its own key (ADR-0268). The value is any write,
        // because what the badge claims is that an operator reaches the row.
        "WriteParam" => Operation::WriteParam {
            deck: 0,
            param: karakuri_operation::ParamAt {
                node: None,
                key: "exposure".to_owned(),
            },
            value: karakuri_operation::ParamValue::Scalar(0.5),
        },
        // **`Save as a kbset`, the one item of a Library row's menu that is
        // not a load** — and the one emission in this list that names *no*
        // destination and never will: sending is a read, and a read's answer
        // goes where the surface that asked puts answers (ADR-0260), which for
        // the panel is the file the operator names in the system's own save
        // dialog (ADR-0311).
        //
        // **The `Send` arm and not the `Take` one**, and that is the whole of
        // why this arm has a choice to make: the row is *Send a Set to
        // somebody, and take one in*, one row and two directions, and the
        // taking half has been reached from this bay since a press on a
        // `presets` row landed — but it is reached from `crates/karakuri`,
        // which builds the operation on the host side of the seam. What
        // `crates/karakuri-console/src` constructs is the sending, so that is
        // what this value is.
        "TransferSet" => Operation::TransferSet {
            transfer: karakuri_operation::SetTransfer::Send {
                id: "night01".to_owned(),
            },
        },
        // **The Sequencer bay's three, and every one of them names the bank it
        // acts on**: implying the armed one is the shape `SelectDeck`'s rule
        // refuses, so the pattern travels *in* the payload where
        // `SelectScope`'s chip travels beside it.
        //
        // **A cell's `step` is a stored slot and not a drawn step.** A pattern
        // holds sixteen slots in both modes and an eighth reads slot `2k`, so
        // the console sends the even ones in the finer reading and a step
        // press cannot race a mode press into an address that means two things
        // (ADR-0320). The value here is any cell, because what the badge
        // claims is that an operator reaches the row.
        //
        // **Both `on` and `muted` are states read off what the control is
        // showing**, which is `SetFavourite`'s arrangement two rows up: the
        // press asks for the state the cell or the lane is *not* in, so either
        // value names the row and there are no toggles in this vocabulary.
        "SetStep" => Operation::SetStep {
            pattern: 0,
            lane: 0,
            step: 4,
            on: true,
        },
        "SetLaneMute" => Operation::SetLaneMute {
            pattern: 0,
            lane: 0,
            muted: true,
        },
        // **The mode pill at the head of that bay**, and the one emission in
        // this list whose payload is what the *pattern* is rather than what a
        // hand prefers: the press asks for the other of the two by naming it,
        // and the count follows the mode (ADR-0306).
        "SetPatternGrid" => Operation::SetPatternGrid {
            pattern: 0,
            grid: karakuri_operation::StepMode::Eighth,
        },
        // **The `+ lane` chooser's items**, and the value is a fader because
        // the faders are the half of the list every console has: a parameter
        // item needs a deck the Inspector holds a pane for, and what the badge
        // claims is that an operator reaches the row.
        "PointLane" => Operation::PointLane {
            pattern: 0,
            target: karakuri_operation::LaneTarget::Fader { deck: 0 },
        },
        // **A bank pill in the bay head**, naming a bank and never a
        // direction: with four fixed banks a press on an empty one is the
        // mock's `+`, which is why there is no second operation for it
        // (ADR-0320, ADR-0327).
        "SelectPattern" => Operation::SelectPattern { pattern: 0 },
        // **The Inspector's three, and the two addresses are the two facts.**
        // A node head's chip names a `NodeAt`, a sensitivity chip names a
        // `BindAt` — a layer, one node of it or all of them, and a key —
        // because an attachment is one layer's where a value's wildcard names
        // no layer at all (ADR-0319).
        //
        // **`SetAuthority` names a destination and never a step**: the press
        // asks for the level it lands on, which is `SetStep`'s arrangement
        // above and P-0090's requirement.
        "SetAuthority" => Operation::SetAuthority {
            deck: 0,
            node: karakuri_operation::NodeAt {
                layer: karakuri_operation::Layer::L1,
                index: 0,
            },
            authority: karakuri_operation::Authority::Suggesting,
        },
        // **The curve chip restates the attachment**, so the value here is a
        // whole one: a source, a shape and the range it is mapped onto. A
        // press changes the shape and sends the other two back unchanged,
        // which is what stops a press for a different curve re-mapping the
        // signal.
        "AttachSignal" => Operation::AttachSignal {
            deck: 0,
            param: bind_at(),
            signal: "energy".to_owned(),
            curve: karakuri_operation::Curve::Sqrt,
            range: [0.1, 2.4],
        },
        // **The chip beside it**, and it removes the attachment rather than
        // suspending it — there is no suspended state anywhere for it to
        // leave behind (ADR-0319).
        "TakeParamBack" => Operation::TakeParamBack {
            deck: 0,
            param: bind_at(),
        },
        // **The Outputs row's chips**, which name an output by a word from a
        // closed list and say whether it is on (ADR-0324). `Projector(0)` is
        // the one this repository owns; the program view's is the same
        // operation naming a different destination.
        "RouteFrame" => Operation::RouteFrame {
            output: karakuri_operation::Output::Projector(0),
            on: true,
        },
        // **The two chips before the fold on an Inspector pane's deck head**,
        // and one operation is one row however many chips name it — the
        // arrangement `SetSync` is already in with its anchor. Each is a field
        // of the aim the slot's watcher is pointed at, so the press is a
        // rebuild off the render thread exactly as the fold beside them is
        // (ADR-0328).
        //
        // **The capacity arm and not the seed one**, and the choice is
        // arbitrary in the way this file's values are: what the badge claims is
        // that an operator reaches the *row*, and the row is one heading over
        // two operations. Neither carries a node — an aim holds one capacity
        // and one seed for the whole slot, which is what the deck this payload
        // names already says.
        // **A pick out of a `uses` line's card in an Inspector pane**, and the
        // one emission in this list addressed **by name at both ends**: an edge
        // survives a reorder and a position does not, which is `Record::Edge`'s
        // own decision. The capsule that opens the card emits nothing and owes
        // this page no row, which is the Library bay's deck pulldown's rule
        // (ADR-0305, ADR-0329). The value is any wiring, because what the badge
        // claims is that an operator reaches the row.
        "WireInput" => Operation::WireInput {
            deck: 0,
            node: "swirl_warp".to_owned(),
            slot: "far".to_owned(),
            to: "sphere_shell".to_owned(),
        },
        // **The mark at the left of a parameter row**, which is the control's
        // position in the deck's published interface and, by being there or
        // not, whether it is published at all.
        //
        // **The whole ordered list and never one entry**, which is the
        // vocabulary's own sentence at the variant: a knob is learned against a
        // position, so an operation that added or removed one at a time would
        // renumber every binding after it. The value is any interface, because
        // what the badge claims is that an operator reaches the row — and it is
        // deliberately **not** empty, since an empty list means *publish
        // everything* and would read as a payload nobody filled in.
        "Publish" => Operation::Publish {
            deck: 0,
            controls: vec![karakuri_operation::Control {
                name: "exposure".to_owned(),
                node: None,
                key: "exposure".to_owned(),
                range: [0.5, 2.0],
            }],
        },
        "SetProperty" => Operation::SetProperty {
            deck: 0,
            property: karakuri_operation::Property::Capacity { elements: 65_536 },
        },
        // **The `keep` capsule on a node group's head**, which writes one
        // node's source into the operator's own library (ADR-0338, decision
        // 4). The value is any node, because what the badge claims is that an
        // operator reaches the row.
        //
        // **`id` is `None` and that is the capsule's own answer**, not an
        // unfilled field: this is the press that types nothing and takes a
        // stamp, which is ADR-0128's second route drawn on one capsule — the
        // first is the name typed into the pane head above, and the host is
        // what pairs the two.
        "KeepProcedure" => Operation::KeepProcedure {
            deck: 0,
            node: karakuri_operation::NodeAt {
                layer: karakuri_operation::Layer::L1,
                index: 0,
            },
            id: None,
        },
        // **The six kind chips under the filter field**, which say which kinds
        // of row the listing holds (ADR-0338, decision 2). The value is any
        // state of the six, because what the badge claims is that an operator
        // reaches the row — and every press names all six, so there is no
        // partial value to choose.
        "FilterLibrary" => Operation::FilterLibrary {
            kinds: karakuri_operation::LibraryKinds {
                l3: true,
                ..karakuri_operation::LibraryKinds::EVERYTHING
            },
        },
        // **A procedure row's load**, reached from the row's own press, the
        // `load` button and the row menu's four items (ADR-0338, decision 3).
        // The value is any deck and any name, for the row above's reason.
        "LoadProcedure" => Operation::LoadProcedure {
            deck: 0,
            procedure: "orbit_wide".to_owned(),
        },
        // **The pulldown on a pane head**, which points that pane at a deck
        // and moves nothing else (ADR-0338, decision 5). The pane is named by
        // the arrangement's own handle for it, which is what the payload
        // carries: this crate has the names and `karakuri-operation` cannot.
        "PointPane" => Operation::PointPane {
            pane: karakuri_console::view::PANE_NAMES[0].to_owned(),
            deck: 0,
        },
        other => panic!(
            "`{SRC}` constructs `Operation::{other}` and this file has no value for it — a \
             control started emitting an operation nobody accounted for. Add an arm here, and \
             then decide whether that row's panel badge in {PAGE} is now `has`"
        ),
    }
}

/// The address an attachment carries — a layer, a node of it, and a key.
fn bind_at() -> karakuri_operation::BindAt {
    karakuri_operation::BindAt {
        layer: karakuri_operation::Layer::L1,
        index: Some(0),
        key: "turbulence".to_owned(),
    }
}

/// Every `.rs` file under `dir`, **including the ones in directories under
/// it**.
///
/// The read was one level deep until 2026-09-07, with the count floor above as
/// its only guard — and the floor could not have caught the case it was
/// written for. There are seven `.rs` files directly under `SRC`; splitting
/// `view.rs` into `view/` would leave seven of them there and take every
/// emission in it out of this scan, silently, with `files.len() >= 7` still
/// true. `crates/karakuri/src/main.rs`'s press handler ran the same listing
/// with the same floor and is gone; this is the other half.
fn walk(dir: &Path, into: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| {
        panic!(
            "{} is under this crate's source and is unreadable: {e}",
            dir.display()
        )
    });
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            walk(&path, into);
        } else if path.extension().is_some_and(|e| e == "rs") {
            into.push(path);
        }
    }
}

/// **Every operation a control in this crate constructs**, by the criterion in
/// this file's header, as an `Operation` apiece.
///
/// Sorted and deduplicated by title, because the question is which rows are
/// reached and one row may be reached from more than one file.
/// **Every `.rs` file under [`SRC`], as one string**, for [`DRAWN`]'s markers
/// to be looked for in.
///
/// Comments are **not** cut, unlike [`emissions`]: a marker naming a field or
/// a function is looked for as text, and a mention of one in a doc comment is
/// a mention of a thing that exists. The failure this guards against is a
/// readout that stopped being drawn, and deleting a field deletes the lines
/// that talk about it.
fn code_of_src() -> String {
    let dir = workspace().join(SRC);
    let mut files = Vec::new();
    walk(&dir, &mut files);
    files.sort();
    files
        .iter()
        .map(|path| {
            fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("{} could not be read: {e}", path.display()))
        })
        .collect()
}

fn emissions() -> Vec<Operation> {
    let dir = workspace().join(SRC);
    let mut files = Vec::new();
    walk(&dir, &mut files);
    files.sort();
    assert!(
        files.len() >= 7,
        "only {} `.rs` files found under {SRC} — a scan that reads nothing would find no \
         emissions and pass every assertion below",
        files.len()
    );

    let mut variants = BTreeSet::new();
    for path in &files {
        let text = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("{} could not be read: {e}", path.display()));
        for line in text.lines() {
            // The first cut: a line that is nothing but a comment. `///`,
            // `//!` and `//` all begin this way.
            let line = line.trim_start();
            if line.starts_with("//") {
                continue;
            }
            // The second cut: whatever trails a `//` on a line of code.
            let code = match line.find("//") {
                Some(at) => &line[..at],
                None => line,
            };
            for after in code.split("Operation::").skip(1) {
                let end = after
                    .find(|c: char| !c.is_alphanumeric() && c != '_')
                    .unwrap_or(after.len());
                if end > 0 {
                    variants.insert(after[..end].to_owned());
                }
            }
        }
    }

    let mut found: Vec<Operation> = variants.iter().map(|v| sample(v)).collect();
    found.sort_by_key(|op| op.title());
    found.dedup_by_key(|op| op.title());
    found
}

/// **The titles of the rows in [`ELSEWHERE`]**, read off the page rather than
/// listed here, so that a row added to that section is exempt the day it lands
/// and a section renamed out from under this file panics instead of quietly
/// exempting nothing.
fn elsewhere() -> BTreeSet<String> {
    let html = page();
    let start = html.find(ELSEWHERE).unwrap_or_else(|| {
        panic!(
            "`{ELSEWHERE}` is gone from {PAGE} — the section whose panel badges are checked \
                by `tests/vocabulary.rs` rather than here"
        )
    });
    let rest = &html[start + ELSEWHERE.len()..];
    let end = rest.find("<h2").unwrap_or(rest.len());
    let mut found = BTreeSet::new();
    for part in rest[..end].split(ROW).skip(1) {
        let Some(open) = part.find("<h3>") else {
            continue;
        };
        let rest = &part[open + "<h3>".len()..];
        let Some(close) = rest.find("</h3>") else {
            continue;
        };
        found.insert(rest[..close].to_owned());
    }
    assert!(
        found.len() >= 6,
        "only {} rows found under `{ELSEWHERE}` in {PAGE} — an exemption that matches nothing \
         is one this file would not notice it had stopped granting",
        found.len()
    );
    found
}

/// **Every row's title and its panel badge**, in page order: the badge's class
/// — `has`, `plan` or `gap` — and the text it names the control's home with.
///
/// Read verbatim and never decoded, for `mcp.rs`'s reason: a `gap` badge says
/// `&mdash;` and a home that needed decoding to match would be a home nobody
/// could find on the console page.
/// **What a readout the panel draws is drawn by**, one row per readout: the
/// title on [`PAGE`] and a marker in [`SRC`] that draws it.
///
/// # Why a `has` badge can be satisfied by this and not only by an emission
///
/// [`every_panel_route_the_page_marks_built_is_emitted_by_a_console_control`]
/// demands that some line of this crate construct `Operation::<the variant>`,
/// and for a **write** that is the only way a surface reaches an operation: a
/// fader that moves nothing is not a fader. **A readout has no gesture at
/// all.** It is answered without being asked, so there is no press to attach
/// an emission to, and inventing one would be ADR-0264's own rejected
/// alternative in a worse form.
///
/// [ADR-0284](../../../docs/adr/0284-a-readout-is-drawn-and-the-panel-badge-does-not-move-because-the-meter-counts-a-gesture.md)
/// left the badge at `plan` and named the correction to start from: relax the
/// check for rows the page marks `read`, and hold the drawing somewhere that
/// can see it. **What it turned that shape down for was what it would stop
/// checking** — relaxing for a class of rows removes the emission check for
/// *every* member of it, including the two that are `has` today because a
/// gesture really does emit. This table is the answer to that objection: the
/// check is not relaxed, it is given a second way to be met, and the second
/// way is as mechanical as the first. A row that is neither emitted nor drawn
/// still fails.
///
/// **A hand-written table, and the precedent is `press_handler::ASKED`** in
/// `crates/karakuri/src/main.rs` — the same shape, the same failure mode, and
/// the same answer to it: an entry that names nothing fails, and a badge with
/// no entry fails. What this one cannot see is the *seam* — that the value
/// reaching the readout is the one the deck said — because `karakuri-console`
/// cannot see `crates/karakuri`. That half is the window's own test, named
/// beside each entry.
const DRAWN: [(&str, &str); 1] = [
    // **The transport row's health capsule** — `landed`, `overloaded` or
    // `failed`, taken off the one drain that writes the Staging lane. The
    // drawing is `tests/transport.rs`'s; the seam is `crates/karakuri`'s
    // `the_swap_report_says_what_the_lane_says`, which takes a device because
    // a `swap::Event` cannot be made without one.
    ("Find out what a write did", "pub health: Option<Stage>"),
];

/// **The titles the page marks `read`**, off the mark
/// [ADR-0281](../../../docs/adr/0281-every-route-reaches-every-write-and-a-read-is-the-routes-own-interface-design.md)
/// put in each one's `op-head`: *it only asks, so a column left empty here is
/// that way in's own design and not something owed*.
fn reads() -> BTreeSet<String> {
    let html = page();
    let mut found = BTreeSet::new();
    for row in html.split(ROW).skip(1) {
        let Some(close) = row.find("</h3>") else {
            continue;
        };
        let Some(open) = row[..close].rfind("<h3>") else {
            continue;
        };
        let head = &row[close..];
        let head = &head[..head.find("</div>").unwrap_or(head.len())];
        if head.contains(r#"<span class="op-kind">read</span>"#) {
            found.insert(row[open + "<h3>".len()..close].to_owned());
        }
    }
    assert!(
        found.len() >= 4,
        "only {} rows of {PAGE} carry the `read` mark — a mark that matches almost nothing is          one this file would not notice had been renamed",
        found.len()
    );
    found
}

fn panel_routes() -> Vec<(String, String, String)> {
    let html = page();
    let mut found = Vec::new();
    for row in html.split(ROW).skip(1) {
        let Some(open) = row.find("<h3>") else {
            continue;
        };
        let rest = &row[open + "<h3>".len()..];
        let Some(close) = rest.find("</h3>") else {
            continue;
        };
        let title = rest[..close].to_string();
        // The row ends where the next section does; a badge found past that
        // would belong to another row.
        let body = &rest[close..];
        let body = &body[..body.find("</section>").unwrap_or(body.len())];
        let mut badge = None;
        for span in body.split(r#"<span class="rt "#).skip(1) {
            let Some(quote) = span.find('"') else {
                continue;
            };
            let class = span[..quote].to_string();
            let Some(text) = span[quote..].strip_prefix(r#"">panel <b>"#) else {
                continue;
            };
            let Some(shut) = text.find("</b>") else {
                continue;
            };
            badge = Some((class, text[..shut].to_string()));
            break;
        }
        let Some((class, home)) = badge else {
            continue;
        };
        found.push((title, class, home));
    }
    found
}

/// The floor under both directions: a scan that matched nothing would satisfy
/// every `for` loop below by iterating over nothing at all.
#[test]
fn the_scan_finds_the_page_and_the_source() {
    let routes = panel_routes();
    assert!(
        routes.len() >= 50,
        "only {} rows with a panel badge found in {PAGE} — is a row still `{ROW}` followed by \
         an `<h3>` and its `rt` badges?",
        routes.len()
    );
    let emitted = emissions();
    assert!(
        emitted.len() >= 5,
        "only {} operations found emitted in {SRC} — the five the Mixer bay's controls emit are \
         the floor, and a scan below it is a scan that has stopped matching code",
        emitted.len()
    );
}

/// **A control reaching past the page.**
///
/// An operation this crate's controls emit whose panel badge is not `has` is a
/// meter that has stopped moving with the thing it measures — ADR-0213's
/// stated failure mode, which is that a badge moved and a figure did not, from
/// the side where the code moved first.
#[test]
fn every_operation_a_console_control_emits_has_a_panel_route_marked_built() {
    let routes = panel_routes();
    for operation in emissions() {
        let title = operation.title();
        let row = routes
            .iter()
            .find(|(row, _, _)| row == title)
            .unwrap_or_else(|| {
                panic!(
                    "a control in {SRC} emits `{title}` and {PAGE} has no row with that heading \
                     — a control reaching an operation nobody specified. The page is the \
                     specification, so add the row there first"
                )
            });
        if UNREACHABLE.contains(&title) {
            continue;
        }
        assert_eq!(
            row.1, "has",
            "a control in {SRC} emits `{title}`, which {PAGE} marks `{}` in the panel column — \
             a control an operator reaches and a page that says no program a player runs does \
             (ADR-0213). Flip the badge, or say here why the control is not reachable",
            row.1
        );
    }
}

/// **[`DRAWN`], held against the page and the source it stands between.**
///
/// Written to fail rather than to pass, on
/// [`the_unreachable_exemption_is_still_the_state_of_the_page`]'s terms: a
/// table that names a row the page no longer marks `read`, or a marker no
/// longer in the source, is a second way to meet a `has` badge that has
/// stopped being met. It is also what stops the table being used on a
/// **write** row, where an emission is the only honest evidence.
#[test]
fn every_drawn_entry_names_a_read_row_the_page_marks_built_and_a_drawing_that_is_there() {
    let reads = reads();
    let routes = panel_routes();
    let src = code_of_src();
    for (title, mark) in DRAWN {
        assert!(
            reads.contains(title),
            "`{title}` is in `DRAWN` and {PAGE} does not mark it `read` — this table is a \
             second way to meet a `has` badge and it is only ever available to a readout. A \
             write reaches an operation by emitting it"
        );
        let row = routes
            .iter()
            .find(|(row, _, _)| row == title)
            .unwrap_or_else(|| panic!("`{title}` is in `DRAWN` and {PAGE} has no row for it"));
        assert_eq!(
            row.1, "has",
            "`{title}` is in `DRAWN` and {PAGE} marks it `{}` in the panel column — an entry \
             here is the evidence behind a built badge, so a row that is not built does not \
             need one and should not carry one",
            row.1
        );
        assert!(
            src.contains(mark),
            "`{title}` is in `DRAWN` naming `{mark}`, and no file under {SRC} contains it — \
             the drawing this badge stands on has gone, or been renamed. The badge is `plan` \
             again, or this marker is"
        );
    }
}

/// **The one exemption, held against the page it exempts.**
///
/// [`UNREACHABLE`] is a list written by hand, so it is written to fail rather
/// than to go stale: an entry nothing emits is an exemption granted to
/// nobody, and an entry whose badge has become `has` is an exemption that has
/// stopped being true — which is what happens the day somebody settles
/// `SetSync`'s record and the row is genuinely reachable. Either way this
/// says so and names the line to delete.
#[test]
fn the_unreachable_exemption_is_still_the_state_of_the_page() {
    let emitted: BTreeSet<&str> = emissions().iter().map(|op| op.title()).collect();
    let routes = panel_routes();
    for title in UNREACHABLE {
        assert!(
            emitted.contains(title),
            "`{title}` is exempted in `UNREACHABLE` and no control in {SRC} emits it — an \
             exemption granted to nobody. Delete the line"
        );
        let (_, class, _) = routes
            .iter()
            .find(|(row, _, _)| row == title)
            .unwrap_or_else(|| panic!("`{title}` is exempted here and {PAGE} has no such row"));
        assert_ne!(
            class, "has",
            "{PAGE} marks `{title}` built in the panel column, and `UNREACHABLE` still says an \
             operator cannot reach it. If the record it owes has been settled, delete the line \
             in `UNREACHABLE` — the exemption has done its job"
        );
    }
}

/// **The page claiming a control that does not exist.**
///
/// It fails apart from the test above because it is a different failure: that
/// one says the console reached past the specification, this one says the
/// specification promises a player a control nothing draws. The home is
/// checked too, in the place `mcp.rs` checks a tool's name — the panel
/// column's badge text is a place on the console rather than an identifier
/// this crate holds, so what is checkable is that a built route names one at
/// all. A `has` badge saying `&mdash;` would be the page asserting an operator
/// reaches it and declining to say from where.
#[test]
fn every_panel_route_the_page_marks_built_is_emitted_by_a_console_control() {
    let emitted: BTreeSet<&str> = emissions().iter().map(|op| op.title()).collect();
    let routes = panel_routes();
    let elsewhere = elsewhere();
    let claimed: Vec<&(String, String, String)> = routes
        .iter()
        .filter(|(_, class, _)| class == "has")
        // The console's own shape is reached through `panel::Op` and through a
        // drag that is no operation at all, so a scan for `Operation::` can
        // only report a built badge there as a control that does not exist.
        // See [`ELSEWHERE`], and `tests/vocabulary.rs` for what does check it.
        .filter(|(title, _, _)| !elsewhere.contains(title))
        .collect();
    assert!(
        claimed.len() >= 5,
        "only {} rows of {PAGE} mark a panel route built — the scan found less than the column \
         holds, which would pass this test by finding nothing",
        claimed.len()
    );
    let reads = reads();
    let src = code_of_src();
    for (title, _, home) in claimed {
        // **A row the page marks `read` may be met by a drawing instead**, and
        // only by one this file can name and find. See [`DRAWN`]: a readout is
        // answered without being asked, so there is no gesture to emit at, and
        // the check is given a second way to be met rather than relaxed.
        let drawn = reads.contains(title.as_str())
            && DRAWN
                .iter()
                .any(|(row, mark)| row == title && src.contains(mark));
        assert!(
            emitted.contains(title.as_str()) || drawn,
            "{PAGE} marks `{title}` built in the panel column, and no control in {SRC} emits \
             it — the page claims a control an operator cannot find. Either the control went \
             and the badge is `plan` again, or it never emitted this operation"
        );
        assert_ne!(
            home, NOWHERE,
            "{PAGE} marks `{title}` built in the panel column and names no home for it — a \
             `has` badge says an operator reaches the operation, so it has to say where the \
             control is"
        );
    }
}
