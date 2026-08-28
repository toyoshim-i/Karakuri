//! The arena, the solve, and the operations that change what is stored.
//!
//! Read the crate documentation first: **solving never writes back into the
//! model**, and everything below is arranged so that it cannot.
//!
//! The arrangement — the nodes, the root, and what a [`Layout::solo`] saved —
//! is one struct, [`Arrangement`]; the buffers a solve writes are another,
//! [`Solved`]. [`Layout::solve`] destructures itself into the two, which gives
//! it disjoint borrows, and hands the solve `&Arrangement` with `&mut Solved`.
//! So the solve is a pair of free functions that *cannot* write a node: there
//! is no `&mut` to one anywhere in the call, and an edit that tried to store a
//! solved size is a borrow error rather than a slow leak nobody sees. Writing
//! a node needs [`Layout::node_mut`], and every caller of it is an operation.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::spec::Spec;
use crate::{Axis, Point, Rect, Sizing};

/// A handle into a [`Layout`]'s arena.
///
/// Opaque on purpose: a name is resolved once by [`Layout::find`], at build or
/// load time, and everything on the frame path carries the id. A name lookup
/// per frame per region is a string comparison the panel never has to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(usize);

/// What a point is touching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    /// A view's interior.
    View(NodeId),
    /// The boundary between visible children `index` and `index + 1` of
    /// `split` — the pair a [`Layout::set_divider`] with the same `index`
    /// moves.
    Divider { split: NodeId, index: usize },
    /// Outside the viewport, or somewhere no visible node claims.
    Nothing,
}

/// What a node is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum Kind {
    View {
        name: String,
    },
    Split {
        /// Optional, where a view's is required: most splits are structure
        /// nobody addresses, and the ones that are — the console's left pane
        /// is one — are addressed by exactly the same name a view is.
        name: Option<String>,
        axis: Axis,
        divider: f32,
        children: Vec<NodeId>,
    },
}

/// One node of the arena. The constraints are along the *parent's* axis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Node {
    kind: Kind,
    sizing: Sizing,
    min: f32,
    /// `f32::INFINITY` means unbounded, and is written as `null` rather than as
    /// a number — see [`unbounded`].
    #[serde(with = "unbounded")]
    max: f32,
    /// Folded by the operator: what [`Layout::collapse`] writes, what a
    /// [`Spec`] can start a node with, and what a saved arrangement carries.
    collapsed: bool,
    /// Set aside by whoever is drawing, because it has put that region
    /// somewhere else or has nowhere to put it.
    ///
    /// **Never saved**, which is the whole of what `skip` says here and the
    /// reason the wire format did not have to grow a field. It is a function
    /// of the geometry the caller has in front of it, so the value that holds
    /// after a load is derived on the next solve rather than remembered — and
    /// a file that carried one could only disagree with the frame that read
    /// it, which is a disagreement no loader could adjudicate because the file
    /// does not know the size of the window it is opening into.
    #[serde(skip)]
    aside: bool,
    parent: Option<NodeId>,
}

impl Node {
    /// The name this node answers to, if it has one.
    fn name(&self) -> Option<&str> {
        match &self.kind {
            Kind::View { name } => Some(name),
            Kind::Split { name, .. } => name.as_deref(),
        }
    }
}

