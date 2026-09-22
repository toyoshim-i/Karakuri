use karakuri_operation::gate::Allowed;
use karakuri_operation::Operation;

use super::history::*;
use super::operate::*;
use super::procedure::*;
use super::set::*;
use super::wire::*;
use super::Called;
use crate::State;

/// What this surface can say of one operation, and why it cannot where it
/// cannot.
///
/// The `operate` tool takes an operation of `karakuri-operation` by its own
/// name — the heading `docs/manual/operations.html` specifies it under — and
/// hands it to the frame the panel performs every other surface's presses on.
/// It does not take all sixty-four, and the four reasons it does not are here
/// rather than in four scattered refusals
/// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).
///
/// No wildcard arm. [`sayable`] is a `match` over every variant, which is
/// `karakuri_operation::gate::standing`'s discipline and its reason: a
/// sixty-fifth operation does not compile until somebody has said whether this
/// surface can name it, and the page's MCP column cannot quietly go stale
/// beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sayable {
    /// `operate` takes it. The audit still answers.
    Operable,
    /// This server publishes a tool of its own for it, which does something only
    /// the server can — a file, a store, a listing. A second spelling of a tool is
    /// a second spelling
    /// ([P-0087](../../../docs/principles/0087-name-the-property-never-the-shape.md)),
    /// so `operate` refuses it and names the tool.
    Tool(&'static str),
    /// A model has no window. The row's MCP badge is `gap` and the sentence is
    /// [ADR-0315](../../../docs/adr/0315-a-model-has-no-window-so-the-twelve-surface-rows-mcp-badges-are-gap.md)'s,
    /// worded once here as it is worded once on the page.
    Window,
    /// This surface will not reach it, and the clause says why and where the route
    /// that does is. The row's MCP badge is `gap`, and what makes it `gap` rather
    /// than `plan` is that nothing is owed: no performer moving onto the drain's
    /// frame would change it, because what stops it is the shape of this protocol
    /// rather than a gap in this program
    /// ([ADR-0341](../../../docs/adr/0341-a-route-that-answers-is-built-and-a-send-that-ends-in-a-dialog-is-gap.md)).
    ///
    /// There was a sixth answer beside this one until 2026-09-10 — `Unperformed`,
    /// *the vocabulary names it and nothing on this frame performs it yet*, which
    /// is what a `plan` badge in the MCP column meant. It went when its last row
    /// did (ADR-0341): every operation this vocabulary names either has a
    /// performer, has a tool, is a window's, is unsettled, or is this. A `plan`
    /// badge in that column is now only an `Undecided` payload, and the day a row
    /// is added that a surface can say and the frame cannot perform, this `match`
    /// has no wildcard and stops the build until somebody puts the answer back.
    Never(&'static str),
}

/// Whether `operate` names this operation, and what it says where it does not.
/// Exhaustive, with no wildcard arm — see [`Sayable`].
pub(crate) fn sayable(operation: &Operation) -> Sayable {
    match operation {
        // ----- the thirty `operate` takes ----------------------------------
        //
        // Twenty-eight of them are refused by the audit until an operator opens
        // their class, which is a built route and not a missing one
        // (ADR-0235: *"a closed class is reached and answered with a refusal"*).
        // `Operation::RestoreProcedure` is the one the audit lets through and
        // the panel then performs, and `Operation::LoadSet` is the one whose
        // class is a predicate over its target.
        Operation::TapBeat
        | Operation::ScaleGrid { .. }
        | Operation::SetLatencyOffset { .. }
        | Operation::SetSync { .. }
        | Operation::ScrubDeck { .. }
        | Operation::SetFreeRunTempo { .. }
        | Operation::AttachBeatSource { .. }
        | Operation::SetResidency { .. }
        | Operation::LoadSet { .. }
        | Operation::SetCompositing { .. }
        | Operation::SetGain { .. }
        | Operation::SetOpacity { .. }
        | Operation::SetMute { .. }
        | Operation::SetSolo { .. }
        | Operation::ClearSolo
        | Operation::SetOnline { .. }
        | Operation::SetBlendMode { .. }
        | Operation::FadeDeck { .. }
        | Operation::Crossfade { .. }
        | Operation::Wipe { .. }
        | Operation::SetMaskShape { .. }
        | Operation::SelectRenderer { .. }
        | Operation::SetMasterOut { .. }
        | Operation::SetChainParam { .. }
        | Operation::AddChainEffect { .. }
        | Operation::RemoveChainEffect { .. }
        | Operation::SetTonemap { .. }
        | Operation::SetExposure { .. }
        | Operation::WriteParam { .. }
        | Operation::AttachSignal { .. }
        | Operation::TakeParamBack { .. }
        | Operation::SetProperty { .. }
        | Operation::SetAuthority { .. }
        | Operation::RestoreProcedure { .. }
        // **The five ADR-0334 left `plan` for want of a performer on this
        // frame, and each of them has one now** (ADR-0341). Four were already
        // written and sat in the window's own press arm — the star's refusal,
        // the projector's window, the `rec` pill's two ends and the publish
        // mark's re-aim — and `App::operated` calls them where the pointer's
        // button-up arm calls them; the mask position was a missing reading and
        // is one arm of `crates/karakuri`'s `reading`. They leave this list by
        // having a performer rather than by this rule bending, which is the
        // shape ADR-0334 said each of them would leave in.
        | Operation::SetMaskPosition { .. }
        | Operation::Publish { .. }
        | Operation::SetFavourite { .. }
        | Operation::RouteFrame { .. }
        | Operation::RecordSession { .. }
        // **And ADR-0338's load, which left the `plan` column on 2026-09-10 for
        // the reason *Narrow the published interface* did**: its performer was
        // never missing. `overlaid` sits in `App::performed`, in the arm the
        // fold and the library load are in, and this drain lands there — so
        // the sentence this row used to carry, *nothing re-aims a slot with
        // one layer replaced yet*, had stopped being true before it was read.
        | Operation::LoadProcedure { .. }
        // **And ADR-0338's keep, which left the `plan` column on 2026-09-10 by
        // gaining a performer.** `App::operated` hands it to
        // `Keeping::keep_procedure` where the pointer's button-up arm hands
        // the capsule's press to the same method, and the two differ in one
        // argument: a model's is `Asked::Model` and lands in
        // `<store>/sandbox/`, stamped and overwriting nothing, where an
        // operator's own act writes `<store>/procedures/`
        // (`docs/principles/0096-…`, `docs/adr/0261-…`).
        //
        // **A model is not refused here where its star is**, and ADR-0301 is
        // why: a favourite has no sandbox form to land in and a kept procedure
        // is a file, so it has one.
        //
        // **The answer arrives when the file does.** A keep is *"on a
        // worker"*, so the drain hands the reply to the write thread rather
        // than saying *performed* on the frame it arrived on — which is
        // `save_set`'s own arrangement one file kind along.
        | Operation::KeepProcedure { .. }
        | Operation::Quit => Sayable::Operable,

        // ----- the seven that have a tool of their own ---------------------
        Operation::ReadProcedure { .. } => Sayable::Tool("read_procedure"),
        Operation::WriteProcedure { .. } => Sayable::Tool("write_procedure"),
        Operation::WireInput { .. } => Sayable::Tool("wire_input"),
        Operation::SwapOutcome => Sayable::Tool("swap_outcome"),
        Operation::SaveSet { .. } => Sayable::Tool("save_set"),
        Operation::ReadSet { .. } => Sayable::Tool("read_set"),
        Operation::ListSets { .. } => Sayable::Tool("list_sets"),
        // **The eighth, and it joined the seven on 2026-09-10 by having its
        // payload settled** — it names the Set it is a walk of now, so this
        // surface can say it. It is a tool rather than an `operate` name for
        // the property that puts the other seven here and not for its shape: it
        // reads the store and answers with rows, which is what only this server
        // can do, and a walk performed on the drain's frame would be a
        // directory walk on the path that must not wait. See
        // `docs/adr/0342-…`.
        Operation::WalkHistory { .. } => Sayable::Tool("walk_history"),

        // ----- the seventeen a model has no window for ---------------------
        //
        // ADR-0315's twelve and the sequencer's six, which carry that record's
        // sentence on the page for the same reason: a route into a surface's
        // own state is a route into a window the model is not looking at.
        Operation::SelectDeck { .. }
        | Operation::SelectScope { .. }
        | Operation::SetTransition { .. }
        | Operation::KeepCandidate { .. }
        | Operation::FoldBay { .. }
        | Operation::FoldPane { .. }
        | Operation::Unfold { .. }
        | Operation::Solo { .. }
        | Operation::ResetArrangement
        | Operation::SaveArrangement { .. }
        | Operation::RestoreArrangement { .. }
        | Operation::SizeWindow { .. }
        | Operation::SetStep { .. }
        | Operation::SetLaneMute { .. }
        | Operation::PointLane { .. }
        // `RemoveLane` is the sixth of them and joins them for their sentence:
        // taking a lane out of a pattern is a move inside one console's window.
        | Operation::RemoveLane { .. }
        | Operation::SetPatternGrid { .. }
        | Operation::SelectPattern { .. }
        // **A bay's own narrowing of what it is drawing**, which is
        // `Operation::SelectScope`'s answer arrived at from the other side:
        // which kinds of row the Library bay shows is a fact about a window,
        // and a model is not looking at one. What a model actually wants here
        // is a listing that holds procedures at all, and that is
        // `Operation::ListSets`' to grow rather than this row's
        // (`docs/adr/0338-a-procedure-is-a-row-of-the-library-and-one-loaded-over-a-layer-makes-a-set-with-no-name.md`).
        | Operation::FilterLibrary { .. }
        // **A pane's target is a pointer inside one console**, like the deck
        // selection two groups up and for its sentence.
        | Operation::PointPane { .. } => Sayable::Window,

        // ----- the group that emptied on 2026-09-10 ------------------------
        //
        // **There were three here — the payloads the vocabulary had not
        // settled — and all three left on one day** (`docs/adr/0342-…`).
        // `Operation::WalkHistory` left by having its payload settled and is a
        // tool above. The other two left by being asked the question this
        // classification is actually about: **not** *is the payload settled*
        // but *what can this surface reach*. `Operation::MoveBoundary` is a
        // surface's own state and is [`Sayable::Window`] with ADR-0315's
        // sentence — it was that record's own thirteenth row, held out of it
        // only because its payload is open. `Operation::WatchFiles` names an
        // event a model does not perform and is [`Sayable::Never`] below.
        //
        // **So `Sayable::Undecided` is gone**, the way `Unperformed` went in
        // ADR-0341 and for its reason: a variant nothing constructs is dead
        // code that says something false about the program, and this `match`
        // has no wildcard, so the day an operation arrives that a surface
        // cannot say the build stops until somebody puts an answer back.
        //
        // `Operation::SelectScope` is here for the window's reason too, which
        // is where it has always been.
        Operation::MoveBoundary { .. } => Sayable::Window,

        // ----- the two no route here will ever take -------------------------
        //
        // **This was the last group and it used to have a neighbour**: rows
        // the vocabulary named that nothing on the drain's frame performed
        // yet, which is what a `plan` badge in the MCP column meant.
        // `SetProperty` left it when the Inspector's deck head grew the two
        // chips that perform it (ADR-0328), the five ADR-0334 named followed
        // on 2026-09-10, and ADR-0338's two went the same day (ADR-0341) — so
        // that group and its answer are gone, and this row is what is left:
        // the one that is not waiting for anything.
        Operation::TransferSet { .. } => Sayable::Never(
            "both halves of it are outside what this protocol carries. A send names no \
             destination and never will — it is a read, and a read's answer goes where the \
             surface that asked puts answers, which on the panel is the system's own save \
             dialog and a model cannot answer one. A take names a file, and paths never cross \
             this protocol. The route is the command line: `--package ID > FILE.kbset` sends \
             one and `--take-in FILE` takes one in",
        ),
        // **A model does not edit a file in another program**, which is the
        // whole of what that row names: somebody saving a source a watcher is
        // looking at, and the rebuild that follows. What a model has instead is
        // the act itself — `write_procedure` **is** its edit — so there is
        // nothing here it would say that the write does not already say, and
        // nothing is owed. That is `docs/adr/0205-…`'s kind of answer: a route
        // whose only content is a second spelling of one that exists is not a
        // route somebody has yet to build (`docs/adr/0342-…`).
        Operation::WatchFiles { .. } => Sayable::Never(
            "a model does not edit a file in another program: it calls `write_procedure`, \
             which **is** its edit — checked, written, built on a worker and swapped at a \
             frame boundary — and there is nothing this row would add to it. The row names \
             an event rather than an act, somebody saving a source a watcher is looking at, \
             and the watching itself is the command line's: `--watch` turns it on for a run",
        ),
    }
}

/// One named operation, done.
///
/// It takes an [`Allowed`] and not an [`Operation`], which is the audit made
/// structural. `karakuri_operation::gate::audit` is the only thing that builds
/// one and its field is private to that crate, so there is no way to reach this
/// function with an operation nobody checked — a path that skipped the gate
/// does not compile rather than passing review.
/// [ADR-0235](../../../docs/adr/0235-mcp-reaches-every-operation-and-what-could-stop-the-show-is-refused-until-the-operator-opens-it.md):
/// *"an audit skipped on one path is the whole mechanism gone."*
///
/// The dispatch is over the vocabulary rather than over the tool's name, which
/// is the whole of what routing buys this surface: the arm that reads a
/// procedure is chosen by [`Operation::ReadProcedure`], so a tool renamed on
/// the wire goes on doing what its operation says, and a tool that named a
/// different operation would visibly do something else.
///
/// The last arm cannot happen — [`asked`](super::asked) builds seven operations and this
/// matches those seven. It is written out rather than left to a wildcard for
/// [`absent`]'s reason: the arm that cannot happen is the one that stops saying
/// so quietly when the shape around it changes, and if an eighth tool ever
/// arrives without an arm here the client is told which operation nothing
/// performs rather than being answered by the wrong one.
pub(crate) fn perform(allowed: &Allowed<'_>, state: &mut State) -> Called {
    match allowed.operation() {
        Operation::ReadProcedure { deck, node } => {
            Called::Answered(read_procedure(*deck, *node, state))
        }
        Operation::WriteProcedure { deck, node, source } => {
            Called::Answered(write_procedure(*deck, *node, source, state))
        }
        Operation::SwapOutcome => Called::Answered(swap_outcome(state)),
        // **Refused here for what this server can decide and waited for
        // elsewhere**, which is `save_set`'s shape and for the same reason: the
        // wiring a slot rebuilds with is the render loop's, and the names in it
        // are the Set's.
        Operation::WireInput {
            deck,
            node,
            slot,
            to,
        } => match wire_input(*deck, node, slot.as_str(), to, state) {
            Ok((news, note)) => Called::Wiring { news, note },
            Err(refusal) => Called::Answered(Err(refusal)),
        },
        // Answered here like a read and unlike `save_set`: a card is a file, the
        // render loop does not hold one, and there is nothing to wait for.
        Operation::ReadSet { id } => Called::Answered(read_set(id, state)),
        // A directory read and a file read per set, and nothing else — see
        // [`list_sets`]. Answered here for the same reason `read_set` is.
        Operation::ListSets { holds, layer } => {
            Called::Answered(list_sets(holds.as_deref(), *layer, state))
        }
        // **The store's other listing, and no file is opened at all** — see
        // [`walk_history`]. Answered here for `list_sets`' reason, and it is
        // the reason this row is a tool rather than a name `operate` takes:
        // handing a directory walk to the render loop would put it on the path
        // that must not wait.
        Operation::WalkHistory { set } => Called::Answered(walk_history(set.as_deref(), state)),
        // **Refused before it is sent and waited for elsewhere.** Everything
        // this module can decide by itself — a slot that does not exist, an `id`
        // that is not a name — was decided in [`asked`] under the lock like any
        // other tool's arguments, and only the wait for somebody else's thread
        // is deferred.
        Operation::SaveSet { deck, id } => match save_set(*deck, id.as_deref(), state) {
            Ok(news) => Called::Saving(news),
            Err(refusal) => Called::Answered(Err(refusal)),
        },
        // **Everything `operate` names, and it is one arm because it is one
        // route.** The seven above are matched by their own operations, so
        // nothing reaches here that has a tool of its own; what does reach here
        // has been through [`operated`], which took it only if [`sayable`] says
        // this surface can name it, and then through [`audited`]. So the answer
        // is *hand it to the frame every other surface's presses are performed
        // on* — and the last arm is the one that cannot happen, kept for
        // [`absent`]'s reason.
        other => match sayable(other) {
            Sayable::Operable => match operate(other, state) {
                Ok(news) => Called::Operating(news),
                Err(refusal) => Called::Answered(Err(refusal)),
            },
            _ => Called::Answered(Err(format!(
                "`{}` is an operation this server publishes no tool for",
                other.title()
            ))),
        },
    }
}
