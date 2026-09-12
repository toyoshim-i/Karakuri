//! CLI launch configuration and startup source resolution.

use crate::deck_letter;
use karakuri_environment::history;

#[derive(Debug, Clone)]
pub(crate) struct Sources {
    pub(crate) l1: std::path::PathBuf,
    pub(crate) l4: std::path::PathBuf,
}

impl Sources {
    /// The pair a run with no paths opens on, under whichever preset library
    /// answered.
    ///
    /// This was `Default`, and what it resolved against was
    /// `env!("CARGO_MANIFEST_DIR")` — the build machine's own tree, baked in at
    /// compile time. That was the only production line in the workspace doing it,
    /// and it meant a binary installed anywhere else found no presets at all: not a
    /// wrong pair, no pair, and the Library's `presets` tier a tier with no file in
    /// it. A POSIX process cannot ask where it is, so the answer is to be *told* —
    /// `--presets` — or to go looking from [`std::env::current_exe`], which is
    /// [`karakuri_environment::places::presets`] and not this file's business.
    ///
    /// What is this file's business is the two names, because they are this
    /// program's choice of what to open on rather than a property of a preset
    /// library: a library is a directory with at least one `.kset` in it
    /// (`karakuri_environment::places`'s `is_a_library`, which asked for a `.kir`
    /// until the authoring form landed), and these two are the parts
    /// `examples/star_vortex.kset` names — a funnel of ten thousand flares, each
    /// one an object with a size and an orientation you can follow with your eye.
    ///
    /// They were `drift_shell.kir` + `soft_points.kir` until 2026-09-07, which is
    /// the pair `examples/drift_cloud.kset` names and what `docs/contributing.md`
    /// §1 calls the reference workload — 262144 alpha-blended sprites, a count
    /// tuned when this panel drew one canvas, where no single element is visible
    /// and the picture reads as fog. That coincidence has ended rather than been
    /// broken: the workload is the named Set and not whatever a bare run opens on
    /// (ADR-0270), so changing these two names is a demo decision and takes no
    /// figure with it, and
    /// [ADR-0271](../../../docs/adr/0271-the-panel-opens-on-the-demo-rather-than-on-the-reference-workloads-pair.md)
    /// is that decision. See
    /// `the_capacity_is_the_l1s_own_declaration_and_the_l4_declares_none`, which
    /// pins the Set's capacity by name and checks these two for a per-file read —
    /// two claims that were one assertion until ADR-0270.
    ///
    /// Two paths and not the `.kset`, which is the shape of this type and costs the
    /// Set's two `bind` records: `star_vortex.kset` binds `energy` to the funnel's
    /// `scale` and `band0` to its `ripple`, and a bare run gets neither. The funnel
    /// turns and the pulse still climbs it, because both are the L1's own motion;
    /// what a bare run does not show is the material answering to sound. Loading
    /// `star_vortex` off the Library bay is where that is, and the same is true of
    /// every shipped Set.
    ///
    /// Under the root rather than under the working directory, and the asymmetry
    /// with a typed path is the one the old `Default` had for the same reason: a
    /// pair nobody named has to be found wherever the run was started from, and a
    /// path an operator typed is theirs and is read from where they typed it.
    pub(crate) fn under(presets: &std::path::Path) -> Sources {
        Sources {
            l1: presets.join("coil_vortex.kir"),
            l4: presets.join("star_flares.kir"),
        }
    }

    /// What the mixer strip calls what this deck is playing, and it is this file's
    /// word rather than the engine's.
    ///
    /// `view::Strip::name` says why there is no other answer: nothing reachable
    /// from a `Deck` carries a name for the material in a slot. A `Set` names its
    /// *nodes* and its *published controls* and has no name of its own, which is
    /// right — a Set is built from a list of `.kir` files, and only whoever passed
    /// that list knows what to call the result. This is that list, derived from the
    /// two paths rather than typed again.
    ///
    /// It is what every slot *opens* on and not what one is playing, which is
    /// [`Gfx::material`]'s distinction: this program builds every slot from the one
    /// pair, and a load moves one of them to a Set the library names. So this
    /// answers once, at startup, and the per-slot name is kept and rewritten there.
    pub(crate) fn material(&self) -> String {
        let stem = |path: &std::path::Path| {
            path.file_stem()
                .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned())
        };
        format!("{} + {}", stem(&self.l1), stem(&self.l4))
    }
}

