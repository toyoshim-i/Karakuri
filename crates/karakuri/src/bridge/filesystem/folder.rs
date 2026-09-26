use super::*;

/// A file-backed library row storing its display identifier and filesystem path (ADR-0230, ADR-0275).
pub(crate) struct FileRow {
    /// Display name of the Set row, derived from the filename without extension.
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

/// Lists available Sets (`.kset` and `.kbset`) directly within a dropped folder directory (ADR-0275).
pub(crate) fn folder_listing(dir: Option<&std::path::Path>) -> Vec<String> {
    folder_files(dir).into_iter().map(|row| row.id).collect()
}

/// Scans a directory for Set files, returning [`FileRow`] entries with IDs and paths (ADR-0275).
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

/// Updates view state with the path of a folder currently being dragged over the window (ADR-0275, P-0091).
pub(crate) fn folder_over(view: &mut View, hovering: &[karakuri_console::egui::HoveredFile]) {
    let over = match hovering {
        [one] => one.path.as_deref(),
        _ => None,
    };
    // Skip updating view state if the hovered path has not changed.
    let same = match (over, view.incoming.as_deref()) {
        (None, None) => true,
        (Some(over), Some(shown)) => *over.to_string_lossy() == *shown,
        _ => false,
    };
    if !same {
        view.incoming = over.map(|path| path.to_string_lossy().into_owned());
    }
}

/// Handles a dropped folder to set the library folder scope and refresh listing (ADR-0275, P-0083, P-0091).
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
            // Mark the folder scope chip in the view.
            view.select_scope(Scope::Folder);
            // Check whether the folder scope was successfully marked in view (ADR-0311).
            let marked = match view.scope() == Some(Scope::Folder) {
                true => "the `folder` chip is marked",
                false => "this console draws no `folder` chip, so nothing is marked",
            };
            // **`None`, and it cannot be anything else here**: this drop has
            // just marked the `folder` chip, so the listing being re-asked is
            // a directory's and never a history's, and a Set id handed in
            // would be a value nothing reads.
            let said = listing(view, store, presets, folder.as_deref(), None);
            // Reset cursor to top of listing for newly loaded folder.
            view.point_at(0);
            Some(format!(
                "  folder: this library is pointed at `{}` — {marked} and the listing under it \
                 is what that directory holds\n{said}",
                one.display(),
            ))
        }
    }
}

/// Checks if a path ends with a supported Set file suffix (`.kset` or `.kbset`; P-0083).
pub(crate) fn set_file(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.ends_with(Store::SET_FILE_SUFFIX)
                || name.ends_with(karakuri_environment::setfile::AUTHORING_SUFFIX)
        })
}
