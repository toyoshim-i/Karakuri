use super::panel_column_common::*;

/// A value of the operation a variant name stands for, so that the row this
/// file looks for is [`Operation::title`]'s answer and never a heading
/// transcribed here.
///
/// The same job `mcp.rs`'s `sample` does for a tool's arguments, and it fails
/// the same way: a control that starts emitting something new arrives as a
/// panic naming the variant, rather than being passed over. The field values
/// are arbitrary — nothing reads them — and the *variant* is compiler-checked,
/// so renaming one in `karakuri-operation` breaks this file at build time.
pub(crate) fn sample(variant: &str) -> Operation {
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
        // The Master bay's effect rows, which name no deck for the master
        // out's reason: the chain reads what the fold produced. One row of
        // *Mixing and output* for all three of them, emitted from a track and
        // from a cut chip — one operation is one row however many controls name
        // it, which is `dedup_by_key`'s case again (ADR-0340). The value is any
        // slot and any setting: the badge claims that an operator reaches the
        // row.
        "SetChainParam" => Operation::SetChainParam {
            at: 0,
            param: karakuri_operation::ChainParam::Cut(karakuri_operation::Cut::Mix),
        },
        // The other two rows of the chain's list: `+ add` and the drop onto
        // the chain both ask for the add, and the `−` at the end of a slot's
        // row asks for the removal. One operation is one row however many
        // controls name it, which is `dedup_by_key`'s case again (ADR-0352).
        "AddChainEffect" => Operation::AddChainEffect {
            procedure: String::new(),
            cut: None,
        },
        "RemoveChainEffect" => Operation::RemoveChainEffect { at: 0 },
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
        // Library bay `read` chip (ADR-0312).
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
        // (`view::Chosen`, ADR-0308). **The payload is the Set the walk is
        // of**, since 2026-09-10: what a walk carries is *which* history, the
        // console holds a deck letter rather than an id, and the host answers
        // it into `view::View::aimed` — which `Chosen::asked` reads on the way
        // out. The value is any Set, because what the badge claims is that an
        // operator reaches the row.
        "WalkHistory" => Operation::WalkHistory {
            set: Some("night01".to_owned()),
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
            node: karakuri_operation::NodeAddress {
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
        // Transport row `rec` pill toggle (ADR-0289).
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
        // **The Sequencer bay's four, and every one of them names the bank it
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
        // **The minus at the end of a lane's row**, addressed by the position
        // the lane is drawn at: the lanes after it move up, so an index names
        // whichever lane holds that position when the press lands (ADR-0352's
        // property, one bay along).
        "RemoveLane" => Operation::RemoveLane {
            pattern: 0,
            lane: 0,
        },
        // **A bank pill in the bay head**, naming a bank and never a
        // direction: with four fixed banks a press on an empty one is the
        // mock's `+`, which is why there is no second operation for it
        // (ADR-0320, ADR-0327).
        "SelectPattern" => Operation::SelectPattern { pattern: 0 },
        // **The Inspector's three, and the two addresses are the two facts.**
        // A node head's chip names a `NodeAddress`, a sensitivity chip names a
        // `BindAt` — a layer, one node of it or all of them, and a key —
        // because an attachment is one layer's where a value's wildcard names
        // no layer at all (ADR-0319).
        //
        // **`SetAuthority` names a destination and never a step**: the press
        // asks for the level it lands on, which is `SetStep`'s arrangement
        // above and P-0090's requirement.
        "SetAuthority" => Operation::SetAuthority {
            deck: 0,
            node: karakuri_operation::NodeAddress {
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
        // Inspector pane deck head capacity/seed chips (ADR-0328) and wiring cards (ADR-0305, ADR-0329).
        "WireInput" => Operation::WireInput {
            deck: 0,
            node: "swirl_warp".to_owned(),
            slot: "far".into(),
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
            node: karakuri_operation::NodeAddress {
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
        "SetSolo" => Operation::SetSolo {
            deck: 0,
            solo: true,
        },
        "SetMute" => Operation::SetMute {
            deck: 0,
            mute: true,
        },
        other => panic!(
            "`{SRC}` constructs `Operation::{other}` and this file has no value for it — a \
             control started emitting an operation nobody accounted for. Add an arm here, and \
             then decide whether that row's panel badge in {PAGE} is now `has`"
        ),
    }
}

/// The address an attachment carries — a layer, a node of it, and a key.
pub(crate) fn bind_at() -> karakuri_operation::BindAt {
    karakuri_operation::BindAt {
        layer: karakuri_operation::Layer::L1,
        index: Some(0),
        key: "turbulence".to_owned(),
    }
}
