use super::*;

// ---------------------------------------------------------------------------
// The Staging lane
// ---------------------------------------------------------------------------

/// The word at the head of the Staging lane, in the source's own capitalisation
/// for [`Kind::Bay`]'s reason: the mock upper-cases in CSS, and that is done at
/// paint time so the word a reader searches for is the word in the source.
pub(super) const STAGING_TITLE: &str = "Staging";

/// Where a candidate stands, in the four words `console.html` uses for it —
/// *"Whether it is on screen: landed, overloaded for costing more than one
/// frame may, refused, or did not compile"*.
///
/// # Two readers, and one set of words
///
/// This is the Staging lane's [`Candidate::stage`] and the transport row's
/// [`Transport::health`]. The page gives them one sentence each and they are
/// the same four answers, so a second enum for the capsule would be
/// `docs/contributing.md` §4's *a name meaning two things*. What differs is
/// which verdict each is showing, not what a verdict is: a lane row is a node a
/// build changed whose verdict is still outstanding, and it leaves on
/// `Accepted` with the rest of its slot's; the capsule is the last verdict
/// there was and stands after the lane empties. One build's verdict can be on
/// several rows at once, which is a fact about how many nodes it changed and
/// not about the verdict (ADR-0326).
///
/// # One variant per `swap::Event` a verdict is outstanding on, and no fifth
///
/// `karakuri_engine::swap::Event` has six variants and this has four. The two
/// that are not here are the two that leave nothing outstanding: `Accepted` is
/// the watchdog saying the version held the budget, at which point the file and
/// the picture agree and the row leaves the lane; and `WorkerLost` is about the
/// *worker* rather than about a version — nothing will be built again, and no
/// candidate changed state when it happened.
///
/// `Refused` and `NotCompiled` are two states and not two spellings, which is
/// the page's own distinction: *"Refused on a row is a build that failed"* —
/// the files checked and the Set they were assembled into would not build —
/// against a `.kir` the *checker* turned down, where there was never a Set to
/// build. The second of them reached no row at all until 2026-09-08, because
/// `karakuri_environment::watch::Watch::poll` printed its diagnostics and
/// answered *nothing to build*; it now answers
/// `karakuri_engine::swap::Polled::Refused` and the deck reports it
/// (`docs/adr/0310-a-source-can-say-it-refused-and-the-lane-draws-it.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// `Event::Swapped` — the build is the live Set and is on screen.
    ///
    /// There is no fourth word for "being judged", and there is no longer anything
    /// to name one after. The watchdog used to hold a candidate on trial for eight
    /// warmup and thirty judged frames, which
    /// `karakuri_engine::swap::HotSwap::on_trial` reported; the page's row said
    /// nothing about it, on the reading that a version being judged is on screen
    /// and this row says what is on screen. ADR-0313 removed the trial — the
    /// verdict is reached on the candidate's own measured cost, in the same call
    /// the swap lands in — so a row on this word is a build that landed and is
    /// running, and a build that landed and was stopped goes straight to
    /// [`Stage::Overloaded`] in the same drain.
    Landed,
    /// `Event::Overloaded` — the row this lane most needs to draw. The version
    /// costs more than one frame may, so it stayed in the slot and the slot stopped
    /// updating: no step, no draw, and the target holding the last image it made,
    /// which the mix and that deck's cell go on reading.
    ///
    /// Nothing was put back, and that is why the row matters more than it used to.
    /// The watchdog used to restore the previous *Set* and could not restore the
    /// previous *file*, so the picture and the disk disagreed silently; now the
    /// picture is a still of the version the operator asked for, which looks like
    /// working material until something says otherwise. Two things say it — this
    /// row, and that deck's caption ([`PREVIEW_OVERLOADED`]) — and it stands until
    /// the operator fades the slot out, lands an earlier version, or saves
    /// something that fits (ADR-0316).
    Overloaded,
    /// `Event::Rejected` — the build failed and nothing changed: the running Set is
    /// still running, with its `t` and its live count untouched, and the disk holds
    /// material that does not assemble.
    Refused,
    /// `Event::SourceRefused` — the checker turned the source down, so nothing was
    /// built at all: no Set, no candidate, and no version, because the edit history
    /// is gated on compiling
    /// (`docs/adr/0089-history-is-gated-on-compiling-not-on-landing.md`). The
    /// picture is whatever was already playing and the disk holds material that
    /// does not check.
    ///
    /// It is the one row that carries a sentence — see [`Candidate::said`], which
    /// is why.
    NotCompiled,
}

