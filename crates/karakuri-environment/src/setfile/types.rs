use karakuri_engine::camera::Orbit;
use karakuri_engine::set::Layering;
use karakuri_engine::{Binding, ParamWrite};
use karakuri_ir::typed::Checked;
use karakuri_ir::Kind;
use karakuri_store::hash::Hash;

use crate::compile::Names;

/// The Set file format version this build writes. One number for the whole
/// file, on `Record::Set`.
pub const VERSION: u32 = 1;

/// The octave count an `fbm` binding gets when it does not say. Matches
/// `karakuri-store`'s `BindNoise` default, which is the record this stands for.
pub const DEFAULT_OCTAVES: u32 = 4;

/// A loaded node in a Set: layer kind, slot index within layer, checked IR, source text, and optional name.
#[derive(Clone, Debug)]
pub struct LoadedNode {
    pub layer: Kind,
    pub index: u32,
    pub checked: Checked,
    pub src: String,
    pub name: Option<String>,
}

/// Set definition decoded into an ordered node collection and playback parameters.
#[cfg_attr(test, derive(Debug))]
pub struct Loaded {
    pub id: String,
    /// The decoded nodes in canonical execution order.
    pub nodes: Vec<LoadedNode>,
    /// Vertex/point capacities per geometry slot, or `None` if engine default.
    pub capacities: Vec<Option<u32>>,
    pub params: Vec<ParamWrite>,
    pub bindings: Vec<Binding>,
    /// Which node fills each declared input slot, as the file recorded it.
    pub edges: Vec<karakuri_engine::set::Edge>,
    pub camera: Option<Orbit>,
    /// Layering mode: [`Layering::Composite`] if `merge` record present, else [`Layering::Overdraw`].
    pub layering: Layering,
    /// Active solo renderer index under compositing, or `None` if all live or overdrawing.
    pub live: Option<u32>,
    /// Per-geometry salt seeds, or `None` if derived from Set seed and ordinal.
    pub salts: Vec<Option<u32>>,
    /// Non-fatal decoding diagnostics and unsupported feature notices.
    pub notes: Vec<String>,
}

impl Loaded {
    /// Returns node names in canonical execution order, padded with `None` where unnamed.
    pub fn node_names(&self) -> impl Iterator<Item = Option<String>> + '_ {
        self.nodes.iter().map(|n| n.name.clone())
    }

    /// Returns all nodes as `(&Checked, &str)` pairs in canonical execution order.
    pub fn nodes(&self) -> impl Iterator<Item = (&Checked, &str)> {
        self.nodes.iter().map(|n| (&n.checked, n.src.as_str()))
    }

    /// Returns procedure sources in node order.
    pub fn srcs(&self) -> Vec<String> {
        self.nodes.iter().map(|n| n.src.clone()).collect()
    }

    /// Returns references to all checked procedures for the specified layer.
    pub fn layer_checked(&self, kind: Kind) -> impl Iterator<Item = &Checked> {
        self.nodes
            .iter()
            .filter(move |n| n.layer == kind)
            .map(|n| &n.checked)
    }

    pub fn l1s(&self) -> Vec<&Checked> {
        self.layer_checked(Kind::L1).collect()
    }
    pub fn l2s(&self) -> Vec<&Checked> {
        self.layer_checked(Kind::L2).collect()
    }
    pub fn l3s(&self) -> Vec<&Checked> {
        self.layer_checked(Kind::L3).collect()
    }
    pub fn fields(&self) -> Vec<&Checked> {
        self.layer_checked(Kind::Field).collect()
    }
    pub fn l4s(&self) -> Vec<&Checked> {
        self.layer_checked(Kind::L4).collect()
    }
    pub fn l5s(&self) -> Vec<&Checked> {
        self.layer_checked(Kind::L5).collect()
    }

    /// Reconstructs [`Names`] partitioned by layer.
    pub fn names(&self) -> Names {
        let mut names = Names::default();
        for node in &self.nodes {
            match node.layer {
                Kind::L1 => names.l1s.push(node.name.clone()),
                Kind::L2 => names.l2s.push(node.name.clone()),
                Kind::L3 => names.l3s.push(node.name.clone()),
                Kind::L4 => names.l4s.push(node.name.clone()),
                Kind::Field => names.fields.push(node.name.clone()),
                Kind::L5 => names.l5s.push(node.name.clone()),
            }
        }
        names
    }

    /// Creates the slot's [`crate::watch::Aim`], writing working copies to scratch storage.
    pub fn aim(
        &self,
        root: &std::path::Path,
        slot: usize,
        salt: u32,
    ) -> Result<crate::watch::Aim, String> {
        let mut named = Vec::with_capacity(self.nodes.len());
        for (at, node) in self.nodes.iter().enumerate() {
            let path = crate::scratch::place(
                root,
                &crate::scratch::node_name(slot, at, &node.checked.name),
                &node.src,
            )?;
            named.push(crate::compile::Named {
                name: node.name.clone(),
                path,
            });
        }
        let mut named = named.into_iter();
        let head = named
            .next()
            .ok_or_else(|| format!("`{}` names no procedure at all", self.id))?;

        let seed_salt = self.salts.first().copied().flatten().unwrap_or(salt);
        let l1_count = self.nodes.iter().filter(|n| n.layer == Kind::L1).count();
        let salts: Vec<u32> = (0..l1_count)
            .map(|at| {
                self.salts
                    .get(at)
                    .copied()
                    .flatten()
                    .unwrap_or_else(|| karakuri_engine::set::derived_salt(seed_salt, at))
            })
            .collect();

        Ok(crate::watch::Aim {
            head,
            rest: named.collect(),
            layering: self.layering,
            live: self.live,
            capacity: self.capacities.first().copied().flatten(),
            seed_salt,
            salts,
            camera: self.camera.unwrap_or_default(),
            overrides: self.params.clone(),
            published: Vec::new(),
            bindings: self.bindings.clone(),
            edges: self.edges.clone(),
            authorities: Vec::new(),
            set: Some(self.id.clone()),
        })
    }
}

