use super::*;

/// **What the mixer strips read this frame**: one per slot the deck has, out
/// of the six things a `Deck` will say about a slot.
///
/// Everything here is reachable from a `Deck` and nothing reaches around one:
/// `slot_count`, `residency`, `gain`, `opacity`, `blend`, `mask` and `level`
/// are its own, and this takes a `&Deck` because it is on the side of the seam
/// that is allowed one — what crosses into the console is a name, a word and
/// four numbers (ADR-0156).
///
/// # Where each one comes from, and the two that are not the deck's
///
/// - **The tally** is `Deck::residency`, which is the **effective** residency
///   the frame loop reads and not `requested_residency`. The governor moves a
///   slot down without anybody asking, and a tally showing the request would
///   be describing a slot that is doing something else.
/// - **And the request beside it** — `Deck::requested_residency`, which is the
///   other half of the same pair. Both are handed over and **neither side
///   computes `Deck::is_parked`**: the engine has the predicate and the
///   console derives its own from the two values (`view::Strip::pending`), so
///   what crosses the seam stays a residency and a residency rather than
///   becoming a bit whose meaning is written down in only one of the two
///   crates. It is the same reading as the four numbers below — the deck says
///   what it is doing, and the surface decides what that looks like.
/// - **The trim and the fader** are `gain` and `opacity`, which are two
///   controls and not one — *"opacity at zero silences under every blend mode,
///   gain at zero does not silence `over`"* — and the bay draws them as two.
/// - **The blend** is [`blend_mode`]: the engine's `Blend` turned into the
///   vocabulary's `BlendMode`, because the chip is a control now and a control
///   has to know which of the three it is on to say what the next one is
///   (ADR-0187). **This is where a fourth engine mode with no operation
///   variant stops the build**, which is the failure worth having — the
///   alternative is a word drawn on a chip no map can ask for.
/// - **The mask** is `Deck::mask(slot).kind()` for the mark, **and its angle
///   beside it** — `view::Strip::mask_angle`, which is read to build the
///   press's operation and drawn nowhere
///   ([ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)).
///   The position and the softness are left behind: the strip's `.mini` says
///   *which shape*, three numbers about that shape are an inspector row, and
///   the two the record needs are read where the record is written rather
///   than carried across this seam.
/// - **The level** is `Deck::level`, which is already `None` for every case
///   where a held reading would be about a different image. Its
///   `frames_behind` is not passed on — `view::Level` is where that argument
///   is written out, and the short of it is that the number means different
///   things on different loops and this loop is a `Fifo` one that never stops
///   asking for frames while anything is live.
/// - **The name** is [`material`], and it is this file's because a `Set` has
///   none. See `view::Strip::name`.
///
/// # Written into the `Vec` the view already holds
///
/// `out` is grown to the deck's slot count and then every field of every strip
/// is written, so nothing a `push` left behind is ever read. The name is the
/// one field that owns anything, and it is rewritten only when it differs —
/// which keeps this off the frame's allocation budget (ADR-0164) rather than
/// putting a `String` per strip on it every frame.
/// **What a scheduled move on this control is taking it to**, or `None` for a
/// control nothing is moving.
///
/// The engine's `Transition` carries the instant it starts, its length in
/// beats and its curve as well, and none of the three crosses this seam: the
/// console has no beat count, so what it could draw out of them is nothing.
/// See `view::Strip::gain_to`.
pub(crate) fn destination(deck: &Deck, slot: EngineSlot, control: Control) -> Option<f32> {
    deck.transitions_on(slot)
        .find(|t| t.control() == control)
        .map(|t| t.to())
}

/// **What each preview cell's risk badge reads**, from the pass that decided
/// it — one entry per deck slot, in slot order.
///
/// `Decision::budgeted_ms` is the number the governor spent and
/// `Decision::basis` says which of its two numbers that is (ADR-0296). The
/// console reads the number into five bands and carries the basis undrawn
/// (ADR-0298), so both halves cross and neither is spent twice.
///
/// **`Basis::Unbudgetable` is written as `None`**, and that is the whole of
/// what this function decides. It is a slot nothing measured and nothing
/// estimated, it is not a zero, and a zero here would draw a **green** dot.
///
/// **A deck with more slots than the row has cells contributes nothing past
/// the fourth**, which is the reading `View::select` refuses a key on.
pub(crate) fn costs(governed: &Report) -> [Option<Budgeted>; DECKS] {
    let mut out = [None; DECKS];
    for decision in &governed.decisions {
        let Some(cell) = out.get_mut(decision.slot) else {
            continue;
        };
        *cell = match (decision.budgeted_ms, decision.basis) {
            (Some(ms), Spent::Estimated) => Some(Budgeted {
                ms,
                basis: Basis::Estimated,
            }),
            (Some(ms), Spent::Measured) => Some(Budgeted {
                ms,
                basis: Basis::Measured,
            }),
            // `Unbudgetable`, and a number arriving without a basis — which
            // cannot happen, and is not worth inventing a band for if it does.
            _ => None,
        };
    }
    out
}

pub(crate) fn mixer(deck: &Deck, names: &[String], out: &mut Vec<view::Strip>) {
    out.truncate(deck.slot_count());
    while out.len() < deck.slot_count() {
        out.push(view::Strip {
            name: String::new(),
            tally: view::Tally::Allocated,
            requested: view::Tally::Allocated,
            gain: 0.0,
            gain_to: None,
            opacity: 0.0,
            opacity_to: None,
            blend: BlendMode::Add,
            mask: view::Mask::None,
            mask_angle: 0.0,
            level: None,
        });
    }
    for (slot, strip) in out.iter_mut().enumerate() {
        // **One name per slot, because a load moves one slot.** It was one
        // name for the whole deck while every slot ran the same pair and
        // nothing could change any of them; a library Set loaded into deck
        // B would then have left every strip reading the pair this program was
        // launched with, which is a readout that is wrong and says nothing
        // (P-0094). Empty for a slot nobody named, which draws no name at all
        // rather than somebody else's.
        let name = names.get(slot).map_or("", String::as_str);
        if strip.name != name {
            strip.name.clear();
            strip.name.push_str(name);
        }
        let slot = EngineSlot(slot as u8);
        strip.tally = tally(deck.residency(slot));
        strip.requested = tally(deck.requested_residency(slot));
        strip.gain = deck.gain(slot);
        strip.opacity = deck.opacity(slot);
        // **Where a scheduled move is taking each fader**, which is
        // `Deck::transitions_on` — *"what is moving on this slot, for a status
        // line"* — read for a surface instead. `karakuri-cli`'s line prints
        // `o>0.80` off the same call and for the same reason: with the default
        // quantum a fade is armed up to a bar before it is due, and a control
        // that changes something invisible is indistinguishable from one that
        // is broken.
        //
        // **At most one per control**, because `Deck::schedule` cancels
        // whatever was moving that pair before it pushes — so `find` is the
        // whole answer rather than the first of several.
        strip.gain_to = destination(deck, slot, Control::Gain);
        strip.opacity_to = destination(deck, slot, Control::Opacity);
        strip.blend = blend_mode(deck.blend(slot));
        strip.mask = masked(deck.mask(slot).kind());
        // **Read for the press and painted nowhere**, which is what
        // `view::Strip::mask_angle` is for: the chip asks for a shape and the
        // operation carries an angle, so the angle it carries is the one the
        // slot already has (ADR-0203). A harness that left this at zero would
        // make every press straighten a diagonal front, and nothing on the
        // panel would show it having happened.
        strip.mask_angle = deck.mask(slot).angle();
        strip.level = deck.level(slot).map(|level| view::Level {
            mean: level.mean,
            peak: level.peak,
        });
    }
}

/// **The Staging lane's rows, off the deck's own verdicts and the per-node
/// hashes its builds reported** — one per node a build changed, or one on the
/// slot where a verdict has no changed node behind it (ADR-0326).
///
/// # It drains, because a `HotSwap`'s events are a caller's to take
///
/// `HotSwap::events` is documented as the caller's — *"a caller that stops
/// draining eventually makes this grow"* — and until this program had a
/// producer there was nothing to drain, so nothing did. `pending_events` is
/// the other reading and is deliberately **not** what this uses: it is the
/// read-only view `Deck::begin_frame` takes *inside* a frame, and a surface
/// that read it without draining would be the caller bug the engine names.
///
/// **So the lane's state is kept here rather than re-derived per frame**, and
/// that is what a drain forces and is also what is wanted: the events are a
/// stream of verdicts and a slot's rows are the newest of them.
/// `view::View::staging` is written on the frames a build landed on and left
/// alone on every other, which is the same budget `mixer`'s name is kept to
/// (ADR-0164) with the frames-that-touch-it much rarer.
///
/// # What each verdict does to a row
///
/// - **`Swapped`, `Rejected`, `Overloaded`, `SourceRefused`** — the slot has
///   rows, on `view::Stage`'s four words. Whether the build is on screen and
///   running is what separates the first three — `Overloaded` is on screen and
///   stopped — and the fourth is the one where there was no build: the checker
///   turned the source down, so the row carries what it said as well as the
///   word (ADR-0310). **How many rows is the diff**: `Swapped` and
///   `Overloaded` draw one per node the build changed, and `Rejected` and
///   `SourceRefused` draw one on the slot, because a build that did not happen
///   has no node list to hold against the one before it.
/// - **`Accepted`** — the watchdog says the version held the budget, so the
///   file and the picture agree and that slot's rows leave the lane. It is not
///   the operator's verdict, which is *keep* and is taste rather than cost:
///   the cost verdict is what clears a row an operator has not pressed, and
///   `view::staging` is where that substitution is argued.
/// - **`WorkerLost`** — the build worker panicked and nothing will be built
///   again this run. **No row changes**, and that is the reading rather than
///   an omission: every verdict already taken still stands, and there is no
///   verdict outstanding for the worker to have taken with it — one is reached
///   in the call its swap lands in. What is lost is the *next* build, and the
///   lane has never been where that is said — the engine prints it.
///
/// **A row is not removed when its slot is parked or its material is
/// replaced.** What takes a row off the lane is a verdict in the candidate's
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
/// **They are written from one drain because there is only one.**
/// `Deck::events` empties the channel; a second pass for the capsule would
/// read nothing at all, which is the same sentence the reporter above is
/// written under.
///
/// # It answers whether the live Set changed, because something else has to
/// know
///
/// `true` when a build was installed, which is the moment `Set::published` says
/// a console should re-read a slot — *"A console reads this when a Set lands,
/// not per frame."* [`inspector`] is what acts on it. **Every other event
/// answers `false`, the budget's verdict included**: since ADR-0316 neither
/// verdict replaces what is playing — one lets the installed Set run and the
/// other stops it where it is — and the install that did replace it was
/// reported by `Swapped` in the same drain.
///
/// # What it costs
///
/// **Nothing on a frame nothing was built on.** The event drain collects into
/// a `Vec` that does not allocate when it is empty, and every allocation below
/// that is inside the `for` over it.
///
/// **On the frame a build lands**, which is the frame that also installed a
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

/// **One node a build changed, as a lane row needs it** — the address a press
/// is spelled with, the address the row draws, and what that node is called.
///
/// **Three fields and not one, because the row and the operation want
/// different halves of the same fact.** `view::Candidate::at` is the payload
/// and `view::Candidate::addr` is the string, which is `view::Param`'s
/// arrangement one bay over; both are written here, in one place, so they are
/// filled together or not at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Changed {
    pub(crate) at: karakuri_operation::NodeAddress,
    pub(crate) addr: String,
    pub(crate) name: String,
}

/// **The changed nodes of a build, turned into rows** — the addresses the
/// watcher reported, resolved against the Set that swap installed.
///
/// **The name is `Set::node_names`' and not the build's label**, which is the
/// whole of why this needs the Set at all: a label is every node's `proc` name
/// joined with ` + `, and a row that is one node wants that node's own. It is
/// the same name the Inspector writes on a node head — `Set::node_named` turns
/// each one back into the `(layer, index)` this compares against — so the two
/// bays call one node one thing.
///
/// **A node the Set does not name is passed over rather than drawn nameless.**
/// The two lists are the same build's, so this cannot happen from a rebuild;
/// what it would mean is that the Set and the report disagree about what the
/// slot holds, and a row invented out of that disagreement would be a press
/// aimed at an address nothing can resolve.
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

/// **A node's address as the mock's `.addr` spells it** — `L1:0`, `L2:0`,
/// `L4:0`.
///
/// [`layer_word`] is the layer half and this is the whole of it, written once
/// because two bays draw it: the Inspector's node heads and, since 2026-09-09,
/// the Staging lane's rows.
pub(crate) fn node_addr(layer: Layer, index: u32) -> String {
    format!("{}:{index}", layer_word(layer))
}

/// **Which of the four cells is showing a still**, in slot order —
/// `view::View::overloaded`, which the caption reads.
///
/// A slot the watchdog stopped holds the frame it last drew and is not stepped
/// or drawn (ADR-0316), and a held frame of good material is indistinguishable
/// from material: the word under the cell is what says which it is (ADR-0269).
///
/// **`false` past `slot_count`, not a panic.** The row is `view::DECKS` cells
/// whatever the deck holds, so the fourth cell of a three-slot deck asks about
/// a slot that is not there — and *there is no slot* is the caption's own word
/// rather than a state of one (`view::PREVIEW_NO_SLOT`).
///
/// A function rather than four lines in the frame loop, so that the bound is
/// written once and can be named from a test.
pub(crate) fn stopped_slots(deck: &Deck) -> [bool; view::DECKS] {
    std::array::from_fn(|slot| slot < deck.slot_count() && deck.overloaded(EngineSlot(slot as u8)))
}

/// **What one verdict does to the lane** — the whole of the mapping, in a
/// function a test can reach without a device.
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

/// **One slot's rows, put where that slot's rows go.**
///
/// **A slot's rows are replaced whole rather than rewritten in place**, and
/// that is the change ADR-0326 made here. A row used to be one slot, so the
/// slot's row could be found and its three fields overwritten; a row is now
/// one node a build changed, so how many rows a slot has moves with every
/// build — two on the save that touched two files, one on the next, none on a
/// verdict with nothing to diff. Nothing is left behind: the newest verdict is
/// the whole of what this slot has to say, and a row from the build before it
/// would be a node reported as unsettled by a build that has been replaced.
///
/// **Slot order, because that is the order the rows are read in** — the lane
/// draws them top to bottom and the deck letters are `A` through `D`, so a slot
/// whose verdict arrived later must not sit above one whose arrived first. The
/// splice is over at most `deck::MAX_SLOTS` slots' worth of rows.
///
/// **And node order within a slot**, which is the order `changed` is in and is
/// the order the Set names its own nodes in — the geometries, the deformers,
/// the cameras, the renderers, the fields. It is the Inspector's pane order
/// read on one slot, so two bays list one slot's nodes the same way round.
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

/// **The layer half of a node's address, as the mock's `.addr` spells it** —
/// `L1:0`, `L2:0`, `L4`.
///
/// `karakuri_ir::Kind` carries no name of its own, and `karakuri-cli`'s
/// `--publish name=L4:0:key` parser is in a package with no library target, so
/// there is nothing to call. The five words are `docs/ir-spec.md`'s and this
/// is a **match** for [`blend_mode`]'s reason: a sixth kind stops the build
/// here rather than drawing an address nothing can be typed back in.
pub(crate) fn layer_word(layer: Layer) -> &'static str {
    match layer {
        Layer::L1 => "L1",
        Layer::L2 => "L2",
        Layer::L3 => "L3",
        Layer::L4 => "L4",
        Layer::Field => "F",
        // **A bare letter like the four above and unlike `F`'s neighbour**,
        // which is the address a press types back in — `L5:0`, on
        // `karakuri_environment::setfile::layer_ordinal`'s numbering.
        Layer::L5 => "L5",
    }
}

/// **A node's layer as the vocabulary names one.** `karakuri_ir::Kind` and
/// `karakuri_operation::Layer` are two spellings of one list, and this is the
/// converter between them; `mcp.rs`'s `layer_of` is the other instance and is
/// private to that crate. A match rather than a cast, for [`layer_word`]'s
/// reason: a sixth kind stops the build here.
pub(crate) fn asked_layer(layer: Layer) -> karakuri_operation::Layer {
    match layer {
        Layer::L1 => karakuri_operation::Layer::L1,
        Layer::L2 => karakuri_operation::Layer::L2,
        Layer::L3 => karakuri_operation::Layer::L3,
        Layer::L4 => karakuri_operation::Layer::L4,
        Layer::Field => karakuri_operation::Layer::Field,
        Layer::L5 => karakuri_operation::Layer::L5,
    }
}

/// **And back**, for the two things that want the compiler's own: spelling an
/// address a press arrived with, and resolving one to the word the store files
/// a version under. `karakuri_mcp`'s `kind_of` is the other
/// instance and is private to that crate; this is [`asked_layer`]'s inverse
/// and a match for its reason, so a sixth layer stops the build in both
/// directions rather than in one.
pub(crate) fn ir_layer(layer: karakuri_operation::Layer) -> Layer {
    match layer {
        karakuri_operation::Layer::L1 => Layer::L1,
        karakuri_operation::Layer::L2 => Layer::L2,
        karakuri_operation::Layer::L3 => Layer::L3,
        karakuri_operation::Layer::L4 => Layer::L4,
        karakuri_operation::Layer::Field => Layer::Field,
        karakuri_operation::Layer::L5 => Layer::L5,
    }
}

/// **Which node a published control belongs to**, and `None` where it belongs
/// to no one node.
///
/// `Published::at` is `Some` for a control an author addressed — the
/// `--publish name=L4:0:exposure` form — and `None` for a **wildcard**, which
/// is what the whole of the *default* interface is made of: *"one control per
/// key, not one per declaration"*, covering every node that declares the key.
/// This program has no `--publish` flag, so every control it ever draws is a
/// wildcard.
///
/// **A wildcard over exactly one node is that node's**, and the resolution
/// invents nothing: *where a bare name lands* is a set the Set itself
/// determines, and where it holds one member there is no second group the row
/// could go in. Over two or more it belongs to several groups at once, and the
/// mock draws no `.param` outside a `.node-group` — so the row is dropped and
/// [`inspector`] says how many were, rather than a place for it being invented
/// here (ADR-0200: *no placeholder, and no empty case the mock did not itself
/// draw*).
///
/// **The landing is asked for rather than worked out here**, and that is a
/// correction rather than a tidying. This walked `Set::params` and counted the
/// nodes holding the key, which is the same walk `Set::write_param` refuses on
/// — agreeing with it by coincidence. The day the built-in camera declared a
/// `radius` of its own the two stopped agreeing: a bare name does not reach
/// that node, so the engine saw one landing where this saw two, and a `radius`
/// row the pane draws every run was dropped as belonging to several groups
/// (ADR-0318). `Set::landing_of` is the one answer, where the write is decided
/// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
pub(crate) fn node_of(set: &Set, control: &Published) -> Option<(Layer, u32)> {
    if let Some(at) = control.at {
        return Some(at);
    }
    let mut declaring = set.landing_of(&control.key).into_iter();
    let first = declaring.next()?;
    match declaring.next() {
        None => Some(first),
        Some(_) => None,
    }
}