impl Stage {
    /// The word drawn at the far end of a candidate row, and the word in the
    /// transport's health capsule, which is `console.html`'s and the mock's own:
    /// the capsule names the same four answers in the same words — *"the other
    /// answers are overloaded, failed to build, and did not compile"*.
    ///
    /// That capsule is in the transport row and this said *the deck head* until
    /// 2026-09-08, which was wrong rather than stale: there is no `landed` pill on
    /// a deck head anywhere in the mock, and the one the sentence is quoting is
    /// `.transport`'s.
    pub fn word(self) -> &'static str {
        match self {
            Stage::Landed => "landed",
            Stage::Overloaded => "overloaded",
            Stage::Refused => "refused",
            Stage::NotCompiled => "did not compile",
        }
    }
}

/// One candidate, which is one node a build changed and nobody has ruled on —
/// or, where no node can be named, one deck slot whose file no longer agrees
/// with its picture.
///
/// # A row is a changed node, and the caller is what knows which
///
/// A `swap::Event` carries an `id` and a `label` and nothing else about *what*
/// was built, because a `karakuri_engine::Request` restates every node of a
/// slot — so a verdict is over a build rather than over a node. Which nodes
/// that build actually *changed* is a diff of the hashes a build reports
/// against the ones the slot was already playing, and the caller is where both
/// lists are in one hand. So a build that changed two nodes is handed in as two
/// of these, carrying one verdict each
/// (`docs/adr/0326-a-staging-row-is-a-changed-node-and-the-row-is-the-keep.md`).
///
/// [`at`](Self::at) is `None` where no node can be named, and that is a state
/// rather than a gap: `Event::Rejected` and `Event::SourceRefused` are builds
/// that did not happen, so there is no new node list to hold against the old
/// one, and a rebuild that restated the stack without changing any of it has an
/// empty diff. Such a row is the slot's, draws no address, and offers neither
/// press — both name a node.
///
/// [`said`](Self::said) is here on the same rule rather than against it:
/// `Event::SourceRefused` carries the diagnostics, so they are a thing the wire
/// has and not a thing this crate derives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// Which deck slot the verdict is about, drawn as [`DECK_LETTERS`]' letter.
    ///
    /// It is not the node, and it is not a stand-in for one: the node is
    /// [`at`](Self::at), and this is the address the *verdict* has. Both are drawn,
    /// because a node address is per slot — two decks running one Set have the same
    /// `L4:0` — and both are needed to spell the operation a press on this row asks
    /// for.
    ///
    /// It is drawn because a label does not say where. The program this panel is
    /// drawn by runs every one of its [`DECKS`] slots from its own copy of the
    /// material — `working_copies` in `karakuri/src/main.rs`, and
    /// `docs/manual/console.html`'s *Every deck runs from its own copy* — so a save
    /// reaches the one deck whose file it is. What it does not do is make the
    /// labels distinct: four slots opened on one preset build four labels that are
    /// the same string, because a label is every node's `proc` name joined and
    /// those are the same procedures. So the label says what was built and only
    /// this says where.
    ///
    /// And where is the reading this lane exists for, though no longer for the
    /// reason it was written with. A trial used to be *frozen* on a slot that was
    /// not being drawn, so a load into a parked deck left a verdict outstanding and
    /// a row in the lane until that deck went on air; ADR-0313 removed the trial,
    /// and an off-air slot's candidate is now judged at the install like any other
    /// (see [`Stage::Landed`]). What the letter is for is unchanged and is the
    /// sentence above it: four slots opened on one preset build four rows whose
    /// labels are the same string, so this is the only thing in the instrument that
    /// says which deck to look at — and *put a node's previous version back* is
    /// likewise an act on one slot.
    ///
    /// This said the program *"plays one pair of files in both its slots, so one
    /// save produces two builds whose labels are the same string"* until
    /// 2026-09-08. That was two slots sharing one pair, which was a defect in the
    /// program rather than a property of the lane; the copies ended it, and the
    /// argument for the letter is the parked deck above.
    pub deck: usize,
    /// Which node of that slot the build changed, and `None` where no node can be
    /// named — see the head of this type.
    ///
    /// It is the payload half and [`addr`](Self::addr) is the drawn half, which is
    /// [`Param`]'s arrangement one bay over: `Param::name` is what the row reads
    /// and `Param::param` is what a press asks with. Both are written by whoever
    /// read the Set, in one place, so they are filled together or not at all.
    ///
    /// It is what both of this row's presses are addressed by. `Keep a candidate`
    /// settles this node and `Put a node's previous version back` steps it back one
    /// version; neither can be spelled without it, so a row where this is `None`
    /// offers neither.
    pub at: Option<NodeAddress>,
    /// The mock's `.addr` — `L4:0` — and empty on a row that names no node.
    ///
    /// Written by whoever read the Set, exactly as [`Node::addr`] is and for its
    /// reason: the layer names are `karakuri-ir`'s `Kind` and this crate depends on
    /// neither it nor the engine.
    pub addr: String,
    /// What the node's procedure calls itself — `Set::node_names`' entry for
    /// [`at`](Self::at), which is the same name the Inspector writes on a node
    /// head, and the harness's word rather than this crate's for [`Strip::name`]'s
    /// reason.
    ///
    /// On a row that names no node it is the build's own label — `Request::label`,
    /// every node's `proc` name joined with ` + ` — because that is the finest
    /// thing such a verdict has. A checker's refusal is the one that is finer than
    /// the build: it is about one file, and the label it hands over is that file's
    /// name.
    ///
    /// Clipped rather than elided where it does not fit, which is the mock's own
    /// answer: `.cand` sets no `text-overflow` where `.strip-name` and `.path` both
    /// do. An empty string draws no name at all.
    pub name: String,
    /// Whether it is on screen — [`Stage`].
    pub stage: Stage,
    /// What the checker said, one line per diagnostic, and empty on every stage but
    /// [`Stage::NotCompiled`].
    ///
    /// # Why this row carries a sentence when no other one does
    ///
    /// The three verdicts above it are about a build the operator can see the
    /// result of: a landed one is on screen, a rolled-back one has the previous Set
    /// on screen, and a refused one names a Set that would not assemble. A source
    /// the checker turned down has produced nothing to look at, so the word alone
    /// tells an operator that a save did not take and nothing whatever about why.
    /// *A refusal carries what the next attempt needs* (`docs/principles/0083-…`),
    /// and on a lane the row is where it can carry it.
    ///
    /// # The whole list, and the row draws the first of it
    ///
    /// Every diagnostic in a file is reported at once so a repair is one round
    /// trip, and the terminal and the MCP surface both get the lot. A row is one
    /// line, so [`staging_into`] draws the first and says how many others there
    /// are. The list is here rather than the first-and-a-count because the count is
    /// the list's own length, and two fields that must agree are two fields that
    /// can stop agreeing.
    ///
    /// Formatted where the file was read, which is the build worker: these are the
    /// strings `karakuri_engine::swap::Refusal` carried, moved through the event
    /// rather than built on the frame they arrive on.
    pub said: Vec<String>,
}

