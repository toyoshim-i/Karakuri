use super::*;

/// One row of a scope whose rows are files: the word the bay draws and the file
/// behind it.
///
/// Two fields because the bay lists names and a take-in needs a path: what
/// crosses into the console is a `String` per row (`view::View::library`), and
/// what this program has to be able to find again on the press is the file that
/// row came off.
///
/// Two scopes have rows of this kind — `presets`, which is a told directory
/// (ADR-0230), and `folder`, which is one somebody dropped on this window
/// (ADR-0275). They are one type because a row of either is a Set file that is
/// not in this store yet and a press on it is the same two operations
/// (`docs/manual/operations.html`'s *Send a Set to somebody, and take one in*):
/// the difference between them is which directory was listed, which is
/// [`Taking`]'s.
pub(crate) struct FileRow {
    /// What the row reads, which is the file's own name without its extension. Not
    /// read out of the file: a listing that opened twenty-three files to draw
    /// twenty-three rows would be a directory read doing a file read's work, and
    /// the id a take-in files the Set under is the one *inside* the file anyway —
    /// read there, on the press, by [`taking_in`].
    pub(crate) id: String,
    pub(crate) path: std::path::PathBuf,
}

/// Lists available Sets (`.kset` files) from the resolved preset library root.
pub(crate) fn presets_listing(
    presets: Option<&karakuri_environment::places::Presets>,
) -> Vec<FileRow> {
    let Some(presets) = presets else {
        return Vec::new();
    };
    match presets.list_sets() {
        Ok(sets) => sets
            .into_iter()
            .map(|set| FileRow {
                id: set.id,
                path: set.file,
            })
            .collect(),
        // Log preset discovery error and return empty listing.
        Err(why) => {
            println!("presets: {why}");
            Vec::new()
        }
    }
}

/// Every Set a dropped folder holds, which is the Set files directly in the
/// directory this bay was pointed at (ADR-0275).
///
/// # What it lists, and why both spellings
///
/// `console.html`'s *A folder scope reads Sets, and a bundle is not a third
/// thing*: *"A Set file and a bundle are the same file … so the scope draws one
/// kind of row rather than two"*, and the authored form that names its parts by
/// relative path *"is a Set file, is one row, and is taken in by the same
/// operation."* So both suffixes are listed and neither is a second kind of row
/// — `Store::SET_FILE_SUFFIX` for the resolved form and
/// `setfile::AUTHORING_SUFFIX` for the authored one, borrowed from the modules
/// that own them rather than spelled here. A `.kir` is not listed: it is one
/// node's source, nothing in the vocabulary takes one, and *"a directory of
/// `.kir` files is a directory of parts"*.
///
/// A directory that holds `night.kset` and `night.kbset` lists two rows reading
/// `night`, and that is deliberate rather than got to by accident: they are two
/// files, each of which is a Set, and choosing between them here would be this
/// listing inventing a precedence between the two forms. Whichever of them a
/// press means is the take-in's question and it is answered by refusing:
/// [`Taking::file`] finds the row by the word that was pressed, two files wear
/// that word, and a press that took one of them would be picking for the
/// operator between two rows they cannot tell apart on screen. See there, where
/// the refusal names both files.
///
/// # Ascending, one directory deep, and the name is all that is read
///
/// `places::Presets::list_sets`' three rules, carried over for its reasons:
/// `read_dir` hands back no order at all, a name the layout does not claim is
/// skipped rather than repaired, and nothing here opens a file — a malformed
/// Set is a refusal at the moment it is taken in, where the operator can see
/// which row they pressed. It is not [`library`]'s most-recent-first
/// (ADR-0263): that order is a store's, where a Set's time is when the operator
/// wrote it, and a folder full of files somebody copied has mtimes that are
/// facts about this machine's disk.
///
/// # Where this belongs, and it is not here
///
/// `karakuri_environment::places` is the right home, beside
/// `Presets::list_sets`, which answers the same question about a directory this
/// run was told about rather than one it was handed: this is that function with
/// two suffixes and no `Found` behind it, and a second walk of a directory of
/// Sets is a second answer to *what is a Set file called*. It is here because
/// ADR-0275's owed work was this file's, and the reason is written down rather
/// than left to be inferred — `declared`'s own shape one bay over.
pub(crate) fn folder_listing(dir: Option<&std::path::Path>) -> Vec<String> {
    folder_files(dir).into_iter().map(|row| row.id).collect()
}

