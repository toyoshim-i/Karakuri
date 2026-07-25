//! Reproducibility depends on this crate never reading a clock: the oscillator
//! advances only by the `steps`/`dt` it is handed, and every noise sample is a
//! pure function of an explicit seed plus the oscillator's own state. This
//! test scans the crate's own source for the APIs that would break that —
//! `Instant::now`, `SystemTime::now`, and friends — rather than just asserting
//! it in a doc comment, so a future edit that sneaks one in fails the build.
//!
//! It reads `src/` at test time via `CARGO_MANIFEST_DIR`, which is a build
//! detail, not a clock read: the path is fixed at compile time and the
//! contents are this crate's own checked-in source.

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
