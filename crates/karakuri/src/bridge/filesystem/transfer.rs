use super::*;

/// How many rows of the edit history one walk asks for.
///
/// `karakuri_environment::history::list` takes the number from its caller and
/// has no default, because *"what a bay can afford to draw and what a model can
/// afford to be handed are different numbers"* (P-0090) — so this is the
/// panel's answer and nowhere else's.
///
/// Larger than any bay can draw, and small enough that the walk stops after a
/// handful of days. The Library bay's list is one row per
/// `view::size::LIB_ROW_H`, so a full-height bay on a tall display draws a few
/// tens of them; the cost of the walk is one `read_dir` per day directory
/// entered and no file opened at all, and it stops entering them once it has
/// this many. What lies past it is not counted — counting it is the cost the
/// cap exists not to pay — and `Listing::stopped_short` says the walk stopped,
/// which is the property that matters.
pub(crate) const HISTORY_MOST: usize = 200;

/// The rows the `history` scope lists, and the sentence about how they were
/// found.
///
/// Split out of [`listing`] because it is the one arm of that function that has
/// something to say beside the count: the walk is capped and it is capped on
/// *days opened* rather than on this Set's rows, so a listing that stopped
/// short has to say so or it reads as the whole history.
///
/// A row is the name the store filed the version under, less the Set id every
/// row here shares — see [`version_row`], which is also how a landing finds the
/// file again.
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

