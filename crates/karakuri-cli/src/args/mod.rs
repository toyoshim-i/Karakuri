use super::*;

mod help;
mod parser;
mod validate;

pub(crate) use help::{fail, BINDINGS, USAGE};
pub(crate) use parser::*;
pub(crate) use validate::*;

/// What a Set file said that no flag can say.
///
/// Not flags: there is no `--seed` and no `--camera`, and inventing two so that
/// a file could be read would be adding surface to carry a value rather than to
/// be used. Slot 0's, always — `--load-set` fills that slot and every other
/// comes from `--set`.
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
    /// `--bind`, applied to every Set on the same terms as `overrides`.
    ///
    /// This flag is a stand-in for a Set file and is shaped so it can be retired
    /// for one. Bindings belong in a `.kbset`, but nothing loads one into the
    /// engine yet — `karakuri-store` decodes records and no other crate reads a
    /// `Record` — and that is its own slice of work. So the flag names the record's
    /// own fields, one comma-separated `field=value` per JSON field, and the day
    /// `--set` takes a Set file the change here is deleting this and calling
    /// `Set::bind` from the decoder instead. Like `--param`, it applies to every
    /// Set, because the record has no slot field: a Set file is per Set, and one
    /// per `--set` is what replaces it.
    pub(crate) bindings: Vec<Binding>,
    /// `--edge`, applied to every Set on the same terms as `overrides`.
    ///
    /// A procedure declares a named input slot and the Set binds it to a node.
    /// `uses far : Geometry` in a `.kir` says what the file needs without naming
    /// which node supplies it — a file that named one would be coupled to one Set —
    /// so this is where the other half is written. Which geometry a morph blends
    /// towards used to be `--set` position 1, written nowhere at all, and
    /// reordering the command line changed the picture in silence.
    ///
    /// The record is the home and the flag writes into it, on the terms `--bind`
    /// follows: an `edge` record in a Set file is what a saved use carries, and
    /// this is the authoring surface that exists today. Like `--param` it applies
    /// to every Set of the deck, and an edge whose node names nothing in a given
    /// Set is a statement about a different one — see
    /// [`karakuri_engine::set::Wiring::edges`].
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
    /// Which slots composite their renderers rather than overdrawing them —
    /// `--merge 0`, repeatable. See `karakuri_engine::set::Layering`.
    ///
    /// Not the whole answer any more: a Set file records its layering, so a slot
    /// filled by `--load-set` may composite without appearing here. Ask
    /// [`layering_for`], which is where the flag and the file meet.
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
    ///
    /// A third outcome rather than a field on [`Args`], and for `--help`'s reason:
    /// this is not a run. Nothing downstream of here — the compile, the scratch,
    /// the deck, the window — has anything to do with a listing, and a flag carried
    /// into [`Args`] would have to be checked above every one of them and would be
    /// wrong the day somebody added one more. The variant makes reaching a GPU with
    /// this flag not a thing that can be forgotten: there is no `Args` to run.
    ///
    /// It carries the store path because that is the whole of what a listing needs,
    /// and `--store` may be given on either side of this flag.
    ListSets(PathBuf),
    /// `--package ID` — or `--package FILE.kset`: write the Set filed under `ID`,
    /// or the one the authoring file at `FILE` names, to standard output with every
    /// source it names inlined, and stop.
    ///
    /// One flag and two moments rather than two flags, which is ADR-0229 part 4:
    /// packaging is *"one operation, two moments"*, and
    /// `docs/manual/operations.html`'s *Send a Set to somebody, and take one in* is
    /// the row both are a moment of. The field is still `id` because the parse
    /// cannot tell which it is without touching a disk and does not try;
    /// [`packaged_set`] decides on the extension, which is what ADR-0231 made the
    /// extension for.
    ///
    /// And the flag is named for what it does at both moments. It writes a store as
    /// often as it reads one — a `.kset` has every part hashed and put — so
    /// `--bundle`, which reads as an export and nothing else, named half of it.
    /// `--package` and [`ParseOutcome::TakeIn`] are the two directions of one row
    /// (`docs/contributing.md` §4: a name means one thing, and one thing has one
    /// name).
    ///
    /// [`ParseOutcome::ListSets`]'s variant for [`ParseOutcome::ListSets`]'s
    /// reason, and the reason is the whole of why it is here: packaging reads a
    /// file and hashes some bytes, and there is no Set to build, no adapter to
    /// request and no window to open. An `Args` field would have to be checked
    /// above every early return in `main` and would be wrong the day somebody added
    /// one more; a variant with no `Args` in it makes reaching a device not a thing
    /// that can be forgotten.
    Package {
        store: PathBuf,
        id: String,
    },
    /// `--take-in FILE`: take a Set file into the store, and stop — a `.kbset` as
    /// it stands, a `.kset` resolved against its own directory first.
    ///
    /// Here for the same reason, and it compiles: each source goes through the
    /// checker so that a metadata card can be written for it. That is the check
    /// pass, which takes no device — `Set::validate` runs without one and a single
    /// procedure certainly does.
    ///
    /// Which of the two forms it is, the extension says, exactly as
    /// [`ParseOutcome::Package`]'s does: the parse carries a path and touches no
    /// disk to classify it, and [`taken_in_file`] decides.
    TakeIn {
        store: PathBuf,
        file: PathBuf,
    },
}
