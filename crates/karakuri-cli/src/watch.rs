//! Watching the two `.kir` files and rebuilding when they change.
//!
//! This is the [`Source`] the engine's build worker polls. It runs on that
//! worker thread, so everything expensive it does — a file read, four
//! validation stages, and the diagnostics it prints when one of them refuses —
//! is already off the render thread before `Set::build` is even reached.
//!
//! ## Polling mtimes, not `notify`
//!
//! The worker is a poll loop already: it has to wake regularly to free Sets
//! the render thread retired, so there is no blocking `recv` for a filesystem
//! event to replace. Given that, `notify` would add a dependency, a platform
//! backend per OS, and a second event vocabulary to debounce, in exchange for
//! shaving a tenth of a second off a loop whose other end is a human editing a
//! file. Two `metadata()` calls every hundred milliseconds is not a cost worth
//! avoiding here, and it is the same amount of code.
//!
//! ## Debouncing
//!
//! An editor that writes in place rather than renaming leaves the file
//! truncated for a moment, and reading it then produces a parse error against
//! a file that is about to be fine. So a change is not acted on the poll it is
//! seen; it is acted on the first poll where the timestamps have stopped
//! moving. That costs one extra interval of latency and removes a whole class
//! of spurious diagnostics.
//!
//! A compile that fails prints every diagnostic it has and returns `None`.
//! Nothing is requested, so nothing is built, so nothing is swapped — the
//! running Set keeps running with its `t` and its element buffers untouched.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use karakuri_engine::{Request, Source};

use crate::compile;

/// How often the two files are stamped. Two polls of quiet are needed before a
/// change is acted on, so this is half the latency of a save reaching the
/// screen — a fifth of a second, against a compile that takes longer than that
/// anyway.
const INTERVAL: Duration = Duration::from_millis(100);

pub struct Watch {
    l1: PathBuf,
    l4: PathBuf,
    capacity: u32,
    seed_salt: u32,
    overrides: Vec<(String, f32)>,
    /// The timestamps as of the previous poll. `None` for a file that does not
    /// exist or cannot be stat'd, which compares equal to itself and so reads
    /// as "unchanged" rather than as a change every interval.
    stamps: [Option<SystemTime>; 2],
    /// A change has been seen but not yet acted on — see "Debouncing" above.
    settling: bool,
}

impl Watch {
    pub fn new(
        l1: PathBuf,
        l4: PathBuf,
        capacity: u32,
        seed_salt: u32,
        overrides: Vec<(String, f32)>,
    ) -> Watch {
        let mut watch = Watch {
            l1,
            l4,
            capacity,
            seed_salt,
            overrides,
            stamps: [None, None],
            settling: false,
        };
        // Seeded from what is on disk right now, so that the pair the CLI
        // already compiled at startup is not immediately compiled again.
        watch.stamps = watch.stamp();
        watch
    }

    fn stamp(&self) -> [Option<SystemTime>; 2] {
        let mtime = |path: &PathBuf| std::fs::metadata(path).and_then(|m| m.modified()).ok();
        [mtime(&self.l1), mtime(&self.l4)]
    }
}

impl Source for Watch {
    fn poll(&mut self) -> Option<Request> {
        std::thread::sleep(INTERVAL);

        let stamps = self.stamp();
        if stamps != self.stamps {
            self.stamps = stamps;
            self.settling = true;
            return None;
        }
        if !self.settling {
            return None;
        }
        self.settling = false;

        eprintln!("recompiling:");
        // Both files, not just the changed one: the composition check needs
        // the pair, and an L4 that stopped being compatible with its L1 is a
        // diagnostic rather than a half-applied edit.
        let l1 = match compile::load(&self.l1) {
            Ok(checked) => checked,
            Err(report) => {
                eprintln!("{report}\nnothing changed; the running Set is still running");
                return None;
            }
        };
        let l4 = match compile::load(&self.l4) {
            Ok(checked) => checked,
            Err(report) => {
                eprintln!("{report}\nnothing changed; the running Set is still running");
                return None;
            }
        };

        let label = format!("{} + {}", l1.name, l4.name);
        Some(Request {
            l1,
            l4,
            capacity: self.capacity,
            seed_salt: self.seed_salt,
            params: self.overrides.clone(),
            label,
        })
    }
}
