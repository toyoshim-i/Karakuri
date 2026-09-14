use super::*;

/// The Staging lane's rows, off the deck's own verdicts and the per-node
/// hashes its builds reported — one per node a build changed, or one on the
/// slot where a verdict has no changed node behind it (ADR-0326).
///
/// # It drains, because a `HotSwap`'s events are a caller's to take
///
/// `HotSwap::events` is documented as the caller's — *"a caller that stops
/// draining eventually makes this grow"* — and until this program had a
/// producer there was nothing to drain, so nothing did. `pending_events` is
/// the other reading and is deliberately not what this uses: it is the
/// read-only view `Deck::begin_frame` takes *inside* a frame, and a surface
/// that read it without draining would be the caller bug the engine names.
///
/// So the lane's state is kept here rather than re-derived per frame, and
/// that is what a drain forces and is also what is wanted: the events are a
/// stream of verdicts and a slot's rows are the newest of them.
/// `view::View::staging` is written on the frames a build landed on and left
/// alone on every other, which is the same budget `mixer`'s name is kept to
/// (ADR-0164) with the frames-that-touch-it much rarer.
///
/// # What each verdict does to a row
///
/// - `Swapped`, `Rejected`, `Overloaded`, `SourceRefused` — the slot has
///   rows, on `view::Stage`'s four words. Whether the build is on screen and
///   running is what separates the first three — `Overloaded` is on screen and
///   stopped — and the fourth is the one where there was no build: the checker
///   turned the source down, so the row carries what it said as well as the
///   word (ADR-0310). How many rows is the diff: `Swapped` and
///   `Overloaded` draw one per node the build changed, and `Rejected` and
///   `SourceRefused` draw one on the slot, because a build that did not happen
///   has no node list to hold against the one before it.
/// - `Accepted` — the watchdog says the version held the budget, so the
///   file and the picture agree and that slot's rows leave the lane. It is not
///   the operator's verdict, which is *keep* and is taste rather than cost:
///   the cost verdict is what clears a row an operator has not pressed, and
///   `view::staging` is where that substitution is argued.
/// - `WorkerLost` — the build worker panicked and nothing will be built
///   again this run. No row changes, and that is the reading rather than
///   an omission: every verdict already taken still stands, and there is no
///   verdict outstanding for the worker to have taken with it — one is reached
///   in the call its swap lands in. What is lost is the *next* build, and the
///   lane has never been where that is said — the engine prints it.
///
/// A row is not removed when its slot is parked or its material is
/// replaced. What takes a row off the lane is a verdict in the candidate's
/// favour, and residency does not reach one: a candidate is judged on its own
/// measured cost wherever the slot is (ADR-0313).
///
/// # It writes the transport's health capsule too, and the two are not the
/// same reading
///
/// `view::Transport::health` is *what the last write did* and the lane is
/// *what is still outstanding*, so the two disagree in both directions and
/// both are the mock's: a run in which the last build landed and was then
/// accepted draws `landed` in the transport with an empty lane, and a run in
/// which one slot was stopped while a later one landed draws `landed` in the
/// transport with an `overloaded` row under it. The capsule carries no deck
/// letter, which is why it can only say the second of those and why the lane
/// is where the address is.
///
/// They are written from one drain because there is only one.
/// `Deck::events` empties the channel; a second pass for the capsule would
/// read nothing at all, which is the same sentence the reporter above is
/// written under.
///
/// # It answers whether the live Set changed, because something else has to
/// know
///
/// `true` when a build was installed, which is the moment `Set::published` says
/// a console should re-read a slot — *"A console reads this when a Set lands,
/// not per frame."* [`inspector`] is what acts on it. Every other event
/// answers `false`, the budget's verdict included: since ADR-0316 neither
/// verdict replaces what is playing — one lets the installed Set run and the
/// other stops it where it is — and the install that did replace it was
/// reported by `Swapped` in the same drain.
///
/// # What it costs
///
/// Nothing on a frame nothing was built on. The event drain collects into
/// a `Vec` that does not allocate when it is empty, and every allocation below
/// that is inside the `for` over it.
///
/// On the frame a build lands, which is the frame that also installed a
/// whole Set built on the worker: a `Vec` of the changed addresses, a `Changed`
/// per one of them with its address and its name, and a `view::Candidate` per
/// row. A slot's rows are replaced whole rather than rewritten in place — see
/// [`settle`], where that is argued — so this is a handful of short strings
/// against a Set build, and a run in which nobody saves pays for none of it.
pub(crate) fn staging(
    deck: &mut Deck,
    // **What each slot is playing, and the channel that says what a build was
    // made of.** Here rather than taken up by the caller afterwards, because
    // the rows *are* the diff: a build's per-node hashes against the ones the
    // slot was already on is what says which nodes changed, and both lists are
    // in one hand exactly once — at the moment the build is taken up
    // (`docs/adr/0326-a-staging-row-is-a-changed-node-and-the-row-is-the-keep.md`).
    // A second pass would be a second answer to *what changed*, and the first
    // of the two lists is gone by then.
    keeping: &mut Keeping,
    // **Where each slot's watcher is pointed**, for the names a build's nodes
    // are spelled with — see [`built_nodes`]. A field of `Engine` beside the
    // deck rather than the whole of one, so the deck above can be borrowed
    // mutably at the same call.
    aims: &[Aiming],
    out: &mut Vec<view::Candidate>,
    // **What the transport row's health capsule reads**, off the same drain
    // and for the same reason the reporter is fed from here:
    // `Deck::events` empties the channel, so a second reader that came back
    // for it would find nothing.
    //
    // **Kept across frames rather than rewritten per frame**, which is what
    // makes it a different reading from the lane beside it: a row leaves the
    // lane when nothing is outstanding, and the last verdict there was does
    // not stop having happened. `Verdict::Settled` and `Verdict::Nothing`
    // therefore leave this alone — the watchdog saying a version held the
    // budget is not a write, and a lost build worker is not one either.
    health: &mut Option<view::Stage>,
) -> bool {
    let mut landed = false;
    for slot in 0..deck.slot_count() {
        let addr = EngineSlot(slot as u8);
        // **Taken out of the channel before anything else is asked of the
        // deck.** `Deck::events` borrows the deck for as long as it is being
        // read, and what the rows need afterwards is the Set the swap in this
        // same drain installed — `Set::node_names`, which is what a node is
        // called. An empty drain collects into a `Vec` that allocates nothing,
        // so a frame on which nothing was built pays for this in a branch.
        let events: Vec<Event> = deck.events(addr).collect();
        // **What the build that landed changed, kept across the drain**, so
        // that the `Overloaded` following its `Swapped` draws the same rows:
        // both are about one build and carry its id, and the diff is taken
        // once. Empty on every frame nothing was built.
        let mut diffed: Vec<(u64, Vec<Changed>)> = Vec::new();
        for event in events {
            // **Where the same event goes when somebody who is not at the
            // panel is watching**, and nothing for a run without `--mcp`. A
            // model that wrote a procedure has no other way to learn that its
            // slot was stopped for cost, and *it compiled* is not the same
            // news as *it is on screen and running*.
            //
            // **A checker's refusal goes out with every diagnostic it had**,
            // which is the round trip `docs/principles/0083-…` is about: the
            // reader at this end is a program writing the next attempt, and
            // the row beside it has room for the first line only.
            //
            // **Reported from here rather than from a second drain.**
            // `Deck::events` empties the channel, so a loop that read it again
            // would read nothing at all: the lane and the server are told by
            // one pass or one of them is told by none.
            if let Some(mcp) = keeping.mcp.as_ref() {
                mcp.swap(slot, &event.to_string());
            }
            // **What a swap changed, taken up here.** A `Swapped` is the only
            // event that changes what a slot is running (ADR-0316), so it is
            // the only one that has a diff to take — and the rows it draws are
            // that diff, one per node, named off the Set the swap installed.
            if let Event::Swapped { id, .. } = &event {
                let changed = keeping.took_up(aims, slot, *id);
                diffed.push((*id, changed_rows(deck.slot(addr).set(), &changed)));
            }
            // **Whether the *live Set* changed**, which is a different
            // question from whether a row did and is why this is read here
            // rather than off the rows: a build that landed replaced what is
            // playing, and nothing else does. A refusal changed nothing, and
            // neither verdict changes it either — one lets the installed Set
            // run and the other stops it where it is (ADR-0316).
            landed |= matches!(verdict(&event), Verdict::Waiting(_, view::Stage::Landed));
            // **The diagnostics, where there are any**, taken off the event
            // beside the verdict rather than carried through `Verdict`: that
            // type is `Copy` and is the mapping a CPU test asserts, and a
            // slice on it would make it neither.
            let said: &[String] = match &event {
                Event::SourceRefused { said, .. } => said,
                _ => &[],
            };
            // **Which nodes this verdict is about**, and empty for every
            // verdict that has none — a build that did not happen has no node
            // list to hold against the one before it, and a rebuild whose
            // stack came back identical has an empty diff. `settle` draws one
            // row on the slot for both, which is the same row and the same
            // reason (ADR-0326).
            let changed: &[Changed] = match &event {
                Event::Swapped { id, .. } | Event::Overloaded { id, .. } => diffed
                    .iter()
                    .find(|(built, _)| built == id)
                    .map_or(&[][..], |(_, rows)| rows.as_slice()),
                _ => &[],
            };
            match verdict(&event) {
                Verdict::Waiting(label, stage) => {
                    // **The newest verdict of the drain wins, whichever slot
                    // it came from.** The capsule is one word with no address
                    // on it — `console.html` draws it beside the frame
                    // readout and gives it no deck letter — so what it can
                    // honestly say is *what the last write did*, and one save
                    // of the pair this program watches builds every slot.
                    // Which slot each verdict was about is the lane's, and
                    // which node of it, and that is why both are drawn.
                    *health = Some(stage);
                    settle(out, slot, label, stage, said, changed)
                }
                // Nothing is outstanding on this slot any more, so it has no
                // rows. `retain` rather than an index: a slot has as many rows
                // as its last build changed nodes, and they go together.
                Verdict::Settled => out.retain(|row| row.deck != slot),
                Verdict::Nothing => {}
            }
        }
    }
    landed
}

