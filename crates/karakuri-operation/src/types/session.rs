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
    /// How many elements each of a deck's geometries runs at, overriding what their
    /// own `capacity` declarations name. `--capacity`.
    ///
    /// A number in a range the material declares, and the refusal is the engine's:
    /// `Set::build` rejects a capacity outside the declared range and names the
    /// range in the sentence, so a surface that offers a value is offering rather
    /// than deciding
    /// ([P-0090](../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    Capacity { elements: u32 },
    /// The salt a deck's hash builtins are seeded from, so re-seeding changes
    /// randomness without touching anything structural. There is no flag for this.
    ///
    /// `u32` and not `u64`, which is the width the engine has always used:
    /// `watch::Aim::seed_salt`, `Set::source_salts` and
    /// `karakuri_engine::set::derived_salt` are all `u32`, and a payload twice as
    /// wide as the field it lands in is a number that can be asked for and cannot
    /// arrive.
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

/// Starting and stopping a session recording.
///
/// A sum rather than `{ recording: bool, id: Option<String> }`, because that
/// shape has a field that means nothing in one of its two states, and a payload
/// nobody reads is a free variable
/// (`docs/principles/0087-name-the-property-never-the-shape.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recording {
    /// Begin session recording under the given ID or timestamp.
    ///
    /// Live deck state is captured as a Set head for the session.
    /// Each recording session requires a unique identifier (P-0092).
    Start { id: Option<String> },
    /// End the one running. Nothing refuses it, and `crates/karakuri`'s `rec` pill
    /// is the other end of the same press that starts one (ADR-0289).
    ///
    /// Neither end happens on the frame. `Recorder::finish` blocks on its writer,
    /// and so does dropping one, so a stop hands the recorder to a thread and the
    /// outcome is said when it lands —
    /// [P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md).
    Stop,
}
