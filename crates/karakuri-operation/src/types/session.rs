//! Controls, deck properties, Set transfers, and session recordings.

use super::address::NodeAddress;
use std::path::PathBuf;

/// One control on a deck's published interface.
///
/// The range narrows the declared parameter range without redefining it.
/// If `node` is `None`, it functions as a wildcard match.
#[derive(Debug, Clone, PartialEq)]
pub struct Control {
    /// What the console shows.
    pub name: String,
    pub node: Option<NodeAddress>,
    pub key: String,
    pub range: [f32; 2],
}

/// Settable deck properties: element capacity or hash builtin seed salt.
///
/// See ADR-0318 and ADR-0328.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Property {
    /// Element count for deck geometries, overriding default declared capacity (`--capacity`).
    Capacity { elements: u32 },
    /// Seed salt for hash builtins in shader nodes, modifying deterministic pseudorandomness.
    Seed { salt: u32 },
}

/// Sending a Set and taking one in — two operations under one heading, and the
/// two are not each other's inverse: one names something the store already
/// holds, the other hands the store something it has never seen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetTransfer {
    /// Write the Set filed under this id with every source it names inlined.
    /// `--package ID` — or `--package FILE.kset`, which resolves an authoring
    /// file's parts into the store first and packages that.
    Send { id: String },
    /// Read a Set file, store its sources, and write its Set file.
    /// The Set ID is extracted from the file; duplicate IDs are rejected.
    Take { file: PathBuf },
}

/// Session recording control state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recording {
    /// Begin session recording under the given ID or timestamp.
    ///
    /// Live deck state is captured as a Set head for the session.
    /// Each recording session requires a unique identifier (P-0092).
    Start { id: Option<String> },
    /// End the active recording session.
    Stop,
}
