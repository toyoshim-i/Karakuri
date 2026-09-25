//! Where this program's two directories are, when nobody has said.
//!
//! # The first row of P-0096's table, which never had an address
//!
//! Where the material lives and who writes each place is
//! `docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md`,
//! and the table is there rather than here. It was written out in this header
//! and in [`crate::scratch`]'s, and two copies of one rule is two things to
//! keep in step: both said the operator's library was written by `--save-set`
//! *"and nothing else"* while the `k` key, the panel and MCP's `save_set` were
//! all writing it, and both had to be corrected at once when the store gained
//! an extension
//! (`docs/adr/0231-a-sets-two-forms-take-two-extensions-and-the-store-holds-only-the-resolved-one.md`
//! counts that as a cost it paid twice). What is left here is the sentence this
//! module is about, which the principle states and does not address.
//!
//! That table has said `examples/` since it was written, and `examples/`
//! relative to *what* was never in it. It was the repository's own directory,
//! reached by whichever program happened to be run from the repository — and
//! [`crate::mcp`]'s own header records what that cost once already, when a
//! model was handed write access to it. This module is that missing address:
//! the app presets are a directory this process is told about or goes looking
//! for, and the store is the other one.
//!
//! # Why this is here and not in either binary
//!
//! A directory on a disk is outside this process, which is the charter in
//! `lib.rs` and ADR-0215 behind it. The immediate reason is narrower and is two
//! transcriptions of one string: `karakuri-cli`'s private `DEFAULT_STORE` and
//! the panel's `const STORE`, both `.karakuri`, which
//! [ADR-0214](../../../docs/adr/0214-the-program-moves-out-of-the-cli-and-two-thin-binaries-sit-over-it.md)
//! named as a transcription the move *deletes rather than carries*. The panel's
//! own doc comment said that deleting it had to wait for
//! `karakuri-cli/src/main.rs` to divide. It did not: the two programs share a
//! package already, and a constant put in the package both reach is the
//! deletion, with no division needed. That comment was wrong about its own
//! blocker for as long as it stood, and nothing of it survives here except the
//! reason for the *value* — see [`STORE`].
//!
//! Each binary keeps its own parser. Nothing here reads `std::env::args` or
//! knows a flag's spelling except to name one in a refusal: `karakuri` has two
//! positional paths and two flags, `karakuri-cli` has thirty-odd flags and no
//! `--presets` at all, and a module here that knew which surface called it
//! would be the boundary drawn in the wrong place.
//!
//! # A path an operator typed is theirs
//!
//! [`presets`] refuses a `--presets` that is not there rather than searching
//! past it. `karakuri-cli` refuses a missing `--store` the same way, for
//! `docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md`'s
//! reason: a program that quietly played something else would leave the
//! operator reading a window that disagrees with the command line they typed,
//! with nothing anywhere saying which one won.
//!
//! And nothing found is a value rather than an error — [`presets`] answers
//! `Ok(None)`. A machine with no preset library is a machine with no preset
//! library; the panel still runs on two paths given by hand, and it is the
//! *caller* that decides whether the state it is in needs one. What must not
//! happen is silence, which is what [`no_preset_library`] is for.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::setfile::AUTHORING_SUFFIX;

/// Where the store lives when nothing says otherwise, for both programs.
///
/// A directory in the working tree rather than under `$HOME`: a session's
/// material belongs beside the session, and a global store shared by every run
/// is a decision an operator should make rather than inherit. That was
/// `karakuri-cli`'s reason for the value and it is unchanged by the move; what
/// changed is that there is one of it.
///
/// Unlike a presets root this is not searched for and not existence-checked
/// here. A store is a place things are *written*, so a run that means to write
/// one creates it, and `karakuri-cli`'s read-only flags each refuse a root that
/// is not there rather than establishing one to report that it is empty.
pub const STORE: &str = ".karakuri";

/// The leaf directory a preset library is called, in every candidate below and
/// in `docs/`'s own prose. Named once so that the three paths built from it
/// cannot drift apart.
const LIBRARY: &str = "examples";

