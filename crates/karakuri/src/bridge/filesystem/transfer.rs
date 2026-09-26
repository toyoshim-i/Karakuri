use super::*;

/// Maximum number of edit history revisions to query and display per walk (ADR-0276).
pub(crate) const HISTORY_MOST: usize = 200;

/// History walk result containing display rows and status explanation.
pub(crate) struct Walked {
    pub(crate) rows: Vec<String>,
    pub(crate) said: String,
}

pub(crate) fn walked(store: &std::path::Path, running: Option<&str>) -> Walked {
    // **No Set, no rows, and not an error.** The deck is playing a pair
    // somebody typed; its versions are filed under no Set, and `why_nothing`
    // is where that is said in words.
    let Some(id) = running else {
        return Walked {
            rows: Vec::new(),
            said: String::new(),
        };
    };
    let found = match karakuri_environment::history::list(store, HISTORY_MOST) {
        Ok(found) => found,
        // Said rather than swallowed, and the scope lists nothing: a history
        // that would not open is a different fact from a Set with no versions,
        // and the two must not draw the same empty list in silence.
        Err(why) => {
            return Walked {
                rows: Vec::new(),
                said: format!("\n  history: {why} — so this scope lists nothing"),
            };
        }
    };
    let rows: Vec<String> = found
        .versions
        .iter()
        // **`Some(id)` and never `None`.** A version written where there was
        // no Set is a version of nothing, so it matches no Set rather than
        // every one of them (ADR-0276).
        .filter(|version| version.set.as_deref() == Some(id))
        .map(version_row)
        .collect();
    let mut said = String::new();
    if found.stopped_short {
        said.push_str(
            "\n  history: the walk stopped with days unread, so this is part of what is \
             there rather than all of it",
        );
    }
    if found.unclaimed > 0 {
        said.push_str(&format!(
            "\n  history: {} entr{} under `history/` that this layout does not claim {} \
             passed over",
            found.unclaimed,
            match found.unclaimed {
                1 => "y",
                _ => "ies",
            },
            match found.unclaimed {
                1 => "was",
                _ => "were",
            }
        ));
    }
    Walked { rows, said }
}

/// Formats a history version record as a display row and revision identifier (ADR-0276).
pub(crate) fn version_row(version: &karakuri_environment::history::Version) -> String {
    // Delegate to history module to keep format consistent across MCP and console (ADR-0342).
    version.filed_as()
}

/// Result of taking a Set into the store: source file, inner Set ID, and status message.
#[derive(Debug)]
pub(crate) struct TakenIn {
    pub(crate) file: std::path::PathBuf,
    pub(crate) id: String,
    pub(crate) said: String,
}

/// Identifies the file listing source for take-in operations (ADR-0230, ADR-0275).
pub(crate) enum Taking<'a> {
    /// The preset library this run resolved (ADR-0230) — a told directory, and the
    /// same one [`presets_listing`] draws the rows of.
    Presets(Option<&'a karakuri_environment::places::Presets>),
    /// The directory somebody dropped on this window (ADR-0275), and `None` for a
    /// bay that has been pointed nowhere.
    Folder(Option<&'a std::path::Path>),
}

impl Taking<'_> {
    /// The rows this listing holds, asked again rather than kept — see
    /// [`taking_in`], where that rule is argued.
    pub(crate) fn rows(&self) -> Vec<FileRow> {
        match self {
            Taking::Presets(presets) => presets_listing(*presets),
            Taking::Folder(dir) => folder_files(*dir),
        }
    }

    /// What the scope is called, for a refusal to name.
    pub(crate) fn scope(&self) -> &'static str {
        match self {
            Taking::Presets(_) => "the preset library",
            Taking::Folder(_) => "the folder this bay is pointed at",
        }
    }

    /// Whose file a row of this listing is, which decides whether an id the
    /// store already holds is refused or replaced (ADR-0347).
    pub(crate) fn came_from(&self) -> karakuri_environment::setfile::CameFrom {
        match self {
            Taking::Presets(_) => karakuri_environment::setfile::CameFrom::TheShippedLibrary,
            Taking::Folder(_) => karakuri_environment::setfile::CameFrom::Somebody,
        }
    }

    /// Resolves the filesystem path for a pressed row name in presets or folder scope (P-0083).
    pub(crate) fn file(&self, row: &str) -> Result<std::path::PathBuf, String> {
        let mut found: Vec<std::path::PathBuf> = self
            .rows()
            .into_iter()
            .filter(|held| held.id == row)
            .map(|held| held.path)
            .collect();
        match found.len() {
            0 => Err(format!("{} has no `{row}` in it any more", self.scope())),
            1 => Ok(found.remove(0)),
            _ => Err(format!(
                "{} holds {} files called `{row}` — {} — and this row names a word rather \
                 than a file, so which of them you meant is not something the listing can \
                 say. Rename or move one of them and press again",
                self.scope(),
                found.len(),
                found
                    .iter()
                    .filter_map(|path| path.file_name().and_then(|name| name.to_str()))
                    .collect::<Vec<_>>()
                    .join(" and ")
            )),
        }
    }
}