/// The Staging lane, laid out: where the candidate rows go and how many of
/// them there is room for.
///
/// # What is drawn, and it is four of the six things a row could be
///
/// `console.html` specifies a row as four things — the node, what the
/// procedure calls itself, whether it is on screen, and when it arrived — and
/// the mock draws two more with no value behind them. This draws the deck,
/// the node's address, the name and the verdict, and, on the one verdict
/// that has produced nothing to look at, what the checker said
/// ([`Candidate::said`]);
/// [ADR-0200](../../../../docs/adr/0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md)
/// is why the rest is omitted outright rather than drawn hollow.
///
/// The node arrived on 2026-09-09 and it is what a row now *is*.
/// `swap::Event` carries an `id` and a `label` and no node at all, because a
/// `Request` restates every node of the slot and a verdict is therefore over a
/// *build*; which node of that build changed is a diff, and the caller is
/// where both lists are in one hand — `karakuri_environment::watch::Built`
/// carries `(layer, index, hash)` for the whole stack on every build, and
/// consecutive builds differ where the hashes do. So one build that changed
/// two nodes is two rows here, each with the build's one verdict on it
/// (`docs/adr/0326-a-staging-row-is-a-changed-node-and-the-row-is-the-keep.md`).
/// A verdict with no diff behind it — a build that did not happen, or a
/// rebuild that restated the stack unchanged — is one row with no address, and
/// [`Candidate::at`] is where that is argued.
///
/// What is still omitted:
///
/// - The coloured dot, and the `you` in `you, 14:41` — who wrote it.
///   `origin` — the prompt, the model, the seed — is specified in
///   `docs/ir-spec.md` and produced by nothing (*"`origin` and `tag` have no
///   producer"*), so a hand in an editor and a model over MCP are the same
///   save down the same path. The dot goes with it, because the colour *is*
///   the producer.
/// - `14:41` — when it arrived. It waits on what the Library bay's `.dim`
///   column waits on and is refused for its reason: the value would exist and
///   a *spelling* does not. `karakuri_environment::history`'s is `%H%M%S-%3f`,
///   which is half a filename; `karakuri_environment::setfile::written_at` is
///   local to the second; the mock's `14:41` is a third. A fourth written here
///   would be the second answer this repository deletes rather than adds, and
///   it would be the *same* decision the Library is waiting on, taken twice.
/// - The head's `2 waiting`. [`Kind::Bay`]'s pills are static words and
///   are the mock's *controls* only — every readout in a bay head is undrawn
///   for that reason, `previews 3 of 4` included. And the number would say
///   what the rows already say: this lane has no truncation to report, where
///   the Library's foot has (`n of m`), so a count over the rows would be one
///   readout of two values against another of the same one.
/// - The mock's third `.cand`, *a rejected candidate costs nothing*. The
///   page says what that is: a note to whoever is reading the mock, and not a
///   thing the lane draws.
///
/// # The two controls, and which part of the row each is
///
/// The row is `Keep a candidate` and the capsule at its end is `Put a
/// node's previous version back` — [`StagingBay::keep`] and
/// [`StagingBay::back`]. It is the Library's list one bay up, arranged the
/// same way: a row that is itself a control with a smaller box inside it,
/// asked first, which there is [`LibraryBay::starred`] and here is the
/// capsule. What decided which act goes on which target is what each costs —
/// keeping moves nothing, writes nothing and takes a line off a list, and a
/// step back writes over the operator's working copy and rebuilds the slot, so
/// the free one takes the large target and the one that writes takes the small
/// one.
///
/// Neither is offered on a row that names no node ([`Candidate::at`]),
/// because `karakuri_operation::Operation` spells both of them with one, and
/// keeping is not offered on [`Stage::Overloaded`] either: a slot that has
/// stopped is not a candidate an operator is choosing between, and settling it
/// would take away the one row saying the slot is not running (ADR-0316). The
/// capsule *is* offered there — landing an earlier version is one of the three
/// ways out of a stopped slot. Their record questions were settled long before
/// the controls were: `KeepCandidate` is `Written::Silent(Silent::Surface)`
/// and `RestoreProcedure` is `Written::Silent(Silent::OnLanding)`.
///
/// # A row is a changed node, and it leaves when nothing is outstanding
///
/// The rows are the caller's ([`View::staging`]), read off
/// `karakuri_engine::deck::Deck`'s per-slot events and the per-node hashes the
/// builds reported: a slot gets its rows when its newest event is `Swapped`,
/// `Rejected`, `Overloaded` or `SourceRefused`, and loses all of them on
/// `Accepted` — the watchdog saying the version held the budget, which is the
/// one outcome that leaves the file and the picture agreeing.
///
/// What that stands in for is the operator's own verdict, and it is not the
/// same judgement. *Keeping* is taste and the watchdog's verdict is cost;
/// the page is explicit that the two are different questions. With no control
/// to keep with, a row that waited for one would never leave the lane, and a
/// lane that never empties is not the lane the page describes — *"empty is
/// this lane's ordinary state"*. So the cost verdict clears the row, and the
/// day a `Keep` control lands it is what clears it instead.
///
/// A parked slot's row is settled like any other's, and that was not
/// always so. `HotSwap::begin_frame_parked` used to freeze a trial — a slot
/// that was not being drawn was not paying for the frames it would be judged
/// on — so a build landing in a parked slot had a verdict outstanding for as
/// long as the slot stayed off air. A candidate is judged on its own measured
/// cost since ADR-0313, which does not move with residency, so the verdict
/// arrives at the install wherever the slot is.
///
/// # Where it goes
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one. Like [`library`] and unlike [`master`] this asks `egui` for nothing:
/// every box in the row is the row's own width or a text box measured at paint
/// time, so no rectangle here is the width of the type in it.
///
/// `None` where there is no candidate and `None` where there is no room for
/// one: a console with no engine behind it is every test in this crate, and
/// what the bay draws then is its card and its head and nothing at all —
/// *"no row, no placeholder, and no standing sentence"*.
pub fn staging(layout: &karakuri_layout::Layout, candidates: &[Candidate]) -> Option<StagingBay> {
    // **Nothing outstanding on any slot, which is this lane's ordinary
    // state** — and the state every run starts in. Drawing an empty list
    // would be the standing sentence the page refuses.
    if candidates.is_empty() {
        return None;
    }
    staging_box(
        to_egui(layout.rect(layout.find("staging")?)),
        candidates.len(),
    )
}