/// The workspace this binary was compiled in, which is the last candidate and
/// the only one that is not a function of where the binary now is.
///
/// The one `CARGO_MANIFEST_DIR` in this workspace that is not in a test,
/// deliberately and with the check that makes it safe: it is tried last, it is
/// asked [`is_a_library`] like every other candidate, and on a machine that is
/// not the build machine it simply fails. What it buys is that `cargo run`
/// finds the repository's own presets from whatever directory it was started
/// in, which is what this program did before it could be installed at all.
const WORKSPACE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

/// Which of the places answered, so a program can say it rather than describe
/// what it probably did.
///
/// This exists because a startup line that reads *"presets: the ones that ship
/// with the program"* is a sentence about a search, and a sentence about a
/// search goes stale the day the search changes. The panel's legend was found
/// lying five ways about itself on 2026-08-30 and every line of it is derived
/// now; this is what lets the presets line be derived too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Found {
    /// `--presets DIR`. Not searched for, not fallen back from.
    Given,
    /// `<exe dir>/../Resources/examples` — a macOS `.app` bundle.
    Bundle,
    /// `<exe dir>/../share/karakuri/examples` — a unix prefix install.
    Prefix,
    /// `<exe dir>/examples` — a portable directory, unpacked anywhere.
    Beside,
    /// The workspace tree this binary was compiled in — the development entry.
    /// Last, and named as what it is rather than hidden: a released build made on
    /// somebody else's machine fails it, which is the intended outcome and not a
    /// bug to be worked around.
    Workspace,
}

impl Found {
    /// How the root was arrived at, in the words a startup line prints.
    ///
    /// A phrase rather than a sentence, so the caller supplies the path and the
    /// punctuation: what is printed is the path the resolution *returned* beside
    /// this, and neither half is a claim about the other.
    pub fn how(self) -> &'static str {
        match self {
            Found::Given => "named with `--presets`",
            Found::Bundle => "found in this app bundle, at `../Resources/examples`",
            Found::Prefix => "found in a unix prefix install, at `../share/karakuri/examples`",
            Found::Beside => "found beside the binary, at `examples`",
            Found::Workspace => "found in the workspace this binary was compiled in",
        }
    }

    /// How the plugin directory was arrived at, in the words a startup line prints.
    pub fn how_plugin(self) -> &'static str {
        match self {
            Found::Given => "named with `--plugins` or `KARAKURI_PLUGINS_DIR`",
            Found::Bundle => "found in this app bundle, at `../PlugIns`",
            Found::Prefix => "found in a unix prefix install, at `../lib/karakuri/plugins`",
            Found::Beside => "found beside the binary, at `plugins`",
            Found::Workspace => "found in the workspace this binary was compiled in",
        }
    }
}

/// A preset library: the directory, and which of [`Found`]'s places it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Presets {
    pub dir: PathBuf,
    pub found: Found,
}

/// The presets root for this run: what `--presets` named, or the first of
/// the four places that has a library in it.
///
/// Three answers and they are three different things, which is why this is a
/// `Result<Option<_>>` rather than either alone:
///
/// - `Ok(Some(_))` — a library, and which place it came from.
/// - `Err(_)` — `--presets` named a directory that is not there. A typed path
///   is refused rather than searched past; see the module header.
/// - `Ok(None)` — there is no preset library on this machine. A state, not a
///   failure: what the caller does about it depends on whether it needed one,
///   and only the caller knows that. Say [`no_preset_library`] about it.
///
/// The search is against [`std::env::current_exe`], because a POSIX process
/// cannot ask where it is by any other means and an installed program has to
/// find the files it was installed with. A platform that will not answer that
/// question at all leaves the first three candidates untried and the
/// development entry still standing, which is the right shape: those three are
/// *about* the binary's location, and one that has none has not got them.
pub fn presets(given: Option<&Path>) -> Result<Option<Presets>, String> {
    if let Some(dir) = given {
        // `exists`, not "has a Set in it": an empty directory an operator
        // typed is an empty library they typed, and this program is not
        // entitled to decide it meant somewhere else. What it must not do is
        // say nothing, and a path that is simply not there is the mistake that
        // actually gets made.
        return match dir.exists() {
            true => Ok(Some(Presets {
                dir: dir.to_path_buf(),
                found: Found::Given,
            })),
            false => Err(no_presets_at(dir)),
        };
    }
    let exe = std::env::current_exe().ok();
    let exe_dir = exe.as_deref().and_then(Path::parent);
    Ok(searched(exe_dir, Path::new(WORKSPACE)))
}