/// One version, as a row of the Library bay's list and as the word a landing
/// names it by.
///
/// It is the name [`karakuri_environment::history::Snapshots::record`] wrote,
/// less the `@<set>` every row of one walk shares and less the `.kir` — when,
/// which slot, which layer and index, and what the procedure called itself,
/// which is what `Version`'s own head says a row is for.
///
/// One spelling, used twice. The bay is handed this and hands it back at the
/// press, and [`restored`] rebuilds it per candidate to find the file again —
/// so the row an operator pressed and the version that is landed cannot come
/// apart, and no path crosses the seam. That is `SetTransfer::Take`'s
/// arrangement: the panel re-asks the listing and finds the row by the word
/// that was pressed.
///
/// The index is spelled only when it is not the first, which is `record`'s own
/// rule read back rather than a second one: a `_0` on every L4 of every
/// ordinary run is noise in the way of what a person is scanning for.
pub(crate) fn version_row(version: &karakuri_environment::history::Version) -> String {
    // **The spelling is the history module's**, since 2026-09-10: a model
    // walking the same history over MCP reads rows out of `walk_history` and
    // hands one back in `Revision::Picked`, so a second `format!` here would be
    // a second answer to *what is this row called* — and the two surfaces hand
    // the name to each other (`docs/adr/0342-…`).
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

    /// The file behind the word that was pressed, or a sentence saying why there is
    /// not one.
    ///
    /// # Two refusals, and the second is the one a folder brought
    ///
    /// A word this listing no longer holds is the row having gone between the
    /// listing and the press — a directory this program neither made nor writes,
    /// which is a folder's ordinary condition and a preset root's unusual one.
    ///
    /// A word two files wear is `folder_files`' own note arriving: a directory
    /// holding `night.kbset` and `night.kset` draws two rows reading `night`, and
    /// neither the listing nor the bay puts a precedence between the two forms. So
    /// the press is refused and both file names go back
    /// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)),
    /// because taking one of them would be this program choosing between two rows
    /// an operator cannot tell apart on screen. A `presets` root can hold only
    /// `.kset` files, so this arm is a folder's in practice and is asked of both
    /// because the rule is the row's rather than the scope's.
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

/// A row of `presets` or of a `folder`, taken into this store, and the id it
/// landed under.
///
/// # Taking it in is not a second operation, and it is what gives it a name
///
/// `docs/manual/operations.html`'s *Send a Set to somebody, and take one in*:
/// *"Taking one in is not a second row — opening a preset is this row"*, so
/// opening a preset is that row performed. `console.html` reaches it from the
/// other side: *"loading a preset is a packaging step, and a packaging step
/// writes into the store: `my sets` gains a row you did not make."* That is
/// what this does, and it is why a preset row is one press rather than two —
/// the take-in is what gives the Set the id the load needs.
///
/// # It is `karakuri-cli`'s own route and not a second one
///
/// `taken_in_file` resolves a `.kset` with `setfile::bundle_authored` — which
/// is `setfile::resolve` behind its wall, and then the inlining — and hands the
/// result to `setfile::unbundle`. Resolved *and* inlined rather than resolved
/// alone, for that function's stated reason: `unbundle` writes a metadata card
/// for each source the lines carry, and handing it resolved lines with nothing
/// inlined would file the Set and leave every artifact cardless. Two routes
/// into one store that reach two different stores is the disagreement a second
/// spelling always is.
///
/// Both spellings, and the branch is that function's too. A `.kbset` has its
/// sources inside it and is read straight off the disk as lines; a `.kset`
/// names its parts by relative path and is resolved first. The `presets` scope
/// lists only the second form, so this branch was not reachable until a folder
/// row could be pressed — `console.html`: *"A Set file and a bundle are the
/// same file … so the scope draws one kind of row rather than two"*, and the
/// difference between them is a property of a file rather than a kind of row.
///
/// The two binaries have no library target between them, which is why this is a
/// second spelling of `karakuri-cli`'s eight lines rather than a call to them,
/// and it is written down here rather than left to be discovered: the branch is
/// the same branch and the two must not come apart the day a third form
/// arrives.
///
/// The wall is `resolve`'s and not this file's: a part named from outside the
/// file's own directory is refused, by path, because *"a Set somebody handed
/// you is not a way of asking this machine for its files"* (ADR-0229). Nothing
/// here loosens it and nothing here repeats it.
///
/// # An id this store already holds is settled there, and not written here
///
/// `setfile::unbundle` asks what the store holds before it writes a byte, and
/// what it does about a taken id depends on [`Taking::came_from`]: a folder row
/// is refused, a preset row replaces and keeps what was there under a stamped
/// id (ADR-0347). Both sentences are that function's. A second check here would
/// be a second answer to *may this be overwritten*, and the two would disagree
/// the day one of them moved. What this side adds is which of the two acts
/// failed where one does: nothing was taken in, so nothing was loaded, and the
/// deck is exactly as it was.
///
/// So a preset row now always ends with the preset loaded. Before ADR-0347 a
/// store holding a copy from before the shipped parts moved met a refusal and
/// no load, on every press.
///
/// # The id is the file's own
///
/// Read off the `set` record in the resolved lines rather than taken from the
/// row's word, because that is the id `unbundle` files it under and the id `my
/// sets` will list. They are the same word in `examples/`, and a preset whose
/// file says otherwise would otherwise be loaded by a name the store does not
/// hold.
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

/// One Set as the bytes of a `.kbset`, which is `karakuri-cli`'s `packaged_set`
/// with the authoring half taken out.
///
/// This side never packages a `.kset`: the id half is the whole of what a row
/// of a library listing can name, and the flag's other spelling is *take in
/// then send in one flag*
/// ([ADR-0260](../../../docs/adr/0260-sending-a-set-is-a-read-and-a-reads-answer-goes-where-the-surface-that-asked-puts-answers.md)),
/// which is two presses here and already reached.
///
/// `setfile::bundle` is where the inlining and its one refusal live (ADR-0231):
/// one missing artifact refuses the whole thing and names the node, because a
/// bundle short of a procedure looks self-contained and is not. Nothing here
/// repeats that and nothing here loosens it.
pub(crate) fn bundled(root: &std::path::Path, id: &str) -> Result<String, String> {
    let store = Store::open(root).map_err(|e| format!("store `{}`: {e}", root.display()))?;
    Ok(setfile::bundle(&store, id)?
        .iter()
        .map(|line| format!("{}\n", line.as_str()))
        .collect())
}

/// Put a library Set on a running deck, which is the whole of what
/// `Operation::LoadSet` needed and is a re-point rather than an install.
///
/// # It writes files and sends a description, and it builds nothing
///
/// `Deck::install` is the one function that puts a built Set in a slot, and it
/// is *"deliberately not reachable from a key or a surface: a live run changes
/// its material by editing a file and letting the worker build it, which is
/// what the budget watchdog is attached to."* So this does what an operator
/// with an editor does, in one press: it reads the Set out of the store, writes
/// every procedure in it into the scratch, and tells that slot's watcher to
/// look there instead. Everything after this line is the path a save already
/// takes — the worker compiles off the render thread, the swap lands at a frame
/// boundary, and the watchdog judges it there on what one frame of that Set
/// costs and rolls it back on its own if that is over the budget. The library
/// gets all of that for nothing, and no second route into a slot is opened.
///
/// Nothing here is on the frame path. A store read, a `setfile::load` that
/// checks every procedure, and up to a handful of small writes — on the press,
/// which is where this file already reads a directory (`arrangement`), and
/// never on a frame (P-0091). The compile is the worker's.
///
/// # The scratch name carries the slot, and it is the directory's rule now
///
/// `scratch::place` writes `<store>/scratch/<name>.kir` and overwrites what is
/// there, so two decks loading two Sets whose procedures happen to share a name
/// would be one file: the second load would move the first deck as well, on its
/// watcher's next poll, and nothing would say why. The name is therefore
/// `A0-drift.kir` — the deck letter, the node's place in the Set, and the
/// procedure's own name — which is unique per slot and per node, stays readable
/// in an editor, and says which deck an open file belongs to.
///
/// It is `scratch::node_name` rather than a `format!` here, because that
/// argument was never about loading. It is about two decks and one directory,
/// which is every run: every slot is materialised under the same spelling at
/// startup ([`working_copies`]), so a load writes into a directory already laid
/// out this way and a second spelling would be a second answer.
///
/// # What the aim states, and why all of it
///
/// [`watch::Aim`] is `Watch::new`'s argument list less the slot, and every
/// field here is read off the Set file rather than left at this program's
/// startup value — which is the failure each of `Watch`'s own fields is
/// documented against, and which would not show on the load at all. A layering,
/// a fold or a camera left behind is a slot that loads correctly and then comes
/// back as a different picture on the first later save.
///
/// The authorities are the one exception and are empty: `Record::Authority` is
/// deliberately not Set-file state, so a Set carries no grants and a load
/// starts a slot with none — which is what `--load-set` gives one.
///
/// The `Err` is a sentence for the operator. Every way this fails leaves the
/// deck exactly as it was: a store that will not open, a Set that is not there,
/// a procedure in it that no longer checks, a scratch that will not be written,
/// or a worker that has gone.
pub(crate) fn loading(
    root: &std::path::Path,
    slot: usize,
    salt: u32,
    // **The slot's [`Aiming`] and not its sender**, so that where the watcher
    // is pointed is kept with what was sent. A load that sent an aim and left
    // `Aiming::at` behind would leave the next rewiring restating the pair the
    // run launched with — see [`Aiming`].
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
            // **The directory's one naming rule, asked rather than spelled
            // again.** It was written out here when this was the only thing in
            // this program that wrote into the scratch; every slot is
            // materialised at startup now, so a second spelling of
            // `A0-drift.kir` would be a second answer to what a scratch file
            // is called — and the two would disagree on the day one of them
            // moved.
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

    // **The file's salts, and a derived one where it recorded none** — the
    // rule `karakuri-cli`'s `salts_for` follows, restated here because that
    // program has no library target. The seed is the file's first salt where
    // it has one and this slot's own where it has not, so a Set that recorded
    // its colours comes back with them and one that did not is salted like the
    // slot it landed in.
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
        // **The first geometry's recorded capacity over all of them**, which
        // is what `--load-set` folds into `--capacity` and is `Watch`'s own
        // shape: one `Option<u32>` for the slot, because a rebuild recompiles
        // the files and each geometry's own declaration is the default. A Set
        // that recorded two different capacities loses the second, which is a
        // limit this program shares with the command line rather than one it
        // invented.
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
        // **What this slot is now running, and what every version it writes
        // from here on is filed under.** It is the id the operator picked out
        // of the library, carried on the aim because that is what a re-point
        // moves: left off, the loaded Set's whole chain would go on being filed
        // under the material the slot was running before the press, in names
        // nothing reads back (ADR-0276).
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