/// The Staging lane, laid out — see [`staging`] for what is drawn in it and for
/// the six things in the mock's lane and the page's row that are not.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StagingBay {
    /// `.stage-list`'s content box: the region under the bay head, inside
    /// [`size::STAGE_LIST_PAD_TOP`], [`size::STAGE_LIST_PAD_X`] and
    /// [`size::STAGE_LIST_PAD_BOTTOM`], where the rows are laid from the top with
    /// [`size::STAGE_GAP`] between them.
    pub list: Rect,
    /// How many rows are drawn, which is how many fit in [`list`](Self::list) —
    /// never more than [`total`](Self::total), and never zero, because a lane with
    /// no room for one row draws no list at all.
    pub rows: usize,
    /// How many candidates the caller handed over. Carried and not drawn: there is
    /// no foot in this bay to say `n of m` in, and the head's count is not drawn
    /// either ([`staging`]). It is here because a lane that had room for fewer rows
    /// than there are candidates is a fact a test should be able to ask about
    /// without counting shapes.
    pub total: usize,
}

impl StagingBay {
    /// The `index`th row's rectangle, counting from the top of the list.
    ///
    /// Derived rather than stored for [`LibraryBay::row`]'s reason — the rows are a
    /// stride and a count, and a `Vec` of them would be an allocation a frame does
    /// not need — with the one difference that this stride carries a gap:
    /// `.stage-list` is a column flex with `gap: 5px` where `.lib-list` states
    /// none.
    pub fn row(&self, index: usize) -> Rect {
        Rect::from_min_size(
            Pos2::new(
                self.list.min.x,
                self.list.min.y + (size::CAND_H + size::STAGE_GAP) * index as f32,
            ),
            egui::vec2(self.list.width(), size::CAND_H),
        )
    }

