//! **A test that takes a GPU device lives under `mod gpu`, and one that does
//! not lives outside it.** This file is what makes that true rather than
//! intended: it reads the workspace's own source and checks every `#[test]` in
//! it, both ways round.
//!
//! # What the convention buys
//!
//! `cargo test -p <crate>` keeps its exact present meaning — everything runs,
//! and the pre-push hook is untouched. What is new is the *subtraction*:
//!
//! ```sh
//! cargo test -p karakuri-engine -- --skip gpu::
//! ```
//!
//! runs the part of a crate that needs no adapter. At the time this landed that
//! was ~690 of ~990 tests for about three seconds of the ~306 the whole
//! workspace costs, because the GPU tests are ~99% of the wall clock and almost
//! none of the tests.
//!
//! `#[ignore]` was the obvious alternative and was rejected: it inverts the
//! default, so `cargo test` would quietly stop meaning "everything" and
//! `.githooks/pre-push` would have to grow `--include-ignored` in the same
//! breath. A module path is inert — it changes what a filter can select and
//! nothing about what runs by default.
//!
//! # Why the check is both ways round
//!
//! `--skip` is a *substring* match on the full test path, so `--skip gpu::`
//! skips anything whose name contains those five characters. That makes the two
//! directions two different failures, and both matter:
//!
//! - a GPU test **outside** `mod gpu` is a slow test in the fast set — the
//!   filtered run takes a device it promised not to, and fails on a machine
//!   without one;
//! - a CPU test **inside** any module called `gpu` is a test that silently
//!   stops running whenever anyone uses the filter. This is not hypothetical:
//!   `karakuri_engine::gpu` is a real module, and a `#[test]` added inside it
//!   would be named `gpu::tests::…` and swept up by the filter. The second
//!   assertion below is what refuses that.
//!
//! # The case no in-process rule can see
//!
//! `karakuri-cli/tests/replay.rs` spawns `CARGO_BIN_EXE_karakuri-cli` and the
//! *binary* reaches `Gpu::headless` in a process of its own. Nothing in that
//! file mentions a device, so a scanner looking only for `Gpu::headless` would
//! report the file clean while five tests took five devices. So spawning a
//! binary that reaches a device counts as reaching one — [`GPU_BINARIES`] names
//! them, and [`gpu_binaries_still_take_a_device`] refuses to let that list go
//! stale. It is deliberately coarse: *any* spawn of such a binary counts, even
//! `--help`, because which flags a test passes decides whether that run builds
//! a Set and no static rule can read that. Coarse in this direction is safe —
//! the worst case is a CPU test kept out of the fast set, never a GPU test let
//! into it.
//!
//! # How it reads the source
//!
//! Comments and string literals are blanked before anything is matched, which
//! is load-bearing twice over: this file's own `"Gpu::headless"` needle is a
//! string, and several test files quote the call in prose. A test is judged by
//! its body plus every same-file function it calls, transitively — helpers are
//! resolved by bare name across the whole file, ignoring modules, which can
//! only ever over-approximate. The shape is
//! `karakuri-signal/tests/no_clock_access.rs`: scan the checked-in source, and
//! carry a floor so the scan cannot silently match nothing.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The one door to a device. Every other GPU-touching API in the workspace
/// needs a `&Device`, and this is the only thing that makes one.
const DOOR: &str = "Gpu::headless";

/// Binaries whose own source reaches [`DOOR`], paired with the crate source
/// that has to keep being true of them. A test that spawns one of these is a
/// GPU test however little it looks like one.
const GPU_BINARIES: &[(&str, &str)] = &[("karakuri-cli", "crates/karakuri-cli/src")];

/// A module named exactly this is the marker. Not a prefix or a suffix match:
/// `gpu_budget` is a fine name for a module of CPU tests, and `--skip gpu::`
/// would not touch it.
const MARKER: &str = "gpu";

