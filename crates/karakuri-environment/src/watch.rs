//! File watching and hot-reloading for slot `.kir` sources.
//!
//! Runs on the build worker thread to poll file changes, validate ASTs,
//! compile shaders, and trigger hot-swaps without stalling the render thread.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use karakuri_engine::swap::{Polled, Refusal};
use karakuri_engine::{Binding, Request, Source};

use crate::compile;

/// Polling interval for filesystem change detection.
const INTERVAL: Duration = Duration::from_millis(100);

/// Built slot metadata and artifact content hashes for session recording.
pub struct Built {
    /// The id carried on every `swap::Event` about this build: `(slot << 32) | n`.
    pub id: u64,
    /// Nodes of the rebuilt slot with their procedure hashes.
    pub nodes: Vec<(&'static str, u32, karakuri_store::hash::Hash)>,
}

/// Target configuration for re-pointing a running slot [`Watch`].
pub struct Aim {
    /// Primary source file.
    pub head: crate::compile::Named,
    /// Additional sources in chain order.
    pub rest: Vec<crate::compile::Named>,
    pub layering: karakuri_engine::set::Layering,
    pub live: Option<u32>,
    pub capacity: Option<u32>,
    pub seed_salt: u32,
    pub salts: Vec<u32>,
    pub camera: karakuri_engine::camera::Orbit,
    pub overrides: Vec<karakuri_engine::ParamWrite>,
    pub published: Vec<karakuri_engine::set::Published>,
    pub bindings: Vec<Binding>,
    pub edges: Vec<karakuri_engine::set::Edge>,
    pub authorities: Vec<karakuri_engine::swap::AuthorityAt>,
    /// Active Set ID for version history filing, or `None`.
    pub set: Option<String>,
}

pub struct Watch {
    /// Number of builds requested by this watcher.
    builds: u64,
    /// Storage destination and notification channel for completed builds.
    stored: Option<(
        std::sync::Arc<karakuri_store::store::Store>,
        std::sync::mpsc::Sender<Built>,
    )>,
    /// Slot index being watched.
    slot: usize,
    /// Primary source file.
    head: crate::compile::Named,
    /// Additional source files.
    rest: Vec<crate::compile::Named>,
    /// Slot layering mode (composite vs overdraw).
    layering: karakuri_engine::set::Layering,
    /// Active folded renderer index, or `None`.
    live: Option<u32>,
    /// User-specified capacity override.
    capacity: Option<u32>,
    seed_salt: u32,
    /// Per-geometry hash salts across rebuilds.
    salts: Vec<u32>,
    /// Active camera orbit parameters.
    camera: karakuri_engine::camera::Orbit,
    /// Initial parameter overrides for the aim build.
    overrides: Vec<karakuri_engine::ParamWrite>,
    /// Whether aim parameter overrides have been applied.
    stated: bool,
    /// Published parameters for external modulation.
    published: Vec<karakuri_engine::set::Published>,
    /// Parameter modulation bindings.
    bindings: Vec<Binding>,
    /// Declared DAG input edges.
    edges: Vec<karakuri_engine::set::Edge>,
    /// Authority grants for node modification permissions across rebuilds.
    authorities: Vec<karakuri_engine::swap::AuthorityAt>,
    /// File content hashes from the previous poll.
    stamps: Vec<Option<u64>>,
    /// Debounce flag indicating pending file modifications.
    settling: bool,
    /// Receiver for external re-aim requests.
    aimed: Option<Receiver<Aim>>,
    /// History snapshot recorder and optional Set ID for version tracking (ADR-0316).
    snapshots: Option<(crate::history::Shared, Option<String>)>,
}

impl Watch {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        slot: usize,
        head: crate::compile::Named,
        rest: Vec<crate::compile::Named>,
        layering: karakuri_engine::set::Layering,
        live: Option<u32>,
        capacity: Option<u32>,
        seed_salt: u32,
        salts: Vec<u32>,
        camera: karakuri_engine::camera::Orbit,
        overrides: Vec<karakuri_engine::ParamWrite>,
        published: Vec<karakuri_engine::set::Published>,
        bindings: Vec<Binding>,
        edges: Vec<karakuri_engine::set::Edge>,
        authorities: Vec<karakuri_engine::swap::AuthorityAt>,
    ) -> Watch {
        let mut watch = Watch {
            builds: 0,
            stored: None,
            aimed: None,
            snapshots: None,
            slot,
            head,
            rest,
            layering,
            live,
            capacity,
            seed_salt,
            salts,
            camera,
            overrides,
            stated: true,
            published,
            bindings,
            edges,
            authorities,
            stamps: Vec::new(),
            settling: false,
        };
        watch.stamps = watch.stamp();
        watch
    }