/// **What is driving one node's parameter**, or `None` where nothing is —
/// the reading behind a `.param.bound` row and the `.sens` row under it.
///
/// # The first match, because that is what the engine writes
///
/// `Set::bindings` is a list and `karakuri_engine::set::effective` takes the
/// **first** entry matching the layer, the key and the node, so a pane that
/// took the last would draw a source that is not the one holding the control.
/// This is that same `find`, and it is the one place in this file that reads a
/// binding at all: the pane's seventh reading, which ADR-0191 kept out while
/// nothing in this program could attach one.
///
/// **Addressed by the node the row was resolved to**, which is safe here for
/// the reason it would not be safe for a *write*: every row this pane draws
/// covers exactly one node — [`node_of`] drops the ones that do not — so
/// asking which binding covers that node is asking about the row itself. The
/// address the take-back and the re-attach carry is the **binding's** own, off
/// the entry found, and not this pair (ADR-0286: the placement is where a row
/// goes, the address is what a control is).
pub(crate) fn source_of(set: &Set, layer: Layer, index: u32, key: &str) -> Option<view::Source> {
    let binding = set
        .bindings()
        .iter()
        .find(|b| b.layer == layer && b.key == key && b.covers(index as usize))?;
    Some(view::Source {
        signal: binding.signal.clone(),
        curve: mix::curve(binding.curve),
        range: binding.range,
        at: karakuri_operation::BindAt {
            layer: asked_layer(binding.layer),
            index: binding.index,
            key: binding.key.clone(),
        },
    })
}

/// **What the Inspector's panes read**: one pane per slot the deck has, up to
/// the [`PANES`] the arrangement has, out of the seven things a `Set` will say
/// about itself.
///
/// Everything here is reachable from a `Deck` and nothing reaches around one —
/// `slot`, `transport` and the `Set` behind each slot are its own — and what
/// crosses into the console is a word, some numbers and some names (ADR-0156).
///
/// # Where each one comes from, and what is checked before it is drawn
///
/// All seven reads answer off a running `Set`, and each was checked against
/// the source rather than taken on trust:
///
/// - **`Set::layering`** is the fold chip. It is a *build* decision —
///   `merge.is_some()` — so it is a readout here and the mock's press is a
///   rebuild rather than a write.
/// - **`Set::inputs`** is the renderer row: one edge per renderer in draw
///   order, and `Input::live` says which one reaches the screen. It is
///   *"empty of meaning under `Layering::Overdraw`"* by the engine's own
///   words, so `live` is only ever passed on under composite — under overdraw
///   every renderer draws and marking one would assert a choice the layering
///   does not make.
/// - **`Set::node_names`** is a name per node, in node order, and
///   `Set::node_named` turns each one back into the `(layer, index)` the mock's
///   `.addr` is.
/// - **`Set::authority`** is the `man / sug / auto` chip, through
///   [`mix::authority`], and the node's own address goes across beside it
///   because a press on a chip has to say which node it is about
///   (`view::NodeAuthority`). **A press writes one now**: `Deck::set_authority`
///   landed with ADR-0319, so the chips are three destinations rather than a
///   drawing. What a run *starts* at is still the default on every node —
///   `Request::authorities` is empty, because `Record::Authority` is
///   deliberately excluded from Set-file state in two places (ADR-0216) — and
///   that is the default being read rather than a placeholder being drawn: a
///   node nobody has spoken for **is** manual.
/// - **`Set::published`** is which controls appear and in what order, which is
///   what numbers the rows. It **allocates and says it is not for the frame
///   path**, which is why this is called once — see below.
/// - **`Set::params`** is what resolves a wildcard control to a node — see
///   [`node_of`].
/// - **`Set::bindings`** is the seventh, and it was the one this did **not**
///   read until 2026-09-09. The reason was ADR-0191's — nothing in this
///   program bound a signal to anything, so it was empty in every run and a
///   `.pval.src` drawn off it would have been a state the engine never
///   entered — and what changed is that the sensitivity row's chips can attach
///   one (ADR-0319). [`source_of`] is the read, at the node each row was
///   resolved to, and it takes the **first** matching entry because that is
///   what `karakuri_engine::set::effective` writes.
///
/// # Read once, and that is the engine's instruction rather than a shortcut
///
/// `Set::published` is documented *"Allocates, so not the frame path. A
/// console reads this when a Set lands, not per frame."* **A Set lands
/// whenever a `.kir` in a slot is saved**, since every slot is watched, so
/// this is called at startup and again on the frame a build is installed —
/// `staging` is what answers *did one land*, and it is the only thing in this
/// program that knows. Between those
/// frames almost every value above used to be constant. **Four things move
/// one now**, and each is a press: a `ride`, a `source`, an `authority` and a
/// `transport`. Every one of them re-reads the panes from
/// [`Readout::performed`], off the *record* rather than off the operation, so
/// a second operation writing one is caught by the same line — and none of
/// them is on a frame.
///
/// **The transport was the first that moved, and it is why this is called a
/// second time.** It had no caller outside its own tests when this was written
/// (ADR-0218); the deck head's scrub is that caller, so a press that writes a
/// `Record::Transport` re-reads the panes in [`Readout::performed`] — on the
/// press, which is where a directory read already happens, and never on a
/// frame. Re-reading the whole pane rather than writing the two numbers back
/// is deliberate: a second writer into `View::inspector` is a second answer to
/// *what is this pane showing*, and the reading that draws the anchor has to
/// be the reading the deck holds.
///
/// **The other three arrived with ADR-0319 and ADR-0280**, which is what that
/// sentence was waiting for: the writer the authority chip wanted is
/// `Deck::set_authority`, and the parameter's is `Deck::write_param`.
pub(crate) fn inspector(
    deck: &Deck,
    names: &[String],
    // **Where each slot's watcher is pointed**, which is what a node's `uses`
    // line reads: `Aiming::at.edges` is the wiring that slot was last sent, so
    // a pane draws the wiring of the deck it is *showing* rather than a run
    // list read once for four decks. It is the same list — `rewired` restates
    // the run's whole wiring to every slot it names — and this is the copy that
    // belongs to the slot the pane is about.
    //
    // **A pane with no aim behind it draws no `uses` line**, which is every
    // test in this crate that hands none in and is honest either way: a slot
    // nothing is pointed at is a slot nothing will rebuild.
    aims: &[Aiming],
    // **Which deck each pane is pointed at**, which is the console's own
    // pointer read back — `view::View::pane_deck`, one per pane, moved by the
    // pulldown on that pane's head (ADR-0338, decision 5).
    //
    // **It is handed in rather than read here**, which is `View::target_deck`'s
    // arrangement one bay along: the pointer belongs to the console and what
    // is under it belongs to the deck, so this function answers *what is that
    // slot playing* and never *which slot should this pane show*.
    //
    // Panes used to be filled slot by slot — pane `n` from slot `n` — so slots
    // C and D had no way onto this bay at all. That arrangement is the default
    // this array opens with (`view::PANE_DECKS`) rather than a rule.
    targets: [u8; view::PANES],
    out: &mut Vec<view::Pane>,
) {
    out.clear();
    // **A pane per target, in pane order**, which is what makes the position
    // of an entry in `out` the pane it belongs to — `view::inspector` reads it
    // by index, and `View::inspector` is walked the same way.
    for target in targets {
        let slot = usize::from(target);
        // **A pane pointed at a deck this run has no slot for draws nothing**,
        // and the panes after it go with it because a pane is addressed by its
        // position in this list. `View::point_pane` refuses a deck the mixer
        // draws no strip for, so the only way here is the tail of the default:
        // a run with one slot opens with the second pane pointed at deck B,
        // which is where `slot_count().min(PANES)` left it before this
        // pointer existed.
        if slot >= deck.slot_count() {
            break;
        }
        // The strip's name for the same slot, and for [`mixer`]'s reason: a
        // pane head reads `deck A · drift_night`, and after a load that is the
        // Set the operator chose rather than the pair the run opened with.
        let material = names.get(slot).map_or("", String::as_str);
        let addr = EngineSlot(target);
        let set = deck.slot(addr).set();
        let transport = deck.transport(addr);
        let composite = set.layering() == Layering::Composite;
        // **The deck head's two build chips**, and all three readings are of
        // what **landed** rather than of what was asked: the number the slot is
        // running at, the declaration it is measured against, and the salt the
        // next one is derived from. That is the fold's own division one field
        // over — a build may still be rolled back, and the Staging lane is what
        // says so — and it is what lets this be read off a `Set` with no aim in
        // sight (ADR-0328).
        let declared = set.declared_capacities();
        let running = set.source_capacities();
        let salts = set.source_salts();
        let aimed = match (running.first(), declared.first(), salts.first()) {
            (Some(&capacity), Some(&[_, _, default]), Some(&salt)) => Some(view::Aimed {
                capacity,
                // **Lit says the deck is not on what its material declares**,
                // which is the fact a chip can state from what landed. *An aim
                // carries a number* is the other candidate and is a reading of
                // what was asked: a hand that steps round to the declared
                // default would leave the chip lit over a slot running exactly
                // what its files say.
                stated: capacity != default,
                capacities: capacity_ladder(declared),
                salt: karakuri_engine::set::derived_salt(salt, 1),
            }),
            // **A deck with no geometry has neither chip**, which is the state
            // this `Option` is: there is no element count to size and no
            // randomness to seed.
            _ => None,
        };

        // Every published control, resolved to the node it belongs to and
        // numbered by its position in the interface — which is the number a
        // MIDI control is learned against, so it counts the controls that were
        // published and not the rows that could be placed.
        let published = set.published();
        // **And every control the material declares**, which is what the
        // publish mark is chosen *from*: a row taken off the interface has to
        // stay drawn or the choice cannot be unmade, and its declared range is
        // what putting it back is over. `Set::declared_interface` is the
        // reading — the default interface, whether or not one is authored
        // (ADR-0100, `docs/adr/0329-…`).
        //
        // **Identity is the address and the key together**, which is
        // `Published`'s own pair: a wildcard control and an addressed one may
        // share a key and are two controls, and comparing names would fold a
        // renamed control onto the declaration it renames.
        let declared = set.declared_interface();
        let mut rows: Vec<(Option<(Layer, u32)>, view::Param)> = Vec::new();
        for (at, control) in published.iter().enumerate() {
            // **By the address the control carries, not by its name.** The
            // built-in camera's three publish addressed, so a Set whose
            // geometry also declares `radius` has two controls under that name
            // and a name lookup answers for whichever comes first (ADR-0318).
            let value = set
                .value_at(control.at, &control.key)
                .unwrap_or(control.range[0]);
            let node = node_of(set, control);
            rows.push((
                node,
                view::Param {
                    ord: Some(at + 1),
                    name: control.name.clone(),
                    value,
                    // **The range and not the position.** A fader a hand can
                    // move has a second reader — the grab — so the map from a
                    // value to a place on the track is one statement in one
                    // place, `view::Param::at` and `view::Param::valued`, and
                    // the guard on a range of no width went with it
                    // (ADR-0286).
                    range: control.range,
                    // **The control the interface published, and not the
                    // group `node_of` put the row in.** A wildcard stays a
                    // wildcard, so a bare name goes on meaning every node that
                    // declares the key and meets `Set::write_param`'s
                    // authority refusal (ADR-0286, ADR-0223).
                    param: karakuri_operation::ParamAt {
                        node: control
                            .at
                            .map(|(layer, index)| karakuri_operation::NodeAddress {
                                layer: asked_layer(layer),
                                index,
                            }),
                        key: control.key.clone(),
                    },
                    // **The seventh reading, and the one this pane used to
                    // leave out.** It was omitted on ADR-0191's terms —
                    // nothing in this program bound anything, so a bound row
                    // was a state the engine could not enter — and what
                    // changed is that a press can attach one now (ADR-0319).
                    // Asked at the node the row was resolved to, which is the
                    // node the row *is*: `node_of` drops a control covering
                    // more than one.
                    bound: node
                        .and_then(|(layer, index)| source_of(set, layer, index, &control.key)),
                },
            ));
        }
        // **Then every declared control the interface leaves out**, drawn with
        // no number, no fader and no figure — the mark that publishes them is
        // the cell the number would be in, so a row that vanished would be a
        // choice nobody could unmake (`docs/adr/0329-…`).
        //
        // **After the published ones**, which is the order they are drawn in
        // *within a group*: a group's published rows come first and the ones
        // off the list follow, so the numbers a reader is counting down do not
        // step over a gap.
        //
        // **On a Set nobody has narrowed this loop adds nothing**, because
        // `Set::published` answers with `declared_interface` itself there —
        // which is why every deck opens looking exactly as it did before this
        // control existed.
        let mut unplaced = 0;
        for control in declared {
            if published
                .iter()
                .any(|shown| shown.at == control.at && shown.key == control.key)
            {
                continue;
            }
            unplaced += 1;
            let node = node_of(set, &control);
            rows.push((
                node,
                view::Param {
                    // **No position, because a control off the interface has
                    // none** — and a position is what a MIDI knob counts.
                    ord: None,
                    name: control.name.clone(),
                    value: set
                        .value_at(control.at, &control.key)
                        .unwrap_or(control.range[0]),
                    // **The declared range and not a narrowed one**: this is
                    // what publishing it back would be over, and the row is
                    // drawn from `declared_interface`, which never narrows.
                    range: control.range,
                    param: karakuri_operation::ParamAt {
                        node: control
                            .at
                            .map(|(layer, index)| karakuri_operation::NodeAddress {
                                layer: asked_layer(layer),
                                index,
                            }),
                        key: control.key.clone(),
                    },
                    // **Nothing can be holding it**, because a binding names a
                    // published control or a param and the row draws neither a
                    // value nor a sensitivity row. Read anyway rather than
                    // assumed `None`: what a Set holds is the engine's answer
                    // and this file does not have a second one.
                    bound: node
                        .and_then(|(layer, index)| source_of(set, layer, index, &control.key)),
                },
            ));
        }

        // **Which L3 node is the built-in camera**, which is the one node on a
        // pane with no procedure behind it and so nothing to keep.
        //
        // **The last camera node, always** — `Set::cameras`: *"Never empty,
        // and the last one is always the built-in orbit."* A Set whose files
        // declare no `kind L3` holds it at `L3:0`, and one that declares two
        // holds it at `L3:2`; either way it is the highest index on that
        // layer, so this is a `max` rather than a check for an empty layer.
        //
        // **Asked here rather than of the store**, because what a `keep` needs
        // is *is there a source at all*, and the Set is what knows. The bytes
        // themselves are the host's to find at the press — `Keeping::playing`
        // — and a pane that named a node the store cannot answer for would be
        // a capsule refusing after it was drawn.
        let builtin_camera = set
            .node_names()
            .iter()
            .filter_map(|name| set.node_named(name))
            .filter(|(layer, _)| *layer == Layer::L3)
            .map(|(_, index)| index)
            .max();
        let mut nodes: Vec<view::Node> = Vec::new();
        let mut renderers: Vec<view::Renderer> = Vec::new();
        let mut renderer_nodes = 0;
        let mut renderer_authority = None;
        let mut renderer_keep = None;
        for name in set.node_names() {
            let Some((layer, index)) = set.node_named(name) else {
                continue;
            };
            // **The address goes with the level**, because a press on a chip
            // has to say which node it is about and the two are absent
            // together — `view::NodeAuthority`, which is why this is one field
            // over there rather than two.
            let authority = set
                .authority(layer, index)
                .map(|level| view::NodeAuthority {
                    at: karakuri_operation::NodeAddress {
                        layer: asked_layer(layer),
                        index,
                    },
                    level: mix::authority(level),
                });
            let params = |layer: Layer, index: u32| {
                rows.iter()
                    .filter(|(at, _)| *at == Some((layer, index)))
                    .map(|(_, param)| param.clone())
                    .collect::<Vec<_>>()
            };
            if layer == Layer::L4 {
                // **The renderers fold into one group**, which is the mock's
                // own `L4 renderers` head over a row of chips: the chips *are*
                // the L4 nodes, and the row is what the fold turns into a
                // choice. Every other layer is one group per node, addressed
                // `L2:0` the way the mock addresses it.
                renderers.push(view::Renderer {
                    name: name.clone(),
                    live: composite
                        && set
                            .inputs()
                            .get(index as usize)
                            .is_some_and(|edge| edge.live),
                });
                renderer_nodes += 1;
                renderer_authority = match renderer_nodes {
                    1 => authority,
                    // **More than one node under one head has no one
                    // authority**, and authority is per node (ADR-0216). The
                    // chip is dropped rather than showing the first of them.
                    _ => None,
                };
                // **And no capsule either, for that sentence** — one `keep` on
                // a head standing over three renderers would keep one of the
                // three and say nothing about which (ADR-0338, decision 4).
                // Open the fold and each renderer has its own; a Set with one
                // renderer has one node under that head and carries the
                // capsule like any other.
                renderer_keep = match renderer_nodes {
                    1 => Some(karakuri_operation::NodeAddress {
                        layer: asked_layer(layer),
                        index,
                    }),
                    _ => None,
                };
                continue;
            }
            nodes.push(view::Node {
                addr: node_addr(layer, index),
                name: name.clone(),
                authority,
                // **Every node but the built-in camera has a source to keep**,
                // which is what `builtin_camera` above answers. The address
                // rides with it rather than beside it, for `view::Node::keep`'s
                // own reason: a head with nothing to keep has no node either.
                keep: (layer != Layer::L3 || Some(index) != builtin_camera).then_some(
                    karakuri_operation::NodeAddress {
                        layer: asked_layer(layer),
                        index,
                    },
                ),
                renderers: Vec::new(),
                uses: match aims.get(slot) {
                    Some(aiming) => uses_of(set, &aiming.at.edges, name),
                    None => Vec::new(),
                },
                params: params(layer, index),
            });
        }
        if renderer_nodes > 0 {
            // The mock's address for the folded head is the bare layer, with
            // no index — because it is not one node's.
            let mut params: Vec<view::Param> = Vec::new();
            for index in 0..renderer_nodes {
                params.extend(
                    rows.iter()
                        .filter(|(at, _)| *at == Some((Layer::L4, index)))
                        .map(|(_, param)| param.clone()),
                );
            }
            // **Published first and in interface order, then the ones off the
            // list.** `Option`'s ordering puts `None` last, which is the order
            // the rows are drawn in within a group and is the reason the sort
            // is on the whole field rather than on a position: a number a
            // reader is counting down should not step over a gap.
            params.sort_by_key(|param| param.ord);
            nodes.push(view::Node {
                addr: layer_word(Layer::L4).to_owned(),
                name: RENDERERS_NODE.to_owned(),
                authority: renderer_authority,
                keep: renderer_keep,
                renderers,
                // **The folded renderer head takes none**, and it is the same
                // reason its authority chip is dropped where it stands over
                // more than one node: a `uses` line names *one* node's
                // declaration, and this head is not a node. A renderer that
                // declares an input is reachable from the file and from a
                // model, and the panel says so rather than drawing one of
                // several answers as the answer (ADR-0216's shape).
                uses: Vec::new(),
                params,
            });
        }

        let placed: usize = nodes.iter().map(|node| node.params.len()).sum();
        if placed < published.len() + unplaced {
            // **Said rather than swallowed**, for the reason every other
            // omission in this file is said: a pane short of a row looks
            // exactly like a Set that published fewer. See `node_of` — a
            // wildcard over two or more nodes belongs to two or more groups,
            // and the mock draws no row outside one.
            println!(
                "inspector: deck {} publishes {} controls and {} of them name no one node, so \
                 they have no group to sit in and are not drawn",
                DECK_LETTERS.get(slot).copied().unwrap_or("?"),
                published.len(),
                published.len() - placed
            );
        }

        out.push(view::Pane {
            deck: slot,
            material: material.to_owned(),
            sync: mix::sync(transport.sync()),
            // **What the sync chip's cycle skips over, asked of the engine
            // three times.** `Deck::sync_allowed` is *"what a surface greys a
            // control out on, and it answers before anything is pressed"* —
            // whether the Set in this slot is closed form and whether it reads
            // `beats`, put through `Transport::allows`. The console is handed
            // the three answers rather than the two properties, because what
            // may be asked for is not a surface's to work out (P-0090) and a
            // third copy of that rule in a crate with no material in it is a
            // rule that can start disagreeing.
            //
            // **`EngineSync::ALL` is in `SYNCS`' order**, which is what makes
            // this array line up with the field it fills;
            // `the_two_crates_walk_the_sync_modes_in_one_order` is what says
            // so rather than this comment.
            allows: EngineSync::ALL.map(|mode| deck.sync_allowed(addr, mode).is_ok()),
            anchor_bpm: transport.anchor_bpm(),
            scrub_beats: transport.scrub_beats(),
            composite,
            aimed,
            nodes,
        });
    }
}

