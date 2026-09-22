use std::fmt::Write;
use std::time::Instant;

use super::*;

impl Live {
    /// What the operator needs and nothing that costs a stall to know: which slots
    /// are on air, what they are faded to, where their simulation clocks are, the
    /// output look, and whether frames are still arriving on time.
    ///
    /// Element live counts are deliberately absent — `Set::live_count` blocks until
    /// the queue drains, so a status line carrying it would put a GPU sync on the
    /// render thread twice a second. It is printed once, at exit.
    pub(super) fn print_status(&mut self) {
        let elapsed = self.status_at.elapsed().as_secs_f32();
        let fps = if elapsed > 0.0 {
            self.frames_since_status as f32 / elapsed
        } else {
            0.0
        };
        self.status_at = Instant::now();
        self.frames_since_status = 0;

        self.status.clear();
        for slot in 0..self.deck.slot_count() {
            let addr = EngineSlot(slot as u8);
            let _ = write!(
                self.status,
                "{}{slot} {} g{:.2} t{:.1}s ",
                if slot == self.focus { ">" } else { " " },
                residency_tag(self.deck.residency(addr), self.deck.is_parked(addr)),
                self.deck.gain(addr),
                self.deck.slot(addr).set().time()
            );
            // **The slot has stopped updating, and only while it has.** The
            // version in it costs more than one frame may, so the engine skips
            // its step and its draw and it holds the frame it last drew
            // (ADR-0316). Printed on the same terms as the transport and the
            // fader below — a run in which no slot is stopped prints the line
            // it always printed — and printed *whole*, not as a four-letter
            // column beside the residency: it is not a residency, a stopped
            // slot is still mixed, and `t` standing still beside `LIVE` is
            // exactly the reading an operator would otherwise take for a bug.
            self.status
                .push_str(stopped_tag(self.deck.overloaded(addr)));
            // **What is moving, and where it is going.** An armed fade is
            // invisible otherwise: with the default quantum it is due up to a
            // bar after the key, and the only thing that said so was one line
            // at press time. A control that changes something invisible is
            // indistinguishable from a control that is broken, which is this
            // file's own argument for printing on every key.
            for t in self.deck.transitions_on(addr) {
                let _ = write!(
                    self.status,
                    "{}>{:.2} ",
                    match t.control() {
                        karakuri_engine::transition::Control::Gain => "g",
                        karakuri_engine::transition::Control::Opacity => "o",
                        karakuri_engine::transition::Control::MaskPosition => "w",
                    },
                    t.to()
                );
            }
            // **And what is waiting to be chosen**, on the same terms and for
            // the same reason: a selection armed for the next bar is invisible
            // between the key and the music, and `r>1` is the only thing on
            // screen that says a renderer is about to change.
            for selection in self.deck.selections_on(addr) {
                let _ = write!(self.status, "r>{} ", selection.renderer());
            }
            // Omit opacity and blend mode when at defaults (opacity 1.0, Add).
            if self.deck.opacity(addr) != 1.0 {
                let _ = write!(self.status, "o{:.2} ", self.deck.opacity(addr));
            }
            if self.deck.blend(addr) != Blend::Add {
                let _ = write!(self.status, "{} ", self.deck.blend(addr).name());
            }
            // Display transport status only when active (non-free).
            let transport = self.deck.transport(addr);
            match transport.sync() {
                Sync::Free => {}
                Sync::Tempo => {
                    let _ = write!(self.status, "T{:.0} ", transport.anchor_bpm());
                }
                Sync::Beat => {
                    let _ = write!(
                        self.status,
                        "B{:.0}{} ",
                        transport.anchor_bpm(),
                        if transport.scrub_beats() == 0.0 {
                            String::new()
                        } else {
                            format!("{:+.2}", transport.scrub_beats())
                        }
                    );
                }
            }
            // The level, which is why the meter exists: two Sets are matched
            // on `m` and `p` warns which one will dominate the mix wherever it
            // lands regardless of its fader. `None` for an off-air slot is the
            // meter saying it has nothing current rather than showing the last
            // thing the slot drew — see `Deck::level`.
            match self.deck.level(addr) {
                Some(level) => {
                    let _ = write!(self.status, "m{:.3} p{:.1}", level.mean, level.peak);
                    // Texels the meter left out because they were not a finite
                    // number, and **only when there are any**. Not a warning:
                    // dividing by a value that reaches zero is an ordinary
                    // thing for a shader to do and what it produces is a
                    // blown-out pixel. It is here because it explains the two
                    // numbers beside it — they are a mean and a peak over the
                    // texels this did not count — and because 3 and 300000 are
                    // a stray sprite and a frame that is gone.
                    if level.bad_texels > 0 {
                        let _ = write!(self.status, " x{}", level.bad_texels);
                    }
                    self.status.push_str("  ");
                }
                None => self.status.push_str("m---- p----  "),
            }
            // What every binding on this slot last wrote. Without it a binding
            // that is doing nothing — an unknown signal, or an invented one at
            // a tenth effect — is indistinguishable from a binding that never
            // attached, which is the whole reason confidence is worth seeing
            // rather than merely being applied. Nothing is printed for a slot
            // with no bindings, so the default status line is unchanged.
            for (key, value) in self.deck.slot(addr).set().bound() {
                let _ = write!(self.status, "{key}={value:.3}  ");
            }
        }
        // What audio is doing, when there is any. A performer cannot tune what
        // they cannot see: the confidence says whether the input is alive, and
        // the phase error says which way the offset wants nudging.
        // **Where the grid is coming from, before what it currently is.** The
        // number an operator needs to trust is the tempo; the thing that tells
        // them whether to trust it is its source, and until now there was only
        // ever one. `link 2p` is a shared grid with two other peers on it;
        // `link 0p` is a source running and alone, which is a different
        // situation from no source at all and has to look different.
        if let Some(source) = &self.tempo_source {
            let _ = write!(
                self.status,
                "| {} {}p{}{} ",
                source.name(),
                source.peers(),
                // **Counted, not swallowed.** Anchors thrown away for
                // unusable numbers mean a source that has gone wrong, and
                // nothing else would ever say so.
                match source.rejected() {
                    0 => String::new(),
                    n => format!(" x{n}"),
                },
                match (source.ended().is_some(), source.playing()) {
                    (true, _) => " GONE",
                    // **Three states and not two.** A source that has greeted
                    // and said nothing else is not a stopped source, and the
                    // two used to print the same thing — which is the pair an
                    // operator would act differently on. Shown and not acted
                    // on, like every other measurement here.
                    (false, None) => " ?",
                    (false, Some(false)) => " stop",
                    (false, Some(true)) => "",
                }
            );
            // **The tempo, when nothing else is going to print it.** The
            // grid's bpm has always lived in the audio group, which is fine
            // while a microphone is the only thing that moves it — and wrong
            // the moment something else does: a run with `--tempo-source` and
            // no `--audio-in` followed a tempo that appeared nowhere on
            // screen. Printed here only when the audio group is absent, so it
            // appears exactly once either way.
            if self.audio.is_none() {
                let _ = write!(
                    self.status,
                    "{:.1}bpm ",
                    self.deck.signals().oscillator().bpm()
                );
            }
        }
        if let Some(audio) = &self.audio {
            let a = audio.status();
            let _ = write!(
                self.status,
                "| e{:.2} on{:.2} c{:.2} | {}{:.1}bpm heard{:.1} c{:.2}{} err{:+.3}b off{:.0}ms ",
                a.energy,
                a.onset,
                a.confidence,
                // The grid's own tempo first, because that is what the picture
                // is actually running at; what the tracker hears second, so a
                // disagreement between the two is visible rather than implied.
                if a.locked { "lock " } else { "free " },
                self.deck.signals().oscillator().bpm(),
                a.estimated_bpm,
                a.estimate_confidence,
                // The tracker's note that this grid might be at half the
                // music's tempo, shown only when the estimate behind it is
                // worth anything. It moves nothing by itself — `.` does, and
                // this is what tells a performer to consider pressing it.
                if a.half_tempo_hint
                    && a.estimate_confidence >= karakuri_audio::lock::GATE_CONFIDENCE
                {
                    " x2?"
                } else {
                    ""
                },
                a.error,
                audio.latency_offset_ms(),
            );
        }
        eprintln!(
            "{}| {} exp {:.2} | {fps:.1} fps",
            self.status,
            op_name(self.look.op),
            self.look.exposure
        );
    }
}
