//! Where this program's two directories are, when nobody has said.
//!
//! # The first row of P-0096's table, which never had an address
//!
//! Where the material lives and who writes each place is
//! `docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md`,
//! and **the table is there rather than here**. It was written out in this
//! header and in [`crate::scratch`]'s, and two copies of one rule is two things
//! to keep in step: both said the operator's library was written by
//! `--save-set` *"and nothing else"* while the `k` key, the panel and MCP's
//! `save_set` were all writing it, and both had to be corrected at once when
//! the store gained an extension
//! (`docs/adr/0231-a-sets-two-forms-take-two-extensions-and-the-store-holds-only-the-resolved-one.md`
//! counts that as a cost it paid twice). What is left here is the sentence this
//! module is about, which the principle states and does not address.
//!
//! **That table has said `examples/` since it was written, and `examples/`
//! relative to *what* was never in it.** It was the repository's own directory,
//! reached by whichever program happened to be run from the repository — and
//! [`crate::mcp`]'s own header records what that cost once already, when a
//! model was handed write access to it. This module is that missing address:
//! **the app presets are a directory this process is told about or goes
//! looking for, and the store is the other one.**
//!
//! # Why this is here and not in either binary
//!
//! A directory on a disk is outside this process, which is the charter in
//! `lib.rs` and ADR-0215 behind it. The immediate reason is narrower and is
//! two transcriptions of one string: `karakuri-cli`'s private `DEFAULT_STORE`
//! and the panel's `const STORE`, both `.karakuri`, which
//! [ADR-0214](../../../docs/adr/0214-the-program-moves-out-of-the-cli-and-two-thin-binaries-sit-over-it.md)
//! named as a transcription the move *deletes rather than carries*. The
//! panel's own doc comment said that deleting it had to wait for
//! `karakuri-cli/src/main.rs` to divide. It did not: the two programs share a
//! package already, and a constant put in the package both reach is the
//! deletion, with no division needed. That comment was wrong about its own
//! blocker for as long as it stood, and nothing of it survives here except
//! the reason for the *value* — see [`STORE`].
//!
//! **Each binary keeps its own parser.** Nothing here reads `std::env::args`
//! or knows a flag's spelling except to name one in a refusal: `karakuri` has
//! two positional paths and two flags, `karakuri-cli` has thirty-odd flags and
//! no `--presets` at all, and a module here that knew which surface called it
//! would be the boundary drawn in the wrong place.
//!
//! # A path an operator typed is theirs
//!
//! [`presets`] refuses a `--presets` that is not there rather than searching
//! past it. `karakuri-cli` refuses a missing `--store` the same way, for
//! `docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md`'s reason: a
//! program that quietly played something else would leave the operator
//! reading a window that disagrees with the command line they typed, with
//! nothing anywhere saying which one won.
//!
//! And **nothing found is a value rather than an error** — [`presets`] answers
//! `Ok(None)`. A machine with no preset library is a machine with no preset
//! library; the panel still runs on two paths given by hand, and it is the
//! *caller* that decides whether the state it is in needs one. What must not
//! happen is silence, which is what [`no_preset_library`] is for.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::setfile::AUTHORING_SUFFIX;

/// **Where the store lives when nothing says otherwise**, for both programs.
///
/// A directory in the working tree rather than under `$HOME`: a session's
/// material belongs beside the session, and a global store shared by every run
/// is a decision an operator should make rather than inherit. That was
/// `karakuri-cli`'s reason for the value and it is unchanged by the move; what
/// changed is that there is one of it.
///
/// Unlike a presets root this is **not** searched for and not existence-checked
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
/// **The one `CARGO_MANIFEST_DIR` in this workspace that is not in a test**,
/// deliberately and with the check that makes it safe: it is tried last, it is
/// asked [`is_a_library`] like every other candidate, and on a machine that is
/// not the build machine it simply fails. What it buys is that `cargo run` finds
/// the repository's own presets from whatever directory it was started in,
/// which is what this program did before it could be installed at all.
const WORKSPACE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

