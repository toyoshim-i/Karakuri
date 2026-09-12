use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use karakuri_store::hash::Hash;
use karakuri_store::ndjson::Line;
use karakuri_store::record::{Layer, NodeAddress, Record};
use karakuri_store::store::Store;

use crate::meta::layer_name;

use super::summary::node_called;

// -- Resolution: the authoring form, into the one a store may hold -------

/// The extension an authoring Set file wears, and the whole of how one is told
/// from a resolved one
/// ([ADR-0231](../../../docs/adr/0231-a-sets-two-forms-take-two-extensions-and-the-store-holds-only-the-resolved-one.md)).
/// `Store::SET_FILE_SUFFIX` is the other half and is the store's, because the
/// store is the thing that may hold only that one.
pub const AUTHORING_SUFFIX: &str = ".kset";

/// Resolve the authoring Set file at `path` into the resolved Set file it
/// names: every `part` read from disk relative to *this file's own directory*,
/// hashed, put in the store as an artifact, and written out as a `slot` naming
/// that address. Every other record is passed through unchanged and in place.
///
/// This is the missing half of packaging, and it is the same operation
/// [`bundle`] performs at another moment
/// ([ADR-0229](../../../docs/adr/0229-a-set-file-is-authored-beside-its-parts-and-travels-as-a-bundle.md)
/// part 4, *"one operation, two moments"*): loading an authoring file *is*
/// packaging it, and packaging for distribution is the same resolution done
/// ahead of time. [`bundle`] starts from `store.read_set(id)` and so can only
/// carry what a store already holds; this starts from a file on a disk that has
/// never been in one.
///
/// A read, a hash and a store put — never a compile. A `slot`'s `proc` is the
/// content address of the `.kir` *source*, so nothing here parses a procedure
/// or asks a device for anything: the checker runs where a Set is built, which
/// is [`from_lines`](crate::setfile::from_lines) and [`unbundle`], and running
/// it here as well would be a second place that decides whether material is
/// admissible.
///
/// Lines back rather than a file written, on [`bundle`]'s terms: where the
/// result goes is the caller's, and the two callers want different things —
/// `--package FILE.kset` inlines them and prints, where a load would hand them
/// to [`from_lines`](crate::setfile::from_lines).
///
/// The wall is this function and not a later one. ADR-0229: *"the wall is not a
/// hardening pass to add afterwards, because the first thing that resolves an
/// include without one is the defect."* Every path is put through [`contained`]
/// before a single byte is read or stored, so a file with one escape in it
/// stores nothing at all — a refusal that had already filed three artifacts
/// would be a refusal an operator has to clean up after.
pub fn resolve(store: &Store, path: &Path) -> Result<Vec<Line>, String> {
    // **The extension is checked here and not only by whoever routed us**,
    // because it is the whole of what says which form a file is, and a function
    // whose contract is "resolve an authoring file" that resolves anything
    // handed to it is a promise nothing keeps. A `.kbset` is *read* rather than
    // resolved — it has nothing left to resolve, which is what its name asserts.
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

    // **The directory the file is in, and the whole of what its parts may
    // name.** Canonical, because the comparison below is against it: on this
    // maintainer's own machine a temporary directory is `/var/folders/…`, which
    // *is* a symlink to `/private/var/folders/…`, so a root taken as spelled
    // would fail to contain every path under it — the wall would refuse
    // everything, and the first fix anybody reaches for when a wall refuses
    // everything is to loosen it.
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
            // **Everything else, unchanged and in place.** `capacity`, `param`,
            // `bind`, `camera`, `seed`, `edge` and `merge` mean the same thing
            // in both forms — only how a node's source is named differs — so
            // resolution is not a rewrite of the file, it is a rewrite of one
            // record type. A `slot` already in an authoring file passes through
            // here too: it names material by address, which the store must
            // already hold, and that is unusual rather than wrong.
            _ => out.push(line.clone()),
        }
    }
    Ok(out)
}

