use std::collections::BTreeMap;
use std::sync::Arc;

use karakuri_store::hash::Hash;
use karakuri_store::ndjson::Line;
use karakuri_store::record::{Layer, Record};
use karakuri_store::store::{Store, StoreError};

use crate::meta::{layer_named, put_meta};

use super::types::Node;

/// Determines the display name for a node, prioritizing explicit slot name, then
/// metadata card declaration, falling back to a 12-character short artifact hash.
pub fn node_called(written: Option<&str>, declared: Option<&str>, hash: &Hash) -> String {
    written
        .or(declared)
        .map_or_else(|| hash.short(12), str::to_string)
}

/// Reads the declared name from an artifact's metadata card in the store, if present.
pub fn card_name(store: &Store, hash: &Hash) -> Option<String> {
    store
        .read_meta(hash)
        .ok()?
        .iter()
        .find_map(|line| match line.record() {
            Record::Meta { name, .. } => Some(name.clone()),
            _ => None,
        })
}

/// One node of a Set, as a listing needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeSummary {
    pub layer: Layer,
    /// Which node of that layer, from 0 — the address `read_set` prints and the one
    /// the other MCP tools take.
    pub index: u32,
    /// What it is called: [`node_called`]'s answer, never a second reading of the
    /// same three candidates.
    pub name: String,
    /// The artifact behind it, whether or not this store holds one.
    pub hash: Hash,
}

/// Summary of a persisted Set: identifier, modification timestamp, node listings, and read errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetSummary {
    pub id: String,
    /// When the file was last written, from the filesystem — a Set file carries no
    /// time of its own. See `Store::list_sets`.
    pub written: std::time::SystemTime,
    pub nodes: Vec<NodeSummary>,
    /// Error description if this Set file failed to read or parse.
    pub unreadable: Option<String>,
}

/// Summarises every Set in the store without compiling or validating procedures.
///
/// Memoises metadata card reads across unnamed nodes and orders entries by ID.
pub fn summarise(store: &Store) -> Result<Vec<SetSummary>, StoreError> {
    let mut cards: BTreeMap<Hash, Option<String>> = BTreeMap::new();
    let mut out = Vec::new();
    for entry in store.list_sets()? {
        let mut nodes = Vec::new();
        let mut unreadable = None;
        match store.read_set(&entry.id) {
            Ok(lines) => {
                for line in &lines {
                    // Preserves file definition order without re-sorting by layer.
                    let Record::Slot {
                        at,
                        name,
                        proc_hash,
                    } = line.record()
                    else {
                        continue;
                    };
                    let declared = match name {
                        Some(_) => None,
                        None => cards
                            .entry(*proc_hash)
                            .or_insert_with(|| card_name(store, proc_hash))
                            .clone(),
                    };
                    nodes.push(NodeSummary {
                        layer: at.layer,
                        index: at.index,
                        name: node_called(name.as_deref(), declared.as_deref(), proc_hash),
                        hash: *proc_hash,
                    });
                }
            }
            Err(e) => unreadable = Some(e.to_string()),
        }
        out.push(SetSummary {
            id: entry.id,
            written: entry.written,
            nodes,
            unreadable,
        });
    }
    Ok(out)
}

/// Formats a filesystem timestamp in local time as `%Y-%m-%d %H:%M:%S`.
pub fn written_at(at: std::time::SystemTime) -> String {
    chrono::DateTime::<chrono::Local>::from(at)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

/// Snapshot of a running node prepared for persistence during a live save.
pub struct SavedNode {
    pub layer: &'static str,
    pub index: u32,
    pub hash: karakuri_store::hash::Hash,
    pub name: Option<String>,
    /// Compiled source text retained from launch, or `None` if already stored by the watcher.
    pub source: Option<Arc<str>>,
    /// Precompiled metadata card for launch sources, or `None` if already stored.
    pub meta: Option<Arc<[Line]>>,
}

/// Collection of active node sources and names staged for persisting a live Set.
pub struct Sources(pub Vec<SavedNode>);

impl Sources {
    /// Returns the number of saved nodes. Zero indicates no nodes.
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Converts saved nodes into persisted store artifacts and returns [`Node`] descriptors.
    pub fn into_nodes(self, store: &Store) -> Result<Vec<Node>, String> {
        self.0
            .into_iter()
            .map(|node| {
                let SavedNode {
                    layer,
                    index,
                    hash,
                    name,
                    source,
                    meta,
                } = node;
                let layer = layer_named(layer)
                    .ok_or_else(|| format!("a node on layer `{layer}` cannot be saved"))?;
                if let Some(source) = source {
                    store
                        .put_artifact(source.as_bytes())
                        .map_err(|e| format!("the source of a `{layer:?}` node: {e}"))?;
                    // Writes accompanying metadata card if present.
                    if let Some(meta) = meta {
                        put_meta(store, &hash, &meta);
                    }
                }
                Ok(Node {
                    hash,
                    layer,
                    index,
                    name,
                })
            })
            .collect()
    }
}