/// The search of [`presets`], over roots rather than over this process — which
/// is what makes every one of its four turns reachable from a test on a
/// temporary directory rather than from four real installs.
fn searched(exe_dir: Option<&Path>, workspace: &Path) -> Option<Presets> {
    let mut places: Vec<(PathBuf, Found)> = Vec::new();
    if let Some(exe_dir) = exe_dir {
        places.push((
            exe_dir.join("..").join("Resources").join(LIBRARY),
            Found::Bundle,
        ));
        places.push((
            exe_dir
                .join("..")
                .join("share")
                .join("karakuri")
                .join(LIBRARY),
            Found::Prefix,
        ));
        places.push((exe_dir.join(LIBRARY), Found::Beside));
    }
    places.push((workspace.join(LIBRARY), Found::Workspace));
    places
        .into_iter()
        .find(|(dir, _)| is_a_library(dir))
        .map(|(dir, found)| Presets { dir, found })
}

/// A directory with at least one `.kset` in it, which is what a candidate has
/// to be to answer — and existence alone is not enough.
///
/// # It asked for a `.kir` until today, and that was the right question then
///
/// A presets root held parts and nothing else: thirty-one `.kir` files, no
/// listing of any kind, and the panel opened on a pair of paths given by hand.
/// The only thing that could tell a preset library from another directory
/// wearing the same name was the material in it, and the material *was* parts —
/// so the parts were what to ask for. Twenty-one `.kset` files landed in
/// `examples/` today, the authoring form of a Set
/// ([`AUTHORING_SUFFIX`](crate::setfile::AUTHORING_SUFFIX), ADR-0229 and
/// ADR-0231), and with them the root became something that can be *asked what
/// it holds* rather than only opened by path.
///
/// A library is a listing of what you can put on a deck, and a directory of
/// parts is not one — `docs/manual/console.html`, *A folder scope reads Sets,
/// and a bundle is not a third thing*: a `.kir` is a single node's source,
/// carrying no layer, no slot and no name a person chose, and nothing in the
/// vocabulary takes one. So a root with only those in it has no row to give the
/// Library bay and no file to hand [`crate::setfile::resolve`]. What changed is
/// the answer to *what makes this a library*, not whether the old answer was
/// true when it was given.
///
/// `examples/` holds both, so no run resolves a different directory today than
/// it did yesterday. The meaning moved; no behaviour did.
///
/// # The trap that forced a content test at all, which a `.kset` still catches
///
/// A cargo workspace builds its example binaries into
/// `<target>/<profile>/examples`, and a debug build of the panel sits in
/// `<target>/<profile>`. So `<exe dir>/examples` — the portable candidate —
/// *exists* for every `cargo run` in this repository and holds four hundred
/// object files and no presets. On an existence test it would win over the
/// development entry every time, and the program would resolve a presets root,
/// print it, and then fail to open `drift_shell.kir` inside it: silently wrong
/// material, then a puzzle (P-0094). One directory read per candidate is what
/// makes the two directories that share a name distinguishable by what is in
/// them, which is the only thing that tells them apart — and it is a *narrower*
/// question now than it was, so nothing that failed it before passes it now.
///
/// One entry, not a count: this asks *is this a preset library*, and a library
/// with one Set in it is one. It is the same question [`Presets::list_sets`]
/// answers with rows — a candidate is a library exactly when that listing would
/// not be empty — and both spell it through [`set_id`] so the two cannot drift
/// into disagreeing about what a Set file is.
fn is_a_library(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        set_id(&entry.file_name()).is_some() && entry.file_type().is_ok_and(|kind| !kind.is_dir())
    })
}