/// **The `uses` lines one node draws**: every input its procedure declares,
/// with the node filling each.
///
/// # The edges *are* the declarations, and that is forced rather than chosen
///
/// Nothing on a built `Set` says which inputs a node declares — the `uses`
/// declaration is read at `Set::validate` and dropped — and nothing has to,
/// because **an unbound declared input is refused where the Set is built**
/// (ADR-0152: *"`If there is exactly one, use it` is the implicit rule the whole
/// item exists to remove, and the refusal names the slot"*). So a slot that is
/// *running* has an edge for every input it declares, and the run's edge list
/// filtered to the nodes this Set holds is that list exactly. A reader on the
/// engine would be a second answer to a question the refusal already settles.
///
/// # The candidates are the nodes on the layer the input already reaches
///
/// A `uses` slot has a type — `Geometry`, `Field`, `Camera`, `Source` — and the
/// build refused anything else, so **the node currently wired is of the right
/// kind by construction** and its layer is the kind. The candidates are the
/// other nodes on that layer, in node order, which is a list every entry of
/// which the build accepts.
///
/// **It is inference and it is honest about being it.** What this cannot do is
/// offer a kind the input takes and the deck currently reaches by no edge — a
/// `Field` input on a deck holding one field has an empty list, and the card
/// does not open. That is a control offering less than the language allows
/// rather than more, which is the side of P-0090 to be wrong on: a name this
/// misses is still reachable from a model and from `--edge`.
///
/// **The declaring node is not in its own list.** A node wired to itself is a
/// cycle the build refuses, and offering it would be offering a refusal.
pub(crate) fn uses_of(
    set: &karakuri_engine::set::Set,
    edges: &[karakuri_engine::set::Edge],
    node: &str,
) -> Vec<view::Uses> {
    edges
        .iter()
        .filter(|edge| edge.node == node)
        .filter_map(|edge| {
            let (kind, _) = set.node_named(&edge.to)?;
            let candidates = set
                .node_names()
                .iter()
                .filter(|name| name.as_str() != node && name.as_str() != edge.to)
                .filter(|name| set.node_named(name).is_some_and(|(at, _)| at == kind))
                .cloned()
                .collect();
            Some(view::Uses {
                slot: edge.slot.as_str().into(),
                to: edge.to.clone(),
                candidates,
            })
        })
        .collect()
}

/// **What a slot's capacity chip steps through**: the powers of two every one
/// of this deck's geometries would accept, ascending.
///
/// # The intersection, because a re-aim sends one number
///
/// `watch::Aim::capacity` is one `Option<u32>` for the whole slot —
/// `--capacity`'s own field, which *"overrides every source"* — so a Set
/// holding two geometries builds both at whatever this asks for, and a number
/// only one of them declares is a build the other refuses. The fold is
/// `lo.max(min)`, `hi.min(max)`, which is `declared`'s arithmetic one bay over
/// where two nodes publish one key: the range is the part every declarer
/// accepts and never any one of them on its own.
///
/// **An empty intersection is an empty list**, and that is a real state rather
/// than an unreachable one: two geometries whose declared ranges do not overlap
/// have no capacity a single re-aim could send. The chip is then drawn and
/// claims nothing, which is what `view::Aimed::capacities` says at the field.
///
/// **Powers of two, and nothing here says why they are the right rungs** — that
/// is the console's affordance and its record
/// (`docs/adr/0328-…`); what this owes is that every rung it offers is one the
/// engine will build, which is `Set::declared_capacities` being the same
/// declaration `karakuri_engine::set::capacity_in_range` refuses against
/// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
pub(crate) fn capacity_ladder(declared: &[[u32; 3]]) -> Vec<u32> {
    let Some(lo) = declared.iter().map(|at| at[0]).max() else {
        return Vec::new();
    };
    let Some(hi) = declared.iter().map(|at| at[1]).min() else {
        return Vec::new();
    };
    // `0..32` and not `0..=32`: `1u32 << 32` is undefined, and 2^31 is the
    // largest power of two a `u32` capacity can be.
    (0..32)
        .map(|k| 1u32 << k)
        .filter(|rung| (lo..=hi).contains(rung))
        .collect()
}

/// **What the mock calls the group its renderer chips sit under.** Not a node
/// name — every L4 node has one of those and they are the chips themselves —
/// but the head over all of them, which the mock writes as `L4 renderers`.
pub(crate) const RENDERERS_NODE: &str = "renderers";

/// **The engine's residency, as the console's word for it** — and, like
/// [`blend_mode`], a `match` so that a fourth `Residency` stops the build here
/// rather than drawing a chip nothing can read.
///
/// One function for both halves of the pair. It was written inline for the
/// effective residency alone; the request needs exactly the same three arms,
/// and a second copy of them is a translation that can start disagreeing with
/// itself about what `Priming` is called.
pub(crate) fn tally(residency: Residency) -> view::Tally {
    match residency {
        Residency::Live => view::Tally::Live,
        Residency::Priming => view::Tally::Priming,
        Residency::Allocated => view::Tally::Allocated,
    }
}

/// **The engine's blend mode, as the vocabulary's** — and the one place the
/// two lists are made to agree.
///
/// `karakuri-operation` owns its own copy of every list a destination is drawn
/// from, which is the cost P-0090 says the vocabulary pays: *"The two rules —
/// be engine-neutral, and have no toggles — are not jointly satisfiable unless
/// the vocabulary owns the lists."* A copy needs somewhere the two meet, and
/// this is that place for this list, on the harness side of the seam — the
/// same side [`mixer`] reads a `Deck` from (ADR-0156).
///
/// **A match, so the day a fourth mode lands in `karakuri_engine::deck::Blend`
/// this stops compiling.** That is the whole reason `view::Strip::blend` is a
/// `BlendMode` and not the engine's word: a `&str` handed through would draw
/// the new mode's name on a chip, and the chip would cycle three ways past a
/// state no operation can name and no MIDI map can reach, with nothing saying
/// so. Failing here is the loud failure P-0094 asks for.
///
/// **It stays this program's, and that is now settled rather than pending.**
/// The console cannot depend on the engine (ADR-0156), and
/// `karakuri-operation-record` cannot either — it is the vocabulary and the
/// records and nothing else, by charter. So the two lists meet on the harness
/// side of the seam, wherever a harness holds both, and ADR-0180's *"one
/// `From` impl per list in `karakuri-cli`"* cannot be written at all: neither
/// [`Blend`] nor [`BlendMode`] is that package's, and the orphan rule refuses
/// it ([ADR-0194](../../../docs/adr/0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md)).
pub(crate) fn blend_mode(blend: Blend) -> BlendMode {
    match blend {
        Blend::Add => BlendMode::Add,
        Blend::Over => BlendMode::Over,
        Blend::Max => BlendMode::Max,
    }
}

// **The blend cycle is `karakuri_console::view::after` and is not here.**
//
// `after_blend` stood here until 2026-09-10 and said the same three modes in
// the same order as the console's own cycle, with a test each. The key that
// asked it was `m`; under the grammar `space` on an addressed blend chip asks
// the console, and the console has always had the cycle because the *chip*
// needs it (ADR-0187, P-0090 — a toggle is an affordance built over operations
// by whoever draws the control). **So there is one cycle where there were
// two**, and `karakuri-console/tests/blend.rs` is what holds it: it walks
// `BlendMode::ALL` through the chip and asserts the wrap, which is exactly
// what the test deleted beside this function asserted from the other side
// (ADR-0333).
//
// `blend_mode` stays, and is what `holding` reads the deck through: the
// engine's three and the vocabulary's three are two crates' words for the same
// states, and a `match` is where they are made to agree.

/// **One press of a gain key.** Linear and additive, because a fader is: the
/// same press means the same amount wherever the trim is standing, rather than
/// a proportion of wherever it happens to be.
///
/// **A tenth, because that is what the other keyboard steps by.**
/// `docs/manual/operations.html` names the keys and says nothing about how far
/// a press goes, so the size comes from `karakuri-cli`'s own `GAIN_STEP` —
/// which `docs/manual.md` documents as *"focused slot gain down / up"* — and
/// it is copied rather than shared because neither binary may depend on the
/// other (ADR-0214). A page that decides otherwise moves this constant.
pub(crate) const GAIN_STEP: f32 = 0.1;

/// One press of an opacity key, and [`GAIN_STEP`]'s sentence one control
/// along: `karakuri-cli`'s `OPACITY_STEP`, which is the same tenth, and the
/// console's page is silent about this one too.
pub(crate) const OPACITY_STEP: f32 = 0.1;

/// **Which of the grammar's four keys a press is**, or `None` for a key that is
/// not one of them.
///
/// # Why the digits are a guard and not ten arms
///
/// `1` in the Mixer is deck A's strip and `1` in the Library is its first
/// row, so ten arms naming ten literals would say the digits are bound and
/// say nothing about what they reach. **The dispatch table is what says
/// that** — `karakuri_console::focus::BUILT` — so the digit is a guard here
/// rather than ten characters (ADR-0333).
///
/// **`key_column::bound` no longer reads this file's text at all** — since
/// the ten literal keys `window_event` binds outside this guard moved into
/// `KEY_BINDINGS`, there is nothing left in this file's source for a scan to
/// find that a declared fact could not say instead. What this function still
/// binds — `space`, `enter`, the four arrows and the digit — is declared in
/// `key_column::bound` alongside `KEY_BINDINGS`' own keys rather than
/// scanned for, the same as the digit always was.
pub(crate) fn grammar(key: &Key<&str>) -> Option<focus::Press> {
    match key {
        Key::Named(NamedKey::Space) => Some(focus::Press::Space),
        Key::Named(NamedKey::Enter) => Some(focus::Press::Enter),
        Key::Named(NamedKey::ArrowUp) => Some(focus::Press::Arrow(focus::Arrow::Up)),
        Key::Named(NamedKey::ArrowDown) => Some(focus::Press::Arrow(focus::Arrow::Down)),
        Key::Named(NamedKey::ArrowLeft) => Some(focus::Press::Arrow(focus::Arrow::Left)),
        Key::Named(NamedKey::ArrowRight) => Some(focus::Press::Arrow(focus::Arrow::Right)),
        Key::Character(text) => digit(text).map(focus::Press::Digit),
        _ => None,
    }
}

/// **One digit, `0` to `9`**, or `None` for anything else a `Key::Character`
/// can be.
///
/// A `Key::Character` is *text* and may be more than one character — a dead key
/// resolving, an IME committing a run — which is why the length is checked
/// rather than the first character taken. `char::to_digit` at radix ten accepts
/// the ASCII ten and nothing else, so a digit from another script is not one
/// here: the digits count what a bay drew and the number row is what a
/// performer finds without looking.
pub(crate) fn digit(text: &str) -> Option<usize> {
    let mut chars = text.chars();
    let one = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    one.to_digit(10).map(|digit| digit as usize)
}

/// **What the deck is holding on `at`**, for the three chips whose next state
/// the console names — or `None` where this deck has no slot there.
///
/// [`held`] is the guard, for its own reason: `Deck::blend` indexes its slots
/// and a panic reachable from an event handler aborts this process rather than
/// unwinding.
///
/// **Read at the press and never off `view::Strip`**, which is what the three
/// mix keys this replaces already said: a strip is this same reading copied
/// once a frame, and a scheduled fade landing between the frame and the press
/// would leave the cycle counting from a state the deck has left behind.
pub(crate) fn holding(deck: &Deck, at: u8) -> Option<focus::Held> {
    let slot = held(deck, at)?;
    Some(focus::Held {
        // **The residency the deck was last *asked* for**, which is what the
        // tally cycles from — `view::Mixer::tally`'s decision, and the one
        // that makes a parked slot's press a withdrawal rather than a
        // re-request.
        requested: tally(deck.requested_residency(slot)),
        blend: blend_mode(deck.blend(slot)),
        mask: masked(deck.mask(slot).kind()),
        // The angle the slot is already wearing, carried through unchanged
        // (ADR-0203).
        mask_angle: deck.mask(slot).angle(),
    })
}

/// **The engine's mask shape as the console's**, and the mirror image of
/// `karakuri_console::view::wipe_kind` on the way back out.
///
/// One function and two callers — [`mixer`] builds a strip from it every frame
/// and [`holding`] reads it at a press — because two copies of a three-arm
/// translation is exactly the shape that goes wrong the day a fourth shape
/// lands: a `match` with no wildcard stops the build in one place instead of
/// two.
pub(crate) fn masked(kind: MaskKind) -> view::Mask {
    match kind {
        MaskKind::None => view::Mask::None,
        MaskKind::Linear => view::Mask::Linear,
        MaskKind::Radial => view::Mask::Radial,
    }
}

/// **Where a press takes the trim it is standing on.**
///
/// [`offset_step`]'s shape one bay along, with the grammar's own word for a
/// direction in place of a letter: which way each press goes is a value this
/// file can be asked about without a window.
///
/// # What the page does not say, and where each answer comes from
///
/// The row names the keys and stops. So the size of a step and the destination
/// [`Step::Default`] names are `karakuri-cli`'s `'['`, `']'` and `'\\'` —
/// *"focused slot gain down / up / back to 1.0"* in `docs/manual.md` — taken
/// whole rather than invented here, because two keyboards that disagree about
/// how far one press goes is the one mistake an operator makes in the dark and
/// cannot see. **The letters were this keyboard's too until 2026-09-10**, and
/// what survived them is the arithmetic rather than the spelling.
///
/// # Floored and not ceilinged, and the clamp is the surface's
///
/// A negative gain would subtract one slot's light from another's, which is a
/// blend mode rather than a level; above 1.0 is ordinary, because the pipeline
/// is HDR
/// ([P-0064](../../../docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md)).
/// It is clamped **here** rather than left to `Deck::set_gain` for
/// `karakuri-cli`'s `clamp_gain` reason: this decides what the *record* says,
/// so a session replays the value that took effect rather than one the engine
/// quietly corrected.
///
/// **So [`Step::Default`] is a destination and the other two are steps**, and
/// all three leave as the same absolute [`Operation::SetGain`] — an absolute
/// value can express every step and a step cannot express a setting.
///
/// **It took the letter and takes the step since 2026-09-10.** `[`, `]` and
/// `\` are unbound: the trim is reached by addressing it — `space` on the
/// Mixer's strip — and the arrows step it (ADR-0259, ADR-0333). The pair of
/// directions and the tenth between them are unchanged and are still
/// `karakuri-cli`'s, which is what the paragraphs above are about; what went is
/// the letter that named each one.
pub(crate) fn gain_key(step: Step, from: f32) -> f32 {
    let asked = match step {
        Step::Down => from - GAIN_STEP,
        Step::Up => from + GAIN_STEP,
        Step::Default => 1.0,
    };
    asked.max(0.0)
}

/// **Where a press takes the fader it is standing on** — [`gain_key`]'s
/// function on the other control.
///
/// The pair and the tenth between them are `karakuri-cli`'s, for the reason
/// written at [`gain_key`]: the page names the pair and not the direction, and
/// `;` down and `'` up were the letters until 2026-09-10.
///
/// **The fader gains a default here and did not have one.** The trim's `\`
/// had no partner on this control, so `space` on an addressed fader is the
/// first way back to unity it has ever had — ADR-0259's *"on a level, the one
/// state worth naming is the value it was declared at"*, which is the clause
/// that record buys with an argument rather than finds.
///
/// **Held inside `[0, 1]` where the gain is only floored**, which is the
/// difference the vocabulary already draws between the two: opacity is a
/// proportion of a blend and there is no such thing as 1.4 of one, where gain
/// is a level into an HDR mix. The clamp is this surface's for [`gain_key`]'s
/// reason — it decides what the record says.
pub(crate) fn opacity_key(step: Step, from: f32) -> f32 {
    let asked = match step {
        Step::Down => from - OPACITY_STEP,
        Step::Up => from + OPACITY_STEP,
        Step::Default => 1.0,
    };
    asked.clamp(0.0, 1.0)
}

/// **Where a press takes the master out** — [`gain_key`]'s function one bay
/// down, and the three answers are the same three.
///
/// **A tenth, and the same tenth**: the trim, the fader and this are one
/// gesture on three controls, and a keyboard that stepped each of them by a
/// different amount would be three keyboards. **Held inside `[0, 1]` where
/// the gain is only floored**, which is [`opacity_key`]'s distinction met on
/// the level the whole programme leaves through: `Knob::Out` drags over
/// exactly that range, so a key and a hand can reach the same values and no
/// others.
///
/// **The default is unity**, which is where a run starts and what the row
/// reads before anybody has touched it.
pub(crate) fn out_key(step: Step, from: f32) -> f32 {
    let asked = match step {
        Step::Down => from - OPACITY_STEP,
        Step::Up => from + OPACITY_STEP,
        Step::Default => 1.0,
    };
    asked.clamp(0.0, 1.0)
}

/// **Where a press takes the exposure** — a **quarter stop**, which is the
/// step the console's own track was built for.
///
/// `karakuri_console::view::EXPOSURE_TRACK_W`'s documentation is where that
/// number comes from and it says the whole argument: *"one pixel a press …  a
/// pointer on this track can ask for any of the 48 positions along it and a
/// keyboard stepping a quarter stop at a time can ask for any of the 48 values
/// between the ends, so neither surface can reach a value the other cannot"*.
/// So the step is taken on the **track's** axis and converted back, rather
/// than as a multiplier written here — the two surfaces then land on the same
/// 48 values by construction.
///
/// **Clamped by the conversion rather than here**: `unit_of` holds a value
/// past either end at that end and `exposure_at` runs over `[0, 1]`, which is
/// where `--exposure 200` is allowed to be unclamped and a press is not.
///
/// **The default is 1.0**, which is the middle of the track and the level a
/// run starts at — ADR-0259's *"`space` returns it to 1.0 — today's `` ` ``"*.
pub(crate) fn exposure_key(step: Step, from: f32) -> f32 {
    let at = view::unit_of(from);
    match step {
        Step::Down => view::exposure_at(at - 1.0 / view::EXPOSURE_TRACK_W),
        Step::Up => view::exposure_at(at + 1.0 / view::EXPOSURE_TRACK_W),
        Step::Default => 1.0,
    }
}