/// Takes an external `.kset` or `.kbset` into the store and returns its resolved ID (ADR-0229, ADR-0231, ADR-0347).
pub(crate) fn taking_in(
    root: &std::path::Path,
    from: Taking<'_>,
    row: &str,
) -> Result<TakenIn, String> {
    // **Asked again rather than kept**, which is [`listing`]'s shape: the rows
    // crossed into the console as words, and the file behind a word is found
    // by asking the library again on the press. A second copy of the listing
    // held on this side is a copy that goes on naming a file that has moved.
    let file = from.file(row)?;
    // A property of the listing the row came off, not of the file.
    let came = from.came_from();
    let store = Store::open(root).map_err(|e| format!("store `{}`: {e}", root.display()))?;
    // **The form is the file's own and the branch is `karakuri-cli`'s** — see
    // this function's head. A name is what says which, and nothing is opened
    // to ask: the two suffixes are the two `folder_files` lists.
    let authored = file
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(karakuri_environment::setfile::AUTHORING_SUFFIX));
    let lines = match authored {
        true => karakuri_environment::setfile::bundle_authored(&store, &file)?,
        false => karakuri_store::ndjson::read(&file)
            .map_err(|e| format!("reading `{}`: {e}", file.display()))?,
    };
    let id = lines
        .iter()
        .find_map(|line| match line.record() {
            Record::Set { id, .. } => Some(id.clone()),
            _ => None,
        })
        .ok_or_else(|| {
            format!(
                "`{}` carries no `set` record, so it names no id to file itself under",
                file.display()
            )
        })?;
    let said = karakuri_environment::setfile::unbundle(&store, came, &lines)?;
    Ok(TakenIn { file, id, said })
}

/// Emits the pair of operations corresponding to taking in a Set and loading it into a deck.
pub(crate) fn taken_in_press(deck: u8, taken: TakenIn) -> [Operation; 2] {
    [
        Operation::TransferSet {
            transfer: SetTransfer::Take { file: taken.file },
        },
        Operation::LoadSet {
            deck,
            set: taken.id,
        },
    ]
}

/// The other direction of that row: a Set out of this store and into a file the
/// operator names, asked for and answered without a frame waiting on either
/// half.
pub(crate) fn sending(
    window: &Arc<Window>,
    root: &std::path::Path,
    folder: Option<&std::path::Path>,
    id: &str,
    tx: std::sync::mpsc::Sender<Sent>,
) {
    let mut dialog = rfd::AsyncFileDialog::new().set_file_name(format!(
        "{id}{}",
        karakuri_store::store::Store::SET_FILE_SUFFIX
    ));
    if let Some(folder) = folder {
        dialog = dialog.set_directory(folder);
    }
    let asked = dialog.set_parent(&**window).save_file();
    let (id, root) = (id.to_owned(), root.to_path_buf());
    std::thread::spawn(move || {
        let answer = pollster::block_on(asked).map(|handle| handle.path().to_path_buf());
        let _ = tx.send(sent(&root, id, answer));
    });
}

/// Processes the outcome of an export file dialog, writing the bundled Set or returning a cancelled status.
pub(crate) fn sent(root: &std::path::Path, id: String, to: Option<std::path::PathBuf>) -> Sent {
    let Some(to) = to else {
        return Sent {
            id,
            to: None,
            outcome: Ok(()),
        };
    };
    let outcome = bundled(root, &id).and_then(|text| {
        std::fs::write(&to, text).map_err(|e| format!("writing `{}`: {e}", to.display()))
    });
    Sent {
        id,
        to: Some(to),
        outcome,
    }
}

