//! CLI launch configuration and startup source resolution.

use crate::deck_letter;
use karakuri_environment::history;

#[derive(Debug, Clone)]
pub(crate) struct Sources {
    pub(crate) l1: std::path::PathBuf,
    pub(crate) l4: std::path::PathBuf,
}

impl Sources {
    /// Returns default launch sources (`coil_vortex.kir` + `star_flares.kir`) located in the specified preset directory.
    pub(crate) fn under(presets: &std::path::Path) -> Sources {
        Sources {
            l1: presets.join("coil_vortex.kir"),
            l4: presets.join("star_flares.kir"),
        }
    }

    /// Returns the human-readable material label string for the pair (e.g. `l1 + l4`).
    pub(crate) fn material(&self) -> String {
        let stem = |path: &std::path::Path| {
            path.file_stem()
                .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned())
        };
        format!("{} + {}", stem(&self.l1), stem(&self.l4))
    }
}

/// Materialises per-slot working copies in scratch storage before starting the application.
///
/// This isolates runtime edits to scratch storage and prevents modifications to original preset files (P-0096).
///
/// # Arguments
/// * `store` - Root directory of the artifact store.
/// * `sources` - Initial geometry (L1) and renderer (L4) sources.
/// * `slots` - Number of deck slots to initialize.
///
/// # Errors
/// Returns an error message if copying fails or files cannot be created.
pub(crate) fn working_copies(
    store: &std::path::Path,
    sources: &Sources,
    slots: usize,
) -> Result<(std::path::PathBuf, Vec<Sources>), String> {
    let mut copies: Vec<Sources> = std::iter::repeat_n(sources.clone(), slots).collect();
    let dir = karakuri_environment::scratch::materialise(
        store,
        copies.iter_mut().map(|pair| [&mut pair.l1, &mut pair.l4]),
    )?;
    Ok((dir, copies))
}

/// Formats a startup diagnostic summary displaying the scratch directory and per-deck source files.
pub(crate) fn running_from(dir: &std::path::Path, copies: &[Sources]) -> String {
    let mut said = format!("scratch: {} (source paths not written to)", dir.display());
    for (slot, pair) in copies.iter().enumerate() {
        let name = |path: &std::path::Path| {
            path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string())
        };
        said.push_str(&format!(
            "\n  deck {}: {} + {}",
            deck_letter(slot as u8),
            name(&pair.l1),
            name(&pair.l4)
        ));
    }
    said
}

/// Seeds initial snapshot history in the store for each running deck before the window opens (ADR-0089).
///
/// Sharing the snapshot instance avoids duplicate entries on initial rebuilds.
pub(crate) fn seeded(store: &std::path::Path, copies: &[Sources]) -> history::Shared {
    let shared = history::Snapshots::shared(store);
    history::seed(
        &shared,
        copies
            .iter()
            .enumerate()
            .map(|(slot, pair)| (slot, None, vec![pair.l1.as_path(), pair.l4.as_path()])),
    );
    shared
}

/// CLI usage help text displayed for `-h`, `--help`, or syntax errors.
pub(crate) const USAGE: &str = "\
usage: karakuri [--presets DIR] [--plugins DIR] [--store DIR] [--mcp PORT] [GEOMETRY.kir RENDERER.kir]

  The console, with a deck behind it. Both paths or neither: a Set is an L1 and
  an L4, and with neither the pair that ships in the preset library is played.

  --presets DIR   the shipped preset library. Given, it is used and a directory
                  that is not there is refused. Not given, it is looked for
                  beside this binary — an .app bundle's Resources, a prefix
                  install's share/karakuri, a portable examples/ — and last in
                  the workspace this binary was compiled in. Which one answered
                  is printed at startup. With none, there is no default pair
                  and the two paths have to be given.
  --plugins DIR   where external output plugins (Spout, Syphon, etc.) live.
                  Given, it is used and a directory that is not there is refused.
                  Not given, it is looked for beside this binary, in the install
                  prefix, or in the workspace root.
  --store DIR     where the Library bay reads Sets and arrangements from, where
                  a save goes, and where the scratch each deck runs from is
                  written. Defaults to .karakuri beside the session.
  --mcp PORT      serve the Model Context Protocol on 127.0.0.1:PORT, so a
                  model can read a deck's procedure, rewrite it, rewire an
                  input and keep what a deck is playing. Loopback only, and
                  0 takes an ephemeral port and prints the one it got.

  Any flag may be given before or after the pair.

  This is not `karakuri-cli`'s command line and does not try to be — that one
  has the audio, the MIDI and the rest of the flags, and its parser is its own.
  It reads the same two directories, and `--store` and `--mcp` mean the same
  thing to both. See `cargo run -p karakuri-cli -- --help`.";

