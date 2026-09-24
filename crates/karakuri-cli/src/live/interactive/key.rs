use super::super::*;
use super::constants::*;

impl Live {
    /// Handles keyboard input events, dispatching operations to the active deck.
    ///
    /// Returns true if the key event was a request to quit (Escape).
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
}
