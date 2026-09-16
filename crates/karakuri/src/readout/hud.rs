//! HUD legend formatting, status string generation, and console logging.

use super::*;
use crate::presets_listing;

impl Readout {
    /// A drag, in words: what was asked, where it landed, what held it, and what
    /// the pair either side is now.
    pub(crate) fn say_drag(&self, d: Dragged) -> String {
        let Dragged::Boundary {
            split,
            index,
            axis,
            asked,
            landed,
            held,
        } = d
        else {
            // A fader's words are the window loop's, because they are about
            // what happened to the *deck* after the operation left here.
            unreachable!("a fader drag says its own line")
        };
        let sizes = match self.panel.pair(split, index) {
            Some((a, b)) => format!(
                "{} {:.0} | {} {:.0}",
                self.label(a),
                axis.extent(self.panel.layout().rect(a)),
                self.label(b),
                axis.extent(self.panel.layout().rect(b))
            ),
            None => "no pair".to_owned(),
        };
        let stop = match held {
            Some(by) => format!(" — held {by:+.1} by a stop, and it stays there until it moves"),
            None => String::new(),
        };
        format!("  drag: asked {asked:.1}, landed {landed:.1}{stop} [{sizes}]")
    }

    /// What an operation did, in words. The model returns the facts; which English
    /// they take is the operation that was asked for, which is why this has both.
    pub(crate) fn say_op(&self, op: Op, outcome: &Outcome) {
        match outcome {
            Outcome::Folded { id, folded, root } => {
                let what = match op {
                    Op::FoldEnclosing(_) => "the split ",
                    _ => "",
                };
                println!(
                    "fold: {what}{} is now {}{}",
                    self.label(*id),
                    folding(*folded),
                    match *root && *folded {
                        true => " — that was the root, so the panel is empty; z brings it back",
                        false => "",
                    }
                );
            }
            Outcome::Unfolded(ids) => match ids.is_empty() {
                true => println!("unfold: nothing is folded"),
                false => {
                    let names: Vec<String> = ids.iter().map(|id| self.label(*id)).collect();
                    println!("unfold: {}", names.join(", "));
                }
            },
            Outcome::Soloed(id) => println!(
                "solo: {} — everything else folded (soloed = {})",
                self.label(*id),
                self.panel.layout().is_soloed()
            ),
            Outcome::Unsoloed { was } => println!(
                "unsolo: {}",
                match was {
                    true => "the arrangement before the solo is back",
                    false => "nothing was soloed",
                }
            ),
            Outcome::Reset => println!("reset: a fresh arrangement, at the same viewport"),
            // **Nothing that goes through here can produce this.** A restore
            // is not an `Op` — it carries a whole arrangement, which only
            // whoever read the store can hand over — so `arrangement` says its
            // own sentence, where the name is, and this arm exists because the
            // match is exhaustive rather than because a line is owed. See
            // `Panel::restore`.
            Outcome::Restored => {}
            // **And nothing that goes through here can produce this one
            // either, since 2026-08-31.** `Op::Report` was `p`, `p` is the
            // latency offset the operations page specifies, and a panel
            // diagnostic with no useful shortcut to point at loses the letter
            // rather than keeping one of a specified pair. Nothing else in
            // this program names the operation, so no key, no control and no
            // pointer route can reach it and there is no sentence to say.
            //
            // **The words went with the route rather than being kept for
            // one.** A formatter for an outcome nothing produces is this file
            // claiming a route it has not got
            // ([`docs/contributing.md` §4](../../../docs/contributing.md)),
            // and the table it printed is not the one the startup legend
            // prints: that one is each region's *min and max*, once, before
            // anything has been dragged, and this was each region's solved
            // rectangle and whether it is folded, at any moment. The
            // operation still answers that, to `karakuri-console`'s own
            // tests; what is gone is this program asking.
            Outcome::Report(_) => {}
            // One operation can still find nothing to act on, and it is not
            // about the pointer: the root has no split enclosing it. *Nothing
            // under the pointer* is said by `key`, before an operation is
            // named at all — see `Panel::under`.
            Outcome::Nothing => {
                println!("fold: that is the root, and nothing encloses it")
            }
        }
    }

