//! Loading `.kir` sources and running them through the validation and compilation pipeline.

use std::path::{Path, PathBuf};

use karakuri_ir::typed::Checked;

use crate::meta::put_meta;

/// Names assigned to each node within a slot, partitioned by layer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Names {
    pub l1s: Vec<Option<String>>,
    pub l2s: Vec<Option<String>>,
    /// Optional names for L3 camera nodes. The built-in orbit camera is named `orbit`.
    pub l3s: Vec<Option<String>>,
    pub l4s: Vec<Option<String>>,
    pub fields: Vec<Option<String>>,
}

impl Names {
    /// Every name that was actually written, which is the only thing worth checking
    /// for a collision: a derived one is disambiguated where it is derived, in
    /// `Set::build_many`.
    fn written(&self) -> impl Iterator<Item = &String> {
        self.l1s
            .iter()
            .flatten()
            .chain(self.l2s.iter().flatten())
            .chain(self.l3s.iter().flatten())
            .chain(self.l4s.iter().flatten())
            .chain(self.fields.iter().flatten())
    }

    /// Verifies that all explicitly assigned node names within this slot are unique.
    pub fn check_unique(&self) -> Result<(), String> {
        let mut seen: Vec<&str> = Vec::new();
        for name in self.written() {
            if seen.contains(&name.as_str()) {
                return Err(format!(
                    "two nodes are both called `{name}` — a name addresses one node in a slot"
                ));
            }
            seen.push(name);
        }
        Ok(())
    }
}

/// Loads and compiles a `.kir` source file, returning both the checked IR and source text.
pub fn load(path: &Path) -> Result<(Checked, String), String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let checked = compile(&src).map_err(|report| format!("{}:\n{report}", path.display()))?;
    Ok((checked, src))
}

/// Diagnostics from a refused compilation unit in full and summary form.
///
/// `report` contains the full formatted compiler error output, while `said`
/// provides single-line summaries per diagnostic for compact UI display.
pub struct Diagnostics {
    /// Rendered against the source it came from, diagnostics separated by a blank
    /// line — what was printed before this type existed, unchanged.
    pub report: String,
    /// The same diagnostics, one line each: where, which stage, and what.
    pub said: Vec<String>,
}

impl std::fmt::Display for Diagnostics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.report)
    }
}

/// Compiles in-memory `.kir` source code, returning the checked IR or formatted error string.
pub fn check(src: &str) -> Result<Checked, String> {
    compile(src).map_err(|diagnostics| diagnostics.report)
}

/// Compiles in-memory `.kir` source code, returning structured [`Diagnostics`] on error.
pub fn diagnose(src: &str) -> Result<Checked, Diagnostics> {
    compile(src)
}

fn compile(src: &str) -> Result<Checked, Diagnostics> {
    let proc = karakuri_ir::parse(src).map_err(|e| render(&e, src))?;
    let checked = karakuri_ir::check::check(&proc).map_err(|e| render(&e, src))?;
    let cost = karakuri_ir::cost::estimate(&checked).map_err(|e| render(&e, src))?;
    // Fields have no element spawn count, reporting ops/evaluation instead.
    if proc.kind == karakuri_ir::Kind::Field {
        eprintln!(
            "  {} — {} ops/evaluation",
            proc.name, cost.ops_per_evaluation
        );
    } else {
        eprintln!(
            "  {} — {} ops/element, {} ops/spawn",
            proc.name, cost.ops_per_element, cost.ops_per_spawn
        );
    }
    Ok(checked)
}

fn render(errors: &[karakuri_ir::IrError], src: &str) -> Diagnostics {
    let rendered: Vec<String> = errors.iter().map(|e| e.render(src)).collect();
    // Extract first line of each rendered diagnostic (`line:col: stage: message`).
    let said = rendered
        .iter()
        .map(|d| d.lines().next().unwrap_or_default().to_owned())
        .collect();
    Diagnostics {
        report: rendered.join("\n\n"),
        said,
    }
}

/// A `.kir` file reference, optionally paired with an explicit node name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Named {
    pub name: Option<String>,
    pub path: PathBuf,
}

impl Named {
    pub fn bare(path: impl Into<PathBuf>) -> Named {
        Named {
            name: None,
            path: path.into(),
        }
    }