/// One node a build changed, as a lane row needs it — the address a press is
/// spelled with, the address the row draws, and what that node is called.
///
/// Three fields and not one, because the row and the operation want different
/// halves of the same fact. `view::Candidate::at` is the payload and
/// `view::Candidate::addr` is the string, which is `view::Param`'s arrangement
/// one bay over; both are written here, in one place, so they are filled
/// together or not at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Changed {
    pub(crate) at: karakuri_operation::NodeAddress,
    pub(crate) addr: String,
    pub(crate) name: String,
}

/// The changed nodes of a build, turned into rows — the addresses the watcher
/// reported, resolved against the Set that swap installed.
///
/// The name is `Set::node_names`' and not the build's label, which is the whole
/// of why this needs the Set at all: a label is every node's `proc` name joined
/// with ` + `, and a row that is one node wants that node's own. It is the same
/// name the Inspector writes on a node head — `Set::node_named` turns each one
/// back into the `(layer, index)` this compares against — so the two bays call
/// one node one thing.
///
/// A node the Set does not name is passed over rather than drawn nameless. The
/// two lists are the same build's, so this cannot happen from a rebuild; what
/// it would mean is that the Set and the report disagree about what the slot
/// holds, and a row invented out of that disagreement would be a press aimed at
/// an address nothing can resolve.
pub(crate) fn changed_rows(set: &Set, changed: &[(&'static str, u32)]) -> Vec<Changed> {
    if changed.is_empty() {
        return Vec::new();
    }
    let mut rows = Vec::new();
    for name in set.node_names() {
        let Some((layer, index)) = set.node_named(name) else {
            continue;
        };
        if !changed
            .iter()
            .any(|(at, of)| *at == setfile::kind_name(layer) && *of == index)
        {
            continue;
        }
        rows.push(Changed {
            at: karakuri_operation::NodeAddress {
                layer: asked_layer(layer),
                index,
            },
            addr: node_addr(layer, index),
            name: name.clone(),
        });
    }
    rows
}

/// What one verdict does to the lane — the whole of the mapping, in a function
/// a test can reach without a device.
///
/// A `match` and not a lookup, for [`blend_mode`]'s reason: a sixth
/// `swap::Event` stops the build here rather than being passed over by a
/// wildcard, and *what it does to the lane* is a question the person adding it
/// should have to answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict<'a> {
    /// The slot has a row, under this name and on this word.
    Waiting(&'a str, view::Stage),
    /// The slot has no row: its file and its picture agree.
    Settled,
    /// The lane does not change.
    Nothing,
}

/// See [`Verdict`] and [`staging`], which is where each arm is argued.
pub(crate) fn verdict(event: &Event) -> Verdict<'_> {
    match event {
        Event::Swapped { label, .. } => Verdict::Waiting(label, view::Stage::Landed),
        Event::Rejected { label, .. } => Verdict::Waiting(label, view::Stage::Refused),
        Event::Overloaded { label, .. } => Verdict::Waiting(label, view::Stage::Overloaded),
        // **A row, because the file and the picture disagree** — which is the
        // whole of what this lane is for. Nothing was built, so nothing
        // replaced what is playing and the operator's newest edit is on disk
        // and nowhere else. The word is the checker's and not the build's:
        // `Refused` above is a Set that would not assemble.
        Event::SourceRefused { label, .. } => Verdict::Waiting(label, view::Stage::NotCompiled),
        Event::Accepted { .. } => Verdict::Settled,
        Event::WorkerLost => Verdict::Nothing,
    }
}

/// One slot's rows, put where that slot's rows go.
///
/// A slot's rows are replaced whole rather than rewritten in place, and that is
/// the change ADR-0326 made here. A row used to be one slot, so the slot's row
/// could be found and its three fields overwritten; a row is now one node a
/// build changed, so how many rows a slot has moves with every build — two on
/// the save that touched two files, one on the next, none on a verdict with
/// nothing to diff. Nothing is left behind: the newest verdict is the whole of
/// what this slot has to say, and a row from the build before it would be a
/// node reported as unsettled by a build that has been replaced.
///
/// Slot order, because that is the order the rows are read in — the lane draws
/// them top to bottom and the deck letters are `A` through `D`, so a slot whose
/// verdict arrived later must not sit above one whose arrived first. The splice
/// is over at most `deck::MAX_SLOTS` slots' worth of rows.
///
/// And node order within a slot, which is the order `changed` is in and is the
/// order the Set names its own nodes in — the geometries, the deformers, the
/// cameras, the renderers, the fields. It is the Inspector's pane order read on
/// one slot, so two bays list one slot's nodes the same way round.
pub(crate) fn settle(
    out: &mut Vec<view::Candidate>,
    deck: usize,
    label: &str,
    stage: view::Stage,
    // **What the checker said**, and empty for every verdict that is about a
    // build — see `view::Candidate::said`. Cloned rather than moved because
    // the event is read through `verdict`, which borrows it for the label;
    // it is a handful of short lines on the frame a refusal arrives, which is
    // a frame on which no Set was built.
    said: &[String],
    // **The nodes this build changed**, and empty for a verdict that has none
    // — see [`Changed`] and [`changed_rows`]. Empty draws **one** row on the
    // slot with no address on it, which is the honest answer for a build that
    // did not happen and for a rebuild that restated the stack unchanged: the
    // verdict is about the slot and there is no node to pin it to.
    changed: &[Changed],
) {
    let at = out.partition_point(|row| row.deck < deck);
    let end = at + out[at..].partition_point(|row| row.deck == deck);
    let rows: Vec<view::Candidate> = match changed.is_empty() {
        true => vec![view::Candidate {
            deck,
            at: None,
            addr: String::new(),
            name: label.to_owned(),
            stage,
            said: said.to_vec(),
        }],
        false => changed
            .iter()
            .map(|node| view::Candidate {
                deck,
                at: Some(node.at),
                addr: node.addr.clone(),
                name: node.name.clone(),
                stage,
                said: said.to_vec(),
            })
            .collect(),
    };
    out.splice(at..end, rows);
}