/// Packages a Set from the store into a bundled `.kbset` byte vector (ADR-0231, ADR-0260).
pub(crate) fn bundled(root: &std::path::Path, id: &str) -> Result<String, String> {
    let store = Store::open(root).map_err(|e| format!("store `{}`: {e}", root.display()))?;
    Ok(setfile::bundle(&store, id)?
        .iter()
        .map(|line| format!("{}\n", line.as_str()))
        .collect())
}

/// Loads a stored Set into a slot by writing procedures to scratch and re-aiming the slot watcher (P-0091).
pub(crate) fn loading(
    root: &std::path::Path,
    slot: usize,
    salt: u32,
    aim: &mut Aiming,
    id: &str,
) -> Result<String, String> {
    let store = Store::open(root).map_err(|e| format!("{}: {e}", root.display()))?;
    let loaded = karakuri_environment::setfile::load(&store, id)?;
    // Said rather than swallowed: a note is the reader telling the operator
    // what it did with a file it could only partly honour, and a load that
    // quietly ignored one is a picture nobody can account for.
    for note in &loaded.notes {
        println!("  load: {note}");
    }

    let letter = deck_letter(slot as u8);
    let mut named = Vec::with_capacity(loaded.srcs.len());
    for (at, ((checked, src), name)) in loaded.nodes().zip(loaded.node_names()).enumerate() {
        let path = karakuri_environment::scratch::place(
            root,
            // Resolve scratch file path using consistent slot and node naming convention.
            &karakuri_environment::scratch::node_name(slot, at, &checked.name),
            src,
        )?;
        // **The Set file's node name and not the procedure's**, which is
        // `--load-set`'s own pairing: an `edge` in the file resolves against
        // the name the file wrote, and a rebuild that called the node whatever
        // its procedure declares would break the slot on its first save.
        named.push(karakuri_environment::compile::Named { name, path });
    }
    let mut named = named.into_iter();
    let head = named
        .next()
        .ok_or_else(|| format!("`{id}` names no procedure at all"))?;

    // Preserve recorded seed/salts or fall back to slot-specific derived seeds.
    let seed_salt = loaded.salts.first().copied().flatten().unwrap_or(salt);
    let salts: Vec<u32> = (0..loaded.l1s.len())
        .map(|at| {
            loaded
                .salts
                .get(at)
                .copied()
                .flatten()
                .unwrap_or_else(|| karakuri_engine::set::derived_salt(seed_salt, at))
        })
        .collect();

    let nodes = loaded.srcs.len();
    aim.re_point(watch::Aim {
        head,
        rest: named.collect(),
        layering: loaded.layering,
        live: loaded.live,
        // Apply recorded capacity override for geometry slots.
        capacity: loaded.capacities.first().copied().flatten(),
        seed_salt,
        salts,
        // A Set holds a built-in camera whatever its files declare, so
        // `Orbit::default()` where the file recorded none is the camera it
        // would have been built with rather than a value invented here.
        camera: loaded.camera.unwrap_or_default(),
        overrides: loaded.params,
        // Nothing in a Set file publishes a control — `setfile` writes none
        // and reads none — so this is empty for the same reason this program's
        // startup watchers pass an empty list: there is no `--publish` here to
        // fold in either (ADR-0216).
        published: Vec::new(),
        bindings: loaded.bindings,
        edges: loaded.edges,
        authorities: Vec::new(),
        // Track the active Set ID so future revisions are recorded under this material (ADR-0276).
        set: Some(id.to_string()),
    })
    .map_err(|()| {
        format!(
            "deck {letter}'s build worker is gone, so `{id}` cannot be built; \
             what is on that deck keeps running"
        )
    })?;
    Ok(format!(
        "  load: deck {letter} <- `{id}` ({nodes} node{}) -> written into {}/{} and its watcher \
         re-pointed; the worker builds it and the budget judges it",
        match nodes {
            1 => "",
            _ => "s",
        },
        root.display(),
        karakuri_environment::scratch::DIR,
    ))
}
