use super::*;

/// One press of a gain key. Linear and additive, because a fader is: the
/// operator wants the same physical move to mean the same amount everywhere,
/// not a proportion of wherever the slot happens to be.
pub(crate) const GAIN_STEP: f32 = 0.1;

/// Where a scheduled fade starts, and what to call it. A bar is four beats
/// here, which is an assumption rather than a measurement: nothing in the
/// signal bus knows a time signature, and four is what the `bar` signal already
/// means. A set in three would want this to be a dial, and would say so by
/// having a fade land in the wrong place — which is a better way to find out
/// than a setting nobody knew to change.
pub(crate) const QUANTA: [(f64, &str); 3] =
    [(4.0, "the next bar"), (1.0, "the next beat"), (0.0, "now")];

/// How long a scheduled fade lasts, in beats. A bar, half a bar, two bars, and
/// a cut — the four an operator reaches for, in the order they are reached for.
pub(crate) const FADE_BEATS: [f64; 4] = [4.0, 2.0, 8.0, 0.0];

/// The shapes a wipe can take, in cycle order, with the angle each runs at and
/// what to call it.
///
/// `None` first, so that a deck nobody has touched wipes with nothing and says
/// so rather than doing something. The rest are the four directions and the
/// iris — the ones a hand reaches for. An arbitrary angle is a dial, and a dial
/// with nowhere to show its value is a control an operator cannot read.
pub(crate) const MASK_SHAPES: [(MaskKind, f32, &str); 6] = [
    (MaskKind::None, 0.0, "off — `c` needs a shape"),
    (MaskKind::Linear, 0.0, "linear, left to right"),
    (
        MaskKind::Linear,
        std::f32::consts::FRAC_PI_2,
        "linear, bottom to top",
    ),
    (
        MaskKind::Linear,
        std::f32::consts::FRAC_PI_4,
        "linear, diagonal",
    ),
    (
        MaskKind::Linear,
        -std::f32::consts::FRAC_PI_4,
        "linear, the other diagonal",
    ),
    (MaskKind::Radial, 0.0, "an iris"),
];

/// One press of an opacity key. Additive for the same reason as [`GAIN_STEP`],
/// and clamped to `[0, 1]` where gain is not: opacity is a proportion of a
/// blend and there is no such thing as 1.4 of one, while gain is a level into
/// an HDR mix and values above 1.0 are ordinary.
pub(crate) const OPACITY_STEP: f32 = 0.1;

/// One press of an exposure key, as a factor. Multiplicative, because exposure
/// is: a stop is a ratio, and an additive step would be enormous at 0.1 and
/// invisible at 8.0. This is a quarter of a stop, near enough.
pub(crate) const EXPOSURE_STEP: f32 = 1.189_207;

/// The interactive bounds, and only the interactive ones. `-` and `=` nudge
/// inside them, because a key that steps has to stop somewhere and a stop it
/// cannot see is worse than one it can. `--exposure` is not held to them, which
/// is deliberate and is `exposure_positive_is_accepted_unclamped`'s own
/// sentence: a batch render asks for something extreme on purpose, and a flag
/// is read once by somebody who typed it rather than nudged into a corner. What
/// the flag refuses is what has no meaning at all — see [`clamp_exposure`] for
/// why zero and negative are neither clamped nor accepted anywhere.
///
/// This comment said the two shared a range and they never have. It was written
/// beside a constant pair pulled out so the bound would not drift, and the
/// drift was the sentence rather than the numbers.
pub(crate) const EXPOSURE_MIN: f32 = 1.0 / 64.0;
pub(crate) const EXPOSURE_MAX: f32 = 64.0;

/// Exposure is a multiplier before the tone map; zero is degenerate (always
/// black) and negative inverts an otherwise-positive HDR value into one no tone
/// mapper is specified for. Pulled out as a pure function so the bound is one
/// piece of logic instead of two copies that could drift, and so it is testable
/// without a `Live` or a GPU.
pub(crate) fn clamp_exposure(exposure: f32) -> f32 {
    exposure.clamp(EXPOSURE_MIN, EXPOSURE_MAX)
}

/// One press of the scrub keys, in beats. A quarter beat — a sixteenth of a bar
/// in four — which is small enough to place a hit by ear and large enough to
/// hear one press.
pub(crate) const SCRUB_BEATS: f64 = 0.25;

/// A level floor: a negative gain would subtract one slot's light from
/// another's, which is a blend mode rather than a level. Not ceilinged — the
/// pipeline is HDR and values above 1.0 are expected. Pure for the same reason
/// as [`clamp_exposure`].
///
/// `Deck::set_gain` floors too, and the two are not a duplicate. This one
/// decides what the record says, so a session replays the value that took
/// effect rather than one the engine quietly corrected; that one guards the
/// engine against every record it did not write, which is the whole of a
/// replay. Deleting either leaves a real hole.
pub(crate) fn clamp_gain(gain: f32) -> f32 {
    gain.max(0.0)
}

/// The tone map cycle `t` steps through. Pure so the cycle — and that it
/// returns to where it started — is checkable without a window.
pub(crate) fn next_tonemap(op: TonemapOp) -> TonemapOp {
    match op {
        TonemapOp::Clamp => TonemapOp::Reinhard,
        TonemapOp::Reinhard => TonemapOp::Aces,
        TonemapOp::Aces => TonemapOp::AgX,
        TonemapOp::AgX => TonemapOp::Clamp,
    }
}

/// What this program does with an operation: the records it writes, or the
/// sentence saying it did nothing.
///
/// [`Live::performed`] takes the readings and this decides, so that the answer
/// a model is handed and the line a terminal is given are one string built
/// once. It is a free function rather than a method for the same reason
/// [`rewired`] is: it needs no `Live`, and a test can hold it against a real
/// call over the socket without a window or a GPU.
///
/// An operation that writes no record writes nothing here. This surface
/// performs an operation by converting it to records and reading them back —
/// there is no second arm — so `Written::Silent` and `Written::Owed` both mean
/// nothing on this run changed, and a caller that reported success for one
/// would be reporting a change it did not make
/// ([P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md):
/// a silently wrong answer loses to a loud failure). That is the whole of why
/// this is a `Result`: `Live::operate`'s printed line reaches an operator who
/// is at the terminal, and a model on `--mcp` is not.
///
/// The refusal names the operation and where it is answered
/// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)),
/// and it is the performer's rather than the gate's — which is
/// [ADR-0341](../../../docs/adr/0341-a-route-that-answers-is-built-and-a-send-that-ends-in-a-dialog-is-gap.md)'s
/// *a route that answers is a built route* read on the surface that has no
/// performer instead of the one that has one. `Silent::why` and `Owed::why` are
/// the reasons in the words the crate that decided them says them in, so
/// nothing is written down twice.
pub(crate) fn answered(operation: &Operation, current: &Current) -> Result<Vec<Record>, String> {
    match karakuri_operation_record::written(operation, current) {
        Written::Records(records) => Ok(records),
        Written::Silent(silent) => Err(format!(
            "`{}` was not performed and nothing on this run changed: {}, and this program \
             performs an operation by writing the records it converts to. `karakuri-cli` has \
             no control for this one — the instrument, `cargo run -p karakuri`, is the \
             surface that answers it.",
            operation.title(),
            silent.why()
        )),
        // **The reading, and not the control.** `Owed` says a record is owed
        // and could not be made here — a deck the run does not hold, or a
        // vocabulary question nobody has settled — so the sentence is that
        // reason and not *this program has no control for it*, which would be
        // false of an operation whose key is on this keyboard.
        Written::Owed(owed) => Err(format!(
            "`{}` was not performed and nothing on this run changed: {}",
            operation.title(),
            owed.why()
        )),
    }
}

/// The answer an operation that was performed goes back with.
///
/// One string, so that [`Live::run_operations`] and the test that drives it
/// over a socket say the same thing — and so that the sentence which says
/// *where a later answer lands* is beside the one that says nothing landed.
pub(crate) fn performed_at_the_frame(title: &str) -> String {
    format!(
        "`{title}` was performed on the frame it arrived on, where the same operation from a \
         key or a mapped control is performed. Anything it started rather than finished is \
         reported where it lands: ask `swap_outcome` for a rebuild, and a scheduled move \
         arrives on the grid."
    )
}