/// Parsed command-line configuration for starting the console.
#[derive(Debug)]
pub(crate) struct Launch {
    pub(crate) sources: Sources,
    /// Root directory of the artifact store (`--store` or default `.karakuri`).
    pub(crate) store: std::path::PathBuf,
    /// Discovered preset library root directory, if available.
    pub(crate) presets: Option<karakuri_environment::places::Presets>,
    /// Discovered plugin directory root, if available.
    pub(crate) plugins: Option<karakuri_environment::places::Plugins>,
    /// Optional MCP server port specified via `--mcp PORT`.
    pub(crate) mcp: Option<u16>,
}

/// Parses command-line arguments into a [`Launch`] configuration.
///
/// Accepts up to two KIR file paths (geometry L1 and renderer L4) or falls back to shipped presets.
/// Also parses `--presets`, `--plugins`, `--store`, and `--mcp` options in any argument order.
pub(crate) fn sources_from<I: IntoIterator<Item = String>>(args: I) -> Result<Launch, String> {
    let args: Vec<String> = args.into_iter().collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        return Err(String::new());
    }
    // Parse flags and positional file arguments regardless of order.
    let mut named_presets: Option<std::path::PathBuf> = None;
    let mut named_plugins: Option<std::path::PathBuf> = None;
    let mut store: Option<std::path::PathBuf> = None;
    let mut mcp: Option<u16> = None;
    let mut paths: Vec<String> = Vec::new();
    let mut rest = args.into_iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--presets" => {
                named_presets = Some(std::path::PathBuf::from(value_for("--presets", &mut rest)?))
            }
            "--plugins" => {
                named_plugins = Some(std::path::PathBuf::from(value_for("--plugins", &mut rest)?))
            }
            "--store" => store = Some(std::path::PathBuf::from(value_for("--store", &mut rest)?)),
            "--mcp" => mcp = Some(number_for("--mcp", "a port number", &mut rest)?),
            other if other.starts_with('-') => return Err(format!("unknown option `{other}`")),
            _ => paths.push(arg),
        }
    }

    let presets = karakuri_environment::places::presets(named_presets.as_deref())?;
    let plugins = karakuri_environment::places::plugins(named_plugins.as_deref())?;
    let sources = match paths.as_slice() {
        [] => match &presets {
            Some(presets) => Sources::under(&presets.dir),
            None => return Err(karakuri_environment::places::no_launch_pair()),
        },
        [l1, l4] => Sources {
            l1: std::path::PathBuf::from(l1),
            l4: std::path::PathBuf::from(l4),
        },
        [one] => {
            return Err(format!(
                "one path given (`{one}`) and a Set needs two: a geometry and a renderer"
            ))
        }
        many => {
            return Err(format!(
                "{} paths given and a Set is built from two: a geometry and a renderer",
                many.len()
            ))
        }
    };
    Ok(Launch {
        sources,
        store: store
            .unwrap_or_else(|| std::path::PathBuf::from(karakuri_environment::places::STORE)),
        presets,
        plugins,
        mcp,
    })
}

/// Extracts the next argument as the value for `flag`, erroring if missing or starting with `-`.
pub(crate) fn value_for(
    flag: &str,
    rest: &mut impl Iterator<Item = String>,
) -> Result<String, String> {
    match rest.next() {
        Some(value) if !value.starts_with('-') => Ok(value),
        Some(value) => Err(format!(
            "`{flag}` was given no value — `{value}` is an option, not one"
        )),
        None => Err(format!("`{flag}` needs a value")),
    }
}

/// Parses the argument following `flag` into numeric type `T`, erroring if missing, malformed, or another flag.
pub(crate) fn number_for<T: std::str::FromStr>(
    flag: &str,
    what: &str,
    rest: &mut impl Iterator<Item = String>,
) -> Result<T, String> {
    let value = value_for(flag, rest)?;
    value
        .parse()
        .map_err(|_| format!("`{flag} {value}` — expected {what}"))
}
