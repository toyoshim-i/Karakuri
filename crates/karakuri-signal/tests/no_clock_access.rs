//! Static analysis test verifying `karakuri-signal` contains no system clock reads.
//!
//! Enforces deterministic replay by scanning `src/` for real-time clock APIs
//! (`Instant::now`, `SystemTime::now`, etc.).

use std::fs;
use std::path::Path;

/// Substrings that would indicate a real clock read. Kept as whole API
/// paths (not just "now") so the check doesn't false-positive on English
/// words like "now" that show up in comments and doc prose.
const FORBIDDEN: &[&str] = &[
    "Instant::now",
    "SystemTime::now",
    "std::time::Instant",
    "std::time::SystemTime",
    "chrono::Utc::now",
    "chrono::Local::now",
    "web_time",
];

#[test]
fn crate_source_never_reads_a_clock() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_dir = Path::new(manifest_dir).join("src");

    let mut checked = 0;
    for entry in fs::read_dir(&src_dir).expect("read src/") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }

        let contents = fs::read_to_string(&path).expect("read source file");
        for needle in FORBIDDEN {
            assert!(
                !contents.contains(needle),
                "{} contains {needle:?} — the oscillator must advance only by \
                 the step count and dt it is given, never by a clock read",
                path.display(),
            );
        }
        checked += 1;
    }

    // Guard against the scan silently checking nothing because the layout
    // changed (e.g. src/ moved, or this test stopped matching any files).
    assert!(
        checked >= 2,
        "expected to scan lib.rs, oscillator.rs, and bus.rs"
    );
}
