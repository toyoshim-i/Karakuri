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

    /// Nudges opacity of the focused slot by `delta`.
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

    /// Fades the focused slot's opacity to `to` over current fade duration.
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

    /// Crossfades from the focused slot to the next slot over current fade duration.
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

    /// Initiates a wipe transition from the focused slot to the next slot.
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

    /// Cycles to the next available wipe mask shape and angle.
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

    /// Cycles the active renderer selection for the focused slot on the quantization grid.
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

    /// Converts and applies an operation to the deck via record emission.
    pub(crate) fn operate(&mut self, operation: &Operation) {
        if let Err(said) = self.performed(operation) {
            eprintln!("{said}");
        }
    }

    /// Executes an operation by translating it to records, returning errors on failure.
    pub(crate) fn performed(&mut self, operation: &Operation) -> Result<(), String> {
        let transport = match operation {
            Operation::ScrubDeck { deck, .. } => EngineSlot::new(*deck, self.deck.slot_count())
                .map(|slot| mix::current_transport(self.deck.transport(slot))),
            _ => None,
        };
        // Capture current mask state for mask and wipe operations.
        let mask = match operation {
            Operation::SetMaskShape { deck, .. }
            | Operation::SetMaskPosition { deck, .. }
            | Operation::Wipe { to: deck, .. } => EngineSlot::new(*deck, self.deck.slot_count())
                .map(|slot| mix::current_mask(self.deck.mask(slot))),
            _ => None,
        };
        // Read current session tempo for sync operations.
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
            // Master chain spec read from presentation pass.
            master_chain: Some(mix::current_chain(&self.present.chain_spec())),
            transport,
            mask,
            tempo,
            transition,
            mix,
            // Empty lanes as CLI does not feature a pattern sequencer.
            lanes: Some(karakuri_operation_record::Lanes::default()),
        };
        for record in answered(operation, &current)? {
            self.record(record);
        }
        Ok(())
    }
}
