use super::super::*;

impl Live {
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
    pub(crate) fn govern(&mut self, why: &str) {
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

    /// A tap on the beat. Authoritative — a performer tapping is stating where the
    /// beat is, not offering evidence — and it goes onto the oscillator through the
    /// same `tempo` record a tracked correction does.
    ///
    /// Not through [`Live::operate`]. [`Operation::TapBeat`] is `Owed::NotSettled`
    /// — what a tap writes is the tracker's answer rather than a value, and the
    /// tracker may refuse — so this is also the one arm [`Live::run_surface`] keeps
    /// for itself. `b` and `note -> tap` are the same function for that reason, and
    /// both move the day the record is settled.
    pub(crate) fn tap(&mut self) {
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
    pub(crate) fn nudge_latency_offset(&mut self, delta_ms: f32) {
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
    pub(crate) fn push_tempo(&mut self, record: karakuri_store::record::Record) {
        if let Some(recorder) = &mut self.recorder {
            recorder.push(record);
        }
    }
}
