use super::*;

/// Scripted key actions for automated transport demonstration as `(seconds, key)`.
pub(crate) const DEMO_SCRIPT: &[(f32, char)] = &[
    // Four seconds of the material as it is, for a before.
    (4.0, 'y'), // beat sync — the picture does not move, deliberately
    // Two bars back, a quarter beat at a time, over about a second and a half.
    (6.0, 'U'),
    (6.1, 'U'),
    (6.2, 'U'),
    (6.3, 'U'),
    (6.4, 'U'),
    (6.5, 'U'),
    (6.6, 'U'),
    (6.7, 'U'),
    (6.8, 'U'),
    (6.9, 'U'),
    (7.0, 'U'),
    (7.1, 'U'),
    (7.2, 'U'),
    (7.3, 'U'),
    (7.4, 'U'),
    (7.5, 'U'),
    // Held there for three seconds: the slot is two bars behind the room and
    // still locked to it, so it moves at the room's rate from where it is.
    // Then forward again, past where it was.
    (10.5, 'i'),
    (10.6, 'i'),
    (10.7, 'i'),
    (10.8, 'i'),
    (10.9, 'i'),
    (11.0, 'i'),
    (11.1, 'i'),
    (11.2, 'i'),
    (11.3, 'i'),
    (11.4, 'i'),
    (11.5, 'i'),
    (11.6, 'i'),
    (11.7, 'i'),
    (11.8, 'i'),
    (11.9, 'i'),
    (12.0, 'i'),
    (12.1, 'i'),
    (12.2, 'i'),
    (12.3, 'i'),
    (12.4, 'i'),
    // Back to free running, which prints the refusal `tempo` earns on material
    // that reads `beats` on its way past.
    (16.0, 'y'),
    // Then a fade out and back in, on the defaults — the next bar, four beats
    // — so what a watcher sees is the gesture as it ships rather than one
    // tuned to be visible. The gap between the press and the movement is the
    // quantum doing its job and is the thing worth watching for.
    (17.5, 'F'),
    (22.0, 'G'),
];

/// How long one pass through [`DEMO_SCRIPT`] lasts before it starts over. Past
/// the last entry, so the run ends free-running for a few seconds — the state
/// it began in, which is what makes the next pass legible as a repeat rather
/// than as something new.
pub(crate) const DEMO_LOOP_SECONDS: f32 = 27.0;

/// Scripted key actions demonstrating dual topology rendering over identical geometry.
pub(crate) const DEMO_LINES_SCRIPT: &[(f32, char)] = &[
    // Five seconds of the mix: strokes and sprites over each other. Then slot
    // 1's fader out, leaving slot 0 alone — sprites *and* strokes, from one
    // simulation.
    (5.0, '1'),
    (5.0, 'F'),
    // Slot 1 back, slot 0 out: strokes alone, the same elements.
    (10.0, 'G'),
    (10.0, '0'),
    (10.0, 'F'),
    // And back to the mix.
    (15.0, 'G'),
];

/// One pass through [`DEMO_LINES_SCRIPT`], with five seconds of the mix after
/// the last press before it starts over.
pub(crate) const DEMO_LINES_LOOP_SECONDS: f32 = 20.0;

/// Available automated demonstration modes for `--demo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Demo {
    /// The transport: beat sync engaged, scrubbed two bars back, held, and run
    /// forward past where it was.
    Transport,
    /// `topology lines`: one L1, drawn as sprites and as strokes.
    Lines,
}

impl Demo {
    pub(crate) fn from_name(name: &str) -> Option<Demo> {
        match name {
            "transport" => Some(Demo::Transport),
            "lines" => Some(Demo::Lines),
            _ => None,
        }
    }

    pub(crate) fn script(self) -> &'static [(f32, char)] {
        match self {
            Demo::Transport => DEMO_SCRIPT,
            Demo::Lines => DEMO_LINES_SCRIPT,
        }
    }

    pub(crate) fn loop_seconds(self) -> f32 {
        match self {
            Demo::Transport => DEMO_LOOP_SECONDS,
            Demo::Lines => DEMO_LINES_LOOP_SECONDS,
        }
    }

    /// Returns default deck setup required for the demonstration if not provided.
    pub(crate) fn deck(self) -> Vec<(Named, Vec<Named>)> {
        match self {
            Demo::Transport => Vec::new(),
            // Two-slot deck enables fading individual slots to demonstrate line/stroke variations.
            Demo::Lines => vec![
                (
                    Named::bare("examples/drift_shell.kir"),
                    vec![
                        Named::bare("examples/soft_points.kir"),
                        Named::bare("examples/drift_streaks.kir"),
                    ],
                ),
                (
                    Named::bare("examples/drift_shell.kir"),
                    vec![Named::bare("examples/drift_streaks.kir")],
                ),
            ],
        }
    }
}

impl Live {
    /// Dispatches scheduled demo key inputs based on wall-clock time.
    pub(super) fn run_demo(&mut self) {
        let Some((demo, next)) = self.demo else {
            return;
        };
        let script = demo.script();
        let elapsed = self.demo_started.elapsed().as_secs_f32();
        let mut at = next;
        while let Some((due, key)) = script.get(at) {
            if elapsed < *due {
                break;
            }
            let key = Key::Character(key.to_string().into());
            self.key(&key);
            at += 1;
        }
        // Demo scripts loop continuously for unattended display.
        if at >= script.len() && elapsed >= demo.loop_seconds() {
            self.demo_started = Instant::now();
            at = 0;
        }
        self.demo = Some((demo, at));
    }
}