    /// Parses an argument string as `name=path` or a bare path.
    pub fn parse(spelled: &str) -> Result<Named, String> {
        let Some((name, path)) = spelled.split_once('=') else {
            return Ok(Named::bare(spelled));
        };
        check_node_name(name)?;
        if path.is_empty() {
            return Err(format!("`{spelled}` — a name with no file after it"));
        }
        Ok(Named {
            name: Some(name.to_string()),
            path: PathBuf::from(path),
        })
    }
}

/// Validates that a node name contains only legal identifiers and is not a reserved layer keyword.
fn check_node_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("a node name cannot be empty".to_string());
    }
    // Layer names are reserved identifiers to disambiguate node names from layer addresses.
    if crate::meta::layer_named(name).is_some() {
        return Err(format!(
            "`{name}` is a layer, so it cannot also be a node's name — a `--param` is told \
             which of the two it names by the shape of what follows"
        ));
    }
    if let Some(bad) = name
        .chars()
        .find(|c| !c.is_ascii_alphanumeric() && *c != '_' && *c != '-')
    {
        return Err(format!(
            "`{name}` cannot be a node name: `{bad}` is not allowed. Letters, digits, `_` and `-`"
        ));
    }
    Ok(())
}

/// Compiled procedures for a single deck slot partitioned by layer pipeline order.
pub struct Material {
    /// The geometry sources, in the order their paths appeared. At least one;
    /// several is a merge.
    pub l1s: Vec<karakuri_ir::typed::Checked>,
    pub l2s: Vec<karakuri_ir::typed::Checked>,
    /// The cameras, in the order their paths appeared. Empty leaves the slot
    /// looking from the built-in orbit, which is a node all the same — so a Set has
    /// at least one camera however this list comes out.
    pub l3s: Vec<karakuri_ir::typed::Checked>,
    /// The fields, in the order their paths appeared. Empty for a slot that
    /// evaluates none.
    pub fields: Vec<karakuri_ir::typed::Checked>,
    pub l4s: Vec<karakuri_ir::typed::Checked>,
    /// What each of those is called, in the same per-layer shape.
    pub names: Names,
}

/// Placement metadata for a compiled file: declared layer, layer node index,
/// and associated invocation name and path.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub named: Named,
    /// Declared procedure name from the `.kir` source.
    pub(crate) proc: String,
    pub layer: karakuri_ir::Kind,
    pub index: u32,
    /// Exact source text this node was compiled from.
    pub source: std::sync::Arc<str>,
    /// Metadata card generated from the compiled IR (see [`crate::meta::card`]).
    pub meta: std::sync::Arc<[karakuri_store::ndjson::Line]>,
}

impl Placed {
    /// Derives the cryptographic content hash of this node's source text.
    pub fn hash(&self) -> karakuri_store::hash::Hash {
        karakuri_store::hash::Hash::of(self.source.as_bytes())
    }

    /// Persists this node's source in `store`.
    ///
    /// Writes the source artifact and attempts to save the metadata card. Returns
    /// the source hash on success.
    pub fn put(
        &self,
        store: &karakuri_store::store::Store,
    ) -> Result<karakuri_store::hash::Hash, String> {
        let hash = store
            .put_artifact(self.source.as_bytes())
            .map_err(|e| format!("{}: {e}", self.named.path.display()))?;
        put_meta(store, &hash, &self.meta);
        Ok(hash)
    }
}

