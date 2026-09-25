//! Resolution and listing for preset library and store directories.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::setfile::AUTHORING_SUFFIX;

/// Default store directory relative to working tree.
pub const STORE: &str = ".karakuri";

/// Subdirectory name expected for preset library directories.
const LIBRARY: &str = "examples";

/// Compile-time workspace root fallback.
const WORKSPACE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

/// Source location where a preset library was discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Found {
    /// `--presets DIR`. Not searched for, not fallen back from.
    Given,
    /// `<exe dir>/../Resources/examples` — a macOS `.app` bundle.
    Bundle,
    /// `<exe dir>/../share/karakuri/examples` — a unix prefix install.
    Prefix,
    /// `<exe dir>/examples` — a portable directory, unpacked anywhere.
    Beside,
    /// The workspace tree this binary was compiled in — the development entry.
    Workspace,
}

impl Found {
    /// Returns human-readable description of how the preset root was resolved.
    pub fn how(self) -> &'static str {
        match self {
            Found::Given => "named with `--presets`",
            Found::Bundle => "found in this app bundle, at `../Resources/examples`",
            Found::Prefix => "found in a unix prefix install, at `../share/karakuri/examples`",
            Found::Beside => "found beside the binary, at `examples`",
            Found::Workspace => "found in the workspace this binary was compiled in",
        }
    }

    /// How the plugin directory was arrived at, in the words a startup line prints.
    pub fn how_plugin(self) -> &'static str {
        match self {
            Found::Given => "named with `--plugins` or `KARAKURI_PLUGINS_DIR`",
            Found::Bundle => "found in this app bundle, at `../PlugIns`",
            Found::Prefix => "found in a unix prefix install, at `../lib/karakuri/plugins`",
            Found::Beside => "found beside the binary, at `plugins`",
            Found::Workspace => "found in the workspace this binary was compiled in",
        }
    }
}

/// A preset library: the directory, and which of [`Found`]'s places it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Presets {
    pub dir: PathBuf,
    pub found: Found,
}

/// Resolves preset library root from optional explicit path or default search locations.
pub fn presets(given: Option<&Path>) -> Result<Option<Presets>, String> {
    if let Some(dir) = given {
        return match dir.exists() {
            true => Ok(Some(Presets {
                dir: dir.to_path_buf(),
                found: Found::Given,
            })),
            false => Err(no_presets_at(dir)),
        };
    }
    let exe = std::env::current_exe().ok();
    let exe_dir = exe.as_deref().and_then(Path::parent);
    Ok(searched(exe_dir, Path::new(WORKSPACE)))
}

/// The search of [`presets`], over roots rather than over this process — which
/// is what makes every one of its four turns reachable from a test on a
/// temporary directory rather than from four real installs.
fn searched(exe_dir: Option<&Path>, workspace: &Path) -> Option<Presets> {
    let mut places: Vec<(PathBuf, Found)> = Vec::new();
    if let Some(exe_dir) = exe_dir {
        places.push((
            exe_dir.join("..").join("Resources").join(LIBRARY),
            Found::Bundle,
        ));
        places.push((
            exe_dir
                .join("..")
                .join("share")
                .join("karakuri")
                .join(LIBRARY),
            Found::Prefix,
        ));
        places.push((exe_dir.join(LIBRARY), Found::Beside));
    }
    places.push((workspace.join(LIBRARY), Found::Workspace));
    places
        .into_iter()
        .find(|(dir, _)| is_a_library(dir))
        .map(|(dir, found)| Presets { dir, found })
}

/// Returns true if `dir` contains at least one non-directory `.kset` file.
fn is_a_library(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        set_id(&entry.file_name()).is_some() && entry.file_type().is_ok_and(|kind| !kind.is_dir())
    })
}

/// Returns the Set ID by stripping `AUTHORING_SUFFIX` (`.kset`) from `name`.
fn set_id(name: &OsStr) -> Option<&str> {
    name.to_str()?.strip_suffix(AUTHORING_SUFFIX)
}

/// Returns procedure name by stripping `.kir` suffix from `name`.
fn procedure_name(name: &OsStr) -> Option<&str> {
    name.to_str()?
        .strip_suffix(karakuri_store::Store::PROCEDURE_FILE_SUFFIX)
}

/// A Set in a presets root found by [`Presets::list_sets`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetSet {
    /// Stem identifier of the Set.
    pub id: String,
    /// Absolute path to the resolved `.kset` file.
    pub file: PathBuf,
}

/// A procedure in a presets root found by [`Presets::list_procedures`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetProcedure {
    /// Stem name of the procedure.
    pub name: String,
    /// Absolute path to the `.kir` file.
    pub file: PathBuf,
    /// Declared procedure layer kind, if present.
    pub kind: Option<&'static str>,
}