/// Whether `at` names a renderer of a slot that draws with `count` of them, and
/// the sentence if it does not. [`slot_in_range`]'s companion, returning the
/// refusal rather than a bool because both callers print it.
pub(crate) fn renderer_in_range(slot: usize, at: usize, count: usize) -> Result<(), String> {
    if at < count {
        Ok(())
    } else {
        Err(no_such_renderer(slot, at, count))
    }
}

impl Live {
    /// A key press. Returns true if it was a request to quit.
    ///
    /// Every branch prints what it did. There is no on-screen UI, and a control
    /// that changes something invisible — a gain on an off-air slot, an
    /// operator on a dark frame — is indistinguishable from a control that is
    /// broken.
    ///
    /// # Every key names an operation, and they fall in four groups
    ///
    /// [`Live::run_surface`] takes a mapped message straight to
    /// [`Live::operate`], because every operation a map line can produce is one
    /// `karakuri_operation_record::written` converts. A keyboard is not that
    /// shape, and this is the survey rather than an intention: of the
    /// thirty-nine keys below, twenty-one name an operation whose record
    /// converts, three name one whose record is owed, twelve name one that
    /// writes no record at all, and three name nothing in the vocabulary.
    ///
    /// - Its record converts, so it goes through [`Live::operate`].
    ///   `space` and `w` (`SetResidency`), `[`, `]` and `\` (`SetGain`),
    ///   `;` and `'` (`SetOpacity`), `m` (`SetBlendMode`),
    ///   `t` (`SetTonemap`), `-`, `=` and the backquote (`SetExposure`), `u`
    ///   and `i` (`ScrubDeck`), `y` (`SetSync`), `f` and `g` (`FadeDeck`),
    ///   `x` (`Crossfade`), `r` (`SelectRenderer`), `c` (`Wipe`). A key and a
    ///   mapped pad reach the deck by one
    ///   derivation, which is the whole of why the two surfaces cannot drift.
    ///   Which way to step is still the keyboard's — a cycle and a nudge are
    ///   translations a surface makes, never operations
    ///   (`docs/principles/0090-a-surface-offers-it-never-decides.md`).
    /// - Its record is owed, so it keeps its present path — each with the
    ///   reason written at the function it ends in: `b` ([`Live::tap`]), `,`
    ///   and `.` ([`Live::shift_octave`]). All three
    ///   operations answer `Owed::NotSettled`, and the day one is settled its
    ///   function is what goes — the promise `run_surface`'s `TapBeat` arm
    ///   already carries. Routing one through `operate` today would print the
    ///   gap where the gesture used to happen, which is the one thing a change
    ///   of route may not do.
    ///
    ///   It was eight, and the five that left are what the promise looks
    ///   like kept three times. `Operation::SetSync` was owed for the
    ///   engine's anchor clamp; the clamp turned out to be the identity on
    ///   every tempo an oscillator can report, the conversion took a session
    ///   tempo as a reading, and [`Live::cycle_sync`] moved to `operate` with
    ///   its record-building deleted rather than kept in step. `FadeDeck`,
    ///   `Crossfade` and `SelectRenderer` went the same way and took a whole
    ///   function with them: the quantum and the length `n` and `j` cycle are
    ///   this surface's, `Current::transition` is where they are handed
    ///   over, and `Live::fade_slot` is gone rather than kept beside the
    ///   conversion. `c` was the one that stayed and is the fifth to go,
    ///   because what a wipe carried beyond a fade turned out to be unassigned
    ///   rather than missing too: the shape its front takes is the third of
    ///   the settings `z`, `n` and `j` write and travels with the other two,
    ///   and the soft edge is read off the mask on the deck being wiped in.
    ///   [`Live::wipe`] keeps only the two refusals and the line it prints.
    ///
    ///   What is left is the beat tracker, and it is a different shape. A
    ///   tap and an octave shift do not want a reading nobody hands over; they
    ///   want the beat lock's answer, which no value a surface holds
    ///   determines — see [`Live::tap`].
    /// - It writes no record, and this surface is what has to perform it.
    ///   `0`–`3` (`SelectDeck`), `z`, `n` and `j` (`SetTransition`), `a`
    ///   (`SizeWindow`), `k` (`SaveSet`), `o` and `p` (`SetLatencyOffset`),
    ///   `esc` (`Quit`). These cannot route through `operate` either, and
    ///   that is a fact about `Silent` rather than an omission: `operate`
    ///   turns an operation into the records it writes and applies those, so an
    ///   operation that writes none would print [`answered`]'s refusal and the
    ///   key would do nothing. What most of them change is a surface's own
    ///   state, and this surface is the only thing holding it.
    ///
    ///   Two of them are reachable over `--mcp` and are refused there, in
    ///   that same sentence: `Operation::SetLatencyOffset` and
    ///   `Operation::Quit` are `Sayable::Operable`, and a key that nudges is
    ///   not a performer for an operation that names a value. See
    ///   [`Live::run_operations`].
    /// - The vocabulary does not name it at all. `S` prints the status line
    ///   and `h`/`?` print [`BINDINGS`]. Neither has a row on
    ///   `docs/manual/operations.html`, which is the specification for which
    ///   operations exist — so neither is an operation anybody has
    ///   specified. Left as found and reported, because a row invented here
    ///   would be a specification written from the implementation.
    ///
    ///   *(Under ADR-0346, superseding ADR-0220, the CLI and GUI keyboard
    ///   mappings converge onto the unified specification in
    ///   `docs/manual/operations.html`. The interim scaffolding divergence
    ///   on single-letter keys is resolved by migrating the colliding CLI
    ///   keys to uppercase keys `F`, `G`, `Z`, `R`, `N`, `U`, `S`, while `o`
    ///   and `p` agree on both keyboards for audio latency offset. See
    ///   `docs/adr/0346-the-gui-and-cli-keymaps-diverged-in-scaffolding-and-converge-on-the-operations-page.md`.)*
    pub(crate) fn key(&mut self, key: &Key) -> bool {
        match key.as_ref() {
            Key::Named(NamedKey::Escape) => return true,
            Key::Named(NamedKey::Space) => self.toggle_focused(),
            Key::Character(s) => match s.chars().next().unwrap_or('\0') {
                c @ '0'..='3' => self.focus_slot(c as usize - '0' as usize),
                '[' => self.nudge_gain(-GAIN_STEP),
                ']' => self.nudge_gain(GAIN_STEP),
                '\\' => self.set_gain(self.focus, 1.0),
                ';' => self.nudge_opacity(-OPACITY_STEP),
                '\'' => self.nudge_opacity(OPACITY_STEP),
                'm' => self.cycle_blend(self.focus),
                'F' => self.fade(0.0),
                'G' => self.fade(1.0),
                'x' => self.crossfade(),
                'c' => self.wipe(),
                'Z' => self.cycle_mask(),
                'R' => self.cycle_renderer(),
                'N' => self.cycle_quantum(),
                'j' => self.cycle_fade_beats(),
                't' => self.cycle_tonemap(),
                '-' => self.set_exposure(self.look.exposure / EXPOSURE_STEP),
                '=' => self.set_exposure(self.look.exposure * EXPOSURE_STEP),
                '`' => self.set_exposure(1.0),
                'w' => self.toggle_priming(self.focus),
                'y' => self.cycle_sync(),
                'U' => self.scrub(-SCRUB_BEATS),
                'i' => self.scrub(SCRUB_BEATS),
                'b' => self.tap(),
                ',' => self.shift_octave(0.5),
                '.' => self.shift_octave(2.0),
                'o' => self.nudge_latency_offset(-audio::LATENCY_OFFSET_STEP_MS),
                'p' => self.nudge_latency_offset(audio::LATENCY_OFFSET_STEP_MS),
                'a' => self.snap_to_canvas(),
                // The focused slot, a stamped name, and nobody waiting: a hand
                // has one slot in front of it, cannot type a name, and is
                // reading the terminal.
                'k' => self.save_set(Asked::Operator, self.focus, None, None),
                'S' => self.print_status(),
                'h' | '?' => eprint!("{BINDINGS}"),
                _ => {}
            },
            _ => {}
        }
        false
    }