    /// The `back` capsule of the `index`th row, or `None` where that row does not
    /// offer one and `None` where the row has no space for it.
    ///
    /// Laid out from the row's right-hand padding, back past the verdict and one
    /// [`size::CAND_GAP`], which is where [`staging_into`] paints it — one
    /// derivation for the painted capsule and the pressed one, which is every other
    /// control on this console ([`crate::input`]): the derivation that draws a
    /// control is asked a second time rather than copied, so what an operator sees
    /// and what a press lands on cannot come apart.
    ///
    /// `None` where the row would be all capsule, which is [`keep_pill`]'s rule and
    /// [`look`]'s: *a control that does not fit in the row it is drawn in is no
    /// control at all, rather than half of one*. The words to its left are a
    /// readout and are clipped; this is a target and is not drawn where it would be
    /// cut. What it has to leave room for is the deck letter and the address,
    /// because a capsule sitting on top of the address would be a press whose
    /// operand is underneath it.
    ///
    /// Offered wherever the row names a node ([`Candidate::at`]), the overloaded
    /// row included — see [`staging`] for which press is offered where, and why
    /// this one is offered on more rows than the keep is.
    pub fn back_capsule(
        &self,
        ctx: &egui::Context,
        candidates: &[Candidate],
        index: usize,
    ) -> Option<Rect> {
        // Fonts are not valid until `egui` has run a pass, exactly as in
        // [`keep_pill`] — and on the frame before the first one there is
        // nothing drawn here to press.
        if ctx.cumulative_pass_nr() == 0 || index >= self.rows {
            return None;
        }
        let candidate = candidates.get(index)?;
        candidate.at?;
        let row = self.row(index);
        let width = |text: &str, at: f32| {
            ctx.fonts_mut(|f| {
                f.layout_job(span_at(text, at, Color32::PLACEHOLDER))
                    .size()
                    .x
            })
        };
        let verdict = width(candidate.stage.word(), size::CAND_WHO_SIZE);
        let w = pill_width(ctx, BACK_LABEL);
        let capsule = Rect::from_min_size(
            Pos2::new(
                row.max.x - size::CAND_PAD_X - verdict - size::CAND_GAP - w,
                row.center().y - size::PILL_H * 0.5,
            ),
            egui::vec2(w, size::PILL_H),
        );
        // **Against what the row's left-hand end is already using**, which is
        // the deck letter and the address: a capsule that reached back over
        // the address would be drawn on top of the thing it is a control for,
        // and a press would then be aimed by something it is covering.
        let letter = DECK_LETTERS.get(candidate.deck).copied().unwrap_or("?");
        let least = size::CAND_PAD_X
            + width(letter, size::BASE)
            + size::CAND_GAP
            + width(&candidate.addr, size::BASE)
            + size::CAND_GAP;
        (capsule.min.x >= row.min.x + least).then_some(capsule)
    }