impl Presets {
    /// Lists all `.kset` files directly within this preset root, ordered ascending by id.
    pub fn list_sets(&self) -> Result<Vec<PresetSet>, String> {
        let entries = std::fs::read_dir(&self.dir).map_err(|e| self.cannot_be_listed(&e))?;
        let mut out = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| self.cannot_be_listed(&e))?;
            let name = entry.file_name();
            let Some(id) = set_id(&name) else {
                continue;
            };
            if entry
                .file_type()
                .map_err(|e| self.cannot_be_listed(&e))?
                .is_dir()
            {
                continue;
            }
            out.push(PresetSet {
                id: id.to_string(),
                file: entry.path(),
            });
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    /// Lists all `.kir` procedures directly within this preset root, ordered ascending by name.
    pub fn list_procedures(&self) -> Result<Vec<PresetProcedure>, String> {
        let entries = std::fs::read_dir(&self.dir).map_err(|e| self.cannot_be_listed(&e))?;
        let mut out = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| self.cannot_be_listed(&e))?;
            let name = entry.file_name();
            let Some(name) = procedure_name(&name) else {
                continue;
            };
            if entry
                .file_type()
                .map_err(|e| self.cannot_be_listed(&e))?
                .is_dir()
            {
                continue;
            }
            let path = entry.path();
            let Ok(source) = std::fs::read(&path) else {
                continue;
            };
            out.push(PresetProcedure {
                name: name.to_string(),
                file: path,
                kind: crate::history::declared_kind(&source),
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    /// Formats an error message indicating that reading the preset library directory failed.
    fn cannot_be_listed(&self, why: &std::io::Error) -> String {
        format!(
            "cannot list the preset library at `{}` ({}): {why}",
            self.dir.display(),
            self.found.how()
        )
    }
}

/// Formats refusal message when an explicit `--presets` directory does not exist.
pub fn no_presets_at(dir: &Path) -> String {
    format!(
        "no presets at `{}` — nothing is there, and nothing was created to find that out. \
         Check `--presets`, or leave it off and this program looks for the library it was \
         installed with.",
        dir.display()
    )
}

/// Returns the message displayed when no preset library directory is found.
pub fn no_preset_library() -> String {
    String::from(
        "no preset library: none of the places this program looks holds one, so nothing \
         ships with it on this machine and nothing was created to say so. Name one with \
         `--presets DIR`.",
    )
}

/// Formats error message when neither preset library nor positional source paths are available.
pub fn no_launch_pair() -> String {
    String::from(
        "nothing to play: there is no preset library, so there is no default pair. Name a \
         library with `--presets DIR`, or give the two paths — a geometry and a renderer — \
         on the command line.",
    )
}

/// The leaf directory an output plugin library is called, in every candidate below.
const PLUGINS_LEAF: &str = "plugins";

/// An output plugin directory: the directory, and which of [`Found`]'s places it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plugins {
    pub dir: PathBuf,
    pub found: Found,
}

impl Plugins {
    /// Lists all executable plugin binaries in this directory, sorted deterministically.
    pub fn list_executables(&self) -> Result<Vec<PathBuf>, String> {
        let entries = std::fs::read_dir(&self.dir).map_err(|e| self.cannot_be_listed(&e))?;
        let mut out = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| self.cannot_be_listed(&e))?;
            let path = entry.path();
            if path.is_file() {
                #[cfg(target_os = "windows")]
                let is_exec = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map_or(false, |ext| ext.eq_ignore_ascii_case("exe"));
                #[cfg(not(target_os = "windows"))]
                let is_exec = {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        std::fs::metadata(&path)
                            .map(|m| m.permissions().mode() & 0o111 != 0)
                            .unwrap_or(false)
                    }
                    #[cfg(not(unix))]
                    {
                        true
                    }
                };
                if is_exec {
                    out.push(path);
                }
            }
        }
        out.sort();
        Ok(out)
    }

    fn cannot_be_listed(&self, why: &std::io::Error) -> String {
        format!(
            "cannot list the plugin directory at `{}` ({}): {why}",
            self.dir.display(),
            self.found.how_plugin()
        )
    }
}

/// Resolves the plugin directory for this run: what was given, `KARAKURI_PLUGINS_DIR`,
/// or the first of the four places that has a plugins directory.
pub fn plugins(given: Option<&Path>) -> Result<Option<Plugins>, String> {
    if let Some(dir) = given {
        return match dir.is_dir() {
            true => Ok(Some(Plugins {
                dir: dir.to_path_buf(),
                found: Found::Given,
            })),
            false => Err(no_plugins_at(dir)),
        };
    }
    if let Ok(env_dir) = std::env::var("KARAKURI_PLUGINS_DIR") {
        let path = PathBuf::from(env_dir);
        return match path.is_dir() {
            true => Ok(Some(Plugins {
                dir: path,
                found: Found::Given,
            })),
            false => Err(no_plugins_at(&path)),
        };
    }
    let exe = std::env::current_exe().ok();
    let exe_dir = exe.as_deref().and_then(Path::parent);
    Ok(searched_plugins(exe_dir, Path::new(WORKSPACE)))
}

fn searched_plugins(exe_dir: Option<&Path>, workspace: &Path) -> Option<Plugins> {
    let mut places: Vec<(PathBuf, Found)> = Vec::new();
    if let Some(exe_dir) = exe_dir {
        places.push((exe_dir.join("..").join("PlugIns"), Found::Bundle));
        places.push((
            exe_dir
                .join("..")
                .join("lib")
                .join("karakuri")
                .join(PLUGINS_LEAF),
            Found::Prefix,
        ));
        places.push((
            exe_dir
                .join("..")
                .join("share")
                .join("karakuri")
                .join(PLUGINS_LEAF),
            Found::Prefix,
        ));
        places.push((exe_dir.join(PLUGINS_LEAF), Found::Beside));
    }
    places.push((workspace.join(PLUGINS_LEAF), Found::Workspace));
    places
        .into_iter()
        .find(|(dir, _)| is_a_plugin_directory(dir))
        .map(|(dir, found)| Plugins { dir, found })
}

fn is_a_plugin_directory(dir: &Path) -> bool {
    dir.is_dir()
}

/// A plugin directory that is not there.
pub fn no_plugins_at(dir: &Path) -> String {
    format!(
        "no plugin directory at `{}` — nothing is there. Check `--plugins` or `KARAKURI_PLUGINS_DIR`.",
        dir.display()
    )
}

#[cfg(test)]
mod tests;