/// An unbounded maximum is `f32::INFINITY`, and JSON has no spelling for it:
/// `serde_json` writes an infinity as `null` and then refuses to read a `null`
/// back as an `f32`, so a layout saved with any default maximum would not load.
/// Writing it as an explicit absence round trips, and reads as what it means.
mod unbounded {
    use super::{Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(v: &f32, s: S) -> Result<S::Ok, S::Error> {
        match v.is_finite() {
            true => s.serialize_some(v),
            false => s.serialize_none(),
        }
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<f32, D::Error> {
        Ok(Option::<f32>::deserialize(d)?.unwrap_or(f32::INFINITY))
    }
}

/// The arrangement itself: what the operator arranged, and nothing that is
/// derived from it.
///
/// **This half is what a solve may not write.** It is a separate struct so
/// that saying so is the compiler's job rather than a comment's: the solve
/// takes `&Arrangement`, so every node in it is read-only for the whole of the
/// solve, and no amount of care is required to keep it that way.
#[derive(Debug, Clone)]
struct Arrangement {
    nodes: Vec<Node>,
    root: NodeId,
    /// What [`Layout::solo`] is holding, and the collapsed flags it replaced.
    /// Both are saved: an arrangement stored while soloed comes back soloed,
    /// and [`Layout::unsolo`] still has something to restore.
    ///
    /// The node rather than a flag, because the flags a solo leaves behind do
    /// not identify it — see [`Layout::soloed`].
    soloed: Option<NodeId>,
    saved: Vec<bool>,
}

/// What a solve writes, and the only thing it writes.
///
/// Sized once at build, so a solved frame allocates nothing.
#[derive(Debug, Clone)]
struct Solved {
    /// Parallel to the arena's nodes. The solve writes here and
    /// [`Layout::rect`] reads here.
    rects: Vec<Rect>,
    /// Scratch for one split's children, sized to the whole arena so it fits
    /// any split. A split's sizes are finished before the solve descends into
    /// any child, so one buffer serves the whole recursion.
    sizes: Vec<f32>,
    frozen: Vec<bool>,
    /// Parallel to the arena's nodes, like `rects`: how much extent each node
    /// **can use** along its parent's axis, written by [`measure`] at the top
    /// of every solve and read while the split above it hands out sizes.
    ///
    /// **It is here, and not on a node, because it is derived.** It is the one
    /// thing in the solve that is computed bottom-up, so the next reader will
    /// expect it to have been written into the tree — it is not, and cannot
    /// be: [`measure`] takes the arrangement by shared reference like the rest
    /// of the solve, so P-0071 holds for it by the same construction. Nothing
    /// here survives the solve that computed it; every entry is rewritten
    /// before it is read again.
    usable: Vec<f32>,
}

/// An arrangement of regions, and the rectangles it currently solves to.
///
/// `serde` on this is the whole of saving and restoring an operator's
/// arrangement: the stored sizes, the collapsed flags and the viewport are all
/// here, and the solved rectangles are not — they are derived, and a derived
/// value on disk is a second answer waiting to disagree with the first.
///
/// **[`set_aside`](Layout::set_aside) is on the derived side of that line and
/// is not written either**, which is why the wire format is the same one it
/// was: a saved arrangement is what the operator arranged, and a node the
/// caller has taken out of the layout is a fact about the geometry it was
/// drawing at the time. A load leaves every node laid out, and the first
/// caller to draw the loaded arrangement says otherwise where it still holds.
///
/// **A file that deserialises without error is one that can be solved, hit
/// tested and operated.** Loading it is where that is established, and it is
/// the only place it has to be: a truncated, hand-edited or older file whose
/// arena is not a tree — an index addressing no node, a node in two places, a
/// parent pointer that disagrees with the list holding it — is refused with a
/// sentence saying which node and what is wrong with it, rather than loading
/// and panicking, aborting or hanging later. See `check_structure`.
#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "Wire")]
pub struct Layout {
    arrangement: Arrangement,
    viewport: Rect,
    solved: Solved,
    dirty: bool,
}

/// The wire form. It exists so that a deserialised `Layout` arrives with its
/// rectangle and scratch buffers already sized: leaving them out of the derive
/// alone would leave them empty, and the first solve after a load would
/// allocate. It is also where a saved arrangement is checked — for the name
/// rule [`Layout::new`] states, and for being an arena that is a tree at all
/// ([`check_structure`]). On this side both are a rejected file rather than a
/// panic, because a file is data and a [`Spec`] is code.
#[derive(Deserialize)]
struct Wire {
    nodes: Vec<Node>,
    root: NodeId,
    viewport: Rect,
    #[serde(default)]
    soloed: Option<NodeId>,
    #[serde(default)]
    saved: Vec<bool>,
}

/// The same shape, borrowed, so saving copies nothing. Named `Layout` on the
/// wire because that is what it is; only the buffers it leaves out differ.
#[derive(Serialize)]
#[serde(rename = "Layout")]
struct WireOut<'a> {
    nodes: &'a [Node],
    root: NodeId,
    viewport: Rect,
    soloed: Option<NodeId>,
    saved: &'a [bool],
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
fn duplicate_name(nodes: &[Node]) -> Option<&str> {
    nodes.iter().enumerate().find_map(|(i, n)| {
        let name = n.name()?;
        nodes[..i]
            .iter()
            .filter_map(Node::name)
            .any(|earlier| earlier == name)
            .then_some(name)
    })
}

/// Why a saved arrangement was refused: one variant per rule the file broke.
///
/// **Every number in a message is an index into the file's own `nodes` array**,
/// because that is what a person looking at a saved session has in front of
/// them — a `NodeId` is written on the wire as the bare number it is, so
/// "node 7" is the eighth entry of `nodes` and nothing has to be counted twice.
///
/// It is deliberately not public, and could not usefully be: `serde` turns it
/// into the deserialiser's own error by way of `Display` before any caller of
/// `from_str` sees it, so the sentence *is* the whole of what reaches anyone.
/// It is a type rather than a `format!` at each site so that the nine
/// sentences sit together where they can be read as one voice, and so that
/// adding a rule to [`check_structure`] is adding a variant here rather than
/// another string somewhere else.
#[derive(Debug, thiserror::Error)]
enum LoadError {
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

/// Everything a saved arrangement has to be before any of it can be solved,
/// hit-tested or operated, checked in one pass on the load path.
///
/// **The arena's invariants, and what each of them costs when it is missing:**
///
/// 1. There is at least one node, and `root` addresses one. Otherwise the first
///    line of [`Layout::solve`] indexes past the rectangle buffer.
/// 2. The root has no parent, and every other node's `parent` is exactly the
///    split whose children list holds it. This is the one that matters most:
///    [`Layout::visible`] and [`Arrangement::is_ancestor`] walk *up*, and a
///    parent chain that closes into a loop makes them **hang**, which is the
///    worst of these failures because there is no panic to read afterwards.
///    Deriving the parents from the walk instead would hide the disagreement
///    rather than report it, and the file would still be a file that disagrees
///    with itself.
/// 3. Every child index addresses a node. Otherwise the first solve indexes
///    past the arena.
/// 4. No node is reached twice. A children list that points at a node already
///    in the tree is either one node in two places or a cycle, and
///    [`solve_subtree`] recurses on children — so a cycle is a stack overflow
///    at load, which aborts the process rather than returning an error.
/// 5. Every node is reached. This one is a decision rather than a crash: an
///    orphan solves to nothing, draws nothing, and cannot be reached by a fold
///    or a solo above it, so it is a region that exists in the file and nowhere
///    else. A loader that accepted it would be accepting a file it has already
///    read as a tree plus some debris.
/// 6. A recorded solo addresses a node. It is an index like any other, and
///    [`Layout::soloed`] hands it to a caller that will ask for its rectangle
///    or its name — so an out-of-range one indexes past the arena at whatever
///    later moment a status line is drawn.
///
/// **What [`Layout::set_aside`] carries is not checked, because it is not
/// here.** It is not on the wire at all, so a file cannot disagree with itself
/// about it and there is no rule for this to enforce: every node of a loaded
/// arrangement is laid out until the caller says otherwise, whatever the file
/// says and whatever the arena it was assembled into had been carrying. A
/// check would need a second copy of the bit to compare against, and putting
/// one in the file to check it is precisely what not saving it avoids.
///
/// **[`Layout::new`] needs none of this**, and does not pay for it: [`build`]
/// pushes each node once, hands it the parent it was built under, and fills a
/// split's children with the ids it just created, so 1 to 5 hold of anything a
/// [`Spec`] can express. Only a file can be wrong in these ways, so only the
/// file is checked.
///
/// Not every disagreement is refused. A `min` above a `max`, a negative or NaN
/// size, a split with no children — a [`Spec`] can express all of those and
/// [`Layout::new`] accepts them, so refusing them here would make a file and
/// the code that writes it disagree about what an arrangement is. The solve is
/// total over them: it clamps at zero, iterates a bounded number of passes and
/// leaves the discrepancy as trailing space.
fn check_structure(nodes: &[Node], root: NodeId, soloed: Option<NodeId>) -> Result<(), LoadError> {
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
        let Kind::Split { children, .. } = &nodes[split].kind else {
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
fn sane(r: Rect) -> Rect {
    Rect::new(r.x, r.y, r.w.max(0.0), r.h.max(0.0))
}

impl Layout {
    /// Build an arrangement from its declarative form.
    ///
    /// The viewport starts empty, so every rectangle is zero until
    /// [`set_viewport`](Layout::set_viewport) says otherwise.
    ///
    /// # Names are unique, and this is where that is checked
    ///
    /// **Panics if two nodes carry the same name**, whether they are views,
    /// splits or one of each. A name is how every surface that is not a
    /// pointer — the keyboard, a MIDI map, MCP — reaches a region, so a name
    /// that answers to two regions is an arrangement that cannot be operated,
    /// and [`find`](Layout::find) resolving whichever came first would make it
    /// look like it could. It is a mistake in a [`Spec`], which is code, so it
    /// is caught at the earliest moment it exists rather than at the first
    /// operation aimed at the wrong pane.
    ///
    /// Deserialising a layout whose names collide fails with the same message
    /// instead of panicking: a file is data, and data is rejected.
    pub fn new(spec: Spec) -> Layout {
        let mut nodes = Vec::new();
        let root = build(&mut nodes, spec, None);
        if let Some(name) = duplicate_name(&nodes) {
            panic!(
                "{}",
                LoadError::DuplicateName {
                    name: name.to_owned()
                }
            );
        }
        let saved = vec![false; nodes.len()];
        let mut layout = Layout::assemble(nodes, root, saved);
        layout.solve();
        layout
    }

    /// The one place the derived buffers are sized, so `new` and a
    /// deserialisation cannot disagree about it.
    fn assemble(nodes: Vec<Node>, root: NodeId, saved: Vec<bool>) -> Layout {
        let n = nodes.len();
        Layout {
            arrangement: Arrangement {
                nodes,
                root,
                soloed: None,
                saved,
            },
            viewport: Rect::new(0.0, 0.0, 0.0, 0.0),
            solved: Solved {
                rects: vec![Rect::new(0.0, 0.0, 0.0, 0.0); n],
                sizes: vec![0.0; n],
                frozen: vec![false; n],
                usable: vec![0.0; n],
            },
            dirty: true,
        }
    }

    /// The root, which always fills the viewport exactly. Its own [`Sizing`],
    /// `min` and `max` are ignored — there is nothing to be relative to.
    pub fn root(&self) -> NodeId {
        self.arrangement.root
    }

    /// Resolve a name to its id. A view always has one; a split has one where
    /// the arrangement gave it one.
    ///
    /// Names are the caller's, and [`Layout::new`] refuses an arrangement that
    /// uses one twice, so the answer here is the only node that could be meant.
    pub fn find(&self, name: &str) -> Option<NodeId> {
        self.arrangement
            .nodes
            .iter()
            .position(|n| n.name() == Some(name))
            .map(NodeId)
    }

    /// A node's name: a view's, a named split's, or `None` for a split the
    /// arrangement left unnamed.
    pub fn name(&self, id: NodeId) -> Option<&str> {
        self.node(id.0).name()
    }

    /// A split's children in order, or an empty slice for a view. A child
    /// that is out of the layout — folded, or
    /// [`set_aside`](Layout::set_aside) — is still here: it has a zero-extent
    /// rectangle, not no rectangle.
    pub fn children(&self, id: NodeId) -> &[NodeId] {
        match &self.node(id.0).kind {
            Kind::Split { children, .. } => children,
            Kind::View { .. } => &[],
        }
    }

    /// A split's **visible** children in order: the ones that are laid out,
    /// which is what a divider index counts and what
    /// [`set_divider`](Layout::set_divider) and [`Hit::Divider`] mean by one.
    ///
    /// **This is the distinction most easily got wrong**, and it was stated
    /// only in prose on `set_divider` while every caller reconstructed the
    /// filter for itself. Boundary 1 of a split whose second child is folded
    /// is between its third and fourth children, and a caller counting
    /// [`children`](Layout::children) would move the wrong pair.
    ///
    /// An iterator rather than a `Vec`, because this is on the frame path: a
    /// view asking which children a split is showing does it per split per
    /// frame, and it must not allocate to do it.
    ///
    /// Only the children's own flags are read, and **both of them are read**:
    /// a child the operator folded and one whoever is drawing
    /// [`set_aside`](Layout::set_aside) are equally not here, because a
    /// divider is drawn between the children a split is showing and neither of
    /// those is one. A visible child of a folded split is still listed here —
    /// it is out by its ancestor rather than by itself, which is exactly the
    /// difference [`visible`](Layout::visible) and
    /// [`is_collapsed`](Layout::is_collapsed) already carry — and everything
    /// under a fold solves to zero extent, so what is derived from it is empty
    /// rather than wrong.
    pub fn visible_children(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.children(id)
            .iter()
            .copied()
            .filter(|c| !self.arrangement.out_of_layout(c.0))
    }

    /// The split `id` hangs from, or `None` for the root.
    ///
    /// **Upward, which nothing else here answers.** A [`hit`](Layout::hit)
    /// only ever resolves to a leaf or to a divider, so *fold the split
    /// enclosing what is under the pointer* has no route without this, and a
    /// caller that keeps its own parent per node has a second copy of the
    /// arena to keep in step — which the console did, and had to rebuild
    /// whenever the layout was replaced.
    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.node(id.0).parent
    }

    /// A split's axis, or `None` for a view.
    pub fn axis(&self, id: NodeId) -> Option<Axis> {
        self.arrangement.split_of(id.0).map(|(axis, _)| axis)
    }

    /// Whether `id` is a view — a leaf, the thing something paints. Everything
    /// else is a split, whose gaps are what is visible between its children.
    ///
    /// The same answer as `axis(id).is_none()`, said as what it means: a
    /// caller drawing regions is asking what kind of node this is, not which
    /// way it lays its children out.
    pub fn is_view(&self, id: NodeId) -> bool {
        self.arrangement.split_of(id.0).is_none()
    }

    /// The divider thickness a split declares. What is actually drawn is this
    /// until the split is too narrow to hold that many, at which point the
    /// dividers shrink with it rather than the children overflowing.
    pub fn divider(&self, id: NodeId) -> Option<f32> {
        self.arrangement.split_of(id.0).map(|(_, divider)| divider)
    }

    /// A node's `[min, max]` along its parent's axis. `max` is
    /// `f32::INFINITY` where it is unbounded.
    ///
    /// Exposed because a caller that has just been told by
    /// [`set_divider`](Layout::set_divider) that a drag landed somewhere other
    /// than where it was aimed has no other way to say which constraint
    /// stopped it.
    pub fn bounds(&self, id: NodeId) -> (f32, f32) {
        (self.node(id.0).min, self.node(id.0).max)
    }

    /// How `id` claims extent along its parent's axis: a size it keeps, or a
    /// share of what is left.
    ///
    /// Exposed for the same reason [`bounds`](Layout::bounds) is, and it is
    /// the other half of that answer. *Why does this region keep its size when
    /// the window widens?* is half of what a person asks the panel, and
    /// `bounds` cannot answer it — a [`Sizing::Fixed`] region with no maximum
    /// still does not grow.
    pub fn sizing(&self, id: NodeId) -> Sizing {
        self.node(id.0).sizing
    }

    /// The viewport as it was last set.
    pub fn viewport(&self) -> Rect {
        self.viewport
    }

    /// Give the layout the space it has. Marks it dirty; nothing is recomputed
    /// until [`solve`](Layout::solve).
    ///
    /// **This is not an edit.** No stored size changes, however small the
    /// viewport gets — see the crate documentation.
    pub fn set_viewport(&mut self, viewport: Rect) {
        let viewport = sane(viewport);
        if viewport != self.viewport {
            self.viewport = viewport;
            self.dirty = true;
        }
    }

    /// The rectangle `id` last solved to.
    ///
    /// Every rectangle has non-negative width and height, and every visible
    /// node's lies inside the viewport.
    pub fn rect(&self, id: NodeId) -> Rect {
        debug_assert!(!self.dirty, "rect() read a stale solve; call solve() first");
        self.solved.rects[id.0]
    }

    /// Whether `id` is drawn: false if it or any ancestor is out of the
    /// layout, either folded by the operator or
    /// [`set_aside`](Layout::set_aside) by whoever is drawing.
    ///
    /// **This is the question every reader that lays something out asks**, and
    /// it is the disjunction rather than either bit. A node is out of the
    /// layout for two independent reasons that arrive from two different
    /// places and are cleared by two different callers, and code that asks
    /// only one of them is correct right up until the other one happens —
    /// which is a defect that appears as an empty strip where a region used to
    /// be, at a moment nobody was editing the arrangement.
    pub fn visible(&self, id: NodeId) -> bool {
        let mut cur = Some(id);
        while let Some(NodeId(i)) = cur {
            if self.arrangement.out_of_layout(i) {
                return false;
            }
            cur = self.node(i).parent;
        }
        true
    }

    /// Whether `id` itself is collapsed — **folded by the operator**,
    /// regardless of its ancestors.
    ///
    /// This is [`collapse`](Layout::collapse)'s bit and only it: the answer to
    /// *did someone fold this*, which is what an operation that unfolds asks
    /// and what is written down when the arrangement is saved. It is **not**
    /// the answer to *is this laid out* — a node nobody folded may still have
    /// been [`set_aside`](Layout::set_aside) — and
    /// [`visible`](Layout::visible) is what answers that.
    pub fn is_collapsed(&self, id: NodeId) -> bool {
        self.node(id.0).collapsed
    }

    /// Whether `id` itself has been [`set_aside`](Layout::set_aside),
    /// regardless of its ancestors and of whether it is also folded.
    ///
    /// The mirror of [`is_collapsed`](Layout::is_collapsed), and read for the
    /// same narrow purpose: to report the bit, or to decide whether to clear
    /// the one you set. A caller working out what to draw wants
    /// [`visible`](Layout::visible).
    pub fn is_set_aside(&self, id: NodeId) -> bool {
        self.node(id.0).aside
    }

    /// What a [`solo`](Layout::solo) is holding, or `None` where none is in
    /// force.
    ///
    /// **Which, not whether.** A status line saying *soloed* and not what is
    /// soloed is telling an operator the one thing they can already see, and
    /// the collapsed flags a solo leaves behind do not say which node it was
    /// aimed at — a split with a single child produces exactly the flags its
    /// child does — so it is stored rather than recovered.
    pub fn soloed(&self) -> Option<NodeId> {
        self.arrangement.soloed
    }

    /// Whether a [`solo`](Layout::solo) is in force. The yes-or-no of
    /// [`soloed`](Layout::soloed), which is what an operation that undoes one
    /// asks.
    pub fn is_soloed(&self) -> bool {
        self.arrangement.soloed.is_some()
    }

    // -- operations ------------------------------------------------------

    /// Fold `id` away: zero extent, and no divider beside it.
    ///
    /// Its stored size is untouched, which is the whole of why
    /// [`expand`](Layout::expand) can restore it exactly. Folding a split
    /// folds everything inside it: nothing under a collapsed node is
    /// [`visible`](Layout::visible), and none of it takes any space.
    pub fn collapse(&mut self, id: NodeId) {
        self.set_collapsed(id, true);
    }

    /// Unfold `id`, back to the size it was storing all along.
    pub fn expand(&mut self, id: NodeId) {
        self.set_collapsed(id, false);
    }

    /// Flip `id`, returning whether it is now collapsed.
    pub fn toggle(&mut self, id: NodeId) -> bool {
        let now = !self.node(id.0).collapsed;
        self.set_collapsed(id, now);
        now
    }

    fn set_collapsed(&mut self, id: NodeId, collapsed: bool) {
        if self.node(id.0).collapsed != collapsed {
            self.node_mut(id.0).collapsed = collapsed;
            self.dirty = true;
        }
    }

    /// Take `id` out of the layout, or put it back, for a reason that is not
    /// the operator's: whoever is drawing has put that region somewhere else,
    /// or has nowhere to put it.
    ///
    /// The effect on the solve is exactly a fold's — zero extent, no divider
    /// beside it, nothing under it [`visible`](Layout::visible), and no stored
    /// size touched. **What differs is whose bit it is**, and that is why it
    /// is a second bit rather than the same one:
    ///
    /// - A fold is the operator's. It is written by
    ///   [`collapse`](Layout::collapse), it is saved with the arrangement, and
    ///   a load brings it back.
    /// - This is a function of the geometry, and of nothing the operator did.
    ///   It is **never saved**, because the next caller to draw the
    ///   arrangement re-derives it from the space it actually has.
    ///
    /// One bit for both makes the two indistinguishable, and both directions
    /// of that cost something an operator sees: unfolding while a region is
    /// set aside would put an empty strip back where the caller had already
    /// drawn that region elsewhere, and a fold the operator never made would
    /// be written into their saved arrangement.
    ///
    /// So the two are independent in both directions.
    /// [`expand`](Layout::expand) does not lay out a node that is set aside,
    /// this does not unfold one the operator folded, and a node that is both
    /// needs both cleared. Each bit is cleared by whoever set it.
    ///
    /// **Writing the value a node already carries marks nothing dirty**, which
    /// is what makes this safe to call every frame for every node: a caller
    /// re-stating the arrangement it stated last frame costs a comparison per
    /// node and no solve at all. A layout that went dirty on every frame would
    /// cost a still panel the price of a moving one.
    pub fn set_aside(&mut self, id: NodeId, aside: bool) {
        if self.node(id.0).aside != aside {
            self.node_mut(id.0).aside = aside;
            self.dirty = true;
        }
    }

    /// Collapse everything that is neither `id`, nor on the path from the root
    /// to it, nor inside it — so `id` is left holding the whole viewport.
    ///
    /// The collapsed state it replaces is kept whole, panes that were already
    /// collapsed included, and [`unsolo`](Layout::unsolo) puts it back. A solo
    /// while already soloed re-aims without saving again: however many times it
    /// is called, one `unsolo` returns to the arrangement before the first.
    ///
    /// **It saves and writes the operator's fold, and only that.** A solo is
    /// an operation on the arrangement, and what
    /// [`set_aside`](Layout::set_aside) carries is not part of the
    /// arrangement: saving it would mean restoring, at the unsolo, a bit the
    /// caller has since rewritten from a geometry that changed *because* of
    /// the solo. So a node that was set aside is still set aside through a
    /// solo and after the unsolo — including the soloed node itself, which is
    /// left holding the viewport and still not laid out until whoever set it
    /// aside decides otherwise. This crate does not guess that for it.
    pub fn solo(&mut self, id: NodeId) {
        let a = &mut self.arrangement;
        if a.soloed.is_none() {
            for i in 0..a.nodes.len() {
                a.saved[i] = a.nodes[i].collapsed;
            }
        }
        a.soloed = Some(id);
        for i in 0..a.nodes.len() {
            a.nodes[i].collapsed = !a.on_solo_path(i, id);
        }
        self.dirty = true;
    }

    /// Restore exactly the collapsed state [`solo`](Layout::solo) replaced.
    /// A no-op when nothing is soloed.
    pub fn unsolo(&mut self) {
        let a = &mut self.arrangement;
        if a.soloed.is_none() {
            return;
        }
        for i in 0..a.nodes.len() {
            a.nodes[i].collapsed = a.saved[i];
        }
        a.soloed = None;
        self.dirty = true;
    }

    /// Move the boundary between visible children `index` and `index + 1` of
    /// `split` to `position`, an **absolute coordinate along the split's
    /// axis**, and return where it actually landed.
    ///
    /// Absolute rather than a delta on purpose. A pointer drag that runs past a
    /// stop and comes back would accumulate drift under deltas — every frame
    /// past the stop contributes a difference the clamp throws away, and the
    /// pointer comes back to find the boundary somewhere it was never dragged
    /// to. With an absolute position there is nothing to accumulate: the same
    /// pointer coordinate always names the same boundary position.
    ///
    /// **A drag stops at the first constraint and never pushes through to a
    /// further neighbour.** Only the two children either side of the boundary
    /// change; their combined extent is what it was.
    ///
    /// The boundary is the *far edge of child `index`* — the near edge of the
    /// divider drawn between them, which is also where [`hit`](Layout::hit)
    /// centres that divider's grab area.
    ///
    /// **This is an edit, and it edits what is on screen.** It reads the
    /// solved rectangles, so a drag made while the viewport is too small to
    /// show the pair — which a pointer cannot do, since
    /// [`hit`](Layout::hit) finds no divider there, but a keyboard or a remote
    /// caller can — writes the sizes that viewport implies. That is not the
    /// viewport writing back into the model; it is a caller asking for a
    /// boundary to be somewhere, at a moment when everywhere is the same
    /// place.
    pub fn set_divider(&mut self, split: NodeId, index: usize, position: f32) -> f32 {
        self.solve();
        let (axis, _) = match self.arrangement.split_of(split.0) {
            Some(s) => s,
            None => return position,
        };
        let (a, b) = match (
            self.arrangement.visible_child(split.0, index),
            self.arrangement.visible_child(split.0, index + 1),
        ) {
            (Some(a), Some(b)) => (a, b),
            _ => return position,
        };

        let start = axis.origin(self.solved.rects[a]);
        let span = axis.extent(self.solved.rects[a]) + axis.extent(self.solved.rects[b]);

        // The pair's own bounds, and the pair's share of the split, are the
        // whole of what constrains this. Nothing beyond `b` is consulted,
        // because nothing beyond `b` moves.
        let lo = self.node(a).min.max(span - self.node(b).max).max(0.0);
        let hi = self.node(a).max.min(span - self.node(b).min).max(lo);
        let size_a = (position - start).clamp(lo, hi);
        let size_b = span - size_a;

        self.resize_pair(split.0, a, b, size_a, size_b, span);
        self.dirty = true;
        self.solve();
        axis.far(self.solved.rects[a])
    }

    /// Write a drag's two sizes into the model. This is an explicit operation,
    /// so it may write; a solve may not.
    ///
    /// A [`Sizing::Fixed`] child simply stores the new size. A
    /// [`Sizing::Flex`] one stores a weight chosen so the next solve reproduces
    /// that size exactly, which is what makes a drag out and back land where it
    /// started rather than a little off each time.
    fn resize_pair(&mut self, split: usize, a: usize, b: usize, sa: f32, sb: f32, span: f32) {
        match (self.node(a).sizing, self.node(b).sizing) {
            (Sizing::Fixed(_), Sizing::Fixed(_)) => {
                self.node_mut(a).sizing = Sizing::Fixed(sa);
                self.node_mut(b).sizing = Sizing::Fixed(sb);
            }
            // Two flexible neighbours: hold the pair's total weight and split
            // it in the new proportion. The pool and the split's total weight
            // are then both unchanged, so the pair keeps its combined extent
            // and divides it as dragged — exactly, and with no reference to
            // any sibling.
            (Sizing::Flex(wa), Sizing::Flex(wb)) => {
                let total = wa.max(0.0) + wb.max(0.0);
                if span > f32::EPSILON && total > 0.0 {
                    self.node_mut(a).sizing = Sizing::Flex(total * sa / span);
                    self.node_mut(b).sizing = Sizing::Flex(total * sb / span);
                }
            }
            (Sizing::Flex(_), Sizing::Fixed(_)) => {
                self.node_mut(b).sizing = Sizing::Fixed(sb);
                self.reweight(split, a, sa);
            }
            (Sizing::Fixed(_), Sizing::Flex(_)) => {
                self.node_mut(a).sizing = Sizing::Fixed(sa);
                self.reweight(split, b, sb);
            }
        }
    }

    /// Give flexible child `c` the weight that makes it solve to `size`, given
    /// what its siblings now claim.
    ///
    /// Its siblings' flexible weights total `others` and share the pool with
    /// it, so `size = pool * w / (others + w)` inverts to the line below. With
    /// no flexible sibling the child takes the whole pool whatever its weight
    /// is, and the weight is left alone.
    ///
    /// **A fixed sibling is counted at its stored size here, and the solve may
    /// claim less of it** — see [`measure`]. Capping it here instead would be
    /// reading a measurement taken before [`resize_pair`](Layout::resize_pair)
    /// wrote the very sizes this is inverting, which for a fixed *view* is the
    /// size it used to store. So the pool can be understated where a split
    /// holds a capped fixed child *and* more than one flexible one, and the
    /// drag lands a little short of where it was aimed —
    /// [`set_divider`](Layout::set_divider) re-solves and returns where it
    /// actually landed, so what a caller is told is still true.
    fn reweight(&mut self, split: usize, c: usize, size: f32) {
        let Some((axis, _)) = self.arrangement.split_of(split) else {
            return;
        };
        let avail = self
            .arrangement
            .avail(split, axis.extent(self.solved.rects[split]));
        let mut fixed = 0.0;
        let mut others = 0.0;
        for k in 0..self.arrangement.child_count(split) {
            let child = self.arrangement.child(split, k);
            if self.arrangement.out_of_layout(child) {
                continue;
            }
            match self.node(child).sizing {
                Sizing::Fixed(s) => fixed += s.max(0.0),
                Sizing::Flex(w) if child != c => others += w.max(0.0),
                Sizing::Flex(_) => {}
            }
        }
        let pool = avail - fixed;
        if others > 0.0 && pool - size > f32::EPSILON {
            self.node_mut(c).sizing = Sizing::Flex(others * size / (pool - size));
        }
    }

    // -- the solve -------------------------------------------------------

    /// Recompute every rectangle, if anything has changed since the last one.
    ///
    /// Allocates nothing: the rectangle buffer and the per-split scratch were
    /// sized when the layout was built. Clean is a flag test, so calling this
    /// once a frame costs nothing on a frame where nothing moved.
    ///
    /// One split, given the extent it has along its axis:
    ///
    /// 1. Collapsed children take **zero** extent. There is no handle strip —
    ///    re-opening a pane is a named operation, not a mouse target.
    /// 2. Dividers sit only *between visible children*, so what is left to
    ///    distribute is `extent - divider * (visible - 1)`, floored at zero.
    /// 3. [`Sizing::Fixed`] children claim their stored size — or what they
    ///    can use, whichever is smaller. See [`measure`]: **no node claims
    ///    more than its visible content can use**, so a fixed split whose
    ///    content has been folded down claims what is left of it rather than
    ///    the size it stores, and its siblings get the difference.
    ///    [`Sizing::Flex`] children share what is left in proportion to their
    ///    weights.
    /// 4. Each child is clamped to its `[min, max]` — where the minimum is
    ///    also capped by what the child can use, since a minimum is what a
    ///    node needs *while it has something to show*, and holding it with
    ///    nothing behind it is holding space the node will leave empty.
    ///    Clamping changes the total, so this iterates: whoever hit a bound
    ///    freezes there and the remainder is redistributed among the rest.
    ///    Each pass freezes at least one child, so it terminates. When nothing
    ///    flexible is left unfrozen and there is still a discrepancy, the
    ///    fixed children take it in proportion — which is also why a split
    ///    with no flexible child at all still tiles its parent rather than
    ///    leaving a gap. **The cap is on what a node claims, and that branch
    ///    is what happens to what nobody claimed**, so it is the one place a
    ///    fixed child scales from its stored size rather than its capped
    ///    claim: only a `max` stops it growing there, which is ADR-0157
    ///    unchanged.
    /// 5. If the viewport is smaller than the sum of the minima, everything
    ///    scales down in proportion, floored at zero. A minimum is a
    ///    preference, not a licence to overflow, and **a rectangle is never
    ///    negative.**
    ///
    /// The one case where children do not account for their parent exactly is
    /// the mirror of step 5: where *every* visible child is sitting at its
    /// maximum and there is still room, the remainder is left as empty space
    /// after the last of them. A maximum is honoured rather than overridden —
    /// a pane that says it is never wider than 480 is not made 1280 wide by
    /// being the only one left on screen — so the alternative would be to hand
    /// the space to whichever child the code happened to reach last, which is
    /// an arrangement nobody asked for and no operator can undo.
    ///
    /// **None of it can write a size back into the arrangement**, and that is
    /// structural rather than careful: the two lines below split this layout
    /// into the arrangement and the buffers, and everything past them holds the
    /// arrangement by shared reference.
    pub fn solve(&mut self) {
        if !self.dirty {
            return;
        }
        // The split borrow that makes P-0071 a compiler error. `arrangement`
        // is re-borrowed as `&` here and stays that way for the whole solve,
        // so a line that stored a solved size into a node would not compile.
        let Layout {
            arrangement,
            viewport,
            solved,
            dirty,
        } = self;
        let arrangement: &Arrangement = arrangement;
        // Bottom-up first, top-down after: what each node can use is a
        // question about its content, and step 3 is a question about its
        // parent's extent. The first is finished for the whole tree before
        // the second starts, so no split reads a stale one.
        measure(arrangement, solved, arrangement.root.0, None);
        solved.rects[arrangement.root.0] = *viewport;
        solve_subtree(arrangement, solved, arrangement.root.0);
        *dirty = false;
    }

    // -- boundaries ------------------------------------------------------

    /// The gap between visible children `index` and `index + 1` of `split` —
    /// the boundary a [`set_divider`](Layout::set_divider) with the same
    /// `index` moves, and the one a [`Hit::Divider`] with the same `index`
    /// names. `None` where `split` is a view, or where either of that pair is
    /// not there.
    ///
    /// **A rectangle rather than a coordinate**, because both callers want it
    /// that way. A view has to *draw* the divider, and what it draws is
    /// exactly this rectangle — the space between two children, which is what
    /// [`divider`](Layout::divider) is thick and spans the split across its
    /// axis. A caller that wants the coordinate `set_divider` speaks in takes
    /// [`Axis::origin`] of it, which is the far edge of the child before it.
    ///
    /// That is what a press needs and had no way to ask for: `set_divider`
    /// says where a drag *landed* and nothing said where a boundary already
    /// is, so a caller could not work out where along the boundary the pointer
    /// took hold — and a divider that does not know that jumps to the pointer
    /// the moment it is picked up.
    ///
    /// **Both sides of the pair are required, and that is the point.** The
    /// console's own version of this checked only that child `index` existed
    /// and returned that child's far edge, so folding the far side of a
    /// boundary mid-drag handed back the split's own far edge — a position for
    /// a boundary that is no longer there. Here the rectangle *is* the space
    /// between the two, so there is nothing to return without both.
    ///
    /// Reads the solved rectangles: [`solve`](Layout::solve) first.
    pub fn boundary(&self, split: NodeId, index: usize) -> Option<Rect> {
        debug_assert!(
            !self.dirty,
            "boundary() read a stale solve; call solve() first"
        );
        let axis = self.axis(split)?;
        let before = self.arrangement.visible_child(split.0, index)?;
        let after = self.arrangement.visible_child(split.0, index + 1)?;
        let start = axis.far(self.solved.rects[before]);
        let size = (axis.origin(self.solved.rects[after]) - start).max(0.0);
        Some(axis.slice(self.solved.rects[split.0], start, size))
    }

    /// Every boundary in the arrangement, as the `(split, index)` pair that
    /// addresses it, splits in the order [`children`](Layout::children) walks
    /// them.
    ///
    /// A window has a pointer to find a boundary with; **a view has this** —
    /// it is what a frame iterates to draw the dividers, and what a caller
    /// with no pointer at all iterates to reach them by name. It was a test
    /// helper first, which is what it looks like when a question the crate
    /// should answer is answered somewhere it cannot be reached from.
    ///
    /// An iterator rather than a `Vec`, because a frame that drew the dividers
    /// would otherwise allocate to find them. Nothing here reads a rectangle,
    /// so it does not need a solve; a boundary inside a folded split is listed
    /// like any other and [`boundary`](Layout::boundary) gives it the zero
    /// extent everything under a fold has.
    pub fn boundaries(&self) -> impl Iterator<Item = (NodeId, usize)> + '_ {
        (0..self.arrangement.nodes.len()).flat_map(move |i| {
            (0..self.arrangement.visible_count(i).saturating_sub(1)).map(move |k| (NodeId(i), k))
        })
    }

    // -- hit testing -----------------------------------------------------

    /// What is under `p`, with a divider's grab area widened by `grab` on each
    /// side.
    ///
    /// **A divider's grab area is wider than the divider it draws.** A boundary
    /// drawn one pixel wide is not a target a hand can find, and widening what
    /// is drawn instead would spend the panel's space on something that is
    /// there to be dragged rather than to be seen. So a point inside the grab
    /// area belongs to the divider, not to the view under it.
    ///
    /// **A node that is not laid out is not hit-testable**, and that includes
    /// the root. The descent below tests a node's *children* against
    /// `out_of_layout` and never the node it starts from, so the root was the
    /// one node nothing asked about — and a folded root keeps the viewport
    /// while everything under it is solved as usual, since a fold takes its
    /// extent out of its *parent* and the root has none. The pointer therefore
    /// went on resolving to bays and dividers on a panel drawing nothing at
    /// all, [`visible`](Layout::visible) having answered no for every one of
    /// them. It is
    /// [P-0073](../../../docs/principles/0073-a-node-claims-only-what-its-visible-content-can-use.md)
    /// and [ADR-0193](../../../docs/adr/0193-a-region-that-is-not-laid-out-declares-nothing-rather-than-being-dropped-later.md)
    /// on the pointer axis: a region that is not laid out claims no space,
    /// declares no staleness, and is under nobody's pointer.
    ///
    /// The way back does not go through here — [`expand`](Layout::expand) and
    /// a solo's undo take a node or no argument at all, and a folded region
    /// has no rectangle for a pointer to reach anyway, which the manual says
    /// outright.
    pub fn hit(&self, p: Point, grab: f32) -> Hit {
        debug_assert!(!self.dirty, "hit() read a stale solve; call solve() first");
        if !self.viewport.contains(p) {
            return Hit::Nothing;
        }
        // Only the root: every other node is reached through the filter in the
        // descent, so this is the whole of *the node reported is laid out*.
        if self.arrangement.out_of_layout(self.arrangement.root.0) {
            return Hit::Nothing;
        }
        let mut cur = self.arrangement.root.0;
        loop {
            let Some((axis, _)) = self.arrangement.split_of(cur) else {
                return Hit::View(NodeId(cur));
            };
            if let Some(index) = self.divider_at(cur, axis, p, grab) {
                return Hit::Divider {
                    split: NodeId(cur),
                    index,
                };
            }
            let mut next = None;
            for k in 0..self.arrangement.child_count(cur) {
                let c = self.arrangement.child(cur, k);
                if !self.arrangement.out_of_layout(c) && self.solved.rects[c].contains(p) {
                    next = Some(c);
                    break;
                }
            }
            match next {
                Some(c) => cur = c,
                // Inside the split but claimed by no child: only reachable at a
                // rounding-width seam, and a seam is nothing rather than a
                // guess.
                None => return Hit::Nothing,
            }
        }
    }

    /// The index of the divider `p` grabs, if any. The gap is read from the
    /// solved rectangles rather than recomputed, so it is exactly the gap that
    /// was drawn.
    fn divider_at(&self, split: usize, axis: Axis, p: Point, grab: f32) -> Option<usize> {
        let along = axis.coord(p);
        let mut index = 0;
        let mut prev: Option<usize> = None;
        for k in 0..self.arrangement.child_count(split) {
            let c = self.arrangement.child(split, k);
            if self.arrangement.out_of_layout(c) {
                continue;
            }
            if let Some(a) = prev {
                let near = axis.far(self.solved.rects[a]);
                let far = axis.origin(self.solved.rects[c]);
                if along >= near - grab && along <= far + grab {
                    return Some(index);
                }
                index += 1;
            }
            prev = Some(c);
        }
        None
    }

    // -- arena access ----------------------------------------------------

    fn node(&self, i: usize) -> &Node {
        &self.arrangement.nodes[i]
    }

    /// The only way to write a node, and every caller of it is an operation —
    /// a fold, a solo or a drag. The solve has no access to this, by
    /// construction rather than by convention: see [`Layout::solve`].
    fn node_mut(&mut self, i: usize) -> &mut Node {
        &mut self.arrangement.nodes[i]
    }
}

impl Arrangement {
    /// The axis and declared divider of a split, or `None` for a view. Returns
    /// by value so a caller can hold it across a write to another field.
    fn split_of(&self, i: usize) -> Option<(Axis, f32)> {
        match &self.nodes[i].kind {
            Kind::Split { axis, divider, .. } => Some((*axis, *divider)),
            Kind::View { .. } => None,
        }
    }

    fn child_count(&self, i: usize) -> usize {
        match &self.nodes[i].kind {
            Kind::Split { children, .. } => children.len(),
            Kind::View { .. } => 0,
        }
    }

    fn child(&self, i: usize, k: usize) -> usize {
        match &self.nodes[i].kind {
            Kind::Split { children, .. } => children[k].0,
            Kind::View { .. } => unreachable!("a view has no children"),
        }
    }

    /// Whether node `i` is out of the layout on its own account: folded by
    /// the operator, or set aside by whoever is drawing.
    ///
    /// **The one place the disjunction is written**, so that *should this be
    /// laid out* is one question with one answer rather than a condition every
    /// reader assembles for itself — which is how the two would come apart,
    /// one reader at a time, the first time a third reason was added or a bit
    /// was renamed. Nothing below reads either flag directly.
    fn out_of_layout(&self, i: usize) -> bool {
        self.nodes[i].collapsed || self.nodes[i].aside
    }

    fn visible_count(&self, i: usize) -> usize {
        (0..self.child_count(i))
            .filter(|k| !self.out_of_layout(self.child(i, *k)))
            .count()
    }

    /// The `nth` visible child of a split, which is what a divider index and
    /// [`Layout::set_divider`] count in.
    fn visible_child(&self, i: usize, nth: usize) -> Option<usize> {
        (0..self.child_count(i))
            .map(|k| self.child(i, k))
            .filter(|c| !self.out_of_layout(*c))
            .nth(nth)
    }

    /// The divider thickness actually used, which is the declared one until the
    /// split is too narrow to hold that many. Shrinking the dividers rather
    /// than overflowing keeps children tiling their parent at every extent,
    /// including zero.
    fn effective_divider(&self, split: usize, extent: f32) -> f32 {
        let Some((_, divider)) = self.split_of(split) else {
            return 0.0;
        };
        let gaps = self.visible_count(split).saturating_sub(1) as f32;
        if gaps > 0.0 && divider * gaps > extent {
            extent / gaps
        } else {
            divider
        }
    }

    /// What is left for the children once the dividers between them are taken
    /// out. Never negative.
    fn avail(&self, split: usize, extent: f32) -> f32 {
        let gaps = self.visible_count(split).saturating_sub(1) as f32;
        (extent - self.effective_divider(split, extent) * gaps).max(0.0)
    }

    /// Whether node `i` survives a solo on `kept`: it is `kept`, an ancestor of
    /// it, or inside it.
    fn on_solo_path(&self, i: usize, kept: NodeId) -> bool {
        self.is_ancestor(i, kept.0) || self.is_ancestor(kept.0, i)
    }

    /// Whether `a` is `b` or an ancestor of it.
    fn is_ancestor(&self, a: usize, b: usize) -> bool {
        let mut cur = Some(NodeId(b));
        while let Some(NodeId(i)) = cur {
            if i == a {
                return true;
            }
            cur = self.nodes[i].parent;
        }
        false
    }
}

/// How much extent node `i` **can use** along its parent's axis, written into
/// `s.usable` for every node and returned for the caller that is summing it.
/// `parent` is the axis the node is being laid out along, and `None` for the
/// root, which is the viewport and is never asked.
///
/// **This is the one pass that runs bottom-up, and it still writes nothing
/// into the arrangement.** It takes `&Arrangement` exactly as the rest of the
/// solve does and puts its answer in the [`Solved`] buffers, so P-0071 is
/// enforced here by the same borrow that enforces it everywhere else — the
/// direction of the pass is not the direction of the writes. Said explicitly
/// because a bottom-up pass is the shape a reader expects to see mutating
/// nodes, and this one cannot.
///
/// The rule, which is short:
///
/// - a [`Sizing::Flex`] leaf can use any amount at all;
/// - a [`Sizing::Fixed`] leaf can use exactly the size it stores;
/// - a split can use the sum of its **visible** children's, plus the dividers
///   that would go between them — infinite if any of them is, and **zero when
///   none of them is visible**, since a split showing nothing needs no room to
///   show it in;
/// - and a node's own `max` caps all of the above, which is also what carries
///   a maximum stated deep in a subtree up to the split that hands out the
///   extent.
///
/// **A split laid out across its parent's axis is unbounded rather than a
/// sum.** Every constraint inside it is stated along *its* axis, so its
/// children's sizes are heights where the parent is handing out widths and
/// adding them up would be arithmetic on two different questions. This crate
/// measures one axis per node, so the honest answer there is "no cap", which
/// is what `f32::INFINITY` says. The `visible` count is still consulted
/// first: nothing is drawn inside a fully folded split whichever way it lays
/// its children out.
fn measure(a: &Arrangement, s: &mut Solved, i: usize, parent: Option<Axis>) -> f32 {
    let content = match a.split_of(i) {
        None => match a.nodes[i].sizing {
            Sizing::Fixed(size) => size.max(0.0),
            Sizing::Flex(_) => f32::INFINITY,
        },
        Some((axis, divider)) => {
            let mut sum = 0.0;
            let mut visible = 0usize;
            for k in 0..a.child_count(i) {
                let c = a.child(i, k);
                let child = measure(a, s, c, Some(axis));
                if a.out_of_layout(c) {
                    continue;
                }
                sum += child;
                visible += 1;
            }
            let gaps = visible.saturating_sub(1) as f32;
            if visible == 0 {
                0.0
            } else if parent == Some(axis) {
                sum + divider.max(0.0) * gaps
            } else {
                f32::INFINITY
            }
        }
    };
    // `min` before `max` so that a NaN in either bound leaves the other one
    // holding, and so nothing negative ever reaches a claim.
    let usable = content.min(a.nodes[i].max).max(0.0);
    s.usable[i] = usable;
    usable
}

/// What child `c` of a split claims of its parent's extent: the size it
/// stores, or what it can use, whichever is smaller.
///
/// Both halves of the cap are this one sentence — see the minimum in
/// [`solve_split`], which is capped the same way and for the same reason.
fn claim(s: &Solved, c: usize, size: f32) -> f32 {
    size.max(0.0).min(s.usable[c])
}

/// The solve, as a function of the arrangement rather than a method on it.
///
/// `a` is shared and `s` is exclusive, which is the whole enforcement of
/// P-0071: there is no path from here to a mutable node.
fn solve_subtree(a: &Arrangement, s: &mut Solved, i: usize) {
    if a.split_of(i).is_none() {
        return;
    }
    solve_split(a, s, i);
    // The scratch is finished with by now, which is what lets one buffer
    // serve the whole recursion.
    for k in 0..a.child_count(i) {
        let c = a.child(i, k);
        solve_subtree(a, s, c);
    }
}

fn solve_split(a: &Arrangement, s: &mut Solved, split: usize) {
    let Some((axis, _)) = a.split_of(split) else {
        return;
    };
    let rect = s.rects[split];
    let extent = axis.extent(rect).max(0.0);
    let n = a.child_count(split);
    let divider = a.effective_divider(split, extent);
    let avail = a.avail(split, extent);

    for k in 0..n {
        let c = a.child(split, k);
        s.sizes[k] = 0.0;
        s.frozen[k] = a.out_of_layout(c);
    }

    // Step 4. One pass per child is enough, since every pass but the last
    // freezes one; the `+ 1` is the pass that settles and breaks.
    for _ in 0..=n {
        let mut frozen_sum = 0.0;
        let mut claimed = 0.0;
        let mut stored = 0.0;
        let mut weight = 0.0;
        for k in 0..n {
            if s.frozen[k] {
                frozen_sum += s.sizes[k];
                continue;
            }
            let c = a.child(split, k);
            match a.nodes[c].sizing {
                Sizing::Fixed(size) => {
                    claimed += claim(s, c, size);
                    stored += size.max(0.0);
                }
                Sizing::Flex(w) => weight += w.max(0.0),
            }
        }

        if weight > 0.0 {
            let pool = (avail - frozen_sum - claimed).max(0.0);
            for k in 0..n {
                if s.frozen[k] {
                    continue;
                }
                let c = a.child(split, k);
                let size = match a.nodes[c].sizing {
                    Sizing::Fixed(size) => claim(s, c, size),
                    Sizing::Flex(w) => pool * w.max(0.0) / weight,
                };
                s.sizes[k] = size;
            }
        } else {
            // Nothing flexible is left unfrozen, so the fixed children take
            // the discrepancy in proportion — in both directions, because a
            // split that came up short would otherwise leave a gap its
            // parent has no other child to fill.
            //
            // **This is the one place the claim is the stored size rather
            // than the capped one, and the reason is that same gap.** A cap
            // says what a node would *ask* for; here nobody is asking, and
            // the split is disposing of space no child claimed. Scaling the
            // capped claims instead would divide it more sensibly right up
            // until every unfrozen child's claim is zero — a split whose
            // children are all folded away is exactly that — and then there
            // is nothing to scale, the pool goes nowhere, and the hole in the
            // middle of the parent is permanent until something is unfolded.
            // So the cap is on what a node claims, and this branch is what
            // happens to what nobody claimed: only a `max` stops a child
            // growing here, which is ADR-0157 unchanged.
            let pool = (avail - frozen_sum).max(0.0);
            let scale = if stored > 0.0 { pool / stored } else { 0.0 };
            for k in 0..n {
                if s.frozen[k] {
                    continue;
                }
                let c = a.child(split, k);
                let size = match a.nodes[c].sizing {
                    Sizing::Fixed(size) => size.max(0.0) * scale,
                    Sizing::Flex(_) => 0.0,
                };
                s.sizes[k] = size;
            }
        }

        let mut bounded = false;
        for k in 0..n {
            if s.frozen[k] {
                continue;
            }
            let c = a.child(split, k);
            // The declared minimum, capped by what the child can use — the
            // same sentence as the claim above. A node that says it needs 200
            // is saying so while it has 200 worth of content; with only a
            // 72-tall row left visible inside it, holding 200 is holding
            // space it will leave empty.
            let min = a.nodes[c].min.max(0.0).min(s.usable[c]);
            let max = a.nodes[c].max;
            if s.sizes[k] < min {
                s.sizes[k] = min;
                s.frozen[k] = true;
                bounded = true;
            } else if s.sizes[k] > max {
                s.sizes[k] = max;
                s.frozen[k] = true;
                bounded = true;
            }
        }
        if !bounded {
            break;
        }
    }

    // Step 5.
    let mut total = 0.0;
    for k in 0..n {
        total += s.sizes[k];
    }
    if total > avail {
        let scale = if total > 0.0 { avail / total } else { 0.0 };
        for k in 0..n {
            s.sizes[k] *= scale;
        }
    }

    let mut cursor = axis.origin(rect);
    let mut placed = 0;
    let visible = a.visible_count(split);
    for k in 0..n {
        let c = a.child(split, k);
        s.rects[c] = axis.slice(rect, cursor, s.sizes[k].max(0.0));
        cursor += s.sizes[k].max(0.0);
        if !a.out_of_layout(c) {
            placed += 1;
            if placed < visible {
                cursor += divider;
            }
        }
    }
}

/// Flatten a [`Spec`] into the arena, parent before children so that a name
/// resolves in declaration order — which matters only for the message
/// [`Layout::new`] refuses a duplicate with, since after that check there is at
/// most one node per name.
fn build(nodes: &mut Vec<Node>, spec: Spec, parent: Option<NodeId>) -> NodeId {
    let (kind, sizing, min, max, collapsed, children) = match spec {
        Spec::View {
            name,
            sizing,
            min,
            max,
            collapsed,
        } => (Kind::View { name }, sizing, min, max, collapsed, Vec::new()),
        Spec::Split {
            name,
            axis,
            divider,
            children,
            sizing,
            min,
            max,
            collapsed,
        } => (
            Kind::Split {
                name,
                axis,
                divider,
                children: Vec::new(),
            },
            sizing,
            min,
            max,
            collapsed,
            children,
        ),
    };
    let id = NodeId(nodes.len());
    nodes.push(Node {
        kind,
        sizing,
        min,
        max,
        collapsed,
        // Nothing a `Spec` can say sets this: it is not the arrangement's, it
        // is the caller's, and it is stated per frame rather than declared.
        aside: false,
        parent,
    });
    let built: Vec<NodeId> = children
        .into_iter()
        .map(|c| build(nodes, c, Some(id)))
        .collect();
    if let Kind::Split { children, .. } = &mut nodes[id.0].kind {
        *children = built;
    }
    id
}