/// The working copy each deck runs from, one pair per slot, made before the
/// window opens.
///
/// # Why a panel copies at all
///
/// `karakuri_environment::scratch` is the one place a live edit may land, and
/// it exists because before it did a surface was given write access to whatever
/// path the material came from and three shipped presets were replaced in one
/// session (P-0096). This program had none of it: every slot watched the two
/// paths the operator typed, so the file an editor opened was the preset
/// itself, and a run started with no paths at all watched `examples/`.
///
/// # And the panel is not `--render`
///
/// `scratch.rs` gates copying on *a run that can be edited*, because creating a
/// directory as a side effect of a pure render *"would make a function of its
/// arguments into one that leaves a mark"*. The gate is a question about this
/// program rather than about a flag, and the answer is that this program is a
/// `--watch` run that cannot be turned off: every slot is built over a
/// `watch::Watch` ([`watched`]), the deck's whole way of changing material is
/// an edit picked up by a poll, and a load off the Library bay writes into this
/// same directory on a key press ([`loading`]). There is no run of this binary
/// that opens a file read-only, so the condition is not *checked* here — it is
/// true, and the copies are made once, before the first frame, rather than on
/// the first press.
///
/// What that costs is the store directory existing on a run that saves nothing,
/// which is the cost `karakuri-cli --watch` already pays and the manual already
/// states: *"a run with `--watch` or `--mcp` opens the store at startup either
/// way — the scratch lives in it."* It is not the cost [`arrangements`] and
/// [`library`] decline — those two decline to create a store in order to *list*
/// it, and a listing that made a directory would be a read with a side effect.
/// This is a run that has already decided to write.
///
/// # One preset in four slots is four files
///
/// [`SLOTS`] copies of the one pair, and `scratch::materialise` names each one
/// for the slot it belongs to — `A0-drift_shell.kir`. That is the requirement
/// rather than a consequence of it: a slot is the unit that gets replaced, so a
/// deck whose file is also another deck's cannot be moved on its own, and one
/// save would rebuild all four. See `scratch.rs`'s header for the rule this
/// replaced and why it went.
///
/// The `Err` is a sentence naming the path that would not be read or written,
/// which is [`materialise`](karakuri_environment::scratch::materialise)'s own.
pub(crate) fn working_copies(
    store: &std::path::Path,
    sources: &Sources,
    slots: usize,
) -> Result<(std::path::PathBuf, Vec<Sources>), String> {
    let mut copies: Vec<Sources> = std::iter::repeat_n(sources.clone(), slots).collect();
    let dir = karakuri_environment::scratch::materialise(
        store,
        // Two distinct fields of one value, which is why this is an array
        // literal rather than a chain: a slot is an L1 and an L4 in node
        // order, and that order is what puts the L1 at `A0` and the renderer
        // at `A1`.
        copies.iter_mut().map(|pair| [&mut pair.l1, &mut pair.l4]),
    )?;
    Ok((dir, copies))
}

