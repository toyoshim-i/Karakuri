use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use karakuri_store::hash::Hash;
use karakuri_store::ndjson::Line;
use karakuri_store::record::{Layer, NodeAddress, Record};
use karakuri_store::store::Store;

use crate::meta::layer_name;

use super::summary::node_called;

// -- Resolution: the authoring form, into the one a store may hold -------

/// File extension for authoring Set files (`.kset`), distinguished from resolved `.kbset` files.
pub const AUTHORING_SUFFIX: &str = ".kset";

/// Resolves an authoring `.kset` file into resolved lines by hashing each referenced
/// `part` relative to the file's parent directory and storing it as an artifact.
///
/// Enforces directory containment for all part paths before reading or storing any artifacts.
pub fn resolve(store: &Store, path: &Path) -> Result<Vec<Line>, String> {
    // Validate the authoring suffix; resolved `.kbset` files must not be re-resolved.
    let named = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if !named.ends_with(AUTHORING_SUFFIX) {
        return Err(format!(
            "`{}`: an authoring Set file is named `<name>{AUTHORING_SUFFIX}` and this is not — \
             a Set's two forms are told apart by their extensions, and the resolved one \
             (`{}`) names every node by content address and is read rather than resolved",
            path.display(),
            Store::SET_FILE_SUFFIX
        ));
    }
    let lines = karakuri_store::ndjson::read(path)
        .map_err(|e| format!("reading `{}`: {e}", path.display()))?;

    // Base directory for resolving bundle parts. Canonicalized to resolve symlinks
    // (such as macOS `/var` -> `/private/var`) before prefix checks.
    let dir = match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        // `foo.kset` with no directory at all: the file is in the working
        // directory, and so are its parts.
        _ => Path::new("."),
    };
    let root = std::fs::canonicalize(dir).map_err(|e| {
        format!(
            "`{}`: its own directory `{}` cannot be resolved ({e}), and a part is named \
             relative to it",
            path.display(),
            dir.display()
        )
    })?;

    // **Every path checked before any of them is read.** See this function's
    // doc: a refusal in the middle of a file that had already stored three
    // artifacts is a refusal with a mess behind it.
    let mut checked: Vec<PathBuf> = Vec::new();
    for line in &lines {
        if let Record::Part {
            layer,
            index,
            name,
            path: include,
        } = line.record()
        {
            checked.push(contained(
                path,
                &root,
                include,
                &part_at(*layer, *index, name.as_deref(), include),
            )?);
        }
    }

    let mut out = Vec::with_capacity(lines.len());
    let mut checked = checked.into_iter();
    for line in &lines {
        match line.record() {
            Record::Part {
                layer,
                index,
                name,
                path: include,
            } => {
                let file = checked
                    .next()
                    .expect("one checked path per part, in the order the parts were met");
                let called = part_at(*layer, *index, name.as_deref(), include);
                let source = std::fs::read(&file).map_err(|e| {
                    format!(
                        "`{}`: {called} names `{include}` and it cannot be read ({e})",
                        path.display()
                    )
                })?;
                // **The bytes as they are on disk**, because the hash is of the
                // bytes: reading the file as text and writing it back would put
                // a re-encoding between what the operator has and what the
                // address names.
                let proc_hash = store.put_artifact(&source).map_err(|e| {
                    format!(
                        "`{}`: {called} names `{include}` and storing it failed ({e})",
                        path.display()
                    )
                })?;
                // **The name is carried across**, because it is the same node:
                // a `part` says what a `slot` says, and an `edge` in this same
                // file points at it by that name.
                out.push(Line::new(Record::Slot {
                    at: NodeAddress {
                        layer: *layer,
                        index: *index,
                    },
                    name: name.clone(),
                    proc_hash,
                }));
            }
            // Pass through all non-part records unchanged.
            _ => out.push(line.clone()),
        }
    }
    Ok(out)
}

/// Resolves the authoring Set file at `path` and inlines every referenced source.
pub fn bundle_authored(store: &Store, path: &Path) -> Result<Vec<Line>, String> {
    let lines = resolve(store, path)?;
    // Extract Set ID from records or fall back to file path for diagnostics.
    let id = lines
        .iter()
        .find_map(|line| match line.record() {
            Record::Set { id, .. } => Some(id.clone()),
            _ => None,
        })
        .unwrap_or_else(|| path.display().to_string());
    with_inlined_source(store, &id, lines)
}