    pub(super) fn focus_slot(&mut self, slot: usize) {
        if !slot_in_range(slot, self.deck.slot_count()) {
            eprintln!("{}", no_such_slot(slot, self.deck.slot_count()));
            return;
        }
        self.focus = slot;
        let addr = EngineSlot(slot as u8);
        eprintln!(
            "focus slot {slot} — {}, gain {:.2}, t {:.2}s",
            residency_name(self.deck.residency(addr), self.deck.is_parked(addr)),
            self.deck.gain(addr),
            self.deck.slot(addr).set().time()
        );
    }

    /// On air and off again. `Allocated` frees nothing and resets nothing, so the
    /// `t` printed on the way out is the `t` printed on the way back in — which is
    /// the whole property, and printing both ends is the only way to see it without
    /// a debugger.
    pub(super) fn toggle_focused(&mut self) {
        self.toggle_on_air(self.focus);
    }

    /// On air and off again, for a named slot.
    ///
    /// The toggle is the key's affordance and not an operation, which is why the
    /// slot is a parameter and the focus is filled in by the caller: what reaches
    /// the deck is `SetResidency` naming one of three
    /// (`docs/principles/0090-a-surface-offers-it-never-decides.md`). A control
    /// surface says which state it wants on the line and does not come through here
    /// at all — it used to, and the state it landed in was this function's to
    /// decide.
    pub(super) fn toggle_on_air(&mut self, slot: usize) {
        let addr = EngineSlot(slot as u8);
        let t = self.deck.slot(addr).set().time();
        match self.deck.residency(addr) {
            Residency::Live => {
                self.operate(&Operation::SetResidency {
                    deck: slot as u8,
                    residency: mix::residency(Residency::Allocated),
                });
                eprintln!("slot {slot} off air — allocated, holding t {t:.2}s");
            }
            // Priming is warming out of sight and is still off air, so space
            // does the same thing to it: puts it on, at whatever `t` it has
            // warmed to.
            Residency::Allocated | Residency::Priming => {
                self.operate(&Operation::SetResidency {
                    deck: slot as u8,
                    residency: mix::residency(Residency::Live),
                });
                eprintln!("slot {slot} on air — resuming at t {t:.2}s");
            }
        }
    }

    /// Ask the focused slot to warm out of sight, or withdraw the request.
    ///
    /// A request, not a command — the governor decides whether it is granted, and
    /// the report printed here says which. Whether it takes effect is exactly the
    /// thing the operator cannot otherwise see: a refused request leaves the slot
    /// at Allocated, which is what an untouched slot looks like too.
    ///
    /// Live slots are left alone. Priming is off-air warming, so asking a slot on
    /// air to prime could only mean taking it off air, and that is what space is
    /// for.
    pub(super) fn toggle_priming(&mut self, slot: usize) {
        let addr = EngineSlot(slot as u8);
        if self.deck.residency(addr) == Residency::Live {
            eprintln!("slot {slot} is on air — priming is off-air warming; take it off with space");
            return;
        }
        let requested = self.deck.requested_residency(addr);
        let want = if requested == Residency::Priming {
            Residency::Allocated
        } else {
            Residency::Priming
        };
        // Said before the record is applied, because applying it runs a
        // governor pass that prints what it decided — and the decision reads as
        // an answer to a question the operator has not seen asked otherwise.
        eprintln!(
            "slot {slot} {}",
            if want == Residency::Priming {
                "asked to warm off air"
            } else {
                "prime request withdrawn"
            }
        );
        self.operate(&Operation::SetResidency {
            deck: slot as u8,
            residency: mix::residency(want),
        });
    }

    /// Take up what a slot is now playing, and say so in the stream if a session is
    /// being recorded.
    ///
    /// `landed` is the id of the build that went in, and there is no other case: a
    /// swap is the only event that changes what a slot holds (ADR-0316). It used to
    /// take an `Option`, whose `None` was a rollback, and the version that came
    /// back had to have been remembered because the stream would never name it
    /// again.
    ///
    /// The bookkeeping is unconditional and the record is not, and the two used to
    /// be one function that began by returning when there was no recorder. What a
    /// slot is playing was therefore a fact only a recorded run had — and
    /// `Live::save_set` needs exactly that fact in the ordinary `--watch` case,
    /// where nothing is being recorded. Splitting it is the whole of what widening
    /// `watch::Watch::stored` is for on this side of the channel.
    pub(super) fn took_up(&mut self, slot: usize, landed: u64) {
        // Drained here rather than per frame: the channel only has anything in
        // it when a build has just been requested, and this runs when one has
        // just landed.
        if let Some(rx) = &self.rebuilds {
            while let Ok(built) = rx.try_recv() {
                self.pending_builds.insert(built.id, built);
            }
        }

        // **A build with nothing in `pending_builds` is one whose sources the
        // watcher could not store**, which it said at the time. It is handed to
        // `landed` as `None` rather than returned on, because the swap happened
        // either way: a slot that took a version nobody can name is a slot with
        // no address, not a slot still on its old one. See [`Running::landed`],
        // which is where that used to go wrong.
        let built = self.pending_builds.remove(&landed);
        let Some(nodes) = self.running.landed(slot, built.map(|built| built.nodes)) else {
            // A slot with nothing in the store behind it — one filled from a Set
            // file with nothing watching it, or one that has just taken a build
            // whose sources could not be stored, which was said at the time.
            // There is no hash to name, so there is nothing this could record.
            return;
        };
        self.record_procedure(slot, &nodes);
    }

    /// Say what a slot is playing, now that it changed.
    ///
    /// One thing these records cannot carry, and ADR-0316 moved which thing that
    /// is. A swap *in* is documented to start cold, so a replay meeting these
    /// records builds afresh and replays exactly. What no record says is that a
    /// slot was stopped: a version over the budget is recorded like any other,
    /// because it is what the slot holds, and a replay judges nothing — so it runs
    /// material the performance had frozen. That is a gap in the record vocabulary
    /// rather than in this function.
    pub(super) fn record_procedure(&mut self, slot: usize, nodes: &Nodes) {
        if self.recorder.is_none() {
            return;
        }
        // One record per node, each at the address the watcher gave it — so a
        // slot with one renderer writes exactly the two lines it always did,
        // and a slot with a chain and two geometries writes a line for each.
        for (layer, index, hash) in nodes {
            let Some(record) = layer_named(layer).map(record_layer) else {
                eprintln!("  a node on layer `{layer}` is not in the record vocabulary");
                continue;
            };
            self.record_only(karakuri_store::record::Record::Procedure {
                slot: DeckSlot(slot as u8),
                at: karakuri_store::record::NodeAddress {
                    layer: record,
                    index: *index,
                },
                proc_hash: *hash,
            });
        }
    }

    /// Push a record without applying it.
    ///
    /// Two records are right here, and they are right for one reason: each
    /// *describes* a change that has already happened rather than asking for one. A
    /// `procedure` says what a slot became when a swap landed, and a `save` says a
    /// file exists — applying either would mean doing the thing a second time.
    /// Everything else goes through `Live::record`, which applies what it wrote.
    pub(super) fn record_only(&mut self, record: karakuri_store::record::Record) {
        if let Some(recorder) = &mut self.recorder {
            recorder.push(record);
        }
    }