/// Resolve the authoring Set file at `path` and inline every source it names:
/// [`resolve`] and then the inlining [`bundle`] does, which is the packaging
/// step end to end.
///
/// The two are separate functions and one call because they are separate facts:
/// resolution is what turns paths into addresses, and inlining is what makes
/// the result travel. A load would want the first and not the second.
pub fn bundle_authored(store: &Store, path: &Path) -> Result<Vec<Line>, String> {
    let lines = resolve(store, path)?;
    // **The id is the file's own**, for the sentences the inlining owes about a
    // node it cannot carry. A file that names none is named by its own path
    // here — and refused later, by `unbundle`, with the sentence that already
    // exists for it: taking a Set in takes the id from the file rather than
    // from the command line.
    let id = lines
        .iter()
        .find_map(|line| match line.record() {
            Record::Set { id, .. } => Some(id.clone()),
            _ => None,
        })
        .unwrap_or_else(|| path.display().to_string());
    with_inlined_source(store, &id, lines)
}

/// The wall: the include, resolved, is under the authoring file's own
/// directory — or it is refused by name.
///
/// `root` is that directory, already canonical. The answer is the file to read.
///
/// What transfers from `mcp::checked_id` and `karakuri`'s `checked_name` is
/// where the wall sits, that it refuses rather than repairs, and that the
/// refusal names what it refused — not their rule. Those two guard one path
/// component and allow letters, digits, `-` and `_`; an include is a relative
/// *path* and has separators in it by construction, so the charset rule cannot
/// be copied. The rule here is containment, and ADR-0229's section *The wall
/// the authoring form needs* is where it was decided.
///
/// Three spellings of one escape, and the third is the one that gets
/// missed:
///
/// - an absolute path, which is not relative to anything;
/// - a `..` that climbs out, refused lexically — before the filesystem is
///   asked anything — so that `../../etc/passwd` is refused whether or not it
///   exists. A `..` that does *not* climb out (`sub/../l1.kir`) is an ordinary
///   path and is allowed: what is refused is leaving, not the spelling;
/// - a symlink pointing out, which is why the comparison is between
///   *canonical* paths. `std::fs::canonicalize` resolves every link in the
///   path, so a `parts` directory that is a link to `/etc` is caught along with
///   a `passwd.kir` that is a link to a file in it.
///
/// And the root is canonical for the same reason the target is. A directory
/// reached *through* a symlink — which is every temporary directory on macOS,
/// where `/var` is a link to `/private/var` — would otherwise contain none of
/// its own children by this comparison, and a wall that refuses everything is a
/// wall somebody switches off.
///
/// Refused, never repaired, which is the precedents' rule and this
/// program's: an include quietly rewritten into one that reads is a rule an
/// operator can only find by experiment, and a Set that silently drew from
/// somewhere else is worse than one that did not open.
///
/// Why it exists at all: an authoring file is a thing you are *sent*. An
/// include that escapes its own directory means opening a Set somebody handed
/// you reads any file on your machine and inlines it into a bundle you then
/// hand on.
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

/// How a refusal names one node of an authoring file: its address, and what it
/// is called.
///
/// [`node_at`]'s counterpart, and it cannot be that function: a `part` has no
/// hash, so the last resort a node's name falls back to — the artifact's short
/// address — does not exist yet. What a `part` has instead is the path it
/// names, which is the only other handle an operator has on it.
pub(crate) fn part_at(layer: Layer, index: u32, name: Option<&str>, path: &str) -> String {
    format!("{}:{index} `{}`", layer_name(layer), name.unwrap_or(path))
}

// -- Bundling: a Set file that carries its own material ------------------

/// Bundle the Set filed under `id`: the file it already is, with every source
/// it names inlined after it.
///
/// The bundled form is `docs/ir-spec.md`'s and it is the one
/// [`from_lines`](crate::setfile::from_lines) already reads — a run of `src`
/// records per artifact, keyed by hash, which wins over the store when both
/// could answer. So a bundle loads on a machine whose store has never held the
/// material, which is the whole of what it is for.
///
/// Lines back rather than a file written. Where a bundle goes is the caller's,
/// and the caller writes it to standard output; see `packaged_set` in
/// `karakuri-cli`, which is `--package`'s half of this.
pub fn bundle(store: &Store, id: &str) -> Result<Vec<Line>, String> {
    let lines = store
        .read_set(id)
        .map_err(|e| format!("reading set `{id}`: {e}"))?;
    with_inlined_source(store, id, lines)
}