/// What the operator opens, deck by deck, said once at startup.
///
/// `karakuri-cli` prints one line of this — *"the deck runs from copies here,
/// so the files you named are not written to"* — and one line is not enough
/// here. Four decks on one preset are four files whose names an operator cannot
/// guess and cannot tell apart by content, since at startup they are identical;
/// what makes a file deck B's is its name, and the name is this program's
/// choice. So the directory, then a line per deck.
///
/// Printed rather than drawn. The panel has no place for a file path: the mixer
/// strip names *material* and would be naming the same thing four times with
/// four spellings, and the Inspector addresses nodes rather than files. The
/// startup print is where this program already says what it resolved — the
/// preset library, the store, the room — and this belongs with those three.
///
/// A free function over the copies so it can be asserted without a window; see
/// `the_startup_print_names_one_file_per_deck`.
pub(crate) fn running_from(dir: &std::path::Path, copies: &[Sources]) -> String {
    let mut said = format!(
        "scratch: {} — every deck runs from its OWN copy here, so the two paths you named \
         are not written to and editing them moves nothing. Point an editor at these:",
        dir.display()
    );
    for (slot, pair) in copies.iter().enumerate() {
        let name = |path: &std::path::Path| {
            path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string())
        };
        said.push_str(&format!(
            "\n  deck {}: {} + {}",
            deck_letter(slot as u8),
            name(&pair.l1),
            name(&pair.l4)
        ));
    }
    said.push_str(
        "\nthe same pair in every slot is one file per deck per node, and that is the point \
         rather than a duplicate: an edit moves the deck whose file it is and no other, so \
         one save puts one candidate in the Staging lane.",
    );
    said
}

/// The run's edit history, with the version every deck starts on already in it
/// — one [`karakuri_environment::history::Snapshots`] for the whole run, handed
/// to every watcher [`Engine::new`] makes.
///
/// # Why the seed and the watchers share one
///
/// They share the dedup. Seeded separately, the first rebuild would write the
/// untouched procedure a second time and the chain an operator walks back
/// through would begin with a duplicate. It is
/// [`karakuri_environment::history`]'s own requirement and is why this is made
/// once, here, rather than per window.
///
/// # Why there is a seed at all
///
/// A first edit whose predecessor was never written down is the one edit that
/// cannot be walked back, so the version a run *starts* on is filed before
/// anything can edit it (ADR-0089). It is filed from the working copies, which
/// are what the watchers poll and what an editor is pointed at.
///
/// # Under no Set, which is the answer and not a placeholder
///
/// Every slot of this program launches on the pair the command line settled,
/// and a pair somebody typed is not a Set. The nearest thing to a name is
/// [`Sources::material`] — a readout for the mixer strip — and filing versions
/// under it would put rows in the history under a Set no listing can ever match
/// (ADR-0276). A library load is what gives a slot an id, and it carries it on
/// the aim ([`loading`], ADR-0304).
///
/// A free function over the copies so it can be asserted without a window, the
/// way [`running_from`] is; see
/// `the_launch_versions_are_filed_before_a_window`. Failures are reported
/// inside `history::seed` and are never fatal: a history that could not be
/// written must not stop a run from starting.
pub(crate) fn seeded(store: &std::path::Path, copies: &[Sources]) -> history::Shared {
    let shared = history::Snapshots::shared(store);
    history::seed(
        &shared,
        copies
            .iter()
            .enumerate()
            .map(|(slot, pair)| (slot, None, vec![pair.l1.as_path(), pair.l4.as_path()])),
    );
    shared
}

/// How this program is called, printed for `--help` and for anything it cannot
/// read as a pair.
///
/// The two flags are in the second paragraph rather than the first, which is
/// where they belong: an operator reading this is looking for how to play
/// something, and the answer is still the pair on the first line. See
/// [`Sources`] for why a flag that answers *where the data lives* is not the
/// second material vocabulary `docs/contributing.md` §4 refuses.
pub(crate) const USAGE: &str = "\
usage: karakuri [--presets DIR] [--store DIR] [--mcp PORT] [GEOMETRY.kir RENDERER.kir]

  The console, with a deck behind it. Both paths or neither: a Set is an L1 and
  an L4, and with neither the pair that ships in the preset library is played.

  --presets DIR   the shipped preset library. Given, it is used and a directory
                  that is not there is refused. Not given, it is looked for
                  beside this binary — an .app bundle's Resources, a prefix
                  install's share/karakuri, a portable examples/ — and last in
                  the workspace this binary was compiled in. Which one answered
                  is printed at startup. With none, there is no default pair
                  and the two paths have to be given.
  --store DIR     where the Library bay reads Sets and arrangements from, where
                  a save goes, and where the scratch each deck runs from is
                  written. Defaults to .karakuri beside the session.
  --mcp PORT      serve the Model Context Protocol on 127.0.0.1:PORT, so a
                  model can read a deck's procedure, rewrite it, rewire an
                  input and keep what a deck is playing. Loopback only, and
                  0 takes an ephemeral port and prints the one it got.

  Any flag may be given before or after the pair.

  This is not `karakuri-cli`'s command line and does not try to be — that one
  has the audio, the MIDI and the rest of the flags, and its parser is its own.
  It reads the same two directories, and `--store` and `--mcp` mean the same
  thing to both. See `cargo run -p karakuri-cli -- --help`.";

