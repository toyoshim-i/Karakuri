//! **Neither real-time host compiles a master chain on its own thread.**
//!
//! The invariant is stated in `crates/karakuri-engine/src/lib.rs` and decided in
//! [ADR-0354](../../../docs/adr/0354-a-chain-is-compiled-on-a-thread-of-its-own-and-lands-at-a-frame-boundary.md):
//! a chain's shader modules, pipelines and targets are made on `karakuri-chain`
//! and installed at a frame boundary. `karakuri_engine::ChainSwap` is the only
//! way to ask for that, and `karakuri_environment::mix::apply_chain` is the only
//! entry the two hosts use.
//!
//! Nothing in a type or a trait can say this: the synchronous functions still
//! exist and are still correct — the offscreen renderer and replay call them,
//! having no frame waiting on a clock — so what is wrong is *who* calls them.
//! That is a fact about two source files, and this reads them.
//!
//! # What "the render path" means here
//!
//! Both files are read whole rather than by function. `karakuri/src/app/handler.rs`
//! is the `winit` handler and `karakuri-cli/src/live/mod.rs` is the live loop;
//! every line of each runs on the thread that encodes frames, so a compile
//! anywhere in either is a compile on the render thread. A coarse read is the
//! right one: it cannot be argued out of by moving the call into a helper in the
//! same file.
//!
//! Comments and string literals are blanked first, so the prose in these files
//! — which names `mix::apply_chain` and ADR numbers freely — is not mistaken for
//! a call.

use std::path::{Path, PathBuf};

/// The two files that run on a thread that encodes frames.
const RENDER_PATH: [&str; 2] = [
    "crates/karakuri/src/app/handler/redraw.rs",
    "crates/karakuri-cli/src/live/mod.rs",
];

/// Every way a chain's pipelines or targets are made on the calling thread.
///
/// `set_chain` is here because installing a chain that carries no targets
/// allocates them where it is installed, which is the same invariant one step
/// along.
const COMPILES_A_CHAIN: [&str; 5] = [
    "install_chain(",
    "build_chain(",
    "Slot::build(",
    "ChainTargets::build(",
    "set_chain(",
];

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn source(at: &str) -> String {
    let path = workspace().join(at);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    blanked(&text)
}

/// Replace every comment and string literal with spaces of the same length, so
/// line numbers and offsets survive and nothing downstream has to know they were
/// there.
///
/// Byte-oriented: the sources are UTF-8 and full of non-ASCII prose, and every
/// delimiter this cares about is ASCII, which a multi-byte character can never
/// contain.
fn blanked(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = vec![b' '; bytes.len()];
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    if bytes[i] == b'\n' {
                        out[i] = b'\n';
                    }
                    i += 1;
                }
                i = (i + 2).min(bytes.len());
            }
            b'"' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    if i < bytes.len() && bytes[i] == b'\n' {
                        out[i] = b'\n';
                    }
                    i += 1;
                }
                i += 1;
            }
            c => {
                out[i] = c;
                i += 1;
            }
        }
    }
    String::from_utf8(out).expect("blanking preserves UTF-8 because it only removes ASCII runs")
}

/// **Neither host names a function that compiles a chain or allocates its
/// targets.**
#[test]
fn the_render_path_compiles_no_chain() {
    for at in RENDER_PATH {
        let code = source(at);
        for spelling in COMPILES_A_CHAIN {
            assert!(
                !code.contains(spelling),
                "{at} calls `{spelling}` — a chain compiled or allocated on the thread \
                 that encodes frames (ADR-0354). `mix::apply_chain` is how this file \
                 asks for a chain."
            );
        }
    }
}

/// **Both hosts ask through `apply_chain` and install through a `ChainSwap`.**
///
/// The other half of the test above: a file that mentions neither would pass it
/// by having no chain at all, which is not what either of these is.
#[test]
fn the_render_path_asks_the_worker_and_installs_at_the_boundary() {
    for at in RENDER_PATH {
        let code = source(at);
        assert!(
            code.contains("apply_chain("),
            "{at} no longer applies a master chain at all"
        );
        assert!(
            code.contains("chain_swap"),
            "{at} applies a chain without a `ChainSwap` to build it on"
        );
        assert!(
            code.contains(".begin_frame("),
            "{at} never installs a finished chain at a frame boundary"
        );
    }
}

/// **The synchronous path is where it says it is.**
///
/// `mix::install_chain` exists for the runs with no frame waiting on a clock —
/// the offscreen renderer and the replay it drives — and this is the one file
/// that may call it. Without this, the test above is satisfied by deleting the
/// synchronous path rather than by keeping it off the render thread.
#[test]
fn the_offline_renderer_is_the_synchronous_path() {
    let code = source("crates/karakuri-environment/src/render.rs");
    assert!(
        code.contains("install_chain("),
        "the offscreen renderer no longer builds its chain where it stands"
    );
    assert!(
        !code.contains("apply_chain("),
        "the offscreen renderer asks a worker for a chain it then does not wait for"
    );
}

/// **`apply_chain` itself compiles nothing.**
///
/// The entry the hosts call is in a crate this test cannot scan by thread, so
/// it is scanned by body: between its signature and the brace that closes it,
/// nothing builds a slot or a target.
#[test]
fn apply_chain_builds_nothing_on_the_callers_thread() {
    let code = source("crates/karakuri-environment/src/mix/chain.rs");
    let at = code
        .find("pub fn apply_chain(")
        .expect("mix::apply_chain has moved or been renamed");
    let body = &code[at..];
    let end = body.find("\n}\n").expect("mix::apply_chain has no end");
    let body = &body[..end];
    for spelling in ["build_chain(", "Slot::build(", "set_chain("] {
        assert!(
            !body.contains(spelling),
            "mix::apply_chain calls `{spelling}` on the caller's thread (ADR-0354)"
        );
    }
    assert!(
        body.contains("set_chain_params("),
        "mix::apply_chain lost the cheap path: a parameter move is a uniform write (P-0091)"
    );
    assert!(
        body.contains("request("),
        "mix::apply_chain no longer asks the chain worker for anything"
    );
}