/// The inlining itself, over lines already in hand.
fn with_inlined_source(store: &Store, id: &str, lines: Vec<Line>) -> Result<Vec<Line>, String> {
    let mut out = lines;
    // **After the records that were already there, and the file's own order is
    // otherwise untouched.** [`from_lines`](crate::setfile::from_lines) folds `src` into a map keyed by
    // hash and line number before it resolves anything, so it requires no
    // position at all — and a bundle that is the saved file plus an appendix
    // diffs against the file it was made from.
    let mut runs = Vec::new();
    // First-reference order, and **one run per artifact however many nodes
    // reference it**: the reader keys `src` by hash, so a second copy of a
    // shared procedure would be bytes nobody reads. `Vec` rather than a set
    // because a Set has a handful of nodes and this keeps the runs in the order
    // the file names them.
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
        // **One missing artifact refuses the whole bundle**, naming the node.
        // A bundle short of one procedure is a file that looks self-contained
        // and is not, and the machine it is carried to is the worst place to
        // find that out — a partial bundle would be discovered by whoever you
        // sent it to rather than by you.
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
        // **`split` and not `lines`**, because this has to be exactly
        // invertible: `s.split('\n').collect::<Vec<_>>().join("\n") == s` for
        // every string, where `lines()` drops a trailing newline and would hand
        // [`unbundle`] bytes that hash to something other than the address the
        // `slot` record names. Every `.kir` ends with one.
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

/// Take a Set somebody sent you into this store: its inlined sources as
/// artifacts, a metadata card per artifact that compiles, and its Set file
/// under the id the file itself carries. `--take-in`'s half of this; the lines
/// are a `.kbset`'s as read, or an authoring file's already put through
/// [`bundle_authored`].
///
/// Nothing is written until every source has been checked. A store's whole
/// guarantee is that a hash names those bytes and no others, so a `src` run
/// whose text hashes to something else is refused — naming the node — before
/// anything reaches the disk. A half-applied bundle would leave the store
/// holding material nobody can name.
///
/// The id comes from the file's own `set` record, and a taken one is refused
/// rather than overwritten. This is deliberately not
/// [`save`](crate::setfile::save)'s rule, which `--save-set ID` and the `k` key
/// share: an id you type is an instruction, and an id that arrived inside
/// somebody else's file is not. Overwriting on a name you chose is you
/// replacing your own preset; overwriting on a name a stranger's file chose is
/// a preset an operator built disappearing because somebody they have never met
/// picked the same word. Being annoying about it costs one rename; the other
/// failure costs work that is gone.
///
/// The report says what happened, including the sources this build's checker
/// will not compile: those are stored and filed all the same, because the Set
/// will then fail on load with the checker's own diagnostics against the source
/// — which tells an operator which line is wrong, where refusing the whole file
/// would tell them only that it was.
pub fn unbundle(store: &Store, lines: &[Line]) -> Result<String, String> {
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
    if held.iter().any(|entry| entry.id == file_id) {
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
    // **The `src` runs are dropped from what is filed.** They are the carrying
    // form — a way to move an artifact between stores — and this store now
    // holds the artifacts, so what is kept is the ordinary Set file that
    // references them by hash. Keeping the runs would file a second copy of
    // every source inside the preset directory, where `--package` can produce
    // one again from the artifacts at any time.
    let kept: Vec<Line> = lines
        .iter()
        .filter(|line| !matches!(line.record(), Record::Src { .. }))
        .cloned()
        .collect();
    store
        .write_set(&file_id, &kept)
        .map_err(|e| format!("writing set `{file_id}`: {e}"))?;

    let mut said = format!(
        "took `{file_id}` in: {} node{}, {} source{} stored, {cards} metadata card{} written\n",
        slots.len(),
        plural(slots.len()),
        sources.len(),
        plural(sources.len()),
        plural(cards),
    );
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
