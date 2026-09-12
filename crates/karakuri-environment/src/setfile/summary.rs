use std::collections::BTreeMap;
use std::sync::Arc;

use karakuri_store::hash::Hash;
use karakuri_store::ndjson::Line;
use karakuri_store::record::{Layer, Record};
use karakuri_store::store::{Store, StoreError};

use crate::meta::{layer_named, put_meta};

use super::types::Node;

/// **What one node of a Set is called** — the one answer, for every surface
/// that has to name one.
///
/// Three candidates in a fixed order: the name this Set file's own `slot`
/// record carries, then the name the artifact's metadata card declares, then
/// the artifact's short hash.
///
/// **The Set's own name wins over the card's, because it is the more specific
/// fact.** A card says what a procedure calls *itself* and says the same thing
/// in every Set that references the artifact; a `slot` name is what *this* Set
/// decided to call *this* node, is what an `edge` in the same file points at,
/// and is what an operator typed in `--set near=lattice.kir`. A listing that
/// preferred the card would give two nodes of one Set the same name wherever a
/// chain instantiates one procedure twice.
///
/// **A node with neither still has a name.** An artifact with no card is an
/// ordinary state of a working store rather than a damaged one — `mcp`'s
/// `node_block` says so at length — and so is a Set naming an artifact this
/// store never had. Neither is a reason to leave a node out of a listing or to
/// print a blank where its name goes, so the short hash is the last resort: it
/// tells two nodes apart, it recognises the same artifact in two Sets, and it
/// is the same shortening every log line in this program uses.
///
/// **One function, called by both surfaces and by `read_set`.** Two derivations
/// that agree today are two answers that stop agreeing the day one of them is
/// edited, and the name a model reads in a listing has to be the name it then
/// finds when it reads that Set.
pub fn node_called(written: Option<&str>, declared: Option<&str>, hash: &Hash) -> String {
    written
        .or(declared)
        .map_or_else(|| hash.short(12), str::to_string)
}

/// What an artifact's metadata card calls it, or `None` where this store has no
/// card for it.
///
/// `None` covers both absences on purpose — no card written yet, and no
/// artifact here at all — because the answer to *what is this node called* is
/// the same for both: whatever the Set file says, and the short hash otherwise.
/// The two are worth telling apart when a model is choosing material, and
/// `read_set` is where that is done.
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
    /// Which node of that layer, from 0 — the address `read_set` prints and
    /// the one the other MCP tools take.
    pub index: u32,
    /// What it is called: [`node_called`]'s answer, never a second reading of
    /// the same three candidates.
    pub name: String,
    /// The artifact behind it, whether or not this store holds one.
    pub hash: Hash,
}

/// **What one saved Set holds**, read off the store: the id that names it, when
/// its file was written, and a line per node.
///
/// **One value, rendered twice.** The MCP `list_sets` tool and `--list-sets`
/// answer one question for two readers, and a library summarised once per
/// reader is two answers that agree until somebody edits one of them. Both
/// render this; neither opens a Set file of its own.
///
/// **What it deliberately does not carry**: what any of those nodes declares,
/// what this Set has turned anything to, and whether it builds. Those need a
/// card per artifact, the rest of the file, and a compile pass respectively —
/// `read_set` does all three for *one* Set on purpose, and a listing that did
/// them for a library would compile a thousand procedures to print a thousand
/// lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetSummary {
    pub id: String,
    /// When the file was last written, from the filesystem — a Set file carries
    /// no time of its own. See `Store::list_sets`.
    pub written: std::time::SystemTime,
    pub nodes: Vec<NodeSummary>,
    /// Why this Set's file could not be read, where it could not be.
    ///
    /// **Listed anyway, and said out loud.** A file in `sets/` that will not
    /// parse is something an operator has to be told about; dropping it from
    /// the listing would answer *what have I kept* with a Set missing, and
    /// reporting it with an empty node list would say it holds nothing.
    pub unreadable: Option<String>,
}

