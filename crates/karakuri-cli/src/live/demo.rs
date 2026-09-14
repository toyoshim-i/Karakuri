use super::*;

/// A demonstration that drives itself, as a list of `(seconds, key)`.
///
/// Every entry goes through [`Live::key`], the same function a keyboard
/// reaches, so what a watcher sees is what pressing those keys does and not a
/// second path that resembles it. Nothing here can do anything a person could
/// not.
///
/// It exists because a window is the only honest demonstration of a transport —
/// the scrub is a motion, and a still frame of it is a still frame — and
/// whoever is *describing* the feature is often not the one at the keyboard.
///
/// The order is the argument: engage first and let it sit, so it is clear that
/// engaging changes nothing; then scrub back a long way, hold, and let it run
/// back onto the grid.
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

/// The two topologies, over geometry that does not change, as a list of
/// `(seconds, key)` on the same terms as [`DEMO_SCRIPT`].
///
/// The deck holds one L1 file paired with a sprite renderer in slot 0 and a
/// stroke renderer in slot 1, so showing each slot without the other is the
/// demonstration. What a watcher sees is the same cloud, in the same places, at
/// the same instant, drawn two ways.
///
/// It fades a slot out rather than auditioning the other one, and that is the
/// one thing here that changed. This script pressed `v` three times — the mix,
/// each slot alone, the mix again — until ADR-0240 retired *Choose what the
/// output shows*. The output is the mix now and always, so the way to see one
/// slot without the other is to take the other out of the mix: `F` on the
/// focused slot's fader, `G` to bring it back, with a digit before each to say
/// which slot. That is `Operation::FadeDeck` where it used to be `SetPreview`,
/// and it isolates a slot exactly as the audition did.
///
/// What it costs is that a fade is scheduled and an audition was not. A press
/// lands on the current grid rather than in the frame it arrives, so the
/// picture changes a beat or two after the entry that asked for it — which is
/// the same gap [`DEMO_SCRIPT`]'s `F` and `G` already have and say is worth
/// watching for. The seconds below leave room for it.
///
/// The mix comes first and last on purpose. Both slots composited is the state
/// that shows they are the same geometry — the strokes lie along the dots — and
/// each slot alone is what shows how different the two look when the other is
/// not there to anchor it.
///
/// Nothing here presses a key that only means something to someone who was told
/// what to expect. Each fade produces a visibly different frame on its own,
/// which is the property [`DEMO_SCRIPT`]'s first entry deliberately does not
/// have and has to say so.
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

/// Which demonstration `--demo` runs.
///
/// A named script rather than a flag, because there is now more than one thing
/// worth showing and a single `--demo` would have to pick. Each is a
/// demonstration harness and not a feature: both drive [`Live::key`], so
/// neither can do anything a person at the keyboard could not.
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

    /// The deck this demonstration needs, for a run that named no material.
    ///
    /// A demonstration that requires the operator to assemble the scene is not one.
    /// `--demo lines` is about two renderers over one geometry, and a watcher
    /// handed a one-slot deck sees the script fade the only thing in the mix out
    /// and back. Overridden the moment any `--set` is given, so this supplies a
    /// scene rather than imposing one.
    pub(crate) fn deck(self) -> Vec<(Named, Vec<Named>)> {
        match self {
            Demo::Transport => Vec::new(),
            // **Two slots, and it has to stay two.** A stack — one slot with
            // both renderers over one simulation — is what several renderers
            // over one geometry now costs, and it is the wrong shape *here*:
            // `DEMO_LINES_SCRIPT` fades one slot's fader out at a time so the
            // other is seen alone, and a fader is per **slot**. A one-slot
            // deck has nothing to take away, so the script would show the same
            // picture and then an empty frame, and the demonstration would
            // run, look like it worked, and demonstrate nothing — which is the
            // defect the doc above already names.
            //
            // Slot 0 carries the stack anyway, so what the demonstration shows
            // is the mix, then one simulation drawn both ways, then strokes
            // alone. `--set drift_shell.kir,soft_points.kir,drift_streaks.kir`
            // is the plain way to ask for a stack and needs no demonstration.
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
    /// Press whatever [`DEMO_SCRIPT`] is due, if this run is driving itself.
    ///
    /// Wall clock rather than frame count, because what is being demonstrated is a
    /// performance and a performance happens in seconds. That makes the demo *not*
    /// reproducible frame for frame, which is fine and is worth saying: it is a
    /// thing to look at, not a thing to diff. Everything it presses goes through
    /// [`Live::key`], so it can do nothing a person at the keyboard could not.
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
        // **It loops**, because a demonstration nobody happened to be looking
        // at is a demonstration that did not happen. Every script is under half
        // a minute and starts over, so glancing at the window at any moment
        // eventually shows the thing.
        if at >= script.len() && elapsed >= demo.loop_seconds() {
            self.demo_started = Instant::now();
            at = 0;
        }
        self.demo = Some((demo, at));
    }
}