/// **Where a press takes the latency offset** — five milliseconds, which is
/// the page's own step and the one `karakuri-cli`'s `o` and `p` use.
///
/// **The sign is the half that gets read wrong at two in the morning**, and
/// `docs/manual/console.html` says so: *"Negative and the picture waits for the
/// music, positive and it leads."* So `Step::Down` is the picture waiting, and
/// this function is where a test can ask which way each direction goes — a
/// pair wired the wrong way round reads correct and points backwards.
///
/// `o` and `p` were this keyboard's letters until 2026-09-10, and what
/// survived them is the step rather than the spelling: the constant is
/// `karakuri_environment::audio`'s, which is what the command line steps by,
/// and this program does not keep a second copy of it.
///
/// **Not clamped here**, which is the one place this differs from [`gain_key`]
/// and [`opacity_key`]: the offset's range is `karakuri_environment::audio`'s
/// and the session holds a press at the end of its travel and says so
/// ([`offset_said`]). A second clamp here would decide the same thing twice.
///
/// **The default is zero**, which is the value the offset is declared at: a
/// session nobody has nudged runs at no offset at all.
pub(crate) fn offset_key(step: Step, from: f32) -> f32 {
    match step {
        Step::Down => from - audio::LATENCY_OFFSET_STEP_MS,
        Step::Up => from + audio::LATENCY_OFFSET_STEP_MS,
        Step::Default => 0.0,
    }
}

/// **The slot a press or a record names, as an index this deck has**, or
/// `None` where it has not got one.
///
/// `Deck::gain` and `Deck::set_gain` index their slots, and a panic reachable
/// from an event handler aborts this process rather than unwinding (see the
/// module documentation), so every route from a letter or a record to the deck
/// asks this first. `mix::change`'s whole reason for taking a `slot_count` is
/// that a stream may name a slot that is not there.
///
/// **Nothing in this file can produce one**: the strips are the deck's own
/// count, and `View::select` refuses a deck the mixer draws no strip for — so
/// this is the guard rather than the message, and the real sentence is
/// `karakuri-cli`'s `no_such_slot`.
///
/// **One derivation and not one per caller**, which is what makes the key arms
/// and [`apply`] refuse the same slot: a press reads the deck before it names
/// a destination and the record writes it afterwards, and a guard on only the
/// second of the two would be a read that panicked on its way to a refusal.
pub(crate) fn held(deck: &Deck, slot: u8) -> Option<EngineSlot> {
    EngineSlot::new(slot, deck.slot_count())
}

/// **The engine's look, as the console reads it** — [`blend_mode`]'s function
/// one row up, on the value every sink is drawn under.
///
/// Two fields of three: `white_point` is Reinhard's parameter, it is on no
/// surface, and a console field for it would be a reading no control names —
/// see `karakuri_console::view::Look`. The operator goes through
/// [`mix::tonemap`], which is the match that makes the engine's list and the
/// vocabulary's agree and stops compiling the day a fifth operator lands on
/// one side only.
pub(crate) fn look(look: &Look) -> view::Look {
    view::Look {
        tonemap: mix::tonemap(look.op),
        exposure: look.exposure,
    }
}

/// **What this window says when a control's operation wrote no record**, and
/// the two ways that happens are not the same thing — so they are not the
/// same sentence.
///
/// [`written`] has three answers and only one of them is a record.
/// A harness that printed a line for that one and nothing at all for the
/// other two would tell an operator that a press did nothing, which is true
/// of neither:
///
/// - [`Written::Silent`] is **settled**. Selecting a deck or folding a bay is
///   a surface's own state and there is nothing to write; the sentence says
///   which of the four kinds of nothing it is, and that is the end of it.
/// - [`Written::Owed`] is **a gap nobody has closed yet**. A tap owes a
///   record and no build can make it, so a press that reads as *nothing
///   happened* is exactly the wrong reading — the sentence names the question
///   instead, which is `Owed::why`'s whole job and the reason `Owed` is not
///   an error.
///
/// `None` for [`Written::Records`], because that line is [`apply`]'s: it says
/// the record *and* what the deck holds afterwards, and printing both would
/// say one press twice.
///
/// **Nothing on this panel reaches the `Owed` arm on purpose any more**, and
/// the paragraph that used to stand here is worth keeping as history because
/// it was twice wrong in the same place. It first said no control could reach
/// either arm and was written for the day one did; the deck head was that day,
/// and it said the sync chip and the anchor beside it were **reachable
/// affordances over an unwritable record** — the press claimed, the operation
/// emitted, this sentence printed with the question in it, and the deck not
/// moving.
///
/// **What made the record unwritable was a question that had already been
/// answered.** `Transport::engaged` decides what engaging a mode means, with
/// the reason at its own definition: the anchor is the session tempo and the
/// scrub is cleared. The clamp that looked like a decision about the bytes on
/// disk is the identity on every tempo an oscillator can report, so there were
/// never two answers to choose between — only a reading nobody was handing in.
/// [`reading`] hands it in now, `written` writes `Record::Transport`, and
/// [`apply`] moves the deck, which is the ninth and tenth of this panel's ten
/// emitting controls arriving where the other eight already were.
///
/// **The refusal to route around it is what made that cheap.** A surface owns
/// the affordance and never the authority
/// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
/// so this file never wrote a `Record::Transport` of its own for a `SetSync` —
/// computing the anchor here would have been a window binary taking a decision
/// about a file format, and the printed line was the right answer until the
/// conversion existed
/// ([P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
/// What changed is the conversion, not this file's authority: the anchor is
/// still the engine's policy and this window still only reads a tempo.
///
/// So all ten controls write records. Five are the mixer's — `SetGain`,
/// `SetOpacity`, `SetBlendMode`, `SetResidency` and `SetMaskShape` — two are
/// the look's, and the last three are the deck head's: the scrub, the chip
/// that cycles and the anchor that re-asks for the mode the deck is in
/// (ADR-0218). The Outputs dot never arrives here at all, because it asks the
/// panel for an arrangement [`Op`] and the panel performs it
/// ([`Acted::Operated`]).
///
/// **The mask is also the one that can reach [`Written::Owed`] by accident**,
/// and that is worth having rather than designing away: a reading that did not
/// arrive answers `Owed(NotRead(Reading::Mask))`, so a harness that stopped
/// handing one in would say so out loud instead of moving nothing.
pub(crate) fn unwritten(operation: &Operation, written: &Written) -> Option<String> {
    match written {
        Written::Records(_) => None,
        Written::Silent(silent) => Some(format!(
            "  emitted: {operation:?} -> no record, and that is settled: {}",
            silent.why()
        )),
        Written::Owed(owed) => Some(format!(
            "  emitted: {operation:?} -> no record, and that is a gap rather than a \
             decision: {}. nothing moved, and nothing here decides it",
            owed.why()
        )),
    }
}

/// **A press on a strip or a deck key, applied to the console's own pointer**,
/// and what to say about it. `None` for every operation that is not it.
///
/// `Operation::SelectDeck` *"writes no record, and is the reason every other
/// variant names its deck instead of meaning the selected one"*, so there is
/// nothing on the deck for [`apply`] to move and the surface that emits it is
/// what performs it (ADR-0198). This is that performance, and it is one line
/// beside [`arrangement`]'s for the same reason: the alternative is a second
/// route into the view.
///
/// **A deck the mixer has no strip for is refused**, and `View::select` is
/// where that rule lives — the ring would be drawn nowhere and the library's
/// pill would name a deck a load could not reach. It is said here rather than
/// swallowed, because a key that does nothing and a key that is not bound are
/// the same experience.
pub(crate) fn pointed(view: &mut View, operation: &Operation) -> Option<String> {
    let Operation::SelectDeck { deck } = *operation else {
        return None;
    };
    let letter = deck_letter(deck);
    if usize::from(deck) >= view.mixer.len() {
        return Some(format!(
            "  select: deck {letter} refused — this deck has {} slot{}, and a selection with no \
             strip under it is a ring drawn nowhere and a `load` pill naming a deck the press \
             could not reach",
            view.mixer.len(),
            match view.mixer.len() {
                1 => "",
                _ => "s",
            }
        ));
    }
    view.select(deck);
    Some(format!(
        "  select: deck {letter} -> SelectDeck {{ deck: {deck} }} -> no record, and that is \
         settled: it is a surface's own pointer. The keys are addressed here, and the library's \
         foot reads `load -> {letter}`"
    ))
}

/// **A pick on a pane head's pulldown, applied to the console's own pointer**,
/// and what to say about it. `None` for every operation that is not it.
///
/// [`pointed`]'s shape one mark along and for its sentence:
/// `Operation::PointPane` is `Silent(Surface)`, so there is nothing on the deck
/// for [`apply`] to move and the surface that emits it performs it.
///
/// **It is not the deck selection**, and nothing here touches it — that is the
/// whole of what this mark is for (ADR-0338, decision 5): a pane can show a
/// deck the keys are not on, which is the Library bay's load pulldown's
/// argument one bay along.
///
/// **The pane is named rather than numbered**, because
/// `Operation::PointPane { pane }` is a `String` — `karakuri-operation` has no
/// dependencies and cannot hold the arrangement's handle type — so this is
/// where the name is resolved back to a position in `View::inspector`. A name
/// no pane has is refused with the two that exist, which is what the next
/// attempt needs (P-0083).
///
/// **A deck the mixer has no strip for is refused**, and `View::point_pane` is
/// where that rule lives — [`pointed`]'s own refusal, read on a pane instead of
/// on the ring.
pub(crate) fn pointed_pane(view: &mut View, operation: &Operation) -> Option<String> {
    let Operation::PointPane { pane, deck } = operation else {
        return None;
    };
    let Some(at) = view::PANE_NAMES.iter().position(|name| name == pane) else {
        return Some(format!(
            "  pane: `{pane}` refused — this console's panes are {}",
            view::PANE_NAMES.join(" and ")
        ));
    };
    let letter = deck_letter(*deck);
    if !view.point_pane(at, *deck) && usize::from(*deck) >= view.mixer.len() {
        return Some(format!(
            "  pane: `{pane}` -> deck {letter} refused — this deck has {} slot{}, and a pane \
             pointed at one it has not got is a head naming a deck with nothing under it",
            view.mixer.len(),
            match view.mixer.len() {
                1 => "",
                _ => "s",
            }
        ));
    }
    Some(format!(
        "  pane: `{pane}` -> deck {letter} -> no record, and that is settled: a pane's target is \
         a surface's own pointer. The keys stay where they are and the pane next door does not \
         move"
    ))
}

/// **What a refused `go` says**, and the whole of what this window puts on
/// this side of that seam.
///
/// `karakuri_console::view::Go` answers *which* refusal, because the console
/// is what can see a shape is unset and how many strips it drew; the sentence
/// is here because this package is the one that has anywhere to print. What
/// each of them owes is
/// [P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md):
/// a rejection is a message to whoever makes the next attempt, and it carries
/// the constraint and where to go rather than *refused*.
///
/// **`karakuri-cli`'s `c` prints the same two**, and its wording is where
/// these come from — *"a wipe needs somewhere to come from — this deck holds
/// one slot"* and *"no mask shape — `z` chooses one, and a wipe is a shape
/// moving"*. What changes on this surface is where the next attempt is made:
/// a pill two capsules to the left rather than a key.
pub(crate) fn refusal(refused: &Go, decks: usize) -> String {
    match refused {
        // **Unreachable while this window builds a full deck** — `SLOTS` is
        // `MAX_SLOTS` and the mixer draws one strip per slot — so what this
        // answers for is a console drawn before the deck reached it, and it
        // is written rather than left to be an unexplained silence. The count
        // is said because it is the thing that would have to change.
        Go::NoOtherDeck => format!(
            "  wipe: refused — this mixer draws {decks} strip{}, and a wipe needs a deck to \
             come from as well as one to arrive. Two decks side by side in the bay is what \
             makes the press mean something",
            match decks {
                1 => "",
                _ => "s",
            }
        ),
        Go::NoShape => String::from(
            "  wipe: refused — no shape chosen, and a wipe is a shape moving. The first pill \
             on this row is where one is picked: it reads `no shape` now, and a press on it \
             takes it to `left`. The vocabulary refuses this one too — \
             `Operation::Wipe` is *refused with no shape chosen* at its own definition — and \
             the refusal is here because the shape is this console's own setting",
        ),
        // A wipe is not a refusal, and this arm exists so that the day a
        // fourth answer lands somebody has to say what it reads rather than
        // a wildcard printing one of these two over it.
        Go::Wipe(operation) => format!(
            "  wipe: {} is not a refusal and this line should not have been reached",
            operation.title()
        ),
    }
}

/// **The four banks a run starts with**: bank 0 holding one lane over deck A's
/// channel fader, **muted**, and three empty banks beside it.
///
/// # Why there is a lane at all before anything has added one
///
/// `Operation::PointLane` is the control that makes a lane and **the console
/// draws none** — the mock's `+ lane` needs a target chooser nobody has drawn,
/// and the bay cannot grow a body while it is running anyway. So a first slice
/// with no lane would draw a ruler over nothing and there would be no way to
/// reach a cell (ADR-0320's consequences, and `view::sequencer`'s own list of
/// what is not drawn). This is the demonstration, written here rather than in
/// `karakuri-pattern` because it is *this program's opening state* and not
/// what a pattern is.
///
/// # Why it is muted
///
/// **An unmuted lane writes its target on every step boundary**, on-steps and
/// off-steps alike — that is what makes a lane a gate rather than a set of
/// impulses (ADR-0320). A lane over deck A's fader with every step off would
/// therefore hold deck A at `off` from the moment the window opened: the deck
/// on air would go dark, and nothing on screen would say a sequencer had done
/// it.
///
/// So the lane arrives the way the mock's third row is drawn — *"the pattern is
/// kept and drives nothing"* — and the first press is the one that starts it.
/// That is also the shape of the demonstration: mute the lane and the fader is
/// the hand's again, which is rule 02's take-back for a lane and what
/// ADR-0322 says the mute is *for*.
///
/// **`on` is 1.0 and `off` is 0.0**, which is a fader's pair: a gate.
pub(crate) fn demonstration_banks() -> karakuri_pattern::Banks {
    let mut banks = karakuri_pattern::Banks::default();
    let mut lane =
        karakuri_pattern::Lane::new(karakuri_operation::LaneTarget::Fader { deck: 0 }, 1.0, 0.0);
    lane.set_muted(true);
    if let Some(bank) = banks.at_mut(0) {
        bank.push(lane);
    }
    banks
}

/// **A lane appended to the bank the press named**, and what to say about it —
/// [`sequenced`]'s `PointLane` arm, lifted out because it is the one arm that
/// reads something other than the pattern.
///
/// # Where the two levels come from, and why they are not in the payload
///
/// A lane carries an `on` and an `off`
/// ([ADR-0320](../../docs/adr/0320-a-pattern-is-one-bar-of-sixteen-slots-a-lane-is-a-target-and-two-levels-and-a-cell-is-a-bit.md)),
/// filled in **at the press** and never read back later, because a pattern
/// outlives the Set it was written against. `Operation::PointLane` carries a
/// bank and a target and nothing else (ADR-0321), so they are filled here —
/// out of `View::inspector`, which is *the console's own published reading*
/// and the very list the chooser drew its items from.
///
/// **That is one reading and not two.** Asking the deck again here would be a
/// second derivation of the range, and the two could name different numbers
/// the frame a Set lands; the operator saw the console's, and the lane gets
/// the console's
/// ([ADR-0327](../../docs/adr/0327-the-lane-chooser-lists-one-decks-keys-and-the-bank-pills-are-the-four-banks.md)).
///
/// **A fader's are 1.0 and 0.0**, which is a gate and is that record's own
/// pair: a channel fader publishes no range, and `[0, 1]` is what the strip
/// draws.
///
/// **A parameter the console holds no row for is refused and said**, rather
/// than defaulted: a lane with invented levels would drive its target to two
/// numbers nobody chose, and the refusal names what the next attempt needs
/// ([P-0083](../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).
pub(crate) fn pointed_lane(
    banks: &mut karakuri_pattern::Banks,
    view: &View,
    pattern: u8,
    target: &karakuri_operation::LaneTarget,
) -> String {
    let levels = match target {
        karakuri_operation::LaneTarget::Fader { .. } => Some((1.0, 0.0)),
        karakuri_operation::LaneTarget::Param { deck, param } => view
            .inspector
            .iter()
            .filter(|pane| pane.deck == usize::from(*deck))
            .flat_map(|pane| pane.nodes.iter())
            .flat_map(|node| node.params.iter())
            .find(|row| &row.param == param)
            .map(|row| (row.range[1], row.range[0])),
    };
    let Some((on, off)) = levels else {
        return format!(
            "  lane: pattern {pattern} is not pointed — this console holds no published range \
             for that control, and a lane's two levels are the range published to it at the \
             press. Point the Library bay's load pulldown at the deck whose control it is and \
             ask again"
        );
    };
    let Some(bank) = banks.at_mut(usize::from(pattern)) else {
        return format!("  lane: pattern {pattern} is not a bank this session holds");
    };
    // **It arrives muted, and that is not caution.** A lane arrives with every
    // slot off, and an off step writes `off` rather than writing nothing — so
    // an unmuted fader lane would hold its deck at zero from the press, which
    // is `demonstration_banks`' argument reached from the other end: the deck
    // would go dark and nothing on screen would say a sequencer had done it.
    // The label is the unmute and it is one press, which is rule 02's take-back
    // drawn where the lane is.
    let mut lane = karakuri_pattern::Lane::new(target.clone(), on, off);
    lane.set_muted(true);
    bank.push(lane);
    let lanes = bank.lanes().len();
    format!(
        "  lane: pattern {pattern} -> a {} lane, on {on} off {off} -> no record. {lanes} lane{} \
         under the rows, muted: every slot is off and an off step writes {off}, so the label is \
         the press that starts it",
        match target {
            karakuri_operation::LaneTarget::Fader { .. } => "fader",
            karakuri_operation::LaneTarget::Param { .. } => "parameter",
        },
        match lanes == 1 {
            true => "",
            false => "s",
        }
    )
}