    /// Write what this run is playing as a Set file.
    ///
    /// The control that closes an open gap: `--save-set` writes what the *flags*
    /// say and exits, so the loop that lets an operator load a preset, edit it and
    /// watch it had no way to keep the result. This is the render-loop half of
    /// closing that — see
    /// `docs/adr/0122-a-save-writes-the-bytes-that-are-on-screen.md`. Every surface
    /// ends here — the key below, the MCP tool, and whatever surface arrives next —
    /// for the same reason every mix control ends in one method. This is the only
    /// save path, which is what makes a refusal and an outcome one sentence each
    /// rather than one sentence per surface.
    ///
    /// The slot is an argument and the key passes its focus in. A Set file
    /// describes one Set and a deck holds four; the one an *operator* means is the
    /// one their hands are already on, which is what focus is — and a model has no
    /// hands and no focus, so it names the slot as it names one to read a
    /// procedure. This is `toggle_on_air`'s split, for its reason.
    ///
    /// `id` is what the caller wanted it called, or a stamp. A key press cannot
    /// type a name, so it passes `None`; see `history::stamped_id`, whose
    /// convention that is and whose reason it borrows — an operator looks for the
    /// time they saved it. A caller that *can* type one is not made to take a
    /// timestamp.
    ///
    /// `reply` is whoever is waiting who is not at the terminal. Every sentence
    /// below goes to both, and each of them is written once: a refusal that reached
    /// a model in different words than it reaches the terminal would be two
    /// refusals to keep in step, and the wording of this one has already had to be
    /// corrected once.
    ///
    /// Read off the live Set, not off `Args`. `saving_seeds` states the hazard from
    /// the other side: a writer with its own copy of the rule records numbers the
    /// run was not using. Every number a Set file carries can have moved since the
    /// flags were parsed — a param through a record, a capacity or a salt through a
    /// rebuilt Set file — so the only reading that cannot be stale is the Set's
    /// own.
    ///
    /// Refused, accepted, gathered, written — and only the third of those needs a
    /// `Deck`. The three sentences a save can produce before the disk speaks are
    /// [`no_such_slot`], [`nothing_to_save`] and [`accepted_save`], all free
    /// functions, so what a client is told is checkable without a window.
    /// `playing_values` is the line that is not, and everything above it here is
    /// above it deliberately.
    ///
    /// Gathered here, written elsewhere. Everything below this line is a read off
    /// values already in memory; the store I/O goes to a thread of its own — one
    /// per save, since saves are rare and a pool would be machinery for a rate of a
    /// few an hour. The outcome comes back over `saves` and the record is written
    /// at the frame it arrives, not at this key press. See `Live::finished_saves`.
    pub(super) fn save_set(
        &mut self,
        asked: Asked,
        slot: usize,
        id: Option<String>,
        reply: Option<mcp::Reply>,
    ) {
        // **Checked here rather than only where the request came from.** A key
        // press cannot name a slot this deck does not hold and a tool call can,
        // and below this line `playing_values` reads `deck.slot(slot)`, which
        // panics on one. The surface that asked refuses it too, in its own
        // words, so a model never reaches this — and this is the guard that
        // does not depend on it having.
        if !slot_in_range(slot, self.deck.slot_count()) {
            return refused(reply, no_such_slot(slot, self.deck.slot_count()));
        }
        let startup = self.startup.get(slot).map_or(&[][..], Vec::as_slice);
        let sources = live_sources(self.running.playing(slot), startup);
        if sources.is_empty() {
            return refused(
                reply,
                nothing_to_save(slot, self.loaded_set.as_deref(), startup.is_empty()),
            );
        }
        // **Named and answered above the GPU line**, which is where the whole
        // of [`accepted_save`]'s doc lives: `playing_values` below is the one
        // read here that needs a `Deck`, and the accept has to be on the side
        // of it a test can reach.
        let id = accepted_save(
            slot,
            asked,
            id,
            &sources,
            &self.store_root,
            reply
                .as_ref()
                .map(|r| r as &dyn karakuri_environment::SaveReply),
        );
        let values = playing_values(self.deck.slot(EngineSlot(slot as u8)).set(), &self.edges);
        let save = Save {
            slot,
            asked,
            id,
            root: self.store_root.clone(),
            sources,
            values,
        };
        let tx = self.save_tx.clone();
        // **A thread per save**, and detached: no frame waits for it. The *run*
        // waits, once, at the end and under a bound — see
        // [`Live::awaited_saves`], which is what this count is for.
        self.saves_in_flight += 1;
        std::thread::spawn(move || {
            let (slot, asked, id) = (save.slot, save.asked, save.id.clone());
            let outcome = save.run();
            // **Carried back rather than answered from here.** This thread
            // knows the outcome and could send it, and that would be a second
            // place a save is reported from: the record is written at the frame
            // the outcome lands, and a model told anything else than what that
            // frame decided would be reading a different story from the stream.
            let _ = tx.send(Saved {
                slot,
                asked,
                id,
                outcome,
                reply,
            });
        });
    }

    /// Every save that has landed since the last frame, said and recorded.
    ///
    /// Drained and never waited on: a frame owes the display a picture and owes a
    /// disk nothing.
    ///
    /// Called at the top of the frame, beside [`Live::run_requests`] and above
    /// `frame::compose` and everything downstream of it — not beside the swap-event
    /// drain, which is where it used to be, below the early returns a frame no
    /// longer has. See the comment at the head of [`Live::frame`]: a window that
    /// has faulted still has saves finishing behind it, and a run that told nobody
    /// about them until it quit was withholding the one answer a waiting client
    /// cannot get anywhere else. The swap drain stays below because a swap *is*
    /// about what was drawn; a save is not.
    ///
    /// The record is written here, at the frame the outcome arrived, which is the
    /// pattern `record_procedure` already follows — a record that describes a
    /// change already made. Writing one at the key press would be a stream claiming
    /// a file that the disk then refused, which is the failure this whole codebase
    /// is arranged against.
    pub(super) fn finished_saves(&mut self) {
        let mut landed: Vec<Saved> = Vec::new();
        while let Ok(saved) = self.saves.try_recv() {
            landed.push(saved);
        }
        for saved in landed {
            self.took_save(saved);
        }
    }

    /// One save's outcome, said, recorded, and answered.
    ///
    /// One sentence for all three. What the terminal is told, what the stream
    /// records and what a waiting client is handed are the same fact, so the words
    /// are formed once here and the client gets the ones the operator got. The
    /// `Ok`/`Err` split is what a tool call's `isError` is built from — see
    /// [`mcp::Reply::settled`].
    pub(super) fn took_save(&mut self, saved: Saved) {
        let Saved {
            slot,
            asked,
            id,
            outcome,
            reply,
        } = saved;
        self.saves_in_flight = self.saves_in_flight.saturating_sub(1);
        let said = match outcome {
            Ok(()) => {
                // **Two sentences because two things are true**, and the second
                // would be a lie in the first's words: `--load-set` reads the
                // library, so telling a model to load what it just wrote into
                // the sandbox would send it after a file that path cannot see.
                // What it is told instead is where the file is, which is what
                // the operator needs to find it after the show
                // (P-0096, ADR-0261).
                let said = match asked {
                    Asked::Operator => {
                        format!("slot {slot}: saved as set `{id}` — load it with `--load-set {id}`")
                    }
                    Asked::Model => format!(
                        "slot {slot}: saved as set `{id}` in the sandbox — \
                         `<store>/{}/{id}{}`. A save asked for over MCP is kept there \
                         rather than in the operator's library, so `--load-set {id}` does \
                         not reach it; the operator's own `k` writes the library",
                        karakuri_store::store::Store::SANDBOX,
                        karakuri_store::store::Store::SET_FILE_SUFFIX,
                    ),
                };
                eprintln!("{said}");
                self.record_only(karakuri_store::record::Record::Save {
                    slot: DeckSlot(slot as u8),
                    id,
                });
                Ok(said)
            }
            // **Printed, and nothing written.** See `Saved::outcome`.
            Err(e) => {
                let said = format!("slot {slot}: set `{id}` was not saved: {e}");
                eprintln!("{said}");
                Err(said)
            }
        };
        if let Some(reply) = reply {
            reply.settled(said);
        }
    }

    /// Every save still being written, waited for — up to [`SAVE_WAIT`].
    ///
    /// A frame owes the disk nothing, which is why [`finished_saves`] drains and
    /// never blocks. The end of the run is the one moment where that is the wrong
    /// trade: a save pressed in the last second reached the disk under an id
    /// nothing in the stream ever named, so the claim that a `save` record exists
    /// for every live save that reached the disk
    /// (`docs/adr/0120-a-record-may-reach-outside-the-stream.md`) was false in
    /// exactly the window an operator is most likely to be in — press `k`, see it
    /// took, quit.
    ///
    /// Bounded, because a disk can hang and quitting must not depend on one. The
    /// alternative was joining the threads, which is unbounded by construction: a
    /// store on a network mount that stops answering would take the window with it.
    /// Past the bound the run says how many saves it left behind and exits, which
    /// is the same trade the recorder makes when it counts the batches it lost
    /// rather than waiting for them.
    ///
    /// The wait is only ever paid by a run that pressed `k` and quit within a few
    /// frames; the count is zero for every other run and this returns without
    /// blocking.
    pub(crate) fn awaited_saves(&mut self) {
        self.finished_saves();
        if self.saves_in_flight == 0 {
            return;
        }
        eprintln!(
            "waiting up to {:.0}s for {} save{} still being written",
            SAVE_WAIT.as_secs_f32(),
            self.saves_in_flight,
            if self.saves_in_flight == 1 { "" } else { "s" }
        );
        let deadline = Instant::now() + SAVE_WAIT;
        for saved in drained_saves(&self.saves, self.saves_in_flight, deadline) {
            self.took_save(saved);
        }
        if self.saves_in_flight > 0 {
            eprintln!(
                "  {} save{} still unfinished after {:.0}s — each is written or it is not, \
                 and no record claims either way",
                self.saves_in_flight,
                if self.saves_in_flight == 1 { "" } else { "s" },
                SAVE_WAIT.as_secs_f32(),
            );
        }
    }