/// Validates that an included path resolves strictly within the authoring file's root directory.
///
/// Rejects absolute paths, lexical parent traversals escaping root, and symlinks escaping root.
fn contained(file: &Path, root: &Path, include: &str, called: &str) -> Result<PathBuf, String> {
    let refusal = |what: String| {
        format!(
            "`{}`: {called} names `{include}`, and a part names a `.kir` under the Set file's \
             own directory — {what}. Refused rather than repaired, and nothing was stored",
            file.display()
        )
    };
    if include.is_empty() {
        return Err(refusal(
            "this names nothing at all, and a node is its procedure".to_string(),
        ));
    }
    let spelled = Path::new(include);
    // Lexical, and first, so that the filesystem is never asked about a path
    // that has already left. Two of the three refusals below need no disk at
    // all, which is what lets them speak about a path that does not exist.
    let mut depth: i32 = 0;
    for part in spelled.components() {
        match part {
            Component::Prefix(_) | Component::RootDir => {
                return Err(refusal(format!(
                    "an absolute path is not relative to anything, so it names a file under \
                     `{}` only by coincidence",
                    root.display()
                )))
            }
            Component::CurDir => {}
            Component::Normal(_) => depth += 1,
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return Err(refusal(format!(
                        "this climbs out of `{}`, which is that directory",
                        root.display()
                    )));
                }
            }
        }
    }
    // **The link check, and it is a second question rather than a stricter
    // version of the first.** Nothing lexical can see a symlink, and nothing
    // about the filesystem can be asked of a path that is not there — so the
    // two run in this order and say different things.
    let real = std::fs::canonicalize(root.join(spelled)).map_err(|e| {
        format!(
            "`{}`: {called} names `{include}` and there is no such file beside the Set file \
             ({e}) — an authoring Set file lives beside the parts it names",
            file.display()
        )
    })?;
    if !real.starts_with(root) {
        return Err(refusal(format!(
            "this resolves to `{}` and leaves `{}` — a symlink out is a `..` that climbs, in \
             another spelling",
            real.display(),
            root.display()
        )));
    }
    Ok(real)
}

/// Formats a node description for an authoring file part prior to hashing.
pub(crate) fn part_at(layer: Layer, index: u32, name: Option<&str>, path: &str) -> String {
    format!("{}:{index} `{}`", layer_name(layer), name.unwrap_or(path))
}

// -- Bundling: a Set file that carries its own material ------------------

/// Bundles a Set by appending inlined source records (`src`) for all referenced artifacts.
pub fn bundle(store: &Store, id: &str) -> Result<Vec<Line>, String> {
    let lines = store
        .read_set(id)
        .map_err(|e| format!("reading set `{id}`: {e}"))?;
    with_inlined_source(store, id, lines)
}

/// The inlining itself, over lines already in hand.
fn with_inlined_source(store: &Store, id: &str, lines: Vec<Line>) -> Result<Vec<Line>, String> {
    let mut out = lines;
    // Append `src` runs after existing records preserving initial file layout.
    let mut runs = Vec::new();
    // Emit one source run per distinct artifact in first-reference order.
    let mut inlined: Vec<Hash> = Vec::new();
    for line in &out {
        let Record::Slot {
            at,
            name,
            proc_hash,
        } = line.record()
        else {
            continue;
        };
        if inlined.contains(proc_hash) {
            continue;
        }
        inlined.push(*proc_hash);
        // Missing artifacts refuse bundling immediately to prevent incomplete packages.
        let bytes = store.get_artifact(proc_hash).map_err(|e| {
            format!(
                "set `{id}`: {} is not in this store ({e}), so it cannot be inlined — \
                 a bundle carries every source or it is not one",
                node_at(at.layer, at.index, name.as_deref(), proc_hash)
            )
        })?;
        let src = String::from_utf8(bytes).map_err(|e| {
            format!(
                "set `{id}`: the source of {} is not UTF-8: {e}",
                node_at(at.layer, at.index, name.as_deref(), proc_hash)
            )
        })?;
        // Use `split('\n')` rather than `lines()` to preserve trailing newlines for exact hash matching.
        for (n, text) in src.split('\n').enumerate() {
            runs.push(Line::new(Record::Src {
                hash: *proc_hash,
                line: n as u32,
                s: text.to_string(),
            }));
        }
    }
    out.append(&mut runs);
    Ok(out)
}