/// Everything the command line settles: what plays, and the two directories
/// this program's data is in.
///
/// One value rather than three returns, because the three are decided together
/// and one of them decides another: with no preset library there is no default
/// pair, so [`Sources`] cannot be settled before `presets` is. Carrying the
/// resolution itself rather than only its directory is what lets the legend say
/// *which* candidate answered without asking again and getting a different
/// answer.
#[derive(Debug)]
pub(crate) struct Launch {
    pub(crate) sources: Sources,
    /// Where the Library bay reads and a save writes — `--store`, or
    /// [`karakuri_environment::places::STORE`].
    pub(crate) store: std::path::PathBuf,
    /// The preset library, and which of the places it was. `None` is a machine with
    /// no library on it, which is a state rather than a failure: it is fatal only
    /// for a run that needed a default pair, and the Library's preset tier is
    /// simply empty. See [`karakuri_environment::places::presets`].
    pub(crate) presets: Option<karakuri_environment::places::Presets>,
    /// The port a model reaches this run on, or `None` for a run that serves
    /// nothing — `--mcp PORT`.
    ///
    /// A port and not an open server, because the two are settled in different
    /// places: this function reads a command line and cannot bind a socket without
    /// either failing here or handing back something a `--help` run would have to
    /// close again. [`main`] binds it, before the window, for the reason the
    /// working copies are made there.
    pub(crate) mcp: Option<u16>,
}