/// The id a Set file in a presets root goes by: the file's stem, which is its
/// name with [`AUTHORING_SUFFIX`](crate::setfile::AUTHORING_SUFFIX) taken off.
/// `None` for every name this layout does not claim.
///
/// This is `Store::list_sets`' rule with the store's suffix swapped for the
/// authoring one, and deliberately the same rule: an id is what a Set is *named
/// by* everywhere it appears, so a preset listed as `drift_cloud` is the
/// `drift_cloud` an operator reads back in `my sets` after taking it in. A
/// second convention here — a title read out of the file, a path shown whole —
/// would be a Set with two names and a bay that shows one of them.
///
/// Case-sensitive, unlike the extension test this replaced.
/// [`crate::setfile::resolve`] refuses anything not ending in exactly `.kset`,
/// and a listing that offered a `FOO.KSET` row would be offering a row whose
/// only operation refuses it.
///
/// A name that is not UTF-8 fails `to_str` and falls out here with everything
/// else the layout does not claim — no lossy repair, and no unwrap for a
/// hostile name to trip.
fn set_id(name: &OsStr) -> Option<&str> {
    name.to_str()?.strip_suffix(AUTHORING_SUFFIX)
}

/// The name a procedure in a presets root goes by: the file's stem, with `.kir`
/// taken off. `None` for every name this layout does not claim.
///
/// [`set_id`]'s rule one extension along, and the same rule for its reason: a
/// row is drawn under the name a file is *called*, so the `orbit_wide` in this
/// bay is the `orbit_wide` the strip reads back after a load.
///
/// The suffix is `karakuri_store::Store::PROCEDURE_FILE_SUFFIX`, which is where
/// a kept procedure's extension is spelled — the two tiers hold the same kind
/// of file (ADR-0227, ADR-0338), and a second literal here would be this tier
/// listing files the other one could not.
fn procedure_name(name: &OsStr) -> Option<&str> {
    name.to_str()?
        .strip_suffix(karakuri_store::Store::PROCEDURE_FILE_SUFFIX)
}

/// A Set a presets root holds, as [`Presets::list_sets`] found it.
///
/// Both fields, because the caller needs both and can derive neither safely: a
/// row is drawn under [`PresetSet::id`] and taken in from [`PresetSet::file`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetSet {
    /// What a store would call this Set — the file's stem. The id the row shows,
    /// and the id it still has once it has been taken in.
    pub id: String,
    /// The file itself, which is what [`crate::setfile::resolve`] takes.
    ///
    /// Handed over rather than left to be rebuilt: `dir.join(format!("{id}{}",
    /// AUTHORING_SUFFIX))` at a call site is this module's naming rule written a
    /// second time, in a crate that cannot see [`set_id`] and where no test here
    /// can fail when the two spellings part company. The listing read the name off
    /// a disk; it may as well say which one it read.
    pub file: PathBuf,
}

/// A procedure a presets root ships, as [`Presets::list_procedures`] found it:
/// the name a row is drawn under, the file the load reads, and the layer it
/// declares.
///
/// [`PresetSet`]'s shape with one field more, and that field is the difference
/// between the two listings: a Set's file says which layers it fills in its own
/// `slot` records, and a procedure says its one `kind` in the source. So this
/// listing opens each file where [`Presets::list_sets`] opens none — one small
/// read per row, on the press that builds a listing and never on a frame
/// ([P-0091](../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetProcedure {
    /// The file's stem, which is the name the row shows and the name a load names
    /// it by.
    pub name: String,
    /// The file itself, which is what a load reads and writes into the deck's
    /// scratch.
    pub file: PathBuf,
    /// What layer it implements, one of [`crate::history::LAYERS`], or `None` for a
    /// file that declares no `kind` at all.
    ///
    /// `None` is a row and not a skip, which is the choice worth stating: a `.kir`
    /// in this directory with no `kind` line is a file somebody put there, and
    /// dropping it from the listing would answer *what ships here* with a file
    /// missing and nothing said. What it costs is a row with no badge, and a load
    /// off it is refused by name — see this crate's
    /// [`crate::history::declared_kind`], which is the one scanner for the line.
    pub kind: Option<&'static str>,
}