    // -- the legend -----------------------------------------------------

    /// What this program is, said once at startup — and every line of it derived.
    ///
    /// `presets` and `store` are the two directories this run resolved, and they
    /// are passed in rather than read here for the reason every other number in
    /// this function is asked of the thing it is about: a legend that described the
    /// search instead of printing its answer is exactly the defect this function
    /// was repaired of, one paragraph along. See the paragraph below for what that
    /// repair cost to find.
    ///
    /// `mcp_port` is the port `karakuri_mcp::serve` actually bound —
    /// `Reporter::port` and not `--mcp`'s argument, so `--mcp 0` prints the
    /// ephemeral port it got — and `None` is a run that was not asked to serve. It
    /// is passed in for the same reason `presets` and `store` are: the sentence
    /// about the `mcp` pills was written unconditionally, said *nothing in this
    /// process serves MCP yet*, and went on saying it to every run that had just
    /// printed the address its server was listening on.
    pub(crate) fn print_legend(
        &mut self,
        budget_ms: Option<f32>,
        governed: &Report,
        presets: Option<&karakuri_environment::places::Presets>,
        store: &std::path::Path,
        mcp_port: Option<u16>,
    ) {
        self.panel.solve();
        let layout = self.panel.layout();
        let viewport = layout.viewport();
        println!();
        println!(
            "the console, in a {:.0} x {:.0} viewport. every leaf gets its region, and what \
             each one draws is the list further down rather than a sentence here: that list \
             is the arrangement's own nodes read against the view the frame is drawn from, \
             so a body that fills in says so without anybody rewriting a line of this. **the \
             sentence this replaces said every body was empty but the Program bay's two**, \
             and it went on saying it while bay after bay drew one — which is what a \
             description kept beside the thing it describes is worth.",
            viewport.w, viewport.h
        );
        // **Where this run's data is, and both lines are what the resolution
        // returned.** Not a sentence about how a presets root is looked for:
        // the directory is printed, and the phrase beside it is
        // `places::Found`'s own — so a candidate added, reordered or removed
        // changes this line without anybody editing it. A legend that said
        // *"the ones that ship with the program"* would be right until the day
        // it was not, which is the whole of what the paragraph above is about.
        match presets {
            Some(presets) => {
                // **Counted once and read off the listing the bay is drawn
                // from**, rather than described: a sentence about what a
                // preset directory probably holds is the kind of line this
                // legend was found lying five ways with.
                let held = presets_listing(Some(presets)).len();
                println!(
                    "presets: {} — {}. that directory is the app-preset tier: what ships \
                 with the program, written by nobody, and what a run with no paths on \
                 the command line opens on. the Library bay's `presets` scope lists the \
                 {} `.kset` file{} in it — the parts beside them are what those files \
                 name rather than rows of their own — and `enter` on one takes it into the \
                 store and then loads it, which is why opening a preset leaves a row \
                 under `all` — and under `my sets` only if you star it.",
                    presets.dir.display(),
                    presets.found.how(),
                    held,
                    match held {
                        1 => "",
                        _ => "s",
                    }
                )
            }
            // Said once, out loud, and it is this program's only occasion to
            // say it: a run that needed the library for a default pair was
            // refused before a window opened, so reaching here means the pair
            // was given by hand and nothing is broken — the preset tier is
            // simply empty.
            None => println!("{}", karakuri_environment::places::no_preset_library()),
        }
        println!(
            "store: {} — where the Library bay below reads Sets from, where this panel's \
             arrangements are filed, where each deck's working copy was written before this \
             window opened, and what `--store` moves. `karakuri-cli --store` names the same \
             directory and the default is the same constant, which it now is rather than \
             looks like: both ask `karakuri_environment::places::STORE`.",
            store.display()
        );
        // **The cells, and this sentence has been wrong four times.** It said
        // C and D had nothing behind them and named the number — *a deck of TWO
        // slots* — which was true of the deck that shipped before this one.
        // Then it said the one sink was an audition, and it was not: nothing
        // called `Deck::set_preview`, so the sink was a second copy of the
        // picture and the word was a claim about a control that did not exist.
        // Then it said the cells were a control an operator pressed to move the
        // audition, and ADR-0240 retired that control. Then it said three cells
        // read `off` because an off-air slot "has no new frame to show", which
        // was true only because the engine refused to draw one — the gap
        // ADR-0241 named, and this pass closed it. **Every sentence here is read
        // off the deck**, which is the only way this legend stops being
        // rewritten each time: the numbers are counted, not written.
        let live: Vec<usize> = (0..DECKS)
            .filter(|slot| {
                self.view
                    .mixer
                    .get(*slot)
                    .is_some_and(|strip| strip.tally == view::Tally::Live)
            })
            .collect();
        // A cell has a slot behind it or it has nothing; this deck is full, so
        // it is every cell. `Deck::slot_view` is `None` past `slot_count` and a
        // strip is a slot, so the mixer's length is the same count from the
        // other end.
        let behind = self.view.mixer.len().min(DECKS);
        println!(
            "{behind} of {DECKS} cells have a deck slot behind them and every one of them \
             is ON, whatever that slot's residency — a cell draws its own slot's material \
             through the same transfer curve the picture goes through, with no fader on it, \
             because a fader is applied in the mix and a cell is upstream of the mix. that \
             is what an operator watches to decide whether material is worth putting on \
             air, so it cannot wait until it is on air (ADR-0258). each cell is presented \
             from `Deck::slot_view` for the slot it is lettered for, so no cell can be \
             showing another deck under the wrong letter, and the picture above them is \
             the mix, always — there is no control that swaps it for one deck, and the \
             cells are why there does not need to be (ADR-0240). {} — that is about the \
             MIX and not about the cells: a slot that is not LIVE is drawn into its own \
             target and skipped by the composite, so it is watchable and inaudible. \
             every slot steps every frame at the room's tempo, whatever its residency, so \
             a cell shows material running rather than a still — a preview that is not on \
             the beat is not a preview of what putting that slot on air would look like \
             (ADR-0269). every cell costs a present pass of its own, and the step and the \
             draw of the three that are not LIVE are outside the compute budget — the \
             bill ADR-0258 says is paid rather than argued, with ADR-0269's three steps \
             added to it.",
            match live.as_slice() {
                [] => "nothing on this deck is LIVE, so the mix is empty".to_owned(),
                [one] => format!(
                    "deck {} is the only slot that is LIVE and reaches the mix",
                    deck_letter(*one as u8)
                ),
                many => format!(
                    "{} slots are LIVE and reach the mix — {}",
                    many.len(),
                    many.iter()
                        .map(|slot| format!("deck {}", deck_letter(*slot as u8)))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            },
        );
        println!(
            "the transport row reads the session's own oscillator — the tempo, the beat \
             inside the bar, and the bar counted from one — beside what the last frame \
             cost{}. the four dots are `karakuri_signal`'s BEATS_PER_BAR, which that \
             crate calls a provisional assumption of common time, so the console takes \
             the number a frame rather than assuming four.",
            match budget_ms {
                Some(budget) => format!(
                    ", against this display's refresh interval of {budget:.1} ms — which is \
                     the budget a frame is held to on a Fifo surface, and not the 20 ms a \
                     candidate Set is held to"
                ),
                // Said rather than passed over: a reading nobody can take is
                // worth a sentence, because the alternative is a reader
                // wondering where the mock's `/16.6` went.
                None => String::from(
                    " — with no budget beside it, because winit will not say what this \
                     display's refresh rate is"
                ),
            }
        );
        // **Two, and the list is not kept here.** `view::transport`'s *What is
        // in the mock's row and is deliberately not here* is the list, item by
        // item with what is missing behind each; this names the count and the
        // two survivors and points at it, because the crate that draws the row
        // is the thing that holds whether an item is drawn. Written out here it
        // was a second copy and it drifted exactly as one does: it said four
        // and named `landed` and `rec`, both of which left that list on
        // 2026-09-08 and both of which this panel draws.
        println!(
            "every control the mock draws in that row is drawn, and that list is empty for \
             the first time. `view::transport` is where it is kept. it has only ever got \
             shorter: `audio-in` left it when this program opened an input, `tap`, the \
             octave and the offset track left it when the tracker group landed, `rec` and \
             `landed` left it when this program grew a session recorder and a watcher on \
             every slot, and `learn` and `map` left it when this program opened a MIDI port \
             — all of them are drawn, they are pressed, and each emits the operation its \
             key already emitted or reports something this program really has. `map` is the \
             one that is a READOUT: it names the file that is loaded, and reaching a \
             different map while running is not built, so it has no chevron and no menu."
        );
        // **What the pill is actually reading, off the view rather than off a
        // sentence.** The legend was found lying five ways on 2026-08-30 by
        // saying what this program probably does; this says what it did.
        println!(
            "{}",
            match self
                .view
                .audio
                .as_ref()
                .and_then(|audio| audio.device.as_deref())
            {
                Some(device) => format!(
                    "the `audio-in` pill reads `{device}` and is drawn armed. energy, onset and \
                     band0..7 on the session's bus are measured from that input every frame, and \
                     the beat lock corrects the oscillator the transport row above draws. `b` \
                     taps, `,` and `.` move the grid an octave, `o` and `p` step the offset, and \
                     the three controls beside the pill are those same operations under a \
                     pointer — the tap capsule, the `1/2 x2` pair, and an 80px track that sets \
                     the offset outright. a press on the pill lists what else this machine has."
                ),
                None => String::from(
                    "the `audio-in` pill reads `none`, which is a state and not a fault: no \
                     input is open, every signal name answers what it answered before audio \
                     existed, and the grid free-runs at the session tempo. a press on the pill \
                     lists what this machine has, and `b`, `,` and `.` — and the tap capsule \
                     and the octave beside the pill — say so rather than doing nothing. there \
                     is no offset track: an offset belongs to a session and there is none, so \
                     the row closes up rather than drawing one at a number nobody chose."
                ),
            }
        );
        println!(
            "the mixer draws {} strip{}, because a strip is a deck SLOT and this deck has \
             {} — the mock's four is the most a deck can hold, and the page keeps its four \
             tracks either way, so a track with nothing behind it is empty rather than a \
             strip full of dashes. the two FADERS are played: drag the knob on the trim or \
             on the tall fader and the panel emits SetGain or SetOpacity, which \
             karakuri-operation-record turns into a Record — the one place an \
             operation becomes one, for every surface — and this file applies to the \
             deck. the strip then follows because the DECK changed, not because \
             anything here remembered. a press on the track \
             off the knob does nothing, deliberately: a fader at 0.3 whose top is clicked \
             must not jump to 1.0 on stage. the chips beside the faders are played too, and \
             this file has stopped counting them: what a pointer reaches on this panel is \
             said once, below, and asked of the crate that hit-tests it.",
            self.view.mixer.len(),
            match self.view.mixer.len() {
                1 => "",
                _ => "s",
            },
            self.view.mixer.len()
        );
        // **The park, said in this file's voice and then in the engine's.**
        // The sentence is this program's, because the words on this window are;
        // the numbers under it are `Report`'s own `Display` and the same three
        // fields `karakuri-cli`'s `report_governing` prints, so an operator
        // reading a park here and a park there is reading one thing.
        // **And why, read off the report rather than asserted here.** This
        // said *the budget has no room* in a fixed string, which was the only
        // park a bare run could reach while the default pair was 262144
        // elements. ADR-0271 moved it to `examples/star_vortex.kset`'s pair,
        // which is closed form — and `Governor::admit` asks *is there anything
        // to warm* before it asks the budget, so the same park comes back as
        // `NoPrimingNeeded` and a sentence naming the budget is this file
        // describing a decision it did not read. The line under this one has
        // printed the governor's own word all along; the two disagreeing is
        // worse than either being wrong alone.
        let parked_decisions: Vec<_> = governed
            .decisions
            .iter()
            .filter(|decision| {
                matches!(
                    decision.reason,
                    Reason::NoHeadroom
                        | Reason::NoPrimingNeeded
                        | Reason::Unmeasured
                        | Reason::CommittedUnknown
                )
            })
            .collect();
        for parked_dec in parked_decisions {
            let parked = deck_letter(parked_dec.slot as u8);
            let why = match parked_dec.reason {
                Reason::NoHeadroom => "the budget has no room",
                Reason::NoPrimingNeeded => {
                    "the Set is closed form and has nothing to warm, which is the one park no \
                     amount of budget resolves"
                }
                Reason::Unmeasured => "nothing measured what that slot costs",
                Reason::CommittedUnknown => {
                    "what this deck is already spending is unknown, so there is no headroom \
                     figure to admit against"
                }
                _ => "the governor did not grant it",
            };
            println!(
                "deck {parked}'s strip is the one that MOVES: its chip reads ALLOC and rolls \
                 part of the way toward PRIM once a second and falls back, never landing, \
                 because what the slot was asked for and what it is doing disagree. slot \
                 {parked} was asked to be primed and {why}, \
                 so `Deck::govern` holds it at allocated with the request intact — that is a \
                 PARK, which is `not now` and not `no`: nothing has to be asked twice, and the \
                 next pass over a deck with room admits it. nothing in this file writes an \
                 effective residency or draws a park; the strip carries both of the deck's own \
                 words for that slot and the view derives the rest."
            );
        }
        // **The slots nobody asked anything of, counted off the report rather
        // than named here.** `Reason::OffAir` is the governor's own word for
        // *allocated, and that is what was asked for*: it was not asked about
        // these slots and did nothing to them. A list written out in this file
        // would be this paragraph going on saying `C and D` the day the deck
        // opens differently.
        let resting: Vec<&str> = governed
            .decisions
            .iter()
            .filter(|decision| decision.reason == Reason::OffAir)
            .map(|decision| deck_letter(decision.slot as u8))
            .collect();
        if !resting.is_empty() {
            println!(
                "the other {} — {} — {} allocated and {} asked for nothing: the governor \
                 reports `OffAir`, which is a slot at REST rather than a slot refused. each \
                 holds this program's one pair at its own seed salt, because a slot cannot \
                 hold nothing and this program has no second pair to give one. it \
                 reaches the mix not at all — but it IS stepped and drawn, into its own \
                 target, every frame, which is what puts running material in its cell and \
                 what ADR-0258 and ADR-0269 ask for; the frame numbers below include both. \
                 REST is about the mix and about what was asked for, and not about whether \
                 the simulation runs: an operator brings one up by cycling its tally or \
                 loading a Set onto it, and what they see in the cell before they do is \
                 what they will get.",
                match resting.len() {
                    1 => "slot",
                    _ => "slots",
                },
                resting.join(" and "),
                match resting.len() {
                    1 => "is",
                    _ => "are",
                },
                match resting.len() {
                    1 => "was",
                    _ => "were",
                },
            );
        }
        println!("what the governor decided, in its own words:");
        println!("  {governed}");
        for decision in governed.parked() {
            println!(
                "  slot {} (deck {}) parked, request held: {:?} — {}, against {}. the \
                 request stands and the slot goes on stepping either way: a park withholds \
                 the grant and not the simulation (ADR-0269)",
                decision.slot,
                deck_letter(decision.slot as u8),
                decision.reason,
                match decision.cost_ms {
                    Some(ms) => format!("warming it was measured at {ms:.3} ms"),
                    None => String::from("nothing has measured it"),
                },
                match governed.headroom_ms() {
                    Some(ms) => format!("{ms:.3} ms of headroom"),
                    None => String::from("a committed cost nobody can know"),
                },
            );
        }
        println!(
            "the transition row under the strips IS drawn — a shape, a grid, a length and \
             the `go` that runs a wipe with them — and this window holds what is armed: \
             the three settings are the console's own, and a wipe is converted against \
             them. the crossfader that used to sit over it is not undrawn but \
             gone: the mixer has no crossfader. of the two focuses only one is drawn: the \
             deck selection is a solid ring round a strip, and keyboard focus is not \
             drawn at all because this console takes none — the two must not look alike, \
             which is why the mock's dashed outline is on nothing."
        );
        println!();
        for node in self.panel.nodes() {
            let (min, max) = layout.bounds(node.id);
            let bounds = format!(
                "min {min:.0}, max {}",
                match max.is_finite() {
                    true => format!("{max:.0}"),
                    false => "none".to_owned(),
                }
            );
            let what = match layout
                .name(node.id)
                .and_then(karakuri_console::view::region)
            {
                Some(region) => match region.kind {
                    // **The pills are named because they are controls.** A
                    // bay head's pills are exactly its controls — the table in
                    // `view::REGIONS` says so, and the Program bay's `solo` is
                    // the only one any bay has — so a head with one is a place
                    // a press reaches and a legend that said `bay` would be
                    // hiding it. Read off the table rather than written here.
                    Kind::Bay { pills, grip, .. } => {
                        let mut what = String::from("bay");
                        if grip {
                            what.push_str(", with a grip");
                        }
                        for pill in pills {
                            what.push_str(&format!(", `{pill}` in its head"));
                        }
                        what
                    }
                    // Four readouts, and then the controls that landed in
                    // this row after them: the audio-in pill, the arrangement
                    // pill and, at the far end, the tone map and the exposure.
                    // What the mock draws here and this panel does not is
                    // `view::transport`'s list, and it is not counted here —
                    // this comment said five while that list was down to two,
                    // which is what a total kept beside a list it does not own
                    // is worth. What a press in this row reaches is not
                    // counted here either — see the pointer's paragraph below.
                    Kind::Transport => "row, no heading: bpm, beat, bar, frame".to_owned(),
                    // The console's first control, and for a while its only
                    // one. How many there are now is
                    // `karakuri_console::input::CONTROLS` and is printed
                    // below; a number kept here would be that claim in a
                    // second place, which is the defect this legend is being
                    // repaired of.
                    Kind::Outputs => "row, one sink: program view".to_owned(),
                    Kind::Pane => "pane, inside a bay".to_owned(),
                    Kind::Picture => "the picture, a sink".to_owned(),
                    // Four cells, and how many are on is counted rather than
                    // written: a cell is on because there is a deck slot behind
                    // it, and a strip is a deck slot. Residency does not come
                    // into it — every slot is drawn — which is the correction
                    // this line carries. A constant here is exactly the legend
                    // naming a control's state from before the control existed,
                    // which this row has been twice already.
                    Kind::Previews => {
                        let on = self.view.mixer.len().min(DECKS);
                        format!("{DECKS} previews, {on} of them monitoring a deck slot")
                    }
                    // A bay like the other five: a row of chips saying which
                    // library is being read, and then a row per Set that one
                    // holds — as many as the bay has room for, and the foot
                    // says so. `n of m`, exactly as the bay draws it, and the
                    // scope is the view's own mark rather than a word written
                    // here.
                    Kind::Library => {
                        match karakuri_console::view::library(
                            layout,
                            &self.view.scopes,
                            &self.view.library,
                            self.view.opened(),
                            self.view.pointed(),
                            self.view.library_scroll(),
                        ) {
                            Some(bay) => format!(
                                "bay, {}, {} listed",
                                match self.view.scope() {
                                    Some(scope) => scope.name(),
                                    None => "no scopes",
                                },
                                bay.count()
                            ),
                            None => "bay, nothing said about any library".to_owned(),
                        }
                    }
                    // A bay like the other six, and then a strip per slot:
                    // a strip is a deck slot, and this deck is built full, so
                    // this reads four.
                    Kind::Mixer => format!(
                        "bay, {} strip{}",
                        self.view.mixer.len(),
                        match self.view.mixer.len() {
                            1 => "",
                            _ => "s",
                        }
                    ),
                    // A bay with four rows in it: the level the composited
                    // frame leaves the mix at, and the three fixed passes it
                    // then goes through. **How many of the three are recorded
                    // is what is worth saying**, because an amount of zero is
                    // no pass at all rather than a pass at nothing — see
                    // `karakuri_engine::master`.
                    Kind::Master => match (self.view.master_out, self.view.master_chain.as_ref()) {
                        (Some(out), Some(chain)) => format!(
                            "bay, out {out:.2}, {} slot{} in the chain",
                            chain.slots.len(),
                            match chain.slots.len() {
                                1 => "",
                                _ => "s",
                            }
                        ),
                        (Some(out), None) => format!("bay, out {out:.2}, no chain behind it"),
                        (None, _) => "bay, no engine behind it".to_owned(),
                    },
                    // A bay like the other five, and then a row per node a
                    // build changed whose verdict is outstanding. **None at
                    // startup, which is where this prints**: nothing has been
                    // rebuilt yet, and empty is this lane's ordinary state —
                    // see `view::staging`.
                    Kind::Staging => match self.view.staging.len() {
                        0 => "bay, nothing waiting".to_owned(),
                        waiting => format!("bay, {waiting} waiting"),
                    },
                    // **The bay whose body is a pattern**, counted off the
                    // pattern rather than written here: how many lanes there
                    // are and how many of them are driving something is what
                    // this bay *is*, and a constant would be the legend naming
                    // a control's state from before the control existed —
                    // which this line has been repaired of twice.
                    Kind::Sequencer => {
                        let pattern = self.sequencer.pattern();
                        let lanes = pattern.lanes().len();
                        let driving = pattern.lanes().iter().filter(|lane| !lane.muted()).count();
                        format!(
                            "bay, bank {} of {}, {} step{}, {lanes} lane{} and {driving} of them \
                             driving",
                            self.sequencer.armed() + 1,
                            karakuri_pattern::BANKS,
                            pattern.mode().count(),
                            match pattern.mode().count() == 1 {
                                true => "",
                                false => "s",
                            },
                            match lanes == 1 {
                                true => "",
                                false => "s",
                            }
                        )
                    }
                },
                None => match layout.axis(node.id) {
                    Some(Axis::Row) => "split, left to right".to_owned(),
                    Some(Axis::Column) => "split, top to bottom".to_owned(),
                    None => "not drawn".to_owned(),
                },
            };
            // **And the class pill where the region draws one**, appended
            // rather than written into each arm: four regions carry one, they
            // are four different `Kind`s, and an arm apiece would be four
            // copies of one sentence — which is the defect this legend was
            // repaired of the last time. What it says is the gate's own words,
            // so the sentence an operator reads here and the sentence a model
            // is refused with cannot drift apart.
            let what = match layout.name(node.id).and_then(class_at) {
                Some(class) => format!(
                    "{what}, `{}` at {} — {}",
                    view::mcp_word(self.view.opening.holds(class)),
                    class.opened_at(),
                    class.title()
                ),
                None => what,
            };
            println!(
                "  {:width$}{:<16} {:<22} {}",
                "",
                self.label(node.id),
                what,
                bounds,
                width = node.depth * 2
            );
        }
        println!();
        println!(
            "the outputs row's dot folds the picture by name, so clicking it and pressing f \n\
             over the picture are the same operation reached from two surfaces. it is lit \n\
             while the picture is on screen. it is one of two kinds of control and a fader \n\
             is the other: the dot names an operation on the ARRANGEMENT, which this crate \n\
             performs, and a fader names one on the MIX, which it cannot — so the operation \n\
             comes out and this file applies it."
        );
        println!();
        // **Whether anything reads these four is read off the server**, not
        // written here. The unconditional sentence this replaces said *nothing
        // in this process serves MCP yet* and printed it on every run,
        // including one whose server `main` had already bound and whose
        // address it had already printed — the same defect as a device named
        // in prose instead of asked of the host, which is why this reads like
        // the `audio-in` line above.
        println!(
            "four of the regions above carry an `mcp` pill, and each says beside its own line \n\
             which state its class is in. each opens one class of operations to a model; every \n\
             operation stays connected either way, and what shut changes is that the call is \n\
             answered with a refusal instead of being performed — one that names the class and \n\
             says which pill opens it. it is the one control on this panel that is neither an \n\
             operation on the ARRANGEMENT nor one on the MIX: it is a setting of the map every \n\
             surface reaches the vocabulary through, and a setting deciding whether a surface \n\
             may reach a class of operations cannot be a member of that class (ADR-0236). \n\
             {}",
            match mcp_port {
                Some(port) => format!(
                    "this process IS serving MCP, on 127.0.0.1:{port}, and \
                     `karakuri_mcp::serve` holds the very handle these four pills \
                     write — not a copy of it — so a pill opened here is read by the server on \
                     the next call it answers, and one shut here refuses the next call in the \
                     class's own words. what they open is live for the length of this run."
                ),
                None => String::from(
                    "no `--mcp` port was given, so nothing in this process serves MCP on this \
                     run and what these four write is read here, by this file's tests, and by \
                     nothing else. the pills still write it, which is the state a run is in \
                     rather than a control that does nothing: `--mcp PORT` is what gives it a \
                     second reader."
                ),
            }
        );
        println!();
        println!(
            "the pointer, and this file no longer keeps a list of what it reaches. a press \n\
             in a gap takes the boundary and it follows the pointer. a press on a LIBRARY \n\
             row takes that Set in hand and nothing happens until you let it go: over a \n\
             mixer strip it loads that deck, anywhere else it loads nothing. that is the \n\
             third way in to the same load `enter` performs in the library, and the only \n\
             one that names both the Set and the deck in one gesture. the second is the \n\
             `load` button in that bay's foot, which lands on the deck the pulldown \n\
             beside it names rather \n\
             than on the deck the keys are addressed to, so a load can be aimed without \n\
             moving the selection (ADR-0305). everywhere else \n\
             `karakuri_console::input::claim` decides, and the {CONTROLS} controls its rule \n\
             4 hit-tests are painted shapes with no widget behind them — nothing but that \n\
             rule knows a press landed on one. the number is that crate's own constant, \n\
             summed over the derivations the rule actually asks, so a control added there \n\
             and not counted is a compile error rather than a sentence that has gone quiet. \n\
             what each of them is, and what a press on it asks for, is written at the \n\
             derivation that draws it: a copy here is precisely how this legend came to \n\
             name three controls while every one of them answered a press."
        );
        println!();
        println!(
            "keys — `tab` and `esc` move the address, drawn as a dashed ring on the bay \n\
             that has it, and the four below them act on whatever that address is on. all \n\
             nine bays have the grammar, and `space` on a bay is the fold wherever you are. \n\
             `g` is addressed to the focused bay too; the letters after it are global:"
        );
        for (key, what) in KEYS {
            println!("  {key:<10}{what}");
        }
        println!();
    }
}

pub(crate) fn folding(folded: bool) -> &'static str {
    match folded {
        true => "folded",
        false => "unfolded",
    }
}

