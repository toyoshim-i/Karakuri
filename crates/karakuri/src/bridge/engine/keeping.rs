use super::*;

/// What each deck is running, as the nodes a Set file names.
pub(crate) struct Playing {
    pub(crate) playing: Vec<Option<Vec<setfile::SavedNode>>>,
}

impl Playing {
    /// Every slot seeded from the material this run compiled, addressed by the
    /// bytes that compile read.
    pub(crate) fn at_launch(
        placed: &[karakuri_environment::compile::Placed],
        slots: usize,
    ) -> Playing {
        let nodes: Vec<setfile::SavedNode> = placed
            .iter()
            .map(|node| setfile::SavedNode {
                layer: setfile::kind_name(node.layer),
                index: node.index,
                hash: node.hash(),
                name: node.named.name.clone(),
                source: Some(std::sync::Arc::clone(&node.source)),
                meta: Some(std::sync::Arc::clone(&node.meta)),
            })
            .collect();
        Playing {
            playing: (0..slots)
                .map(|_| match nodes.is_empty() {
                    true => None,
                    false => Some(nodes.iter().map(copied).collect()),
                })
                .collect(),
        }
    }

    /// What `slot` is running, or `None` for a slot with no address to name.
    pub(crate) fn at(&self, slot: usize) -> Option<&Vec<setfile::SavedNode>> {
        self.playing.get(slot).and_then(Option::as_ref)
    }

    /// A build landed. `nodes` is `None` when that build's sources never reached
    /// the store.
    pub(crate) fn landed(&mut self, slot: usize, nodes: Option<Vec<setfile::SavedNode>>) {
        if slot >= self.playing.len() {
            return;
        }
        self.playing[slot] = nodes;
    }
}

/// One saved node copied.
pub(crate) fn copied(node: &setfile::SavedNode) -> setfile::SavedNode {
    setfile::SavedNode {
        layer: node.layer,
        index: node.index,
        hash: node.hash,
        name: node.name.clone(),
        source: node.source.clone(),
        meta: node.meta.clone(),
    }
}

/// What a build that just landed is running, from the addresses the watcher
/// reported and the names the slot is spelled with.
pub(crate) fn built_nodes(built: &watch::Built, at: &watch::Aim) -> Vec<setfile::SavedNode> {
    let names: Vec<Option<String>> = std::iter::once(at.head.name.clone())
        .chain(at.rest.iter().map(|node| node.name.clone()))
        .collect();
    built
        .nodes
        .iter()
        .enumerate()
        .map(|(node, (layer, index, hash))| setfile::SavedNode {
            layer,
            index: *index,
            hash: *hash,
            name: names.get(node).cloned().flatten(),
            source: None,
            meta: None,
        })
        .collect()
}

/// What a Set file says about the Set that is playing, read off that Set.
pub(crate) fn playing_values(
    set: &karakuri_engine::Set,
    edges: &[karakuri_engine::set::Edge],
) -> setfile::Owned {
    setfile::Owned {
        nodes: Vec::new(),
        capacities: set.source_capacities(),
        params: set
            .params()
            .map(|(layer, index, key, value)| {
                karakuri_engine::ParamWrite::at(layer, index, key, value)
            })
            .collect(),
        bindings: set.bindings().to_vec(),
        edges: edges.to_vec(),
        camera: set.orbit(),
        layering: set.layering(),
        live: selected_renderer(set.inputs()),
        seeds: set.source_salts().to_vec(),
    }
}

/// Which renderer a Set is folded to.
pub(crate) fn selected_renderer(inputs: &[karakuri_engine::mix::Input]) -> Option<u32> {
    if inputs.iter().all(|input| input.live) {
        return None;
    }
    let mut live = inputs.iter().enumerate().filter(|(_, input)| input.live);
    match (live.next(), live.next()) {
        (Some((at, _)), None) => Some(at as u32),
        _ => None,
    }
}

/// Whether this deck holds `slot`.
pub(crate) fn slot_in_range(slot: usize, slot_count: usize) -> bool {
    u8::try_from(slot)
        .ok()
        .is_some_and(|slot| DeckSlot::new(slot, slot_count).is_some())
}