/// **Which of the places answered**, so a program can say it rather than
/// describe what it probably did.
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
    /// Last, and named as what it is rather than hidden: a released build made
    /// on somebody else's machine fails it, which is the intended outcome and
    /// not a bug to be worked around.
    Workspace,
}

impl Found {
    /// **How the root was arrived at, in the words a startup line prints.**
    ///
    /// A phrase rather than a sentence, so the caller supplies the path and
    /// the punctuation: what is printed is the path the resolution *returned*
    /// beside this, and neither half is a claim about the other.
    pub fn how(self) -> &'static str {
        match self {
            Found::Given => "named with `--presets`",
            Found::Bundle => "found in this app bundle, at `../Resources/examples`",
            Found::Prefix => "found in a unix prefix install, at `../share/karakuri/examples`",
            Found::Beside => "found beside the binary, at `examples`",
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

/// **The presets root for this run**: what `--presets` named, or the first of
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

/// **A directory with at least one `.kset` in it**, which is what a candidate
/// has to be to answer — and existence alone is not enough.
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
/// **A library is a listing of what you can put on a deck, and a directory of
/// parts is not one** — `docs/manual/console.html`, *A folder scope reads Sets,
/// and a bundle is not a third thing*: a `.kir` is a single node's source,
/// carrying no layer, no slot and no name a person chose, and nothing in the
/// vocabulary takes one. So a root with only those in it has no row to give the
/// Library bay and no file to hand [`crate::setfile::resolve`]. What changed is
/// the answer to *what makes this a library*, not whether the old answer was
/// true when it was given.
///
/// `examples/` holds both, so no run resolves a different directory today than
/// it did yesterday. The **meaning** moved; no behaviour did.
///
/// # The trap that forced a content test at all, which a `.kset` still catches
///
/// A cargo workspace builds its example binaries into `<target>/<profile>/examples`,
/// and a debug build of the panel sits in `<target>/<profile>`. So `<exe
/// dir>/examples` — the portable candidate — *exists* for every `cargo run` in
/// this repository and holds four hundred object files and no presets. On an
/// existence test it would win over the development entry every time, and the
/// program would resolve a presets root, print it, and then fail to open
/// `drift_shell.kir` inside it: silently wrong material, then a puzzle
/// (P-0094). One directory read per candidate is what makes the two
/// directories that share a name distinguishable by what is in them, which is
/// the only thing that tells them apart — and it is a *narrower* question now
/// than it was, so nothing that failed it before passes it now.
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

/// **The id a Set file in a presets root goes by**: the file's stem, which is
/// its name with [`AUTHORING_SUFFIX`](crate::setfile::AUTHORING_SUFFIX) taken
/// off. `None` for every name this layout does not claim.
///
/// This is `Store::list_sets`' rule with the store's suffix swapped for the
/// authoring one, and deliberately the same rule: an id is what a Set is
/// *named by* everywhere it appears, so a preset listed as `drift_cloud` is the
/// `drift_cloud` an operator reads back in `my sets` after taking it in. A
/// second convention here — a title read out of the file, a path shown whole —
/// would be a Set with two names and a bay that shows one of them.
///
/// **Case-sensitive**, unlike the extension test this replaced.
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

/// **A Set a presets root holds**, as [`Presets::list_sets`] found it.
///
/// Both fields, because the caller needs both and can derive neither safely: a
/// row is drawn under [`PresetSet::id`] and taken in from [`PresetSet::file`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetSet {
    /// What a store would call this Set — the file's stem. The id the row
    /// shows, and the id it still has once it has been taken in.
    pub id: String,
    /// The file itself, which is what [`crate::setfile::resolve`] takes.
    ///
    /// Handed over rather than left to be rebuilt: `dir.join(format!("{id}{}",
    /// AUTHORING_SUFFIX))` at a call site is this module's naming rule written
    /// a second time, in a crate that cannot see [`set_id`] and where no test
    /// here can fail when the two spellings part company. The listing read the
    /// name off a disk; it may as well say which one it read.
    pub file: PathBuf,
}

impl Presets {
    /// **What this root holds that can go on a deck**: the `.kset` files
    /// directly in it, each under the id a store would give it, in ascending id
    /// order.
    ///
    /// `Store::list_sets` is the shape this follows, because the two answer the
    /// same question about two directories and the Library bay draws their rows
    /// the same way. What is carried over from it, and why:
    ///
    /// - **Ordering on the id, and nothing else.** `read_dir` hands back
    ///   whatever the filesystem felt like, which is not an order and can
    ///   differ between two calls on an unchanged directory; ids are unique
    ///   within one directory by construction, so ordering on them is total and
    ///   repeatable. A shipped library is a fixed list an operator will learn
    ///   the shape of, and a list that shuffles between runs is one nobody can.
    /// - **A name the layout does not claim is skipped, not repaired.** An
    ///   editor's backup, a `.tmp` from a copy that died, a `.kbset` somebody
    ///   dropped in, a subdirectory named like a Set file — those belong to
    ///   whoever put them there, and reporting one as a Set under a truncated
    ///   id would invent a row that [`crate::setfile::resolve`] refuses when
    ///   pressed.
    /// - **The name is all that is read.** Nothing here opens a file: a
    ///   listing that parsed twenty-one Sets to draw twenty-one rows would pay
    ///   for a load nobody asked for, and a malformed one is a refusal at the
    ///   moment it is taken in, where the operator can see which row they
    ///   pressed.
    ///
    /// **One directory deep and not a walk.** The scope is *this* root, and a
    /// subdirectory under it is a folder somebody made — the folder scope is
    /// what asks about those, and it is owed an operation that does not exist
    /// yet (`docs/manual/console.html`, *What is owed is the asking*).
    ///
    /// **No time beside a row**, which is the one field of `SetEntry` dropped
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
    /// An **empty** root is a value, not an error, and reachable: `--presets`
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