fn workspace() -> PathBuf {
    // Fixed at compile time, and what it reads is this workspace's own
    // checked-in source — the same move `karakuri-cli/tests/replay.rs` makes.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

/// Replace every comment and string literal with spaces, preserving length and
/// line structure so nothing downstream has to care that they were there.
///
/// Byte-oriented on purpose: the sources are UTF-8 and full of non-ASCII prose,
/// but every delimiter this cares about is ASCII, and a multi-byte character
/// can never contain an ASCII byte — so a byte scan cannot land inside one.
fn blank_comments_and_strings(src: &str) -> String {
    let b = src.as_bytes();
    let mut out: Vec<u8> = b.to_vec();
    let mut i = 0;
    // Blank a range, but keep newlines: a blanked block comment or raw string
    // must not merge the lines around it.
    let blank = |out: &mut Vec<u8>, from: usize, to: usize| {
        for c in out.iter_mut().take(to.min(b.len())).skip(from) {
            if *c != b'\n' {
                *c = b' ';
            }
        }
    };
    while i < b.len() {
        // Line comment.
        if b[i..].starts_with(b"//") {
            let end = src[i..].find('\n').map_or(b.len(), |n| i + n);
            blank(&mut out, i, end);
            i = end;
            continue;
        }
        // Block comment, nested as Rust allows.
        if b[i..].starts_with(b"/*") {
            let start = i;
            let mut depth = 0usize;
            while i < b.len() {
                if b[i..].starts_with(b"/*") {
                    depth += 1;
                    i += 2;
                } else if b[i..].starts_with(b"*/") {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            blank(&mut out, start, i);
            continue;
        }
        // Raw string, any number of hashes. Checked before the plain string so
        // `r"..."` is not read as an identifier followed by a string.
        if b[i] == b'r' && matches!(b.get(i + 1), Some(b'#') | Some(b'"')) {
            let mut j = i + 1;
            while j < b.len() && b[j] == b'#' {
                j += 1;
            }
            if j < b.len() && b[j] == b'"' {
                let hashes = j - i - 1;
                let close = format!("\"{}", "#".repeat(hashes));
                let start = i;
                let end = src[j + 1..]
                    .find(&close)
                    .map_or(b.len(), |n| j + 1 + n + close.len());
                blank(&mut out, start, end);
                i = end;
                continue;
            }
        }
        // Plain string.
        if b[i] == b'"' {
            let start = i;
            i += 1;
            while i < b.len() {
                match b[i] {
                    b'\\' => i += 2,
                    b'"' => {
                        i += 1;
                        break;
                    }
                    _ => i += 1,
                }
            }
            blank(&mut out, start, i);
            continue;
        }
        // Character literal, which has to be told from a lifetime: `'a` is a
        // lifetime, `'a'` and `'\n'` are literals. Getting this wrong would let
        // a `'"'` open a string that swallows real code.
        if b[i] == b'\'' {
            let lit_end = if b.get(i + 1) == Some(&b'\\') {
                src[i + 2..].find('\'').map(|n| i + 2 + n + 1)
            } else if b.get(i + 2) == Some(&b'\'') {
                Some(i + 3)
            } else {
                None
            };
            if let Some(end) = lit_end {
                blank(&mut out, i, end);
                i = end;
                continue;
            }
        }
        i += 1;
    }
    String::from_utf8(out).expect("blanking only ever writes spaces")
}

struct Function {
    name: String,
    /// `::`-joined module path the function sits in, empty at file root.
    module: String,
    /// The body with comments and strings blanked out — what [`DOOR`] is
    /// looked for in, so a mention in prose is not a call.
    body: String,
    /// The same span of the file as written. The one marker that lives *in* a
    /// string literal — the binary name inside `env!("CARGO_BIN_EXE_…")` — has
    /// to be read here, because the blanking has eaten it from `body`.
    raw: String,
    is_test: bool,
}

/// Every function in one file, with its module path and body. `blanked` and
/// `src` are the same file and therefore the same length, which is what lets
/// one set of offsets cut both.
fn functions(blanked: &str, src: &str) -> Vec<Function> {
    let b = blanked.as_bytes();
    let mut depth = 0i32;
    let mut mods: Vec<(i32, String)> = Vec::new();
    // (brace depth of the body, name, body start, module path, is_test)
    let mut open: Vec<(i32, String, usize, String, bool)> = Vec::new();
    let mut done: Vec<Function> = Vec::new();
    let mut i = 0;
    let word_at = |at: usize| -> Option<String> {
        let bytes = blanked.as_bytes();
        let mut j = at;
        while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
            j += 1;
        }
        (j > at).then(|| blanked[at..j].to_string())
    };
    while i < b.len() {
        match b[i] {
            b'{' => {
                depth += 1;
                i += 1;
                continue;
            }
            b'}' => {
                while open.last().is_some_and(|f| f.0 == depth) {
                    let (_, name, start, module, is_test) = open.pop().expect("just checked");
                    done.push(Function {
                        name,
                        module,
                        body: blanked[start..i].to_string(),
                        raw: src[start..i].to_string(),
                        is_test,
                    });
                }
                while mods.last().is_some_and(|m| m.0 == depth) {
                    mods.pop();
                }
                depth -= 1;
                i += 1;
                continue;
            }
            c if c.is_ascii_alphanumeric() || c == b'_' => {
                let word = word_at(i).expect("starts with a word byte");
                let starts_word = i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_');
                if starts_word && (word == "mod" || word == "fn") {
                    let rest = &blanked[i + word.len()..];
                    let name_at = i + word.len() + (rest.len() - rest.trim_start().len());
                    if let Some(name) = word_at(name_at) {
                        if word == "mod" {
                            // `mod foo;` declares, `mod foo {` opens. Only the
                            // second one puts anything on the path.
                            if blanked[name_at + name.len()..]
                                .trim_start()
                                .starts_with('{')
                            {
                                mods.push((depth + 1, name));
                            }
                        } else {
                            // A body, not a signature: `fn f();` in a trait
                            // has no braces of its own, and taking the next
                            // `{` in the file would hand it somebody else's.
                            let after = &blanked[name_at + name.len()..];
                            let brace = match (after.find('{'), after.find(';')) {
                                (Some(o), Some(s)) if s < o => None,
                                (Some(o), _) => Some(name_at + name.len() + o),
                                (None, _) => None,
                            };
                            // Attributes and doc comments sit between the last
                            // item and this one; `#[test]` anywhere in that gap
                            // belongs to this function.
                            let gap_from = blanked[..i].rfind(['}', ';']).map_or(0, |n| n + 1);
                            let is_test = blanked[gap_from..i].contains("#[test]");
                            let module = mods
                                .iter()
                                .map(|m| m.1.as_str())
                                .collect::<Vec<_>>()
                                .join("::");
                            if let Some(brace) = brace {
                                open.push((depth + 1, name, brace + 1, module, is_test));
                            }
                        }
                    }
                }
                i += word.len();
                continue;
            }
            _ => i += 1,
        }
    }
    done
}

/// Does this function reach a device, directly or through a same-file helper?
fn reaches_a_device(
    f: (&str, &str),
    by_name: &HashMap<&str, (&str, &str)>,
    seen: &mut Vec<String>,
) -> bool {
    let (body, raw) = f;
    if body.contains(DOOR) {
        return true;
    }
    // The whole expression, not just the variable name: `env!` is the only way
    // to reach a `CARGO_BIN_EXE_*` (Cargo sets it for the compile, not for the
    // run), so matching the call rather than the name keeps prose that merely
    // mentions one from reading as a spawn. Read out of `raw` because the name
    // lives inside a string literal, which the blanking has eaten.
    if GPU_BINARIES
        .iter()
        .any(|(bin, _)| raw.contains(&format!("env!(\"CARGO_BIN_EXE_{bin}\")")))
    {
        return true;
    }
    for (name, helper) in by_name {
        // A call, not a mention: the name followed by `(`. Cheap and it only
        // over-approximates — a helper shadowed in another module of the same
        // file is read as the one call site's.
        if body.contains(&format!("{name}(")) && !seen.iter().any(|s| s == name) {
            seen.push((*name).to_string());
            if reaches_a_device(*helper, by_name, seen) {
                return true;
            }
        }
    }
    false
}

/// Every `.rs` under `crates/*/src` and `crates/*/tests` — the two places a
/// `#[test]` can live in this workspace. `examples/` is left out: nothing there
/// is a test, and those files do take a device.
fn sources() -> Vec<PathBuf> {
    fn walk(dir: &Path, into: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(&path, into);
            } else if path.extension().is_some_and(|e| e == "rs") {
                into.push(path);
            }
        }
    }
    let mut found = Vec::new();
    for crate_dir in fs::read_dir(workspace().join("crates")).expect("crates/") {
        let crate_dir = crate_dir.expect("dir entry").path();
        walk(&crate_dir.join("src"), &mut found);
        walk(&crate_dir.join("tests"), &mut found);
    }
    // So a failure names the same file every run.
    found.sort();
    found
}

