use karakuri_midi::{Map, Message, Out, Port};
use karakuri_operation::Operation;
use std::path::Path;

use super::{appended, Feedback, Interface, Router};

/// An open surface: a [`Router`] with a port in front of it.
pub struct Surface {
    port: Port,
    /// Paired output port for LED/motor fader feedback, or `None` if input-only.
    out: Option<Out>,
    /// Whether an output drop warning has been logged in this session.
    said_dropped: bool,
    /// Pre-allocated buffer for outgoing wire bytes to avoid allocations on render thread.
    outbox: Vec<[u8; 3]>,
    router: Router,
    /// Name stem of the active map file, or `None` if running unmapped.
    map_name: Option<String>,
    /// Pre-allocated inbox buffer for draining incoming MIDI messages.
    inbox: Vec<Message>,
}

/// Buffer capacity for incoming messages per frame.
const INBOX: usize = 256;

impl Surface {
    /// Opens the specified input port and loads its map file if provided.
    pub fn open(port: &str, map_path: Option<&Path>) -> Result<Surface, String> {
        let (surface, notes) = Surface::assembled(Port::open(port)?, map_path)?;
        for note in &notes {
            eprintln!("  midi map: {note}");
        }
        eprintln!(
            "midi in: `{}`, {} mapping{}{}",
            surface.port_name(),
            surface.mappings(),
            if surface.mappings() == 1 { "" } else { "s" },
            if surface.mappings() == 0 {
                " — turn a knob and this will print the line that would map it"
            } else {
                ""
            }
        );
        if let Some(out) = surface.out_name() {
            eprintln!("midi out: `{out}` — mapped controls follow the deck");
        }
        Ok(surface)
    }

    /// Opens the default available MIDI input port with a wake callback for background processing.
    ///
    /// Returns the opened surface and any map parse warnings.
    pub fn first(
        map_path: Option<&Path>,
        wake: impl Fn() + Send + 'static,
    ) -> Result<(Surface, Vec<String>), String> {
        Surface::assembled(Port::waking("", wake)?, map_path)
    }

    /// Assembles a surface from an open port and optional map file path.
    fn assembled(port: Port, map_path: Option<&Path>) -> Result<(Surface, Vec<String>), String> {
        let (map, notes) = match map_path {
            Some(path) => {
                let text = std::fs::read_to_string(path)
                    .map_err(|e| format!("reading MIDI map `{}`: {e}", path.display()))?;
                Map::parse(&text)
            }
            None => (Map::default(), Vec::new()),
        };
        Ok((
            Surface {
                out: paired_out(port.name()),
                said_dropped: false,
                outbox: Vec::with_capacity(INBOX),
                port,
                router: Router::new(map),
                map_name: map_path
                    .and_then(Path::file_stem)
                    .map(|stem| stem.to_string_lossy().into_owned()),
                inbox: Vec::with_capacity(INBOX),
            },
            notes,
        ))
    }

    /// The port that was opened, as the device named it.
    pub fn port_name(&self) -> &str {
        self.port.name()
    }

    /// What the map in use is called, or `None` for a surface running without one —
    /// see [`Surface::map_name`](Surface::map_name)'s field.
    pub fn map_name(&self) -> Option<&str> {
        self.map_name.as_deref()
    }

    /// How many mappings loaded — [`Router::mappings`], through the surface that
    /// holds it rather than a second count.
    pub fn mappings(&self) -> usize {
        self.router.mappings()
    }

    /// Everything that arrived since the last frame, as operations this deck can
    /// answer.
    pub fn take(&mut self, slot_count: usize, interface: &dyn Interface, out: &mut Vec<Operation>) {
        self.port.drain(&mut self.inbox);
        // Split rather than borrowed together: `route` writes to the router and
        // reads the inbox, and both are fields of `self`. `mem::take` would
        // hand the allocation back only if nothing panicked in between.
        let Surface { router, inbox, .. } = self;
        router.route(inbox, slot_count, interface, out);
        for notice in router.notices() {
            eprintln!("  midi: {notice}");
        }
    }

    /// Returns the first non-release MIDI message received this frame for interactive learn mode.
    pub fn learning(&mut self) -> Option<Message> {
        self.port.drain(&mut self.inbox);
        self.inbox
            .iter()
            .copied()
            .find(|message| !matches!(message, Message::NoteOff { .. }))
    }

    /// Binds `message` to `target` and updates or creates the map file at `to` (P-0096).
    pub fn learn(&mut self, message: Message, target: &str, to: &Path) -> Result<String, String> {
        let line = self.router.map.learn(message, target)?;
        if let Some(dir) = to.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("making `{}`: {e}", dir.display()))?;
        }
        let held = std::fs::read_to_string(to).ok();
        let key = line.split_once("->").map(|(from, _)| from).unwrap_or("");
        let text = appended(held, &self.seed(), key, &line);
        std::fs::write(to, text).map_err(|e| format!("writing `{}`: {e}", to.display()))?;
        self.map_name = to
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned());
        self.router.relit();
        Ok(line)
    }

    /// Returns sorted map declaration lines to seed a newly initialized map file.
    fn seed(&self) -> String {
        let mut lines: Vec<String> = self.router.map.lines().collect();
        lines.sort();
        let mut text = String::from(
            "# Written by `karakuri` on the first learn of a run, from the map it was \
             playing.\n# Every line below is one this program loaded; edit it by hand as \
             freely as any other.\n\n",
        );
        for line in lines {
            text.push_str(&line);
            text.push('\n');
        }
        text
    }

    /// Which message reaches `target`, or `None` for a control nothing is mapped to
    /// — `karakuri_midi::Map::bound` through the surface that holds the map, so a
    /// caller drawing a tooltip needs no second handle on it.
    pub fn bound(&self, target: &str) -> Option<String> {
        self.router.map.bound(target)
    }

    /// The output port that was opened, or `None` for a surface that only sends —
    /// for the line a program says in its legend.
    pub fn out_name(&self) -> Option<&str> {
        self.out.as_ref().map(Out::name)
    }

    /// Dispatches feedback wire messages for changed deck controls to the hardware output port (P-0094).
    pub fn show(&mut self, values: &dyn Feedback) {
        let Some(out) = self.out.as_ref() else {
            return;
        };
        let Surface { router, outbox, .. } = self;
        router.shown(values, outbox);
        for message in outbox.iter() {
            out.send(*message);
        }
        if !self.said_dropped && self.out.as_ref().is_some_and(|out| out.dropped() > 0) {
            self.said_dropped = true;
            eprintln!(
                "  midi out: the surface is not keeping up and messages are being dropped — a \
                 control may show where the deck was rather than where it is"
            );
        }
    }
}

/// Discovers matching output port for a given input device name by full name or prefix match.
fn paired_out(input: &str) -> Option<Out> {
    let first = input.split_whitespace().next().unwrap_or(input);
    Out::open(input).or_else(|_| Out::open(first)).ok()
}