/// The same walk with the file names kept, which is what a take-in needs: the
/// bay lists words and the press has to find the file the word came off again
/// ([`FileRow`]).
///
/// One walk and not two, which is why [`folder_listing`] is a `map` over this
/// rather than a second `read_dir`: a listing the bay drew and a listing the
/// press searched that disagreed would be a press acting on a row nobody saw.
/// It is [`presets_listing`]'s shape one scope along, and that function answers
/// `FileRow`s for the same reason.
pub(crate) fn folder_files(dir: Option<&std::path::Path>) -> Vec<FileRow> {
    let Some(dir) = dir else {
        return Vec::new();
    };
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        // Log folder read failure and return empty listing.
        Err(why) => {
            println!("  folder: `{}` could not be listed: {why}", dir.display());
            return Vec::new();
        }
    };
    let mut out: Vec<(String, std::ffi::OsString)> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        // Non-UTF-8 falls out of the listing with everything else the layout
        // does not claim — `Store::list_sets`' own rule, no lossy repair.
        let Some(id) = name.to_str().and_then(|name| {
            name.strip_suffix(Store::SET_FILE_SUFFIX)
                .or_else(|| name.strip_suffix(karakuri_environment::setfile::AUTHORING_SUFFIX))
        }) else {
            continue;
        };
        // A subdirectory named like a Set file belongs to whoever made it.
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        out.push((id.to_owned(), name));
    }
    out.sort();
    out.into_iter()
        .map(|(id, name)| FileRow {
            id,
            path: dir.join(name),
        })
        .collect()
}

/// Returns descriptive explanation when a given library scope contains no listing rows.
pub(crate) fn why_nothing(scope: Scope, pointed: bool, running: bool) -> &'static str {
    match scope {
        Scope::AllSets => {
            "this store holds no Sets yet, which is a library nobody has filled: `k` keeps \
             what a deck is playing, and loading a preset leaves one here too"
        }
        Scope::MySets => {
            "nothing in this store is starred — `my sets` is the starred subset of `all` \
             and never the listing of it, so press the star at the left of a row under \
             `all` and that Set appears here"
        }
        Scope::Presets => {
            "this run found no preset library, or the one it found holds no `.kset` file — \
             `--presets DIR` is what names one, and a directory of `.kir` parts is not a \
             library"
        }
        Scope::Folder if !pointed => {
            "no folder has been dropped on this window yet — drag one off the desktop and \
             let go of it anywhere over this window, and this scope lists the Sets in it"
        }
        Scope::Folder => {
            "the folder this bay is pointed at holds no Set file, which is a directory \
             nobody has put one in: a folder scope lists `.kbset` and `.kset` files, and a \
             directory of `.kir` parts is not a library"
        }
        Scope::History if !running => {
            "the deck the `load` pulldown names is playing a typed pair rather than a Set, so \
             its versions are filed under no Set at all and a listing narrowed to one matches \
             none of them — aim that pulldown at a deck you have loaded a Set onto, or load \
             one"
        }
        Scope::History => {
            "this Set has no versions yet — every write that compiles is kept, so edit one of \
             its nodes, or let a model write one, and the version it replaced is the first row \
             here"
        }
    }
}

