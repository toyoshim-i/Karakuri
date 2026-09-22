use super::super::*;
use super::constants::*;

impl Live {
    pub(crate) fn nudge_gain(&mut self, delta: f32) {
        self.set_gain(
            self.focus,
            self.deck.gain(EngineSlot(self.focus as u8)) + delta,
        );
    }

    pub(crate) fn set_gain(&mut self, slot: usize, gain: f32) {
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
    pub(crate) fn nudge_opacity(&mut self, delta: f32) {
        let slot = self.focus;
        self.set_opacity(slot, self.deck.opacity(EngineSlot(slot as u8)) + delta);
    }

    pub(crate) fn set_opacity(&mut self, slot: usize, value: f32) {
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
    pub(crate) fn fade(&mut self, to: f32) {
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
    pub(crate) fn crossfade(&mut self) {
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
    pub(crate) fn wipe(&mut self) {
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
    pub(crate) fn cycle_mask(&mut self) {
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
    pub(crate) fn cycle_renderer(&mut self) {
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
    pub(crate) fn cycle_quantum(&mut self) {
        let at = QUANTA
            .iter()
            .position(|(q, _)| *q == self.quantum)
            .unwrap_or(0);
        self.quantum = QUANTA[(at + 1) % QUANTA.len()].0;
        eprintln!("fades start {}", self.quantum_name());
    }

    /// How long a scheduled move lasts.
    pub(crate) fn cycle_fade_beats(&mut self) {
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

    pub(crate) fn quantum_name(&self) -> &'static str {
        QUANTA
            .iter()
            .find(|(q, _)| *q == self.quantum)
            .map(|(_, name)| *name)
            .unwrap_or("now")
    }

    /// Cycle the focused slot's blend mode. No refusals here — unlike sync, every
    /// mode is available to every slot, because a blend mode is a question about
    /// pixels and not about what the material can do.
    pub(crate) fn cycle_blend(&mut self, slot: usize) {
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

    pub(crate) fn cycle_tonemap(&mut self) {
        self.operate(&Operation::SetTonemap {
            tonemap: mix::tonemap(next_tonemap(self.look.op)),
        });
        eprintln!(
            "tonemap {} (exposure {:.2})",
            op_name(self.look.op),
            self.look.exposure
        );
    }

    pub(crate) fn set_exposure(&mut self, exposure: f32) {
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
    pub(crate) fn operate(&mut self, operation: &Operation) {
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
    pub(crate) fn performed(&mut self, operation: &Operation) -> Result<(), String> {
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
        // Transition settings and current mix state for operations scheduling moves
        // (FadeDeck, Crossfade, SelectRenderer, Wipe).
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
            // **Which lanes hold which controls, and on this program the
            // answer is none of them**: there is no sequencer here, no pattern
            // and no bank, so nothing can be holding a fader when a move is
            // scheduled. It is a reading that was taken and is empty rather
            // than `None`, which is a reading nobody took — the two behave
            // alike today and say different things, and this one is the true
            // one (ADR-0323).
            lanes: Some(karakuri_operation_record::Lanes::default()),
        };
        for record in answered(operation, &current)? {
            self.record(record);
        }
        Ok(())
    }
}
