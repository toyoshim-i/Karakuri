//! Where this program's two directories are, when nobody has said.
//!
//! # The third row of [`crate::scratch`]'s table, which never had an address
//!
//! | | where | who writes it |
//! |---|---|---|
//! | app presets | `examples/` | nobody — they ship with the program |
//! | user presets | `<store>/sets/<id>.kbset` | `--save-set`, and nothing else |
//! | scratch | `<store>/scratch/` | `--watch`, `--mcp`, and the operator's editor |
//!
//! That table has said `examples/` since it was written, and `examples/`
//! relative to *what* was never in it. It was the repository's own directory,
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
//! `docs/principles/0027-silently-wrong-loses-to-loud-failure.md`'s reason: a
//! program that quietly played something else would leave the operator
//! reading a window that disagrees with the command line they typed, with
//! nothing anywhere saying which one won.
//!
//! And **nothing found is a value rather than an error** — [`presets`] answers
//! `Ok(None)`. A machine with no preset library is a machine with no preset
//! library; the panel still runs on two paths given by hand, and it is the
//! *caller* that decides whether the state it is in needs one. What must not
//! happen is silence, which is what [`no_preset_library`] is for.

use std::path::{Path, PathBuf};

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
        // `exists`, not "has a `.kir` in it": an empty directory an operator
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

/// **A directory with at least one `.kir` in it**, which is what a candidate
/// has to be to answer — and existence alone is not enough.
///
/// A cargo workspace builds its example binaries into `<target>/<profile>/examples`,
/// and a debug build of the panel sits in `<target>/<profile>`. So `<exe
/// dir>/examples` — the portable candidate — *exists* for every `cargo run` in
/// this repository and holds four hundred object files and no presets. On an
/// existence test it would win over the development entry every time, and the
/// program would resolve a presets root, print it, and then fail to open
/// `drift_shell.kir` inside it: silently wrong material, then a puzzle
/// (P-0027). Asking for one `.kir` costs one directory read per candidate and
/// makes the two directories that share a name distinguishable by what is in
/// them, which is the only thing that tells them apart.
///
/// One entry, not a count: this asks *is this a preset library*, and a library
/// with one preset in it is one.
fn is_a_library(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        entry
            .path()
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("kir"))
    })
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

    /// A directory with a preset in it, at `path`.
    fn library(path: PathBuf) -> PathBuf {
        std::fs::create_dir_all(&path).expect("mkdir");
        std::fs::write(path.join("drift_shell.kir"), "kind L1\n").expect("write");
        path
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
    /// say nothing about it (P-0027).
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
