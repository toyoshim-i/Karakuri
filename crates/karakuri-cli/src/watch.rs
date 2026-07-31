//! Watching one slot's two `.kir` files and rebuilding when they change.
//!
//! This is the [`Source`] the engine's build worker polls. It runs on that
//! worker thread, so everything expensive it does — a file read, four
//! validation stages, and the diagnostics it prints when one of them refuses —
//! is already off the render thread before `Set::build` is even reached.
//!
//! **One of these per deck slot, over that slot's own pair.** A slot is the
//! unit that gets replaced — that is what it means for each slot to own its own
//! `HotSwap` — so the way "rebuild the slot whose files changed" is enforced is
//! that no watcher can see another slot's files at all. Two slots given the
//! same pair both rebuild, which is right: the same edit reached both of them.
//!
//! ## Polling, not `notify`
//!
//! The worker is a poll loop already: it has to wake regularly to free Sets
//! the render thread retired, so there is no blocking `recv` for a filesystem
//! event to replace. Given that, `notify` would add a dependency, a platform
//! backend per OS, and a second event vocabulary to debounce, in exchange for
//! shaving a tenth of a second off a loop whose other end is a human editing a
//! file. Reading two small files every hundred milliseconds is not a cost worth
//! avoiding here, and it is the same amount of code.
//!
//! ## Contents, not timestamps
//!
//! What is compared is a hash of each file's bytes, not its mtime. A swap is
//! not free — the incoming Set starts cold, at `t` zero, with nothing primed —
//! so it must not happen for a save that changed no text. `touch`, a formatter
//! that rewrites identical bytes, and `git checkout` of the branch you are
//! already on all move the mtime, and under an mtime comparison each of them
//! restarts the visual for no reason. Reading the file is not extra work
//! either: this `Source` reads it a moment later anyway to compile it.
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

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::time::Duration;

use karakuri_engine::{Request, Source};

use crate::compile;

/// How often the two files are stamped. Two polls of quiet are needed before a
/// change is acted on, so this is half the latency of a save reaching the
/// screen — a fifth of a second, against a compile that takes longer than that
/// anyway.
const INTERVAL: Duration = Duration::from_millis(100);

pub struct Watch {
    /// Which slot this rebuilds. Carried only so that the diagnostics this
    /// prints — from a worker thread, interleaved with every other slot's — say
    /// which of the four they are about.
    slot: usize,
    l1: PathBuf,
    l4: PathBuf,
    capacity: u32,
    seed_salt: u32,
    overrides: Vec<(String, f32)>,
    /// A hash of each file's contents as of the previous poll. `None` for a
    /// file that does not exist or cannot be read, which compares equal to
    /// itself and so reads as "unchanged" rather than as a change every
    /// interval — a deleted file is not an edit to react to.
    stamps: [Option<u64>; 2],
    /// A change has been seen but not yet acted on — see "Debouncing" above.
    settling: bool,
}

impl Watch {
    pub fn new(
        slot: usize,
        l1: PathBuf,
        l4: PathBuf,
        capacity: u32,
        seed_salt: u32,
        overrides: Vec<(String, f32)>,
    ) -> Watch {
        let mut watch = Watch {
            slot,
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

    fn stamp(&self) -> [Option<u64>; 2] {
        // Not a cryptographic hash and not trying to be: this compares a file
        // against its own previous contents seconds earlier, where the only
        // adversary is an editor writing the same bytes back.
        let digest = |path: &PathBuf| {
            std::fs::read(path).ok().map(|bytes| {
                let mut h = DefaultHasher::new();
                bytes.hash(&mut h);
                h.finish()
            })
        };
        [digest(&self.l1), digest(&self.l4)]
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

        let slot = self.slot;
        eprintln!("slot {slot}: recompiling:");
        // Both files, not just the changed one: the composition check needs
        // the pair, and an L4 that stopped being compatible with its L1 is a
        // diagnostic rather than a half-applied edit.
        let l1 = match compile::load(&self.l1) {
            Ok(checked) => checked,
            Err(report) => {
                eprintln!("{report}\nslot {slot} unchanged; its Set is still running");
                return None;
            }
        };
        let l4 = match compile::load(&self.l4) {
            Ok(checked) => checked,
            Err(report) => {
                eprintln!("{report}\nslot {slot} unchanged; its Set is still running");
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

#[cfg(test)]
mod tests {
    use super::*;

    fn watch_on(dir: &std::path::Path) -> Watch {
        Watch::new(0, dir.join("a.kir"), dir.join("b.kir"), 4096, 1, Vec::new())
    }

    /// A save that changed no bytes is not an edit. Under an mtime comparison
    /// `touch`, a formatter, or a `git checkout` of the branch already checked
    /// out each restarts the visual from `t` zero for nothing.
    #[test]
    fn rewriting_identical_bytes_is_not_a_change() {
        let dir = std::env::temp_dir().join("karakuri-watch-identical");
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(dir.join("a.kir"), "proc a {}").expect("write");
        std::fs::write(dir.join("b.kir"), "proc b {}").expect("write");

        let w = watch_on(&dir);
        let before = w.stamp();
        std::thread::sleep(Duration::from_millis(10));
        std::fs::write(dir.join("a.kir"), "proc a {}").expect("rewrite");
        assert_eq!(before, w.stamp(), "identical bytes read as a change");

        std::fs::write(dir.join("a.kir"), "proc a { }").expect("edit");
        assert_ne!(before, w.stamp(), "a real edit read as unchanged");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A missing file is stable rather than a change every interval, so
    /// deleting one does not put the watcher into a recompile loop against a
    /// path that is not there.
    #[test]
    fn a_missing_file_is_stable() {
        let dir = std::env::temp_dir().join("karakuri-watch-missing");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let w = watch_on(&dir);
        assert_eq!(w.stamp(), [None, None]);
        assert_eq!(w.stamp(), w.stamp());
        std::fs::remove_dir_all(&dir).ok();
    }
}
