use super::super::*;

impl Live {
    /// Evaluates governor rules and logs any changes in slot readiness or status.
    pub(crate) fn govern(&mut self, why: &str) {
        report_governing(&self.deck.govern(), why);
    }

    /// Cycles the focused slot's sync mode through supported options, skipping disallowed modes.
    pub(crate) fn cycle_sync(&mut self) {
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
    pub(crate) fn scrub(&mut self, beats: f64) {
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

    /// Manually taps the tempo beat, updating the audio tracker and signals grid.
    pub(crate) fn tap(&mut self) {
        let started = self.started;
        let mut signals = *self.deck.signals();
        let Some(audio) = self.audio.as_mut() else {
            eprintln!("tap: no audio input — run with --audio-in to tap the beat");
            return;
        };
        let record = audio.tap(&mut signals, Instant::now(), started);
        self.deck.set_signals(signals);
        self.push_tempo(record);
        eprintln!(
            "tap: {:.1} bpm, phase set",
            self.deck.signals().oscillator().bpm()
        );
    }

    /// Halves or doubles the grid tempo and tracking window.
    pub(crate) fn shift_octave(&mut self, factor: f32) {
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
            None => eprintln!(
                "beat: {before:.1} bpm {} would leave the trackable range",
                if factor > 1.0 { "doubled" } else { "halved" }
            ),
        }
    }

    /// Nudges external latency offset (audio vs video presentation timing).
    pub(crate) fn nudge_latency_offset(&mut self, delta_ms: f32) {
        match self.audio.as_mut() {
            Some(audio) => {
                let ms = audio.nudge_latency_offset(delta_ms);
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

    /// Pushes a tempo change record directly to the recorder.
    pub(crate) fn push_tempo(&mut self, record: karakuri_store::record::Record) {
        if let Some(recorder) = &mut self.recorder {
            recorder.push(record);
        }
    }
}