    /// One governor pass and what it decided, printed.
    ///
    /// Called when something the decision depends on moved — a residency request, a
    /// build landing — and never per frame: it allocates, and the answer cannot
    /// change between those events.
    ///
    /// The parked slots are named individually rather than counted, because each
    /// one is waiting on something different and the reasons call for different
    /// actions: `NoHeadroom` waits for a slot to come off air, `CommittedUnknown`
    /// for a measurement, and `NoPrimingNeeded` for nothing at all — that Set is
    /// closed form and can go straight on air.
    pub(super) fn govern(&mut self, why: &str) {
        report_governing(&self.deck.govern(), why);
    }

    /// Cycle the focused slot's sync mode, skipping the modes its material cannot
    /// take and saying why.
    ///
    /// This is the CLI's form of a control greyed out: there is no widget to dim,
    /// so the unavailable modes are stepped over and the reason is printed with the
    /// result. Silently skipping would leave an operator pressing a key and
    /// watching two of three modes never arrive; printing on every press without
    /// skipping would make the key refuse to do anything at all on material that
    /// only allows one mode.
    ///
    /// Its record goes through [`Live::operate`] like every other settled key's,
    /// and it used to be built here. What kept it out was `Owed::NotSettled` on
    /// [`Operation::SetSync`]: `Transport::engaged` clamps the anchor against the
    /// session tempo, so whether the record carried the tempo that was asked for or
    /// the one the engine settled on read as a decision about the bytes on disk.
    /// The two are the same number — the clamp holds the anchor inside
    /// `karakuri_signal::oscillator::BPM_RANGE` and an oscillator's tempo is
    /// already inside it — so what was missing was never a decision but a reading,
    /// and `Current::tempo` is it. A cycle is still this surface's own:
    /// `Sync::ALL`, the skipping and the refusals are translations a keyboard
    /// makes, and what comes out of them is a destination (P-0090).
    pub(super) fn cycle_sync(&mut self) {
        let slot = self.focus;
        let addr = EngineSlot(slot as u8);
        let current = self.deck.transport(addr).sync();
        let at = Sync::ALL.iter().position(|s| *s == current).unwrap_or(0);

        let mut refused: Vec<String> = Vec::new();
        let mut next = None;
        // Every other mode, in cycle order, starting after the current one.
        for step in 1..=Sync::ALL.len() {
            let candidate = Sync::ALL[(at + step) % Sync::ALL.len()];
            match self.deck.sync_allowed(addr, candidate) {
                Ok(()) => {
                    next = Some(candidate);
                    break;
                }
                Err(refusal) => refused.push(format!("{} — {refusal}", candidate.name())),
            }
        }

        let Some(next) = next else {
            // Unreachable while `Sync::Free` is never refused, and handled
            // rather than unwrapped because that is a property of `allows` and
            // not of this loop.
            eprintln!("slot {slot}: no sync mode is available for this material");
            return;
        };
        if next == current {
            eprintln!(
                "slot {slot}: {} is the only mode this material takes",
                next.name()
            );
        }
        for reason in &refused {
            eprintln!("  skipped {reason}");
        }

        // Read before the operation rather than after it, because `operate`
        // is about to anchor the slot at exactly this and the line below says
        // what the press did.
        let bpm = mix::current_tempo(self.deck.signals().oscillator());
        self.operate(&Operation::SetSync {
            deck: slot as u8,
            sync: mix::sync(next),
        });
        eprintln!(
            "slot {slot} sync {} at {bpm:.1} bpm{}",
            next.name(),
            match next {
                Sync::Free => " — wall time, the room does nothing to it",
                Sync::Tempo => " — 1x here, faster if the room is",
                Sync::Beat => " — locked to the room's position; u/i scrub",
            }
        );
    }

    /// Scrub the focused slot, in beats. The one control that goes backwards.
    pub(super) fn scrub(&mut self, beats: f64) {
        let slot = self.focus;
        let addr = EngineSlot(slot as u8);
        let transport = self.deck.transport(addr);
        if transport.sync() != Sync::Beat {
            eprintln!(
                "slot {slot} is {} — scrubbing moves a position, and only beat sync has one (y)",
                transport.sync().name()
            );
            return;
        }
        self.operate(&Operation::ScrubDeck {
            deck: slot as u8,
            beats,
        });
        eprintln!(
            "slot {slot} scrub {:+.2} beats",
            self.deck.transport(addr).scrub_beats()
        );
    }

    /// A tap on the beat. Authoritative — a performer tapping is stating where the
    /// beat is, not offering evidence — and it goes onto the oscillator through the
    /// same `tempo` record a tracked correction does.
    ///
    /// Not through [`Live::operate`]. [`Operation::TapBeat`] is `Owed::NotSettled`
    /// — what a tap writes is the tracker's answer rather than a value, and the
    /// tracker may refuse — so this is also the one arm [`Live::run_surface`] keeps
    /// for itself. `b` and `note -> tap` are the same function for that reason, and
    /// both move the day the record is settled.
    pub(super) fn tap(&mut self) {
        let started = self.started;
        let mut signals = *self.deck.signals();
        let Some(audio) = self.audio.as_mut() else {
            eprintln!("tap: no audio input — run with --audio-in to tap the beat");
            return;
        };
        let record = audio.tap(&mut signals, Instant::now(), started);
        self.deck.set_signals(signals);
        // **The record, or the tap did not happen as far as the stream is
        // concerned.** `Audio::tap` builds one and applies it; dropping it here
        // left a session whose grid had been moved by a hand with nothing in
        // the timeline to say so, and a replay then ran every `beats`-bound
        // parameter on a different phase. Found the day a control surface made
        // "every control writes a record" a claim rather than a habit.
        self.push_tempo(record);
        eprintln!(
            "tap: {:.1} bpm, phase set",
            self.deck.signals().oscillator().bpm()
        );
    }

    /// Halve or double the grid — the operator's last word on the octave.
    ///
    /// The tracker folds every candidate tempo into a one-octave window centred on
    /// the grid, so an octave error is stable rather than self-correcting: a set
    /// started at 87 for a track that is 174 will track 87 all night. This moves
    /// the grid and the window together, and the picture keeps its phase — doubling
    /// subdivides the beats already there.
    ///
    /// Not through [`Live::operate`], for [`Live::tap`]'s reason:
    /// [`Operation::ScaleGrid`] is `Owed::NotSettled` because moving the grid needs
    /// the beat tracker rather than a value, and the refusal below is the tracker's
    /// to give.
    pub(super) fn shift_octave(&mut self, factor: f32) {
        let mut signals = *self.deck.signals();
        let Some(audio) = self.audio.as_mut() else {
            eprintln!("no audio input — the octave keys move the tracker's window, which only exists with --audio-in");
            return;
        };
        let before = signals.oscillator().bpm();
        match audio.octave(&mut signals, factor) {
            Some(record) => {
                self.deck.set_signals(signals);
                self.push_tempo(record);
                eprintln!(
                    "beat: grid {} to {:.1} bpm — the tracker's window moved with it",
                    if factor > 1.0 { "doubled" } else { "halved" },
                    self.deck.signals().oscillator().bpm()
                );
            }
            // Refused rather than applied and undone two seconds later: outside
            // the range the tracker searches there is nothing to lock to.
            None => eprintln!(
                "beat: {before:.1} bpm {} would leave the trackable range",
                if factor > 1.0 { "doubled" } else { "halved" }
            ),
        }
    }