    fn stamp(&self) -> Vec<Option<u64>> {
        let digest = |path: &PathBuf| {
            std::fs::read(path).ok().map(|bytes| {
                let mut h = DefaultHasher::new();
                bytes.hash(&mut h);
                h.finish()
            })
        };
        std::iter::once(digest(&self.head.path))
            .chain(self.rest.iter().map(|n| digest(&n.path)))
            .collect()
    }
}

impl Watch {
    /// Configures artifact storage and notification channel for completed builds.
    pub fn storing_to(
        mut self,
        store: std::sync::Arc<karakuri_store::store::Store>,
        tx: std::sync::mpsc::Sender<Built>,
    ) -> Watch {
        self.stored = Some((store, tx));
        self
    }

    /// Configures version snapshot recording for compiling builds.
    pub fn snapshotting_to(
        mut self,
        snapshots: crate::history::Shared,
        set: Option<String>,
    ) -> Watch {
        self.snapshots = Some((snapshots, set));
        self
    }

    /// Attaches a receiver to accept re-point requests for slot hot-reloading.
    pub fn aimed_by(mut self, rx: Receiver<Aim>) -> Watch {
        self.aimed = Some(rx);
        self
    }
}

impl Source for Watch {
    fn poll(&mut self) -> Option<Polled> {
        std::thread::sleep(INTERVAL);

        // Immediate rebuild upon re-aim without debounce delay.
        if self.repointed() {
            return self.rebuild();
        }

        let stamps = self.stamp();
        if stamps != self.stamps {
            self.stamps = stamps;
            self.settling = true;
            return None;
        }
        if !self.settling {
            return None;
        }
        self.settling = false;
        self.rebuild()
    }
}

impl Watch {
    /// Drains the aim channel and applies the newest target configuration if available.
    fn repointed(&mut self) -> bool {
        let Some(rx) = &self.aimed else {
            return false;
        };
        let mut aim = None;
        while let Ok(next) = rx.try_recv() {
            aim = Some(next);
        }
        let Some(aim) = aim else {
            return false;
        };
        let Aim {
            head,
            rest,
            layering,
            live,
            capacity,
            seed_salt,
            salts,
            camera,
            overrides,
            published,
            bindings,
            edges,
            authorities,
            set,
        } = aim;
        self.head = head;
        self.rest = rest;
        self.layering = layering;
        self.live = live;
        self.capacity = capacity;
        self.seed_salt = seed_salt;
        self.salts = salts;
        self.camera = camera;
        self.overrides = overrides;
        self.stated = false;
        self.published = published;
        self.bindings = bindings;
        self.edges = edges;
        self.authorities = authorities;
        if let Some((_, running)) = &mut self.snapshots {
            *running = set;
        }
        self.stamps = self.stamp();
        self.settling = false;
        true
    }