    /// A press on the `back` capsule of a row, as the operation it asks for: this
    /// node's previous version, one step and never a cursor.
    ///
    /// The listing goes in with the point, exactly as it does for the Library bay's
    /// rows and for its reason: what a row *is* is a value the caller derived off a
    /// deck and handed over, and this crate holds none of it between frames.
    pub fn back(
        &self,
        ctx: &egui::Context,
        candidates: &[Candidate],
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        let at = egui::pos2(p.x, p.y);
        (0..self.rows).find_map(|index| {
            let capsule = self.back_capsule(ctx, candidates, index)?;
            let node = candidates.get(index)?.at?;
            capsule.contains(at).then(|| Operation::RestoreProcedure {
                deck: candidates[index].deck as u8,
                revision: Revision::Previous(node),
            })
        })
    }

    /// A press on a candidate row, as the operation it asks for: keep this
    /// candidate, which settles the node and takes the row off the lane.
    ///
    /// The capsule inside the row is not part of it, and that is a refusal here
    /// rather than an order at the call site: the row is asked after
    /// [`StagingBay::back`] on every route, but a control that depended on being
    /// asked second would be one press away from doing two things the day somebody
    /// reordered a `match`. [`crate::input::claim`]'s rule 4 is *a control claims
    /// what it acts on and no more*, and what this acts on is the row less its
    /// capsule.
    ///
    /// `None` on a row that offers no keep, which is two cases and one sentence
    /// each: a row with no node has nothing to settle, and a row on
    /// [`Stage::Overloaded`] is a slot that has stopped rather than a candidate
    /// anyone is choosing between (ADR-0316). A press on either is nobody's, and
    /// [`crate::input::claim`] hands it to `egui` exactly as it hands over a press
    /// on the list's own ground.
    pub fn keep(
        &self,
        ctx: &egui::Context,
        candidates: &[Candidate],
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        let at = egui::pos2(p.x, p.y);
        (0..self.rows).find_map(|index| {
            let candidate = candidates.get(index)?;
            let node = candidate.at?;
            if candidate.stage == Stage::Overloaded || !self.row(index).contains(at) {
                return None;
            }
            if self
                .back_capsule(ctx, candidates, index)
                .is_some_and(|capsule| capsule.contains(at))
            {
                return None;
            }
            Some(Operation::KeepCandidate {
                deck: candidate.deck as u8,
                node,
            })
        })
    }
}

/// The word in the capsule at the end of a candidate row, and it is not `keep`:
/// this console already spends that word on the Inspector pane head's capsule,
/// where it names *Keep what a deck is playing* and writes a Set into the
/// library. Two capsules reading `keep` and meaning two acts would be
/// `docs/contributing.md` §4's *a name meaning two things*, and the more
/// expensive misreading of the two is the one where a person presses this
/// expecting a save. The row's own act needs no word at all, because the
/// control is the row.
const BACK_LABEL: &str = "back";