/// How a refusal names one node: its address, and what it is called.
fn node_at(layer: Layer, index: u32, name: Option<&str>, hash: &Hash) -> String {
    format!(
        "{}:{index} `{}`",
        layer_name(layer),
        node_called(name, None, hash)
    )
}

/// What one `slot` record of a bundle says, kept for the sentences [`unbundle`]
/// owes about it.
struct Slot {
    layer: Layer,
    index: u32,
    name: Option<String>,
    hash: Hash,
}

impl Slot {
    fn called(&self) -> String {
        node_at(self.layer, self.index, self.name.as_deref(), &self.hash)
    }
}

/// Where the file being taken in came from. Decides whether an id this store
/// already holds is refused or replaced — ADR-0347.
///
/// No `Default`: like [`crate::Asked`], every call site says which.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameFrom {
    /// A `.kbset` that arrived, a row of a dropped folder, a path typed at
    /// `--take-in`. A taken id is refused.
    Somebody,
    /// The preset library this run resolved (ADR-0230). A taken id is replaced,
    /// and what was there is kept under [`retired_as`] first.
    TheShippedLibrary,
}

/// What the Set filed under `id` is filed again as before it is replaced:
/// `<id>-<stamp>`, in [`crate::history::stamped_id`]'s spelling.
pub fn retired_as(id: &str) -> String {
    format!("{id}-{}", crate::history::stamped_id())
}