/// **A press in the Sequencer bay, applied to the pattern it names**, and what
/// to say about it. `None` for every operation that is not one of the three.
///
/// [`scheduled`]'s shape one bay along, and for the same reason:
/// `written` answers `Silent(Surface)` for all five of the sequencer's
/// operations — a pattern is library data under the store on the arrangement's
/// terms, and what a *lane* does reaches the stream as its own writes
/// (ADR-0320, ADR-0322) — so there is nothing on the deck for [`apply`] to
/// move and the surface that emits it is what performs it.
///
/// **Every arm names its bank**, and a press on a bank this session does not
/// have is refused and said rather than swallowed, which is [`pointed`]'s rule:
/// a press that does nothing and a press that is not bound are the same
/// experience.
///
/// **A mode press and a bank press reset the playhead**, and a step press does
/// not. The first two change what a step *index* means — the same bar read at
/// another width, or another pattern's lanes under it — so a remembered index
/// would hold the new reading silent until the bar came round. Turning a cell
/// on changes what the *current* step is worth and not which step it is, and
/// the mock already says when that is heard: *"the next time the playhead
/// reaches the cell rather than when you asked for it"*.
pub(crate) fn sequenced(
    banks: &mut karakuri_pattern::Banks,
    playhead: &mut karakuri_pattern::Playhead,
    view: &View,
    operation: &Operation,
) -> Option<String> {
    if let Operation::PointLane { pattern, target } = operation {
        return Some(pointed_lane(banks, view, *pattern, target));
    }
    match *operation {
        Operation::SetStep {
            pattern,
            lane,
            step,
            on,
        } => {
            let Some(lane_at) = banks
                .at_mut(usize::from(pattern))
                .and_then(|at| at.lane_mut(usize::from(lane)))
            else {
                return Some(format!(
                    "  step: pattern {pattern} lane {lane} is not a lane this session holds — a \
                     press names a bank, a lane and a slot, and this one names no row"
                ));
            };
            lane_at.set_slot(usize::from(step), on);
            Some(format!(
                "  step: pattern {pattern} lane {lane} slot {step} -> {} -> no record, and that \
                 is settled: a pattern is library data and what a lane does is its own writes. \
                 Heard the next time the playhead reaches it",
                match on {
                    true => "on",
                    false => "off",
                }
            ))
        }
        Operation::SetLaneMute {
            pattern,
            lane,
            muted,
        } => {
            let Some(lane_at) = banks
                .at_mut(usize::from(pattern))
                .and_then(|at| at.lane_mut(usize::from(lane)))
            else {
                return Some(format!(
                    "  lane: pattern {pattern} lane {lane} is not a lane this session holds"
                ));
            };
            lane_at.set_muted(muted);
            Some(format!(
                "  lane: pattern {pattern} lane {lane} -> {} -> no record. {}",
                match muted {
                    true => "muted",
                    false => "driving",
                },
                match muted {
                    true =>
                        "The pattern is kept and drives nothing, and the fader is a hand's \
                             again — which is where this lane's take-back sits",
                    false => "It writes its target at the next step boundary",
                }
            ))
        }
        Operation::SetPatternGrid { pattern, grid } => {
            let Some(at) = banks.at_mut(usize::from(pattern)) else {
                return Some(format!(
                    "  grid: pattern {pattern} is not a bank this session holds"
                ));
            };
            at.set_mode(grid);
            // The same bar at another width, so the index it was remembering
            // is about a reading that has gone.
            playhead.reset();
            Some(format!(
                "  grid: pattern {pattern} -> {} -> no record. The bar is one bar, so the count \
                 follows: {} steps over the same row, and the sixteen slots underneath are \
                 untouched",
                grid.name(),
                grid.count()
            ))
        }
        Operation::SelectPattern { pattern } => {
            if !banks.select(usize::from(pattern)) {
                return Some(format!(
                    "  pattern: there is no bank {pattern} — this session holds {}",
                    karakuri_pattern::BANKS
                ));
            }
            playhead.reset();
            Some(format!(
                "  pattern: bank {pattern} armed -> no record. {} lane{} under the rows",
                banks.pattern().lanes().len(),
                match banks.pattern().lanes().len() == 1 {
                    true => "",
                    false => "s",
                }
            ))
        }
        _ => None,
    }
}

/// **A press on one of the transition row's three pills, applied to the
/// console's own setting**, and what to say about it. `None` for every
/// operation that is not it.
///
/// [`pointed`]'s shape one row down, and for the same reason:
/// `Operation::SetTransition` *"changes nothing you can see and writes nothing
/// to the stream"* — `written` answers `Silent(Surface)` for it and no record
/// in `karakuri-store` carries a quantum, a length or a wipe shape — so there
/// is nothing on the deck for [`apply`] to move and the surface that emits it
/// is what performs it. `View::set_transition` is the only door into that
/// setting, which is where the refusal below lives.
///
/// **What it changes is what the next wipe means**, and that is why the line
/// says the whole row rather than the field that moved: an operator reading
/// *the shape is an iris* still has to know what grid it starts on.
///
/// **A setting no pill can draw is refused**, and it is said rather than
/// swallowed for [`pointed`]'s reason — a press that does nothing and a press
/// that is not bound are the same experience. Nothing this window emits can
/// reach it: the pills name a destination out of the console's own cycles. It
/// is a mapped controller or an MCP call that could, the day either reaches
/// this row.
pub(crate) fn scheduled(view: &mut View, operation: &Operation) -> Option<String> {
    let Operation::SetTransition { setting } = operation else {
        return None;
    };
    if !view.set_transition(*setting) {
        let at = view.transition();
        return Some(format!(
            "  transition: {setting:?} refused or already there — the row is on `{}`, `{}`, \
             `{}`, and a pill draws only what its own cycle names",
            at.shape_word(),
            at.quantum_word(),
            at.length_word()
        ));
    }
    let at = view.transition();
    Some(format!(
        "  transition: {setting:?} -> no record, and that is settled: it is a surface's own \
         setting. The next fade, crossfade or wipe is `{}` on the `{}`, over {} beat{}",
        at.shape_word(),
        at.quantum_word(),
        at.length,
        match at.length == 1.0 {
            true => "",
            false => "s",
        }
    ))
}

/// **A candidate kept, applied to the lane**, and what to say about it.
/// `None` for every operation that is not it.
///
/// [`scheduled`]'s shape one bay over, and for its reason: `written` answers
/// `Silent(Silent::Surface)` for `Operation::KeepCandidate`, so there is no
/// record for [`apply`] to move a deck with and the surface that draws the row
/// is what performs the press. What it changes is one line in one list.
///
/// **Nothing else moves, and that is the operation rather than a shortfall.**
/// The version is where it was, the store holds every version it held, and the
/// picture is the picture. What a keep says is that a person has looked at
/// this node and is done with it — `console.html`'s *Accepting settles the
/// node and changes nothing on screen*.
///
/// **A row that is not there is said rather than swallowed**, which is
/// [`pointed`]'s rule: nothing this window emits can reach it — the control is
/// the row and a row that is not drawn takes no press — so a line here is a
/// mapped controller or an MCP call arriving at a node with no candidate on
/// it, the day either reaches this row.
pub(crate) fn kept(view: &mut View, operation: &Operation) -> Option<String> {
    let Operation::KeepCandidate { deck, node } = operation else {
        return None;
    };
    let slot = usize::from(*deck);
    let addr = node_addr(ir_layer(node.layer), node.index);
    let before = view.staging.len();
    view.staging
        .retain(|row| row.deck != slot || row.at != Some(*node));
    let letter = deck_letter(*deck);
    if view.staging.len() == before {
        return Some(format!(
            "  keep: deck {letter} {addr} has no candidate row — nothing was outstanding on \
             that node, and the lane is as it was"
        ));
    }
    Some(format!(
        "  keep: deck {letter} {addr} -> KeepCandidate -> no record, and that is settled: the \
         material already changed and its `procedure` record was written at the swap. The row \
         leaves the lane and nothing else moves; {} still waiting",
        match view.staging.len() {
            0 => "nothing".to_owned(),
            n => format!(
                "{n} row{}",
                match n {
                    1 => "",
                    _ => "s",
                }
            ),
        }
    ))
}

/// **Which salt a slot's material is seeded from** — the one it was built at,
/// and the one a load restates when the Set file recorded none.
///
/// Deck A's is [`SEED_SALT`] and every slot after it is one further along, so
/// this deck's four slots are four different simulations of the one procedure
/// [`Sources`] names. **That generalises the reason the second salt was
/// written for and then retires the constant.** `WARM_SEED_SALT` existed so
/// that *the slot the budget parks is a different simulation rather than a
/// second copy of the same one* — an argument about the parked slot, made when
/// the parked slot was the only other slot there was. What it was really
/// saying is that a deck of one picture repeated is not a mixer, and that is
/// true of every slot rather than of deck B, so it is said once here and no
/// constant states a reason that has gone
/// ([`docs/contributing.md` §4](../../../docs/contributing.md)).
///
/// **`+ slot` rather than a table**, because a table of four numbers is four
/// values with nothing to say about each other, and what is wanted is exactly
/// *distinct, and deck A's is the one the CLI's tests use*. Distinctness is
/// then arithmetic rather than four typed numbers nobody re-reads — which
/// `every_slot_is_its_own_simulation` asserts salt by salt, off the Sets the
/// deck actually built rather than off this function.
///
/// **The salts the run was built with, and no others**: a Set loaded into a
/// slot is new *material* and not a new simulation, so a rebuild that derived
/// its own seed would repaint every element in the slot for a reason nobody
/// asked for — which is `Watch::salts`' own argument, met from the loading
/// side.
pub(crate) fn slot_salt(slot: usize) -> u32 {
    SEED_SALT + slot as u32
}

/// **What the derived material a procedure load leaves is a derivation *of***:
/// the Set the slot is filed under, or the pair the run was launched with where
/// it is filed under none.
///
/// **`watch::Aim::set` first**, because that is the one field a load moves and a
/// procedure load does not (ADR-0304, ADR-0338): a slot that has been loaded is
/// running that Set with one layer over it, and the strip has to say so. A slot
/// nobody has loaded is running the launch pair, which no id names — that is
/// the state `Aim::set` is `None` in, and the launch pair is what the strip has
/// been reading since the first frame.
pub(crate) fn base_material(set: Option<&str>, launch: &str) -> String {
    set.map(str::to_owned).unwrap_or_else(|| launch.to_owned())
}

/// **What the strip reads once a layer has been written over what a deck is
/// playing**: `<base> + <kir>`, which is the maintainer's own
/// `drift_night + orbit_wide`.
///
/// So what is on air says what it is made of and never claims to be a Set the
/// library holds — `keep` is what gives it a name (ADR-0338).
pub(crate) fn derived_material(base: &str, procedure: &str) -> String {
    format!("{base} + {procedure}")
}

/// **Write one procedure over the layer it declares and re-aim the slot**, or
/// say why it did not.
///
/// # Where the file comes from, and it is the two tiers and nothing else
///
/// `<store>/procedures/<name>.kir` first and the presets root's
/// `<name>.kir` after it, which is the order the Library bay lists them in and
/// the only two places a procedure row can have come from (ADR-0227's two tiers,
/// ADR-0338's decision 1). The content-addressed artifacts at the store root are
/// **not** searched: that population is the edit history's, addressed by hash,
/// and a name is not one.
///
/// # Which position it lands on, and the limit is recorded rather than designed around
///
/// **The first node of that kind.** A procedure declares one `kind` and nothing
/// about where it goes, and a library row cannot say an index — so the payload
/// carries none, and `L4:0` is the renderer a `kind L4` replaces. The second
/// renderer of a three-renderer Set is unreachable from this row, and the day
/// the Inspector's node head grows a *replace this node* control is the day the
/// payload gains a `NodeAddress` (ADR-0338, stated at the point it bites).
///
/// **Where the slot has no node of that kind the procedure is added as node 0 of
/// it**, which is the case the request is about: a Set of a geometry and a
/// renderer declares no camera, so it holds the built-in orbit at `L3:0` and a
/// `kind L3` row takes that position — the picture changes camera with nothing
/// else moving.
///
/// # What each file already on the slot is
///
/// Read off the files themselves with `history::declared_kind`, which is the one
/// scanner for a `kind` line, and with `compile`'s own fallback where a file
/// declares none — the first node is an L1 and the rest are L4s, which is what
/// a bare pair is. That is one small read per node, on the press, and it
/// compiles nothing (P-0091).
///
/// # The node name is kept, and that is what keeps the edges
///
/// A replaced position keeps the **name the Set gave that node**, because an
/// `edge` and a `bind` in the aim resolve against it: a rebuild that renamed the
/// node would break the wiring the slot is running. A node that is *added* is
/// named after the row, and a name the slot already holds is refused rather than
/// shadowed.
pub(crate) fn overlaying(
    root: &std::path::Path,
    presets: Option<&std::path::Path>,
    slot: usize,
    aim: &mut Aiming,
    name: &str,
) -> Result<String, String> {
    let (source, tier) = kept_source(root, presets, name)?;
    let kind = karakuri_environment::history::declared_kind(&source).ok_or_else(|| {
        format!(
            "`{name}` declares no `kind`, so there is no layer to write it over — a procedure \
             says which layer it implements in a `kind` line, and this one says nothing"
        )
    })?;
    // **Head and rest are one list here**, because *the first node of that kind*
    // is a question about the slot's files in order and the split is only how a
    // `watch::Aim` carries them.
    let mut files: Vec<karakuri_environment::compile::Named> = std::iter::once(aim.at.head.clone())
        .chain(aim.at.rest.iter().cloned())
        .collect();
    let at = files.iter().enumerate().find_map(|(index, file)| {
        let source = std::fs::read(&file.path).ok()?;
        let declared = karakuri_environment::history::declared_kind(&source)
            .unwrap_or(if index == 0 { "L1" } else { "L4" });
        (declared == kind).then_some(index)
    });
    let (at, added) = match at {
        Some(at) => (at, false),
        None => (files.len(), true),
    };
    if added && files.iter().any(|file| file.name.as_deref() == Some(name)) {
        return Err(format!(
            "deck {} already holds a node called `{name}`, and this row would add a second — \
             rename one of them and load again",
            deck_letter(slot as u8)
        ));
    }
    let path = karakuri_environment::scratch::place(
        root,
        &karakuri_environment::scratch::node_name(slot, at, name),
        &String::from_utf8_lossy(&source),
    )?;
    match added {
        // **Named after the row**, because nothing in the slot named it: a node
        // added here is addressed by the name an operator can read off the row
        // they pressed.
        true => files.push(karakuri_environment::compile::Named {
            name: Some(name.to_string()),
            path,
        }),
        // **The node keeps the name the Set gave it**, which is what an `edge`
        // resolves against — see this function's own head.
        false => files[at].path = path,
    }
    let mut files = files.into_iter();
    let head = files
        .next()
        .ok_or_else(|| String::from("this deck is running no files at all"))?;
    let rest: Vec<karakuri_environment::compile::Named> = files.collect();
    // **Every other field restated**, which is `Aiming::changed`'s single
    // derivation: the layering, the fold, the capacity, the seed and the salts,
    // the camera, the overrides, the wiring, the grants and the Set this slot is
    // filed under all come back as the slot's own rather than as a default
    // (ADR-0314). `Aim::set` is among them, which is why the history goes on
    // filing under the base.
    aim.changed(|at| {
        at.head = head;
        at.rest = rest;
    })
    .map_err(|()| {
        format!(
            "deck {}'s build worker is gone, so `{name}` cannot be built; what is on that deck \
             keeps running",
            deck_letter(slot as u8)
        )
    })?;
    let where_ = match added {
        true => format!("added as {kind}:0, which this deck had no node for"),
        false => format!("written over {kind}:0"),
    };
    Ok(format!(
        "  load: `{name}` ({tier}) {where_} on deck {} — the slot is recompiling on the worker \
         with every other layer where it was, and the staging lane says whether the build \
         landed, was overloaded or did not compile",
        deck_letter(slot as u8)
    ))
}

/// **A procedure's bytes, out of whichever tier holds it**, with the word for
/// the tier so the sentence a press prints says where the file came from.
///
/// The operator's own first and what ships after it, which is the order the
/// listing draws them under `all` and `presets`: a name kept in this store is
/// this store's answer, and nothing this program does writes where the presets
/// are (P-0096).
pub(crate) fn kept_source(
    root: &std::path::Path,
    presets: Option<&std::path::Path>,
    name: &str,
) -> Result<(Vec<u8>, &'static str), String> {
    if let Ok(store) = Store::open(root) {
        if let Ok(source) = store.read_procedure(name) {
            return Ok((source, "kept"));
        }
    }
    let shipped = presets.map(|dir| dir.join(format!("{name}.kir")));
    if let Some(path) = shipped {
        if let Ok(source) = std::fs::read(&path) {
            return Ok((source, "shipped"));
        }
    }
    Err(format!(
        "no procedure named `{name}` — this store's `{}/` does not hold one and the presets root \
         does not ship one, so the row it was listed under has gone since the listing was built",
        Store::PROCEDURES
    ))
}

/// **A deck's renderers folded or overdrawn, performed** — the Inspector deck
/// head's fold pressed, and `None` for every operation that is not one.
///
/// # It is [`played`]'s shape with one field instead of every field
///
/// A library load re-points a slot at a different Set's files; this re-points a
/// slot at *the files it is already on*, with the layering changed. Both are
/// one `Aiming::changed`, both are judged by the same watchdog, and neither
/// touches the deck — see [`Aiming::changed`], where the argument is, and
/// `docs/adr/0314-…`, which is the record.
///
/// **`written` answers `Silent(NoRecord)`**, exactly as it does for
/// `Operation::LoadSet`, so the surface that names it is the surface that
/// performs it and there is nothing for [`apply`] to do. What a session stream
/// has for a layering is `Record::Merge`, which is a **Set file's** statement
/// about a Set and carries no slot; nothing in the vocabulary says *the Set in
/// slot 3 composites*, and inventing a record here would be inventing the
/// record stream (ADR-0046's rule, met from the panel).
///
/// # A press that asks for the state the slot is in is refused rather than sent
///
/// Not because asking twice is wrong — [`DeckHead::compositing`] names a
/// destination, and naming the one you are on is how the anchor beside it
/// re-anchors — but because *here* it would buy a recompile of the whole slot
/// and change nothing about the picture, which is a cost paid for nothing
/// ([P-0091](../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)).
/// The chip cannot produce one, since it reads the state the frame drew; MIDI,
/// a key or a model can, the day any of them names this operation.
///
/// Every failure is a sentence and none of them moves anything: a slot the deck
/// has not got, or a build worker that has gone.
///
/// **A free function over the aims and not over [`Gfx`]**, which is [`rewired`]'s
/// arrangement and its reason: the whole of what this decides is the field, the
/// refusal and the sentence, and none of the three needs a window, a device or
/// a `Deck` to check. [`played`] beside it takes the program because a load
/// reads a store and writes the strip's name; this one touches nothing but the
/// aim.
pub(crate) fn composited(aims: &mut [Aiming], operation: &Operation) -> Option<String> {
    let Operation::SetCompositing { deck, compositing } = operation else {
        return None;
    };
    let slot = usize::from(*deck);
    let letter = deck_letter(*deck);
    let count = aims.len();
    let Some(aim) = aims.get_mut(slot) else {
        return Some(format!(
            "  composite: {}, and nothing was re-aimed",
            karakuri_environment::no_such_slot(slot, count)
        ));
    };
    let want = match compositing {
        true => karakuri_engine::set::Layering::Composite,
        false => karakuri_engine::set::Layering::Overdraw,
    };
    let word = match compositing {
        true => "composite",
        false => "overdraw",
    };
    if aim.at.layering == want {
        return Some(format!(
            "  composite: deck {letter} is already set to {word} its renderers — nothing was \
             sent, because a re-aim rebuilds the whole slot and this one would land on the same \
             picture"
        ));
    }
    match aim.changed(|at| at.layering = want) {
        Ok(()) => Some(format!(
            "  composite: deck {letter} re-aimed to {word} its renderers — the slot is \
             recompiling on the worker, and the staging lane says whether the build landed, was \
             overloaded or did not compile"
        )),
        Err(()) => Some(format!(
            "  composite: deck {letter} will {word} its renderers from the next build on, but \
             this slot's build worker has ended, so nothing will rebuild and what is on that \
             deck is still running"
        )),
    }
}