    /// The offset for everything past the two outputs, which nothing here can
    /// measure. Found from where the audience stands, not from this machine — see
    /// `karakuri-audio`'s crate doc.
    pub(super) fn nudge_latency_offset(&mut self, delta_ms: f32) {
        match self.audio.as_mut() {
            Some(audio) => {
                let ms = audio.nudge_latency_offset(delta_ms);
                // Which way it now points, said in words: the sign is the part
                // an operator gets wrong at 2 a.m., and "-15 ms" alone does not
                // say whether that is the picture waiting or the sound.
                let sense = if ms < 0.0 {
                    "the picture waits for the music"
                } else {
                    "the picture leads the music"
                };
                eprintln!("latency offset {ms:+.0} ms — {sense}");
            }
            None => eprintln!(
                "no audio input — the latency offset only means something with --audio-in"
            ),
        }
    }

    pub(super) fn nudge_gain(&mut self, delta: f32) {
        self.set_gain(
            self.focus,
            self.deck.gain(EngineSlot(self.focus as u8)) + delta,
        );
    }

    pub(super) fn set_gain(&mut self, slot: usize, gain: f32) {
        self.operate(&Operation::SetGain {
            deck: slot as u8,
            gain: clamp_gain(gain),
        });
        eprintln!(
            "slot {slot} gain {:.2}",
            self.deck.gain(EngineSlot(slot as u8))
        );
    }

    /// The fader, and the one control that silences a slot under every blend mode —
    /// which makes it the way out of material that has gone NaN. `[` and `]` move
    /// the *level*, and under `over` a level of zero is a black card that still
    /// covers what is beneath it.
    ///
    /// The mode is printed with the number because what the number does depends on
    /// it: under `add` opacity and gain are the same dial twice.
    pub(super) fn nudge_opacity(&mut self, delta: f32) {
        let slot = self.focus;
        self.set_opacity(slot, self.deck.opacity(EngineSlot(slot as u8)) + delta);
    }

    pub(super) fn set_opacity(&mut self, slot: usize, value: f32) {
        let opacity = value.clamp(0.0, 1.0);
        self.operate(&Operation::SetOpacity {
            deck: slot as u8,
            opacity,
        });
        let addr = EngineSlot(slot as u8);
        eprintln!(
            "slot {slot} opacity {:.2} ({})",
            self.deck.opacity(addr),
            self.deck.blend(addr).name()
        );
    }

    /// Fade the focused slot's fader to `to`, over the current length, starting on
    /// the current quantum.
    ///
    /// Opacity rather than gain, because opacity is the fader: it silences a slot
    /// under every blend mode, where a gain of zero under `over` is a black card
    /// that still covers. A gain fade is reachable through the record and
    /// deliberately has no key — two keys that look alike and differ only under one
    /// blend mode is how an operator ends up fading the wrong one in the dark.
    /// `Operation::FadeDeck` says the same thing at its own definition, which is
    /// why the choice is not repeated in the record this writes.
    ///
    /// One `operate` call, and [`Live::fade_slot`] is gone. This built its record
    /// where it stood while the quantum and the length had no owner; they are the
    /// surface's, they are handed over as
    /// `karakuri_operation_record::Current::transition`, and what this key writes
    /// is `written`'s answer like every other settled key's.
    pub(super) fn fade(&mut self, to: f32) {
        let slot = self.focus;
        self.operate(&Operation::FadeDeck {
            deck: slot as u8,
            to,
        });
        eprintln!(
            "slot {slot} fading to {to:.2} over {} beat{} from {}",
            self.fade_beats,
            if self.fade_beats == 1.0 { "" } else { "s" },
            self.quantum_name()
        );
    }

    /// A crossfade: the focused slot out and the next one in, together.
    ///
    /// Two scheduled moves rather than a `Crossfade` object, which is the whole
    /// argument of `karakuri_engine::transition` seen from the keyboard — the
    /// first-class thing is the move, and every gesture anyone names is made of
    /// those. They share a start and a length, so they are one gesture without
    /// being one type.
    ///
    /// The incoming slot is put at silence and then on air, in that order, and both
    /// halves of that are load-bearing. A slot comes up at full opacity and going
    /// off air does not lower it, so putting one on air without silencing it first
    /// shows it at full immediately — up to a bar before the fade it is supposed to
    /// arrive on, which is a cut with a decorative fade attached. And a fade to
    /// something that is not being composited is a fade to black, so it does have
    /// to go on air.
    ///
    /// The silencing is a `opacity` record like any other, so it cancels nothing
    /// the operator wanted and replays like anything else.
    ///
    /// All four of its records are one operation now, and this function is one
    /// `operate` call — which is the sentence that used to be here as a promise. It
    /// read *"the day that is settled this function is one `operate` call"*, and
    /// the day was the one the quantum and the length got an owner: they are the
    /// surface's, `karakuri_operation_record::Current::transition` is where this
    /// program hands them over, and `written` answers `Operation::Crossfade` with
    /// the four records in the order above.
    ///
    /// The two remaining lines are the keyboard's translation and not the gesture.
    /// *The next deck* is what `x` means here and the operation names both decks,
    /// so working out which one and refusing a deck with nowhere to go stays;
    /// everything past that is the conversion's.
    ///
    /// The put-on-air is written even where the slot is already live, which is the
    /// one thing this changed about the stream. It used to be conditional on
    /// `Deck::residency`, and the conversion has no deck to ask:
    /// `Operation::Crossfade` says four records at its own definition, and a
    /// `residency` record for a slot that is already live decodes to a state it is
    /// already in.
    pub(super) fn crossfade(&mut self) {
        let from = self.focus;
        let to = (from + 1) % self.deck.slot_count();
        if to == from {
            eprintln!("crossfade needs somewhere to go — this deck holds one slot");
            return;
        }
        self.operate(&Operation::Crossfade {
            from: from as u8,
            to: to as u8,
        });
        eprintln!(
            "crossfade {from} to {to} over {} beat{} from {}",
            self.fade_beats,
            if self.fade_beats == 1.0 { "" } else { "s" },
            self.quantum_name()
        );
    }

    /// Wipe the next slot in over the focused one.
    ///
    /// A mask and one scheduled move, and that is the whole of it: the incoming
    /// slot is given the current shape at position 0 — revealing nothing — put on
    /// air under `over` so that what it reveals *hides* what is beneath, and then
    /// one transition carries the front from 0 to 1. Nothing in the transition
    /// system knows what a mask is and nothing in the mask knows what a beat is.
    ///
    /// All six of its records are one operation now, and this function is one
    /// `operate` call — six where nothing is already where the wipe is putting it,
    /// and four or five where something is — which is [`Live::crossfade`]'s
    /// paragraph one gesture along and the last of them to be written. It built
    /// five of the six out of five separate operations and the sixth by hand, while
    /// what a wipe owed had no owner: the *shape* its front takes and the soft
    /// edge. Both have one. The shape is `Operation::SetTransition`'s third setting
    /// — the one this program holds in `mask_kind` and `mask_angle` and the `z` key
    /// writes — so it goes over with the quantum and the length inside
    /// `karakuri_operation_record::Current::transition`, which is where its two
    /// neighbours already were. The soft edge is read off the mask on the deck
    /// being wiped in, which is `mix::current_mask` and was already the reading
    /// `Operation::SetMaskShape` takes.
    ///
    /// The two lines that are left are the keyboard's translation and not the
    /// gesture. *The next slot* is what `c` means here and the operation names both
    /// decks, so working out which one and refusing a deck with nowhere to go
    /// stays. So does the refusal with no shape chosen: `Operation::Wipe` says
    /// *"Refused with no shape chosen"* at its own definition, `written` has no
    /// answer that is a refusal, and the shape is this surface's own setting — so
    /// this is the only place that can turn a wipe with nothing to move away, and
    /// it does it before it asks.
    ///
    /// The one decision this function used to make is made in the conversion now,
    /// and it is not a record. Under `add` the same gesture is a wipe *on* rather
    /// than a wipe *over*, which is a different picture and a legitimate one — so
    /// the mode is left wherever the operator had it and `over` is written only
    /// where the slot is still at the mode a slot starts in, which is what makes
    /// `m` in front of `c` mean something. The put-on-air is written only where the
    /// slot is not already live, on the same terms. Those were two `if`s here and
    /// they are the `Wipe` arm's now, because the sentence belongs beside the
    /// records it governs rather than on one of the surfaces that can reach them
    /// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
    /// — a rule held in one surface binds none of the other three). What the
    /// conversion needed to keep it is a reading, which is the third this gesture
    /// takes: `karakuri_operation_record::Current::mix`, handed over by
    /// [`Live::operate`] out of the deck this function no longer touches.
    ///
    /// Six records where it once wrote five, and the extra one is what routing the
    /// mask honestly costs rather than an accident: the shape and the front are two
    /// operations
    /// (`docs/adr/0201-the-mask-is-two-rows-because-a-control-change-can-only-set.md`)
    /// and each of them writes a whole `Record::Mask`, because the record is a
    /// state and not an ask. The pair lands in that order, so the front is at 0
    /// when the move is scheduled — which is the same picture the one hand-built
    /// record made.
    pub(super) fn wipe(&mut self) {
        let under = self.focus;
        let over = (under + 1) % self.deck.slot_count();
        if over == under {
            eprintln!("a wipe needs somewhere to come from — this deck holds one slot");
            return;
        }
        if self.mask_kind == MaskKind::None {
            eprintln!("no mask shape — `z` chooses one, and a wipe is a shape moving");
            return;
        }
        self.operate(&Operation::Wipe {
            from: under as u8,
            to: over as u8,
        });
        eprintln!(
            "wipe {over} over {under} — {} at {:.0}°, over {} beat{} from {}",
            self.mask_kind.name(),
            self.mask_angle.to_degrees(),
            self.fade_beats,
            if self.fade_beats == 1.0 { "" } else { "s" },
            self.quantum_name()
        );
    }

