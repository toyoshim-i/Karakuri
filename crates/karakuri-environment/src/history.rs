//! Procedure edit history and snapshot tracking.
//!
//! Stores compiled procedure versions in date-partitioned directories (`history/YYYY/MM/DD/`)
//! tagged by slot, layer, index, procedure name, and Set id for undo and version inspection.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The subdirectory of the store this lives in.
pub const DIR: &str = "history";

/// Local time format down to milliseconds (`%H%M%S-%3f`).
const TIME: &str = "%H%M%S-%3f";

/// Generates a unique, collision-resistant identifier using local date and millisecond time.
pub fn stamped_id() -> String {
    let now = chrono::Local::now();
    let stamp = format!("{}-{}", now.format("%Y%m%d"), now.format(TIME));
    static ISSUED: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<String>>> =
        std::sync::OnceLock::new();
    let issued = ISSUED.get_or_init(Default::default);
    match issued.lock() {
        Ok(mut issued) => unused(&mut issued, stamp),
        // A poisoned lock means a caller panicked holding it. The stamp is
        // still a name, and refusing to save over it would be this bookkeeping
        // deciding whether an operator keeps their work.
        Err(_) => stamp,
    }
}

/// Appends a sequential numeric suffix (`-1`, `-2`) if `stamp` was already issued in this run.
fn unused(issued: &mut std::collections::HashSet<String>, stamp: String) -> String {
    if issued.insert(stamp.clone()) {
        return stamp;
    }
    let mut nth = 1u32;
    loop {
        let candidate = format!("{stamp}-{nth}");
        if issued.insert(candidate.clone()) {
            return candidate;
        }
        nth += 1;
    }
}

/// In-memory cache tracking the most recently saved snapshot content per node and Set.
pub struct Snapshots {
    root: PathBuf,
    /// The chain — a node of a Set — against what it last wrote. The `Option` is a
    /// version written where there was no Set, which is a state and not a missing
    /// key; [`Snapshots::record`] says which runs are in it.
    last: HashMap<(usize, &'static str, usize, Option<String>), Vec<u8>>,
}

/// One history for the run, shared between the launch-time seeding and every
/// slot's watcher — see the module doc on why they cannot each have their own.
pub type Shared = std::sync::Arc<std::sync::Mutex<Snapshots>>;

impl Snapshots {
    pub fn new(store_root: &Path) -> Snapshots {
        Snapshots {
            root: store_root.join(DIR),
            last: HashMap::new(),
        }
    }

    pub fn shared(store_root: &Path) -> Shared {
        std::sync::Arc::new(std::sync::Mutex::new(Snapshots::new(store_root)))
    }

    /// Writes `source` as the newest snapshot for `(slot, layer, index, set)` if content changed.
    ///
    /// Naming format is `<HHMMSS-millis>_slot<slot>_<layer>[index]_<proc>[@set].kir`.
    /// Returns the written snapshot path, or `None` if `source` equals the previous snapshot.
    pub fn record(
        &mut self,
        slot: usize,
        layer: &'static str,
        index: usize,
        set: Option<&str>,
        proc_name: &str,
        source: &[u8],
    ) -> Result<Option<PathBuf>, String> {
        // Owned to look up, which is one allocation on a path that is about to
        // create a directory and write a file. A borrowed key would be a
        // lifetime on this whole map to save it.
        let chain = (slot, layer, index, set.map(str::to_string));
        if self.last.get(&chain).is_some_and(|s| s == source) {
            return Ok(None);
        }
        let now = chrono::Local::now();
        let dir = self.root.join(now.format("%Y/%m/%d").to_string());
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;

        let stamp = now.format(TIME).to_string();
        // The index is in the name only when it is not the first, so every
        // name a one-renderer run has ever written is the name it still writes.
        // A file is read by a person looking for what they changed, and a `_0`
        // on every L4 of every ordinary run is noise in the way of that.
        let at = if index == 0 {
            String::new()
        } else {
            format!("{index}")
        };
        // The Set is in the name only when there is one, so every name a run
        // with no Set has ever written is the name it still writes — the same
        // argument the index above is left off a `_0` by, and the same one that
        // lets `list` go on claiming the names written before this field was.
        let of = match set {
            Some(set) => format!("@{}", sanitize(set)),
            None => String::new(),
        };
        let name = format!(
            "{stamp}_slot{slot}_{layer}{at}_{}{of}.kir",
            sanitize(proc_name)
        );
        let path = dir.join(name);
        std::fs::write(&path, source).map_err(|e| format!("{}: {e}", path.display()))?;
        self.last.insert(chain, source.to_vec());
        Ok(Some(path))
    }
}

/// A recorded procedure version discovered by [`list`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    /// Timestamp identifier in `YYYYMMDD-HHMMSS-millis` format.
    pub at: String,
    /// The slot this was a version of, as `record` was given it.
    pub slot: usize,
    /// `L1`, `L2`, `L3`, `L4` or `Field` — one of [`LAYERS`], which is the same
    /// list [`declared_kind`] answers with and the same list [`crate::mcp`]'s
    /// `Slots` addresses a node by.
    pub layer: &'static str,
    /// Which renderer of that layer, counting from zero. A name carries this only
    /// when it is not the first ([`Snapshots::record`] says why), so a name without
    /// one reads back as `0` — the row and the writer's argument are the same
    /// number either way.
    pub index: usize,
    /// What the procedure called itself, through [`sanitize`]. It is what the file
    /// was named after and not what the file says now; nothing is opened.
    pub proc_name: String,
    /// Set identifier if this version was recorded under an active Set.
    pub set: Option<String>,
    /// The snapshot itself — what to read, and what a load would be pointed at.
    pub file: PathBuf,
}