/// **A deck's element count moved, performed** — the Inspector deck head's
/// capacity chip pressed, and `None` for every operation that is not one.
///
/// # It is [`composited`]'s shape with a different field of the aim
///
/// A capacity is one field of the description a slot's watcher is pointed at,
/// so this restates the other thirteen and sends it and the worker recompiles
/// the slot off the render thread — the route ADR-0228 opened and ADR-0314
/// walked, and the reason a *setter* on `Set` was never what this waited on.
/// `written` answers `Silent(NoRecord)` for `SetProperty` exactly as it does
/// for `SetCompositing` and `LoadSet`, so there is nothing for [`apply`] to do:
/// `Record::Capacity` is a **Set file's** statement about a Set and carries no
/// slot. A session replayed therefore does not come back at a capacity a hand
/// stepped to — the load's cost, unchanged in size; **a deck kept does**, since
/// a keep writes one `capacity` record per geometry off what the Set is running
/// at (`docs/adr/0328-…`).
///
/// # It is the whole slot, and that is the aim's shape rather than a shortcut
///
/// `watch::Aim::capacity` is one `Option<u32>` and is `--capacity`'s own field:
/// *"`--capacity` overrides every source"*. So a Set holding two geometries
/// runs both at this number. That is ADR-0228's recorded limit met from the
/// asking side rather than worked around, and it is why
/// `karakuri_operation::Property::Capacity` names no node.
///
/// **A press asking for the capacity the slot is already aimed at is refused
/// with a sentence** and nothing is sent, on the fold's terms: it would buy a
/// recompile of the whole slot and land on the same picture. The chip cannot
/// produce one — its step is strictly above what the slot is running — and a
/// **model can**, since `set_property` names the number outright and arrives
/// here as an `Acted::Emitted` like any press. That is why the guard is here
/// and not in the console: what may be asked for is not a surface's to decide,
/// so every way in meets the same wall in the same sentence
/// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
///
/// **A free function over the aims**, for [`composited`]'s reason: the field,
/// the refusal and the sentence are the whole of what it decides.
pub(crate) fn resized(aims: &mut [Aiming], operation: &Operation) -> Option<String> {
    let Operation::SetProperty {
        deck,
        property: karakuri_operation::Property::Capacity { elements },
    } = operation
    else {
        return None;
    };
    let slot = usize::from(*deck);
    let letter = deck_letter(*deck);
    let count = aims.len();
    let Some(aim) = aims.get_mut(slot) else {
        return Some(format!(
            "  capacity: {}, and nothing was re-aimed",
            karakuri_environment::no_such_slot(slot, count)
        ));
    };
    if aim.at.capacity == Some(*elements) {
        return Some(format!(
            "  capacity: deck {letter} is already aimed at {elements} elements a geometry — \
             nothing was sent, because a re-aim rebuilds the whole slot and this one would land \
             on the same picture"
        ));
    }
    match aim.changed(|at| at.capacity = Some(*elements)) {
        Ok(()) => Some(format!(
            "  capacity: deck {letter} re-aimed to {elements} elements a geometry — the slot is \
             recompiling on the worker, and the staging lane says whether the build landed, was \
             overloaded or did not compile. A number outside what a geometry declares is refused \
             there, by name and with the range"
        )),
        Err(()) => Some(format!(
            "  capacity: deck {letter} will run at {elements} elements a geometry from the next \
             build on, but this slot's build worker has ended, so nothing will rebuild and what \
             is on that deck is still running"
        )),
    }
}

/// **An input rewired, performed** — a pick out of a `uses` line's card, and
/// `None` for every operation that is not one.
///
/// # It is the route a model's `wire_input` already takes, reached from a press
///
/// [`rewired`] is the whole of what a rewiring decides — the run's wiring, the
/// re-aim and the sentence — and it was written for the MCP surface, one
/// request per frame, with a slot number it does not trust. A press is one
/// request of exactly that shape, so this hands it one and prints what comes
/// back: the panel and a model rewire through one function, and a defect in
/// either is a defect in both rather than in whichever was tried
/// ([P-0085](../../../docs/principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md)).
///
/// **Nothing is validated here.** The card offers nodes the pane could see and
/// a name the Set cannot use is refused where the Set is *built*, by name and
/// with what the Set does hold — which is the wall every way in meets, in one
/// sentence
/// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
///
/// **`written` answers `Silent(NoRecord)`** for `WireInput` and still does:
/// `Record::Edge` is a **Set file's** statement about a Set and carries no
/// slot, so a session replayed does not come back rewired where a hand asked
/// for it — and **a keep does**, since a keep writes the run's edges into the
/// file it saves. That is `SetProperty`'s division and `LoadSet`'s hole, not a
/// new one (`docs/adr/0329-…`).
pub(crate) fn wired_input(
    edges: &mut Vec<karakuri_engine::set::Edge>,
    aims: &mut [Aiming],
    slot_count: usize,
    operation: &Operation,
) -> Option<String> {
    let Operation::WireInput {
        deck,
        node,
        slot,
        to,
    } = operation
    else {
        return None;
    };
    let asked = [(
        usize::from(*deck),
        karakuri_engine::set::Edge {
            node: node.clone(),
            slot: slot.as_str().into(),
            to: to.clone(),
        },
    )];
    // **One request, so one answer** — `rewired` answers per request and this
    // hands it exactly one, which is why the `into_iter().next()` below cannot
    // be an empty list.
    let said = rewired(&asked, edges, aims, slot_count)
        .into_iter()
        .next()?;
    Some(match said {
        Ok(line) => format!("  wire: {line}"),
        Err(line) => format!("  wire: {line}"),
    })
}

/// **A deck's published interface narrowed or widened, performed** — a
/// parameter row's publish mark pressed, and `None` for every operation that is
/// not one.
///
/// # It is a field of the aim, which is what makes the choice survive
///
/// `watch::Aim::published` is the interface a slot's watcher states at every
/// build, and it has been empty in every run this program has ever had — an
/// empty list *is* **publish everything**, so nothing had to fill it until
/// something narrowed. This fills it, and the reason it is the aim rather than
/// a writer into the live `Set` is the one thing that decides between them: a
/// live write is wiped by the next rebuild, and the next rebuild is the
/// operator's own next save of any `.kir` in the deck. A control that undoes
/// itself on an unrelated act is the defect ADR-0280 §6 named for parameters
/// and ADR-0282 closed; there is no `Set::carry_moved_from` for an interface,
/// so the aim is where it has to live (`docs/adr/0329-…`).
///
/// **The cost is a recompile for a choice about a display**, and it is named
/// rather than hidden: the build is the same files at the same capacity, so it
/// is a build that has already landed once, and the Staging lane carries the
/// verdict like every other.
///
/// **`written` answers `Silent(NoRecord)`**, and here that is a **gap in the
/// format** rather than a record with no slot: nothing in this vocabulary says
/// what a Set publishes, in a Set file or in a session. So a replay does not
/// come back narrowed and **neither does a keep** — which is what makes this
/// the weakest of the four re-aims on that row, and both manual pages say so.
///
/// **An empty list is not nothing.** `Publish { controls: [] }` asks for *every
/// declared control published*, which is what an unnarrowed deck is, and it is
/// what a press that takes the last control off the interface would mean if
/// anything could produce one — nothing can, because taking a row off leaves
/// the rest on it.
pub(crate) fn attended(aims: &mut [Aiming], operation: &Operation) -> Option<String> {
    let Operation::Publish { deck, controls } = operation else {
        return None;
    };
    let slot = usize::from(*deck);
    let letter = deck_letter(*deck);
    let count = aims.len();
    let Some(aim) = aims.get_mut(slot) else {
        return Some(format!(
            "  publish: {}, and nothing was re-aimed",
            karakuri_environment::no_such_slot(slot, count)
        ));
    };
    let published: Vec<karakuri_engine::set::Published> = controls
        .iter()
        .map(|control| karakuri_engine::set::Published {
            name: control.name.clone(),
            at: control.node.map(|node| (ir_layer(node.layer), node.index)),
            key: control.key.clone(),
            range: control.range,
        })
        .collect();
    let shown = published.len();
    match aim.changed(|at| at.published = published) {
        Ok(()) => Some(format!(
            "  publish: deck {letter} re-aimed to publish {shown} control{} — the slot is              recompiling on the worker, and the staging lane says whether the build landed, was              overloaded or did not compile. What is off the interface is still written by              `--param`, by a `param` record and by a model naming its address",
            match shown {
                1 => "",
                _ => "s",
            }
        )),
        Err(()) => Some(format!(
            "  publish: deck {letter} will publish {shown} control{} from the next build on, but              this slot's build worker has ended, so nothing will rebuild and what is on that              deck is still running",
            match shown {
                1 => "",
                _ => "s",
            }
        )),
    }
}

/// **A deck re-seeded, performed** — the Inspector deck head's `re-salt`
/// capsule pressed, and `None` for every operation that is not one.
///
/// # The salts are cleared and the seed is stated, which is one derivation
///
/// `watch::Aim` carries both a `seed_salt` for the slot and a `salts` list per
/// geometry, and the list wins where it is filled: `Set::build` reads a
/// recorded salt and falls back to `derived_salt(seed_salt, ordinal)`. So a
/// re-salt that wrote only the seed would move nothing on a slot filled from a
/// Set file, which records one `seed` line per geometry. It **clears the list**
/// instead of rewriting it, which is the same numbers with the arithmetic left
/// where it belongs: the engine derives each geometry's salt from the slot's,
/// and this program does not keep a second copy of that function
/// ([P-0087](../../../docs/principles/0087-name-the-property-never-the-shape.md)).
///
/// # Nothing here invents the number
///
/// The salt arrives in the operation, because the console was handed it: the
/// pane reads `Set::source_salts`, which is what the slot is actually running,
/// and the next value of the sequence comes off
/// `karakuri_engine::set::derived_salt` — so a press names a destination like
/// every other control on this row, the same press from the same place lands on
/// the same picture twice, and nothing on this panel produces a frame a later
/// run cannot produce again
/// ([P-0092](../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)).
///
/// **Nothing is refused.** The sequence goes forward, so a press cannot ask for
/// the salt the slot is already on, and a re-seed always changes the picture —
/// which is what the fold's *already in that state* guard exists for and this
/// one does not need.
pub(crate) fn re_salted(aims: &mut [Aiming], operation: &Operation) -> Option<String> {
    let Operation::SetProperty {
        deck,
        property: karakuri_operation::Property::Seed { salt },
    } = operation
    else {
        return None;
    };
    let slot = usize::from(*deck);
    let letter = deck_letter(*deck);
    let count = aims.len();
    let Some(aim) = aims.get_mut(slot) else {
        return Some(format!(
            "  re-salt: {}, and nothing was re-aimed",
            karakuri_environment::no_such_slot(slot, count)
        ));
    };
    match aim.changed(|at| {
        at.seed_salt = *salt;
        at.salts.clear();
    }) {
        Ok(()) => Some(format!(
            "  re-salt: deck {letter} re-aimed to seed {salt} — the slot is recompiling on the \
             worker, and its randomness moves while its structure does not. The staging lane says \
             whether the build landed, was overloaded or did not compile"
        )),
        Err(()) => Some(format!(
            "  re-salt: deck {letter} will be seeded from {salt} from the next build on, but this \
             slot's build worker has ended, so nothing will rebuild and what is on that deck is \
             still running"
        )),
    }
}

/// **What a [`karakuri_operation::Revision`] asked for, as a refusal names
/// it** — a version by the name it was filed under, or a node by its address.
///
/// One function because the two refusals about the *deck* are the same
/// sentence whichever arm arrived: a slot the deck has not got and a deck
/// playing no Set are answered before anything is read, and what they have to
/// name is only what was asked for.
pub(crate) fn asked_for(revision: &karakuri_operation::Revision) -> String {
    match revision {
        karakuri_operation::Revision::Picked(name) => format!("`{name}`"),
        karakuri_operation::Revision::Previous(node) => format!(
            "the version before {}'s",
            node_addr(ir_layer(node.layer), node.index)
        ),
    }
}

/// **The half of [`restored`] that reaches a disk**, split out for the reason
/// [`seeded`] is a free function: `main` cannot be entered from a test, a
/// `Gfx` cannot be built without a device, and what this does is worth
/// asserting — it writes over the file a deck is playing from.
///
/// Everything it needs is an argument: the store to walk, the slot and its
/// letter, the Set the slot is running, the revision that was asked for, and
/// where that slot's nodes are ([`karakuri_mcp::Slots`], the
/// run's one published layout, which the caller reads off [`Engine::pointing`]).
/// The refusals here are the ones that are about **files** — a version that is
/// not in the listing, a node with nothing behind the one it is playing, a
/// node this slot does not hold, and a file that will not be read or written —
/// where the two about the *deck* are the caller's and are answered before
/// this is reached.
///
/// # One function and two ways of naming the file
///
/// `karakuri_operation::Revision` has two arms because two surfaces can ask
/// and each says the half it holds (ADR-0308), and what differs between them
/// is **which row of this listing** — nothing after that. So the walk, the
/// read, the address and the write are one path, and the arms are one `match`
/// over the same `Vec<Version>`:
///
/// - `Picked` is a name the Library bay's `history` scope handed over, matched
///   back against the listing that produced it by rebuilding each row's
///   spelling — `SetTransfer::Take`'s own arrangement, and the reason no
///   surface here spells a path.
/// - `Previous` is a node the Staging lane's row handed over, and the version
///   is **the one before the one running**: the listing is most recent first,
///   the newest entry for that node is what the slot is playing — the history
///   is gated on compiling and not on landing, so a version that was stopped
///   for cost is filed too — and the entry after it is the step back
///   (`docs/adr/0326-a-staging-row-is-a-changed-node-and-the-row-is-the-keep.md`).
///   A node with exactly one version in the listing is refused naming what is
///   in the way, which is `P-0083`: it says the node has nothing behind what
///   it is playing rather than that the press failed.
///
/// **The narrowing to the Set is both arms'**, and it is the same narrowing
/// for the same reason — a version filed under no Set is a version of nothing.
pub(crate) fn put_back(
    store: &std::path::Path,
    slot: usize,
    letter: &str,
    id: &str,
    revision: &karakuri_operation::Revision,
    slots: &karakuri_mcp::Slots,
) -> String {
    let asked = asked_for(revision);
    let found = match karakuri_environment::history::list(store, HISTORY_MOST) {
        Ok(found) => found,
        Err(why) => {
            return format!(
                "  put back: the edit history could not be read: {why} — nothing moved, and \
                 what is on deck {letter} is still running"
            );
        }
    };
    // **`Some(id)` and never `None`**, which is [`walked`]'s filter said again
    // where it decides what is written rather than what is drawn: a version
    // filed under no Set is a version of nothing, and landing one because a
    // `None` read as a wildcard would put another run's edit on a deck.
    let of_the_set = || {
        found
            .versions
            .iter()
            .filter(|version| version.set.as_deref() == Some(id))
    };
    let version = match revision {
        karakuri_operation::Revision::Picked(picked) => {
            let Some(version) = of_the_set().find(|version| version_row(version) == *picked) else {
                return format!(
                    "  put back: `{picked}` is not one of `{id}`'s versions in the last \
                     {HISTORY_MOST} this store wrote — a day directory an operator deleted by \
                     hand is the ordinary way that happens, since `rm -rf history/YYYY/MM` is \
                     the whole of the retention policy; nothing moved, and deck {letter} is \
                     still playing `{id}`"
                );
            };
            version
        }
        karakuri_operation::Revision::Previous(node) => {
            let layer = setfile::kind_name(ir_layer(node.layer));
            let addr = node_addr(ir_layer(node.layer), node.index);
            // **Most recent first is `history::list`'s own order**, kept
            // rather than re-sorted here: the newest is what the slot is
            // running and the one after it is the step back.
            let mut chain = of_the_set()
                .filter(|version| version.layer == layer && version.index == node.index as usize);
            let running = chain.next();
            let Some(version) = running.and(chain.next()) else {
                return format!(
                    "  put back: deck {letter} {addr} has {} in `{id}`'s history, so there is \
                     nothing before what it is playing to step back to — the history keeps \
                     every version that compiled, and this node has only ever had the one. \
                     Nothing moved.",
                    match running {
                        Some(_) => "one version",
                        None => "no version",
                    }
                );
            };
            version
        }
    };
    let source = match std::fs::read(&version.file) {
        Ok(source) => source,
        Err(e) => {
            return format!(
                "  put back: {}: {e} — the row is a name and the file behind it is gone, so \
                 nothing moved and deck {letter} is still playing `{id}`",
                version.file.display()
            );
        }
    };
    let target = match slots.file(slot, version.layer, version.index) {
        Ok(path) => path.clone(),
        Err(why) => {
            return format!(
                "  put back: deck {letter} does not hold the node {asked} was a version \
                 of — {why}; `{id}` has been edited since, or this version came off another \
                 slot. Nothing moved."
            );
        }
    };
    match std::fs::write(&target, &source) {
        Ok(()) => format!(
            "  put back: deck {letter} {}:{} <- `{}` -> written into {}; the worker \
             builds it and the budget judges it, and the version it replaces is kept because \
             the rebuild compiles it",
            version.layer,
            version.index,
            version_row(version),
            target.display()
        ),
        Err(e) => format!(
            "  put back: {}: {e} — nothing moved, and what is on deck {letter} is still \
             running",
            target.display()
        ),
    }
}

