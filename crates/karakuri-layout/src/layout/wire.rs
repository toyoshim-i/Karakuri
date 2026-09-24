use serde::{Deserialize, Serialize, Serializer};

use super::types::{Node, NodeId};
use super::Layout;
use crate::Rect;

/// Intermediate deserialization format for [`Layout`].
///
/// Pre-sizes rectangles and scratch buffers to prevent allocations during subsequent solves,
/// and validates structure and name uniqueness via [`check_structure`].
#[derive(Deserialize)]
pub(crate) struct Wire {
    pub(crate) nodes: Vec<Node>,
    pub(crate) root: NodeId,
    pub(crate) viewport: Rect,
    #[serde(default)]
    pub(crate) soloed: Option<NodeId>,
    #[serde(default)]
    pub(crate) saved: Vec<bool>,
}

/// The same shape, borrowed, so saving copies nothing. Named `Layout` on the
/// wire because that is what it is; only the buffers it leaves out differ.
#[derive(Serialize)]
#[serde(rename = "Layout")]
pub(crate) struct WireOut<'a> {
    pub(crate) nodes: &'a [Node],
    pub(crate) root: NodeId,
    pub(crate) viewport: Rect,
    pub(crate) soloed: Option<NodeId>,
    pub(crate) saved: &'a [bool],
}

impl Serialize for Layout {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        WireOut {
            nodes: &self.arrangement.nodes,
            root: self.arrangement.root,
            viewport: self.viewport,
            soloed: self.arrangement.soloed,
            saved: &self.arrangement.saved,
        }
        .serialize(s)
    }
}

impl TryFrom<Wire> for Layout {
    type Error = LoadError;

    fn try_from(w: Wire) -> Result<Layout, LoadError> {
        // Structure before names: everything below assumes an arena that is a
        // tree, including the solve this ends with, and a file that is not one
        // makes the name rule the least of what is wrong with it.
        check_structure(&w.nodes, w.root, w.soloed)?;
        if let Some(name) = duplicate_name(&w.nodes) {
            return Err(LoadError::DuplicateName {
                name: name.to_owned(),
            });
        }
        let mut saved = w.saved;
        saved.resize(w.nodes.len(), false);
        let mut layout = Layout::assemble(w.nodes, w.root, saved);
        layout.arrangement.soloed = w.soloed;
        layout.viewport = sane(w.viewport);
        layout.solve();
        Ok(layout)
    }
}

/// The first name that appears twice, in declaration order, or `None` where
/// every name is its own. Linear because an arrangement is tens of nodes and a
/// hash set would cost more than it saved.
pub(crate) fn duplicate_name(nodes: &[Node]) -> Option<&str> {
    nodes.iter().enumerate().find_map(|(i, n)| {
        let name = n.name()?;
        nodes[..i]
            .iter()
            .filter_map(Node::name)
            .any(|earlier| earlier == name)
            .then_some(name)
    })
}

/// Detailed reasons for rejecting a serialized layout arrangement.
///
/// Error messages report node numbers matching raw indices in the serialized
/// `nodes` array for straightforward debugging of saved files.
#[derive(Debug, thiserror::Error)]
pub(crate) enum LoadError {
    /// A file truncated to nothing still parses as a `Layout` with an empty
    /// arena, and every index into it is out of range — starting with the root.
    #[error(
        "the arrangement has no nodes: a saved arrangement is a root and whatever hangs from \
         it, so there is nothing here to lay out"
    )]
    NoNodes,
    #[error(
        "the root is node {root}, and the arrangement has {len} nodes: every index in a saved \
         arrangement addresses a node of the same file"
    )]
    RootOutOfRange { root: usize, len: usize },
    #[error(
        "the root, node {root}, records node {parent} as its parent: the root is where the \
         arrangement starts, so nothing stands above it and walking up from any node has to \
         stop there"
    )]
    RootHasParent { root: usize, parent: usize },
    #[error(
        "node {split} lists node {child} as its child {index}, and the arrangement has {len} \
         nodes: every index in a saved arrangement addresses a node of the same file"
    )]
    ChildOutOfRange {
        split: usize,
        index: usize,
        child: usize,
        len: usize,
    },
    #[error(
        "node {split} lists node {child} as a child, and node {child} has already been reached \
         from the root: an arrangement is a tree, so a second route to a node is either one \
         node in two places or a loop, and the solve follows a loop until it runs out of stack"
    )]
    ReachedTwice { split: usize, child: usize },
    #[error(
        "node {child} is a child of node {split} but {}: a parent pointer and a children list \
         are one fact written twice, and where they disagree, walking up from a node reaches \
         somewhere it does not hang from — or never stops",
        records(*parent)
    )]
    ParentDisagrees {
        child: usize,
        split: usize,
        parent: Option<usize>,
    },
    #[error(
        "nothing reaches node {node} from the root: an arrangement is one tree, and a node no \
         children list mentions is a region that is never solved, never drawn and never moved \
         by anything folded above it"
    )]
    Unreachable { node: usize },
    #[error(
        "two nodes are named {name:?}: a name addresses one region, and every surface \
         that is not a pointer reaches a region by its name"
    )]
    DuplicateName { name: String },
    #[error(
        "node {node} is recorded as soloed, and the arrangement has {len} nodes: every index in \
         a saved arrangement addresses a node of the same file, and a solo names the region it \
         is holding so that a surface can say which one"
    )]
    SoloOutOfRange { node: usize, len: usize },
}