/// What a folder over this window reads in the `.path` row, written into the
/// console for the pass that is about to draw it.
///
/// The one thing about this bay that is a frame's business, and it is `egui`'s
/// doing rather than a choice here: `RawInput::take` *clones* `hovered_files`
/// where it *moves* `dropped_files`, so a drag over the window is a fact about
/// every pass while it lasts and there is no event to hang it off. Nothing is
/// asked of the file system for it — whether the path is a folder is the drop's
/// question (P-0091, ADR-0275) — and nothing is allocated on a pass where the
/// answer has not changed.
///
/// More than one path over the window reads as none. The row says *"the path a
/// release would set"*, a release sets nothing where two arrived (ADR-0275),
/// and drawing the first of them would be this row picking one out of a list
/// the desktop happened to build — which is the choice the refusal below exists
/// to refuse.
pub(crate) fn folder_over(view: &mut View, hovering: &[karakuri_console::egui::HoveredFile]) {
    let over = match hovering {
        [one] => one.path.as_deref(),
        _ => None,
    };
    // **Compared before it is written**, so a drag held still over the window
    // costs nothing per pass. `to_string_lossy` borrows for a path that is
    // UTF-8, which every path drawn here is in practice, and it is the same
    // conversion `Path::display` makes — so what is compared is what would be
    // drawn rather than an approximation of it.
    let same = match (over, view.incoming.as_deref()) {
        (None, None) => true,
        (Some(over), Some(shown)) => *over.to_string_lossy() == *shown,
        _ => false,
    };
    if !same {
        view.incoming = over.map(|path| path.to_string_lossy().into_owned());
    }
}

