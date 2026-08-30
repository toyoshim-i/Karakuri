//! Every `.kir` in `examples/`, through the whole front end — and every `.kset`
//! beside them, which is what says which of those parts make a Set.
//!
//! **These are the files the manual tells people to run**, and
//! before this nothing in the suite opened one. A stale example is not a cosmetic
//! problem: it is the first thing anyone types, and the first thing a model is
//! pointed at when it asks what the language looks like.
//!
//! Two claims. Every file on its own terms, which is where rot shows first — a
//! renamed builtin, a tightened range, a `blend` value that stopped existing —
//! and then the *combinations* the documentation offers, since a command line in
//! the manual is a claim about a combination and `Set::build` is where one is
//! judged.
//!
//! **The combinations used to be an array in this file, and now they are the
//! `.kset` files themselves.** That array's own header said why it had to be by
//! hand — *"Which L1 goes with which L4 is written in prose and in the files'
//! own comments, never in the files, so the pair list below is by hand"* — and
//! the authoring form ([ADR-0229], [ADR-0231]) is the language being asked. So
//! this test stopped holding the answer and started checking the files that do:
//! it reads `examples/*.kset`, and a combination nobody wrote down is a
//! combination nothing here builds. Two checks come with the move that a list in
//! Rust could not make — that every part a Set names is a file that is there, and
//! that every part in the directory is named by some Set.
//!
//! [ADR-0229]: ../../../docs/adr/0229-a-set-file-is-authored-beside-its-parts-and-travels-as-a-bundle.md
//! [ADR-0231]: ../../../docs/adr/0231-a-sets-two-forms-take-two-extensions-and-the-store-holds-only-the-resolved-one.md

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

/// One `examples/*.kset`, read as the file rather than through a decoder.
///
/// **Deliberately parsed here rather than by `karakuri_store::Record`.** What
/// these tests defend is the *shipped files*, and a reader that shared the
/// program's vocabulary would pass on a file the program will one day stop
/// understanding — a renamed field would move both halves at once and nothing
/// would fail. So this reads `t`, the four fields it needs, and ignores the
/// rest, which is what the format's own forward-compatibility rule says a reader
/// does with a `t` it does not know.
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

/// Parse, check and cost. All three, because they refuse different things: the
/// parser knows the grammar, the check pass knows the contracts and the types,
/// and cost estimation is what a ceiling refuses — the last of which has already
/// turned an example away once, when a raymarcher met a budget calibrated for
/// sprites.
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

/// **A Set names its parts by relative path, so the check a hand-written list
/// could never make is that the paths are real.**
///
/// It is the whole difference between the pairing living in prose and living in
/// a file: prose that names a file that was renamed reads exactly as well as
/// prose that does not, and this does not. The layer is checked against the
/// procedure's own `kind` for the same reason — a `.kset` saying `L2` over a file
/// declaring `L4` is a Set that will not build, and finding that out here names
/// the line rather than the driver.
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

/// **And the other direction: a part in this directory that no Set names.**
///
/// The list this replaced could not ask it, being a list — it knew what it held
/// and nothing about what it left out, so a `.kir` added beside it joined the
/// suite as a file that parses and was in no Set anybody built. Every part here
/// is documented with a command line in its own header, which is what a `.kset`
/// now carries; a part that is genuinely in no Set belongs in the list below
/// **with the reason**, rather than in a Set invented to hold it.
#[test]
fn every_part_is_in_some_set() {
    let named: Vec<String> = ksets()
        .iter()
        .flat_map(|s| s.parts.iter().map(|p| p.path.clone()))
        .collect();
    // Nothing is on it. Kept because an empty exception list is a statement —
    // it says the directory is parts and Sets and no leftovers — and because
    // the next file that cannot be in one needs somewhere to say why.
    let in_no_set: [(&str, &str); 0] = [];

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

// `Set::build` needs a device, so the composition check does; parsing, checking
// and costing every example does not, and that is the half worth running
// anywhere. See `karakuri-engine/tests/gpu_tests_are_under_mod_gpu.rs`.
mod gpu {
    use super::*;

    /// **The Sets shipped beside the parts actually compose.** `Set::build` is
    /// where `consumes ⊆ emit` is decided, and it is also where the rules a
    /// single file cannot express live — `blend weighted` on a fullscreen L4,
    /// for one. A `.kset` is a claim about a combination, not about a directory
    /// of files, and this is where the claim is judged.
    ///
    /// **With the edges the file carries.** A node that declares a geometry,
    /// field or camera slot is refused unless the Set says which node fills it,
    /// so the wiring is as much a part of the Set as the part list is — and one
    /// built without it here would be a file this test says composes and an
    /// operator cannot load.
    #[test]
    fn the_sets_beside_the_parts_compose() {
        // Not a silent skip on a machine with no GPU: a test that passes by not
        // running is worse than one that fails, and every other GPU test in this
        // workspace says the same thing this way.
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
            // **Every L1, each at the capacity its own file declares** — which is
            // what the documented command line does, and what a pairing Set
            // needs two of.
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
                    slot: e.slot.clone(),
                    to: e.to.clone(),
                })
                .collect();

            // **A pair is still built the way a pair is built.** `Set::build` is
            // the entry the documented two-file command line reaches and the one
            // `Sources::default()` uses, so a Set that is one geometry and one
            // renderer goes through it rather than through the general form it
            // delegates to.
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