/// One live save, from the frame that asked for it to the file on disk.
pub(crate) struct Save {
    pub(crate) slot: usize,
    pub(crate) asked: Asked,
    pub(crate) id: String,
    pub(crate) root: std::path::PathBuf,
    pub(crate) sources: setfile::Sources,
    pub(crate) values: setfile::Owned,
}

impl Save {
    /// Write it off the render thread.
    pub(crate) fn run(self) -> Result<(), String> {
        let Save {
            asked,
            id,
            root,
            sources,
            mut values,
            ..
        } = self;
        let store = Store::open(&root).map_err(|e| format!("store `{}`: {e}", root.display()))?;
        values.nodes = sources.into_nodes(&store)?;
        setfile::save(&store, asked, &id, values.saving())
    }
}

/// What a live save came back with, at the frame it arrives.
pub(crate) struct Saved {
    pub(crate) slot: usize,
    pub(crate) asked: Asked,
    pub(crate) id: String,
    pub(crate) outcome: Result<(), String>,
    pub(crate) reply: Option<mcp::Reply>,
}

/// One node's procedure kept.
pub(crate) struct Kept {
    pub(crate) asked: Asked,
    pub(crate) name: String,
    pub(crate) root: std::path::PathBuf,
    pub(crate) source: Option<std::sync::Arc<str>>,
    pub(crate) hash: karakuri_store::hash::Hash,
    pub(crate) addr: String,
    pub(crate) outcome: Result<std::path::PathBuf, String>,
    pub(crate) reply: Option<mcp::Reply>,
}

impl Kept {
    /// Write it off the render thread.
    pub(crate) fn run(self) -> Kept {
        let Kept {
            asked,
            name,
            root,
            source,
            hash,
            addr,
            reply,
            ..
        } = self;
        let outcome = (|| {
            let store =
                Store::open(&root).map_err(|e| format!("store `{}`: {e}", root.display()))?;
            let bytes = match &source {
                Some(source) => source.as_bytes().to_vec(),
                None => store
                    .get_artifact(&hash)
                    .map_err(|e| format!("the source of `{addr}`: {e}"))?,
            };
            match asked {
                Asked::Operator => store.write_procedure(&name, &bytes),
                Asked::Model => store.write_sandbox_procedure(&name, &bytes),
            }
            .map_err(|e| e.to_string())
        })();
        Kept {
            asked,
            name,
            root,
            source,
            hash,
            addr,
            outcome,
            reply,
        }
    }

    /// The sentence describing this outcome.
    pub(crate) fn said(&self) -> Result<String, String> {
        match &self.outcome {
            Ok(path) => Ok(match self.asked {
                Asked::Operator => format!(
                    "  keep: {} kept as procedure `{}` — `{}`. The Library bay lists it, and a \
                     press on that row writes it over that layer of what a deck is playing",
                    self.addr,
                    self.name,
                    path.display()
                ),
                Asked::Model => format!(
                    "  keep: {} kept as `{}` in the sandbox — `{}`. A keep asked for over MCP is \
                     written there rather than in the operator's library, so the Library bay does \
                     not list it and no load off a row reaches it",
                    self.addr,
                    self.name,
                    path.display()
                ),
            }),
            Err(e) => Err(format!(
                "  keep: {}: procedure `{}` was not kept: {e}",
                self.addr, self.name
            )),
        }
    }
}

/// What a send came back with, at the frame it arrives.
pub(crate) struct Sent {
    pub(crate) id: String,
    pub(crate) to: Option<std::path::PathBuf>,
    pub(crate) outcome: Result<(), String>,
}

impl Sent {
    /// The sentence describing this outcome.
    pub(crate) fn said(&self) -> String {
        let Sent { id, to, outcome } = self;
        match (to, outcome) {
            (None, _) => format!(
                "  send: `{id}` was not written — the save dialog was dismissed, and nothing was \
                 asked of the disk"
            ),
            (Some(to), Ok(())) => format!("  send: `{id}` written to `{}`", to.display()),
            (Some(to), Err(e)) => {
                format!("  send: `{id}` was not written to `{}`: {e}", to.display())
            }
        }
    }
}

/// A save that will not happen, to the terminal and to whoever asked.
pub(crate) fn refused(reply: Option<mcp::Reply>, said: String) {
    println!("{said}");
    if let Some(reply) = reply {
        reply.settled(Err(said));
    }
}