#[test]
fn every_gpu_test_is_under_mod_gpu_and_nothing_else_is() {
    let root = workspace();
    let (mut files, mut tests, mut gpu_tests) = (0, 0, 0);

    for path in sources() {
        let src = fs::read_to_string(&path).expect("read source");
        let blanked = blank_comments_and_strings(&src);
        let fns = functions(&blanked, &src);
        let by_name: HashMap<&str, (&str, &str)> = fns
            .iter()
            .map(|f| (f.name.as_str(), (f.body.as_str(), f.raw.as_str())))
            .collect();
        files += 1;

        for f in fns.iter().filter(|f| f.is_test) {
            tests += 1;
            let reaches = reaches_a_device((&f.body, &f.raw), &by_name, &mut Vec::new());
            let marked = f.module.split("::").any(|seg| seg == MARKER);
            let at = format!(
                "{}::{}",
                path.strip_prefix(&root)
                    .expect("under the workspace")
                    .display(),
                if f.module.is_empty() {
                    f.name.clone()
                } else {
                    format!("{}::{}", f.module, f.name)
                }
            );
            if reaches {
                gpu_tests += 1;
                assert!(
                    marked,
                    "{at} reaches a device and is not under `mod gpu` — \
                     `cargo test -- --skip gpu::` would run it, and fail on any \
                     machine without an adapter"
                );
            } else {
                assert!(
                    !marked,
                    "{at} is under a module called `{MARKER}` and takes no device — \
                     `--skip gpu::` is a substring match, so this test would stop \
                     running whenever anyone filters, and nothing would say so"
                );
            }
        }
    }

    // Floors, not counts: the point is that the scan cannot come back empty
    // because a path moved or the blanking ate the file. They read 120, 997 and
    // 301 when this landed, and are meant to be raised, never lowered to fit a
    // smaller scan. Three of them rather than one because they fail apart: a
    // wrong root gives 0 files, a broken blanking gives files but no tests, and
    // a renamed `Gpu::headless` gives tests but no GPU ones — and that last one
    // would otherwise make the whole check pass by finding nothing to check.
    assert!(
        files >= 60,
        "only {files} source files scanned — is the layout still crates/*/{{src,tests}}?"
    );
    assert!(
        tests >= 800,
        "only {tests} `#[test]` functions found — the scan is not seeing the suite"
    );
    assert!(
        gpu_tests >= 250,
        "only {gpu_tests} tests read as reaching a device — is `{DOOR}` still the door?"
    );
}

