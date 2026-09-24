use std::fmt::Write;
use std::time::Instant;

use super::*;

impl Live {
    /// Emits periodic status line summarizing slot states, faders, fps, and audio metrics.
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
            // Display tag if slot execution is overloaded/stopped.
            self.status
                .push_str(stopped_tag(self.deck.overloaded(addr)));
            // Display active or pending control transitions.
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
            // And what is waiting to be chosen.
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
            // Output mean and peak luminance levels if slot rendered.
            match self.deck.level(addr) {
                Some(level) => {
                    let _ = write!(self.status, "m{:.3} p{:.1}", level.mean, level.peak);
                    // Report non-finite texel count if any were discarded during metering.
                    if level.bad_texels > 0 {
                        let _ = write!(self.status, " x{}", level.bad_texels);
                    }
                    self.status.push_str("  ");
                }
                None => self.status.push_str("m---- p----  "),
            }
            // Output active signal binding values.
            for (key, value) in self.deck.slot(addr).set().bound() {
                let _ = write!(self.status, "{key}={value:.3}  ");
            }
        }
        // Display tempo source status and peer count when active.
        if let Some(source) = &self.tempo_source {
            let _ = write!(
                self.status,
                "| {} {}p{}{} ",
                source.name(),
                source.peers(),
                // Anchors thrown away for unusable numbers.
                match source.rejected() {
                    0 => String::new(),
                    n => format!(" x{n}"),
                },
                match (source.ended().is_some(), source.playing()) {
                    (true, _) => " GONE",
                    // Distinguish between greeting, stopped, and running source states.
                    (false, None) => " ?",
                    (false, Some(false)) => " stop",
                    (false, Some(true)) => "",
                }
            );
            // Display tempo here when audio input group is absent.
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