    /// **The presets root could not be read**, in the shape [`no_presets_at`]
    /// refuses in: it names the path, and it carries the reason the filesystem
    /// gave rather than translating it into a guess. A root that resolved at
    /// startup and cannot be read now is a fact about the machine since then,
    /// and [`Found::how`] is what says which of the places it was — so the
    /// operator can tell a moved checkout from a bundle they deleted.
    fn cannot_be_listed(&self, why: &std::io::Error) -> String {
        format!(
            "cannot list the preset library at `{}` ({}): {why}",
            self.dir.display(),
            self.found.how()
        )
    }
}

/// **A `--presets` that is not there**, in the words both programs would say
/// it in.
///
/// `karakuri-cli`'s missing-store refusal is the sentence this is shaped
/// after — *"no store at `…` — nothing has ever been kept there, and nothing
/// was created to find that out. Check `--store` …"* — and the two halves that
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

/// **There is no preset library on this machine**, said once and out loud.
///
/// Not an error and not a silence. The four places were tried and none of them
/// holds one, which an operator can act on — by naming one — and cannot act on
/// if nobody says it. It names no path because there is no path to name: the
/// candidates are a function of where the binary is, and reciting four
/// speculative directories at somebody is a sentence about a search rather
/// than about their machine.
pub fn no_preset_library() -> String {
    String::from(
        "no preset library: none of the places this program looks holds one, so nothing \
         ships with it on this machine and nothing was created to say so. Name one with \
         `--presets DIR`.",
    )
}

/// **Nothing to play**: no preset library, and no paths given either.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory with a Set in it, at `path` — which is what a library is
    /// since [`is_a_library`] started asking for the listing rather than the
    /// parts. The part is written too, because a real root holds both and a
    /// fixture that held only the file under test would pass a check that
    /// happened to be looking at the wrong one.
    fn library(path: PathBuf) -> PathBuf {
        parts(path.clone());
        set(&path, "drift_cloud");
        path
    }

    /// A directory of parts and no listing — `.kir` files, which is every
    /// preset root there was before today and is no longer a library.
    fn parts(path: PathBuf) -> PathBuf {
        std::fs::create_dir_all(&path).expect("mkdir");
        std::fs::write(path.join("drift_shell.kir"), "kind L1\n").expect("write");
        path
    }