/// **Summarise every Set the store holds.**
///
/// One directory read, one read per Set file, and a card read per node the file
/// left unnamed — nothing else. Nothing is compiled, no adapter is opened and
/// `karakuri_engine::Set::validate` is never called: `load` and `read_set` do
/// that for one named Set on purpose, and *what is in my library* is not the
/// question that should pay for it.
///
/// **The card reads are memoised across the whole listing**, keyed by the
/// artifact rather than by the node: a library where fifty Sets reference one
/// unnamed geometry reads that card once. A named node reads no card at all,
/// because [`node_called`] would not have used it.
///
/// The order is `Store::list_sets`'s — ascending by id, which is total and
/// repeatable. A surface that wants recency sorts on [`SetSummary::written`]
/// and says why.
pub fn summarise(store: &Store) -> Result<Vec<SetSummary>, StoreError> {
    let mut cards: BTreeMap<Hash, Option<String>> = BTreeMap::new();
    let mut out = Vec::new();
    for entry in store.list_sets()? {
        let mut nodes = Vec::new();
        let mut unreadable = None;
        match store.read_set(&entry.id) {
            Ok(lines) => {
                for line in &lines {
                    // **The file's own order**, which is the order [`save`](crate::setfile::save)
                    // wrote the nodes in and the order a hand-written file
                    // chose — the same reading `read_set` gives, for the same
                    // reason: sorting by layer here would impose an order
                    // nobody wrote.
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

/// When a Set was written, spelled the one way every listing spells it.
///
/// **Local, for the reason [`crate::history::stamped_id`] is local**: the
/// answer has to be the one the person would say out loud, and a UTC clock is
/// the wrong one for half the world and half the day. To the second, because
/// that is as fine as a filesystem mtime is worth reading and finer than
/// anybody scanning a column wants.
pub fn written_at(at: std::time::SystemTime) -> String {
    chrono::DateTime::<chrono::Local>::from(at)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

/// One node of a live save: its layer as a record spells it, which node of that
/// layer, the address its source has, the name the operator gave the file, and
/// — for a node still at the version the run launched with — the bytes to put
/// in the store on the way past.
///
/// **The name is the part no hash could carry**, which is why this is not
/// the surface's `Nodes`: a name belongs to the *use* rather than to the procedure, so it
/// comes from the command line and travels beside the address rather than
/// inside it.
pub struct SavedNode {
    pub layer: &'static str,
    pub index: u32,
    pub hash: karakuri_store::hash::Hash,
    pub name: Option<String>,
    /// The launch source, when this node is still running it, and `None` when a
    /// build put the version there instead.
    ///
    /// **Which is the whole of what "lazily" means.** The watcher puts what it
    /// builds in the store as it builds it, so a rebuilt node's bytes are
    /// already there and there is nothing to carry. A node still on its launch
    /// version has bytes that live only in this process — see [`crate::compile::Placed`] — so
    /// they ride along and reach the store at the moment a file names them.
    /// That is why a windowed run creates nothing until somebody presses `k`.
    ///
    /// These are the *compiled* bytes and never a re-read of the path, so a
    /// `.kir` rewritten since launch cannot reach a saved file.
    pub source: Option<Arc<str>>,
    /// The card that goes down with those bytes, carried on exactly the same
    /// condition and for the same reason — see [`crate::compile::Placed::meta`]. `None`
    /// wherever `source` is `None`: a rebuilt node's card was written by the
    /// build that stored it, and there is nothing here to add.
    ///
    /// **That is held by construction rather than asserted.** The surface's
    /// `live_sources`
    /// asks the one predicate once and takes the pair off it, because it used
    /// to ask it twice — two derivations of "are these the bytes on screen"
    /// with nothing keeping them equal, where [`Sources::into_nodes`] writes the
    /// card *inside* the branch that puts the source and would have dropped a
    /// card whose `source` had gone `None` without a word.
    ///
    /// A field beside `source` rather than derived from it, because deriving it
    /// would mean re-compiling the source on the save thread — see
    /// [`crate::compile::Placed::meta`], which declined the same thing on the same grounds.
    pub meta: Option<Arc<[Line]>>,
}

/// **Where one live save's sources come from**: the hashes of the versions this
/// slot is running, with the name the operator gave each file beside them.
///
/// **One answer, not two.** This used to be an enum — the landed hashes where a
/// build had landed, and the startup *paths* where none had — and the second arm
/// was a save that read the disk. Reading the disk answers a different question:
/// a slot rolled back to what it launched with, an edit that never compiled, and
/// a run with no watcher at all are all states where the file and the picture
/// disagree, and every one of them wrote down a version nobody had seen. The
/// hashes are seeded at launch instead — see the surface's `Running::at_launch` —
/// so there
/// is one representation of "what bytes is this node running", derived once
/// from the text the compile read and a hash from the first frame onward.
///
/// See `Live::save_set`. Owned, because it crosses onto the thread that does the
/// store I/O.
pub struct Sources(pub Vec<SavedNode>);

impl Sources {
    /// How many nodes this names. Zero is a slot with nothing behind it — see
    /// the surface's `Live::save_set`, which refuses rather than writing a file
    /// describing no Set.
    ///
    /// **`pub(crate)` where [`Sources::is_empty`] is `pub`**, which is the
    /// widening rule and not an oversight: the surface asks *whether* there is
    /// anything to save and this crate's own [`crate::accepted_save`] asks
    /// *how many*, to say "saving 3 nodes" out loud.
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The nodes a Set file will name, **with every one of them in the store**.
    ///
    /// **Nothing here reads a `.kir`**, which is what collapsing the two arms
    /// bought: a node a build put there was stored by the watcher that built
    /// it, and a node still on its launch version carries the bytes the compile
    /// read. Either way the address was derived from bytes this process has
    /// held all along, and the only thing left to do is make sure the store has
    /// them — `put_artifact` is content-addressed, so putting one that is
    /// already there costs an `exists` and writes nothing.
    ///
    /// **This is the moment a windowed run first touches the store.** Seeding
    /// it at launch instead created a directory for every run whether or not
    /// anything was ever saved; see the surface's `Running::at_launch`.
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
                    // **Beside the bytes, on [`Placed::put`]'s terms**: the
                    // card is derived and the artifact is not, so a card that
                    // will not write is said and not raised. Inside the `if`
                    // because it is the same condition — a node whose bytes
                    // were already in the store has a card there too.
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
