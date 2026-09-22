use super::super::*;
use super::constants::*;

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
}