impl Version {
    /// Constructs the display name for UI listing: `<timestamp>_slot<slot>_<layer>[index]_<proc>`.
    #[must_use]
    pub fn filed_as(&self) -> String {
        let index = match self.index {
            0 => String::new(),
            index => index.to_string(),
        };
        format!(
            "{}_slot{}_{}{index}_{}",
            self.at, self.slot, self.layer, self.proc_name
        )
    }
}

/// Results of a history directory listing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Listing {
    /// Most recent first, at most as many as were asked for.
    pub versions: Vec<Version>,
    /// True if directory traversal reached the limit before scanning all available history.
    pub stopped_short: bool,
    /// Count of unrecognized or non-snapshot files passed over in inspected directories.
    pub unclaimed: usize,
}

/// Canonical layer identifiers recognized in snapshot file naming.
pub const LAYERS: [&str; 6] = ["L1", "L2", "L3", "L4", "Field", "L5"];

/// One of [`LAYERS`], or `None` for a word this module does not spell a layer
/// with.
fn known_layer(word: &str) -> Option<&'static str> {
    LAYERS.into_iter().find(|layer| *layer == word)
}

/// Lists history versions ordered most recent first, returning at most `most` entries.
///
/// Scans date directories (`YYYY/MM/DD/`) lazily and skips unrecognized files.
/// See ADR-0262, ADR-0263, Principle 0090, and Principle 0091.
pub fn list(store_root: &Path, most: usize) -> Result<Listing, String> {
    let mut listing = Listing::default();
    let root = store_root.join(DIR);
    'walk: for (year, year_dir) in dated(&root, 4, &mut listing.unclaimed)? {
        for (month, month_dir) in dated(&year_dir, 2, &mut listing.unclaimed)? {
            for (day, day_dir) in dated(&month_dir, 2, &mut listing.unclaimed)? {
                // Checked before the directory is opened, not after: this is
                // the whole of the bound, and a check on the way out would have
                // already paid for the day it was refusing.
                if listing.versions.len() >= most {
                    listing.stopped_short = true;
                    break 'walk;
                }
                let date = format!("{year}{month}{day}");
                let mut of_the_day = Vec::new();
                for entry in read(&day_dir)? {
                    let entry = entry.map_err(|e| cannot_be_listed(&day_dir, &e))?;
                    let claimed = !entry
                        .file_type()
                        .map_err(|e| cannot_be_listed(&day_dir, &e))?
                        .is_dir();
                    match claimed.then(|| version(&date, &entry)).flatten() {
                        Some(found) => of_the_day.push(found),
                        None => listing.unclaimed += 1,
                    }
                }
                // One day, so `at` differs only in its time half — but sorted
                // on the whole of it anyway, because the field that orders the
                // listing across days is the field that has to order it within
                // one, or the two would be free to disagree.
                of_the_day.sort_by(|a, b| b.at.cmp(&a.at).then_with(|| a.file.cmp(&b.file)));
                listing.versions.append(&mut of_the_day);
            }
        }
    }
    if listing.versions.len() > most {
        listing.versions.truncate(most);
        listing.stopped_short = true;
    }
    Ok(listing)
}

/// Discovers numeric date subdirectories of exact length `digits`, returned in descending order.
fn dated(
    dir: &Path,
    digits: usize,
    unclaimed: &mut usize,
) -> Result<Vec<(String, PathBuf)>, String> {
    let mut out = Vec::new();
    for entry in read(dir)? {
        let entry = entry.map_err(|e| cannot_be_listed(dir, &e))?;
        let name = entry.file_name();
        let claimed = name
            .to_str()
            .filter(|n| n.len() == digits && n.bytes().all(|b| b.is_ascii_digit()))
            .map(str::to_string);
        match claimed {
            Some(name)
                if entry
                    .file_type()
                    .map_err(|e| cannot_be_listed(dir, &e))?
                    .is_dir() =>
            {
                out.push((name, entry.path()))
            }
            _ => *unclaimed += 1,
        }
    }
    out.sort();
    out.reverse();
    Ok(out)
}