/// Persisted node descriptor specifying artifact hash, layer kind, slot index, and optional name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// The store's address for this node's source. Already `put`, because a hash
    /// nothing has stored is a Set file that does not load.
    pub hash: Hash,
    pub layer: Kind,
    /// Which node of that layer, numbered from 0 with no gaps. Index is position:
    /// the L2s deform in it and the L4s draw in it.
    pub index: u32,
    /// Custom name assigned to the node, or `None`.
    pub name: Option<String>,
}

impl crate::compile::Placed {
    /// This node as [`Node`]. No store and no disk — the address comes off the
    /// bytes the compile read.
    pub fn node(&self) -> Node {
        Node {
            hash: self.hash(),
            layer: self.layer,
            index: self.index,
            name: self.named.name.clone(),
        }
    }
}

/// Everything a Set file records, gathered so [`save`] takes one argument for
/// the Set rather than nine for its parts. The fields are the records, in the
/// order they are written.
pub struct Saving<'a> {
    /// Every node of the Set, in any order — [`save`] writes them by layer and
    /// index, so the file is the same bytes however the caller gathered them.
    pub nodes: &'a [Node],
    /// Capacity overrides per geometry slot in index order.
    pub capacities: &'a [u32],
    pub params: &'a [ParamWrite],
    pub bindings: &'a [Binding],
    /// Which node fills each declared input slot, exactly as the run was wired.
    /// Empty for a Set no node of which takes a second geometry, which is most of
    /// them.
    pub edges: &'a [karakuri_engine::set::Edge],
    /// Active orbit camera state at save time.
    pub camera: &'a Orbit,
    /// Whether this Set composites its renderers or overdraws them.
    ///
    /// [`Layering::Composite`] serializes as a `merge` record; [`Layering::Overdraw`]
    /// is the default and omits the record.
    pub layering: Layering,
    /// Solo renderer index under composite layering, or `None`.
    pub live: Option<u32>,
    /// Salt seeds per geometry slot in index order.
    pub seeds: &'a [u32],
}

/// Thread-safe, fully owned representation of [`Saving`] for cross-thread persistence.
pub struct Owned {
    pub nodes: Vec<Node>,
    pub capacities: Vec<u32>,
    pub params: Vec<ParamWrite>,
    pub bindings: Vec<Binding>,
    pub edges: Vec<karakuri_engine::set::Edge>,
    pub camera: Orbit,
    /// See [`Saving::layering`].
    pub layering: Layering,
    /// See [`Saving::live`].
    pub live: Option<u32>,
    pub seeds: Vec<u32>,
}

impl Owned {
    /// What [`save`] takes, borrowed out of this.
    pub fn saving(&self) -> Saving<'_> {
        Saving {
            nodes: &self.nodes,
            capacities: &self.capacities,
            params: &self.params,
            bindings: &self.bindings,
            edges: &self.edges,
            camera: &self.camera,
            layering: self.layering,
            live: self.live,
            seeds: &self.seeds,
        }
    }
}