/// Which deck, in the letter the preview cells are drawn with — asked of the
/// view rather than written out again here.
///
/// The vocabulary counts decks from zero (`Operation::SetGain { deck: u8 }`)
/// and the console draws them `A` through `D`, so this is the one place the two
/// meet. A deck outside the four cannot be built by anything in this file — a
/// `Deck` holds `MAX_SLOTS` slots and `DECKS` is that number — so an index past
/// the end is a bug and reads as one rather than wrapping quietly.
pub(crate) fn deck_letter(deck: u8) -> &'static str {
    DECK_LETTERS
        .get(usize::from(deck))
        .copied()
        .unwrap_or("(no such deck)")
}

/// Which fader, in the bay's own word for it.
pub(crate) fn knob_word(knob: &Knob) -> &'static str {
    match knob {
        Knob::Trim { .. } => "trim",
        Knob::Fader { .. } => "fader",
        Knob::Out => "out",
        Knob::Chain { .. } => "chain effect",
        Knob::Param { .. } => "parameter",
    }
}

/// Whose fader it is, for a line a reader has to place: a deck by its letter,
/// and the master out by the bay it is in.
///
/// The master out names no deck — it is one level on the whole fold
/// ([ADR-0224](../../../docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md))
/// — so a line that said *deck A* over it would be naming a slot nothing in the
/// gesture ever touched. `Knob::deck` is what answers, and this is the only
/// caller: everything that *acts* takes the deck out of the operation.
pub(crate) fn knob_where(knob: &Knob) -> String {
    match knob.deck() {
        Some(deck) => format!("deck {}", deck_letter(deck)),
        None => "master".to_owned(),
    }
}