/// Reads directory entries, returning an empty vector if `dir` does not exist.
fn read(dir: &Path) -> Result<Vec<std::io::Result<std::fs::DirEntry>>, String> {
    match std::fs::read_dir(dir) {
        Ok(entries) => Ok(entries.collect()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(cannot_be_listed(dir, &e)),
    }
}

/// The directory could not be read, named, with the reason the filesystem gave
/// rather than a guess at it — [`Snapshots::record`]'s spelling for the same
/// kind of failure, which is the one an operator sees beside it.
fn cannot_be_listed(dir: &Path, why: &std::io::Error) -> String {
    format!("cannot list the edit history at `{}`: {why}", dir.display())
}

/// Parses a directory entry name into a [`Version`], or returns `None` if unrecognized.
fn version(date: &str, entry: &std::fs::DirEntry) -> Option<Version> {
    let name = entry.file_name();
    let stem = name.to_str()?.strip_suffix(".kir")?;
    // Set identifier is stripped from the tail after the final `@`.
    let (stem, set) = match stem.rsplit_once('@') {
        Some((head, set)) if !set.is_empty() && sanitize(set) == set => {
            (head, Some(set.to_string()))
        }
        Some(_) => return None,
        None => (stem, None),
    };
    let mut fields = stem.splitn(4, '_');
    let (time, slot, addressed, proc_name) = (
        fields.next()?,
        fields.next()?.strip_prefix("slot")?,
        fields.next()?,
        fields.next()?,
    );
    // Verify valid timestamp format: 6 digits, hyphen, 3 digits.
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let stamp = time.as_bytes();
    if stamp.len() != 10
        || stamp[6] != b'-'
        || !stamp[..6].iter().all(u8::is_ascii_digit)
        || !stamp[7..].iter().all(u8::is_ascii_digit)
    {
        return None;
    }
    let (layer, index) = LAYERS
        .into_iter()
        .find_map(|layer| addressed.strip_prefix(layer).map(|rest| (layer, rest)))?;
    let index = if index.is_empty() {
        0
    } else if digits(index) {
        index.parse().ok()?
    } else {
        return None;
    };
    if !digits(slot) || proc_name.is_empty() || sanitize(proc_name) != proc_name {
        return None;
    }
    Some(Version {
        at: format!("{date}-{time}"),
        slot: slot.parse().ok()?,
        layer,
        index,
        proc_name: proc_name.to_string(),
        set,
        file: entry.path(),
    })
}

/// Sanitizes a procedure name for safe filesystem path usage.
fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "unnamed".to_string()
    } else {
        cleaned
    }
}

/// Records initial snapshots of all active slot procedures at startup to establish undo baselines.
pub fn seed<'a>(
    shared: &Shared,
    sets: impl Iterator<Item = (usize, Option<&'a str>, Vec<&'a Path>)>,
) {
    let Ok(mut snapshots) = shared.lock() else {
        return;
    };
    for (slot, set, paths) in sets {
        // Seed each file under its declared layer and index within the slot.
        let mut counts: std::collections::HashMap<&'static str, usize> =
            std::collections::HashMap::new();
        for (positional, path) in paths.into_iter().enumerate() {
            let Ok(source) = std::fs::read(path) else {
                continue;
            };
            let layer = declared_kind(&source).unwrap_or(if positional == 0 { "L1" } else { "L4" });
            let index = counts.entry(layer).or_insert(0);
            let (layer, index) = (layer, *index);
            *counts.get_mut(layer).expect("just inserted") += 1;
            // Scan procedure name from source text before compilation.
            let name = declared_name(&source).unwrap_or_else(|| "start".to_string());
            if let Err(e) = snapshots.record(slot, layer, index, set, &name, &source) {
                eprintln!("slot {slot}: the starting {layer} is not in the edit history: {e}");
            }
        }
    }
}

/// Extracts the declared layer keyword following `kind` in the source text.
pub fn declared_kind(source: &[u8]) -> Option<&'static str> {
    let text = std::str::from_utf8(source).ok()?;
    for line in text.lines() {
        let Some(rest) = line.trim_start().strip_prefix("kind") else {
            continue;
        };
        if !rest.starts_with(char::is_whitespace) {
            continue;
        }
        return known_layer(rest.trim_start().split(char::is_whitespace).next()?);
    }
    None
}

/// Extracts the procedure identifier following `proc` in the source text.
fn declared_name(source: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(source).ok()?;
    for line in text.lines() {
        let line = line.trim_start();
        if let Some(rest) = line.strip_prefix("proc ") {
            let name = rest
                .trim_start()
                .split(|c: char| c.is_whitespace() || c == '{')
                .next()?;
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests;