/// What a node's `parent` field says, as the middle of a sentence. A missing
/// one is a distinct fault from a wrong one and reads as one.
fn records(parent: Option<usize>) -> String {
    match parent {
        Some(p) => format!("records node {p} as its parent"),
        None => "records no parent at all".to_owned(),
    }
}

/// Validates tree structure and invariants for deserialized arrangements in a single pass.
///
/// Ensures the following invariants:
/// 1. The arena has at least one node and `root` addresses a valid node.
/// 2. The root has no parent, and every other node's `parent` correctly matches the split holding it.
/// 3. Every child index addresses a valid node in the arena.
/// 4. No node is referenced more than once (preventing sharing and cycles).
/// 5. Every node in the arena is reachable from the root.
/// 6. A recorded solo target addresses a valid node.
///
/// Note that programmatic specs built via [`Layout::new`] guarantee these
/// invariants by construction.
pub(crate) fn check_structure(
    nodes: &[Node],
    root: NodeId,
    soloed: Option<NodeId>,
) -> Result<(), LoadError> {
    let len = nodes.len();
    if len == 0 {
        return Err(LoadError::NoNodes);
    }
    if root.0 >= len {
        return Err(LoadError::RootOutOfRange { root: root.0, len });
    }
    if let Some(NodeId(node)) = soloed {
        if node >= len {
            return Err(LoadError::SoloOutOfRange { node, len });
        }
    }
    if let Some(NodeId(parent)) = nodes[root.0].parent {
        return Err(LoadError::RootHasParent {
            root: root.0,
            parent,
        });
    }

    // An explicit stack rather than recursion: the depth here is the file's
    // rather than the arrangement's, and meeting a deep one by overflowing the
    // stack is one of the failures this exists to refuse.
    let mut reached = vec![false; len];
    reached[root.0] = true;
    let mut pending = vec![root.0];
    while let Some(split) = pending.pop() {
        let crate::layout::types::Kind::Split { children, .. } = &nodes[split].kind else {
            continue;
        };
        for (index, &NodeId(child)) in children.iter().enumerate() {
            if child >= len {
                return Err(LoadError::ChildOutOfRange {
                    split,
                    index,
                    child,
                    len,
                });
            }
            // Before the parent check, because a node the walk has already
            // placed is a loop or a duplicate whichever way its parent points,
            // and that is the fault worth naming.
            if reached[child] {
                return Err(LoadError::ReachedTwice { split, child });
            }
            if nodes[child].parent != Some(NodeId(split)) {
                return Err(LoadError::ParentDisagrees {
                    child,
                    split,
                    parent: nodes[child].parent.map(|NodeId(p)| p),
                });
            }
            reached[child] = true;
            pending.push(child);
        }
    }

    match reached.iter().position(|r| !r) {
        Some(node) => Err(LoadError::Unreachable { node }),
        None => Ok(()),
    }
}

/// A viewport with a negative — or NaN — size is a caller's arithmetic, not a
/// state this crate has an answer for. It becomes zero, because every rectangle
/// this crate produces is non-negative and a negative one would propagate
/// through every child.
pub(crate) fn sane(r: Rect) -> Rect {
    Rect::new(r.x, r.y, r.w.max(0.0), r.h.max(0.0))
}
