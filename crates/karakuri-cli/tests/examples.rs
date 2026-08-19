//! Every `.kir` in `examples/`, through the whole front end.
//!
//! **These are the files the README and the manual tell people to run**, and
//! before this nothing in the suite opened one. A stale example is not a cosmetic
//! problem: it is the first thing anyone types, and the first thing a model is
//! pointed at when it asks what the language looks like.
//!
//! Two claims. Every file on its own terms, which is where rot shows first — a
//! renamed builtin, a tightened range, a `blend` value that stopped existing —
//! and then the *pairs* the documentation offers, since a command line in the
//! README is a claim about a pair and `Set::build` is where a pair is judged.
//!
//! Which L1 goes with which L4 is written in prose and in the files' own
//! comments, never in the files, so the pair list below is by hand. Deriving one
//! would be this test inventing an answer to a question the language has not
//! asked.

use std::path::{Path, PathBuf};

fn examples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .canonicalize()
        .expect("examples/")
}

fn kir_files() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(examples())
        .expect("read examples/")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "kir"))
        .collect();
    // So a failure names the same file every run.
    found.sort();
    found
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
        let name = path.file_name().expect("a file name").to_string_lossy().to_string();
        let src = std::fs::read_to_string(&path).expect("read");
        let render = |errs: &[karakuri_ir::IrError]| {
            errs.iter().map(|e| e.render(&src)).collect::<Vec<_>>().join("\n")
        };

        let parsed = karakuri_ir::parse(&src)
            .unwrap_or_else(|e| panic!("{name} does not parse:\n{}", render(&e)));
        let checked = karakuri_ir::check::check(&parsed)
            .unwrap_or_else(|e| panic!("{name} does not check:\n{}", render(&e)));
        karakuri_ir::cost::estimate(&checked)
            .unwrap_or_else(|e| panic!("{name} is over budget:\n{}", render(&e)));
    }
}

/// **The pairs the documentation names actually compose.** `Set::build` is where
/// `consumes ⊆ emit` is decided, and it is also where the rules a single file
/// cannot express live — `blend weighted` on a fullscreen L4, for one. A README
/// that offers a command line is making a claim about the pair, not about two
/// files.
///
#[test]
fn the_pairs_the_docs_offer_compose() {
    // Not a silent skip on a machine with no GPU: a test that passes by not
    // running is worse than one that fails, and every other GPU test in this
    // workspace says the same thing this way.
    let gpu = karakuri_engine::Gpu::headless().expect("no GPU available");
    let pairs = [
        ("drift_shell.kir", "soft_points.kir"),
        ("drift_shell.kir", "drift_streaks.kir"),
        ("drift_shell.kir", "glass_shell.kir"),
        ("drift_shell.kir", "field_march.kir"),
        ("strand_shell.kir", "strand_strokes.kir"),
        // The three a model wrote in one session; the README names them as a
        // group, so both of its L4s are paired with its L1.
        ("beat_strands.kir", "beat_strokes.kir"),
        ("beat_strands.kir", "beat_bloom.kir"),
        // Named for what they are rather than for a partner — `beat_shell`
        // reads `beats` and `spark_fountain` spawns and kills — so each is
        // paired with the default renderer, which is how anyone would first
        // run one.
        ("beat_shell.kir", "soft_points.kir"),
        ("spark_fountain.kir", "soft_points.kir"),
    ];

    for (l1, l4) in pairs {
        let compile = |file: &str| {
            let src = std::fs::read_to_string(examples().join(file)).expect("read");
            let parsed = karakuri_ir::parse(&src).expect("parses");
            let checked = karakuri_ir::check::check(&parsed).expect("checks");
            karakuri_ir::cost::estimate(&checked).expect("costs");
            checked
        };
        let (a, b) = (compile(l1), compile(l4));
        // The capacity the L1 declares as its own default, so this is the pair
        // exactly as the documented command line would build it.
        let capacity = a.capacity.expect("an L1 declares a capacity").default;
        karakuri_engine::Set::build(&gpu.device, &gpu.queue, &a, &b, capacity, 0)
            .unwrap_or_else(|e| panic!("{l1} + {l4} does not build: {e}"));
    }

    // **And the chains**, which a pair list cannot express and which the
    // examples' own comments give as command lines. Each is one L1, the middle
    // files in the order written, and one renderer — sorted by the `kind` each
    // declares, exactly as `--set` sorts them.
    for chain in [
        // `swirl_warp.kir`'s own header offers this one.
        &["drift_shell.kir", "swirl_warp.kir", "soft_points.kir"][..],
        // `beat_jump.kir`'s does.
        &["drift_shell.kir", "beat_jump.kir", "soft_points.kir"][..],
        // `late_bloom.kir`'s does, and it is the one with two modulators in it:
        // a chain is where an L2 being stateless stops being a claim and starts
        // being the thing that lets the pair be written in either order.
        &["drift_shell.kir", "swirl_warp.kir", "late_bloom.kir", "soft_points.kir"][..],
        // `kaleidoscope.kir`'s own header offers this one, and it is the only
        // chain here that changes the element count.
        &["drift_shell.kir", "kaleidoscope.kir", "soft_points.kir"][..],
        // `field_lens.kir`'s does: a marcher containing no shape, and a shape
        // that is nothing else. Neither builds without the other.
        &["drift_shell.kir", "melt_blob.kir", "field_lens.kir"][..],
        // `morph.kir`'s does, and it is the only one with **two geometries** in
        // it. The loop below takes every L1 it finds with that L1's own declared
        // capacity, which is what makes this line a test of more than the sort:
        // a pairing Set is refused unless both sources are the same size.
        &["lattice_shell.kir", "sphere_shell.kir", "morph.kir", "soft_points.kir"][..],
    ] {
        let compiled: Vec<karakuri_ir::typed::Checked> = chain
            .iter()
            .map(|file| {
                let src = std::fs::read_to_string(examples().join(file)).expect("read");
                let parsed = karakuri_ir::parse(&src).expect("parses");
                let checked = karakuri_ir::check::check(&parsed).expect("checks");
                karakuri_ir::cost::estimate(&checked).expect("costs");
                checked
            })
            .collect();
        let by = |kind| compiled.iter().filter(move |c| c.kind == kind);
        // **Every** L1, each at the capacity its own file declares — which is
        // what the command line does, and what a pairing chain needs two of.
        let sources: Vec<(&karakuri_ir::typed::Checked, u32)> = by(karakuri_ir::Kind::L1)
            .map(|l1| (l1, l1.capacity.expect("an L1 declares a capacity").default))
            .collect();
        assert!(!sources.is_empty(), "a chain starts with an L1");
        let l2s: Vec<&karakuri_ir::typed::Checked> = by(karakuri_ir::Kind::L2).collect();
        let l3 = by(karakuri_ir::Kind::L3).next();
        let l4s: Vec<&karakuri_ir::typed::Checked> = by(karakuri_ir::Kind::L4).collect();
        karakuri_engine::Set::build_many(
            &gpu.device,
            &gpu.queue,
            &sources,
            &l2s,
            l3,
            by(karakuri_ir::Kind::Field).next(),
            &l4s,
            karakuri_engine::set::Layering::Overdraw,
            0,
        )
        .unwrap_or_else(|e| panic!("{} does not build: {e}", chain.join(" + ")));
    }
}