/// The arithmetic of the lane, away from the layout it reads.
///
/// Term for term from `style.css`:
///
/// - `.stage-list { padding: 6px 9px 8px; display: flex; flex-direction:
///   column; gap: 5px }` — what is left under the bay head, inset by those
///   three numbers, with the rows stacked from the top of it.
/// - `.cand { padding: 4px 7px }` — [`size::CAND_H`] each, with
///   [`size::STAGE_GAP`] between one and the next and none above the first or
///   under the last.
///
/// # How many rows fit, and the gap is one fewer than the rows
///
/// `n` rows occupy `n * CAND_H + (n - 1) * STAGE_GAP`, so the count is
/// `floor((h + gap) / (row + gap))` — the standard trick of lending the last
/// row a gap it does not have. At the height the arrangement pins this bay to
/// it is three: 125 less the head's 27 and the list's 6 and 8 is 84, and
/// `(84 + 5) / 29.5` is 3.01. Three is the mock's own lane, which is why the
/// arrangement's 125 was written from three rows and two gaps — so the number
/// of rows this bay has room for and the number its height was derived from
/// are one derivation or neither. At the bay's declared minimum of 66 it is
/// one: 66 less 27, 6 and 8 is 25, and `(25 + 5) / 29.5` is 1.01.
///
/// The half-pixel of slack in both is the arrangement's rounding and not a
/// coincidence: 27 + 14 + 3 × 24.5 + 2 × 5 is 124.5 and the bay is pinned at
/// 125, and 27 + 14 + 24.5 is 65.5 against a minimum of 66. Half a pixel is
/// less than the gap, so neither number buys a row it was not written for.
///
/// `None` where the region cannot hold one row, which is [`picture_rect`]'s
/// rule stated on a list.
fn staging_box(region: Rect, total: usize) -> Option<StagingBay> {
    let list = Rect::from_min_max(
        Pos2::new(
            region.min.x + size::STAGE_LIST_PAD_X,
            region.min.y + size::HEAD_H + size::STAGE_LIST_PAD_TOP,
        ),
        Pos2::new(
            region.max.x - size::STAGE_LIST_PAD_X,
            region.max.y - size::STAGE_LIST_PAD_BOTTOM,
        ),
    );
    // **Narrower than its own padding is no list**, which is
    // [`library::library_box`]'s refusal across the same axis.
    if list.width() <= 0.0 {
        return None;
    }
    let fits = ((list.height() + size::STAGE_GAP) / (size::CAND_H + size::STAGE_GAP))
        .floor()
        .max(0.0) as usize;
    let rows = fits.min(total);
    (rows > 0).then_some(StagingBay { list, rows, total })
}