/// Unbundles inlined sources and writes artifacts and Set definitions into the store.
///
/// Verifies source hashes against referenced slot addresses before writing any files.
/// Existing IDs are protected from overwrite unless originating from the shipped library.
pub fn unbundle(store: &Store, came: CameFrom, lines: &[Line]) -> Result<String, String> {
    let mut file_id = None;
    let mut slots: Vec<Slot> = Vec::new();
    // Keyed and folded exactly as [`from_lines`](crate::setfile::from_lines) does it, so what is hashed
    // below is the text the reader will reconstruct rather than a second
    // reading of the same records.
    let mut inlined: BTreeMap<Hash, BTreeMap<u32, String>> = BTreeMap::new();
    for line in lines {
        match line.record() {
            Record::Set { id, .. } => file_id = Some(id.clone()),
            Record::Slot {
                at,
                name,
                proc_hash,
            } => slots.push(Slot {
                layer: at.layer,
                index: at.index,
                name: name.clone(),
                hash: *proc_hash,
            }),
            Record::Src { hash, line, s } => {
                inlined.entry(*hash).or_default().insert(*line, s.clone());
            }
            _ => {}
        }
    }
    let Some(file_id) = file_id else {
        return Err(
            "this file carries no `set` record, so it names no id to file itself \
                    under — taking a Set in takes the id from the file rather than from \
                    the command line"
                .to_string(),
        );
    };
    // Asked before a byte is written, for the reason in this function's own
    // doc: the id in the file is somebody else's choice of word.
    let held = store
        .list_sets()
        .map_err(|e| format!("reading what this store already holds: {e}"))?;
    // Decided here, performed at the write, so `Somebody` still returns before
    // an artifact is stored.
    let replacing = held.iter().any(|entry| entry.id == file_id);
    if replacing && came == CameFrom::Somebody {
        return Err(format!(
            "set `{file_id}` is already in this store, and taking a Set in does not \
             overwrite one: the id came from the file rather than from you. Nothing was \
             stored. Edit the `set` record's id, or move the set you have"
        ));
    }
    // **Every inlined source hashes to the hash its `slot` record names**, or
    // the file is refused whole. This is the check that makes a bundle
    // trustworthy at all: without it a `src` run is a way to file arbitrary
    // text under an address an operator recognises.
    let mut sources = Vec::new();
    for (hash, run) in &inlined {
        let text = run.values().cloned().collect::<Vec<_>>().join("\n");
        let actual = Hash::of(text.as_bytes());
        let called = slots
            .iter()
            .find(|slot| slot.hash == *hash)
            .map(Slot::called)
            .unwrap_or_else(|| format!("`{}`", hash.short(12)));
        if actual != *hash {
            return Err(format!(
                "{called}: the inlined source hashes to {} and the file files it under {} — \
                 a content-addressed store's whole guarantee is that a hash names those \
                 bytes, so this bundle is refused and nothing was stored",
                actual.short(12),
                hash.short(12)
            ));
        }
        sources.push((*hash, text, called));
    }
    // **A `slot` naming an artifact that is neither inlined nor already here**
    // is a file that is not self-contained, and it is refused naming it. One
    // that is already in the store and not inlined is fine — that is an
    // ordinary partial bundle, and the store answers for it.
    for slot in &slots {
        if inlined.contains_key(&slot.hash) || store.get_artifact(&slot.hash).is_ok() {
            continue;
        }
        return Err(format!(
            "{}: neither inlined in this file nor in this store, so the file is not \
             self-contained and nothing was stored — it wants bundling again where its \
             material is",
            slot.called()
        ));
    }

    let mut notes = Vec::new();
    let mut cards = 0;
    for (hash, text, called) in &sources {
        store
            .put_artifact(text.as_bytes())
            .map_err(|e| format!("{called}: storing the source: {e}"))?;
        if !slots.iter().any(|slot| slot.hash == *hash) {
            notes.push(format!(
                "{called} is inlined and no `slot` record references it; it is stored anyway"
            ));
        }
        // **The card is what a compile produces**, so it is written here and by
        // `put_meta` — the one both compile paths already go through — rather
        // than by a second writer of the same file.
        match crate::compile::check(text) {
            Ok(checked) => {
                if crate::meta::put_meta(store, hash, &crate::meta::card(hash, &checked)).is_none()
                {
                    cards += 1;
                }
            }
            // **Stored, filed, and reported** — see this function's doc. The
            // note carries the checker's own words, because "one source did not
            // compile" is not something an operator can act on and a diagnostic
            // with a span is.
            Err(report) => notes.push(format!(
                "{called} does not compile on this build, so it has no metadata card. It is \
                  stored and it keeps its slot; `--load-set {file_id}` will refuse it and say:\n{}",
                report
                    .lines()
                    .map(|l| format!("      {l}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            )),
        }
    }
    // Strip `src` records from the persisted Set file since artifacts are now stored.
    let kept: Vec<Line> = lines
        .iter()
        .filter(|line| !matches!(line.record(), Record::Src { .. }))
        .cloned()
        .collect();
    // Only reachable as the shipped library: `Somebody` returned above.
    let retired = match replacing {
        false => None,
        true => {
            let there = store
                .read_set(&file_id)
                .map_err(|e| format!("reading the `{file_id}` this store holds: {e}"))?;
            // The records and not the lines: `Line` carries its original text
            // too, so a different key order would read as a change.
            if there
                .iter()
                .map(Line::record)
                .eq(kept.iter().map(Line::record))
            {
                return Ok(format!(
                    "`{file_id}` is already what the preset library ships — nothing was \
                     written, and the Set this store holds is the one you pressed\n"
                ));
            }
            let retired = retired_as(&file_id);
            // `stamped_id` hands out each spelling once, so this cannot
            // happen in one run. Asked anyway: a collision would overwrite the
            // copy this branch exists to keep.
            if held.iter().any(|entry| entry.id == retired) {
                return Err(format!(
                    "keeping the `{file_id}` this store holds would be filed as `{retired}`, \
                     which this store also holds. Nothing was written"
                ));
            }
            // The `set` record is rewritten to the id it is filed under, so
            // the file does not carry two answers to what it is called.
            let renamed: Vec<Line> = there
                .iter()
                .map(|line| match line.record() {
                    Record::Set { v, .. } => Line::new(Record::Set {
                        id: retired.clone(),
                        v: *v,
                    }),
                    _ => line.clone(),
                })
                .collect();
            store
                .write_set(&retired, &renamed)
                .map_err(|e| format!("keeping the `{file_id}` this store holds: {e}"))?;
            Some(retired)
        }
    };
    store
        .write_set(&file_id, &kept)
        .map_err(|e| format!("writing set `{file_id}`: {e}"))?;

    let mut said = format!(
        "{} `{file_id}` in: {} node{}, {} source{} stored, {cards} metadata card{} written\n",
        match retired.is_some() {
            true => "replaced",
            false => "took",
        },
        slots.len(),
        plural(slots.len()),
        sources.len(),
        plural(sources.len()),
        plural(cards),
    );
    if let Some(retired) = &retired {
        said.push_str(&format!(
            "  the `{file_id}` that was here is kept as `{retired}`\n"
        ));
    }
    for note in &notes {
        said.push_str(&format!("  {note}\n"));
    }
    Ok(said)
}

/// The `s` on a count, so a report reads as a sentence rather than as a form.
pub(crate) fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}