impl Presets {
    /// What this root holds that can go on a deck: the `.kset` files
    /// directly in it, each under the id a store would give it, in ascending id
    /// order.
    ///
    /// `Store::list_sets` is the shape this follows, because the two answer the
    /// same question about two directories and the Library bay draws their rows
    /// the same way. What is carried over from it, and why:
    ///
    /// - Ordering on the id, and nothing else. `read_dir` hands back
    ///   whatever the filesystem felt like, which is not an order and can
    ///   differ between two calls on an unchanged directory; ids are unique
    ///   within one directory by construction, so ordering on them is total and
    ///   repeatable. A shipped library is a fixed list an operator will learn
    ///   the shape of, and a list that shuffles between runs is one nobody can.
    /// - A name the layout does not claim is skipped, not repaired. An
    ///   editor's backup, a `.tmp` from a copy that died, a `.kbset` somebody
    ///   dropped in, a subdirectory named like a Set file — those belong to
    ///   whoever put them there, and reporting one as a Set under a truncated
    ///   id would invent a row that [`crate::setfile::resolve`] refuses when
    ///   pressed.
    /// - The name is all that is read. Nothing here opens a file: a
    ///   listing that parsed twenty-two Sets to draw twenty-two rows would pay
    ///   for a load nobody asked for, and a malformed one is a refusal at the
    ///   moment it is taken in, where the operator can see which row they
    ///   pressed.
    ///
    /// One directory deep and not a walk. The scope is *this* root, and a
    /// subdirectory under it is a folder somebody made — the folder scope is
    /// what asks about those, and it is owed an operation that does not exist
    /// yet (`docs/manual/console.html`, *What is owed is the asking*).
    ///
    /// No time beside a row, which is the one field of `SetEntry` dropped
    /// rather than mirrored. A store's Sets carry when they were written
    /// because the operator wrote them; a shipped file's mtime is when it was
    /// installed or checked out, which is a fact about this machine's disk and
    /// not about the Set. A column that means one thing under `my sets` and
    /// another under `presets` is worse than no column.
    ///
    /// # A root that has gone since it was resolved is an error
    ///
    /// Not an empty listing, on `Store::list_sets`' reasoning exactly: the
    /// caller asked what is there and we have no answer, and an empty `Vec`
    /// would say *"this library holds nothing"* to a question that could not be
    /// read. It is also the likelier accident here than it is for a store — the
    /// store is a directory this program writes, while a presets root is
    /// somebody's checkout or an unpacked bundle that can be moved, renamed or
    /// deleted between the resolution at startup and a keypress an hour later.
    ///
    /// An empty root is a value, not an error, and reachable: `--presets`
    /// takes a directory an operator typed without asking what is in it, so a
    /// root of parts and no Sets lists nothing and says so with `Ok(vec![])`.
    ///
    /// The error is a `String` because that is what everything in this module
    /// refuses in — see [`no_presets_at`] — and both binaries print rather than
    /// match on it.
    pub fn list_sets(&self) -> Result<Vec<PresetSet>, String> {
        let entries = std::fs::read_dir(&self.dir).map_err(|e| self.cannot_be_listed(&e))?;
        let mut out = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| self.cannot_be_listed(&e))?;
            let name = entry.file_name();
            let Some(id) = set_id(&name) else {
                continue;
            };
            if entry
                .file_type()
                .map_err(|e| self.cannot_be_listed(&e))?
                .is_dir()
            {
                continue;
            }
            out.push(PresetSet {
                id: id.to_string(),
                file: entry.path(),
            });
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    /// What this root ships that can go over a layer: the `.kir` files directly in
    /// it, each under the name a row is drawn with and the kind it declares, in
    /// ascending name order.
    ///
    /// [`Presets::list_sets`]' shape one extension along, and everything that
    /// method carries transfers: the order is the name's and nothing else's, a name
    /// the layout does not claim is skipped rather than repaired, the scope is
    /// *this* root and not a walk, no time is carried beside a row, and a root that
    /// has gone since it was resolved is an error where an empty one is a value.
    ///
    /// The one sentence that does not transfer is *the name is all that is read*. A
    /// badge is a procedure's `kind` and a `kind` is a line of the file, so this
    /// opens each one and scans it with [`crate::history::declared_kind`] — the
    /// same scanner the edit history files a snapshot under and the same one MCP
    /// resolves an address with, so a row's badge and an address's layer cannot
    /// come apart. That is a read per row rather than none, paid on the press that
    /// builds a listing and never on a frame, and it compiles nothing (ADR-0338,
    /// [P-0091](../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)).
    ///
    /// A file that will not read is skipped and a file with no `kind` is not. The
    /// first is a fact about this machine's disk at this instant and there is
    /// nothing to draw for it; the second is a file somebody shipped and it gets a
    /// row with no badge, which is [`PresetProcedure::kind`]'s own note.
    pub fn list_procedures(&self) -> Result<Vec<PresetProcedure>, String> {
        let entries = std::fs::read_dir(&self.dir).map_err(|e| self.cannot_be_listed(&e))?;
        let mut out = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| self.cannot_be_listed(&e))?;
            let name = entry.file_name();
            let Some(name) = procedure_name(&name) else {
                continue;
            };
            if entry
                .file_type()
                .map_err(|e| self.cannot_be_listed(&e))?
                .is_dir()
            {
                continue;
            }
            let path = entry.path();
            let Ok(source) = std::fs::read(&path) else {
                continue;
            };
            out.push(PresetProcedure {
                name: name.to_string(),
                file: path,
                kind: crate::history::declared_kind(&source),
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    /// The presets root could not be read, in the shape [`no_presets_at`] refuses
    /// in: it names the path, and it carries the reason the filesystem gave rather
    /// than translating it into a guess. A root that resolved at startup and cannot
    /// be read now is a fact about the machine since then, and [`Found::how`] is
    /// what says which of the places it was — so the operator can tell a moved
    /// checkout from a bundle they deleted.
    fn cannot_be_listed(&self, why: &std::io::Error) -> String {
        format!(
            "cannot list the preset library at `{}` ({}): {why}",
            self.dir.display(),
            self.found.how()
        )
    }
}

/// A `--presets` that is not there, in the words both programs would say it in.
///
/// `karakuri-cli`'s missing-store refusal is the sentence this is shaped after
/// — *"no store at `…` — nothing has ever been kept there, and nothing was
/// created to find that out. Check `--store` …"* — and the two halves that
/// matter are carried over: it names the path, and it says that nothing was
/// created to discover the path was empty. A flag that only reads must not
/// leave a directory behind to report that it found none.
pub fn no_presets_at(dir: &Path) -> String {
    format!(
        "no presets at `{}` — nothing is there, and nothing was created to find that out. \
         Check `--presets`, or leave it off and this program looks for the library it was \
         installed with.",
        dir.display()
    )
}

/// Returns the message displayed when no preset library directory is found.
pub fn no_preset_library() -> String {
    String::from(
        "no preset library: none of the places this program looks holds one, so nothing \
         ships with it on this machine and nothing was created to say so. Name one with \
         `--presets DIR`.",
    )
}

/// Nothing to play: no preset library, and no paths given either.
///
/// The one place where [`no_preset_library`]'s state is a refusal rather than a
/// remark — a program whose default material is drawn from a library that is
/// not there has nothing to open on, and the alternative to saying so is a
/// black window. Both ways out are named, because both are real: install or
/// name a library, or give the pair on the command line and never need one.
pub fn no_launch_pair() -> String {
    String::from(
        "nothing to play: there is no preset library, so there is no default pair. Name a \
         library with `--presets DIR`, or give the two paths — a geometry and a renderer — \
         on the command line.",
    )
}

/// The leaf directory an output plugin library is called, in every candidate below.
const PLUGINS_LEAF: &str = "plugins";

/// An output plugin directory: the directory, and which of [`Found`]'s places it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plugins {
    pub dir: PathBuf,
    pub found: Found,
}

impl Plugins {
    /// Lists all executable plugin binaries in this directory, sorted deterministically.
    pub fn list_executables(&self) -> Result<Vec<PathBuf>, String> {
        let entries = std::fs::read_dir(&self.dir).map_err(|e| self.cannot_be_listed(&e))?;
        let mut out = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| self.cannot_be_listed(&e))?;
            let path = entry.path();
            if path.is_file() {
                #[cfg(target_os = "windows")]
                let is_exec = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map_or(false, |ext| ext.eq_ignore_ascii_case("exe"));
                #[cfg(not(target_os = "windows"))]
                let is_exec = {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        std::fs::metadata(&path)
                            .map(|m| m.permissions().mode() & 0o111 != 0)
                            .unwrap_or(false)
                    }
                    #[cfg(not(unix))]
                    {
                        true
                    }
                };
                if is_exec {
                    out.push(path);
                }
            }
        }
        out.sort();
        Ok(out)
    }

    fn cannot_be_listed(&self, why: &std::io::Error) -> String {
        format!(
            "cannot list the plugin directory at `{}` ({}): {why}",
            self.dir.display(),
            self.found.how_plugin()
        )
    }
}

