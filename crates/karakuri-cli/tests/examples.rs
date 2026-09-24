//! End-to-end integration tests validating all example `.kir` and `.kset` files.
//!
//! Validates syntax, type checking, and contracts for standalone `.kir` files
//! as well as combinations defined in `examples/*.kset` (ADR-0229, ADR-0231).

use std::path::{Path, PathBuf};

fn examples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .canonicalize()
        .expect("examples/")
}

/// Every file in `examples/` with the given extension, sorted so a failure names
/// the same file every run.
fn files_with(ext: &str) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(examples())
        .expect("read examples/")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == ext))
        .collect();
    found.sort();
    found
}

fn kir_files() -> Vec<PathBuf> {
    files_with("kir")
}

/// One node of a Set, as the authoring form writes it: a layer, an index within
/// that layer, and the `.kir` by relative path.
struct Part {
    layer: String,
    index: u32,
    path: String,
}

/// One edge, in the `.kset`'s own terms — both ends by name, neither of them a
/// position.
struct Edge {
    node: String,
    slot: String,
    to: String,
}

/// Represents an `examples/*.kset` file, parsed independently of `karakuri_store::Record`
/// to guard against silent format drift in shipped files.
struct Kset {
    /// The file's stem, which is the id an operator will point at.
    stem: String,
    id: String,
    parts: Vec<Part>,
    edges: Vec<Edge>,
}

fn ksets() -> Vec<Kset> {
    files_with("kset")
        .into_iter()
        .map(|path| {
            let stem = path
                .file_stem()
                .expect("a file name")
                .to_string_lossy()
                .to_string();
            let text = std::fs::read_to_string(&path).expect("read");
            let mut id = String::new();
            let (mut parts, mut edges) = (Vec::new(), Vec::new());
            for (at, line) in text.lines().enumerate() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let at = at + 1;
                let record: serde_json::Value = serde_json::from_str(line)
                    .unwrap_or_else(|e| panic!("{stem}.kset:{at} is not one JSON record: {e}"));
                let field = |key: &str| -> String {
                    record[key]
                        .as_str()
                        .unwrap_or_else(|| panic!("{stem}.kset:{at} has no `{key}`"))
                        .to_string()
                };
                match record["t"].as_str() {
                    Some("set") => id = field("id"),
                    Some("part") => parts.push(Part {
                        layer: field("layer"),
                        // Absent means 0 and 0 is not written, which is the rule
                        // a `slot`'s index already follows.
                        index: record["index"].as_u64().unwrap_or(0) as u32,
                        path: field("path"),
                    }),
                    Some("edge") => edges.push(Edge {
                        node: field("node"),
                        slot: field("slot"),
                        to: field("to"),
                    }),
                    _ => {}
                }
            }
            assert!(!id.is_empty(), "{stem}.kset has no `set` record");
            Kset {
                stem,
                id,
                parts,
                edges,
            }
        })
        .collect()
}