/// **The record, applied to the deck**, and what to say about it.
///
/// **This is not the half ADR-0185 promised to delete, and it did not go with
/// it.** Turning an `Operation` into a `Record` was the shortcut — that
/// function is gone and [`written`] answers instead
/// ([ADR-0194](../../../docs/adr/0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md)).
/// Turning a record into a *deck movement* is a different job and is the
/// harness's by design: `karakuri-operation-record` has no engine and never
/// will, so somebody who owns a deck has to decode.
///
/// `karakuri-cli`'s `mix::change` is the real decoder and it does two things
/// this does not: it refuses a slot the deck has not got, with the same
/// sentence every other surface refuses one with, and it turns a record into a
/// `Change` that a caller applies. This is the shortest path from the records
/// [`written`] answers with to the setters they name.
///
/// The line it returns is the loop closing, printed so that it can be read
/// rather than inferred: the operation, the record, and **what the deck says
/// afterwards** — which is where the next frame's strip comes from.
///
/// # It takes the look as well as the deck, and that is not a second target
///
/// `Record::Look` is the one record here that does not name a slot: the look
/// is what *every* sink is drawn under, so it is `Engine::look` rather than
/// anything on the deck ([`Engine::look`], and `karakuri_engine::frame::Look`
/// for why the master out is deliberately not in it). Handing both in is what
/// keeps this one function the only place a record becomes a movement — a
/// second `apply_look` beside it would be the second route into the engine
/// that P-0090 exists to refuse.
pub(crate) fn apply(
    record: &Record,
    deck: &mut Deck,
    look: &mut Look,
    chain: &mut Vec<karakuri_engine::SlotSpec>,
) -> Option<String> {
    // **A slot the deck has not got is refused rather than indexed**, and the
    // guard is [`held`] rather than a closure here, because the key arms in
    // `window_event` need the same answer one step earlier: a press reads the
    // trim it is stepping from before it can name where it is going, so a
    // guard on the record alone would be a read that panicked on its way to a
    // refusal. The argument for refusing at all is at that function.
    match *record {
        Record::Gain { slot, value } => {
            let slot = held(deck, slot.0)?;
            deck.set_gain(slot, value);
            Some(format!(
                "  fader: deck {} trim -> SetGain {{ deck: {slot}, gain: {value:.3} }} \
                 -> Record::Gain -> deck.gain({slot}) = {:.3}",
                deck_letter(slot.0),
                deck.gain(slot)
            ))
        }
        Record::Opacity { slot, value } => {
            let slot = held(deck, slot.0)?;
            deck.set_opacity(slot, value);
            Some(format!(
                "  fader: deck {} fader -> SetOpacity {{ deck: {slot}, opacity: {value:.3} }} \
                 -> Record::Opacity -> deck.opacity({slot}) = {:.3}",
                deck_letter(slot.0),
                deck.opacity(slot)
            ))
        }
        // **The mode comes back off the wire name, and an unknown one is
        // refused rather than defaulted.** `Record::Blend` carries a `String`
        // because what a mode is allowed to be is the engine's to say, so this
        // is the engine saying it — `mix::change` refuses the same way, with
        // the sentence `karakuri-cli`'s `no_such_blend` writes. Nothing in
        // this file can produce a name the engine has not got, since the chip
        // only ever emits one of `BlendMode::ALL`, so this is the guard rather
        // than the message.
        Record::Blend { slot, ref mode } => {
            let slot = held(deck, slot.0)?;
            let blend = Blend::from_name(mode)?;
            deck.set_blend(slot, blend);
            Some(format!(
                "  blend: deck {} -> SetBlendMode {{ deck: {slot}, blend: {mode} }} \
                 -> Record::Blend -> deck.blend({slot}) = {}",
                deck_letter(slot.0),
                deck.blend(slot).name()
            ))
        }
        // **The one record here that is followed by a governor pass**, and it
        // is not a flourish: `Deck::set_residency` writes the request *and
        // grants it*, so a harness that stopped there would put a slot the
        // budget has no room for into `Priming` and draw a primed deck the
        // governor never admitted. That is
        // [ADR-0191](../../../docs/adr/0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md)
        // exactly — the panel's parked deck is the governor's verdict or it is
        // a drawing of one — and it is `karakuri-cli`'s own order, where
        // `mix::Change::Residency` sets the level and calls `govern` beside
        // it. The report is dropped here rather than printed: the line below
        // says what the deck ended up at, which is the half this window shows.
        Record::Residency { slot, ref level } => {
            let slot = held(deck, slot.0)?;
            let residency = mix::parse_residency(level)?;
            deck.set_residency(slot, residency);
            deck.govern();
            Some(format!(
                "  tally: deck {} -> SetResidency {{ deck: {slot}, residency: {level} }} \
                 -> Record::Residency -> deck.requested_residency({slot}) = {:?}, \
                 deck.residency({slot}) = {:?}",
                deck_letter(slot.0),
                deck.requested_residency(slot),
                deck.residency(slot)
            ))
        }
        // **The whole mask, because the record is a state and not an ask.**
        // `Record::Mask` carries a shape, an angle, a position and a softness,
        // and `Deck::set_mask` is what it decodes to — the engine says so at
        // that setter. Reaching for `Deck::set_mask_shape` instead, to keep a
        // running wipe alive, would be this file decoding a record by picking
        // two fields out of it and dropping the softness on the floor: a
        // second route to the deck, where P-0090 is that every control ends in
        // the same record. **So a shape press stops a wipe on that deck**, and
        // that is not a fault here — it is the honest limit
        // `docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md`
        // states about the record stream, met by the first surface to make the
        // press
        // ([ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)).
        //
        // **The shape comes back off the wire name**, refused rather than
        // defaulted, exactly as the blend's and the residency's do. Nothing in
        // this file can produce a name the engine has not got, since the chip
        // only ever emits one of `WipeKind`'s three and the record is written
        // from `WipeKind::name`.
        Record::Mask {
            slot,
            ref kind,
            angle,
            position,
            softness,
        } => {
            let slot = held(deck, slot.0)?;
            let shape = MaskKind::from_name(kind)?;
            deck.set_mask(slot, Mask::new(shape, angle, position, softness));
            Some(format!(
                "  mask: deck {} -> SetMaskShape {{ deck: {slot}, kind: {kind}, \
                 angle: {angle:.3} }} -> Record::Mask -> deck.mask({slot}) = {} \
                 at {:.3} rad, front at {:.3}",
                deck_letter(slot.0),
                deck.mask(slot).kind().name(),
                deck.mask(slot).angle(),
                deck.mask(slot).position()
            ))
        }
        // **The whole look, because the record is a state and not an ask.**
        // `Record::Look` carries the operator, the exposure and the white
        // point together for its own stated reason — a stream that set a level
        // without naming the operator would describe a look nobody can
        // reconstruct — and each of the two controls asks for one of the three
        // (ADR-0192). The other two arrive here already filled in from the
        // reading [`reading`] took, so this writes what it is given and picks
        // nothing out of it, exactly as the mask arm does.
        //
        // **The operator comes back off the wire name, refused rather than
        // defaulted**, as the blend's, the residency's and the shape's do.
        // Nothing in this file can produce a name the engine has not got: the
        // capsule only ever emits one of `Tonemap`'s four and the record is
        // written from `Tonemap::name`, which is the same lower-case spelling
        // `mix::op_wire_name` parses.
        //
        // **No `set_tonemap` call.** `compose` uploads the tone-map uniform
        // every frame from the `Committed` the closure hands back, so writing
        // the field *is* the write — and a `Present::set_tonemap` here would be
        // a second writer, with the last one each frame winning.
        Record::Look {
            ref op,
            exposure,
            white_point,
        } => {
            let op = mix::parse_op(op)?;
            *look = Look {
                op,
                exposure,
                white_point,
            };
            Some(format!(
                "  look: -> Record::Look {{ op: {op_name}, exposure: {exposure:.3}, \
                 white_point: {white_point:.3} }} -> every sink is drawn under {op_name} at \
                 exposure {exposure:.3}",
                op_name = mix::op_wire_name(op)
            ))
        }
        // **A scheduled move, and the first record this window applies that
        // does not land now.** `Record::Transition` carries the slot, which
        // control is moving, where it ends up, the beat it starts on, how long
        // it lasts and the shape it eases with — and the one thing it does not
        // carry is where the move starts *from*, because that is where the
        // control already is at the moment the record is applied. Reading it
        // here rather than off the record is what makes a replay fade from
        // where the run did, and it is `karakuri-cli`'s `schedule_from`, in
        // the one place this window needs it.
        //
        // **It landed with the `go` capsule, and until then the window drew a
        // wipe's other records and dropped this one on the floor**: the
        // arriving deck was masked to nothing and put on air, and the front
        // never travelled. A record with no arm here is silent — the `_` at
        // the foot of this match — which is why the gap was invisible.
        //
        // **The two names come back off the wire, refused rather than
        // defaulted**, as the blend's, the residency's and the sync mode's do:
        // a control this engine has not got and a curve it cannot ease with
        // are both a record from a stream this build does not understand, and
        // guessing at either would schedule a move nobody wrote.
        Record::Transition {
            slot,
            ref control,
            to,
            start,
            beats,
            ref curve,
        } => {
            let slot = held(deck, slot.0)?;
            let control = Control::from_name(control)?;
            let curve = karakuri_engine::binding::Curve::parse(curve)?;
            let from = match control {
                Control::Gain => deck.gain(slot),
                Control::Opacity => deck.opacity(slot),
                Control::MaskPosition => deck.mask(slot).position(),
            };
            deck.schedule(karakuri_engine::Transition::new(
                slot.index(),
                control,
                from,
                to,
                start,
                beats,
                curve,
            ));
            Some(format!(
                "  transition: deck {} {} {from:.3} -> {to:.3} -> Record::Transition {{ \
                 start: {start:.3}, beats: {beats:.3}, curve: {} }} -> \
                 deck.transitions_on({slot}) = {}",
                deck_letter(slot.0),
                control.name(),
                curve.name(),
                deck.transitions_on(slot).count()
            ))
        }
        // **One level on the whole fold, and the one arm here that names no
        // slot at all.** `Record::MasterOut` carries a number and nothing
        // else, so unlike the look and the mask there is no other half of it
        // to fill in from what is running — which is why the reading below
        // has no arm for this control (ADR-0224).
        //
        // **The engine clamps and this does not.** `Deck::set_out` floors at
        // zero and is deliberately open above 1.0, through the same
        // `clamp_gain` the per-slot gain goes through, because the mix is HDR
        // and this level is applied to values a tone mapper has not seen —
        // [P-0064](../../../docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md).
        // A clamp here would be a second opinion about a range the setter
        // already holds, which is the rule the whole conversion is written
        // under.
        //
        // **No `cancel` to worry about**, where the gain and the fader each
        // stop whatever was moving them: a `Control` is per slot and this is
        // not, so nothing in the engine can be moving it and there is nothing
        // for a hand to win against.
        // **The one record that reaches inside a Set**, and the row the
        // Inspector bay was blocked on. `Deck::write_param` is the public road
        // and it compiles nothing — the map is packed into the uniform by the
        // next `Set::prepare`, so the value is on screen on the next frame.
        //
        // **Decoded by `mix::change` rather than here**, which is the one arm
        // in this function that does not take the short path, and the reason is
        // the expansion: a `vec3` value is three writes under the component
        // keys ADR-0268 made, and spelling that a second time in this file is
        // the drift `karakuri-operation-record` exists to end. It is also what
        // refuses a slot this deck has not got, in the sentence every other
        // surface refuses one with — so `held` is not asked first here.
        Record::Ride { .. } => {
            let writes = match karakuri_environment::mix::change(record, deck.slot_count()) {
                Ok(Some(karakuri_environment::mix::Change::Ride { slot, writes })) => {
                    let mut reached = 0;
                    for write in &writes {
                        match deck.write_param(EngineSlot(slot as u8), write) {
                            Ok(n) => reached += n,
                            Err(refused) => return Some(format!("  {refused}")),
                        }
                    }
                    (slot, writes, reached)
                }
                Ok(_) => return None,
                Err(refused) => return Some(format!("  {refused}")),
            };
            let (slot, writes, reached) = writes;
            match reached {
                0 => Some(format!(
                    "  {}",
                    karakuri_environment::no_such_param(slot, &writes[0].key)
                )),
                _ => Some(format!(
                    "  knob: deck {} -> WriteParam -> Record::Ride -> {} on {reached} node(s)",
                    deck_letter(slot as u8),
                    writes
                        .iter()
                        .map(|w| format!("{} = {:.3}", w.key, w.value))
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
            }
        }
        // **What is driving a knob, and the taking of it back** — the other
        // record that reaches inside a Set, and the one the Inspector's
        // sensitivity row was blocked on. `Deck::bind` and `Deck::unbind` are
        // the public roads and neither compiles anything: a binding is
        // resolved by the next `Set::prepare`, so an attachment is riding on
        // the next frame.
        //
        // **Decoded by `mix::change` rather than here**, which is the ride's
        // reason one arm up: the three diagnostics a binding owes — a `bpm`
        // source, an `octaves` without `fbm`, a generator on a binding that is
        // not to `noise` — belong to `setfile::binding_from_record` and are
        // said in its words on every route, so spelling them a second time
        // here is the drift `karakuri-operation-record` exists to end.
        Record::Source { .. } => {
            let (slot, key, bound) = match karakuri_environment::mix::change(
                record,
                deck.slot_count(),
            ) {
                Ok(Some(karakuri_environment::mix::Change::Source {
                    slot,
                    layer,
                    index,
                    key,
                    binding,
                })) => match binding {
                    Some(binding) => {
                        let signal = binding.signal.clone();
                        let curve = binding.curve.name();
                        let range = binding.range;
                        match deck.bind(EngineSlot(slot as u8), binding) {
                                karakuri_engine::set::Bound::Yes => (
                                    slot,
                                    key,
                                    format!(
                                        "{signal} through {curve} onto [{:.2}, {:.2}]",
                                        range[0], range[1]
                                    ),
                                ),
                                karakuri_engine::set::Bound::NoSuchParam => {
                                    return Some(format!(
                                        "  {}",
                                        karakuri_environment::no_such_param(slot, &key)
                                    ))
                                }
                                karakuri_engine::set::Bound::NoSuchControl => {
                                    return Some(format!(
                                        "  slot {slot}: `{signal}` is not a control this Set                                          publishes"
                                    ))
                                }
                            }
                    }
                    None => match deck.unbind(EngineSlot(slot as u8), layer, index, &key) {
                        true => (slot, key, "nothing — taken back".to_string()),
                        false => {
                            return Some(format!("  slot {slot}: nothing was driving `{key}`"))
                        }
                    },
                },
                Ok(_) => return None,
                Err(refused) => return Some(format!("  {refused}")),
            };
            Some(format!(
                "  source: deck {} -> Record::Source -> `{key}` is driven by {bound}",
                deck_letter(slot as u8)
            ))
        }
        // **Who may move one node**, and the writer ADR-0211 said the engine
        // owed. It is not enforcement: nothing writes a parameter on an
        // agent's behalf here, so what a level reaches today is
        // `Set::write_param`'s wildcard refusal — a bare-name control over
        // nodes that no longer agree is refused whole from the next press.
        Record::Authority { .. } => {
            let (slot, level) = match karakuri_environment::mix::change(record, deck.slot_count()) {
                Ok(Some(karakuri_environment::mix::Change::Authority {
                    slot,
                    layer,
                    index,
                    authority,
                })) => match deck.set_authority(EngineSlot(slot as u8), layer, index, authority) {
                    true => (slot, authority),
                    false => {
                        return Some(format!(
                            "  slot {slot}: no node {}:{index} to speak for",
                            karakuri_environment::setfile::layer_name(
                                karakuri_environment::setfile::layer_of(layer)
                            )
                        ))
                    }
                },
                Ok(_) => return None,
                Err(refused) => return Some(format!("  {refused}")),
            };
            Some(format!(
                "  authority: deck {} -> SetAuthority -> Record::Authority -> {}",
                deck_letter(slot as u8),
                level.name()
            ))
        }
        Record::MasterOut { value } => {
            deck.set_out(value);
            Some(format!(
                "  master: out -> SetMasterOut {{ out: {value:.3} }} -> Record::MasterOut -> \
                 deck.out() = {:.3}, at the entry to the master chain",
                deck.out()
            ))
        }
        // **The chain itself, one record for all three passes.** Written whole
        // for `Record::Look`'s reason and applied whole: the value lands on
        // [`Engine::chain`] and the frame loop hands it to
        // `Present::set_chain`, so nothing here touches a uniform. That is the
        // look arm's arrangement one pass upstream, and it is why there is no
        // `set_chain` call in this function.
        //
        // **The cut comes back off the wire word, refused rather than
        // defaulted**, as the tone map operator and the blend mode do: a cut
        // this engine has not got is a stream saying something this build
        // cannot draw, and a default would silently play the other picture.
        //
        // **No clamp here either.** `Chain::clamped` is the wall and it is
        // inside the setter, so a stream carrying 4.0 meets the same ceiling
        // a fader does.
        Record::MasterChain(ref want) => {
            let mut slots = Vec::with_capacity(want.slots.len());
            for slot in &want.slots {
                slots.push(karakuri_engine::SlotSpec {
                    procedure: slot.procedure.clone(),
                    cut: match &slot.cut {
                        None => None,
                        Some(word) => Some(Cut::parse(word)?),
                    },
                    params: slot.params.clone(),
                });
            }
            let said = format!(
                "  master: chain -> Record::MasterChain -> {} slot{} — {}",
                slots.len(),
                if slots.len() == 1 { "" } else { "s" },
                if slots.is_empty() {
                    "the frame is the mix".to_string()
                } else {
                    slots
                        .iter()
                        .map(|s| {
                            let cut = s
                                .cut
                                .map(|c| format!(" ({})", c.name()))
                                .unwrap_or_default();
                            format!("{}{cut}", &s.procedure[..s.procedure.len().min(14)])
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            );
            *chain = slots;
            Some(said)
        }
        // **The whole of one slot's clock, because the record is a state and
        // not an ask.** `Record::Transport` carries the sync mode, the anchor
        // and the scrub together for a stated reason — a scrub position
        // without the mode and the anchor beside it *"would replay a slot onto
        // a grid it was never on"* — and `Deck::set_transport` is what it
        // decodes to. The mode and the anchor arrive here unchanged, from the
        // reading [`reading`] took a moment earlier; only the scrub has moved.
        //
        // **The mode comes back off the wire name, refused rather than
        // defaulted**, as the blend's, the residency's, the shape's and the
        // operator's do. Nothing in this file can produce a name the engine
        // has not got: the scrub only ever writes back the mode it just read
        // off the same deck, and the record is written from `Sync::name`.
        //
        // **`set_transport` refuses, and the refusal is dropped here on
        // purpose.** It refuses a mode this slot's material cannot honour, and
        // a scrub cannot present one — it names the mode the slot is already
        // in, which the slot is in because the engine allowed it. What it
        // guards against is the case the engine names at `sync_allowed`: a
        // swap that puts accumulating material into a slot that is beat-synced
        // makes the mode it is *already in* unavailable, *"and whatever wires
        // swapping to this owes it"*. **Both slots are watched now, so that
        // case is reachable**: save an accumulating procedure into a
        // beat-synced slot and the next scrub is refused. What should be
        // printed then is the deck's own refusal rather than this arm's line,
        // and it is what this owes.
        Record::Transport {
            slot,
            ref sync,
            anchor_bpm,
            scrub_beats,
        } => {
            let slot = held(deck, slot.0)?;
            let mode = EngineSync::from_name(sync)?;
            deck.set_transport(slot, mode, anchor_bpm, scrub_beats)
                .ok()?;
            Some(format!(
                "  scrub: deck {} -> ScrubDeck {{ deck: {slot} }} -> Record::Transport {{ \
                 sync: {sync}, anchor_bpm: {anchor_bpm:.1}, scrub_beats: {scrub_beats:+.2} }} \
                 -> deck.transport({slot}).scrub_beats() = {:+.2} beats",
                deck_letter(slot.0),
                deck.transport(slot).scrub_beats()
            ))
        }
        // **A choice and not a position, so nothing interpolates and nothing
        // is left running.** `Record::Select` carries the slot, the renderer
        // and the instant alone — *"half way to renderer 2" does not name a
        // picture* — and `Deck::schedule_selection` is what it decodes to: a
        // later selection on a slot replaces the earlier one, which is
        // `Deck::schedule`'s own rule for a fade.
        //
        // **The renderer is not checked here and is checked at the beat.** The
        // Set in a slot can change under a hot swap between the schedule and
        // the instant, so a selection that no longer names a renderer is
        // dropped where it is applied. The *slot* is checked, by `held`,
        // because a deck does not change size.
        //
        // **It says nothing about a slot that overdraws**, and that is carried
        // rather than refused: `Record::Select` names an edge into the Set's
        // L5 and a slot built without a composite fold has none, so refusing
        // it would make a replay fail on a line describing a performance that
        // happened.
        Record::Select {
            slot,
            renderer,
            start,
        } => {
            let slot = held(deck, slot.0)?;
            deck.schedule_selection(Selection::new(slot.index(), renderer as usize, start));
            Some(format!(
                "  renderer: deck {} -> SelectRenderer {{ renderer: {renderer} }} -> \
                 Record::Select {{ start: {start:.3} }} -> deck.selections_on({slot}) = {} \
                 armed, landing on the beat grid",
                deck_letter(slot.0),
                deck.selections_on(slot).count()
            ))
        }
        // **The session's grid, and the one record here that names no slot and
        // touches no deck control at all.** `Record::Tempo` is a correction —
        // a tempo, a phase shift and how much the estimate behind it was
        // believed — and `karakuri_environment::audio::apply_tempo` is *"the
        // only way a correction reaches the oscillator, live or on replay"*.
        // So this arm is the live half of that sentence, and it is one call
        // rather than a `signals.correct` beside it for exactly the reason
        // that function says so of itself.
        //
        // **The signals are copied out of the deck and back in**, which is
        // what `Deck::signals` and `set_signals` are for and is
        // [`measure_audio`]'s own line: the bus is a `Copy` value and the deck
        // is the model of record for it.
        //
        // **What reaches here today is `Operation::SetFreeRunTempo` and
        // nothing else**, which is *what the grid runs at with nothing driving
        // it* — the one control on this panel that works in the state this
        // program actually runs in, where there is no device and `tapped` and
        // `scaled` both refuse because there is no room. A tap and an octave
        // end in this same record and do **not** come through here: theirs is
        // the beat lock's answer and `written` cannot build it (ADR-0278), so
        // they are applied where they are computed.
        //
        // **The confidence is not printed and the shift is.** A hand-named
        // tempo carries `shift: 0.0` and `confidence: 0.0` — `Record::Tempo`'s
        // own words for a free-running tempo being stated — and a `0.0`
        // confidence beside a tempo an operator just chose would read as *this
        // is not believed*, which is the opposite of what it means. The shift
        // is printed because a beat that did not move is the claim this arm
        // makes.
        Record::Tempo { bpm, shift, .. } => {
            let mut signals = *deck.signals();
            karakuri_environment::audio::apply_tempo(&mut signals, record);
            deck.set_signals(signals);
            Some(format!(
                "  tempo: -> Record::Tempo {{ bpm: {bpm:.1}, shift: {shift:+.3} }} -> \
                 deck.signals().oscillator().bpm() = {:.1}, from now on and without moving a \
                 beat that has already happened",
                deck.signals().oscillator().bpm()
            ))
        }
        _ => None,
    }
}

/// **The reading an operation's record needs, taken off the deck it names.**
///
/// [`written`] builds `Record::Mask` **whole** — a shape, an angle, a position
/// and a softness — out of an operation that names two of the four, and the
/// other two come from a reading of the mask that is running (ADR-0201). This
/// is that reading, and it is the harness's because the deck is
/// (ADR-0156, ADR-0194).
///
/// **`Current::default()` is *I read nothing*, and it is still the answer for
/// four of this panel's emitting controls**: a gain, an opacity, a blend
/// mode and a residency each carry everything their record carries, so handing
/// a reading in would be this file inventing a value. The mask mini is one of
/// those that need one, and it needs it for the deck the operation *names*
/// rather than for the deck the pointer is over — which is `Reading::Mask`'s
/// own wording and the reason this takes the operation and not a slot.
///
/// **The two look controls are two of the other three**, and they read one
/// thing between them: the look that is running. Each names a third of
/// `Record::Look` and the other two thirds come from here — which is
/// [`Reading::Look`]'s own wording and the reason the reading is taken for the
/// operation rather than per control.
///
/// **The scrub's two arrows are the fourth**, and the reading they take is the
/// one thing on this list that is not a completion: see the arm.
///
/// **The sync chip and the anchor are the fifth and sixth**, and they read the
/// one thing here that belongs to no deck: the session tempo. That arm used to
/// be absent and the two controls used to print a question instead of moving
/// anything — see [`unwritten`] for what the question turned out to be.
///
/// **The softness is read back**, where `karakuri-cli`'s `mix::current_mask`
/// substitutes its own `MASK_SOFTNESS`: that program writes wipes and has a
/// softness of its own to write, and this window has never written one. What
/// is read back here is therefore what is actually on the slot, and reading it
/// back is what stops a press rewriting it — the same argument the angle's is,
/// one field along.
///
/// **The `go` capsule is the last of them and it is the one that reads
/// three**: a wipe is written against the transition settings, the mask of the
/// deck arriving and where that deck already sits in the mix. The first is the
/// console's own — `settings` is what [`View::transition`] holds and what the
/// row's three pills move — and the other two are the deck's, taken for the
/// `to` slot and never for the `from`, which is `Current::mask`'s own wording:
/// everything a wipe writes is about the deck arriving.
///
/// A slot the deck has not got answers `None`, and [`written`] then says the
/// reading was owed rather than indexing something that is not there — the
/// guard [`apply`] has, at the other end of the same press.
pub(crate) fn reading(
    operation: &Operation,
    deck: &Deck,
    look: &Look,
    chain: &[karakuri_engine::SlotSpec],
    settings: TransitionSettings,
) -> Current {
    // **The whole chain, for whichever pass was asked for.** The look arm's
    // argument one bay along: `Record::MasterChain` needs all four numbers and
    // each row's press carries one pass, so the running chain is handed in and
    // `written` takes the rows the press did not name.
    let master_chain = match *operation {
        Operation::SetFeedback { .. }
        | Operation::SetBloom { .. }
        | Operation::SetRgbShift { .. } => Some(mix::current_chain(chain)),
        _ => None,
    };
    let look = match *operation {
        // **Two thirds of the record, for whichever third was asked for.**
        // `SetTonemap` carries an operator and `SetExposure` a level, and
        // `Record::Look` needs all three — so the running look is handed in
        // and `written` takes the two the press did not name. That is
        // ADR-0192's argument executable in this file: the operation carries
        // what a surface can say and the translator completes the record.
        // `white_point` is on no surface at all, so it survives every press by
        // arriving here and going straight back out.
        Operation::SetTonemap { .. } | Operation::SetExposure { .. } => {
            Some(karakuri_operation_record::Look {
                tonemap: mix::tonemap(look.op),
                exposure: look.exposure,
                white_point: look.white_point,
            })
        }
        _ => None,
    };
    // **The scrub is the third reading, and it is the only one that is
    // relative.** `Record::Transport` is absolute — a sync mode, an anchor and
    // a scrub position — and `ScrubDeck` names an amount, so the record is
    // where the slot already is plus what was asked for. The reading is
    // therefore not a completion of a record the way the look's and the mask's
    // are: it is the left-hand side of an addition, and without it the
    // conversion answers `Owed(NotRead(Transport))` rather than starting a
    // deck's scrub from zero.
    //
    // **All three fields, because the record is written whole.** A scrub that
    // wrote a position without the mode and the anchor beside it *"would
    // replay a slot onto a grid it was never on"* —
    // `karakuri_operation_record::Transport` says so at its own definition —
    // and the two it does not touch survive the press by arriving here and
    // going straight back out, which is the white point's arrangement one
    // reading up.
    let transport = match *operation {
        Operation::ScrubDeck { deck: slot, .. } => {
            EngineSlot::new(slot, deck.slot_count()).map(|slot| {
                let transport = deck.transport(slot);
                karakuri_operation_record::Transport {
                    sync: mix::sync(transport.sync()),
                    anchor_bpm: transport.anchor_bpm(),
                    scrub_beats: transport.scrub_beats(),
                }
            })
        }
        _ => None,
    };
    // **The tempo the room is going at, and nothing about a slot.** Engaging
    // a sync mode anchors the slot at the session tempo so that the picture
    // does not move at the instant it goes on the grid, which is
    // `karakuri_engine::transport::Transport::engaged`'s policy and the whole
    // of what this reading is for. `mix::current_tempo` takes the oscillator
    // rather than an `f32`, so this window cannot hand in a tempo the session
    // never ran at — and it reads the grid rather than a clock, which is what
    // lets the record be replayed
    // ([P-0092](../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)).
    //
    // **No slot check, unlike the three below.** The tempo is the session's,
    // so a `SetSync` naming a slot the deck has not got is a record with a slot
    // out of range rather than a reading that could not be taken, and
    // `apply`'s own guard is what says so at the other end of the press.
    let tempo = match *operation {
        Operation::SetSync { .. } => Some(mix::current_tempo(deck.signals().oscillator())),
        _ => None,
    };
    // **Three operations read this and one of them names two decks.** A wipe
    // writes `Record::Mask` for the deck *arriving* — twice, at the front's
    // present position and then at 0 — so the mask handed over is `to`'s and
    // `from` is read for nothing at all. Handing in the covered deck's would
    // put somebody else's soft edge on the front that is about to cross the
    // frame, which is `Current::mask`'s own sentence and `karakuri-cli`'s
    // `Live::operate` arm exactly.
    //
    // **`SetMaskPosition` was missing from this list until 2026-09-10**, and
    // it is the one arm ADR-0334 recorded as a defect rather than a scope:
    // both halves of a mask write `Record::Mask` whole, so a position needs
    // the shape, the angle and the soft edge it does not name, and without
    // this `written` answered `Owed(NotRead(Mask))` and nothing moved. It was
    // reachable from a mapped controller before it was reachable from a model,
    // so the hole was a MIDI knob that did nothing as well as a call that
    // could not be accepted. `karakuri-cli`'s arm has always named all three,
    // which is what makes this a slip in one file rather than a decision.
    let mask = match *operation {
        Operation::SetMaskShape { deck: slot, .. }
        | Operation::SetMaskPosition { deck: slot, .. }
        | Operation::Wipe { to: slot, .. } => {
            EngineSlot::new(slot, deck.slot_count()).map(|slot| {
                let mask = deck.mask(slot);
                karakuri_operation_record::Mask {
                    kind: wipe_kind(mask.kind()),
                    angle: mask.angle(),
                    position: mask.position(),
                    softness: mask.softness(),
                }
            })
        }
        _ => None,
    };
    // **Every field named, and no `..Current::default()` behind them.** Every
    // reading the type carries is answered here, so a fill would be dead — and
    // the day it grows one more, this stops compiling and somebody has to say
    // whether this window can take it, rather than a `None` arriving silently.
    //
    // **The count that used to be in this sentence is gone rather than
    // corrected.** It said four where there are three and named a fifth that
    // would be a fourth, which is a figure nothing checks going stale in the
    // one comment whose whole argument is that the compiler does the checking.
    //
    // **The transition settings used to be answered `None` here, on the
    // grounds that this panel drew no control that set any of them.** That
    // sentence was true of a window with no transition row in it and is not
    // true of this one: the row is drawn, its three pills emit
    // `Operation::SetTransition`, and `View::transition` is the model of
    // record for what they arrive at. So the reading is taken, and it is the
    // one here that is read off the *console* rather than off the deck —
    // a quantum, a length and a wipe shape are a surface's setting deciding
    // what the next move means, and this surface now holds one.
    //
    // **Four operations, which is every one that schedules a move**, and it
    // is `karakuri-cli`'s `Live::operate` arm exactly: only the wipe has a
    // control on this panel today, and the other three are answered because
    // what the reading *is* does not depend on which surface asked. A
    // conversion that came back `Owed(NotRead(Transition))` for a fade the
    // day a fader learned to schedule one would be this arm having to be
    // found again.
    //
    // `mix::current_transition` takes the oscillator and the quantum rather
    // than an instant, so the start is
    // `karakuri_engine::transition::quantise`'s answer and this file cannot
    // hand in a beat the grid was never on — the arrangement `mix::current_tempo`
    // is in one reading up
    // ([P-0092](../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)).
    // `mix::FADE_CURVE` is the shape every scheduled fade takes, and it is
    // asked for by name rather than spelled here because two surfaces easing
    // one fade differently is a value a replay carries.
    let transition = match *operation {
        Operation::FadeDeck { .. }
        | Operation::Crossfade { .. }
        | Operation::SelectRenderer { .. }
        | Operation::Wipe { .. } => Some(mix::current_transition(
            deck.signals().oscillator(),
            settings.quantum,
            settings.length,
            mix::FADE_CURVE,
            mask_kind(settings.kind),
            settings.angle,
        )),
        _ => None,
    };
    // **Where the deck a wipe is arriving on already sits in the mix**, and
    // the one reading here taken so that a record can be left *out* rather
    // than written. A wipe puts that deck under `over` and on air, and both
    // are a state it may be in already: under `add` the same gesture is a wipe
    // *on* rather than a wipe *over*, a different picture and a legitimate
    // one, so the mode is left where the operator put it. The condition is the
    // conversion's and what it needs to hold it is this
    // ([P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
    //
    // **What the deck reports rather than what it was asked for**, which is
    // `Deck::residency`'s answer: the governor may hold a slot below the
    // request, and what a wipe needs to know is whether the put-on-air it is
    // about to write would change anything.
    let mix = match *operation {
        Operation::Wipe { to: slot, .. } => EngineSlot::new(slot, deck.slot_count())
            .map(|slot| mix::current_mix(deck.blend(slot), deck.residency(slot))),
        _ => None,
    };
    Current {
        look,
        master_chain,
        mask,
        transport,
        tempo,
        transition,
        mix,
    }
}

/// **The master chain, as the console reads it** — a level's reading rather
/// than the level (ADR-0156), and the one place the engine's `Chain` becomes
/// the panel's.
///
/// **Not `mix::current_chain`, and the two are not the same reading.** That one
/// answers the *conversion* — what `written` completes a record from, in the
/// engine's own amounts — and this one answers a *fader*, in track positions.
/// The crossing they share, engine cut to vocabulary cut, is `mix::cut` and is
/// made once.
///
/// **The feedback amount arrives as a track position**, `[0, 1]`, where the
/// engine holds `[0, 0.95]`: a fader draws where it is along its own travel,
/// and `Knob::Feedback` multiplies back by `Feedback::MAX` on the way out. The
/// other two are `[0, 1]` at both ends and pass through.
pub(crate) fn chain_view(chain: &[karakuri_engine::SlotSpec]) -> view::Chain {
    // **The three rows read the three shipped slots**, which is
    // `mix::current_chain`'s own arrangement and is here so the reading is made
    // once: a row whose procedure is not in the chain reads zero, which is the
    // honest reading of *this pass is not running*. The rows retire in M5.16's
    // second pass (ADR-0340 §7) and this function goes with them.
    let running = mix::current_chain(chain);
    view::Chain {
        feedback: running.feedback.amount / karakuri_operation::Feedback::MAX,
        cut: running.feedback.cut,
        bloom: running.bloom,
        rgb_shift: running.rgb_shift,
    }
}

/// **The engine's mask shape, as the vocabulary's** — [`blend_mode`]'s
/// function one control along, and the one place these two lists are made to
/// agree.
///
/// A match, so the day a fourth `MaskKind` lands in the engine this stops
/// compiling rather than reading a shape the vocabulary cannot name into a
/// record that has to name one. The mirror image of it is `view::Mixer::mask`,
/// which turns the console's own word into the same vocabulary — three names
/// for three shapes, which is the cost `karakuri-operation` pays for depending
/// on nothing (P-0090).
pub(crate) fn wipe_kind(kind: MaskKind) -> karakuri_operation::WipeKind {
    match kind {
        MaskKind::None => karakuri_operation::WipeKind::None,
        MaskKind::Linear => karakuri_operation::WipeKind::Linear,
        MaskKind::Radial => karakuri_operation::WipeKind::Radial,
    }
}

/// **The vocabulary's mask shape, as the engine's** — [`wipe_kind`] read the
/// other way, and the two are a pair rather than one function because the
/// crossing happens in both directions in this file.
///
/// The console holds the shape the *next* wipe takes as a
/// [`karakuri_operation::WipeKind`] — `TransitionSetting::WipeShape` is what
/// its pill emits — and `mix::current_transition` takes the engine's, so a
/// wipe's front crosses here on its way to the reading. It crosses back
/// inside that function, through `mix`'s own `wipe_kind`, which is the one
/// place `karakuri-environment` makes the two lists agree: what a record
/// carries is the vocabulary's word either way, and this round trip is the
/// price of a signature that speaks the engine's types to a caller holding
/// them (`karakuri-cli` is that caller).
///
/// A match for [`wipe_kind`]'s reason, so a fourth shape on either side stops
/// the build here rather than at a record naming a shape nothing can read.
pub(crate) fn mask_kind(kind: karakuri_operation::WipeKind) -> MaskKind {
    match kind {
        karakuri_operation::WipeKind::None => MaskKind::None,
        karakuri_operation::WipeKind::Linear => MaskKind::Linear,
        karakuri_operation::WipeKind::Radial => MaskKind::Radial,
    }
}

/// **What a frame has to fit in on this window**: the display's refresh
/// interval, in milliseconds — the `/16.6` in the mock's transport, at the
/// 60 Hz it was drawn against.
///
/// **It is the refresh interval because that is what this window is held to.**
/// The surface is `PresentMode::Fifo`, so a frame that takes longer than one
/// interval to build is a frame that misses a vsync, and every millisecond
/// under it is the headroom the mock's own tooltip is about. `Cost::wait` is
/// the other side of the same number: at 60 Hz most of the frame is spent
/// blocked in `get_current_texture` waiting for it.
///
/// **It is not `karakuri_engine`'s `DEFAULT_BUDGET_MS`**, which is 20 and is a
/// different budget with the same word on it: that one is what a *candidate
/// Set* has to hold to survive a hot swap, measured offscreen at a fixed size
/// and judged on a median. The mock's tooltip runs the two together — *"12.4
/// of 16.6 — there is headroom. A candidate that cannot hold this is rolled
/// back on its own"* — and they are two numbers. This row draws the one the
/// frame is actually against.
///
/// `None` where `winit` will not say, which is a monitor it cannot name or a
/// mode with no refresh rate on it. The row then draws the frame time and no
/// budget, rather than a plausible 16.6 nothing measured.
///
/// **Read once, when the window opens.** A window dragged onto a 120 Hz
/// display keeps the interval it opened on, which is a real limitation and is
/// the price of not asking the platform for a monitor handle sixty times a
/// second.
pub(crate) fn budget_ms(window: &Window) -> Option<f32> {
    let millihertz = window.current_monitor()?.refresh_rate_millihertz()?;
    match millihertz > 0 {
        true => Some(1.0e6 / millihertz as f32),
        false => None,
    }
}