/// Sorts compiled procedures by declared layer kind, preserving list order within each layer.
///
/// Validates layer compatibility (e.g. requires at least one L4 renderer) and builds
/// node placement records. Returns an error description on validation failure.
pub fn sort_compiled(
    compiled: Vec<(Named, karakuri_ir::typed::Checked, std::sync::Arc<str>)>,
) -> Result<(Material, Vec<Placed>), String> {
    // Taken before the loop consumes the list. The "nothing draws" refusal
    // names the file the slot was given, which is the one an operator looks at
    // first — and by then it has been moved from.
    let head = compiled
        .first()
        .map(|(named, ..)| named.path.display().to_string());
    let mut names = Names::default();
    let mut l1s = Vec::new();
    let mut l2s = Vec::new();
    let mut l3s: Vec<karakuri_ir::typed::Checked> = Vec::new();
    let mut fields: Vec<karakuri_ir::typed::Checked> = Vec::new();
    let mut l4s = Vec::new();
    let mut placed = Vec::new();
    for (named, checked, source) in compiled {
        let name = named.name.clone();
        let proc = checked.name.clone();
        // Retain the checked IR reference to construct the node's metadata card below.
        let (layer, index, filed) = match checked.kind {
            karakuri_ir::Kind::L2 => {
                names.l2s.push(name);
                l2s.push(checked);
                (karakuri_ir::Kind::L2, l2s.len() - 1, l2s.last())
            }
            karakuri_ir::Kind::L4 => {
                names.l4s.push(name);
                l4s.push(checked);
                (karakuri_ir::Kind::L4, l4s.len() - 1, l4s.last())
            }
            // Support multiple camera nodes; renderers bind to them by edge name or default to the first.
            karakuri_ir::Kind::L3 => {
                names.l3s.push(name);
                l3s.push(checked);
                (karakuri_ir::Kind::L3, l3s.len() - 1, l3s.last())
            }
            // Support multiple field nodes bound via edge declarations.
            karakuri_ir::Kind::Field => {
                names.fields.push(name);
                fields.push(checked);
                (karakuri_ir::Kind::Field, fields.len() - 1, fields.last())
            }
            // Support multiple independent L1 geometry sources within a single slot.
            karakuri_ir::Kind::L1 => {
                names.l1s.push(name);
                l1s.push(checked);
                (karakuri_ir::Kind::L1, l1s.len() - 1, l1s.last())
            }
            // L5 procedures are not yet routable in deck slots; reject until master chain slot support lands.
            karakuri_ir::Kind::L5 => {
                return Err(format!(
                    "{} declares `kind L5`, and a Set has nowhere to put one yet — a frame \
                     effect runs in the master chain, which is still three fixed passes. The \
                     file compiles and can be stored; what is missing is the chain's slots",
                    named.path.display()
                ));
            }
        };
        let checked = filed.expect("the procedure was pushed onto that layer's list above");
        // Derive node hash and build metadata card before storing placement record.
        let mut node = Placed {
            named,
            proc,
            layer,
            index: index as u32,
            source,
            meta: Vec::new().into(),
        };
        node.meta = crate::meta::card(&node.hash(), checked).into();
        placed.push(node);
    }
    if l4s.is_empty() {
        return Err(match head {
            Some(head) => format!(
                "nothing here draws — {head} names no L4, and a Set with no \
                 renderer has no frame to give"
            ),
            // Nothing at all to sort is the same refusal with no file to point
            // at. Neither caller can reach it — a slot is spelled with a head —
            // and answering it here is cheaper than making that a precondition.
            None => "nothing here draws — a Set with no renderer has no frame to give".to_string(),
        });
    }
    names.check_unique()?;
    Ok((
        Material {
            l1s,
            l2s,
            l3s,
            fields,
            l4s,
            names,
        },
        placed,
    ))
}

/// Loads, compiles, and sorts procedures for a slot, exiting on any compilation or sorting error.
pub fn sort_slot(slot: usize, l1: &Named, rest: &[Named]) -> (Material, Vec<Placed>) {
    let named: Vec<String> = rest.iter().map(|p| p.path.display().to_string()).collect();
    eprintln!(
        "  slot {slot}: {} + {}",
        l1.path.display(),
        named.join(" + ")
    );
    // The text each file was compiled from travels with it.
    let compiled: Vec<(Named, karakuri_ir::typed::Checked, std::sync::Arc<str>)> =
        std::iter::once(l1)
            .chain(rest)
            .map(|named| match load(&named.path) {
                Ok((checked, src)) => (named.clone(), checked, std::sync::Arc::from(src.as_str())),
                Err(report) => {
                    eprintln!("{report}");
                    std::process::exit(1);
                }
            })
            .collect();
    match sort_compiled(compiled) {
        Ok(sorted) => sorted,
        Err(e) => {
            eprintln!("slot {slot}: {e}");
            std::process::exit(1);
        }
    }
}