/// The Staging lane's candidate rows, painted.
///
/// Where everything goes is [`staging`]'s, so this paints and derives nothing.
///
/// Term for term from `style.css`:
///
/// - `.cand` — `background: var(--c-well)` at `border-radius: 8px`, which is
///   the one row in this console that has a well behind it, and
///   `color: var(--c-dim)` for the type in it.
/// - the deck's letter — `pal.faint`, in the place the mock puts its
///   `.dot`: the row's first item, [`size::CAND_GAP`] before the address. It
///   is the letter [`DECK_LETTERS`] gives and the same word the preview cells
///   carry, which the manual calls *"the only thing naming a deck"*.
/// - `.addr` — `pal.lav` at [`size::BASE`], between the letter and the name,
///   which is the mock's own colour for a node address and the one the
///   Inspector's node heads are already drawn in. Empty on a row that names no
///   node, which then draws nothing there and no gap either.
/// - `.pill` — the `back` capsule, laid out by [`StagingBay::back_capsule`]
///   and painted from that same derivation, so what is drawn and what a press
///   lands on are one rectangle. Drawn only where that answers, which is a row
///   with a node and room for it.
/// - `.cand .who` — `pal.faint` at [`size::CAND_WHO_SIZE`], hard against the
///   far end of the row's padding box, which is what `.sep`'s `flex: 1` does
///   to it in the mock. The mock puts the producer there and this puts the
///   verdict, for the reason [`staging`] gives: the producer has no value
///   behind it and the verdict is the whole of what the row is for.
/// - what the checker said, in the same faint and at the same size,
///   between the name and the verdict — drawn only where there is one, which
///   is [`Stage::NotCompiled`] and nothing else. It is not a term from the
///   mock, which draws no such row; it is [`Candidate::said`], and the
///   argument for it is there.
///
/// A name too long for the track is clipped rather than elided, which is
/// the mock's own answer — `.cand` sets no `text-overflow` — and the clip is
/// the list's box, the same `with_clip_rect` the Library's rows are drawn
/// inside. The verdict is painted after the name and inside the same clip, so
/// a name that runs the width of the row is drawn under it rather than over
/// it: the verdict is the one thing in the row that must stay readable.
pub(super) fn staging_into(ui: &Ui, pal: &Palette, bay: &StagingBay, candidates: &[Candidate]) {
    let painter = ui.painter().with_clip_rect(bay.list);
    for (index, candidate) in candidates.iter().take(bay.rows).enumerate() {
        let row = bay.row(index);
        painter.rect_filled(row, size::CAND_RADIUS, pal.well);

        let letter = DECK_LETTERS.get(candidate.deck).copied().unwrap_or("?");
        let deck = painter.layout_job(span_at(letter, size::BASE, pal.faint));
        let after = deck.size().x;
        painter.galley(
            Pos2::new(
                row.min.x + size::CAND_PAD_X,
                row.center().y - deck.size().y * 0.5,
            ),
            deck,
            pal.faint,
        );

        // **The node's address, in the lavender the mock gives `.addr`** — the
        // same colour and the same spelling the Inspector writes on a node
        // head, because it is the same address. A row that names no node draws
        // nothing here and takes no gap for it: an empty galley is zero wide
        // and the gap is added to what the address measured, so the name sits
        // where it sat before this column existed.
        let addr = painter.layout_job(span_at(&candidate.addr, size::BASE, pal.lav));
        let addressed = addr.size().x;
        painter.galley(
            Pos2::new(
                row.min.x + size::CAND_PAD_X + after + size::CAND_GAP,
                row.center().y - addr.size().y * 0.5,
            ),
            addr,
            pal.lav,
        );
        let after = match candidate.addr.is_empty() {
            true => after,
            false => after + size::CAND_GAP + addressed,
        };

        let name = painter.layout_job(span_at(&candidate.name, size::BASE, pal.dim));
        let named = name.size().x;
        painter.galley(
            Pos2::new(
                row.min.x + size::CAND_PAD_X + after + size::CAND_GAP,
                row.center().y - name.size().y * 0.5,
            ),
            name,
            pal.dim,
        );

        // **What the checker said, after the name and in the faint** — see
        // [`Candidate::said`]. The first diagnostic, with a count of the
        // others after it, at the same size the verdict is drawn at: this is
        // the row's second reading and not its first, and a line of `2:8:
        // parse: unknown kind` set in the name's size would read as the
        // material's name.
        //
        // Drawn before the verdict and inside the same clip, so a long
        // diagnostic goes under the word rather than over it — the rule the
        // name is already drawn under, and for its reason: the verdict is the
        // one thing in the row that must stay readable.
        if let Some(first) = candidate.said.first() {
            let rest = candidate.said.len() - 1;
            let text = match rest {
                0 => first.clone(),
                1 => format!("{first} · 1 more"),
                more => format!("{first} · {more} more"),
            };
            let said = painter.layout_job(span_at(&text, size::CAND_WHO_SIZE, pal.faint));
            painter.galley(
                Pos2::new(
                    row.min.x + size::CAND_PAD_X + after + size::CAND_GAP + named + size::CAND_GAP,
                    row.center().y - said.size().y * 0.5,
                ),
                said,
                pal.faint,
            );
        }

        let word = painter.layout_job(span_at(
            candidate.stage.word(),
            size::CAND_WHO_SIZE,
            pal.faint,
        ));
        painter.galley(
            Pos2::new(
                row.max.x - size::CAND_PAD_X - word.size().x,
                row.center().y - word.size().y * 0.5,
            ),
            word,
            pal.faint,
        );

        // **The `back` capsule, from the derivation a press is answered
        // from** — [`StagingBay::back_capsule`], asked here rather than laid
        // out a second time, so the capsule an operator sees and the capsule a
        // press lands on are one rectangle. It is drawn last because it is the
        // one thing in the row that is a target: a name long enough to reach
        // it goes under it rather than over it, which is the rule the verdict
        // above is already drawn under.
        if let Some(capsule) = bay.back_capsule(ui.ctx(), candidates, index) {
            pill_at(ui, pal, capsule, BACK_LABEL);
        }
    }
}