    /// The shape the next wipe uses, and which way it runs.
    ///
    /// One key for both, because the shapes and the angles an operator actually
    /// reaches for are a short list rather than two dials: across, up, the two
    /// diagonals, and an iris. A continuous dial for the angle is not built: there
    /// is nowhere on this surface to see one.
    pub(super) fn cycle_mask(&mut self) {
        let at = MASK_SHAPES
            .iter()
            .position(|(k, a, _)| *k == self.mask_kind && *a == self.mask_angle)
            .unwrap_or(0);
        let (kind, angle, name) = MASK_SHAPES[(at + 1) % MASK_SHAPES.len()];
        self.mask_kind = kind;
        self.mask_angle = angle;
        eprintln!("wipes are {name}");
    }

    /// Choose which renderer of the focused slot is the live one, on the grid.
    ///
    /// The first half of a variant pool, and the half that is true today: several
    /// renderers over *one* simulation, in one Set, one of them folded into the
    /// picture at a time. A pool spanning deck slots that differ at a layer slot
    /// was the alternative and is rejected — see
    /// `docs/adr/0148-a-variant-pool-is-a-set-and-the-deck-stays-a-mixer.md` — and
    /// the rest of it — an alternative that differs at L1, priming it off air,
    /// sharing the geometry between them — is not built. See the manual, "Selecting
    /// one renderer of a slot".
    ///
    /// What it does not save. The renderers that are not selected go on drawing,
    /// each into a target of its own: at 1280x720 that is 7.03 MB and a render pass
    /// apiece, every frame, whether or not anybody is looking at them. That is what
    /// makes the selection a uniform write and a cut on the beat rather than a
    /// build — cheap, and not free. The line printed says how many are still
    /// drawing for exactly that reason.
    ///
    /// Cycling from the selection that is armed, not from the one on screen. A
    /// press lands up to a bar later, so two presses inside a bar read the same
    /// live edge and would both choose the same renderer — the second press would
    /// do nothing and say it had. `Deck::selections_on` is what makes the second
    /// press mean "the one after that".
    ///
    /// One way, and it is stated rather than discovered: there is no position in
    /// the cycle that puts every renderer back. A Set comes up with all of them
    /// folded — or folded to the one its Set file's `merge` record named, which is
    /// the other way to start — and the first press leaves that state for good,
    /// which is what "makes one live and the rest not" costs when the record names
    /// one renderer. Restoring the fold is a different statement and wants its own
    /// vocabulary — an `Option` carrying `null` for "all of them" is the shape it
    /// would take, which is the shape the retired `preview` record had.
    ///
    /// What it chooses is kept. A live save reads the fold off the Set — see
    /// `playing_values` — so `k` after a press writes `{"t":"merge", "live":N}` and
    /// loading that file back comes up on renderer N.
    ///
    /// Its record comes out of [`Live::operate`], which it did not while the
    /// instant a selection lands on had nobody to supply it: the quantum is this
    /// program's setting, `mix::current_transition` is where it becomes an instant,
    /// and `written` writes the `select` record from it. Which renderer to move to
    /// is the keyboard's own translation and stays here, which is the whole of what
    /// is left of this function's arithmetic.
    pub(super) fn cycle_renderer(&mut self) {
        let slot = self.focus;
        let addr = EngineSlot(slot as u8);
        let set = self.deck.slot(addr).set();
        let count = set.inputs().len();
        let composited = set.layering() == karakuri_engine::set::Layering::Composite;
        // The live edges, read before anything is scheduled: with nothing
        // selected yet every one of them is live, which is the state a Set
        // builds in.
        let live: Vec<usize> = set
            .inputs()
            .iter()
            .enumerate()
            .filter(|(_, input)| input.live)
            .map(|(at, _)| at)
            .collect();
        if count < 2 {
            eprintln!(
                "slot {slot} draws with one renderer — there is nothing to choose between. \
                 A second `.kir` on the `--set` for this slot is what makes a choice"
            );
            return;
        }
        if !composited {
            eprintln!(
                "slot {slot} overdraws its {count} renderers — they share one target and \
                 meet through their own blend states, so there is no edge to silence. \
                 `--merge {slot}` gives each a target of its own"
            );
            return;
        }
        let armed = self.deck.selections_on(addr).next().map(|s| s.renderer());
        let next = match (armed, live.as_slice()) {
            // The one waiting to land, so a second press inside the same bar
            // moves past it rather than choosing it again.
            (Some(at), _) => (at + 1) % count,
            // Exactly one live is a selection that has landed.
            (None, [at]) => (at + 1) % count,
            // Every renderer live, which is what a Set comes up as, or none of
            // them, which nothing here produces: start at the first.
            (None, _) => 0,
        };
        self.operate(&Operation::SelectRenderer {
            deck: slot as u8,
            renderer: next as u32,
        });
        eprintln!(
            "slot {slot} renderer {next} of {count} from {} — {}",
            self.quantum_name(),
            // The cost, said on every press rather than in the manual alone: a
            // selection that reads as free is one an operator will reach for
            // where a rebuild was wanted.
            if count == 2 {
                "the other one still draws into a target of its own".to_string()
            } else {
                format!(
                    "the other {} still draw into targets of their own",
                    count - 1
                )
            }
        );
    }

    /// Where a scheduled move starts: now, the next beat, or the next bar.
    pub(super) fn cycle_quantum(&mut self) {
        let at = QUANTA
            .iter()
            .position(|(q, _)| *q == self.quantum)
            .unwrap_or(0);
        self.quantum = QUANTA[(at + 1) % QUANTA.len()].0;
        eprintln!("fades start {}", self.quantum_name());
    }

    /// How long a scheduled move lasts.
    pub(super) fn cycle_fade_beats(&mut self) {
        let at = FADE_BEATS
            .iter()
            .position(|b| *b == self.fade_beats)
            .unwrap_or(0);
        self.fade_beats = FADE_BEATS[(at + 1) % FADE_BEATS.len()];
        eprintln!(
            "fades last {} beat{}",
            self.fade_beats,
            if self.fade_beats == 1.0 { "" } else { "s" }
        );
    }