/// A folder let go on this window, which is how the `folder` scope is given a
/// directory — and the three answers ADR-0275 settles, in the words that record
/// and `console.html` write.
///
/// # It is done here, on the pass the drop arrives on, and nothing is put by
///
/// `dropped_files` is visible for exactly one pass and then gone
/// (`RawInput::take` moves it), so the release does the whole thing rather than
/// asking a question: it sets the directory and marks the `folder` chip, and
/// the listing under it is the next thing drawn. A bay that had put the path
/// aside and waited for the chip to be pressed would be waiting on an operator
/// who has already made the gesture, holding a path nothing will hand it a
/// second time.
///
/// So the file system is asked here, on a frame, which is the one place this
/// program does that and it is P-0091's rule rather than an exception to it:
/// what is asked once is asked once, and a drop is one act. A drop is also the
/// only moment the question can be asked at all — the event carries a path and
/// nothing else, and *"a file and a directory are indistinguishable at the
/// event"* (ADR-0275).
///
/// # The two refusals, and each is a policy the plumbing does not answer
///
/// A path that is not a directory is refused, naming what was dropped. A file
/// is not read as the folder it sits in — that would point this bay at a
/// directory nobody pointed at, which is the mistake the carry one bay over
/// refuses when it declines to snap a drop mark to the nearest strip (ADR-0273)
/// — and a `.kbset` is not taken in where it fell, because taking a Set in is a
/// press on a row of a listing and a file landing on this window has no row
/// under it.
///
/// More than one path is refused, and all of them are. A multi-item drag
/// arrives whole, so three folders let go together are three entries in one
/// pass and not three drops: there is no first to act on and a rest to ignore,
/// nothing says which was aimed at, and the order is the desktop's rather than
/// the operator's. The bay keeps the directory it had and the refusal counts
/// what arrived.
///
/// Both name what arrived and what to do instead (P-0083), and both say where
/// this library is still pointed — because a refusal that left an operator
/// wondering whether the bay had moved anyway is a refusal that costs a second
/// gesture to read.
///
/// `None` where nothing was dropped, which is every pass but one.
pub(crate) fn folder_dropped(
    view: &mut View,
    folder: &mut Option<std::path::PathBuf>,
    store: &std::path::Path,
    presets: Option<&karakuri_environment::places::Presets>,
    dropped: &[&std::path::Path],
) -> Option<String> {
    // **Where the bay is still pointed**, read before anything moves, because
    // both refusals say it and the accept below replaces it.
    let kept = match folder.as_deref() {
        Some(at) => format!("This library is still pointed at `{}`.", at.display()),
        None => String::from("This library is still pointed nowhere."),
    };
    let one = match dropped {
        [] => return None,
        [one] => *one,
        many => {
            return Some(format!(
                "  folder: {} paths were let go together and none of them was taken — a drop \
                 tells this window a path and never a place, so nothing says which of them was \
                 aimed at and the order is the desktop's rather than yours. Let go of one \
                 folder on its own. {kept}",
                many.len()
            ))
        }
    };
    // **Asked once, here.** `is_dir` would answer `false` for a path that
    // cannot be examined at all, which is a different thing and is said as one.
    match std::fs::metadata(one) {
        Err(why) => Some(format!(
            "  folder: `{}` could not be examined ({why}), so whether it is a folder is not \
             known and nothing was taken. {kept}",
            one.display()
        )),
        Ok(what) if !what.is_dir() => Some(match set_file(one) {
            true => format!(
                "  folder: `{}` is a Set file and what this takes is a folder — a Set is taken \
                 in by pressing its row in a listing, and a file let go on this window has no \
                 row under it. Let go of the folder that holds it and press the row. {kept}",
                one.display()
            ),
            false => format!(
                "  folder: `{}` is not a folder and what this takes is one — this bay is \
                 pointed at a directory and lists the Sets in it, so let go of the folder that \
                 holds it rather than the file itself. {kept}",
                one.display()
            ),
        }),
        Ok(_) => {
            *folder = Some(one.to_path_buf());
            // **The line the bay draws is spelled here**, which is
            // `View::library`'s seam one row up: this side reads the disk and
            // the panel is handed what to draw (ADR-0156).
            view.folder = Some(one.display().to_string());
            // **And the chip is marked in the same act**, which is the whole
            // of *the drop is recorded on the frame it is seen on*: the
            // directory and the mark are one gesture's outcome, and a bay
            // pointed at a folder it is not showing would be waiting for a
            // press nobody owes it.
            view.select_scope(Scope::Folder);
            // **Asked of the mark rather than of the call**, because
            // `select_scope` answers *whether it moved* and a second drop
            // while `folder` is already marked moves nothing: the sentence is
            // about where the mark **is**. `false` here is a console handed no
            // `folder` chip, which cannot mark one — the row is still drawn,
            // since it is where a send's save dialog opens whichever scope is
            // marked (ADR-0311), and the listing under it is whatever scope this
            // console does have.
            let marked = match view.scope() == Some(Scope::Folder) {
                true => "the `folder` chip is marked",
                false => "this console draws no `folder` chip, so nothing is marked",
            };
            // **`None`, and it cannot be anything else here**: this drop has
            // just marked the `folder` chip, so the listing being re-asked is
            // a directory's and never a history's, and a Set id handed in
            // would be a value nothing reads.
            let said = listing(view, store, presets, folder.as_deref(), None);
            // **And the cursor goes back to the top of a listing it has never
            // seen**, which is `View::select_scope`'s own rule reached the
            // other way: that method resets the cursor when the *mark* moves,
            // and a second drop while `folder` is already marked moves the
            // listing without moving the mark. Left where it was it would
            // point at the fifteenth row of a directory of three — a Set
            // nobody chose, sitting under a pill that says a press will load
            // it.
            view.point_at(0);
            Some(format!(
                "  folder: this library is pointed at `{}` — {marked} and the listing under it \
                 is what that directory holds\n{said}",
                one.display(),
            ))
        }
    }
}

/// Whether a name is one a Set file wears, which is the two suffixes
/// [`folder_listing`] lists and is asked here for one reason: an operator who
/// let go of a `.kbset` on this window was trying to take a Set in, and the
/// refusal owes them the press that does it (P-0083).
///
/// It is a name and not a reading: nothing is opened, exactly as nothing is
/// opened to draw a row.
pub(crate) fn set_file(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.ends_with(Store::SET_FILE_SUFFIX)
                || name.ends_with(karakuri_environment::setfile::AUTHORING_SUFFIX)
        })
}