    /// One authoring Set file named `id`, with the two records a real one
    /// opens with.
    fn set(dir: &Path, id: &str) -> PathBuf {
        let file = dir.join(format!("{id}{AUTHORING_SUFFIX}"));
        std::fs::write(
            &file,
            format!(
                "{{\"t\":\"set\",\"id\":\"{id}\",\"v\":1}}\n\
                 {{\"t\":\"part\",\"layer\":\"L1\",\"path\":\"drift_shell.kir\"}}\n"
            ),
        )
        .expect("write");
        file
    }

    /// A root as [`presets`] would have answered with one, so the listing is
    /// asked of the thing the program actually holds.
    fn root(dir: PathBuf) -> Presets {
        Presets {
            dir,
            found: Found::Given,
        }
    }

    /// **A path an operator typed is the answer, and nothing else is
    /// consulted.**
    #[test]
    fn a_typed_presets_path_that_exists_is_the_root_and_says_it_was_given() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = library(tmp.path().join("shipped"));

        let found = presets(Some(&dir))
            .expect("a directory that is there is not a refusal")
            .expect("a directory that is there is a library");
        assert_eq!(found.dir, dir);
        assert_eq!(
            found.found,
            Found::Given,
            "a typed path was reported as something the program went looking for"
        );

        // And it is theirs even with nothing in it: an empty library an
        // operator named is not a reason to go and play something else.
        let empty = tmp.path().join("empty");
        std::fs::create_dir_all(&empty).expect("mkdir");
        assert_eq!(
            presets(Some(&empty)).expect("an empty directory is not a refusal"),
            Some(Presets {
                dir: empty,
                found: Found::Given
            })
        );
    }

    /// **A typed path that is not there is refused by name**, rather than
    /// searched past — which would play material the operator did not name and
    /// say nothing about it (P-0094).
    #[test]
    fn a_typed_presets_path_that_is_not_there_is_refused_naming_the_flag() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let missing = tmp.path().join("nowhere");

        let why = presets(Some(&missing)).expect_err(
            "a `--presets` that is not there was accepted, so the search would have quietly \
             answered with somewhere else",
        );
        assert_eq!(why, no_presets_at(&missing));
        assert!(
            why.contains("--presets"),
            "the refusal `{why}` does not name the flag the operator would have to fix"
        );
        assert!(
            why.contains(&missing.display().to_string()),
            "the refusal `{why}` does not name the path it refused"
        );
        assert!(
            !missing.exists(),
            "the refusal created the directory it was complaining about"
        );
    }

    /// **Each of the four places answers when it is the one that has a
    /// library**, and says which it was.
    #[test]
    fn each_candidate_answers_in_its_own_turn_and_names_itself() {
        // `<exe dir>` is `<root>/bin`, so `..` is a real directory in every
        // one of these rather than a path that only normalises to one.
        for (relative, expected) in [
            ("Resources/examples", Found::Bundle),
            ("share/karakuri/examples", Found::Prefix),
            ("bin/examples", Found::Beside),
        ] {
            let tmp = tempfile::tempdir().expect("tempdir");
            let exe_dir = tmp.path().join("bin");
            std::fs::create_dir_all(&exe_dir).expect("mkdir");
            let dir = library(tmp.path().join(relative));

            let found = searched(Some(&exe_dir), &tmp.path().join("no-workspace"))
                .unwrap_or_else(|| panic!("a library at {relative} was not found"));
            assert_eq!(
                found.found, expected,
                "a library at {relative} was reported as {:?}",
                found.found
            );
            assert!(
                std::fs::canonicalize(&found.dir).expect("canonicalize")
                    == std::fs::canonicalize(&dir).expect("canonicalize"),
                "{relative} answered with {}",
                found.dir.display()
            );
        }

        // The development entry, which is the one that is not a function of
        // where the binary is — and it answers with no binary location at all,
        // which is the platform that will not say where it is.
        let tmp = tempfile::tempdir().expect("tempdir");
        let workspace = tmp.path().join("tree");
        library(workspace.join("examples"));
        let found = searched(None, &workspace).expect("the workspace tree holds a library");
        assert_eq!(found.found, Found::Workspace);
    }

    /// **Order decides, and it is the order of the table**: a machine that has
    /// two of these has the earlier one.
    #[test]
    fn an_earlier_candidate_wins_over_every_later_one() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let exe_dir = tmp.path().join("bin");
        std::fs::create_dir_all(&exe_dir).expect("mkdir");
        let workspace = tmp.path().join("tree");
        library(workspace.join("examples"));
        library(tmp.path().join("bin/examples"));
        library(tmp.path().join("share/karakuri/examples"));
        library(tmp.path().join("Resources/examples"));

        // All four are libraries. Peel them off in order and the next one down
        // answers, which checks the order rather than only its first entry.
        for expected in [
            Found::Bundle,
            Found::Prefix,
            Found::Beside,
            Found::Workspace,
        ] {
            let found =
                searched(Some(&exe_dir), &workspace).expect("one of the four holds a library");
            assert_eq!(
                found.found,
                expected,
                "{} answered ahead of its turn",
                found.dir.display()
            );
            std::fs::remove_dir_all(&found.dir).expect("rmdir");
        }
    }

    /// **A directory called `examples` is not a library by its name**, and
    /// this is the case that is not hypothetical: `cargo` builds example
    /// binaries into `<target>/<profile>/examples`, which is exactly where the
    /// portable candidate looks for a `cargo run`.
    #[test]
    fn a_directory_of_something_else_is_not_a_library_however_it_is_named() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let exe_dir = tmp.path().join("target/debug");
        std::fs::create_dir_all(exe_dir.join("examples")).expect("mkdir");
        std::fs::write(exe_dir.join("examples/panel-36562087879dc46c"), "elf").expect("write");
        let workspace = tmp.path().join("tree");
        library(workspace.join("examples"));

        let found = searched(Some(&exe_dir), &workspace).expect("the workspace tree holds one");
        assert_eq!(
            found.found,
            Found::Workspace,
            "cargo's own `examples` directory answered as a preset library, and the pair \
             this program opens on is not in it"
        );
    }

    /// **A directory of parts is not a library**, which is the whole of what
    /// changed today: `.kir` files are what a Set is made of, and a library is
    /// the listing of what can go on a deck rather than the material it is cut
    /// from. Before the `.kset` files existed this directory *was* the answer,
    /// so this test would have failed on purpose yesterday.
    #[test]
    fn a_directory_of_parts_is_not_a_library_because_a_library_lists_what_goes_on_a_deck() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let exe_dir = tmp.path().join("bin");
        std::fs::create_dir_all(&exe_dir).expect("mkdir");
        parts(exe_dir.join("examples"));
        let workspace = tmp.path().join("tree");
        library(workspace.join("examples"));

        let found = searched(Some(&exe_dir), &workspace).expect("the workspace tree holds one");
        assert_eq!(
            found.found,
            Found::Workspace,
            "a directory holding only parts answered as a preset library, and there is \
             nothing in it the Library bay can draw a row for"
        );
    }

    /// **Nothing found is a value**, and the sentence about it names the flag.
    #[test]
    fn no_library_anywhere_is_an_answer_rather_than_an_error() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let exe_dir = tmp.path().join("bin");
        std::fs::create_dir_all(&exe_dir).expect("mkdir");

        assert_eq!(
            searched(Some(&exe_dir), &tmp.path().join("no-workspace")),
            None,
            "a machine with no preset library reported one"
        );
        assert!(
            no_preset_library().contains("--presets"),
            "the sentence about having no library does not say how to name one"
        );
        assert!(
            no_launch_pair().contains("--presets"),
            "the refusal for having nothing to play does not say how to name a library"
        );
    }

    /// **A root's Sets are listed under the id a store would give them, with
    /// the file beside each one**, in ascending id order whatever order the
    /// filesystem hands them back in.
    #[test]
    fn a_root_of_set_files_lists_them_by_id_with_the_file_to_take_in() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = parts(tmp.path().join("examples"));
        // Written out of alphabetical order, so a listing that passed
        // `read_dir`'s order through would have to be lucky to pass.
        for id in ["glow_lattice", "beat_cloud", "drift_cloud"] {
            set(&dir, id);
        }

        let listed = root(dir.clone()).list_sets().expect("the root is there");
        assert_eq!(
            listed.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
            ["beat_cloud", "drift_cloud", "glow_lattice"],
            "the listing is not the ids in ascending order"
        );
        for entry in &listed {
            assert_eq!(
                entry.file,
                dir.join(format!("{}{AUTHORING_SUFFIX}", entry.id)),
                "the file beside `{}` is not the one the id names",
                entry.id
            );
            assert!(
                entry.file.is_file(),
                "the listing named `{}`, which is not a file anything can take in",
                entry.file.display()
            );
        }
    }

    /// **A root of parts holds no Sets, and that is a value rather than a
    /// failure** — an operator may type `--presets` at a directory of `.kir`
    /// files, and the answer is an empty library rather than an error about it.
    #[test]
    fn a_root_of_parts_lists_nothing_and_is_not_a_refusal() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = parts(tmp.path().join("examples"));

        assert_eq!(
            root(dir)
                .list_sets()
                .expect("a readable directory is not a refusal"),
            Vec::new(),
            "a directory of parts reported Sets in it"
        );
    }

    /// **A name the layout does not claim is skipped rather than repaired**:
    /// each of these would become a row nothing can open if its id were guessed
    /// at, and every one of them is a file somebody else put there.
    #[test]
    fn a_name_the_layout_does_not_claim_is_skipped_rather_than_repaired() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = parts(tmp.path().join("examples"));
        set(&dir, "beat_cloud");
        for name in [
            "notes.txt",               // not a Set by any reading
            "beat_glow.kbset",         // the resolved form: a store's, not a root's
            "beat_glow.kset~",         // an editor's backup
            "half_written.kset.tmp",   // a copy that died
            "GLASS_CLOUD.KSET",        // `setfile::resolve` refuses this spelling
            "beat_glow.kset.disabled", // switched off by rename
        ] {
            std::fs::write(dir.join(name), "{}\n").expect("write");
        }

        let listed = root(dir).list_sets().expect("the root is there");
        assert_eq!(
            listed.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
            ["beat_cloud"],
            "a name the layout does not claim was listed as a Set"
        );
    }

    /// **A subdirectory named like a Set file is not one**, and this is the
    /// case a checked-out bundle actually produces: a directory somebody
    /// unpacked beside the file it came from.
    #[test]
    fn a_subdirectory_that_looks_like_a_set_file_is_not_one() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = parts(tmp.path().join("examples"));
        set(&dir, "beat_cloud");
        std::fs::create_dir_all(dir.join(format!("unpacked{AUTHORING_SUFFIX}"))).expect("mkdir");
        // And one under it, so a listing that walked would find it.
        set(&dir.join(format!("unpacked{AUTHORING_SUFFIX}")), "buried");

        let listed = root(dir).list_sets().expect("the root is there");
        assert_eq!(
            listed.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
            ["beat_cloud"],
            "the listing walked into a subdirectory, or reported one as a Set"
        );
    }

    /// **A root that has gone since it was resolved is refused rather than
    /// reported empty** — the difference between *this library holds nothing*
    /// and *this library is not there* is the whole of what the operator has to
    /// act on, and only the second one is fixable.
    #[test]
    fn a_root_that_has_gone_since_it_was_resolved_is_refused_rather_than_reported_empty() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = library(tmp.path().join("examples"));
        let presets = root(dir.clone());
        std::fs::remove_dir_all(&dir).expect("rmdir");

        let why = presets.list_sets().expect_err(
            "a presets root that is no longer there listed as an empty library, which says \
             it holds nothing to somebody whose disk it is no longer on",
        );
        assert!(
            why.contains(&dir.display().to_string()),
            "the refusal `{why}` does not name the root it could not read"
        );
        assert!(
            !dir.exists(),
            "the listing created the directory it was complaining about"
        );
    }

    /// **One store directory, and both programs read this one.** The value is
    /// what it always was; what is checked here is that it is still the
    /// relative directory the reason above argues for, because an absolute one
    /// or a `$HOME` one would be a different decision arrived at by an edit.
    #[test]
    fn the_default_store_is_one_directory_beside_the_session() {
        assert_eq!(STORE, ".karakuri");
        assert!(
            Path::new(STORE).is_relative(),
            "the default store is no longer beside the session"
        );
    }
}