fn compile(file: &str) -> karakuri_ir::typed::Checked {
    let path = examples().join(file);
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {file}: {e}"));
    let render = |errs: &[karakuri_ir::IrError]| {
        errs.iter()
            .map(|e| e.render(&src))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let parsed = karakuri_ir::parse(&src)
        .unwrap_or_else(|e| panic!("{file} does not parse:\n{}", render(&e)));
    let checked = karakuri_ir::check::check(&parsed)
        .unwrap_or_else(|e| panic!("{file} does not check:\n{}", render(&e)));
    karakuri_ir::cost::estimate(&checked)
        .unwrap_or_else(|e| panic!("{file} is over budget:\n{}", render(&e)));
    checked
}

/// Validates that all examples parse, type-check, and pass cost estimates.
#[test]
fn every_example_parses_checks_and_costs() {
    let files = kir_files();
    assert!(
        files.len() >= 12,
        "only {} `.kir` files found in {} — is the path right?",
        files.len(),
        examples().display()
    );

    for path in files {
        compile(&path.file_name().expect("a file name").to_string_lossy());
    }
}

/// Validates that every .kset file references existing source files with matching layer declarations.
#[test]
fn every_set_names_parts_that_are_there() {
    let sets = ksets();
    assert!(
        !sets.is_empty(),
        "no `.kset` files in {} — is the path right?",
        examples().display()
    );

    for set in &sets {
        // The store derives an id by stripping the suffix, so a file whose
        // header disagrees with its name is a Set that answers to two names and
        // is loadable under one.
        assert_eq!(
            set.id, set.stem,
            "{}.kset calls itself `{}` — the id is the file name without the suffix",
            set.stem, set.id
        );
        assert!(!set.parts.is_empty(), "{}.kset names no parts", set.stem);

        for part in &set.parts {
            // These ship beside their parts, so every path here is a bare
            // neighbour. Containment is the resolver's wall to enforce and this
            // is not it — but a separator appearing in a *shipped* file is
            // something to hear about from the suite rather than from the wall.
            assert!(
                !part.path.contains('/') && !part.path.contains('\\'),
                "{}.kset names `{}` — the parts sit in the same directory",
                set.stem,
                part.path
            );
            assert!(
                examples().join(&part.path).is_file(),
                "{}.kset names `{}`, which is not in {}",
                set.stem,
                part.path,
                examples().display()
            );
            let kind = format!("{:?}", compile(&part.path).kind);
            assert_eq!(
                kind, part.layer,
                "{}.kset puts `{}` on {} and the file declares {kind}",
                set.stem, part.path, part.layer
            );
        }
    }
}

/// Ensures every .kir source file in examples belongs to at least one .kset, or is explicitly exempted.
#[test]
fn every_part_is_in_some_set() {
    let named: Vec<String> = ksets()
        .iter()
        .flat_map(|s| s.parts.iter().map(|p| p.path.clone()))
        .collect();
    // Frame effects operating in the master chain rather than inside Sets.
    let in_no_set: [(&str, &str); 3] = [
        (
            "feedback.kir",
            "a master chain slot, and the chain is one level out from every Set",
        ),
        (
            "bloom.kir",
            "a master chain slot, and the chain is one level out from every Set",
        ),
        (
            "rgb_shift.kir",
            "a master chain slot, and the chain is one level out from every Set",
        ),
    ];

    for path in kir_files() {
        let file = path.file_name().expect("a file name").to_string_lossy();
        if let Some((_, why)) = in_no_set.iter().find(|(f, _)| *f == file) {
            assert!(!why.is_empty(), "{file} is excused without a reason");
            continue;
        }
        assert!(
            named.contains(&file.to_string()),
            "{file} is in no `.kset`. Write the Set its header offers, or say \
             here why it is part of nothing."
        );
    }
}

mod gpu {
    use super::*;

    /// Verifies that all bundled .kset definitions successfully compile and compose on GPU.
    #[test]
    fn the_sets_beside_the_parts_compose() {
        let gpu = karakuri_engine::Gpu::headless().expect("no GPU available");

        for set in ksets() {
            let compiled: Vec<(&Part, karakuri_ir::typed::Checked)> = set
                .parts
                .iter()
                .map(|part| (part, compile(&part.path)))
                .collect();
            // The order within a layer is the file's, which is what an `index`
            // addresses and what `--set` produces from a command line.
            let on = |layer: &str| -> Vec<&karakuri_ir::typed::Checked> {
                let mut of: Vec<&(&Part, karakuri_ir::typed::Checked)> =
                    compiled.iter().filter(|(p, _)| p.layer == layer).collect();
                of.sort_by_key(|(p, _)| p.index);
                of.iter().map(|(_, c)| c).collect()
            };
            let l1s: Vec<(&karakuri_ir::typed::Checked, u32)> = on("L1")
                .into_iter()
                .map(|l1| (l1, l1.capacity.expect("an L1 declares a capacity").default))
                .collect();
            assert!(!l1s.is_empty(), "{}.kset starts with no L1", set.stem);
            let (l2s, l3s, l4s, fields) = (on("L2"), on("L3"), on("L4"), on("Field"));
            let edges: Vec<karakuri_engine::set::Edge> = set
                .edges
                .iter()
                .map(|e| karakuri_engine::set::Edge {
                    node: e.node.clone(),
                    slot: e.slot.as_str().into(),
                    to: e.to.clone(),
                })
                .collect();

            let built = if l1s.len() == 1
                && l4s.len() == 1
                && l2s.is_empty()
                && l3s.is_empty()
                && fields.is_empty()
                && edges.is_empty()
            {
                let (l1, capacity) = l1s[0];
                karakuri_engine::Set::build(&gpu.device, &gpu.queue, l1, l4s[0], capacity, 0)
            } else {
                karakuri_engine::Set::build_many(
                    &gpu.device,
                    &gpu.queue,
                    &l1s,
                    &l2s,
                    &l3s,
                    &fields,
                    &l4s,
                    karakuri_engine::set::Layering::Overdraw,
                    0,
                    &[],
                    karakuri_engine::set::Wiring {
                        edges: &edges,
                        ..Default::default()
                    },
                )
            };
            built.unwrap_or_else(|e| panic!("{}.kset does not build: {e}", set.stem));
        }
    }
}