/// Resolves the plugin directory for this run: what was given, `KARAKURI_PLUGINS_DIR`,
/// or the first of the four places that has a plugins directory.
pub fn plugins(given: Option<&Path>) -> Result<Option<Plugins>, String> {
    if let Some(dir) = given {
        return match dir.is_dir() {
            true => Ok(Some(Plugins {
                dir: dir.to_path_buf(),
                found: Found::Given,
            })),
            false => Err(no_plugins_at(dir)),
        };
    }
    if let Ok(env_dir) = std::env::var("KARAKURI_PLUGINS_DIR") {
        let path = PathBuf::from(env_dir);
        return match path.is_dir() {
            true => Ok(Some(Plugins {
                dir: path,
                found: Found::Given,
            })),
            false => Err(no_plugins_at(&path)),
        };
    }
    let exe = std::env::current_exe().ok();
    let exe_dir = exe.as_deref().and_then(Path::parent);
    Ok(searched_plugins(exe_dir, Path::new(WORKSPACE)))
}

fn searched_plugins(exe_dir: Option<&Path>, workspace: &Path) -> Option<Plugins> {
    let mut places: Vec<(PathBuf, Found)> = Vec::new();
    if let Some(exe_dir) = exe_dir {
        places.push((exe_dir.join("..").join("PlugIns"), Found::Bundle));
        places.push((
            exe_dir
                .join("..")
                .join("lib")
                .join("karakuri")
                .join(PLUGINS_LEAF),
            Found::Prefix,
        ));
        places.push((
            exe_dir
                .join("..")
                .join("share")
                .join("karakuri")
                .join(PLUGINS_LEAF),
            Found::Prefix,
        ));
        places.push((exe_dir.join(PLUGINS_LEAF), Found::Beside));
    }
    places.push((workspace.join(PLUGINS_LEAF), Found::Workspace));
    places
        .into_iter()
        .find(|(dir, _)| is_a_plugin_directory(dir))
        .map(|(dir, found)| Plugins { dir, found })
}

fn is_a_plugin_directory(dir: &Path) -> bool {
    dir.is_dir()
}

/// A plugin directory that is not there.
pub fn no_plugins_at(dir: &Path) -> String {
    format!(
        "no plugin directory at `{}` — nothing is there. Check `--plugins` or `KARAKURI_PLUGINS_DIR`.",
        dir.display()
    )
}

#[cfg(test)]
mod tests;
