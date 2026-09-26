//! Invariant tests ensuring master chains are compiled off the render thread.

use std::path::{Path, PathBuf};

/// The two files that run on a thread that encodes frames.
const RENDER_PATH: [&str; 2] = [
    "crates/karakuri/src/app/handler/redraw.rs",
    "crates/karakuri-cli/src/live/mod.rs",
];

/// Signatures allocating or compiling master chains.
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

/// Blanks comments and string literals with spaces while preserving byte positions.
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

/// Verifies render loop contains no chain compilation or target allocations.
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

/// Chain installation occurs via `apply_chain` and `ChainSwap` transfer.
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

/// Verifies that only the offline renderer invokes synchronous chain installation.
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

/// Verifies that apply_chain compiles nothing synchronously on the caller's thread.
#[test]
fn apply_chain_builds_nothing_on_the_callers_thread() {
    let code = source("crates/karakuri-environment/src/mix/chain.rs");
    let at = code
        .find("pub fn apply_chain(")
        .expect("mix::apply_chain has moved or been renamed");
    let body = &code[at..];
    let end = body
        .find("\n}\n")
        .or_else(|| body.find("\r\n}\r\n"))
        .expect("mix::apply_chain has no end");
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
