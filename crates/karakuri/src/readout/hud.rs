//! HUD legend formatting, status string generation, and console logging.

use super::*;
use crate::presets_listing;

impl Readout {
    /// A drag, in words: what was asked, where it landed, what held it, and what
    /// the pair either side is now.
    pub(crate) fn say_drag(&self, d: Dragged) -> String {
        let Dragged::Boundary {
            split,
            index,
            axis,
            asked,
            landed,
            held,
        } = d
        else {
            // A fader's words are the window loop's, because they are about
            // what happened to the *deck* after the operation left here.
            unreachable!("a fader drag says its own line")
        };
        let sizes = match self.panel.pair(split, index) {
            Some((a, b)) => format!(
                "{} {:.0} | {} {:.0}",
                self.label(a),
                axis.extent(self.panel.layout().rect(a)),
                self.label(b),
                axis.extent(self.panel.layout().rect(b))
            ),
            None => "no pair".to_owned(),
        };
        let stop = match held {
            Some(by) => format!(" — held {by:+.1} by a stop, and it stays there until it moves"),
            None => String::new(),
        };
        format!("  drag: asked {asked:.1}, landed {landed:.1}{stop} [{sizes}]")
    }

    /// What an operation did, in words. The model returns the facts; which English
    /// they take is the operation that was asked for, which is why this has both.
    pub(crate) fn say_op(&self, op: Op, outcome: &Outcome) {
        match outcome {
            Outcome::Folded { id, folded, root } => {
                let what = match op {
                    Op::FoldEnclosing(_) => "the split ",
                    _ => "",
                };
                println!(
                    "fold: {what}{} is now {}{}",
                    self.label(*id),
                    folding(*folded),
                    match *root && *folded {
                        true => " — that was the root, so the panel is empty; z brings it back",
                        false => "",
                    }
                );
            }
            Outcome::Unfolded(ids) => match ids.is_empty() {
                true => println!("unfold: nothing is folded"),
                false => {
                    let names: Vec<String> = ids.iter().map(|id| self.label(*id)).collect();
                    println!("unfold: {}", names.join(", "));
                }
            },
            Outcome::Soloed(id) => println!(
                "solo: {} — everything else folded (soloed = {})",
                self.label(*id),
                self.panel.layout().is_soloed()
            ),
            Outcome::Unsoloed { was } => println!(
                "unsolo: {}",
                match was {
                    true => "the arrangement before the solo is back",
                    false => "nothing was soloed",
                }
            ),
            Outcome::Reset => println!("reset: a fresh arrangement, at the same viewport"),
            Outcome::Restored => {}
            Outcome::Report(_) => {}
            Outcome::Nothing => {
                println!("fold: that is the root, and nothing encloses it")
            }
        }
    }

    // -- the legend -----------------------------------------------------

    /// What this program is, said once at startup.
    pub(crate) fn print_legend(
        &mut self,
        budget_ms: Option<f32>,
        governed: &Report,
        presets: Option<&karakuri_environment::places::Presets>,
        store: &std::path::Path,
        mcp_port: Option<u16>,
    ) {
        self.panel.solve();
        let layout = self.panel.layout();
        let viewport = layout.viewport();
        println!();
        println!(
            "the console, in a {:.0} x {:.0} viewport",
            viewport.w, viewport.h
        );
        match presets {
            Some(presets) => {
                let held = presets_listing(Some(presets)).len();
                println!(
                    "  presets: {} ({} presets, {})",
                    presets.dir.display(),
                    held,
                    presets.found.how()
                );
            }
            None => println!("  presets: none"),
        }
        println!("  store: {}", store.display());

        let live: Vec<usize> = (0..DECKS)
            .filter(|slot| {
                self.view
                    .mixer
                    .get(*slot)
                    .is_some_and(|strip| strip.tally == view::Tally::Live)
            })
            .collect();
        let live_names: Vec<String> = live
            .iter()
            .map(|slot| format!("deck {}", deck_letter(*slot as u8)))
            .collect();
        let behind = self.view.mixer.len().min(DECKS);
        println!(
            "  decks: {behind} configured, {} live ({})",
            live.len(),
            if live_names.is_empty() {
                "none".to_string()
            } else {
                live_names.join(", ")
            }
        );

        if let Some(budget) = budget_ms {
            println!("  refresh budget: {budget:.1} ms");
        }

        let audio_desc = self
            .view
            .audio
            .as_ref()
            .and_then(|audio| audio.device.as_deref())
            .unwrap_or("none");
        println!("  audio-in: {audio_desc}");

        for decision in governed.parked() {
            println!(
                "  parked: deck {} ({:?})",
                deck_letter(decision.slot as u8),
                decision.reason
            );
        }

        match mcp_port {
            Some(port) => println!("  mcp: serving on 127.0.0.1:{port}"),
            None => println!("  mcp: disabled (use --mcp PORT to enable)"),
        }

        println!();
        println!("keys:");
        for (key, what) in KEYS {
            println!("  {key:<10}{what}");
        }
        println!();
    }
}

pub(crate) fn folding(folded: bool) -> &'static str {
    match folded {
        true => "folded",
        false => "unfolded",
    }
}

pub(crate) fn deck_letter(deck: u8) -> &'static str {
    DECK_LETTERS
        .get(usize::from(deck))
        .copied()
        .unwrap_or("(no such deck)")
}

pub(crate) fn knob_word(knob: &Knob) -> &'static str {
    match knob {
        Knob::Trim { .. } => "trim",
        Knob::Fader { .. } => "fader",
        Knob::Out => "out",
        Knob::Chain { .. } => "chain effect",
        Knob::Param { .. } => "parameter",
    }
}

pub(crate) fn knob_where(knob: &Knob) -> String {
    match knob.deck() {
        Some(deck) => format!("deck {}", deck_letter(deck)),
        None => "master".to_owned(),
    }
}