    pub(super) fn quantum_name(&self) -> &'static str {
        QUANTA
            .iter()
            .find(|(q, _)| *q == self.quantum)
            .map(|(_, name)| *name)
            .unwrap_or("now")
    }

    /// Cycle the focused slot's blend mode. No refusals here — unlike sync, every
    /// mode is available to every slot, because a blend mode is a question about
    /// pixels and not about what the material can do.
    pub(super) fn cycle_blend(&mut self, slot: usize) {
        let addr = EngineSlot(slot as u8);
        let current = self.deck.blend(addr);
        let at = Blend::ALL.iter().position(|b| *b == current).unwrap_or(0);
        let next = Blend::ALL[(at + 1) % Blend::ALL.len()];
        self.operate(&Operation::SetBlendMode {
            deck: slot as u8,
            blend: mix::blend_mode(next),
        });
        eprintln!(
            "slot {slot} blend {} — gain {:.2}, opacity {:.2}",
            self.deck.blend(addr).name(),
            self.deck.gain(addr),
            self.deck.opacity(addr)
        );
    }

    pub(super) fn cycle_tonemap(&mut self) {
        self.operate(&Operation::SetTonemap {
            tonemap: mix::tonemap(next_tonemap(self.look.op)),
        });
        eprintln!(
            "tonemap {} (exposure {:.2})",
            op_name(self.look.op),
            self.look.exposure
        );
    }

    pub(super) fn set_exposure(&mut self, exposure: f32) {
        self.operate(&Operation::SetExposure {
            exposure: clamp_exposure(exposure),
        });
        eprintln!(
            "exposure {:.3} ({})",
            self.look.exposure,
            op_name(self.look.op)
        );
    }

    /// Every mix change goes through here, and here goes through a record.
    ///
    /// Built, decoded, and only then applied — so what drives the deck is what a
    /// replay would decode from a session stream, rather than a second path that
    /// happens to agree with it today. `karakuri-environment`'s `audio.rs` does the
    /// same thing with the two records it emits; see the program's `mix.rs` for the
    /// whole argument.
    ///
    /// A record this build cannot obey is printed and nothing moves. It cannot
    /// happen from a key press — every caller here built the record a moment ago
    /// out of the engine's own types — and it is handled rather than unwrapped
    /// because the replay driver will hand this same function lines off a file, and
    /// a file is where an unobeyable record comes from. A tempo correction into the
    /// stream.
    ///
    /// Separate from [`Live::record`] because a `tempo` is applied where it is
    /// decided rather than read back — the oscillator is moved by the code that
    /// worked out how far, and `apply_replayed` is what re-applies it on the way
    /// back. What this owes is the *writing*, and it is one function so that a
    /// third thing moving the grid cannot forget it: two already had.
    ///
    /// Scalars only, so pushing it allocates nothing, which is what lets the frame
    /// path call it as well as the two keys.
    pub(super) fn push_tempo(&mut self, record: karakuri_store::record::Record) {
        if let Some(recorder) = &mut self.recorder {
            recorder.push(record);
        }
    }

    /// A surface's operation, as the records it writes — and then written.
    ///
    /// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
    /// puts every control at the same record, and this is where a key press ends:
    /// the operation is named, `karakuri-operation-record` says what it writes, and
    /// [`Live::record`] writes it and reads it back the way every other record here
    /// is read back. A console fader and a mapped MIDI control are the same thing
    /// exactly because they arrive at this function carrying the same name.
    ///
    /// The reading is taken here and nowhere else. The conversion is not pure —
    /// `SetExposure` becomes a `look` record carrying the operator and the white
    /// point too — so what is running has to be read back and handed over, and this
    /// is the only place in this program that knows both what was asked for and
    /// what is on screen.
    ///
    /// The two answers that are not records are printed rather than swallowed,
    /// which is what this wrapper is: a key press and a mapped control have nobody
    /// waiting on an answer, and an operator pressing a key that does nothing
    /// deserves the sentence.
    ///
    /// A model does have somebody waiting, so [`Live::run_operations`] calls
    /// [`Live::performed`] instead and hands the same sentence back as the call's
    /// error. The words are one string built in one place ([`answered`]), which is
    /// [`refused`]'s rule: a copy of them for the second audience is free to be
    /// right on the day it is written and wrong at the next correction.
    pub(super) fn operate(&mut self, operation: &Operation) {
        if let Err(said) = self.performed(operation) {
            eprintln!("{said}");
        }
    }

    /// [`Live::operate`], with the answer handed back rather than printed.
    ///
    /// `Ok` means the records were written and read back, which is the whole of
    /// what performing an operation is on this surface. `Err` is the sentence
    /// [`answered`] built, and it means nothing on this run changed — see there for
    /// why that is a refusal rather than a line on a terminal somebody may not be
    /// reading.
    pub(super) fn performed(&mut self, operation: &Operation) -> Result<(), String> {
        let transport = match operation {
            Operation::ScrubDeck { deck, .. } => EngineSlot::new(*deck, self.deck.slot_count())
                .map(|slot| mix::current_transport(self.deck.transport(slot))),
            _ => None,
        };
        // The mask of the deck the operation names, on the transport's terms:
        // both halves of a mask write the record whole, so each of them needs
        // the half it did not ask for.
        // A wipe names two decks and is read for one of them: everything it
        // writes is about the deck arriving, so the mask handed over is
        // `to`'s. The soft edge is the only field of it a wipe does not say,
        // and giving it the covered deck's would put somebody else's edge on
        // the front that is about to cross the frame.
        let mask = match operation {
            Operation::SetMaskShape { deck, .. }
            | Operation::SetMaskPosition { deck, .. }
            | Operation::Wipe { to: deck, .. } => EngineSlot::new(*deck, self.deck.slot_count())
                .map(|slot| mix::current_mask(self.deck.mask(slot))),
            _ => None,
        };
        // **The session tempo, for the one operation anchored to it.**
        // Engaging a mode anchors the slot at the tempo the room is going at,
        // and this reading is what `written` builds `Record::Transport` from —
        // `mix::current_tempo` takes the oscillator rather than a number, so
        // nothing here can hand in a tempo the session never ran at.
        let tempo = match operation {
            Operation::SetSync { .. } => Some(mix::current_tempo(self.deck.signals().oscillator())),
            _ => None,
        };
        // **The transition settings, for the four operations that schedule a
        // move**, and the one reading here that is read off nothing: `z`, `n`
        // and `j` set the shape, the quantum and the length, no record carries
        // any of them, and they are this program's own state until it hands
        // them over. That is the whole of what settling these four
        // conversions decided — see `karakuri_operation_record::Transition`.
        //
        // **The shape is the third setting and the wipe is the only reader.**
        // `mask_kind` and `mask_angle` are what `z` cycles, they are the shape
        // the *next* wipe takes rather than the shape any slot is wearing, and
        // handing them over here is what stopped `Live::wipe` building a
        // record of its own.
        //
        // `mix::current_transition` takes the oscillator and the quantum
        // rather than an instant, so the start is
        // `karakuri_engine::transition::quantise`'s answer and this file
        // cannot hand in a beat the grid was never on.
        // **Where the deck a wipe is arriving on already sits in the mix**,
        // and the one reading here taken so that a record can be left *out*.
        // A wipe puts that deck under `over` and on air; this program is what
        // knows the deck is already there, and the conversion is where the two
        // records that would say so again are dropped. `c` used to make that
        // decision here, in the two `if`s that are gone — the mode is the
        // operator's, and `m` in front of `c` is what it buys.
        //
        // **The deck as it reports, which is what the governor may have held
        // below the request.** `mix::current_mix` takes the engine's two
        // values rather than the vocabulary's, so the crossing is made in one
        // place, exactly as the mask's and the transition's are.
        let mix = match operation {
            Operation::Wipe { to: deck, .. } => EngineSlot::new(*deck, self.deck.slot_count())
                .map(|slot| mix::current_mix(self.deck.blend(slot), self.deck.residency(slot))),
            _ => None,
        };
        let transition = match operation {
            Operation::FadeDeck { .. }
            | Operation::Crossfade { .. }
            | Operation::SelectRenderer { .. }
            | Operation::Wipe { .. } => Some(mix::current_transition(
                self.deck.signals().oscillator(),
                self.quantum,
                self.fade_beats,
                mix::FADE_CURVE,
                self.mask_kind,
                self.mask_angle,
            )),
            _ => None,
        };
        let current = Current {
            look: Some(mix::current_look(&self.look)),
            // **The chain that is running, read off the `Present` that holds
            // it** — `Engine::look`'s seam one pass along, and unconditional
            // for the look's reason: this surface has one of each and asking
            // which operation wants which would be a second list to keep in
            // step with `written`'s.
            //
            // **No key reaches the three master rows on this surface**, so
            // nothing here writes one today; it is handed in all the same,
            // because what decides whether a chain operation can be answered
            // is whether the reading was taken and not which surface asked
            // (ADR-0317, and `written`'s `Owed::NotRead`).
            master_chain: Some(mix::current_chain(&self.present.chain_spec())),
            transport,
            mask,
            tempo,
            transition,
            mix,
        };
        for record in answered(operation, &current)? {
            self.record(record);
        }
        Ok(())
    }
}
