use super::*;

mod help;
mod parser;
mod validate;

pub(crate) use help::{fail, BINDINGS, USAGE};
pub(crate) use parser::*;
pub(crate) use validate::*;

/// Configuration loaded from a Set file that has no command-line flag equivalent.
#[derive(Clone, Debug, Default)]
pub(crate) struct FromSet {
    /// Per-geometry seeds read from the Set file, indexed by geometry slot.
    /// See [`salts_for`].
    pub(crate) salts: Vec<Option<u32>>,
    pub(crate) camera: Option<karakuri_engine::camera::Orbit>,
    /// Layering mode specified by the Set file. See [`layering_for`].
    pub(crate) layering: karakuri_engine::set::Layering,
    /// Which renderer the file left folded to, and `None` where every one of them
    /// is live — see `setfile::Loaded::live`, whose value this is.
    pub(crate) live: Option<u32>,
    /// Per-geometry capacities read from the Set file. See [`capacities_for`].
    pub(crate) capacities: Vec<Option<u32>>,
}

#[cfg_attr(test, derive(Debug))]
pub(crate) struct Args {
    /// `--param name=value`, applied after each Set is built, to every Set. A
    /// parameter change is a uniform write, not a structural change, which is why
    /// it needs no fork and no recompilation.
    pub(crate) overrides: Vec<ParamWrite>,
    /// Dynamic parameter bindings to apply to built sets.
    pub(crate) bindings: Vec<Binding>,
    /// Procedure input slot wiring across sets.
    pub(crate) edges: Vec<karakuri_engine::set::Edge>,
    /// The session tempo. There is no tempo record in the v0.2 vocabulary, so
    /// unlike `--bind` this flag has nothing to map onto yet; it is here because a
    /// binding to `beat` is meaningless at a tempo nobody can set.
    pub(crate) bpm: f32,
    /// One deck slot per entry, in composite order: the L1 that simulates, and the
    /// renderers drawn over it in list order — see `Set::build_many`. One renderer
    /// is the ordinary case; several is one simulation drawn several ways, which
    /// costs a draw pass apiece and no extra memory.
    pub(crate) sets: Vec<(Named, Vec<Named>)>,
    /// What `--capacity` was given, or [`karakuri_ir::DEFAULT_CAPACITY`] when it
    /// was not — read only through [`capacity_for`], which prefers the procedure's
    /// own declaration to this unless `capacity_given`.
    pub(crate) capacity: u32,
    /// Whether `--capacity` was *typed*. Without it a procedure's own declared
    /// default is used — see [`capacity_for`].
    pub(crate) capacity_given: bool,
    pub(crate) render_to: Option<PathBuf>,
    pub(crate) seq_to: Option<PathBuf>,
    pub(crate) frames: u32,
    /// The preview window's size, and nothing else. A window is a preview of what
    /// leaves by some other route, so it has no say in what is drawn — see
    /// `canvas`. Meaningless without a window, and refused rather than ignored when
    /// there is none.
    pub(crate) size: (u32, u32),
    /// What the run renders at. The canvas every `VideoSource` draws into, what
    /// every deck slot is sized to match, and what an offscreen render writes.
    /// Fixed for the run: changing it reallocates every slot's target, and the
    /// frame path allocates nothing.
    pub(crate) canvas: (u32, u32),
    /// Whether `--canvas` was *typed*. Carried rather than resolved at parse time
    /// because what it is refused against — whether the session being replayed
    /// carries a canvas of its own — is not known until the stream is read. See
    /// `replay_session`.
    pub(crate) canvas_given: bool,
    /// Watch every pair and hot-swap the slot whose files changed. Off by default:
    /// a run that is not being edited should not carry a worker thread per slot and
    /// a watchdog it will never use.
    pub(crate) watch: bool,
    /// Slot indices to composite rather than overdraw (`--merge <index>`).
    pub(crate) merge: Vec<usize>,
    /// The interface every slot's Set publishes — `--publish
    /// name=L4:0:exposure[0.2..0.8]`, repeatable. Empty means every Set publishes
    /// everything it declares, which is what happened before an interface existed.
    pub(crate) published: Vec<karakuri_engine::set::Published>,
    /// `--mcp`. A port to serve the Model Context Protocol on, loopback only.
    /// `None` is the ordinary case and nothing in the frame path changes: this is a
    /// third control surface beside the keyboard and MIDI, and like them it can do
    /// nothing they cannot.
    pub(crate) mcp: Option<u16>,
    /// `--tempo-source`. A command to run as a child process that says where the
    /// beat is — see [`crate::tempo_source`]. `None` is the ordinary case and
    /// changes nothing: the grid comes from `--bpm`, the beat tracker and the tap
    /// keys exactly as it always has.
    pub(crate) tempo_source: Option<String>,
    /// `--audio-in`. `None` is no device at all, which is not the same as a device
    /// that is silent: with no device the bus answers `energy` and the bands
    /// exactly as it did before audio existed, and the oscillator free-runs.
    pub(crate) audio_in: Option<String>,
    /// `--midi-in`: a substring of the input port's name, or empty for the first
    /// one there is.
    pub(crate) midi_in: Option<String>,
    /// `--midi-map`: the operator's table. Optional even with a port, because a
    /// surface with no map still prints what it sends, which is the state an
    /// operator is in before they have written one.
    pub(crate) midi_map: Option<PathBuf>,
    /// The unmeasurable half of the output lag, in milliseconds. See
    /// `audio::DEFAULT_LATENCY_OFFSET_MS`.
    pub(crate) latency_offset_ms: f32,
    /// `--store DIR`: where content-addressed artifacts and Set files live.
    pub(crate) store: PathBuf,
    /// `--save-set ID`: write the material as a Set file and stop.
    pub(crate) save_set: Option<String>,
    /// `--load-set ID`: take the material from a Set file instead of from two
    /// `.kir` paths and the flags.
    pub(crate) load_set: Option<String>,
    /// `--record-session ID`: write the timeline as it happens.
    pub(crate) record_session: Option<String>,
    /// `--replay ID`: render a recorded session instead of running one.
    pub(crate) replay: Option<String>,
    /// What a Set file said that no flag says, when one was loaded — see
    /// [`FromSet`].
    pub(crate) from_set: Option<FromSet>,
    /// Drive the deck from a named script instead of waiting for a keyboard. A
    /// demonstration harness, not a feature: it presses keys.
    pub(crate) demo: Option<Demo>,
    /// The frame budget the watchdog holds a swapped-in Set to, in milliseconds.
    /// Exposed mostly so that the verdict against can be provoked on demand —
    /// `--budget-ms 0` stops every slot a build lands in — rather than only by
    /// writing a procedure slow enough to trip it.
    pub(crate) budget_ms: f32,
    pub(crate) look: Look,
}

/// What [`parse_args_from`] found, short of a full [`Args`]: `--help` is a
/// request to print and exit successfully, not an error.
pub(crate) enum ParseOutcome {
    Run(Box<Args>),
    Help,
    /// `--list-sets`: print what the store at this path holds, and stop.
    ListSets(PathBuf),
    /// `--package ID | FILE.kset`: exports the specified Set or file to standard output.
    Package {
        store: PathBuf,
        id: String,
    },
    /// `--take-in FILE`: take a Set file into the store, and stop — a `.kbset` as
    /// it stands, a `.kset` resolved against its own directory first.
    TakeIn {
        store: PathBuf,
        file: PathBuf,
    },
}