    /// Compiles and builds the currently watched slot files, returning `None` on failure.
    fn rebuild(&mut self) -> Option<Polled> {
        let slot = self.slot;
        eprintln!("slot {slot}: recompiling:");
        let named: Vec<&crate::compile::Named> = std::iter::once(&self.head)
            .chain(self.rest.iter())
            .collect();
        let paths: Vec<&std::path::Path> = named.iter().map(|n| n.path.as_path()).collect();
        let mut srcs = Vec::with_capacity(paths.len());
        for path in &paths {
            match std::fs::read_to_string(path) {
                Ok(src) => srcs.push(src),
                Err(e) => {
                    eprintln!("{}: {e}\nslot {slot} unchanged", path.display());
                    return None;
                }
            }
        }
        let mut compiled = Vec::with_capacity(paths.len());
        for (named, src) in named.iter().zip(&srcs) {
            match compile::diagnose(src) {
                Ok(checked) => compiled.push((
                    (*named).clone(),
                    checked,
                    std::sync::Arc::from(src.as_str()),
                )),
                Err(diagnostics) => {
                    eprintln!(
                        "{}:\n{diagnostics}\nslot {slot} unchanged; its Set is still running",
                        named.path.display()
                    );
                    return Some(Polled::Refused(Refusal {
                        label: named.name.clone().unwrap_or_else(|| {
                            named
                                .path
                                .file_name()
                                .map(|f| f.to_string_lossy().into_owned())
                                .unwrap_or_default()
                        }),
                        said: diagnostics.said,
                    }));
                }
            }
        }
        let (material, placed) = match crate::compile::sort_compiled(compiled) {
            Ok(sorted) => sorted,
            Err(e) => {
                eprintln!("slot {slot}: {e}\nslot {slot} unchanged; its Set is still running");
                return None;
            }
        };

        if let Some((snapshots, set)) = &self.snapshots {
            match snapshots.lock() {
                Ok(mut snapshots) => {
                    for node in &placed {
                        let layer = crate::setfile::kind_name(node.layer);
                        let index = node.index as usize;
                        if let Err(e) = snapshots.record(
                            slot,
                            layer,
                            index,
                            set.as_deref(),
                            &node.proc,
                            node.source.as_bytes(),
                        ) {
                            eprintln!("slot {slot}: this version is not in the edit history: {e}");
                        }
                    }
                }
                Err(_) => eprintln!("slot {slot}: the edit history is not being written"),
            }
        }

        let label = placed
            .iter()
            .map(|node| node.proc.as_str())
            .collect::<Vec<_>>()
            .join(" + ");
        self.builds += 1;
        let id = (self.slot as u64) << 32 | self.builds;
        if let Some((store, tx)) = &self.stored {
            let stored: Result<Vec<_>, _> = placed
                .iter()
                .map(|node| {
                    node.put(store)
                        .map(|hash| (crate::setfile::kind_name(node.layer), node.index, hash))
                })
                .collect();
            match stored {
                Ok(nodes) => {
                    let _ = tx.send(Built { id, nodes });
                }
                Err(e) => eprintln!(
                    "slot {slot}: this build's sources are not in the store: {e} — \
                     a replay will show the procedure it started with, and a save \
                     of this slot will write the files it started from"
                ),
            }
        }
        let crate::compile::Material {
            l1s,
            l2s,
            l3s,
            fields,
            l4s,
            names,
        } = material;
        let params = if std::mem::replace(&mut self.stated, true) {
            Vec::new()
        } else {
            self.overrides.clone()
        };
        Some(Polled::Build(Request {
            id,
            l1s: l1s
                .into_iter()
                .map(|l1| {
                    let capacity = self.capacity.unwrap_or_else(|| {
                        l1.capacity
                            .map_or(karakuri_ir::DEFAULT_CAPACITY, |c| c.default)
                    });
                    (l1, capacity)
                })
                .collect(),
            l2s,
            l3s,
            fields,
            l4s,
            names: karakuri_engine::swap::RequestNames {
                l1s: names.l1s,
                l2s: names.l2s,
                l3s: names.l3s,
                l4s: names.l4s,
                fields: names.fields,
            },
            edges: self.edges.clone(),
            layering: self.layering,
            live: self.live,
            seed_salt: self.seed_salt,
            salts: self.salts.iter().copied().map(Some).collect(),
            camera: self.camera,
            params,
            published: self.published.clone(),
            bindings: self.bindings.clone(),
            authorities: self.authorities.clone(),
            label,
        }))
    }
}

#[cfg(test)]
mod tests;