/// The command line, read. Two paths or none, and two flags that are not about
/// paths; `--help` or `-h` prints [`USAGE`]; anything else is a refusal that
/// prints it.
///
/// A free function over an iterator rather than a read of `std::env::args`
/// inside [`main`], for the reason [`karakuri_environment`]'s refusals are free
/// functions: `main` cannot be called from a test and a refusal nobody can
/// reach is a refusal nobody checked. See
/// `a_set_is_two_paths_or_none_and_anything_else_is_refused` and
/// `the_two_flags_say_where_the_data_is_and_may_sit_on_either_side_of_the_pair`.
///
/// It reaches a disk now, which it did not before: resolving a presets root is
/// existence checks on up to four directories. That is not a purity this
/// function had for its own sake — it had it because the answer was a compiled
/// constant — and the alternative is `main` doing the resolution and this
/// function returning something that is not yet an answer, which puts the one
/// refusal an operator will actually meet back out of a test's reach.
pub(crate) fn sources_from<I: IntoIterator<Item = String>>(args: I) -> Result<Launch, String> {
    let args: Vec<String> = args.into_iter().collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        return Err(String::new());
    }
    // **A flag on either side of the pair**, which is what a pass over the
    // whole line buys and a `match` on the slice could not: an operator types
    // the flags in whatever order they think of them, and `karakuri-cli`
    // accepts `--store` before or after its own command for the same reason
    // (`list_sets_prints_and_is_never_a_run`).
    let mut named: Option<std::path::PathBuf> = None;
    let mut store: Option<std::path::PathBuf> = None;
    let mut mcp: Option<u16> = None;
    let mut paths: Vec<String> = Vec::new();
    let mut rest = args.into_iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--presets" => {
                named = Some(std::path::PathBuf::from(value_for("--presets", &mut rest)?))
            }
            "--store" => store = Some(std::path::PathBuf::from(value_for("--store", &mut rest)?)),
            // **The same two silences the other two flags refuse**, because
            // [`value_for`] is underneath this one as well: a `--mcp` at the
            // end of the line does not fall back to a port, and `--mcp
            // --store x` does not read `--store` as a number. What this adds
            // is the third — a value that is not a port — which is
            // [`number_for`]'s and is why that function exists here at all.
            "--mcp" => mcp = Some(number_for("--mcp", "a port number", &mut rest)?),
            // **An unknown option is not a path.** Without this a `--prests`
            // typo becomes the first half of a Set and is reported as a file
            // that will not open, which sends the operator looking at their
            // disk for a mistake they made on the command line.
            other if other.starts_with('-') => return Err(format!("unknown option `{other}`")),
            _ => paths.push(arg),
        }
    }

    let presets = karakuri_environment::places::presets(named.as_deref())?;
    let sources = match paths.as_slice() {
        [] => match &presets {
            Some(presets) => Sources::under(&presets.dir),
            // The one place where having no preset library is fatal rather
            // than empty: there is nothing to open on, and the alternative to
            // saying so is a black window. The Library bay's own answer to the
            // same fact is a tier with nothing in it.
            None => return Err(karakuri_environment::places::no_launch_pair()),
        },
        [l1, l4] => Sources {
            l1: std::path::PathBuf::from(l1),
            l4: std::path::PathBuf::from(l4),
        },
        [one] => {
            return Err(format!(
                "one path given (`{one}`) and a Set needs two: a geometry and a renderer"
            ))
        }
        many => {
            return Err(format!(
                "{} paths given and a Set is built from two: a geometry and a renderer",
                many.len()
            ))
        }
    };
    Ok(Launch {
        sources,
        store: store
            .unwrap_or_else(|| std::path::PathBuf::from(karakuri_environment::places::STORE)),
        presets,
        mcp,
    })
}

/// The value after a flag, refused rather than defaulted or swallowed.
///
/// `karakuri-cli`'s `value_for` is the same function with the same two silences
/// written on it, and this is the second surface rather than a copy with a
/// different opinion: a flag at the end of the line with nothing after it must
/// not fall back, and a flag whose value is missing must not eat the next flag
/// — `--presets --store x` reading `--store` as a directory would then blame
/// `x` for being an unknown option.
///
/// Two of the three flags here take a directory, and a directory beginning with
/// `-` is a path an operator can still name as `./-odd`. The third takes a
/// number and reads it through [`number_for`], which is where the parse and its
/// refusal are — so this function goes on answering one question.
pub(crate) fn value_for(
    flag: &str,
    rest: &mut impl Iterator<Item = String>,
) -> Result<String, String> {
    match rest.next() {
        Some(value) if !value.starts_with('-') => Ok(value),
        Some(value) => Err(format!(
            "`{flag}` was given no value — `{value}` is an option, not one"
        )),
        None => Err(format!("`{flag}` needs a value")),
    }
}

/// The number after a flag, refused rather than defaulted, swallowed or
/// clamped.
///
/// [`value_for`] with a parse behind it, which is `karakuri-cli`'s `number_for`
/// spelled a second time for that file's reason: it is a binary with no library
/// target and there is nothing to call. The three refusals are the three that
/// program gives — no value, a value that is the next flag, and a value that is
/// not a number — so `--mcp` on this command line and `--mcp` on that one are
/// wrong in the same words.
///
/// A port beginning with `-` is caught by the first two rather than by the
/// parse, which is the right refusal and not a lucky one: `--mcp -1` is a line
/// where the value is missing far more often than it is a negative number
/// somebody meant.
pub(crate) fn number_for<T: std::str::FromStr>(
    flag: &str,
    what: &str,
    rest: &mut impl Iterator<Item = String>,
) -> Result<T, String> {
    let value = value_for(flag, rest)?;
    value
        .parse()
        .map_err(|_| format!("`{flag} {value}` — expected {what}"))
}