/// The entry in [`GPU_BINARIES`] is a claim about another crate's source, and a
/// stale one would reopen exactly the hole it was added to close: `replay.rs`
/// would go back to reading as five CPU tests. So the claim is checked.
#[test]
fn gpu_binaries_still_take_a_device() {
    let root = workspace();
    for (bin, src) in GPU_BINARIES {
        let dir = root.join(src);
        let mut found = false;
        let mut checked = 0;
        for entry in fs::read_dir(&dir).expect("the named crate source") {
            let path = entry.expect("dir entry").path();
            if path.extension().is_some_and(|e| e == "rs") {
                checked += 1;
                found |= fs::read_to_string(&path).expect("read").contains(DOOR);
            }
        }
        // A floor of one, and it was two until ADR-0214's move: the guard is
        // against reading *nothing* — a renamed directory, or an extension
        // filter that stopped matching — and `karakuri-cli/src` legitimately
        // holds one file now. It is not the claim. The claim is `found`
        // below, which needs the door read out of a real file, so an emptied
        // or moved directory fails there too and says something useful when
        // it does. Raising this back would mean asserting a crate's shape
        // from a test about GPU markers, which is not this file's business.
        assert!(
            checked >= 1,
            "{} holds no Rust source at all — has the path moved?",
            dir.display()
        );
        assert!(
            found,
            "`{bin}` no longer reaches `{DOOR}`, so spawning it is not a GPU \
             reach any more — drop it from GPU_BINARIES and move the tests that \
             spawn it out of `mod gpu`"
        );
    }
}
